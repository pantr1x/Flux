// Python knižnice: okno s chýbajúcimi knižnicami (Inštalovať) a zastaranými (Aktualizovať); pip beží v paneli Output.
use super::{App, Bottom};
use crate::i18n::{t, tf};
use crate::pylibs::{self, Outdated};
use crate::{theme, widgets};
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use flux_core::runner;
use std::sync::{Arc, Mutex};

pub enum Out {
    Loading,
    Done(Vec<Outdated>),
    Err(String),
}

pub struct Libs {
    pub out: Arc<Mutex<Out>>,
}

impl App {
    fn py_path(&self) -> String {
        self.python.as_ref().and_then(|v| v["path"].as_str()).unwrap_or("").to_string()
    }

    // knižnice, ktoré chýbajú v aktívnom Python súbore
    pub(super) fn missing_libs(&self) -> Vec<String> {
        let Some(tab) = self.tabs.get(self.active) else { return vec![] };
        self.lib_missing.lock().unwrap().get(&tab.path).cloned().unwrap_or_default()
    }

    pub(super) fn open_libs(&mut self) {
        self.libs = Some(Libs { out: Arc::new(Mutex::new(Out::Loading)) });
        self.libs_check();
    }

    fn libs_check(&mut self) {
        let Some(l) = &self.libs else { return };
        let out = l.out.clone();
        *out.lock().unwrap() = Out::Loading;
        let py = self.py_path();
        std::thread::spawn(move || {
            *out.lock().unwrap() = if py.is_empty() { Out::Err("Python was not found.".into()) } else { pylibs::outdated(&py).map(Out::Done).unwrap_or_else(Out::Err) };
        });
    }

    // pip install [--upgrade] … v paneli Output aktívneho súboru
    pub(super) fn pip_run(&mut self, pkgs: &[String], upgrade: bool) {
        let py = self.py_path();
        if py.is_empty() {
            self.status = t("Python was not found. Install it from python.org or the Microsoft Store.");
            return;
        }
        let cwd = self.workspace().or_else(|| self.tabs.get(self.active).and_then(|t| std::path::Path::new(&t.path).parent().map(|p| p.to_string_lossy().to_string()))).unwrap_or_default();
        self.panel_open = true;
        let base = self.emit.clone();
        let Some(s) = self.sess_mut() else {
            self.status = t("Open a file first – pip shows its progress in the Output panel.");
            return;
        };
        s.bottom = Bottom::Output;
        s.out.clear();
        let mut args: Vec<String> = ["-m", "pip", "install"].iter().map(|s| s.to_string()).collect();
        if upgrade {
            args.push("--upgrade".into());
        }
        args.extend(pkgs.iter().cloned());
        let r = runner::start(&s.out.pty, &s.emit(&base), &runner::Cmd { cmd: py, args }, &cwd, "pip");
        if r["ok"] == serde_json::Value::Bool(false) {
            s.out.feed(&format!("\x1b[31m{}\x1b[0m\r\n", r["error"].as_str().unwrap_or("pip failed")));
            return;
        }
        self.pip_wait = true;
    }

    // otázka „Nainštalovať?“ po kliknutí na podčiarknutý import
    pub(super) fn ask_install(&mut self, pkg: &str) {
        self.ask = Some(super::menus::Ask {
            text: tf("Install the Python library \"{name}\"? Flux will run pip install {name}.", &[("name", pkg)]),
            yes: t("Install"),
            action: format!("pip-install:{pkg}"),
        });
    }

    // pip skončil: výsledky kontroly sa zahodia a súbory sa skontrolujú znova
    pub(super) fn pip_done(&mut self) {
        self.pip_wait = false;
        pylibs::forget();
        self.lint_asked.clear();
        if self.libs.is_some() {
            self.libs_check();
        }
    }

