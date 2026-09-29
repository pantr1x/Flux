// Plynulé posúvanie kolieskom – ako springStep() v Electron Fluxe: koliesko posúva cieľ a pohľad k nemu
// ide ako kriticky tlmená pružina (w = 0.016 / ms). Rýchlosť sa pri ďalšom zúbku kolieska neláme.
use eframe::egui::{self, Id, Rect};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct S {
    pos: f32,
    vel: f32,
    target: f32,
    active: bool,
    rect: Rect,
    max: f32,
}

impl Default for S {
    fn default() -> Self {
        S { pos: 0.0, vel: 0.0, target: 0.0, active: false, rect: Rect::NOTHING, max: 0.0 }
    }
}

#[derive(Default)]
pub struct Smooth {
    map: HashMap<Id, S>,
    pub on: bool,
}

const W: f32 = 16.0; // 0.016 / ms

impl Smooth {
    // Pred ScrollArea::show: prevezme koliesko nad oblasťou a vráti posun, ktorý treba nastaviť.
    pub fn begin(&mut self, ctx: &egui::Context, id: Id, layer: egui::LayerId, jump: Option<f32>) -> Option<f32> {
        let s = self.map.entry(id).or_default();
        if let Some(j) = jump {
            // skok (minimapa, hľadanie) – plynulo, ak sú animácie zapnuté
            if !s.active {
                s.vel = 0.0;
            }
            s.target = j.clamp(0.0, s.max.max(0.0));
            s.active = self.on;
            if !self.on {
                s.pos = s.target;
                return Some(s.pos);
            }
        }
        if self.on {
            // koliesko len nad touto oblasťou a len keď nad ňou nie je iné okno (nastavenia, ponuka)
            let pos = ctx.input(|i| i.pointer.hover_pos());
            let hovered = pos.map(|p| s.rect.contains(p) && ctx.layer_id_at(p).map(|l| l == layer).unwrap_or(true)).unwrap_or(false);
            if hovered && s.max > 0.0 {
                let dy = ctx.input_mut(|i| {
                    let d = i.smooth_scroll_delta.y;
                    if i.modifiers.ctrl || i.modifiers.command {
                        return 0.0;
                    }
                    i.smooth_scroll_delta.y = 0.0;
                    d
                });
                if dy.abs() > 0.1 {
                    if !s.active {
                        s.vel = 0.0;
                    }
                    // koliesko: o trochu väčší krok, nech je to podobné prehliadaču
                    s.target = (s.target - dy * 1.15).clamp(0.0, s.max);
                    s.active = true;
                }
            }
        }
        if !s.active {
            return None;
        }
        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.05);
        // presné riešenie kriticky tlmenej pružiny pre krok dt
        let d = s.pos - s.target;
        let e = (-W * dt).exp();
        let nd = (d + (s.vel + W * d) * dt) * e;
        let nv = (s.vel - W * (s.vel + W * d) * dt) * e;
        s.pos = s.target + nd;
        s.vel = nv;
        if nd.abs() < 0.4 && nv.abs() < 4.0 {
            s.pos = s.target;
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
