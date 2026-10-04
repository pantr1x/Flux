// Ponuky a hľadanie ako v Electron Fluxe: ☰ (appMenus: Súbor, Upraviť, Zobraziť, Spustiť, Pomoc),
// pravý klik v strome a pri projektoch, paleta (Ctrl+P súbory, Ctrl+Shift+A všetko, Ctrl+Shift+P príkazy)
// a potvrdzovacie okno (odstránenie do Koša).
use super::{App, Bottom};
use crate::i18n::{t, tf};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind};
use flux_core::fsops;
use serde_json::json;
use std::path::Path;

#[derive(Clone, Copy, PartialEq)]
pub enum PaletteMode {
    Files,
    Everything,
    Commands,
}

pub struct Palette {
    mode: PaletteMode,
    q: String,
    sel: usize,
    files: Vec<String>,
    opened: f64,
}

impl Palette {
    pub fn set_query(&mut self, q: &str) {
        self.q = q.to_string();
        self.sel = 0;
    }
}

// Potvrdenie (napr. „Odstrániť …?“) – akcia sa vykoná po „áno“.
pub struct Ask {
    pub text: String,
    pub yes: String,
    pub action: String,
}

// (id, text, skratka, ikona)
const COMMANDS: [(&str, &str, &str, &str); 17] = [
    ("new-file", "New file…", "Ctrl+N", "filePlus"),
    ("new-project", "New project…", "Ctrl+Shift+N", "plus"),
    ("open-folder", "Open folder…", "Ctrl+O", "folderOpen"),
    ("open-file", "Open file…", "Ctrl+Alt+O", "file"),
    ("quick-open", "Quick open file…", "Ctrl+P", "search"),
    ("save", "Save", "Ctrl+S", "save"),
    ("save-all", "Save all", "Ctrl+Shift+S", "save"),
    ("close", "Close file", "Ctrl+W", "x"),
    ("settings", "Settings", "Ctrl+,", "settings"),
    ("run", "Run current file", "F5", "play"),
    ("stop", "Stop program", "Shift+F5", "stop"),
    ("terminal", "Terminal", "", "terminal"),
    ("sidebar", "Sidebar", "Ctrl+B", "sidebar"),
    ("panel", "Panel", "Ctrl+J", "panel"),
    ("theme", "Toggle light / dark theme", "Ctrl+Shift+L", "sun"),
    ("home", "Home", "", "template"),
    ("shortcuts", "Shortcuts", "", "command"),
];

// jednoduché „fuzzy“ – všetky znaky v poradí; skóre = kratší rozptyl je lepší
fn fuzzy(hay: &str, q: &str) -> Option<i32> {
    if q.is_empty() {
        return Some(0);
    }
    let h = hay.to_lowercase();
    let mut last = 0usize;
    let mut first = None;
    let mut chars = h.char_indices();
    for qc in q.to_lowercase().chars().filter(|c| !c.is_whitespace()) {
        loop {
            let (i, c) = chars.next()?;
            if c == qc {
                first.get_or_insert(i);
                last = i;
                break;
            }
        }
    }
    let spread = (last - first.unwrap_or(0)) as i32;
    Some(-spread - if h.contains(&q.to_lowercase()) { 0 } else { 50 })
}

// položka ponuky: ikona, text, skratka vpravo, fajka
pub fn item(ui: &mut egui::Ui, icon: Option<&str>, label: &str, key: &str, checked: bool, enabled: bool, p: &crate::theme::Pal) -> Response {
    // šírka podľa ponuky (nie celého okna)
    let w = ui.min_rect().width().max(ui.spacing().menu_width.min(260.0)).max(230.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), if enabled { Sense::click() } else { Sense::hover() });
    let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), enabled && resp.hovered(), 0.1);
    if hk > 0.0 {
        ui.painter().rect_filled(r, CornerRadius::same(6), p.hover.gamma_multiply(hk));
    }
    let c = if enabled { p.text } else { p.text3.gamma_multiply(0.6) };
    if checked {
        widgets::icon_at(ui, pos2(r.left() + 13.0, r.center().y), 13.0, "check", c);
    } else if let Some(i) = icon {
        widgets::icon_at(ui, pos2(r.left() + 13.0, r.center().y), 14.0, i, p.text2);
    }
    ui.painter().text(pos2(r.left() + 30.0, r.center().y), Align2::LEFT_CENTER, label, theme::ui(13.0), c);
    if !key.is_empty() {
        ui.painter().text(pos2(r.right() - 10.0, r.center().y), Align2::RIGHT_CENTER, key, theme::ui(11.5), p.text3);
    }
    resp
}

