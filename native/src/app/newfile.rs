// Nový súbor – okno s výberom jazyka: meno (prípona sa doplní), priečinok, krátky začiatok podľa jazyka.
use super::App;
use crate::i18n::{t, tf};
use crate::{theme, widgets};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use flux_core::fsops;
use std::path::Path;

// (id, názov, prípona, predvolené meno, začiatok súboru; {name} = meno bez prípony)
const LANGS: [(&str, &str, &str, &str, &str); 21] = [
    ("python", "Python", "py", "main", "print(\"Hello!\")\n"),
    ("html", "HTML", "html", "index", "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n  <title>{name}</title>\n</head>\n<body>\n  <h1>{name}</h1>\n</body>\n</html>\n"),
    ("css", "CSS", "css", "style", "body {\n  font-family: system-ui, sans-serif;\n}\n"),
    ("js", "JavaScript", "js", "script", "console.log('Hello!');\n"),
    ("ts", "TypeScript", "ts", "main", "const name: string = \"TypeScript\";\nconsole.log(`Hello from ${name}!`);\n"),
    ("java", "Java", "java", "Main", "public class {name} {\n    public static void main(String[] args) {\n        System.out.println(\"Hello from Java!\");\n    }\n}\n"),
    ("c", "C", "c", "main", "#include <stdio.h>\n\nint main(void) {\n    printf(\"Hello from C!\\n\");\n    return 0;\n}\n"),
    ("cpp", "C++", "cpp", "main", "#include <iostream>\n\nint main() {\n    std::cout << \"Hello from C++!\" << std::endl;\n    return 0;\n}\n"),
    ("csharp", "C#", "cs", "Program", "Console.WriteLine(\"Hello from C#!\");\n"),
    ("go", "Go", "go", "main", "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"Hello from Go!\")\n}\n"),
    ("rust", "Rust", "rs", "main", "fn main() {\n    println!(\"Hello from Rust!\");\n}\n"),
    ("ruby", "Ruby", "rb", "main", "puts \"Hello from Ruby!\"\n"),
    ("php", "PHP", "php", "index", "<?php\n\necho \"Hello from PHP!\\n\";\n"),
    ("lua", "Lua", "lua", "main", "print(\"Hello from Lua!\")\n"),
    ("zig", "Zig", "zig", "main", "const std = @import(\"std\");\n\npub fn main() void {\n    std.debug.print(\"Hello from Zig!\\n\", .{});\n}\n"),
    ("r", "R", "R", "main", "cat(\"Hello from R!\\n\")\n"),
    ("julia", "Julia", "jl", "main", "println(\"Hello from Julia!\")\n"),
    ("perl", "Perl", "pl", "main", "print \"Hello from Perl!\\n\";\n"),
    ("md", "Markdown", "md", "README", "# {name}\n\n"),
    ("json", "JSON", "json", "data", "{\n}\n"),
    ("txt", "Text", "txt", "notes", ""),
];

pub struct NewFile {
    lang: usize,
    name: String,
    touched: bool,
    dir: String,
    focus: bool,
    h: f32,
}

impl NewFile {
    fn default_name(&self) -> String {
        let (_, _, ext, base, _) = LANGS[self.lang];
        (1..100).map(|i| if i == 1 { base.to_string() } else { format!("{base}-{i}") }).find(|n| !Path::new(&self.dir).join(format!("{n}.{ext}")).exists()).unwrap_or_else(|| base.into())
    }

    // meno súboru s príponou (vlastná prípona má prednosť)
    fn file_name(&self) -> String {
        let n = self.name.trim();
        if n.contains('.') {
            n.to_string()
        } else {
            format!("{n}.{}", LANGS[self.lang].2)
        }
    }
}

// druh projektu / jazyk z úvodu → jazyk nového súboru
fn lang_of(kind: &str) -> Option<usize> {
    let id = match kind {
        "web" => "html",
        "node" => "js",
        k => k,
    };
    LANGS.iter().position(|l| l.0 == id)
}

