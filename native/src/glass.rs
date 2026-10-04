// Priehľadné okno „rozmazané“: Windows rozmaže to, čo je za Fluxom (ako ponuka Štart). Robí to DWM, nie Flux,
// takže to nestojí výkon ani pamäť. Klasické rozmazanie (ACCENT_ENABLE_BLURBEHIND) funguje aj na okne bez rámu
// vo Windows 10 aj 11 a nespomaľuje ťahanie okna ako Acrylic; farbu/stmavenie kreslí Flux sám (glassDim).

#[cfg(windows)]
pub fn blur(w: &impl raw_window_handle::HasWindowHandle, on: bool) -> bool {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    #[repr(C)]
    struct Accent {
        state: u32,
        flags: u32,
        gradient: u32,
        anim: u32,
    }
    #[repr(C)]
    struct Data {
        attrib: u32,
        data: *mut std::ffi::c_void,
        size: usize,
    }
    type SetWca = unsafe extern "system" fn(*mut std::ffi::c_void, *mut Data) -> i32;
    let Ok(h) = w.window_handle() else { return false };
    let RawWindowHandle::Win32(h) = h.as_raw() else { return false };
    let hwnd = h.hwnd.get() as *mut std::ffi::c_void;
    unsafe {
        let user32: Vec<u16> = "user32.dll\0".encode_utf16().collect();
        let m = GetModuleHandleW(user32.as_ptr());
        if m.is_null() {
            return false;
        }
        // nezdokumentovaná funkcia user32 (používa ju aj Windows Terminal, Tauri…) – načíta sa až za behu
        let Some(f) = GetProcAddress(m, b"SetWindowCompositionAttribute\0".as_ptr()) else { return false };
        let f: SetWca = std::mem::transmute(f);
        let mut accent = Accent { state: if on { 3 } else { 0 }, flags: 0, gradient: 0, anim: 0 }; // 3 = ACCENT_ENABLE_BLURBEHIND, 0 = vypnuté
        let mut data = Data { attrib: 19, data: &mut accent as *mut Accent as *mut _, size: std::mem::size_of::<Accent>() }; // 19 = WCA_ACCENT_POLICY
        f(hwnd, &mut data) != 0
    }
}

#[cfg(not(windows))]
pub fn blur<T>(_w: &T, _on: bool) -> bool {
    false
}

// Priehľadné okno: Windows pod ním kreslí vlastný rám a tlačidlá − □ ×, ktoré presvitali cez lištu Fluxu
// (zdvojené tlačidlá). Vypnúť kreslenie rámu cez DWM.
#[cfg(windows)]
pub fn no_frame(w: &impl raw_window_handle::HasWindowHandle) -> bool {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY};
    let Ok(h) = w.window_handle() else { return false };
    let RawWindowHandle::Win32(h) = h.as_raw() else { return false };
    let policy: i32 = DWMNCRP_DISABLED;
    unsafe { DwmSetWindowAttribute(h.hwnd.get() as _, DWMWA_NCRENDERING_POLICY as u32, &policy as *const i32 as *const _, 4) == 0 }
}

#[cfg(not(windows))]
pub fn no_frame<T>(_w: &T) -> bool {
    false
}

// Windows 11: oblé rohy okna a bez 1 px okraja (svetlá čiara hore cez priehľadnú lištu). Windows 10 to ignoruje.
#[cfg(windows)]
pub fn round(w: &impl raw_window_handle::HasWindowHandle) {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
    let Ok(h) = w.window_handle() else { return };
    let RawWindowHandle::Win32(h) = h.as_raw() else { return };
    let hwnd = h.hwnd.get() as _;
    let corner: i32 = 2; // DWMWA_WINDOW_CORNER_PREFERENCE (33) = DWMWCP_ROUND
    let border: u32 = 0xFFFF_FFFE; // DWMWA_BORDER_COLOR (34) = DWMWA_COLOR_NONE
    unsafe {
        DwmSetWindowAttribute(hwnd, 33, &corner as *const i32 as *const _, 4);
        DwmSetWindowAttribute(hwnd, 34, &border as *const u32 as *const _, 4);
    }
}

#[cfg(not(windows))]
pub fn round<T>(_w: &T) {}

// winit pri okne bez rámu posunie klientsku plochu o 1 px nadol (kvôli tieňu, WM_NCCALCSIZE) – ten 1 px hore
// kreslí Windows ako tenkú čiaru nad lištou. Vlastná obsluha okna to po winite vráti späť (okrem maximalizovaného).
#[cfg(windows)]
static OLD_PROC: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(windows)]
unsafe extern "system" fn nc_proc(hwnd: windows_sys::Win32::Foundation::HWND, msg: u32, wp: usize, lp: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{CallWindowProcW, IsZoomed, NCCALCSIZE_PARAMS, WM_NCCALCSIZE, WNDPROC};
    let old: WNDPROC = std::mem::transmute(OLD_PROC.load(std::sync::atomic::Ordering::Relaxed));
    let r = CallWindowProcW(old, hwnd, msg, wp, lp);
    if msg == WM_NCCALCSIZE && wp != 0 && lp != 0 && IsZoomed(hwnd) == 0 {
        let p = &mut *(lp as *mut NCCALCSIZE_PARAMS);
        p.rgrc[0].top -= 1;
        p.rgrc[0].bottom -= 1;
    }
    r
}

#[cfg(windows)]
pub fn no_top_line(w: &impl raw_window_handle::HasWindowHandle) {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, SetWindowPos, GWLP_WNDPROC, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_NOACTIVATE};
    let Ok(h) = w.window_handle() else { return };
    let RawWindowHandle::Win32(h) = h.as_raw() else { return };
    let hwnd = h.hwnd.get() as _;
    if OLD_PROC.load(std::sync::atomic::Ordering::Relaxed) != 0 {
        return;
    }
    unsafe {
        let old = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, nc_proc as *const () as isize);
        if old == 0 {
            return;
        }
        OLD_PROC.store(old, std::sync::atomic::Ordering::Relaxed);
        // prepočítať rám hneď
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

#[cfg(not(windows))]
pub fn no_top_line<T>(_w: &T) {}
