// Nastavenia v tom istom súbore ako Electron verzia (%APPDATA%\Flux\settings.json, ~/.config/Flux) –
// prechod medzi verziami nič nestratí.
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn user_data() -> PathBuf {
    if let Ok(d) = std::env::var("FLUX_USER_DATA") {
        return PathBuf::from(d);
    }
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("Flux")
}

pub fn load() -> Value {
    let base = json!({ "recent": [], "theme": "dark", "pythonOverrides": {} });
    let file = user_data().join("settings.json");
    let saved: Value = std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}));
    let mut out = base;
    if let (Some(o), Some(s)) = (out.as_object_mut(), saved.as_object()) {
        for (k, v) in s {
            o.insert(k.clone(), v.clone());
        }
    }
    out
}

pub fn save(settings: &Value) {
    let dir = user_data();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(dir.join("settings.json"), text);
    }
}

pub fn platform() -> &'static str {
    match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        other => other,
    }
}

// Preklady: pribalené locales/<jazyk>.json (vo vývoji priamo z repozitára).
pub fn locale(app: &AppHandle, lang: &str) -> Value {
    if lang == "en" {
        return json!({});
    }
    let name = format!("{}.json", lang.replace(['/', '\\', '.'], ""));
    let mut dirs = vec![];
    if let Ok(r) = app.path().resource_dir() {
        dirs.push(r.join("locales"));
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../locales"));
    for d in dirs {
        if let Some(v) = std::fs::read_to_string(d.join(&name)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
            return v.get("strings").cloned().unwrap_or(v);
        }
    }
    json!({})
}

pub fn open_external(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
