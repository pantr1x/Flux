// Flux Native – okno v rovnakom rozložení ako Electron Flux: bočný panel (logo, projekty, voľné súbory, strom, Nastavenia),
// horná lišta (späť/dopredu, karty súborov, hľadanie, AI, ▶ Run), zaoblená karta s editorom + minimapou,
// Výstupom/Terminálom a stavovým riadkom; bez otvoreného súboru stránka projektu. Logika je v flux-core.
mod intro;

use crate::code::Code;
use crate::i18n::t;
use crate::term::TermView;
use crate::theme::{self, Pal, GAP, RADIUS, SIDE_W, TOP_H};
use crate::widgets::{self, Lead};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind};
use flux_core::{fsops, python, runner, settings, Core, Emit};
use notify::{RecursiveMode, Watcher};
use serde_json::{json, Value};
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
        widgets::file_name(&self.path)
    }
    fn ext(&self) -> String {
        ext_of(&self.path)
    }
    fn dirty(&self) -> bool {
        self.text != self.saved
    }
}

fn ext_of(p: &str) -> String {
    Path::new(p).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

#[derive(PartialEq, Clone, Copy)]
enum Bottom {
    Output,
    Terminal,
}

pub struct App {
    core: Arc<Core>,
    emit: Emit,
    rx: Receiver<(String, Value)>,
    pal: Pal,
    code: Code,
    tabs: Vec<Tab>,
    active: usize,
    home: bool, // stránka projektu namiesto editora
    hist: Vec<String>,
    hist_i: usize,
    tree: HashMap<String, Vec<(String, String, bool)>>, // priečinok → (meno, cesta, je priečinok)
    open_dirs: HashSet<String>,
    projects: Vec<Value>,
    summaries: HashMap<String, Value>,
    recent_files: Vec<String>,
    show_hidden: bool,
    new_project: Option<String>,
    new_item: Option<(bool, String)>, // (priečinok?, meno) pre nový súbor/priečinok v strome
    new_todo: String,
    side_open: bool,
    out: TermView,
    sh: TermView,
    shell_started: bool,
    bottom: Bottom,
    panel_open: bool,
    panel_h: f32,
    running: bool,
    python: Option<Value>,
    last_edit: Option<Instant>,
    status: String,
    watcher: Option<notify::RecommendedWatcher>,
    test: Vec<(f32, String)>,
    started: Instant,
    cursor: (usize, usize),
    scroll_to: Option<f32>,
    intro: Option<intro::Intro>,
    trim: crate::mem::Trim,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let core = Arc::new(Core::load());
        // svetlá/tmavá podľa témy kódu – ako isDark() v Electron Fluxe
        let code = Code::new(core.setting("codeTheme").as_str().unwrap_or(""));
        let pal = theme::palette_for(code.dark, core.setting("accent").as_str().unwrap_or("mono"), core.setting("darkLift").as_f64().unwrap_or(0.0));
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
            code,
            tabs: vec![],
            active: 0,
            home: true,
            hist: vec![],
            hist_i: 0,
            tree: HashMap::new(),
            open_dirs: HashSet::new(),
            projects: vec![],
            summaries: HashMap::new(),
            recent_files: vec![],
            show_hidden: false,
            new_project: None,
            new_item: None,
            new_todo: String::new(),
            side_open: true,
            out: TermView::new("flux-output"),
            sh: TermView::new("flux-shell"),
            shell_started: false,
            bottom: Bottom::Output,
            panel_open: true,
            panel_h: 190.0,
            running: false,
            python: None,
            last_edit: None,
            status: String::new(),
            watcher: None,
            test: vec![],
            started: Instant::now(),
            cursor: (1, 1),
            scroll_to: None,
            intro: None,
            trim: Default::default(),
        };
        crate::i18n::set_language(app.core.setting("language").as_str().unwrap_or("en"));
        if app.core.setting("onboarded").as_bool() != Some(true) || std::env::var("FLUX_INTRO").is_ok() {
            app.intro = Some(intro::Intro::new(app.core.setting("userName").as_str().unwrap_or("")));
        }
        app.out.feed(&format!("\x1b[90m{}\x1b[0m\r\n", t("Program output appears here. Press F5 or ▶ Run.")));
        app.reload_projects();
        if let Some(last) = app.core.setting("lastFolder").as_str().map(String::from) {
            app.open_folder(&last);
        }
        // súhrny všetkých projektov (jazyky, počet súborov) a Python – na pozadí, okno sa ukáže hneď
        let dirs: Vec<String> = app.projects.iter().filter_map(|p| p["dir"].as_str().map(String::from)).collect();
        let emit = app.emit.clone();
        let ws = app.workspace();
        std::thread::spawn(move || {
            emit("py:info", python::find(ws.as_deref(), None));
            for d in dirs {
                emit("proj:summary", json!({ "dir": d, "s": fsops::summary(&d) }));
            }
        });
        // testy: FLUX_TEST="2:open=main.py;4:run;7:type=Rust\r" (čas v s : akcia)
        if let Ok(t) = std::env::var("FLUX_TEST") {
            app.test = t.split(';').filter_map(|p| p.split_once(':').map(|(a, b)| (a.parse().unwrap_or(0.0), b.to_string()))).collect();
        }
        app
    }

    fn palette(&self) -> Pal {
        theme::palette_for(self.code.dark, self.core.setting("accent").as_str().unwrap_or("mono"), self.core.setting("darkLift").as_f64().unwrap_or(0.0))
    }

    // znova načíta farby z nastavení (accent, darkLift)
    fn apply_look(&mut self, ctx: &egui::Context) {
        self.pal = self.palette();
        theme::apply(ctx, &self.pal);
    }

    fn workspace(&self) -> Option<String> {
        self.core.workspace.lock().unwrap().clone()
    }

    fn reload_projects(&mut self) {
        self.projects = fsops::projects(&self.core).as_array().cloned().unwrap_or_default();
        self.recent_files = fsops::recent_files(&self.core).as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    }

    fn update_settings(&self, f: impl FnOnce(&mut serde_json::Map<String, Value>)) {
        let mut set = self.core.settings.lock().unwrap();
        if let Some(o) = set.as_object_mut() {
            f(o);
        }
        settings::save(&set);
    }

    fn summarize(&self, dir: &str) {
        let emit = self.emit.clone();
        let d = dir.to_string();
        std::thread::spawn(move || emit("proj:summary", json!({ "dir": d, "s": fsops::summary(&d) })));
    }

    fn open_folder(&mut self, dir: &str) {
        if fsops::open_workspace(&self.core, dir).is_null() {
            return;
        }
        self.tree.clear();
        self.open_dirs.clear();
        self.reload_projects();
        self.home = true;
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
        self.summarize(dir);
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

    fn activate(&mut self, i: usize) {
        self.active = i;
        self.home = false;
        if let Some(t) = self.tabs.get(i) {
            let p = t.path.clone();
            if self.hist.get(self.hist_i) != Some(&p) {
                self.hist.truncate(self.hist_i + 1);
                self.hist.push(p);
                self.hist_i = self.hist.len() - 1;
            }
        }
    }

    fn open_file(&mut self, path: &str) {
        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
            self.activate(i);
            return;
        }
        match fsops::read(path, true) {
            Ok(v) => {
                let text = v.as_str().unwrap_or("").replace("\r\n", "\n");
                self.tabs.push(Tab { path: path.to_string(), saved: text.clone(), text });
                self.activate(self.tabs.len() - 1);
            }
            Err(e) => self.status = e,
        }
    }

    fn go(&mut self, back: bool) {
        let i = if back { self.hist_i.checked_sub(1) } else { Some(self.hist_i + 1).filter(|i| *i < self.hist.len()) };
        if let Some(i) = i {
            self.hist_i = i;
            let p = self.hist[i].clone();
            if let Some(t) = self.tabs.iter().position(|t| t.path == p) {
                self.active = t;
                self.home = false;
            } else {
                self.open_file(&p);
            }
        }
    }

    fn close_tab(&mut self, i: usize) {
        if i < self.tabs.len() {
            if self.tabs[i].dirty() {
                self.save(i);
            }
            self.tabs.remove(i);
            if self.active > i || self.active >= self.tabs.len() {
                self.active = self.active.saturating_sub(1);
            }
            if self.tabs.is_empty() {
                self.home = true;
            }
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
        if self.tabs.is_empty() || self.home {
            return;
        }
        self.save(self.active);
        let tab = &self.tabs[self.active];
        let path = tab.path.clone();
        if matches!(tab.ext().as_str(), "py" | "pyw") && self.python.as_ref().and_then(|v| v["path"].as_str()).is_none() {
            let ws = self.workspace();
            let v = python::find(ws.as_deref(), None);
            self.python = if v["path"].is_string() { Some(v) } else { None };
            if self.python.is_none() {
                self.out.feed("\r\n\x1b[31mPython was not found. Install it from python.org or the Microsoft Store.\x1b[0m\r\n");
                return;
            }
        }
        self.bottom = Bottom::Output;
        self.panel_open = true;
        self.out.clear();
        let py = self.python.as_ref().and_then(|v| v["path"].as_str()).unwrap_or("").to_string();
        let r = runner::run_file(&self.out.pty, &self.emit, &path, &py, "");
        if r["ok"] == Value::Bool(false) {
            self.out.feed(&format!("\x1b[31m{}\x1b[0m\r\n", r["error"].as_str().unwrap_or("Can't run this file.")));
        } else if let Some(ws) = self.workspace() {
            // počítadlo spustení – to isté ako activity.js v Electron Fluxe
            self.update_settings(|o| {
                let mut all = o.get("activity").cloned().unwrap_or(json!({}));
                let runs = all[&ws]["runs"].as_u64().unwrap_or(0) + 1;
                all[&ws]["runs"] = json!(runs);
                o.insert("activity".into(), all);
            });
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
                    let (color, word) = if code == 0 { ("32", t("Finished")) } else { ("31", crate::i18n::tf("Exited with code {code}", &[("code", &code.to_string())])) };
                    self.out.feed(&format!("\r\n\x1b[{color}m{word}\x1b[0m \x1b[90min {:.2} s\x1b[0m\r\n", v["ms"].as_u64().unwrap_or(0) as f64 / 1000.0));
                }
                "shell:data" => self.sh.feed(v.as_str().unwrap_or("")),
                "shell:exit" => self.shell_started = false,
                "py:info" => {
                    if v["path"].is_string() {
                        self.python = Some(v);
                    }
                }
                "proj:summary" => {
                    if let Some(d) = v["dir"].as_str() {
                        self.summaries.insert(d.to_string(), v["s"].clone());
                    }
                }
                "fs:changed" => {
                    self.tree.clear();
                    if let Some(w) = self.workspace() {
                        self.summarize(&w);
                    }
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
                "home" => self.home = true,
                "light" | "dark" => self.set_theme(a == "dark", ctx),
                "theme" => self.set_code_theme(arg, ctx),
                "next" => {
                    if let Some(i) = self.intro.as_mut() {
                        i.step = (i.step + 1).min(intro::STEPS.len() - 1);
                    }
                }
                _ => {}
            }
        }
    }

    // prepne na naposledy použitú tmavú/svetlú tému kódu (toggleTheme v app.js)
    fn set_theme(&mut self, dark: bool, ctx: &egui::Context) {
        let want = if dark { "lastDark" } else { "lastLight" };
        let last = self.core.setting(want).as_str().map(String::from).filter(|id| crate::gen::code_theme(id).map(|t| t.0 == dark).unwrap_or(false));
        let next = last.unwrap_or_else(|| if dark { crate::gen::DEFAULT_THEME.into() } else { "vscode-light".into() });
        self.set_code_theme(&next, ctx);
    }

    fn set_code_theme(&mut self, id: &str, ctx: &egui::Context) {
        self.code = Code::new(id);
        self.pal = self.palette();
        theme::apply(ctx, &self.pal);
        let dark = self.code.dark;
        let id = id.to_string();
        self.update_settings(|o| {
            o.insert(if dark { "lastDark" } else { "lastLight" }.into(), json!(id));
            o.insert("codeTheme".into(), json!(id));
        });
    }

    // ---------- bočný panel ----------
    fn side_top(&mut self, ui: &mut egui::Ui, with_toggle: bool) {
        let p = self.pal;
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), TOP_H), Sense::hover());
        let brand = Rect::from_min_size(pos2(r.left() + 2.0, r.center().y - 14.0), vec2(70.0, 28.0));
        let resp = ui.interact(brand, ui.id().with("brand"), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(brand, CornerRadius::same(8), p.hover);
        }
        if resp.clicked() {
            self.home = true;
        }
        widgets::brand_mark(ui, Rect::from_min_size(pos2(brand.left() + 4.0, r.center().y - 10.0), vec2(20.0, 20.0)), &p);
        ui.painter().text(pos2(brand.left() + 32.0, r.center().y), Align2::LEFT_CENTER, "flux", theme::bold(15.0), p.text);
        widgets::icon_button_at(ui, Rect::from_center_size(pos2(brand.right() + 17.0, r.center().y), vec2(28.0, 28.0)), "menu", 16.0, &p, true).on_hover_text(t("Menu"));
        if with_toggle {
            let t = Rect::from_center_size(pos2(r.right() - 14.0, r.center().y), vec2(28.0, 28.0));
            if widgets::icon_button_at(ui, t, "sidebar", 16.0, &p, true).on_hover_text(crate::i18n::t("Hide sidebar")).clicked() {
                self.side_open = false;
            }
        }
    }

    fn project_sub(&self, dir: &str) -> String {
        let Some(s) = self.summaries.get(dir) else {
            return String::new();
        };
        let langs: Vec<&str> = s["langs"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
        let files = s["files"].as_u64().unwrap_or(0);
        let mut parts = vec![];
        if let Some(main) = langs.first() {
            parts.push(if langs.len() > 1 { format!("{} +{}", kind_name(main), langs.len() - 1) } else { kind_name(main).to_string() });
        }
        parts.push(format!("{files} {}", if files == 1 { t("file") } else { t("files") }));
        let secs = self.core.setting("projectTime")[dir].as_u64().unwrap_or(0);
        if secs >= 60 {
            parts.push(format_time(secs));
        }
        parts.join(" \u{00B7} ")
    }

    fn main_kind(&self, dir: &str) -> Option<String> {
        self.summaries.get(dir).and_then(|s| s["langs"][0].as_str().map(String::from))
    }

    fn sidebar(&mut self, root: &mut egui::Ui) {
        let p = self.pal;
        egui::Panel::left("side").resizable(false).exact_size(SIDE_W).frame(Frame::new().fill(p.base).inner_margin(Margin { left: 8, right: 6, top: 0, bottom: 0 })).show(root, |ui| {
            self.side_top(ui, true);
            let ws = self.workspace();
            // ---- PROJECTS ----
            if widgets::section(ui, &t("Projects").to_uppercase(), Some("folderOpen"), &p).map(|r| r.on_hover_text(t("Open folder")).clicked()) == Some(true) {
                if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                    self.open_folder(&d.to_string_lossy());
                }
            }
            let mut open = None;
            let hidden = self.projects.iter().filter(|pr| pr["hidden"].as_bool() == Some(true)).count();
            egui::ScrollArea::vertical().id_salt("projects").max_height(200.0).auto_shrink([false, true]).show(ui, |ui| {
                for pr in &self.projects {
                    if pr["hidden"].as_bool() == Some(true) && !self.show_hidden {
                        continue;
                    }
                    let dir = pr["dir"].as_str().unwrap_or("").to_string();
                    let name = pr["name"].as_str().unwrap_or("").to_string();
                    let sel = ws.as_deref() == Some(dir.as_str());
                    let sub = self.project_sub(&dir);
                    let kind = self.main_kind(&dir);
                    let icon_name = kind.as_deref().map(kind_file).unwrap_or("");
                    let lead = if icon_name.is_empty() { Lead::Line("folder") } else { Lead::File(icon_name) };
                    let r = widgets::row(ui, sel, 10.0, lead, &name, Some(&sub), true, &p).on_hover_text(&dir);
                    if pr["pinned"].as_bool() == Some(true) {
                        widgets::icon_at(ui, pos2(r.rect.right() - 16.0, r.rect.center().y), 12.0, "pin", p.text3);
                    }
                    if r.clicked() {
                        open = Some(dir);
                    }
                }
            });
            if let Some(d) = open {
                if ws.as_deref() == Some(d.as_str()) {
                    self.home = true;
                } else {
                    self.open_folder(&d);
                }
            }
            if let Some(name) = &mut self.new_project {
                let r = ui.add(egui::TextEdit::singleline(name).hint_text("Project name").desired_width(f32::INFINITY).margin(Margin::symmetric(10, 6)));
                r.request_focus();
                if r.lost_focus() {
                    let n = name.trim().to_string();
                    self.new_project = None;
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !n.is_empty() {
                        match fsops::create_project(&n, "") {
                            Ok(d) => self.open_folder(d.as_str().unwrap_or("")),
                            Err(e) => self.status = e,
                        }
                    }
                }
            } else if widgets::row(ui, false, 10.0, Lead::Line("plus"), &t("New project"), None, false, &p).clicked() {
                self.new_project = Some(String::new());
            }
            if hidden > 0
                && widgets::row(ui, false, 10.0, Lead::Line(if self.show_hidden { "eyeOff" } else { "eye" }), &crate::i18n::tf("{n} hidden", &[("n", &hidden.to_string())]), None, false, &p).clicked()
            {
                self.show_hidden = !self.show_hidden;
            }
            // ---- FILES (súbory otvorené mimo projektu) ----
            if !self.recent_files.is_empty() {
                widgets::separator(ui, &p);
                if widgets::section(ui, &t("Files").to_uppercase(), Some("filePlus"), &p).map(|r| r.on_hover_text(t("New file")).clicked()) == Some(true) {
                    if let Some(f) = rfd::FileDialog::new().set_title(t("New file")).save_file() {
                        let f = f.to_string_lossy().to_string();
                        if fsops::create(&f, false).is_ok() {
                            fsops::allow_file(&self.core, &f);
                            self.reload_projects();
                            self.open_file(&f);
                        }
                    }
                }
                let mut open_f = None;
                egui::ScrollArea::vertical().id_salt("loose").max_height(170.0).auto_shrink([false, true]).show(ui, |ui| {
                    for f in &self.recent_files {
                        let sel = !self.home && self.tabs.get(self.active).map(|t| &t.path == f).unwrap_or(false);
                        if widgets::row(ui, sel, 10.0, Lead::File(&widgets::file_name(f)), &widgets::file_name(f), Some(&short_dir(f)), false, &p).on_hover_text(f).clicked() {
                            open_f = Some(f.clone());
                        }
                    }
                });
                if let Some(f) = open_f {
                    self.open_file(&f);
                }
            }
            // ---- strom súborov projektu ----
            let footer_h = 36.0;
            if let Some(w) = ws.clone() {
                widgets::separator(ui, &p);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.add_space(4.0);
                    if widgets::icon_button(ui, "filePlus", &p, true).on_hover_text(t("New file")).clicked() {
                        self.new_item = Some((false, String::new()));
                    }
                    if widgets::icon_button(ui, "folderPlus", &p, true).on_hover_text(t("New folder")).clicked() {
                        self.new_item = Some((true, String::new()));
                    }
                    if widgets::icon_button(ui, "refresh", &p, true).on_hover_text(t("Refresh")).clicked() {
                        self.tree.clear();
                    }
                    if widgets::icon_button(ui, "collapse", &p, true).on_hover_text(t("Collapse all")).clicked() {
                        self.open_dirs.clear();
                    }
                });
                ui.add_space(4.0);
                let h = (ui.available_height() - footer_h).max(40.0);
                egui::ScrollArea::vertical().id_salt("tree").max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                    if let Some((is_dir, name)) = &mut self.new_item {
                        let r =
                            ui.add(egui::TextEdit::singleline(name).hint_text(if *is_dir { t("Folder name") } else { t("File name") }).desired_width(f32::INFINITY).margin(Margin::symmetric(10, 5)));
                        r.request_focus();
                        if r.lost_focus() {
                            let (d, n) = (*is_dir, name.trim().to_string());
                            self.new_item = None;
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !n.is_empty() {
                                let target = Path::new(&w).join(&n).to_string_lossy().to_string();
                                match fsops::create(&target, d) {
                                    Ok(_) => {
                                        self.tree.clear();
                                        if !d {
                                            self.open_file(&target);
                                        }
                                    }
                                    Err(e) => self.status = e,
                                }
                            }
                        }
                    }
                    self.tree_ui(ui, &w, 0);
                });
            } else {
                // bez priečinka: „No folder is open.“ + Open folder
                widgets::separator(ui, &p);
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(t("No folder is open.")).color(p.text3).size(12.5));
                    ui.add_space(6.0);
                    if widgets::button(ui, None, &t("Open folder"), p.accent, p.accent_fg, 28.0, &p).clicked() {
                        if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                            self.open_folder(&d.to_string_lossy());
                        }
                    }
                });
                ui.add_space((ui.available_height() - footer_h).max(0.0));
            }
            // ---- päta: Nastavenia, skratky, svetlá/tmavá ----
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), footer_h), Sense::hover());
            ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, p.line));
            let s = Rect::from_min_size(pos2(r.left(), r.top() + 5.0), vec2(96.0, 28.0));
            let sr = ui.interact(s, ui.id().with("settings"), Sense::click());
            if sr.hovered() {
                ui.painter().rect_filled(s, CornerRadius::same(8), p.hover);
            }
            widgets::icon_at(ui, pos2(s.left() + 15.0, s.center().y), 15.0, "settings", p.text2);
            ui.painter().text(pos2(s.left() + 31.0, s.center().y), Align2::LEFT_CENTER, t("Settings"), theme::bold(13.0), p.text2);
            if sr.on_hover_text(t("Settings are coming to Flux Native soon")).clicked() {
                self.status = "Settings are coming to Flux Native soon.".into();
            }
            let sun = Rect::from_center_size(pos2(r.right() - 12.0, s.center().y), vec2(26.0, 26.0));
            let cmd = sun.translate(vec2(-26.0, 0.0));
            widgets::icon_button_at(ui, cmd, "command", 15.0, &p, true).on_hover_text(t("F5 Run · Ctrl+S Save · Ctrl+O Open folder · Ctrl+W Close"));
            if widgets::icon_button_at(ui, sun, if p.dark { "sun" } else { "moon" }, 15.0, &p, true).on_hover_text(t("Light / dark")).clicked() {
                self.set_theme(!p.dark, ui.ctx());
            }
        });
    }

    fn tree_ui(&mut self, ui: &mut egui::Ui, dir: &str, depth: usize) {
        let p = self.pal;
        let items = self.list(dir);
        for (name, path, is_dir) in items {
            let indent = 6.0 + depth as f32 * 14.0;
            let open = self.open_dirs.contains(&path);
            let active = !self.home && self.tabs.get(self.active).map(|t| t.path == path).unwrap_or(false);
            let dirty = self.tabs.iter().any(|t| t.path == path && t.dirty());
            let lead = if is_dir { Lead::Folder { open } } else { Lead::File(&name) };
            let ind = if is_dir { indent } else { indent + 20.0 };
            let r = widgets::row(ui, active, ind, lead, &name, None, false, &p);
            if dirty {
                ui.painter().circle_filled(pos2(r.rect.right() - 12.0, r.rect.center().y), 3.0, p.accent);
            }
            if r.clicked() {
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

    // ---------- horná lišta ----------
    fn top_bar(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = self.pal;
        let mut x = r.left();
        let cy = r.center().y;
        if !self.side_open {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(x + 8.0, r.top()), vec2(140.0, TOP_H))));
            self.side_top(&mut child, false);
            x += 138.0;
            if widgets::icon_button_at(ui, Rect::from_center_size(pos2(x + 14.0, cy), vec2(28.0, 28.0)), "sidebar", 16.0, &p, true).on_hover_text(t("Show sidebar")).clicked() {
                self.side_open = true;
            }
            x += 30.0;
        }
        let back = self.hist_i > 0;
        let fwd = self.hist_i + 1 < self.hist.len();
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(x + 14.0, cy), vec2(28.0, 28.0)), "arrowLeft", 16.0, &p, back).on_hover_text(t("Back")).clicked() {
            self.go(true);
        }
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(x + 44.0, cy), vec2(28.0, 28.0)), "arrowRight", 16.0, &p, fwd).on_hover_text(t("Forward")).clicked() {
            self.go(false);
        }
        x += 70.0;
        // vpravo: Stop, ▶ Run, AI, hľadanie
        let mut rx = r.right();
        let can_run = !self.tabs.is_empty() && !self.home;
        // na stránke projektu ▶ Run nie je (ako v Electron Fluxe)
        if can_run || self.running {
            let stop = Rect::from_min_size(pos2(rx - 30.0, cy - 15.0), vec2(30.0, 30.0));
            let sr = ui.interact(stop, ui.id().with("stop"), if self.running { Sense::click() } else { Sense::hover() });
            ui.painter().rect_filled(stop, CornerRadius::same(10), if self.running && sr.hovered() { p.active } else { p.hover });
            widgets::icon_at(ui, stop.center(), 12.0, "stop", if self.running { p.red } else { p.text3.gamma_multiply(0.6) });
            if self.running && sr.on_hover_text(t("Stop")).clicked() {
                self.out.pty.kill();
            }
            rx -= 37.0;
            let run_w = widgets::text_w(ui, &t("Run"), theme::bold(13.5)) + 46.0;
            let run = Rect::from_min_size(pos2(rx - run_w, cy - 15.0), vec2(run_w, 30.0));
            let rr = ui.interact(run, ui.id().with("run"), Sense::click());
            let fill = if !can_run {
                p.accent.gamma_multiply(0.45)
            } else if self.running {
                p.accent.lerp_to_gamma(p.card, 0.3)
            } else if rr.hovered() {
                p.accent.lerp_to_gamma(Color32::WHITE, 0.08)
            } else {
                p.accent
            };
            ui.painter().rect_filled(run, CornerRadius::same(10), fill);
            widgets::icon_at(ui, pos2(run.left() + 18.0, cy), 14.0, "play", p.accent_fg);
            ui.painter().text(pos2(run.left() + 32.0, cy), Align2::LEFT_CENTER, t("Run"), theme::bold(13.5), p.accent_fg);
            if can_run && rr.on_hover_text(t("Run (F5)")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.run();
            }
            rx -= run_w + 7.0;
        }
        let ai = Rect::from_min_size(pos2(rx - 56.0, cy - 15.0), vec2(56.0, 30.0));
        let ar = ui.interact(ai, ui.id().with("ai"), Sense::click());
        ui.painter().rect_filled(ai, CornerRadius::same(10), if ar.hovered() { p.active } else { p.hover });
        widgets::icon_at(ui, pos2(ai.left() + 17.0, cy), 14.0, "sparkle", p.text2);
        ui.painter().text(pos2(ai.left() + 29.0, cy), Align2::LEFT_CENTER, "AI", theme::bold(13.0), p.text2);
        ar.on_hover_text(t("Claude AI is coming to Flux Native soon"));
        rx -= 63.0;
        widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx - 14.0, cy), vec2(30.0, 30.0)), "search", 16.0, &p, true).on_hover_text(t("Search (coming soon)"));
        rx -= 36.0;
        // karty súborov
        let mut close = None;
        let mut act = None;
        let tabs_clip = Rect::from_x_y_ranges(x..=rx, r.y_range());
        let painter = ui.painter().with_clip_rect(tabs_clip);
        for (i, t) in self.tabs.iter().enumerate() {
            let sel = i == self.active && !self.home;
            let name = t.name();
            let w = (widgets::text_w(ui, &name, theme::ui(13.0)) + 16.0 + 7.0 + 20.0 + 16.0).min(220.0);
            let tr = Rect::from_min_size(pos2(x, cy - 15.0), vec2(w, 30.0));
            if tr.left() > rx {
                break;
            }
            let resp = ui.interact(tr.intersect(tabs_clip), ui.id().with(("tab", i)), Sense::click());
            if sel {
                painter.add(egui::Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(if p.dark { 70 } else { 18 }) }.as_shape(tr, CornerRadius::same(9)));
                painter.rect_filled(tr, CornerRadius::same(9), p.card);
                painter.rect_stroke(tr, CornerRadius::same(9), Stroke::new(1.0, p.line), StrokeKind::Inside);
            } else if resp.hovered() {
                painter.rect_filled(tr, CornerRadius::same(9), p.hover);
            }
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(tabs_clip));
            child.set_clip_rect(tabs_clip);
            widgets::file_icon(&child, Rect::from_min_size(pos2(tr.left() + 10.0, cy - 8.0), vec2(16.0, 16.0)), &name);
            widgets::text(&child, pos2(tr.left() + 33.0, cy), Align2::LEFT_CENTER, &name, theme::ui(13.0), if sel || resp.hovered() { p.text } else { p.text2 }, w - 55.0);
            let cr = Rect::from_center_size(pos2(tr.right() - 16.0, cy), vec2(20.0, 20.0));
            let c = ui.interact(cr.intersect(tabs_clip), ui.id().with(("tabx", i)), Sense::click());
            if t.dirty() && !c.hovered() {
                painter.circle_filled(cr.center(), 3.5, p.text2);
            } else {
                if c.hovered() {
                    painter.rect_filled(cr, CornerRadius::same(6), p.active);
                }
                if sel || resp.hovered() || c.hovered() {
                    widgets::icon_at(&child, cr.center(), 12.0, "x", if c.hovered() { p.text } else { p.text3 });
                }
            }
            if c.clicked() || resp.middle_clicked() {
                close = Some(i);
            } else if resp.clicked() {
                act = Some(i);
            }
            x += w + 4.0;
        }
        if let Some(i) = close {
            self.close_tab(i);
        } else if let Some(i) = act {
            self.activate(i);
        }
    }

    // ---------- karta: editor ----------
    fn editor(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = self.pal;
        const LH: f32 = 20.0; // výška riadku ako v Monacu
        const PAD: f32 = 16.0;
        const GUTTER: f32 = 67.0;
        const MINI: f32 = 112.0;
        let font = theme::mono(14.0);
        let ed_rect = Rect::from_min_max(rect.min, pos2(rect.right() - MINI, rect.bottom()));
        let mini = Rect::from_min_max(pos2(rect.right() - MINI, rect.top()), rect.max);
        let code = &self.code;
        let tab = &mut self.tabs[self.active];
        let ext = tab.ext();
        let lang = if ext.is_empty() { "txt".to_string() } else { ext };
        let cur_line = self.cursor.0;
        let mut edited = false;
        let mut cursor = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(ed_rect));
        child.set_clip_rect(ed_rect);
        let mut sa = egui::ScrollArea::both().auto_shrink(false).id_salt(&tab.path);
        if let Some(y) = self.scroll_to.take() {
            sa = sa.vertical_scroll_offset(y.max(0.0));
        }
        let out = sa.show(&mut child, |ui| {
            ui.add_space(PAD);
            ui.horizontal_top(|ui| {
                ui.add_space(GUTTER);
                let hl = ui.painter().add(egui::Shape::Noop);
                let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
                    let mut job = code.highlight(ui.ctx(), ui.style(), buf.as_str(), &lang);
                    job.wrap.max_width = f32::INFINITY;
                    for s in job.sections.iter_mut() {
                        s.format.line_height = Some(LH);
                        s.format.valign = egui::Align::Center;
                    }
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let lines = tab.text.lines().count().max(1) + usize::from(tab.text.ends_with('\n'));
                let te = egui::TextEdit::multiline(&mut tab.text)
                    .code_editor()
                    .font(font.clone())
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .desired_width(f32::INFINITY)
                    .desired_rows(lines.max(1))
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .show(ui);
                edited = te.response.changed();
                if let Some(r) = te.cursor_range {
                    let idx = r.primary.index;
                    let before: String = tab.text.chars().take(idx.into()).collect();
                    let ln = before.matches('\n').count() + 1;
                    let col = before.rsplit('\n').next().map(|s| s.chars().count()).unwrap_or(0) + 1;
                    cursor = Some((ln, col));
                }
                // čísla riadkov, zvýraznený aktuálny riadok, vodiace čiary odsadenia
                let g = &te.galley;
                let clip = ui.clip_rect();
                let cw = ui.fonts_mut(|f| f.glyph_width(&font, ' '));
                let src_lines: Vec<&str> = tab.text.split('\n').collect();
                let num_x = te.galley_pos.x - GUTTER + 45.0;
                for (i, row) in g.rows.iter().enumerate() {
                    let rr = row.rect().translate(te.galley_pos.to_vec2());
                    if rr.bottom() < clip.top() || rr.top() > clip.bottom() {
                        continue;
                    }
                    let n = i + 1;
                    if n == cur_line {
                        let band = Rect::from_x_y_ranges(clip.left()..=clip.right(), rr.top()..=rr.top() + LH);
                        ui.painter().set(hl, egui::Shape::rect_filled(band, 0.0, if p.dark { Color32::from_white_alpha(7) } else { Color32::from_black_alpha(8) }));
                    }
                    let c = if n == cur_line { p.text } else { p.text3.gamma_multiply(0.75) };
                    ui.painter().text(pos2(num_x, rr.top() + LH / 2.0), Align2::RIGHT_CENTER, n.to_string(), theme::mono(13.0), c);
                    if let Some(l) = src_lines.get(i) {
                        let spaces = l.chars().take_while(|c| *c == ' ').count();
                        for k in 1..=(spaces / 4) {
                            let gx = te.galley_pos.x + ((k - 1) * 4) as f32 * cw + 0.5;
                            if k * 4 <= spaces && k > 0 {
                                ui.painter().vline(gx, rr.top()..=rr.top() + LH, Stroke::new(1.0, p.line));
                            }
                        }
                    }
                }
                let _ = lines;
            });
            ui.add_space(PAD);
        });
        if edited {
            self.last_edit = Some(Instant::now());
        }
        if let Some(c) = cursor {
            self.cursor = c;
        }
        // ---- minimapa (ako v Monacu: znak = 1 px, riadok = 2 px) ----
        ui.painter().vline(mini.left(), mini.y_range(), Stroke::new(1.0, p.line));
        let tab = &self.tabs[self.active];
        let job = self.code.highlight(ui.ctx(), ui.style(), &tab.text, &lang);
        let painter = ui.painter().with_clip_rect(mini);
        let (mut line, mut col) = (0usize, 0usize);
        let (x0, y0) = (mini.left() + 8.0, mini.top() + 4.0);
        let max_lines = ((mini.height() - 8.0) / 2.0) as usize;
        'outer: for s in &job.sections {
            let color = s.format.color.gamma_multiply(0.75);
            let mut run: Option<usize> = None;
            for ch in job.text[s.byte_range.start.0..s.byte_range.end.0].chars() {
                if ch == '\n' || ch == ' ' || ch == '\t' {
                    if let Some(st) = run.take() {
                        painter.rect_filled(Rect::from_min_size(pos2(x0 + st as f32, y0 + line as f32 * 2.0), vec2((col - st) as f32, 1.6)), 0.0, color);
                    }
                    if ch == '\n' {
                        line += 1;
                        col = 0;
                        if line > max_lines {
                            break 'outer;
                        }
                    } else {
                        col += if ch == '\t' { 4 } else { 1 };
                    }
                    continue;
                }
                if run.is_none() {
                    run = Some(col);
                }
                col += 1;
            }
            if let Some(st) = run {
                painter.rect_filled(Rect::from_min_size(pos2(x0 + st as f32, y0 + line as f32 * 2.0), vec2((col - st) as f32, 1.6)), 0.0, color);
            }
        }
        // posuvník minimapy: viditeľná časť; klik/ťahanie posúva editor
        let mr = ui.interact(mini, ui.id().with("minimap"), Sense::click_and_drag());
        let view_h = out.inner_rect.height();
        let top_line = ((out.state.offset.y - PAD).max(0.0)) / LH;
        let slider = Rect::from_min_size(pos2(mini.left() + 1.0, y0 + top_line * 2.0), vec2(MINI - 1.0, view_h / LH * 2.0));
        if mr.hovered() || mr.dragged() {
            painter.rect_filled(slider, 0.0, p.hover);
        }
        if let Some(pos) = mr.interact_pointer_pos() {
            if mr.clicked() || mr.dragged() {
                let target_line = (pos.y - y0) / 2.0;
                self.scroll_to = Some(target_line * LH + PAD - view_h / 2.0);
                ui.ctx().request_repaint();
            }
        }
    }

    // ---------- karta: stránka projektu ----------
    fn project_page(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = self.pal;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.set_clip_rect(rect);
        let Some(ws) = self.workspace() else {
            // bez priečinka: logo, „flux“ a tri akcie (ako prázdny stav v Electron Fluxe)
            let (cx, cy) = (rect.center().x, rect.center().y - 20.0);
            let sub = t("Open a project folder and run your code with one click.");
            let tw = widgets::text_w(ui, &sub, theme::ui(12.5)).max(200.0) + 60.0;
            let x0 = cx - tw / 2.0;
            let lr = Rect::from_min_size(pos2(x0, cy - 48.0), vec2(48.0, 48.0));
            ui.painter().rect_filled(lr, CornerRadius::same(12), Color32::from_rgb(0x11, 0x11, 0x14));
            ui.painter().rect_stroke(lr, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            widgets::icon_at(ui, lr.center(), 22.0, "code", Color32::WHITE);
            ui.painter().text(pos2(lr.right() + 12.0, cy - 33.0), Align2::LEFT_CENTER, "flux", theme::bold(26.0), p.text);
            ui.painter().text(pos2(lr.right() + 12.0, cy - 10.0), Align2::LEFT_CENTER, &sub, theme::ui(12.5), p.text2);
            let mut acts = child.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(x0, cy + 14.0), vec2(tw + 60.0, 32.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            acts.spacing_mut().item_spacing.x = 6.0;
            if widgets::button(&mut acts, Some("plus"), &t("New project"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                self.new_project = Some(String::new());
            }
            if widgets::button(&mut acts, Some("folderOpen"), &t("Open folder"), p.card2, p.text, 30.0, &p).clicked() {
                if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                    self.open_folder(&d.to_string_lossy());
                }
            }
            if widgets::button(&mut acts, Some("file"), &t("Open file"), p.card2, p.text, 30.0, &p).clicked() {
                if let Some(f) = rfd::FileDialog::new().set_title(t("Open file")).pick_file() {
                    let f = f.to_string_lossy().to_string();
                    fsops::allow_file(&self.core, &f);
                    self.reload_projects();
                    self.open_file(&f);
                }
            }
            return;
        };
        let meta = self.core.setting("projectMeta")[&ws].clone();
        let summary = self.summaries.get(&ws).cloned().unwrap_or(Value::Null);
        let langs: Vec<String> = summary["langs"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let items = self.list(&ws);
        egui::ScrollArea::vertical().id_salt("project-page").auto_shrink(false).show(&mut child, |ui| {
            let full = ui.available_width();
            let inner_w = (full - 60.0).min(960.0);
            let left = rect.left() + (full - inner_w) / 2.0;
            ui.add_space(40.0);
            // ---- hlavička: ikona, meno, popis, jazyky, akcie ----
            let (head, _) = ui.allocate_exact_size(vec2(full, 104.0), Sense::hover());
            let icon_box = Rect::from_min_size(pos2(left, head.top()), vec2(64.0, 64.0));
            ui.painter().rect_filled(icon_box, CornerRadius::same(14), p.card2);
            ui.painter().rect_stroke(icon_box, CornerRadius::same(14), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            match langs.first() {
                Some(k) => widgets::file_icon(ui, Rect::from_center_size(icon_box.center(), vec2(36.0, 36.0)), kind_file(k)),
                None => widgets::icon_at(ui, icon_box.center(), 30.0, "folder", p.text2),
            }
            let tx = icon_box.right() + 19.0;
            ui.painter().text(pos2(tx, head.top() + 20.0), Align2::LEFT_CENTER, widgets::file_name(&ws), theme::bold(30.0), p.text);
            let desc = meta["description"].as_str().filter(|d| !d.is_empty()).map(String::from).unwrap_or_else(|| ws.clone());
            widgets::text(ui, pos2(tx, head.top() + 57.0), Align2::LEFT_CENTER, &desc, theme::ui(14.0), p.text2, inner_w - 420.0);
            let mut cx = tx;
            for k in &langs {
                let name = kind_name(k);
                let w = widgets::text_w(ui, name, theme::bold(12.0)) + 30.0;
                let chip = Rect::from_min_size(pos2(cx, head.top() + 79.0), vec2(w, 22.0));
                ui.painter().rect_filled(chip, CornerRadius::same(7), p.hover);
                ui.painter().rect_stroke(chip, CornerRadius::same(7), Stroke::new(1.0, p.line), StrokeKind::Inside);
                widgets::file_icon(ui, Rect::from_min_size(pos2(chip.left() + 7.0, chip.center().y - 6.0), vec2(12.0, 12.0)), kind_file(k));
                ui.painter().text(pos2(chip.left() + 24.0, chip.center().y), Align2::LEFT_CENTER, name, theme::bold(12.0), p.text2);
                cx += w + 6.0;
            }
            // akcie vpravo
            let main_file = main_file(&items);
            let mut acts = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_max(pos2(left + inner_w - 420.0, head.top() + 6.0), pos2(left + inner_w, head.top() + 40.0)))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            acts.spacing_mut().item_spacing.x = 6.0;
            if widgets::button(&mut acts, Some("filePlus"), &t("New file"), p.card2, p.text, 32.0, &p).clicked() {
                self.new_item = Some((false, String::new()));
            }
            if let Some((name, path)) = &main_file {
                let web = name.ends_with(".html");
                if widgets::button(&mut acts, Some(if web { "globe" } else { "play" }), &if web { t("Open page") } else { t("Run it") }, p.accent, p.accent_fg, 32.0, &p).on_hover_text(name).clicked()
                {
                    if web {
                        settings::open_external(path);
                    } else {
                        self.open_file(path);
                        self.run();
                    }
                }
            }
            ui.add_space(24.0);
            // ---- dlaždice so štatistikami ----
            let runs = self.core.setting("activity")[&ws]["runs"].as_u64().unwrap_or(0);
            let secs = self.core.setting("projectTime")[&ws].as_u64().unwrap_or(0);
            let tiles = [
                ("clock", if secs >= 60 { format_time(secs) } else { "0 min".into() }, "coding time"),
                ("play", runs.to_string(), "runs"),
                ("code", summary["lines"].as_u64().map(|n| n.to_string()).unwrap_or("\u{2013}".into()), "lines of code"),
                ("file", summary["files"].as_u64().map(|n| n.to_string()).unwrap_or("\u{2013}".into()), "files"),
                ("refresh", summary["last"].as_u64().map(ago).unwrap_or("\u{2013}".into()), "last change"),
            ];
            let (row, _) = ui.allocate_exact_size(vec2(full, 84.0), Sense::hover());
            let tw = (inner_w - 4.0 * 11.0) / 5.0;
            for (i, (ic, val, label)) in tiles.iter().enumerate() {
                let t = Rect::from_min_size(pos2(left + i as f32 * (tw + 11.0), row.top()), vec2(tw, 83.0));
                card(ui, t, 12, &p);
                let ib = Rect::from_min_size(pos2(t.right() - 40.0, t.top() + 12.0), vec2(28.0, 28.0));
                ui.painter().rect_filled(ib, CornerRadius::same(8), p.active);
                widgets::icon_at(ui, ib.center(), 14.0, ic, p.text);
                widgets::text(ui, pos2(t.left() + 16.0, t.top() + 31.0), Align2::LEFT_CENTER, val, theme::bold(if val.len() > 8 { 18.0 } else { 24.0 }), p.text, tw - 64.0);
                ui.painter().text(pos2(t.left() + 16.0, t.top() + 60.0), Align2::LEFT_CENTER, crate::i18n::t(label), theme::ui(12.5), p.text3);
            }
            ui.add_space(22.0);
            // ---- súbory (podľa druhu) a TO-DO ----
            let groups = file_groups(&items);
            let rows: usize = groups.iter().map(|g| g.1.len() + 2).sum();
            let files_h = (rows as f32 * 33.0 + 20.0).max(120.0);
            let lw = ((inner_w - 18.0) * 0.575).floor();
            let (cards, _) = ui.allocate_exact_size(vec2(full, files_h), Sense::hover());
            let fc = Rect::from_min_size(pos2(left, cards.top()), vec2(lw, files_h));
            card(ui, fc, 14, &p);
            let mut y = fc.top() + 16.0;
            let mut open = None;
            for (title, files) in &groups {
                caption(ui, pos2(fc.left() + 16.0, y + 8.0), &t(title).to_uppercase(), &p);
                ui.painter().text(pos2(fc.right() - 16.0, y + 8.0), Align2::RIGHT_CENTER, files.len().to_string(), theme::bold(10.5), p.text3);
                y += 26.0;
                for (name, path) in files {
                    let r = Rect::from_min_size(pos2(fc.left() + 16.0, y), vec2(lw - 32.0, 32.0));
                    let resp = ui.interact(r, ui.id().with(("pf", path)), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, CornerRadius::same(8), p.hover);
                    }
                    widgets::file_icon(ui, Rect::from_min_size(pos2(r.left() + 10.0, r.center().y - 8.0), vec2(16.0, 16.0)), name);
                    ui.painter().text(pos2(r.left() + 36.0, r.center().y), Align2::LEFT_CENTER, name, theme::bold(13.0), p.text);
                    if main_file.as_ref().map(|m| &m.1 == path).unwrap_or(false) {
                        let b = Rect::from_min_size(pos2(r.right() - 52.0, r.center().y - 9.0), vec2(43.0, 18.0));
                        ui.painter().rect_filled(b, CornerRadius::same(9), p.active);
                        ui.painter().text(b.center(), Align2::CENTER_CENTER, "main", theme::bold(11.0), p.text);
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some(path.clone());
                    }
                    y += 33.0;
                }
                y += 12.0;
            }
            if groups.is_empty() {
                ui.painter().text(pos2(fc.center().x, fc.center().y), Align2::CENTER_CENTER, t("No files yet – create one with New file."), theme::ui(13.0), p.text3);
            }
            if let Some(f) = open {
                self.open_file(&f);
            }
            // TO-DO (settings.projectMeta[dir].todos – rovnaké ako v Electron Fluxe)
            let todos: Vec<Value> = meta["todos"].as_array().cloned().unwrap_or_default();
            let done = todos.iter().filter(|t| t["done"].as_bool() == Some(true)).count();
            let th = 104.0 + todos.len() as f32 * 36.0;
            let tc = Rect::from_min_size(pos2(left + lw + 18.0, cards.top()), vec2(inner_w - lw - 18.0, th));
            card(ui, tc, 14, &p);
            caption(ui, pos2(tc.left() + 16.0, tc.top() + 24.0), &t("To-do").to_uppercase(), &p);
            ui.painter().text(pos2(tc.right() - 16.0, tc.top() + 24.0), Align2::RIGHT_CENTER, format!("{done} / {}", todos.len()), theme::bold(10.5), p.text3);
            let input = Rect::from_min_size(pos2(tc.left() + 16.0, tc.top() + 41.0), vec2(tc.width() - 32.0, 40.0));
            ui.painter().rect_filled(input, CornerRadius::same(10), p.card2);
            ui.painter().rect_stroke(input, CornerRadius::same(10), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            widgets::icon_at(ui, pos2(input.left() + 18.0, input.center().y), 14.0, "plus", p.text3);
            let mut ic = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(input.left() + 32.0, input.top() + 11.0), pos2(input.right() - 10.0, input.bottom() - 8.0))));
            let r = ic.add(egui::TextEdit::singleline(&mut self.new_todo).hint_text("Add a task and press Enter\u{2026}").frame(Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(13.5)));
            let mut change: Option<Box<dyn FnOnce(&mut Vec<Value>)>> = None;
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.new_todo.trim().is_empty() {
                let text = self.new_todo.trim().to_string();
                self.new_todo.clear();
                r.request_focus();
                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0).to_string();
                change = Some(Box::new(move |t: &mut Vec<Value>| t.push(json!({ "id": id, "text": text, "done": false }))));
            }
            let mut ty = input.bottom() + 8.0;
            for (i, t) in todos.iter().enumerate() {
                let row = Rect::from_min_size(pos2(tc.left() + 16.0, ty), vec2(tc.width() - 32.0, 36.0));
                let resp = ui.interact(row, ui.id().with(("todo", i)), Sense::click());
                let is_done = t["done"].as_bool() == Some(true);
                let bx = Rect::from_min_size(pos2(row.left() + 12.0, row.center().y - 9.0), vec2(18.0, 18.0));
                if is_done {
                    ui.painter().rect_filled(bx, CornerRadius::same(5), p.text);
                    widgets::icon_at(ui, bx.center(), 12.0, "check", p.card);
                } else {
                    ui.painter().rect_stroke(bx, CornerRadius::same(5), Stroke::new(1.5, if resp.hovered() { p.text2 } else { p.text3 }), StrokeKind::Inside);
                }
                let tr = widgets::text(
                    ui,
                    pos2(bx.right() + 10.0, row.center().y),
                    Align2::LEFT_CENTER,
                    t["text"].as_str().unwrap_or(""),
                    theme::ui(13.5),
                    if is_done { p.text3 } else { p.text },
                    row.width() - 50.0,
                );
                if is_done {
                    ui.painter().hline(tr.x_range(), tr.center().y + 1.0, Stroke::new(1.0, p.text3));
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    change = Some(Box::new(move |list: &mut Vec<Value>| {
                        if let Some(x) = list.get_mut(i) {
                            x["done"] = json!(!is_done);
                        }
                    }));
                }
                ty += 36.0;
            }
            if let Some(f) = change {
                self.update_settings(|o| {
                    let mut all = o.get("projectMeta").cloned().unwrap_or(json!({}));
                    let mut list = all[&ws]["todos"].as_array().cloned().unwrap_or_default();
                    f(&mut list);
                    all[&ws]["todos"] = Value::Array(list);
                    o.insert("projectMeta".into(), all);
                });
            }
            ui.add_space(40.0);
        });
    }

    // ---------- karta: Výstup / Terminál ----------
    fn bottom_panel(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = self.pal;
        ui.painter().hline(rect.x_range(), rect.top(), Stroke::new(1.0, p.line));
        // pilulky Output | Terminal
        let group = Rect::from_min_size(pos2(rect.left() + 15.0, rect.top() + 7.0), vec2(150.0, 24.0));
        ui.painter().rect_filled(group, CornerRadius::same(8), p.card2);
        let mut x = group.left() + 2.0;
        for (b, label) in [(Bottom::Output, "Output"), (Bottom::Terminal, "Terminal")] {
            let w = widgets::text_w(ui, &t(label), theme::bold(12.5)) + 20.0;
            let r = Rect::from_min_size(pos2(x, group.top() + 2.0), vec2(w, 20.0));
            let resp = ui.interact(r, ui.id().with(label), Sense::click());
            let sel = self.bottom == b;
            if sel {
                ui.painter().rect_filled(r, CornerRadius::same(6), p.active);
            }
            ui.painter().text(r.center(), Align2::CENTER_CENTER, t(label), theme::bold(12.5), if sel || resp.hovered() { p.text } else { p.text3 });
            if resp.clicked() {
                self.bottom = b;
                if b == Bottom::Terminal {
                    self.start_shell();
                    self.sh.focus(ui.ctx());
                } else {
                    self.out.focus(ui.ctx());
                }
            }
            x += w;
        }
        let cy = group.center().y;
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rect.right() - 22.0, cy), vec2(26.0, 26.0)), "panel", 15.0, &p, true).on_hover_text(t("Hide panel")).clicked() {
            self.panel_open = false;
        }
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rect.right() - 60.0, cy), vec2(26.0, 26.0)), "trash", 15.0, &p, true).on_hover_text(t("Clear")).clicked() {
            if self.bottom == Bottom::Output {
                self.out.clear();
            } else {
                self.sh.clear();
            }
        }
        let body = Rect::from_min_max(pos2(rect.left() + 10.0, rect.top() + 36.0), pos2(rect.right() - 10.0, rect.bottom() - 4.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
        child.set_clip_rect(body);
        if self.bottom == Bottom::Output {
            self.out.show(&mut child, &p);
        } else {
            self.sh.show(&mut child, &p);
        }
    }

    // ---------- karta: stavový riadok ----------
    fn status_bar(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = self.pal;
        ui.painter().hline(rect.x_range(), rect.top(), Stroke::new(1.0, p.line));
        let cy = rect.center().y;
        let mut x = rect.left() + 16.0;
        let small = theme::ui(12.0);
        if let Some(py) = &self.python {
            widgets::icon_at(ui, pos2(x + 6.0, cy), 13.0, "python", p.text3);
            x += 19.0;
            let v = format!("Python {}", py["version"].as_str().unwrap_or(""));
            let r = widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, &v, small.clone(), p.text2, 200.0);
            x = r.right() + 6.0;
            let src = py["source"].as_str().unwrap_or("");
            let r = widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, if src == "PATH" { "PATH" } else { src }, theme::ui(11.0), p.text3, 120.0);
            x = r.right() + 18.0;
        }
        let (dot, label) = if self.running { (p.green, t("Running")) } else { (p.text3, t("Ready")) };
        ui.painter().circle_filled(pos2(x + 3.0, cy), 3.0, dot);
        let r = widgets::text(ui, pos2(x + 11.0, cy), Align2::LEFT_CENTER, &label, small.clone(), if self.running { p.green } else { p.text3 }, 100.0);
        x = r.right() + 18.0;
        if !self.status.is_empty() {
            widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, &self.status, small.clone(), p.red, (rect.right() - x - 320.0).max(40.0));
        }
        // vpravo: Ln/Col, slová, Auto save, jazyk
        let mut rx = rect.right() - 16.0;
        if let Some(t) = self.tabs.get(self.active).filter(|_| !self.home) {
            let r = widgets::text(ui, pos2(rx, cy), Align2::RIGHT_CENTER, &crate::i18n::t(lang_name(&t.ext())), small.clone(), p.text2, 120.0);
            rx = r.left() - 18.0;
        }
        if self.core.setting("autosave").as_bool() != Some(false) {
            let r = widgets::text(ui, pos2(rx, cy), Align2::RIGHT_CENTER, &t("Auto save"), small.clone(), p.text2, 100.0);
            widgets::icon_at(ui, pos2(r.left() - 9.0, cy), 12.0, "save", p.text2);
            rx = r.left() - 34.0;
        }
        if let Some(t) = self.tabs.get(self.active).filter(|_| !self.home) {
            let words = t.text.split_whitespace().count();
            let r = widgets::text(ui, pos2(rx, cy), Align2::RIGHT_CENTER, &crate::i18n::tf("{n} words", &[("n", &words.to_string())]), small.clone(), p.text3, 100.0);
            rx = r.left() - 18.0;
            widgets::text(
                ui,
                pos2(rx, cy),
                Align2::RIGHT_CENTER,
                &crate::i18n::tf("Ln {line}, Col {col}", &[("line", &self.cursor.0.to_string()), ("col", &self.cursor.1.to_string())]),
                small,
                p.text3,
                120.0,
            );
        }
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
            if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                self.open_folder(&d.to_string_lossy());
            }
        }
        if close && !self.tabs.is_empty() && !self.home {
            self.close_tab(self.active);
        }
    }
}