    pub(super) fn libs_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let bg = ui.interact(full, ui.id().with("libs-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(80));
        let w = (full.width() - 48.0).min(560.0);
        let h = (full.height() - 64.0).min(560.0);
        let card = Rect::from_center_size(full.center(), vec2(w, h));
        let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if bg.clicked() && !card.contains(bg.interact_pointer_pos().unwrap_or_default()) {
            close = true;
        }
        ui.painter().add(egui::Shadow { offset: [0, 30], blur: 90, spread: 0, color: Color32::from_black_alpha(128) }.as_shape(card, CornerRadius::same(20)));
        ui.painter().rect_filled(card, CornerRadius::same(20), p.solid);
        ui.painter().rect_stroke(card, CornerRadius::same(20), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let inner = card.shrink2(vec2(24.0, 20.0));
        let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        cu.set_clip_rect(card);
        cu.horizontal(|ui| {
            ui.label(egui::RichText::new(t("Python libraries")).font(theme::bold(20.0)).color(p.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::icon_button(ui, "x", &p, true).on_hover_text(t("Close (Esc)")).clicked() {
                    close = true;
                }
                if widgets::icon_button(ui, "refresh", &p, true).on_hover_text(t("Check again")).clicked() {
                    pylibs::forget();
                    self.lint_asked.clear();
                    self.libs_check();
                }
            });
        });
        cu.add_space(10.0);
        let missing = self.missing_libs();
        let outd = self.libs.as_ref().map(|l| match &*l.out.lock().unwrap() {
            Out::Loading => (true, vec![], String::new()),
            Out::Done(v) => (false, v.clone(), String::new()),
            Out::Err(e) => (false, vec![], e.clone()),
        });
        let (loading, outd, err) = outd.unwrap_or_default();
        let mut install: Option<(Vec<String>, bool)> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(inner.height() - 44.0).show(&mut cu, |ui| {
            ui.set_width(inner.width());
            ui.label(egui::RichText::new(t("Needed by this file")).font(theme::bold(12.5)).color(p.text2));
            ui.add_space(6.0);
            if missing.is_empty() {
                ui.label(egui::RichText::new(t("Nothing is missing.")).font(theme::ui(13.0)).color(p.text3));
            }
            for m in &missing {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(m).font(theme::mono(13.0)).color(p.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::button(ui, Some("download"), &t("Install"), p.accent, p.accent_fg, 28.0, &p).clicked() {
                            install = Some((vec![m.clone()], false));
                        }
                    });
                });
                ui.add_space(4.0);
            }
            if missing.len() > 1 && widgets::button(ui, Some("download"), &t("Install all"), p.card2, p.text, 28.0, &p).clicked() {
                install = Some((missing.clone(), false));
            }
            ui.add_space(14.0);
            ui.label(egui::RichText::new(t("Updates")).font(theme::bold(12.5)).color(p.text2));
            ui.add_space(6.0);
            if loading {
                ui.label(egui::RichText::new(t("Checking for updates…")).font(theme::ui(13.0)).color(p.text3));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            } else if !err.is_empty() {
                ui.label(egui::RichText::new(err).font(theme::ui(13.0)).color(p.red));
            } else if outd.is_empty() {
                ui.label(egui::RichText::new(t("All libraries are up to date.")).font(theme::ui(13.0)).color(p.text3));
            }
            for o in &outd {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&o.name).font(theme::mono(13.0)).color(p.text));
                    ui.label(egui::RichText::new(format!("{} → {}", o.current, o.latest)).font(theme::ui(12.0)).color(p.text3));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::button(ui, Some("refresh"), &t("Update"), p.card2, p.text, 28.0, &p).clicked() {
                            install = Some((vec![o.name.clone()], true));
                        }
                    });
                });
                ui.add_space(4.0);
            }
            if outd.len() > 1 && widgets::button(ui, Some("refresh"), &t("Update all"), p.card2, p.text, 28.0, &p).clicked() {
                install = Some((outd.iter().map(|o| o.name.clone()).collect(), true));
            }
        });
        let _ = pos2(0.0, 0.0);
        if let Some((pkgs, up)) = install {
            self.ask = Some(super::menus::Ask {
                text: if up { tf("Update {list}? Flux will run pip install --upgrade.", &[("list", &pkgs.join(", "))]) } else { tf("Install {list}? Flux will run pip install.", &[("list", &pkgs.join(", "))]) },
                yes: if up { t("Update") } else { t("Install") },
                action: format!("{}:{}", if up { "pip-update" } else { "pip-install" }, pkgs.join(" ")),
            });
        }
        if close && self.ask.is_none() {
            self.libs = None;
        }
    }
}
