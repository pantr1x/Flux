// Návrhy pri písaní: kľúčové slová a vstavané funkcie jazyka + slová z otvoreného súboru.
// Poradie: rovnaký začiatok (aj veľkosť písmen) > začiatok bez ohľadu na veľkosť > obsahuje; potom podľa početnosti.
use std::collections::HashMap;

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
}

impl Item {
    fn plain(label: String, kind: Kind) -> Self {
        Item { label, kind, insert: None, detail: None }
    }
    fn snip(label: &str, kind: Kind, body: &str, detail: &str) -> Self {
        Item { label: label.into(), kind, insert: Some(body.into()), detail: (!detail.is_empty()).then(|| detail.into()) }
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
    v.into_iter().take(limit).map(|(_, _, label, kind)| Item::plain(label, kind)).collect()
}

// ---- návrhy podľa miesta v kóde (ako VS Code): značky HTML, vlastnosti CSS, bloky kódu ----

// kde sa píše: hneď za „<“ (značka HTML), vnútri otvorenej značky (atribút), za „vlastnosť:“ v CSS (hodnota)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ctx {
    pub after_lt: bool,
    pub in_tag: bool,
    pub css_value: Option<String>,
}

pub fn context(lang: &str, before: &[char]) -> Ctx {
    let mut c = Ctx::default();
    let plen = before.iter().rev().take_while(|ch| ident_char(**ch, lang)).count();
    let head = &before[..before.len() - plen];
    match lang {
        "html" | "htm" | "xml" | "vue" | "svelte" | "php" => {
            c.after_lt = head.last() == Some(&'<');
            if !c.after_lt {
                let lt = head.iter().rposition(|ch| *ch == '<');
                let gt = head.iter().rposition(|ch| *ch == '>');
                c.in_tag = lt.is_some_and(|l| gt.is_none_or(|g| g < l) && head.get(l + 1) != Some(&'/') && head.get(l + 1) != Some(&'!'));
                // vnútri úvodzoviek atribútu nič
                if c.in_tag {
                    let q = head[lt.unwrap()..].iter().filter(|ch| **ch == '"').count();
                    c.in_tag = q % 2 == 0;
                }
            }
        }
        "css" | "scss" | "less" => {
            let semi = head.iter().rposition(|ch| matches!(*ch, ';' | '{' | '}'));
            if let Some(col) = head.iter().rposition(|ch| *ch == ':').filter(|col| semi.is_none_or(|s| s < *col)) {
                let start = semi.map(|s| s + 1).unwrap_or(0);
                c.css_value = Some(head[start..col].iter().collect::<String>().trim().to_string());
            }
        }
        _ => {}
    }
    c
}

pub const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];
const TAGS: &[&str] = &[
    "a", "abbr", "article", "aside", "audio", "b", "blockquote", "body", "br", "button", "canvas", "code", "div", "em", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "hr",
    "html", "i", "iframe", "img", "input", "label", "li", "link", "main", "meta", "nav", "ol", "option", "p", "pre", "script", "section", "select", "small", "span", "strong", "style", "table",
    "tbody", "td", "textarea", "th", "thead", "title", "tr", "u", "ul", "video",
];
const ATTRS: &[&str] = &["class", "id", "href", "src", "alt", "type", "name", "value", "placeholder", "style", "title", "rel", "target", "width", "height", "for", "onclick", "disabled", "checked", "lang", "charset", "content"];
const HTML5: &str = "<!DOCTYPE html>\n<html lang=\"${1:en}\">\n<head>\n\t<meta charset=\"UTF-8\">\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\t<title>${2:Document}</title>\n</head>\n<body>\n\t$0\n</body>\n</html>";

