// Pluginy pre Flux Native: JavaScript (QuickJS cez rquickjs), každý plugin vo vlastnom kontexte.
// Plugin vidí len objekt `flux` (prelúdium nižšie) – žiadny Node, súbory ani sieť. Volania z JS len zapisujú
// „efekty“ (__fx), ktoré potom vykoná App (toast, vloženie textu, príkaz, stavový riadok, úryvok, téma, značky…).
// Každé volanie má 200 ms (nekonečná slučka sa preruší) a runtime má strop 64 MB.
use rquickjs::{Context, Ctx, Function, Runtime};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

// koniec dovoleného času aktuálneho volania (ms od štartu procesu) – vlastný pre každý Host
use std::time::{Duration, Instant};

const CALL_MS: u64 = 200;

static FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);

fn now_ms() -> u64 {
    static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    T0.get_or_init(Instant::now).elapsed().as_millis() as u64
}

// témy z pluginov (kľúč, názov, tmavá, 16 farieb) – číta ich code::theme_of a výber tém
pub static THEMES: Mutex<Vec<(String, String, String, bool, [u32; 16])>> = Mutex::new(Vec::new());

#[derive(Clone, Debug)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub publisher: String,
    pub dir: PathBuf,
    pub entry: String,
    pub dev: bool,
}

pub fn read_manifest(dir: &Path, dev: bool) -> Option<Manifest> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("plugin.json")).ok()?).ok()?;
    // pre Flux Native: pole „native“ = súbor (alebo true = „main“)
    let entry = match &v["native"] {
        Value::String(s) => s.clone(),
        Value::Bool(true) => v["main"].as_str().unwrap_or("plugin.js").to_string(),
        _ => return None,
    };
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    Some(Manifest { id: s("id"), name: s("name"), version: s("version"), description: s("description"), publisher: s("publisher"), dir: dir.to_path_buf(), entry, dev })
}

pub fn installed_dir() -> PathBuf {
    flux_core::settings::user_data().join("native-plugins")
}

pub struct Plugin {
    pub man: Manifest,
    ctx: Option<Context>,
    pub error: Option<String>,
    pub timers: bool,
}

pub struct Host {
    rt: Runtime,
    deadline: Arc<AtomicU64>,
    pub list: Vec<Plugin>,
    pub log: Vec<String>,
}

