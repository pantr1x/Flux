// Malé prvky Flux Native kreslené priamo (bez písma s ikonami): riadok v bočnom paneli,
// ikony priečinkov a súborov (farebný štítok podľa jazyka), bodka stavu, logo.
use crate::theme::Pal;
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind};

pub enum Icon<'a> {
    Folder(bool),
    File(&'a str),
    Project(bool),
}

// farba a skratka jazyka podľa prípony (ako ikony súborov vo Fluxe)
fn badge(ext: &str) -> (Color32, &'static str) {
    match ext {
        "py" | "pyw" => (Color32::from_rgb(0x3b, 0x77, 0xa8), "Py"),
        "js" | "mjs" | "cjs" => (Color32::from_rgb(0xd9, 0xb8, 0x2a), "JS"),
        "ts" => (Color32::from_rgb(0x31, 0x78, 0xc6), "TS"),
        "html" | "htm" => (Color32::from_rgb(0xe3, 0x4f, 0x26), "<>"),
        "css" | "scss" => (Color32::from_rgb(0x29, 0x65, 0xf1), "#"),
        "json" => (Color32::from_rgb(0x8a, 0x8a, 0x8a), "{}"),
        "md" => (Color32::from_rgb(0x51, 0x9a, 0xba), "M"),
        "c" | "h" => (Color32::from_rgb(0x55, 0x55, 0xaa), "C"),
        "cpp" | "cc" | "cxx" | "hpp" => (Color32::from_rgb(0x00, 0x59, 0x9c), "C+"),
        "rs" => (Color32::from_rgb(0xb7, 0x41, 0x0e), "Rs"),
        "java" => (Color32::from_rgb(0xe7, 0x6f, 0x00), "J"),
        "go" => (Color32::from_rgb(0x00, 0xad, 0xd8), "Go"),
        "cs" => (Color32::from_rgb(0x68, 0x21, 0x7a), "C#"),
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => (Color32::from_rgb(0x3e, 0xa3, 0x6b), "Im"),
        _ => (Color32::from_rgb(0x70, 0x6c, 0x66), ""),
    }
}

fn paint_icon(ui: &egui::Ui, icon: &Icon, at: Rect, p: &Pal) {
    let painter = ui.painter();
    let c = at.center();
    match icon {
        Icon::Folder(open) => {
            // šípka + priečinok
            let a = pos2(at.left() - 11.0, c.y);
            let pts = if *open { vec![pos2(a.x - 3.5, a.y - 2.0), pos2(a.x + 3.5, a.y - 2.0), pos2(a.x, a.y + 2.5)] } else { vec![pos2(a.x - 2.0, a.y - 3.5), pos2(a.x - 2.0, a.y + 3.5), pos2(a.x + 2.5, a.y)] };
            painter.add(egui::Shape::convex_polygon(pts, p.text2, Stroke::NONE));
            let body = Rect::from_center_size(c + vec2(0.0, 1.0), vec2(14.0, 10.0));
            painter.rect_stroke(body, CornerRadius::same(2), Stroke::new(1.3, p.text2), StrokeKind::Inside);
            painter.rect_filled(Rect::from_min_size(body.min - vec2(0.0, 2.0), vec2(6.0, 3.0)), CornerRadius::same(1), p.text2);
        }
        Icon::File(ext) => {
            let (col, txt) = badge(ext);
            let r = Rect::from_center_size(c, vec2(16.0, 16.0));
            if txt.is_empty() {
                painter.rect_stroke(r.shrink2(vec2(2.5, 1.0)), CornerRadius::same(2), Stroke::new(1.2, p.text2), StrokeKind::Inside);
            } else {
                painter.rect_filled(r, CornerRadius::same(4), col);
                painter.text(c, egui::Align2::CENTER_CENTER, txt, FontId::proportional(8.5), Color32::WHITE);
            }
        }
        Icon::Project(pinned) => {
            painter.rect_filled(Rect::from_center_size(c, vec2(16.0, 16.0)), CornerRadius::same(4), p.card2);
            painter.rect_stroke(Rect::from_center_size(c, vec2(16.0, 16.0)), CornerRadius::same(4), Stroke::new(1.0, p.line), StrokeKind::Inside);
            painter.circle_filled(c, 3.0, if *pinned { p.accent } else { p.text2 });
        }
    }
}

// Riadok v bočnom paneli (zarovnaný vľavo, zvýraznenie pri prejdení a pri výbere).
pub fn row(ui: &mut egui::Ui, selected: bool, indent: f32, icon: Icon, text: &str, sub: Option<&str>, p: &Pal) -> Response {
    let h = if sub.is_some() { 38.0 } else { 26.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    let bg = if selected { p.card2 } else if resp.hovered() { p.card2.gamma_multiply(0.6) } else { Color32::TRANSPARENT };
    ui.painter().rect_filled(rect, CornerRadius::same(7), bg);
    let has_icon = true;
    let icon_rect = Rect::from_min_size(pos2(rect.left() + indent + 16.0, rect.top() + (h - 16.0) / 2.0), vec2(16.0, 16.0));
    if has_icon {
        paint_icon(ui, &icon, icon_rect, p);
    }
    let x = if has_icon { icon_rect.right() + 8.0 } else { rect.left() + indent + 10.0 };
    let color = if selected { p.text } else { p.text.gamma_multiply(0.88) };
    if let Some(s) = sub {
        ui.painter().text(pos2(x, rect.top() + 11.0), egui::Align2::LEFT_CENTER, text, FontId::proportional(13.5), color);
        ui.painter().text(pos2(x, rect.top() + 27.0), egui::Align2::LEFT_CENTER, s, FontId::proportional(11.0), p.text2);
    } else {
        ui.painter().text(pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, text, FontId::proportional(13.5), color);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn dot(ui: &mut egui::Ui, color: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(r.center(), 3.5, color);
}

pub fn logo(ui: &mut egui::Ui, p: &Pal) {
    let (r, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::same(7), if p.dark { Color32::from_rgb(0x0e, 0x0d, 0x0b) } else { p.text });
    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "</>", FontId::monospace(10.0), if p.dark { p.text } else { p.card });
}
