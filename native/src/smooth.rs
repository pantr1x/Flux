// Plynulé posúvanie ako na tablete:
// - koliesko myši: každý zúbok stránku „postrčí“ (pridá rýchlosť) a tá sa plynulo stráca – pri točení ide plynulo,
//   po pustení dobehne a spomalí (jeden zúbok prejde presne svoju vzdialenosť);
// - touchpad: kým sú prsty na ploche, ide presne pod nimi; po pustení pokračuje rýchlosťou švihnutia a spomaľuje;
// - skoky (minimapa, hľadanie): pevný úsek 220 ms po Hermitovej krivke.
use eframe::egui::{self, Id, Rect};
use std::collections::HashMap;
use std::time::Instant;

const TAU_WHEEL: f32 = 0.30; // s – ako rýchlo spomalí po zúbku
const TAU_TOUCH: f32 = 0.45; // s – dlhší dobeh po švihnutí na touchpade
const MAX_VEL: f32 = 9000.0; // px/s
const STOP_VEL: f32 = 3.0; // px/s – zvyšok dráhy (< 1 px) sa pridá naraz
const TOUCH_GAP: f64 = 0.05; // s bez pohybu prstov = pustené
const TOUCH_MIN_FLING: f32 = 120.0; // px/s – pomalší pohyb po pustení nedobieha
const JUMP: f32 = 0.22; // s – skok z minimapy/hľadania
const NOTCH: f32 = 1.5; // násobok vzdialenosti jedného zúbka (ako doteraz)

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Idle,
    Fling(f32), // tau
    Jump { from: f32, from_v: f32, target: f32, t: f32 },
}

#[derive(Clone, Copy)]
struct S {
    pos: f32,
    vel: f32,
    mode: Mode,
    rect: Rect,
    max: f32,
    last: f64,                // čas poslednej snímky tejto oblasti
    touching: bool,           // prsty sa práve hýbu
    last_touch: f64,          // čas posledného posunu prstami
    touch_vel: f32,           // rýchlosť prstov (px/s, vyhladená)
}

impl Default for S {
    fn default() -> Self {
        S { pos: 0.0, vel: 0.0, mode: Mode::Idle, rect: Rect::NOTHING, max: 0.0, last: 0.0, touching: false, last_touch: 0.0, touch_vel: 0.0 }
    }
}

impl S {
    fn clamp(&self, y: f32) -> f32 {
        y.clamp(0.0, self.max.max(0.0))
    }

    // zúbok kolieska: px > 0 = obsah ide hore (posun nadol)
    fn wheel(&mut self, px: f32) {
        self.touching = false;
        if let Mode::Jump { .. } = self.mode {
            self.mode = Mode::Idle;
        }
        // opačný smer = najprv zastaviť
        if self.vel * px < 0.0 || self.mode == Mode::Idle {
            self.vel = 0.0;
        }
        self.vel = (self.vel + px / TAU_WHEEL).clamp(-MAX_VEL, MAX_VEL);
        self.mode = Mode::Fling(TAU_WHEEL);
    }

    // posun prstami (px > 0 = posun nadol): hneď a presne, rýchlosť sa len zaznamená
    fn touch(&mut self, px: f32, now: f64) {
        let dt = (now - self.last_touch) as f32;
        let v = if dt > 0.0005 { px / dt.max(0.004) } else { self.touch_vel };
        // po dlhšej pauze nová rýchlosť, inak vyhladená
        self.touch_vel = if !self.touching || dt > 0.1 { v } else { 0.6 * v + 0.4 * self.touch_vel };
        self.touching = true;
        self.last_touch = now;
        self.mode = Mode::Idle;
        self.vel = 0.0;
        self.pos = self.clamp(self.pos + px);
    }

    fn jump(&mut self, target: f32) {
        self.touching = false;
        let from_v = if self.mode == Mode::Idle { 0.0 } else { self.vel };
        self.mode = Mode::Jump { from: self.pos, from_v, target: self.clamp(target), t: 0.0 };
    }

