// Témy ako súbory (ako témy terminálu kitty): jeden súbor `<id>.conf`, v každom riadku `kľúč hodnota`.
// Pribalené sú z priečinka `themes/` v repozitári (build.rs ich vloží do programu – nová téma = nový súbor),
// vlastné sa načítajú z `userData/themes/*.conf` a majú prednosť. Návod: docs/THEMES.md.
use crate::gen::THEME_KEYS;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

include!(concat!(env!("OUT_DIR"), "/themes_bundled.rs"));

// kľúče farieb aplikácie (nepovinné)
pub const UI_KEYS: [&str; 6] = ["background", "panel", "text", "text2", "border", "accent"];

#[derive(Clone, Debug)]
pub struct ThemeFile {
    pub id: String,
    pub name: String,
    pub dark: bool,
    pub code: [u32; 16],
    pub ui: HashMap<String, u32>,
    pub user: bool,
}

static LIST: Mutex<Option<Vec<ThemeFile>>> = Mutex::new(None);
// téma z obchodu na vyskúšanie (ešte nenainštalovaná) – správa sa ako nainštalovaná, kým sa náhľad nezruší
static PREVIEW: Mutex<Option<ThemeFile>> = Mutex::new(None);

pub fn set_preview(t: Option<ThemeFile>) {
    *PREVIEW.lock().unwrap() = t;
}

pub fn user_dir() -> PathBuf {
    flux_core::settings::user_data().join("themes")
}

fn color(v: &str) -> Option<u32> {
    let h = v.trim().trim_start_matches('#');
    if h.len() == 6 {
        u32::from_str_radix(h, 16).ok()
    } else if h.len() == 3 {
        let n = u32::from_str_radix(h, 16).ok()?;
        let (r, g, b) = ((n >> 8) & 15, (n >> 4) & 15, n & 15);
        Some((r * 17) << 16 | (g * 17) << 8 | (b * 17))
    } else {
        None
    }
}

// iné mená tých istých kľúčov (aby sa súbor písal prirodzene)
fn alias(k: &str) -> &str {
    match k {
        "foreground" | "code_text" => "fg",
        "punctuation" | "operator" => "delimiter",
        "attribute" => "attr",
        "regex" => "regexp",
        "class" => "type",
        "builtin" => "storage",
        "border_color" | "line" => "border",
        "secondary_text" => "text2",
        _ => k,
    }
}

// rozbor súboru: `kľúč hodnota` (oddelené medzerou, „=“ alebo „:“), komentáre začínajú „#“ na začiatku riadka
// alebo „ # “ za hodnotou; chýbajúce farby kódu sa doplnia z `based_on` (alebo predvolenej tmavej/svetlej témy)
pub fn parse(id: &str, src: &str) -> Option<ThemeFile> {
    let mut kv: HashMap<String, String> = HashMap::new();
    for line in src.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with("//") {
            continue;
        }
        let l = l.split(" # ").next().unwrap_or(l).trim();
        let (k, v) = match l.find(|c: char| c.is_whitespace() || c == '=' || c == ':') {
            Some(i) => (&l[..i], l[i + 1..].trim_start_matches(|c: char| c.is_whitespace() || c == '=' || c == ':').trim()),
            None => continue,
        };
        let k = alias(&k.to_lowercase()).to_string();
        // „type dark/light“ = druh témy (ako v pluginoch); „type #farba“ = farba typov v kóde
        let k = if (k == "type" || k == "variant") && matches!(v.to_lowercase().as_str(), "dark" | "light") { "mode".to_string() } else { k };
        kv.insert(k, v.to_string());
    }
    let dark = kv.get("mode").map(|t| t.to_lowercase() != "light").unwrap_or(true);
    let base = kv.get("based_on").and_then(|b| crate::gen::code_theme(b)).or_else(|| crate::gen::code_theme(if dark { crate::gen::DEFAULT_THEME } else { "vscode-light" }))?;
    let mut code = base.1;
    for (i, k) in THEME_KEYS.iter().enumerate() {
        if let Some(c) = kv.get(*k).and_then(|v| color(v)) {
            code[i] = c;
        }
    }
    let ui = UI_KEYS.iter().filter_map(|k| kv.get(*k).and_then(|v| color(v)).map(|c| (k.to_string(), c))).collect();
    let name = kv.get("name").cloned().unwrap_or_else(|| id.replace(['-', '_'], " "));
    Some(ThemeFile { id: id.to_string(), name, dark, code, ui, user: false })
}

