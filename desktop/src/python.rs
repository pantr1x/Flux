// Hľadanie Pythonu – ako src/main/python.js: ručne vybraný → .venv/venv v projekte → $VIRTUAL_ENV → py launcher → PATH.
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

pub fn probe(cmd: &str, pre: &[&str]) -> Option<Value> {
    let mut c = Command::new(cmd);
    c.args(pre).args(["-c", "import sys; print(sys.executable); print(sys.version.split()[0])"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let out = c.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut lines = text.lines();
    let exe = lines.next()?.trim().to_string();
    let ver = lines.next()?.trim().to_string();
    // Windows „Store“ alias nič nespustí
    if exe.is_empty() || ver.is_empty() || exe.to_lowercase().contains("windowsapps") {
        return None;
    }
    Some(json!({ "path": exe, "version": ver }))
}

fn venv_python(dir: &Path) -> std::path::PathBuf {
    if cfg!(windows) { dir.join("Scripts").join("python.exe") } else { dir.join("bin").join("python") }
}

fn windows_installs() -> Vec<String> {
    let mut found = vec![];
    let roots = [std::env::var("LOCALAPPDATA").ok().map(|l| format!("{l}\\Programs\\Python")), std::env::var("ProgramFiles").ok()];
    for r in roots.into_iter().flatten() {
        if let Ok(rd) = std::fs::read_dir(&r) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.to_lowercase().starts_with("python3") {
                    found.push(e.path().join("python.exe").to_string_lossy().to_string());
                }
            }
        }
    }
    found.sort();
    found.reverse();
    found
}

pub fn find(workspace: Option<&str>, chosen: Option<&str>) -> Value {
    let mut cands: Vec<(String, Vec<&str>, String)> = vec![];
    if let Some(o) = chosen {
        cands.push((o.into(), vec![], "chosen manually".into()));
    }
    if let Some(w) = workspace {
        for n in [".venv", "venv", "env"] {
            let exe = venv_python(&Path::new(w).join(n));
            if exe.exists() {
                cands.push((exe.to_string_lossy().to_string(), vec![], n.into()));
            }
        }
    }
    if let Ok(v) = std::env::var("VIRTUAL_ENV") {
        cands.push((venv_python(Path::new(&v)).to_string_lossy().to_string(), vec![], "VIRTUAL_ENV".into()));
    }
    if cfg!(windows) {
        cands.push(("py".into(), vec!["-3"], "py launcher".into()));
        cands.push(("python".into(), vec![], "PATH".into()));
        for exe in windows_installs() {
            cands.push((exe, vec![], "installed".into()));
        }
    } else {
        cands.push(("python3".into(), vec![], "PATH".into()));
        cands.push(("python".into(), vec![], "PATH".into()));
    }
    for (cmd, pre, source) in cands {
        if let Some(mut info) = probe(&cmd, &pre) {
            info["source"] = json!(source);
            return info;
        }
    }
    Value::Null
}
