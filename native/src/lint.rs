// Vysvetlivky chýb priamo pri riadku: Python kontroluje skutočný prekladač Pythonu (compile), JSON serde_json,
// ostatné jazyky vlastná kontrola – neuzavreté/nespárované zátvorky, úvodzovky a komentáre, chýbajúca „;“
// v C/C++/Jave/C#, chýbajúca „:“ v Pythone (bez Pythonu), CSS deklarácie a HTML značky. Texty sú pre začiatočníkov.
use crate::i18n::{t, tf};

#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub line: usize, // od 0
    pub col: usize,  // znak od 0
    pub len: usize,  // znakov (0 = do konca riadku)
    pub err: bool,   // chyba (červená) / tip (oranžová)
    pub msg: String,
    pub color: Option<eframe::egui::Color32>, // farba od pluginu (flux.marks); None = podľa err
}

fn diag(line: usize, col: usize, len: usize, err: bool, msg: String) -> Diag {
    Diag { line, col, len, err, msg, color: None }
}

pub fn check(lang: &str, text: &str, python: Option<&str>) -> Vec<Diag> {
    let mut out = match lang {
        "py" | "pyw" => python.and_then(|py| python_compile(py, text)).unwrap_or_else(|| {
            let mut v = brackets(text, &Syntax::of("py"));
            v.extend(py_colons(text));
            v
        }),
        "json" => json(text),
        "html" | "htm" => {
            let mut v = html(text);
            v.extend(brackets_only_braces_in(text));
            v
        }
        "css" | "scss" | "less" => {
            let mut v = brackets(text, &Syntax::of("css"));
            v.extend(css_semicolons(text));
            v
        }
        "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hh" | "java" | "cs" => {
            let mut v = brackets(text, &Syntax::of(lang));
            if v.iter().all(|d| !d.err) {
                v.extend(semicolons(text, &Syntax::of(lang)));
            }
            v
        }
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" | "rs" | "go" | "kt" | "swift" | "php" | "dart" | "lua" | "rb" | "sh" | "ps1" | "r" | "jl" | "zig" | "pl" => brackets(text, &Syntax::of(lang)),
        _ => vec![],
    };
    out.sort_by_key(|d| (d.line, d.col));
    out.dedup_by_key(|d| d.line);
    out.truncate(50);
    out
}

// ---------- Python: skutočný prekladač ----------

const PY_CHECK: &str = r#"import sys, json
src = sys.stdin.buffer.read().decode('utf-8', 'replace')
try:
    compile(src, '<flux>', 'exec')
    print('[]')
except SyntaxError as e:
    print(json.dumps([[e.lineno or 1, e.offset or 1, getattr(e, 'end_offset', None) or 0, e.msg or '', getattr(e, 'end_lineno', None) or 0]]))
"#;

