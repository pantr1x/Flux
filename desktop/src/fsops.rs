// Súbory a projekty – rovnaké odpovede ako Electron verzia (src/main/main.js), aby rozhranie fungovalo bez zmien.
use crate::{settings, Flux};
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
