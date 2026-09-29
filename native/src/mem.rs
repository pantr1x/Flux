// Uvoľnenie pamäte na pozadí (ako trimMemory v Electron Fluxe): keď je okno 20 s neaktívne a potom
// každé 2 min, Windows presunie nepoužívané stránky z RAM (K32EmptyWorkingSet). Pri práci sa vrátia samé.
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Trim {
    since: Option<Instant>,
    last: Option<Instant>,
}

impl Trim {
    // volá sa každý snímok; vráti, kedy sa má okno znova zobudiť
    pub fn tick(&mut self, focused: bool, enabled: bool) -> Option<Duration> {
        if focused || !enabled {
            self.since = None;
            self.last = None;
            return None;
        }
        let since = *self.since.get_or_insert_with(Instant::now);
        let due = match self.last {
            None => since + Duration::from_secs(20),
            Some(l) => l + Duration::from_secs(120),
        };
        let now = Instant::now();
        if now >= due {
            trim();
            self.last = Some(now);
            return Some(Duration::from_secs(120));
        }
        Some(due - now)
    }
}

#[cfg(windows)]
fn trim() {
    unsafe {
        windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet(windows_sys::Win32::System::Threading::GetCurrentProcess());
    }
}

#[cfg(not(windows))]
fn trim() {}

pub fn trim_now() {
    trim();
}

// pamäť programu ako v Správcovi úloh: súkromná pracovná množina (Windows 10 1809+), nie celý „commit“ –
// do toho by sa rátala aj pamäť, ktorú si ovládač grafiky len rezervuje (preto intro ukazovalo ~400 MB)
#[cfg(windows)]
pub fn used_mb() -> Option<f64> {
    use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX2};
    unsafe {
        let mut c: PROCESS_MEMORY_COUNTERS_EX2 = std::mem::zeroed();
        c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32;
        if K32GetProcessMemoryInfo(windows_sys::Win32::System::Threading::GetCurrentProcess(), &mut c as *mut _ as *mut _, c.cb) != 0 {
            let v = if c.PrivateWorkingSetSize > 0 { c.PrivateWorkingSetSize } else { c.WorkingSetSize };
            return Some(v as f64 / 1048576.0);
        }
    }
    None
}

#[cfg(not(windows))]
pub fn used_mb() -> Option<f64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    let kb: f64 = s.lines().find(|l| l.starts_with("VmRSS:"))?.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024.0)
}
