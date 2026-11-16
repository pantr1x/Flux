// Dáta vo výstupe programu: `print(slovník)` z Pythonu (repr: 'a', True, None, (1, 2)) aj JSON.
// parse() ich prečíta do stromu (poradie kľúčov ostane), find_all() ich nájde v texte Outputu.

#[derive(Clone, Debug, PartialEq)]
pub enum D {
    Map(Vec<(String, D)>),
    List(Vec<D>, char), // '[' zoznam, '(' tuple, '{' množina
    Str(String),
    Num(String),
    Bool(bool),
    Null,      // None / null
    Raw(String), // datetime.date(2026, 1, 1), nan, <object …>
}

impl D {
    pub fn len(&self) -> usize {
        match self {
            D::Map(v) => v.len(),
            D::List(v, _) => v.len(),
            _ => 0,
        }
    }
    pub fn is_container(&self) -> bool {
        matches!(self, D::Map(_) | D::List(..))
    }
    // druh pre ľudí (typ hodnoty v strome)
    pub fn kind(&self) -> &'static str {
        match self {
            D::Map(_) => "dict",
            D::List(_, '(') => "tuple",
            D::List(_, '{') => "set",
            D::List(..) => "list",
            D::Str(_) => "str",
            D::Num(n) if n.contains(['.', 'e', 'E']) => "float",
            D::Num(_) => "int",
            D::Bool(_) => "bool",
            D::Null => "None",
            D::Raw(_) => "object",
        }
    }
    // krátky text hodnoty (listy stromu)
    pub fn short(&self) -> String {
        match self {
            D::Str(s) => format!("\"{s}\""),
            D::Num(n) => n.clone(),
            D::Bool(b) => if *b { "True".into() } else { "False".into() },
            D::Null => "None".into(),
            D::Raw(r) => r.clone(),
            D::Map(v) => format!("{{{}}}", v.len()),
            D::List(v, c) => format!("{}{}{}", c, v.len(), match c { '(' => ')', '{' => '}', _ => ']' }),
        }
    }
}

