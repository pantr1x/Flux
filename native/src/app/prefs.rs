// Nastavenia – ten istý panel ako openSettings() v Electron Fluxe: karty vľavo (s podpoložkami),
// hľadanie, sekcie so skupinami riadkov. Hodnoty idú do toho istého settings.json (rovnaké kľúče a predvolené hodnoty).
use super::App;
use crate::i18n::{t, tf};
use crate::theme::{self, ACCENTS};
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use serde_json::{json, Value};

// DEFAULTS z app.js (len kľúče, ktoré Flux Native používa alebo ukazuje)
pub fn default_of(key: &str) -> Value {
    match key {
        "codeTheme" | "lastDark" => json!(crate::gen::DEFAULT_THEME),
        "lastLight" => json!("vscode-light"),
        "accent" => json!("mono"),
        "fontFamily" => json!("Consolas"),
        "fontSize" => json!(14),
        "lineHeight" => json!(1.45),
        "minimap" | "autosave" | "clearOnRun" | "showSearch" | "transitions" | "inertia" | "trimMemory" | "bracketColors" | "autoUpdate" => json!(true),
        "wordWrap" | "autoReload" | "lite" | "searchWide" => json!(false),
        "uiZoom" => json!(100),
        "lineNumbers" => json!("on"),
        "cornerRadius" => json!(14),
        "darkLift" => json!(0),
        "density" => json!("comfortable"),
        "terminalFontSize" => json!(13),
        "panelPos" => json!("bottom"),
        "sidePos" => json!("left"),
        "userName" | "uiText" | "uiText2" | "uiBase" | "uiCard" | "uiLine" | "fontCustom" => json!(""),
        _ => Value::Null,
    }
}

// FONTS z app.js: (id, popis)
pub const FONTS: [(&str, &str); 6] = [
    ("Consolas", "Consolas (like VS Code)"),
    ("Cascadia Code", "Cascadia Code"),
    ("Cascadia Mono", "Cascadia Mono"),
    ("JetBrains Mono", "JetBrains Mono (if installed)"),
    ("Fira Code", "Fira Code (if installed)"),
    ("Courier New", "Courier New"),
];

const TABS: [(&str, &str, &str); 7] = [
    ("general", "settings", "General"),
    ("appearance", "palette", "Appearance"),
    ("editor", "code", "Editor"),
    ("running", "play", "Running"),
    ("tools", "download", "Languages"),
    ("plugins", "puzzle", "Plugins"),
    ("ai", "sparkle", "AI"),
];

