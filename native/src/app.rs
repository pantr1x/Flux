// Flux Native – okno: horná lišta so súbormi a ▶ Run, bočný panel (projekty + strom súborov),
// editor s farbami kódu, dole Výstup / Terminál a stavový riadok. Logika je v flux-core (rovnaká ako v Tauri verzii).
use crate::term::TermView;
use crate::widgets::{self, Icon};
use crate::theme::{self, Pal};
use eframe::egui::{self, Color32, CornerRadius, FontId, Frame, Margin, RichText, Stroke};
use flux_core::{fsops, python, runner, settings, Core, Emit};
use notify::{RecursiveMode, Watcher};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Tab {
    path: String,
    text: String,
    saved: String,
}

impl Tab {
    fn name(&self) -> String {
        Path::new(&self.path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    }
    fn ext(&self) -> String {
        Path::new(&self.path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
    }
    fn dirty(&self) -> bool {
        self.text != self.saved
    }
}

#[derive(PartialEq)]
enum Bottom {
    Output,
    Terminal,
}

pub struct App {
    core: Arc<Core>,
    emit: Emit,
    rx: Receiver<(String, Value)>,
    pal: Pal,
    tabs: Vec<Tab>,
    active: usize,
    tree: HashMap<String, Vec<(String, String, bool)>>, // priečinok → (meno, cesta, je priečinok)
    open_dirs: HashSet<String>,
    projects: Vec<Value>,
    out: TermView,
    sh: TermView,
    shell_started: bool,
    bottom: Bottom,
    running: bool,
    python: Option<String>,
    last_edit: Option<Instant>,
    status: String,
    watcher: Option<notify::RecommendedWatcher>,
    test: Vec<(f32, String)>,
    started: Instant,
    cursor: (usize, usize),
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let core = Arc::new(Core::load());
        let dark = core.setting("theme").as_str() != Some("light");
        let pal = theme::palette(dark);
        theme::fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, &pal);
        // udalosti z jadra (výstup programu, terminál, zmeny súborov) → fronta + prekreslenie
        let (tx, rx) = channel::<(String, Value)>();
        let ctx = cc.egui_ctx.clone();
        let emit: Emit = Arc::new(move |ch: &str, v: Value| {
            let _ = tx.send((ch.to_string(), v));
            ctx.request_repaint();
        });
        let mut app = App {
            core,
            emit,
            rx,
            pal,
            tabs: vec![],
            active: 0,
            tree: HashMap::new(),
            open_dirs: HashSet::new(),
            projects: vec![],
            out: TermView::new("flux-output"),
            sh: TermView::new("flux-shell"),
            shell_started: false,
            bottom: Bottom::Output,
            running: false,
            python: None,
            last_edit: None,
            status: String::new(),
            watcher: None,
            test: vec![],
            started: Instant::now(),
            cursor: (1, 1),
        };
        app.out.feed("Program output appears here. Press F5 or \u{25B6} Run.\r\n");
        app.projects = fsops::projects(&app.core).as_array().cloned().unwrap_or_default();
        if let Some(last) = app.core.setting("lastFolder").as_str().map(String::from) {
            app.open_folder(&last);
        }
        // testy: FLUX_TEST="2:open=main.py;4:run;7:type=Rust\r" (čas v s : akcia)
        if let Ok(t) = std::env::var("FLUX_TEST") {
            app.test = t.split(';').filter_map(|p| p.split_once(':').map(|(a, b)| (a.parse().unwrap_or(0.0), b.to_string()))).collect();
        }
        app
    }

    fn workspace(&self) -> Option<String> {
        self.core.workspace.lock().unwrap().clone()
    }