struct P<'a> {
    s: &'a [char],
    i: usize,
    depth: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }
    fn eat(&mut self, c: char) -> bool {
        self.ws();
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self) -> Option<D> {
        self.ws();
        if self.depth > 200 {
            return None;
        }
        let c = self.peek()?;
        match c {
            '{' => {
                self.i += 1;
                self.depth += 1;
                let r = self.map_or_set();
                self.depth -= 1;
                r
            }
            '[' | '(' => {
                self.i += 1;
                self.depth += 1;
                let close = if c == '[' { ']' } else { ')' };
                let mut v = vec![];
                loop {
                    if self.eat(close) {
                        break;
                    }
                    v.push(self.value()?);
                    if !self.eat(',') {
                        if !self.eat(close) {
                            return None;
                        }
                        break;
                    }
                }
                self.depth -= 1;
                Some(D::List(v, c))
            }
            '\'' | '"' => self.string().map(D::Str),
            'b' if matches!(self.s.get(self.i + 1), Some('\'' | '"')) => {
                self.i += 1;
                self.string().map(|s| D::Raw(format!("b\"{s}\"")))
            }
            '-' | '+' | '0'..='9' | '.' => {
                let st = self.i;
                self.i += 1;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || matches!(self.s[self.i], '.' | '_' | '+' | '-')) {
                    // 1e-5 áno, 1-2 nie (mínus len po e)
                    if matches!(self.s[self.i], '+' | '-') && !matches!(self.s[self.i - 1], 'e' | 'E') {
                        break;
                    }
                    self.i += 1;
                }
                let n: String = self.s[st..self.i].iter().collect();
                if n.parse::<f64>().is_ok() || n.chars().filter(|c| c.is_ascii_digit()).count() > 0 && n.chars().all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c)) {
                    Some(D::Num(n))
                } else {
                    None
                }
            }
            '<' => {
                // <object at 0x…> – po najbližšie '>'
                let st = self.i;
                while self.i < self.s.len() && self.s[self.i] != '>' && self.s[self.i] != '\n' {
                    self.i += 1;
                }
                if self.peek() != Some('>') {
                    return None;
                }
                self.i += 1;
                Some(D::Raw(self.s[st..self.i].iter().collect()))
            }
            c if c.is_alphabetic() || c == '_' => {
                let st = self.i;
                while self.i < self.s.len() && (self.s[self.i].is_alphanumeric() || matches!(self.s[self.i], '_' | '.')) {
                    self.i += 1;
                }
                let w: String = self.s[st..self.i].iter().collect();
                match w.as_str() {
                    "True" | "true" => return Some(D::Bool(true)),
                    "False" | "false" => return Some(D::Bool(false)),
                    "None" | "null" => return Some(D::Null),
                    _ => {}
                }
                // volanie: datetime.date(2026, 1, 1), Decimal('1.5') – celé ako text
                if self.peek() == Some('(') {
                    let mut d = 0usize;
                    let mut q: Option<char> = None;
                    while self.i < self.s.len() {
                        let ch = self.s[self.i];
                        match q {
                            Some(_) if ch == '\\' => self.i += 1,
                            Some(qc) if ch == qc => q = None,
                            Some(_) => {}
                            None if ch == '\'' || ch == '"' => q = Some(ch),
                            None if ch == '(' => d += 1,
                            None if ch == ')' => {
                                d -= 1;
                                if d == 0 {
                                    self.i += 1;
                                    break;
                                }
                            }
                            None => {}
                        }
                        self.i += 1;
                    }
                    if d != 0 {
                        return None;
                    }
                    return Some(D::Raw(self.s[st..self.i].iter().collect()));
                }
                // nan, inf, Ellipsis…
                Some(D::Raw(w))
            }
            _ => None,
        }
    }
    fn map_or_set(&mut self) -> Option<D> {
        if self.eat('}') {
            return Some(D::Map(vec![]));
        }
        let first = self.value()?;
        if self.eat(':') {
            let mut v = vec![(key_text(&first), self.value()?)];
            loop {
                if !self.eat(',') {
                    return self.eat('}').then_some(D::Map(v));
                }
                if self.eat('}') {
                    return Some(D::Map(v));
                }
                let k = self.value()?;
                if !self.eat(':') {
                    return None;
                }
                v.push((key_text(&k), self.value()?));
            }
        }
        // množina {1, 2}
        let mut v = vec![first];
        loop {
            if !self.eat(',') {
                return self.eat('}').then_some(D::List(v, '{'));
            }
            if self.eat('}') {
                return Some(D::List(v, '{'));
            }
            v.push(self.value()?);
        }
    }
    fn string(&mut self) -> Option<String> {
        let q = self.peek()?;
        self.i += 1;
        let mut out = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '\\' => {
                    let e = self.peek()?;
                    self.i += 1;
                    match e {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        'u' | 'x' => {
                            let n = if e == 'u' { 4 } else { 2 };
                            let h: String = self.s.get(self.i..self.i + n)?.iter().collect();
                            self.i += n;
                            out.push(u32::from_str_radix(&h, 16).ok().and_then(char::from_u32).unwrap_or('?'));
                        }
                        other => out.push(other),
                    }
                }
                '\n' => return None,
                c if c == q => return Some(out),
                c => out.push(c),
            }
        }
        None
    }
}

// kľúč ako text: reťazec bez úvodzoviek, ostatné ako v Pythone (1, (1, 2), True)
fn key_text(d: &D) -> String {
    match d {
        D::Str(s) => s.clone(),
        D::List(v, c) => {
            let inner: Vec<String> = v.iter().map(|x| if let D::Str(s) = x { format!("'{s}'") } else { key_text(x) }).collect();
            format!("{}{}{}", c, inner.join(", "), if *c == '(' { ")" } else { "]" })
        }
        other => other.short(),
    }
}