fn python_compile(py: &str, text: &str) -> Option<Vec<Diag>> {
    use std::io::Write;
    let mut c = std::process::Command::new(py);
    c.args(["-I", "-c", PY_CHECK]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // bez okna konzoly
    }
    let mut child = c.spawn().ok()?;
    child.stdin.take()?.write_all(text.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let mut res = vec![];
    for e in v.as_array()? {
        let line = e[0].as_u64().unwrap_or(1).max(1) as usize - 1;
        let off = e[1].as_u64().unwrap_or(1).max(1) as usize - 1;
        let end = e[2].as_u64().unwrap_or(0) as usize;
        let end_line = e[4].as_u64().unwrap_or(0) as usize;
        let msg = e[3].as_str().unwrap_or("").to_string();
        let len = if end > off + 1 && (end_line == 0 || end_line == line + 1) { end - 1 - off } else { 1 };
        let (line, off, len) = py_place(text, line, off, len, &msg);
        res.push(diag(line, off, len, true, explain_py(&msg)));
    }
    Some(res)
}

// „expected ':'“ Python hlási na konci riadku – podčiarknuť posledný znak riadku, nie ďalší riadok
fn py_place(text: &str, line: usize, off: usize, len: usize, msg: &str) -> (usize, usize, usize) {
    let lines: Vec<&str> = text.split('\n').collect();
    let n = lines.get(line).map(|l| l.chars().count()).unwrap_or(0);
    if msg.contains("expected ':'") && line < lines.len() {
        let l = lines[line].trim_end();
        let c = l.chars().count();
        return (line, c.saturating_sub(1), 1);
    }
    (line, off.min(n.saturating_sub(1)), len.max(1))
}

pub fn explain_py(msg: &str) -> String {
    let m = msg.to_lowercase();
    if m.contains("expected ':'") {
        t("Missing colon (:) at the end of this line – Python needs it after if, elif, else, for, while, def, class, try and with.")
    } else if m.contains("was never closed") {
        let b = msg.chars().find(|c| "([{".contains(*c)).unwrap_or('(');
        let close = match b {
            '[' => ']',
            '{' => '}',
            _ => ')',
        };
        tf("This {open} is never closed – add the matching {close}.", &[("open", &b.to_string()), ("close", &close.to_string())])
    } else if m.contains("unmatched") || m.contains("does not match opening parenthesis") {
        t("This bracket has no matching opening bracket – remove it or add the opening one.")
    } else if m.contains("unterminated string") || m.contains("eol while scanning") {
        t("This text (string) has no closing quote – add \" or ' at its end.")
    } else if m.contains("unterminated triple-quoted") {
        t("This text block with three quotes is never closed – add the closing three quotes.")
    } else if m.contains("perhaps you forgot a comma") {
        t("Maybe a comma is missing between these values.")
    } else if m.contains("expected an indented block") {
        t("The next line must be indented (moved right) – a line ending with a colon starts a block.")
    } else if m.contains("unexpected indent") {
        t("This line is indented more than it should be – move it left to match the lines around it.")
    } else if m.contains("unindent does not match") || m.contains("inconsistent use of tabs") {
        t("This line is indented differently than the lines around it – use the same number of spaces.")
    } else if m.contains("cannot assign to") || m.contains("maybe you meant '=='") {
        t("You can't assign here – to compare two values use == instead of =.")
    } else if m.contains("missing parentheses in call to 'print'") {
        t("print needs brackets in Python 3: print(\"text\").")
    } else if m.contains("invalid character") {
        t("This character is not allowed in code – maybe a typographic quote or dash was pasted.")
    } else if m.contains("invalid decimal literal") || m.contains("invalid syntax") {
        tf("Python doesn't understand this part ({msg}) – check for a missing operator, comma or quote.", &[("msg", msg)])
    } else {
        msg.to_string()
    }
}

// Python bez Pythonu: riadok s if/for/def… bez dvojbodky
fn py_colons(text: &str) -> Vec<Diag> {
    let mut v = vec![];
    let mut depth = 0i32;
    for (i, raw) in text.split('\n').enumerate() {
        let code = raw.split('#').next().unwrap_or("");
        let trimmed = code.trim();
        let word = trimmed.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
        let starts = depth == 0 && matches!(word, "if" | "elif" | "else" | "for" | "while" | "def" | "class" | "try" | "except" | "finally" | "with");
        for c in code.chars() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
        }
        depth = depth.max(0);
        if starts && depth == 0 && !trimmed.ends_with(':') && !trimmed.ends_with('\\') && !trimmed.contains(" lambda") {
            let c = code.trim_end().chars().count();
            v.push(diag(i, c.saturating_sub(1), 1, true, explain_py("expected ':'")));
        }
    }
    v
}

// ---------- JSON ----------

fn json(text: &str) -> Vec<Diag> {
    if text.trim().is_empty() {
        return vec![];
    }
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(_) => vec![],
        Err(e) => {
            let m = e.to_string();
            let msg = if m.contains("trailing comma") {
                t("Remove this comma – JSON allows no comma before } or ].")
            } else if m.contains("key must be a string") {
                t("Names (keys) in JSON must be in double quotes, like \"name\".")
            } else if m.contains("expected `,` or `}`") || m.contains("expected `,` or `]`") {
                t("A comma is missing between these values.")
            } else if m.contains("EOF while parsing") {
                t("The file ends too early – a } or ] is missing at the end.")
            } else if m.contains("expected value") {
                t("A value is expected here – text needs double quotes (\"…\"), and true/false/null are lowercase.")
            } else {
                m.split(" at line").next().unwrap_or(&m).to_string()
            };
            vec![diag(e.line().max(1) - 1, e.column().max(1) - 1, 1, true, msg)]
        }
    }
}

