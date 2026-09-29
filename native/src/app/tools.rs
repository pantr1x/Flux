// Nastavenia → Jazyky: programovacie jazyky na stiahnutie (ako tools.js v Electron Fluxe).
// Stav a inštalácie bežia vo vláknach, riadky sa prekresľujú podľa zdieľaného stavu.
use super::App;
use crate::i18n::{t, tf};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use flux_core::toolchains::{self, Status, TOOLCHAINS};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Default)]
pub struct Tools {
    pub status: Option<(Vec<Status>, bool)>,
    loaded: Option<Instant>,
    loading: bool,
    updates: HashMap<&'static str, (String, String)>,
    progress: HashMap<&'static str, (Option<u32>, String)>,
    installing: Option<&'static str>,
    queue: VecDeque<(&'static str, bool)>,
    errors: HashMap<&'static str, String>,
}

pub type Shared = Arc<Mutex<Tools>>;

// ikona jazyka (TOOL_FILE v tools.js)
fn file_of(id: &str) -> &'static str {
    match id {
        "git" => "a.git",
        "python" => "a.py",
        "node" => "a.js",
        "java" => "a.java",
        "cpp" => "a.cpp",
        "go" => "a.go",
        "csharp" => "a.cs",
        "rust" => "a.rs",
        "ruby" => "a.rb",
        "php" => "a.php",
        "lua" => "a.lua",
        "zig" => "a.zig",
        "r" => "a.r",
        "julia" => "a.jl",
        _ => "a.txt",
    }
}

fn mb(n: u32) -> String {
    if n >= 1000 {
        format!("{} GB", format!("{:.1}", n as f32 / 1000.0).trim_end_matches(".0"))
    } else {
        format!("{n} MB")
    }
}

// načíta stav (najviac raz za 5 min), potom aktualizácie z wingetu
pub fn refresh(tools: &Shared, ctx: &egui::Context, force: bool) {
    {
        let mut s = tools.lock().unwrap();
        if s.loading || (!force && s.loaded.map(|l| l.elapsed().as_secs() < 300).unwrap_or(false)) {
            return;
        }
        s.loading = true;
    }
    let (tools, ctx) = (tools.clone(), ctx.clone());
    std::thread::spawn(move || {
        let st = toolchains::status();
        {
            let mut s = tools.lock().unwrap();
            s.status = Some(st);
            s.loaded = Some(Instant::now());
        }
        ctx.request_repaint();
        let ups = toolchains::check_updates();
        let mut s = tools.lock().unwrap();
        s.updates = ups.into_iter().map(|(id, a, b)| (id, (a, b))).collect();
        s.loading = false;
        ctx.request_repaint();
    });
}

// pridá jazyk do frontu; inštalujú sa jeden po druhom
fn enqueue(tools: &Shared, ctx: &egui::Context, id: &'static str, upgrade: bool) {
    let start = {
        let mut s = tools.lock().unwrap();
        s.errors.remove(id);
        if s.installing == Some(id) || s.queue.iter().any(|q| q.0 == id) {
            return;
        }
        s.queue.push_back((id, upgrade));
        s.installing.is_none()
    };
    if start {
        let (tools, ctx) = (tools.clone(), ctx.clone());
        std::thread::spawn(move || loop {
            let Some((id, up)) = ({
                let mut s = tools.lock().unwrap();
                let n = s.queue.pop_front();
                s.installing = n.map(|x| x.0);
                n
            }) else {
                break;
            };
            let (t2, c2) = (tools.clone(), ctx.clone());
            let r = toolchains::install(id, up, move |pct, line| {
                t2.lock().unwrap().progress.insert(id, (pct, line.to_string()));
                c2.request_repaint();
            });
            let mut s = tools.lock().unwrap();
            s.progress.remove(id);
            match r {
                Ok(st) => {
                    if let (Some((list, _)), Some(i)) = (s.status.as_mut(), TOOLCHAINS.iter().position(|t| t.id == id)) {
                        list[i] = st;
                    }
                    s.updates.remove(id);
                }
                Err(e) => {
                    s.errors.insert(id, e);
                }
            }
            s.installing = None;
            ctx.request_repaint();
        });
    }
}

