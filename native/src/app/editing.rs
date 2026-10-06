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

// Enter: nový riadok s odsadením aktuálneho riadka (ako VS Code). Za otvorenou zátvorkou / značkou / „:“ v Pythone
// odsadí o krok navyše; medzi `{|}` alebo `<ul>|</ul>` rozdelí na tri riadky. Riadok len z medzier sa vyprázdni.
pub fn enter(text: &mut String, a: usize, b: usize, ext: &str) -> (usize, usize) {
    const STEP: &str = "    ";
    let (lo, hi) = (a.min(b), a.max(b));
    let (blo, bhi) = (byte_at(text, lo), byte_at(text, hi));
    let ls = text[..blo].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let before_line = text[ls..blo].to_string();
    let indent: String = before_line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let blank = before_line.trim().is_empty();
    let before = before_line.trim_end();
    let line_end = text[bhi..].find('\n').map(|i| bhi + i).unwrap_or(text.len());
    let after_t = text[bhi..line_end].trim_start().to_string();
    let html = matches!(ext, "html" | "htm" | "xml" | "vue" | "svelte" | "php" | "svg");
    let brace = before.chars().last().filter(|c| "{([".contains(*c)).map(|c| match c {
        '{' => '}',
        '(' => ')',
        _ => ']',
    });
    let open_tag = html && !blank && before.ends_with('>') && !before.ends_with("/>") && {
        let seg = &before[before.rfind('<').unwrap_or(0)..];
        let name: String = seg.chars().skip(1).take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ':').collect();
        seg.chars().nth(1).is_some_and(|c| c.is_ascii_alphabetic()) && !crate::complete::VOID.contains(&name.to_ascii_lowercase().as_str())
    };
    let opens = !blank && (brace.is_some() || open_tag || (matches!(ext, "py" | "pyw") && before.ends_with(':')));
    let closes_here = opens && ((open_tag && after_t.starts_with("</")) || brace.is_some_and(|c| after_t.starts_with(c)));
    let mid = format!("\n{indent}{}", if opens { STEP } else { "" });
    let (from, ins) = if closes_here { (if blank { ls } else { blo }, format!("{mid}\n{indent}")) } else { (if blank { ls } else { blo }, mid.clone()) };
    // kurzor: na konci stredného riadka (pri rozdelení pred zatvárací riadok)
    let caret_byte = from + mid.len();
    // riadok len z medzier sa vyprázdni: odstráni sa od začiatku riadka, odsadenie ide do nového riadka
    let tail_from = if closes_here { bhi } else { bhi };
    let tail = text[tail_from..].to_string();
    let mut out = String::with_capacity(text.len() + ins.len());
    out.push_str(&text[..from]);
    out.push_str(&ins);
    let trimmed_tail = if closes_here { tail.trim_start_matches([' ', '\t']).to_string() } else { tail };
    out.push_str(&trimmed_tail);
    *text = out;
    let c = text[..caret_byte].chars().count();
    (c, c)
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
    // pri úvodzovkách nie za písmenom (napr. „don't“); predpona reťazca (f"…", r'…', b"…") sa zatvára
    if (typed == '"' || typed == '\'') && cursor >= 2 {
        let head = &text[..byte_at(text, cursor - 1)];
        if head.chars().last().map(|c| c.is_alphanumeric()).unwrap_or(false) {
            let word: String = head.chars().rev().take_while(|c| c.is_alphanumeric()).collect::<Vec<_>>().into_iter().rev().collect();
            let prefix = matches!(word.to_lowercase().as_str(), "f" | "r" | "b" | "u" | "fr" | "rf" | "br" | "rb" | "l" | "u8" | "ur");
            if !prefix {
                return false;
            }
        }
    }
    text.insert(bi, close);
    true
}

