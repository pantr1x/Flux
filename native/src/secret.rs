// Tajné údaje Flux Native (GitHub token, API kľúč Anthropic, tokeny MCP) – mimo settings.json.
// Windows: Správca poverení (Credential Manager, ciele „Flux Native/<kľúč>“) – bežné miesto pre tokeny aplikácií.
// Inde: userData/flux-native-secrets.json s právami 0600.
// Electron Flux ich šifruje cez safeStorage („enc:…“), to Rust neprečíta – Native má vlastné.
#[cfg(not(windows))]
use serde_json::{json, Value};

#[cfg(not(windows))]
fn file() -> std::path::PathBuf {
    flux_core::settings::user_data().join("flux-native-secrets.json")
}

#[cfg(not(windows))]
fn load() -> Value {
    std::fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(json!({}))
}

#[cfg(not(windows))]
fn store(v: &Value) {
    let f = file();
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&f, serde_json::to_string_pretty(v).unwrap_or_default());
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(windows))]
pub fn get(key: &str) -> Option<String> {
    load()[key].as_str().filter(|s| !s.is_empty()).map(String::from)
}

#[cfg(not(windows))]
pub fn set(key: &str, value: &str) {
    let mut v = load();
    v[key] = json!(value);
    store(&v);
}

#[cfg(not(windows))]
pub fn remove(key: &str) {
    let mut v = load();
    if let Some(o) = v.as_object_mut() {
        if o.remove(key).is_some() {
            store(&v);
        }
    }
}

#[cfg(windows)]
fn target(key: &str) -> Vec<u16> {
    format!("Flux Native/{key}").encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub fn get(key: &str) -> Option<String> {
    use windows_sys::Win32::Security::Credentials::{CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC};
    let t = target(key);
    let mut c: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        if CredReadW(t.as_ptr(), CRED_TYPE_GENERIC, 0, &mut c) == 0 || c.is_null() {
            return None;
        }
        let r = &*c;
        let v = std::slice::from_raw_parts(r.CredentialBlob, r.CredentialBlobSize as usize).to_vec();
        CredFree(c as *const _);
        String::from_utf8(v).ok().filter(|s| !s.is_empty())
    }
}

#[cfg(windows)]
pub fn set(key: &str, value: &str) {
    use windows_sys::Win32::Security::Credentials::{CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC};
    let mut t = target(key);
    let mut user: Vec<u16> = "Flux".encode_utf16().chain(std::iter::once(0)).collect();
    let mut blob = value.as_bytes().to_vec();
    let c = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: t.as_mut_ptr(),
        Comment: std::ptr::null_mut(),
        LastWritten: unsafe { std::mem::zeroed() },
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: std::ptr::null_mut(),
        UserName: user.as_mut_ptr(),
    };
    unsafe { CredWriteW(&c, 0) };
}

#[cfg(windows)]
pub fn remove(key: &str) {
    use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};
    let t = target(key);
    unsafe { CredDeleteW(t.as_ptr(), CRED_TYPE_GENERIC, 0) };
}

// „…abcd“ – koniec kľúča na ukážku (keyHint v ai.js)
pub fn hint(key: &str) -> Option<String> {
    get(key).map(|k| format!("…{}", k.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<String>()))
}
