// Efekty editora (nastavenie editorFx + anim_on): písané znaky sa jemne objavia, kurzor kĺže, vložený návrh
// sa „dopíše“ svetelným pásom, miesta úryvku raz pulznú, nový riadok sa otvorí, ▶ Run blikne.
// Všetko sa kreslí ako tvary navrch (len viditeľné riadky); prekreslenie sa pýta len kým efekt beží.
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, Galley, Pos2, Rect, Shape, Stroke, StrokeKind};
use std::collections::{HashMap, HashSet};

pub const TYPE_S: f64 = 0.18; // napísaný znak: farba akcentu → farba syntaxe
pub const SWEEP_S: f64 = 0.32; // vložený návrh / úryvok / vložený text
pub const PULSE_S: f64 = 0.5; // miesta úryvku ($1, $2…)
pub const OPEN_S: f64 = 0.26; // nový riadok (Enter)
pub const CARET_S: f64 = 0.08; // kĺzanie kurzora
pub const RUN_S: f64 = 0.55; // záblesk spusteného kódu
pub const FIND_S: f64 = 0.18; // nálezy hľadania
pub const SUG_S: f64 = 0.14; // okno návrhov
pub const TAB_S: f64 = 0.18; // nová karta
pub const PILL_S: f64 = 0.5; // vlnka na ▶ Run
pub const SHIMMER_S: f64 = 6.0; // ako dlho sa hýbe pásik „beží“ (potom stojí – bez prekresľovania)

// pomalý koniec (ease-out cubic)
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

// 0 → 1 za `dur` sekúnd od t0
pub fn prog(now: f64, t0: f64, dur: f64) -> f32 {
    (((now - t0) / dur) as f32).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Type,  // jeden-dva písané znaky
    Sweep, // dlhší vložený text (návrh, úryvok, Emmet, vloženie zo schránky)
}

#[derive(Clone, Debug)]
struct Ins {
    path: String,
    a: usize, // znaky [a, b)
    b: usize,
    t0: f64,
    kind: Kind,
}

#[derive(Clone)]
struct Caret {
    path: String,
    from: Rect, // v súradniciach galley
    to: Rect,
    t0: f64,
    moved: f64, // posledný pohyb – od neho blikanie
}

#[derive(Default)]
pub struct Fx {
    pub on: bool,
    pub now: f64, // čas tejto snímky (pre volania bez ctx, napr. accept_sug)
    ins: Vec<Ins>,
    pulses: Vec<Ins>,
    opened: Vec<(String, usize, f64)>, // nový riadok: cesta, znak jeho začiatku, čas
    caret: Option<Caret>,
    run_flash: Option<(String, Option<(usize, usize)>, f64)>,
    pub run_at: Option<f64>, // vlnka na tlačidle ▶ Run
    find: (String, f64),
    sug_frame: u64,
    sug_t0: f64,
    tabs_known: HashSet<String>,
    tabs_init: bool,
    tab_open: HashMap<String, f64>,
}

// jeden úsek rozsahu na jednom rade galley: logický riadok, stĺpce v ňom a obdĺžnik (súradnice galley)
pub struct Seg {
    pub line: usize,
    pub col_a: usize,
    pub col_b: usize,
    pub abs_a: usize, // prvý znak úseku v celom texte
    pub rect: Rect,
}

// rady galley, ktoré pokrýva rozsah znakov [a, b); len rady v `vis` (súradnice galley), inak sa preskočia
pub fn segs(g: &Galley, a: usize, b: usize, vis: Rect) -> Vec<Seg> {
    let mut out = vec![];
    let (mut off, mut line, mut line_start) = (0usize, 0usize, 0usize);
    for row in &g.rows {
        let n = row.glyphs.len();
        let (rs, re) = (off, off + n + usize::from(row.ends_with_newline));
        if rs > b || (rs == b && a != b) {
            break;
        }
        let rr = row.rect();
        let hit = if a == b { rs <= a && a <= off + n } else { re > a };
        if hit && rr.bottom() >= vis.top() && rr.top() <= vis.bottom() {
            let from = a.max(rs) - rs;
            let to = (b.min(off + n).max(a.max(rs))) - rs;
            let x_of = |k: usize| if k < n { row.pos.x + row.glyphs[k].pos.x } else { row.pos.x + row.glyphs.last().map(|g| g.pos.x + g.advance_width).unwrap_or(0.0) };
            let (x0, mut x1) = (x_of(from), x_of(to));
            if x1 - x0 < 3.0 {
                x1 = x0 + 3.0; // aspoň kúsok (napr. vložený koniec riadka)
            }
            out.push(Seg { line, col_a: rs - line_start + from, col_b: rs - line_start + to, abs_a: rs + from, rect: Rect::from_min_max(pos2(x0, rr.top()), pos2(x1, rr.bottom())) });
        }
        off = re;
        if row.ends_with_newline {
            line += 1;
            line_start = off;
        }
    }
    out
}

