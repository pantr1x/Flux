// Oprava odkazov na Flux (pripnutie na paneli úloh, ponuka Štart, plocha). Aktualizácie do 0.9.9 premenovali bežiaci
// program na .old – Windows presunul pripnutie za ním a po zmazaní .old ostal odkaz mŕtvy. Pri štarte ho nasmerujeme späť.

#[cfg(windows)]
pub fn repair() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let _ = run();
    });
}

#[cfg(not(windows))]
pub fn repair() {}

// odkaz ukazuje na našu starú kópiu (.old, .old-<čas>, .bad) alebo na neexistujúci súbor s naším menom v našom priečinku
#[cfg_attr(not(windows), allow(dead_code))]
fn stale(target: &std::path::Path, exe: &std::path::Path) -> bool {
    let low = |p: &std::path::Path| p.to_string_lossy().to_lowercase();
    if target.as_os_str().is_empty() || low(target) == low(exe) || target.parent().map(low) != exe.parent().map(low) {
        return false;
    }
    let stem = exe.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let name = target.file_name().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    name == format!("{stem}.old") || name.starts_with(&format!("{stem}.old-")) || name == format!("{stem}.bad") || (name.starts_with(&stem) && !target.exists())
}

#[cfg(windows)]
fn run() -> windows::core::Result<()> {
    use windows::core::{Interface, HSTRING, PCWSTR};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READWRITE};
    use windows::Win32::UI::Shell::{IShellLinkW, SHChangeNotify, ShellLink, SHCNE_UPDATEITEM, SHCNF_PATHW};
    let Ok(exe) = std::env::current_exe() else { return Ok(()) };
    let env = |k: &str| std::env::var_os(k).map(std::path::PathBuf::from);
    let mut dirs = vec![];
    if let Some(a) = env("APPDATA") {
        dirs.push(a.join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar"));
        dirs.push(a.join(r"Microsoft\Internet Explorer\Quick Launch"));
        dirs.push(a.join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(u) = env("USERPROFILE") {
        dirs.push(u.join("Desktop"));
        dirs.push(u.join(r"OneDrive\Desktop"));
    }
    if let Some(t) = env("FLUX_PINS_DIR") {
        dirs.push(t); // CI test
    }
    let mut links = vec![];
    for d in &dirs {
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        for f in rd.flatten() {
            let p = f.path();
            if p.is_dir() {
                // ponuka Štart: jedna úroveň priečinkov
                if let Ok(sub) = std::fs::read_dir(&p) {
                    links.extend(sub.flatten().map(|s| s.path()));
                }
            } else {
                links.push(p);
            }
        }
    }
    links.retain(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")));
    if links.is_empty() {
        return Ok(());
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        for l in links {
            let Ok(link) = CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER) else { continue };
            let Ok(pf) = link.cast::<IPersistFile>() else { continue };
            let lw = HSTRING::from(l.as_os_str());
            if pf.Load(&lw, STGM_READWRITE).is_err() {
                continue;
            }
            let mut buf = [0u16; 1024];
            if link.GetPath(&mut buf, std::ptr::null_mut(), 0).is_err() {
                continue;
            }
            let n = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
            let target = std::path::PathBuf::from(String::from_utf16_lossy(&buf[..n]));
            if !stale(&target, &exe) {
                continue;
            }
            if link.SetPath(&HSTRING::from(exe.as_os_str())).is_ok() {
                if let Some(dir) = exe.parent() {
                    let _ = link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()));
                }
                if pf.Save(PCWSTR::null(), true).is_ok() {
                    SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, Some(lw.as_ptr() as _), None);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn stale_links() {
        let exe = std::path::Path::new("/x/Flux-Native(18).exe");
        assert!(super::stale("/x/Flux-Native(18).old".as_ref(), exe));
        assert!(super::stale("/X/flux-native(18).old-123".as_ref(), exe));
        assert!(!super::stale("/x/Flux-Native(18).exe".as_ref(), exe));
        assert!(!super::stale("/y/Flux-Native(18).old".as_ref(), exe));
        assert!(!super::stale("/x/Other.old".as_ref(), exe));
    }
}
