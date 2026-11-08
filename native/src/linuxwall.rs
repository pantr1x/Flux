// Tapeta plochy na Linuxe (port src/main/wallpaper.js): každé prostredie ju ukladá inde – KDE Plasma,
// GNOME/Cinnamon/MATE, XFCE, Hyprland (swww, hyprpaper), feh, nitrogen. Výsledok sa pamätá 10 s (nič = 60 s),
// lebo Flux sa pýta každé 3 s z hlavného vlákna.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static CACHE: Mutex<Option<(Instant, Option<PathBuf>)>> = Mutex::new(None);

fn is_img(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp" | "bmp"))
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

fn run(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

// %20 → medzera (file:// adresy)
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

// file:///… alebo cesta (aj ~/…) → existujúci obrázok; priečinok = balík tapiet KDE (contents/images/WxH.jpg)
pub fn to_image(value: &str) -> Option<PathBuf> {
    let v = value.trim().trim_matches(|c| c == '\'' || c == '"');
    if v.is_empty() {
        return None;
    }
    let v = match v.strip_prefix("file://") {
        Some(rest) => unescape(rest),
        None => v.to_string(),
    };
    let p = match v.strip_prefix("~/") {
        Some(rest) => home().join(rest),
        None => PathBuf::from(v),
    };
    if p.is_file() {
        return is_img(&p).then_some(p);
    }
    if p.is_dir() {
        return from_package(&p);
    }
    None
}

// balík tapiet: najväčší obrázok v contents/images (názvy ako 5120x2880.png)
fn from_package(dir: &Path) -> Option<PathBuf> {
    for sub in ["contents/images", "contents/images_dark", "."] {
        let Ok(rd) = std::fs::read_dir(dir.join(sub)) else { continue };
        let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && is_img(p)).collect();
        if files.is_empty() {
            continue;
        }
        let px = |p: &PathBuf| {
            let n = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            n.split_once('x').and_then(|(a, b)| Some(a.parse::<u64>().ok()? * b.parse::<u64>().ok()?)).unwrap_or(0)
        };
        files.sort_by_key(|f| std::cmp::Reverse(px(f)));
        return files.into_iter().next();
    }
    None
}

// KDE Plasma: prvá plocha s obrázkom – [Containments][N][Wallpaper][org.kde.image][General] → Image=
pub fn kde_from(cfg: &str) -> Option<PathBuf> {
    let mut in_image = false;
    for line in cfg.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_image = l.ends_with("[Wallpaper][org.kde.image][General]");
        } else if in_image {
            if let Some(v) = l.strip_prefix("Image=") {
                if let Some(img) = to_image(v) {
                    return Some(img);
                }
            }
        }
    }
    None
}

fn kde() -> Option<PathBuf> {
    let cfg = std::fs::read_to_string(home().join(".config/plasma-org.kde.plasma.desktop-appletsrc")).unwrap_or_default();
    kde_from(&cfg).or_else(|| desktop().contains("kde").then(|| from_package(Path::new("/usr/share/wallpapers/Next"))).flatten())
}

fn gnome() -> Option<PathBuf> {
    let dark = run("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"]).contains("dark");
    for (schema, key) in [
        ("org.gnome.desktop.background", if dark { "picture-uri-dark" } else { "picture-uri" }),
        ("org.gnome.desktop.background", "picture-uri"),
        ("org.cinnamon.desktop.background", "picture-uri"),
        ("org.mate.background", "picture-filename"),
    ] {
        if let Some(img) = to_image(&run("gsettings", &["get", schema, key])) {
            return Some(img);
        }
    }
    None
}

fn xfce() -> Option<PathBuf> {
    let list = run("xfconf-query", &["-c", "xfce4-desktop", "-l"]);
    let prop = list.lines().find(|l| l.ends_with("/last-image"))?;
    to_image(&run("xfconf-query", &["-c", "xfce4-desktop", "-p", prop]))
}

fn swww() -> Option<PathBuf> {
    let out = run("swww", &["query"]);
    out.lines().find_map(|l| l.split_once("image: ").and_then(|(_, p)| to_image(p)))
}

fn hyprpaper() -> Option<PathBuf> {
    let cfg = std::fs::read_to_string(home().join(".config/hypr/hyprpaper.conf")).unwrap_or_default();
    for l in cfg.lines() {
        let l = l.trim();
        let v = if let Some(r) = l.strip_prefix("wallpaper") {
            r.trim_start().strip_prefix('=').and_then(|r| r.split_once(',')).map(|(_, p)| p)
        } else if let Some(r) = l.strip_prefix("path") {
            r.trim_start().strip_prefix('=')
        } else {
            None
        };
        if let Some(img) = v.and_then(to_image) {
            return Some(img);
        }
    }
    None
}

fn feh() -> Option<PathBuf> {
    let s = std::fs::read_to_string(home().join(".fehbg")).unwrap_or_default();
    s.lines().rev().find_map(|l| {
        let l = l.trim_end();
        let end = l.strip_suffix('\'')?;
        let start = end.rfind('\'')?;
        to_image(&end[start + 1..])
    })
}

fn nitrogen() -> Option<PathBuf> {
    let s = std::fs::read_to_string(home().join(".config/nitrogen/bg-saved.cfg")).unwrap_or_default();
    s.lines().find_map(|l| l.strip_prefix("file=").and_then(to_image))
}

fn desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase()
}

pub fn find() -> Option<PathBuf> {
    if let Some((at, p)) = CACHE.lock().unwrap().as_ref() {
        if at.elapsed() < Duration::from_secs(if p.is_some() { 10 } else { 60 }) {
            return p.clone();
        }
    }
    let d = desktop();
    // najprv zdroj pre aktuálne prostredie, potom ostatné
    let first = if d.contains("kde") {
        "kde"
    } else if ["gnome", "cinnamon", "mate", "budgie", "unity"].iter().any(|x| d.contains(x)) {
        "gnome"
    } else if d.contains("xfce") {
        "xfce"
    } else if d.contains("hyprland") {
        "swww"
    } else {
        ""
    };
    let all: [(&str, fn() -> Option<PathBuf>); 7] = [("kde", kde), ("gnome", gnome), ("xfce", xfce), ("swww", swww), ("hyprpaper", hyprpaper), ("feh", feh), ("nitrogen", nitrogen)];
    let found = all.iter().filter(|(k, _)| *k == first).chain(all.iter().filter(|(k, _)| *k != first)).find_map(|(_, f)| f());
    *CACHE.lock().unwrap() = Some((Instant::now(), found.clone()));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linuxwall_kde_and_paths() {
        let dir = std::env::temp_dir().join(format!("flux-lw-{}", std::process::id()));
        let pkg = dir.join("My Pack/contents/images");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(pkg.join("1920x1080.jpg"), b"x").unwrap();
        std::fs::write(pkg.join("3840x2160.jpg"), b"x").unwrap();
        let img = dir.join("a b.png");
        std::fs::write(&img, b"x").unwrap();
        let cfg = format!(
            "[Containments][1][General]\nImage=nope\n[Containments][1][Wallpaper][org.kde.image][General]\nImage=file://{}\n",
            dir.join("a%20b.png").to_string_lossy()
        );
        assert_eq!(kde_from(&cfg), Some(img.clone()));
        let cfg2 = format!("[Containments][7][Wallpaper][org.kde.image][General]\nImage={}\n", dir.join("My Pack").to_string_lossy());
        assert_eq!(kde_from(&cfg2), Some(pkg.join("3840x2160.jpg")));
        assert_eq!(to_image("'/no/such.png'"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
