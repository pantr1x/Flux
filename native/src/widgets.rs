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
    // aj vypnuté tlačidlo zachytí klik – inak by prepadol do lišty okna (dvojklik = maximalizovať)
    let mut resp = ui.interact(r, ui.id().with(("ib", name, r.min.x as i32, r.min.y as i32)), Sense::click());
    if !enabled {
        resp.flags.remove(egui::response::Flags::CLICKED);
    }
    // plynulé zvýraznenie pri prejdení myšou
    let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), enabled && resp.hovered(), ui.style().animation_time);
    if hk > 0.0 {
        ui.painter().rect_filled(r, CornerRadius::same(8), p.hover.gamma_multiply(hk));
    }
    let c = if !enabled { p.text3.gamma_multiply(0.55) } else { p.text2.lerp_to_gamma(p.text, hk) };
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
    Project(&'a str),      // ikona projektu: „file:a.py“, „line:rocket“, „emoji:🤖“
}

// Riadok v bočnom paneli: .row (28 px) alebo dvojriadkový s podnadpisom (projekty, voľné súbory).
pub fn row(ui: &mut egui::Ui, selected: bool, indent: f32, lead: Lead, name: &str, sub: Option<&str>, bold: bool, p: &Pal) -> Response {
    row_ex(ui, selected, true, indent, lead, name, sub, bold, p)
}

// fill = false: vybraný riadok bez vlastnej výplne (kreslí ju kĺzavé zvýraznenie, napr. strom súborov)
#[allow(clippy::too_many_arguments)]
pub fn row_ex(ui: &mut egui::Ui, selected: bool, fill: bool, indent: f32, lead: Lead, name: &str, sub: Option<&str>, bold: bool, p: &Pal) -> Response {
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
    let bg = if selected && fill {
        p.active
    } else if selected {
        Color32::TRANSPARENT
    } else {
        p.hover.gamma_multiply(hk)
    };
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
        Lead::Project(spec) => {
            let r = Rect::from_min_size(pos2(x, cy - 8.0), vec2(16.0, 16.0));
            if let Some(n) = spec.strip_prefix("line:") {
                icon(ui, r, n, p.text2);
            } else if let Some(e) = spec.strip_prefix("emoji:") {
                ui.painter().text(r.center(), Align2::CENTER_CENTER, e, egui::FontId::proportional(14.0), p.text);
            } else if let Some(f) = spec.strip_prefix("file:") {
                file_icon(ui, r, f);
            } else {
                icon(ui, r, "folder", p.text3);
            }
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
    let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), ui.style().animation_time);
    let hover_fill = if fill == Color32::TRANSPARENT || fill == p.card2 { p.active } else { fill.lerp_to_gamma(Color32::WHITE, 0.1) };
    let bg = fill.lerp_to_gamma(hover_fill, hk);
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

// Rozbaľovací zoznam ako .s-row select (rámik, šípka dole, zoznam pod ním). Vráti vybraný index.
pub fn select(ui: &mut egui::Ui, id: egui::Id, current: &str, options: &[String], width: f32, p: &Pal) -> Option<usize> {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    let hk = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered(), ui.style().animation_time);
    ui.painter().rect_filled(r, CornerRadius::same(9), p.card2.lerp_to_gamma(p.hover, hk));
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
    ui.painter().rect_stroke(r, CornerRadius::same(9), Stroke::new(if open { 2.0 } else { 1.0 }, if open { p.accent } else { p.line_strong }), StrokeKind::Inside);
    text(ui, pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, current, theme::ui(13.0), p.text, width - 40.0);
    let c = pos2(r.right() - 16.0, r.center().y);
    let pts = vec![pos2(c.x - 4.0, c.y - 2.0), pos2(c.x, c.y + 2.0), pos2(c.x + 4.0, c.y - 2.0)];
    ui.painter().line(pts, Stroke::new(1.6, p.text2));
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let mut pick = None;
    egui::Popup::menu(&resp).width(width).show(|ui| {
        ui.set_min_width(width - 12.0);
        for (i, o) in options.iter().enumerate() {
            let (rr, rs) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
            let on = o == current;
            if rs.hovered() || on {
                ui.painter().rect_filled(rr, CornerRadius::same(6), if on { p.active } else { p.hover });
            }
            if on {
                icon_at(ui, pos2(rr.right() - 14.0, rr.center().y), 12.0, "check", p.text);
            }
            text(ui, pos2(rr.left() + 10.0, rr.center().y), Align2::LEFT_CENTER, o, theme::ui(13.0), p.text, rr.width() - 34.0);
            if rs.clicked() {
                pick = Some(i);
            }
        }
    });
    pick
}

