// Vlastná titulná lišta ako v Electron Fluxe (titleBarOverlay): okno je bez rámu Windows,
// hornú lištu ťaháš myšou, dvojklik maximalizuje, vpravo hore sú − □ × a okraje menia veľkosť.
use super::App;
use crate::i18n::t;
use crate::theme::TOP_H;
use eframe::egui::{self, pos2, vec2, Color32, Rect, Sense, Stroke, ViewportCommand};

pub const BTN_W: f32 = 46.0;
pub const CONTROLS_W: f32 = BTN_W * 3.0;

fn maximized(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

// prázdne miesto v lište: ťahanie presúva okno, dvojklik maximalizuje / obnoví
pub fn drag_area(ui: &mut egui::Ui, r: Rect, id: &str) {
    let resp = ui.interact(r, egui::Id::new(("drag", id)), Sense::click_and_drag());
    if resp.double_clicked() {
        let m = maximized(ui.ctx());
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!m));
    } else if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

impl App {
    // − □ × a okraje na zmenu veľkosti, vždy navrchu (aj nad nastaveniami a introm)
    pub(super) fn window_chrome(&mut self, ctx: &egui::Context) {
        let full = ctx.content_rect();
        let p = self.pal;
        let max = maximized(ctx);
        egui::Area::new(egui::Id::new("window-chrome")).order(egui::Order::Tooltip).fixed_pos(full.min).interactable(true).show(ctx, |ui| {
            let top = full.top();
            let mut x = full.right() - CONTROLS_W;
            let h = if max { TOP_H - 8.0 } else { TOP_H - 10.0 };
            for (i, what) in ["min", "max", "close"].iter().enumerate() {
                let r = Rect::from_min_size(pos2(x, top), vec2(BTN_W, h));
                let resp = ui.interact(r, egui::Id::new(("wbtn", i)), Sense::click());
                let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.1);
                let close = *what == "close";
                let hover_bg = if close { Color32::from_rgb(0xc4, 0x2b, 0x1c) } else { p.hover.gamma_multiply(1.6) };
                if hk > 0.0 {
                    // pravý horný roh okna je zaoblený (Windows 11) – × ho kopíruje
                    let cr = if close && !max { egui::CornerRadius { ne: 8, ..Default::default() } } else { egui::CornerRadius::ZERO };
                    ui.painter().rect_filled(r, cr, hover_bg.gamma_multiply(hk));
                }
                let col = if close { p.text2.lerp_to_gamma(Color32::WHITE, hk) } else { p.text2.lerp_to_gamma(p.text, hk) };
                let s = Stroke::new(1.0, col);
                let c = r.center();
                match *what {
                    "min" => {
                        ui.painter().hline(c.x - 5.0..=c.x + 5.0, c.y + 0.5, s);
                    }
                    "max" if max => {
                        // obnoviť: dva prekrývajúce sa štvorce
                        ui.painter().rect_stroke(Rect::from_min_size(pos2(c.x - 5.0, c.y - 3.0), vec2(8.0, 8.0)), 1.5, s, egui::StrokeKind::Inside);
                        ui.painter().line_segment([pos2(c.x - 3.0, c.y - 5.0), pos2(c.x + 5.0, c.y - 5.0)], s);
                        ui.painter().line_segment([pos2(c.x + 5.0, c.y - 5.0), pos2(c.x + 5.0, c.y + 3.0)], s);
                    }
                    "max" => {
                        ui.painter().rect_stroke(Rect::from_center_size(c, vec2(10.0, 10.0)), 1.5, s, egui::StrokeKind::Inside);
                    }
                    _ => {
                        ui.painter().line_segment([pos2(c.x - 5.0, c.y - 5.0), pos2(c.x + 5.0, c.y + 5.0)], s);
                        ui.painter().line_segment([pos2(c.x + 5.0, c.y - 5.0), pos2(c.x - 5.0, c.y + 5.0)], s);
                    }
                }
                let resp = resp.on_hover_text(match *what {
                    "min" => t("Minimize"),
                    "max" if max => t("Restore"),
                    "max" => t("Maximize"),
                    _ => t("Close"),
                });
                if resp.clicked() {
                    ctx.send_viewport_cmd(match *what {
                        "min" => ViewportCommand::Minimized(true),
                        "max" => ViewportCommand::Maximized(!max),
                        _ => ViewportCommand::Close,
                    });
                }
                x += BTN_W;
            }
            if max {
                return;
            }
            // okraje a rohy na zmenu veľkosti (6 px)
            use egui::viewport::ResizeDirection as D;
            use egui::CursorIcon as C;
            let e = 5.0;
            let (l, r, t0, b) = (full.left(), full.right(), full.top(), full.bottom());
            let edges = [
                (Rect::from_min_max(pos2(l, t0), pos2(l + e * 2.0, t0 + e * 2.0)), D::NorthWest, C::ResizeNorthWest),
                (Rect::from_min_max(pos2(r - e * 2.0, b - e * 2.0), pos2(r, b)), D::SouthEast, C::ResizeSouthEast),
                (Rect::from_min_max(pos2(l, b - e * 2.0), pos2(l + e * 2.0, b)), D::SouthWest, C::ResizeSouthWest),
                (Rect::from_min_max(pos2(l + e * 2.0, t0), pos2(r - CONTROLS_W, t0 + 3.0)), D::North, C::ResizeVertical),
                (Rect::from_min_max(pos2(l + e * 2.0, b - e), pos2(r - e * 2.0, b)), D::South, C::ResizeVertical),
                (Rect::from_min_max(pos2(l, t0 + e * 2.0), pos2(l + e, b - e * 2.0)), D::West, C::ResizeHorizontal),
                (Rect::from_min_max(pos2(r - e, t0 + TOP_H), pos2(r, b - e * 2.0)), D::East, C::ResizeHorizontal),
            ];
            for (i, (rect, dir, cur)) in edges.into_iter().enumerate() {
                let resp = ui.interact(rect, egui::Id::new(("resize", i)), Sense::drag()).on_hover_cursor(cur);
                if resp.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                }
            }
        });
    }
}
