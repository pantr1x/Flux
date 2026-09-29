// Farby a písma Flux Native – tie isté hodnoty ako Electron Flux (src/renderer/styles.css, body.theme-dark/light + no-mica).
use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, Visuals};
use std::sync::Arc;

// rozmery z :root v styles.css
pub const TOP_H: f32 = 44.0;
pub const SIDE_W: f32 = 248.0;
pub const GAP: f32 = 8.0;

#[derive(Clone, Copy)]
pub struct Pal {
    pub dark: bool,
    pub base: Color32,        // --base: okolie karty
    pub card: Color32,        // --card: editor a karta
    pub solid: Color32,       // --card-solid: okná nad všetkým (nastavenia, hľadanie)
    pub card2: Color32,       // --card-2: jemne odlíšené plochy na karte
    pub line: Color32,        // --line
    pub line_strong: Color32, // --line-strong
    pub text: Color32,
    pub text2: Color32,
    pub text3: Color32,
    pub hover: Color32,
    pub active: Color32,
    pub accent: Color32,
    pub accent_fg: Color32,
    pub green: Color32,
    pub red: Color32,
}

// rgba(r, g, b, a) z CSS
fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, (a * 255.0).round() as u8)
}

// ACCENTS z app.js (mono = biela v tmavej, čierna vo svetlej téme)
pub const ACCENTS: [(&str, u32); 13] = [
    ("mono", 0),
    ("violet", 0x8b7bff),
    ("indigo", 0x6366f1),
    ("blue", 0x4f9dff),
    ("sky", 0x38bdf8),
    ("teal", 0x2ec4b6),
    ("green", 0x3ecf8e),
    ("lime", 0xa3d635),
    ("yellow", 0xf5c542),
    ("orange", 0xff9f5a),
    ("red", 0xff5f6d),
    ("pink", 0xff6fb1),
    ("gray", 0x9ca3af),
];

pub fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

// Paleta podľa nastavení: téma kódu (tmavá/svetlá), accent (meno alebo #hex) a darkLift (0–100).
pub fn palette_for(dark: bool, accent: &str, lift: f64) -> Pal {
    let mut p = base_palette(dark);
    let a = ACCENTS.iter().find(|(n, _)| *n == accent).map(|(_, c)| *c).or_else(|| u32::from_str_radix(accent.trim_start_matches('#'), 16).ok().filter(|_| accent.starts_with('#')));
    if let Some(c) = a.filter(|c| *c != 0) {
        p.accent = hex(c);
        // readableOn() z app.js
        let (r, g, b) = ((c >> 16) & 255, (c >> 8) & 255, c & 255);
        p.accent_fg = if 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64 > 160.0 { hex(0x16161a) } else { Color32::WHITE };
    }
    // --dark-lift: biela vrstva cez tmavé plochy (0.0038 na jednotku)
    let l = (lift.clamp(0.0, 100.0) * 0.0038) as f32;
    if dark && l > 0.0 {
        p.base = p.base.lerp_to_gamma(Color32::WHITE, l);
        p.card = p.card.lerp_to_gamma(Color32::WHITE, l);
    }
    p
}

fn base_palette(dark: bool) -> Pal {
    if dark {
        Pal {
            dark,
            base: Color32::from_rgb(0x1e, 0x1c, 0x19),
            card: Color32::from_rgb(0x0e, 0x0d, 0x0b),
            solid: Color32::from_rgb(0x13, 0x12, 0x0f),
            card2: rgba(255, 245, 225, 0.04),
            line: rgba(255, 245, 225, 0.075),
            line_strong: rgba(255, 245, 225, 0.12),
            text: Color32::from_rgb(0xec, 0xeb, 0xe6),
            text2: Color32::from_rgb(0xa7, 0xa3, 0x9a),
            text3: Color32::from_rgb(0x82, 0x7e, 0x75),
            hover: rgba(255, 245, 225, 0.06),
            active: rgba(255, 245, 225, 0.1),
            accent: Color32::from_rgb(0xec, 0xeb, 0xe6),
            accent_fg: Color32::from_rgb(0x16, 0x15, 0x12),
            green: Color32::from_rgb(0x3e, 0xcf, 0x8e),
            red: Color32::from_rgb(0xff, 0x6b, 0x7a),
        }
    } else {
        Pal {
            dark,
            base: Color32::from_rgb(0xe7, 0xe5, 0xdf),
            card: Color32::from_rgb(0xf7, 0xf6, 0xf2),
            solid: Color32::from_rgb(0xf5, 0xf4, 0xf0),
            card2: rgba(40, 30, 10, 0.035),
            line: rgba(40, 30, 10, 0.08),
            line_strong: rgba(40, 30, 10, 0.13),
            text: Color32::from_rgb(0x1c, 0x1b, 0x18),
            text2: Color32::from_rgb(0x55, 0x52, 0x4b),
            text3: Color32::from_rgb(0x76, 0x72, 0x6a),
            hover: rgba(40, 30, 10, 0.05),
            active: rgba(40, 30, 10, 0.085),
            accent: Color32::from_rgb(0x1c, 0x1b, 0x18),
            accent_fg: Color32::from_rgb(0xf5, 0xf4, 0xf0),
            green: Color32::from_rgb(0x17, 0xa8, 0x6b),
            red: Color32::from_rgb(0xe0, 0x36, 0x4a),
        }
    }
}

