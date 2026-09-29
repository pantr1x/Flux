// Súbory a projekty – rovnaké odpovede ako Electron verzia (src/main/main.js), aby rozhranie fungovalo bez zmien.
use crate::{settings, Core as Flux};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const IGNORED: &[&str] = &["node_modules", ".git", "__pycache__", ".mypy_cache", ".pytest_cache", ".ruff_cache", ".idea", ".vs", ".venv", "venv"];

fn name_of(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

pub fn list(dir: &str) -> Result<Value, String> {
    let mut items: Vec<(bool, String, String)> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if IGNORED.contains(&name.as_str()) {
                return None;
            }
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            Some((is_dir, name, e.path().to_string_lossy().to_string()))
        })
        .collect();
    // priečinky prvé, potom podľa mena (bez ohľadu na veľké písmená)
    items.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    Ok(Value::Array(items.into_iter().map(|(d, n, p)| json!({ "name": n, "path": p, "dir": d })).collect()))
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 12 || out.len() > 5000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if IGNORED.contains(&name.as_str()) {
            continue;
        }
        let p = e.path();
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            walk(&p, depth + 1, out);
        } else {
            out.push(p);
        }
    }
}

pub fn list_all(s: &Flux) -> Value {
    let ws = s.workspace.lock().unwrap().clone();
    let mut out = vec![];
    if let Some(w) = ws {
        walk(Path::new(&w), 0, &mut out);
    }
    Value::Array(out.into_iter().map(|p| json!(p.to_string_lossy())).collect())
}

pub fn read(file: &str, text_only: bool) -> Result<Value, String> {
    let meta = std::fs::metadata(file).map_err(|e| e.to_string())?;
    if meta.len() > 5 * 1024 * 1024 {
        return Err("The file is too large (over 5 MB).".into());
    }
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    if text_only && bytes.iter().take(8000).any(|b| *b == 0) {
        return Err("This is a binary file – the editor cannot show it.".into());
    }
    Ok(json!(String::from_utf8_lossy(&bytes)))
}

pub fn write(file: &str, content: &str) -> Result<Value, String> {
    std::fs::write(file, content).map_err(|e| e.to_string())?;
    Ok(json!(true))
}

pub fn create(target: &str, is_dir: bool) -> Result<Value, String> {
    let p = Path::new(target);
    if p.exists() {
        return Err("Already exists".into());
    }
    if is_dir {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    } else {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(p, "").map_err(|e| e.to_string())?;
    }
    Ok(json!(target))
}

pub fn rename(from: &str, to: &str) -> Result<Value, String> {
    if Path::new(to).exists() {
        return Err("Already exists".into());
    }
    std::fs::rename(from, to).map_err(|e| e.to_string())?;
    Ok(json!(to))
}

pub fn projects(s: &Flux) -> Value {
    let set = s.settings.lock().unwrap();
    let recent: Vec<String> = set.get("recent").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    let list: Vec<Value> = set.get("projects").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut out: Vec<Value> = list
        .iter()
        .filter_map(|p| {
            let dir = p.get("dir")?.as_str()?;
            if !Path::new(dir).is_dir() {
                return None;
            }
            let pinned = p.get("pinned").and_then(|v| v.as_bool()).unwrap_or(false);
            Some(json!({
                "dir": dir,
                "pinned": pinned,
                "hidden": p.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false),
                "name": name_of(Path::new(dir)),
                "kind": "folder",
                "langs": [],
                "github": false,
                "recent": recent.iter().position(|r| r == dir).map(|i| i as i64).unwrap_or(-1),
            }))
        })
        .collect();
    out.sort_by_key(|p| !p["pinned"].as_bool().unwrap_or(false));
    Value::Array(out)
}

pub fn open_workspace(s: &Flux, dir: &str) -> Value {
    if !Path::new(dir).is_dir() {
        return Value::Null;
    }
    *s.workspace.lock().unwrap() = Some(dir.to_string());
    let mut set = s.settings.lock().unwrap();
    if let Some(o) = set.as_object_mut() {
        let mut recent: Vec<Value> = o.get("recent").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        recent.retain(|r| r.as_str() != Some(dir));
        recent.insert(0, json!(dir));
        recent.truncate(8);
        o.insert("recent".into(), Value::Array(recent));
        let mut projects: Vec<Value> = o.get("projects").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        if !projects.iter().any(|p| p.get("dir").and_then(|d| d.as_str()) == Some(dir)) {
            projects.push(json!({ "dir": dir, "pinned": false }));
        }
        o.insert("projects".into(), Value::Array(projects));
        o.insert("lastFolder".into(), json!(dir));
    }
    settings::save(&set);
    json!(dir)
}

