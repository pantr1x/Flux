// ▶ Run: ktorý program spustí súbor (ako src/main/runner.js → commandFor) a spustenie v pseudoterminále.
use crate::pty::Pty;
use serde_json::{json, Value};
use std::path::Path;
use crate::Emit;

pub struct Cmd {
    pub cmd: String,
    pub args: Vec<String>,
}

fn c(cmd: &str, args: &[&str]) -> Option<Cmd> {
    Some(Cmd { cmd: cmd.into(), args: args.iter().map(|s| s.to_string()).collect() })
}

fn git_bash() -> Option<String> {
    ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .chain(std::env::var("LOCALAPPDATA").ok().map(|l| format!("{l}\\Programs")))
        .map(|r| format!("{r}\\Git\\bin\\bash.exe"))
        .find(|p| Path::new(p).exists())
}

// C/C++/Rust: najprv preložiť, potom spustiť (ako Code Runner vo VS Code).
fn compile_and_run(src: &str, compiler: &str) -> Option<Cmd> {
    let stem = Path::new(src).file_stem()?.to_string_lossy().to_string();
    if cfg!(windows) {
        let out = format!("{stem}.exe");
        let q = |s: &str| format!("'{}'", s.replace('\'', "''"));
        c("powershell.exe", &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &format!("& {compiler} {} -o {}; if ($?) {{ & {} }}", q(src), q(&out), q(&format!(".\\{out}")))])
    } else {
        c("bash", &["-c", &format!("{compiler} \"$1\" -o \"$2\" && \"./$2\""), "flux", src, &stem])
    }
}

pub fn command_for(file: &str, python: &str, lang: &str) -> Option<Cmd> {
    let p = Path::new(file);
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let base = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if ext.is_empty() && lang == "python" {
        return (!python.is_empty()).then(|| Cmd { cmd: python.into(), args: vec!["-u".into(), file.into()] });
    }
    match ext.as_str() {
        "py" | "pyw" => (!python.is_empty()).then(|| Cmd { cmd: python.into(), args: vec!["-u".into(), file.into()] }),
        "js" | "mjs" | "cjs" => c("node", &[file]),
        "ts" | "mts" | "cts" => c("node", &["--experimental-strip-types", "--no-warnings", file]),
        "pl" => c("perl", &[file]),
        "java" => c("java", &[&base]),
        "go" => c("go", &["run", &base]),
        "cs" => c("dotnet", &["run", &base]),
        "rs" => compile_and_run(&base, "rustc"),
        "rb" => c("ruby", &[&base]),
        "php" => c("php", &[&base]),
        "lua" => c("lua", &[&base]),
        "zig" => c("zig", &["run", &base]),
        "r" => c("Rscript", &[&base]),
        "jl" => c("julia", &[&base]),
        "c" => compile_and_run(&base, "gcc"),
        "cpp" | "cc" | "cxx" => compile_and_run(&base, "g++"),
        "bat" | "cmd" if cfg!(windows) => c("cmd.exe", &["/d", "/c", file]),
        "ps1" => c(if cfg!(windows) { "powershell.exe" } else { "pwsh" }, &["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", file]),
        "sh" if !cfg!(windows) => c("bash", &[file]),
        "sh" => git_bash().and_then(|b| c(&b, &[file])),
        _ => None,
    }
}

// Spustí príkaz a pošle run:start (rozhranie potom čaká na run:data / run:exit).
pub fn start(pty: &Pty, emit: &Emit, cmd: &Cmd, cwd: &str, label: &str) -> Value {
    match pty.spawn(emit, &cmd.cmd, &cmd.args, cwd, &[("PYTHONIOENCODING", "utf-8"), ("PYTHONUTF8", "1")], "run:data", "run:exit") {
        Ok(pid) => {
            emit("run:start", json!({ "label": label, "cwd": cwd, "pid": pid, "pty": true }));
            json!({ "ok": true })
        }
        Err(e) => {
            emit("run:start", json!({ "label": label, "cwd": cwd, "pid": 0 }));
            emit("run:exit", json!({ "code": -1, "error": e, "ms": 0 }));
            json!({ "ok": true })
        }
    }
}

pub fn run_file(pty: &Pty, emit: &Emit, file: &str, python: &str, lang: &str) -> Value {
    let cwd = Path::new(file).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let label = Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    match command_for(file, python, lang) {
        Some(cmd) => start(pty, emit, &cmd, &cwd, &label),
        None => json!({ "ok": false, "error": "Can't run this type of file yet." }),
    }
}

// Terminál: PowerShell na Windows, inak shell používateľa.
pub fn shell_command() -> Cmd {
    if cfg!(windows) {
        return Cmd { cmd: "powershell.exe".into(), args: vec!["-NoLogo".into()] };
    }
    let sh = std::env::var("SHELL").ok().filter(|s| Path::new(s).exists()).unwrap_or_else(|| "/bin/bash".into());
    Cmd { cmd: sh, args: vec!["-i".into()] }
}
