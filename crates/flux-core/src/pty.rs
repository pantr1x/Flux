// Proces v pseudoterminále (ConPTY na Windows) – pre ▶ Run aj pre Terminál.
// Výstup ide do okna ako udalosť `data_ev`, koniec ako `exit_ev`; klávesy idú priamo programu.
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde_json::json;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::Emit;

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
    pub fn spawn(&self, emit: &Emit, cmd: &str, args: &[String], cwd: &str, env: &[(&str, &str)], data_ev: &'static str, exit_ev: &'static str) -> Result<u32, String> {
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
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let emit_r = emit.clone();
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
                        emit_r(data_ev, serde_json::Value::String(text));
                    }
                }
            }
            let _ = done_tx.send(());
        });
        // koniec programu sleduje samostatné vlákno: ConPTY (Windows) nedá čítačke EOF, kým je pseudokonzola otvorená,
        // takže po skončení procesu ju zavrieme sami (drop master) – inak by Flux ukazoval „Running“ navždy
        let emit = emit.clone();
        let live = self.live.clone();
        std::thread::spawn(move || {
            let code = loop {
                std::thread::sleep(Duration::from_millis(50));
                let mut g = live.lock().unwrap();
                match g.as_mut() {
                    Some(l) if l.id == id => {
                        if let Ok(Some(st)) = l.child.try_wait() {
                            break Some(st.exit_code() as i64);
                        }
                    }
                    // zastavené (Stop) alebo nahradené novým spustením
                    _ => break None,
                }
            };
            if let Some(code) = code {
                // zvyšok výstupu ešte dobehne; potom zavrieť pseudoterminál
                if done_rx.recv_timeout(Duration::from_millis(300)).is_err() {
                    let old = {
                        let mut g = live.lock().unwrap();
                        if g.as_ref().is_some_and(|l| l.id == id) { g.take() } else { None }
                    };
                    drop(old);
                    let _ = done_rx.recv_timeout(Duration::from_secs(1));
                } else {
                    let mut g = live.lock().unwrap();
                    if g.as_ref().is_some_and(|l| l.id == id) {
                        *g = None;
                    }
                }
                emit(exit_ev, json!({ "code": code, "ms": started.elapsed().as_millis() as u64 }));
            } else {
                let _ = done_rx.recv_timeout(Duration::from_secs(1));
                // medzitým beží nové spustenie – jeho koniec príde samostatne, toto by ho omylom „ukončilo“
                if live.lock().unwrap().is_some() {
                    return;
                }
                emit(exit_ev, json!({ "code": -1, "ms": started.elapsed().as_millis() as u64 }));
            }
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn pty_exit_after_output() {
        let got: Arc<Mutex<Vec<(String, serde_json::Value)>>> = Arc::default();
        let g = got.clone();
        let emit: Emit = Arc::new(move |ch: &str, v: serde_json::Value| g.lock().unwrap().push((ch.to_string(), v)));
        let pty = Pty::default();
        pty.spawn(&emit, "sh", &["-c".into(), "echo hi; exit 3".into()], "", &[], "d", "x").unwrap();
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(3) && !got.lock().unwrap().iter().any(|(c, _)| c == "x") {
            std::thread::sleep(Duration::from_millis(20));
        }
        let ev = got.lock().unwrap().clone();
        let last = ev.last().expect("events");
        assert_eq!(last.0, "x", "{ev:?}");
        assert_eq!(last.1["code"], 3);
        assert!(ev.iter().any(|(c, v)| c == "d" && v.as_str().unwrap_or("").contains("hi")), "{ev:?}");
        assert!(!pty.running());
    }
}
