// Návrhy kľúčov JSON dát v Pythone: `data = requests.get(url).json()` / `json.load(open("x.json"))`
// → pri písaní `data["releases"][0]["ker` Flux stiahne (raz) tie isté dáta a ponúkne ich kľúče.
use crate::complete::{Item, Kind};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq)]
pub enum Seg {
    Key(String),
    Idx(i64),
    Any, // premenná ako index (`[i]`) – berie sa prvý prvok
}

#[derive(Clone, Debug, PartialEq)]
pub enum Src {
    Url(String),
    File(String),
}

enum St {
    Loading,
    Ok(Arc<Value>),
    Err,
}

fn cache() -> &'static Mutex<HashMap<Src, St>> {
    static C: OnceLock<Mutex<HashMap<Src, St>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

impl std::hash::Hash for Src {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        match self {
            Src::Url(u) => ("u", u).hash(h),
            Src::File(f) => ("f", f).hash(h),
        }
    }
}
impl Eq for Src {}

static READY: AtomicBool = AtomicBool::new(false);

// dáta sa práve načítali → editor znova otvorí návrhy
pub fn take_ready() -> bool {
    READY.swap(false, Ordering::Relaxed)
}

const MAX_BYTES: usize = 8 << 20;

fn get(src: &Src) -> Option<Arc<Value>> {
    let mut c = cache().lock().unwrap();
    match c.get(src) {
        Some(St::Ok(v)) => return Some(v.clone()),
        Some(_) => return None,
        None => {}
    }
    c.insert(src.clone(), St::Loading);
    let s = src.clone();
    std::thread::spawn(move || {
        let body = match &s {
            Src::Url(u) => crate::net::request("GET", u, &["Accept: application/json".into()], None, 15).ok().filter(|(code, _)| (200..300).contains(code)).map(|(_, b)| b),
            Src::File(f) => std::fs::metadata(f).ok().filter(|m| m.len() as usize <= MAX_BYTES).and_then(|_| std::fs::read_to_string(f).ok()),
        };
        let v = body.filter(|b| b.len() <= MAX_BYTES).and_then(|b| serde_json::from_str::<Value>(&b).ok());
        let ok = v.is_some();
        cache().lock().unwrap().insert(s, v.map(|v| St::Ok(Arc::new(v))).unwrap_or(St::Err));
        if ok {
            READY.store(true, Ordering::Relaxed);
        }
    });
    None
}

