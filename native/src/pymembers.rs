// Návrhy za bodkou z naozaj nainštalovaných Python modulov: `platform.` → dir(platform) cez Python
// (funkcie s parametrami, triedy, podmoduly). Po `pip install` sa nový balíček ukáže sám (forget()).
use crate::complete::{Item, Kind};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Member {
    name: String,
    kind: String, // function / class / module / value
    sig: String,
    doc: &'static str,
}

enum St {
    Loading,
    Done(Vec<Member>),
}

fn cache() -> &'static Mutex<HashMap<(String, String), St>> {
    static C: OnceLock<Mutex<HashMap<(String, String), St>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

static READY: AtomicBool = AtomicBool::new(false);

pub fn take_ready() -> bool {
    READY.swap(false, Ordering::Relaxed)
}

// po pip install / update: zabudnúť (aj prázdne výsledky nenájdených modulov)
pub fn forget() {
    cache().lock().unwrap().clear();
}

// popisy sa pamätajú natrvalo (Item.doc je &'static) – rovnaký text len raz
fn intern(s: &str) -> &'static str {
    static I: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut m = I.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    if let Some(v) = m.get(s) {
        return v;
    }
    let v: &'static str = Box::leak(s.to_string().into_boxed_str());
    m.insert(s.to_string(), v);
    v
}

// -I: bez priečinka projektu na ceste – vlastné súbory používateľa sa tu nikdy nespúšťajú
const DIR: &str = r#"import sys, json, importlib, inspect, io, contextlib
path = sys.argv[1].split('.')
out = []
try:
    obj = None
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        for i in range(len(path), 0, -1):
            try:
                obj = importlib.import_module('.'.join(path[:i]))
                rest = path[i:]
                break
            except Exception:
                pass
        if obj is not None:
            for r in rest:
                obj = getattr(obj, r)
    if obj is not None:
        for n in dir(obj):
            if n.startswith('_'):
                continue
            try:
                v = getattr(obj, n)
            except Exception:
                continue
            # moduly, ktoré si modul len sám importuje (platform → re, os), nie sú jeho časťou
            if inspect.ismodule(v) and inspect.ismodule(obj) and not getattr(v, '__name__', '').startswith(obj.__name__ + '.') and n not in getattr(obj, '__all__', ()):
                continue
            k = 'module' if inspect.ismodule(v) else 'class' if inspect.isclass(v) else 'function' if callable(v) else 'value'
            sig = ''
            if k in ('function', 'class'):
                try:
                    sig = str(inspect.signature(v))
                except Exception:
                    sig = '(…)'
            if k == 'value':
                doc = type(v).__name__ + ' = ' + repr(v)[:60]
            else:
                doc = (inspect.getdoc(v) or '').strip().split('\n\n')[0].replace('\n', ' ')[:220]
            out.append([n, k, sig, doc])
except Exception:
    pass
print()
print(json.dumps(out))
"#;

fn introspect(py: &str, path: &str) -> Vec<Member> {
    let mut c = Command::new(py);
    c.args(["-I", "-c", DIR, path]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let Ok(mut child) = c.spawn() else { return vec![] };
    // veľký balíček sa môže importovať dlho; nad 10 s to vzdať
    let t0 = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if t0.elapsed() < Duration::from_secs(10) => std::thread::sleep(Duration::from_millis(30)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return vec![];
            }
        }
    }
    let Ok(out) = child.wait_with_output() else { return vec![] };
    let text = String::from_utf8_lossy(&out.stdout);
    let Some(last) = text.lines().rev().find(|l| !l.trim().is_empty()) else { return vec![] };
    let v: serde_json::Value = serde_json::from_str(last).unwrap_or_default();
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| {
                    let s = |i: usize| m[i].as_str().unwrap_or("").to_string();
                    let name = s(0);
                    (!name.is_empty()).then(|| Member { name, kind: s(1), sig: s(2), doc: intern(&s(3)) })
                })
                .collect()
        })
        .unwrap_or_default()
}

