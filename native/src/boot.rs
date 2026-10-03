// Záznam štartu a núdzový režim: userData/flux-native-start.json = { stage, pid, fails, version }.
// stage: starting (pred oknom) → window (App::new) → ok (3 s snímok) → closed (zavreté).
// Ak minulé spustenie neprešlo do „ok“ (spadlo alebo visí bez okna), ďalší štart je núdzový:
// bez priehľadného okna a tapety, od druhého zlyhania aj s grafikou wgpu. Visiaci proces sa ukončí.
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

pub static SAFE: AtomicBool = AtomicBool::new(false);
pub static FAILS: AtomicU8 = AtomicU8::new(0);

fn file() -> std::path::PathBuf {
    flux_core::settings::user_data().join("flux-native-start.json")
}

fn read() -> Value {
    std::fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
}

fn write(stage: &str, fails: u8) {
    let f = file();
    let _ = std::fs::create_dir_all(f.parent().unwrap_or(std::path::Path::new(".")));
    let v = json!({ "stage": stage, "pid": std::process::id(), "fails": fails, "version": env!("CARGO_PKG_VERSION") });
    let _ = std::fs::write(f, v.to_string());
}

pub fn safe() -> bool {
    SAFE.load(Ordering::Relaxed)
}

// na začiatku main(): vyhodnotí minulé spustenie
pub fn begin() {
    let last = read();
    let stage = last["stage"].as_str().unwrap_or("closed");
    let mut fails = last["fails"].as_u64().unwrap_or(0).min(9) as u8;
    let forced = std::env::var("FLUX_SAFE").as_deref() == Ok("1");
    if stage == "hidden" {
        fails = fails.max(1);
    }
    if stage == "starting" || stage == "window" {
        fails += 1;
        // minulá inštancia ešte beží bez okna → ukončiť, inak by sa hromadili neviditeľné procesy
        if let Some(pid) = last["pid"].as_u64() {
            kill_if_flux(pid as u32);
        }
    }
    FAILS.store(fails, Ordering::Relaxed);
    if fails > 0 || forced {
        SAFE.store(true, Ordering::Relaxed);
        if !forced || stage == "hidden" {
            crate::notice(&format!(
                "Flux Native did not start properly last time (stage: {stage}), so it is starting in safe mode now: no see-through window and no wallpaper{}.\n\nIf it works, it will start normally next time.",
                if fails >= 2 { ", simpler graphics" } else { "" }
            ));
        }
    }
    write("starting", fails);
}

// okno vzniklo (App::new)
pub fn window() {
    write("window", FAILS.load(Ordering::Relaxed));
}

// Flux beží a kreslí → ďalší štart je normálny
pub fn ok() {
    eprintln!("[flux] started ok");
    FAILS.store(0, Ordering::Relaxed);
    write("ok", 0);
}

// len ak je záznam náš – pri reštarte po aktualizácii už môže patriť novej inštancii
pub fn closed() {
    if read()["pid"].as_u64() == Some(std::process::id() as u64) {
        write("closed", 0);
    }
}

#[cfg(windows)]
fn kill_if_flux(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE};
    if pid == std::process::id() {
        return;
    }
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE, 0, pid);
        if h.is_null() {
            return;
        }
        // len ak je to naozaj Flux Native (pid sa mohol medzitým použiť inde)
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        if QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut n) != 0 {
            let name = String::from_utf16_lossy(&buf[..n as usize]).to_lowercase();
            let me = std::env::current_exe().ok().and_then(|e| e.file_name().map(|f| f.to_string_lossy().to_lowercase())).unwrap_or_default();
            if name.ends_with(&me) || name.contains("flux-native") {
                TerminateProcess(h, 1);
            }
        }
        CloseHandle(h);
    }
}

#[cfg(not(windows))]
fn kill_if_flux(_pid: u32) {}