fn is_id(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

// obsah `[ … ]` → segment
fn seg_of(inner: &str) -> Seg {
    let t = inner.trim();
    let q = t.chars().next().unwrap_or(' ');
    if (q == '"' || q == '\'') && t.len() >= 2 && t.ends_with(q) {
        return Seg::Key(t[1..t.len() - 1].to_string());
    }
    t.parse::<i64>().map(Seg::Idx).unwrap_or(Seg::Any)
}

// reťaz `meno[..][..]` končiaca na konci `s` → (meno, segmenty)
fn chain_end(s: &[char]) -> Option<(String, Vec<Seg>)> {
    let mut j = s.len();
    while j > 0 && s[j - 1] == ' ' {
        j -= 1;
    }
    let mut segs = vec![];
    while j > 0 && s[j - 1] == ']' {
        let close = j - 1;
        let mut k = close;
        let mut q: Option<char> = None;
        loop {
            if k == 0 {
                return None;
            }
            k -= 1;
            let c = s[k];
            match q {
                Some(qq) if c == qq => q = None,
                Some(_) => {}
                None if c == '"' || c == '\'' => q = Some(c),
                None if c == '[' => break,
                None => {}
            }
        }
        segs.push(seg_of(&s[k + 1..close].iter().collect::<String>()));
        j = k;
    }
    let st = s[..j].iter().rposition(|c| !is_id(*c)).map(|p| p + 1).unwrap_or(0);
    let var: String = s[st..j].iter().collect();
    if var.is_empty() || var.chars().next().unwrap().is_ascii_digit() {
        return None;
    }
    segs.reverse();
    Some((var, segs))
}

pub struct At {
    pub var: String,
    pub segs: Vec<Seg>,
    pub quote: Option<char>, // v úvodzovkách kľúča; None = hneď za „[“
    pub typed: String,
}

// kurzor v `var[...]["ty|` alebo `var[...][|`
pub fn at_cursor(before: &[char]) -> Option<At> {
    let line_start = before.iter().rposition(|c| *c == '\n').map(|p| p + 1).unwrap_or(0);
    let s = &before[line_start..];
    let mut i = s.len();
    while i > 0 && !matches!(s[i - 1], '"' | '\'' | '[' | ']' | '\n' | '(' | ')' | '{' | '}' | ',') {
        i -= 1;
    }
    let typed: String = s[i..].iter().collect();
    let (quote, open) = match s.get(i.wrapping_sub(1)) {
        Some('"') | Some('\'') => {
            let mut k = i - 1;
            while k > 0 && s[k - 1] == ' ' {
                k -= 1;
            }
            (Some(s[i - 1]), k.checked_sub(1).filter(|&k| s[k] == '[')?)
        }
        Some('[') if typed.trim().chars().all(|c| c.is_ascii_digit() || c == '-') => (None, i - 1),
        _ => return None,
    };
    let (var, segs) = chain_end(&s[..open])?;
    Some(At { var, segs, quote, typed })
}

// prvý reťazcový argument volania za `call(` v `rhs`, alebo premenná s reťazcom
fn first_arg(rhs: &str, call: &str, text: &str) -> Option<String> {
    let p = rhs.find(call)? + call.len();
    let arg = rhs[p..].trim_start();
    let arg = arg.strip_prefix("url=").unwrap_or(arg);
    let arg = arg.trim_start_matches(['f', 'r']);
    if let Some(q) = arg.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let body = &arg[1..];
        let end = body.find(q)?;
        let s = &body[..end];
        return (!s.contains('{')).then(|| s.to_string());
    }
    let name: String = arg.chars().take_while(|c| is_id(*c)).collect();
    if name.is_empty() {
        return None;
    }
    let v = assigned(text, &name)?;
    let v = v.trim().trim_start_matches(['f', 'r']);
    let q = v.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let body = &v[1..];
    let s = &body[..body.find(q)?];
    (!s.contains('{')).then(|| s.to_string())
}

// pravá strana posledného `name = …` v súbore
fn assigned(text: &str, name: &str) -> Option<String> {
    text.lines().rev().find_map(|l| {
        let t = l.trim_start();
        let r = t.strip_prefix(name)?.trim_start();
        let r = r.strip_prefix('=')?;
        (!r.starts_with('=')).then(|| r.trim().to_string())
    })
}

const GETS: [&str; 4] = ["requests.get(", "httpx.get(", "urlopen(", "session.get("];

// zdroj dát premennej (+ cesta, ak je to len kus iných dát: `rel = data["releases"]`)
pub fn resolve(text: &str, dir: &str, var: &str, depth: u8) -> Option<(Src, Vec<Seg>)> {
    if depth > 6 {
        return None;
    }
    let rhs = assigned(text, var)?;
    let url_src = |r: &str| GETS.iter().find_map(|g| first_arg(r, g, text)).filter(|u| u.starts_with("http://") || u.starts_with("https://")).map(Src::Url);
    // requests.get(url).json()
    if rhs.contains(".json()") {
        if let Some(s) = url_src(&rhs) {
            return Some((s, vec![]));
        }
        // resp.json()
        let name: String = rhs.chars().take_while(|c| is_id(*c)).collect();
        if !name.is_empty() && rhs[name.len()..].starts_with(".json()") {
            let r2 = assigned(text, &name)?;
            return url_src(&r2).map(|s| (s, vec![]));
        }
    }
    // json.loads(resp.text)
    if let Some(p) = rhs.find("json.loads(") {
        let name: String = rhs[p + 11..].chars().take_while(|c| is_id(*c)).collect();
        let r2 = assigned(text, &name)?;
        return url_src(&r2).map(|s| (s, vec![]));
    }
    // json.load(open("x.json")) / with open("x.json") as f: … json.load(f)
    if let Some(p) = rhs.find("json.load(") {
        let inner = &rhs[p + 10..];
        let file = first_arg(inner, "open(", text).or_else(|| {
            let f: String = inner.chars().take_while(|c| is_id(*c)).collect();
            text.lines().find_map(|l| {
                let a = l.find(&format!("as {f}:")).or_else(|| l.find(&format!("as {f} :")))?;
                first_arg(&l[..a], "open(", text)
            })
        })?;
        let path = std::path::Path::new(dir).join(&file);
        return Some((Src::File(path.to_string_lossy().to_string()), vec![]));
    }
    // kus iných dát: rel = data["releases"][0]
    let cs: Vec<char> = rhs.chars().collect();
    let (v2, segs) = chain_end(&cs)?;
    if v2 == var {
        return None;
    }
    let (src, mut base) = resolve(text, dir, &v2, depth + 1)?;
    base.extend(segs);
    Some((src, base))
}