// meno v kóde → plná cesta modulu: `import numpy as np` (np → numpy), `from os import path` (path → os.path)
pub fn imported(text: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for raw in text.lines() {
        let l = raw.split('#').next().unwrap_or("").trim();
        if let Some(r) = l.strip_prefix("import ") {
            for part in r.split(',') {
                let mut w = part.split_whitespace();
                let full = w.next().unwrap_or("");
                if full.is_empty() {
                    continue;
                }
                match (w.next(), w.next()) {
                    (Some("as"), Some(al)) => m.insert(al.to_string(), full.to_string()),
                    _ => m.insert(full.split('.').next().unwrap_or(full).to_string(), full.split('.').next().unwrap_or(full).to_string()),
                };
            }
        } else if let Some(r) = l.strip_prefix("from ") {
            let Some((base, names)) = r.split_once(" import ") else { continue };
            let base = base.trim();
            if base.starts_with('.') {
                continue;
            }
            for part in names.replace(['(', ')'], "").split(',') {
                let mut w = part.split_whitespace();
                let name = w.next().unwrap_or("");
                if name.is_empty() || name == "*" {
                    continue;
                }
                let al = match (w.next(), w.next()) {
                    (Some("as"), Some(al)) => al,
                    _ => name,
                };
                m.insert(al.to_string(), format!("{base}.{name}"));
            }
        }
    }
    m
}

fn is_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_') && !s.starts_with(|c: char| c.is_ascii_digit())
}

// `chain` = „platform“ / „np.linalg“ (pred bodkou) → návrhy, ak je to importovaný modul a už je načítaný
pub fn complete(text: &str, py: &str, chain: &str, prefix: &str) -> Option<Vec<Item>> {
    if py.is_empty() || !chain.split('.').all(is_id) {
        return None;
    }
    let mut parts = chain.splitn(2, '.');
    let head = parts.next()?;
    let full = imported(text).get(head)?.clone();
    let path = match parts.next() {
        Some(rest) => format!("{full}.{rest}"),
        None => full,
    };
    let key = (py.to_string(), path.clone());
    let members = {
        let mut c = cache().lock().unwrap();
        match c.get(&key) {
            Some(St::Done(v)) => v.clone(),
            Some(St::Loading) => return None,
            None => {
                c.insert(key.clone(), St::Loading);
                let py = py.to_string();
                std::thread::spawn(move || {
                    let v = introspect(&py, &path);
                    let any = !v.is_empty();
                    cache().lock().unwrap().insert(key, St::Done(v));
                    if any {
                        READY.store(true, Ordering::Relaxed);
                    }
                });
                return None;
            }
        }
    };
    if members.is_empty() {
        return None;
    }
    let pl = prefix.to_lowercase();
    let mut v: Vec<(u8, Item)> = members
        .iter()
        .filter_map(|m| {
            let nl = m.name.to_lowercase();
            let rank = if m.name.starts_with(prefix) { 0 } else if nl.starts_with(&pl) { 1 } else if nl.contains(&pl) { 2 } else { return None };
            let (kind, insert, detail) = match m.kind.as_str() {
                "function" | "class" => {
                    let no_args = m.sig == "()";
                    (Kind::Builtin, Some(if no_args { format!("{}()$0", m.name) } else { format!("{}($0)", m.name) }), Some(format!("{}{}", m.name, m.sig)))
                }
                "module" => (Kind::Module, None, Some("module".to_string())),
                _ => (Kind::Property, None, None),
            };
            let doc = (!m.doc.is_empty()).then_some(m.doc);
            Some((rank, Item { label: m.name.clone(), kind, insert, detail, doc }))
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.label.to_lowercase().cmp(&b.1.label.to_lowercase())));
    Some(v.into_iter().map(|x| x.1).take(60).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pymembers_imports() {
        let m = imported("import platform\nimport numpy as np, os.path\nfrom os import path as p, getcwd\nfrom . import x\n");
        assert_eq!(m["platform"], "platform");
        assert_eq!(m["np"], "numpy");
        assert_eq!(m["os"], "os");
        assert_eq!(m["p"], "os.path");
        assert_eq!(m["getcwd"], "os.getcwd");
        assert!(!m.contains_key("x"));
    }

    #[test]
    fn pymembers_real_python() {
        let py = if cfg!(windows) { "python" } else { "python3" };
        if Command::new(py).arg("--version").output().is_err() {
            return;
        }
        let text = "import platform\n";
        assert!(complete(text, py, "platform", "rel").is_none()); // prvýkrát sa načítava
        let t0 = Instant::now();
        let items = loop {
            if let Some(v) = complete(text, py, "platform", "rel") {
                break v;
            }
            assert!(t0.elapsed() < Duration::from_secs(15));
            std::thread::sleep(Duration::from_millis(50));
        };
        let rel = items.iter().find(|i| i.label == "release").expect("release");
        assert_eq!(rel.insert.as_deref(), Some("release()$0"));
        assert!(complete(text, py, "nonimported", "x").is_none());
    }
}
