// Prechod zo starého Fluxu (Electron) na Flux Native – jeden Flux namiesto dvoch.
// Electron 1.5+ spustí `Flux-Native.exe --migrate <pid> <Flux.exe> [--no-window]` a skončí. Tu sa len
// presmerujú odkazy a pripnutie na paneli úloh na tento program a zapíše sa to do nastavení.
// Zámerne nič v registroch a žiadne mazanie iných programov: s tým (0.9.18) Windows Defender označil
// Flux Native za trójsky kôň a zmazal ho. Typy súborov zapisuje starý Flux sám (toNative.js), ten zostáva
// ako spúšťač, ktorý hneď odovzdá Native.
use serde_json::json;
use std::path::{Path, PathBuf};

fn low(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
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
    let mut s = flux_core::settings::load();
    if let Some(o) = s.as_object_mut() {
        o.insert("migratedFromElectron".into(), json!({ "at": chrono::Local::now().to_rfc3339(), "links": n }));
        // ďalší štart ukáže „Čo je nové“ s poslednými verziami
        o.insert("nativeSeenVersion".into(), json!("0.9.14"));
        o.insert("nativePath".into(), json!(exe.to_string_lossy()));
    }
    flux_core::settings::save(&s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_path_match() {
        assert_eq!(low(Path::new(r"C:\Users\A\AppData\Local\Programs\flux\Flux.exe")), low(Path::new(r"c:/users/a/appdata/local/programs/flux/flux.exe")));
    }
}