    // jedna snímka; vráti true, kým sa niečo hýbe
    fn tick(&mut self, now: f64) -> bool {
        let dt = ((now - self.last) as f32).clamp(0.001, 0.05);
        self.last = now;
        if self.touching && now - self.last_touch > TOUCH_GAP {
            // prsty pustené → dobeh rýchlosťou švihnutia
            self.touching = false;
            if self.touch_vel.abs() > TOUCH_MIN_FLING {
                self.vel = self.touch_vel.clamp(-MAX_VEL, MAX_VEL);
                self.mode = Mode::Fling(TAU_TOUCH);
            }
        }
        match self.mode {
            Mode::Idle => self.touching,
            Mode::Fling(tau) => {
                let e = (-dt / tau).exp();
                let next = self.pos + self.vel * tau * (1.0 - e);
                self.vel *= e;
                let c = self.clamp(next);
                if c != next {
                    self.vel = 0.0; // okraj: zastaviť bez odrazu
                }
                self.pos = c;
                if self.vel.abs() < STOP_VEL {
                    self.pos = self.clamp(self.pos + self.vel * tau);
                    self.vel = 0.0;
                    self.mode = Mode::Idle;
                    return false;
                }
                true
            }
            Mode::Jump { from, from_v, target, t } => {
                let t = t + dt;
                let u = (t / JUMP).min(1.0);
                let (u2, u3) = (u * u, u * u * u);
                let (h00, h10, h01) = (2.0 * u3 - 3.0 * u2 + 1.0, u3 - 2.0 * u2 + u, -2.0 * u3 + 3.0 * u2);
                let (d00, d10, d01) = (6.0 * u2 - 6.0 * u, 3.0 * u2 - 4.0 * u + 1.0, -6.0 * u2 + 6.0 * u);
                self.pos = self.clamp(h00 * from + h10 * JUMP * from_v + h01 * target);
                self.vel = (d00 * from + d10 * JUMP * from_v + d01 * target) / JUMP;
                if u >= 1.0 {
                    self.pos = target;
                    self.vel = 0.0;
                    self.mode = Mode::Idle;
                    return false;
                }
                self.mode = Mode::Jump { from, from_v, target, t };
                true
            }
        }
    }
}

pub struct Smooth {
    map: HashMap<Id, S>,
    pub on: bool,
    clock: Instant,
    // vstup z tejto snímky (raw_input_hook): zúbky kolieska a posun prstami, v bodoch
    notches: f32,
    touch: f32,
    // kedy prišiel naposledy posun prstami – celý riadok hneď po ňom je koniec švihnutia (Windows), nie koliesko
    touch_at: Option<f64>,
}

impl Default for Smooth {
    fn default() -> Self {
        Smooth { map: HashMap::new(), on: false, clock: Instant::now(), notches: 0.0, touch: 0.0, touch_at: None }
    }
}

impl Smooth {
    fn now(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
    }

    // z raw_input_hook: koliesko so zúbkami (celé riadky) → notches; touchpad (zlomky, body) → touch
    pub fn feed(&mut self, raw: &mut egui::RawInput, line: f32, k: f32) {
        let now = self.now();
        self.notches = 0.0; // nová snímka; nespotrebovaný vstup (mimo oblastí) sa nehromadí
        self.touch = 0.0;
        for e in raw.events.iter_mut() {
            if let egui::Event::MouseWheel { unit, delta, modifiers, .. } = e {
                if modifiers.ctrl || modifiers.command {
                    continue;
                }
                let notch = *unit == egui::MouseWheelUnit::Line && delta.x == 0.0 && delta.y.abs() >= 0.99 && (delta.y - delta.y.round()).abs() < 0.02;
                let recent = self.touch_at.is_some_and(|t| now - t < 0.4);
                if notch && recent {
                    // koniec švihnutia na touchpade: Windows pošle ešte celý riadok → zahodiť (inak „zastaví a posunie o riadok“)
                    *delta = egui::Vec2::ZERO;
                } else if notch {
                    self.notches += delta.y * line * k;
                    *delta *= k;
                } else {
                    if *unit == egui::MouseWheelUnit::Line {
                        *unit = egui::MouseWheelUnit::Point;
                        *delta *= line;
                    }
                    *delta *= k;
                    self.touch += delta.y;
                    self.touch_at = Some(now);
                }
            }
        }
    }

    // Pred ScrollArea::show: prevezme koliesko nad oblasťou a vráti posun, ktorý treba nastaviť.
    pub fn begin(&mut self, ctx: &egui::Context, id: Id, layer: egui::LayerId, jump: Option<f32>) -> Option<f32> {
        let now = self.now();
        let on = self.on;
        let s = self.map.entry(id).or_default();
        if let Some(j) = jump {
            if !on {
                s.mode = Mode::Idle;
                s.pos = s.clamp(j);
                return Some(s.pos);
            }
            s.jump(j);
        }
        if on {
            // koliesko len nad touto oblasťou a len keď nad ňou nie je iné okno (nastavenia, ponuka)
            let pos = ctx.input(|i| i.pointer.hover_pos());
            let hovered = pos.map(|p| s.rect.contains(p) && ctx.layer_id_at(p).map(|l| l == layer).unwrap_or(true)).unwrap_or(false);
            if hovered && s.max > 0.0 {
                // egui-ho vyhladený posun sa zahodí (inak by ho ScrollArea pridala ešte raz) – pohyb rieši len S
                ctx.input_mut(|i| {
                    if !(i.modifiers.ctrl || i.modifiers.command) {
                        i.smooth_scroll_delta.y = 0.0;
                    }
                });
                let touch = std::mem::take(&mut self.touch);
                let notches = std::mem::take(&mut self.notches);
                if touch != 0.0 {
                    s.touch(-touch, now);
                } else if notches.abs() > 0.1 {
                    s.wheel(-notches * NOTCH);
                }
            }
        }
        if s.last == 0.0 {
            s.last = now;
        }
        let moving = s.tick(now);
        if moving {
            ctx.request_repaint();
        }
        // inak None: posuvník a klávesy ovláda ScrollArea sama
        (moving || s.touching || jump.is_some()).then_some(s.pos)
    }

