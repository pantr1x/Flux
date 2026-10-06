// Návrhy pri písaní: kľúčové slová a vstavané funkcie jazyka + slová z otvoreného súboru.
// Poradie: rovnaký začiatok (aj veľkosť písmen) > začiatok bez ohľadu na veľkosť > obsahuje; potom podľa početnosti.
use std::collections::HashMap;

mod data;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Keyword,
    Builtin,
    Word,
    Snippet,
    Tag,
    Property,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub label: String,
    pub kind: Kind,
    // čo sa vloží namiesto napísaného slova ($1, ${1:text}, $0 = kurzor); None = label
    pub insert: Option<String>,
    // náhľad vpravo v okne (napr. „<title></title>“)
    pub detail: Option<String>,
    // krátky popis vybranej položky (ako dokumentácia vo VS Code)
    pub doc: Option<&'static str>,
}

impl Item {
    fn plain(label: String, kind: Kind) -> Self {
        Item { label, kind, insert: None, detail: None, doc: None }
    }
    fn doc(mut self, d: &'static str) -> Self {
        self.doc = Some(d);
        self
    }
    fn snip(label: &str, kind: Kind, body: &str, detail: &str) -> Self {
        Item { label: label.into(), kind, insert: Some(body.into()), detail: (!detail.is_empty()).then(|| detail.into()), doc: None }
    }
}

pub fn ident_char(c: char, lang: &str) -> bool {
    c.is_alphanumeric() || c == '_' || (c == '-' && matches!(lang, "css" | "scss" | "less" | "html" | "htm")) || (c == '$' && matches!(lang, "js" | "ts" | "php"))
}

fn keywords(lang: &str) -> &'static [&'static str] {
    match lang {
        "py" | "pyw" => &[
            "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
            "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield", "match", "case", "self",
        ],
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => &[
            "async", "await", "break", "case", "catch", "class", "const", "continue", "default", "delete", "do", "else", "export", "extends", "false", "finally", "for", "function", "if", "import",
            "in", "instanceof", "let", "new", "null", "return", "static", "super", "switch", "this", "throw", "true", "try", "typeof", "undefined", "var", "void", "while", "yield", "interface", "type",
        ],
        "c" | "h" => &[
            "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "float", "for", "goto", "if", "int", "long", "register", "return", "short", "signed",
            "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned", "void", "volatile", "while", "include", "define", "bool", "true", "false",
        ],
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => &[
            "auto", "bool", "break", "case", "catch", "char", "class", "const", "constexpr", "continue", "default", "delete", "do", "double", "else", "enum", "explicit", "false", "float", "for",
            "friend", "if", "include", "inline", "int", "long", "namespace", "new", "nullptr", "operator", "private", "protected", "public", "return", "short", "signed", "sizeof", "static", "struct",
            "switch", "template", "this", "throw", "true", "try", "typedef", "typename", "unsigned", "using", "virtual", "void", "while", "override",
        ],
        "java" => &[
            "abstract", "boolean", "break", "byte", "case", "catch", "char", "class", "continue", "default", "do", "double", "else", "enum", "extends", "final", "finally", "float", "for", "if",
            "implements", "import", "instanceof", "int", "interface", "long", "new", "null", "package", "private", "protected", "public", "return", "short", "static", "super", "switch", "this", "throw",
            "throws", "true", "false", "try", "void", "while", "var",
        ],
        "cs" => &[
            "abstract", "async", "await", "bool", "break", "case", "catch", "char", "class", "const", "continue", "decimal", "default", "do", "double", "else", "enum", "false", "finally", "float", "for",
            "foreach", "if", "in", "int", "interface", "internal", "long", "namespace", "new", "null", "object", "override", "private", "protected", "public", "readonly", "return", "static", "string",
            "struct", "switch", "this", "throw", "true", "try", "using", "var", "virtual", "void", "while",
        ],
        "rs" => &[
            "as", "async", "await", "break", "const", "continue", "crate", "else", "enum", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
            "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
        ],
        "go" => &[
            "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map", "package", "range", "return", "select",
            "struct", "switch", "type", "var", "true", "false", "nil",
        ],
        _ => &[],
    }
}

fn builtins(lang: &str) -> &'static [&'static str] {
    match lang {
        "py" | "pyw" => &[
            "print", "input", "len", "range", "int", "str", "float", "bool", "list", "dict", "set", "tuple", "open", "enumerate", "zip", "sorted", "reversed", "sum", "min", "max", "abs", "round",
            "type", "isinstance", "map", "filter", "any", "all", "format", "super", "append", "extend", "insert", "remove", "split", "join", "strip", "lower", "upper", "replace", "startswith",
            "endswith", "keys", "values", "items", "random", "randint", "choice", "math", "sqrt", "time", "sleep", "datetime", "os", "sys", "json", "tkinter", "pygame", "__name__", "__main__",
            "Exception", "ValueError", "KeyError", "IndexError",
        ],
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => &[
            "console", "log", "error", "document", "window", "getElementById", "querySelector", "querySelectorAll", "addEventListener", "createElement", "appendChild", "innerHTML", "textContent",
            "classList", "style", "Math", "random", "floor", "JSON", "parse", "stringify", "Array", "Object", "String", "Number", "Promise", "fetch", "then", "setTimeout", "setInterval", "length",
            "push", "pop", "map", "filter", "forEach", "reduce", "includes", "indexOf", "slice", "splice", "join", "split", "keys", "values", "entries", "localStorage", "require", "module",
        ],
        "c" | "h" => &["printf", "scanf", "puts", "gets", "fgets", "malloc", "calloc", "free", "strlen", "strcpy", "strcmp", "strcat", "memset", "memcpy", "main", "NULL", "stdio", "stdlib", "string", "FILE", "fopen", "fclose"],
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => &[
            "std", "cout", "cin", "endl", "string", "vector", "map", "unordered_map", "set", "pair", "push_back", "size", "begin", "end", "sort", "iostream", "main", "printf", "getline", "auto",
            "make_shared", "unique_ptr", "shared_ptr",
        ],
        "java" => &["System", "out", "println", "print", "String", "Integer", "Double", "Scanner", "nextLine", "nextInt", "ArrayList", "HashMap", "List", "Map", "length", "size", "add", "get", "main", "Math"],
        "cs" => &["Console", "WriteLine", "ReadLine", "Write", "List", "Dictionary", "Math", "Parse", "ToString", "Length", "Count", "Add", "System", "Main", "Linq", "string", "int"],
        "rs" => &["println!", "print!", "format!", "vec!", "String", "Vec", "Option", "Some", "None", "Result", "Ok", "Err", "Box", "HashMap", "unwrap", "expect", "clone", "iter", "collect", "len", "push", "main"],
        "go" => &["fmt", "Println", "Printf", "Sprintf", "len", "append", "make", "error", "string", "int", "main", "Scan"],
        "html" | "htm" => &[
            "html", "head", "body", "title", "meta", "link", "script", "style", "div", "span", "p", "a", "img", "ul", "ol", "li", "h1", "h2", "h3", "button", "input", "form", "label", "table", "tr",
            "td", "header", "footer", "main", "section", "nav", "class", "href", "src", "alt", "type", "value", "placeholder",
        ],
        "css" | "scss" | "less" => &[
            "color", "background", "background-color", "margin", "padding", "border", "border-radius", "display", "flex", "grid", "position", "absolute", "relative", "width", "height", "font-size",
            "font-family", "font-weight", "text-align", "justify-content", "align-items", "gap", "box-shadow", "transition", "transform", "opacity", "cursor", "overflow", "z-index", "none", "auto",
        ],
        _ => &[],
    }
}

// slová zo súboru (aspoň 3 znaky, nezačínajú číslom) s početnosťou
pub fn words(text: &str, lang: &str) -> HashMap<String, usize> {
    let mut m: HashMap<String, usize> = HashMap::new();
    let mut cur = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if ident_char(c, lang) {
            cur.push(c);
        } else if !cur.is_empty() {
            if cur.chars().count() >= 3 && !cur.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                *m.entry(std::mem::take(&mut cur)).or_insert(0) += 1;
            } else {
                cur.clear();
            }
        }
    }
    m
}