impl App {
    // dir = priečinok zo stromu (kontextová ponuka), inak projekt, inak priečinok projektov
    pub(super) fn open_new_file(&mut self, dir: Option<String>) {
        let ws = self.workspace();
        let dir = dir.or_else(|| self.new_in.take()).or_else(|| ws.clone()).unwrap_or_else(|| fsops::default_root().to_string_lossy().to_string());
        let codes: Vec<String> = self.core.setting("codeLangs").as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let lang = ws.as_deref().and_then(|w| self.main_kind(w)).and_then(|k| lang_of(&k)).or_else(|| codes.iter().find_map(|c| lang_of(c))).unwrap_or(0);
        let mut nf = NewFile { lang, name: String::new(), touched: false, dir, focus: true, h: 470.0 };
        nf.name = nf.default_name();
        self.new_file = Some(nf);
    }

    fn create_file(&mut self) {
        let Some(nf) = self.new_file.as_ref() else { return };
        if nf.name.trim().is_empty() {
            return;
        }
        let file = nf.file_name();
        let path = Path::new(&nf.dir).join(&file).to_string_lossy().to_string();
        if Path::new(&path).exists() {
            self.status = tf("{name} already exists.", &[("name", &file)]);
            return;
        }
        let stem = Path::new(&file).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let content = LANGS[nf.lang].4.replace("{name}", &stem);
        let dir = nf.dir.clone();
        let _ = std::fs::create_dir_all(&dir);
        if let Err(e) = fsops::create(&path, false).and_then(|_| fsops::write(&path, &content)) {
            self.status = e;
            return;
        }
        self.new_file = None;
        // mimo otvoreného projektu = voľný súbor (sekcia FILES)
        let inside = self.workspace().is_some_and(|w| dir.starts_with(&w));
        if !inside {
            fsops::allow_file(&self.core, &path);
            self.reload_projects();
        }
        self.tree.clear();
        self.open_dirs.insert(dir);
        self.start = false;
        self.open_file(&path);
    }

