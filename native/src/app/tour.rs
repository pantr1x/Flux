// Prehliadka funkcií (TOUR v onboarding.js): stmavené okno s „reflektorom“ na jednej časti a bublina
// s textom, n / 6, Skip a Next/Finish. Obdĺžniky častí si okno zapamätá pri kreslení (tour_rects).
use super::App;
use crate::i18n::t;
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};

pub const STEPS: [(&str, &str, &str); 6] = [
    ("projects", "Projects", "All your projects in one place. Hover to pin one, right-click to rename it."),
    ("files", "Files", "Create files from templates (Ctrl+N), folders and refresh the tree."),
    ("brand", "Start screen", "Click the logo any time to pick what to build next."),
    ("run", "Run & Live Server", "F5 runs Python instantly. For websites you get a live preview that reloads on save."),
    ("status", "Status bar", "Python version, autocomplete and errors – click the error count to jump to a problem."),
    ("settings", "Settings", "Themes, colors, fonts, language and more. Tip: Ctrl+Shift+P finds any command."),
];

impl App {
    pub(super) fn tour_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let Some(step) = self.tour else { return };
        let p = self.pal;
        // krok bez viditeľnej časti (napr. Run bez otvoreného súboru) sa preskočí
        let mut step = step;
        while step < STEPS.len() && !self.tour_rects.contains_key(STEPS[step].0) {
            step += 1;
        }
        if step >= STEPS.len() {
            self.tour = None;
            return;
        }
        self.tour = Some(step);
        let (key, title, text) = STEPS[step];
        let target = self.tour_rects[key].expand(6.0);
        // reflektor sa presúva plynulo
        let anim = if self.anim_on() { 0.25 } else { 0.0 };
        let ctx = ui.ctx().clone();
        let a = |n: &str, v: f32| ctx.animate_value_with_time(egui::Id::new(("tour", n)), v, anim);
        let spot = Rect::from_min_max(pos2(a("x0", target.left()), a("y0", target.top())), pos2(a("x1", target.right()), a("y1", target.bottom())));
        let dim = Color32::from_black_alpha(if p.dark { 150 } else { 110 });
        // štyri pásy okolo reflektora (klik mimo bubliny nič nerobí)
        let _ = ui.interact(full, egui::Id::new("tour-bg"), Sense::click());
        for r in [
            Rect::from_min_max(full.min, pos2(full.right(), spot.top())),
            Rect::from_min_max(pos2(full.left(), spot.bottom()), full.max),
            Rect::from_min_max(pos2(full.left(), spot.top()), pos2(spot.left(), spot.bottom())),
            Rect::from_min_max(pos2(spot.right(), spot.top()), pos2(full.right(), spot.bottom())),
        ] {
            ui.painter().rect_filled(r, 0.0, dim);
        }
        ui.painter().rect_stroke(spot, CornerRadius::same(10), Stroke::new(2.0, p.accent), StrokeKind::Outside);
        // bublina vedľa cieľa (vpravo, inak vľavo; pod ním, ak je hore dosť miesta nižšie)
        let (w, h) = (300.0, 150.0);
        let x = if spot.right() + 16.0 + w < full.right() - 8.0 { spot.right() + 16.0 } else { (spot.left() - 16.0 - w).max(full.left() + 8.0) };
        let y = spot.top().clamp(full.top() + 8.0, full.bottom() - h - 8.0);
        let x = if x + w > full.right() { full.right() - w - 8.0 } else { x };
        let tip = Rect::from_min_size(pos2(x, y), vec2(w, h));
        ui.painter().add(egui::Shadow { offset: [0, 10], blur: 30, spread: 0, color: Color32::from_black_alpha(100) }.as_shape(tip, CornerRadius::same(14)));
        ui.painter().rect_filled(tip, CornerRadius::same(14), p.solid);
        ui.painter().rect_stroke(tip, CornerRadius::same(14), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let _ = ui.interact(tip, egui::Id::new("tour-tip"), Sense::click());
        ui.painter().text(pos2(tip.left() + 18.0, tip.top() + 22.0), Align2::LEFT_CENTER, format!("{} / {}", step + 1, STEPS.len()), theme::ui(11.5), p.text3);
        ui.painter().text(pos2(tip.left() + 18.0, tip.top() + 44.0), Align2::LEFT_CENTER, t(title), theme::bold(16.0), p.text);
        let g = ui.painter().layout(t(text), theme::ui(12.5), p.text2, w - 36.0);
        ui.painter().galley(pos2(tip.left() + 18.0, tip.top() + 60.0), g, p.text2);
        let mut bar = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(pos2(tip.left() + 14.0, tip.bottom() - 44.0), pos2(tip.right() - 14.0, tip.bottom() - 12.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        bar.spacing_mut().item_spacing.x = 6.0;
        let last = step + 1 == STEPS.len();
        let next = widgets::button(&mut bar, None, &if last { t("Finish") } else { t("Next") }, p.accent, p.accent_fg, 30.0, &p).clicked()
            || ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::ArrowRight));
        let skip = widgets::button(&mut bar, None, &t("Skip"), p.card2, p.text, 30.0, &p).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape));
        if skip || (next && last) {
            self.tour = None;
        } else if next {
            self.tour = Some(step + 1);
        }
    }
}