impl Fx {
    fn gc(&mut self, now: f64) {
        self.ins.retain(|i| now - i.t0 < if i.kind == Kind::Type { TYPE_S } else { SWEEP_S });
        self.pulses.retain(|i| now - i.t0 < PULSE_S);
        self.opened.retain(|o| now - o.2 < OPEN_S);
        if self.run_flash.as_ref().is_some_and(|r| now - r.2 >= RUN_S) {
            self.run_flash = None;
        }
    }

    // beží niečo v editore tohto súboru (vtedy treba prekresľovať)
    pub fn busy(&mut self, path: &str, now: f64) -> bool {
        self.gc(now);
        self.ins.iter().chain(self.pulses.iter()).any(|i| i.path == path) || self.opened.iter().any(|o| o.0 == path) || self.run_flash.as_ref().is_some_and(|r| r.0 == path)
    }

    // vložený text [s0, c) namiesto výberu [s0, s1): efekty za ním sa posunú, prekryté zmiznú
    pub fn insert(&mut self, path: &str, s0: usize, s1: usize, c: usize, kind: Kind, now: f64) {
        if !self.on || c <= s0 {
            return;
        }
        let d = (c - s0) as i64 - (s1 - s0) as i64;
        for v in [&mut self.ins, &mut self.pulses] {
            v.retain(|i| i.path != path || i.b <= s0 || i.a >= s1.max(s0 + 1) || s1 == s0);
            for i in v.iter_mut().filter(|i| i.path == path && i.a >= s1) {
                i.a = (i.a as i64 + d).max(0) as usize;
                i.b = (i.b as i64 + d).max(0) as usize;
            }
        }
        self.ins.push(Ins { path: path.into(), a: s0, b: c, t0: now, kind });
    }

    pub fn sweep(&mut self, path: &str, a: usize, b: usize, now: f64) {
        if self.on && b > a {
            self.ins.retain(|i| i.path != path || i.b <= a || i.a >= b);
            self.ins.push(Ins { path: path.into(), a, b, t0: now, kind: Kind::Sweep });
        }
    }

    pub fn pulse(&mut self, path: &str, a: usize, b: usize, now: f64) {
        if self.on && b > a {
            self.pulses.push(Ins { path: path.into(), a, b, t0: now, kind: Kind::Sweep });
        }
    }

    pub fn line_open(&mut self, path: &str, at: usize, now: f64) {
        if self.on {
            self.opened.push((path.into(), at, now));
        }
    }

    pub fn run_flash(&mut self, path: &str, sel: Option<(usize, usize)>, now: f64) {
        if self.on {
            self.run_flash = Some((path.into(), sel.map(|(a, b)| (a.min(b), a.max(b))), now));
            self.run_at = Some(now);
        }
    }

    // iná zmena textu (mazanie, Ctrl+Z…): staré miesta už nesedia
    pub fn clear(&mut self, path: &str) {
        self.ins.retain(|i| i.path != path);
        self.pulses.retain(|i| i.path != path);
        self.opened.retain(|o| o.0 != path);
    }

    // napísané znaky po riadkoch: riadok → (stĺpec od, do, postup 0…1) pre farebnú vrstvu
    pub fn line_fades(&self, path: &str, g: &Galley, vis: Rect, now: f64) -> HashMap<usize, Vec<(usize, usize, f32)>> {
        let mut m: HashMap<usize, Vec<(usize, usize, f32)>> = HashMap::new();
        for i in self.ins.iter().filter(|i| i.path == path) {
            if i.kind == Kind::Type {
                let k = ease(prog(now, i.t0, TYPE_S));
                for s in segs(g, i.a, i.b, vis) {
                    if s.col_b > s.col_a {
                        m.entry(s.line).or_default().push((s.col_a, s.col_b, k));
                    }
                }
            } else if i.b - i.a <= 600 {
                // vložený text sa „dopisuje“: znaky za čelom pásu sú ešte bledé, pred ním dobiehajú do farby
                let total = (i.b - i.a) as f32;
                let front = ease((prog(now, i.t0, SWEEP_S) * 1.6).min(1.0)) * total;
                for s in segs(g, i.a, i.b, vis) {
                    for c in s.col_a..s.col_b {
                        let j = (s.abs_a + c - s.col_a - i.a) as f32;
                        let k = ((front - j) / 4.0).clamp(0.0, 1.0);
                        if k < 1.0 {
                            m.entry(s.line).or_default().push((c, c + 1, k));
                        }
                    }
                }
            }
        }
        // úseky po stĺpcoch (append_faded ich ide po poradí)
        for v in m.values_mut() {
            v.sort_by_key(|f| f.0);
        }
        m
    }

