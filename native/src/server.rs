// Live Server pre Flux Native: malý HTTP server nad priečinkom projektu (127.0.0.1:5500+).
// Do každej HTML stránky vloží skript, ktorý sa pýta na /__flux/ver a pri zmene (uloženie súboru)
// stránku znova načíta – ako Live Server v Electron Fluxe, bez závislostí.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Server {
    pub port: u16,
    pub root: PathBuf,
    stop: Arc<AtomicBool>,
    ver: Arc<AtomicU64>,
    css: Arc<AtomicBool>, // posledná zmena bola len CSS → stránka vymení štýly bez načítania
    live: Live,
    hover: Hover,
}

// neuložený text otvorených súborov – stránka sa mení už počas písania
type Live = Arc<Mutex<HashMap<String, String>>>;
// nad čím je myš v stránke: (súbor, riadok v zdroji)
type Hover = Arc<Mutex<Option<(String, usize)>>>;

pub fn key(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

// stránka drží otvorené „dlhé pýtanie“ /__flux/ver?v=N – server odpovie hneď, ako sa číslo zmení (bez 200 ms intervalu);
// odpoveď = „číslo:css“ alebo „číslo:page“; pri CSS sa len vymenia štýly (stránka nebliká).
// Druhý skript posiela, nad ktorým prvkom (data-flux-l = riadok v zdroji) je myš – Flux ten riadok zvýrazní.
const RELOAD: &str = "<script>(async()=>{let v='';for(;;){try{const t=await (await fetch('/__flux/ver?v='+v,{cache:'no-store'})).text();const[n,k]=t.split(':');if(v!==''&&n!==v){if(k==='css'){document.querySelectorAll('link[rel=stylesheet]').forEach(l=>{const u=new URL(l.href);u.searchParams.set('v',n);l.href=u.href})}else{location.reload();return}}v=n}catch(e){await new Promise(r=>setTimeout(r,400))}}})();(()=>{let last=-1;const send=l=>{if(l===last)return;last=l;fetch('/__flux/hover?p='+encodeURIComponent(location.pathname)+'&l='+l,{cache:'no-store'}).catch(()=>{})};document.addEventListener('mouseover',e=>{const x=e.target.closest&&e.target.closest('[data-flux-l]');send(x?+x.dataset.fluxL:0)},true);document.documentElement.addEventListener('mouseleave',()=>send(0))})()</script>";

// skript sa vloží na začiatok dokumentu (za <head>/<html>/doctype), nie na koniec: pri písaní býva značka rozpísaná
// (`<ol` bez `>`) a skript vložený za ňou by sa zmenil na jej atribúty a ukázal sa ako text
pub fn inject(html: &str) -> String {
    let low = html.to_ascii_lowercase();
    let after = |tag: &str| -> Option<usize> {
        let i = low.find(tag)?;
        let end = low[i..].find('>')?;
        // značka musí byť celá: v jej vnútri nesmie byť ďalšie „<“
        (!low[i + 1..i + end].contains('<')).then_some(i + end + 1)
    };
    let at = after("<head").or_else(|| after("<html")).or_else(|| low.starts_with("<!doctype").then(|| low.find('>').map(|e| e + 1)).flatten()).unwrap_or(0);
    format!("{}{RELOAD}{}", &html[..at], &html[at..])
}

// do každej otváracej značky pridá data-flux-l="riadok" (číslo riadku v zdroji); komentáre a obsah script/style preskočí
pub fn annotate(html: &str) -> String {
    let b = html.as_bytes();
    let mut out = String::with_capacity(html.len() + html.len() / 8);
    let (mut i, mut line, mut last) = (0usize, 1usize, 0usize);
    while i < b.len() {
        if b[i] == b'\n' {
            line += 1;
        }
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &html[i..];
        if rest.starts_with("<!--") {
            let end = rest.find("-->").map(|e| i + e + 3).unwrap_or(b.len());
            line += html[i..end].matches('\n').count();
            i = end;
            continue;
        }
        if b.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic()) {
            let name_end = rest[1..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).map(|e| i + 1 + e).unwrap_or(b.len());
            let name = html[i + 1..name_end].to_ascii_lowercase();
            if !matches!(name.as_str(), "html" | "head" | "script" | "style" | "meta" | "link" | "title" | "base") {
                out.push_str(&html[last..name_end]);
                out.push_str(&format!(" data-flux-l=\"{line}\""));
                last = name_end;
            }
            if name == "script" || name == "style" {
                let close = format!("</{name}");
                let from = name_end;
                let end = html[from..].to_ascii_lowercase().find(&close).map(|e| from + e).unwrap_or(b.len());
                line += html[i..end].matches('\n').count();
                i = end.max(i + 1);
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&html[last..]);
    out
}

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

fn handle(s: TcpStream, root: &Path, ver: &AtomicU64, css: &AtomicBool, live: &Live, hover: &Hover) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    if BufReader::new(&s).read_line(&mut line).is_err() {
        return;
    }
    let target = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let query = target.split_once('?').map(|x| x.1.split('#').next().unwrap_or("").to_string()).unwrap_or_default();
    let arg = |k: &str| query.split('&').find_map(|kv| kv.strip_prefix(&format!("{k}="))).map(decode).unwrap_or_default();
    let path = decode(target.split(['?', '#']).next().unwrap_or("/"));
    if path == "/__flux/ver" {
        // čaká (najviac 20 s), kým sa číslo zmení oproti tomu, ktoré stránka už má
        let have = arg("v");
        let t0 = std::time::Instant::now();
        while !have.is_empty() && have == ver.load(Ordering::Relaxed).to_string() && t0.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(4));
        }
        let kind = if css.load(Ordering::Relaxed) { "css" } else { "page" };
        return respond(s, "200 OK", "text/plain", format!("{}:{kind}", ver.load(Ordering::Relaxed)).as_bytes());
    }
    // len súbory v projekte (žiadne „..“)
    let resolve = |p: &str| {
        let rel: PathBuf = p.split('/').filter(|c| !c.is_empty() && *c != "." && *c != "..").collect();
        let f = root.join(rel);
        if f.is_dir() {
            f.join("index.html")
        } else {
            f
        }
    };
    if path == "/__flux/hover" {
        let l: usize = arg("l").parse().unwrap_or(0);
        *hover.lock().unwrap() = if l > 0 { Some((resolve(&arg("p")).to_string_lossy().to_string(), l)) } else { None };
        return respond(s, "200 OK", "text/plain", b"");
    }
    let file = resolve(&path);
    let unsaved = live.lock().unwrap().get(&key(&file)).cloned();
    match unsaved.map(|t| Ok(t.into_bytes())).unwrap_or_else(|| std::fs::read(&file)) {
        Ok(mut body) => {
            let ct = mime(&file);
            if ct.starts_with("text/html") {
                let html = annotate(&String::from_utf8_lossy(&body));
                let with = inject(&html);
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
        let css = Arc::new(AtomicBool::new(false));
        let live: Live = Default::default();
        let hover: Hover = Default::default();
        let (st, v, r, c, l, h0) = (stop.clone(), ver.clone(), root.to_path_buf(), css.clone(), live.clone(), hover.clone());
        std::thread::spawn(move || {
            while !st.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((s, _)) => {
                        let _ = s.set_nonblocking(false);
                        let (v, r, c, l, h) = (v.clone(), r.clone(), c.clone(), l.clone(), h0.clone());
                        std::thread::spawn(move || handle(s, &r, &v, &c, &l, &h));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(40)),
                }
            }
        });
        Ok(Server { port, root: root.to_path_buf(), stop, ver, css, live, hover })
    }

    // adresa stránky (súbor relatívne ku koreňu)
    pub fn url(&self, file: &str) -> String {
        let rel = Path::new(file).strip_prefix(&self.root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        format!("http://127.0.0.1:{}/{}", self.port, rel)
    }

    // súbor sa zmenil → otvorené stránky sa znova načítajú
    pub fn bump(&self) {
        self.css.store(false, Ordering::Relaxed);
        self.ver.fetch_add(1, Ordering::Relaxed);
    }

    // neuložený text súboru → stránka ho ukáže hneď (CSS bez načítania stránky)
    pub fn set_live(&self, file: &str, text: &str) {
        let p = Path::new(file);
        if !p.starts_with(&self.root) {
            return;
        }
        self.live.lock().unwrap().insert(key(p), text.to_string());
        let is_css = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("css"));
        self.css.store(is_css, Ordering::Relaxed);
        self.ver.fetch_add(1, Ordering::Relaxed);
    }

    // nad ktorým riadkom zdroja je myš v stránke
    pub fn hover(&self) -> Option<(String, usize)> {
        self.hover.lock().unwrap().clone()
    }

    // uložené – znova z disku
    pub fn clear_live(&self, file: &str) {
        self.live.lock().unwrap().remove(&key(Path::new(file)));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::{annotate, inject};

    #[test]
    fn server_inject_survives_broken_markup() {
        let h = "<!doctype html>\n<html>\n<head><title>x</title></head>\n<body>\n<ol\n</body></html>";
        let o = inject(h);
        assert!(o.find("<script>").unwrap() < o.find("<body>").unwrap());
        let h2 = "<p>bez hlavicky</p>";
        assert!(inject(h2).starts_with("<script>"));
        assert!(inject("<!doctype html><p>a</p>").starts_with("<!doctype html><script>"));
    }

    #[test]
    fn server_annotate_lines() {
        let h = "<!doctype html>\n<html>\n<body>\n<!-- <p> -->\n<h1 class=\"a\">Hi</h1>\n<script>if (a<b) {}</script>\n<p>x</p>\n</body></html>";
        let o = annotate(h);
        assert!(o.contains("<h1 data-flux-l=\"5\" class"));
        assert!(o.contains("<p data-flux-l=\"7\">"));
        assert!(o.contains("<body data-flux-l=\"3\">"));
        assert!(!o.contains("<html data-flux"));
        assert!(!o.contains("<script data-flux"));
        assert!(!o.contains("<p> -->\n<h1 data-flux-l=\"5\" class=\"a\">Hi</h1>\n<script data"));
    }
}