    fn open_folder(&mut self, dir: &str) {
        if fsops::open_workspace(&self.core, dir).is_null() {
            return;
        }
        self.tree.clear();
        self.open_dirs.clear();
        self.projects = fsops::projects(&self.core).as_array().cloned().unwrap_or_default();
        self.python = None;
        // zmeny v projekte (aj z iných programov) → fs:changed
        let emit = self.emit.clone();
        let last = std::sync::Mutex::new(Instant::now() - Duration::from_secs(1));
        self.watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if res.is_err() {
                return;
            }
            let mut l = last.lock().unwrap();
            if l.elapsed() > Duration::from_millis(200) {
                *l = Instant::now();
                emit("fs:changed", Value::Null);
            }
        })
        .ok()
        .and_then(|mut w| w.watch(Path::new(dir), RecursiveMode::Recursive).ok().map(|_| w));
    }

    fn list(&mut self, dir: &str) -> Vec<(String, String, bool)> {
        if let Some(v) = self.tree.get(dir) {
            return v.clone();
        }
        let items: Vec<(String, String, bool)> = fsops::list(dir)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .map(|e| (e["name"].as_str().unwrap_or("").to_string(), e["path"].as_str().unwrap_or("").to_string(), e["dir"].as_bool().unwrap_or(false)))
            .collect();
        self.tree.insert(dir.to_string(), items.clone());
        items
    }

    fn open_file(&mut self, path: &str) {
        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
            self.active = i;
            return;
        }
        match fsops::read(path, true) {
            Ok(v) => {
                let text = v.as_str().unwrap_or("").replace("\r\n", "\n");
                self.tabs.push(Tab { path: path.to_string(), saved: text.clone(), text });
                self.active = self.tabs.len() - 1;
            }
            Err(e) => self.status = e,
        }
    }

    fn save(&mut self, i: usize) {
        if let Some(t) = self.tabs.get_mut(i) {
            match fsops::write(&t.path, &t.text) {
                Ok(_) => t.saved = t.text.clone(),
                Err(e) => self.status = e,
            }
        }
    }

    fn run(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.save(self.active);
        let tab = &self.tabs[self.active];
        let path = tab.path.clone();
        if matches!(tab.ext().as_str(), "py" | "pyw") && self.python.is_none() {
            let ws = self.workspace();
            self.python = python::find(ws.as_deref(), None)["path"].as_str().map(String::from);
            if self.python.is_none() {
                self.out.feed("\r\n\x1b[31mPython was not found. Install it from python.org or the Microsoft Store.\x1b[0m\r\n");
                return;
            }
        }
        self.bottom = Bottom::Output;
        self.out.clear();
        let r = runner::run_file(&self.out.pty, &self.emit, &path, self.python.as_deref().unwrap_or(""), "");
        if r["ok"] == Value::Bool(false) {
            self.out.feed(&format!("\x1b[31m{}\x1b[0m\r\n", r["error"].as_str().unwrap_or("Can't run this file.")));
        }
    }

    fn events(&mut self) {
        while let Ok((ch, v)) = self.rx.try_recv() {
            match ch.as_str() {
                "run:start" => {
                    self.running = true;
                    self.out.feed(&format!("\x1b[90m\u{25B6} {}\x1b[0m\r\n", v["label"].as_str().unwrap_or("")));
                }
                "run:data" => self.out.feed(v.as_str().unwrap_or("")),
                "run:exit" => {
                    self.running = false;
                    let code = v["code"].as_i64().unwrap_or(-1);
                    if let Some(e) = v["error"].as_str() {
                        self.out.feed(&format!("\r\n\x1b[31m{e}\x1b[0m\r\n"));
                    }
                    let (color, word) = if code == 0 { ("32", "Finished") } else { ("31", "Exited with code") };
                    let tail = if code == 0 { String::new() } else { format!(" {code}") };
                    self.out.feed(&format!("\r\n\x1b[{color}m{word}{tail}\x1b[0m \x1b[90min {:.2} s\x1b[0m\r\n", v["ms"].as_u64().unwrap_or(0) as f64 / 1000.0));
                }
                "shell:data" => self.sh.feed(v.as_str().unwrap_or("")),
                "shell:exit" => self.shell_started = false,
                "fs:changed" => {
                    self.tree.clear();
                    // otvorené neupravené súbory sa načítajú znova, ak ich zmenil iný program
                    for t in self.tabs.iter_mut() {
                        if !t.dirty() {
                            if let Ok(v) = fsops::read(&t.path, true) {
                                let text = v.as_str().unwrap_or("").replace("\r\n", "\n");
                                if text != t.saved {
                                    t.text = text.clone();
                                    t.saved = text;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn start_shell(&mut self) {
        if self.shell_started {
            return;
        }
        let cmd = runner::shell_command();
        let cwd = self.workspace().or_else(|| dirs::home_dir().map(|h| h.to_string_lossy().to_string())).unwrap_or_default();
        self.shell_started = self.sh.pty.spawn(&self.emit, &cmd.cmd, &cmd.args, &cwd, &[], "shell:data", "shell:exit").is_ok();
    }

    fn run_tests(&mut self, ctx: &egui::Context) {
        let t = self.started.elapsed().as_secs_f32();
        while let Some((at, action)) = self.test.first().cloned() {
            if at > t {
                ctx.request_repaint_after(Duration::from_millis(100));
                break;
            }
            self.test.remove(0);
            let (a, arg) = action.split_once('=').unwrap_or((action.as_str(), ""));
            match a {
                "open" => {
                    let p = self.workspace().map(|w| Path::new(&w).join(arg).to_string_lossy().to_string()).unwrap_or_default();
                    self.open_file(&p);
                }
                "run" => self.run(),
                "type" => self.out.pty.write(&arg.replace("\\r", "\r")),
                "terminal" => {
                    self.bottom = Bottom::Terminal;
                    self.start_shell();
                }
                "shell" => self.sh.pty.write(&arg.replace("\\r", "\r")),
                _ => {}
            }
        }
    }

    // ---------- časti okna ----------
    fn top_bar(&mut self, root: &mut egui::Ui) {
        let p = self.pal;
        egui::Panel::top("top").frame(Frame::new().fill(p.base).inner_margin(Margin::symmetric(12, 8))).show(root, |ui| {
            ui.horizontal(|ui| {
                widgets::logo(ui, &p);
                ui.label(RichText::new("flux").strong().size(16.0));
                ui.add_space(16.0);
                let mut close = None;
                for (i, t) in self.tabs.iter().enumerate() {
                    let sel = i == self.active;
                    let label = format!("{}{}", t.name(), if t.dirty() { "  \u{2022}" } else { "" });
                    let r = ui.add(egui::Button::selectable(sel, RichText::new(label).size(13.0)));
                    if r.clicked() {
                        self.active = i;
                    }
                    if r.middle_clicked() {
                        close = Some(i);
                    }
                    if sel && ui.small_button("\u{00D7}").clicked() {
                        close = Some(i);
                    }
                }
                if let Some(i) = close {
                    self.tabs.remove(i);
                    self.active = self.active.min(self.tabs.len().saturating_sub(1));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.running {
                        if ui.add(egui::Button::new(RichText::new("\u{25A0} Stop").color(p.red))).clicked() {
                            self.out.pty.kill();
                        }
                    }
                    let run = egui::Button::new(RichText::new("\u{25B6}  Run").color(p.accent_fg).strong()).fill(p.accent).corner_radius(CornerRadius::same(9));
                    if ui.add_enabled(!self.tabs.is_empty(), run).on_hover_text("F5").clicked() {
                        self.run();
                    }
                });
            });
        });
    }

    fn tree_ui(&mut self, ui: &mut egui::Ui, dir: &str, depth: usize) {
        let p = self.pal;
        let items = self.list(dir);
        for (name, path, is_dir) in items {
            let indent = 4.0 + depth as f32 * 14.0;
            let open = self.open_dirs.contains(&path);
            let active = self.tabs.get(self.active).map(|t| t.path == path).unwrap_or(false);
            let ext = Path::new(&name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            let icon = if is_dir { Icon::Folder(open) } else { Icon::File(&ext) };
            if widgets::row(ui, active, indent, icon, &name, None, &p).clicked() {
                if is_dir {
                    if open {
                        self.open_dirs.remove(&path);
                    } else {
                        self.open_dirs.insert(path.clone());
                    }
                } else {
                    self.open_file(&path);
                }
            }
            if is_dir && self.open_dirs.contains(&path) {
                self.tree_ui(ui, &path, depth + 1);
            }
        }
    }

    fn sidebar(&mut self, root: &mut egui::Ui) {
        let p = self.pal;
        egui::Panel::left("side").resizable(true).default_size(240.0).frame(Frame::new().fill(p.base).inner_margin(Margin::symmetric(8, 6))).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.label(RichText::new("PROJECTS").size(11.0).color(p.text2).strong());
                let ws = self.workspace();
                let mut open = None;
                for pr in &self.projects {
                    if pr["hidden"].as_bool() == Some(true) {
                        continue;
                    }
                    let dir = pr["dir"].as_str().unwrap_or("").to_string();
                    let name = pr["name"].as_str().unwrap_or("").to_string();
                    let pinned = pr["pinned"].as_bool() == Some(true);
                    let sel = ws.as_deref() == Some(dir.as_str());
                    let r = widgets::row(ui, sel, 0.0, Icon::Project(pinned), &name, None, &p).on_hover_text(&dir);
                    if r.clicked() && !sel {
                        open = Some(dir);
                    }
                }
                if let Some(d) = open {
                    self.open_folder(&d);
                }
                if ui.button(RichText::new("+  Open folder\u{2026}").color(p.text2)).clicked() {
                    if let Some(d) = rfd::FileDialog::new().set_title("Open folder").pick_folder() {
                        self.open_folder(&d.to_string_lossy());
                    }
                }
                if let Some(w) = ws {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(RichText::new(Path::new(&w).file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default()).size(11.0).color(p.text2).strong());
                    self.tree_ui(ui, &w, 0);
                }
            });
        });
    }

    fn status_bar(&mut self, root: &mut egui::Ui) {
        let p = self.pal;
        egui::Panel::bottom("status").frame(Frame::new().fill(p.base).inner_margin(Margin::symmetric(14, 4))).show(root, |ui| {
            ui.horizontal(|ui| {
                widgets::dot(ui, if self.running { p.green } else { p.text2 });
                ui.label(RichText::new(if self.running { "Running" } else { "Ready" }).size(12.0).color(p.text2));
                if let Some(py) = &self.python {
                    ui.label(RichText::new(format!("Python  {}", Path::new(py).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())).size(12.0).color(p.text2));
                }
                if !self.status.is_empty() {
                    ui.label(RichText::new(&self.status).size(12.0).color(p.red));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(t) = self.tabs.get(self.active) {
                        ui.label(RichText::new(format!("Ln {}, Col {}", self.cursor.0, self.cursor.1)).size(12.0).color(p.text2));
                        ui.label(RichText::new(lang_name(&t.ext())).size(12.0).color(p.text2));
                    }
                    if self.core.setting("autoSave").as_bool() != Some(false) {
                        ui.label(RichText::new("Auto save").size(12.0).color(p.text2));
                    }
                });
            });
        });
    }

    fn bottom_panel(&mut self, root: &mut egui::Ui) {
        let p = self.pal;
        egui::Panel::bottom("panel")
            .resizable(true)
            .default_size(220.0)
            .frame(Frame::new().fill(Color32::TRANSPARENT).inner_margin(Margin::symmetric(10, 6)))
            .show(root, |ui| {
                // čiara nad panelom
                let r = ui.max_rect();
                ui.painter().hline(r.x_range(), r.top() - 6.0, Stroke::new(1.0, p.line));
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::selectable(self.bottom == Bottom::Output, "Output")).clicked() {
                        self.bottom = Bottom::Output;
                    }
                    if ui.add(egui::Button::selectable(self.bottom == Bottom::Terminal, "Terminal")).clicked() {
                        self.bottom = Bottom::Terminal;
                        self.start_shell();
                        self.sh.focus(ui.ctx());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.bottom == Bottom::Output && ui.small_button("Clear").clicked() {
                            self.out.clear();
                        }
                    });
                });
                if self.bottom == Bottom::Output {
                    self.out.show(ui, &p);
                } else {
                    self.sh.show(ui, &p);
                }
            });
    }

    fn editor(&mut self, root: &mut egui::Ui) {
        let p = self.pal;
        egui::CentralPanel::default().frame(Frame::new().inner_margin(Margin::same(4))).show(root, |ui| {
            if self.tabs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.35);
                    ui.label(RichText::new("Flux").size(34.0).strong());
                    ui.label(RichText::new("Open a file from the sidebar, or a folder with Ctrl+O.").color(p.text2));
                });
                return;
            }
            let font = FontId::monospace(14.0);
            let theme = if p.dark { egui_extras::syntax_highlighting::CodeTheme::dark(14.0) } else { egui_extras::syntax_highlighting::CodeTheme::light(14.0) };
            let tab = &mut self.tabs[self.active];
            let ext = tab.ext();
            let lang = if ext.is_empty() { "txt".to_string() } else { ext };
            let lines = tab.text.lines().count().max(1) + usize::from(tab.text.ends_with('\n'));
            let mut edited = false;
            let mut cursor = None;
            egui::ScrollArea::both().auto_shrink(false).id_salt(&tab.path).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    // čísla riadkov
                    let nums: String = (1..=lines).map(|n| format!("{n:>4}\n")).collect();
                    ui.add_space(6.0);
                    ui.label(RichText::new(nums).font(font.clone()).color(p.text2.gamma_multiply(0.7)));
                    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
                        let mut job = egui_extras::syntax_highlighting::highlight(ui.ctx(), ui.style(), &theme, buf.as_str(), &lang);
                        job.wrap.max_width = wrap;
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let out = egui::TextEdit::multiline(&mut tab.text)
                        .code_editor()
                        .font(font.clone())
                        .frame(Frame::NONE)
                        .desired_width(f32::INFINITY)
                        .desired_rows(lines.max(30))
                        .lock_focus(true)
                        .layouter(&mut layouter)
                        .show(ui);
                    edited = out.response.changed();
                    if let Some(r) = out.cursor_range {
                        let idx = r.primary.index;
                        let before: String = tab.text.chars().take(idx.into()).collect();
                        let ln = before.matches('\n').count() + 1;
                        let col = before.rsplit('\n').next().map(|s| s.chars().count()).unwrap_or(0) + 1;
                        cursor = Some((ln, col));
                    }
                });
            });
            if edited {
                self.last_edit = Some(Instant::now());
            }
            if let Some(c) = cursor {
                self.cursor = c;
            }
        });
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let (save, run, open, close) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
                i.consume_key(egui::Modifiers::NONE, egui::Key::F5),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::O),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::W),
            )
        });
        if save {
            self.save(self.active);
        }
        if run {
            self.run();
        }
        if open {
            if let Some(d) = rfd::FileDialog::new().set_title("Open folder").pick_folder() {
                self.open_folder(&d.to_string_lossy());
            }
        }
        if close && !self.tabs.is_empty() {
            self.tabs.remove(self.active);
            self.active = self.active.min(self.tabs.len().saturating_sub(1));
        }
    }
}

