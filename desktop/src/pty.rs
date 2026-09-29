// Proces v pseudoterminále (ConPTY na Windows) – pre ▶ Run aj pre Terminál.
// Výstup ide do okna ako udalosť `data_ev`, koniec ako `exit_ev`; klávesy idú priamo programu.
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde_json::json;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

struct Live {
    child: Box<dyn Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    id: u64,
}

#[derive(Default)]
pub struct Pty {
    live: Arc<Mutex<Option<Live>>>,
    size: Mutex<(u16, u16)>,
    seq: Mutex<u64>,
}

impl Pty {
    pub fn running(&self) -> bool {
        self.live.lock().unwrap().is_some()
    }

    // Spustí program; vráti pid alebo chybu.
    pub fn spawn(&self, app: &AppHandle, cmd: &str, args: &[String], cwd: &str, env: &[(&str, &str)], data_ev: &'static str, exit_ev: &'static str) -> Result<u32, String> {
        self.kill();
        let (cols, rows) = {
            let s = *self.size.lock().unwrap();
            if s.0 == 0 { (100, 24) } else { s }
        };
        let pair = native_pty_system().openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }).map_err(|e| e.to_string())?;
        let mut c = CommandBuilder::new(cmd);
        c.args(args);
        if !cwd.is_empty() {
            c.cwd(cwd);
        }
        c.env("TERM", "xterm-256color");
        c.env("TERM_PROGRAM", "Flux");
        for (k, v) in env {
            c.env(k, v);
        }
        let child = pair.slave.spawn_command(c).map_err(|e| format!("{cmd}: {e}"))?;
        drop(pair.slave);
        let pid = child.process_id().unwrap_or(0);
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let id = {
            let mut s = self.seq.lock().unwrap();
            *s += 1;
            *s
        };
        *self.live.lock().unwrap() = Some(Live { child, master: pair.master, writer, id });
        let started = Instant::now();
        let app = app.clone();
        let live = self.live.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 16384];
            let mut pending: Vec<u8> = vec![];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        // UTF-8 znak rozdelený medzi dve čítania – dokončiť ho v ďalšom kole
                        pending.extend_from_slice(&buf[..n]);
                        let cut = match std::str::from_utf8(&pending) {
                            Ok(_) => pending.len(),
                            Err(e) if e.error_len().is_none() => e.valid_up_to(),
                            Err(_) => pending.len(),
                        };
                        let text = String::from_utf8_lossy(&pending[..cut]).to_string();
                        pending.drain(..cut);
                        let _ = app.emit(data_ev, text);
                    }
                }
            }
            // koniec: kód programu (ak ho medzitým niekto nezastavil)
            let code = {
                let mut g = live.lock().unwrap();
                match g.as_mut() {
                    Some(l) if l.id == id => {
                        let code = l.child.wait().ok().map(|s| s.exit_code() as i64).unwrap_or(-1);
                        *g = None;
                        code
                    }
                    _ => -1,
                }
            };
            let _ = app.emit(exit_ev, json!({ "code": code, "ms": started.elapsed().as_millis() as u64 }));
        });
        Ok(pid)
    }

    pub fn write(&self, text: &str) {
        if let Some(l) = self.live.lock().unwrap().as_mut() {
            let _ = l.writer.write_all(text.as_bytes());
            let _ = l.writer.flush();
        }
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let cols = cols.max(20);
        let rows = rows.max(4);
        *self.size.lock().unwrap() = (cols, rows);
        if let Some(l) = self.live.lock().unwrap().as_ref() {
            let _ = l.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        }
    }

    pub fn kill(&self) {
        if let Some(mut l) = self.live.lock().unwrap().take() {
            let _ = l.child.kill();
        }
    }
}