// zaoblená plocha na karte (dlaždice, zoznam súborov, TO-DO)
fn card(ui: &egui::Ui, r: Rect, radius: u8, p: &Pal) {
    ui.painter().rect_filled(r, CornerRadius::same(radius), p.card2);
    ui.painter().rect_stroke(r, CornerRadius::same(radius), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
}

fn caption(ui: &egui::Ui, pos: egui::Pos2, s: &str, p: &Pal) {
    let mut job = egui::text::LayoutJob::default();
    job.append(s, 0.0, egui::TextFormat { font_id: theme::bold(10.5), color: p.text3, extra_letter_spacing: 1.0, ..Default::default() });
    let g = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(pos2(pos.x, pos.y - g.size().y / 2.0), g, p.text3);
}

// súbory v koreni projektu podľa druhu – ako „Pages / Styles / Scripts“ na stránke projektu v Electron Fluxe
fn file_groups(items: &[(String, String, bool)]) -> Vec<(&'static str, Vec<(String, String)>)> {
    let order = ["Pages", "Styles", "Scripts", "Python", "Code", "Notes"];
    let mut groups: Vec<(&'static str, Vec<(String, String)>)> = order.iter().map(|g| (*g, vec![])).collect();
    for (name, path, is_dir) in items {
        if *is_dir {
            continue;
        }
        let g = match ext_of(name).as_str() {
            "html" | "htm" => "Pages",
            "css" | "scss" => "Styles",
            "js" | "mjs" | "ts" | "jsx" | "tsx" => "Scripts",
            "py" | "pyw" => "Python",
            "c" | "h" | "cpp" | "hpp" | "rs" | "go" | "java" | "cs" | "rb" | "php" | "lua" | "zig" | "r" | "jl" => "Code",
            "md" | "txt" => "Notes",
            _ => continue,
        };
        if let Some(x) = groups.iter_mut().find(|x| x.0 == g) {
            x.1.push((name.clone(), path.clone()));
        }
    }
    groups.retain(|g| !g.1.is_empty());
    groups
}

// hlavný súbor projektu (index.html, main.py…)
fn main_file(items: &[(String, String, bool)]) -> Option<(String, String)> {
    for want in ["index.html", "main.py", "app.py", "main.js", "index.js", "main.rs", "main.cpp", "main.c", "main.go", "Main.java", "Program.cs"] {
        if let Some((n, p, _)) = items.iter().find(|(n, _, d)| !d && n == want) {
            return Some((n.clone(), p.clone()));
        }
    }
    None
}

fn kind_name(k: &str) -> &'static str {
    match k {
        "python" => "Python",
        "web" => "HTML/CSS",
        "node" => "JavaScript",
        "java" => "Java",
        "cpp" => "C/C++",
        "go" => "Go",
        "csharp" => "C#",
        "rust" => "Rust",
        "ruby" => "Ruby",
        "php" => "PHP",
        "lua" => "Lua",
        "zig" => "Zig",
        "r" => "R",
        "julia" => "Julia",
        _ => "Files",
    }
}

// KIND_FILE z app.js: druh projektu → meno súboru pre ikonu
fn kind_file(k: &str) -> &'static str {
    match k {
        "python" => "a.py",
        "web" => "a.html",
        "node" => "a.js",
        "java" => "a.java",
        "cpp" => "a.cpp",
        "c" => "a.c",
        "go" => "a.go",
        "csharp" => "a.cs",
        "rust" => "a.rs",
        "ruby" => "a.rb",
        "php" => "a.php",
        "lua" => "a.lua",
        "zig" => "a.zig",
        "r" => "a.r",
        "julia" => "a.jl",
        _ => "",
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

// „/…/scratchpad/tf“ – posledné dva priečinky cesty
fn short_dir(f: &str) -> String {
    let parent = Path::new(f).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let sep = if parent.contains('\\') { '\\' } else { '/' };
    let parts: Vec<&str> = parent.split(sep).filter(|s| !s.is_empty()).collect();
    if parts.len() <= 2 {
        return parent;
    }
    format!("{sep}\u{2026}{sep}{}", parts[parts.len() - 2..].join(&sep.to_string()))
}

fn format_time(secs: u64) -> String {
    let m = secs / 60;
    if m < 60 {
        format!("{m} min")
    } else {
        format!("{} h {} min", m / 60, m % 60)
    }
}

fn ago(ms: u64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let s = now.saturating_sub(ms) / 1000;
    match s {
        0..=59 => t("just now"),
        60..=3599 => crate::i18n::tf("{n} min ago", &[("n", &(s / 60).to_string())]),
        3600..=86399 => crate::i18n::tf("{n} h ago", &[("n", &(s / 3600).to_string())]),
        _ => {
            let d = s / 86400;
            if d == 1 {
                t("yesterday")
            } else {
                crate::i18n::tf("{n} days ago", &[("n", &d.to_string())])
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let ctx = &ctx;
        self.events();
        self.run_tests(ctx);
        // pamäť na pozadí (trimMemory, predvolene zapnuté)
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        if let Some(d) = self.trim.tick(focused, self.core.setting("trimMemory").as_bool() != Some(false)) {
            ctx.request_repaint_after(d);
        }
        if self.intro.is_some() {
            let r = root.max_rect();
            egui::CentralPanel::default().frame(Frame::new().fill(self.pal.base)).show(root, |ui| self.intro_ui(ui, r));
            return;
        }
        self.shortcuts(ctx);
        // automatické ukladanie 1 s po poslednej zmene
        if let Some(t) = self.last_edit {
            if t.elapsed() >= Duration::from_millis(1000) {
                self.last_edit = None;
                if self.core.setting("autosave").as_bool() != Some(false) {
                    self.save(self.active);
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(1000) - t.elapsed());
            }
        }
        if self.side_open {
            self.sidebar(root);
        }
        let p = self.pal;
        egui::CentralPanel::default().frame(Frame::new().fill(p.base)).show(root, |ui| {
            let full = ui.max_rect();
            let left = if self.side_open { full.left() } else { full.left() };
            let top = Rect::from_min_max(pos2(left, full.top()), pos2(full.right() - GAP, full.top() + TOP_H));
            self.top_bar(ui, top);
            // jedna zaoblená karta: editor/stránka projektu, Výstup/Terminál, stavový riadok (#card)
            let card_r = Rect::from_min_max(pos2(left + if self.side_open { 0.0 } else { GAP }, top.bottom()), pos2(full.right() - GAP, full.bottom() - GAP));
            ui.painter().add(egui::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(if p.dark { 110 } else { 25 }) }.as_shape(card_r, CornerRadius::same(RADIUS)));
            ui.painter().rect_filled(card_r, CornerRadius::same(RADIUS), p.card);
            ui.painter().rect_stroke(card_r, CornerRadius::same(RADIUS), Stroke::new(1.0, p.line), StrokeKind::Outside);
            let mut card_ui = ui.new_child(egui::UiBuilder::new().max_rect(card_r));
            card_ui.set_clip_rect(card_r.shrink(1.0));
            let status = Rect::from_min_max(pos2(card_r.left(), card_r.bottom() - 28.0), card_r.max);
            self.status_bar(&mut card_ui, status);
            let mut main = Rect::from_min_max(card_r.min, pos2(card_r.right(), status.top()));
            let show_editor = !self.home && !self.tabs.is_empty();
            if self.panel_open && show_editor {
                let h = self.panel_h.clamp(90.0, (main.height() - 120.0).max(90.0));
                let panel = Rect::from_min_max(pos2(main.left(), main.bottom() - h), main.max);
                // ťahadlo nad panelom
                let grip = Rect::from_min_max(pos2(panel.left(), panel.top() - 3.0), pos2(panel.right(), panel.top() + 3.0));
                let gr = card_ui.interact(grip, card_ui.id().with("grip"), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeRow);
                if gr.dragged() {
                    self.panel_h = (h - gr.drag_delta().y).max(90.0);
                }
                self.bottom_panel(&mut card_ui, panel);
                main.max.y = panel.top();
            } else if show_editor {
                // skrytý panel: malé tlačidlo na jeho vrátenie
                let b = Rect::from_center_size(pos2(main.right() - 136.0, main.bottom() - 18.0), vec2(26.0, 26.0));
                if widgets::icon_button_at(&mut card_ui, b, "panel", 15.0, &p, true).on_hover_text(t("Show Output")).clicked() {
                    self.panel_open = true;
                }
            }
            if show_editor {
                self.editor(&mut card_ui, main);
            } else {
                self.project_page(&mut card_ui, main);
            }
        });
    }
}
