// Presmerovanie odkazov (pripnutie, ponuka Štart, plocha) zo starého Fluxu (Electron) – len raz pri --migrate.
// Pri bežnom štarte Flux odkazy neprehľadáva ani neprepisuje (Windows Defender to považuje za podozrivé).

#[cfg(not(windows))]
pub fn retarget(_matches: &dyn Fn(&std::path::Path) -> bool, _exe: &std::path::Path, _clear_id: bool) -> usize {
    0
}

// priečinky s odkazmi: pripnutia na paneli úloh, Quick Launch, ponuka Štart, plocha
#[cfg(windows)]
fn links() -> Vec<std::path::PathBuf> {
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
    links
}

// nasmeruje odkazy, ktorých cieľ spĺňa `matches`, na `exe`; clear_id zmaže AppUserModelID (odkazy Electron Fluxu
// majú „dev.flux.ide“ – bez zmazania by okno Flux Native malo na paneli úloh vlastné tlačidlo vedľa pripnutia)
#[cfg(windows)]
pub fn retarget(matches: &dyn Fn(&std::path::Path) -> bool, exe: &std::path::Path, clear_id: bool) -> usize {
    use windows::core::{Interface, HSTRING, PCWSTR};
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READWRITE};
    use windows::Win32::Foundation::PROPERTYKEY;
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
    use windows::Win32::UI::Shell::{IShellLinkW, SHChangeNotify, ShellLink, SHCNE_UPDATEITEM, SHCNF_PATHW};
    // PKEY_AppUserModel_ID = {9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}, 5
    const AUMID: PROPERTYKEY = PROPERTYKEY { fmtid: windows::core::GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3), pid: 5 };
    let links = links();
    let mut n = 0;
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
            let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
            let target = std::path::PathBuf::from(String::from_utf16_lossy(&buf[..len]));
            if !matches(&target) {
                continue;
            }
            if link.SetPath(&HSTRING::from(exe.as_os_str())).is_err() {
                continue;
            }
            if let Some(dir) = exe.parent() {
                let _ = link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()));
            }
            if clear_id {
                let _ = link.SetIconLocation(&HSTRING::from(exe.as_os_str()), 0);
                if let Ok(ps) = link.cast::<IPropertyStore>() {
                    if ps.SetValue(&AUMID, &PROPVARIANT::default()).is_ok() {
                        let _ = ps.Commit();
                    }
                }
            }
            if pf.Save(PCWSTR::null(), true).is_ok() {
                SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, Some(lw.as_ptr() as _), None);
                n += 1;
            }
        }
    }
    n
}
