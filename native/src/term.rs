// Terminál pre Výstup (▶ Run) aj Terminál: emulátor VT z Alacritty + náš pseudoterminál (flux-core::pty).
// Kreslí sa ako mriežka znakov s farbami; klávesy idú priamo programu.
use crate::theme::Pal;
use alacritty_terminal::event::{Event, EventListener};
use std::sync::{Arc, Mutex};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor};
use eframe::egui::{self, text::LayoutJob, Color32, FontId, Sense, TextFormat};
use flux_core::pty::Pty;

struct Size {
    cols: usize,
    rows: usize,
}
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

// odpovede terminálu programu (pozícia kurzora ESC[6n, ESC[c…) – Windows ConPTY bez odpovede na ESC[6n
// nepustí von žiadny výstup, takže program „visí“
#[derive(Clone, Default)]
struct Replies(Arc<Mutex<Vec<String>>>);
impl EventListener for Replies {
    fn send_event(&self, e: Event) {
        if let Event::PtyWrite(s) = e {
            self.0.lock().unwrap().push(s);
        }
    }
}

pub struct TermView {
    term: Term<Replies>,
    replies: Replies,
    parser: Processor,
    cols: usize,
    rows: usize,
    pub pty: Pty,
    id: egui::Id,
    pub font_size: f32,
    sel: Option<((i32, usize), (i32, usize))>, // výber myšou: (riadok mriežky, stĺpec) začiatok a koniec
}

impl TermView {
    pub fn new(id: &str) -> Self {
        let size = Size { cols: 100, rows: 24 };
        let config = Config { scrolling_history: 5000, ..Config::default() };
        let replies = Replies::default();
        TermView { term: Term::new(config, &size, replies.clone()), replies, parser: Processor::new(), cols: 100, rows: 24, pty: Pty::default(), id: egui::Id::new(id), font_size: 13.0, sel: None }
    }

    pub fn feed(&mut self, text: &str) {
        // nový výstup posúva riadky mriežky, výber by ukazoval inam
        self.sel = None;
        self.parser.advance(&mut self.term, text.as_bytes());
        for r in self.take_replies() {
            self.pty.write(&r);
        }
    }

    fn take_replies(&mut self) -> Vec<String> {
        std::mem::take(&mut *self.replies.0.lock().unwrap())
    }

    pub fn clear(&mut self) {
        // \x1b[3J zmaže históriu, \x1b[H\x1b[2J obrazovku
        self.feed("\x1b[3J\x1b[H\x1b[2J");
    }

    pub fn focus(&self, ctx: &egui::Context) {
        ctx.memory_mut(|m| m.request_focus(self.id));
    }

    fn color(c: Color, p: &Pal, fg: bool) -> Option<Color32> {
        let named = |n: NamedColor| -> Option<Color32> {
            Some(match n {
                NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::DimForeground => p.text,
                NamedColor::Background => return None,
                NamedColor::Black => {
                    if p.dark {
                        Color32::from_rgb(0x2a, 0x2a, 0x35)
                    } else {
                        Color32::from_rgb(0x1d, 0x1d, 0x24)
                    }
                }
                NamedColor::Red | NamedColor::BrightRed => p.red,
                NamedColor::Green | NamedColor::BrightGreen => p.green,
                NamedColor::Yellow | NamedColor::BrightYellow => Color32::from_rgb(0xf5, 0xb9, 0x4a),
                NamedColor::Blue | NamedColor::BrightBlue => Color32::from_rgb(0x6e, 0xa8, 0xff),
                NamedColor::Magenta | NamedColor::BrightMagenta => Color32::from_rgb(0xc7, 0x92, 0xea),
                NamedColor::Cyan | NamedColor::BrightCyan => Color32::from_rgb(0x5c, 0xcf, 0xe6),
                NamedColor::White | NamedColor::BrightWhite => p.text,
                NamedColor::BrightBlack => p.text2,
                _ => p.text,
            })
        };
        match c {
            Color::Named(n) => named(n),
            Color::Spec(rgb) => Some(Color32::from_rgb(rgb.r, rgb.g, rgb.b)),
            Color::Indexed(i) => {
                const N: [NamedColor; 16] = [
                    NamedColor::Black,
                    NamedColor::Red,
                    NamedColor::Green,
                    NamedColor::Yellow,
                    NamedColor::Blue,
                    NamedColor::Magenta,
                    NamedColor::Cyan,
                    NamedColor::White,
                    NamedColor::BrightBlack,
                    NamedColor::BrightRed,
                    NamedColor::BrightGreen,
                    NamedColor::BrightYellow,
                    NamedColor::BrightBlue,
                    NamedColor::BrightMagenta,
                    NamedColor::BrightCyan,
                    NamedColor::BrightWhite,
                ];
                if i < 16 {
                    return named(N[i as usize]);
                }
                if i >= 232 {
                    let v = 8 + (i - 232) * 10;
                    return Some(Color32::from_rgb(v, v, v));
                }
                let i = i - 16;
                let s = |x: u8| if x == 0 { 0 } else { 55 + x * 40 };
                Some(Color32::from_rgb(s(i / 36), s((i / 6) % 6), s(i % 6)))
            }
        }
        .or(if fg { Some(p.text) } else { None })
    }