// ---------- zátvorky, úvodzovky, komentáre ----------

pub struct Syntax {
    line: &'static [&'static str],
    block: Option<(&'static str, &'static str)>,
    quotes: &'static [char],
    multiline: &'static [char], // úvodzovky, ktoré môžu pokračovať na ďalšom riadku (` v JS)
    triple: bool,               // Python ''' a """
    chars: bool,                // C-jazyky: 'a' je znak
}

impl Syntax {
    pub fn of(lang: &str) -> Syntax {
        match lang {
            "py" | "pyw" => Syntax { line: &["#"], block: None, quotes: &['"', '\''], multiline: &[], triple: true, chars: false },
            "rb" | "sh" | "ps1" | "r" | "jl" | "pl" => Syntax { line: &["#"], block: None, quotes: &['"', '\''], multiline: &['"', '\''], triple: false, chars: false },
            "lua" => Syntax { line: &["--"], block: None, quotes: &['"', '\''], multiline: &[], triple: false, chars: false },
            "css" | "scss" | "less" => Syntax { line: if lang == "css" { &[] } else { &["//"] }, block: Some(("/*", "*/")), quotes: &['"', '\''], multiline: &[], triple: false, chars: false },
            "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" | "dart" | "kt" | "swift" | "php" | "go" => {
                Syntax { line: &["//"], block: Some(("/*", "*/")), quotes: &['"', '\'', '`'], multiline: &['`'], triple: false, chars: false }
            }
            "rs" => Syntax { line: &["//"], block: Some(("/*", "*/")), quotes: &['"'], multiline: &['"'], triple: false, chars: false },
            _ => Syntax { line: &["//"], block: Some(("/*", "*/")), quotes: &['"', '\''], multiline: &[], triple: false, chars: true },
        }
    }
}

// kód bez komentárov a reťazcov (nahradené medzerami, stĺpce sedia) – pre zátvorky a „;“
// vráti (riadky, chyby neuzavretých reťazcov/komentárov)
fn strip(text: &str, sx: &Syntax) -> (Vec<Vec<char>>, Vec<Diag>) {
    let mut lines: Vec<Vec<char>> = vec![];
    let mut errs = vec![];
    let mut in_block: Option<(usize, usize)> = None;
    let mut in_str: Option<(char, bool, usize, usize)> = None; // (úvodzovka, trojitá, riadok, stĺpec)
    for (li, raw) in text.split('\n').enumerate() {
        let cs: Vec<char> = raw.trim_end_matches('\r').chars().collect();
        let mut out = vec![' '; cs.len()];
        let mut i = 0;
        while i < cs.len() {
            let rest: String = cs[i..].iter().take(3).collect();
            if let Some((bl, bc)) = in_block {
                let end = sx.block.map(|b| b.1).unwrap_or("*/");
                if cs[i..].iter().collect::<String>().starts_with(end) {
                    in_block = None;
                    i += end.chars().count();
                    let _ = (bl, bc);
                } else {
                    i += 1;
                }
                continue;
            }
            if let Some((q, triple, _, _)) = in_str {
                if cs[i] == '\\' {
                    i += 2;
                    continue;
                }
                if triple {
                    if rest == format!("{q}{q}{q}") {
                        in_str = None;
                        i += 3;
                        continue;
                    }
                } else if cs[i] == q {
                    in_str = None;
                }
                i += 1;
                continue;
            }
            // komentáre
            if sx.line.iter().any(|l| cs[i..].iter().collect::<String>().starts_with(l)) {
                break;
            }
            if let Some((b, _)) = sx.block {
                if cs[i..].iter().collect::<String>().starts_with(b) {
                    in_block = Some((li, i));
                    i += b.chars().count();
                    continue;
                }
            }
            let c = cs[i];
            if sx.quotes.contains(&c) {
                // C: 'a' je znak – preskočiť krátky znak
                if sx.chars && c == '\'' {
                    let mut j = i + 1;
                    while j < cs.len() && j < i + 6 && cs[j] != '\'' {
                        j += if cs[j] == '\\' { 2 } else { 1 };
                    }
                    if j < cs.len() && cs[j] == '\'' {
                        i = j + 1;
                        continue;
                    }
                }
                let triple = sx.triple && rest == format!("{c}{c}{c}");
                in_str = Some((c, triple, li, i));
                i += if triple { 3 } else { 1 };
                continue;
            }
            out[i] = c;
            i += 1;
        }
        // obyčajný reťazec nemôže pokračovať na ďalšom riadku
        if let Some((q, triple, sl, sc)) = in_str {
            if !triple && !sx.multiline.contains(&q) && !cs.last().is_some_and(|c| *c == '\\') {
                errs.push(diag(sl, sc, cs.len().saturating_sub(sc).max(1), true, t("This text (string) has no closing quote – add the same quote at its end.")));
                in_str = None;
            }
        }
        lines.push(out);
    }
    if let Some((q, _, sl, sc)) = in_str {
        errs.push(diag(sl, sc, 1, true, tf("This text that starts with {q} is never closed.", &[("q", &q.to_string())])));
    }
    if let Some((bl, bc)) = in_block {
        errs.push(diag(bl, bc, 2, true, t("This comment is never closed – add */ at its end.")));
    }
    (lines, errs)
}

