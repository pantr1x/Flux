// Obrázky dosiek: Arduino Uno a Mega 2560 (pätice hore a dole), Nano, ESP32 DevKit a Raspberry Pi Pico.
// Vykreslené maľovaním (bez obrázkov); piny sa dajú klikať, použité piny majú očíslovanú bublinu.
use super::arduino::mode_info;
use crate::arduino as ard;
use crate::i18n::t;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind};

// pin dosky na obrázku: popisok, kľúč pre kód (číslo GPIO; prázdny = nejde použiť), druh: 0 GPIO, 1 GND, 2 napájanie, 3 iné
#[derive(Clone, Copy)]
pub struct Pad {
    pub label: &'static str,
    pub key: &'static str,
    pub kind: u8,
}

const fn pd(label: &'static str, key: &'static str, kind: u8) -> Pad {
    Pad { label, key, kind }
}

// ESP32 DevKitC (38 pinov), USB dole
pub const ESP_L: [Pad; 19] = [
    pd("3V3", "", 2),
    pd("EN", "", 3),
    pd("VP", "36", 0),
    pd("VN", "39", 0),
    pd("34", "34", 0),
    pd("35", "35", 0),
    pd("32", "32", 0),
    pd("33", "33", 0),
    pd("25", "25", 0),
    pd("26", "26", 0),
    pd("27", "27", 0),
    pd("14", "14", 0),
    pd("12", "12", 0),
    pd("GND", "", 1),
    pd("13", "13", 0),
    pd("SD2", "", 3),
    pd("SD3", "", 3),
    pd("CMD", "", 3),
    pd("5V", "", 2),
];
pub const ESP_R: [Pad; 19] = [
    pd("GND", "", 1),
    pd("23", "23", 0),
    pd("22", "22", 0),
    pd("TX0", "1", 0),
    pd("RX0", "3", 0),
    pd("21", "21", 0),
    pd("GND", "", 1),
    pd("19", "19", 0),
    pd("18", "18", 0),
    pd("5", "5", 0),
    pd("17", "17", 0),
    pd("16", "16", 0),
    pd("4", "4", 0),
    pd("0", "0", 0),
    pd("2", "2", 0),
    pd("15", "15", 0),
    pd("SD1", "", 3),
    pd("SD0", "", 3),
    pd("CLK", "", 3),
];
// Raspberry Pi Pico, USB hore
pub const PICO_L: [Pad; 20] = [
    pd("GP0", "0", 0),
    pd("GP1", "1", 0),
    pd("GND", "", 1),
    pd("GP2", "2", 0),
    pd("GP3", "3", 0),
    pd("GP4", "4", 0),
    pd("GP5", "5", 0),
    pd("GND", "", 1),
    pd("GP6", "6", 0),
    pd("GP7", "7", 0),
    pd("GP8", "8", 0),
    pd("GP9", "9", 0),
    pd("GND", "", 1),
    pd("GP10", "10", 0),
    pd("GP11", "11", 0),
    pd("GP12", "12", 0),
    pd("GP13", "13", 0),
    pd("GND", "", 1),
    pd("GP14", "14", 0),
    pd("GP15", "15", 0),
];
pub const PICO_R: [Pad; 20] = [
    pd("VBUS", "", 2),
    pd("VSYS", "", 2),
    pd("GND", "", 1),
    pd("3V3_EN", "", 3),
    pd("3V3", "", 2),
    pd("VREF", "", 3),
    pd("GP28", "28", 0),
    pd("GND", "", 1),
    pd("GP27", "27", 0),
    pd("GP26", "26", 0),
    pd("RUN", "", 3),
    pd("GP22", "22", 0),
    pd("GND", "", 1),
    pd("GP21", "21", 0),
    pd("GP20", "20", 0),
    pd("GP19", "19", 0),
    pd("GP18", "18", 0),
    pd("GND", "", 1),
    pd("GP17", "17", 0),
    pd("GP16", "16", 0),
];
// Arduino Nano, USB hore
pub const NANO_L: [Pad; 15] = [
    pd("TX1", "1", 0),
    pd("RX0", "0", 0),
    pd("RST", "", 3),
    pd("GND", "", 1),
    pd("2", "2", 0),
    pd("3", "3", 0),
    pd("4", "4", 0),
    pd("5", "5", 0),
    pd("6", "6", 0),
    pd("7", "7", 0),
    pd("8", "8", 0),
    pd("9", "9", 0),
    pd("10", "10", 0),
    pd("11", "11", 0),
    pd("12", "12", 0),
];
pub const NANO_R: [Pad; 15] = [
    pd("VIN", "", 2),
    pd("GND", "", 1),
    pd("RST", "", 3),
    pd("5V", "", 2),
    pd("A7", "A7", 0),
    pd("A6", "A6", 0),
    pd("A5", "A5", 0),
    pd("A4", "A4", 0),
    pd("A3", "A3", 0),
    pd("A2", "A2", 0),
    pd("A1", "A1", 0),
    pd("A0", "A0", 0),
    pd("REF", "", 3),
    pd("3V3", "", 2),
    pd("13", "13", 0),
];