// HTML: po napísaní „>“ za <div …> doplní </div> za kurzor (nie pri </x>, <x/>, <!…> a prázdnych značkách ako <br>)
pub fn close_tag(text: &mut String, cursor: usize) -> bool {
    let bi = byte_at(text, cursor);
    let head = &text[..bi];
    if !head.ends_with('>') {
        return false;
    }
    let Some(lt) = head.rfind('<') else { return false };
    let inner = &head[lt + 1..head.len() - 1];
    if inner.starts_with('/') || inner.starts_with('!') || inner.starts_with('?') || inner.ends_with('/') || inner.contains('>') {
        return false;
    }
    let name: String = inner.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    if name.is_empty() || crate::complete::VOID.contains(&name.to_lowercase().as_str()) {
        return false;
    }
    let close = format!("</{name}>");
    if text[bi..].starts_with(&close) {
        return false;
    }
    text.insert_str(bi, &close);
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

impl App {
    // okno výberu farby pri štvorčeku v kóde; každá zmena sa hneď zapíše do textu v rovnakom zápise
    pub(super) fn color_pick_ui(&mut self, ctx: &egui::Context) {
        let p = self.pal;
        let Some(cp) = self.color_pick.as_ref() else { return };
        if self.tabs.get(self.active).map(|t| t.path.as_str()) != Some(cp.path.as_str()) {
            self.color_pick = None;
            return;
        }
        let (anchor, opened, mut color, fmt) = (cp.anchor, cp.opened, cp.color, cp.fmt);
        let h = 330.0;
        let screen = ctx.content_rect();
        let pos = if anchor.top() - h - 8.0 > screen.top() { pos2(anchor.left() - 8.0, anchor.top() - h - 8.0) } else { pos2(anchor.left() - 8.0, anchor.bottom() + 8.0) };
        let mut changed = false;
        let mut close = false;
        let hex_id = egui::Id::new("color-pick-hex");
        let area = egui::Area::new(egui::Id::new("color-pick")).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
            egui::Frame::NONE.fill(p.solid).stroke(Stroke::new(1.0, p.line_strong)).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                ui.set_width(232.0);
                // väčšie pole s farbami ako v prehliadači
                ui.spacing_mut().slider_width = 232.0;
                ui.label(egui::RichText::new(t("Pick a color")).font(theme::bold(13.0)).color(p.text));
                ui.add_space(6.0);
                changed |= egui::widgets::color_picker::color_picker_color32(ui, &mut color, egui::widgets::color_picker::Alpha::OnlyBlend);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    // hex / pôvodný zápis na úpravu rukou
                    let mut s = ui.data_mut(|d| d.get_temp::<String>(hex_id)).unwrap_or_else(|| crate::colors::format(color, fmt));
                    let r = ui.add(egui::TextEdit::singleline(&mut s).font(theme::mono(12.5)).desired_width(150.0));
                    if r.changed() {
                        if let Some(c) = crate::colors::parse(&s) {
                            color = c;
                            changed = true;
                        }
                        ui.data_mut(|d| d.insert_temp(hex_id, s));
                    } else if !r.has_focus() {
                        ui.data_mut(|d| d.remove::<String>(hex_id));
                    }
                    if widgets::button(ui, None, &t("Done"), p.accent, p.accent_fg, 28.0, &p).clicked() {
                        close = true;
                    }
                });
            });
        });
        let now = ctx.input(|i| i.time);
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || (now - opened > 0.25 && area.response.clicked_elsewhere()) {
            close = true;
        }
        if changed {
            let new = crate::colors::format(color, fmt);
            if let Some(cp) = self.color_pick.as_mut() {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.path == cp.path) {
                    let b = |c: usize| tab.text.char_indices().nth(c).map(|(b, _)| b).unwrap_or(tab.text.len());
                    let (a, e) = (b(cp.start), b(cp.start + cp.len));
                    tab.text.replace_range(a..e, &new);
                    cp.len = new.chars().count();
                    cp.color = color;
                    self.last_edit = Some(std::time::Instant::now());
                }
            }
        }
        if close {
            self.color_pick = None;
            ctx.data_mut(|d| d.remove::<String>(hex_id));
        }
    }
}