pub fn suggest(lang: &str, prefix: &str, words: &HashMap<String, usize>, limit: usize) -> Vec<Item> {
    if prefix.is_empty() {
        return vec![];
    }
    let lp = prefix.to_lowercase();
    let mut all: HashMap<String, (Kind, usize)> = HashMap::new();
    for (w, n) in words {
        // slovo, ktoré práve píšem (raz v texte), nie je návrh
        if w == prefix && *n <= 1 {
            continue;
        }
        all.insert(w.clone(), (Kind::Word, *n));
    }
    for k in keywords(lang) {
        all.insert(k.to_string(), (Kind::Keyword, 1000));
    }
    for b in builtins(lang) {
        all.entry(b.to_string()).and_modify(|e| e.0 = Kind::Builtin).or_insert((Kind::Builtin, 500));
    }
    let mut v: Vec<(u8, usize, String, Kind)> = all
        .into_iter()
        // úplne napísaný kľúčový výraz/vstavaná funkcia (str, print) sa ukáže ďalej; vlastné slovo, ktoré práve píšem, nie
        .filter(|(w, (k, _))| w != prefix || !matches!(k, Kind::Word))
        .filter_map(|(w, (k, n))| {
            let lw = w.to_lowercase();
            let rank = if w.starts_with(prefix) {
                0
            } else if lw.starts_with(&lp) {
                1
            } else if lp.chars().count() >= 3 && lw.contains(&lp) {
                2
            } else {
                return None;
            };
            Some((rank, n, w, k))
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(a.2.len().cmp(&b.2.len())).then(a.2.cmp(&b.2)));
    v.into_iter().take(limit).map(|(_, _, label, kind)| Item::plain(label, kind)).collect()
}

// ---- návrhy podľa miesta v kóde (ako VS Code): značky HTML, vlastnosti CSS, členy za bodkou, moduly ----

// kde sa píše (podľa textu pred kurzorom)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ctx {
    pub lang: Option<&'static str>,          // v HTML vnútri <style> = "css", <script> = "js"
    pub after_lt: bool,                      // hneď za „<“ → značky
    pub close: bool,                         // za „</“ → neuzavreté značky
    pub open_tags: Vec<String>,              // neuzavreté značky pred kurzorom (najvnútornejšia posledná)
    pub in_tag: Option<String>,              // vnútri značky (meno) → atribúty
    pub attr_value: Option<(String, String)>, // v úvodzovkách atribútu (značka, atribút) → hodnoty
    pub css_value: Option<String>,           // za „vlastnosť:“ → hodnoty
    pub css_block: bool,                     // vnútri { } → vlastnosti, inak selektor
    pub css_pseudo: bool,                    // „a:ho“ v selektore → pseudo-triedy
    pub css_at: bool,                        // „@me“ → @media, @keyframes…
    pub member: Option<String>,              // „objekt.“ / „std::“ → členy objektu
    pub import: Option<String>,              // „import “ = Some(""), „from os import “ = Some("os")
    pub emmet: Option<String>,               // HTML text: skratka Emmet pred kurzorom („div.box“, „ul>li*3“, „fo“)
    pub doc_tags: Vec<String>,               // vlastné značky použité v súbore (<my-card>) pre „<“
}

impl Ctx {
    // miesto s vlastnými návrhmi: stačí jedno písmeno (inak dve)
    pub fn rich(&self) -> bool {
        self.after_lt || self.close || self.emmet.as_ref().is_some_and(|e| e.contains(['.', '#', '>', '*', '+', '{'])) || self.in_tag.is_some() || self.attr_value.is_some() || self.css_value.is_some() || self.css_pseudo || self.css_at || self.member.is_some() || self.import.is_some()
    }
}

fn family(lang: &str) -> &'static str {
    match lang {
        "html" | "htm" | "xml" | "vue" | "svelte" | "php" => "html",
        "css" | "scss" | "less" => "css",
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => "js",
        "py" | "pyw" => "py",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "c" | "h" => "c",
        "java" => "java",
        "cs" => "cs",
        "go" => "go",
        "rs" => "rs",
        _ => "",
    }
}

fn lower(cs: &[char]) -> String {
    cs.iter().map(|c| c.to_ascii_lowercase()).collect()
}

// neuzavreté značky v texte (bez prázdnych, komentárov a obsahu <script>/<style>)
pub fn open_tags(before: &[char]) -> Vec<String> {
    let mut st: Vec<String> = vec![];
    let n = before.len();
    let mut i = 0;
    while i < n {
        if before[i] != '<' {
            i += 1;
            continue;
        }
        if before[i + 1..].starts_with(&['!', '-', '-']) {
            // komentár
            let rest: String = before[i..].iter().collect();
            i += rest.find("-->").map(|p| rest[..p].chars().count() + 3).unwrap_or(n);
            continue;
        }
        let closing = before.get(i + 1) == Some(&'/');
        let s = i + 1 + usize::from(closing);
        let mut e = s;
        while e < n && (before[e].is_alphanumeric() || before[e] == '-') {
            e += 1;
        }
        if e == s {
            i += 1;
            continue;
        }
        let name = lower(&before[s..e]);
        // koniec značky
        let mut j = e;
        let mut q: Option<char> = None;
        while j < n {
            let c = before[j];
            match q {
                Some(x) if c == x => q = None,
                Some(_) => {}
                None if c == '"' || c == '\'' => q = Some(c),
                None if c == '>' => break,
                _ => {}
            }
            j += 1;
        }
        if j >= n {
            break; // značka sa ešte píše
        }
        let self_close = before[j - 1] == '/';
        if closing {
            if let Some(k) = st.iter().rposition(|t| *t == name) {
                st.truncate(k);
            }
        } else if !self_close && !VOID.contains(&name.as_str()) {
            if name == "script" || name == "style" {
                // obsah preskočiť až po </script>
                let rest: String = lower(&before[j..]);
                match rest.find(&format!("</{name}")) {
                    Some(p) => {
                        i = j + rest[..p].chars().count();
                        continue;
                    }
                    None => {
                        st.push(name);
                        break;
                    }
                }
            }
            st.push(name);
        }
        i = j + 1;
    }
    st
}

