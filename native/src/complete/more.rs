// Ďalšie jazyky pre návrhy pri písaní: kľúčové slová, vstavané mená a úryvky (Ruby, PHP, Lua, Zig, R, Julia,
// Perl, shell, PowerShell, Kotlin, Swift, Dart, SQL). Základné jazyky sú v complete.rs.

pub fn keywords(lang: &str) -> &'static [&'static str] {
    match lang {
        "rb" => &[
            "alias", "and", "begin", "break", "case", "class", "def", "defined?", "do", "else", "elsif", "end", "ensure", "false", "for", "if", "in", "module", "next", "nil", "not", "or", "redo",
            "rescue", "retry", "return", "self", "super", "then", "true", "undef", "unless", "until", "when", "while", "yield", "require", "attr_accessor", "attr_reader",
        ],
        "php" => &[
            "abstract", "and", "array", "as", "break", "case", "catch", "class", "const", "continue", "default", "do", "echo", "else", "elseif", "extends", "false", "final", "finally", "fn", "for",
            "foreach", "function", "global", "if", "implements", "include", "interface", "namespace", "new", "null", "or", "private", "protected", "public", "require", "return", "static", "switch",
            "throw", "true", "try", "use", "var", "while", "match", "readonly",
        ],
        "lua" => &["and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while"],
        "zig" => &[
            "align", "and", "anytype", "break", "catch", "comptime", "const", "continue", "defer", "else", "enum", "errdefer", "error", "export", "extern", "false", "fn", "for", "if", "inline", "null",
            "or", "orelse", "packed", "pub", "return", "struct", "switch", "test", "true", "try", "undefined", "union", "unreachable", "var", "void", "while", "usize", "bool",
        ],
        "r" => &["if", "else", "repeat", "while", "function", "for", "next", "break", "TRUE", "FALSE", "NULL", "NA", "Inf", "NaN", "in", "return", "library"],
        "jl" => &[
            "abstract", "baremodule", "begin", "break", "catch", "const", "continue", "do", "else", "elseif", "end", "export", "false", "finally", "for", "function", "global", "if", "import", "in",
            "let", "local", "macro", "module", "mutable", "nothing", "quote", "return", "struct", "true", "try", "using", "where", "while",
        ],
        "pl" | "pm" => &["my", "our", "local", "sub", "if", "elsif", "else", "unless", "while", "until", "for", "foreach", "last", "next", "return", "use", "package", "require", "print", "die", "eq", "ne", "and", "or", "not"],
        "sh" | "bash" | "zsh" => &[
            "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "until", "case", "esac", "function", "return", "break", "continue", "local", "export", "readonly", "exit", "echo", "read",
            "source", "true", "false",
        ],
        "ps1" | "psm1" => &[
            "if", "elseif", "else", "foreach", "for", "while", "do", "until", "switch", "function", "param", "return", "break", "continue", "try", "catch", "finally", "throw", "in", "begin", "process",
            "end", "$true", "$false", "$null",
        ],
        "kt" | "kts" => &[
            "as", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in", "interface", "is", "null", "object", "package", "return", "super", "this", "throw", "true", "try",
            "typealias", "val", "var", "when", "while", "data", "import", "override", "private", "public", "suspend", "companion", "sealed", "enum",
        ],
        "swift" => &[
            "associatedtype", "break", "case", "catch", "class", "continue", "default", "defer", "do", "else", "enum", "extension", "false", "for", "func", "guard", "if", "import", "in", "init", "let",
            "nil", "private", "protocol", "public", "repeat", "return", "self", "static", "struct", "switch", "throw", "true", "try", "var", "where", "while", "async", "await",
        ],
        "dart" => &[
            "abstract", "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "default", "do", "else", "enum", "extends", "false", "final", "finally", "for", "if",
            "implements", "import", "in", "is", "late", "new", "null", "required", "return", "static", "super", "switch", "this", "throw", "true", "try", "var", "void", "while", "with",
        ],
        "sql" => &[
            "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "ALTER", "DROP", "INDEX", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "ON", "GROUP",
            "BY", "ORDER", "HAVING", "LIMIT", "OFFSET", "AS", "AND", "OR", "NOT", "NULL", "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "DEFAULT", "UNIQUE", "DISTINCT", "UNION", "ASC", "DESC", "LIKE",
            "IN", "BETWEEN", "EXISTS", "COUNT", "SUM", "AVG", "MIN", "MAX", "INTEGER", "TEXT", "VARCHAR", "BOOLEAN", "DATE", "TIMESTAMP",
        ],
        _ => &[],
    }
}