pub fn apply(ctx: &egui::Context, p: &Pal) {
    let mut v = if p.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = p.base;
    v.window_fill = p.solid;
    v.extreme_bg_color = p.card;
    v.faint_bg_color = p.card2;
    v.override_text_color = Some(p.text);
    v.window_stroke = Stroke::new(1.0, p.line_strong);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.selection.bg_fill = if p.dark { rgba(120, 150, 255, 0.28) } else { rgba(60, 100, 230, 0.22) };
    v.selection.stroke = Stroke::new(1.0, p.text);
    v.hyperlink_color = p.text;
    v.text_cursor.stroke = Stroke::new(2.0, p.text);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = CornerRadius::same(8);
        w.fg_stroke.color = p.text;
        w.bg_stroke = Stroke::NONE;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_fill = p.hover;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.active.weak_bg_fill = p.active;
    v.widgets.active.bg_fill = p.active;
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.scroll.floating = true;
        s.spacing.scroll.bar_width = 8.0;
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(13.0));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(13.0));
    });
}

// tučné písmo (Segoe UI Semibold) – egui má na rodinu len jednu hrúbku, tak je to vlastná rodina
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn ui(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

// Písma Windows (Segoe UI, Cascadia Mono); na Linuxe DejaVu; inak zostanú pribalené písma egui (aj ako záloha znakov).
pub fn fonts(ctx: &egui::Context, code_font: &str) {
    let mut defs = FontDefinitions::default();
    let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let fonts_dir = std::path::Path::new(&win).join("Fonts");
    let linux = std::path::Path::new("/usr/share/fonts/truetype/dejavu");
    let read = |files: &[&str]| -> Option<Vec<u8>> { files.iter().find_map(|f| std::fs::read(fonts_dir.join(f)).ok().or_else(|| std::fs::read(linux.join(f)).ok())) };
    let fallback: Vec<String> = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let add = |defs: &mut FontDefinitions, name: &str, files: &[&str], family: FontFamily| {
        if let Some(bytes) = read(files) {
            defs.font_data.insert(name.into(), Arc::new(FontData::from_owned(bytes)));
            defs.families.entry(family).or_default().insert(0, name.into());
        }
    };
    add(&mut defs, "ui", &["segoeui.ttf", "DejaVuSans.ttf"], FontFamily::Proportional);
    // písmo kódu podľa nastavenia fontFamily (FONTS v app.js), s náhradou
    let mut files: Vec<&str> = match code_font {
        "Cascadia Code" => vec!["CascadiaCode.ttf"],
        "Cascadia Mono" => vec!["CascadiaMono.ttf"],
        "JetBrains Mono" => vec!["JetBrainsMono-Regular.ttf"],
        "Fira Code" => vec!["FiraCode-Regular.ttf"],
        "Courier New" => vec!["cour.ttf"],
        _ => vec!["consola.ttf"],
    };
    files.extend(["CascadiaMono.ttf", "consola.ttf", "DejaVuSansMono.ttf"]);
    // písma nainštalované len pre používateľa
    let user = std::env::var("LOCALAPPDATA").map(|l| std::path::Path::new(&l).join("Microsoft\\Windows\\Fonts")).ok();
    let mut added = false;
    if let Some(u) = &user {
        if let Some(b) = std::fs::read(u.join(files[0])).ok() {
            defs.font_data.insert("code".into(), Arc::new(FontData::from_owned(b)));
            defs.families.entry(FontFamily::Monospace).or_default().insert(0, "code".into());
            added = true;
        }
    }
    if !added {
        add(&mut defs, "code", &files, FontFamily::Monospace);
    }
    let bold = FontFamily::Name("bold".into());
    defs.families.insert(bold.clone(), fallback);
    add(&mut defs, "ui-bold", &["seguisb.ttf", "segoeuib.ttf", "DejaVuSans-Bold.ttf"], bold);
    ctx.set_fonts(defs);
}