    // Po ScrollArea::show: zapamätá si oblasť a rozsah; bez pohybu drží krok so skutočným posunom.
    pub fn end<R>(&mut self, id: Id, out: &egui::scroll_area::ScrollAreaOutput<R>) {
        let s = self.map.entry(id).or_default();
        s.rect = out.inner_rect;
        s.max = (out.content_size.y - out.inner_rect.height()).max(0.0);
        if s.mode == Mode::Idle && !s.touching {
            s.pos = out.state.offset.y;
            s.vel = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> S {
        S { max: 100_000.0, pos: 50_000.0, ..Default::default() }
    }

    // beží snímky po 1/60 s, kým sa hýbe; vráti priebeh polohy
    fn run(s: &mut S, t: &mut f64, frames: usize) -> Vec<f32> {
        let mut out = vec![];
        for _ in 0..frames {
            *t += 1.0 / 60.0;
            s.tick(*t);
            out.push(s.pos);
        }
        out
    }

    #[test]
    fn smooth_one_notch_travels_its_distance() {
        let mut s = area();
        let mut t = 1.0;
        s.last = t;
        s.wheel(60.0);
        let p = run(&mut s, &mut t, 200);
        let d = p.last().unwrap() - 50_000.0;
        assert!((d - 60.0).abs() < 2.0, "distance {d}");
        assert!(p.windows(2).all(|w| w[1] >= w[0]), "monotonic");
        assert_eq!(s.mode, Mode::Idle);
    }

    #[test]
    fn smooth_many_notches_glide_without_jumps() {
        let mut s = area();
        let mut t = 1.0;
        s.last = t;
        let mut all = vec![];
        for _ in 0..10 {
            s.wheel(60.0);
            all.extend(run(&mut s, &mut t, 3)); // zúbok každých 50 ms
        }
        all.extend(run(&mut s, &mut t, 120));
        let steps: Vec<f32> = all.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(steps.iter().all(|d| *d >= 0.0));
        // žiadny skok: rýchlosť sa mení postupne
        let max_step = steps.iter().cloned().fold(0.0, f32::max);
        assert!(max_step < 150.0, "max step {max_step}");
        // po poslednom zúbku dobieha a spomaľuje
        let tail = &steps[30..60];
        assert!(tail.windows(2).all(|w| w[1] <= w[0] + 0.01));
        assert!(tail[0] > 1.0);
        assert_eq!(s.mode, Mode::Idle);
    }

    #[test]
    fn smooth_touch_follows_then_glides() {
        let mut s = area();
        let mut t = 1.0;
        s.last = t;
        for _ in 0..10 {
            t += 1.0 / 120.0;
            s.touch(10.0, t); // 1200 px/s
            s.tick(t);
        }
        let at_lift = s.pos;
        assert!((at_lift - 50_100.0).abs() < 0.01, "1:1 while touching");
        let p = run(&mut s, &mut t, 120);
        let d = p.last().unwrap() - at_lift;
        assert!(d > 300.0, "glides after lift: {d}");
        let steps: Vec<f32> = p.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(steps.iter().skip(4).collect::<Vec<_>>().windows(2).all(|w| *w[1] <= *w[0] + 0.01), "slows down");
    }

    #[test]
    fn smooth_slow_touch_does_not_glide() {
        let mut s = area();
        let mut t = 1.0;
        s.last = t;
        for _ in 0..10 {
            t += 1.0 / 60.0;
            s.touch(0.5, t); // 30 px/s
            s.tick(t);
        }
        let at_lift = s.pos;
        run(&mut s, &mut t, 60);
        assert_eq!(s.pos, at_lift);
    }

    #[test]
    fn smooth_edges_clamp_without_bounce() {
        let mut s = S { max: 1000.0, pos: 950.0, ..Default::default() };
        let mut t = 1.0;
        s.last = t;
        for _ in 0..5 {
            s.wheel(200.0);
        }
        let p = run(&mut s, &mut t, 120);
        assert!(p.iter().all(|y| *y <= 1000.0));
        assert_eq!(*p.last().unwrap(), 1000.0);
        assert_eq!(s.mode, Mode::Idle);
    }

    #[test]
    fn smooth_line_after_swipe_is_dropped() {
        let mut sm = Smooth::default();
        let ev = |unit, y| egui::Event::MouseWheel { unit, delta: egui::vec2(0.0, y), modifiers: egui::Modifiers::NONE, phase: egui::TouchPhase::Move };
        let mut raw = egui::RawInput { events: vec![ev(egui::MouseWheelUnit::Point, -12.5)], ..Default::default() };
        sm.feed(&mut raw, 40.0, 1.0);
        assert!(sm.touch != 0.0);
        let mut raw = egui::RawInput { events: vec![ev(egui::MouseWheelUnit::Line, -1.0)], ..Default::default() };
        sm.feed(&mut raw, 40.0, 1.0);
        assert_eq!(sm.notches, 0.0);
        assert!(matches!(raw.events[0], egui::Event::MouseWheel { delta, .. } if delta == egui::Vec2::ZERO));
    }
}