    // efekty pod textom: žiara pod novými znakmi, svetelný pás vloženého textu, nový riadok, záblesk Run
    #[allow(clippy::too_many_arguments)]
    pub fn back(&self, path: &str, g: &Galley, origin: Pos2, clip: Rect, lh: f32, accent: Color32, now: f64) -> Vec<Shape> {
        let mut out = vec![];
        let vis = clip.translate(-origin.to_vec2());
        for i in self.ins.iter().filter(|i| i.path == path) {
            match i.kind {
                Kind::Type => {
                    let k = ease(prog(now, i.t0, TYPE_S));
                    for s in segs(g, i.a, i.b, vis) {
                        let r = s.rect.translate(origin.to_vec2()).expand2(vec2(1.5 + 2.0 * k, 0.0)).shrink2(vec2(0.0, 1.0));
                        out.push(Shape::rect_filled(r, CornerRadius::same(4), accent.gamma_multiply(0.26 * (1.0 - k))));
                    }
                }
                Kind::Sweep => {
                    // „dopisovanie“: čelo pásu prejde zľava doprava (cez všetky znaky), za ním svetlo pomaly hasne
                    let p = prog(now, i.t0, SWEEP_S);
                    let front = ease((p * 1.6).min(1.0));
                    let fade = 1.0 - ease(((p - 0.25) / 0.75).max(0.0));
                    // svieti len už „napísaná“ časť (po čelo pásu)
                    let at = i.a + ((i.b - i.a) as f32 * front).round() as usize;
                    for s in segs(g, i.a, at.max(i.a + 1).min(i.b), vis) {
                        let r = s.rect.translate(origin.to_vec2());
                        out.push(Shape::rect_filled(r.expand2(vec2(1.0, -1.0)), CornerRadius::same(3), accent.gamma_multiply(0.16 * fade)));
                    }
                    // čelo pásu: jasnejší prúžok na mieste, kam práve „píše“
                    if p < 0.7 {
                        if let Some(s) = segs(g, at.min(i.b.saturating_sub(1)), at.min(i.b.saturating_sub(1)) + 1, vis).first() {
                            let r = s.rect.translate(origin.to_vec2());
                            let w = 14.0;
                            let x = if at >= i.b { r.right() } else { r.left() };
                            out.push(grad_h(Rect::from_min_max(pos2(x - w, r.top() + 1.0), pos2(x, r.bottom() - 1.0)), Color32::TRANSPARENT, accent.gamma_multiply(0.5 * (1.0 - p / 0.7))));
                        }
                    }
                }
            }
        }
        // nový riadok sa „otvorí“: pás rastie do výšky riadka a hasne
        for o in self.opened.iter().filter(|o| o.0 == path) {
            let k = ease(prog(now, o.2, OPEN_S));
            if let Some(s) = segs(g, o.1, o.1, vis).first() {
                let top = origin.y + s.rect.top() - 2.0;
                let r = Rect::from_x_y_ranges(clip.left()..=clip.right(), top..=top + lh * (0.35 + 0.65 * k));
                out.push(Shape::rect_filled(r, 0.0, accent.gamma_multiply(0.13 * (1.0 - k))));
                out.push(Shape::rect_filled(Rect::from_min_size(pos2(clip.left(), top), vec2(3.0, lh)), CornerRadius::same(1), accent.gamma_multiply(0.7 * (1.0 - k))));
            }
        }
        // ▶ Run: výber blikne; celý súbor = svetlý pás prebehne zhora nadol
        if let Some((_, sel, t0)) = self.run_flash.as_ref().filter(|r| r.0 == path) {
            let p = prog(now, *t0, RUN_S);
            match sel {
                Some((a, b)) if b > a => {
                    let fade = 1.0 - ease(p);
                    let mut lines: Vec<(usize, Rect)> = vec![];
                    for s in segs(g, *a, *b, vis) {
                        match lines.last_mut() {
                            Some(l) if l.0 == s.line => l.1 = l.1.union(s.rect),
                            _ => lines.push((s.line, s.rect)),
                        }
                    }
                    for (_, r) in lines {
                        let r = r.translate(origin.to_vec2());
                        let band = Rect::from_x_y_ranges(clip.left()..=clip.right(), r.top() - 2.0..=r.top() - 2.0 + lh);
                        out.push(Shape::rect_filled(band, 0.0, accent.gamma_multiply(0.16 * fade)));
                    }
                }
                _ => {
                    let y = clip.top() + (clip.height() + lh * 4.0) * ease(p) - lh * 2.0;
                    let h = lh * 3.0;
                    let a = accent.gamma_multiply(0.13 * (1.0 - p * 0.6));
                    out.push(grad_v(Rect::from_x_y_ranges(clip.x_range(), y - h..=y), Color32::TRANSPARENT, a));
                    out.push(Shape::hline(clip.x_range(), y, Stroke::new(1.0, accent.gamma_multiply(0.35 * (1.0 - p)))));
                }
            }
        }
        out
    }