// reťazec pod kurzorom myši: obsah úvodzoviek, url(…) alebo holá adresa http(s)://; (začiatok, koniec v znakoch, text)
pub fn token_at(text: &str, idx: usize) -> Option<(usize, usize, String)> {
    let cs: Vec<char> = text.chars().collect();
    if idx > cs.len() {
        return None;
    }
    let ls = cs[..idx].iter().rposition(|c| *c == '\n').map(|p| p + 1).unwrap_or(0);
    let le = cs[idx..].iter().position(|c| *c == '\n').map(|p| p + idx).unwrap_or(cs.len());
    // v úvodzovkách
    let mut open: Option<(char, usize)> = None;
    for (k, &c) in cs.iter().enumerate().take(le).skip(ls) {
        match open {
            Some((q, s)) if c == q => {
                if idx > s && idx <= k {
                    let t: String = cs[s + 1..k].iter().collect();
                    return (!t.trim().is_empty()).then_some((s + 1, k, t));
                }
                open = None;
            }
            None if c == '"' || c == '\'' || c == '`' => open = Some((c, k)),
            _ => {}
        }
    }
    // url(…) bez úvodzoviek
    let line: String = cs[ls..le].iter().collect();
    let col = idx - ls;
    let mut from = 0;
    while let Some(p) = line[from..].find("url(") {
        let s = from + p + 4;
        let sc = line[..s].chars().count();
        if let Some(e) = line[s..].find(')') {
            let ec = line[..s + e].chars().count();
            if col >= sc && col <= ec {
                return Some((ls + sc, ls + ec, line[s..s + e].trim().to_string()));
            }
            from = s + e;
        } else {
            break;
        }
    }
    // holá adresa
    let is_url_char = |c: char| !c.is_whitespace() && !matches!(c, '"' | '\'' | '<' | '>' | '(' | ')' | '`');
    let mut a = idx;
    while a > ls && is_url_char(cs[a - 1]) {
        a -= 1;
    }
    let mut b = idx;
    while b < le && is_url_char(cs[b]) {
        b += 1;
    }
    let w: String = cs[a..b].iter().collect();
    // „href=https://…“: adresa začína až pri http
    let off = w.find("https://").or_else(|| w.find("http://"))?;
    let a = a + w[..off].chars().count();
    (idx >= a).then(|| (a, b, w[off..].trim_end_matches(['.', ',', ';']).to_string()))
}

// kam vedie text z kódu: adresa, alebo existujúci súbor/priečinok (k súboru, k projektu, „/…“ od koreňa projektu)
pub fn link_target(raw: &str, file_dir: &str, root: Option<&str>) -> Option<String> {
    let raw = raw.trim();
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return (!raw.contains(char::is_whitespace)).then(|| raw.to_string());
    }
    if raw.is_empty() || raw.len() > 300 || raw.contains('\n') || !raw.chars().any(|c| c.is_alphanumeric()) || raw.contains("://") || raw.starts_with('#') {
        return None;
    }
    let clean = raw.split(['?', '#']).next().unwrap_or(raw);
    let mut cands: Vec<std::path::PathBuf> = vec![];
    let p = std::path::Path::new(clean);
    if p.is_absolute() && !clean.starts_with('/') || cfg!(not(windows)) && p.is_absolute() {
        cands.push(p.to_path_buf());
    }
    if let Some(r) = root {
        if let Some(rest) = clean.strip_prefix('/') {
            cands.push(std::path::Path::new(r).join(rest));
        }
    }
    cands.push(std::path::Path::new(file_dir).join(clean));
    if let Some(r) = root {
        cands.push(std::path::Path::new(r).join(clean.trim_start_matches('/')));
    }
    cands.into_iter().find(|c| c.exists()).map(|c| c.to_string_lossy().to_string())
}

#[cfg(test)]
mod tag_tests {
    #[test]
    fn complete_close_tag() {
        let mut t = String::from("<div class=\"a\">");
        assert!(super::close_tag(&mut t, 15));
        assert_eq!(t, "<div class=\"a\"></div>");
        let mut br = String::from("<br>");
        assert!(!super::close_tag(&mut br, 4));
        let mut end = String::from("</p>");
        assert!(!super::close_tag(&mut end, 4));
    }

