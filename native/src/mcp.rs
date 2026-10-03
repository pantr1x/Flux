// MCP server Fluxu (port src/main/mcpServer.js): Claude Desktop, Claude Code, Cursor… pracujú s projektom otvoreným vo Fluxe.
// Beží len na 127.0.0.1, pýta si tajný kľúč (settings.mcpServer.token) a odmieta webové stránky (Origin).
// Most pre Claude Desktop: `Flux-Native.exe --mcp-bridge` číta JSON-RPC zo stdin a posiela ho bežiacemu Fluxu.
use flux_core::{settings, Core, Emit};
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 39217;
const PROTOCOL: &str = "2025-06-18";

pub struct Server {
    pub port: u16,
    stop: Arc<AtomicBool>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // prebudiť accept()
        let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], self.port).into(), Duration::from_millis(200));
    }
}

fn cfg(core: &Core) -> Value {
    core.setting("mcpServer")
}

fn save_cfg(core: &Core, f: impl FnOnce(&mut serde_json::Map<String, Value>)) {
    let mut set = core.settings.lock().unwrap();
    if let Some(o) = set.as_object_mut() {
        let mut m = o.get("mcpServer").and_then(|v| v.as_object().cloned()).unwrap_or_default();
        f(&mut m);
        o.insert("mcpServer".into(), Value::Object(m));
    }
    settings::save(&set);
}

// náhodný kľúč bez ďalšej knižnice: RandomState je zakaždým inak náhodne nasadený systémom
fn random_key() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut s = String::new();
    for i in 0..3u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i ^ std::process::id() as u64);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        s.push_str(&format!("{:016x}", h.finish()));
    }
    s
}

pub fn token(core: &Core) -> String {
    if let Some(t) = cfg(core)["token"].as_str().filter(|t| !t.is_empty()) {
        return t.to_string();
    }
    let t = random_key();
    let tk = t.clone();
    save_cfg(core, |m| {
        m.insert("token".into(), json!(tk));
    });
    t
}

pub fn new_key(core: &Core) {
    let t = random_key();
    save_cfg(core, |m| {
        m.insert("token".into(), json!(t));
    });
}

pub fn set_enabled(core: &Core, on: bool) {
    save_cfg(core, |m| {
        m.insert("enabled".into(), json!(on));
    });
}

pub fn enabled(core: &Core) -> bool {
    cfg(core)["enabled"].as_bool() == Some(true)
}

pub fn url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

pub fn start(core: Arc<Core>, emit: Emit) -> Result<Server, String> {
    let wanted = cfg(&core)["port"].as_u64().map(|p| p as u16).filter(|p| *p > 1024).unwrap_or(DEFAULT_PORT);
    let (listener, port) = (wanted..wanted + 20).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok().map(|l| (l, p))).ok_or("No free port for the MCP server.")?;
    if cfg(&core)["port"].as_u64() != Some(port as u64) {
        save_cfg(&core, |m| {
            m.insert("port".into(), json!(port));
        });
    }
    token(&core);
    let stop = Arc::new(AtomicBool::new(false));
    let st = stop.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            if st.load(Ordering::Relaxed) {
                break;
            }
            let Ok(c) = conn else { continue };
            let (core, emit) = (core.clone(), emit.clone());
            std::thread::spawn(move || {
                let _ = c.set_read_timeout(Some(Duration::from_secs(30)));
                serve(c, &core, &emit);
            });
        }
    });
    Ok(Server { port, stop })
}

