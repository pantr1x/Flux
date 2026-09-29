// Nový projekt – okno ako newProject() v Electron Fluxe: druh projektu, ikona (jazyk, čiarová ikona alebo emoji),
// názov, krátky popis a umiestnenie. Popis, druh a ikona idú do settings.projectMeta[dir] (zdieľané s Electron Fluxom).
use super::App;
use crate::i18n::{t, tf};
use crate::{theme, widgets};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use flux_core::fsops;
use serde_json::{json, Value};
use std::path::Path;

// (id, názov, popis, ikona súboru, nástroj z Jazykov) – PROJECT_KINDS z app.js
pub const KINDS: [(&str, &str, &str, &str, &str); 4] = [
    ("python", "Python", "Scripts, games and apps", "a.py", "python"),
    ("web", "Website", "HTML, CSS and JavaScript", "a.html", ""),
    ("node", "JavaScript", "Scripts with Node.js", "a.js", "node"),
    ("empty", "Empty", "Start from scratch", "", ""),
];

// ďalšie jazyky: (id, názov, ikona, nástroj, hlavný súbor, obsah)
pub const MORE: [(&str, &str, &str, &str, &str, &str); 14] = [
    ("java", "Java", "a.java", "java", "Main.java", "public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello from Java!\");\n    }\n}\n"),
    ("cpp", "C++", "a.cpp", "cpp", "main.cpp", "#include <iostream>\n\nint main() {\n    std::cout << \"Hello from C++!\" << std::endl;\n    return 0;\n}\n"),
    ("c", "C", "a.c", "cpp", "main.c", "#include <stdio.h>\n\nint main(void) {\n    printf(\"Hello from C!\\n\");\n    return 0;\n}\n"),
    ("go", "Go", "a.go", "go", "main.go", "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"Hello from Go!\")\n}\n"),
    ("csharp", "C#", "a.cs", "csharp", "Program.cs", "Console.WriteLine(\"Hello from C#!\");\n"),
    ("rust", "Rust", "a.rs", "rust", "main.rs", "fn main() {\n    println!(\"Hello from Rust!\");\n}\n"),
    ("ruby", "Ruby", "a.rb", "ruby", "main.rb", "puts \"Hello from Ruby!\"\n"),
    ("php", "PHP", "a.php", "php", "index.php", "<?php\n\necho \"Hello from PHP!\\n\";\n"),
    ("lua", "Lua", "a.lua", "lua", "main.lua", "print(\"Hello from Lua!\")\n"),
    ("zig", "Zig", "a.zig", "zig", "main.zig", "const std = @import(\"std\");\n\npub fn main() void {\n    std.debug.print(\"Hello from Zig!\\n\", .{});\n}\n"),
    ("r", "R", "a.r", "r", "main.R", "cat(\"Hello from R!\\n\")\n"),
    ("julia", "Julia", "a.jl", "julia", "main.jl", "println(\"Hello from Julia!\")\n"),
    ("ts", "TypeScript", "a.ts", "node", "main.ts", "const name: string = \"TypeScript\";\nconsole.log(`Hello from ${name}!`);\n"),
    ("perl", "Perl", "a.pl", "perl", "main.pl", "print \"Hello from Perl!\\n\";\n"),
];

// ikony na výber: čiarové ikony Fluxu a emoji (ako favicon)
const LINE_ICONS: [&str; 13] = ["robot", "code", "globe", "rocket", "star", "puzzle", "terminal", "sparkle", "flask", "palette", "camera", "phone", "todo"];
const EMOJI: [&str; 12] = ["🎮", "🐍", "🚀", "🎨", "🎵", "📊", "📱", "🔥", "⚡", "🐱", "💡", "🏆"];

pub struct NewProj {
    kind: String,
    name: String,
    touched: bool,
    desc: String,
    icon: String, // "" = ikona jazyka, "line:rocket", "emoji:🤖"
    root: String,
    focus: bool,
    h: f32, // výška okna z minulého snímku (na vycentrovanie)
}

