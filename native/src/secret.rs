// Tajné údaje Flux Native (GitHub token, API kľúč Anthropic, tokeny MCP) – mimo settings.json.
// Electron Flux ich šifruje cez safeStorage („enc:…“), to Rust neprečíta, preto má Native vlastný súbor:
// userData/flux-native-secrets.json, na Windows každá hodnota zašifrovaná cez DPAPI (len tento používateľ).
use serde_json::{json, Value};
use std::path::PathBuf;

fn file() -> PathBuf {
    flux_core::settings::user_data().join("flux-native-secrets.json")
}

fn load() -> Value {
    std::fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(json!({}))
}

fn store(v: &Value) {
    let f = file();
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&f, serde_json::to_string_pretty(v).unwrap_or_default());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600));
    }
}

pub fn get(key: &str) -> Option<String> {
    let v = load();
    let s = v[key].as_str()?;
    let out = if let Some(h) = s.strip_prefix("dpapi:") { unprotect(&unhex(h)?)? } else { s.as_bytes().to_vec() };
    String::from_utf8(out).ok().filter(|s| !s.is_empty())
}

pub fn set(key: &str, value: &str) {
    let mut v = load();
    let enc = match protect(value.as_bytes()) {
        Some(b) => format!("dpapi:{}", hex(&b)),
        None => value.to_string(),
    };
    v[key] = json!(enc);
    store(&v);
}

pub fn remove(key: &str) {
    let mut v = load();
    if let Some(o) = v.as_object_mut() {
        if o.remove(key).is_some() {
            store(&v);
        }
    }
}

// „…abcd“ – koniec kľúča na ukážku (keyHint v ai.js)
pub fn hint(key: &str) -> Option<String> {
    get(key).map(|k| format!("…{}", k.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<String>()))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    (0..s.len()).step_by(2).map(|i| s.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok())).collect()
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Option<Vec<u8>> {
    dpapi(data, true)
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    dpapi(data, false)
}

#[cfg(windows)]
fn dpapi(data: &[u8], encrypt: bool) -> Option<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
    let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    // CRYPTPROTECT_UI_FORBIDDEN = 1
    let ok = unsafe {
        if encrypt {
            CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 1, &mut out)
        } else {
            CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 1, &mut out)
        }
    };
    if ok == 0 || out.pbData.is_null() {
        return None;
    }
    let v = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec() };
    unsafe { LocalFree(out.pbData as _) };
    Some(v)
}

// mimo Windows: súbor s právami 0600, bez šifrovania
#[cfg(not(windows))]
fn protect(_: &[u8]) -> Option<Vec<u8>> {
    None
}

#[cfg(not(windows))]
fn unprotect(_: &[u8]) -> Option<Vec<u8>> {
    None
}