#[derive(Clone)]
enum Row {
    Toggle(&'static str, &'static str, &'static str),
    Select(&'static str, &'static str, &'static str, Vec<(Value, String)>),
    Range(&'static str, &'static str, &'static str, f64, f64, f64),
    Number(&'static str, &'static str, f64, f64),
    Text(&'static str, &'static str, &'static str, &'static str),
    Color(&'static str, &'static str, &'static str),
    Button(&'static str, &'static str, &'static str, &'static str), // (akcia, názov, popis, tlačidlo)
    Info(&'static str, String),
    Custom(&'static str),
}

pub struct SettingsUi {
    pub tab: String,
    find: String,
    jump: Option<String>,
    pub opened: f64,
}

impl SettingsUi {
    pub fn new(tab: &str, now: f64) -> Self {
        SettingsUi { tab: tab.to_string(), find: String::new(), jump: None, opened: now }
    }
}

fn sections(tab: &str, app: &App) -> Vec<(&'static str, Vec<Row>)> {
    let o = |v: &[(&str, &str)]| v.iter().map(|(a, b)| (json!(a), t(b))).collect::<Vec<_>>();
    match tab {
        "general" => vec![
            ("About & updates", vec![Row::Custom("about")]),
            ("You", vec![Row::Text("userName", "Your name", "for the greeting on the home screen", "e.g. Šimon")]),
            (
                "Memory & speed",
                vec![
                    Row::Custom("memory"),
                    Row::Toggle("lite", "Save memory", "Uses less memory and turns off effects – good for slower PCs. You can change each part under Advanced."),
                    Row::Toggle("trimMemory", "Free memory when Flux is in the background", "Windows moves unused memory out of RAM while you work in another app. Switching back can take a moment."),
                ],
            ),
            ("Language", vec![Row::Custom("language")]),
            ("Welcome", vec![Row::Button("intro", "Intro and tour", "Replay the first-start intro or the feature tour.", "Intro")]),
            ("Shortcuts", vec![Row::Custom("keys")]),
        ],
        "appearance" => vec![
            ("Code theme", vec![Row::Custom("themes")]),
            ("Accent color", vec![Row::Custom("accents")]),
            (
                "App colors",
                vec![
                    Row::Color("uiText", "Text", "menus, file names and buttons"),
                    Row::Color("uiText2", "Secondary text", "hints and small text"),
                    Row::Color("uiBase", "Background", "behind the panels"),
                    Row::Color("uiCard", "Panels", "editor, sidebar and windows"),
                    Row::Color("uiLine", "Borders", "lines between parts"),
                    Row::Button("colors-reset", "Reset all colors", "back to the colors of your theme", "Reset"),
                ],
            ),
            (
                "Text & fonts",
                vec![
                    Row::Select("lineNumbers", "Line numbers", "", o(&[("on", "on"), ("relative", "relative"), ("off", "off")])),
                    Row::Toggle("bracketColors", "Colored brackets", "matching brackets get the same color"),
                ],
            ),
            (
                "Window",
                vec![
                    Row::Toggle("inertia", "Smooth scrolling with inertia", "the editor, settings, lists and panels keep gliding a bit after you stop the wheel"),
                    Row::Toggle("transitions", "Transition animations", "a soft fade when you switch files, settings pages and screens"),
                    Row::Toggle("showSearch", "Search button", "a magnifier at the top that finds files, commands and settings"),
                    Row::Select("panelPos", "Panel position", "where output and the terminal are", o(&[("bottom", "Bottom"), ("right", "Right"), ("left", "Left")])),
                    Row::Select("sidePos", "Sidebar position", "projects and files", o(&[("left", "Left"), ("right", "Right")])),
                    Row::Range("uiZoom", "Size of everything", "", 80.0, 140.0, 5.0),
                    Row::Select("density", "Density", "Compact fits more files, tabs and lines on the screen.", o(&[("comfortable", "comfortable"), ("compact", "compact")])),
                    Row::Range("cornerRadius", "Rounded corners", "", 0.0, 26.0, 1.0),
                    Row::Range("darkLift", "Brightness of dark areas", "Only backgrounds get lighter – text and outlines stay the same. Turn it up if your wallpaper is very dark.", 0.0, 100.0, 1.0),
                ],
            ),
            ("", vec![Row::Button("look-reset", "Reset the look", "cursor, pointer, fonts, size, corners and background go back to default – your theme and colors stay", "Reset all")]),
        ],
        "editor" => vec![
            (
                "Text",
                vec![
                    Row::Select("fontFamily", "Font", "", FONTS.iter().map(|(id, l)| (json!(id), t(l))).collect()),
                    Row::Number("fontSize", "Font size", 9.0, 32.0),
                    Row::Select("lineHeight", "Line height", "", vec![(json!(1.3), t("compact")), (json!(1.45), t("normal")), (json!(1.6), t("relaxed")), (json!(1.8), t("large"))]),
                    Row::Toggle("wordWrap", "Wrap long lines", ""),
                ],
            ),
            (
                "Behaviour",
                vec![
                    Row::Toggle("minimap", "Code map", "small preview of the code on the right"),
                    Row::Toggle("autosave", "Auto save", "saves the file shortly after you stop typing"),
                    Row::Toggle("autoReload", "Reload changed files without asking", "when another program changes an open file, the editor shows the new version right away"),
                ],
            ),
        ],
        "running" => {
            let py = app.python.as_ref().map(|p| format!("{} · {}", p["version"].as_str().unwrap_or(""), p["path"].as_str().unwrap_or(""))).unwrap_or_else(|| t("not found"));
            vec![
                ("Python", vec![Row::Info("Interpreter", py), Row::Button("python", "Choose another Python", "", "Change…")]),
                ("Output", vec![Row::Toggle("clearOnRun", "Clear output before running", ""), Row::Number("terminalFontSize", "Output font size", 9.0, 28.0)]),
            ]
        }
        "tools" => vec![("", vec![Row::Custom("tools")])],
        "plugins" => vec![("", vec![Row::Custom("plugins")])],
        _ => vec![("", vec![Row::Custom("ai")])],
    }
}

fn row_words(r: &Row) -> String {
    match r {
        Row::Toggle(_, a, b) | Row::Range(_, a, b, ..) | Row::Text(_, a, b, _) | Row::Color(_, a, b) | Row::Select(_, a, b, _) => format!("{} {}", t(a), t(b)),
        Row::Number(_, a, ..) => t(a),
        Row::Button(_, a, b, _) => format!("{} {}", t(a), t(b)),
        Row::Info(a, b) => format!("{} {}", t(a), b),
        Row::Custom(id) => id.to_string(),
    }
    .to_lowercase()
}

impl App {
    // hodnota nastavenia s predvolenou hodnotou z DEFAULTS
    pub(super) fn get(&self, key: &str) -> Value {
        let v = self.core.setting(key);
        if v.is_null() {
            default_of(key)
        } else {
            v
        }
    }

    pub(super) fn set(&mut self, key: &str, v: Value, ctx: &egui::Context) {
        self.update_settings(|o| {
            o.insert(key.into(), v);
        });
        match key {
            "language" => crate::i18n::set_language(self.get("language").as_str().unwrap_or("en")),
            "fontFamily" | "fontCustom" => theme::fonts(ctx, self.get("fontFamily").as_str().unwrap_or("Consolas")),
            _ => {}
        }
        self.apply_look(ctx);
    }

    pub(super) fn settings_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        let Some(st) = self.settings.as_ref() else { return };
        // animácia otvorenia (fade + mierne zväčšenie), vypnutá bez „transitions“
        let anim = self.anim_on();
        let k = if anim { (((ctx.input(|i| i.time) - st.opened) / 0.18).clamp(0.0, 1.0)) as f32 } else { 1.0 };
        let k = 1.0 - (1.0 - k).powi(3);
        if k < 1.0 {
            ctx.request_repaint();
        }
        // stmavenie pozadia; klik mimo zavrie
        let bg = ui.interact(full, ui.id().with("s-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha((if p.dark { 90.0 } else { 40.0 } * k) as u8));
        let w = (full.width() - 140.0).clamp(640.0, 1000.0);
        let h = full.height() - 78.0;
        let card = Rect::from_center_size(full.center(), vec2(w, h) * (0.97 + 0.03 * k));
        if bg.clicked() && !card.contains(bg.interact_pointer_pos().unwrap_or_default()) {
            self.settings = None;
            return;
        }
        let alpha = k;
        ui.painter().add(egui::Shadow { offset: [0, 18], blur: 50, spread: 0, color: Color32::from_black_alpha((120.0 * alpha) as u8) }.as_shape(card, CornerRadius::same(16)));
        ui.painter().rect_filled(card, CornerRadius::same(16), p.card.gamma_multiply(alpha).to_opaque().lerp_to_gamma(p.card, alpha));
        ui.painter().rect_stroke(card, CornerRadius::same(16), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        // ---- ľavá ponuka ----
        let nav = Rect::from_min_size(card.min, vec2(210.0, card.height()));
        ui.painter().rect_filled(nav, CornerRadius { nw: 16, sw: 16, ne: 0, se: 0 }, p.card2);
        ui.painter().vline(nav.right(), nav.y_range(), Stroke::new(1.0, p.line));
        ui.painter().text(pos2(nav.left() + 20.0, nav.top() + 31.0), Align2::LEFT_CENTER, t("Settings"), theme::bold(18.0), p.text);
        let fr = Rect::from_min_size(pos2(nav.left() + 10.0, nav.top() + 60.0), vec2(nav.width() - 20.0, 32.0));
        ui.painter().rect_filled(fr, CornerRadius::same(10), p.hover);
        ui.painter().rect_stroke(fr, CornerRadius::same(10), Stroke::new(1.0, p.line), StrokeKind::Inside);
        widgets::icon_at(ui, pos2(fr.left() + 16.0, fr.center().y), 13.0, "search", p.text3);
        let st = self.settings.as_mut().unwrap();
        let mut fc = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(fr.left() + 30.0, fr.top() + 8.0), pos2(fr.right() - 8.0, fr.bottom() - 6.0))));
        fc.add(egui::TextEdit::singleline(&mut st.find).hint_text(t("Search settings…")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(13.0)));
        let finding = st.find.trim().to_lowercase();
        let cur_tab = st.tab.clone();
        let mut y = fr.bottom() + 12.0;
        let mut go_tab = None;
        let mut go_sub = None;
        for (id, ic, label) in TABS {
            let r = Rect::from_min_size(pos2(nav.left() + 10.0, y), vec2(nav.width() - 20.0, 36.0));
            let resp = ui.interact(r, ui.id().with(("stab", id)), Sense::click());
            let on = id == cur_tab && finding.is_empty();
            let hov = ctx.animate_bool_with_time(ui.id().with(("stab-h", id)), resp.hovered() || on, if anim { 0.12 } else { 0.0 });
            if hov > 0.0 {
                ui.painter().rect_filled(r, CornerRadius::same(10), if on { p.active } else { p.hover.gamma_multiply(hov) });
            }
            widgets::icon_at(ui, pos2(r.left() + 17.0, r.center().y), 15.0, ic, if on { p.text } else { p.text2 });
            ui.painter().text(pos2(r.left() + 36.0, r.center().y), Align2::LEFT_CENTER, t(label), theme::bold(13.0), if on { p.text } else { p.text2 });
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                go_tab = Some(id.to_string());
            }
            y += 38.0;
            // podpoložky (h3) pre General a Appearance
            if on && (id == "general" || id == "appearance") {
                let subs: Vec<&str> = sections(id, self).iter().map(|s| s.0).filter(|s| !s.is_empty()).collect();
                let top = y;
                for s in subs {
                    let r = Rect::from_min_size(pos2(nav.left() + 42.0, y), vec2(nav.width() - 52.0, 26.0));
                    let resp = ui.interact(r, ui.id().with(("ssub", s)), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, CornerRadius::same(7), p.hover);
                    }
                    widgets::text(ui, pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, &t(s), theme::ui(12.5), if resp.hovered() { p.text } else { p.text2 }, r.width() - 14.0);
                    if resp.clicked() {
                        go_sub = Some(s.to_string());
                    }
                    y += 26.0;
                }
                ui.painter().vline(nav.left() + 33.0, top..=y, Stroke::new(1.0, p.line));
                y += 6.0;
            }
        }
        ui.painter().text(pos2(nav.left() + 20.0, nav.bottom() - 22.0), Align2::LEFT_CENTER, format!("Flux Native {}", env!("CARGO_PKG_VERSION")), theme::ui(12.0), p.text3);
        if let Some(tb) = go_tab {
            let st = self.settings.as_mut().unwrap();
            st.tab = tb;
            st.find.clear();
            st.opened = ctx.input(|i| i.time) - 0.1;
        }
        if let Some(s) = go_sub {
            self.settings.as_mut().unwrap().jump = Some(s);
        }
        // ---- pravá časť: hlavička + obsah ----
        let main = Rect::from_min_max(pos2(nav.right() + 1.0, card.top()), card.max);
        let st = self.settings.as_ref().unwrap();
        let title = if finding.is_empty() { TABS.iter().find(|x| x.0 == st.tab).map(|x| t(x.2)).unwrap_or_default() } else { t("Search settings…").trim_end_matches('…').to_string() };
        ui.painter().text(pos2(main.left() + 28.0, main.top() + 32.0), Align2::LEFT_CENTER, &title, theme::bold(20.0), p.text);
        let close = Rect::from_center_size(pos2(main.right() - 32.0, main.top() + 32.0), vec2(30.0, 30.0));
        let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if widgets::icon_button_at(ui, close, "x", 16.0, &p, true).on_hover_text(t("Close (Esc)")).clicked() || esc {
            self.settings = None;
            return;
        }
        ui.painter().hline(main.x_range(), main.top() + 64.0, Stroke::new(1.0, p.line));
        let body = Rect::from_min_max(pos2(main.left() + 1.0, main.top() + 65.0), pos2(main.right() - 1.0, main.bottom() - 8.0));
        let mut bui = ui.new_child(egui::UiBuilder::new().max_rect(body));
        bui.set_clip_rect(body);
        // obsah – pri hľadaní všetky karty, inak len aktívna; prechod: jemný posun + fade
        let tabs: Vec<&str> = if finding.is_empty() { vec![TABS.iter().find(|x| x.0 == st.tab).map(|x| x.0).unwrap_or("general")] } else { TABS.iter().map(|x| x.0).collect() };
        let jump = self.settings.as_mut().unwrap().jump.take();
        let slide = (1.0 - k) * 14.0;
        egui::ScrollArea::vertical().id_salt(("s-body", tabs.join(","))).auto_shrink(false).show(&mut bui, |ui| {
            ui.add_space(8.0 + slide);
            let inner = ui.available_width() - 56.0;
            for tab in tabs {
                for (title, rows) in sections(tab, self) {
                    let rows: Vec<Row> = if finding.is_empty() {
                        rows
                    } else {
                        rows.into_iter().filter(|r| finding.split_whitespace().all(|w| row_words(r).contains(w) || t(title).to_lowercase().contains(w))).collect()
                    };
                    if rows.is_empty() {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.add_space(28.0);
                        ui.vertical(|ui| {
                            ui.set_width(inner);
                            if !title.is_empty() {
                                ui.add_space(14.0);
                                let (r, _) = ui.allocate_exact_size(vec2(inner, 22.0), Sense::hover());
                                let mut job = egui::text::LayoutJob::default();
                                job.append(&t(title).to_uppercase(), 0.0, egui::TextFormat { font_id: theme::bold(11.0), color: p.text3, extra_letter_spacing: 1.0, ..Default::default() });
                                let g = ui.fonts_mut(|f| f.layout_job(job));
                                ui.painter().galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, p.text3);
                                if jump.as_deref() == Some(title) {
                                    ui.scroll_to_rect(r, Some(egui::Align::TOP));
                                }
                                ui.add_space(6.0);
                            } else {
                                ui.add_space(10.0);
                            }
                            self.group(ui, rows, inner, &ctx);
                        });
                    });
                }
            }
            ui.add_space(30.0);
        });
    }

    fn group(&mut self, ui: &mut egui::Ui, rows: Vec<Row>, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        let custom_only = rows.len() == 1 && matches!(rows[0], Row::Custom(id) if matches!(id, "themes" | "accents" | "language" | "keys" | "tools" | "plugins" | "ai" | "about"));
        if custom_only {
            if let Row::Custom(id) = rows[0] {
                self.custom(ui, id, w, ctx);
            }
            return;
        }
        let start = ui.cursor().top();
        let bg = ui.painter().add(egui::Shape::Noop);
        let n = rows.len();
        for (i, row) in rows.into_iter().enumerate() {
            let top = ui.cursor().top();
            ui.horizontal(|ui| {
                ui.set_min_height(54.0);
                ui.add_space(16.0);
                self.row(ui, row, w - 32.0, ctx);
            });
            let bottom = ui.cursor().top();
            if i + 1 < n {
                ui.painter().hline(ui.min_rect().left() + 16.0..=ui.min_rect().left() + w - 16.0, bottom - 2.0, Stroke::new(1.0, p.line));
            }
            let _ = top;
        }
        let r = Rect::from_min_max(pos2(ui.min_rect().left(), start), pos2(ui.min_rect().left() + w, ui.cursor().top()));
        ui.painter().set(bg, egui::Shape::rect_filled(r, CornerRadius::same(12), p.card2));
        ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
    }

    fn label(&self, ui: &mut egui::Ui, title: &str, hint: &str, w: f32) {
        let p = self.pal;
        ui.vertical(|ui| {
            ui.set_width(w);
            ui.add_space(9.0);
            ui.label(egui::RichText::new(t(title)).font(theme::bold(13.0)).color(p.text));
            if !hint.is_empty() {
                ui.add(egui::Label::new(egui::RichText::new(t(hint)).font(theme::ui(11.5)).color(p.text3)).wrap());
            }
            ui.add_space(9.0);
        });
    }

    fn row(&mut self, ui: &mut egui::Ui, row: Row, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        let lw = w * 0.62;
        match row {
            Row::Toggle(key, title, hint) => {
                self.label(ui, title, hint, lw);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut on = self.get(key).as_bool().unwrap_or(false);
                    if switch(ui, &mut on, &p, self.anim_on()) {
                        self.set(key, json!(on), ctx);
                    }
                });
            }
            Row::Select(key, title, hint, opts) => {
                self.label(ui, title, hint, lw);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cur = self.get(key);
                    let cur_label = opts.iter().find(|o| o.0 == cur || o.0.as_f64().is_some() && o.0.as_f64() == cur.as_f64()).map(|o| o.1.clone()).unwrap_or_else(|| cur.to_string());
                    let mut pick = None;
                    egui::ComboBox::from_id_salt(("sel", key)).selected_text(cur_label).width(190.0).show_ui(ui, |ui| {
                        for (v, l) in &opts {
                            if ui.selectable_label(*v == cur, l).clicked() {
                                pick = Some(v.clone());
                            }
                        }
                    });
                    if let Some(v) = pick {
                        self.set(key, v, ctx);
                    }
                });
            }
            Row::Range(key, title, hint, min, max, step) => {
                self.label(ui, title, hint, lw);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut v = self.get(key).as_f64().unwrap_or(min);
                    ui.spacing_mut().slider_width = 180.0;
                    let r = ui.add(egui::Slider::new(&mut v, min..=max).step_by(step).show_value(key == "uiZoom" || key == "cornerRadius"));
                    if r.drag_stopped() || (r.changed() && !r.dragged()) || (r.changed() && key == "darkLift") {
                        self.set(key, json!(v), ctx);
                    }
                });
            }
            Row::Number(key, title, min, max) => {
                self.label(ui, title, "", lw);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut v = self.get(key).as_f64().unwrap_or(min);
                    if ui.add(egui::DragValue::new(&mut v).range(min..=max).speed(0.2)).changed() {
                        self.set(key, json!(v.round()), ctx);
                    }
                });
            }
            Row::Text(key, title, hint, placeholder) => {
                self.label(ui, title, hint, lw);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut s = self.get(key).as_str().unwrap_or("").to_string();
                    if ui.add(egui::TextEdit::singleline(&mut s).hint_text(t(placeholder)).desired_width(200.0).margin(egui::Margin::symmetric(10, 6))).changed() {
                        self.set(key, json!(s), ctx);
                    }
                });
            }
            Row::Color(key, title, hint) => {
                let custom = self.get(key).as_str().filter(|s| !s.is_empty()).map(String::from);
                let hint2 = if custom.is_none() { format!("{} · {}", t(hint), t("from the current theme")) } else { t(hint) };
                ui.vertical(|ui| {
                    ui.set_width(lw);
                    ui.add_space(9.0);
                    ui.label(egui::RichText::new(t(title)).font(theme::bold(13.0)).color(p.text));
                    ui.label(egui::RichText::new(hint2).font(theme::ui(11.5)).color(p.text3));
                    ui.add_space(9.0);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::button(ui, None, &t("Reset"), p.card2, p.text, 30.0, &p).clicked() {
                        self.set(key, json!(""), ctx);
                    }
                    let cur = match key {
                        "uiText" => p.text,
                        "uiText2" => p.text2,
                        "uiBase" => p.base,
                        "uiCard" => p.card,
                        _ => p.line_strong.to_opaque(),
                    };
                    let mut c = cur;
                    let hex = format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
                    ui.label(egui::RichText::new(hex).font(theme::mono(11.5)).color(p.text));
                    if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque).changed() {
                        self.set(key, json!(format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())), ctx);
                    }
                });
            }
            Row::Button(action, title, hint, button) => {
                self.label(ui, title, hint, lw);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let icon = if action.ends_with("reset") { Some("refresh") } else { None };
                    if widgets::button(ui, icon, &t(button), p.card2, p.text, 30.0, &p).clicked() {
                        self.action(action, ctx);
                    }
                });
            }
            Row::Info(title, value) => self.label_owned(ui, title, &value, lw),
            Row::Custom(id) => self.custom(ui, id, w, ctx),
        }
    }

    fn label_owned(&self, ui: &mut egui::Ui, title: &str, value: &str, w: f32) {
        let p = self.pal;
        ui.vertical(|ui| {
            ui.set_width(w);
            ui.add_space(9.0);
            ui.label(egui::RichText::new(t(title)).font(theme::bold(13.0)).color(p.text));
            ui.label(egui::RichText::new(value).font(theme::ui(11.5)).color(p.text3));
            ui.add_space(9.0);
        });
    }

    fn action(&mut self, action: &str, ctx: &egui::Context) {
        match action {
            "intro" => {
                self.settings = None;
                self.intro = Some(super::intro::Intro::new(self.get("userName").as_str().unwrap_or("")));
            }
            "colors-reset" => {
                for k in ["uiText", "uiText2", "uiBase", "uiCard", "uiLine"] {
                    self.update_settings(|o| {
                        o.remove(k);
                    });
                }
                self.apply_look(ctx);
            }
            "look-reset" => {
                for k in ["uiZoom", "cornerRadius", "darkLift", "density", "lineNumbers", "fontCustom"] {
                    self.update_settings(|o| {
                        o.remove(k);
                    });
                }
                self.apply_look(ctx);
            }
            "python" => {
                if let Some(f) = rfd::FileDialog::new().set_title(t("Select Python interpreter")).pick_file() {
                    let f = f.to_string_lossy().to_string();
                    match flux_core::python::probe(&f, &[]) {
                        Some(mut info) => {
                            info["source"] = json!("chosen manually");
                            if let Some(ws) = self.workspace() {
                                let path = f.clone();
                                self.update_settings(|o| {
                                    let mut all = o.get("pythonOverrides").cloned().unwrap_or(json!({}));
                                    all[&ws] = json!(path);
                                    o.insert("pythonOverrides".into(), all);
                                });
                            }
                            self.python = Some(info);
                        }
                        None => self.status = t("The selected file is not a working Python."),
                    }
                }
            }
            _ => {}
        }
    }

    fn custom(&mut self, ui: &mut egui::Ui, id: &str, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        match id {
            "about" => {
                let (r, _) = ui.allocate_exact_size(vec2(w, 86.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(12), p.card2);
                ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                let lr = Rect::from_min_size(pos2(r.left() + 16.0, r.top() + 17.0), vec2(52.0, 52.0));
                widgets::brand_mark(ui, lr, &p);
                ui.painter().text(pos2(lr.right() + 14.0, r.top() + 32.0), Align2::LEFT_CENTER, "Flux Native", theme::bold(17.0), p.text);
                ui.painter().text(pos2(lr.right() + 14.0, r.top() + 55.0), Align2::LEFT_CENTER, tf("Version {v}", &[("v", env!("CARGO_PKG_VERSION"))]), theme::ui(12.5), p.text3);
                let mut b = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_max(pos2(r.right() - 330.0, r.top() + 27.0), pos2(r.right() - 16.0, r.top() + 60.0)))
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                if widgets::button(&mut b, Some("globe"), &t("All versions"), p.card2, p.text, 30.0, &p).clicked() {
                    flux_core::settings::open_external("https://pantr1x.github.io/Flux/#releases");
                }
            }
            "memory" => {
                let mb = crate::mem::used_mb();
                self.label_owned(ui, "Memory used by Flux", &mb.map(|m| format!("{m:.1} MB")).unwrap_or_else(|| "–".into()), w * 0.62);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::button(ui, Some("refresh"), &t("Free memory"), p.card2, p.text, 30.0, &p).clicked() {
                        crate::mem::trim_now();
                    }
                });
                ctx.request_repaint_after(std::time::Duration::from_secs(2));
            }
            "language" => {
                let langs: Vec<Value> = serde_json::from_str(crate::i18n::LANGS).unwrap_or_default();
                let cur = self.get("language").as_str().unwrap_or("en").to_string();
                let (cw, ch) = ((w - 16.0) / 3.0, 42.0);
                let (area, _) = ui.allocate_exact_size(vec2(w, ((langs.len() + 2) / 3) as f32 * (ch + 8.0)), Sense::hover());
                let mut pick = None;
                for (i, l) in langs.iter().enumerate() {
                    let code = l["code"].as_str().unwrap_or("en");
                    let r = Rect::from_min_size(area.min + vec2((i % 3) as f32 * (cw + 8.0), (i / 3) as f32 * (ch + 8.0)), vec2(cw, ch));
                    let resp = ui.interact(r, ui.id().with(("slang", code)), Sense::click());
                    ui.painter().rect_filled(r, CornerRadius::same(10), if resp.hovered() { p.hover } else { p.card2 });
                    ui.painter().rect_stroke(r, CornerRadius::same(10), if code == cur { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
                    if let Some(svg) = crate::gen::flag(code) {
                        egui::Image::from_bytes(format!("bytes://flag/{code}.svg"), svg.as_bytes()).paint_at(ui, Rect::from_min_size(pos2(r.left() + 10.0, r.center().y - 7.0), vec2(21.0, 14.0)));
                    }
                    ui.painter().text(pos2(r.left() + 39.0, r.top() + 15.0), Align2::LEFT_CENTER, l["native"].as_str().unwrap_or(code), theme::bold(12.0), p.text);
                    ui.painter().text(pos2(r.left() + 39.0, r.top() + 29.0), Align2::LEFT_CENTER, l["name"].as_str().unwrap_or(""), theme::ui(10.0), p.text3);
                    if resp.clicked() {
                        pick = Some(code.to_string());
                    }
                }
                if let Some(c) = pick {
                    self.set("language", json!(c), ctx);
                }
            }
            "keys" => {
                let keys = [
                    ("Run current file", "F5"),
                    ("Save", "Ctrl+S"),
                    ("Open folder", "Ctrl+O"),
                    ("Close tab", "Ctrl+W"),
                    ("Settings", "Ctrl+,"),
                    ("Toggle output panel", "Ctrl+J"),
                    ("Toggle sidebar", "Ctrl+B"),
                    ("Toggle light / dark theme", "Ctrl+Shift+L"),
                ];
                let start = ui.cursor().top();
                let bg = ui.painter().add(egui::Shape::Noop);
                for (i, (label, key)) in keys.iter().enumerate() {
                    let (r, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
                    ui.painter().text(pos2(r.left() + 16.0, r.center().y), Align2::LEFT_CENTER, t(label), theme::ui(13.0), p.text);
                    let kw = widgets::text_w(ui, key, theme::mono(11.5)) + 16.0;
                    let kr = Rect::from_min_size(pos2(r.right() - 16.0 - kw, r.center().y - 11.0), vec2(kw, 22.0));
                    ui.painter().rect_stroke(kr, CornerRadius::same(6), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                    ui.painter().text(kr.center(), Align2::CENTER_CENTER, *key, theme::mono(11.5), p.text);
                    if i + 1 < keys.len() {
                        ui.painter().hline(r.left() + 16.0..=r.right() - 16.0, r.bottom(), Stroke::new(1.0, p.line));
                    }
                }
                let r = Rect::from_min_max(pos2(ui.min_rect().left(), start), pos2(ui.min_rect().left() + w, ui.cursor().top()));
                ui.painter().set(bg, egui::Shape::rect_filled(r, CornerRadius::same(12), p.card2));
                ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            }
            "themes" => {
                let cur = self.get("codeTheme").as_str().unwrap_or("").to_string();
                let ids = ["vscode-dark", "flux", "one-dark", "dracula", "tokyo-night", "catppuccin", "nord", "github-dark", "monokai", "vscode-light", "github-light", "catppuccin-latte"];
                let names =
                    ["VS Code Dark", "Flux", "One Dark Pro", "Dracula", "Tokyo Night", "Catppuccin Mocha", "Nord", "GitHub Dark", "Monokai", "VS Code Light", "GitHub Light", "Catppuccin Latte"];
                let cw = (w - 16.0) / 3.0;
                let (area, _) = ui.allocate_exact_size(vec2(w, 4.0 * 59.0), Sense::hover());
                let mut pick = None;
                for (i, id) in ids.iter().enumerate() {
                    let r = Rect::from_min_size(area.min + vec2((i % 3) as f32 * (cw + 8.0), (i / 3) as f32 * 59.0), vec2(cw, 52.0));
                    let resp = ui.interact(r, ui.id().with(("sth", *id)), Sense::click());
                    let on = *id == cur;
                    ui.painter().rect_filled(r, CornerRadius::same(12), if resp.hovered() { p.hover } else { p.card2 });
                    ui.painter().rect_stroke(r, CornerRadius::same(12), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line_strong) }, StrokeKind::Inside);
                    let (dark, c, _) = crate::code::theme_of(id);
                    for (k, idx) in [2usize, 7, 4, 5, 6].iter().enumerate() {
                        ui.painter().rect_filled(Rect::from_min_size(pos2(r.left() + 10.0 + k as f32 * 8.0, r.center().y - 7.0), vec2(6.0, 14.0)), CornerRadius::same(2), theme::hex(c[*idx]));
                    }
                    ui.painter().text(pos2(r.left() + 62.0, r.top() + 18.0), Align2::LEFT_CENTER, names[i], theme::bold(13.0), p.text);
                    ui.painter().text(pos2(r.left() + 62.0, r.top() + 36.0), Align2::LEFT_CENTER, if dark { t("dark") } else { t("light") }, theme::ui(11.5), p.text3);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        pick = Some(id.to_string());
                    }
                }
                if let Some(id) = pick {
                    self.set_code_theme(&id, ctx);
                }
            }
            "accents" => {
                let cur = self.get("accent").as_str().unwrap_or("mono").to_string();
                let (r, _) = ui.allocate_exact_size(vec2(w, 56.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(12), p.card2);
                ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                let mut pick = None;
                for (i, (name, hex)) in ACCENTS.iter().enumerate() {
                    let c = pos2(r.left() + 28.0 + i as f32 * 34.0, r.center().y);
                    let resp = ui.interact(Rect::from_center_size(c, vec2(28.0, 28.0)), ui.id().with(("sacc", *name)), Sense::click());
                    let col = if *hex == 0 {
                        if p.dark {
                            Color32::from_rgb(0xec, 0xeb, 0xe6)
                        } else {
                            Color32::from_rgb(0x1c, 0x1b, 0x18)
                        }
                    } else {
                        theme::hex(*hex)
                    };
                    if *name == cur {
                        ui.painter().circle_stroke(c, 14.0, Stroke::new(2.0, col));
                    }
                    let grow = ctx.animate_bool_with_time(resp.id, resp.hovered(), 0.1);
                    ui.painter().circle_filled(c, 11.0 + grow * 1.5, col);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        pick = Some(name.to_string());
                    }
                }
                // vlastná farba
                let cc = pos2(r.left() + 28.0 + ACCENTS.len() as f32 * 34.0, r.center().y);
                let mut col = if cur.starts_with('#') { p.accent } else { Color32::from_rgb(0x8b, 0x7b, 0xff) };
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_center_size(cc, vec2(28.0, 22.0))));
                if egui::color_picker::color_edit_button_srgba(&mut child, &mut col, egui::color_picker::Alpha::Opaque).on_hover_text(t("Custom color")).changed() {
                    pick = Some(format!("#{:02x}{:02x}{:02x}", col.r(), col.g(), col.b()));
                }
                if let Some(a) = pick {
                    self.set("accent", json!(a), ctx);
                }
            }
            "tools" | "plugins" | "ai" => {
                let text = match id {
                    "tools" => t("Flux keeps its installer small. Programming languages are downloaded from their official sources only when you need them."),
                    "plugins" => t("Plugins"),
                    _ => t("Flux can use Claude as a coding assistant (Ctrl+I). You pay Anthropic directly with your own API key – it is stored encrypted on this computer."),
                };
                ui.add_space(6.0);
                ui.add(egui::Label::new(egui::RichText::new(text).font(theme::ui(13.0)).color(p.text2)).wrap());
                ui.add_space(10.0);
                ui.label(egui::RichText::new(t("This part is coming to Flux Native soon – for now use it in Flux.")).font(theme::ui(12.5)).color(p.text3));
            }
            _ => {}
        }
    }

    pub(super) fn settings_jump(&mut self, section: &str) {
        if let Some(st) = self.settings.as_mut() {
            st.jump = Some(section.to_string());
        }
    }

    pub(super) fn settings_find(&mut self, q: &str) {
        if let Some(st) = self.settings.as_mut() {
            st.find = q.to_string();
        }
    }

    pub(super) fn anim_on(&self) -> bool {
        self.get("transitions").as_bool() != Some(false) && self.core.setting("optAnim").as_bool().unwrap_or(self.get("lite").as_bool() != Some(true))
    }
}

// prepínač ako input.switch v Electron Fluxe (36×20, kolieska sa posúva)
fn switch(ui: &mut egui::Ui, on: &mut bool, p: &crate::theme::Pal, anim: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(36.0, 20.0), Sense::click());
    let k = ui.ctx().animate_bool_with_time(resp.id, *on, if anim { 0.14 } else { 0.0 });
    let bg = p.hover.lerp_to_gamma(p.accent, k);
    ui.painter().rect_filled(r, CornerRadius::same(10), if k > 0.0 { bg } else { p.active });
    ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
    let x = egui::lerp(r.left() + 10.0..=r.right() - 10.0, k);
    ui.painter().circle_filled(pos2(x, r.center().y), 7.0, if k > 0.5 { p.accent_fg } else { p.text2 });
    let clicked = resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    if clicked {
        *on = !*on;
    }
    clicked
}
