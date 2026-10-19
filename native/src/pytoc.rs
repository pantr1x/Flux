// Python → Arduino C++: malý prekladač pre programy na dosky (pyFirmata2, MicroPython, CircuitPython).
// Podporuje to, čo Flux sám generuje a bežné jednoduché programy; všetko ostatné zastaví s číslom riadku,
// aby sa na dosku nikdy nenahralo nič pokazené. Výsledok sa skompiluje cez arduino-cli ako obyčajný sketch.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(String),
    Id(String),
    Str(String),
    FStr(String),
    Op(String),
}

type R<T> = Result<T, String>;

fn is_op(t: &Tok, s: &str) -> bool {
    matches!(t, Tok::Op(o) if o == s)
}

fn lex(s: &str) -> R<Vec<Tok>> {
    let c: Vec<char> = s.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() || (ch == '.' && c.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let st = i;
            while i < c.len()
                && (c[i].is_ascii_alphanumeric()
                    || c[i] == '.'
                    || c[i] == '_'
                    || ((c[i] == '-' || c[i] == '+') && matches!(c[i - 1], 'e' | 'E') && !c[st..i].iter().collect::<String>().starts_with("0x")))
            {
                i += 1;
            }
            out.push(Tok::Num(c[st..i].iter().filter(|x| **x != '_').collect()));
        } else if ch.is_alphabetic() || ch == '_' {
            let st = i;
            loop {
                while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                    i += 1;
                }
                if i + 1 < c.len() && c[i] == '.' && (c[i + 1].is_alphabetic() || c[i + 1] == '_') {
                    i += 1;
                } else {
                    break;
                }
            }
            let word: String = c[st..i].iter().collect();
            if matches!(word.as_str(), "f" | "r" | "b" | "F" | "R" | "B") && matches!(c.get(i), Some('"') | Some('\'')) {
                let q = c[i];
                i += 1;
                let st2 = i;
                while i < c.len() && c[i] != q {
                    if c[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
                let body: String = c[st2..i.min(c.len())].iter().collect();
                i += 1;
                out.push(if word.eq_ignore_ascii_case("f") { Tok::FStr(body) } else { Tok::Str(body) });
            } else {
                out.push(Tok::Id(word));
            }
        } else if ch == '"' || ch == '\'' {
            let q = ch;
            i += 1;
            let mut body = String::new();
            while i < c.len() && c[i] != q {
                if c[i] == '\\' && i + 1 < c.len() {
                    body.push(c[i]);
                    i += 1;
                } else if c[i] == '"' {
                    body.push('\\');
                }
                body.push(c[i]);
                i += 1;
            }
            if i >= c.len() {
                return Err("a text is not closed".into());
            }
            i += 1;
            out.push(Tok::Str(body));
        } else {
            let three: String = c[i..(i + 3).min(c.len())].iter().collect();
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            if three == "//=" || three == "**=" {
                out.push(Tok::Op(three));
                i += 3;
            } else if ["//", "**", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=", "%=", "->", "<<", ">>"].contains(&two.as_str()) {
                out.push(Tok::Op(two));
                i += 2;
            } else if "+-*/%()[]{},:=<>&|^~.".contains(ch) {
                out.push(Tok::Op(ch.to_string()));
                i += 1;
            } else {
                return Err(format!("the character `{ch}` is not supported"));
            }
        }
    }
    Ok(out)
}

#[derive(Debug)]
struct Stmt {
    n: usize,
    text: String,
    body: Vec<Stmt>,
}

// odstráni komentár mimo textu
fn strip_comment(l: &str) -> String {
    let mut q: Option<char> = None;
    let mut prev = ' ';
    for (i, ch) in l.char_indices() {
        match q {
            Some(c) => {
                if ch == c && prev != '\\' {
                    q = None;
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    q = Some(ch);
                } else if ch == '#' {
                    return l[..i].to_string();
                }
            }
        }
        prev = ch;
    }
    l.to_string()
}

fn parse(src: &str) -> R<Vec<Stmt>> {
    // riadky; zátvorky cez viac riadkov sa spoja
    let mut lines: Vec<(usize, usize, String)> = vec![];
    let mut acc = String::new();
    let (mut start, mut depth) = (0usize, 0i32);
    for (i, raw) in src.lines().enumerate() {
        let l = strip_comment(raw).trim_end().to_string();
        if acc.is_empty() && l.trim().is_empty() {
            continue;
        }
        if acc.is_empty() {
            start = i + 1;
        }
        let part = if acc.is_empty() { l.clone() } else { format!(" {}", l.trim()) };
        for ch in l.chars() {
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
        }
        acc.push_str(&part);
        if depth <= 0 {
            let ind = acc.len() - acc.trim_start().len();
            if acc.contains('\t') && acc[..ind].contains('\t') {
                return Err(format!("line {start}: use spaces, not tabs, for indentation"));
            }
            lines.push((start, ind, acc.trim().to_string()));
            acc.clear();
            depth = 0;
        }
    }
    if !acc.is_empty() {
        return Err(format!("line {start}: a bracket is not closed"));
    }
    fn block(lines: &[(usize, usize, String)], pos: &mut usize, ind: usize) -> R<Vec<Stmt>> {
        let mut out = vec![];
        while *pos < lines.len() {
            let (n, i, t) = &lines[*pos];
            if *i < ind {
                break;
            }
            if *i > ind {
                return Err(format!("line {n}: unexpected indentation"));
            }
            *pos += 1;
            let mut st = Stmt { n: *n, text: t.clone(), body: vec![] };
            if t.ends_with(':') {
                if let Some((_, ni, _)) = lines.get(*pos) {
                    if *ni > ind {
                        st.body = block(lines, pos, *ni)?;
                    }
                }
            }
            out.push(st);
        }
        Ok(out)
    }
    let mut pos = 0;
    let first = lines.first().map(|l| l.1).unwrap_or(0);
    block(&lines, &mut pos, first)
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Out,
    In,
    Pull,
    Pwm,
    Tone,
    Servo,
    Analog,
}

#[derive(Clone, Debug)]
struct PinObj {
    pin: String,
    kind: Kind,
    firmata: bool, // pyFirmata2: analog read() = 0..1
}

struct Tr {
    pins: BTreeMap<String, PinObj>,
    vars: BTreeMap<String, bool>,             // globálne premenné: true = desatinné
    lists: BTreeMap<String, (bool, String)>,  // zoznamy: (desatinný, telo)
    funcs: BTreeMap<String, bool>,            // funkcie: vracia hodnotu
    locals: Vec<BTreeSet<String>>,            // názvy platné vo funkcii (parametre, lokálne, cykly)
    callbacks: Vec<(String, String, String)>, // (pin, premenná, telo)
    prints: bool,
    servos: bool,
}

