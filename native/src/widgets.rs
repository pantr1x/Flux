// Prvky Flux Native kreslené tak, ako ich kreslí Electron Flux: ikony (tie isté SVG ako src/renderer/icons.js),
// riadky bočného panela, ikonové tlačidlá, nadpisy sekcií, text s „…“.
use crate::gen;
use crate::theme::{self, Pal};
use eframe::egui::{self, pos2, text::LayoutJob, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, TextFormat};
use std::path::Path;

// Známe dvojité prípony (app.min.js, x.d.ts…); iné ako „mod.wh.cpp“ dostanú všeobecnú ikonu – ako fileIcon() v icons.js.
const COMMON_MID: &[&str] = &[
    "min",
    "test",
    "spec",
    "d",
    "config",
    "module",
    "stories",
    "esm",
    "cjs",
    "umd",
    "prod",
    "dev",
    "local",
    "bundle",
    "page",
    "layout",
    "server",
    "client",
    "service",
    "component",
    "routes",
    "types",
    "schema",
    "setup",
    "story",
    "e2e",
];

fn file_key(name: &str) -> &'static str {
    let lower = name.trim_start_matches('.').to_lowercase();
    let parts: Vec<&str> = lower.split('.').collect();
    if parts.len() >= 3 {
        let mid = parts[parts.len() - 2];
        if (1..=3).contains(&mid.len()) && mid.chars().all(|c| c.is_ascii_lowercase()) && !COMMON_MID.contains(&mid) {
            return "file";
        }
    }
    if !name.contains('.') {
        return "file";
    }
    gen::ext_key(name.rsplit('.').next().unwrap_or(""))
}

// farebná ikona súboru podľa mena (alebo kľúča ako "python" s predponou "@")
pub fn file_icon(ui: &egui::Ui, rect: Rect, name: &str) {
    let key = name.strip_prefix('@').unwrap_or_else(|| file_key(name));
    let Some((svg, vb, labels)) = gen::file(key) else {
        return;
    };
    egui::Image::from_bytes(format!("bytes://fi/{key}.svg"), svg.as_bytes()).paint_at(ui, rect);
    let k = rect.width() / vb;
    for (x, y, t, c, size) in labels {
        let col = Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, *c as u8);
        let s = size * k;
        ui.painter().text(rect.min + vec2(x * k, y * k + s * 0.24), Align2::CENTER_BOTTOM, *t, theme::bold(s), col);
    }
}

// čiarová ikona (Lucide) v danej farbe
pub fn icon(ui: &egui::Ui, rect: Rect, name: &str, color: Color32) {
    if let Some(svg) = gen::line(name) {
        egui::Image::from_bytes(format!("bytes://li/{name}.svg"), svg.as_bytes()).tint(color).paint_at(ui, rect);
    }
}

pub fn icon_at(ui: &egui::Ui, center: egui::Pos2, size: f32, name: &str, color: Color32) {
    icon(ui, Rect::from_center_size(center, vec2(size, size)), name, color);
}

// Text v jednom riadku, pri nedostatku miesta skrátený na „…“.
pub fn text(ui: &egui::Ui, pos: egui::Pos2, align: Align2, s: &str, font: FontId, color: Color32, max_w: f32) -> Rect {
    let mut job = LayoutJob::single_section(s.to_string(), TextFormat { font_id: font, color, ..Default::default() });
    job.wrap.max_width = max_w.max(1.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('\u{2026}');
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let rect = align.anchor_size(pos, galley.size());
    ui.painter().galley(rect.min, galley, color);
    rect
}

pub fn text_w(ui: &egui::Ui, s: &str, font: FontId) -> f32 {
    ui.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font, Color32::WHITE).size().x)
}

// Nadpis sekcie v bočnom paneli („PROJECTS“): 11 px, tučné, s rozostupom písmen; vpravo voliteľné ikonové tlačidlo.
pub fn section(ui: &mut egui::Ui, title: &str, action: Option<&str>, p: &Pal) -> Option<Response> {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
    let mut job = LayoutJob::default();
    job.append(title, 0.0, TextFormat { font_id: theme::bold(11.0), color: p.text3, extra_letter_spacing: 1.0, ..Default::default() });
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0), galley, p.text3);
    action.map(|name| {
        let r = Rect::from_center_size(pos2(rect.right() - 11.0, rect.center().y), vec2(24.0, 24.0));
        icon_button_at(ui, r, name, 14.0, p, true)
    })
}

pub fn icon_button_at(ui: &mut egui::Ui, r: Rect, name: &str, size: f32, p: &Pal, enabled: bool) -> Response {
    let resp = ui.interact(r, ui.id().with(("ib", name, r.min.x as i32, r.min.y as i32)), if enabled { Sense::click() } else { Sense::hover() });
    if enabled && resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(8), p.hover);
    }
    let c = if !enabled {
        p.text3.gamma_multiply(0.55)
    } else if resp.hovered() {
        p.text
    } else {
        p.text2
    };
    icon_at(ui, r.center(), size, name, c);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