// znova načíta pribalené a vlastné témy (vlastná s rovnakým id nahradí pribalenú)
pub fn reload() {
    let mut list: Vec<ThemeFile> = BUNDLED.iter().filter_map(|(id, src)| parse(id, src)).collect();
    if let Ok(rd) = std::fs::read_dir(user_dir()) {
        let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "conf")).collect();
        files.sort();
        for f in files {
            let id = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(mut t) = std::fs::read_to_string(&f).ok().and_then(|s| parse(&id, &s)) {
                t.user = true;
                list.retain(|x| x.id != id);
                list.push(t);
            }
        }
    }
    *LIST.lock().unwrap() = Some(list);
}

pub fn all() -> Vec<ThemeFile> {
    let mut g = LIST.lock().unwrap();
    if g.is_none() {
        drop(g);
        reload();
        g = LIST.lock().unwrap();
    }
    let mut l = g.clone().unwrap_or_default();
    drop(g);
    if let Some(p) = PREVIEW.lock().unwrap().clone() {
        if !l.iter().any(|x| x.id == p.id) {
            l.push(p);
        }
    }
    l
}

pub fn get(id: &str) -> Option<ThemeFile> {
    all().into_iter().find(|t| t.id == id)
}

pub fn code(id: &str) -> Option<(bool, [u32; 16], bool)> {
    get(id).map(|t| (t.dark, t.code, false))
}

// nová téma z existujúcej: súbor so všetkými kľúčmi a vysvetlivkami (otvorí sa v editore)
pub fn template(name: &str, dark: bool, code: &[u32; 16], ui: &[(String, u32)]) -> String {
    let mut s = String::new();
    s.push_str(&format!("# {name} – Flux theme\n# One line per value: key, spaces, then a color (#rrggbb). Lines with # are comments.\n# Save the file and Flux shows the change right away. Guide: docs/THEMES.md\n\n"));
    s.push_str(&format!("name        {name}\nmode        {}   # dark or light\n\n# code colors\n", if dark { "dark" } else { "light" }));
    let notes = [
        ("fg", "normal text"),
        ("comment", "comments"),
        ("keyword", "if, for, return, def…"),
        ("storage", "true, None, self, int…"),
        ("string", "\"text in quotes\""),
        ("number", "42, 3.14"),
        ("type", "class and type names"),
        ("function", "function names"),
        ("variable", "variables"),
        ("parameter", "function parameters"),
        ("property", "object.property"),
        ("constant", "CONSTANTS"),
        ("tag", "<div> HTML tags"),
        ("attr", "class=\"…\" attributes"),
        ("delimiter", "brackets, commas, operators"),
        ("regexp", "regular expressions"),
    ];
    for (i, (k, note)) in notes.iter().enumerate() {
        s.push_str(&format!("{:<11} #{:06x}   # {}\n", k, code[i], note));
    }
    s.push_str("\n# app colors (optional – delete a line to keep Flux's own color)\n");
    let ui_notes = [("background", "around the panels"), ("panel", "editor and sidebar"), ("text", "menus and file names"), ("text2", "hints and small text"), ("border", "lines between parts"), ("accent", "buttons and highlights")];
    for (k, note) in ui_notes {
        match ui.iter().find(|x| x.0 == k) {
            Some((_, c)) => s.push_str(&format!("{:<11} #{:06x}   # {}\n", k, c, note)),
            None => s.push_str(&format!("# {:<9} #000000   # {}\n", k, note)),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_parse_and_bundled() {
        let t = parse("x", "# c\nname My\ntype light\nkeyword #ff0000\nforeground = #111\naccent: #00ff00 # zelená\nbogus line\n").unwrap();
        assert_eq!(t.name, "My");
        assert!(!t.dark);
        assert_eq!(t.code[2], 0xff0000);
        assert_eq!(t.code[0], 0x111111);
        assert_eq!(t.ui.get("accent"), Some(&0x00ff00));
        // každá pribalená téma sa dá prečítať a má meno
        assert!(!BUNDLED.is_empty());
        for (id, src) in BUNDLED {
            let t = parse(id, src).unwrap_or_else(|| panic!("theme {id}"));
            assert!(!t.name.is_empty());
        }
    }
}

#[cfg(test)]
mod mode_tests {
    #[test]
    fn themes_light_mode() {
        let t = super::get("solarized-light").unwrap();
        assert!(!t.dark, "{:?}", t);
    }
}
