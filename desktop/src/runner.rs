// Spúšťanie súboru (▶ Run): výstup ide do okna ako udalosti run:start / run:data / run:exit,
// rovnako ako v Electron verzii (src/main/runner.js). Zatiaľ bez pseudoterminálu – vstup sa posiela po riadkoch.
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

#[derive(Default)]
pub struct Runner {
    child: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
}

fn command_for(file: &str, python: &str, lang: &str) -> Option<(String, Vec<String>)> {
    let ext = std::path::Path::new(file).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let py = if python.is_empty() { if cfg!(windows) { "python".to_string() } else { "python3".to_string() } } else { python.to_string() };
    match (lang, ext.as_str()) {
        (_, "py" | "pyw") | ("python", _) => Some((py, vec!["-u".into(), file.into()])),
        (_, "js" | "mjs" | "cjs") => Some(("node".into(), vec![file.into()])),
        _ => None,
    }
}

impl Runner {
    pub fn run(&self, app: &AppHandle, file: &str, python: &str, lang: &str) -> Result<Value, String> {
        self.stop();
        let cwd = std::path::Path::new(file).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        let label = std::path::Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let Some((cmd, args)) = command_for(file, python, lang) else {
            let _ = app.emit("run:start", json!({ "label": label, "cwd": cwd, "pid": 0 }));
            let _ = app.emit("run:exit", json!({ "code": -1, "error": "This file type cannot be run yet in this build.", "ms": 0 }));
            return Ok(json!(false));
        };
        let mut c = Command::new(&cmd);
        c.args(&args).current_dir(&cwd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).env("PYTHONIOENCODING", "utf-8");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let started = Instant::now();
        let mut child = match c.spawn() {
            Ok(ch) => ch,
            Err(e) => {
                let _ = app.emit("run:start", json!({ "label": label, "cwd": cwd, "pid": 0 }));
                let _ = app.emit("run:exit", json!({ "code": -1, "error": format!("{cmd}: {e}"), "ms": 0 }));
                return Ok(json!(false));
            }
        };
        let _ = app.emit("run:start", json!({ "label": label, "cwd": cwd, "pid": child.id(), "pty": false }));
        let mut readers = vec![];
        for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>)].into_iter().flatten() {
            let app = app.clone();
            readers.push(std::thread::spawn(move || {
                let mut stream = stream;
                let mut buf = [0u8; 8192];
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let _ = app.emit("run:data", String::from_utf8_lossy(&buf[..n]).to_string());
                }
            }));
        }
        *self.stdin.lock().unwrap() = child.stdin.take();
        *self.child.lock().unwrap() = Some(child);
        // čakanie na koniec programu vo vlastnom vlákne
        let slot = self.child.clone();
        let app = app.clone();
        std::thread::spawn(move || {
            for r in readers {
                let _ = r.join();
            }
            let code = loop {
                let mut g = slot.lock().unwrap();
                match g.as_mut().map(|c| c.try_wait()) {
                    Some(Ok(Some(st))) => {
                        *g = None;
                        break st.code().unwrap_or(-1);
                    }
                    Some(Ok(None)) => {}
                    _ => break -1, // zastavené cez stop()
                }
                drop(g);
                std::thread::sleep(std::time::Duration::from_millis(20));
            };
            let _ = app.emit("run:exit", json!({ "code": code, "ms": started.elapsed().as_millis() as u64 }));
        });
        Ok(json!(true))
    }

    pub fn input(&self, text: &str) {
        if let Some(s) = self.stdin.lock().unwrap().as_mut() {
            let _ = s.write_all(text.as_bytes());
            let _ = s.flush();
        }
    }

    pub fn stop(&self) {
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let _ = c.kill();
        }
        *self.stdin.lock().unwrap() = None;
    }
}
