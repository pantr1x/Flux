// Nový projekt – ikona (automaticky podľa jazyka, čiarová ikona alebo emoji), názov, krátky popis a umiestnenie,
// a kde: na tomto počítači, nový repozitár na GitHube, alebo import z GitHubu.
// Popis a ikona idú do settings.projectMeta[dir] (zdieľané s Electron Fluxom).
use super::App;
use crate::i18n::{t, tf};
use crate::{theme, widgets};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use flux_core::fsops;
use serde_json::{json, Value};
use std::path::Path;

// ikony na výber: čiarové ikony Fluxu a emoji (ako favicon)
const LINE_ICONS: [&str; 13] = ["robot", "code", "globe", "rocket", "star", "puzzle", "terminal", "sparkle", "flask", "palette", "camera", "phone", "todo"];
const EMOJI: [&str; 12] = ["🎮", "🐍", "🚀", "🎨", "🎵", "📊", "📱", "🔥", "⚡", "🐱", "💡", "🏆"];

#[derive(PartialEq, Clone, Copy)]
pub enum Where {
    Local,
    GitHub,
    Import,
}

pub struct NewProj {
    place: Where,
    name: String,
    desc: String,
    icon: String, // "" = automaticky (jazyk), "line:rocket", "emoji:🎮"
    root: String,
    private: bool,
    link: String, // import: odkaz na repozitár
    focus: bool,
    h: f32, // výška okna z minulého snímku (na vycentrovanie)
}

impl NewProj {
    pub fn new() -> Self {
        let root = fsops::default_root().to_string_lossy().to_string();
        let name = (1..100).map(|i| if i == 1 { "new-project".to_string() } else { format!("new-project-{i}") }).find(|n| !Path::new(&root).join(n).exists()).unwrap_or_else(|| "new-project".into());
        NewProj { place: Where::Local, name, desc: String::new(), icon: String::new(), root, private: true, link: String::new(), focus: true, h: 560.0 }
    }
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
        let resp = if spec.is_empty() { resp.on_hover_text(t("Automatic: the icon of the main language")) } else { resp };
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            pick = Some(spec.clone());
        }
    }
    pick
}

impl App {
    pub(super) fn open_new_project(&mut self) {
        self.new_project = Some(NewProj::new());
    }