    // miesta úryvku raz pulznú (rámik sa roztiahne a zhasne)
    pub fn front(&self, path: &str, g: &Galley, origin: Pos2, clip: Rect, accent: Color32, now: f64) -> Vec<Shape> {
        let mut out = vec![];
        let vis = clip.translate(-origin.to_vec2());
        for i in self.pulses.iter().filter(|i| i.path == path) {
            let p = prog(now, i.t0, PULSE_S);
            let k = ease(p);
            for s in segs(g, i.a, i.b, vis) {
                let r = s.rect.translate(origin.to_vec2()).shrink2(vec2(0.0, 1.0));
                out.push(Shape::rect_filled(r.expand(1.0), CornerRadius::same(3), accent.gamma_multiply(0.22 * (1.0 - k))));
                out.push(Shape::rect_stroke(r.expand(1.0 + 3.0 * k), CornerRadius::same(4), Stroke::new(1.5, accent.gamma_multiply(0.85 * (1.0 - k))), StrokeKind::Outside));
            }
        }
        out
    }

    // kurzor kĺže na nové miesto (súradnice galley); vráti kreslený obdĺžnik, či sa hýbe a čas posledného pohybu
    pub fn caret(&mut self, path: &str, target: Rect, now: f64) -> (Rect, bool, f64) {
        let c = match self.caret.as_mut() {
            Some(c) if c.path == path => c,
            _ => {
                self.caret = Some(Caret { path: path.into(), from: target, to: target, t0: now, moved: now });
                return (target, false, now);
            }
        };
        if c.to != target {
            let k = ease(prog(now, c.t0, CARET_S));
            let cur = lerp_rect(c.from, c.to, k);
            // veľký skok (iný koniec súboru) – kĺzať len kúsok, inak by kurzor letel cez celú obrazovku
            c.from = if (cur.center() - target.center()).length() > 600.0 { target.translate(vec2(0.0, -8.0 * (target.center().y - cur.center().y).signum())) } else { cur };
            c.to = target;
            c.t0 = now;
            c.moved = now;
        }
        let p = prog(now, c.t0, CARET_S);
        (lerp_rect(c.from, c.to, ease(p)), p < 1.0, c.moved)
    }

    // hľadaný text sa zmenil → nálezy sa objavia postupne
    pub fn find_k(&mut self, q: &str, now: f64) -> f32 {
        if self.find.0 != q {
            self.find = (q.to_string(), now);
        }
        if self.on {
            ease(prog(now, self.find.1, FIND_S))
        } else {
            1.0
        }
    }

    // okno návrhov sa práve otvorilo (v minulej snímke nebolo) → 0…1
    pub fn sug_k(&mut self, frame: u64, now: f64) -> f32 {
        if frame > self.sug_frame + 1 {
            self.sug_t0 = now;
        }
        self.sug_frame = frame;
        if self.on {
            ease(prog(now, self.sug_t0, SUG_S))
        } else {
            1.0
        }
    }

    // nová karta: šírka a viditeľnosť 0 → 1 (karty z prvého vykreslenia a obnovené pri štarte bez efektu)
    pub fn tab_k(&mut self, paths: &[&str], path: &str, now: f64) -> f32 {
        if !self.tabs_init {
            self.tabs_init = true;
            self.tabs_known = paths.iter().map(|p| p.to_string()).collect();
        }
        if self.tabs_known.len() > paths.len() {
            self.tabs_known.retain(|k| paths.contains(&k.as_str()));
            self.tab_open.retain(|k, _| paths.contains(&k.as_str()));
        }
        if !self.tabs_known.contains(path) {
            self.tabs_known.insert(path.to_string());
            if self.on {
                self.tab_open.insert(path.to_string(), now);
            }
        }
        match self.tab_open.get(path) {
            Some(t0) if now - t0 < TAB_S => ease(prog(now, *t0, TAB_S)),
            Some(_) => {
                self.tab_open.remove(path);
                1.0
            }
            None => 1.0,
        }
    }
}

