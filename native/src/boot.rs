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
        if !forced {
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
