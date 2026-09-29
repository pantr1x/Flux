// Live Server pre Flux Native: malý HTTP server nad priečinkom projektu (127.0.0.1:5500+).
// Do každej HTML stránky vloží skript, ktorý sa pýta na /__flux/ver a pri zmene (uloženie súboru)
// stránku znova načíta – ako Live Server v Electron Fluxe, bez závislostí.
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct Server {
    pub port: u16,
    pub root: PathBuf,
    stop: Arc<AtomicBool>,
    ver: Arc<AtomicU64>,
}

const RELOAD: &str =
    "<script>(()=>{let v=null;setInterval(async()=>{try{const t=await (await fetch('/__flux/ver',{cache:'no-store'})).text();if(v!==null&&t!==v)location.reload();v=t}catch(e){}},500)})()</script>";

fn mime(p: &Path) -> &'static str {
    match p.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

// %20 → medzera atď.
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn respond(mut s: TcpStream, code: &str, ctype: &str, body: &[u8]) {
    let head = format!("HTTP/1.1 {code}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", body.len());
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body);
}

fn handle(s: TcpStream, root: &Path, ver: &AtomicU64) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    if BufReader::new(&s).read_line(&mut line).is_err() {
        return;
    }
    let path = line.split_whitespace().nth(1).unwrap_or("/");
    let path = decode(path.split(['?', '#']).next().unwrap_or("/"));
    if path == "/__flux/ver" {
        return respond(s, "200 OK", "text/plain", ver.load(Ordering::Relaxed).to_string().as_bytes());
    }
    // len súbory v projekte (žiadne „..“)
    let rel: PathBuf = path.split('/').filter(|c| !c.is_empty() && *c != "." && *c != "..").collect();
    let mut file = root.join(rel);
    if file.is_dir() {
        file = file.join("index.html");
    }
    match std::fs::read(&file) {
        Ok(mut body) => {
            let ct = mime(&file);
            if ct.starts_with("text/html") {
                let html = String::from_utf8_lossy(&body).into_owned();
                let low = html.to_lowercase();
                let with = match low.rfind("</body>") {
                    Some(i) => format!("{}{RELOAD}{}", &html[..i], &html[i..]),
                    None => format!("{html}{RELOAD}"),
                };
                body = with.into_bytes();
            }
            respond(s, "200 OK", ct, &body)
        }
        Err(_) => respond(s, "404 Not Found", "text/html; charset=utf-8", format!("<h1>404</h1><p>{}</p>{RELOAD}", path.replace('<', "&lt;")).as_bytes()),
    }
}

impl Server {
    // prvý voľný port od 5500
    pub fn start(root: &Path) -> std::io::Result<Server> {
        let listener = (5500..5600).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).ok_or_else(|| std::io::Error::other("no free port"))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let ver = Arc::new(AtomicU64::new(1));
        let (st, v, r) = (stop.clone(), ver.clone(), root.to_path_buf());
        std::thread::spawn(move || {
            while !st.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((s, _)) => {
                        let _ = s.set_nonblocking(false);
                        let (v, r) = (v.clone(), r.clone());
                        std::thread::spawn(move || handle(s, &r, &v));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(40)),
                }
            }
        });
        Ok(Server { port, root: root.to_path_buf(), stop, ver })
    }

    // adresa stránky (súbor relatívne ku koreňu)
    pub fn url(&self, file: &str) -> String {
        let rel = Path::new(file).strip_prefix(&self.root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        format!("http://127.0.0.1:{}/{}", self.port, rel)
    }

    // súbor sa zmenil → otvorené stránky sa znova načítajú
    pub fn bump(&self) {
        self.ver.fetch_add(1, Ordering::Relaxed);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
