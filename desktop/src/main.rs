// Flux v Ruste (Tauri): okno ukazuje to isté rozhranie ako Electron verzia (dist/renderer),
// všetko ostatné robí tento program. Rozhranie volá window.flux.* (src/bridge.js, generované z preload.js)
// → jeden príkaz „ipc“ s názvom kanála ako v Electrone, takže rozhranie netreba meniť.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fsops;
mod runner;
mod settings;

use serde_json::{json, Value};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub struct Flux {
    pub settings: Mutex<Value>,
    pub workspace: Mutex<Option<String>>,
    pub runner: runner::Runner,
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Null)
}
fn arg_str(args: &[Value], i: usize) -> String {
    args.get(i).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}

#[tauri::command]
async fn ipc(app: AppHandle, state: State<'_, Flux>, ch: String, args: Vec<Value>) -> Result<Value, String> {
    let s = &*state;
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
            let lang = s.settings.lock().unwrap().get("language").and_then(|v| v.as_str()).unwrap_or("en").to_string();
            Ok(settings::locale(&app, &lang))
        }
        "i18n:list" => Ok(json!([])),
        "app:memory" => Ok(json!({ "total": 0, "parts": [] })),
        "shell:open-external" => {
            let url = arg_str(&args, 0);
            if url.starts_with("https://") || url.starts_with("http://") {
                settings::open_external(&url);
            }
            Ok(Value::Null)
        }
        // ---------- projekty ----------
        "workspace:projects" => Ok(fsops::projects(s)),
        "workspace:open" => Ok(fsops::open_workspace(s, &arg_str(&args, 0))),
        "project:stats" => Ok(fsops::stats(&arg_str(&args, 0))),
        "project:root" => Ok(json!(dirs::document_dir().map(|d| d.join("Flux Projects").to_string_lossy().to_string()))),
        // ---------- súbory ----------
        "fs:list" => fsops::list(&arg_str(&args, 0)),
        "fs:list-all" => Ok(fsops::list_all(s)),
        "fs:read" => fsops::read(&arg_str(&args, 0), true),
        "fs:read-any" => fsops::read(&arg_str(&args, 0), false),
        "fs:write" => fsops::write(&arg_str(&args, 0), &arg_str(&args, 1)),
        "fs:create" => fsops::create(&arg_str(&args, 0), arg(&args, 1).as_bool().unwrap_or(false)),
        "fs:rename" => fsops::rename(&arg_str(&args, 0), &arg_str(&args, 1)),
        "file:recent" | "file:startup" | "plugins:active" | "plugins:installed" | "config:themes" | "plugins:builtin" | "app:background-history" | "update:notes" => Ok(json!([])),
        "update:state" => Ok(json!({ "version": app.package_info().version.to_string(), "status": "idle", "latest": "", "progress": 0, "error": "" })),
        "gh:info" => Ok(json!({ "connected": false })),
        "mcp:info" => Ok(json!({ "enabled": false })),
        "ai:config" => Ok(json!({})),
        "toolchains:status" => Ok(json!({})),
        // ---------- spúšťanie kódu ----------
        "run:file" => s.runner.run(&app, &arg_str(&args, 0), &arg_str(&args, 1), &arg_str(&args, 2)),
        "run:input" => {
            s.runner.input(&arg_str(&args, 0));
            Ok(Value::Null)
        }
        "run:stop" => {
            s.runner.stop();
            Ok(Value::Null)
        }
        // zatiaľ neprenesené časti – bezpečná prázdna odpoveď, rozhranie pokračuje
        _ => Ok(Value::Null),
    }
}

fn main() {
    tauri::Builder::default()
        .manage(Flux { settings: Mutex::new(settings::load()), workspace: Mutex::new(None), runner: runner::Runner::default() })
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Flux sa nepodarilo spustiť");
}
