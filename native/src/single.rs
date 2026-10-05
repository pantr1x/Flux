// Flux beží len raz: druhé spustenie (napr. dvojklik na súbor v Prieskumníkovi) pošle cestu bežiacemu Fluxu
// cez 127.0.0.1 a skončí. Port je v userData/flux-native-ipc.json.
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn port_file() -> std::path::PathBuf {
    flux_core::settings::user_data().join("flux-native-ipc.json")
}

// súbor/priečinok z príkazového riadka (prvý argument, ktorý nie je prepínač)
pub fn path_arg(args: &[String]) -> Option<String> {
    // pri prechode zo starého Fluxu je za --migrate cesta k jeho Flux.exe – to nie je súbor na otvorenie
    if args.iter().any(|a| a == "--migrate" || a == "--apply-update") {
        return None;
    }
    args.iter().skip(1).find(|a| !a.starts_with("--") && std::path::Path::new(a.as_str()).exists()).cloned()
}

// true = bežiaci Flux to prevzal (tento proces má skončiť)
pub fn forward(path: Option<&str>) -> bool {
    let Ok(text) = std::fs::read_to_string(port_file()) else { return false };
    let Some(port) = serde_json::from_str::<serde_json::Value>(&text).ok().and_then(|v| v["port"].as_u64()) else { return false };
    // bežiaci Flux bez viditeľného okna (napr. zaseknutý) by spustenie prevzal a nič by sa neotvorilo (0.9.18) –
    // vtedy sa spustí nový Flux; starý proces sa neukončuje (Windows Defender to považuje za podozrivé)
    #[cfg(windows)]
    if let Some(pid) = serde_json::from_str::<serde_json::Value>(&text).ok().and_then(|v| v["pid"].as_u64()) {
        if !crate::boot::has_visible_window(pid as u32) && !crate::mcp::bridge_mark(pid as u32).exists() {
            return false;
        }
    }
    let Ok(mut s) = TcpStream::connect_timeout(&([127, 0, 0, 1], port as u16).into(), Duration::from_millis(400)) else { return false };
    let _ = s.set_read_timeout(Some(Duration::from_millis(1500)));
    let msg = serde_json::json!({ "flux": 1, "open": path }).to_string();
    if s.write_all(format!("{msg}\n").as_bytes()).is_err() {
        return false;
    }
    let mut line = String::new();
    if !(BufReader::new(s).read_line(&mut line).is_ok() && line.trim() == "flux-ok") {
        return false;
    }
    if let Some(pid) = serde_json::from_str::<serde_json::Value>(&text).ok().and_then(|v| v["pid"].as_u64()) {
        crate::boot::bring_to_front(pid as u32);
    }
    true
}

// počúva a každú požiadavku pošle do aplikácie ako udalosť „ipc:open“ (cesta alebo null = len do popredia)
pub fn listen(emit: flux_core::Emit) {
    let Ok(l) = TcpListener::bind("127.0.0.1:0") else { return };
    let Ok(addr) = l.local_addr() else { return };
    let _ = std::fs::write(port_file(), serde_json::json!({ "port": addr.port(), "pid": std::process::id() }).to_string());
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
            let mut r = BufReader::new(&s);
            let mut line = String::new();
            if r.read_line(&mut line).is_err() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else { continue };
            if v["flux"] != 1 {
                continue;
            }
            emit("ipc:open", v["open"].clone());
            let _ = (&s).write_all(b"flux-ok\n");
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn single_forward_roundtrip() {
        let dir = std::env::temp_dir().join(format!("flux-single-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("FLUX_USER_DATA", &dir);
        assert!(!super::forward(None));
        let got = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        let g = got.clone();
        super::listen(std::sync::Arc::new(move |ch: &str, v: serde_json::Value| g.lock().unwrap().push((ch.to_string(), v))));
        assert!(super::forward(Some("/tmp/a.py")));
        assert_eq!(got.lock().unwrap()[0], ("ipc:open".to_string(), serde_json::json!("/tmp/a.py")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