pub fn stats(dir: &str) -> Value {
    let mut files = vec![];
    walk(Path::new(dir), 0, &mut files);
    json!({ "files": files.len(), "lines": 0, "chars": 0, "lastModified": 0, "kinds": {}, "time": 0 })
}

// Súhrn projektu pre bočný panel a stránku projektu: počet súborov a riadkov, jazyky (od najčastejšieho),
// posledná zmena (ms od 1970). Jazyky majú rovnaké mená ako „kind“ v Electron Fluxe (python, web, node…).
pub fn summary(dir: &str) -> Value {
    let mut files = vec![];
    walk(Path::new(dir), 0, &mut files);
    let mut kinds: Vec<(&str, usize)> = vec![];
    let (mut lines, mut last) = (0usize, 0u128);
    for f in &files {
        let ext = f.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let kind = match ext.as_str() {
            "py" | "pyw" => "python",
            "html" | "htm" | "css" | "scss" => "web",
            "js" | "mjs" | "cjs" | "ts" | "jsx" | "tsx" => "node",
            "java" => "java",
            "c" | "h" | "cpp" | "cc" | "hpp" => "cpp",
            "go" => "go",
            "cs" => "csharp",
            "rs" => "rust",
            "rb" => "ruby",
            "php" => "php",
            "lua" => "lua",
            "zig" => "zig",
            "r" => "r",
            "jl" => "julia",
            _ => "",
        };
        let Ok(meta) = f.metadata() else { continue };
        if let Ok(m) = meta.modified() {
            last = last.max(m.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
        }
        if kind.is_empty() {
            continue;
        }
        match kinds.iter_mut().find(|k| k.0 == kind) {
            Some(k) => k.1 += 1,
            None => kinds.push((kind, 1)),
        }
        if meta.len() < 1_000_000 {
            if let Ok(t) = std::fs::read_to_string(f) {
                lines += t.lines().count();
            }
        }
    }
    kinds.sort_by(|a, b| b.1.cmp(&a.1));
    json!({ "files": files.len(), "lines": lines, "langs": kinds.iter().map(|k| k.0).collect::<Vec<_>>(), "last": last as u64 })
}

// ---------- ďalšie operácie so súbormi a projektmi ----------
fn update_projects(s: &Flux, f: impl FnOnce(&mut serde_json::Map<String, Value>)) {
    let mut set = s.settings.lock().unwrap();
    if let Some(o) = set.as_object_mut() {
        f(o);
    }
    settings::save(&set);
}

fn with_project(s: &Flux, dir: &str, f: impl FnOnce(&mut serde_json::Map<String, Value>)) {
    update_projects(s, |o| {
        if let Some(list) = o.get_mut("projects").and_then(|v| v.as_array_mut()) {
            if let Some(p) = list.iter_mut().find(|p| p.get("dir").and_then(|d| d.as_str()) == Some(dir)) {
                if let Some(po) = p.as_object_mut() {
                    f(po);
                }
            }
        }
    });
}

pub fn pin(s: &Flux, dir: &str, on: bool) -> Value {
    with_project(s, dir, |p| {
        p.insert("pinned".into(), json!(on));
    });
    json!(true)
}

pub fn hide(s: &Flux, dir: &str, on: bool) -> Value {
    with_project(s, dir, |p| {
        p.insert("hidden".into(), json!(on));
    });
    json!(true)
}

pub fn forget(s: &Flux, dir: &str) -> Value {
    update_projects(s, |o| {
        for key in ["recent", "projects"] {
            if let Some(list) = o.get_mut(key).and_then(|v| v.as_array_mut()) {
                list.retain(|x| x.as_str() != Some(dir) && x.get("dir").and_then(|d| d.as_str()) != Some(dir));
            }
        }
    });
    json!(true)
}

fn valid_name(n: &str) -> bool {
    !n.is_empty() && n.trim() == n && !n.chars().any(|c| "\\/:*?\"<>|".contains(c))
}

pub fn rename_project(s: &Flux, dir: &str, name: &str) -> Result<Value, String> {
    if !valid_name(name) {
        return Err("Invalid folder name.".into());
    }
    let target = Path::new(dir).parent().unwrap_or(Path::new("")).join(name);
    if target.exists() {
        return Err("A folder with this name already exists.".into());
    }
    std::fs::rename(dir, &target).map_err(|e| e.to_string())?;
    let t = target.to_string_lossy().to_string();
    update_projects(s, |o| {
        let swap = |v: &mut Value| {
            if v.as_str() == Some(dir) {
                *v = json!(t);
            }
        };
        if let Some(list) = o.get_mut("recent").and_then(|v| v.as_array_mut()) {
            list.iter_mut().for_each(swap);
        }
        if let Some(list) = o.get_mut("projects").and_then(|v| v.as_array_mut()) {
            for p in list.iter_mut() {
                if let Some(d) = p.get_mut("dir") {
                    swap(d);
                }
            }
        }
        if o.get("lastFolder").and_then(|v| v.as_str()) == Some(dir) {
            o.insert("lastFolder".into(), json!(t));
        }
    });
    let mut ws = s.workspace.lock().unwrap();
    if ws.as_deref() == Some(dir) {
        *ws = Some(t.clone());
    }
    Ok(json!(t))
}

pub fn default_root() -> PathBuf {
    dirs::document_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default()).join("Flux Projects")
}

