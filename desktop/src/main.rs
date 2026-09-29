// Flux v Ruste (Tauri): okno ukazuje to isté rozhranie ako Electron verzia (dist/renderer),
// všetko ostatné robí tento program. Rozhranie volá window.flux.* (src/bridge.js, generované z preload.js)
// → jeden príkaz „ipc“ s názvom kanála ako v Electrone, takže rozhranie netreba meniť.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use flux_core::{fsops, pty, python, runner, settings, Core, Emit};

use notify::{RecursiveMode, Watcher};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

pub struct Flux {
    pub core: Core,
    pub run: pty::Pty,
    pub shell: pty::Pty,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

// nastavenia a projekt sú v spoločnom jadre (flux-core)
impl std::ops::Deref for Flux {
    type Target = Core;
    fn deref(&self) -> &Core {
        &self.core
    }
}

// udalosti z jadra → okno
fn emitter(app: &AppHandle) -> Emit {
    let app = app.clone();
    std::sync::Arc::new(move |ch: &str, v: Value| {
        let _ = app.emit(ch, v);
    })
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Null)
}
fn arg_str(args: &[Value], i: usize) -> String {
    args.get(i).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn arg_bool(args: &[Value], i: usize) -> bool {
    args.get(i).and_then(|v| v.as_bool()).unwrap_or(false)
}

// Zmeny v projekte (aj z iných programov) → fs:changed, najviac raz za 150 ms.
fn watch(app: &AppHandle, s: &Flux, dir: &str) {
    let app2 = app.clone();
    let last = Mutex::new(Instant::now() - Duration::from_secs(1));
    let w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        let ignored = ev.paths.iter().all(|p| p.components().any(|c| matches!(c.as_os_str().to_str(), Some("node_modules" | ".git" | "__pycache__" | ".venv" | "venv"))));
        if ignored {
            return;
        }
        let mut l = last.lock().unwrap();
        if l.elapsed() < Duration::from_millis(150) {
            return;
        }
        *l = Instant::now();
        let app3 = app2.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            let _ = app3.emit("fs:changed", Value::Null);
        });
    });
    let mut slot = s.watcher.lock().unwrap();
    *slot = None;
    if let Ok(mut w) = w {
        if w.watch(std::path::Path::new(dir), RecursiveMode::Recursive).is_ok() {
            *slot = Some(w);
        }
    }
}

async fn pick_folder(title: &str) -> Option<String> {
    rfd::AsyncFileDialog::new().set_title(title).pick_folder().await.map(|f| f.path().to_string_lossy().to_string())
}