fn gray(g: u8) -> Color32 {
    Color32::from_gray(g)
}

fn pickable(l: &str) -> bool {
    !l.is_empty() && (l.chars().all(|c| c.is_ascii_digit()) || (l.starts_with('A') && l.len() <= 3 && l.len() > 1 && l[1..].chars().all(|c| c.is_ascii_digit())))
}

// Arduino Uno (mega = false) alebo Mega 2560 (mega = true): vráti kľúč kliknutého pinu
pub fn draw_avr(ui: &mut egui::Ui, _p: &theme::Pal, w: f32, mega: bool, pins: &[ard::Pin], sel: &Option<String>) -> Option<String> {
    let side = if mega { 40.0 } else { 6.0 };
    let bw = (w - 28.0 - side).max(120.0);
    let bh = bw * if mega { 0.52 } else { 0.64 };
    let margin = if mega { 60.0 } else { 42.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(w, bh + 2.0 * margin), Sense::hover());
    let pa = ui.painter_at(rect);
    let board = Rect::from_min_size(pos2(rect.left() + 24.0, rect.top() + margin), vec2(bw, bh));
    let at = |fx: f32, fy: f32| pos2(board.left() + bw * fx, board.top() + bh * fy);
    let area = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(at(x0, y0), at(x1, y1));
    let teal = if mega { Color32::from_rgb(0x00, 0x7a, 0x86) } else { Color32::from_rgb(0x00, 0x87, 0x8f) };
    // doska + jemný lesk
    pa.rect_filled(board.translate(vec2(0.0, 3.0)), CornerRadius::same(12), Color32::from_black_alpha(70));
    pa.rect_filled(board, CornerRadius::same(12), teal);
    pa.rect_filled(area(0.0, 0.0, 1.0, 0.18), CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 }, Color32::from_white_alpha(10));
    pa.rect_stroke(board, CornerRadius::same(12), Stroke::new(1.0, Color32::from_black_alpha(90)), StrokeKind::Inside);
    // montážne otvory
    for (fx, fy) in [(0.12, 0.93), (0.74, 0.93), (0.12, 0.07), (0.78, 0.07)] {
        pa.circle_filled(at(fx, fy), (bh * 0.022).max(2.0), Color32::from_black_alpha(150));
    }
    // USB-B (strieborné), DC jack (čierny)
    pa.rect_filled(area(-0.04, 0.09, 0.14, 0.36), CornerRadius::same(3), gray(196));
    pa.rect_filled(area(-0.02, 0.13, 0.10, 0.32), CornerRadius::same(2), gray(120));
    pa.rect_filled(area(-0.035, 0.62, 0.12, 0.90), CornerRadius::same(3), gray(26));
    pa.circle_filled(at(0.02, 0.76), bh * 0.05, gray(60));
    // reset
    pa.circle_filled(at(0.20, 0.13), bh * 0.04, gray(210));
    pa.circle_filled(at(0.20, 0.13), bh * 0.025, Color32::from_rgb(0xc8, 0x30, 0x30));
    // čipy
    pa.rect_filled(area(0.22, 0.40, 0.31, 0.54), CornerRadius::same(2), gray(28));
    pa.rect_filled(area(0.33, 0.50, 0.38, 0.58), CornerRadius::same(2), gray(205));
    if mega {
        pa.rect_filled(area(0.44, 0.46, 0.60, 0.74), CornerRadius::same(2), gray(24));
        pa.text(at(0.52, 0.60), Align2::CENTER_CENTER, "ATMEGA\n2560", theme::bold((bh * 0.045).clamp(5.0, 9.0)), gray(120));
    } else {
        pa.rect_filled(area(0.42, 0.55, 0.80, 0.70), CornerRadius::same(2), gray(24));
        pa.circle_filled(at(0.425, 0.625), bh * 0.02, gray(60));
        pa.text(at(0.62, 0.625), Align2::CENTER_CENTER, "ATMEGA328P", theme::bold((bh * 0.05).clamp(5.0, 10.0)), gray(120));
    }
    // LED (L, TX, RX, ON)
    for (i, c) in [Color32::from_rgb(0xff, 0xb3, 0x47), Color32::from_rgb(0xff, 0xd0, 0x40), Color32::from_rgb(0xff, 0xd0, 0x40), Color32::from_rgb(0x4a, 0xe0, 0x7a)].iter().enumerate() {
        pa.rect_filled(Rect::from_center_size(at(0.40 + i as f32 * 0.035, 0.30), vec2(bh * 0.04, bh * 0.025)), CornerRadius::same(1), *c);
    }
    // ICSP
    for r in 0..2 {
        for c in 0..3 {
            let x = if mega { 0.80 } else { 0.90 } + c as f32 * 0.018;
            pa.rect_filled(Rect::from_center_size(at(x, 0.48 + r as f32 * 0.05), vec2(3.0, 3.0)), CornerRadius::ZERO, Color32::from_rgb(0xe0, 0xc0, 0x60));
        }
    }
    // logo (dva krúžky s + a −) a nápis modelu
    let lr = bh * 0.055;
    let lc = at(if mega { 0.27 } else { 0.28 }, 0.74);
    pa.circle_stroke(lc - vec2(lr * 0.9, 0.0), lr, Stroke::new(2.0, Color32::from_white_alpha(230)));
    pa.circle_stroke(lc + vec2(lr * 0.9, 0.0), lr, Stroke::new(2.0, Color32::from_white_alpha(230)));
    pa.line_segment([lc - vec2(lr * 1.4, 0.0), lc - vec2(lr * 0.4, 0.0)], Stroke::new(1.6, Color32::from_white_alpha(230)));
    pa.line_segment([lc + vec2(lr * 0.4, 0.0), lc + vec2(lr * 1.4, 0.0)], Stroke::new(1.6, Color32::from_white_alpha(230)));
    pa.line_segment([lc + vec2(lr * 0.9, -lr * 0.5), lc + vec2(lr * 0.9, lr * 0.5)], Stroke::new(1.6, Color32::from_white_alpha(230)));
    pa.text(at(if mega { 0.52 } else { 0.62 }, 0.38), Align2::CENTER_CENTER, if mega { "MEGA 2560" } else { "UNO" }, theme::bold((bh * 0.13).clamp(11.0, 26.0)), Color32::from_white_alpha(235));
    pa.text(at(if mega { 0.52 } else { 0.62 }, 0.26), Align2::CENTER_CENTER, "ARDUINO", theme::bold((bh * 0.05).clamp(6.0, 11.0)), Color32::from_white_alpha(170));

    // pätice
    let top: Vec<&str> = if mega {
        vec!["SCL", "SDA", "AREF", "GND", "13", "12", "11", "10", "9", "8", "", "7", "6", "5", "4", "3", "2", "1", "0", "", "14", "15", "16", "17", "18", "19", "20", "21"]
    } else {
        vec!["SCL", "SDA", "AREF", "GND", "13", "12", "11", "10", "9", "8", "", "7", "6", "5", "4", "3", "2", "1", "0"]
    };
    let bottom: Vec<&str> = if mega {
        vec!["", "IOREF", "RST", "3V3", "5V", "GND", "GND", "VIN", "", "A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "", "A8", "A9", "A10", "A11", "A12", "A13", "A14", "A15"]
    } else {
        vec!["", "IOREF", "RST", "3V3", "5V", "GND", "GND", "VIN", "", "A0", "A1", "A2", "A3", "A4", "A5"]
    };
    let used = |name: &str| pins.iter().position(|x| x.pin == name);
    let power = pins.iter().any(|x| matches!(x.mode, "in" | "read" | "analog" | "servo" | "neo" | "dht" | "onewire" | "lcd" | "serial"));
    let clicked = std::cell::Cell::new(None::<String>);
    let levels = if mega { 3 } else { 2 };
    let s = (bw / if mega { 120.0 } else { 52.0 }).clamp(3.0, 8.0);
    let draw_row = |labels: &[&str], y: f32, x0: f32, x1: f32, up: bool, row: u8| {
        let n = labels.len() as f32;
        // čierna pätica
        let strip = Rect::from_min_max(pos2(board.left() + bw * x0 - s, board.top() + bh * y - s * 0.9), pos2(board.left() + bw * x1 + s, board.top() + bh * y + s * 0.9));
        pa.rect_filled(strip, CornerRadius::same(2), gray(16));
        for (i, l) in labels.iter().enumerate() {
            if l.is_empty() {
                continue;
            }
            let x = board.left() + bw * (x0 + (x1 - x0) * i as f32 / (n - 1.0));
            let c = pos2(x, board.top() + bh * y);
            pa.rect_filled(Rect::from_center_size(c, vec2(s * 0.7, s * 0.7)), 1.0, gray(80));
            let step = bw * (x1 - x0) / (n - 1.0);
            let ty = if up { c.y + s + 7.0 } else { c.y - s - 7.0 };
            let short = match *l {
                "SCL" | "SDA" | "AREF" | "IOREF" | "RST" | "VIN" if step < 22.0 => "",
                "GND" if step < 22.0 => "G",
                "3V3" if step < 22.0 => "3V",
                other => other,
            };
            pa.text(pos2(x, ty), Align2::CENTER_CENTER, short, theme::ui((step * 0.7).clamp(5.0, 10.0)), Color32::from_white_alpha(215));
            let pwr = power && (*l == "5V" || *l == "GND");
            if pickable(l) {
                let hit = Rect::from_center_size(pos2(x, c.y + if up { 7.0 } else { -7.0 }), vec2(step.max(7.0), 26.0));
                let resp = ui.interact(hit, ui.id().with(("ard-pad", row, i)), Sense::click());
                if resp.hovered() {
                    pa.circle_stroke(c, s * 1.6, Stroke::new(1.5, Color32::from_white_alpha(170)));
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("{} {l}", t("Pin"))).clicked() {
                    clicked.set(Some(l.to_string()));
                }
            }
            if sel.as_deref() == Some(*l) {
                pa.circle_filled(c, s * 1.5, super::dash::ORANGE.gamma_multiply(0.28));
                pa.circle_stroke(c, s * 1.5, Stroke::new(2.0, super::dash::ORANGE));
            }
            if let Some(k) = used(l) {
                let (col, _, _) = mode_info(pins[k].mode);
                let lift = (i % levels) as f32 * 19.0;
                let out = pos2(x, if up { rect.top() + 11.0 + lift } else { rect.bottom() - 11.0 - lift });
                pa.line_segment([c, out], Stroke::new(2.0, col));
                bubble(&pa, out, 10.0, col, &pins[k], k + 1);
                ui.interact(Rect::from_center_size(out, vec2(20.0, 20.0)), ui.id().with(("ard-bub", 1, k)), Sense::hover()).on_hover_text(bubble_tip(&pins[k]));
                pa.circle_stroke(c, s * 1.0, Stroke::new(2.0, col));
            } else if pwr {
                pa.circle_stroke(c, s * 1.0, Stroke::new(1.5, if *l == "5V" { Color32::from_rgb(0xff, 0x5c, 0x5c) } else { gray(235) }));
            }
        }
    };
    if mega {
        draw_row(&top, 0.06, 0.15, 0.82, true, 0);
        draw_row(&bottom, 0.94, 0.20, 0.82, false, 1);
        // dvojstĺpcová pätica 22–53 na pravom okraji
        let rows = 16;
        let (xa, xb) = (0.905, 0.958);
        let (y0, y1) = (0.16, 0.90);
        let strip = Rect::from_min_max(at(xa, y0).to_vec2().to_pos2() - vec2(s, s), at(xb, y1).to_vec2().to_pos2() + vec2(s, s));
        pa.rect_filled(strip, CornerRadius::same(2), gray(16));
        for r in 0..rows {
            let y = y0 + (y1 - y0) * r as f32 / (rows as f32 - 1.0);
            for (col, x) in [(0, xa), (1, xb)] {
                let label = (22 + r * 2 + col).to_string();
                let c = at(x, y);
                pa.rect_filled(Rect::from_center_size(c, vec2(s * 0.7, s * 0.7)), 1.0, gray(80));
                let (anchor, tx) = if col == 0 { (Align2::RIGHT_CENTER, c.x - s - 2.0) } else { (Align2::LEFT_CENTER, c.x + s + 2.0) };
                pa.text(pos2(tx, c.y), anchor, &label, theme::ui(6.0), Color32::from_white_alpha(200));
                let row_h = bh * (y1 - y0) / (rows as f32 - 1.0);
                let hit = Rect::from_center_size(c, vec2(s * 4.0, row_h.max(8.0)));
                let resp = ui.interact(hit, ui.id().with(("ard-mpad", r, col)), Sense::click());
                if resp.hovered() {
                    pa.circle_stroke(c, s * 1.6, Stroke::new(1.5, Color32::from_white_alpha(170)));
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("{} {label}", t("Pin"))).clicked() {
                    clicked.set(Some(label.clone()));
                }
                if sel.as_deref() == Some(label.as_str()) {
                    pa.circle_filled(c, s * 1.5, super::dash::ORANGE.gamma_multiply(0.28));
                    pa.circle_stroke(c, s * 1.5, Stroke::new(2.0, super::dash::ORANGE));
                }
                if let Some(k) = used(&label) {
                    let (mc, _, _) = mode_info(pins[k].mode);
                    let out = pos2(board.right() + 18.0 + ((r % 2) as f32) * 19.0, c.y);
                    pa.line_segment([c, out], Stroke::new(2.0, mc));
                    bubble(&pa, out, 9.5, mc, &pins[k], k + 1);
                    ui.interact(Rect::from_center_size(out, vec2(20.0, 20.0)), ui.id().with(("ard-bub", 2, k)), Sense::hover()).on_hover_text(bubble_tip(&pins[k]));
                    pa.circle_stroke(c, s * 1.0, Stroke::new(2.0, mc));
                }
            }
        }
    } else {
        draw_row(&top, 0.065, 0.30, 0.97, true, 0);
        draw_row(&bottom, 0.935, 0.40, 0.97, false, 1);
    }
    clicked.take()
}

