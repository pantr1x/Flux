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
// malé čiarkové ikony (LED/žiarovka, tlačidlo, PWM, senzor, servo, bzučiak); k = mierka (1.0 ≈ 16 px)
pub fn glyph(pa: &egui::Painter, kind: &str, c: Pos2, col: Color32, k: f32) {
    let w = (1.6 * k).max(1.1);
    let st = Stroke::new(w, col);
    let v = |x: f32, y: f32| c + vec2(x, y) * k;
    match kind {
        "led" => {
            pa.circle_stroke(v(0.0, -1.0), 4.5 * k, st);
            pa.line_segment([v(-2.0, 3.8), v(-2.0, 7.0)], st);
            pa.line_segment([v(2.0, 3.8), v(2.0, 7.0)], st);
            pa.line_segment([v(-2.0, 7.0), v(2.0, 7.0)], st);
            for (x0, y0, x1, y1) in [(0.0, -8.0, 0.0, -10.0), (-6.0, -4.5, -8.0, -5.5), (6.0, -4.5, 8.0, -5.5)] {
                pa.line_segment([v(x0, y0), v(x1, y1)], Stroke::new(w * 0.85, col));
            }
        }
        "button" => {
            pa.rect_stroke(egui::Rect::from_center_size(c, vec2(15.0, 15.0) * k), CornerRadius::same((4.0 * k) as u8), st, StrokeKind::Inside);
            pa.circle_filled(c, 3.2 * k, col);
        }
        "pwm" => {
            let pts: Vec<Pos2> = (0..=16).map(|i| v(-8.0 + i as f32, (i as f32 * 0.8).sin() * -4.5)).collect();
            pa.line(pts, st);
        }
        "analog" => {
            pa.circle_stroke(c, 6.5 * k, st);
            pa.line_segment([c, v(3.5, -3.5)], st);
        }
        "servo" => {
            pa.rect_stroke(egui::Rect::from_center_size(v(0.0, 2.5), vec2(15.0, 8.0) * k), CornerRadius::same((2.0 * k) as u8), st, StrokeKind::Inside);
            pa.circle_stroke(v(3.0, -3.5), 2.2 * k, st);
            pa.line_segment([v(3.0, -3.5), v(-3.0, -6.0)], st);
        }
        "buzzer" => {
            pa.add(egui::Shape::convex_polygon(vec![v(-7.0, -2.5), v(-3.5, -2.5), v(0.5, -6.0), v(0.5, 6.0), v(-3.5, 2.5), v(-7.0, 2.5)], col, Stroke::NONE));
            pa.line(vec![v(3.0, -3.0), v(4.5, 0.0), v(3.0, 3.0)], Stroke::new(w * 0.85, col));
            pa.line(vec![v(5.5, -5.0), v(8.0, 0.0), v(5.5, 5.0)], Stroke::new(w * 0.85, col));
        }
        _ => {}
    }
}

// režim pinu (out, pwm, in, pull, analog, servo, tone) → ikona súčiastky
pub fn mode_glyph(mode: &str) -> &'static str {
    match mode {
        "out" => "led",
        "pwm" => "pwm",
        "in" | "pull" => "button",
        "analog" => "analog",
        "servo" => "servo",
        "tone" => "buzzer",
        _ => "",
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
    glyph(ui.painter(), kind, pos2(r.left() + 21.0, r.center().y), if active { cyan() } else { text2() }, 1.0);
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