fn respond(mut c: TcpStream, code: u16, body: &str) {
    let status = match code {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let _ = write!(c, "HTTP/1.1 {code} {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
}

// porovnanie bez skratky (rovnaký čas pri zlom kľúči)
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn serve(c: TcpStream, core: &Core, emit: &Emit) {
    let mut r = std::io::BufReader::new(match c.try_clone() {
        Ok(x) => x,
        Err(_) => return,
    });
    let mut first = String::new();
    if r.read_line(&mut first).is_err() {
        return;
    }
    let mut parts = first.split_whitespace();
    let (method, target) = (parts.next().unwrap_or("").to_string(), parts.next().unwrap_or("").to_string());
    let (mut len, mut auth, mut origin) = (0usize, String::new(), String::new());
    loop {
        let mut l = String::new();
        if r.read_line(&mut l).unwrap_or(0) == 0 || l.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => len = v.trim().parse().unwrap_or(0),
                "authorization" => auth = v.trim().to_string(),
                "origin" => origin = v.trim().to_string(),
                _ => {}
            }
        }
    }
    // webové stránky (DNS rebinding): prehliadač vždy posiela Origin
    if !origin.is_empty() {
        let host = origin.split("://").nth(1).unwrap_or("").split(':').next().unwrap_or("");
        if !(host == "127.0.0.1" || host.eq_ignore_ascii_case("localhost")) {
            return respond(c, 403, "");
        }
    }
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    if path != "/mcp" {
        return respond(c, 404, "\"Flux MCP server – use /mcp\"");
    }
    let given = auth.strip_prefix("Bearer ").map(String::from).or_else(|| query.split('&').find_map(|kv| kv.strip_prefix("token=").map(String::from))).unwrap_or_default();
    if !same(&given, &token(core)) {
        return respond(c, 401, &json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32001, "message": "Wrong or missing Flux MCP key" } }).to_string());
    }
    if method != "POST" {
        return respond(c, 405, "");
    }
    if len > 20 * 1024 * 1024 {
        return respond(c, 400, "");
    }
    let mut body = vec![0u8; len];
    if r.read_exact(&mut body).is_err() {
        return respond(c, 400, "");
    }
    let Ok(msg) = serde_json::from_slice::<Value>(&body) else {
        return respond(c, 400, &json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "Parse error" } }).to_string());
    };
    let list = if let Value::Array(a) = &msg { a.clone() } else { vec![msg.clone()] };
    let answers: Vec<Value> = list.iter().filter_map(|m| handle(m, core, emit)).collect();
    if answers.is_empty() {
        return respond(c, 202, "");
    }
    let out = if msg.is_array() { json!(answers) } else { answers[0].clone() };
    respond(c, 200, &out.to_string());
}

fn handle(msg: &Value, core: &Core, emit: &Emit) -> Option<Value> {
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = msg["method"].as_str().filter(|_| msg["jsonrpc"] == "2.0") else {
        return Some(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32600, "message": "Invalid request" } }));
    };
    if id.is_null() {
        return None; // notifikácia – bez odpovede
    }
    let params = &msg["params"];
    let result = match method {
        "initialize" => json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or(PROTOCOL),
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "flux", "title": "Flux", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Flux is the code editor on this computer. Use these tools to see and change the project that is open in Flux: its files, description and to-do list. Changes show up in Flux right away.",
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => {
            let (text, err) = match call(core, params["name"].as_str().unwrap_or(""), &params["arguments"]) {
                Ok(v) => (if let Value::String(s) = v { s } else { serde_json::to_string_pretty(&v).unwrap_or_default() }, false),
                Err(e) => (e, true),
            };
            if !err {
                emit("mcp:changed", json!({}));
            }
            json!({ "content": [{ "type": "text", "text": text }], "isError": err })
        }
        _ => return Some(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("Method not found: {method}") } })),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

// rovnaké nástroje ako v ai.js (TOOLS)
pub fn tools() -> Value {
    let obj = |props: Value, req: &[&str]| json!({ "type": "object", "properties": props, "required": req, "additionalProperties": false });
    json!([
        { "name": "flux_project_info", "description": "Get the open Flux project: name, folder, type, short description and to-do list (with ids and done state), plus the files in the project. Call this before answering questions about the project.", "inputSchema": obj(json!({}), &[]) },
        { "name": "flux_read_file", "description": "Read a text file from the open project. Path is relative to the project folder, e.g. \"index.html\" or \"src/main.py\".", "inputSchema": obj(json!({ "path": { "type": "string" } }), &["path"]) },
        { "name": "flux_add_todos", "description": "Add one or more tasks to the project's to-do list shown on the project page.", "inputSchema": obj(json!({ "tasks": { "type": "array", "items": { "type": "string" } } }), &["tasks"]) },
        { "name": "flux_update_todo", "description": "Mark a to-do as done or not done, rename it, or remove it. Use the id from flux_project_info.", "inputSchema": obj(json!({ "id": { "type": "number" }, "done": { "type": "boolean" }, "text": { "type": "string" }, "remove": { "type": "boolean" } }), &["id"]) },
        { "name": "flux_set_description", "description": "Set the short description of the project (one sentence, shown under the project name).", "inputSchema": obj(json!({ "description": { "type": "string" } }), &["description"]) },
    ])
}