// prelúdium: objekt flux v čistom JS; efekty idú do __fx, Rust ich vyberie po každom volaní
const PRELUDE: &str = r#"
var __fx = [], __cmds = {}, __status = {}, __ev = { open: [], save: [], change: [], selection: [] }, __timers = [], __n = 0, __plugin = {};
var __snap = { file: null, folder: null, storage: {} };
function __setSnap(s) { __snap = s; }
function __out() { var f = __fx; __fx = []; return JSON.stringify(f); }
var console = { log: function () { __fx.push({ t: 'log', text: Array.prototype.map.call(arguments, function (a) { return typeof a === 'string' ? a : JSON.stringify(a); }).join(' ') }); } };
console.error = console.warn = console.info = console.log;
var flux = {
  id: __ID, version: 'native-1',
  toast: function (text, kind) { __fx.push({ t: 'toast', text: String(text), kind: kind || 'info' }); },
  activeFile: function () { return __snap.file ? JSON.parse(JSON.stringify(__snap.file)) : null; },
  insertText: function (text) { __fx.push({ t: 'insert', text: String(text) }); },
  replaceSelection: function (text) { __fx.push({ t: 'replace', text: String(text) }); },
  commands: {
    register: function (id, label, run, opts) { __cmds[id] = run; __fx.push({ t: 'cmd', id: String(id), label: String(label), key: (opts && opts.key) || '' }); },
    run: function (id) { __fx.push({ t: 'run', id: String(id) }); }
  },
  statusBar: {
    add: function (o) {
      o = o || {}; var k = 's' + (++__n); __status[k] = o.onClick || null;
      __fx.push({ t: 'status', k: k, text: String(o.text || ''), title: String(o.title || ''), show: true });
      return {
        set: function (t) { __fx.push({ t: 'status', k: k, text: String(t) }); },
        setTitle: function (t) { __fx.push({ t: 'status', k: k, title: String(t) }); },
        show: function (b) { __fx.push({ t: 'status', k: k, show: !!b }); },
        remove: function () { delete __status[k]; __fx.push({ t: 'status-rm', k: k }); }
      };
    }
  },
  snippets: { add: function (lang, prefix, body, desc) { __fx.push({ t: 'snippet', lang: String(lang), prefix: String(prefix), body: String(body), desc: String(desc || '') }); } },
  themes: { add: function (key, def) { __fx.push({ t: 'theme', key: String(key), def: def || {} }); } },
  marks: {
    set: function (list) { __fx.push({ t: 'marks', list: list || [] }); },
    clear: function () { __fx.push({ t: 'marks', list: [] }); }
  },
  onOpen: function (cb) { __ev.open.push(cb); },
  onSave: function (cb) { __ev.save.push(cb); },
  onChange: function (cb) { __ev.change.push(cb); },
  onSelection: function (cb) { __ev.selection.push(cb); },
  every: function (ms, cb) { __timers.push({ ms: Math.max(250, ms | 0), cb: cb, next: 0 }); __fx.push({ t: 'timers' }); },
  project: {
    folder: function () { return __snap.folder; },
    files: function () { return JSON.parse(__files()); },
    read: function (p) { var t = __read(String(p)); if (t === null || t === undefined) throw new Error('Cannot read ' + p); return t; }
  },
  storage: {
    get: function (k, d) { return Object.prototype.hasOwnProperty.call(__snap.storage, k) ? __snap.storage[k] : d; },
    set: function (k, v) { __snap.storage[k] = v; __fx.push({ t: 'store', key: String(k), value: v === undefined ? null : v }); }
  }
};
function __emit(name) { var a = __ev[name] || []; for (var i = 0; i < a.length; i++) a[i](flux.activeFile()); }
function __cmd(id) { if (__cmds[id]) __cmds[id](); }
function __click(k) { if (__status[k]) __status[k](); }
function __tick(t) { for (var i = 0; i < __timers.length; i++) { var x = __timers[i]; if (t >= x.next) { x.next = t + x.ms; x.cb(); } } }
"#;