fn tag_body(t: &str) -> String {
    match t {
        "a" => "a href=\"$1\">$0</a>".into(),
        "img" => "img src=\"$1\" alt=\"$2\">".into(),
        "link" => "link rel=\"stylesheet\" href=\"$1\">".into(),
        "script" => "script src=\"$1\"></script>".into(),
        "input" => "input type=\"${1:text}\" $0>".into(),
        "meta" => "meta $0>".into(),
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

// hlavné návrhy: úryvky a značky/vlastnosti podľa miesta, potom slová a funkcie (so zátvorkami)
pub fn suggest_at(lang: &str, prefix: &str, cx: &Ctx, words: &HashMap<String, usize>, calls: &std::collections::HashSet<String>, extra: &[Item], limit: usize) -> Vec<Item> {
    // HTML: za „<“ len značky
    if cx.after_lt {
        let mut v: Vec<(u8, Item)> = TAGS
            .iter()
            .filter_map(|t| rank(prefix, t).map(|r| (r, Item::snip(t, Kind::Tag, &tag_body(t), &tag_preview(t)))))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.label.len().cmp(&b.1.label.len())));
        return v.into_iter().take(limit).map(|x| x.1).collect();
    }
    if cx.in_tag {
        let mut v: Vec<(u8, Item)> = ATTRS.iter().filter_map(|a| rank(prefix, a).map(|r| (r, Item::snip(a, Kind::Property, &format!("{a}=\"$0\""), &format!("{a}=\"\""))))).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        return v.into_iter().take(limit).map(|x| x.1).collect();
    }
    let mut out: Vec<(u8, u8, Item)> = vec![];
    if let Some(prop) = &cx.css_value {
        for v in css_values(prop) {
            if let Some(r) = rank(prefix, v) {
                out.push((r, 0, Item::snip(v, Kind::Keyword, &format!("{v};$0"), "")));
            }
        }
    } else if matches!(lang, "css" | "scss" | "less") {
        for p in builtins(lang).iter().filter(|w| w.contains('-') || CSS_PROPS.contains(w)) {
            if let Some(r) = rank(prefix, p) {
                out.push((r, 0, Item::snip(p, Kind::Property, &format!("{p}: $0;"), &format!("{p}: ;"))));
            }
        }
    }
    if matches!(lang, "html" | "htm") && "!".starts_with(prefix) {
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

const CSS_PROPS: &[&str] = &["color", "background", "margin", "padding", "border", "display", "flex", "grid", "position", "width", "height", "gap", "transition", "transform", "opacity", "cursor", "overflow"];

fn css_values(prop: &str) -> &'static [&'static str] {
    match prop {
        "display" => &["block", "inline", "inline-block", "flex", "grid", "none"],
        "position" => &["relative", "absolute", "fixed", "sticky", "static"],
        "justify-content" => &["center", "flex-start", "flex-end", "space-between", "space-around", "space-evenly"],
        "align-items" => &["center", "flex-start", "flex-end", "stretch", "baseline"],
        "flex-direction" => &["row", "column", "row-reverse", "column-reverse"],
        "text-align" => &["left", "center", "right", "justify"],
        "font-weight" => &["normal", "bold", "400", "500", "600", "700"],
        "cursor" => &["pointer", "default", "text", "move", "not-allowed"],
        "overflow" => &["hidden", "auto", "scroll", "visible"],
        _ => &["auto", "none", "inherit", "initial"],
    }
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
        let s = suggest_at("html", "ti", &cx, &HashMap::new(), &Default::default(), &[], 8);
        assert_eq!(s[0].label, "title");
        let (t, stops) = expand(s[0].insert.as_deref().unwrap(), "", "  ");
        assert_eq!(t, "title></title>");
        assert_eq!(stops, vec![(6, 6)]);
        let b2: Vec<char> = "<div cl".chars().collect();
        let cx2 = context("html", &b2);
        assert!(cx2.in_tag);
        assert_eq!(suggest_at("html", "cl", &cx2, &HashMap::new(), &Default::default(), &[], 8)[0].insert.as_deref(), Some("class=\"$0\""));
    }

    #[test]
    fn complete_python_for() {
        let before: Vec<char> = "    fo".chars().collect();
        let cx = context("py", &before);
        let s = suggest_at("py", "fo", &cx, &HashMap::new(), &Default::default(), &[], 8);
        assert_eq!(s[0].label, "for");
        let (t, stops) = expand(s[0].insert.as_deref().unwrap(), "    ", "    ");
        assert_eq!(t, "for item in items:\n        ");
        assert_eq!(stops, vec![(4, 8), (12, 17), (27, 27)]);
        // vstavaná funkcia so zátvorkami
        let p = suggest_at("py", "pri", &cx, &HashMap::new(), &Default::default(), &[], 8);
        assert_eq!(p[0].insert.as_deref(), Some("print($0)"));
    }

    #[test]
    fn complete_css_property_and_value() {
        let b: Vec<char> = "a {\n  disp".chars().collect();
        let s = suggest_at("css", "disp", &context("css", &b), &HashMap::new(), &Default::default(), &[], 8);
        assert_eq!(s[0].insert.as_deref(), Some("display: $0;"));
        let b2: Vec<char> = "a { display: fl".chars().collect();
        let cx = context("css", &b2);
        assert_eq!(cx.css_value.as_deref(), Some("display"));
        assert_eq!(suggest_at("css", "fl", &cx, &HashMap::new(), &Default::default(), &[], 8)[0].label, "flex");
    }
}
