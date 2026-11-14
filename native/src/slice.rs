//! Spustenie výberu (Python): z označeného kódu a toho, čo potrebuje (importy, funkcie, triedy,
//! premenné), poskladá malý skript. Bez parsera – rozdelenie na bloky najvyššej úrovne a mená.
use std::collections::HashSet;

pub struct Sliced {
    pub code: String,
    pub deps: usize, // koľko blokov okolo výberu sa pribalilo
}

const KW: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

struct Lexed {
    stripped: String, // bez komentárov, reťazce nahradené „“; počet riadkov zostáva
    tops: Vec<bool>,  // riadok začína mimo zátvoriek a reťazca
    fexprs: Vec<String>,
}

fn is_id(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn lex(src: &str) -> Lexed {
    let b: Vec<char> = src.chars().collect();
    let (mut out, mut tops, mut fexprs) = (String::new(), vec![true], Vec::new());
    let (mut i, mut depth, mut cont) = (0usize, 0i32, false);
    while i < b.len() {
        let c = b[i];
        match c {
            '#' => {
                while i < b.len() && b[i] != '\n' {
                    i += 1;
                }
            }
            '\n' => {
                out.push('\n');
                tops.push(depth == 0 && !cont);
                cont = false;
                i += 1;
            }
            '\\' if b.get(i + 1) == Some(&'\n') => {
                cont = true;
                out.push(' ');
                i += 1;
            }
            '(' | '[' | '{' => {
                depth += 1;
                out.push(c);
                i += 1;
            }
            ')' | ']' | '}' => {
                depth = (depth - 1).max(0);
                out.push(c);
                i += 1;
            }
            '\'' | '"' => {
                // predpona reťazca (r, b, f, u…) sa odoberie z výstupu
                let mut pre = String::new();
                while out.chars().last().is_some_and(|l| "rRbBfFuU".contains(l)) && pre.len() < 2 {
                    pre.push(out.pop().unwrap());
                }
                if out.chars().last().is_some_and(is_id) {
                    out.push_str(&pre.chars().rev().collect::<String>()); // bolo to meno, nie predpona
                    pre.clear();
                }
                let is_f = pre.chars().any(|p| p == 'f' || p == 'F');
                let triple = b.get(i + 1) == Some(&c) && b.get(i + 2) == Some(&c);
                i += if triple { 3 } else { 1 };
                let mut body = String::new();
                while i < b.len() {
                    if b[i] == '\\' {
                        body.push(b[i]);
                        if let Some(n) = b.get(i + 1) {
                            body.push(*n);
                        }
                        i += 2;
                        continue;
                    }
                    if b[i] == c {
                        if !triple {
                            i += 1;
                            break;
                        }
                        if b.get(i + 1) == Some(&c) && b.get(i + 2) == Some(&c) {
                            i += 3;
                            break;
                        }
                    }
                    if b[i] == '\n' && !triple {
                        break; // nedokončený reťazec
                    }
                    body.push(b[i]);
                    i += 1;
                }
                for _ in body.chars().filter(|x| *x == '\n') {
                    out.push('\n');
                    tops.push(false);
                }
                out.push_str("\"\"");
                if is_f {
                    fexprs.extend(fstring_exprs(&body));
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Lexed { stripped: out, tops, fexprs }
}

// výrazy v {…} f-reťazca
fn fstring_exprs(body: &str) -> Vec<String> {
    let b: Vec<char> = body.chars().collect();
    let mut v = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '{' {
            if b.get(i + 1) == Some(&'{') {
                i += 2;
                continue;
            }
            let (mut d, mut e) = (1, String::new());
            i += 1;
            while i < b.len() {
                match b[i] {
                    '{' => d += 1,
                    '}' => {
                        d -= 1;
                        if d == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                e.push(b[i]);
                i += 1;
            }
            v.push(e);
        }
        i += 1;
    }
    v
}

fn idents_of(stripped: &str, out: &mut HashSet<String>) {
    let b: Vec<char> = stripped.chars().collect();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_digit() {
            while i < b.len() && (is_id(b[i]) || b[i] == '.') {
                i += 1;
            }
        } else if c.is_alphabetic() || c == '_' {
            let st = i;
            while i < b.len() && is_id(b[i]) {
                i += 1;
            }
            let w: String = b[st..i].iter().collect();
            let mut j = st;
            while j > 0 && b[j - 1] == ' ' {
                j -= 1;
            }
            let attr = j > 0 && b[j - 1] == '.' && !(j > 1 && b[j - 2] == '.');
            if !attr && !KW.contains(&w.as_str()) {
                out.insert(w);
            }
        } else {
            i += 1;
        }
    }
}

fn idents(src: &str) -> HashSet<String> {
    let lx = lex(src);
    let mut s = HashSet::new();
    idents_of(&lx.stripped, &mut s);
    for e in &lx.fexprs {
        idents_of(&lex(e).stripped, &mut s);
    }
    s
}

// logické riadky (spojené cez zátvorky) zo „stripped“ textu
fn logical(stripped: &str) -> Vec<String> {
    let (mut v, mut cur, mut d) = (Vec::new(), String::new(), 0i32);
    for l in stripped.split('\n') {
        for c in l.chars() {
            match c {
                '(' | '[' | '{' => d += 1,
                ')' | ']' | '}' => d -= 1,
                _ => {}
            }
        }
        cur.push_str(l);
        cur.push(' ');
        if d <= 0 {
            d = 0;
            v.push(std::mem::take(&mut cur));
        }
    }
    if !cur.trim().is_empty() {
        v.push(cur);
    }
    v
}

fn first_ident(s: &str) -> Option<String> {
    let s = s.trim_start_matches(|c: char| c == '*' || c.is_whitespace());
    let w: String = s.chars().take_while(|c| is_id(*c)).collect();
    (!w.is_empty() && !w.chars().next().unwrap().is_ascii_digit() && !KW.contains(&w.as_str())).then_some(w)
}

// mená v cieli priradenia: `a, (b, c), *d`, `a.x`, `a[0]` → koreň
fn targets(lhs: &str, out: &mut Vec<String>) {
    let (mut d, mut cur, mut parts) = (0i32, String::new(), Vec::new());
    for c in lhs.chars() {
        match c {
            '(' | '[' | '{' => {
                d += 1;
                cur.push(c);
            }
            ')' | ']' | '}' => {
                d -= 1;
                cur.push(c);
            }
            ',' if d == 0 => parts.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    for p in parts {
        let p = p.trim().trim_start_matches('*').trim();
        if let Some(inner) = p.strip_prefix('(').and_then(|x| x.strip_suffix(')')).or_else(|| p.strip_prefix('[').and_then(|x| x.strip_suffix(']'))) {
            targets(inner, out);
        } else if let Some(w) = first_ident(p) {
            out.push(w);
        }
    }
}

struct Assign {
    names: Vec<String>,
    mutable: Vec<String>,  // premenné priradené z literálu zoznamu/slovníka/…
    mutated: Vec<String>,  // `x.append(…)` a pod.
}

fn analyze_line(line: &str, a: &mut Assign) {
    let l = line.trim();
    if let Some(r) = l.strip_prefix("import ") {
        for part in r.split(',') {
            let part = part.trim();
            let name = match part.split(" as ").nth(1) {
                Some(al) => al.trim().to_string(),
                None => part.split('.').next().unwrap_or("").trim().to_string(),
            };
            if !name.is_empty() {
                a.names.push(name);
            }
        }
        return;
    }
    if l.starts_with("from ") {
        if let Some(p) = l.find(" import ") {
            for part in l[p + 8..].replace(['(', ')'], "").split(',') {
                let part = part.trim();
                let name = part.split(" as ").last().unwrap_or("").trim();
                if !name.is_empty() && name != "*" {
                    a.names.push(name.to_string());
                }
            }
        }
        return;
    }
    for kw in ["def ", "async def ", "class "] {
        if let Some(r) = l.strip_prefix(kw) {
            if let Some(w) = first_ident(r) {
                a.names.push(w);
            }
            return;
        }
    }
    let for_part = l.strip_prefix("for ").or_else(|| l.strip_prefix("async for "));
    if let Some(r) = for_part {
        if let Some(p) = r.find(" in ") {
            targets(&r[..p], &mut a.names);
        }
    }
    if l.starts_with("with ") || l.starts_with("async with ") || l.starts_with("except") {
        for seg in l.split(" as ").skip(1) {
            if let Some(w) = first_ident(seg) {
                a.names.push(w);
            }
        }
    }
    // morž: `(x := …)`
    let mut from = 0;
    while let Some(p) = l[from..].find(":=") {
        let at = from + p;
        if let Some(w) = l[..at].trim_end().rsplit(|c: char| !is_id(c)).next().filter(|w| !w.is_empty()) {
            a.names.push(w.to_string());
        }
        from = at + 2;
    }
    // priradenie na nulovej hĺbke
    let cs: Vec<char> = l.chars().collect();
    let (mut d, mut eqs) = (0i32, Vec::new());
    for (i, &c) in cs.iter().enumerate() {
        match c {
            '(' | '[' | '{' => d += 1,
            ')' | ']' | '}' => d -= 1,
            '=' if d == 0 => {
                let prev = if i > 0 { cs[i - 1] } else { ' ' };
                let next = cs.get(i + 1).copied().unwrap_or(' ');
                if next != '=' && !"=!<>:".contains(prev) {
                    eqs.push(i);
                } else if next == '=' {
                    // `==` – preskoč oba znaky
                }
            }
            _ => {}
        }
    }
    // `==`: druhé `=` už bolo vylúčené cez prev, prvé cez next
    if !eqs.is_empty() {
        let mut start = 0;
        for (n, &e) in eqs.iter().enumerate() {
            let mut seg: String = cs[start..e].iter().collect();
            // rozšírené priradenie `+=`, `//=`, `**=` … odstráni operátor
            while seg.ends_with(|c: char| "+-*/%&|^@<>".contains(c)) {
                seg.pop();
            }
            // anotácia `x: int`
            let (mut dd, mut cut) = (0i32, seg.len());
            for (i, c) in seg.char_indices() {
                match c {
                    '(' | '[' | '{' => dd += 1,
                    ')' | ']' | '}' => dd -= 1,
                    ':' if dd == 0 => {
                        cut = i;
                        break;
                    }
                    _ => {}
                }
            }
            let seg = &seg[..cut];
            targets(seg, &mut a.names);
            if n + 1 == eqs.len() && eqs.len() == 1 {
                let rhs: String = cs[e + 1..].iter().collect();
                let rhs = rhs.trim();
                let lit = rhs.starts_with('[') || rhs.starts_with('{') || ["list(", "dict(", "set(", "deque(", "defaultdict(", "Counter(", "OrderedDict("].iter().any(|p| rhs.starts_with(p));
                if lit {
                    if let Some(w) = first_ident(seg.trim()).filter(|w| seg.trim() == w) {
                        a.mutable.push(w);
                    }
                }
            }
            start = e + 1;
        }
        return;
    }
    // `x.append(1)`, `x[0].y()` – mení premennú bez priradenia
    if let Some(w) = first_ident(l) {
        let rest = l[w.len()..].trim_start();
        if rest.starts_with('.') || rest.starts_with('[') {
            a.mutated.push(w);
        }
    }
}

struct Block {
    s: usize, // prvý riadok
    e: usize, // posledný riadok
    text: String,
    defs: HashSet<String>,
    uses: HashSet<String>,
    mutated: Vec<String>,
    mutable: Vec<String>,
    is_def: bool,
    always: bool, // `from … import *`, `from __future__`
    guard: bool,  // if __name__ == "__main__":
}

fn blocks(lines: &[&str], tops: &[bool]) -> Vec<Block> {
    let mut starts: Vec<usize> = Vec::new();
    let mut prev_deco = false;
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let indent0 = !l.starts_with([' ', '\t']);
        let top = tops.get(i).copied().unwrap_or(true);
        let cont = ["else", "elif ", "except", "finally"].iter().any(|k| t.starts_with(k)) && (t.starts_with("else:") || t.starts_with("elif ") || t.starts_with("except") || t.starts_with("finally"));
        let is_def = ["def ", "async def ", "class "].iter().any(|k| t.starts_with(k));
        if indent0 && top && !t.starts_with([')', ']', '}']) {
            if !(cont || (prev_deco && is_def)) {
                starts.push(i);
            }
            prev_deco = t.starts_with('@');
        }
    }
    let mut v = Vec::new();
    for (n, &s) in starts.iter().enumerate() {
        let mut e = starts.get(n + 1).map(|x| x - 1).unwrap_or(lines.len() - 1);
        while e > s && lines[e].trim().is_empty() {
            e -= 1;
        }
        let text = lines[s..=e].join("\n");
        let lx = lex(&text);
        let head = text.lines().find(|l| !l.trim().is_empty() && !l.trim().starts_with('@')).unwrap_or("").trim().to_string();
        let is_def = ["def ", "async def ", "class "].iter().any(|k| head.starts_with(k));
        let mut a = Assign { names: Vec::new(), mutable: Vec::new(), mutated: Vec::new() };
        if is_def {
            let first = logical(&lx.stripped).into_iter().find(|l| !l.trim().starts_with('@')).unwrap_or_default();
            analyze_line(&first, &mut a);
        } else {
            for l in logical(&lx.stripped) {
                analyze_line(&l, &mut a);
            }
        }
        let mut uses = HashSet::new();
        idents_of(&lx.stripped, &mut uses);
        for e in &lx.fexprs {
            idents_of(&lex(e).stripped, &mut uses);
        }
        v.push(Block {
            s,
            e,
            defs: a.names.into_iter().collect(),
            uses,
            mutated: a.mutated,
            mutable: a.mutable,
            is_def,
            always: head.starts_with("from __future__") || (head.starts_with("from ") && head.contains(" import *")),
            guard: head.starts_with("if __name__"),
            text,
        });
    }
    v
}

fn dedent(s: &str) -> String {
    let ind = s.lines().filter(|l| !l.trim().is_empty()).map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').count()).min().unwrap_or(0);
    s.lines().map(|l| l.chars().skip(ind.min(l.chars().take_while(|c| c.is_whitespace()).count())).collect::<String>()).collect::<Vec<_>>().join("\n")
}

/// `a`, `b` = znakové pozície výberu v `text`.
pub fn python(text: &str, a: usize, b: usize) -> Sliced {
    let chars: Vec<char> = text.chars().collect();
    let (a, b) = (a.min(b).min(chars.len()), a.max(b).min(chars.len()));
    if a == b {
        return Sliced { code: String::new(), deps: 0 };
    }
    let line_of = |ci: usize| chars[..ci].iter().filter(|c| **c == '\n').count();
    let b2 = if chars[b - 1] == '\n' && b - 1 > a { b - 1 } else { b };
    let (ls, le) = (line_of(a), line_of(b2 - 1));
    let lines: Vec<&str> = text.split('\n').collect();
    let sel = if ls != le { dedent(&lines[ls..=le.min(lines.len() - 1)].join("\n")) } else { chars[a..b2].iter().collect::<String>().trim().to_string() };
    if sel.trim().is_empty() {
        return Sliced { code: String::new(), deps: 0 };
    }

    let lx = lex(text);
    let bl = blocks(&lines, &lx.tops);
    let overlaps = |x: &Block| x.s <= le && x.e >= ls;
    // premenné, ktoré sa dajú meniť metódou (zoznam, slovník…) – inak by sa pri `os.remove()` pustil cudzí kód
    let mutable: HashSet<String> = bl.iter().flat_map(|x| x.mutable.iter().cloned()).collect();
    let cand = |i: usize| {
        let x = &bl[i];
        !overlaps(x) && !x.guard && (x.e < ls || x.is_def)
    };
    let defs: Vec<HashSet<String>> = bl
        .iter()
        .map(|x| {
            let mut d = x.defs.clone();
            d.extend(x.mutated.iter().filter(|m| mutable.contains(*m)).cloned());
            d
        })
        .collect();

    let mut needed = idents(&sel);
    let mut inc = vec![false; bl.len()];
    for (i, x) in bl.iter().enumerate() {
        if x.always && cand(i) && x.e < ls {
            inc[i] = true;
        }
    }
    loop {
        let mut changed = false;
        for i in 0..bl.len() {
            if inc[i] || !cand(i) || defs[i].is_disjoint(&needed) {
                continue;
            }
            inc[i] = true;
            changed = true;
            needed.extend(bl[i].uses.iter().cloned());
        }
        if !changed {
            break;
        }
    }
    let mut code = String::new();
    let mut deps = 0;
    for (i, x) in bl.iter().enumerate() {
        if inc[i] {
            code.push_str(&x.text);
            code.push_str("\n\n");
            deps += 1;
        }
    }
    code.push_str(&sel);
    code.push('\n');
    Sliced { code, deps }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str, needle: &str) -> Sliced {
        let a = src.find(needle).unwrap();
        let a = src[..a].chars().count();
        python(src, a, a + needle.chars().count())
    }

    #[test]
    fn print_pulls_import_and_function() {
        let src = "import math\nimport os\nimport sys\n\ndef area(r):\n    return math.pi * r * r\n\nx = 3\nprint('start')\nprint(area(x))\nos.remove('a')\n";
        let s = run(src, "print(area(x))");
        assert!(s.code.contains("import math"));
        assert!(s.code.contains("def area"));
        assert!(s.code.contains("x = 3"));
        assert!(!s.code.contains("import os"));
        assert!(!s.code.contains("import sys"));
        assert!(!s.code.contains("print('start')"));
        assert!(s.code.trim_end().ends_with("print(area(x))"));
        assert_eq!(s.deps, 3);
    }

    #[test]
    fn plain_print_has_no_deps() {
        let s = run("import os\nprint(1)\n", "print(1)");
        assert_eq!(s.code, "print(1)\n");
        assert_eq!(s.deps, 0);
    }

    #[test]
    fn fstring_names_and_rebinding() {
        let src = "name = 'a'\nname = name + 'b'\nunused = 5\nprint(f'hi {name}')\n";
        let s = run(src, "print(f'hi {name}')");
        assert!(s.code.contains("name = 'a'") && s.code.contains("name = name + 'b'"));
        assert!(!s.code.contains("unused"));
    }

    #[test]
    fn list_mutation_and_main_guard() {
        let src = "items = []\nitems.append(1)\nos.system('x')\nif __name__ == '__main__':\n    items = [9]\nprint(items)\n";
        let s = run(src, "print(items)");
        assert!(s.code.contains("items = []") && s.code.contains("items.append(1)"));
        assert!(!s.code.contains("os.system") && !s.code.contains("[9]"));
    }

    #[test]
    fn multiline_selection_and_decorator() {
        let src = "import time\nimport random\n\n@deco\ndef f():\n    return random.random()\n\nfor i in range(2):\n    print(f())\n    time.sleep(0)\n";
        let s = run(src, "print(f())\n    time.sleep(0)");
        assert!(s.code.contains("import random") && s.code.contains("import time") && s.code.contains("def f"));
        assert!(s.code.contains("print(f())\ntime.sleep(0)"));
    }

    #[test]
    fn from_import_alias_and_multiline_call() {
        let src = "from os.path import join as j, exists\nfrom json import dumps\ndata = {\n    'a': 1,\n}\nprint(j('a', 'b'), data)\n";
        let s = run(src, "print(j('a', 'b'), data)");
        assert!(s.code.contains("from os.path import join as j, exists") && s.code.contains("data = {"));
        assert!(!s.code.contains("dumps"));
    }
}