fn sep(ui: &mut egui::Ui, p: &crate::theme::Pal) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
    ui.painter().hline(r.x_range(), r.center().y, Stroke::new(1.0, p.line));
}

fn caption(ui: &mut egui::Ui, s: &str, p: &crate::theme::Pal) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    ui.painter().text(pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, s, theme::bold(11.0), p.text3);
}

impl App {
    // vykoná príkaz (z ponuky, palety alebo skratky)
    pub(super) fn act(&mut self, id: &str, ctx: &egui::Context) {
        match id {
            "reload-plugins" => self.plug_reload_all(),
            "ai" => self.toggle_ai(),
            "new-file" => self.open_new_file(None),
            "new-folder" => {
                self.new_item = Some((true, String::new()));
                self.side_open = true;
            }
            "new-project" => {
                self.open_new_project();
                self.side_open = true;
            }
            "open-folder" => {
                if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                    self.open_folder(&d.to_string_lossy());
                }
            }
            "open-file" => {
                if let Some(f) = rfd::FileDialog::new().set_title(t("Open file")).pick_file() {
                    let f = f.to_string_lossy().to_string();
                    fsops::allow_file(&self.core, &f);
                    self.reload_projects();
                    self.open_file(&f);
                }
            }
            "quick-open" => self.open_palette(PaletteMode::Files, ctx),
            "search" => self.open_palette(PaletteMode::Everything, ctx),
            "commands" => self.open_palette(PaletteMode::Commands, ctx),
            "save" => self.save(self.active),
            "save-all" => {
                for i in 0..self.tabs.len() {
                    if self.tabs[i].dirty() {
                        self.save(i);
                    }
                }
            }
            "close" => {
                if !self.tabs.is_empty() && !self.home {
                    self.close_tab(self.active);
                }
            }
            "settings" => self.open_settings("general", ctx),
            "shortcuts" => {
                self.open_settings("general", ctx);
                self.settings_jump("Shortcuts");
            }
            "run" => self.run(),
            "stop" => {
                // zastaví program, inak Live Server
                if self.running {
                    self.out.pty.kill();
                } else {
                    self.close_preview();
                }
            }
            "terminal" => {
                self.panel_open = true;
                self.bottom = Bottom::Terminal;
                self.start_shell();
            }
            "sidebar" => self.side_open = !self.side_open,
            "panel" => self.panel_open = !self.panel_open,
            "theme" => self.set_theme(!self.pal.dark, ctx),
            "home" => self.open_start(ctx.input(|i| i.time)),
            "website" => flux_core::settings::open_external("https://pantr1x.github.io/Flux/"),
            "star" => flux_core::settings::open_external("https://github.com/pantr1x/Flux"),
            "issue" => flux_core::settings::open_external("https://github.com/pantr1x/Flux/issues/new"),
            "releases" => flux_core::settings::open_external("https://pantr1x.github.io/Flux/#releases"),
            _ => {
                if let Some(v) = id.strip_prefix("panelPos:") {
                    self.set("panelPos", json!(v), ctx);
                } else if let Some(v) = id.strip_prefix("sidePos:") {
                    self.set("sidePos", json!(v), ctx);
                } else if let Some(p) = id.strip_prefix("trash:") {
                    match fsops::move_to_trash(p) {
                        Ok(_) => {
                            // zavrie karty z odstráneného miesta
                            let gone: Vec<usize> = self.tabs.iter().enumerate().filter(|(_, x)| x.path.starts_with(p)).map(|(i, _)| i).rev().collect();
                            for i in gone {
                                self.tabs.remove(i);
                            }
                            self.active = self.active.min(self.tabs.len().saturating_sub(1));
                            if self.tabs.is_empty() {
                                self.home = true;
                            }
                            self.tree.clear();
                        }
                        Err(e) => self.status = e,
                    }
                } else if let Some(d) = id.strip_prefix("trash-project:") {
                    let d = d.to_string();
                    if self.workspace().as_deref() == Some(d.as_str()) {
                        // Windows nezmaže priečinok, ktorý drží sledovanie zmien alebo terminál
                        self.watcher = None;
                        self.out.pty.kill();
                        self.sh.pty.kill();
                        self.shell_started = false;
                        self.server = None;
                        self.close_preview();
                    }
                    match fsops::move_to_trash(&d) {
                        Ok(_) => {
                            fsops::forget(&self.core, &d);
                            if self.workspace().as_deref() == Some(d.as_str()) {
                                *self.core.workspace.lock().unwrap() = None;
                            }
                            let pre = format!("{d}{}", std::path::MAIN_SEPARATOR);
                            self.tabs.retain(|t| !t.path.starts_with(&pre));
                            self.active = self.active.min(self.tabs.len().saturating_sub(1));
                            self.home = true;
                            self.reload_projects();
                            // späť na domovskú obrazovku
                            let now = ctx.input(|i| i.time);
                            self.open_start(now);
                            self.note(tf("{name} was moved to the Recycle Bin.", &[("name", &widgets::file_name(&d))]));
                        }
                        Err(e) => self.status = e.to_string(),
                    }
                }
            }
        }
    }

    // ---------- ☰ ponuka ----------
    pub(super) fn app_menu(&mut self, resp: &Response) {
        let p = self.pal;
        let mut picked: Option<String> = None;
        let has_file = !self.tabs.is_empty() && !self.home;
        egui::Popup::menu(resp).show(|ui| {
            ui.set_min_width(250.0);
                ui.set_max_width(250.0);
            if item(ui, Some("template"), &t("Home"), "", false, true, &p).clicked() {
                picked = Some("home".into());
            }
            sep(ui, &p);
            ui.menu_button(t("File"), |ui| {
                ui.set_min_width(260.0);
                ui.set_max_width(260.0);
                for (id, label, key, icon) in
                    [("new-file", "New file…", "Ctrl+N", "filePlus"), ("new-project", "New project…", "Ctrl+Shift+N", "plus"), ("new-folder", "New folder…", "", "folderPlus")]
                {
                    if item(ui, Some(icon), &t(label), key, false, true, &p).clicked() {
                        picked = Some(id.into());
                    }
                }
                sep(ui, &p);
                for (id, label, key, icon) in
                    [("open-folder", "Open folder…", "Ctrl+O", "folderOpen"), ("open-file", "Open file…", "Ctrl+Alt+O", "file"), ("quick-open", "Quick open file…", "Ctrl+P", "search")]
                {
                    if item(ui, Some(icon), &t(label), key, false, true, &p).clicked() {
                        picked = Some(id.into());
                    }
                }
                sep(ui, &p);
                if item(ui, Some("save"), &t("Save"), "Ctrl+S", false, has_file, &p).clicked() {
                    picked = Some("save".into());
                }
                if item(ui, None, &t("Save all"), "Ctrl+Shift+S", false, true, &p).clicked() {
                    picked = Some("save-all".into());
                }
                if item(ui, Some("x"), &t("Close file"), "Ctrl+W", false, has_file, &p).clicked() {
                    picked = Some("close".into());
                }
                sep(ui, &p);
                if item(ui, Some("settings"), &t("Settings"), "Ctrl+,", false, true, &p).clicked() {
                    picked = Some("settings".into());
                }
            });
            ui.menu_button(t("Edit"), |ui| {
                ui.set_min_width(260.0);
                ui.set_max_width(260.0);
                // úpravy textu robí editor sám (Ctrl+Z / Ctrl+Y / Ctrl+A)
                item(ui, None, &t("Undo"), "Ctrl+Z", false, false, &p);
                item(ui, None, &t("Redo"), "Ctrl+Y", false, false, &p);
            });
            ui.menu_button(t("View"), |ui| {
                ui.set_min_width(270.0);
                ui.set_max_width(270.0);
                if item(ui, Some("search"), &t("Search everything…"), "Ctrl+Shift+A", false, true, &p).clicked() {
                    picked = Some("search".into());
                }
                if item(ui, Some("command"), &t("Commands"), "Ctrl+Shift+P", false, true, &p).clicked() {
                    picked = Some("commands".into());
                }
                sep(ui, &p);
                if item(ui, None, &t("Sidebar"), "Ctrl+B", self.side_open, true, &p).clicked() {
                    picked = Some("sidebar".into());
                }
                if item(ui, None, &t("Panel"), "Ctrl+J", self.panel_open, true, &p).clicked() {
                    picked = Some("panel".into());
                }
                sep(ui, &p);
                caption(ui, &t("Panel position"), &p);
                let pp = self.get("panelPos").as_str().unwrap_or("bottom").to_string();
                for (v, l) in [("bottom", "Bottom"), ("right", "Right"), ("left", "Left")] {
                    if item(ui, None, &t(l), "", pp == v, true, &p).clicked() {
                        picked = Some(format!("panelPos:{v}"));
                    }
                }
                caption(ui, &t("Sidebar position"), &p);
                let sp = self.get("sidePos").as_str().unwrap_or("left").to_string();
                for (v, l) in [("left", "Left"), ("right", "Right")] {
                    if item(ui, None, &t(l), "", sp == v, true, &p).clicked() {
                        picked = Some(format!("sidePos:{v}"));
                    }
                }
                sep(ui, &p);
                if item(ui, Some(if p.dark { "sun" } else { "moon" }), &t("Toggle light / dark theme"), "Ctrl+Shift+L", false, true, &p).clicked() {
                    picked = Some("theme".into());
                }
            });
            ui.menu_button(t("Run"), |ui| {
                ui.set_min_width(250.0);
                ui.set_max_width(250.0);
                if item(ui, Some("play"), &t("Run current file"), "F5", false, has_file, &p).clicked() {
                    picked = Some("run".into());
                }
                if item(ui, Some("stop"), &t("Stop program"), "Shift+F5", false, self.running, &p).clicked() {
                    picked = Some("stop".into());
                }
                sep(ui, &p);
                if item(ui, Some("terminal"), &t("Terminal"), "", false, true, &p).clicked() {
                    picked = Some("terminal".into());
                }
            });
            ui.menu_button(t("Help"), |ui| {
                ui.set_min_width(250.0);
                ui.set_max_width(250.0);
                for (id, label, icon) in [
                    ("shortcuts", "Shortcuts", "command"),
                    ("releases", "Release notes", "file"),
                    ("website", "Website", "globe"),
                    ("star", "Star Flux on GitHub", "star"),
                    ("issue", "Report a problem", "external"),
                ] {
                    if id == "website" {
                        sep(ui, &p);
                    }
                    if item(ui, Some(icon), &t(label), "", false, true, &p).clicked() {
                        picked = Some(id.into());
                    }
                }
            });
        });
        if let Some(id) = picked {
            self.act(&id, &resp.ctx);
        }
    }

    // ---------- pravý klik v strome ----------
    pub(super) fn tree_menu(&mut self, resp: &Response, path: &str, is_dir: bool) {
        let p = self.pal;
        let mut picked: Option<&str> = None;
        let ws = self.workspace();
        resp.context_menu(|ui| {
            ui.set_min_width(230.0);
            if !is_dir {
                if item(ui, Some("play"), &t("Run"), "F5", false, true, &p).clicked() {
                    picked = Some("run");
                }
                sep(ui, &p);
            }
            if item(ui, Some("filePlus"), &t("New file…"), "", false, true, &p).clicked() {
                picked = Some("new-file");
            }
            if item(ui, Some("folderPlus"), &t("New folder…"), "", false, true, &p).clicked() {
                picked = Some("new-folder");
            }
            sep(ui, &p);
            if ws.as_deref() != Some(path) {
                if item(ui, Some("edit"), &t("Rename…"), "", false, true, &p).clicked() {
                    picked = Some("rename");
                }
                if item(ui, Some("trash"), &t("Delete"), "", false, true, &p).clicked() {
                    picked = Some("delete");
                }
                sep(ui, &p);
            }
            if item(ui, None, &t("Copy path"), "", false, true, &p).clicked() {
                picked = Some("copy");
            }
            if item(ui, Some("folderOpen"), &t(if cfg!(windows) { "Show in File Explorer" } else { "Show in folder" }), "", false, true, &p).clicked() {
                picked = Some("reveal");
            }
        });
        let ctx = resp.ctx.clone();
        let base = if is_dir { path.to_string() } else { Path::new(path).parent().map(|x| x.to_string_lossy().to_string()).unwrap_or_default() };
        match picked {
            Some("run") => {
                self.open_file(path);
                self.run();
            }
            Some("new-file") => self.open_new_file(Some(base.clone())),
            Some("new-folder") => {
                self.new_in = Some(base.clone());
                self.open_dirs.insert(base);
                self.new_item = Some((true, String::new()));
            }
            Some("rename") => self.renaming = Some((path.to_string(), widgets::file_name(path))),
            Some("delete") => {
                self.ask = Some(Ask {
                    text: tf("Delete “{name}”? It goes to the Recycle Bin, so you can still restore it.", &[("name", &widgets::file_name(path))]),
                    yes: t("Move to Recycle Bin"),
                    action: format!("trash:{path}"),
                })
            }
            Some("copy") => ctx.copy_text(path.to_string()),
            Some("reveal") => fsops::reveal(path),
            _ => {}
        }
    }

    // ---------- pravý klik na projekt ----------
    pub(super) fn project_menu(&mut self, resp: &Response, dir: &str, pinned: bool, hidden: bool) {
        let p = self.pal;
        let mut picked: Option<&str> = None;
        resp.context_menu(|ui| {
            ui.set_min_width(300.0);
            caption(ui, &widgets::file_name(dir), &p);
            if item(ui, Some("pin"), &t(if pinned { "Unpin" } else { "Pin to top" }), "", false, true, &p).clicked() {
                picked = Some("pin");
            }
            if item(ui, Some("edit"), &t("Rename…"), "", false, true, &p).clicked() {
                picked = Some("rename");
            }
            if item(ui, Some("sidebar"), &t(if hidden { "Show in the sidebar" } else { "Hide from the sidebar (stays on the home screen)" }), "", false, true, &p).clicked() {
                picked = Some("hide");
            }
            if item(ui, Some("x"), &t("Remove from Flux (files stay on disk)"), "", false, true, &p).clicked() {
                picked = Some("forget");
            }
            if item(ui, Some("trash"), &t("Delete project… (moves the folder to the Recycle Bin)"), "", false, true, &p).clicked() {
                picked = Some("trash");
            }
        });
        match picked {
            Some("pin") => {
                fsops::pin(&self.core, dir, !pinned);
            }
            Some("hide") => {
                fsops::hide(&self.core, dir, !hidden);
            }
            Some("forget") => {
                fsops::forget(&self.core, dir);
            }
            Some("rename") => self.renaming = Some((dir.to_string(), widgets::file_name(dir))),
            Some("trash") => {
                self.ask = Some(Ask {
                    text: tf("Delete “{name}”? The folder goes to the Recycle Bin, so you can still restore it.", &[("name", &widgets::file_name(dir))]),
                    yes: t("Delete project"),
                    action: format!("trash-project:{dir}"),
                })
            }
            _ => {}
        }
        if picked.is_some() {
            self.reload_projects();
        }
    }

    // premenovanie súboru/priečinka alebo projektu (Enter potvrdí)
    pub(super) fn finish_rename(&mut self, from: &str, name: &str) {
        let name = name.trim();
        if name.is_empty() || name == widgets::file_name(from) {
            return;
        }
        let is_project = self.projects.iter().any(|x| x["dir"].as_str() == Some(from));
        if is_project {
            // otvorený projekt: Windows nepremenuje priečinok, kým ho drží sledovanie zmien, terminál alebo program
            let open = self.workspace().as_deref() == Some(from);
            if open {
                self.watcher = None;
                self.out.pty.kill();
                self.sh.pty.kill();
                self.shell_started = false;
                self.server = None;
                self.close_preview();
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
            match fsops::rename_project(&self.core, from, name) {
                Ok(v) => {
                    let to = v["dir"].as_str().map(String::from).unwrap_or_else(|| Path::new(from).parent().unwrap_or(Path::new("")).join(name).to_string_lossy().to_string());
                    // popis, úlohy a ikona idú s projektom
                    self.update_settings(|o| {
                        if let Some(all) = o.get_mut("projectMeta").and_then(|m| m.as_object_mut()) {
                            if let Some(m) = all.remove(from) {
                                all.insert(to.clone(), m);
                            }
                        }
                        if o.get("lastFolder").and_then(|v| v.as_str()) == Some(from) {
                            o.insert("lastFolder".into(), serde_json::json!(to));
                        }
                    });
                    for tab in self.tabs.iter_mut() {
                        if tab.path.starts_with(&format!("{from}{}", std::path::MAIN_SEPARATOR)) {
                            tab.path = format!("{to}{}", &tab.path[from.len()..]);
                        }
                    }
                    self.reload_projects();
                    if open {
                        self.open_folder(&to);
                    }
                    self.note(tf("Renamed to {name}.", &[("name", name)]));
                }
                Err(e) => {
                    self.status = t(&e);
                    if open {
                        self.open_folder(from);
                    }
                }
            }
            return;
        }
        let to = Path::new(from).parent().unwrap_or(Path::new("")).join(name).to_string_lossy().to_string();
        match fsops::rename(from, &to) {
            Ok(_) => {
                for tab in self.tabs.iter_mut() {
                    if tab.path == from || tab.path.starts_with(&format!("{from}{}", std::path::MAIN_SEPARATOR)) {
                        tab.path = format!("{to}{}", &tab.path[from.len()..]);
                    }
                }
                self.tree.clear();
            }
            Err(e) => self.status = e,
        }
    }

    // ---------- paleta ----------
    pub(super) fn open_palette(&mut self, mode: PaletteMode, ctx: &egui::Context) {
        let files = fsops::list_all(&self.core).as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        self.palette = Some(Palette { mode, q: String::new(), sel: 0, files, opened: ctx.input(|i| i.time) });
    }

    pub(super) fn palette_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        let anim = self.anim_on();
        let ws = self.workspace().unwrap_or_default();
        let Some(pal) = self.palette.as_mut() else { return };
        let k = if anim { (((ctx.input(|i| i.time) - pal.opened) / 0.14).clamp(0.0, 1.0)) as f32 } else { 1.0 };
        if k < 1.0 {
            ctx.request_repaint();
        }
        // výsledky: (skóre, ikona/súbor, text, detail, akcia)
        let q = pal.q.trim().to_string();
        let mut res: Vec<(i32, String, String, String, String)> = vec![];
        if pal.mode != PaletteMode::Commands {
            for f in &pal.files {
                let rel = f.strip_prefix(&ws).unwrap_or(f).trim_start_matches(['/', '\\']).to_string();
                if let Some(s) = fuzzy(&rel, &q) {
                    let name = widgets::file_name(f);
                    let boost = if name.to_lowercase().contains(&q.to_lowercase()) { 100 } else { 0 };
                    let folder = Path::new(&rel).parent().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                    res.push((s + boost, format!("file:{name}"), name, folder, format!("open:{f}")));
                }
            }
        }
        if pal.mode != PaletteMode::Files {
            for (id, label, key, icon) in COMMANDS {
                let l = t(label);
                if let Some(s) = fuzzy(&l, &q) {
                    res.push((s + 20, format!("icon:{icon}"), l, key.to_string(), format!("cmd:{id}")));
                }
            }
        }
        if pal.mode != PaletteMode::Files {
            // príkazy z pluginov + znova načítať pluginy
            for c in &self.plug.cmds {
                if let Some(s) = fuzzy(&c.label, &q) {
                    res.push((s + 15, "icon:puzzle".into(), c.label.clone(), c.plugin.clone(), format!("plug:{}|{}", c.plugin, c.id)));
                }
            }
            let l = t("Reload plugins");
            if let Some(s) = fuzzy(&l, &q) {
                res.push((s, "icon:puzzle".into(), l, String::new(), "cmd:reload-plugins".into()));
            }
        }
        if pal.mode == PaletteMode::Everything {
            for pr in &self.projects {
                let name = pr["name"].as_str().unwrap_or("").to_string();
                if let Some(s) = fuzzy(&name, &q) {
                    res.push((s + 10, "icon:folder".into(), name, t("Projects"), format!("project:{}", pr["dir"].as_str().unwrap_or(""))));
                }
            }
            if !q.is_empty() {
                res.push((-1000, "icon:settings".into(), tf("Search settings for “{q}”", &[("q", &q)]), t("Settings"), format!("settings:{q}")));
            }
        }
        res.sort_by(|a, b| b.0.cmp(&a.0));
        res.truncate(60);
        // klávesy
        let (up, down, enter, esc) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        if down {
            pal.sel = (pal.sel + 1).min(res.len().saturating_sub(1));
        }
        if up {
            pal.sel = pal.sel.saturating_sub(1);
        }
        pal.sel = pal.sel.min(res.len().saturating_sub(1));
        // vzhľad (#palette): pole hore v strede, zoznam pod ním
        let bg = ui.interact(full, ui.id().with("pal-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha((50.0 * k) as u8));
        let w = (full.width() - 80.0).min(620.0);
        let rows = res.len().clamp(1, 12);
        let h = 52.0 + rows as f32 * 40.0 + 10.0;
        let card = Rect::from_min_size(pos2(full.center().x - w / 2.0, full.top() + 56.0 - (1.0 - k) * 10.0), vec2(w, h));
        ui.painter().add(egui::Shadow { offset: [0, 14], blur: 40, spread: 0, color: Color32::from_black_alpha((110.0 * k) as u8) }.as_shape(card, CornerRadius::same(14)));
        ui.painter().rect_filled(card, CornerRadius::same(14), p.solid);
        ui.painter().rect_stroke(card, CornerRadius::same(14), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        widgets::icon_at(ui, pos2(card.left() + 22.0, card.top() + 26.0), 16.0, "search", p.text3);
        let hint = match pal.mode {
            PaletteMode::Files => t("Quick open file…"),
            PaletteMode::Commands => t("Commands"),
            PaletteMode::Everything => t("Search files, commands, settings and projects"),
        };
        let mut ic = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(card.left() + 42.0, card.top() + 16.0), pos2(card.right() - 14.0, card.top() + 38.0))));
        let te = ic.add(egui::TextEdit::singleline(&mut pal.q).hint_text(hint).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(15.0)));
        te.request_focus();
        if te.changed() {
            pal.sel = 0;
        }
        ui.painter().hline(card.x_range(), card.top() + 52.0, Stroke::new(1.0, p.line));
        let mut pick = if enter { res.get(pal.sel).map(|r| r.4.clone()) } else { None };
        let list = Rect::from_min_max(pos2(card.left() + 6.0, card.top() + 57.0), pos2(card.right() - 6.0, card.bottom() - 5.0));
        let mut lc = ui.new_child(egui::UiBuilder::new().max_rect(list));
        lc.set_clip_rect(list);
        let sel = pal.sel;
        let sk = egui::Id::new("sa-palette");
        let off = self.smooth.begin(&ctx, sk, lc.layer_id(), None);
        let mut sa = egui::ScrollArea::vertical().id_salt("pal-list").auto_shrink(false);
        if let Some(o) = off {
            sa = sa.vertical_scroll_offset(o);
        }
        let sout = sa.show(&mut lc, |ui| {
            if res.is_empty() {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(t("No results")).color(p.text3));
            }
            for (i, (_, icon, label, detail, action)) in res.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.1);
                if i == sel {
                    ui.painter().rect_filled(r, CornerRadius::same(9), p.active);
                } else if hk > 0.0 {
                    ui.painter().rect_filled(r, CornerRadius::same(9), p.hover.gamma_multiply(hk));
                }
                if i == sel && (up || down) {
                    ui.scroll_to_rect(r, None);
                }
                let ir = Rect::from_min_size(pos2(r.left() + 12.0, r.center().y - 8.0), vec2(16.0, 16.0));
                if let Some(f) = icon.strip_prefix("file:") {
                    widgets::file_icon(ui, ir, f);
                } else if let Some(n) = icon.strip_prefix("icon:") {
                    widgets::icon(ui, ir, n, p.text2);
                }
                let lr = widgets::text(ui, pos2(r.left() + 38.0, r.center().y), Align2::LEFT_CENTER, label, theme::ui(13.5), p.text, r.width() * 0.55);
                widgets::text(ui, pos2(lr.right() + 10.0, r.center().y), Align2::LEFT_CENTER, detail, theme::ui(12.0), p.text3, r.right() - lr.right() - 20.0);
                if resp.clicked() {
                    pick = Some(action.clone());
                }
            }
        });
        self.smooth.end(sk, &sout);
        let card_clicked_outside = bg.clicked() && !card.contains(bg.interact_pointer_pos().unwrap_or_default());
        if esc || card_clicked_outside {
            self.palette = None;
            return;
        }
        if let Some(a) = pick {
            self.palette = None;
            if let Some(f) = a.strip_prefix("open:") {
                self.open_file(f);
            } else if let Some(c) = a.strip_prefix("cmd:") {
                self.act(c, &ctx);
            } else if let Some((pl, id)) = a.strip_prefix("plug:").and_then(|x| x.split_once('|')) {
                let (pl, id) = (pl.to_string(), id.to_string());
                self.plug_command(&pl, &id);
            } else if let Some(d) = a.strip_prefix("project:") {
                self.open_folder(d);
            } else if let Some(q) = a.strip_prefix("settings:") {
                self.open_settings("general", &ctx);
                self.settings_find(q);
            }
        }
    }

    // ---------- potvrdzovacie okno ----------
    pub(super) fn ask_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let Some(ask) = self.ask.as_ref() else { return };
        ui.interact(full, ui.id().with("ask-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(80));
        let w = 420.0;
        let g = ui.painter().layout(ask.text.clone(), theme::ui(13.5), p.text, w - 48.0);
        let h = g.size().y + 100.0;
        let card = Rect::from_center_size(full.center(), vec2(w, h));
        ui.painter().add(egui::Shadow { offset: [0, 14], blur: 40, spread: 0, color: Color32::from_black_alpha(110) }.as_shape(card, CornerRadius::same(14)));
        ui.painter().rect_filled(card, CornerRadius::same(14), p.solid);
        ui.painter().rect_stroke(card, CornerRadius::same(14), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        ui.painter().galley(card.min + vec2(24.0, 24.0), g, p.text);
        let mut b = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(pos2(card.left() + 20.0, card.bottom() - 52.0), pos2(card.right() - 20.0, card.bottom() - 18.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        b.spacing_mut().item_spacing.x = 8.0;
        let yes = widgets::button(&mut b, Some("trash"), &ask.yes, p.red, Color32::WHITE, 32.0, &p).clicked();
        let no = widgets::button(&mut b, None, &t("Cancel"), p.card2, p.text, 32.0, &p).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape));
        if yes {
            let a = ask.action.clone();
            self.ask = None;
            let ctx = ui.ctx().clone();
            self.act(&a, &ctx);
        } else if no {
            self.ask = None;
        }
    }
}