pub fn create_project(name: &str, root: &str) -> Result<Value, String> {
    if !valid_name(name) {
        return Err("Invalid project name.".into());
    }
    let base = if root.is_empty() { default_root() } else { PathBuf::from(root) };
    let dir = base.join(name);
    if dir.exists() {
        return Err("This project already exists.".into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(json!(dir.to_string_lossy()))
}

// Súbor otvorený mimo projektu – pridá sa do nedávnych súborov.
pub fn allow_file(s: &Flux, p: &str) -> Value {
    let abs = std::fs::canonicalize(p).map(|x| x.to_string_lossy().trim_start_matches(r"\\?\").to_string()).unwrap_or_else(|_| p.to_string());
    let path = Path::new(&abs);
    if path.is_dir() {
        return json!({ "dir": abs });
    }
    if !path.is_file() {
        return Value::Null;
    }
    update_projects(s, |o| {
        let mut list: Vec<Value> = o.get("recentFiles").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        list.retain(|x| x.as_str().map(|v| v.to_lowercase()) != Some(abs.to_lowercase()));
        list.insert(0, json!(abs));
        list.truncate(12);
        o.insert("recentFiles".into(), Value::Array(list));
    });
    json!(abs)
}

pub fn recent_files(s: &Flux) -> Value {
    let set = s.settings.lock().unwrap();
    let list: Vec<Value> = set.get("recentFiles").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    Value::Array(list.into_iter().filter(|f| f.as_str().map(|p| Path::new(p).is_file()).unwrap_or(false)).collect())
}

pub fn forget_recent(s: &Flux, p: &str) -> Value {
    update_projects(s, |o| {
        if let Some(list) = o.get_mut("recentFiles").and_then(|v| v.as_array_mut()) {
            list.retain(|x| x.as_str().map(|v| v.to_lowercase()) != Some(p.to_lowercase()));
        }
    });
    json!(true)
}

pub fn read_image(file: &str) -> Result<Value, String> {
    use base64::Engine;
    let meta = std::fs::metadata(file).map_err(|e| e.to_string())?;
    if meta.len() > 25 * 1024 * 1024 {
        return Err("The image is too large.".into());
    }
    let ext = Path::new(file).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mime = match ext.as_str() {
        "svg" => "image/svg+xml".to_string(),
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "ico" => "image/x-icon".to_string(),
        e => format!("image/{e}"),
    };
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    Ok(json!({ "url": format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)), "size": meta.len() }))
}

pub async fn trash(target: &str) -> Result<Value, String> {
    let name = Path::new(target).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ok = rfd::AsyncMessageDialog::new()
        .set_title("Flux")
        .set_description(format!("Delete “{name}”?\n\nIt will be moved to the Recycle Bin, where you can restore it."))
        .set_buttons(rfd::MessageButtons::OkCancelCustom("Move to Recycle Bin".into(), "Cancel".into()))
        .show()
        .await;
    if !matches!(ok, rfd::MessageDialogResult::Custom(ref b) if b == "Move to Recycle Bin") && ok != rfd::MessageDialogResult::Ok {
        return Ok(json!(false));
    }
    trash::delete(target).map_err(|e| e.to_string())?;
    Ok(json!(true))
}

// Do Koša bez otázky (otázku ukáže okno aplikácie).
pub fn move_to_trash(target: &str) -> Result<(), String> {
    trash::delete(target).map_err(|e| e.to_string())
}

pub fn reveal(target: &str) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(format!("/select,{target}")).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").args(["-R", target]).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(Path::new(target).parent().unwrap_or(Path::new("/"))).spawn();
}