// dosky s dvoma stĺpcami pinov po stranách: ESP32, Pico, Nano; `title` určuje ozdoby modulu
#[allow(clippy::too_many_arguments)]
pub fn draw_vertical(ui: &mut egui::Ui, _p: &theme::Pal, w: f32, title: &str, left: &[Pad], right: &[Pad], usb_top: bool, body: Color32, pins: &[ard::Pin], sel: &Option<String>) -> Option<String> {
    let n = left.len().max(right.len());
    let row_h = 15.0;
    let h = n as f32 * row_h + 76.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let pa = ui.painter_at(rect);
    let bw = 132.0f32.min(w * 0.5);
    let bx = rect.center().x;
    let board = Rect::from_min_max(pos2(bx - bw / 2.0, rect.top() + 8.0), pos2(bx + bw / 2.0, rect.bottom() - 8.0));
    pa.rect_filled(board.translate(vec2(0.0, 3.0)), CornerRadius::same(9), Color32::from_black_alpha(70));
    pa.rect_filled(board, CornerRadius::same(9), body);
    pa.rect_stroke(board, CornerRadius::same(9), Stroke::new(1.0, Color32::from_black_alpha(90)), StrokeKind::Inside);
    let usb = Rect::from_center_size(pos2(bx, if usb_top { board.top() + 5.0 } else { board.bottom() - 5.0 }), vec2(if title == "Nano" { 26.0 } else { 36.0 }, 15.0));
    pa.rect_filled(usb, CornerRadius::same(3), gray(196));
    pa.rect_filled(usb.shrink(3.0), CornerRadius::same(2), gray(110));
    let cy = board.center().y;
    match title {
        "ESP32" => {
            // kovový modul s anténou hore
            let m = Rect::from_center_size(pos2(bx, board.top() + 62.0), vec2(bw * 0.62, 70.0));
            pa.rect_filled(m, CornerRadius::same(3), gray(185));
            pa.text(m.center() + vec2(0.0, 6.0), Align2::CENTER_CENTER, "ESP32\nWROOM", theme::bold(9.0), gray(70));
            let ant = Rect::from_min_max(pos2(m.left(), m.top() - 24.0), pos2(m.right(), m.top() - 2.0));
            pa.rect_filled(ant, CornerRadius::same(2), Color32::from_rgb(0x12, 0x14, 0x18));
            for k in 0..6 {
                let x = ant.left() + 5.0 + k as f32 * (ant.width() - 10.0) / 6.0;
                pa.line_segment([pos2(x, ant.bottom() - 3.0), pos2(x, ant.top() + if k % 2 == 0 { 3.0 } else { 9.0 })], Stroke::new(1.5, Color32::from_rgb(0xc8, 0xa8, 0x50)));
            }
            // tlačidlá EN / BOOT a čip USB
            for dx in [-18.0f32, 18.0] {
                pa.rect_filled(Rect::from_center_size(pos2(bx + dx, board.bottom() - 34.0), vec2(14.0, 10.0)), CornerRadius::same(2), gray(210));
            }
            pa.rect_filled(Rect::from_center_size(pos2(bx, board.bottom() - 58.0), vec2(16.0, 16.0)), CornerRadius::same(2), gray(26));
            pa.text(pos2(bx, cy + 24.0), Align2::CENTER_CENTER, "ESP32", theme::bold(15.0), Color32::from_white_alpha(70));
        }
        "Pico" => {
            pa.rect_filled(Rect::from_center_size(pos2(bx, board.top() + 52.0), vec2(14.0, 8.0)), CornerRadius::same(2), gray(235)); // BOOTSEL
            pa.rect_filled(Rect::from_center_size(pos2(bx, cy), vec2(34.0, 34.0)), CornerRadius::same(3), gray(22));
            pa.text(pos2(bx, cy), Align2::CENTER_CENTER, "RP2040", theme::bold(7.5), gray(120));
            pa.rect_filled(Rect::from_center_size(pos2(bx, board.bottom() - 50.0), vec2(30.0, 12.0)), CornerRadius::same(2), gray(190)); // flash
            pa.circle_filled(pos2(bx + 14.0, board.top() + 52.0), 2.5, Color32::from_rgb(0x4a, 0xe0, 0x7a));
            pa.text(pos2(bx, cy + 38.0), Align2::CENTER_CENTER, "Pico", theme::bold(15.0), Color32::from_white_alpha(210));
        }
        _ => {
            pa.rect_filled(Rect::from_center_size(pos2(bx, cy - 6.0), vec2(30.0, 30.0)), CornerRadius::same(3), gray(22));
            pa.text(pos2(bx, cy - 6.0), Align2::CENTER_CENTER, "ATMEGA\n328P", theme::bold(6.5), gray(120));
            pa.rect_filled(Rect::from_center_size(pos2(bx, board.top() + 46.0), vec2(16.0, 8.0)), CornerRadius::same(2), gray(210));
            pa.text(pos2(bx, cy + 30.0), Align2::CENTER_CENTER, "NANO", theme::bold(15.0), Color32::from_white_alpha(225));
        }
    }
    let y0 = if usb_top { board.top() + 30.0 } else { board.top() + 14.0 };
    let clicked = std::cell::Cell::new(None::<String>);
    for (side, pads) in [(0u8, left), (1u8, right)] {
        for (i, pad) in pads.iter().enumerate() {
            let y = y0 + i as f32 * row_h + row_h / 2.0;
            let px = if side == 0 { board.left() + 9.0 } else { board.right() - 9.0 };
            let c = pos2(px, y);
            let col = match pad.kind {
                0 => gray(22),
                1 => gray(150),
                2 => Color32::from_rgb(0xc0, 0x40, 0x40),
                _ => gray(70),
            };
            pa.rect_filled(Rect::from_center_size(c, vec2(7.0, 7.0)), 1.0, col);
            let (anchor, tx) = if side == 0 { (Align2::LEFT_CENTER, px + 8.0) } else { (Align2::RIGHT_CENTER, px - 8.0) };
            pa.text(pos2(tx, y), anchor, pad.label, theme::ui(8.5), Color32::from_white_alpha(if pad.kind == 0 { 220 } else { 135 }));
            if pad.kind == 0 && !pad.key.is_empty() {
                let hit = if side == 0 {
                    Rect::from_min_max(pos2(board.left() - 4.0, y - row_h / 2.0), pos2(board.center().x, y + row_h / 2.0))
                } else {
                    Rect::from_min_max(pos2(board.center().x, y - row_h / 2.0), pos2(board.right() + 4.0, y + row_h / 2.0))
                };
                let resp = ui.interact(hit, ui.id().with(("ard-vpad", title, side, i)), Sense::click());
                if resp.hovered() {
                    pa.circle_stroke(c, 6.0, Stroke::new(1.5, Color32::from_white_alpha(170)));
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("{} {}", t("Pin"), pad.key)).clicked() {
                    clicked.set(Some(pad.key.to_string()));
                }
                if sel.as_deref() == Some(pad.key) {
                    pa.circle_filled(c, 7.5, super::dash::ORANGE.gamma_multiply(0.28));
                    pa.circle_stroke(c, 7.5, Stroke::new(2.0, super::dash::ORANGE));
                }
                if let Some(k) = pins.iter().position(|x| x.pin == pad.key) {
                    let (mc, _, _) = mode_info(pins[k].mode);
                    let lift = if i % 2 == 1 { 22.0 } else { 0.0 };
                    let ox = if side == 0 { board.left() - 16.0 - lift } else { board.right() + 16.0 + lift };
                    let out = Pos2::new(ox, y);
                    pa.line_segment([c, out], Stroke::new(2.0, mc));
                    bubble(&pa, out, 9.0, mc, &pins[k], k + 1);
                    ui.interact(Rect::from_center_size(out, vec2(20.0, 20.0)), ui.id().with(("ard-bub", 3, k)), Sense::hover()).on_hover_text(bubble_tip(&pins[k]));
                    pa.circle_stroke(c, 5.0, Stroke::new(2.0, mc));
                }
            }
        }
    }
    clicked.take()
}

