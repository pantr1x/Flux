// Terminál pre Výstup (▶ Run) aj Terminál: emulátor VT z Alacritty + náš pseudoterminál (flux-core::pty).
// Kreslí sa ako mriežka znakov s farbami; klávesy idú priamo programu.
use crate::theme::Pal;
use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
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

pub struct TermView {
    term: Term<VoidListener>,
    parser: Processor,
    cols: usize,
    rows: usize,
    pub pty: Pty,
    id: egui::Id,
    pub font_size: f32,
}

impl TermView {
    pub fn new(id: &str) -> Self {
        let size = Size { cols: 100, rows: 24 };
        let config = Config { scrolling_history: 5000, ..Config::default() };
        TermView { term: Term::new(config, &size, VoidListener), parser: Processor::new(), cols: 100, rows: 24, pty: Pty::default(), id: egui::Id::new(id), font_size: 13.0 }
    }

    pub fn feed(&mut self, text: &str) {
        self.parser.advance(&mut self.term, text.as_bytes());
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

    // Klávesy → bajty pre program (ako xterm).
    fn keys(&mut self, ui: &egui::Ui) -> Vec<String> {
        let mut out = vec![];
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
                            egui::Key::C if modifiers.ctrl && !modifiers.shift => "\x03",
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
        out
    }

    pub fn show(&mut self, ui: &mut egui::Ui, p: &Pal) {
        let font = FontId::monospace(self.font_size);
        let (cw, ch) = ui.fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let avail = ui.available_size();
        let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
        let resp = ui.interact(rect, self.id, Sense::click());
        if resp.clicked() {
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
        if resp.has_focus() {
            ui.memory_mut(|m| m.set_focus_lock_filter(self.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true }));
            for s in self.keys(ui) {
                self.pty.write(&s);
            }
        }
        let painter = ui.painter_at(rect);
        let content = self.term.renderable_content();
        let cursor = content.cursor.point;
        let offset = content.display_offset as i32;
        let mut jobs: Vec<LayoutJob> = (0..rows).map(|_| LayoutJob::default()).collect();
        for cell in content.display_iter {
            let line = cell.point.line.0 + offset;
            if line < 0 || line as usize >= rows || cell.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let mut fg = Self::color(cell.cell.fg, p, true).unwrap_or(p.text);
            let mut bg = Self::color(cell.cell.bg, p, false).unwrap_or(Color32::TRANSPARENT);
            if cell.cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
                if fg == Color32::TRANSPARENT {
                    fg = p.card;
                }
            }
            let job = &mut jobs[line as usize];
            let c = if cell.cell.c == '\0' { ' ' } else { cell.cell.c };
            job.append(&c.to_string(), 0.0, TextFormat { font_id: font.clone(), color: fg, background: bg, ..Default::default() });
        }
        for (i, job) in jobs.into_iter().enumerate() {
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