pub fn context(lang: &str, before: &[char]) -> Ctx {
    let fam = family(lang);
    let plen = before.iter().rev().take_while(|ch| ident_char(**ch, lang)).count();
    let head = &before[..before.len() - plen];
    let mut c = Ctx::default();
    match fam {
        "html" => {
            // vnútri <style> / <script> sa navrhuje CSS / JavaScript
            let low = lower(before);
            for (tag, l) in [("style", "css"), ("script", "js")] {
                if let Some(o) = low.rfind(&format!("<{tag}")) {
                    if low.rfind(&format!("</{tag}")).is_none_or(|cl| cl < o) {
                        if let Some(gt) = low[o..].find('>') {
                            let start = low[..o + gt + 1].chars().count();
                            if start <= head.len() && !low[o..o + gt].contains("src=") {
                                let mut inner = context(l, &before[start..]);
                                inner.lang = Some(l);
                                return inner;
                            }
                        }
                    }
                }
            }
            if head.ends_with(&['<', '/']) {
                c.close = true;
                c.open_tags = open_tags(&before[..head.len() - 2]);
                return c;
            }
            c.after_lt = head.last() == Some(&'<');
            if c.after_lt {
                c.open_tags = open_tags(&head[..head.len() - 1]);
                c.doc_tags = doc_tags(&head[..head.len() - 1]);
                return c;
            }
            let lt = head.iter().rposition(|ch| *ch == '<');
            let gt = head.iter().rposition(|ch| *ch == '>');
            let Some(l) = lt.filter(|l| gt.is_none_or(|g| g < *l)) else {
                c.emmet = emmet_abbr(before);
                return c;
            };
            if matches!(head.get(l + 1), Some('/') | Some('!') | Some('?')) {
                return c;
            }
            let mut e = l + 1;
            while e < head.len() && (head[e].is_alphanumeric() || head[e] == '-') {
                e += 1;
            }
            if e == l + 1 {
                return c;
            }
            let tag = lower(&head[l + 1..e]);
            // v úvodzovkách = hodnota atribútu
            let mut q: Option<(char, usize)> = None;
            for (k, ch) in head.iter().enumerate().skip(e) {
                match q {
                    Some((x, _)) if *ch == x => q = None,
                    None if *ch == '"' || *ch == '\'' => q = Some((*ch, k)),
                    _ => {}
                }
            }
            if let Some((_, k)) = q {
                if k >= 1 && head[k - 1] == '=' {
                    let mut s = k - 1;
                    while s > 0 && (head[s - 1].is_alphanumeric() || head[s - 1] == '-' || head[s - 1] == ':') {
                        s -= 1;
                    }
                    c.attr_value = Some((tag, lower(&head[s..k - 1])));
                }
                return c;
            }
            if head.last().is_some_and(|ch| ch.is_whitespace()) {
                c.in_tag = Some(tag);
            }
        }
        "css" => {
            let depth = head.iter().fold(0i32, |d, ch| match ch {
                '{' => d + 1,
                '}' => (d - 1).max(0),
                _ => d,
            });
            c.css_block = depth > 0;
            if head.last() == Some(&'@') {
                c.css_at = true;
                return c;
            }
            let semi = head.iter().rposition(|ch| matches!(*ch, ';' | '{' | '}'));
            if c.css_block {
                if let Some(col) = head.iter().rposition(|ch| *ch == ':').filter(|col| semi.is_none_or(|s| s < *col)) {
                    let start = semi.map(|s| s + 1).unwrap_or(0);
                    let prop = head[start..col].iter().collect::<String>().trim().to_string();
                    // „a:hover {“ vo vnorenom SCSS nie je vlastnosť
                    if !prop.is_empty() && prop.chars().all(|ch| ch.is_alphanumeric() || ch == '-') {
                        c.css_value = Some(prop);
                    }
                }
            } else if head.last() == Some(&':') {
                c.css_pseudo = true;
            }
        }
        "" => {}
        _ => {
            let line_start = head.iter().rposition(|ch| *ch == '\n').map(|i| i + 1).unwrap_or(0);
            let line: String = head[line_start..].iter().collect();
            let lt = line.trim_start();
            if fam == "py" {
                if lt == "import " || lt == "from " || (lt.starts_with("import ") && lt.ends_with(", ")) {
                    c.import = Some(String::new());
                    return c;
                }
                if let Some(rest) = lt.strip_prefix("from ") {
                    if let Some((m, after)) = rest.split_once(" import ") {
                        if after.is_empty() || after.trim_end().ends_with(',') {
                            c.import = Some(m.trim().to_string());
                            return c;
                        }
                    }
                }
            }
            if fam == "js" && (lt.ends_with("from '") || lt.ends_with("from \"") || lt.ends_with("require('") || lt.ends_with("require(\"") || lt.ends_with("import '") || lt.ends_with("import \"")) {
                c.import = Some(String::new());
                return c;
            }
            // člen: „objekt.“ alebo „std::“
            let sep = if head.last() == Some(&'.') {
                1
            } else if matches!(fam, "cpp" | "rs") && head.ends_with(&[':', ':']) {
                2
            } else {
                0
            };
            if sep > 0 {
                let end = head.len() - sep;
                let mut s = end;
                while s > 0 {
                    let ch = head[s - 1];
                    if ident_char(ch, lang) || ch == '.' || (ch == ':' && matches!(fam, "cpp" | "rs")) {
                        s -= 1;
                    } else {
                        break;
                    }
                }
                let chain: String = head[s..end].iter().collect::<String>().replace("::", ".");
                let chain = chain.trim_matches('.').to_string();
                // „3.“ je číslo, nie objekt
                if !chain.is_empty() && !chain.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
                    c.member = Some(chain);
                } else if head[..end].last().is_some_and(|ch| matches!(ch, ')' | ']' | '"' | '\'' | '`')) {
                    c.member = Some("*".into());
                }
            }
        }
    }
    c
}

// vlastné značky v texte (nie štandardné HTML), napr. <my-card>
fn doc_tags(before: &[char]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for (i, ch) in before.iter().enumerate() {
        if *ch == '<' && before.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic()) {
            let name: String = before[i + 1..].iter().take_while(|c| c.is_ascii_alphanumeric() || **c == '-').map(|c| c.to_ascii_lowercase()).collect();
            if !db().tags.iter().any(|t| t.0 == name) && !out.contains(&name) && name.len() < 40 {
                out.push(name);
            }
        }
    }
    out
}

// skratka Emmet pred kurzorom v texte HTML (nie v značke): „div.box“, „ul>li*3“, „p{Ahoj}“, alebo len slovo
fn emmet_abbr(before: &[char]) -> Option<String> {
    let ok = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '#' | '>' | '*' | '+' | '{' | '}' | '$' | '_');
    let mut s = before.len();
    while s > 0 && ok(before[s - 1]) {
        s -= 1;
    }
    // „<p>div.x“: časť po značke
    if s > 0 && before[s - 1] == '<' {
        let gt = before[s..].iter().position(|c| *c == '>')?;
        s += gt + 1;
    }
    if s > 0 && !before[s - 1].is_whitespace() && before[s - 1] != '>' {
        return None;
    }
    let abbr: String = before[s..].iter().collect();
    let first = abbr.chars().next()?;
    if !(first.is_ascii_alphabetic() || first == '.' || first == '#') {
        return None;
    }
    if abbr.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Some(abbr);
    }
    emmet(&abbr).map(|_| abbr)
}

#[derive(Default)]
struct ENode {
    tag: String,
    id: String,
    classes: Vec<String>,
    text: Option<String>,
    count: usize,
    kids: Vec<ENode>,
}

// Emmet: element (+ element)* ; element > deti ; tag.trieda#id{text}*N
fn e_list(cs: &[char], i: &mut usize) -> Option<Vec<ENode>> {
    let mut v = vec![e_node(cs, i)?];
    while *i < cs.len() && cs[*i] == '+' {
        *i += 1;
        v.push(e_node(cs, i)?);
    }
    Some(v)
}

fn e_node(cs: &[char], i: &mut usize) -> Option<ENode> {
    let name = |i: &mut usize| {
        let s = *i;
        while *i < cs.len() && (cs[*i].is_ascii_alphanumeric() || cs[*i] == '-' || cs[*i] == '_' || cs[*i] == '$') {
            *i += 1;
        }
        cs[s..*i].iter().collect::<String>()
    };
    let mut n = ENode { count: 1, ..Default::default() };
    n.tag = name(i);
    if !n.tag.is_empty() && !n.tag.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    loop {
        match cs.get(*i) {
            Some('.') => {
                *i += 1;
                let c = name(i);
                if c.is_empty() {
                    return None;
                }
                n.classes.push(c);
            }
            Some('#') => {
                *i += 1;
                n.id = name(i);
                if n.id.is_empty() {
                    return None;
                }
            }
            Some('{') => {
                let e = cs[*i..].iter().position(|c| *c == '}')? + *i;
                n.text = Some(cs[*i + 1..e].iter().collect());
                *i = e + 1;
            }
            Some('*') => {
                *i += 1;
                let d = name(i);
                n.count = d.parse::<usize>().ok().filter(|k| (1..=50).contains(k))?;
            }
            _ => break,
        }
    }
    if n.tag.is_empty() {
        if n.classes.is_empty() && n.id.is_empty() && n.text.is_none() {
            return None;
        }
        n.tag = "div".into();
    }
    if *i < cs.len() && cs[*i] == '>' {
        *i += 1;
        n.kids = e_list(cs, i)?;
    }
    Some(n)
}

