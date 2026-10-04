// Návrhy pri písaní: kľúčové slová a vstavané funkcie jazyka + slová z otvoreného súboru.
// Poradie: rovnaký začiatok (aj veľkosť písmen) > začiatok bez ohľadu na veľkosť > obsahuje; potom podľa početnosti.
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Keyword,
    Builtin,
    Word,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub label: String,
    pub kind: Kind,
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
        .filter(|(w, _)| w != prefix)
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
    v.into_iter().take(limit).map(|(_, _, label, kind)| Item { label, kind }).collect()
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
    }

    #[test]
    fn complete_words_and_css() {
        let w = words("a1 abc abc 9xy my-var", "css");
        assert_eq!(w.get("abc"), Some(&2));
        assert!(w.contains_key("my-var"));
        assert!(!w.contains_key("9xy"));
        assert_eq!(suggest("css", "backg", &HashMap::new(), 3)[0].label, "background");
    }
}