pub fn brackets(text: &str, sx: &Syntax) -> Vec<Diag> {
    let (lines, mut out) = strip(text, sx);
    let mut stack: Vec<(char, usize, usize)> = vec![];
    for (li, l) in lines.iter().enumerate() {
        for (ci, &c) in l.iter().enumerate() {
            match c {
                '(' | '[' | '{' => stack.push((c, li, ci)),
                ')' | ']' | '}' => {
                    let want = match c {
                        ')' => '(',
                        ']' => '[',
                        _ => '{',
                    };
                    match stack.last() {
                        Some(&(o, _, _)) if o == want => {
                            stack.pop();
                        }
                        Some(&(o, ol, _)) => {
                            out.push(diag(
                                li,
                                ci,
                                1,
                                true,
                                tf("This {c} doesn't match the {o} on line {n} – one of them is wrong or a bracket is missing in between.", &[("c", &c.to_string()), ("o", &o.to_string()), ("n", &(ol + 1).to_string())]),
                            ));
                            return out;
                        }
                        None => {
                            out.push(diag(li, ci, 1, true, tf("This {c} has no opening {o} – remove it or add the opening one.", &[("c", &c.to_string()), ("o", &want.to_string())])));
                            return out;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(&(o, li, ci)) = stack.last() {
        let close = match o {
            '(' => ')',
            '[' => ']',
            _ => '}',
        };
        out.push(diag(li, ci, 1, true, tf("This {open} is never closed – add the matching {close}.", &[("open", &o.to_string()), ("close", &close.to_string())])));
    }
    out
}

// HTML: len {} v <style>/<script> by bolo zložité – stačia značky
fn brackets_only_braces_in(_text: &str) -> Vec<Diag> {
    vec![]
}

// ---------- chýbajúca „;“ (C, C++, Java, C#) ----------

fn semicolons(text: &str, sx: &Syntax) -> Vec<Diag> {
    let (lines, _) = strip(text, sx);
    let strs: Vec<String> = lines.iter().map(|l| l.iter().collect::<String>()).collect();
    let mut out = vec![];
    // druhy otvorených { : true = zoznam (enum, inicializácia „= {“), tam „;“ nepatrí
    let mut kinds: Vec<bool> = vec![];
    let mut paren = 0i32;
    for (i, s) in strs.iter().enumerate() {
        let code = s.trim();
        let start_paren = paren;
        for c in code.chars() {
            match c {
                '(' => paren += 1,
                ')' => paren -= 1,
                '{' => {
                    let list = code.contains("enum") || code.trim_end_matches('{').trim_end().ends_with('=') || code.contains("= {") || kinds.last() == Some(&true);
                    kinds.push(list);
                }
                '}' => {
                    kinds.pop();
                }
                _ => {}
            }
        }
        paren = paren.max(0);
        if code.is_empty() || start_paren > 0 || paren > 0 || kinds.last() == Some(&true) {
            continue;
        }
        let head = code.trim_start_matches('}').trim_start();
        let word = head.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
        if code.starts_with('#') || code.starts_with('@') || code.starts_with('[') || head.is_empty() {
            continue;
        }
        if matches!(
            word,
            "if" | "for" | "while" | "switch" | "else" | "do" | "try" | "catch" | "finally" | "case" | "default" | "namespace" | "class" | "struct" | "enum" | "union" | "template" | "public" | "private" | "protected" | "interface" | "extern" | "typedef" | "foreach" | "using_" | "get" | "set" | "lock" | "unsafe" | "fixed" | "checked"
        ) && !code.ends_with(';')
        {
            // using namespace … bez „;“ je chyba, ostatné riadky s týmito slovami nie
            continue;
        }
        let last = code.chars().last().unwrap_or(' ');
        let ends_value = last == ')' || last == ']' || last.is_alphanumeric() || last == '_' || code.ends_with("++") || code.ends_with("--");
        // strip() nahradil reťazce medzerami: riadok končiaci reťazcom končí na „=“ alebo „(“ – nevadí
        if !ends_value {
            continue;
        }
        // ďalší neprázdny riadok
        let Some(next) = strs[i + 1..].iter().map(|s| s.trim()).find(|s| !s.is_empty()) else { continue };
        let nf = next.chars().next().unwrap_or(' ');
        if !(nf.is_alphanumeric() || nf == '_' || nf == '}' || nf == '*' && next.starts_with("*p")) {
            continue;
        }
        if next.starts_with("else") || next.starts_with("catch") || next.starts_with("finally") || next.starts_with("while") && code.starts_with('}') {
            continue;
        }
        // jednoriadkové telo „if (x)\n  y();“ – riadok je hlavička, nie príkaz
        if word.is_empty() {
            continue;
        }
        // definícia funkcie/metódy bez „;“ pred „{“ na ďalšom riadku už vylúčilo nf == '{'
        let col = s.trim_end().chars().count();
        out.push(diag(i, col.saturating_sub(1), 1, false, t("Maybe a ; is missing at the end of this line.")));
    }
    out
}

// ---------- CSS ----------

fn css_semicolons(text: &str) -> Vec<Diag> {
    let (lines, _) = strip(text, &Syntax::of("css"));
    let strs: Vec<String> = lines.iter().map(|l| l.iter().collect::<String>()).collect();
    let mut out = vec![];
    let mut depth = 0i32;
    for (i, s) in strs.iter().enumerate() {
        let code = s.trim();
        let inside = depth > 0;
        depth += code.matches('{').count() as i32 - code.matches('}').count() as i32;
        if !inside || code.is_empty() || code.ends_with(';') || code.ends_with('{') || code.ends_with('}') || code.ends_with(',') || !code.contains(':') {
            continue;
        }
        let Some(next) = strs[i + 1..].iter().map(|s| s.trim()).find(|s| !s.is_empty()) else { continue };
        if next.contains(':') && !next.starts_with('}') && !next.ends_with('{') {
            let col = s.trim_end().chars().count();
            out.push(diag(i, col.saturating_sub(1), 1, true, t("A ; is missing at the end of this line – every CSS rule ends with a semicolon.")));
        }
    }
    out
}

// ---------- HTML ----------

const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr", "!doctype"];
const OPTIONAL: &[&str] = &["p", "li", "td", "tr", "th", "option", "dt", "dd", "thead", "tbody", "tfoot", "html", "head", "body", "colgroup", "optgroup", "rt", "rp"];

fn html(text: &str) -> Vec<Diag> {
    let mut out = vec![];
    let bytes: Vec<char> = text.chars().collect();
    let (mut line, mut col) = (0usize, 0usize);
    let mut stack: Vec<(String, usize, usize)> = vec![];
    let mut i = 0;
    let advance = |c: char, line: &mut usize, col: &mut usize| {
        if c == '\n' {
            *line += 1;
            *col = 0;
        } else {
            *col += 1;
        }
    };
    while i < bytes.len() {
        if bytes[i] == '<' {
            let rest: String = bytes[i..bytes.len().min(i + 4)].iter().collect();
            if rest.starts_with("<!--") {
                let (sl, sc) = (line, col);
                let mut j = i + 4;
                let mut closed = false;
                while j < bytes.len() {
                    if bytes[j] == '-' && bytes.get(j + 1) == Some(&'-') && bytes.get(j + 2) == Some(&'>') {
                        closed = true;
                        break;
                    }
                    j += 1;
                }
                if !closed {
                    out.push(diag(sl, sc, 4, true, t("This comment is never closed – add --> at its end.")));
                    return out;
                }
                for &c in &bytes[i..j + 3] {
                    advance(c, &mut line, &mut col);
                }
                i = j + 3;
                continue;
            }
            let closing = bytes.get(i + 1) == Some(&'/');
            let s = i + if closing { 2 } else { 1 };
            let mut e = s;
            while e < bytes.len() && (bytes[e].is_alphanumeric() || bytes[e] == '-' || bytes[e] == '!') {
                e += 1;
            }
            let name: String = bytes[s..e].iter().collect::<String>().to_lowercase();
            // koniec značky (s ohľadom na úvodzovky v atribútoch)
            let mut j = e;
            let mut q: Option<char> = None;
            while j < bytes.len() {
                match (q, bytes[j]) {
                    (None, '"') | (None, '\'') => q = Some(bytes[j]),
                    (Some(a), b) if a == b => q = None,
                    (None, '>') => break,
                    _ => {}
                }
                j += 1;
            }
            let self_close = j > 0 && bytes.get(j - 1) == Some(&'/');
            let (tl, tc) = (line, col);
            for &c in &bytes[i..(j + 1).min(bytes.len())] {
                advance(c, &mut line, &mut col);
            }
            if name.is_empty() {
                i = j + 1;
                continue;
            }
            if j >= bytes.len() {
                out.push(diag(tl, tc, name.chars().count() + 1, true, tf("The tag <{name} is never closed with >.", &[("name", &name)])));
                return out;
            }
            if closing {
                if let Some(pos) = stack.iter().rposition(|(n, _, _)| *n == name) {
                    // vynechané značky medzi tým (okrem tých, ktoré sa smú vynechať) = neuzavreté
                    for (n, l, c) in stack.drain(pos..).skip(1).collect::<Vec<_>>() {
                        if !OPTIONAL.contains(&n.as_str()) {
                            out.push(diag(l, c, n.chars().count() + 1, true, tf("<{name}> is never closed – add </{name}>.", &[("name", &n)])));
                        }
                    }
                } else if !OPTIONAL.contains(&name.as_str()) {
                    out.push(diag(tl, tc, name.chars().count() + 3, true, tf("</{name}> has no opening <{name}>.", &[("name", &name)])));
                }
            } else if !VOID.contains(&name.as_str()) && !self_close {
                stack.push((name.clone(), tl, tc));
                // obsah <script> a <style> nie je HTML
                if name == "script" || name == "style" {
                    let end = format!("</{name}");
                    let mut k = j + 1;
                    while k < bytes.len() {
                        if bytes[k] == '<' && bytes[k..bytes.len().min(k + end.len())].iter().collect::<String>().to_lowercase() == end {
                            break;
                        }
                        k += 1;
                    }
                    for &c in &bytes[(j + 1).min(bytes.len())..k] {
                        advance(c, &mut line, &mut col);
                    }
                    i = k;
                    continue;
                }
            }
            i = j + 1;
            continue;
        }
        advance(bytes[i], &mut line, &mut col);
        i += 1;
    }
    for (n, l, c) in stack {
        if !OPTIONAL.contains(&n.as_str()) {
            out.push(diag(l, c, n.chars().count() + 1, true, tf("<{name}> is never closed – add </{name}>.", &[("name", &n)])));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[Diag]) -> Vec<usize> {
        v.iter().map(|d| d.line).collect()
    }

    #[test]
    fn lint_brackets() {
        let sx = Syntax::of("js");
        assert!(brackets("function f(a) {\n  return [a, \"(\"];\n}\n", &sx).is_empty());
        assert_eq!(lines(&brackets("if (x {\n  y();\n}\n", &sx)), vec![0]);
        assert_eq!(lines(&brackets("a = [1, 2);\n", &sx)), vec![0]);
        assert_eq!(lines(&brackets("x = 1;\n}\n", &sx)), vec![1]);
        assert_eq!(lines(&brackets("let s = \"abc;\nlet t = 1;\n", &sx)), vec![0]);
        assert!(brackets("const t = `a\nb ${x}`;\n", &sx).is_empty());
        assert!(brackets("// ( neuzavretá v komentári\n/* { */ x();\n", &sx).is_empty());
    }

    #[test]
    fn lint_semicolons_no_false_positives() {
        let sx = Syntax::of("cpp");
        let ok = "#include <stdio.h>\nint main()\n{\n    int x = 5;\n    if (x)\n        printf(\"%d\", x);\n    for (int i = 0; i < 3; i++)\n    {\n        x++;\n    }\n    std::cout << x\n              << std::endl;\n    enum C { A, B };\n    int a[] = {\n        1,\n        2\n    };\n    return 0;\n}\n";
        assert!(semicolons(ok, &sx).is_empty(), "{:?}", semicolons(ok, &sx));
        let bad = "int main() {\n    int x = 5\n    return x;\n}\n";
        assert_eq!(lines(&semicolons(bad, &sx)), vec![1]);
        let bad2 = "void f() {\n    g()\n}\n";
        assert_eq!(lines(&semicolons(bad2, &sx)), vec![1]);
        let java = "public class A {\n    public static void main(String[] args) {\n        System.out.println(\"hi\");\n    }\n}\n";
        assert!(semicolons(java, &Syntax::of("java")).is_empty());
    }

    #[test]
    fn lint_python_fallback_and_explain() {
        assert_eq!(lines(&py_colons("if x == 1\n    print(x)\n")), vec![0]);
        assert!(py_colons("if (a and\n    b):\n    pass\n").is_empty());
        assert!(explain_py("expected ':'").contains(':'));
        assert!(explain_py("'(' was never closed").contains(')'));
    }

    #[test]
    fn lint_json_css_html() {
        assert_eq!(lines(&json("{\n  \"a\": 1,\n}\n")), vec![2]);
        assert!(json("{\"a\": [1, 2]}").is_empty());
        assert_eq!(lines(&css_semicolons("a {\n  color: red\n  margin: 0;\n}\n")), vec![1]);
        assert!(css_semicolons("a {\n  color: red;\n  margin: 0\n}\n").is_empty());
        assert!(html("<!doctype html>\n<html><body><p>a<br>b</p><img src=\"x\"></body></html>\n").is_empty());
        assert_eq!(lines(&html("<div>\n  <span>x\n</div>\n")), vec![1]);
        assert_eq!(lines(&html("<div></div>\n</section>\n")), vec![1]);
        assert!(html("<script>if (a < b) { x(); }</script>").is_empty());
    }

    #[test]
    fn lint_python_real_compiler() {
        let Some(py) = ["python3", "python"].into_iter().find(|p| std::process::Command::new(p).arg("-V").output().is_ok()) else { return };
        let d = python_compile(py, "x = 1\nif x == 1\n    print(x)\n").unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].line, 1);
        assert!(d[0].msg.contains("colon"), "{}", d[0].msg);
        assert!(python_compile(py, "print('ok')\n").unwrap().is_empty());
        let d = python_compile(py, "print('a'\nx = 2\n").unwrap();
        assert_eq!(d[0].line, 0, "{:?}", d);
    }
}