fn walk(root: &Path, d: &Path, depth: usize, out: &mut Vec<String>) {
    if depth > 6 || out.len() > 400 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(d) else { return };
    let mut items: Vec<_> = rd.flatten().collect();
    items.sort_by_key(|e| e.file_name());
    for e in items {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || matches!(name.as_str(), "node_modules" | "__pycache__" | "venv" | "target" | "dist") {
            continue;
        }
        let p = e.path();
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            walk(root, &p, depth + 1, out);
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn meta_update(core: &Core, ws: &str, f: impl FnOnce(&mut serde_json::Map<String, Value>) -> Result<Value, String>) -> Result<Value, String> {
    let mut set = core.settings.lock().unwrap();
    let Some(o) = set.as_object_mut() else { return Err("Settings are not available.".into()) };
    let mut all = o.get("projectMeta").cloned().filter(|v| v.is_object()).unwrap_or(json!({}));
    let mut m = all[ws].as_object().cloned().unwrap_or_default();
    let out = f(&mut m)?;
    all[ws] = Value::Object(m);
    o.insert("projectMeta".into(), all);
    settings::save(&set);
    Ok(out)
}

pub fn call(core: &Core, name: &str, input: &Value) -> Result<Value, String> {
    let ws = core.workspace.lock().unwrap().clone().ok_or("No project is open in Flux. Open one first.")?;
    match name {
        "flux_project_info" => {
            let mut files = vec![];
            walk(Path::new(&ws), Path::new(&ws), 0, &mut files);
            let meta = core.setting("projectMeta")[&ws].clone();
            let s = flux_core::fsops::summary(&ws);
            let todos: Vec<Value> =
                meta["todos"].as_array().cloned().unwrap_or_default().iter().map(|x| json!({ "id": x["id"], "text": x["text"], "done": x["done"].as_bool() == Some(true) })).collect();
            Ok(json!({
                "name": Path::new(&ws).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                "folder": ws,
                "type": s["langs"][0].as_str().unwrap_or("files"),
                "description": meta["description"].as_str().unwrap_or(""),
                "todos": todos,
                "files": files,
            }))
        }
        "flux_read_file" => {
            let rel = input["path"].as_str().ok_or("Invalid input: \"path\" must be a string.")?;
            let file: PathBuf = Path::new(&ws).join(rel);
            let (Ok(f), Ok(root)) = (file.canonicalize(), Path::new(&ws).canonicalize()) else { return Err(format!("File not found: {rel}")) };
            if !f.starts_with(&root) {
                return Err("That file is outside the project.".into());
            }
            if f.metadata().map(|m| m.len()).unwrap_or(0) > 400 * 1024 {
                return Err("The file is too large to read (over 400 KB).".into());
            }
            std::fs::read_to_string(&f).map(Value::String).map_err(|_| format!("File not found: {rel}"))
        }
        "flux_add_todos" => {
            let tasks: Vec<String> = input["tasks"]
                .as_array()
                .ok_or("Invalid input: \"tasks\" must be a list.")?
                .iter()
                .map(|x| x.as_str().map(String::from).ok_or("Invalid input: tasks must be strings."))
                .collect::<Result<_, _>>()?;
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            meta_update(core, &ws, |m| {
                let mut l = m.get("todos").and_then(|v| v.as_array().cloned()).unwrap_or_default();
                let added: Vec<Value> = tasks.iter().enumerate().map(|(i, t)| json!({ "id": now + i as u64, "text": t.chars().take(200).collect::<String>(), "done": false })).collect();
                l.extend(added.iter().cloned());
                m.insert("todos".into(), json!(l));
                Ok(json!({ "added": added.iter().map(|x| json!({ "id": x["id"], "text": x["text"] })).collect::<Vec<_>>() }))
            })
        }
        "flux_update_todo" => {
            let id = input["id"].as_f64().ok_or("Invalid input: \"id\" must be a number.")?;
            meta_update(core, &ws, |m| {
                let mut l = m.get("todos").and_then(|v| v.as_array().cloned()).unwrap_or_default();
                let i = l.iter().position(|x| x["id"].as_f64() == Some(id)).ok_or(format!("No to-do with id {id}."))?;
                if input["remove"].as_bool() == Some(true) {
                    l.remove(i);
                } else {
                    if let Some(d) = input["done"].as_bool() {
                        l[i]["done"] = json!(d);
                    }
                    if let Some(t) = input["text"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                        l[i]["text"] = json!(t.chars().take(200).collect::<String>());
                    }
                }
                m.insert("todos".into(), json!(l));
                Ok(json!("ok"))
            })
        }
        "flux_set_description" => {
            let d = input["description"].as_str().ok_or("Invalid input: \"description\" must be a string.")?.trim().chars().take(160).collect::<String>();
            meta_update(core, &ws, |m| {
                m.insert("description".into(), json!(d));
                Ok(json!("ok"))
            })
        }
        _ => Err(format!("Unknown tool: {name}")),
    }
}

// ---------- Claude Desktop ----------

// claude_desktop_config.json (aj verzia z Microsoft Store má nastavenia v priečinku balíka)
fn claude_desktop_paths() -> Vec<PathBuf> {
    let mut list = vec![];
    if let Some(d) = dirs::config_dir() {
        list.push(d.join("Claude").join("claude_desktop_config.json"));
    }
    #[cfg(windows)]
    if let Some(local) = dirs::data_local_dir() {
        if let Ok(rd) = std::fs::read_dir(local.join("Packages")) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with("Claude_") || n.starts_with("AnthropicPBC.Claude_") {
                    list.push(e.path().join("LocalCache").join("Roaming").join("Claude").join("claude_desktop_config.json"));
                }
            }
        }
    }
    list
}