fn e_render(nodes: &[ENode], depth: usize, stop: &mut usize, out: &mut Vec<String>) {
    let ind = "\t".repeat(depth);
    for n in nodes {
        for k in 1..=n.count {
            let num = |s: &str| s.replace('$', &k.to_string());
            let mut open = format!("<{}", n.tag);
            if !n.id.is_empty() {
                open += &format!(" id=\"{}\"", num(&n.id));
            }
            if !n.classes.is_empty() {
                open += &format!(" class=\"{}\"", n.classes.iter().map(|c| num(c)).collect::<Vec<_>>().join(" "));
            }
            match n.tag.as_str() {
                "a" => open += " href=\"\"",
                "img" => open += " src=\"\" alt=\"\"",
                "input" => open += " type=\"text\"",
                _ => {}
            }
            open.push('>');
            if VOID.contains(&n.tag.as_str()) {
                out.push(format!("{ind}{open}"));
            } else if !n.kids.is_empty() {
                out.push(format!("{ind}{open}"));
                e_render(&n.kids, depth + 1, stop, out);
                out.push(format!("{ind}</{}>", n.tag));
            } else {
                let inner = match &n.text {
                    Some(t) => num(t),
                    None => {
                        *stop += 1;
                        format!("${}", stop)
                    }
                };
                out.push(format!("{ind}{open}{inner}</{}>", n.tag));
            }
        }
    }
}

// náhľad úryvku bez značiek kurzora ($1, ${1:x} → x)
fn strip_stops(body: &str) -> String {
    let (t, _) = expand(body, "", "");
    t
}

// skratka Emmet → úryvok (s $1, $2… v prázdnych značkách); None = nie je to platná skratka
pub fn emmet(abbr: &str) -> Option<String> {
    let cs: Vec<char> = abbr.chars().collect();
    let mut i = 0;
    let nodes = e_list(&cs, &mut i)?;
    if i != cs.len() {
        return None;
    }
    let mut out = vec![];
    let mut stop = 0;
    e_render(&nodes, 0, &mut stop, &mut out);
    Some(out.join("\n"))
}

// znak, ktorý sám otvorí návrhy (ako „trigger characters“ vo VS Code)
pub fn trigger(lang: &str, before: &[char]) -> bool {
    let Some(&ch) = before.last() else { return false };
    let prev = before.len().checked_sub(2).map(|i| before[i]);
    if !matches!(ch, '<' | '/' | ' ' | '"' | '\'' | ':' | '.' | '@' | '}') {
        return false;
    }
    let cx = context(lang, before);
    match ch {
        '<' => cx.after_lt,
        '/' => cx.close,
        ' ' => (cx.in_tag.is_some() && !prev.is_some_and(|p| p.is_whitespace())) || (cx.css_value.is_some() && prev == Some(':')) || cx.import.is_some(),
        '"' | '\'' => cx.attr_value.is_some() || cx.import.is_some(),
        ':' => cx.css_pseudo || cx.css_value.is_some() || cx.member.is_some(),
        '.' => cx.member.is_some(),
        '@' => cx.css_at,
        '}' => cx.emmet.is_some(),
        _ => false,
    }
}

pub const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr", "param"];
const HTML5: &str = "<!DOCTYPE html>\n<html lang=\"${1:en}\">\n<head>\n\t<meta charset=\"UTF-8\">\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\t<title>${2:Document}</title>\n</head>\n<body>\n\t$0\n</body>\n</html>";

// údaje z data.rs, rozložené raz
struct Db {
    tags: Vec<(&'static str, &'static str)>,
    tag_attrs: HashMap<&'static str, Vec<&'static str>>,
    values: HashMap<&'static str, Vec<&'static str>>,
    css: Vec<(&'static str, Vec<&'static str>)>,
    members: HashMap<(&'static str, &'static str), Vec<&'static str>>,
}

fn split_vals(v: &'static str) -> Vec<&'static str> {
    if v.contains('|') {
        v.split('|').map(str::trim).filter(|x| !x.is_empty()).collect()
    } else {
        v.split_whitespace().collect()
    }
}

fn db() -> &'static Db {
    static DB: std::sync::OnceLock<Db> = std::sync::OnceLock::new();
    DB.get_or_init(|| {
        let pairs = |s: &'static str| s.lines().filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim(), v)).collect::<Vec<_>>();
        Db {
            tags: data::TAGS.lines().filter_map(|l| l.split_once('|')).collect(),
            tag_attrs: pairs(data::TAG_ATTRS).into_iter().map(|(k, v)| (k, v.split_whitespace().collect())).collect(),
            values: pairs(data::ATTR_VALUES).into_iter().map(|(k, v)| (k, split_vals(v))).collect(),
            css: pairs(data::CSS).into_iter().map(|(k, v)| (k, split_vals(v))).collect(),
            members: data::MEMBERS
                .lines()
                .filter_map(|l| {
                    let (k, v) = l.split_once(": ")?;
                    let (lang, obj) = k.split_once(' ')?;
                    Some(((lang, obj), v.split(';').map(str::trim).filter(|x| !x.is_empty()).collect()))
                })
                .collect(),
        }
    })
}

fn tag_body(t: &str) -> String {
    match t {
        "a" => "a href=\"$1\">$0</a>".into(),
        "img" => "img src=\"$1\" alt=\"$2\">".into(),
        "link" => "link rel=\"stylesheet\" href=\"$1\">".into(),
        "script" => "script src=\"$1\"></script>".into(),
        "input" => "input type=\"${1:text}\" $0>".into(),
        "meta" => "meta $0>".into(),
        "html" => "html lang=\"${1:en}\">\n$0\n</html>".into(),
        "form" => "form action=\"$1\">$0</form>".into(),
        "label" => "label for=\"$1\">$0</label>".into(),
        "iframe" => "iframe src=\"$1\" frameborder=\"0\">$0</iframe>".into(),
        "video" => "video src=\"$1\" controls>$0</video>".into(),
        "audio" => "audio src=\"$1\" controls>$0</audio>".into(),
        "source" => "source src=\"$1\" type=\"$2\">".into(),
        "button" => "button type=\"${1:button}\">$0</button>".into(),
        "option" => "option value=\"$1\">$0</option>".into(),
        "ul" | "ol" => format!("{t}>\n\t<li>$0</li>\n</{t}>"),
        "table" => "table>\n\t<tr>\n\t\t<td>$0</td>\n\t</tr>\n</table>".into(),
        t if VOID.contains(&t) => format!("{t}>"),
        t => format!("{t}>$0</{t}>"),
    }
}