// tvrdé pády mimo Rustu (ovládač grafiky, Media Foundation, WebView2…) → záznam do crash logu
#[cfg(windows)]
pub fn catch_hard_crashes() {
    use windows_sys::Win32::System::Diagnostics::Debug::{SetUnhandledExceptionFilter, EXCEPTION_POINTERS};
    use windows_sys::Win32::System::Threading::SetThreadStackGuarantee;
    unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
        let (code, addr) = unsafe { info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()).map(|r| (r.ExceptionCode as u32, r.ExceptionAddress as usize)).unwrap_or((0, 0)) };
        let msg = match code {
            0xC00000FD => "stack overflow".to_string(),
            0xC0000005 => "access violation".to_string(),
            _ => "crash".to_string(),
        };
        crate::report(&format!("{msg} (code 0x{code:08X} at 0x{addr:X})"));
        1 // EXCEPTION_EXECUTE_HANDLER – ukončiť
    }
    unsafe {
        let mut g: u32 = 64 * 1024;
        SetThreadStackGuarantee(&mut g);
        SetUnhandledExceptionFilter(Some(filter));
    }
}

#[cfg(not(windows))]
pub fn catch_hard_crashes() {}

// má proces viditeľné okno najvyššej úrovne?
#[cfg(windows)]
fn has_visible_window(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible};
    struct Q {
        pid: u32,
        found: bool,
    }
    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> i32 {
        let q = unsafe { &mut *(l as *mut Q) };
        let mut p = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut p) };
        // skutočné okno: viditeľné a aspoň 300×200 (nie pomocné 16×16 okná)
        let mut r = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let big = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(h, &mut r) } != 0 && r.right - r.left >= 300 && r.bottom - r.top >= 200;
        // zbalené okno (na paneli úloh) je v poriadku
        let big = big || unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(h) } != 0;
        if p == q.pid && big && unsafe { IsWindowVisible(h) } != 0 {
            q.found = true;
            return 0;
        }
        1
    }
    let mut q = Q { pid, found: false };
    unsafe { EnumWindows(Some(each), &mut q as *mut Q as LPARAM) };
    q.found
}

// „duchovia“: iné procesy Flux Native bez viditeľného okna (napr. z priehľadného okna vo verziách 0.7–0.8.3)
// blokujú premenovanie aj prepísanie programu – ukončiť ich
#[cfg(windows)]
pub fn kill_ghosts() {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    let me = std::process::id();
    let mut ghosts = vec![];
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut e) != 0;
        while ok {
            let n = e.szExeFile.iter().position(|c| *c == 0).unwrap_or(e.szExeFile.len());
            let name = String::from_utf16_lossy(&e.szExeFile[..n]).to_lowercase();
            if e.th32ProcessID != me && name.starts_with("flux-native") && name.ends_with(".exe") {
                ghosts.push(e.th32ProcessID);
            }
            ok = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
    }
    for pid in ghosts {
        // most MCP (--mcp-bridge) okno nemá a byť ho má
        if !has_visible_window(pid) && !crate::mcp::bridge_mark(pid).exists() {
            unsafe {
                let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
                if !h.is_null() {
                    TerminateProcess(h, 1);
                    CloseHandle(h);
                }
            }
        }
    }
}

#[cfg(not(windows))]
pub fn kill_ghosts() {}

// strážca: ak po 12 s nemá Flux viditeľné okno, spustí sa znova v núdzovom režime a tento proces skončí
#[cfg(windows)]
pub fn watchdog() {
    if safe() || std::env::var("FLUX_NO_WATCHDOG").is_ok() {
        return;
    }
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(12));
        if has_visible_window(std::process::id()) {
            return;
        }
        write("hidden", FAILS.load(Ordering::Relaxed).saturating_add(1));
        if let Ok(e) = std::env::current_exe() {
            let _ = std::process::Command::new(e).env("FLUX_SAFE", "1").spawn();
        }
        std::process::exit(2);
    });
}

#[cfg(not(windows))]
pub fn watchdog() {}
