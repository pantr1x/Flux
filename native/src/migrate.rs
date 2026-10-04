// Prechod zo starého Fluxu (Electron) na Flux Native – jeden Flux namiesto dvoch.
// Electron 1.5+ spustí `Flux-Native.exe --migrate <pid> <Flux.exe> [--no-window]` a skončí. Tu:
// odkazy a pripnutie na paneli úloh → na nás, typy súborov („Otvoriť vo Fluxe“) → na nás, priečinok Electronu preč.
// Nastavenia sú spoločné (%APPDATA%\Flux\settings.json), takže projekty a všetko ostatné ostane.
use serde_json::json;
use std::path::{Path, PathBuf};

// tie isté prípony ako build/installer.nsh (FluxTypes)
#[cfg_attr(not(windows), allow(dead_code))]
pub const EXTS: &[&str] = &[
    "txt", "md", "markdown", "log", "csv", "json", "xml", "yml", "yaml", "toml", "ini", "cfg", "py", "pyw", "js", "mjs", "cjs", "ts", "jsx", "tsx", "html", "htm", "css", "scss", "java", "c", "h", "cpp", "hpp",
    "cs", "go", "rs", "rb", "php", "lua", "sh", "bat", "ps1", "sql", "pl",
];

fn low(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

// priečinok Electronu sa smie zmazať len keď je to naozaj Flux (resources\app.asar), je v %LOCALAPPDATA%
// (inštalácia pre jedného používateľa) a nie je to náš vlastný priečinok
pub fn removable(dir: &Path, local: &Path, ours: &Path) -> bool {
    let (d, l, o) = (low(dir), low(local), low(ours));
    !l.is_empty() && d.starts_with(&format!("{l}\\")) && d != o && !o.starts_with(&format!("{d}\\")) && dir.join("resources").join("app.asar").is_file()
}

pub fn run(args: &[String]) {
    let i = args.iter().position(|a| a == "--migrate").unwrap_or(0);
    let pid = args.get(i + 1).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
    let Some(old) = args.get(i + 2).map(PathBuf::from) else { return };
    let Ok(exe) = std::env::current_exe() else { return };
    if pid != 0 {
        crate::update::wait_exit(pid, std::time::Duration::from_secs(30));
    }
    let old_l = low(&old);
    let n = crate::pins::retarget(&|t| low(t) == old_l, &exe, true);
    associate(&exe);
    let mut removed = false;
    if let (Some(dir), Some(local), Some(ours)) = (old.parent(), std::env::var_os("LOCALAPPDATA"), exe.parent()) {
        if removable(dir, Path::new(&local), ours) {
            removed = remove_dir(dir);
            if removed {
                forget_uninstall(dir);
            }
        }
    }
    let mut s = flux_core::settings::load();
    if let Some(o) = s.as_object_mut() {
        o.insert("migratedFromElectron".into(), json!({ "at": chrono::Local::now().to_rfc3339(), "links": n, "removed": removed }));
        // ďalší štart ukáže „Čo je nové“ s poslednými verziami
        o.insert("nativeSeenVersion".into(), json!("0.9.14"));
        o.insert("nativePath".into(), json!(exe.to_string_lossy()));
    }
    flux_core::settings::save(&s);
}

fn remove_dir(dir: &Path) -> bool {
    let t = std::time::Instant::now();
    loop {
        if std::fs::remove_dir_all(dir).is_ok() || !dir.exists() {
            return true;
        }
        if t.elapsed() > std::time::Duration::from_secs(10) {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

#[cfg(windows)]
mod reg {
    use windows_sys::Win32::System::Registry::*;
    pub fn w(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
    // HKCU\<key> [name] = value (REG_SZ); name None = predvolená hodnota
    pub fn set(key: &str, name: Option<&str>, value: &str) {
        unsafe {
            let mut h: HKEY = std::ptr::null_mut();
            if RegCreateKeyExW(HKEY_CURRENT_USER, w(key).as_ptr(), 0, std::ptr::null(), 0, KEY_WRITE, std::ptr::null(), &mut h, std::ptr::null_mut()) != 0 {
                return;
            }
            let v = w(value);
            let n = name.map(w);
            RegSetValueExW(h, n.as_ref().map(|n| n.as_ptr()).unwrap_or(std::ptr::null()), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32);
            RegCloseKey(h);
        }
    }
    pub fn get(root: HKEY, key: &str, name: &str) -> Option<String> {
        unsafe {
            let mut buf = vec![0u16; 1024];
            let mut len = (buf.len() * 2) as u32;
            if RegGetValueW(root, w(key).as_ptr(), w(name).as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr() as _, &mut len) != 0 {
                return None;
            }
            let n = buf.iter().position(|c| *c == 0).unwrap_or(0);
            Some(String::from_utf16_lossy(&buf[..n]))
        }
    }
    pub fn subkeys(key: &str) -> Vec<String> {
        let mut out = vec![];
        unsafe {
            let mut h: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, w(key).as_ptr(), 0, KEY_READ, &mut h) != 0 {
                return out;
            }
            let mut i = 0;
            loop {
                let mut buf = [0u16; 256];
                let mut len = buf.len() as u32;
                if RegEnumKeyExW(h, i, buf.as_mut_ptr(), &mut len, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) != 0 {
                    break;
                }
                out.push(String::from_utf16_lossy(&buf[..len as usize]));
                i += 1;
            }
            RegCloseKey(h);
        }
        out
    }
    pub fn delete_tree(key: &str) {
        unsafe {
            RegDeleteTreeW(HKEY_CURRENT_USER, w(key).as_ptr());
        }
    }
}

// „Otvoriť vo Fluxe“ a typy súborov Flux.<prípona> → tento program (to isté, čo zapisoval inštalátor Electronu)
#[cfg(windows)]
pub fn associate(exe: &Path) {
    let e = exe.to_string_lossy();
    let name = exe.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Flux-Native.exe".into());
    let cmd = format!("\"{e}\" \"%1\"");
    let icon = format!("{e},0");
    let cls = r"Software\Classes";
    reg::set(&format!(r"{cls}\Flux.File"), None, "Flux file");
    reg::set(&format!(r"{cls}\Flux.File\DefaultIcon"), None, &icon);
    reg::set(&format!(r"{cls}\Flux.File\shell\open\command"), None, &cmd);
    let app = format!(r"{cls}\Applications\{name}");
    reg::set(&app, Some("FriendlyAppName"), "Flux");
    reg::set(&format!(r"{app}\DefaultIcon"), None, &icon);
    reg::set(&format!(r"{app}\shell\open\command"), None, &cmd);
    for x in EXTS {
        let t = format!(r"{cls}\Flux.{x}");
        reg::set(&t, None, &format!("{x} file (Flux)"));
        reg::set(&format!(r"{t}\DefaultIcon"), None, &icon);
        reg::set(&format!(r"{t}\shell\open\command"), None, &cmd);
        reg::set(&format!(r"{cls}\.{x}\OpenWithProgids"), Some(&format!("Flux.{x}")), "");
        reg::set(&format!(r"{app}\SupportedTypes"), Some(&format!(".{x}")), "");
    }
    unsafe {
        use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}

#[cfg(not(windows))]
pub fn associate(_exe: &Path) {}

// záznam „Flux“ v Aplikáciách (Pridať/odstrániť programy) po zmazaní priečinka
#[cfg(windows)]
fn forget_uninstall(dir: &Path) {
    let base = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
    let d = low(dir);
    for k in reg::subkeys(base) {
        let key = format!(r"{base}\{k}");
        let loc = reg::get(windows_sys::Win32::System::Registry::HKEY_CURRENT_USER, &key, "InstallLocation").unwrap_or_default();
        if !loc.is_empty() && low(Path::new(loc.trim_matches('"'))) == d {
            reg::delete_tree(&key);
        }
    }
}

#[cfg(not(windows))]
fn forget_uninstall(_dir: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_removable_rules() {
        let tmp = std::env::temp_dir().join(format!("flux-mig-{}", std::process::id()));
        let local = tmp.join("Local");
        let app = local.join("Programs").join("flux");
        std::fs::create_dir_all(app.join("resources")).unwrap();
        let ours = local.join("Programs").join("Flux Native");
        // bez app.asar to nie je Electron Flux
        assert!(!removable(&app, &local, &ours));
        std::fs::write(app.join("resources").join("app.asar"), b"x").unwrap();
        assert!(removable(&app, &local, &ours));
        // mimo %LOCALAPPDATA% (Program Files) nie
        assert!(!removable(&app, &tmp.join("Other"), &ours));
        // náš vlastný priečinok ani jeho rodič nie
        assert!(!removable(&app, &local, &app));
        assert!(!removable(&app, &local, &app.join("sub")));
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(EXTS.contains(&"py") && EXTS.len() == 40);
    }
}
