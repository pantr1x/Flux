// HTTP cez systémový curl (súčasť Windows 10/11) – Flux nepotrebuje vlastného HTTP klienta ani TLS knižnicu.
// Adresa, hlavičky aj telo idú curl-u cez stdin ako konfigurácia (-K -), takže tokeny nie sú v príkazovom riadku.
use std::io::Write;
use std::process::{Child, Command, Stdio};

fn curl() -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new("curl");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // bez okna konzoly
    }
    c
}

// reťazec pre konfiguračný súbor curl-u ("…" s \\ \" \n)
fn q(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => o.push_str("\\\\"),
            '"' => o.push_str("\\\""),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn config(method: &str, url: &str, headers: &[String], body: Option<&str>, timeout: u32) -> String {
    let mut c = format!("url = {}\nrequest = {}\nsilent\nshow-error\nlocation\nmax-time = {timeout}\n", q(url), q(method));
    c.push_str(&format!("header = {}\n", q("User-Agent: Flux")));
    for h in headers {
        c.push_str(&format!("header = {}\n", q(h)));
    }
    if let Some(b) = body {
        c.push_str(&format!("data-binary = {}\n", q(b)));
    }
    c
}

fn spawn(cfg: &str, extra: &[&str]) -> Result<Child, String> {
    let mut child = curl().args(["-K", "-"]).args(extra).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("curl: {e}"))?;
    if let Some(mut i) = child.stdin.take() {
        let _ = i.write_all(cfg.as_bytes());
    }
    Ok(child)
}

// jedna požiadavka → (HTTP kód, telo)
pub fn request(method: &str, url: &str, headers: &[String], body: Option<&str>, timeout: u32) -> Result<(u16, String), String> {
    let cfg = config(method, url, headers, body, timeout);
    let out = spawn(&cfg, &["-w", "\n%{http_code}"])?.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() && text.trim().is_empty() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { "No connection.".into() } else { err });
    }
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
    Ok((code.trim().parse().unwrap_or(0), body.to_string()))
}

// JSON odpoveď; chyba = správa z tela (message / error_description / error.message)
pub fn json(method: &str, url: &str, headers: &[String], body: Option<&str>) -> Result<serde_json::Value, String> {
    let (code, text) = request(method, url, headers, body, 60)?;
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if !(200..300).contains(&code) {
        let msg = v["error"]["message"].as_str().or(v["message"].as_str()).or(v["error_description"].as_str()).map(String::from).unwrap_or_else(|| format!("HTTP {code}"));
        return Err(msg);
    }
    Ok(v)
}

// prúd odpovede (SSE) – volajúci číta stdout po riadkoch a môže proces zabiť (Zastaviť)
pub fn stream(url: &str, headers: &[String], body: &str) -> Result<Child, String> {
    let cfg = config("POST", url, headers, Some(body), 600);
    spawn(&cfg, &["-N"])
}

// súbor (napr. avatar) do pamäte
pub fn bytes(url: &str) -> Option<Vec<u8>> {
    let cfg = config("GET", url, &[], None, 30);
    let out = spawn(&cfg, &["-f"]).ok()?.wait_with_output().ok()?;
    out.status.success().then_some(out.stdout)
}