impl NewProj {
    pub fn new(kind: &str) -> Self {
        let root = fsops::default_root().to_string_lossy().to_string();
        let mut np = NewProj { kind: kind.into(), name: String::new(), touched: false, desc: String::new(), icon: String::new(), root, focus: true, h: 600.0 };
        np.name = np.default_name();
        np
    }

    fn default_name(&self) -> String {
        let base = match self.kind.as_str() {
            "python" => "my-program".to_string(),
            "web" => "my-website".into(),
            "node" => "my-script".into(),
            "empty" => "new-project".into(),
            k => format!("my-{k}-app"),
        };
        // voľné meno v priečinku projektov
        (1..100).map(|i| if i == 1 { base.clone() } else { format!("{base}-{i}") }).find(|n| !Path::new(&self.root).join(n).exists()).unwrap_or(base)
    }
}

fn kind_icon(kind: &str) -> &'static str {
    KINDS.iter().find(|k| k.0 == kind).map(|k| k.3).or(MORE.iter().find(|k| k.0 == kind).map(|k| k.2)).unwrap_or("")
}

fn kind_tool(kind: &str) -> &'static str {
    KINDS.iter().find(|k| k.0 == kind).map(|k| k.4).or(MORE.iter().find(|k| k.0 == kind).map(|k| k.3)).unwrap_or("")
}

// ikona projektu: „line:…“, „emoji:…“ alebo „file:a.py“ (ikona jazyka); prázdne = priečinok
pub fn paint_icon(ui: &egui::Ui, r: Rect, spec: &str, p: &theme::Pal) {
    if let Some(n) = spec.strip_prefix("line:") {
        widgets::icon(ui, r, n, p.text);
    } else if let Some(e) = spec.strip_prefix("emoji:") {
        ui.painter().text(r.center(), Align2::CENTER_CENTER, e, egui::FontId::proportional(r.height() * 0.86), p.text);
    } else if let Some(f) = spec.strip_prefix("file:").filter(|f| !f.is_empty()) {
        widgets::file_icon(ui, r, f);
    } else {
        widgets::icon(ui, r.shrink(r.width() * 0.08), "folder", p.text2);
    }
}

// výber ikony: vráti novú hodnotu (prázdna = ikona jazyka)
pub fn icon_picker(ui: &mut egui::Ui, cur: &str, lang_file: &str, w: f32, p: &theme::Pal) -> Option<String> {
    let mut all: Vec<String> = vec![String::new()];
    all.extend(LINE_ICONS.iter().map(|n| format!("line:{n}")));
    all.extend(EMOJI.iter().map(|e| format!("emoji:{e}")));
    let s = 34.0;
    let gap = 6.0;
    let cols = ((w + gap) / (s + gap)).floor().max(1.0) as usize;
    let rows = all.len().div_ceil(cols);
    let (area, _) = ui.allocate_exact_size(vec2(w, rows as f32 * (s + gap) - gap), Sense::hover());
    let mut pick = None;
    for (i, spec) in all.iter().enumerate() {
        let r = Rect::from_min_size(area.min + vec2((i % cols) as f32 * (s + gap), (i / cols) as f32 * (s + gap)), vec2(s, s));
        let resp = ui.interact(r, ui.id().with(("np-ic", i)), Sense::click());
        let on = spec == cur;
        let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.1);
        ui.painter().rect_filled(r, CornerRadius::same(9), if on { p.active } else { p.card2.lerp_to_gamma(p.hover, hk) });
        ui.painter().rect_stroke(r, CornerRadius::same(9), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
        let inner = Rect::from_center_size(r.center(), vec2(18.0, 18.0));
        if spec.is_empty() {
            paint_icon(ui, inner, &format!("file:{lang_file}"), p);
        } else {
            paint_icon(ui, inner, spec, p);
        }
        let resp = if spec.is_empty() { resp.on_hover_text(t("Language icon")) } else { resp };
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            pick = Some(spec.clone());
        }
    }
    pick
}