// text pri značke: pin, každá súčiastka s tým, čo zapojiť, a varovanie pri konflikte
pub(super) fn bubble_tip(p: &ard::Pin) -> String {
    let mut s = format!("{} {}", t("Pin"), p.pin);
    if let Some(n) = &p.name {
        s += &format!(" ({n})");
    }
    for pt in &p.parts {
        let m = p.uses.iter().find(|u| ard::part_of(u.0) == *pt).map(|u| u.0).unwrap_or(p.mode);
        let (_, label, h) = mode_info(m);
        s += &format!("\n• {label} – {}", t(h));
    }
    // digitalRead na výstupe (prepínanie): len poznámka
    if p.parts.first() == Some(&"led") && p.uses.iter().any(|u| u.0 == "read") {
        s += &format!("\n• {}", t("Also read with digitalRead() – that only checks its state, no extra part."));
    }
    if p.conflict() {
        s += &format!("\n⚠ {}", t("Two different parts use this pin in your code – they would get in each other's way. Pick a free pin for the new one."));
    }
    s
}

// značka pri doske: ikona hlavnej súčiastky v farbe pinu, číslo v rohu; viac súčiastok = „+N“, konflikt = červený krúžok
pub(super) fn bubble(pa: &egui::Painter, c: Pos2, r: f32, col: Color32, pin: &ard::Pin, n: usize) {
    if pin.conflict() {
        pa.circle_stroke(c, r + 2.5, Stroke::new(2.0, Color32::from_rgb(0xff, 0x55, 0x55)));
    }
    pa.circle_filled(c, r, col);
    pa.circle_stroke(c, r, Stroke::new(1.0, Color32::from_black_alpha(90)));
    super::dash::glyph(pa, super::dash::mode_glyph(pin.mode), c + vec2(-0.6, 0.8), Color32::from_black_alpha(235), r / 13.0);
    let b = c + vec2(r * 0.75, -r * 0.75);
    pa.circle_filled(b, r * 0.52, Color32::from_black_alpha(225));
    pa.text(b, Align2::CENTER_CENTER, n.to_string(), theme::bold(r * 0.72), Color32::WHITE);
    if pin.parts.len() > 1 {
        let e = c + vec2(r * 0.75, r * 0.75);
        pa.circle_filled(e, r * 0.55, Color32::from_rgb(0xff, 0x55, 0x55));
        pa.text(e, Align2::CENTER_CENTER, format!("+{}", pin.parts.len() - 1), theme::bold(r * 0.62), Color32::WHITE);
    }
}