// Python meno, ktoré by v C++ kolidovalo s kľúčovým slovom alebo funkciou Arduina
fn sn(n: &str) -> String {
    const BAD: [&str; 40] = [
        "double",
        "float",
        "int",
        "long",
        "short",
        "char",
        "unsigned",
        "signed",
        "new",
        "delete",
        "class",
        "struct",
        "enum",
        "union",
        "template",
        "this",
        "default",
        "switch",
        "case",
        "do",
        "goto",
        "register",
        "static",
        "const",
        "void",
        "bool",
        "auto",
        "namespace",
        "private",
        "public",
        "random",
        "map",
        "min",
        "max",
        "abs",
        "round",
        "delay",
        "millis",
        "setup",
        "loop",
    ];
    if BAD.contains(&n) {
        format!("py_{n}")
    } else {
        n.to_string()
    }
}

// odstráni zbytočné medzery okolo zátvoriek, čiarok a `!` (mimo textov)
fn tidy(s: &str) -> String {
    let mut out = String::new();
    let mut q = false;
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch == '"' && (i == 0 || c[i - 1] != '\\') {
            q = !q;
        }
        if !q && ch == ' ' {
            let prev = out.chars().last().unwrap_or(' ');
            let next = c.get(i + 1).copied().unwrap_or(' ');
            if matches!(prev, '(' | '[' | '!') || matches!(next, ')' | ']' | ',' | '[' | '(') && !prev.is_alphanumeric() && prev != ')' && prev != ']' && false {
                i += 1;
                continue;
            }
            if matches!(next, ')' | ']' | ',') {
                i += 1;
                continue;
            }
            if prev == '-' {
                let before = out[..out.len() - 1].trim_end().chars().last();
                if before.is_none_or(|b| !(b.is_alphanumeric() || matches!(b, ')' | ']' | '"' | '_'))) {
                    i += 1;
                    continue;
                }
            }
        }
        out.push(ch);
        i += 1;
    }
    out
}