fn lerp_rect(a: Rect, b: Rect, k: f32) -> Rect {
    Rect::from_min_max(a.min.lerp(b.min, k), a.max.lerp(b.max, k))
}

// farba písaného znaku: z akcentu (a priesvitná) do farby syntaxe
pub fn glyph_color(c: Color32, accent: Color32, k: f32) -> Color32 {
    accent.lerp_to_gamma(c, k).gamma_multiply(0.3 + 0.7 * k)
}

// obdĺžnik s prechodom zľava doprava / zhora nadol (mesh s farbami vrcholov)
pub fn grad_h(r: Rect, left: Color32, right: Color32) -> Shape {
    grad(r, [left, right, right, left])
}

pub fn grad_v(r: Rect, top: Color32, bottom: Color32) -> Shape {
    grad(r, [top, top, bottom, bottom])
}

fn grad(r: Rect, c: [Color32; 4]) -> Shape {
    let mut m = egui::Mesh::default();
    for (p, col) in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].into_iter().zip(c) {
        m.colored_vertex(p, col);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    Shape::mesh(m)
}

// farebná vrstva riadka: úsek [from, to) s napísanými znakmi (bajty, postup) zafarbenými podľa glyph_color
pub fn append_faded(job: &mut egui::text::LayoutJob, text: &str, from: usize, to: usize, toks: &[usize], fmt: &egui::TextFormat, fades: &[(usize, usize, f32)], accent: Color32) {
    let mut a = from;
    for &(fa, fb, k) in fades {
        let (fa, fb) = (fa.max(a), fb.min(to));
        if fa >= fb {
            continue;
        }
        crate::colors::append(job, text, a, fa, toks, fmt);
        crate::colors::append(job, text, fa, fb, toks, &egui::TextFormat { color: glyph_color(fmt.color, accent, k), ..fmt.clone() });
        a = fb;
    }
    crate::colors::append(job, text, a, to, toks, fmt);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fx_insert_shifts_and_expires() {
        let mut f = Fx { on: true, ..Default::default() };
        f.insert("a", 5, 5, 6, Kind::Type, 0.0);
        // písanie pred ním posunie starý efekt
        f.insert("a", 2, 2, 3, Kind::Type, 0.01);
        assert_eq!((f.ins[0].a, f.ins[0].b), (6, 7));
        // písanie za ním ho nechá
        f.insert("a", 7, 7, 8, Kind::Type, 0.02);
        assert_eq!((f.ins[0].a, f.ins[0].b), (6, 7));
        assert!(f.busy("a", 0.05));
        assert!(!f.busy("b", 0.05));
        assert!(!f.busy("a", 1.0));
        // vypnuté efekty nič nepridajú
        let mut off = Fx::default();
        off.insert("a", 0, 0, 1, Kind::Type, 0.0);
        off.sweep("a", 0, 4, 0.0);
        assert!(!off.busy("a", 0.0));
    }

    #[test]
    fn fx_easing_and_caret() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert!(ease(0.5) > 0.5);
        let mut f = Fx { on: true, ..Default::default() };
        let r0 = Rect::from_min_size(pos2(0.0, 0.0), vec2(1.0, 20.0));
        let r1 = r0.translate(vec2(80.0, 0.0));
        assert_eq!(f.caret("a", r0, 0.0).0, r0);
        let (mid, moving, _) = f.caret("a", r1, 1.0);
        assert!(moving && mid.left() < 1.0);
        let (half, _, _) = f.caret("a", r1, 1.0 + CARET_S / 2.0);
        assert!(half.left() > 10.0 && half.left() < 80.0);
        let (end, moving, _) = f.caret("a", r1, 2.0);
        assert!(!moving && end == r1);
    }

    #[test]
    fn fx_tabs_open() {
        let mut f = Fx { on: true, ..Default::default() };
        assert_eq!(f.tab_k(&["x"], "x", 0.0), 1.0); // prvé vykreslenie bez efektu
        assert!(f.tab_k(&["x", "y"], "y", 1.0) < 0.1);
        assert_eq!(f.tab_k(&["x", "y"], "y", 2.0), 1.0);
    }
}