impl App {
    pub(super) fn open_new_project(&mut self) {
        let langs: Vec<String> = self.core.setting("codeLangs").as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let kind = if langs.iter().any(|l| l == "web") && !langs.iter().any(|l| l == "python") { "web" } else { "python" };
        self.new_project = Some(NewProj::new(kind));
    }

    // ikona projektu: vlastná z projectMeta, inak hlavný jazyk, inak priečinok
    pub(super) fn project_icon(&self, dir: &str) -> String {
        if let Some(i) = self.core.setting("projectMeta")[dir]["icon"].as_str().filter(|i| !i.is_empty()) {
            return i.to_string();
        }
        match self.main_kind(dir).as_deref().map(super::kind_file).filter(|f| !f.is_empty()) {
            Some(f) => format!("file:{f}"),
            None => match self.core.setting("projectMeta")[dir]["kind"].as_str().map(kind_icon).filter(|f| !f.is_empty()) {
                Some(f) => format!("file:{f}"),
                None => String::new(),
            },
        }
    }

    pub(super) fn set_project_meta(&self, dir: &str, patch: Value) {
        self.update_settings(|o| {
            let mut all = o.get("projectMeta").cloned().unwrap_or(json!({}));
            let mut m = all[dir].as_object().cloned().unwrap_or_default();
            if let Some(pa) = patch.as_object() {
                for (k, v) in pa {
                    if v.is_null() {
                        m.remove(k);
                    } else {
                        m.insert(k.clone(), v.clone());
                    }
                }
            }
            all[dir] = Value::Object(m);
            o.insert("projectMeta".into(), all);
        });
    }

    fn create_project(&mut self) {
        let Some(np) = self.new_project.as_ref() else { return };
        let name = np.name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let (kind, desc, icon, root) = (np.kind.clone(), np.desc.trim().to_string(), np.icon.clone(), np.root.clone());
        let dir = match fsops::create_project(&name, &root) {
            Ok(d) => d.as_str().unwrap_or("").to_string(),
            Err(e) => {
                self.status = t(&e);
                return;
            }
        };
        self.new_project = None;
        let mut meta = json!({ "description": desc, "kind": kind, "todos": [] });
        if !icon.is_empty() {
            meta["icon"] = json!(icon);
        }
        self.set_project_meta(&dir, meta);
        self.start = false;
        self.open_folder(&dir);
        // prvé súbory podľa druhu (šablóny ako na domovskej obrazovke)
        let files: Vec<(String, String)> = match kind.as_str() {
            "python" => vec![("main.py".into(), format!("print(\"Hello from {name}!\")\n"))],
            "web" => super::home::template_files("web").into_iter().map(|(f, c)| (f.to_string(), c.replace("{{title}}", &name))).collect(),
            "node" => super::home::template_files("js").into_iter().map(|(f, c)| (f.to_string(), c.to_string())).collect(),
            k => MORE.iter().find(|m| m.0 == k).map(|m| vec![(m.4.to_string(), m.5.to_string())]).unwrap_or_default(),
        };
        let mut first = None;
        for (f, c) in &files {
            let path = Path::new(&dir).join(f).to_string_lossy().to_string();
            if fsops::write(&path, c).is_ok() && first.is_none() {
                first = Some(path);
            }
        }
        self.tree.clear();
        self.reload_projects();
        if let Some(f) = first {
            self.open_file(&f);
        }
        self.summarize(&dir);
        self.note(tf("Project {name} is ready.", &[("name", &name)]));
    }