    // ikona projektu: vlastná z projectMeta, inak hlavný jazyk, inak priečinok
    pub(super) fn project_icon(&self, dir: &str) -> String {
        if let Some(i) = self.core.setting("projectMeta")[dir]["icon"].as_str().filter(|i| !i.is_empty()) {
            return i.to_string();
        }
        match self.main_kind(dir).as_deref().map(super::kind_file).filter(|f| !f.is_empty()) {
            Some(f) => format!("file:{f}"),
            None => match self.core.setting("projectMeta")[dir]["kind"].as_str().map(super::kind_file).filter(|f| !f.is_empty()) {
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

    // vytvorí priečinok (a pri GitHube aj repozitár) a otvorí ho
    fn create_project(&mut self, ctx: &egui::Context) {
        let Some(np) = self.new_project.as_ref() else { return };
        let name = np.name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let (desc, icon, root, place, private) = (np.desc.trim().to_string(), np.icon.clone(), np.root.clone(), np.place, np.private);
        let dir = match fsops::create_project(&name, &root) {
            Ok(d) => d.as_str().unwrap_or("").to_string(),
            Err(e) => {
                self.status = t(&e);
                return;
            }
        };
        self.new_project = None;
        let mut meta = json!({ "description": desc, "todos": [] });
        if !icon.is_empty() {
            meta["icon"] = json!(icon);
        }
        self.set_project_meta(&dir, meta);
        self.start = false;
        self.open_folder(&dir);
        self.tree.clear();
        self.reload_projects();
        self.summarize(&dir);
        if place == Where::GitHub {
            self.gh_publish(&dir, &name, &desc, private, ctx);
            self.note(tf("Creating {name} on GitHub…", &[("name", &name)]));
        } else {
            self.note(tf("Project {name} is ready.", &[("name", &name)]));
        }
    }

    pub(super) fn new_project_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        self.gh_poll(&ctx);
        let Some(np) = self.new_project.as_ref() else { return };
        let bg = ui.interact(full, ui.id().with("np-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(80));
        let w = (full.width() - 48.0).min(600.0);
        let h = np.h.min(full.height() - 32.0);
        // horný okraj pevný – pri prepínaní kariet okno neposkakuje
        let top = full.top() + ((full.height() - 660.0) / 2.0).max(16.0);
        let card = Rect::from_min_size(pos2(full.center().x - w / 2.0, top), vec2(w, h.min(full.bottom() - 16.0 - top)));
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
        // kde: tento počítač / nový na GitHube / import z GitHubu
        let tabs = [(Where::Local, "laptop", "On this computer"), (Where::GitHub, "github", "New on GitHub"), (Where::Import, "download", "Import from GitHub")];
        let tw = (iw - 2.0 * 6.0) / 3.0;
        let (area, _) = cu.allocate_exact_size(vec2(iw, 40.0), Sense::hover());
        let cur = self.new_project.as_ref().unwrap().place;
        let mut pick = None;
        for (i, (wh, ic, label)) in tabs.iter().enumerate() {
            let r = Rect::from_min_size(area.min + vec2(i as f32 * (tw + 6.0), 0.0), vec2(tw, 40.0));
            let resp = cu.interact(r, cu.id().with(("np-where", i)), Sense::click());
            let on = *wh == cur;
            let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
            cu.painter().rect_filled(r, CornerRadius::same(11), if on { p.active } else { p.card2.lerp_to_gamma(p.hover, hk) });
            cu.painter().rect_stroke(r, CornerRadius::same(11), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
            let lw = widgets::text_w(&cu, &t(label), theme::bold(12.5));
            let x0 = r.center().x - (lw + 22.0) / 2.0;
            widgets::icon_at(&cu, pos2(x0 + 7.0, r.center().y), 14.0, ic, if on { p.text } else { p.text2 });
            widgets::text(&cu, pos2(x0 + 22.0, r.center().y), Align2::LEFT_CENTER, &t(label), theme::bold(12.5), if on { p.text } else { p.text2 }, tw - 34.0);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                pick = Some(*wh);
            }
        }
        if let Some(wh) = pick {
            self.new_project.as_mut().unwrap().place = wh;
            self.new_project.as_mut().unwrap().focus = true;
        }
        let place = self.new_project.as_ref().unwrap().place;
        let connected = self.gh_connected();
        let (busy, err, cloning) = self.gh_state();
        let mut create = false;
        let mut sign_in = false;
        // GitHub bez prihlásenia
        if place != Where::Local && !connected {
            cu.add_space(12.0);
            egui::Frame::new().fill(p.hover).stroke(Stroke::new(1.0, p.line)).corner_radius(12).inner_margin(egui::Margin::symmetric(14, 10)).show(&mut cu, |ui| {
                ui.set_width(iw - 28.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(if place == Where::GitHub {
                            t("Sign in with GitHub to create repositories.")
                        } else {
                            t("Sign in to see your repositories – or paste a link to a public one below.")
                        })
                        .font(theme::ui(12.5))
                        .color(p.text2),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::button(ui, Some("github"), &t("Sign in"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                            sign_in = true;
                        }
                    });
                });
            });
        }
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
        if place == Where::Import {
            // import: zoznam repozitárov (po prihlásení) alebo odkaz
            if connected {
                cu.add_space(8.0);
                let g = super::github::group_begin(&mut cu);
                self.gh_repos(&mut cu, iw, &ctx);
                super::github::group_end(&mut cu, g, iw, &p);
            }
            field(&mut cu, &t("Link to a repository"), false);
            let np = self.new_project.as_mut().unwrap();
            let mut go = false;
            cu.horizontal(|ui| {
                let te = ui.add(egui::TextEdit::singleline(&mut np.link).hint_text("https://github.com/owner/repo").desired_width(iw - 110.0).margin(egui::Margin::symmetric(12, 8)));
                if np.focus {
                    np.focus = false;
                    te.request_focus();
                }
                if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    go = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if cloning.is_some() { t("Cloning…") } else { t("Import") };
                    if widgets::button(ui, Some("download"), &label, p.accent, p.accent_fg, 32.0, &p).clicked() {
                        go = true;
                    }
                });
            });
            if go && cloning.is_none() {
                match super::github::repo_of(&np.link) {
                    Some(full) => self.gh_import(&full, &ctx),
                    None => self.status = t("That is not a link to a GitHub repository."),
                }
            }
        } else {
            let np = self.new_project.as_mut().unwrap();
            // ikona
            cu.add_space(14.0);
            cu.label(egui::RichText::new(t("Icon")).font(theme::bold(12.5)).color(p.text2));
            cu.add_space(6.0);
            if let Some(ic) = icon_picker(&mut cu, &np.icon, "", iw, &p) {
                np.icon = ic;
            }
            field(&mut cu, &t("Name"), false);
            let te = cu.add(egui::TextEdit::singleline(&mut np.name).desired_width(iw).margin(egui::Margin::symmetric(12, 8)).font(theme::ui(14.0)));
            if np.focus {
                np.focus = false;
                te.request_focus();
            }
            if te.lost_focus() && cu.input(|i| i.key_pressed(egui::Key::Enter)) {
                create = true;
            }
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
                }
            }
            if place == Where::GitHub {
                cu.add_space(12.0);
                cu.horizontal(|ui| {
                    ui.label(egui::RichText::new(t("Private repository")).font(theme::bold(12.5)).color(p.text2));
                    ui.label(egui::RichText::new(t("only you can see it")).font(theme::ui(11.5)).color(p.text3));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        super::prefs::switch(ui, &mut np.private, &p, true);
                    });
                });
            }
        }
        if let Some(e) = err.as_ref().filter(|_| place != Where::Local) {
            cu.add_space(10.0);
            cu.add(egui::Label::new(egui::RichText::new(e).font(theme::ui(12.0)).color(p.red)).wrap());
        }
        // pätička
        cu.add_space(20.0);
        cu.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if place != Where::Import {
                    let label = if place == Where::GitHub { t("Create on GitHub") } else { t("Create project") };
                    let ok = place == Where::Local || (connected && !busy);
                    if widgets::button(ui, Some(if place == Where::GitHub { "github" } else { "plus" }), &label, if ok { p.accent } else { p.card2 }, if ok { p.accent_fg } else { p.text3 }, 32.0, &p)
                        .clicked()
                        && ok
                    {
                        create = true;
                    }
                }
                if widgets::button(ui, None, &t("Cancel"), p.card2, p.text, 32.0, &p).clicked() {
                    close = true;
                }
            });
        });
        let used = cu.min_rect().height() + 40.0;
        if let Some(np) = self.new_project.as_mut() {
            if (np.h - used).abs() > 1.0 {
                np.h = used;
                ctx.request_repaint();
            }
        }
        if sign_in {
            self.new_project = None;
            self.open_settings("github", &ctx);
        } else if close {
            self.new_project = None;
        } else if create && (place == Where::Local || connected) {
            self.create_project(&ctx);
        }
    }
}