// úryvky podľa jazyka: (spúšťač, kód, náhľad)
fn snippets(lang: &str) -> &'static [(&'static str, &'static str, &'static str)] {
    match lang {
        "py" | "pyw" => &[
            ("for", "for ${1:item} in ${2:items}:\n\t$0", "for item in items:"),
            ("fori", "for ${1:i} in range(${2:10}):\n\t$0", "for i in range(10):"),
            ("if", "if ${1:condition}:\n\t$0", "if condition:"),
            ("elif", "elif ${1:condition}:\n\t$0", "elif condition:"),
            ("else", "else:\n\t$0", "else:"),
            ("while", "while ${1:condition}:\n\t$0", "while condition:"),
            ("def", "def ${1:name}(${2}):\n\t$0", "def name():"),
            ("class", "class ${1:Name}:\n\tdef __init__(self${2}):\n\t\t$0", "class Name:"),
            ("try", "try:\n\t$1\nexcept ${2:Exception} as e:\n\t$0", "try / except"),
            ("with", "with open(${1:\"file.txt\"}) as ${2:f}:\n\t$0", "with open(...) as f:"),
            ("main", "if __name__ == \"__main__\":\n\t$0", "if __name__ == \"__main__\":"),
            ("print", "print($0)", "print()"),
            ("input", "input(\"$1\")$0", "input(\"\")"),
            ("lambda", "lambda ${1:x}: $0", "lambda x:"),
        ],
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => &[
            ("log", "console.log($0);", "console.log()"),
            ("for", "for (let ${1:i} = 0; ${1:i} < ${2:n}; ${1:i}++) {\n\t$0\n}", "for (let i = 0; …)"),
            ("forof", "for (const ${1:item} of ${2:items}) {\n\t$0\n}", "for (const item of items)"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("else", "else {\n\t$0\n}", "else { }"),
            ("while", "while (${1:condition}) {\n\t$0\n}", "while (condition) { }"),
            ("function", "function ${1:name}(${2}) {\n\t$0\n}", "function name() { }"),
            ("fn", "const ${1:name} = (${2}) => {\n\t$0\n};", "const name = () => { }"),
            ("class", "class ${1:Name} {\n\tconstructor(${2}) {\n\t\t$0\n\t}\n}", "class Name { }"),
            ("try", "try {\n\t$1\n} catch (${2:err}) {\n\t$0\n}", "try / catch"),
            ("qs", "document.querySelector('$1')$0", "document.querySelector()"),
            ("ael", "addEventListener('${1:click}', (${2:e}) => {\n\t$0\n});", "addEventListener()"),
            ("fetch", "const ${1:res} = await fetch('$2');\nconst ${3:data} = await ${1:res}.json();$0", "await fetch()"),
            ("timeout", "setTimeout(() => {\n\t$0\n}, ${1:1000});", "setTimeout()"),
        ],
        "c" | "h" => &[
            ("main", "#include <stdio.h>\n\nint main(void) {\n\t$0\n\treturn 0;\n}", "int main()"),
            ("printf", "printf(\"$1\\n\"$2);$0", "printf()"),
            ("scanf", "scanf(\"%${1:d}\", &$2);$0", "scanf()"),
            ("for", "for (int ${1:i} = 0; ${1:i} < ${2:n}; ${1:i}++) {\n\t$0\n}", "for (int i = 0; …)"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("while", "while (${1:condition}) {\n\t$0\n}", "while (condition) { }"),
            ("inc", "#include <${1:stdio.h}>$0", "#include <…>"),
        ],
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => &[
            ("main", "#include <iostream>\n\nint main() {\n\t$0\n\treturn 0;\n}", "int main()"),
            ("cout", "std::cout << $1 << std::endl;$0", "std::cout << … << std::endl;"),
            ("cin", "std::cin >> $1;$0", "std::cin >> …;"),
            ("for", "for (int ${1:i} = 0; ${1:i} < ${2:n}; ${1:i}++) {\n\t$0\n}", "for (int i = 0; …)"),
            ("forr", "for (auto& ${1:x} : ${2:items}) {\n\t$0\n}", "for (auto& x : items)"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("while", "while (${1:condition}) {\n\t$0\n}", "while (condition) { }"),
            ("class", "class ${1:Name} {\npublic:\n\t$0\n};", "class Name { };"),
            ("inc", "#include <${1:iostream}>$0", "#include <…>"),
        ],
        "java" => &[
            ("main", "public static void main(String[] args) {\n\t$0\n}", "public static void main"),
            ("sout", "System.out.println($0);", "System.out.println()"),
            ("for", "for (int ${1:i} = 0; ${1:i} < ${2:n}; ${1:i}++) {\n\t$0\n}", "for (int i = 0; …)"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("class", "public class ${1:Name} {\n\t$0\n}", "public class Name { }"),
        ],
        "cs" => &[
            ("main", "static void Main(string[] args)\n{\n\t$0\n}", "static void Main"),
            ("cw", "Console.WriteLine($0);", "Console.WriteLine()"),
            ("for", "for (int ${1:i} = 0; ${1:i} < ${2:n}; ${1:i}++)\n{\n\t$0\n}", "for (int i = 0; …)"),
            ("foreach", "foreach (var ${1:item} in ${2:items})\n{\n\t$0\n}", "foreach (var item in items)"),
            ("if", "if (${1:condition})\n{\n\t$0\n}", "if (condition) { }"),
        ],
        "rs" => &[
            ("main", "fn main() {\n\t$0\n}", "fn main()"),
            ("println", "println!(\"$1\"$2);$0", "println!()"),
            ("fn", "fn ${1:name}(${2}) {\n\t$0\n}", "fn name() { }"),
            ("for", "for ${1:x} in ${2:items} {\n\t$0\n}", "for x in items { }"),
            ("if", "if ${1:condition} {\n\t$0\n}", "if condition { }"),
            ("match", "match ${1:value} {\n\t${2:_} => $0,\n}", "match value { }"),
        ],
        "go" => &[
            ("main", "package main\n\nimport \"fmt\"\n\nfunc main() {\n\t$0\n}", "func main()"),
            ("fmt", "fmt.Println($0)", "fmt.Println()"),
            ("for", "for ${1:i} := 0; ${1:i} < ${2:n}; ${1:i}++ {\n\t$0\n}", "for i := 0; …"),
            ("if", "if ${1:condition} {\n\t$0\n}", "if condition { }"),
            ("func", "func ${1:name}(${2}) {\n\t$0\n}", "func name() { }"),
        ],
        _ => &[],
    }
}

// vstavané funkcie (volajú sa so zátvorkami) – pri vložení dostanú ($0)
fn callable(lang: &str, w: &str) -> bool {
    match lang {
        "py" | "pyw" => !matches!(w, "math" | "time" | "datetime" | "os" | "sys" | "json" | "tkinter" | "pygame" | "random" | "__name__" | "__main__" | "Exception" | "ValueError" | "KeyError" | "IndexError" | "self"),
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => {
            matches!(w, "log" | "error" | "getElementById" | "querySelector" | "querySelectorAll" | "addEventListener" | "createElement" | "appendChild" | "parse" | "stringify" | "fetch" | "then" | "setTimeout" | "setInterval" | "push" | "pop" | "map" | "filter" | "forEach" | "reduce" | "includes" | "indexOf" | "slice" | "splice" | "join" | "split" | "keys" | "values" | "entries" | "require" | "floor" | "random")
        }
        "c" | "h" => matches!(w, "printf" | "scanf" | "puts" | "gets" | "fgets" | "malloc" | "calloc" | "free" | "strlen" | "strcpy" | "strcmp" | "strcat" | "memset" | "memcpy" | "fopen" | "fclose"),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => matches!(w, "push_back" | "size" | "begin" | "end" | "sort" | "getline" | "printf" | "make_shared"),
        "java" => matches!(w, "println" | "print" | "nextLine" | "nextInt" | "length" | "size" | "add" | "get"),
        "cs" => matches!(w, "WriteLine" | "ReadLine" | "Write" | "Parse" | "ToString" | "Add"),
        "go" => matches!(w, "Println" | "Printf" | "Sprintf" | "len" | "append" | "make" | "Scan"),
        _ => false,
    }
}

// funkcie zo súboru: slová, za ktorými niekde stojí „(“
pub fn calls(text: &str, lang: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let mut cur = String::new();
    for c in text.chars() {
        if ident_char(c, lang) {
            cur.push(c);
        } else {
            if c == '(' && cur.chars().count() >= 3 {
                out.insert(cur.clone());
            }
            cur.clear();
        }
    }
    out
}

fn rank(prefix: &str, w: &str) -> Option<u8> {
    let lp = prefix.to_lowercase();
    let lw = w.to_lowercase();
    if w.starts_with(prefix) {
        Some(0)
    } else if lw.starts_with(&lp) {
        Some(1)
    } else if lp.chars().count() >= 3 && lw.contains(&lp) {
        Some(2)
    } else {
        None
    }
}

// slová zo súboru, ktoré stoja za bodkou (self.meno, obj.metoda) – návrhy za „objekt.“
pub fn members(text: &str, lang: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let mut cur = String::new();
    let mut dot = false;
    let mut prev = ' ';
    for c in text.chars().chain(std::iter::once(' ')) {
        if ident_char(c, lang) {
            if cur.is_empty() {
                dot = prev == '.';
            }
            cur.push(c);
        } else {
            if dot && cur.chars().count() >= 2 && !cur.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
                out.insert(std::mem::take(&mut cur));
            }
            cur.clear();
        }
        prev = c;
    }
    out
}

// poradie: zhoda začiatku > bez veľkosti písmen > obsahuje; potom skupina; pri prázdnom začiatku poradie zoznamu
fn ranked(prefix: &str, list: Vec<(u8, Item)>, limit: usize) -> Vec<Item> {
    let mut v: Vec<(u8, u8, usize, Item)> = list.into_iter().enumerate().filter_map(|(i, (g, it))| rank(prefix, &it.label).map(|r| (r, g, i, it))).collect();
    if prefix.is_empty() {
        v.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
    } else {
        v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.3.label.len().cmp(&b.3.label.len())).then(a.3.label.cmp(&b.3.label)));
    }
    let mut seen = std::collections::HashSet::new();
    v.into_iter().filter(|x| seen.insert(x.3.label.clone())).take(limit).map(|x| x.3).collect()
}

// člen z podpisu „log(...data)“ → položka s „log($0)“
fn member_item(sig: &str) -> Item {
    match sig.split_once('(') {
        Some((name, args)) => {
            let body = if args.starts_with(')') { format!("{name}()$0") } else { format!("{name}($0)") };
            Item { label: name.into(), kind: Kind::Builtin, insert: Some(body), detail: Some(sig.into()), doc: None }
        }
        None => Item { label: sig.into(), kind: Kind::Property, insert: None, detail: None, doc: None },
    }
}

fn value_item(v: &str) -> Item {
    match v.strip_suffix("()") {
        Some(f) => Item::snip(v, Kind::Builtin, &format!("{f}($0)"), ""),
        None => Item::plain(v.to_string(), Kind::Keyword),
    }
}

// hlavné návrhy podľa miesta: značky/atribúty/hodnoty, vlastnosti CSS, členy, moduly; inak úryvky, slová a funkcie
#[allow(clippy::too_many_arguments)]
pub fn suggest_at(
    lang: &str,
    prefix: &str,
    cx: &Ctx,
    words: &HashMap<String, usize>,
    calls: &std::collections::HashSet<String>,
    mems: &std::collections::HashSet<String>,
    extra: &[Item],
    limit: usize,
) -> Vec<Item> {
    let lang = cx.lang.unwrap_or(lang);
    let fam = family(lang);
    let d = db();
    // HTML: „</“ → neuzavreté značky (najvnútornejšia prvá)
    if cx.close {
        let mut v: Vec<(u8, Item)> = cx.open_tags.iter().rev().map(|t| (0, Item::snip(t, Kind::Tag, &format!("{t}>$0"), &format!("</{t}>")))).collect();
        v.extend(d.tags.iter().filter(|t| !VOID.contains(&t.0)).map(|(t, _)| (1, Item::snip(t, Kind::Tag, &format!("{t}>$0"), &format!("</{t}>")))));
        return ranked(prefix, v, limit);
    }
    // HTML: za „<“ všetky značky s popisom
    if cx.after_lt {
        // v <head> najprv značky hlavičky; zastarané (font, center…) až za ostatnými
        let in_head = cx.open_tags.last().is_some_and(|t| t == "head");
        let group = |t: &str, doc: &str| {
            if in_head && matches!(t, "title" | "meta" | "link" | "script" | "style" | "base" | "noscript") {
                0
            } else if doc.contains("deprecated") {
                2
            } else {
                1
            }
        };
        let mut v: Vec<(u8, Item)> = d.tags.iter().map(|(t, doc)| (group(t, doc), Item::snip(t, Kind::Tag, &tag_body(t), &tag_preview(t)).doc(doc))).collect();
        v.extend(cx.doc_tags.iter().map(|t| (1, Item::snip(t, Kind::Tag, &format!("{t}>$0</{t}>"), &tag_preview(t)))));
        if "!".starts_with(prefix) || prefix.is_empty() {
            v.push((1, Item::snip("!--", Kind::Snippet, "!-- $0 -->", "<!-- -->")));
        }
        return ranked(prefix, v, limit);
    }
    if let Some((tag, attr)) = &cx.attr_value {
        let mut v: Vec<(u8, Item)> = vec![];
        for key in [format!("{tag}.{attr}"), attr.clone()] {
            if let Some(vals) = d.values.get(key.as_str()) {
                v.extend(vals.iter().map(|x| (0, Item::plain(x.to_string(), Kind::Keyword))));
                break;
            }
        }
        return ranked(prefix, v, limit);
    }
    if let Some(tag) = &cx.in_tag {
        let bools: Vec<&str> = data::BOOL_ATTRS.split_whitespace().collect();
        let mk = |a: &str| {
            if a == "data-" {
                Item::snip("data-", Kind::Property, "data-$1=\"$0\"", "data-*=\"\"")
            } else if bools.contains(&a) {
                Item::plain(a.to_string(), Kind::Property)
            } else {
                Item::snip(a, Kind::Property, &format!("{a}=\"$0\""), &format!("{a}=\"\""))
            }
        };
        let mut v: Vec<(u8, Item)> = d.tag_attrs.get(tag.as_str()).map(|l| l.iter().map(|a| (0, mk(a))).collect()).unwrap_or_default();
        v.extend(data::GLOBAL_ATTRS.split_whitespace().map(|a| (1, mk(a))));
        v.extend(data::EVENT_ATTRS.split_whitespace().map(|a| (2, mk(a))));
        return ranked(prefix, v, limit);
    }
    if fam == "css" {
        if cx.css_at {
            let v = data::CSS_AT.iter().map(|(n, body)| (0, Item::snip(n, Kind::Snippet, body, &format!("@{n}")))).collect();
            return ranked(prefix, v, limit);
        }
        if cx.css_pseudo {
            let v = data::CSS_PSEUDO.split_whitespace().map(|p| (0, match p.strip_suffix("()") {
                Some(f) => Item::snip(p, Kind::Keyword, &format!("{f}($0)"), ""),
                None => Item::plain(p.to_string(), Kind::Keyword),
            })).collect();
            return ranked(prefix, v, limit);
        }
        if let Some(prop) = &cx.css_value {
            let mut v: Vec<(u8, Item)> = d.css.iter().find(|(p, _)| p == prop).map(|(_, vals)| vals.iter().map(|x| (0, value_item(x))).collect()).unwrap_or_default();
            if prop.contains("color") || matches!(prop.as_str(), "background" | "border" | "border-top" | "border-bottom" | "border-left" | "border-right" | "outline" | "fill" | "stroke" | "box-shadow" | "text-shadow" | "caret-color") {
                v.extend(data::CSS_COLORS.split_whitespace().map(|x| (1, value_item(x))));
            }
            v.extend(data::CSS_COMMON.split_whitespace().map(|x| (2, value_item(x))));
            return ranked(prefix, v, limit);
        }
        if cx.css_block {
            let v = d.css.iter().map(|(p, _)| (0, Item::snip(p, Kind::Property, &format!("{p}: $0;"), &format!("{p}: ;")))).collect();
            return ranked(prefix, v, limit);
        }
        // selektor: značky HTML a triedy/mená zo súboru
        if prefix.is_empty() {
            return vec![];
        }
        let mut v: Vec<(u8, Item)> = d.tags.iter().map(|(t, doc)| (0, Item::plain(t.to_string(), Kind::Tag).doc(doc))).collect();
        v.extend(words.keys().map(|w| (1, Item::plain(w.clone(), Kind::Word))));
        return ranked(prefix, v, limit);
    }
    if let Some(m) = &cx.import {
        let mut v: Vec<(u8, Item)> = vec![];
        if m.is_empty() {
            let mods = if fam == "js" { data::JS_MODULES } else { data::PY_MODULES };
            v.extend(mods.split_whitespace().map(|x| (0, Item::plain(x.to_string(), Kind::Keyword))));
        } else if let Some(list) = d.members.get(&(fam, m.as_str())) {
            v.extend(list.iter().map(|sig| (0, Item::plain(sig.split('(').next().unwrap_or(sig).to_string(), Kind::Builtin))));
        }
        return ranked(prefix, v, limit);
    }
    if let Some(obj) = &cx.member {
        let lf = if fam == "c" { "cpp" } else { fam };
        let last = obj.rsplit('.').next().unwrap_or(obj);
        let known = d.members.get(&(lf, obj.as_str())).or_else(|| d.members.get(&(lf, last)));
        let mut v: Vec<(u8, Item)> = known.map(|l| l.iter().map(|s| (0, member_item(s))).collect()).unwrap_or_default();
        // slová zo súboru za bodkou (vlastné metódy, self.meno…)
        let mut own: Vec<&String> = mems.iter().filter(|w| w.as_str() != prefix).collect();
        own.sort();
        v.extend(own.into_iter().map(|w| {
            let mut it = Item::plain(w.clone(), Kind::Word);
            if calls.contains(w) {
                it.insert = Some(format!("{w}($0)"));
                it.detail = Some(format!("{w}()"));
            }
            (1, it)
        }));
        if known.is_none() {
            if let Some(l) = d.members.get(&(lf, "*")) {
                v.extend(l.iter().map(|s| (2, member_item(s))));
            }
        }
        return ranked(prefix, v, limit);
    }
    // Emmet v texte HTML: „div.box“ → celý úryvok; slovo → všetky značky s ním (ako VS Code)
    if let Some(abbr) = &cx.emmet {
        let mut v: Vec<(u8, Item)> = vec![];
        if abbr.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            for (t, doc) in &d.tags {
                if t.starts_with(&abbr.to_ascii_lowercase()) {
                    let body = format!("<{}", tag_body(t));
                    v.push((u8::from(doc.contains("deprecated")), Item::snip(t, Kind::Tag, &body, &tag_preview(t)).doc(doc)));
                }
            }
            v.extend(snippets(lang).iter().filter(|s| s.0.starts_with(abbr.as_str())).map(|(t, b, p)| (2, Item::snip(t, Kind::Snippet, b, p))));
            let mut ws: Vec<(&String, &usize)> = words.iter().filter(|(w, n)| w.starts_with(abbr.as_str()) && (w.as_str() != abbr || **n > 1)).collect();
            ws.sort_by(|a, b| b.1.cmp(a.1));
            v.extend(ws.into_iter().map(|(w, _)| (3, Item::plain(w.clone(), Kind::Word))));
        } else if let Some(body) = emmet(abbr) {
            let prev = strip_stops(&body).replace('\t', "").replace('\n', "");
            v.push((0, Item::snip(abbr, Kind::Snippet, &body, &prev).doc("Emmet abbreviation")));
        }
        if !v.is_empty() {
            return ranked(abbr, v, limit);
        }
    }
    let mut out: Vec<(u8, u8, Item)> = vec![];
    if fam == "html" && "!".starts_with(prefix) {
        out.push((0, 0, Item::snip("!", Kind::Snippet, HTML5, "HTML5 page")));
    }
    for (trig, body, prev) in snippets(lang) {
        if let Some(r) = rank(prefix, trig) {
            out.push((r, 0, Item::snip(trig, Kind::Snippet, body, prev)));
        }
    }
    for it in extra {
        if let Some(r) = rank(prefix, &it.label) {
            out.push((r, 0, it.clone()));
        }
    }
    let have: std::collections::HashSet<String> = out.iter().map(|x| x.2.label.clone()).collect();
    for mut it in suggest(lang, prefix, words, limit * 2) {
        if have.contains(&it.label) {
            continue;
        }
        // HTML: názov atribútu (type, class, href…) sa všade dopĺňa ako atribút – s =""
        if fam == "html" && it.insert.is_none() && !d.tags.iter().any(|t| t.0 == it.label) {
            let known = d.tag_attrs.values().any(|l| l.contains(&it.label.as_str())) || data::GLOBAL_ATTRS.split_whitespace().chain(data::EVENT_ATTRS.split_whitespace()).any(|a| a == it.label);
            if known && !data::BOOL_ATTRS.split_whitespace().any(|a| a == it.label) {
                it.insert = Some(format!("{}=\"$0\"", it.label));
                it.detail = Some(format!("{}=\"\"", it.label));
                it.kind = Kind::Property;
            }
        }
        let f = (it.kind == Kind::Builtin && callable(lang, &it.label)) || (it.kind == Kind::Word && calls.contains(&it.label));
        if f && !it.label.ends_with('!') {
            it.insert = Some(format!("{}($0)", it.label));
            it.detail = Some(format!("{}()", it.label));
        }
        let r = rank(prefix, &it.label).unwrap_or(2);
        out.push((r, 1, it));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    out.into_iter().take(limit).map(|x| x.2).collect()
}

fn tag_preview(t: &str) -> String {
    if VOID.contains(&t) {
        format!("<{t}>")
    } else {
        format!("<{t}></{t}>")
    }
}

// úryvok → text a miesta kurzora: $1, ${1:text} (vybrané), $0 (koniec); \t = odsadenie
// vráti (text, zastávky v poradí 1, 2, … a nakoniec 0); rovnaké číslo viackrát = prvý výskyt
pub fn expand(body: &str, indent: &str, tab: &str) -> (String, Vec<(usize, usize)>) {
    let mut out = String::new();
    let mut n = 0usize; // počet znakov v out
    let mut stops: Vec<(u32, usize, usize)> = vec![];
    let cs: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '$' && i + 1 < cs.len() && (cs[i + 1].is_ascii_digit() || cs[i + 1] == '{') {
            let (num, def, next) = if cs[i + 1] == '{' {
                let close = cs[i..].iter().position(|ch| *ch == '}').map(|p| p + i).unwrap_or(cs.len() - 1);
                let inner: String = cs[i + 2..close].iter().collect();
                let (a, b) = inner.split_once(':').unwrap_or((inner.as_str(), ""));
                (a.parse::<u32>().unwrap_or(0), b.to_string(), close + 1)
            } else {
                let mut j = i + 1;
                while j < cs.len() && cs[j].is_ascii_digit() {
                    j += 1;
                }
                (cs[i + 1..j].iter().collect::<String>().parse::<u32>().unwrap_or(0), String::new(), j)
            };
            let len = def.chars().count();
            if !stops.iter().any(|s| s.0 == num) {
                stops.push((num, n, n + len));
            }
            out.push_str(&def);
            n += len;
            i = next;
            continue;
        }
        if c == '\n' {
            out.push('\n');
            out.push_str(indent);
            n += 1 + indent.chars().count();
        } else if c == '\t' {
            out.push_str(tab);
            n += tab.chars().count();
        } else {
            out.push(c);
            n += 1;
        }
        i += 1;
    }
    stops.sort_by_key(|s| if s.0 == 0 { u32::MAX } else { s.0 });
    let mut v: Vec<(usize, usize)> = stops.into_iter().map(|s| (s.1, s.2)).collect();
    if v.is_empty() {
        v.push((n, n));
    }
    (out, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_ranks_prefix_first() {
        let w = words("total_price = 3\ntotal_price += pri\nprinter = 1\n", "py");
        let s = suggest("py", "pri", &w, 8);
        assert_eq!(s[0].label, "print", "{s:?}");
        assert!(s.iter().any(|i| i.label == "printer"));
        assert!(s.iter().any(|i| i.label == "total_price"), "contains match: {s:?}");
        assert!(!s.iter().any(|i| i.label == "pri"));
        // vstavaná funkcia napísaná celá sa ponúka ďalej
        let b = suggest("py", "str", &HashMap::new(), 8);
        assert_eq!(b[0].label, "str", "{b:?}");
    }

    #[test]
    fn complete_words_and_css() {
        let w = words("a1 abc abc 9xy my-var", "css");
        assert_eq!(w.get("abc"), Some(&2));
        assert!(w.contains_key("my-var"));
        assert!(!w.contains_key("9xy"));
        assert_eq!(suggest("css", "backg", &HashMap::new(), 3)[0].label, "background");
    }

    #[test]
    fn complete_html_tag() {
        let before: Vec<char> = "<head>\n  <ti".chars().collect();
        let cx = context("html", &before);
        assert!(cx.after_lt);
        let s = suggest_at("html", "ti", &cx, &HashMap::new(), &Default::default(), &Default::default(), &[], 8);
        assert_eq!(s[0].label, "title");
        let (t, stops) = expand(s[0].insert.as_deref().unwrap(), "", "  ");
        assert_eq!(t, "title></title>");
        assert_eq!(stops, vec![(6, 6)]);
        let b2: Vec<char> = "<div cl".chars().collect();
        let cx2 = context("html", &b2);
        assert_eq!(cx2.in_tag.as_deref(), Some("div"));
        assert_eq!(suggest_at("html", "cl", &cx2, &HashMap::new(), &Default::default(), &Default::default(), &[], 8)[0].insert.as_deref(), Some("class=\"$0\""));
    }

    #[test]
    fn complete_python_for() {
        let before: Vec<char> = "    fo".chars().collect();
        let cx = context("py", &before);
        let s = suggest_at("py", "fo", &cx, &HashMap::new(), &Default::default(), &Default::default(), &[], 8);
        assert_eq!(s[0].label, "for");
        let (t, stops) = expand(s[0].insert.as_deref().unwrap(), "    ", "    ");
        assert_eq!(t, "for item in items:\n        ");
        assert_eq!(stops, vec![(4, 8), (12, 17), (27, 27)]);
        // vstavaná funkcia so zátvorkami
        let p = suggest_at("py", "pri", &cx, &HashMap::new(), &Default::default(), &Default::default(), &[], 8);
        assert_eq!(p[0].insert.as_deref(), Some("print($0)"));
    }

    #[test]
    fn complete_css_property_and_value() {
        let b: Vec<char> = "a {\n  disp".chars().collect();
        let s = suggest_at("css", "disp", &context("css", &b), &HashMap::new(), &Default::default(), &Default::default(), &[], 8);
        assert_eq!(s[0].insert.as_deref(), Some("display: $0;"));
        let b2: Vec<char> = "a { display: fl".chars().collect();
        let cx = context("css", &b2);
        assert_eq!(cx.css_value.as_deref(), Some("display"));
        assert_eq!(suggest_at("css", "fl", &cx, &HashMap::new(), &Default::default(), &Default::default(), &[], 8)[0].label, "flex");
    }

    fn at(lang: &str, text: &str) -> Vec<Item> {
        let b: Vec<char> = text.chars().collect();
        let cx = context(lang, &b);
        let plen = b.iter().rev().take_while(|ch| ident_char(**ch, lang)).count();
        let prefix: String = b[b.len() - plen..].iter().collect();
        suggest_at(lang, &prefix, &cx, &words(text, lang), &calls(text, lang), &members(text, lang), &[], 60)
    }

    #[test]
    fn complete_html_lt_and_font() {
        // „<“ samo = všetky značky (ako VS Code), so spúšťačom
        let b: Vec<char> = "<body>\n<".chars().collect();
        assert!(trigger("html", &b));
        let all = at("html", "<body>\n<");
        assert!(all.len() >= 50, "{}", all.len());
        let f = at("html", "<p><fo");
        assert!(f.iter().any(|i| i.label == "font"), "{f:?}");
        let font = f.iter().find(|i| i.label == "font").unwrap();
        assert_eq!(font.insert.as_deref(), Some("font>$0</font>"));
        assert!(font.doc.is_some());
        // atribúty značky font a ich hodnoty
        let a = at("html", "<font ");
        assert!(trigger("html", &"<font ".chars().collect::<Vec<_>>()));
        assert_eq!(a[0].label, "color");
        let v = at("html", "<font size=\"");
        assert!(v.iter().any(|i| i.label == "3"));
        let t = at("html", "<input type=\"che");
        assert_eq!(t[0].label, "checkbox");
        // bez hodnoty
        assert_eq!(at("html", "<input disa")[0].insert, None);
    }

    #[test]
    fn complete_html_attr_word_gets_quotes() {
        // aj mimo značky (napr. v rozpísanom `</ul ty`) sa „type“ dopĺňa ako type=""
        let s = at("html", "<ul>\n</ul ty");
        let it = s.iter().find(|i| i.label == "type").expect("type");
        assert_eq!(it.insert.as_deref(), Some("type=\"$0\""));
        let s = at("html", "<body>\ncla");
        assert_eq!(s.iter().find(|i| i.label == "class").and_then(|i| i.insert.clone()).as_deref(), Some("class=\"$0\""));
    }

    #[test]
    fn complete_html_close_tag() {
        let text = "<div>\n  <ul>\n    <li>a</li>\n  </";
        assert!(trigger("html", &text.chars().collect::<Vec<_>>()));
        let s = at("html", text);
        assert_eq!(s[0].label, "ul");
        assert_eq!(s[0].insert.as_deref(), Some("ul>$0"));
        assert_eq!(s[1].label, "div");
        assert_eq!(open_tags(&"<html><body><br><img src=\"a\"/><p>x</p><!-- <b> -->".chars().collect::<Vec<_>>()), vec!["html", "body"]);
    }

    #[test]
    fn complete_html_embedded_css_js() {
        let s = at("html", "<style>\n  body { disp");
        assert_eq!(s[0].insert.as_deref(), Some("display: $0;"));
        let j = at("html", "<script>\n  console.");
        assert!(j.iter().any(|i| i.label == "log"), "{j:?}");
        // po </style> zase HTML
        assert!(at("html", "<style></style>\n<sp").iter().any(|i| i.label == "span"));
    }

    #[test]
    fn complete_members() {
        let s = at("js", "console.");
        assert_eq!(s[0].label, "log");
        assert_eq!(s[0].insert.as_deref(), Some("log($0)"));
        assert!(trigger("js", &"document.".chars().collect::<Vec<_>>()));
        assert!(at("js", "document.getE").iter().any(|i| i.label == "getElementById"));
        assert!(!trigger("js", &"x = 3.".chars().collect::<Vec<_>>()));
        let p = at("py", "import os\nos.path.");
        assert!(p.iter().any(|i| i.label == "join"));
        // vlastné členy zo súboru
        let o = at("py", "class A:\n    def go(self):\n        self.count = 1\n        self.");
        assert!(o.iter().any(|i| i.label == "count"), "{o:?}");
        let n = at("py", "name = 'a'\nname.up");
        assert_eq!(n[0].label, "upper");
        assert_eq!(n[0].insert.as_deref(), Some("upper()$0"));
        assert!(at("cpp", "std::co").iter().any(|i| i.label == "cout"));
        assert_eq!(at("py", "import ra")[0].label, "random");
        assert!(at("py", "from math import sq").iter().any(|i| i.label == "sqrt"));
    }

    #[test]
    fn complete_css_more() {
        assert!(at("css", "a:ho").iter().any(|i| i.label == "hover"));
        assert!(at("css", "@me").iter().any(|i| i.label == "media"));
        let c = at("css", "a { color: re");
        assert_eq!(c[0].label, "red");
        assert!(at("css", "a { background-color: ").iter().any(|i| i.label == "transparent"));
        assert!(trigger("css", &"a { color: ".chars().collect::<Vec<_>>()));
        assert!(at("css", "a { gri").iter().any(|i| i.label == "grid-template-columns"));
    }

    #[test]
    fn complete_emmet() {
        assert_eq!(emmet("div.box").as_deref(), Some("<div class=\"box\">$1</div>"));
        assert_eq!(emmet("ul>li*3").as_deref(), Some("<ul>\n\t<li>$1</li>\n\t<li>$2</li>\n\t<li>$3</li>\n</ul>"));
        assert_eq!(emmet("p{Ahoj}+br").as_deref(), Some("<p>Ahoj</p>\n<br>"));
        assert_eq!(emmet("#top.a.b").as_deref(), Some("<div id=\"top\" class=\"a b\">$1</div>"));
        assert_eq!(emmet("li.item$*2").as_deref(), Some("<li class=\"item1\">$1</li>\n<li class=\"item2\">$2</li>"));
        assert_eq!(emmet("div."), None);
        // v texte HTML: slovo → značky (aj font), skratka → celý úryvok
        let s = at("html", "<body>\n  fo");
        assert!(s.iter().any(|i| i.label == "font" && i.insert.as_deref() == Some("<font>$0</font>")), "{s:?}");
        let b = at("html", "<body>\n  div.box");
        assert_eq!(b[0].insert.as_deref(), Some("<div class=\"box\">$1</div>"));
        assert!(trigger("html", &"<p>p{Hi}".chars().collect::<Vec<_>>()));
        // nie vnútri značky ani v próze s bodkou
        assert_eq!(context("html", &"<div cl".chars().collect::<Vec<_>>()).emmet, None);
        assert_eq!(context("html", &"Ahoj e.g.".chars().collect::<Vec<_>>()).emmet, None);
        // vlastná značka zo súboru po „<“
        assert!(at("html", "<my-card></my-card>\n<my").iter().any(|i| i.label == "my-card"));
    }
}