// cesta k hodnote ako v Pythone: ["current"]["temperature_2m"], [0]
pub fn step(key: &str, parent_is_map: bool, index: usize) -> String {
    if parent_is_map {
        if key.parse::<i64>().is_ok() { format!("[{key}]") } else { format!("[\"{}\"]", key.replace('"', "\\\"")) }
    } else {
        format!("[{index}]")
    }
}

#[cfg(test)]
pub fn parse(s: &str) -> Option<D> {
    let chars: Vec<char> = s.chars().collect();
    let mut p = P { s: &chars, i: 0, depth: 0 };
    let v = p.value()?;
    p.ws();
    (p.i == chars.len()).then_some(v)
}

// Dáta vypísané programom: každý riadok, kde začína { alebo [ (aj za „Data: “) a hodnota sa dá prečítať
// (môže pokračovať na ďalších riadkoch – pprint). Vráti (riadok výstupu od 1, hodnota); len slovníky a zoznamy s obsahom.
pub fn find_all(text: &str) -> Vec<(usize, D)> {
    const MAX: usize = 4_000_000;
    let chars: Vec<char> = text.chars().take(MAX).collect();
    let mut out = vec![];
    let mut i = 0usize;
    let mut line_no = 1usize;
    while i < chars.len() {
        // začiatok riadku
        let ls = i;
        let mut le = i;
        while le < chars.len() && chars[le] != '\n' {
            le += 1;
        }
        let mut next = le + 1;
        let mut lines_used = 1;
        if let Some(off) = chars[ls..le].iter().position(|c| *c == '{' || *c == '[') {
            let st = ls + off;
            let mut p = P { s: &chars, i: st, depth: 0 };
            if let Some(v) = p.value() {
                // za hodnotou na riadku už nič (len medzery) – inak to bol bežný text so zátvorkou
                let mut e = p.i;
                while e < chars.len() && chars[e] != '\n' && chars[e].is_whitespace() {
                    e += 1;
                }
                let ok = (e >= chars.len() || chars[e] == '\n') && v.len() > 0 && p.i - st >= 8;
                if ok {
                    lines_used = chars[ls..e].iter().filter(|c| **c == '\n').count() + 1;
                    out.push((line_no, v));
                    next = e + 1;
                }
            }
        }
        line_no += lines_used;
        i = next;
    }
    if out.len() > 30 {
        out.drain(..out.len() - 30);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pydata_python_repr() {
        let d = parse("{'latitude': 48.43, 'ok': True, 'none': None, 'tz': 'GMT', 'units': {'t': '°C'}, 'l': [1, -2.5e-3, (1, 2)], 's': {1, 2}}").unwrap();
        let D::Map(v) = &d else { panic!() };
        assert_eq!(v[0].0, "latitude");
        assert_eq!(v[0].1, D::Num("48.43".into()));
        assert_eq!(v[1].1, D::Bool(true));
        assert_eq!(v[2].1, D::Null);
        assert_eq!(v[4].1.len(), 1);
        assert_eq!(v[5].1.kind(), "list");
        assert_eq!(v[6].1.kind(), "set");
        assert_eq!(parse("{\"a\": [true, null, \"x\\\"y\"]}").unwrap().len(), 1);
        assert!(parse("{'a': 1").is_none());
        assert_eq!(parse("[datetime.date(2026, 1, 1), <Foo object at 0x1>]").unwrap().len(), 2);
    }

    #[test]
    fn pydata_find_in_output() {
        let out = "Starting\nLatitude: 48.4317357\n{'a': 1, 'b': {'c': [1, 2]}}\nno {data} here ok\nData: [1, 2, 3, 4]\n{'x':\n  1,\n 'y': 2}\nend\n";
        let f = find_all(out);
        assert_eq!(f.len(), 3, "{f:?}");
        assert_eq!(f[0].0, 3);
        assert_eq!(f[1].0, 5);
        assert_eq!(f[2].0, 6);
        assert_eq!(step("current", true, 0), "[\"current\"]");
        assert_eq!(step("", false, 3), "[3]");
    }
}