// export function activate(flux) … → obyčajné funkcie v obale; import nie je (jeden súbor)
fn wrap(src: &str) -> String {
    let mut out = String::new();
    for line in src.lines() {
        let tl = line.trim_start();
        let ind = &line[..line.len() - tl.len()];
        if let Some(rest) = tl.strip_prefix("export default ") {
            out.push_str(&format!("{ind}var __default = {rest}\n"));
        } else if let Some(rest) = tl.strip_prefix("export ") {
            out.push_str(ind);
            out.push_str(rest);
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    format!(
        "__plugin = (function () {{\n{out}\n;var d = typeof __default === 'object' && __default ? __default : {{}};\nreturn {{ activate: typeof activate === 'function' ? activate : d.activate, deactivate: typeof deactivate === 'function' ? deactivate : d.deactivate }};\n}})();"
    )
}

// jednoduchá kontrola kódu pred inštaláciou (ako pluginScan.js)
pub fn scan(src: &str) -> Vec<&'static str> {
    let mut v = vec![];
    if src.contains("eval(") {
        v.push("uses eval()");
    }
    if src.contains("Function(") {
        v.push("builds code with Function()");
    }
    if src.lines().any(|l| l.len() > 2000) {
        v.push("has very long lines (minified or hidden code)");
    }
    if src.contains("\\x") && src.matches("\\x").count() > 40 {
        v.push("looks obfuscated");
    }
    v
}

fn err_text(ctx: &Ctx, e: rquickjs::Error) -> String {
    if matches!(e, rquickjs::Error::Exception) {
        let v = ctx.catch();
        if let Some(x) = v.as_exception() {
            let m = x.message().unwrap_or_default();
            if m.contains("interrupted") {
                return "took too long and was stopped".into();
            }
            return m;
        }
        if let Some(s) = v.as_string().and_then(|s| s.to_string().ok()) {
            return s;
        }
    }
    let s = e.to_string();
    if s.contains("interrupted") {
        "took too long and was stopped".into()
    } else {
        s
    }
}

impl Host {
    pub fn new() -> Self {
        let rt = Runtime::new().expect("quickjs");
        rt.set_memory_limit(64 * 1024 * 1024);
        let deadline = Arc::new(AtomicU64::new(u64::MAX));
        let d = deadline.clone();
        rt.set_interrupt_handler(Some(Box::new(move || now_ms() > d.load(Ordering::Relaxed))));
        Host { rt, deadline, list: vec![], log: vec![] }
    }

    // nainštalované + vývojárske priečinky; vypnuté (settings.nativePlugins[id].enabled == false) sa nespustia
    pub fn discover(dev_dirs: &[String]) -> Vec<Manifest> {
        let mut out = vec![];
        if let Ok(rd) = std::fs::read_dir(installed_dir()) {
            for e in rd.flatten() {
                if let Some(m) = read_manifest(&e.path(), false) {
                    out.push(m);
                }
            }
        }
        for d in dev_dirs {
            if let Some(m) = read_manifest(Path::new(d), true) {
                out.retain(|x: &Manifest| x.id != m.id);
                out.push(m);
            }
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out
    }

    // spustí plugin; vráti jeho efekty z activate()
    pub fn load(&mut self, man: Manifest, snap: &Value, enabled: bool) -> Vec<Value> {
        let mut p = Plugin { man, ctx: None, error: None, timers: false };
        if !enabled {
            self.list.push(p);
            return vec![];
        }
        let src = match std::fs::read_to_string(p.man.dir.join(&p.man.entry)) {
            Ok(s) => s,
            Err(e) => {
                p.error = Some(format!("{}: {e}", p.man.entry));
                self.list.push(p);
                return vec![];
            }
        };
        let ctx = match Context::full(&self.rt) {
            Ok(c) => c,
            Err(e) => {
                p.error = Some(e.to_string());
                self.list.push(p);
                return vec![];
            }
        };
        let id = p.man.id.clone();
        let dl = self.deadline.clone();
        let res: Result<String, String> = ctx.with(|ctx| {
            let g = ctx.globals();
            let read = Function::new(ctx.clone(), |path: String| -> Option<String> { read_in_project(&path) }).map_err(|e| e.to_string())?;
            let files = Function::new(ctx.clone(), || -> String { project_files() }).map_err(|e| e.to_string())?;
            g.set("__read", read).map_err(|e| e.to_string())?;
            g.set("__files", files).map_err(|e| e.to_string())?;
            g.set("__ID", id.clone()).map_err(|e| e.to_string())?;
            dl.store(now_ms() + CALL_MS, Ordering::Relaxed);
            ctx.eval::<(), _>(PRELUDE).map_err(|e| err_text(&ctx, e))?;
            ctx.eval::<(), _>(format!("__setSnap({snap})")).map_err(|e| err_text(&ctx, e))?;
            ctx.eval::<(), _>(wrap(&src)).map_err(|e| err_text(&ctx, e))?;
            ctx.eval::<(), _>("if (__plugin.activate) __plugin.activate(flux);").map_err(|e| err_text(&ctx, e))?;
            ctx.eval::<String, _>("__out()").map_err(|e| err_text(&ctx, e))
        });
        dl.store(u64::MAX, Ordering::Relaxed);
        let fx = match res {
            Ok(s) => serde_json::from_str::<Vec<Value>>(&s).unwrap_or_default(),
            Err(e) => {
                p.error = Some(e);
                vec![]
            }
        };
        p.timers = fx.iter().any(|f| f["t"] == "timers");
        if p.error.is_none() {
            p.ctx = Some(ctx);
        }
        self.list.push(p);
        fx.into_iter().map(|mut f| {
            f["plugin"] = json!(self.list.last().map(|p| p.man.id.clone()).unwrap_or_default());
            f
        }).collect()
    }

    // zavolá kód v plugine (príkaz, udalosť, časovač); vráti efekty
    pub fn call(&mut self, id: &str, code: &str, snap: &Value) -> Vec<Value> {
        let dl = self.deadline.clone();
        let Some(p) = self.list.iter_mut().find(|p| p.man.id == id) else { return vec![] };
        let Some(ctx) = p.ctx.as_ref() else { return vec![] };
        let res: Result<String, String> = ctx.with(|ctx| {
            dl.store(now_ms() + CALL_MS, Ordering::Relaxed);
            ctx.eval::<(), _>(format!("__setSnap({snap})")).map_err(|e| err_text(&ctx, e))?;
            let r = ctx.eval::<(), _>(code).map_err(|e| err_text(&ctx, e));
            // efekty aj po chybe (toast pred výnimkou)
            let out = ctx.eval::<String, _>("__out()").unwrap_or_else(|_| "[]".into());
            r.map(|_| out)
        });
        dl.store(u64::MAX, Ordering::Relaxed);
        match res {
            Ok(s) => {
                let mut fx = serde_json::from_str::<Vec<Value>>(&s).unwrap_or_default();
                if fx.iter().any(|f| f["t"] == "timers") {
                    p.timers = true;
                }
                for f in fx.iter_mut() {
                    f["plugin"] = json!(id);
                }
                fx
            }
            Err(e) => {
                let msg = format!("{}: {e}", p.man.name);
                self.log.push(msg.clone());
                vec![json!({ "t": "error", "plugin": id, "text": msg })]
            }
        }
    }

    pub fn unload(&mut self, id: &str) {
        let dl = self.deadline.clone();
        if let Some(p) = self.list.iter_mut().find(|p| p.man.id == id) {
            if let Some(ctx) = p.ctx.as_ref() {
                ctx.with(|ctx| {
                    dl.store(now_ms() + CALL_MS, Ordering::Relaxed);
                    let _ = ctx.eval::<(), _>("if (__plugin.deactivate) __plugin.deactivate();");
                });
                dl.store(u64::MAX, Ordering::Relaxed);
            }
        }
        self.list.retain(|p| p.man.id != id);
        THEMES.lock().unwrap().retain(|t| t.0 != id);
    }

    pub fn ids_with_timers(&self) -> Vec<String> {
        self.list.iter().filter(|p| p.timers && p.ctx.is_some()).map(|p| p.man.id.clone()).collect()
    }

    pub fn running(&self) -> Vec<String> {
        self.list.iter().filter(|p| p.ctx.is_some()).map(|p| p.man.id.clone()).collect()
    }
}

pub fn set_folder(f: Option<PathBuf>) {
    *FOLDER.lock().unwrap() = f;
}

// project.read: len súbory v otvorenom projekte (žiadne ../)
fn read_in_project(rel: &str) -> Option<String> {
    let root = FOLDER.lock().unwrap().clone()?;
    let p = if Path::new(rel).is_absolute() { PathBuf::from(rel) } else { root.join(rel) };
    let p = p.canonicalize().ok()?;
    if !p.starts_with(root.canonicalize().ok()?) {
        return None;
    }
    let meta = std::fs::metadata(&p).ok()?;
    if meta.len() > 2_000_000 {
        return None;
    }
    std::fs::read_to_string(p).ok()
}

fn project_files() -> String {
    let Some(root) = FOLDER.lock().unwrap().clone() else { return "[]".into() };
    let mut out = vec![];
    let mut stack = vec![root.clone()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || matches!(name.as_str(), "node_modules" | "target" | "__pycache__" | "dist" | "build" | "venv") {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(r) = p.strip_prefix(&root) {
                out.push(r.to_string_lossy().replace('\\', "/"));
                if out.len() >= 5000 {
                    return serde_json::to_string(&out).unwrap_or_default();
                }
            }
        }
    }
    out.sort();
    serde_json::to_string(&out).unwrap_or_default()
}

// téma z pluginu: farby podľa kľúčov ako v Electron Fluxe (THEME_KEYS), chýbajúce z basedOn/predvolenej
pub fn add_theme(plugin: &str, key: &str, def: &Value) {
    let base = def["basedOn"].as_str().and_then(crate::gen::code_theme).or_else(|| crate::gen::code_theme(if def["type"] == "light" { "vscode-light" } else { crate::gen::DEFAULT_THEME }));
    let Some((_, mut colors, _)) = base else { return };
    for (i, k) in crate::gen::THEME_KEYS.iter().enumerate() {
        if let Some(h) = def["colors"][*k].as_str() {
            if let Ok(c) = u32::from_str_radix(h.trim_start_matches('#'), 16) {
                colors[i] = c;
            }
        }
    }
    let dark = def["type"].as_str() != Some("light");
    let name = def["name"].as_str().unwrap_or(key).to_string();
    let mut t = THEMES.lock().unwrap();
    t.retain(|x| x.1 != key);
    t.push((plugin.to_string(), key.to_string(), name, dark, colors));
}

pub fn theme(key: &str) -> Option<(bool, [u32; 16], bool)> {
    THEMES.lock().unwrap().iter().find(|t| t.1 == key).map(|t| (t.3, t.4, false))
}

// jazyk pluginov = id jazyka z Monaca (python, javascript…) podľa prípony
pub fn lang_id(ext: &str) -> &'static str {
    match ext {
        "py" | "pyw" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "javascriptreact",
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "json" => "json",
        "md" | "markdown" => "markdown",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "cs" => "csharp",
        "java" => "java",
        "go" => "go",
        "rs" => "rust",
        "rb" => "ruby",
        "php" => "php",
        "lua" => "lua",
        "sh" => "shell",
        _ => "plaintext",
    }
}

pub fn timer_period() -> Duration {
    Duration::from_millis(250)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_plugin(name: &str, code: &str) -> Manifest {
        let dir = std::env::temp_dir().join(format!("flux-plug-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.json"), format!(r#"{{"id":"test.{name}","name":"{name}","version":"1.0.0","native":"native.js"}}"#)).unwrap();
        std::fs::write(dir.join("native.js"), code).unwrap();
        read_manifest(&dir, true).unwrap()
    }

    #[test]
    fn plugin_effects_and_calls() {
        let mut h = Host::new();
        let m = tmp_plugin(
            "a",
            r#"
export function activate(flux) {
  flux.commands.register('hi', 'Say hi', () => flux.toast('Hi ' + flux.activeFile().name));
  const s = flux.statusBar.add({ text: 'x' });
  flux.snippets.add('python', 'pr', 'print($0)', 'print');
  flux.themes.add('t1', { name: 'Test', type: 'dark', colors: { keyword: '#ff0000' } });
  flux.marks.set([{ line: 0, message: 'hey' }]);
  flux.storage.set('n', 1);
  flux.onSave(f => s.set('saved ' + f.name));
}"#,
        );
        let snap = json!({ "file": { "name": "a.py" }, "folder": null, "storage": {} });
        let fx = h.load(m, &snap, true);
        let kinds: Vec<&str> = fx.iter().filter_map(|f| f["t"].as_str()).collect();
        assert_eq!(kinds, vec!["cmd", "status", "snippet", "theme", "marks", "store"], "{fx:?}");
        let out = h.call("test.a", "__cmd('hi')", &snap);
        assert_eq!(out[0]["text"], "Hi a.py");
        let out = h.call("test.a", "__emit('save')", &snap);
        assert_eq!(out[0]["text"], "saved a.py");
    }

    #[test]
    fn plugin_endless_loop_is_stopped() {
        let mut h = Host::new();
        let m = tmp_plugin("loop", "export function activate(flux) { flux.commands.register('x', 'X', () => { while (true) {} }); }");
        let snap = json!({ "file": null, "folder": null, "storage": {} });
        h.load(m, &snap, true);
        let t = Instant::now();
        let out = h.call("test.loop", "__cmd('x')", &snap);
        assert!(t.elapsed() < Duration::from_secs(2));
        assert_eq!(out[0]["t"], "error");
        assert!(out[0]["text"].as_str().unwrap().contains("too long"), "{out:?}");
        // plugin žije ďalej
        assert!(h.running().contains(&"test.loop".to_string()));
    }

    #[test]
    fn plugin_scan_flags_eval() {
        assert!(!scan("eval('1')").is_empty());
        assert!(scan("flux.toast('ok')").is_empty());
    }
}