fn lang_name(ext: &str) -> &'static str {
    match ext {
        "py" | "pyw" => "Python",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" => "TypeScript",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "json" => "JSON",
        "md" => "Markdown",
        "c" => "C",
        "cpp" | "cc" | "cxx" | "h" | "hpp" => "C++",
        "rs" => "Rust",
        "java" => "Java",
        "go" => "Go",
        "cs" => "C#",
        _ => "Plain text",
    }
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let ctx = &ctx;
        self.events();
        self.run_tests(ctx);
        self.shortcuts(ctx);
        // automatické ukladanie 1 s po poslednej zmene
        if let Some(t) = self.last_edit {
            if t.elapsed() >= Duration::from_millis(1000) {
                self.last_edit = None;
                if self.core.setting("autoSave").as_bool() != Some(false) {
                    self.save(self.active);
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(1000) - t.elapsed());
            }
        }
        self.top_bar(root);
        self.status_bar(root);
        self.sidebar(root);
        // editor a Výstup/Terminál v jednej zaoblenej karte (ako #card vo Fluxe)
        let p = self.pal;
        egui::CentralPanel::default().frame(Frame::new().fill(p.base).inner_margin(Margin { left: 0, right: 10, top: 0, bottom: 4 })).show(root, |ui| {
            Frame::new().fill(p.card).corner_radius(CornerRadius::same(14)).stroke(Stroke::new(1.0, p.line)).inner_margin(Margin::same(2)).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.bottom_panel(ui);
                self.editor(ui);
            });
        });
        let _ = settings::platform();
        let _ = Color32::TRANSPARENT;
    }
}