// zapíše mcpServers.flux = { command: <tento program>, args: ["--mcp-bridge"] } → cesta k súboru
pub fn add_to_claude_desktop() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let entry = json!({ "command": exe.to_string_lossy(), "args": ["--mcp-bridge"] });
    let mut written: Vec<PathBuf> = vec![];
    for file in claude_desktop_paths() {
        let mut data = match std::fs::read_to_string(&file) {
            Ok(t) if t.trim().is_empty() => json!({}),
            Ok(t) => serde_json::from_str::<Value>(&t).map_err(|e| format!("Claude Desktop settings could not be read: {e}"))?,
            Err(_) => {
                // priečinok balíka zo Store bez nastavení – stačí bežné miesto
                if !written.is_empty() && !file.parent().is_some_and(|p| p.exists()) {
                    continue;
                }
                json!({})
            }
        };
        if !data.is_object() {
            data = json!({});
        }
        if !data["mcpServers"].is_object() {
            data["mcpServers"] = json!({});
        }
        data["mcpServers"]["flux"] = entry.clone();
        if let Some(p) = file.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        std::fs::write(&file, serde_json::to_string_pretty(&data).unwrap_or_default()).map_err(|e| e.to_string())?;
        written.push(file);
    }
    written.first().map(|f| f.to_string_lossy().to_string()).ok_or("Claude Desktop settings folder was not found.".into())
}

// ---------- most (Flux-Native.exe --mcp-bridge) ----------

// procesy mosta nemajú okno – boot::kill_ghosts ich podľa tohto súboru nechá bežať
pub fn bridge_mark(pid: u32) -> PathBuf {
    settings::user_data().join("mcp-bridges").join(pid.to_string())
}

fn post(port: u16, token: &str, body: &str) -> std::io::Result<(u16, String)> {
    let mut c = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_secs(2))?;
    c.set_read_timeout(Some(Duration::from_secs(120)))?;
    write!(c, "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())?;
    let mut out = String::new();
    c.read_to_string(&mut out)?;
    let code = out.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let body = out.split_once("\r\n\r\n").map(|x| x.1.to_string()).unwrap_or_default();
    Ok((code, body))
}

// Flux na pozadí: bez zdedeného stdin/stdout (výpisy by pokazili JSON-RPC) a na Windows mimo úlohy (job) Claude Desktop,
// aby sa nezavrel spolu s mostom
fn launch(exe: &Path) {
    let cmd = || {
        let mut c = std::process::Command::new(exe);
        c.arg("--background").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        c
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_BREAKAWAY_FROM_JOB | DETACHED_PROCESS; ak úloha odchod nedovolí, aspoň DETACHED_PROCESS
        if cmd().creation_flags(0x0100_0000 | 0x0000_0008).spawn().is_ok() {
            return;
        }
        let _ = cmd().creation_flags(0x0000_0008).spawn();
    }
    #[cfg(not(windows))]
    let _ = cmd().spawn();
}

pub fn bridge() {
    let me = std::process::id();
    let mark = bridge_mark(me);
    let _ = std::fs::create_dir_all(mark.parent().unwrap());
    let _ = std::fs::write(&mark, "");
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut launched = false;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let id = serde_json::from_str::<Value>(&line).ok().and_then(|v| v.get("id").cloned()).unwrap_or(Value::Null);
        let fail = |msg: &str| json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": msg } }).to_string();
        let mut answer = None;
        // Flux nebeží → spustiť ho na pozadí a chvíľu počkať
        for attempt in 0..40 {
            let s = settings::load();
            let m = &s["mcpServer"];
            if m["enabled"].as_bool() != Some(true) {
                answer = Some(fail("Flux for other AI apps is turned off. Turn it on in Flux: Settings → AI → Use Flux from other AI apps."));
                break;
            }
            let port = m["port"].as_u64().map(|p| p as u16).unwrap_or(DEFAULT_PORT);
            match post(port, m["token"].as_str().unwrap_or(""), &line) {
                Ok((202, _)) => break,
                Ok((_, b)) if !b.trim().is_empty() => {
                    answer = Some(b);
                    break;
                }
                Ok((code, _)) => {
                    answer = Some(fail(&format!("Flux answered with HTTP {code}.")));
                    break;
                }
                Err(_) if !launched && attempt == 0 => {
                    launched = true;
                    if let Ok(exe) = std::env::current_exe() {
                        launch(&exe);
                    }
                }
                Err(_) => {}
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        if !id.is_null() {
            let a = answer.unwrap_or_else(|| fail("Flux is not running and could not be started."));
            let _ = writeln!(out, "{}", a.trim());
            let _ = out.flush();
        }
    }
    let _ = std::fs::remove_file(&mark);
}