fn walk<'a>(mut v: &'a Value, segs: &[Seg]) -> Option<&'a Value> {
    for s in segs {
        v = match (s, v) {
            (Seg::Key(k), Value::Object(m)) => m.get(k)?,
            (Seg::Idx(i), Value::Array(a)) => {
                let i = if *i < 0 { a.len() as i64 + i } else { *i };
                a.get(usize::try_from(i).ok()?)?
            }
            (Seg::Any, Value::Array(a)) => a.first()?,
            _ => return None,
        };
    }
    Some(v)
}

fn preview(v: &Value) -> String {
    match v {
        Value::String(s) => {
            let s: String = s.chars().take(40).collect();
            format!("\"{s}\"")
        }
        Value::Array(a) => format!("[{} items]", a.len()),
        Value::Object(m) => format!("{{{} keys}}", m.len()),
        other => other.to_string(),
    }
}

// návrhy pre kurzor; `pending` = dáta sa ešte sťahujú
pub fn items(text: &str, dir: &str, at: &At) -> (Vec<Item>, bool) {
    let Some((src, mut segs)) = resolve(text, dir, &at.var, 0) else { return (vec![], false) };
    segs.extend(at.segs.iter().cloned());
    let Some(root) = get(&src) else { return (vec![], true) };
    let Some(v) = walk(&root, &segs) else { return (vec![], false) };
    let typed = at.typed.to_lowercase();
    let mut out: Vec<(u8, Item)> = vec![];
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let kl = k.to_lowercase();
                let rank = if kl.starts_with(&typed) { 0 } else if kl.contains(&typed) { 1 } else { continue };
                if k.contains('$') {
                    continue; // $ by úryvok (expand) bral ako miesto kurzora
                }
                let insert = if at.quote.is_none() { Some(format!("\"{k}\"")) } else { None };
                out.push((rank, Item { label: k.clone(), kind: Kind::Json, insert, detail: Some(preview(val)), doc: None }));
            }
        }
        Value::Array(a) if at.quote.is_none() => {
            for (i, val) in a.iter().enumerate().take(50) {
                let l = i.to_string();
                if !l.starts_with(&typed) {
                    continue;
                }
                out.push((0, Item { label: l, kind: Kind::Json, insert: None, detail: Some(preview(val)), doc: None }));
            }
        }
        _ => {}
    }
    out.sort_by_key(|x| x.0);
    (out.into_iter().map(|x| x.1).take(60).collect(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn json_at_cursor() {
        let a = at_cursor(&chars("print(f\"x {data[\"releases\"][0][\"ker")).unwrap();
        assert_eq!((a.var.as_str(), a.quote, a.typed.as_str()), ("data", Some('"'), "ker"));
        assert_eq!(a.segs, [Seg::Key("releases".into()), Seg::Idx(0)]);
        let a = at_cursor(&chars("x = data['a'][")).unwrap();
        assert_eq!((a.quote, a.typed.as_str()), (None, ""));
        assert_eq!(a.segs, [Seg::Key("a".into())]);
        assert!(at_cursor(&chars("print(\"hello")).is_none());
    }

    #[test]
    fn json_resolve_and_items() {
        let text = "import requests\nurl = \"https://example.com/r.json\"\nresponse = requests.get(url)\ndata = response.json()\nrel = data[\"releases\"][0]\n";
        let src = Src::Url("https://example.com/r.json".into());
        assert_eq!(resolve(text, ".", "data", 0), Some((src.clone(), vec![])));
        assert_eq!(resolve(text, ".", "rel", 0), Some((src.clone(), vec![Seg::Key("releases".into()), Seg::Idx(0)])));
        let v: Value = serde_json::from_str(r#"{"releases":[{"version":"2026.11.01","kernel_version":"6.17"}]}"#).unwrap();
        cache().lock().unwrap().insert(src, St::Ok(Arc::new(v)));
        let at = at_cursor(&chars("print(data[\"releases\"][0][\"ker")).unwrap();
        let (it, pending) = items(text, ".", &at);
        assert!(!pending);
        assert_eq!(it.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(), ["kernel_version"]);
        let w = word_items(text, "./x.py", "ker");
        assert_eq!(w[0].insert.as_deref(), Some("rel[\"kernel_version\"]")); // kratšia cesta prvá
        assert!(w.iter().any(|i| i.insert.as_deref() == Some("data[\"releases\"][0][\"kernel_version\"]")));
        let at = at_cursor(&chars("rel[")).unwrap();
        let (it, _) = items(text, ".", &at);
        assert_eq!(it.len(), 2);
        assert!(it.iter().all(|i| i.insert.as_deref().is_some_and(|x| x.starts_with('"'))));
        assert!(resolve("data = requests.get(\"https://a.b/x\").json()\n", ".", "data", 0).is_some());
        assert_eq!(resolve("with open(\"d.json\") as f:\n    data = json.load(f)\n", "/p", "data", 0), Some((Src::File("/p/d.json".into()), vec![])));
    }
}

// pre editor: (návrhy, dĺžka napísanej časti kľúča)
pub fn complete(text: &str, path: &str, before: &[char]) -> Option<(Vec<Item>, usize)> {
    let at = at_cursor(before)?;
    let dir = std::path::Path::new(path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let (it, _pending) = items(text, &dir, &at);
    (!it.is_empty()).then(|| (it, at.typed.chars().count()))
}

fn key_expr(k: &str) -> String {
    format!("[\"{}\"]", k.replace('\\', "\\\\").replace('"', "\\\""))
}

// premenné v súbore, ktoré nesú JSON dáta (data, rel = data["releases"]…)
fn json_vars(text: &str, dir: &str) -> Vec<(String, Src, Vec<Seg>)> {
    let mut seen = std::collections::HashSet::new();
    let mut out = vec![];
    for l in text.lines() {
        let t = l.trim_start();
        let name: String = t.chars().take_while(|c| is_id(*c)).collect();
        if name.is_empty() || !t[name.len()..].trim_start().starts_with('=') || t[name.len()..].trim_start().starts_with("==") || !seen.insert(name.clone()) {
            continue;
        }
        if let Some((src, segs)) = resolve(text, dir, &name, 0) {
            out.push((name, src, segs));
        }
        if seen.len() > 80 {
            break;
        }
    }
    out
}

// písané slovo (`ker`) → kľúče z JSON dát s celou cestou: data["releases"][0]["kernel_version"]
pub fn word_items(text: &str, path: &str, prefix: &str) -> Vec<Item> {
    let dir = std::path::Path::new(path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let pl = prefix.to_lowercase();
    let mut out: Vec<(u8, usize, Item)> = vec![];
    let mut labels = std::collections::HashSet::new();
    for (var, src, segs) in json_vars(text, &dir) {
        let Some(root) = get(&src) else { continue };
        let Some(v) = walk(&root, &segs) else { continue };
        // prechod do šírky: prvý prvok poľa ako vzor, hĺbka ≤ 5, ≤ 4000 uzlov
        let mut q: std::collections::VecDeque<(&Value, String, usize)> = std::collections::VecDeque::from([(v, var.clone(), 0)]);
        let mut n = 0;
        while let Some((node, expr, depth)) = q.pop_front() {
            n += 1;
            if n > 4000 || depth > 5 {
                break;
            }
            match node {
                Value::Object(m) => {
                    for (k, val) in m {
                        let e = format!("{expr}{}", key_expr(k));
                        let kl = k.to_lowercase();
                        let rank = if kl.starts_with(&pl) { 0 } else if kl.contains(&pl) { 1 } else { u8::MAX };
                        if rank != u8::MAX && !e.contains('$') && labels.insert((k.clone(), var.clone())) {
                            let insert = e.clone();
                            out.push((rank, depth, Item { label: k.clone(), kind: Kind::Json, insert: Some(insert), detail: Some(format!("{e}  {}", preview(val))), doc: None }));
                        }
                        q.push_back((val, e, depth + 1));
                    }
                }
                Value::Array(a) => {
                    if let Some(first) = a.first() {
                        q.push_back((first, format!("{expr}[0]"), depth + 1));
                    }
                }
                _ => {}
            }
        }
    }
    out.sort_by_key(|x| (x.0, x.1));
    out.into_iter().map(|x| x.2).take(12).collect()
}