    pub(super) fn new_project_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        let Some(np) = self.new_project.as_mut() else { return };
        let bg = ui.interact(full, ui.id().with("np-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(80));
        let w = (full.width() - 48.0).min(660.0);
        let h = np.h.min(full.height() - 32.0);
        let card = Rect::from_center_size(full.center(), vec2(w, h));
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
        // hlavička
        cu.horizontal(|ui| {
            ui.label(egui::RichText::new(t("Create a project")).font(theme::bold(20.0)).color(p.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::icon_button(ui, "x", &p, true).on_hover_text(t("Close (Esc)")).clicked() {
                    close = true;
                }
            });
        });
        cu.add_space(14.0);
        let np = self.new_project.as_mut().unwrap();
        // druhy projektu
        let cw = (iw - 3.0 * 8.0) / 4.0;
        let (area, _) = cu.allocate_exact_size(vec2(iw, 96.0), Sense::hover());
        let mut kind_pick = None;
        for (i, (id, title, text, file, _)) in KINDS.iter().enumerate() {
            let r = Rect::from_min_size(area.min + vec2(i as f32 * (cw + 8.0), 0.0), vec2(cw, 96.0));
            let resp = cu.interact(r, cu.id().with(("np-kind", *id)), Sense::click());
            let on = np.kind == *id;
            let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
            cu.painter().rect_filled(r, CornerRadius::same(14), if on { p.active } else { p.card2.lerp_to_gamma(p.hover, hk) });
            cu.painter().rect_stroke(r, CornerRadius::same(14), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
            let ic = Rect::from_center_size(pos2(r.left() + 26.0, r.top() + 26.0), vec2(26.0, 26.0));
            if file.is_empty() {
                widgets::icon(&cu, ic, "folder", p.text2);
            } else {
                widgets::file_icon(&cu, ic, file);
            }
            cu.painter().text(pos2(r.left() + 13.0, r.top() + 58.0), Align2::LEFT_CENTER, t(title), theme::bold(13.0), p.text);
            widgets::text(&cu, pos2(r.left() + 13.0, r.top() + 77.0), Align2::LEFT_CENTER, &t(text), theme::ui(11.0), p.text3, cw - 20.0);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                kind_pick = Some(id.to_string());
            }
        }
        cu.add_space(10.0);
        // ďalšie jazyky ako čipy
        cu.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.label(egui::RichText::new(t("More languages")).font(theme::ui(12.0)).color(p.text3));
            for (id, title, file, ..) in MORE.iter() {
                let tw = widgets::text_w(ui, title, theme::bold(12.0));
                let (r, resp) = ui.allocate_exact_size(vec2(tw + 36.0, 28.0), Sense::click());
                let on = np.kind == *id;
                let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
                ui.painter().rect_filled(r, CornerRadius::same(9), if on { p.active } else { p.card2.lerp_to_gamma(p.hover, hk) });
                ui.painter().rect_stroke(r, CornerRadius::same(9), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
                widgets::file_icon(ui, Rect::from_min_size(pos2(r.left() + 9.0, r.center().y - 7.0), vec2(14.0, 14.0)), file);
                ui.painter().text(pos2(r.left() + 28.0, r.center().y), Align2::LEFT_CENTER, *title, theme::bold(12.0), p.text);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    kind_pick = Some(id.to_string());
                }
            }
        });
        if let Some(k) = kind_pick {
            np.kind = k;
            if !np.touched {
                np.name = np.default_name();
            }
        }
        // jazyk ešte nie je na počítači → tlačidlo Inštalovať
        let tool = kind_tool(&np.kind);
        let kind = np.kind.clone();
        let _ = kind;
        if !tool.is_empty() {
            super::tools::refresh(&self.tools, &ctx, false);
            let st = {
                let s = self.tools.lock().unwrap();
                s.status.as_ref().and_then(|(l, _)| flux_core::toolchains::TOOLCHAINS.iter().zip(l.iter()).find(|(tc, _)| tc.id == tool).map(|(tc, st)| (tc.name, st.installed)))
            };
            if let Some((tname, false)) = st {
                cu.add_space(10.0);
                let mut install = false;
                egui::Frame::new().fill(p.hover).stroke(Stroke::new(1.0, p.line)).corner_radius(12).inner_margin(egui::Margin::symmetric(14, 10)).show(&mut cu, |ui| {
                    ui.set_width(iw - 28.0);
                    ui.horizontal(|ui| {
                        widgets::icon_at(ui, pos2(ui.cursor().left() + 8.0, ui.cursor().top() + 15.0), 14.0, "download", p.text2);
                        ui.add_space(22.0);
                        ui.label(egui::RichText::new(tf("You will need {name} to run this project:", &[("name", tname)])).font(theme::ui(12.5)).color(p.text2));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if widgets::button(ui, Some("download"), &t("Install"), p.card2, p.text, 30.0, &p).clicked() {
                                install = true;
                            }
                        });
                    });
                });
                if install {
                    super::tools::install(&self.tools, &ctx, tool);
                    self.note(tf("Installing {name}…", &[("name", tname)]));
                }
            }
        }
        let np = self.new_project.as_mut().unwrap();
        // ikona
        cu.add_space(14.0);
        cu.label(egui::RichText::new(t("Icon")).font(theme::bold(12.5)).color(p.text2));
        cu.add_space(6.0);
        if let Some(ic) = icon_picker(&mut cu, &np.icon, kind_icon(&np.kind), iw, &p) {
            np.icon = ic;
        }
        // názov, popis, umiestnenie
        let field = |ui: &mut egui::Ui, label: &str, opt: bool| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(label).font(theme::bold(12.5)).color(p.text2));
                if opt {
                    ui.label(egui::RichText::new(t("optional")).font(theme::ui(11.5)).color(p.text3));
                }
            });
            ui.add_space(5.0);
        };
        field(&mut cu, &t("Name"), false);
        let te = cu.add(egui::TextEdit::singleline(&mut np.name).desired_width(iw).margin(egui::Margin::symmetric(12, 8)).font(theme::ui(14.0)));
        if np.focus {
            np.focus = false;
            te.request_focus();
        }
        if te.changed() {
            np.touched = true;
        }
        let enter = te.lost_focus() && cu.input(|i| i.key_pressed(egui::Key::Enter));
        field(&mut cu, &t("Short description"), true);
        cu.add(egui::TextEdit::singleline(&mut np.desc).hint_text(t("e.g. A game where you catch falling stars")).desired_width(iw).char_limit(160).margin(egui::Margin::symmetric(12, 8)));
        field(&mut cu, &t("Location"), false);
        let mut change = false;
        cu.horizontal(|ui| {
            let loc = Path::new(&np.root).join(np.name.trim()).to_string_lossy().to_string();
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
            if let Some(d) = rfd::FileDialog::new().set_title(t("Location")).set_directory(&np.root).pick_folder() {
                np.root = d.to_string_lossy().to_string();
                if !np.touched {
                    np.name = np.default_name();
                }
            }
        }
        // pätička
        cu.add_space(20.0);
        let mut create = enter;
        let mut github = false;
        cu.horizontal(|ui| {
            if widgets::button(ui, Some("github"), &t("Open from GitHub…"), p.card2, p.text, 32.0, &p).clicked() {
                github = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(ui, Some("plus"), &t("Create project"), p.accent, p.accent_fg, 32.0, &p).clicked() {
                    create = true;
                }
                if widgets::button(ui, None, &t("Cancel"), p.card2, p.text, 32.0, &p).clicked() {
                    close = true;
                }
            });
        });
        let used = cu.min_rect().height() + 40.0;
        let np = self.new_project.as_mut().unwrap();
        if (np.h - used).abs() > 1.0 {
            np.h = used;
            ctx.request_repaint();
        }
        if github {
            self.new_project = None;
            self.open_settings("github", &ctx);
        } else if close {
            self.new_project = None;
        } else if create {
            self.create_project();
        }
    }
}
