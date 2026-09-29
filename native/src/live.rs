// Živé tapety ako v Electron Fluxe (src/main/liveWallpaper.js): ak beží Lively Wallpaper alebo
// Wallpaper Engine, tapeta Windows je stará – Flux zistí, čo majú tieto programy nastavené (video,
// GIF, obrázok; pri scénach a webových tapetách ich náhľad). Len číta ich súbory s nastaveniami.
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub file: PathBuf,
    pub video: bool,
    pub preview: Option<PathBuf>, // náhľad, keď sa video nedá prehrať
    pub source: &'static str,
}

const VIDEO: [&str; 7] = ["mp4", "webm", "mov", "m4v", "mkv", "avi", "wmv"];
const IMAGE: [&str; 6] = ["jpg", "jpeg", "png", "bmp", "webp", "gif"];

fn ext(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}
fn is_video(p: &Path) -> bool {
    VIDEO.contains(&ext(p).as_str())
}
fn is_image(p: &Path) -> bool {
    IMAGE.contains(&ext(p).as_str())
}
fn read_json(p: &Path) -> Option<Value> {
    let s = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(s.trim_start_matches('\u{feff}')).ok()
}
// všetky reťazce v JSON-e (štruktúra nastavení sa medzi verziami mení – hľadáme podľa obsahu)
fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

// príkaz bez okna konzoly
fn quiet(cmd: &str) -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new(cmd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

// ---------- Lively Wallpaper ----------
fn lively() -> Option<Found> {
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    let mut dirs = vec![local.join("Lively Wallpaper")];
    // verzia z Microsoft Store
    if let Ok(rd) = std::fs::read_dir(local.join("Packages")) {
        for d in rd.flatten() {
            if d.file_name().to_string_lossy().to_lowercase().contains("livelywallpaper") {
                dirs.push(d.path().join("LocalCache").join("Local").join("Lively Wallpaper"));
            }
        }
    }
    for dir in dirs {
        let Some(layout) = read_json(&dir.join("WallpaperLayout.json")) else { continue };
        let mut all = vec![];
        strings(&layout, &mut all);
        for folder in all.iter().filter(|s| s.contains('/') || s.contains('\\')).map(PathBuf::from) {
            let Some(info) = read_json(&folder.join("LivelyInfo.json")) else { continue };
            let pick = |k: &str| info[k].as_str().filter(|s| !s.is_empty()).map(|f| if Path::new(f).is_absolute() { PathBuf::from(f) } else { folder.join(f) }).filter(|p| p.is_file());
            let preview = [pick("Preview"), pick("Thumbnail")].into_iter().flatten().find(|p| is_image(p));
            if let Some(main) = pick("FileName").filter(|p| is_video(p) || is_image(p)) {
                return Some(Found { video: is_video(&main), file: main, preview, source: "Lively Wallpaper" });
            }
            // web, aplikácia, stream… → náhľad
            if let Some(p) = preview {
                return Some(Found { file: p, video: false, preview: None, source: "Lively Wallpaper" });
            }
        }
    }
    None
}

// ---------- Wallpaper Engine (Steam) ----------
fn steam_roots() -> Vec<PathBuf> {
    let mut roots = vec![];
    if let Some(p) = std::env::var_os("FLUX_STEAM_PATH") {
        roots.push(PathBuf::from(p));
    }
    if cfg!(windows) {
        if let Ok(o) = quiet("reg").args(["query", "HKCU\\Software\\Valve\\Steam", "/v", "SteamPath"]).output() {
            let out = String::from_utf8_lossy(&o.stdout).to_string();
            if let Some(v) = out.lines().find_map(|l| l.split("REG_SZ").nth(1)) {
                roots.push(PathBuf::from(v.trim()));
            }
        }
        roots.push("C:\\Program Files (x86)\\Steam".into());
        roots.push("C:\\Program Files\\Steam".into());
    }
    // ďalšie knižnice Steamu (iné disky)
    for r in roots.clone() {
        if let Ok(vdf) = std::fs::read_to_string(r.join("steamapps").join("libraryfolders.vdf")) {
            for l in vdf.lines() {
                let parts: Vec<&str> = l.split('"').collect();
                if parts.len() >= 4 && parts[1] == "path" {
                    roots.push(PathBuf::from(parts[3].replace("\\\\", "\\")));
                }
            }
        }
    }
    let mut seen = vec![];
    roots.retain(|r| {
        let k = r.to_string_lossy().to_lowercase();
        !seen.contains(&k) && {
            seen.push(k);
            true
        }
    });
    roots
}

fn wallpaper_engine() -> Option<Found> {
    for root in steam_roots() {
        let Some(cfg) = read_json(&root.join("steamapps").join("common").join("wallpaper_engine").join("config.json")) else { continue };
        // vybrané tapety sú cesty k project.json („selectedwallpapers“ pre každý monitor)
        let mut all = vec![];
        strings(&cfg, &mut all);
        for pj in all.iter().filter(|s| s.to_lowercase().ends_with("project.json")).map(PathBuf::from).filter(|p| p.is_file()) {
            let Some(info) = read_json(&pj) else { continue };
            let folder = pj.parent().unwrap_or(Path::new("")).to_path_buf();
            let file = info["file"].as_str().map(|f| folder.join(f)).filter(|p| p.is_file());
            let preview = info["preview"].as_str().map(|f| folder.join(f)).filter(|p| p.is_file() && is_image(p));
            if info["type"].as_str().map(|s| s.to_lowercase()) == Some("video".into()) {
                if let Some(f) = file.filter(|f| is_video(f)) {
                    return Some(Found { file: f, video: true, preview, source: "Wallpaper Engine" });
                }
            }
            // scéna / web / aplikácia → náhľad (často GIF)
            if let Some(p) = preview {
                return Some(Found { file: p, video: false, preview: None, source: "Wallpaper Engine" });
            }
        }
    }
    None
}

// beží program? keď je zavretý, na ploche je zas obyčajná tapeta Windows
fn running() -> (bool, bool) {
    if !cfg!(windows) {
        return (true, true); // test mimo Windows (FLUX_STEAM_PATH, LOCALAPPDATA)
    }
    match quiet("tasklist").args(["/FO", "CSV", "/NH"]).output() {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout).to_lowercase();
            (out.contains("\"lively.exe\"") || out.contains("\"lively.ui.winui.exe\""), out.contains("\"wallpaper32.exe\"") || out.contains("\"wallpaper64.exe\""))
        }
        Err(_) => (false, false),
    }
}

// volá sa na pozadí (spúšťa tasklist), najviac raz za 10 s
pub fn detect() -> Option<Found> {
    let (lv, we) = running();
    (if lv { lively() } else { None }).or_else(|| if we { wallpaper_engine() } else { None })
}