// Posuvník ako input[type=range] vo Fluxe: tenká dráha, vyplnená časť vo farbe zvýraznenia, koliesko.
// Vráti (zmenené počas ťahania, pustené).
pub fn slider(ui: &mut egui::Ui, id: egui::Id, v: &mut f64, min: f64, max: f64, step: f64, width: f32, p: &Pal) -> (bool, bool) {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 24.0), Sense::click_and_drag());
    let track = Rect::from_min_max(pos2(r.left() + 8.0, r.center().y - 2.0), pos2(r.right() - 8.0, r.center().y + 2.0));
    let mut changed = false;
    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() || resp.drag_started() {
            let k = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0) as f64;
            let nv = ((min + k * (max - min)) / step).round() * step;
            if (nv - *v).abs() > f64::EPSILON {
                *v = nv.clamp(min, max);
                changed = true;
            }
        }
    }
    let k = ((*v - min) / (max - min)).clamp(0.0, 1.0) as f32;
    let x = egui::lerp(track.left()..=track.right(), k);
    ui.painter().rect_filled(track, CornerRadius::same(2), p.active);
    ui.painter().rect_filled(Rect::from_min_max(track.min, pos2(x, track.bottom())), CornerRadius::same(2), p.accent);
    let hk = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered() || resp.dragged(), ui.style().animation_time);
    ui.painter().circle_filled(pos2(x, r.center().y), 7.0 + hk * 1.5, p.accent);
    ui.painter().circle_stroke(pos2(x, r.center().y), 7.0 + hk * 1.5, Stroke::new(2.0, p.card));
    (changed, resp.drag_stopped() || resp.clicked())
}

// farebné pole ako v Electron Fluxe (.color-field): zaoblený box s okrúhlou nepriehľadnou vzorkou
// a hex kódom; klik otvorí výber farby s hex políčkom. `dot` = len okrúhla vzorka (vlastný accent).
pub fn color_field(ui: &mut egui::Ui, r: Rect, id: &str, cur: Color32, dot: bool, p: &Pal) -> Option<Color32> {
    let cur = cur.to_opaque();
    let resp = ui.interact(r, ui.id().with(("cf", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
    let hex = format!("#{:02X}{:02X}{:02X}", cur.r(), cur.g(), cur.b());
    if dot {
        ui.painter().circle_filled(r.center(), 11.0 + hk * 1.5, cur);
        ui.painter().circle_stroke(r.center(), 11.0 + hk * 1.5, Stroke::new(1.0, p.line_strong));
        icon_at(ui, r.center(), 11.0, "plus", if cur.intensity() > 0.6 { Color32::from_rgb(0x16, 0x16, 0x1a) } else { Color32::WHITE });
    } else {
        ui.painter().rect_filled(r, CornerRadius::same(9), p.card2.lerp_to_gamma(p.hover, hk));
        ui.painter().rect_stroke(r, CornerRadius::same(9), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let c = pos2(r.left() + 16.0, r.center().y);
        ui.painter().circle_filled(c, 8.0, cur);
        ui.painter().circle_stroke(c, 8.0, Stroke::new(1.0, p.line_strong));
        ui.painter().text(pos2(r.left() + 31.0, r.center().y), Align2::LEFT_CENTER, &hex, crate::theme::mono(12.0), p.text);
    }
    let mut out = None;
    let hid = resp.id.with("hex");
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        ui.spacing_mut().slider_width = 200.0;
        let mut c = cur;
        if egui::color_picker::color_picker_color32(ui, &mut c, egui::color_picker::Alpha::Opaque) && c != cur {
            out = Some(c);
        }
        ui.add_space(4.0);
        let mut text = ui.data(|d| d.get_temp::<String>(hid)).unwrap_or_else(|| hex.clone());
        let te = ui.add(egui::TextEdit::singleline(&mut text).font(crate::theme::mono(12.5)).desired_width(200.0).margin(egui::Margin::symmetric(8, 5)));
        if te.changed() {
            let h = text.trim().trim_start_matches('#');
            if h.len() == 6 {
                if let Ok(v) = u32::from_str_radix(h, 16) {
                    out = Some(crate::theme::hex(v));
                }
            }
        }
        if te.has_focus() {
            ui.data_mut(|d| d.insert_temp(hid, text));
        } else {
            ui.data_mut(|d| d.remove::<String>(hid));
        }
    });
    out
}