    #[test]
    fn link_token_and_target() {
        let t = "<img src=\"logo.png\" alt=\"x\">\n<a href=https://flux.dev/a>go</a> a { background: url(img/b.png) }";
        let i = t.find("logo").unwrap() + 2;
        assert_eq!(super::token_at(t, i).map(|x| x.2), Some("logo.png".into()));
        let j = t.chars().count() - 6;
        assert_eq!(super::token_at(t, j).map(|x| x.2), Some("img/b.png".into()));
        let k = t.find("flux.dev").unwrap();
        assert_eq!(super::token_at(t, k).map(|x| x.2), Some("https://flux.dev/a".into()));
        let d = std::env::temp_dir().join("flux-link-test");
        let _ = std::fs::create_dir_all(d.join("img"));
        std::fs::write(d.join("img/b.png"), b"x").unwrap();
        let ds = d.to_string_lossy().to_string();
        assert!(super::link_target("img/b.png", &ds, None).is_some());
        assert!(super::link_target("/img/b.png?v=2", "/nowhere", Some(&ds)).is_some());
        assert!(super::link_target("missing.png", &ds, None).is_none());
        assert!(super::link_target("#top", &ds, None).is_none());
        assert_eq!(super::link_target("https://a.b/c", &ds, None).as_deref(), Some("https://a.b/c"));
    }
}

#[cfg(test)]
mod enter_tests {
    use super::enter;

    fn run(text: &str, caret: usize, ext: &str) -> (String, usize) {
        let mut t = text.to_string();
        let (c, _) = enter(&mut t, caret, caret, ext);
        (t, c)
    }

    #[test]
    fn editing_auto_close_string_prefix() {
        // f"…" sa zatvorí, don't nie; v f-reťazci { sa zatvorí na {}
        let mut t = String::from("print(f\")");
        assert!(super::auto_close(&mut t, 8, '"'));
        assert_eq!(t, "print(f\"\")");
        let mut t = String::from("x = r'");
        assert!(super::auto_close(&mut t, 6, '\''));
        let mut t = String::from("don'");
        assert!(!super::auto_close(&mut t, 4, '\''));
        let mut t = String::from("print(f\"Ahoj {\")");
        assert!(super::auto_close(&mut t, 14, '{'));
        assert_eq!(t, "print(f\"Ahoj {}\")");
    }

    #[test]
    fn editing_enter_keeps_indent() {
        let (t, c) = run("    <dd>idk</dd>", 16, "html");
        assert_eq!(t, "    <dd>idk</dd>\n    ");
        assert_eq!(c, t.chars().count());
    }

    #[test]
    fn editing_enter_trims_blank_line() {
        // riadok len z medzier: vyprázdni sa, nový dostane to isté odsadenie
        let (t, c) = run("a\n    ", 6, "txt");
        assert_eq!(t, "a\n\n    ");
        assert_eq!(c, t.chars().count());
        // 8× Enter nenechá za sebou medzery
        let mut t = "    x".to_string();
        let mut c = 5;
        for _ in 0..8 {
            let r = enter(&mut t, c, c, "txt");
            c = r.0;
        }
        assert_eq!(t, "    x\n\n\n\n\n\n\n\n    ");
        assert_eq!(c, t.chars().count());
    }

    #[test]
    fn editing_enter_between_braces() {
        let (t, c) = run("fn a() {}", 8, "rs");
        assert_eq!(t, "fn a() {\n    \n}");
        assert_eq!(c, "fn a() {\n    ".chars().count());
        let (t, _) = run("if x {", 6, "js");
        assert_eq!(t, "if x {\n    ");
    }

    #[test]
    fn editing_enter_html_open_tag() {
        let (t, c) = run("<ul></ul>", 4, "html");
        assert_eq!(t, "<ul>\n    \n</ul>");
        assert_eq!(c, "<ul>\n    ".chars().count());
        let (t, _) = run("<body>\n    <div>", 16, "html");
        assert_eq!(t, "<body>\n    <div>\n        ");
    }

    #[test]
    fn editing_enter_void_and_comment() {
        assert_eq!(run("  <br>", 6, "html").0, "  <br>\n  ");
        assert_eq!(run("  <img src=\"a\">", 15, "html").0, "  <img src=\"a\">\n  ");
        assert_eq!(run("<!-- x -->", 10, "html").0, "<!-- x -->\n");
        assert_eq!(run("</div>", 6, "html").0, "</div>\n");
        assert_eq!(run("<br/>", 5, "html").0, "<br/>\n");
    }

    #[test]
    fn editing_enter_python_colon() {
        assert_eq!(run("def f():", 8, "py").0, "def f():\n    ");
        assert_eq!(run("x = {1: 2}", 10, "py").0, "x = {1: 2}\n");
    }
}