fn split_args(t: &[Tok]) -> Vec<Vec<Tok>> {
    let mut out: Vec<Vec<Tok>> = vec![];
    let mut cur = vec![];
    let mut d = 0;
    for x in t {
        match x {
            Tok::Op(o) if o == "(" || o == "[" || o == "{" => {
                d += 1;
                cur.push(x.clone());
            }
            Tok::Op(o) if o == ")" || o == "]" || o == "}" => {
                d -= 1;
                cur.push(x.clone());
            }
            Tok::Op(o) if o == "," && d == 0 => out.push(std::mem::take(&mut cur)),
            _ => cur.push(x.clone()),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn matching(t: &[Tok], open: usize) -> Option<usize> {
    let mut d = 0;
    for (k, x) in t.iter().enumerate().skip(open) {
        if is_op(x, "(") || is_op(x, "[") {
            d += 1;
        } else if is_op(x, ")") || is_op(x, "]") {
            d -= 1;
            if d == 0 {
                return Some(k);
            }
        }
    }
    None
}

// číslo → pin: 13, board.D13, board.GP13, board.A0, A0
fn pin_of(arg: &[Tok], consts: &BTreeMap<String, String>) -> Option<String> {
    let a = match arg {
        [Tok::Num(n)] => n.clone(),
        [Tok::Id(i)] => {
            if let Some(b) = i.strip_prefix("board.") {
                let u = b.to_uppercase();
                if let Some(n) = u.strip_prefix("GP").or_else(|| u.strip_prefix('D')).filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
                    return Some(n.to_string());
                }
                return (u.starts_with('A') && u[1..].chars().all(|c| c.is_ascii_digit()) && u.len() > 1).then_some(u);
            }
            consts.get(i)?.clone()
        }
        [Tok::Str(s)] => s.clone(),
        _ => return None,
    };
    a.chars().all(|c| c.is_ascii_digit()).then(|| a.trim_start_matches('0').to_string()).map(|s| if s.is_empty() { "0".into() } else { s })
}

fn flat<'a>(stmts: &'a [Stmt], out: &mut Vec<&'a Stmt>) {
    for s in stmts {
        out.push(s);
        flat(&s.body, out);
    }
}

fn floaty(t: &[Tok], vars: &BTreeMap<String, bool>, tr_pins: &BTreeMap<String, PinObj>) -> bool {
    t.iter().any(|x| match x {
        Tok::Num(n) => !n.starts_with("0x") && (n.contains('.') || n.contains('e') || n.contains('E')),
        Tok::Op(o) => o == "/",
        Tok::Id(i) => {
            matches!(i.as_str(), "float" | "random.random" | "time.monotonic" | "monotonic" | "math.sqrt" | "sqrt" | "math.sin" | "math.cos" | "math.pi" | "pow")
                || vars.get(i).copied().unwrap_or(false)
                || i.rsplit_once('.').is_some_and(|(o, m)| m == "read" && tr_pins.get(o).is_some_and(|p| p.firmata && p.kind == Kind::Analog))
        }
        _ => false,
    })
}

impl Tr {
    fn pin_ref(&self, name: &str) -> Option<&PinObj> {
        self.pins.get(name)
    }

    fn known(&self, n: &str) -> bool {
        self.vars.contains_key(n) || self.lists.contains_key(n) || self.locals.last().is_some_and(|l| l.contains(n))
    }

    // pin volaný výrazom: čítanie
    fn read(&self, p: &PinObj, method: &str) -> R<String> {
        let pn = &p.pin;
        match (p.kind, method) {
            (Kind::Analog, "read") if p.firmata => Ok(format!("(analogRead({pn}) / 1023.0)")),
            (Kind::Analog, "read") => Ok(format!("analogRead({pn})")),
            (Kind::Analog, "read_u16" | "value") => Ok(format!("(analogRead({pn}) * 64L)")),
            (Kind::Out | Kind::In | Kind::Pull, "read" | "value") => Ok(format!("digitalRead({pn})")),
            _ => Err(format!("`{method}` can't be read from this pin")),
        }
    }

    fn name(&self, n: &str) -> R<String> {
        Ok(match n {
            "True" => "true".into(),
            "False" => "false".into(),
            "None" => "0".into(),
            "and" => "&&".into(),
            "or" => "||".into(),
            "not" => "!".into(),
            "math.pi" => "PI".into(),
            _ => {
                if let Some((o, m)) = n.rsplit_once('.') {
                    if let Some(p) = self.pin_ref(o) {
                        return self.read(p, m);
                    }
                    return Err(format!("`{n}` is not supported"));
                }
                if self.known(n) {
                    sn(n)
                } else if self.pins.contains_key(n) {
                    return Err(format!("`{n}` is a pin – use it like {n}.value()"));
                } else if matches!(n, "HIGH" | "LOW") {
                    n.to_string()
                } else {
                    return Err(format!("unknown name `{n}`"));
                }
            }
        })
    }

    fn call(&mut self, name: &str, args: &[Vec<Tok>]) -> R<String> {
        let a: Vec<String> = args.iter().map(|x| self.expr(x)).collect::<R<_>>()?;
        let arg = |i: usize| a.get(i).cloned().ok_or_else(|| format!("`{name}` needs more arguments"));
        if let Some((o, m)) = name.rsplit_once('.') {
            if let Some(p) = self.pin_ref(o).cloned() {
                return self.read(&p, m);
            }
        }
        Ok(match name {
            "abs" | "min" | "max" | "round" | "map" | "constrain" | "sqrt" | "pow" => format!("{name}({})", a.join(", ")),
            "math.sqrt" => format!("sqrt({})", arg(0)?),
            "math.sin" | "math.cos" | "math.tan" => format!("{}({})", &name[5..], arg(0)?),
            "int" => format!("((long)({}))", arg(0)?),
            "float" => format!("((float)({}))", arg(0)?),
            "bool" => format!("(({}) != 0)", arg(0)?),
            "len" => {
                let n = args.first().and_then(|t| if let [Tok::Id(i)] = t.as_slice() { Some(i.clone()) } else { None }).ok_or("len() works only on a list name")?;
                if !self.lists.contains_key(&n) {
                    return Err(format!("`{n}` is not a list"));
                }
                format!("((long)(sizeof({n}) / sizeof({n}[0])))")
            }
            "random.randint" | "randint" => format!("random({}, ({}) + 1)", arg(0)?, arg(1)?),
            "random.randrange" | "randrange" => format!("random({}, {})", arg(0)?, arg(1)?),
            "random.random" => "(random(10000) / 10000.0)".into(),
            "time.ticks_ms" | "ticks_ms" | "utime.ticks_ms" => "millis()".into(),
            "time.ticks_diff" | "ticks_diff" | "utime.ticks_diff" => format!("(({}) - ({}))", arg(0)?, arg(1)?),
            "time.monotonic" | "monotonic" => "(millis() / 1000.0)".into(),
            _ if self.funcs.contains_key(name) => format!("{}({})", sn(name), a.join(", ")),
            _ => return Err(format!("`{name}()` is not supported")),
        })
    }

    fn expr(&mut self, t: &[Tok]) -> R<String> {
        let mut out: Vec<String> = vec![];
        let mut i = 0;
        while i < t.len() {
            match &t[i] {
                Tok::Num(n) => out.push(n.clone()),
                Tok::Str(s) => out.push(format!("\"{s}\"")),
                Tok::FStr(_) => return Err("an f-string can only be used inside print()".into()),
                Tok::Op(o) => match o.as_str() {
                    "//" => out.push("/".into()),
                    "/" => out.push("/ (float)".into()),
                    "**" => return Err("`**` is not supported – use pow(a, b)".into()),
                    "." => return Err("this `.` is not supported".into()),
                    "{" | "}" | ":" | "->" => return Err(format!("`{o}` is not supported here")),
                    _ => out.push(o.clone()),
                },
                Tok::Id(n) => {
                    if matches!(n.as_str(), "if" | "else" | "lambda" | "in" | "is" | "yield") {
                        return Err(format!("`{n}` inside an expression is not supported – use a normal if block"));
                    }
                    if t.get(i + 1).is_some_and(|x| is_op(x, "(")) {
                        let close = matching(t, i + 1).ok_or("a bracket is not closed")?;
                        let args = split_args(&t[i + 2..close]);
                        let r = self.call(n, &args)?;
                        out.push(r);
                        i = close + 1;
                        continue;
                    }
                    out.push(self.name(n)?);
                }
            }
            i += 1;
        }
        Ok(tidy(&out.join(" ")))
    }

    fn print(&mut self, args: &[Vec<Tok>], ind: &str) -> R<Vec<String>> {
        self.prints = true;
        let mut out = vec![];
        let mut parts: Vec<String> = vec![];
        for (k, a) in args.iter().enumerate() {
            if k > 0 {
                parts.push("Serial.print(\" \");".into());
            }
            match a.as_slice() {
                [Tok::FStr(f)] => {
                    let c: Vec<char> = f.chars().collect();
                    let (mut i, mut lit) = (0, String::new());
                    while i < c.len() {
                        if c[i] == '{' && c.get(i + 1) == Some(&'{') {
                            lit.push('{');
                            i += 2;
                        } else if c[i] == '}' && c.get(i + 1) == Some(&'}') {
                            lit.push('}');
                            i += 2;
                        } else if c[i] == '{' {
                            let e = c[i + 1..].iter().position(|x| *x == '}').ok_or("a { in the f-string is not closed")?;
                            let inner: String = c[i + 1..i + 1 + e].iter().collect();
                            if inner.contains(':') || inner.contains('!') {
                                return Err("formats inside an f-string are not supported".into());
                            }
                            if !lit.is_empty() {
                                parts.push(format!("Serial.print(\"{}\");", std::mem::take(&mut lit).replace('"', "\\\"")));
                            }
                            let toks = lex(&inner)?;
                            parts.push(format!("Serial.print({});", self.expr(&toks)?));
                            i += e + 2;
                        } else {
                            lit.push(c[i]);
                            i += 1;
                        }
                    }
                    if !lit.is_empty() {
                        parts.push(format!("Serial.print(\"{}\");", lit.replace('"', "\\\"")));
                    }
                }
                [Tok::Str(s)] => parts.push(format!("Serial.print(\"{s}\");")),
                _ => parts.push(format!("Serial.print({});", self.expr(a)?)),
            }
        }
        parts.push("Serial.println();".into());
        for p in parts {
            out.push(format!("{ind}{p}"));
        }
        Ok(out)
    }

    // príkaz s pinom: X.metóda(args) alebo X.atribút = hodnota
    fn pin_stmt(&mut self, t: &[Tok], ind: &str) -> R<Option<Vec<String>>> {
        let Some(Tok::Id(full)) = t.first() else { return Ok(None) };
        let Some((o, m)) = full.rsplit_once('.') else { return Ok(None) };
        let Some(p) = self.pin_ref(o).cloned() else { return Ok(None) };
        let pn = p.pin.clone();
        if (is_read_only(t, m))
            || (matches!(m, "direction" | "pull" | "mode") && t.get(1).is_some_and(|x| is_op(x, "=")))
            || matches!(m, "register_callback" | "enable_reporting" | "disable_reporting")
        {
            return Ok(Some(vec![]));
        }
        let (val, is_call) = if t.get(1).is_some_and(|x| is_op(x, "(")) {
            let close = matching(t, 1).ok_or("a bracket is not closed")?;
            if close + 1 != t.len() {
                return Err("only one pin command per line is supported".into());
            }
            let args = split_args(&t[2..close]);
            let first = args.first().map(|a| self.expr(a)).transpose()?;
            (first, true)
        } else if t.get(1).is_some_and(|x| is_op(x, "=")) {
            (Some(self.expr(&t[2..])?), false)
        } else {
            return Ok(None);
        };
        let v = || val.clone().ok_or_else(|| format!("`{m}` needs a value"));
        let s = match (p.kind, m, is_call) {
            (Kind::Out, "on", true) => format!("digitalWrite({pn}, HIGH);"),
            (Kind::Out, "off", true) => format!("digitalWrite({pn}, LOW);"),
            (Kind::Out, "toggle", true) => format!("digitalWrite({pn}, !digitalRead({pn}));"),
            (Kind::Out, "value" | "write", _) => format!("digitalWrite({pn}, ({}) ? HIGH : LOW);", v()?),
            (Kind::Pwm, "write", true) => format!("analogWrite({pn}, (int)(({}) * 255));", v()?),
            (Kind::Pwm, "duty_u16", true) | (Kind::Pwm, "duty_cycle", false) => format!("analogWrite({pn}, (int)(({}) / 257L));", v()?),
            (Kind::Pwm, "duty", true) => format!("analogWrite({pn}, (int)(({}) / 4));", v()?),
            (Kind::Pwm, "freq" | "frequency", _) => String::new(),
            (Kind::Tone, "freq", true) | (Kind::Tone, "frequency", false) => format!("tone_{pn} = (int)({});", v()?),
            (Kind::Tone, "duty_u16", true) | (Kind::Tone, "duty_cycle", false) | (Kind::Tone, "write", true) => {
                let x = v()?;
                format!("if (({x}) > 0) tone({pn}, tone_{pn}); else noTone({pn});")
            }
            (Kind::Servo, "write", true) => format!("servo_{pn}.write((int)({}));", v()?),
            (Kind::Servo, "duty_u16", true) | (Kind::Servo, "duty_cycle", false) => format!("servo_{pn}.write(constrain(map((long)({}), 1638, 8192, 0, 180), 0, 180));", v()?),
            (_, "enable_reporting" | "disable_reporting" | "direction" | "pull" | "mode", _) => String::new(),
            _ => return Err(format!("`{m}` is not supported on this pin")),
        };
        Ok(Some(if s.is_empty() { vec![] } else { vec![format!("{ind}{s}")] }))
    }
}

// `X.read()` ako samostatný príkaz nič nerobí
fn is_read_only(t: &[Tok], m: &str) -> bool {
    matches!(m, "read" | "value" | "read_u16") && t.len() == 3 && t.get(1).is_some_and(|x| is_op(x, "(")) && t.get(2).is_some_and(|x| is_op(x, ")"))
}

struct Ctx<'a> {
    tr: &'a mut Tr,
    in_def: bool,
}

