// Tmavý dashboard dosky na stránke projektu: farby z dizajnu, ikony predvolieb, editor kódu so zvýraznením a číslami riadkov.
use crate::theme::{self, Pal};
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, Pos2, Sense, Stroke, StrokeKind};

pub const ORANGE: Color32 = Color32::from_rgb(0xff, 0x8a, 0x3d);

// farby sa berú z témy Flux (nastaví ich `set()` na začiatku vykreslenia karty)
thread_local! {
    static CUR: std::cell::Cell<Option<Pal>> = const { std::cell::Cell::new(None) };
}
pub fn set(p: &Pal) {
    CUR.with(|c| c.set(Some(*p)));
}
fn cur() -> Pal {
    CUR.with(|c| c.get()).unwrap_or_else(|| crate::theme::palette_for(true, "mono", 0.0))
}
pub fn bg() -> Color32 {
    cur().card
}
pub fn panel_fill() -> Color32 {
    cur().card2
}
pub fn field() -> Color32 {
    cur().card
}
pub fn border() -> Color32 {
    cur().line
}
pub fn border2() -> Color32 {
    cur().line_strong
}
pub fn hover() -> Color32 {
    cur().hover
}
pub fn cyan() -> Color32 {
    cur().accent
}
pub fn cyan_fg() -> Color32 {
    cur().accent_fg
}
pub fn text() -> Color32 {
    cur().text
}
pub fn text2() -> Color32 {
    cur().text2
}
pub fn text3() -> Color32 {
    cur().text3
}

// paleta pre widgety v paneli (tlačidlá, výber…): rovnaké polia ako Pal, ale podľa dizajnu
pub fn pal(base: &Pal) -> Pal {
    *base
}

// panel s rámikom
pub fn panel(ui: &mut egui::Ui, margin: i8, add: impl FnOnce(&mut egui::Ui)) -> egui::Rect {
    let w = ui.available_width();
    let r = egui::Frame::NONE.fill(panel_fill()).stroke(Stroke::new(1.0, border())).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(margin)).show(ui, |ui| {
        ui.set_width(w - 2.0 * margin as f32 - 2.0);
        add(ui);
    });
    r.response.rect
}

pub fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(theme::bold(11.0)).color(text3()));
}

// malé čiarkové ikony predvolieb (LED, tlačidlo, PWM, senzor, servo, bzučiak)
fn glyph(ui: &egui::Ui, kind: &str, c: Pos2, col: Color32) {
    let pa = ui.painter();
    let st = Stroke::new(1.6, col);
    match kind {
        "led" => {
            pa.circle_stroke(c + vec2(0.0, 1.0), 4.5, st);
            for k in 0..5 {
                let a = -std::f32::consts::PI * (0.15 + 0.175 * k as f32 * 4.0 / 4.0) - 0.0;
                let d = vec2(a.cos(), a.sin());
                pa.line_segment([c + vec2(0.0, 1.0) + d * 7.0, c + vec2(0.0, 1.0) + d * 9.0], Stroke::new(1.4, col));
            }
            pa.line_segment([c + vec2(-2.0, 7.0), c + vec2(-2.0, 9.0)], st);
            pa.line_segment([c + vec2(2.0, 7.0), c + vec2(2.0, 9.0)], st);
        }
        "button" => {
            pa.rect_stroke(egui::Rect::from_center_size(c, vec2(15.0, 15.0)), CornerRadius::same(4), st, StrokeKind::Inside);
            pa.circle_filled(c, 3.2, col);
        }
        "pwm" => {
            let pts: Vec<Pos2> = (0..=16).map(|i| c + vec2(-8.0 + i as f32, (i as f32 * 0.8).sin() * -4.5)).collect();
            pa.line(pts, st);
        }
        "analog" => {
            pa.circle_stroke(c, 6.5, st);
            pa.line_segment([c, c + vec2(3.5, -3.5)], st);
        }
        "servo" => {
            pa.rect_stroke(egui::Rect::from_center_size(c + vec2(0.0, 2.5), vec2(15.0, 8.0)), CornerRadius::same(2), st, StrokeKind::Inside);
            pa.circle_stroke(c + vec2(3.0, -3.5), 2.2, st);
            pa.line_segment([c + vec2(3.0, -3.5), c + vec2(-3.0, -6.0)], st);
        }
        "buzzer" => {
            pa.add(egui::Shape::convex_polygon(vec![c + vec2(-7.0, -2.5), c + vec2(-3.5, -2.5), c + vec2(0.5, -6.0), c + vec2(0.5, 6.0), c + vec2(-3.5, 2.5), c + vec2(-7.0, 2.5)], col, Stroke::NONE));
            pa.line(vec![c + vec2(3.0, -3.0), c + vec2(4.5, 0.0), c + vec2(3.0, 3.0)], Stroke::new(1.4, col));
            pa.line(vec![c + vec2(5.5, -5.0), c + vec2(8.0, 0.0), c + vec2(5.5, 5.0)], Stroke::new(1.4, col));
        }
        _ => {}
    }
}

// tlačidlo predvoľby: ikona + názov, aktívne = tyrkysový okraj; pri focuse (Tab) hrubší okraj
pub fn preset(ui: &mut egui::Ui, kind: &str, label: &str, active: bool) -> egui::Response {
    let tw = crate::widgets::text_w(ui, label, theme::ui(13.0));
    let (r, resp) = ui.allocate_exact_size(vec2(tw + 52.0, 36.0), Sense::click());
    let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
    let fill = if active { cyan().gamma_multiply(0.14) } else { field().lerp_to_gamma(hover(), hk) };
    ui.painter().rect_filled(r, CornerRadius::same(9), fill);
    let (w, col) = if resp.has_focus() {
        (2.0, cyan())
    } else if active {
        (1.5, cyan())
    } else {
        (1.0, if hk > 0.0 { border2() } else { border() })
    };
    ui.painter().rect_stroke(r, CornerRadius::same(9), Stroke::new(w, col), StrokeKind::Inside);
    glyph(ui, kind, pos2(r.left() + 21.0, r.center().y), if active { cyan() } else { text2() });
    ui.painter().text(pos2(r.left() + 38.0, r.center().y), egui::Align2::LEFT_CENTER, label, theme::ui(13.0), if active { text() } else { text2() });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// čip dosky pre druhý riadok výberu
pub fn chip_of(fqbn: &str) -> &'static str {
    match fqbn {
        f if f.contains(":mega") => "ATmega2560",
        f if f.contains("nona4809") => "ATmega4809",
        f if f.contains("renesas") => "Renesas RA4M1",
        f if f.contains("esp32s3") => "ESP32-S3",
        f if f.contains("esp32") => "ESP32",
        f if f.contains("esp8266") => "ESP8266",
        f if f.contains("rp2040") => "RP2040",
        f if f.contains("leonardo") || f.contains(":micro") => "ATmega32U4",
        f if f.contains("avr") => "ATmega328P",
        _ => "",
    }
}
