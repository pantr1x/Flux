// Plynulé posúvanie kolieskom: koliesko posúva cieľ a pohľad k nemu dôjde za pevný čas (DUR) po krivke,
// ktorá nadväzuje na aktuálnu rýchlosť (Hermite) – rýchlosť sa pri ďalšom zúbku neláme a v cieli presne
// skončí. Pružina predtým pri dlhom skrolovaní dobiehala pomalým chvostom („zastaví a ešte posunie“).
use eframe::egui::{self, Id, Rect};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct S {
    pos: f32,
    vel: f32,
    target: f32,
    from: f32,   // začiatok aktuálneho úseku
    from_v: f32, // rýchlosť na jeho začiatku
    t: f32,      // čas od začiatku úseku (s)
    active: bool,
    rect: Rect,
    max: f32,
}

impl Default for S {
    fn default() -> Self {
        S { pos: 0.0, vel: 0.0, target: 0.0, from: 0.0, from_v: 0.0, t: 0.0, active: false, rect: Rect::NOTHING, max: 0.0 }
    }
}

#[derive(Default)]
pub struct Smooth {
    map: HashMap<Id, S>,
    pub on: bool,
    // zúbky kolieska myši z tejto snímky (surové, v bodoch) – vyhladí ich len pružina, nie aj egui
    pub notches: f32,
    // kedy prišiel naposledy presný posun (touchpad, kolieska bez zúbkov): ten ide 1:1, zotrvačnosť má systém
    pub precise_at: Option<std::time::Instant>,
}

impl Smooth {
    // z raw_input_hook: koliesko so zúbkami (celé riadky) → notches; touchpad → body (egui ho nevyhladzuje)
    pub fn feed(&mut self, raw: &mut egui::RawInput, line: f32, k: f32) {
        self.notches = 0.0; // nová snímka; nespotrebované zúbky (mimo oblastí) sa nehromadia
        for e in raw.events.iter_mut() {
            if let egui::Event::MouseWheel { unit, delta, modifiers, .. } = e {
                if modifiers.ctrl || modifiers.command {
                    continue;
                }
                let notch = *unit == egui::MouseWheelUnit::Line && delta.x == 0.0 && delta.y.abs() >= 0.99 && (delta.y - delta.y.round()).abs() < 0.02;
                if notch {
                    self.notches += delta.y * line * k;
                    *delta *= k;
                } else {
                    if *unit == egui::MouseWheelUnit::Line {
                        *unit = egui::MouseWheelUnit::Point;
                        *delta *= line;
                    }
                    *delta *= k;
                    self.precise_at = Some(std::time::Instant::now());
                }
            }
        }
    }
}

const DUR: f32 = 0.22; // s – dĺžka dobehnutia po zúbku

// nový cieľ: úsek začína z aktuálnej polohy a rýchlosti
fn retarget(s: &mut S, target: f32) {
    if !s.active {
        s.vel = 0.0;
    }
    s.from = s.pos;
    s.from_v = s.vel;
    s.t = 0.0;
    s.target = target.clamp(0.0, s.max.max(0.0));
    s.active = true;
}

impl Smooth {
    // Pred ScrollArea::show: prevezme koliesko nad oblasťou a vráti posun, ktorý treba nastaviť.
    pub fn begin(&mut self, ctx: &egui::Context, id: Id, layer: egui::LayerId, jump: Option<f32>) -> Option<f32> {
        let s = self.map.entry(id).or_default();
        if let Some(j) = jump {
            // skok (minimapa, hľadanie) – plynulo, ak sú animácie zapnuté
            if !self.on {
                s.target = j.clamp(0.0, s.max.max(0.0));
                s.pos = s.target;
                s.active = false;
                return Some(s.pos);
            }
            retarget(s, j);
        }
        if self.on {
            // koliesko len nad touto oblasťou a len keď nad ňou nie je iné okno (nastavenia, ponuka)
            let pos = ctx.input(|i| i.pointer.hover_pos());
            let hovered = pos.map(|p| s.rect.contains(p) && ctx.layer_id_at(p).map(|l| l == layer).unwrap_or(true)).unwrap_or(false);
            if hovered && s.max > 0.0 {
                // egui-ho vyhladený posun si berieme celý (inak by ho ScrollArea pridala ešte raz)
                let dy = ctx.input_mut(|i| {
                    if i.modifiers.ctrl || i.modifiers.command {
                        return 0.0;
                    }
                    std::mem::take(&mut i.smooth_scroll_delta.y)
                });
                let precise = self.precise_at.is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(400));
                let notches = std::mem::take(&mut self.notches);
                if precise && notches == 0.0 {
                    // touchpad: presne pod prstom, bez vlastnej zotrvačnosti (tú posiela Windows)
                    if dy.abs() > 0.0 {
                        s.target = (s.target - dy).clamp(0.0, s.max);
                        s.pos = s.target;
                        s.vel = 0.0;
                        s.active = false;
                        return Some(s.pos);
                    }
                } else if notches.abs() > 0.1 {
                    // koliesko: surový zúbok (egui-ho rozmazaná verzia sa zahodí) → jedna pružina
                    let base = if s.active { s.target } else { s.pos };
                    retarget(s, base - notches * 1.5);
                }
            }
        }
        if !s.active {
            return None;
        }
        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.05);
        s.t += dt;
        let u = (s.t / DUR).min(1.0);
        // kubická Hermitova krivka: z (from, from_v) do (target, 0) za DUR
        let (u2, u3) = (u * u, u * u * u);
        let (h00, h10, h01) = (2.0 * u3 - 3.0 * u2 + 1.0, u3 - 2.0 * u2 + u, -2.0 * u3 + 3.0 * u2);
        let (d00, d10, d01) = (6.0 * u2 - 6.0 * u, 3.0 * u2 - 4.0 * u + 1.0, -6.0 * u2 + 6.0 * u);
        s.pos = (h00 * s.from + h10 * DUR * s.from_v + h01 * s.target).clamp(0.0, s.max.max(0.0));
        s.vel = (d00 * s.from + d10 * DUR * s.from_v + d01 * s.target) / DUR;
        if u >= 1.0 {
            s.pos = s.target;
            s.vel = 0.0;
            s.active = false;
        } else {
            ctx.request_repaint();
        }
        Some(s.pos)
    }

    // Po ScrollArea::show: zapamätá si oblasť a rozsah; bez animácie drží krok so skutočným posunom.
    pub fn end<R>(&mut self, id: Id, out: &egui::scroll_area::ScrollAreaOutput<R>) {
        let s = self.map.entry(id).or_default();
        s.rect = out.inner_rect;
        s.max = (out.content_size.y - out.inner_rect.height()).max(0.0);
        if !s.active {
            s.pos = out.state.offset.y;
            s.target = s.pos;
            s.vel = 0.0;
        }
    }
}
