// Linux: náhľad Live Servera = pomocný program flux-preview (WebKitGTK) ako podokno nášho X11 okna.
// Flux sám knižnicu webkit2gtk nepotrebuje – bez nej (alebo bez pomocníka) ostane karta „Otvoriť v prehliadači“.
// Pomocník sa stiahne pri prvom náhľade z tej istej vetvy ako aktualizácie (native.json → preview.sha256).
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum Get {
    Busy,
    Ready(PathBuf),
    Failed(String),
}

fn here() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|d| d.join("flux-preview"))
}

fn sha_of(p: &std::path::Path) -> Option<String> {
    std::fs::read(p).ok().map(|b| format!("{:x}", Sha256::digest(&b)))
}

// nájde alebo stiahne pomocníka (blokuje – volať vo vlákne)
fn ensure() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("FLUX_PREVIEW_HELPER") {
        return Ok(p.into());
    }
    let path = here().ok_or("no exe dir")?;
    // vlastná zostava: pomocník zostavený vedľa (target/…/flux-preview)
    if crate::update::SHA == "dev" && path.exists() {
        return Ok(path);
    }
    let info = std::process::Command::new("curl").args(["-fsSL", "--max-time", "20"]).arg(format!("{}native.json", crate::update::base())).output().ok().filter(|o| o.status.success()).and_then(|o| serde_json::from_slice::<Value>(&o.stdout).ok());
    let want = info.as_ref().and_then(|j| j["preview"]["sha256"].as_str()).map(|s| s.to_lowercase());
    let have = sha_of(&path);
    match (want, have) {
        // vlastná zostava (dev) alebo bez spojenia: použiť, čo leží vedľa
        (None, Some(_)) => Ok(path),
        (None, None) => Err("not available".into()),
        (Some(w), Some(h)) if w == h => Ok(path),
        (Some(w), _) => {
            let part = path.with_extension("part");
            let ok = std::process::Command::new("curl").args(["-fsSL", "--max-time", "300", "-o"]).arg(&part).arg(format!("{}Flux-Preview", crate::update::base())).status().map(|s| s.success()).unwrap_or(false);
            if !ok || sha_of(&part).as_deref() != Some(w.as_str()) {
                let _ = std::fs::remove_file(&part);
                return if path.exists() { Ok(path) } else { Err("download failed".into()) };
            }
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755));
            std::fs::rename(&part, &path).map_err(|e| e.to_string())?;
            Ok(path)
        }
    }
}

// raz za beh Fluxu; výsledok zdieľaný
pub fn get() -> Get {
    static STATE: Mutex<Option<Get>> = Mutex::new(None);
    let mut s = STATE.lock().unwrap();
    match &*s {
        Some(g) => g.clone(),
        None => {
            *s = Some(Get::Busy);
            std::thread::spawn(|| {
                let r = match ensure() {
                    Ok(p) => Get::Ready(p),
                    Err(e) => Get::Failed(e),
                };
                *STATE.lock().unwrap() = Some(r);
            });
            Get::Busy
        }
    }
}

pub enum Ev {
    Nav(String),
    Tab(String),
}

pub struct Helper {
    child: Child,
    stdin: ChildStdin,
    pub rx: mpsc::Receiver<Ev>,
    pub err: Arc<Mutex<String>>,
    last: String, // naposledy poslané (bez opakovania toho istého každý snímok)
}

impl Helper {
    pub fn spawn(path: &std::path::Path, xid: u64, url: &str, bounds: [f64; 4]) -> std::io::Result<Helper> {
        let mut child = Command::new(path)
            .args(["--parent", &xid.to_string(), "--url", url, "--bounds", &json!(bounds).to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| std::io::Error::other("stdin"))?;
        let (tx, rx) = mpsc::channel();
        if let Some(out) = child.stdout.take() {
            std::thread::spawn(move || {
                for l in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                    let Ok(v) = serde_json::from_str::<Value>(&l) else { continue };
                    let ev = if let Some(u) = v["nav"].as_str() {
                        Ev::Nav(u.into())
                    } else if let Some(u) = v["tab"].as_str() {
                        Ev::Tab(u.into())
                    } else {
                        continue;
                    };
                    if tx.send(ev).is_err() {
                        break;
                    }
                }
            });
        }
        let err = Arc::new(Mutex::new(String::new()));
        if let Some(e) = child.stderr.take() {
            let err2 = err.clone();
            std::thread::spawn(move || {
                for l in std::io::BufReader::new(e).lines().map_while(Result::ok) {
                    if let Ok(mut s) = err2.lock() {
                        if s.len() < 4000 {
                            s.push_str(&l);
                            s.push('\n');
                        }
                    }
                }
            });
        }
        Ok(Helper { child, stdin, rx, err, last: String::new() })
    }

    pub fn send(&mut self, v: Value) {
        let _ = writeln!(self.stdin, "{v}");
        let _ = self.stdin.flush();
    }

    // stav (poloha, viditeľnosť, zväčšenie) – pošle len pri zmene
    pub fn state(&mut self, bounds: [f64; 4], visible: bool, zoom: f64) {
        let v = json!({ "bounds": bounds, "visible": visible, "zoom": zoom });
        let s = v.to_string();
        if s != self.last {
            self.last = s;
            self.send(v);
        }
    }

    // skončil (chýba knižnica, nejde X11…)
    pub fn exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