fn emit(cx: &mut Ctx, stmts: &[Stmt], ind: usize, out: &mut Vec<String>) -> R<()> {
    let sp = "  ".repeat(ind);
    for (idx, st) in stmts.iter().enumerate() {
        let err = |e: String| format!("line {}: {e}", st.n);
        let text = st.text.trim();
        let toks = lex(text).map_err(err)?;
        let first = match toks.first() {
            Some(Tok::Id(i)) => i.as_str(),
            _ => "",
        };
        match first {
            "import" | "from" | "pass" | "global" | "nonlocal" => continue,
            "board" if toks.get(1).is_some_and(|x| is_op(x, "=")) => continue,
            "if" | "while" => {
                let cond = cx.tr.expr(&toks[1..toks.len() - 1]).map_err(err)?;
                let cond = if first == "while" && (cond == "true" || cond == "1") { "true".to_string() } else { cond };
                out.push(format!("{sp}{first} ({cond}) {{"));
                emit(cx, &st.body, ind + 1, out)?;
                out.push(format!("{sp}}}"));
            }
            "elif" => {
                let cond = cx.tr.expr(&toks[1..toks.len() - 1]).map_err(err)?;
                out.push(format!("{sp}else if ({cond}) {{"));
                emit(cx, &st.body, ind + 1, out)?;
                out.push(format!("{sp}}}"));
            }
            "else" => {
                out.push(format!("{sp}else {{"));
                emit(cx, &st.body, ind + 1, out)?;
                out.push(format!("{sp}}}"));
            }
            "for" => {
                // for i in range(a, b, c):
                let (Some(Tok::Id(v)), Some(Tok::Id(inn)), Some(Tok::Id(r))) = (toks.get(1), toks.get(2), toks.get(3)) else { return Err(err("only `for i in range(...)` is supported".into())) };
                if inn != "in" || r != "range" || !toks.get(4).is_some_and(|x| is_op(x, "(")) {
                    return Err(err("only `for i in range(...)` is supported".into()));
                }
                let close = matching(&toks, 4).ok_or_else(|| err("a bracket is not closed".into()))?;
                let args = split_args(&toks[5..close]);
                let a: Vec<String> = args.iter().map(|x| cx.tr.expr(x)).collect::<R<_>>().map_err(err)?;
                let (start, end, step) = match a.len() {
                    1 => ("0".to_string(), a[0].clone(), "1".to_string()),
                    2 => (a[0].clone(), a[1].clone(), "1".to_string()),
                    3 => (a[0].clone(), a[1].clone(), a[2].clone()),
                    _ => return Err(err("range() needs 1 to 3 numbers".into())),
                };
                let neg = step.trim_start().starts_with('-');
                cx.tr.vars.remove(v);
                cx.tr.locals.last_mut().map(|l| l.insert(v.clone()));
                {
                    let sv = sn(v);
                    out.push(format!("{sp}for (long {sv} = {start}; {sv} {} {end}; {sv} += {step}) {{", if neg { ">" } else { "<" }));
                }
                emit(cx, &st.body, ind + 1, out)?;
                out.push(format!("{sp}}}"));
            }
            "break" | "continue" => out.push(format!("{sp}{first};")),
            "return" => {
                if toks.len() == 1 {
                    out.push(format!("{sp}return;"));
                } else {
                    let e = cx.tr.expr(&toks[1..]).map_err(err)?;
                    out.push(format!("{sp}return {e};"));
                }
            }
            "def" | "class" | "try" | "except" | "finally" | "with" | "lambda" | "async" | "await" | "assert" | "raise" | "del" => {
                return Err(err(format!("`{first}` can't be used here")));
            }
            _ => {
                if text.ends_with(':') {
                    return Err(err("this block is not supported".into()));
                }
                let ind_s = sp.clone();
                // pin: X.metóda(...) / X.atribút = ...
                if let Some(lines) = cx.tr.pin_stmt(&toks, &ind_s).map_err(err)? {
                    out.extend(lines);
                    continue;
                }
                // volania: print, sleep…
                if let (Some(Tok::Id(f)), true) = (toks.first(), toks.get(1).is_some_and(|x| is_op(x, "("))) {
                    if let Some(close) = matching(&toks, 1) {
                        if close + 1 == toks.len() {
                            let args = split_args(&toks[2..close]);
                            match f.as_str() {
                                "print" => {
                                    let l = cx.tr.print(&args, &ind_s).map_err(err)?;
                                    out.extend(l);
                                    continue;
                                }
                                "sleep" | "time.sleep" | "sleep_ms" | "time.sleep_ms" | "sleep_us" | "time.sleep_us" | "delay" => {
                                    let e = args.first().map(|a| cx.tr.expr(a)).transpose().map_err(err)?.ok_or_else(|| err("sleep() needs a time".into()))?;
                                    let lit = if let Some([Tok::Num(n)]) = args.first().map(|a| a.as_slice()) { n.parse::<f64>().ok() } else { None };
                                    let line = if f.ends_with("_us") {
                                        format!("{sp}delayMicroseconds({e});")
                                    } else if f.ends_with("_ms") || f == "delay" {
                                        format!("{sp}delay({e});")
                                    } else if let Some(x) = lit {
                                        format!("{sp}delay({});", (x * 1000.0).round() as i64)
                                    } else {
                                        format!("{sp}delay((unsigned long)(({e}) * 1000.0));")
                                    };
                                    out.push(line);
                                    continue;
                                }
                                _ if f.ends_with(".enable_reporting") || f.ends_with(".disable_reporting") || f == "board.samplingOn" || f == "board.exit" || f.ends_with(".register_callback") => {
                                    continue
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // priradenie: a = …, a += …, a[i] = …
                let mut d = 0;
                let eq = toks.iter().position(|x| {
                    if is_op(x, "(") || is_op(x, "[") {
                        d += 1;
                    } else if is_op(x, ")") || is_op(x, "]") {
                        d -= 1;
                    }
                    d == 0 && matches!(x, Tok::Op(o) if ["=", "+=", "-=", "*=", "/=", "%=", "//="].contains(&o.as_str()))
                });
                if let Some(e) = eq {
                    let Tok::Op(op) = &toks[e] else { unreachable!() };
                    if let (Some(Tok::Id(n)), Some(Tok::Op(b))) = (toks.first(), toks.get(e + 1)) {
                        if b == "[" && cx.tr.lists.contains_key(n) {
                            continue;
                        }
                    }
                    let lhs = cx.tr.expr(&toks[..e]).map_err(err)?;
                    let rhs = cx.tr.expr(&toks[e + 1..]).map_err(err)?;
                    let line = match op.as_str() {
                        "/=" => format!("{sp}{lhs} /= (float)({rhs});"),
                        "//=" => format!("{sp}{lhs} /= {rhs};"),
                        o => format!("{sp}{lhs} {o} {rhs};"),
                    };
                    out.push(line);
                    continue;
                }
                let e = cx.tr.expr(&toks).map_err(err)?;
                out.push(format!("{sp}{e};"));
            }
        }
        let _ = idx;
    }
    let _ = cx.in_def;
    Ok(())
}

// jednoduché priradenie `NAME = <hodnota>`: (meno, tokeny hodnoty)
fn assign_of(t: &[Tok]) -> Option<(String, Vec<Tok>, String)> {
    let Tok::Id(n) = t.first()? else { return None };
    let Tok::Op(o) = t.get(1)? else { return None };
    if n.contains('.') || !["=", "+=", "-=", "*=", "/=", "%=", "//="].contains(&o.as_str()) {
        return None;
    }
    Some((n.clone(), t[2..].to_vec(), o.clone()))
}

pub fn translate(src: &str, file: &str) -> Result<String, String> {
    // board.digital[N] / board.analog[N] → vlastné piny
    let mut text = src.to_string();
    for (kw, pre) in [("board.digital[", "__d"), ("board.analog[", "__a")] {
        while let Some(k) = text.find(kw) {
            let rest = &text[k + kw.len()..];
            let Some(e) = rest.find(']') else { break };
            let n = rest[..e].trim().to_string();
            text.replace_range(k..k + kw.len() + e + 1, &format!("{pre}{n}"));
        }
    }
    let prog = parse(&text)?;
    let mut all: Vec<&Stmt> = vec![];
    flat(&prog, &mut all);
    let mut tr = Tr { pins: BTreeMap::new(), vars: BTreeMap::new(), lists: BTreeMap::new(), funcs: BTreeMap::new(), locals: vec![BTreeSet::new()], callbacks: vec![], prints: false, servos: false };
    // konštanty: NAME = 13
    let mut consts: BTreeMap<String, String> = BTreeMap::new();
    for s in &prog {
        if let Ok(t) = lex(&s.text) {
            if let Some((n, v, op)) = assign_of(&t) {
                if op == "=" {
                    if let [Tok::Num(x)] = v.as_slice() {
                        consts.insert(n, x.clone());
                    }
                }
            }
        }
    }
    // 1. piny
    let mut pin_stmts: BTreeSet<usize> = BTreeSet::new();
    let uses = |name: &str, what: &str| all.iter().any(|s| s.text.contains(&format!("{name}.{what}")));
    for s in &all {
        let Ok(t) = lex(&s.text) else { continue };
        let Some((n, v, op)) = assign_of(&t) else { continue };
        if op != "=" {
            continue;
        }
        let Some(Tok::Id(ctor)) = v.first() else { continue };
        let args = v.get(1).filter(|x| is_op(x, "(")).and_then(|_| matching(&v, 1)).map(|c| split_args(&v[2..c])).unwrap_or_default();
        let ln = s.n;
        let mut def: Option<PinObj> = None;
        if ctor == "board.get_pin" {
            if let Some([Tok::Str(spec)]) = args.first().map(|a| a.as_slice()) {
                let f: Vec<&str> = spec.split(':').collect();
                if f.len() >= 3 {
                    let (kind, pin) = match (f[0], f[2]) {
                        ("a", _) => (Kind::Analog, format!("A{}", f[1])),
                        (_, "o") => (Kind::Out, f[1].to_string()),
                        (_, "p") => (Kind::Pwm, f[1].to_string()),
                        (_, "s") => (Kind::Servo, f[1].to_string()),
                        _ => (Kind::In, f[1].to_string()),
                    };
                    def = Some(PinObj { pin, kind, firmata: true });
                }
            }
        } else if matches!(ctor.as_str(), "Pin" | "machine.Pin") {
            if let Some(p) = args.first().and_then(|a| pin_of(a, &consts)) {
                let mode = args.get(1).map(|a| a.iter().map(|x| if let Tok::Id(i) = x { i.clone() } else { String::new() }).collect::<String>()).unwrap_or_default();
                let pull = args.iter().skip(2).any(|a| a.iter().any(|x| matches!(x, Tok::Id(i) if i.ends_with("PULL_UP"))));
                let kind = if mode.ends_with("OUT") {
                    Kind::Out
                } else if pull {
                    Kind::Pull
                } else {
                    Kind::In
                };
                def = Some(PinObj { pin: p, kind, firmata: false });
            }
        } else if matches!(ctor.as_str(), "PWM" | "machine.PWM" | "pwmio.PWMOut") {
            let inner: Vec<Tok> = match args.first() {
                Some(a) if matches!(a.first(), Some(Tok::Id(i)) if i == "Pin" || i == "machine.Pin") => {
                    let c = matching(a, 1).unwrap_or(a.len() - 1);
                    a[2..c].to_vec()
                }
                Some(a) => a.clone(),
                None => vec![],
            };
            if let Some(p) = pin_of(&inner, &consts) {
                let f50 = args.iter().any(|a| a.len() >= 3 && matches!(&a[0], Tok::Id(i) if i == "freq" || i == "frequency") && matches!(&a[2], Tok::Num(x) if x == "50"));
                let kind = if f50 {
                    Kind::Servo
                } else if uses(&n, "freq") || uses(&n, "frequency") {
                    Kind::Tone
                } else {
                    Kind::Pwm
                };
                def = Some(PinObj { pin: p, kind, firmata: false });
            }
        } else if matches!(ctor.as_str(), "ADC" | "machine.ADC" | "analogio.AnalogIn") {
            let inner: Vec<Tok> = match args.first() {
                Some(a) if matches!(a.first(), Some(Tok::Id(i)) if i == "Pin" || i == "machine.Pin") => {
                    let c = matching(a, 1).unwrap_or(a.len() - 1);
                    a[2..c].to_vec()
                }
                Some(a) => a.clone(),
                None => vec![],
            };
            if let Some(p) = pin_of(&inner, &consts) {
                let pin = if p.starts_with('A') { p } else { format!("A{p}") };
                def = Some(PinObj { pin, kind: Kind::Analog, firmata: false });
            }
        } else if matches!(ctor.as_str(), "digitalio.DigitalInOut") {
            if let Some(p) = args.first().and_then(|a| pin_of(a, &consts)) {
                let input = uses(&n, "direction") && all.iter().any(|x| x.text.contains(&format!("{n}.direction")) && x.text.contains("INPUT"));
                let pull = all.iter().any(|x| x.text.contains(&format!("{n}.pull")) && x.text.contains("UP"));
                let kind = if !input {
                    Kind::Out
                } else if pull {
                    Kind::Pull
                } else {
                    Kind::In
                };
                def = Some(PinObj { pin: p, kind, firmata: false });
            }
        }
        if let Some(d) = def {
            if d.kind == Kind::Servo {
                tr.servos = true;
            }
            tr.pins.insert(n.clone(), d);
            pin_stmts.insert(ln);
        } else if tr.pins.contains_key(&n) {
            return Err(format!("line {ln}: `{n}` can't be changed after it became a pin"));
        } else if ["Pin", "PWM", "ADC", "board.get_pin", "digitalio.DigitalInOut", "pwmio.PWMOut", "analogio.AnalogIn", "Arduino"].contains(&ctor.as_str()) && ctor != "Arduino" {
            return Err(format!("line {ln}: this pin definition is not supported (use a number or board.D13)"));
        }
    }
    // board.digital[N] / board.analog[N] (nahradené názvami __dN / __aN)
    for s in &all {
        let Ok(t) = lex(&s.text) else { continue };
        for x in &t {
            if let Tok::Id(i) = x {
                let Some((o, _)) = i.rsplit_once('.') else { continue };
                if let Some(n) = o.strip_prefix("__d").filter(|n| n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty()) {
                    let kind = if uses(o, "write") { Kind::Out } else { Kind::In };
                    tr.pins.entry(o.to_string()).or_insert(PinObj { pin: n.to_string(), kind, firmata: true });
                } else if let Some(n) = o.strip_prefix("__a").filter(|n| n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty()) {
                    tr.pins.entry(o.to_string()).or_insert(PinObj { pin: format!("A{n}"), kind: Kind::Analog, firmata: true });
                }
            }
        }
        // callbacky: X.register_callback(lambda v: BODY)
        if let Some(k) = s.text.find(".register_callback(") {
            let obj = s.text[..k].trim().to_string();
            let rest = &s.text[k + 19..];
            if let Some(l) = rest.trim_start().strip_prefix("lambda ") {
                if let Some((var, body)) = l.split_once(':') {
                    let body = body.trim_end().strip_suffix(')').unwrap_or(body.trim_end()).trim().to_string();
                    tr.callbacks.push((obj, var.trim().to_string(), body));
                    continue;
                }
            }
            return Err(format!("line {}: only `register_callback(lambda v: ...)` is supported", s.n));
        }
    }
    // 2. funkcie
    for s in &prog {
        if let Some(rest) = s.text.strip_prefix("def ") {
            let name = rest.split('(').next().unwrap_or("").trim().to_string();
            let mut fl: Vec<&Stmt> = vec![];
            flat(&s.body, &mut fl);
            let ret = fl.iter().any(|b| b.text.starts_with("return ") && b.text.trim() != "return");
            tr.funcs.insert(name, ret);
        }
    }
    // 3. premenné a zoznamy (mimo funkcií)
    let mut assigns: Vec<(String, Vec<Tok>, String)> = vec![];
    let mut top: Vec<&Stmt> = vec![];
    for s in &prog {
        if !s.text.starts_with("def ") {
            top.push(s);
            flat(&s.body, &mut top);
        }
    }
    top.sort_by_key(|s| s.n);
    top.dedup_by_key(|s| s.n);
    for s in &top {
        if pin_stmts.contains(&s.n) {
            continue;
        }
        let Ok(t) = lex(&s.text) else { continue };
        if let Some((n, v, op)) = assign_of(&t) {
            if matches!(v.first(), Some(Tok::Op(o)) if o == "[") && op == "=" && s.n > 0 && prog.iter().any(|p| p.n == s.n) {
                let c = matching(&v, 0).ok_or(format!("line {}: a bracket is not closed", s.n))?;
                let items = split_args(&v[1..c]);
                let fl = items.iter().any(|i| floaty(i, &tr.vars, &tr.pins));
                let mut body = vec![];
                for it in &items {
                    body.push(match it.as_slice() {
                        [Tok::Num(x)] => x.clone(),
                        [Tok::Op(m), Tok::Num(x)] if m == "-" => format!("-{x}"),
                        _ => return Err(format!("line {}: a list can only hold numbers", s.n)),
                    });
                }
                tr.lists.insert(n, (fl, body.join(", ")));
                continue;
            }
            if tr.pins.contains_key(&n) || n == "board" {
                continue;
            }
            assigns.push((n, v, op));
        }
    }
    for _ in 0..3 {
        for (n, v, op) in &assigns {
            let f = op == "/=" || floaty(v, &tr.vars, &tr.pins);
            let e = tr.vars.entry(n.clone()).or_insert(false);
            *e = *e || f;
        }
    }
    // for-premenné nie sú globálne
    for s in all.iter() {
        if let Some(rest) = s.text.strip_prefix("for ") {
            if let Some(v) = rest.split_whitespace().next() {
                if !assigns.iter().any(|a| a.0 == v) {
                    tr.vars.remove(v);
                }
            }
        }
    }
    // 4. hlavná slučka: posledné `while True:` na najvyššej úrovni
    let main_idx = prog.iter().rposition(|s| matches!(s.text.trim(), "while True:" | "while 1:" | "while true:"));
    if let Some(m) = main_idx {
        if prog[m + 1..].iter().any(|s| !s.text.starts_with("def ")) {
            return Err(format!("line {}: code after the main `while True:` loop never runs", prog[m + 1..].iter().find(|s| !s.text.starts_with("def ")).map(|s| s.n).unwrap_or(0)));
        }
    }
    // 5. funkcie
    let mut fn_out: Vec<String> = vec![];
    let mut protos: Vec<String> = vec![];
    for s in &prog {
        let Some(rest) = s.text.strip_prefix("def ") else { continue };
        let rest = rest.trim_end_matches(':');
        let (name, params) = rest.split_once('(').ok_or(format!("line {}: bad function", s.n))?;
        let params: Vec<String> = params.trim_end_matches(')').split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
        if params.iter().any(|p| !p.chars().all(|c| c.is_alphanumeric() || c == '_')) {
            return Err(format!("line {}: function arguments can't have defaults or types", s.n));
        }
        let name = name.trim();
        let ret = tr.funcs.get(name).copied().unwrap_or(false);
        let sig = format!("{} {}({})", if ret { "long" } else { "void" }, sn(name), params.iter().map(|p| format!("long {}", sn(p))).collect::<Vec<_>>().join(", "));
        protos.push(format!("{sig};"));
        // lokálne premenné funkcie
        let mut fl: Vec<&Stmt> = vec![];
        flat(&s.body, &mut fl);
        let globals: BTreeSet<String> = fl.iter().filter_map(|b| b.text.strip_prefix("global ")).flat_map(|g| g.split(',').map(|x| x.trim().to_string())).collect();
        let mut loc: BTreeSet<String> = params.iter().cloned().collect();
        let mut hoist: Vec<(String, bool)> = vec![];
        for b in &fl {
            if let Ok(t) = lex(&b.text) {
                if let Some((n, v, op)) = assign_of(&t) {
                    if !loc.contains(&n) && !globals.contains(&n) {
                        let f = op == "/=" || floaty(&v, &tr.vars, &tr.pins);
                        loc.insert(n.clone());
                        hoist.push((n, f));
                    } else if let Some(h) = hoist.iter_mut().find(|h| h.0 == n) {
                        h.1 = h.1 || floaty(&v, &tr.vars, &tr.pins);
                    }
                }
            }
            if let Some(r) = b.text.strip_prefix("for ") {
                if let Some(v) = r.split_whitespace().next() {
                    loc.insert(v.to_string());
                }
            }
        }
        tr.locals.push(loc);
        let mut body = vec![];
        {
            let mut cx = Ctx { tr: &mut tr, in_def: true };
            emit(&mut cx, &s.body, 1, &mut body)?;
        }
        tr.locals.pop();
        fn_out.push(format!("{sig} {{"));
        for (n, f) in hoist {
            fn_out.push(format!("  {} {} = 0;", if f { "float" } else { "long" }, sn(&n)));
        }
        fn_out.extend(body);
        fn_out.push("}".into());
        fn_out.push(String::new());
    }
    // 6. setup a loop
    let mut setup: Vec<String> = vec![];
    let mut lp: Vec<String> = vec![];
    let setup_stmts: Vec<Stmt> = vec![];
    let _ = setup_stmts;
    {
        let mut cx = Ctx { tr: &mut tr, in_def: false };
        let upto = main_idx.unwrap_or(prog.len());
        let pre: Vec<&Stmt> = prog[..upto].iter().filter(|s| !s.text.starts_with("def ") && !pin_stmts.contains(&s.n)).collect();
        for s in pre {
            let one = std::slice::from_ref(s);
            // elif / else sa musí zhodovať s predchádzajúcim if: emit() ich spracuje po jednom
            emit(&mut cx, one, 1, &mut setup)?;
        }
        if let Some(m) = main_idx {
            emit(&mut cx, &prog[m].body, 1, &mut lp)?;
        }
    }
    // callbacky tlačidiel a senzorov: v loop() pri zmene hodnoty
    let mut cb: Vec<String> = vec![];
    for (obj, var, body) in std::mem::take(&mut tr.callbacks) {
        let Some(p) = tr.pins.get(&obj).cloned() else { return Err(format!("`{obj}` is not a pin")) };
        let read = if p.kind == Kind::Analog { format!("analogRead({})", p.pin) } else { format!("digitalRead({})", p.pin) };
        let id = obj.replace('.', "_");
        tr.locals.push([var.clone()].into_iter().collect());
        let toks = lex(&body).map_err(|e| format!("in the callback: {e}"))?;
        let st = Stmt { n: 0, text: body.clone(), body: vec![] };
        let _ = toks;
        let mut inner = vec![];
        {
            let mut cx = Ctx { tr: &mut tr, in_def: false };
            emit(&mut cx, std::slice::from_ref(&st), 3, &mut inner).map_err(|e| format!("in the callback: {}", e.trim_start_matches("line 0: ")))?;
        }
        tr.locals.pop();
        cb.push(format!("  {{\n    static long last_{id} = -1;\n    long {var} = {read};\n    if ({var} != last_{id}) {{\n      last_{id} = {var};"));
        cb.extend(inner.into_iter().map(|l| l.replacen("      ", "      ", 1)));
        cb.push("    }\n  }".into());
    }
    // výstup
    let mut o: Vec<String> = vec![format!("// Generated by Flux from {file} – change the Python file, not this one."), String::new()];
    if tr.servos {
        o.push("#include <Servo.h>".into());
        o.push(String::new());
    }
    for (n, (f, b)) in &tr.lists {
        o.push(format!("{} {}[] = {{{b}}};", if *f { "float" } else { "long" }, sn(n)));
    }
    for (n, f) in &tr.vars {
        if !tr.lists.contains_key(n) {
            let init = consts.get(n).cloned().unwrap_or_else(|| "0".into());
            o.push(format!("{} {} = {init};", if *f { "float" } else { "long" }, sn(n)));
        }
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for p in tr.pins.values() {
        if p.kind == Kind::Servo && seen.insert(format!("s{}", p.pin)) {
            o.push(format!("Servo servo_{};", p.pin));
        }
        if p.kind == Kind::Tone && seen.insert(format!("t{}", p.pin)) {
            o.push(format!("int tone_{} = 1000;", p.pin));
        }
    }
    o.push(String::new());
    o.extend(protos);
    if !o.last().is_some_and(|l| l.is_empty()) {
        o.push(String::new());
    }
    o.extend(fn_out);
    o.push("void setup() {".into());
    if tr.prints {
        o.push("  Serial.begin(9600);".into());
    }
    let mut done: BTreeSet<String> = BTreeSet::new();
    for p in tr.pins.values() {
        if !done.insert(format!("{}{:?}", p.pin, p.kind)) {
            continue;
        }
        let line = match p.kind {
            Kind::Out | Kind::Pwm | Kind::Tone => format!("  pinMode({}, OUTPUT);", p.pin),
            Kind::In => format!("  pinMode({}, INPUT);", p.pin),
            Kind::Pull => format!("  pinMode({}, INPUT_PULLUP);", p.pin),
            Kind::Servo => format!("  servo_{}.attach({});", p.pin, p.pin),
            Kind::Analog => continue,
        };
        o.push(line);
    }
    o.extend(setup);
    o.push("}".into());
    o.push(String::new());
    o.push("void loop() {".into());
    o.extend(cb);
    o.extend(lp);
    o.push("}".into());
    o.push(String::new());
    Ok(o.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STUB: &str = r#"
#include <cmath>
#include <algorithm>
using std::min; using std::max; using std::abs;
#define HIGH 1
#define LOW 0
#define INPUT 0
#define OUTPUT 1
#define INPUT_PULLUP 2
#define PI 3.14159
enum { A0 = 14, A1, A2, A3, A4, A5, A6, A7 };
void pinMode(int, int); void digitalWrite(int, int); int digitalRead(int); int analogRead(int); void analogWrite(int, int);
void delay(unsigned long); void delayMicroseconds(unsigned int); void tone(int, unsigned int); void noTone(int);
unsigned long millis(); long random(long); long random(long, long);
long map(long, long, long, long, long); long constrain(long, long, long);
struct S_ { void begin(long); template <class T> void print(T); void println(); template <class T> void println(T); };
extern S_ Serial;
void setup(); void loop();
"#;

    // preloží a skontroluje, že výsledok je platné C++ (kontrola syntaxe cez g++, ak je k dispozícii)
    fn check(py: &str) -> String {
        let cpp = translate(py, "t.py").unwrap_or_else(|e| panic!("{e}\n{py}"));
        if std::env::var("FLUX_SHOW").is_ok() {
            eprintln!("{cpp}");
        }
        if let Ok(gpp) = std::process::Command::new("g++").arg("--version").output().map(|o| o).ok().filter(|_| cfg!(not(windows))).ok_or(()) {
            if gpp.status.success() {
                let dir = std::env::temp_dir().join(format!("flux-pytoc-{}-{}", std::process::id(), py.len()));
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join("Arduino.h"), STUB).unwrap();
                std::fs::write(dir.join("Servo.h"), "struct Servo { void attach(int); void write(int); };").unwrap();
                std::fs::write(dir.join("s.cpp"), format!("#include \"Arduino.h\"\n{cpp}")).unwrap();
                let out = std::process::Command::new("g++").args(["-fsyntax-only", "-I"]).arg(&dir).arg(dir.join("s.cpp")).output().unwrap();
                assert!(out.status.success(), "g++ failed:\n{}\n--- C++ ---\n{cpp}", String::from_utf8_lossy(&out.stderr));
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
        cpp
    }

    #[test]
    fn pytoc_firmata_blink_and_button() {
        let c = check("import time\nfrom pyfirmata2 import Arduino\n\nboard = Arduino(\"/dev/ttyUSB0\")\nled_13 = board.get_pin('d:13:o')\nbutton_2 = board.get_pin('d:2:i')\nbutton_2.register_callback(lambda v: print(\"Pin 2:\", v))\nbutton_2.enable_reporting()\nboard.samplingOn()\n\nwhile True:\n    led_13.write(1)\n    time.sleep(0.5)\n    led_13.write(0)\n    time.sleep(0.5)\n    button_2.read()\n");
        assert!(c.contains("pinMode(13, OUTPUT);") && c.contains("digitalWrite(13, (1) ? HIGH : LOW);") && c.contains("delay(500);") && c.contains("last_button_2"), "{c}");
    }

    #[test]
    fn pytoc_micropython_styles() {
        let c = check("from machine import Pin, PWM\nfrom time import sleep\nled = Pin(13, Pin.OUT)\nbtn = Pin(2, Pin.IN, Pin.PULL_UP)\nbuzzer = PWM(Pin(8))\nservo = PWM(Pin(9), freq=50)\nwhile True:\n    led.toggle()\n    if btn.value() == 0:\n        print(\"Button on pin 2 pressed\")\n        buzzer.freq(1000)\n        buzzer.duty_u16(32768)\n        sleep(0.2)\n        buzzer.duty_u16(0)\n    servo.duty_u16(4915)\n    sleep(0.5)\n");
        assert!(
            c.contains("Servo servo_9;")
                && c.contains("tone(8, tone_8)")
                && c.contains("pinMode(2, INPUT_PULLUP);")
                && c.contains("digitalWrite(13, !digitalRead(13));")
                && c.contains("Serial.begin(9600);"),
            "{c}"
        );
    }

    #[test]
    fn pytoc_circuitpython_style() {
        let c = check("import time\nimport board\nimport digitalio\nimport analogio\nled = digitalio.DigitalInOut(board.D13)\nled.direction = digitalio.Direction.OUTPUT\nbutton = digitalio.DigitalInOut(board.D2)\nbutton.direction = digitalio.Direction.INPUT\nbutton.pull = digitalio.Pull.UP\npot = analogio.AnalogIn(board.A0)\nwhile True:\n    led.value = not led.value\n    if not button.value:\n        print(pot.value)\n    time.sleep(0.5)\n");
        assert!(c.contains("digitalWrite(13, (!digitalRead(13)) ? HIGH : LOW);") && c.contains("pinMode(2, INPUT_PULLUP);") && c.contains("analogRead(A0) * 64L"), "{c}");
    }

    #[test]
    fn pytoc_logic_functions_lists() {
        let c = check("from machine import Pin\nfrom time import sleep\nLED = 13\nled = Pin(LED, Pin.OUT)\nnotes = [262, 294, 330]\ncount = 0\ntotal = 0.0\n\ndef blink(n, wait):\n    for i in range(n):\n        led.on()\n        sleep(wait)\n        led.off()\n        sleep(wait)\n\ndef double(x):\n    y = x * 2\n    return y\n\nwhile True:\n    count += 1\n    total = total + count / 2\n    if count > 10:\n        count = 0\n    elif count == 5:\n        blink(2, 0.1)\n    else:\n        print(f\"count {count} total {total} first {notes[0]} n {len(notes)}\", double(count))\n    for k in range(10, 0, -2):\n        sleep_ms(k)\n");
        assert!(c.contains("float total = 0;") || c.contains("float total = 0.0;"), "{c}");
        assert!(c.contains("long notes[] = {262, 294, 330};"), "{c}");
        assert!(c.contains("else if (count == 5)"), "{c}");
        assert!(c.contains("for (long k = 10; k > 0; k += -2)"), "{c}");
    }

    #[test]
    fn pytoc_errors_have_lines() {
        let e = translate("import requests\nx = requests.get('a')\n", "t.py").unwrap_err();
        assert!(e.starts_with("line 2:"), "{e}");
        let e = translate("class A:\n    pass\n", "t.py").unwrap_err();
        assert!(e.starts_with("line 1:"), "{e}");
        let e = translate("d = {1: 2}\n", "t.py").unwrap_err();
        assert!(e.contains("line 1"), "{e}");
        let e = translate("from machine import Pin\nled = Pin(13, Pin.OUT)\nwhile True:\n    led.toggle()\nprint(1)\n", "t.py").unwrap_err();
        assert!(e.contains("line 5"), "{e}");
    }

    #[test]
    fn pytoc_flux_presets_translate() {
        // všetko, čo Flux generuje cez okno pinu, sa dá preložiť
        for (mode, kinds) in [
            ("firmata", vec!["led", "button", "pwm", "analog", "servo", "buzzer"]),
            ("micropython", vec!["led", "button", "pwm", "analog", "servo", "buzzer"]),
            ("circuitpython", vec!["led", "button", "pwm", "analog", "servo", "buzzer"]),
        ] {
            for k in kinds {
                let pin = if k == "analog" { "A0" } else { "9" };
                let (_, py) = crate::arduino::new_for_pin(mode, pin, k, "/dev/ttyUSB0", false);
                check(&py);
            }
        }
    }
}