pub fn builtins(lang: &str) -> &'static [&'static str] {
    match lang {
        "rb" => &["puts", "print", "gets", "chomp", "each", "map", "select", "reject", "length", "size", "push", "pop", "to_s", "to_i", "to_f", "include?", "empty?", "times", "upto", "format", "require_relative", "Array", "Hash", "String", "Integer"],
        "php" => &[
            "echo", "print_r", "var_dump", "count", "strlen", "str_replace", "substr", "strpos", "explode", "implode", "array_push", "array_pop", "array_map", "array_filter", "array_keys", "in_array",
            "isset", "empty", "json_encode", "json_decode", "file_get_contents", "file_put_contents", "intval", "floatval", "trim", "strtolower", "strtoupper", "date", "time", "htmlspecialchars", "$_GET",
            "$_POST", "$_SESSION", "$this",
        ],
        "lua" => &["print", "pairs", "ipairs", "type", "tostring", "tonumber", "require", "table", "insert", "remove", "concat", "string", "format", "sub", "len", "math", "floor", "random", "io", "read", "write", "os", "time", "setmetatable", "error", "pcall"],
        "zig" => &["std", "debug", "print", "@import", "@intCast", "@as", "@sizeOf", "@TypeOf", "allocator", "ArrayList", "mem", "heap", "io", "u8", "u32", "u64", "i32", "i64", "f32", "f64", "main"],
        "r" => &[
            "print", "cat", "paste", "paste0", "c", "length", "seq", "rep", "sum", "mean", "median", "sd", "min", "max", "round", "data.frame", "read.csv", "write.csv", "head", "tail", "summary", "str",
            "plot", "hist", "lines", "apply", "lapply", "sapply", "nrow", "ncol", "names", "is.na", "as.numeric", "as.character", "readline",
        ],
        "jl" => &["println", "print", "length", "push!", "pop!", "append!", "map", "filter", "reduce", "sum", "sort", "sort!", "zeros", "ones", "rand", "range", "string", "parse", "readline", "typeof", "Int", "Float64", "String", "Vector", "Dict", "Array"],
        "pl" | "pm" => &["print", "printf", "chomp", "push", "pop", "shift", "unshift", "split", "join", "length", "scalar", "keys", "values", "sort", "reverse", "open", "close", "strict", "warnings", "STDIN", "defined"],
        "sh" | "bash" | "zsh" => &["echo", "printf", "read", "cd", "ls", "cat", "grep", "sed", "awk", "find", "mkdir", "rm", "cp", "mv", "chmod", "curl", "test", "set", "shift", "trap", "sleep", "date", "basename", "dirname"],
        "ps1" | "psm1" => &["Write-Host", "Write-Output", "Read-Host", "Get-ChildItem", "Get-Content", "Set-Content", "Get-Item", "Remove-Item", "New-Item", "Copy-Item", "Move-Item", "Test-Path", "ForEach-Object", "Where-Object", "Select-Object", "Start-Process", "Get-Date"],
        "kt" | "kts" => &["println", "print", "readLine", "listOf", "mutableListOf", "mapOf", "mutableMapOf", "setOf", "String", "Int", "Double", "Boolean", "List", "Map", "size", "forEach", "map", "filter", "toInt", "toString", "main"],
        "swift" => &["print", "readLine", "String", "Int", "Double", "Bool", "Array", "Dictionary", "count", "append", "remove", "map", "filter", "reduce", "forEach", "sorted", "isEmpty", "contains", "Foundation"],
        "dart" => &["print", "String", "int", "double", "bool", "List", "Map", "Set", "Future", "length", "add", "remove", "map", "where", "forEach", "toString", "parse", "stdin", "readLineSync", "main"],
        _ => &[],
    }
}

