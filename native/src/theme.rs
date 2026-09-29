// Farby a písma Flux Native – rovnaká teplá tmavá (a svetlá) paleta ako Electron Flux (src/renderer/styles.css).
use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Visuals};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct Pal {
    pub dark: bool,
    pub base: Color32,  // pozadie okna (okolo kariet)
    pub card: Color32,  // editor, panely
    pub card2: Color32, // prvky na karte (tlačidlá, riadky pri prejdení)
    pub line: Color32,
    pub text: Color32,
    pub text2: Color32, // tlmený text
    pub accent: Color32,
    pub accent_fg: Color32,
    pub green: Color32,
    pub red: Color32,
}

pub fn palette(dark: bool) -> Pal {
    if dark {
        Pal {
            dark,
            base: Color32::from_rgb(0x1e, 0x1c, 0x19),
            card: Color32::from_rgb(0x13, 0x12, 0x0f),
            card2: Color32::from_rgb(0x26, 0x24, 0x20),
            line: Color32::from_rgb(0x2e, 0x2b, 0x27),
            text: Color32::from_rgb(0xe8, 0xe6, 0xe1),
            text2: Color32::from_rgb(0x9a, 0x96, 0x8e),
            accent: Color32::from_rgb(0xf2, 0xef, 0xe8),
            accent_fg: Color32::from_rgb(0x16, 0x15, 0x12),
            green: Color32::from_rgb(0x3e, 0xcf, 0x8e),
            red: Color32::from_rgb(0xff, 0x6b, 0x7a),
        }
    } else {
        Pal {
            dark,
            base: Color32::from_rgb(0xe7, 0xe5, 0xdf),
            card: Color32::from_rgb(0xf5, 0xf4, 0xf0),
            card2: Color32::from_rgb(0xe9, 0xe7, 0xe1),
            line: Color32::from_rgb(0xd8, 0xd5, 0xcd),
            text: Color32::from_rgb(0x1d, 0x1c, 0x1a),
            text2: Color32::from_rgb(0x6f, 0x6c, 0x66),
            accent: Color32::from_rgb(0x1d, 0x1c, 0x1a),
            accent_fg: Color32::from_rgb(0xf5, 0xf4, 0xf0),
            green: Color32::from_rgb(0x17, 0xa8, 0x6b),
            red: Color32::from_rgb(0xe0, 0x36, 0x4a),
        }
    }
}

pub fn apply(ctx: &egui::Context, p: &Pal) {
    let mut v = if p.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = p.base;
    v.window_fill = p.card;
    v.extreme_bg_color = p.card;
    v.faint_bg_color = p.card2;
    v.override_text_color = Some(p.text);
    v.window_stroke = Stroke::new(1.0, p.line);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.selection.bg_fill = p.accent.gamma_multiply(0.28);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = CornerRadius::same(8);
        w.fg_stroke.color = p.text;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_fill = p.card2;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.weak_bg_fill = p.card2;
    v.widgets.hovered.bg_fill = p.card2;
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.weak_bg_fill = p.line;
    v.widgets.active.bg_fill = p.line;
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.scroll.floating = true;
        s.spacing.scroll.bar_width = 8.0;
    });
}

// Písma Windows (Segoe UI, Cascadia Mono) ak sú; inak zostanú pribalené písma egui.
pub fn fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let try_add = |defs: &mut FontDefinitions, name: &str, files: &[&str], family: FontFamily| {
        for f in files {
            let p = std::path::Path::new(&win).join("Fonts").join(f);
            if let Ok(bytes) = std::fs::read(&p) {
                defs.font_data.insert(name.into(), Arc::new(FontData::from_owned(bytes)));
                defs.families.entry(family).or_default().insert(0, name.into());
                return;
            }
        }
    };
    try_add(&mut defs, "ui", &["segoeui.ttf"], FontFamily::Proportional);
    try_add(&mut defs, "code", &["CascadiaMono.ttf", "CascadiaCode.ttf", "consola.ttf"], FontFamily::Monospace);
    ctx.set_fonts(defs);
}
