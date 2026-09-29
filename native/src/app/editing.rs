// Úpravy textu ako v Monacu: Tab / Shift+Tab (odsadenie riadkov), Ctrl+/ (komentár), automatické
// zatváranie zátvoriek a úvodzoviek, Ctrl+F / Ctrl+H (hľadanie a nahrádzanie) so zvýraznením nálezov.
use super::App;
use crate::i18n::{t, tf};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind};

pub struct Find {
    pub q: String,
    pub repl: String,
    pub replace: bool,
    pub idx: usize,
    pub focus: bool,
}

// komentár podľa jazyka (comment tokens z editorExtras.js)
fn comment_of(ext: &str) -> (&'static str, &'static str) {
    match ext {
        "py" | "pyw" | "rb" | "sh" | "bash" | "r" | "pl" | "toml" | "yml" | "yaml" | "ps1" | "jl" => ("# ", ""),
        "lua" | "sql" => ("-- ", ""),
        "html" | "htm" | "xml" | "md" | "svg" => ("<!-- ", " -->"),
        "css" | "scss" => ("/* ", " */"),
        "bat" | "cmd" => ("REM ", ""),
        _ => ("// ", ""),
    }
}

// char index ↔ byte index
fn byte_at(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}
fn char_at(s: &str, bi: usize) -> usize {
    s[..bi.min(s.len())].chars().count()
}

// riadky (začiatok bajtu každého riadku), ktorých sa týka výber
fn line_starts(text: &str, a: usize, b: usize) -> Vec<usize> {
    let (ab, bb) = (byte_at(text, a.min(b)), byte_at(text, a.max(b)));
    let first = text[..ab].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let mut out = vec![first];
    // posledný riadok nezahŕňame, ak výber končí na jeho začiatku
    let end = if bb > ab && text[..bb].ends_with('\n') { bb - 1 } else { bb };
    for (i, c) in text[first..end].char_indices() {
        if c == '\n' {
            out.push(first + i + 1);
        }
    }
    out
}

// Tab / Shift+Tab: bez výberu vloží 4 medzery, s výberom odsadí/vráti celé riadky. Vráti nový výber.
pub fn indent(text: &mut String, a: usize, b: usize, back: bool) -> (usize, usize) {
    if !back && a == b {
        let bi = byte_at(text, a);
        text.insert_str(bi, "    ");
        return (a + 4, a + 4);
    }
    let starts = line_starts(text, a, b);
    let (mut lo, mut hi) = (a.min(b), a.max(b));
    let first_line_char = char_at(text, starts[0]);
    for (n, st) in starts.iter().enumerate().rev() {
        let st = *st;
        if back {
            let spaces = text[st..].chars().take(4).take_while(|c| *c == ' ').count();
            let tab = text[st..].starts_with('\t');
            let remove = if spaces > 0 {
                spaces
            } else if tab {
                1
            } else {
                0
            };
            if remove > 0 {
                text.replace_range(st..st + remove, "");
                hi = hi.saturating_sub(remove);
                if n == 0 {
                    lo = lo.saturating_sub(remove.min(lo - first_line_char));
                }
            }
        } else {
            text.insert_str(st, "    ");
            hi += 4;
            if n == 0 {
                lo += 4;
            }
        }
    }
    if a <= b {
        (lo, hi)
    } else {
        (hi, lo)
    }
}

// Ctrl+/: zakomentuje alebo odkomentuje riadky výberu
pub fn toggle_comment(text: &mut String, a: usize, b: usize, ext: &str) -> (usize, usize) {
    let (open, close) = comment_of(ext);
    let starts = line_starts(text, a, b);
    let line_of = |t: &str, st: usize| t[st..].split('\n').next().unwrap_or("").to_string();
    let all = starts.iter().all(|st| {
        let l = line_of(text, *st);
        l.trim().is_empty() || l.trim_start().starts_with(open.trim_end())
    });
    let mut delta: i64 = 0;
    for st in starts.iter().rev() {
        let l = line_of(text, *st);
        if l.trim().is_empty() {
            continue;
        }
        let ind = l.len() - l.trim_start().len();
        let body_start = st + ind;
        if all {
            let o = if text[body_start..].starts_with(open) { open.len() } else { open.trim_end().len() };
            text.replace_range(body_start..body_start + o, "");
            delta -= o as i64;
            if !close.is_empty() {
                let end = st + line_of(text, *st).len();
                let c = if text[..end].ends_with(close) {
                    close.len()
                } else if text[..end].ends_with(close.trim_start()) {
                    close.trim_start().len()
                } else {
                    0
                };
                text.replace_range(end - c..end, "");
                delta -= c as i64;
            }
        } else {
            if !close.is_empty() {
                let end = st + l.len();
                text.insert_str(end, close);
                delta += close.len() as i64;
            }
            text.insert_str(body_start, open);
            delta += open.len() as i64;
        }
    }
    let lo = char_at(text, starts[0]);
    let hi = (a.max(b) as i64 + delta).max(lo as i64) as usize;
    (lo, hi.min(text.chars().count()))
}