// (spúšťač, kód, náhľad)
pub fn snippets(lang: &str) -> &'static [(&'static str, &'static str, &'static str)] {
    match lang {
        "rb" => &[
            ("def", "def ${1:name}(${2})\n\t$0\nend", "def name … end"),
            ("class", "class ${1:Name}\n\tdef initialize(${2})\n\t\t$0\n\tend\nend", "class Name … end"),
            ("if", "if ${1:condition}\n\t$0\nend", "if … end"),
            ("each", "${1:items}.each do |${2:item}|\n\t$0\nend", "items.each do |item|"),
            ("puts", "puts ${1:\"$2\"}$0", "puts \"\""),
            ("while", "while ${1:condition}\n\t$0\nend", "while … end"),
        ],
        "php" => &[
            ("php", "<?php\n\n$0", "<?php"),
            ("function", "function ${1:name}(${2}) {\n\t$0\n}", "function name() { }"),
            ("foreach", "foreach (${1:\\$items} as ${2:\\$item}) {\n\t$0\n}", "foreach ($items as $item) { }"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("class", "class ${1:Name} {\n\tpublic function __construct(${2}) {\n\t\t$0\n\t}\n}", "class Name { }"),
            ("echo", "echo ${1:\"$2\"};$0", "echo \"\";"),
        ],
        "lua" => &[
            ("function", "function ${1:name}(${2})\n\t$0\nend", "function name() … end"),
            ("local", "local ${1:name} = $0", "local name ="),
            ("if", "if ${1:condition} then\n\t$0\nend", "if … then … end"),
            ("for", "for ${1:i} = 1, ${2:10} do\n\t$0\nend", "for i = 1, 10 do"),
            ("forp", "for ${1:key}, ${2:value} in pairs(${3:t}) do\n\t$0\nend", "for key, value in pairs(t)"),
            ("while", "while ${1:condition} do\n\t$0\nend", "while … do … end"),
        ],
        "zig" => &[
            ("main", "const std = @import(\"std\");\n\npub fn main() !void {\n\t$0\n}", "pub fn main()"),
            ("fn", "fn ${1:name}(${2}) ${3:void} {\n\t$0\n}", "fn name() void { }"),
            ("print", "std.debug.print(\"${1}\\\\n\", .{${2}});$0", "std.debug.print()"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("for", "for (${1:items}) |${2:item}| {\n\t$0\n}", "for (items) |item| { }"),
            ("while", "while (${1:condition}) {\n\t$0\n}", "while (condition) { }"),
        ],
        "r" => &[
            ("function", "${1:name} <- function(${2}) {\n\t$0\n}", "name <- function() { }"),
            ("for", "for (${1:i} in ${2:1:10}) {\n\t$0\n}", "for (i in 1:10) { }"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("while", "while (${1:condition}) {\n\t$0\n}", "while (condition) { }"),
            ("df", "${1:df} <- data.frame(${2:x} = c($0))", "data.frame()"),
        ],
        "jl" => &[
            ("function", "function ${1:name}(${2})\n\t$0\nend", "function name() … end"),
            ("for", "for ${1:i} in ${2:1:10}\n\t$0\nend", "for i in 1:10 … end"),
            ("if", "if ${1:condition}\n\t$0\nend", "if … end"),
            ("while", "while ${1:condition}\n\t$0\nend", "while … end"),
            ("struct", "struct ${1:Name}\n\t$0\nend", "struct Name … end"),
            ("println", "println($0)", "println()"),
        ],
        "pl" | "pm" => &[
            ("sub", "sub ${1:name} {\n\tmy (${2}) = @_;\n\t$0\n}", "sub name { }"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("foreach", "foreach my ${1:\\$item} (${2:@items}) {\n\t$0\n}", "foreach my $item (@items) { }"),
            ("strict", "use strict;\nuse warnings;\n$0", "use strict; use warnings;"),
        ],
        "sh" | "bash" | "zsh" => &[
            ("bash", "#!/usr/bin/env bash\nset -euo pipefail\n\n$0", "#!/usr/bin/env bash"),
            ("if", "if [ ${1:condition} ]; then\n\t$0\nfi", "if [ … ]; then … fi"),
            ("for", "for ${1:item} in ${2:items}; do\n\t$0\ndone", "for item in items; do … done"),
            ("while", "while ${1:condition}; do\n\t$0\ndone", "while …; do … done"),
            ("function", "${1:name}() {\n\t$0\n}", "name() { }"),
            ("case", "case ${1:\\$value} in\n\t${2:pattern})\n\t\t$0\n\t\t;;\nesac", "case … esac"),
        ],
        "ps1" | "psm1" => &[
            ("function", "function ${1:Verb-Noun} {\n\tparam(${2})\n\t$0\n}", "function Verb-Noun { }"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("foreach", "foreach (${1:\\$item} in ${2:\\$items}) {\n\t$0\n}", "foreach ($item in $items) { }"),
            ("try", "try {\n\t$1\n} catch {\n\t$0\n}", "try { } catch { }"),
        ],
        "kt" | "kts" => &[
            ("main", "fun main() {\n\t$0\n}", "fun main()"),
            ("fun", "fun ${1:name}(${2}) {\n\t$0\n}", "fun name() { }"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("for", "for (${1:item} in ${2:items}) {\n\t$0\n}", "for (item in items) { }"),
            ("when", "when (${1:value}) {\n\t${2:1} -> $0\n\telse -> {}\n}", "when (value) { }"),
            ("class", "class ${1:Name}(${2}) {\n\t$0\n}", "class Name() { }"),
            ("println", "println($0)", "println()"),
        ],
        "swift" => &[
            ("func", "func ${1:name}(${2}) {\n\t$0\n}", "func name() { }"),
            ("if", "if ${1:condition} {\n\t$0\n}", "if condition { }"),
            ("for", "for ${1:item} in ${2:items} {\n\t$0\n}", "for item in items { }"),
            ("guard", "guard ${1:condition} else {\n\t${2:return}\n}\n$0", "guard … else { }"),
            ("struct", "struct ${1:Name} {\n\t$0\n}", "struct Name { }"),
            ("print", "print($0)", "print()"),
        ],
        "dart" => &[
            ("main", "void main() {\n\t$0\n}", "void main()"),
            ("if", "if (${1:condition}) {\n\t$0\n}", "if (condition) { }"),
            ("for", "for (var ${1:i} = 0; ${1:i} < ${2:n}; ${1:i}++) {\n\t$0\n}", "for (var i = 0; …)"),
            ("forin", "for (final ${1:item} in ${2:items}) {\n\t$0\n}", "for (final item in items)"),
            ("class", "class ${1:Name} {\n\t$0\n}", "class Name { }"),
            ("print", "print($0);", "print();"),
        ],
        "sql" => &[
            ("select", "SELECT ${1:*}\nFROM ${2:table}\nWHERE ${3:condition};$0", "SELECT … FROM … WHERE"),
            ("insert", "INSERT INTO ${1:table} (${2:columns})\nVALUES (${3:values});$0", "INSERT INTO … VALUES"),
            ("update", "UPDATE ${1:table}\nSET ${2:column} = ${3:value}\nWHERE ${4:condition};$0", "UPDATE … SET … WHERE"),
            ("create", "CREATE TABLE ${1:name} (\n\tid INTEGER PRIMARY KEY,\n\t$0\n);", "CREATE TABLE"),
            ("join", "JOIN ${1:table} ON ${2:a.id} = ${3:b.id}$0", "JOIN … ON"),
        ],
        _ => &[],
    }
}

// vstavané mená, ktoré sa volajú so zátvorkami
pub fn callable(lang: &str, w: &str) -> bool {
    match lang {
        "lua" => matches!(w, "print" | "pairs" | "ipairs" | "type" | "tostring" | "tonumber" | "require" | "insert" | "remove" | "concat" | "format" | "floor" | "random" | "setmetatable" | "error" | "pcall"),
        "r" => !matches!(w, "c") && w.chars().all(|c| c.is_ascii_lowercase() || c == '.' || c == '0'),
        "jl" => w.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "php" => !w.starts_with('$') && !matches!(w, "echo"),
        "kt" | "kts" => matches!(w, "println" | "print" | "readLine" | "listOf" | "mutableListOf" | "mapOf" | "mutableMapOf" | "setOf" | "toInt" | "toString"),
        "swift" => matches!(w, "print" | "readLine" | "append" | "remove" | "sorted" | "contains"),
        "dart" => matches!(w, "print" | "add" | "remove" | "toString" | "parse" | "readLineSync"),
        "zig" => w.starts_with('@') || w == "print",
        _ => false,
    }
}