// ikonové tlačidlo v toku rozloženia (.icon-btn 28×28)
pub fn icon_button(ui: &mut egui::Ui, name: &str, p: &Pal, enabled: bool) -> Response {
    let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
    icon_button_at(ui, r, name, 16.0, p, enabled)
}

static DENSE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

// hustota „compact“ – nižšie riadky v bočnom paneli
pub fn set_dense(on: bool) {
    DENSE.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub enum Lead<'a> {
    File(&'a str),         // meno súboru → farebná ikona
    Folder { open: bool }, // šípka + priečinok
    Line(&'a str),         // čiarová ikona
}

// Riadok v bočnom paneli: .row (28 px) alebo dvojriadkový s podnadpisom (projekty, voľné súbory).
pub fn row(ui: &mut egui::Ui, selected: bool, indent: f32, lead: Lead, name: &str, sub: Option<&str>, bold: bool, p: &Pal) -> Response {
    let dense = DENSE.load(std::sync::atomic::Ordering::Relaxed);
    let h = match (sub.is_some(), dense) {
        (true, false) => 38.0,
        (true, true) => 34.0,
        (false, false) => 28.0,
        (false, true) => 24.0,
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    let hovered = resp.hovered();
    // plynulé zvýraznenie pri prejdení myšou
    let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), hovered, ui.style().animation_time);
    let bg = if selected { p.active } else { p.hover.gamma_multiply(hk) };
    ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
    let mut x = rect.left() + indent;
    let cy = rect.center().y;
    match lead {
        Lead::Folder { open } => {
            let c = pos2(x + 7.0, cy);
            let r = Rect::from_center_size(c, vec2(12.0, 12.0));
            if open {
                // šípka otočená o 90°
                let pts = [pos2(c.x - 3.5, c.y - 1.5), pos2(c.x, c.y + 2.0), pos2(c.x + 3.5, c.y - 1.5)];
                ui.painter().line(pts.to_vec(), Stroke::new(1.5, p.text3));
            } else {
                icon(ui, r, "chevron", p.text3);
            }
            x += 16.0;
            icon(ui, Rect::from_min_size(pos2(x, cy - 7.5), vec2(15.0, 15.0)), if open { "folderOpen" } else { "folder" }, p.text2);
            x += 21.0;
        }
        Lead::File(n) => {
            file_icon(ui, Rect::from_min_size(pos2(x, cy - 8.0), vec2(16.0, 16.0)), n);
            x += 23.0;
        }
        Lead::Line(n) => {
            icon(ui, Rect::from_min_size(pos2(x, cy - 8.0), vec2(16.0, 16.0)), n, p.text3);
            x += 23.0;
        }
    }
    let color = if selected || hovered || bold { p.text } else { p.text2 };
    let font = if bold { theme::bold(13.0) } else { theme::ui(13.0) };
    let max_w = rect.right() - x - 8.0;
    if let Some(s) = sub {
        text(ui, pos2(x, rect.top() + 12.0), Align2::LEFT_CENTER, name, font, color, max_w);
        text(ui, pos2(x, rect.top() + 27.0), Align2::LEFT_CENTER, s, theme::ui(11.0), p.text3, max_w);
    } else {
        text(ui, pos2(x, cy), Align2::LEFT_CENTER, name, font, color, max_w);
    }
    resp
}

pub fn separator(ui: &mut egui::Ui, p: &Pal) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
    ui.painter().hline(r.x_range(), r.center().y, Stroke::new(1.0, p.line));
}

// logo Fluxu: tmavý štvorček so zaoblením a ikonou „code“ (.brand-mark)
pub fn brand_mark(ui: &egui::Ui, r: Rect, p: &Pal) {
    ui.painter().rect_filled(r, CornerRadius::same(6), Color32::from_rgb(0x11, 0x11, 0x14));
    ui.painter().rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0, Color32::from_white_alpha(36)), StrokeKind::Inside);
    icon_at(ui, r.center(), r.width() * 0.65, "code", Color32::WHITE);
    let _ = p;
}

// Tlačidlo s ikonou a textom (akcie na stránke projektu, AI v hornej lište).
pub fn button(ui: &mut egui::Ui, lead: Option<&str>, label: &str, fill: Color32, fg: Color32, h: f32, p: &Pal) -> Response {
    let font = theme::bold(13.0);
    let tw = text_w(ui, label, font.clone());
    let w = tw + if lead.is_some() { 46.0 } else { 26.0 };
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let hovered = resp.hovered();
    let bg = if hovered {
        if fill == Color32::TRANSPARENT {
            p.hover
        } else {
            fill.gamma_multiply(1.12)
        }
    } else {
        fill
    };
    ui.painter().rect_filled(r, CornerRadius::same(10), bg);
    if fill == Color32::TRANSPARENT || fill == p.hover || fill == p.card2 {
        ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
    }
    let mut x = r.left() + 13.0;
    if let Some(i) = lead {
        icon_at(ui, pos2(x + 6.0, r.center().y), 14.0, i, fg);
        x += 20.0;
    }
    ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, label, font, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn file_name(path: &str) -> String {
    Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string())
}