// automatické zatváranie: po napísaní ( [ { " ' vloží pár, ak za kurzorom nie je písmeno/číslo
pub fn auto_close(text: &mut String, cursor: usize, typed: char) -> bool {
    let close = match typed {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '"' => '"',
        '\'' => '\'',
        _ => return false,
    };
    let bi = byte_at(text, cursor);
    let next = text[bi..].chars().next();
    if next.map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false) {
        return false;
    }
    // pri úvodzovkách nie za písmenom (napr. „don't“)
    if (typed == '"' || typed == '\'') && cursor >= 2 {
        let prev = text[..byte_at(text, cursor - 1)].chars().last();
        if prev.map(|c| c.is_alphanumeric()).unwrap_or(false) {
            return false;
        }
    }
    text.insert(bi, close);
    true
}

// nálezy (bez ohľadu na veľkosť písmen) ako rozsahy znakov
pub fn matches(text: &str, q: &str) -> Vec<(usize, usize)> {
    if q.is_empty() {
        return vec![];
    }
    let lt = text.to_lowercase();
    let lq = q.to_lowercase();
    if lt.len() != text.len() {
        return vec![]; // zriedkavé znaky, ktoré menia dĺžku – bez zvýraznenia
    }
    let n = q.chars().count();
    lt.match_indices(&lq)
        .take(5000)
        .map(|(bi, _)| {
            let c = char_at(text, bi);
            (c, c + n)
        })
        .collect()
}

impl App {
    // lišta hľadania vpravo hore v editore (ako find widget v Monacu)
    pub(super) fn find_bar(&mut self, ui: &mut egui::Ui, ed: Rect, count: usize) -> Option<&'static str> {
        let p = self.pal;
        let f = self.find.as_mut()?;
        let w = 380.0;
        let h = if f.replace { 70.0 } else { 38.0 };
        let r = Rect::from_min_size(pos2(ed.right() - w - 18.0, ed.top() + 6.0), vec2(w, h));
        ui.painter().add(egui::Shadow { offset: [0, 6], blur: 18, spread: 0, color: egui::Color32::from_black_alpha(90) }.as_shape(r, CornerRadius::same(10)));
        ui.painter().rect_filled(r, CornerRadius::same(10), p.solid);
        ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let _ = ui.interact(r, ui.id().with("find-bg"), Sense::click());
        let mut act = None;
        let mut qc = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(r.left() + 12.0, r.top() + 9.0), pos2(r.right() - 150.0, r.top() + 30.0))));
        let te = qc.add(egui::TextEdit::singleline(&mut f.q).hint_text(t("Find")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(13.0)));
        if f.focus {
            te.request_focus();
            f.focus = false;
        }
        if te.changed() {
            f.idx = 0;
            act = Some("goto");
        }
        if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            act = Some(if ui.input(|i| i.modifiers.shift) { "prev" } else { "next" });
            te.request_focus();
        }
        let label = if f.q.is_empty() {
            String::new()
        } else if count == 0 {
            t("No results")
        } else {
            tf("{i} of {n}", &[("i", &(f.idx % count + 1).to_string()), ("n", &count.to_string())])
        };
        ui.painter().text(pos2(r.right() - 140.0, r.top() + 19.0), Align2::LEFT_CENTER, label, theme::ui(11.5), if count == 0 && !f.q.is_empty() { p.red } else { p.text3 });
        let y = r.top() + 19.0;
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(r.right() - 70.0, y), vec2(24.0, 24.0)), "arrowLeft", 13.0, &p, count > 0).on_hover_text("Shift+Enter").clicked() {
            act = Some("prev");
        }
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(r.right() - 44.0, y), vec2(24.0, 24.0)), "arrowRight", 13.0, &p, count > 0).on_hover_text("Enter").clicked() {
            act = Some("next");
        }
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(r.right() - 18.0, y), vec2(24.0, 24.0)), "x", 13.0, &p, true).on_hover_text("Esc").clicked()
            || ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            act = Some("close");
        }
        if f.replace {
            ui.painter().hline(r.left() + 8.0..=r.right() - 8.0, r.top() + 37.0, Stroke::new(1.0, p.line));
            let mut rc = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(r.left() + 12.0, r.top() + 42.0), pos2(r.right() - 150.0, r.top() + 63.0))));
            rc.add(egui::TextEdit::singleline(&mut f.repl).hint_text(t("Replace")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(13.0)));
            let mut bc = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_max(pos2(r.right() - 146.0, r.top() + 40.0), pos2(r.right() - 8.0, r.top() + 66.0)))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            bc.spacing_mut().item_spacing.x = 4.0;
            if widgets::button(&mut bc, None, &t("All"), p.card2, p.text, 24.0, &p).on_hover_text(t("Replace all")).clicked() {
                act = Some("replace-all");
            }
            if widgets::button(&mut bc, None, &t("Replace"), p.card2, p.text, 24.0, &p).clicked() {
                act = Some("replace");
            }
        }
        act
    }
}