#[tauri::command]
async fn ipc(app: AppHandle, state: State<'_, Flux>, ch: String, args: Vec<Value>) -> Result<Value, String> {
    let s = &*state;
    let ws = || s.workspace.lock().unwrap().clone();
    let setting = |k: &str| s.settings.lock().unwrap().get(k).cloned().unwrap_or(Value::Null);
    match ch.as_str() {
        "log" => {
            eprintln!("[okno] {}", arg_str(&args, 0));
            Ok(Value::Null)
        }
        // ---------- aplikácia a nastavenia ----------
        "app:init" => {
            let set = s.settings.lock().unwrap().clone();
            let last = set.get("lastFolder").and_then(|v| v.as_str()).filter(|d| std::path::Path::new(d).is_dir()).map(String::from);
            Ok(json!({
                "platform": settings::platform(),
                "mica": false,
                "material": "none",
                "hasWallpaper": false,
                "settings": set,
                "version": app.package_info().version.to_string(),
                "home": dirs::home_dir().map(|h| h.to_string_lossy().to_string()),
                "lastFolder": last,
            }))
        }
        "app:set-settings" => {
            let mut set = s.settings.lock().unwrap();
            if let (Some(obj), Some(patch)) = (set.as_object_mut(), arg(&args, 0).as_object()) {
                for (k, v) in patch {
                    obj.insert(k.clone(), v.clone());
                }
            }
            settings::save(&set);
            Ok(set.clone())
        }
        "i18n:current" => {
            let lang = setting("language").as_str().unwrap_or("en").to_string();
            Ok(settings::locale(app.path().resource_dir().ok(), &lang))
        }
        "i18n:list" => Ok(settings::languages(app.path().resource_dir().ok())),
        "i18n:use" => {
            let code = arg_str(&args, 0);
            let data = settings::locale(app.path().resource_dir().ok(), &code);
            let mut set = s.settings.lock().unwrap();
            set["language"] = json!(code);
            settings::save(&set);
            Ok(data)
        }
        "app:zoom" => {
            let z = arg(&args, 0).as_f64().unwrap_or(1.0).clamp(0.7, 1.6);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_zoom(z);
            }
            Ok(Value::Null)
        }
        "win:fullscreen" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_fullscreen(arg_bool(&args, 0));
            }
            Ok(Value::Null)
        }
        "app:reload" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.eval("location.reload()");
            }
            Ok(Value::Null)
        }
        "app:close" => {
            app.exit(0);
            Ok(Value::Null)
        }
        "shell:open-external" => {
            let url = arg_str(&args, 0);
            if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") {
                settings::open_external(&url);
            }
            Ok(Value::Null)
        }
        "app:memory" => Ok(json!({ "total": 0, "parts": [] })),
        // ---------- projekty ----------
        "workspace:projects" => Ok(fsops::projects(s)),
        "workspace:open" => {
            let dir = arg_str(&args, 0);
            let r = fsops::open_workspace(s, &dir);
            if !r.is_null() {
                watch(&app, s, &dir);
            }
            Ok(r)
        }
        "workspace:open-dialog" => match pick_folder("Open folder").await {
            Some(dir) => {
                let r = fsops::open_workspace(s, &dir);
                watch(&app, s, &dir);
                Ok(r)
            }
            None => Ok(Value::Null),
        },
        "workspace:forget" => Ok(fsops::forget(s, &arg_str(&args, 0))),
        "project:pin" => Ok(fsops::pin(s, &arg_str(&args, 0), arg_bool(&args, 1))),
        "project:hide" => Ok(fsops::hide(s, &arg_str(&args, 0), arg_bool(&args, 1))),
        "project:rename" => fsops::rename_project(s, &arg_str(&args, 0), &arg_str(&args, 1)),
        "project:trash" => {
            let dir = arg_str(&args, 0);
            let r = fsops::trash(&dir).await?;
            if r == json!(true) {
                fsops::forget(s, &dir);
            }
            Ok(r)
        }
        "project:stats" => Ok(fsops::stats(&arg_str(&args, 0))),
        "project:root" => Ok(json!(fsops::default_root().to_string_lossy())),
        "project:create" => fsops::create_project(&arg_str(&args, 0), &arg_str(&args, 1)),
        "project:choose-root" => Ok(json!(pick_folder("Where to save the new project").await)),
        // ---------- súbory ----------
        "fs:list" => fsops::list(&arg_str(&args, 0)),
        "fs:list-all" => Ok(fsops::list_all(s)),
        "fs:read" => fsops::read(&arg_str(&args, 0), true),
        "fs:read-any" => fsops::read(&arg_str(&args, 0), false),
        "fs:read-image" => fsops::read_image(&arg_str(&args, 0)),
        "fs:write" => fsops::write(&arg_str(&args, 0), &arg_str(&args, 1)),
        "fs:create" => fsops::create(&arg_str(&args, 0), arg_bool(&args, 1)),
        "fs:rename" => fsops::rename(&arg_str(&args, 0), &arg_str(&args, 1)),
        "fs:trash" => fsops::trash(&arg_str(&args, 0)).await,
        "fs:reveal" => {
            fsops::reveal(&arg_str(&args, 0));
            Ok(Value::Null)
        }
        "file:open-dialog" => {
            let files = rfd::AsyncFileDialog::new().set_title("Open file").pick_files().await.unwrap_or_default();
            Ok(Value::Array(files.iter().map(|f| fsops::allow_file(s, &f.path().to_string_lossy())).filter(|v| !v.is_null()).collect()))
        }
        "file:new-dialog" => {
            let ext: String = arg_str(&args, 0).chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
            let ext = if ext.is_empty() { "txt".to_string() } else { ext };
            let dir = dirs::document_dir().unwrap_or_default().join("Flux Files");
            let _ = std::fs::create_dir_all(&dir);
            let mut name = format!("untitled.{ext}");
            let mut n = 1;
            while dir.join(&name).exists() {
                n += 1;
                name = format!("untitled-{n}.{ext}");
            }
            match rfd::AsyncFileDialog::new().set_title("New file").set_directory(&dir).set_file_name(&name).save_file().await {
                Some(f) => {
                    let p = f.path().to_path_buf();
                    if !p.exists() {
                        let _ = std::fs::write(&p, "");
                    }
                    Ok(fsops::allow_file(s, &p.to_string_lossy()))
                }
                None => Ok(Value::Null),
            }
        }
        "file:allow" | "file:open-recent" => Ok(fsops::allow_file(s, &arg_str(&args, 0))),
        "file:recent" => Ok(fsops::recent_files(s)),
        "file:forget-recent" => Ok(fsops::forget_recent(s, &arg_str(&args, 0))),
        "file:startup" => Ok(Value::Array(std::env::args().skip(1).filter(|a| !a.starts_with('-') && std::path::Path::new(a).is_file()).map(|a| fsops::allow_file(s, &a)).filter(|v| !v.is_null()).collect())),
        // ---------- Python a spúšťanie ----------
        "python:find" => {
            let w = ws();
            let chosen = w.as_ref().and_then(|w| setting("pythonOverrides").get(w).and_then(|v| v.as_str()).map(String::from));
            let (w2, c2) = (w.clone(), chosen.clone());
            Ok(tauri::async_runtime::spawn_blocking(move || python::find(w2.as_deref(), c2.as_deref())).await.unwrap_or(Value::Null))
        }
        "python:choose" => {
            let Some(f) = rfd::AsyncFileDialog::new().set_title("Select Python interpreter").pick_file().await else { return Ok(Value::Null) };
            let p = f.path().to_string_lossy().to_string();
            let Some(info) = python::probe(&p, &[]) else { return Err("The selected file is not a working Python.".into()) };
            if let Some(w) = ws() {
                let mut set = s.settings.lock().unwrap();
                if !set["pythonOverrides"].is_object() {
                    set["pythonOverrides"] = json!({});
                }
                set["pythonOverrides"][w] = json!(p);
                settings::save(&set);
            }
            Ok(info)
        }
        "python:reset" => {
            if let Some(w) = ws() {
                let mut set = s.settings.lock().unwrap();
                if let Some(o) = set["pythonOverrides"].as_object_mut() {
                    o.remove(&w);
                }
                settings::save(&set);
            }
            Ok(json!(true))
        }
        "run:file" => Ok(runner::run_file(&s.run, &emitter(&app), &arg_str(&args, 0), &arg_str(&args, 1), &arg_str(&args, 2))),
        "run:pip" => {
            let pkg = arg_str(&args, 1);
            if pkg.is_empty() || !pkg.chars().all(|c| c.is_alphanumeric() || "._-[]".contains(c)) {
                return Ok(json!({ "ok": false, "error": "Invalid package name." }));
            }
            let cmd = runner::Cmd { cmd: arg_str(&args, 0), args: vec!["-m".into(), "pip".into(), "install".into(), pkg.clone()] };
            Ok(runner::start(&s.run, &emitter(&app), &cmd, &ws().unwrap_or_default(), &format!("pip install {pkg}")))
        }
        "run:create-venv" => {
            let Some(w) = ws() else { return Ok(json!({ "ok": false })) };
            let cmd = runner::Cmd { cmd: arg_str(&args, 0), args: vec!["-m".into(), "venv".into(), ".venv".into()] };
            Ok(runner::start(&s.run, &emitter(&app), &cmd, &w, "python -m venv .venv"))
        }
        "run:input" => {
            s.run.write(&arg_str(&args, 0));
            Ok(Value::Null)
        }
        "run:stop" => {
            s.run.kill();
            Ok(Value::Null)
        }
        "run:resize" => {
            s.run.resize(arg(&args, 0).as_u64().unwrap_or(100) as u16, arg(&args, 1).as_u64().unwrap_or(24) as u16);
            Ok(Value::Null)
        }
        // ---------- terminál ----------
        "shell:start" => {
            if arg_bool(&args, 0) {
                s.shell.kill();
            } else if s.shell.running() {
                return Ok(json!(true));
            }
            let cmd = runner::shell_command();
            let cwd = ws().filter(|d| std::path::Path::new(d).is_dir()).or_else(|| dirs::home_dir().map(|h| h.to_string_lossy().to_string())).unwrap_or_default();
            match s.shell.spawn(&emitter(&app), &cmd.cmd, &cmd.args, &cwd, &[], "shell:data", "shell:exit") {
                Ok(_) => Ok(json!(true)),
                Err(e) => {
                    let _ = app.emit("shell:data", format!("\r\n{e}\r\n"));
                    Ok(json!(false))
                }
            }
        }
        "shell:input" => {
            s.shell.write(&arg_str(&args, 0));
            Ok(Value::Null)
        }
        "shell:resize" => {
            s.shell.resize(arg(&args, 0).as_u64().unwrap_or(100) as u16, arg(&args, 1).as_u64().unwrap_or(20) as u16);
            Ok(Value::Null)
        }
        "shell:kill" => {
            s.shell.kill();
            Ok(Value::Null)
        }
        // ---------- zatiaľ neprenesené: bezpečné odpovede, aby rozhranie fungovalo ----------
        "plugins:active" | "plugins:installed" | "config:themes" | "plugins:builtin" | "app:background-history" | "update:notes" | "plugins:registry" => Ok(json!([])),
        "update:state" => Ok(json!({ "version": app.package_info().version.to_string(), "status": "idle", "latest": "", "progress": 0, "error": "" })),
        "gh:info" => Ok(json!({ "connected": false })),
        "mcp:info" => Ok(json!({ "enabled": false })),
        "ai:config" => Ok(json!({})),
        "toolchains:status" => Ok(json!({ "list": [], "canInstall": false })),
        _ => Ok(Value::Null),
    }
}

fn main() {
    tauri::Builder::default()
        .manage(Flux { core: Core::load(), run: pty::Pty::default(), shell: pty::Pty::default(), watcher: Mutex::new(None) })
        .invoke_handler(tauri::generate_handler![ipc])
        .setup(|app| {
            // okno s mostom window.flux (bridge.js beží pred app.js)
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                .title("Flux")
                .inner_size(1400.0, 900.0)
                .min_inner_size(760.0, 480.0)
                .background_color(tauri::window::Color(0x1e, 0x1c, 0x19, 0xff))
                .initialization_script(include_str!("bridge.js"))
                .build()?;
            // testy (len vývojová zostava): FLUX_TEST_JS sa spustí v okne po načítaní
            #[cfg(debug_assertions)]
            if let Ok(js) = std::env::var("FLUX_TEST_JS") {
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(8));
                    if let Some(w) = h.get_webview_window("main") {
                        let _ = w.eval(&js);
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Flux sa nepodarilo spustiť");
}