    // vybraný rozsah v poradí (začiatok, koniec); prázdny výber (klik) sa nepočíta
    fn ordered(&self) -> Option<((i32, usize), (i32, usize))> {
        let (a, b) = self.sel?;
        if a == b {
            return None;
        }
        Some(if a <= b { (a, b) } else { (b, a) })
    }

    // text vybraných buniek: konce riadkov bez medzier, zalomené riadky sa spoja
    fn selected_text(&self) -> Option<String> {
        let (a, b) = self.ordered()?;
        let (top, bot) = (self.term.topmost_line().0, self.term.bottommost_line().0);
        let cols = self.term.columns();
        let mut out = String::new();
        for l in a.0.max(top)..=b.0.min(bot) {
            let from = if l == a.0 { a.1 } else { 0 };
            let to = if l == b.0 { b.1 } else { cols - 1 };
            let row = &self.term.grid()[Line(l)];
            let mut line = String::new();
            for c in from..=to.min(cols - 1) {
                let cell = &row[Column(c)];
                if !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    line.push(if cell.c == '\0' { ' ' } else { cell.c });
                }
            }
            let wrapped = row[Column(cols - 1)].flags.contains(Flags::WRAPLINE) && to == cols - 1;
            out.push_str(if wrapped { &line } else { line.trim_end() });
            if l < b.0 && !wrapped {
                out.push('\n');
            }
        }
        Some(out)
    }

    // Klávesy → bajty pre program (ako xterm); Ctrl+C s výberom (alebo Ctrl+Shift+C) kopíruje namiesto prerušenia
    fn keys(&mut self, ui: &egui::Ui, has_sel: bool) -> (Vec<String>, bool) {
        let mut out = vec![];
        let mut copy = false;
        ui.input(|i| {
            for ev in &i.events {
                match ev {
                    egui::Event::Text(t) => out.push(t.clone()),
                    egui::Event::Paste(t) => out.push(t.replace("\r\n", "\r").replace('\n', "\r")),
                    egui::Event::Key { key, pressed: true, modifiers, .. } => {
                        let s = match key {
                            egui::Key::Enter => "\r",
                            egui::Key::Backspace => "\x7f",
                            egui::Key::Tab => "\t",
                            egui::Key::Escape => "\x1b",
                            egui::Key::ArrowUp => "\x1b[A",
                            egui::Key::ArrowDown => "\x1b[B",
                            egui::Key::ArrowRight => "\x1b[C",
                            egui::Key::ArrowLeft => "\x1b[D",
                            egui::Key::Home => "\x1b[H",
                            egui::Key::End => "\x1b[F",
                            egui::Key::Delete => "\x1b[3~",
                            egui::Key::C if modifiers.ctrl && (modifiers.shift || has_sel) => {
                                copy = true;
                                ""
                            }
                            egui::Key::C if modifiers.ctrl => "\x03",
                            egui::Key::D if modifiers.ctrl => "\x04",
                            egui::Key::L if modifiers.ctrl => "\x0c",
                            _ => "",
                        };
                        if !s.is_empty() {
                            out.push(s.to_string());
                        }
                    }
                    _ => {}
                }
            }
        });
        (out, copy)
    }

    pub fn show(&mut self, ui: &mut egui::Ui, p: &Pal) {
        let font = FontId::monospace(self.font_size);
        let (cw, ch) = ui.fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let avail = ui.available_size();
        let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
        let resp = ui.interact(rect, self.id, Sense::click_and_drag());
        if resp.clicked() || resp.drag_started() {
            resp.request_focus();
        }
        // veľkosť mriežky podľa miesta
        let cols = ((rect.width() - 8.0) / cw).floor().max(20.0) as usize;
        let rows = ((rect.height() - 4.0) / ch).floor().max(4.0) as usize;
        if cols != self.cols || rows != self.rows {
            self.cols = cols;
            self.rows = rows;
            self.term.resize(Size { cols, rows });
            self.pty.resize(cols as u16, rows as u16);
        }
        // koliesko = história
        if resp.hovered() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            if dy.abs() > 0.5 {
                let lines = (dy / ch).round() as i32;
                if lines != 0 {
                    self.term.scroll_display(alacritty_terminal::grid::Scroll::Delta(lines));
                }
            }
        }
        // výber myšou: ťahanie označí bunky, klik ho zruší; mimo okna sa história posúva
        let offset0 = self.term.grid().display_offset() as i32;
        let cell_at = |pos: egui::Pos2| {
            let row = (((pos.y - rect.min.y - 2.0) / ch).floor().max(0.0) as usize).min(rows - 1);
            let col = (((pos.x - rect.min.x - 4.0) / cw).floor().max(0.0) as usize).min(cols - 1);
            (row as i32 - offset0, col)
        };
        if resp.drag_started() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let c = cell_at(pos);
                self.sel = Some((c, c));
            }
        } else if resp.dragged() {
            if let (Some(pos), Some((a, _))) = (ui.ctx().pointer_interact_pos(), self.sel) {
                self.sel = Some((a, cell_at(pos)));
                if pos.y < rect.top() {
                    self.term.scroll_display(alacritty_terminal::grid::Scroll::Delta(1));
                    ui.ctx().request_repaint();
                } else if pos.y > rect.bottom() {
                    self.term.scroll_display(alacritty_terminal::grid::Scroll::Delta(-1));
                    ui.ctx().request_repaint();
                }
            }
        } else if resp.clicked() {
            self.sel = None;
        }
        let mut copy = false;
        if resp.has_focus() {
            ui.memory_mut(|m| m.set_focus_lock_filter(self.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true }));
            let (keys, c) = self.keys(ui, self.ordered().is_some());
            copy = c;
            for s in keys {
                self.pty.write(&s);
            }
        }
        resp.context_menu(|ui| {
            if ui.add_enabled(self.ordered().is_some(), egui::Button::new(crate::i18n::t("Copy"))).clicked() {
                copy = true;
                ui.close();
            }
            if ui.button(crate::i18n::t("Select all")).clicked() {
                self.sel = Some(((self.term.topmost_line().0, 0), (self.term.bottommost_line().0, cols - 1)));
                ui.close();
            }
        });
        if copy {
            if let Some(text) = self.selected_text() {
                ui.ctx().copy_text(text);
            }
        }
        let painter = ui.painter_at(rect);
        let content = self.term.renderable_content();
        let cursor = content.cursor.point;
        let offset = content.display_offset as i32;
        if let Some((a, b)) = self.ordered() {
            for r in 0..rows {
                let l = r as i32 - offset;
                if l < a.0 || l > b.0 {
                    continue;
                }
                let from = if l == a.0 { a.1 } else { 0 };
                let to = if l == b.0 { b.1 } else { cols - 1 };
                let x = rect.min.x + 4.0 + from as f32 * cw;
                let r = egui::Rect::from_min_size(egui::pos2(x, rect.min.y + 2.0 + r as f32 * ch), egui::vec2((to + 1 - from) as f32 * cw, ch));
                painter.rect_filled(r, 0.0, p.accent.gamma_multiply(0.35));
            }
        }
        let mut jobs: Vec<LayoutJob> = (0..rows).map(|_| LayoutJob::default()).collect();
        // susedné bunky s rovnakými farbami idú do jedného úseku a medzery na konci riadku sa vynechajú
        // (predtým jeden append + String na každú bunku ≈ 2 400 za snímku)
        let fmt = |fg: Color32, bg: Color32| TextFormat { font_id: font.clone(), color: fg, background: bg, ..Default::default() };
        let mut cur: Option<usize> = None;
        let mut run = String::new();
        let mut run_f = (p.text, Color32::TRANSPARENT);
        let mut spaces = 0usize;
        let flush = |jobs: &mut Vec<LayoutJob>, row: Option<usize>, run: &mut String, f: (Color32, Color32)| {
            if let Some(r) = row {
                if !run.is_empty() {
                    jobs[r].append(run, 0.0, fmt(f.0, f.1));
                }
            }
            run.clear();
        };
        for cell in content.display_iter {
            let line = cell.point.line.0 + offset;
            if line < 0 || line as usize >= rows || cell.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let line = line as usize;
            if cur != Some(line) {
                // nový riadok: medzery na konci predošlého zahodiť
                flush(&mut jobs, cur, &mut run, run_f);
                spaces = 0;
                cur = Some(line);
            }
            let mut fg = Self::color(cell.cell.fg, p, true).unwrap_or(p.text);
            let mut bg = Self::color(cell.cell.bg, p, false).unwrap_or(Color32::TRANSPARENT);
            if cell.cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
                if fg == Color32::TRANSPARENT {
                    fg = p.card;
                }
            }
            let c = if cell.cell.c == '\0' { ' ' } else { cell.cell.c };
            if c == ' ' && bg == Color32::TRANSPARENT {
                spaces += 1;
                continue;
            }
            if spaces > 0 {
                if run.is_empty() || run_f.1 != Color32::TRANSPARENT {
                    flush(&mut jobs, cur, &mut run, run_f);
                    run_f = (fg, Color32::TRANSPARENT);
                }
                run.extend(std::iter::repeat_n(' ', spaces));
                spaces = 0;
            }
            if (fg, bg) != run_f {
                flush(&mut jobs, cur, &mut run, run_f);
                run_f = (fg, bg);
            }
            run.push(c);
        }
        flush(&mut jobs, cur, &mut run, run_f);
        for (i, job) in jobs.into_iter().enumerate() {
            if job.sections.is_empty() {
                continue;
            }
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            painter.galley(rect.min + egui::vec2(4.0, 2.0 + i as f32 * ch), galley, p.text);
        }
        // kurzor
        if resp.has_focus() && offset == 0 {
            let x = rect.min.x + 4.0 + cursor.column.0 as f32 * cw;
            let y = rect.min.y + 2.0 + cursor.line.0 as f32 * ch;
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(2.0, ch)), 0.0, p.accent);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // výber vráti text bez medzier na konci riadkov, aj cez viac riadkov
    #[test]
    fn term_selection_text() {
        let mut v = TermView::new("t");
        v.parser.advance(&mut v.term, b"hello world\r\nsecond   line\r\n");
        assert_eq!(v.selected_text(), None);
        v.sel = Some(((0, 6), (0, 10)));
        assert_eq!(v.selected_text().as_deref(), Some("world"));
        v.sel = Some(((1, 5), (0, 3))); // opačný smer
        assert_eq!(v.selected_text().as_deref(), Some("lo world\nsecond"));
        v.sel = Some(((0, 2), (0, 2)));
        assert_eq!(v.selected_text(), None);
    }

    #[test]
    fn term_answers_cursor_query() {
        let mut v = TermView::new("t");
        v.parser.advance(&mut v.term, b"ab\x1b[6n");
        assert_eq!(v.take_replies(), vec!["\x1b[1;3R".to_string()]);
    }
}