impl App {
    pub(super) fn tools_ui(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        refresh(&self.tools, ctx, false);
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new(t("Flux keeps its installer small. Programming languages are downloaded from their official sources only when you need them."))
                    .font(theme::ui(12.5))
                    .color(p.text2),
            )
            .wrap(),
        );
        ui.add_space(10.0);
        let s = self.tools.lock().unwrap();
        let Some((list, can_install)) = s.status.clone() else {
            drop(s);
            ui.label(egui::RichText::new(t("Checking…")).font(theme::ui(12.5)).color(p.text3));
            return;
        };
        let updates = s.updates.clone();
        let progress = s.progress.clone();
        let installing = s.installing;
        let queue: Vec<&str> = s.queue.iter().map(|q| q.0).collect();
        let errors = s.errors.clone();
        drop(s);
        let start = ui.cursor().top();
        let bg = ui.painter().add(egui::Shape::Noop);
        let mut act: Option<(&'static str, &'static str)> = None;
        for (i, tc) in TOOLCHAINS.iter().enumerate() {
            let st = &list[i];
            let (r, _) = ui.allocate_exact_size(vec2(w, 56.0), Sense::hover());
            if i > 0 {
                ui.painter().hline(r.left() + 16.0..=r.right() - 16.0, r.top(), Stroke::new(1.0, p.line));
            }
            widgets::file_icon(ui, Rect::from_min_size(pos2(r.left() + 16.0, r.center().y - 11.0), vec2(22.0, 22.0)), file_of(tc.id));
            let tx = r.left() + 52.0;
            let nw = widgets::text_w(ui, tc.name, theme::bold(13.0));
            ui.painter().text(pos2(tx, r.top() + 19.0), Align2::LEFT_CENTER, tc.name, theme::bold(13.0), p.text);
            if !tc.detail.is_empty() {
                ui.painter().text(pos2(tx + nw + 6.0, r.top() + 19.0), Align2::LEFT_CENTER, t(tc.detail), theme::ui(12.0), p.text3);
            }
            let my = r.top() + 38.0;
            let meta_w = w - 52.0 - 150.0;
            if installing == Some(tc.id) {
                let (pct, line) = progress.get(tc.id).cloned().unwrap_or((None, String::new()));
                let bar = Rect::from_min_size(pos2(tx, my - 2.0), vec2(120.0, 4.0));
                ui.painter().rect_filled(bar, CornerRadius::same(2), p.hover);
                ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(120.0 * pct.unwrap_or(4) as f32 / 100.0, 4.0)), CornerRadius::same(2), p.accent);
                widgets::text(ui, pos2(tx + 130.0, my), Align2::LEFT_CENTER, &if line.is_empty() { t("Starting…") } else { line }, theme::ui(11.5), p.text3, meta_w - 130.0);
            } else if let Some(e) = errors.get(tc.id) {
                widgets::text(ui, pos2(tx, my), Align2::LEFT_CENTER, e, theme::ui(11.5), p.red, meta_w);
            } else if let (true, Some((a, b))) = (st.installed, updates.get(tc.id)) {
                widgets::icon_at(ui, pos2(tx + 6.0, my), 12.0, "download", p.accent);
                let from = if st.version.is_empty() { a.clone() } else { st.version.clone() };
                widgets::text(ui, pos2(tx + 16.0, my), Align2::LEFT_CENTER, &tf("Update available: {from} → {to}", &[("from", &from), ("to", b)]), theme::ui(11.5), p.text2, meta_w);
            } else if st.installed {
                widgets::icon_at(ui, pos2(tx + 6.0, my), 12.0, "check", p.green);
                let v = if st.version.is_empty() { t("Installed") } else { format!("{} \u{00B7} {}", t("Installed"), st.version) };
                widgets::text(ui, pos2(tx + 16.0, my), Align2::LEFT_CENTER, &v, theme::ui(11.5), p.green, meta_w);
            } else if queue.contains(&tc.id) {
                ui.painter().text(pos2(tx, my), Align2::LEFT_CENTER, t("Waiting…"), theme::ui(11.5), p.text3);
            } else {
                let m = tf("{download} download · {disk} on disk", &[("download", &mb(tc.download)), ("disk", &mb(tc.disk))]);
                widgets::text(ui, pos2(tx, my), Align2::LEFT_CENTER, &m, theme::ui(11.5), p.text3, meta_w);
            }
            // tlačidlo vpravo
            let mut b = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_max(pos2(r.right() - 170.0, r.top() + 12.0), pos2(r.right() - 16.0, r.bottom() - 12.0)))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            if installing == Some(tc.id) {
                widgets::button(&mut b, None, &t("Installing…"), p.card2, p.text3, 30.0, &p);
            } else if queue.contains(&tc.id) {
            } else if st.installed && updates.contains_key(tc.id) && can_install {
                if widgets::button(&mut b, Some("download"), &t("Update"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                    act = Some((tc.id, "update"));
                }
            } else if st.installed {
            } else if !can_install {
                if widgets::button(&mut b, Some("external"), &t("Download"), p.card2, p.text, 30.0, &p).clicked() {
                    flux_core::settings::open_external(tc.url);
                }
            } else if widgets::button(&mut b, Some("download"), &t("Install"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                act = Some((tc.id, "install"));
            }
        }
        let rr = Rect::from_min_max(pos2(ui.min_rect().left(), start), pos2(ui.min_rect().left() + w, ui.cursor().top()));
        ui.painter().set(bg, egui::Shape::rect_filled(rr, CornerRadius::same(12), p.card2));
        ui.painter().rect_stroke(rr, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        if let Some((id, what)) = act {
            enqueue(&self.tools, ctx, id, what == "update");
        }
        ui.add_space(10.0);
        if widgets::button(ui, Some("refresh"), &t("Check again"), p.card2, p.text, 30.0, &p).clicked() {
            refresh(&self.tools, ctx, true);
        }
    }
}