    pub(super) fn new_file_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        let Some(nf) = self.new_file.as_ref() else { return };
        let bg = ui.interact(full, ui.id().with("nf-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(80));
        let w = (full.width() - 48.0).min(620.0);
        let top = full.top() + ((full.height() - 520.0) / 2.0).max(16.0);
        let card = Rect::from_min_size(pos2(full.center().x - w / 2.0, top), vec2(w, nf.h.min(full.bottom() - 16.0 - top)));
        let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if bg.clicked() && !card.contains(bg.interact_pointer_pos().unwrap_or_default()) {
            close = true;
        }
        ui.painter().add(egui::Shadow { offset: [0, 30], blur: 90, spread: 0, color: Color32::from_black_alpha(128) }.as_shape(card, CornerRadius::same(20)));
        ui.painter().rect_filled(card, CornerRadius::same(20), p.solid);
        ui.painter().rect_stroke(card, CornerRadius::same(20), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let inner = card.shrink2(vec2(24.0, 20.0));
        let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(inner.min, vec2(inner.width(), 2000.0))));
        cu.set_clip_rect(card);
        let iw = inner.width();
        cu.horizontal(|ui| {
            ui.label(egui::RichText::new(t("New file")).font(theme::bold(20.0)).color(p.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::icon_button(ui, "x", &p, true).on_hover_text(t("Close (Esc)")).clicked() {
                    close = true;
                }
            });
        });
        cu.add_space(12.0);
        cu.label(egui::RichText::new(t("Which language?")).font(theme::bold(12.5)).color(p.text2));
        cu.add_space(6.0);
        // jazyky ako karty (7 v rade)
        let cols = 7usize;
        let gap = 6.0;
        let cw = (iw - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = 58.0;
        let rows = LANGS.len().div_ceil(cols);
        let (area, _) = cu.allocate_exact_size(vec2(iw, rows as f32 * (ch + gap) - gap), Sense::hover());
        let cur = nf.lang;
        let mut pick = None;
        for (i, (_, title, ext, ..)) in LANGS.iter().enumerate() {
            let r = Rect::from_min_size(area.min + vec2((i % cols) as f32 * (cw + gap), (i / cols) as f32 * (ch + gap)), vec2(cw, ch));
            let resp = cu.interact(r, cu.id().with(("nf-lang", i)), Sense::click());
            let on = i == cur;
            let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.1);
            cu.painter().rect_filled(r, CornerRadius::same(10), if on { p.active } else { p.card2.lerp_to_gamma(p.hover, hk) });
            cu.painter().rect_stroke(r, CornerRadius::same(10), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
            widgets::file_icon(&cu, Rect::from_center_size(pos2(r.center().x, r.top() + 20.0), vec2(20.0, 20.0)), &format!("a.{}", ext.to_lowercase()));
            widgets::text(&cu, pos2(r.center().x, r.top() + 43.0), Align2::CENTER_CENTER, &t(title), theme::bold(11.0), if on { p.text } else { p.text2 }, cw - 6.0);
            if resp.on_hover_text(format!(".{ext}")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                pick = Some(i);
            }
        }
        let nf = self.new_file.as_mut().unwrap();
        if let Some(i) = pick {
            nf.lang = i;
            if !nf.touched {
                nf.name = nf.default_name();
            }
            nf.focus = true;
        }
        // meno + prípona
        cu.add_space(14.0);
        cu.label(egui::RichText::new(t("Name")).font(theme::bold(12.5)).color(p.text2));
        cu.add_space(5.0);
        let mut create = false;
        cu.horizontal(|ui| {
            let ext = if nf.name.contains('.') { String::new() } else { format!(".{}", LANGS[nf.lang].2) };
            let ew = if ext.is_empty() { 0.0 } else { widgets::text_w(ui, &ext, theme::mono(13.0)) + 20.0 };
            let te = ui.add(egui::TextEdit::singleline(&mut nf.name).desired_width(iw - ew - 8.0).margin(egui::Margin::symmetric(12, 8)).font(theme::ui(14.0)));
            if nf.focus {
                nf.focus = false;
                te.request_focus();
            }
            if te.changed() {
                nf.touched = true;
            }
            if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                create = true;
            }
            if ew > 0.0 {
                let (r, _) = ui.allocate_exact_size(vec2(ew, 34.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(9), p.card2);
                ui.painter().text(r.center(), Align2::CENTER_CENTER, &ext, theme::mono(13.0), p.text2);
            }
        });
        // priečinok
        cu.add_space(12.0);
        cu.label(egui::RichText::new(t("Location")).font(theme::bold(12.5)).color(p.text2));
        cu.add_space(5.0);
        let mut change = false;
        cu.horizontal(|ui| {
            let loc = Path::new(&nf.dir).join(nf.file_name()).to_string_lossy().to_string();
            let (r, _) = ui.allocate_exact_size(vec2(iw - 110.0, 32.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::same(9), p.card2);
            widgets::text(ui, pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, &loc, theme::mono(12.0), p.text2, r.width() - 20.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(ui, None, &t("Change…"), p.card2, p.text, 30.0, &p).clicked() {
                    change = true;
                }
            });
        });
        if change {
            if let Some(d) = rfd::FileDialog::new().set_title(t("Location")).set_directory(&nf.dir).pick_folder() {
                nf.dir = d.to_string_lossy().to_string();
                if !nf.touched {
                    nf.name = nf.default_name();
                }
            }
        }
        cu.add_space(18.0);
        cu.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(ui, Some("filePlus"), &t("Create file"), p.accent, p.accent_fg, 32.0, &p).clicked() {
                    create = true;
                }
                if widgets::button(ui, None, &t("Cancel"), p.card2, p.text, 32.0, &p).clicked() {
                    close = true;
                }
            });
        });
        let used = cu.min_rect().height() + 40.0;
        if let Some(nf) = self.new_file.as_mut() {
            if (nf.h - used).abs() > 1.0 {
                nf.h = used;
                ctx.request_repaint();
            }
        }
        if close {
            self.new_file = None;
        } else if create {
            self.create_file();
        }
    }
}
