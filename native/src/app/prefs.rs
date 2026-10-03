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
        "minimap" | "liveWallpaper" | "autosave" | "clearOnRun" | "showSearch" | "transitions" | "inertia" | "trimMemory" | "bracketColors" | "autoUpdate" => json!(true),
        "wordWrap" | "autoReload" | "lite" | "searchWide" => json!(false),
        "liveWallMode" => json!("play"),
        "uiZoom" | "scrollSpeed" => json!(100),
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

const TABS: [(&str, &str, &str); 9] = [
    ("general", "settings", "General"),
    ("appearance", "palette", "Appearance"),
    ("editor", "code", "Editor"),
    ("running", "play", "Running"),
    ("tools", "download", "Languages"),
    ("plugins", "puzzle", "Plugins"),
    ("ai", "sparkle", "AI"),
    ("github", "github", "GitHub"),
    ("developer", "flask", "Developer"), // len vývojár (App::developer)
];

#[derive(Clone)]
enum Row {
    Toggle(&'static str, &'static str, &'static str),
    Select(&'static str, &'static str, &'static str, Vec<(Value, String)>),
    Range(&'static str, &'static str, &'static str, f64, f64, f64),
    Number(&'static str, &'static str, &'static str, f64, f64),
    Text(&'static str, &'static str, &'static str, &'static str),
    Color(&'static str, &'static str, &'static str),
    Button(&'static str, &'static str, &'static str, &'static str), // (akcia, názov, popis, tlačidlo)
    Info(&'static str, String),
    Lead(&'static str), // úvodný odsek sekcie (.s-lead)
    Custom(&'static str),
}

pub struct SettingsUi {
    pub tab: String,
    find: String,
    jump: Option<String>,
    pub opened: f64,
    notes_open: std::collections::HashSet<String>, // rozbalené verzie v poznámkach k vydaniam
    notes_all: bool,                               // ukázať aj staršie verzie
    spy: Option<String>,                           // časť, ktorá je práve navrchu (zvýraznená v ponuke)
    spy_hold: f64,                                 // po kliknutí na podpoložku chvíľu nemeniť zvýraznenie
}

impl SettingsUi {
    pub fn new(tab: &str, now: f64) -> Self {
        SettingsUi { tab: tab.to_string(), find: String::new(), jump: None, opened: now, notes_open: Default::default(), notes_all: false, spy: None, spy_hold: 0.0 }
    }
}

// native/CHANGELOG.md rozdelený na verzie (raz za beh)
pub struct Note {
    pub ver: String,
    pub date: String,
    pub summary: String,
    pub lines: Vec<&'static str>,
}

pub fn release_notes() -> &'static [Note] {
    static NOTES: std::sync::OnceLock<Vec<Note>> = std::sync::OnceLock::new();
    NOTES.get_or_init(|| {
        let mut out: Vec<Note> = vec![];
        for line in include_str!("../../CHANGELOG.md").lines() {
            if let Some(h) = line.strip_prefix("## ") {
                let (ver, date) = h.split_once(" – ").map(|(a, b)| (a.trim().to_string(), b.trim().to_string())).unwrap_or((h.trim().to_string(), String::new()));
                out.push(Note { ver, date, summary: String::new(), lines: vec![] });
            } else if let Some(n) = out.last_mut() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Some(h) = line.strip_prefix("### ") {
                    n.summary = if n.summary.is_empty() { h.to_string() } else { format!("{} \u{00B7} {h}", n.summary) };
                }
                n.lines.push(line);
            }
        }
        out
    })
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
                    Row::Toggle("optFx", "Transparency and blur", "see-through panels and your blurred wallpaper"),
                    Row::Toggle("optAnim", "Animations", "windows, menus and cards glide in and out"),
                    Row::Button("opt-reset", "Reset advanced", "every part follows “Save memory” again", "Reset"),
                ],
            ),
            ("Language", vec![Row::Custom("language")]),
            ("Welcome", vec![Row::Button("intro", "Intro", "Replay the first-start intro.", "Intro"), Row::Button("tour", "Feature tour", "A short walk through the main parts of Flux.", "Tour")]),
            ("Shortcuts", vec![Row::Custom("keys")]),
        ],
        // vlastná veľká karta len pre GitHub účet vývojára (devAllowed v updater.js)
        "developer" if app.developer() => {
            let sha = crate::update::SHA;
            vec![
                (
                    "Build & updates",
                    vec![
                        Row::Info("Build", format!("{} \u{00B7} {}", env!("CARGO_PKG_VERSION"), &sha[..sha.len().min(7)])),
                        Row::Button("dev-check", "Check for updates", "Ask the ci-native branch for a newer build now.", "Check now"),
                        Row::Button("dev-reinstall", "Reinstall the latest build", "Download the newest build from ci-native even if you already have it.", "Reinstall"),
                    ],
                ),
                (
                    "Intro and tour",
                    vec![Row::Button("intro", "Intro", "Replay the first-start intro.", "Intro"), Row::Button("tour", "Feature tour", "A short walk through the main parts of Flux.", "Tour")],
                ),
                (
                    "Memory",
                    vec![
                        Row::Info("Memory", format!("{} MB", crate::mem::used_mb().map(|m| format!("{m:.0}")).unwrap_or("–".into()))),
                        // rozpis: textúry egui (písmo, tapeta, ikony) a čo ešte beží
                        Row::Info("Textures", app.tex_info.clone()),
                        Row::Info(
                            "Running",
                            [
                                app.wall.describe(),
                                if crate::TRANSPARENT.load(std::sync::atomic::Ordering::Relaxed) { "see-through window".into() } else { String::new() },
                                if app.preview.is_some() { "Live Server preview".into() } else { String::new() },
                                if app.running { "program".into() } else { String::new() },
                            ]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" · "),
                        ),
                        Row::Button("dev-trim", "Free memory now", "Move unused memory out of RAM, like when Flux is in the background.", "Trim"),
                        Row::Toggle("devFps", "Show frames per second", "in the status bar – when nothing moves it should drop to 0"),
                    ],
                ),
                (
                    "Folders",
                    vec![
                        Row::Button("dev-data", "Settings folder", "settings.json, translations and backgrounds", "Open"),
                        Row::Button("dev-exe", "Program folder", "where Flux-Native.exe is", "Open"),
                    ],
                ),
            ]
        }
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
                    Row::Range("cardAlpha", "Panel transparency", "how much the background shines through the panels", 30.0, 100.0, 1.0),
                    Row::Button("colors-reset", "Reset all colors", "back to the colors of your theme", "Reset"),
                ],
            ),
            (
                "Text & fonts",
                vec![
                    Row::Select("lineNumbers", "Line numbers", "numbers at the left edge of the code", o(&[("on", "on"), ("relative", "relative"), ("off", "off")])),
                    Row::Toggle("bracketColors", "Colored brackets", "matching brackets get the same color"),
                ],
            ),
            (
                "Window",
                vec![
                    Row::Select(
                        "material",
                        "Window translucency",
                        "“Wallpaper” stays translucent even when the window is not active. With Acrylic/Mica, Windows turns the window grey when inactive.",
                        o(&[("wallpaper", "Wallpaper (recommended)"), ("none", "Off")]),
                    ),
                    Row::Button("bg-pick", "Background", "your own picture instead of the Windows wallpaper", "Image…"),
                    Row::Toggle("liveWallpaper", "Use Lively Wallpaper and Wallpaper Engine", "when one of them is running, Flux shows the same wallpaper behind its panels"),
                    Row::Select(
                        "liveWallMode",
                        "Live wallpaper",
                        "Flux plays your live wallpaper behind its panels. Still image uses less memory.",
                        o(&[("play", "Moving (like the desktop)"), ("still", "Still image (less memory)")]),
                    ),
                    Row::Range("scrollSpeed", "Scroll distance", "how far one turn of the mouse wheel scrolls", 50.0, 300.0, 10.0),
                    Row::Toggle("inertia", "Smooth scrolling with inertia", "the editor, settings, lists and panels keep gliding a bit after you stop the wheel"),
                    Row::Toggle("transitions", "Transition animations", "a soft fade when you switch files, settings pages and screens"),
                    Row::Toggle("showSearch", "Search button", "a magnifier at the top that finds files, commands and settings"),
                    Row::Toggle("searchWide", "Wide search field", "a search box at the top instead of just the magnifier"),
                    Row::Select("panelPos", "Panel position", "where output and the terminal are", o(&[("bottom", "Bottom"), ("right", "Right"), ("left", "Left")])),
                    Row::Select("sidePos", "Sidebar position", "projects and files", o(&[("left", "Left"), ("right", "Right")])),
                    Row::Range("uiZoom", "Size of everything", "makes text, buttons and panels bigger or smaller", 80.0, 140.0, 5.0),
                    Row::Select("density", "Density", "Compact fits more files, tabs and lines on the screen.", o(&[("comfortable", "comfortable"), ("compact", "compact")])),
                    Row::Range("cornerRadius", "Rounded corners", "how round the corners of panels and buttons are", 0.0, 26.0, 1.0),
                    Row::Range("darkLift", "Brightness of dark areas", "Only backgrounds get lighter – text and outlines stay the same. Turn it up if your wallpaper is very dark.", 0.0, 100.0, 1.0),
                ],
            ),
            (
                "",
                // priehľadnosť okna sa dá zmeniť len novým štartom – ponúknuť ho, keď sa líši od želania
                if crate::want_transparent() != crate::TRANSPARENT.load(std::sync::atomic::Ordering::Relaxed) {
                    vec![
                        Row::Button("restart", "Restart Flux", "to switch the live wallpaper mode", "Restart"),
                        Row::Button("look-reset", "Reset the look", "cursor, pointer, fonts, size, corners and background go back to default – your theme and colors stay", "Reset all"),
                    ]
                } else {
                    vec![Row::Button("look-reset", "Reset the look", "cursor, pointer, fonts, size, corners and background go back to default – your theme and colors stay", "Reset all")]
                },
            ),
        ],
        "editor" => vec![
            (
                "Text",
                vec![
                    Row::Select("fontFamily", "Font", "the typeface of your code", FONTS.iter().map(|(id, l)| (json!(id), t(l))).collect()),
                    Row::Number("fontSize", "Font size", "size of the code text in points", 9.0, 32.0),
                    Row::Select("lineHeight", "Line height", "space between lines of code", vec![(json!(1.3), t("compact")), (json!(1.45), t("normal")), (json!(1.6), t("relaxed")), (json!(1.8), t("large"))]),
                    Row::Toggle("wordWrap", "Wrap long lines", "long lines continue on the next line instead of scrolling sideways"),
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
                ("Python", vec![Row::Info("Interpreter", py), Row::Button("python", "Choose another Python", "pick python.exe yourself if Flux found the wrong one", "Change…")]),
                ("Output", vec![Row::Toggle("clearOnRun", "Clear output before running", "each run starts with an empty Output panel"), Row::Number("terminalFontSize", "Output font size", "size of the text in Output and Terminal", 9.0, 28.0)]),
            ]
        }
        "tools" => vec![("", vec![Row::Custom("tools")])],
        "plugins" => {
            vec![("", vec![Row::Lead("Plugins add features to Flux. Install GitHub when you want it – the other parts are built in, community plugins come later.")]), ("Built into Flux", vec![Row::Custom("plugins")])]
        }
        "github" if app.gh_plugin() => vec![
            ("", vec![Row::Lead("Connect your GitHub account to open your repositories as projects and to save (push) your work online.")]),
            ("Account", vec![Row::Custom("gh-account")]),
            ("Git", vec![Row::Custom("gh-git")]),
        ],
        "developer" | "github" => vec![], // nie vývojár / bez pluginu GitHub – karta je skrytá
        _ => vec![
            ("", vec![Row::Lead("Flux can use Claude as a coding assistant (Ctrl+I). You pay Anthropic directly with your own API key – it is stored encrypted on this computer.")]),
            ("API key", vec![Row::Custom("ai-key")]),
            ("MCP connectors", vec![Row::Lead("Connect remote MCP servers (for example GitHub, Linear or your own) – Claude can then use their tools while answering."), Row::Custom("ai-mcp")]),
            (
                "Use Flux from other AI apps",
                vec![
                    Row::Lead("Claude Desktop, Claude Code, Cursor and other AI apps can see and change the project open in Flux – its files, description and to-do list. It runs only on this computer and needs a secret key."),
                    Row::Custom("ai-flux"),
                ],
            ),
        ],
    }
}

// slová riadku na hľadanie: preložené aj anglické (nájde sa „font“ aj „písmo“)
fn row_words(r: &Row) -> String {
    let en = match r {
        Row::Toggle(_, a, b) | Row::Range(_, a, b, ..) | Row::Text(_, a, b, _) | Row::Color(_, a, b) | Row::Select(_, a, b, _) | Row::Number(_, a, b, ..) | Row::Button(_, a, b, _) => {
            format!("{a} {b}")
        }
        Row::Info(a, _) | Row::Lead(a) | Row::Custom(a) => a.to_string(),
    };
    format!("{} {}", row_words_t(r), en.to_lowercase())
}

fn row_words_t(r: &Row) -> String {
    match r {
        Row::Toggle(_, a, b) | Row::Range(_, a, b, ..) | Row::Text(_, a, b, _) | Row::Color(_, a, b) | Row::Select(_, a, b, _) => format!("{} {}", t(a), t(b)),
        Row::Number(_, a, b, ..) => format!("{} {}", t(a), t(b)),
        Row::Button(_, a, b, _) => format!("{} {}", t(a), t(b)),
        Row::Info(a, b) => format!("{} {}", t(a), b),
        Row::Lead(a) => t(a),
        Row::Custom(id) => id.to_string(),
    }
    .to_lowercase()
}

impl App {
    // hodnota nastavenia s predvolenou hodnotou z DEFAULTS
    pub(super) fn get(&self, key: &str) -> Value {
        let v = self.core.setting(key);
        // optX sa bez vlastnej hodnoty riadi „Šetriť pamäť“ (optOn v app.js)
        if v.is_null() && key.starts_with("opt") {
            return json!(self.core.setting("lite").as_bool() != Some(true));
        }
        // „auto“ = tapeta (materialMode v main.js)
        if key == "material" && (v.is_null() || v.as_str() == Some("auto") || v.as_str() == Some("acrylic") || v.as_str() == Some("mica")) {
            return json!("wallpaper");
        }
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
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha((77.0 * k) as u8));
        // .s-card: min(1000px, 94vw) × min(740px, 90vh), v strede
        let w = (full.width() * 0.94).min(1000.0);
        let h = (full.height() * 0.9).min(740.0);
        let card = Rect::from_center_size(full.center(), vec2(w, h) * (0.97 + 0.03 * k));
        if bg.clicked() && !card.contains(bg.interact_pointer_pos().unwrap_or_default()) {
            self.settings = None;
            return;
        }
        let alpha = k;
        ui.painter().add(egui::Shadow { offset: [0, 30], blur: 90, spread: 0, color: Color32::from_black_alpha((128.0 * alpha) as u8) }.as_shape(card, CornerRadius::same(20)));
        ui.painter().rect_filled(card, CornerRadius::same(20), p.solid.gamma_multiply(0.3 + 0.7 * alpha));
        ui.painter().rect_stroke(card, CornerRadius::same(20), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        // ---- ľavá ponuka ----
        let nav = Rect::from_min_size(card.min, vec2(210.0, card.height()));
        ui.painter().rect_filled(nav, CornerRadius { nw: 20, sw: 20, ne: 0, se: 0 }, p.text.gamma_multiply(0.03));
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
        let dev = self.developer();
        let gh = self.gh_plugin();
        for (id, ic, label) in TABS {
            if (id == "developer" && !dev) || (id == "github" && !gh) {
                continue;
            }
            // vývojárska karta oddelená čiarou
            if id == "developer" {
                ui.painter().hline(nav.left() + 20.0..=nav.right() - 20.0, y + 4.0, Stroke::new(1.0, p.line));
                y += 10.0;
            }
            let r = Rect::from_min_size(pos2(nav.left() + 10.0, y), vec2(nav.width() - 20.0, 36.0));
            let resp = ui.interact(r, ui.id().with(("stab", id)), Sense::click());
            let on = id == cur_tab && finding.is_empty();
            let hov = ctx.animate_bool_with_time(ui.id().with(("stab-h", id)), resp.hovered() || on, if anim { 0.12 } else { 0.0 });
            if hov > 0.0 {
                ui.painter().rect_filled(r, CornerRadius::same(10), if on { p.active } else { p.hover.gamma_multiply(hov) });
            }
            // aktívna karta: ikona vo farbe zvýraznenia (.s-tab.on svg)
            widgets::icon_at(
                ui,
                pos2(r.left() + 20.0, r.center().y),
                16.0,
                ic,
                if on {
                    p.accent
                } else if resp.hovered() {
                    p.text
                } else {
                    p.text2
                },
            );
            ui.painter().text(pos2(r.left() + 38.0, r.center().y), Align2::LEFT_CENTER, t(label), theme::bold(13.0), if on || resp.hovered() { p.text } else { p.text2 });
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                go_tab = Some(id.to_string());
            }
            y += 38.0;
            // podpoložky (h3) pre General a Appearance
            if on && (id == "general" || id == "appearance" || id == "developer") {
                let subs: Vec<&str> = sections(id, self).iter().map(|s| s.0).filter(|s| !s.is_empty()).collect();
                let top = y;
                // časť, ktorá je práve navrchu obsahu (scroll spy) – zvýraznenie kĺže
                let spy = self.settings.as_ref().and_then(|s| s.spy.clone()).unwrap_or_else(|| subs.first().map(|s| s.to_string()).unwrap_or_default());
                ui.painter().vline(nav.left() + 33.0, top..=top + subs.len() as f32 * 26.0, Stroke::new(1.0, p.line));
                if let Some(i) = subs.iter().position(|s| *s == spy) {
                    let ty = ctx.animate_value_with_time(egui::Id::new(("spy-y", id)), top + i as f32 * 26.0, if anim { 0.18 } else { 0.0 });
                    let r = Rect::from_min_size(pos2(nav.left() + 42.0, ty), vec2(nav.width() - 52.0, 26.0));
                    ui.painter().rect_filled(r, CornerRadius::same(7), p.active);
                    ui.painter().vline(nav.left() + 33.0, ty + 5.0..=ty + 21.0, Stroke::new(2.0, p.text));
                }
                for s in subs {
                    let r = Rect::from_min_size(pos2(nav.left() + 42.0, y), vec2(nav.width() - 52.0, 26.0));
                    let resp = ui.interact(r, ui.id().with(("ssub", s)), Sense::click());
                    let cur = s == spy;
                    if resp.hovered() && !cur {
                        ui.painter().rect_filled(r, CornerRadius::same(7), p.hover);
                    }
                    widgets::text(ui, pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, &t(s), theme::ui(12.5), if resp.hovered() || cur { p.text } else { p.text2 }, r.width() - 14.0);
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        go_sub = Some(s.to_string());
                    }
                    y += 26.0;
                }
                y += 6.0;
            }
        }
        ui.painter().text(pos2(nav.left() + 20.0, nav.bottom() - 20.0), Align2::LEFT_CENTER, format!("Flux Native {}", env!("CARGO_PKG_VERSION")), theme::ui(11.0), p.text3);
        if let Some(tb) = go_tab {
            let st = self.settings.as_mut().unwrap();
            st.tab = tb;
            st.spy = None;
            st.find.clear();
            st.opened = ctx.input(|i| i.time) - 0.1;
        }
        if let Some(s) = go_sub {
            let st = self.settings.as_mut().unwrap();
            st.spy = Some(s.clone());
            st.spy_hold = ctx.input(|i| i.time) + 0.6;
            st.jump = Some(s);
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
        let sk = egui::Id::new(("sa-settings", tabs.join(",")));
        let off = self.smooth.begin(&ctx, sk, bui.layer_id(), None);
        let mut sa = egui::ScrollArea::vertical().id_salt(("s-body", tabs.join(","))).auto_shrink(false);
        if let Some(o) = off {
            sa = sa.vertical_scroll_offset(o);
        }
        let mut tops: Vec<(&'static str, f32)> = vec![];
        let sout = sa.show(&mut bui, |ui| {
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
                                ui.add_space(22.0);
                                let (r, _) = ui.allocate_exact_size(vec2(inner, 22.0), Sense::hover());
                                let mut job = egui::text::LayoutJob::default();
                                job.append(&t(title).to_uppercase(), 0.0, egui::TextFormat { font_id: theme::bold(11.0), color: p.text3, extra_letter_spacing: 1.0, ..Default::default() });
                                let g = ui.fonts_mut(|f| f.layout_job(job));
                                ui.painter().galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, p.text3);
                                if jump.as_deref() == Some(title) {
                                    ui.scroll_to_rect(r, Some(egui::Align::TOP));
                                }
                                tops.push((title, r.top()));
                                ui.add_space(4.0);
                            } else {
                                ui.add_space(10.0);
                            }
                            // úvodné odseky (.s-lead) nad skupinou
                            let mut rows = rows;
                            while let Some(Row::Lead(text)) = rows.first().cloned() {
                                rows.remove(0);
                                ui.add(egui::Label::new(egui::RichText::new(t(text)).font(theme::ui(13.0)).color(p.text2)).wrap());
                                ui.add_space(12.0);
                            }
                            if !rows.is_empty() {
                                self.group(ui, rows, inner, &ctx);
                            }
                        });
                    });
                }
            }
            ui.add_space(30.0);
        });
        self.smooth.end(sk, &sout);
        // scroll spy: posledná časť, ktorej nadpis už prešiel pod hlavičku; na konci zoznamu vždy posledná
        if finding.is_empty() && !tops.is_empty() && jump.is_none() {
            let max = (sout.content_size.y - sout.inner_rect.height()).max(0.0);
            let at_end = sout.state.offset.y >= max - 2.0;
            let cur = if at_end { tops.last().map(|x| x.0) } else { tops.iter().rev().find(|(_, y)| *y <= body.top() + 60.0).or(tops.first()).map(|x| x.0) };
            if let (Some(c), Some(st)) = (cur, self.settings.as_mut()) {
                // kliknutá časť ostane zvýraznená, kým sa k nej obsah nedoscroluje
                if st.spy.as_deref() != Some(c) && ctx.input(|i| i.time) > st.spy_hold {
                    st.spy = Some(c.to_string());
                }
            }
        }
    }

    fn group(&mut self, ui: &mut egui::Ui, rows: Vec<Row>, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        let custom_only = rows.len() == 1
            && matches!(rows[0], Row::Custom(id) if matches!(id, "themes" | "accents" | "language" | "keys" | "tools" | "plugins" | "about" | "gh-account" | "gh-git" | "ai-key" | "ai-mcp" | "ai-flux"));
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
            // riadok s odsadením 16 px zľava aj sprava (ovládacie prvky nevyčnievajú z rámika)
            ui.horizontal(|ui| {
                ui.set_min_height(52.0);
                ui.add_space(16.0);
                ui.allocate_ui_with_layout(vec2(w - 32.0, 52.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_max_width(w - 32.0);
                    self.row(ui, row, w - 32.0, ctx);
                });
            });
            let bottom = ui.cursor().top();
            if i + 1 < n {
                ui.painter().hline(ui.min_rect().left() + 16.0..=ui.min_rect().left() + w - 16.0, bottom - 2.0, Stroke::new(1.0, p.line));
            }
            let _ = top;
        }
        let r = Rect::from_min_max(pos2(ui.min_rect().left(), start), pos2(ui.min_rect().left() + w, ui.cursor().top()));
        ui.painter().set(bg, egui::Shape::rect_filled(r, CornerRadius::same(14), p.hover));
        ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
    }

    fn label(&self, ui: &mut egui::Ui, title: &str, hint: &str, w: f32) {
        let p = self.pal;
        ui.vertical(|ui| {
            ui.set_width(w);
            ui.add_space(9.0);
            ui.label(egui::RichText::new(t(title)).font(theme::bold(13.0)).color(p.text));
            if !hint.is_empty() {
                ui.add(egui::Label::new(egui::RichText::new(t(hint)).font(theme::ui(12.0)).color(p.text3)).wrap());
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
                    let labels: Vec<String> = opts.iter().map(|o| o.1.clone()).collect();
                    if let Some(i) = widgets::select(ui, egui::Id::new(("sel", key)), &cur_label, &labels, 200.0, &self.pal) {
                        self.set(key, opts[i].0.clone(), ctx);
                    }
                });
            }
            Row::Range(key, title, hint, min, max, step) => {
                self.label(ui, title, hint, lw);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut v = self.get(key).as_f64().unwrap_or(min);
                    // hodnota vedľa posuvníka (ako #s-zoom-v v Electron Fluxe)
                    let val = if key == "uiZoom" || key == "scrollSpeed" { format!("{v:.0} %") } else { format!("{v:.0}") };
                    let (changed, released) = widgets::slider(ui, egui::Id::new(("rng", key)), &mut v, min, max, step, 180.0, &p);
                    ui.label(egui::RichText::new(val).font(theme::ui(12.0)).color(p.text3));
                    // náhľad počas ťahania; uiZoom až po pustení (inak by sa okno menilo pod myšou)
                    if (changed && key != "uiZoom") || (released && key == "uiZoom") {
                        self.set(key, json!(v), ctx);
                    } else if changed {
                        self.update_settings(|o| {
                            o.insert(key.into(), json!(v));
                        });
                    }
                });
            }
            Row::Number(key, title, hint, min, max) => {
                self.label(ui, title, hint, lw);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // − hodnota + (ako input[type=number])
                    let v = self.get(key).as_f64().unwrap_or(min);
                    let mut nv = v;
                    if widgets::button(ui, None, "+", p.card2, p.text, 30.0, &p).clicked() {
                        nv = (v + 1.0).min(max);
                    }
                    let (r, _) = ui.allocate_exact_size(vec2(44.0, 30.0), Sense::hover());
                    ui.painter().rect_stroke(r, CornerRadius::same(8), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{v:.0}"), theme::bold(13.0), p.text);
                    if widgets::button(ui, None, "−", p.card2, p.text, 30.0, &p).clicked() {
                        nv = (v - 1.0).max(min);
                    }
                    if nv != v {
                        self.set(key, json!(nv), ctx);
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
                    // zobrazená farba = vlastná, inak nepriehľadná farba témy (nie priesvitná nad tapetou)
                    let th = theme::palette_for(p.dark, "mono", 0.0);
                    let cur = custom.as_deref().and_then(|h| u32::from_str_radix(h.trim_start_matches('#'), 16).ok()).map(theme::hex).unwrap_or(match key {
                        "uiText" => th.text,
                        "uiText2" => th.text2,
                        "uiBase" => th.base,
                        "uiCard" => th.card,
                        _ => th.card.lerp_to_gamma(th.text, 0.12),
                    });
                    ui.add_space(6.0);
                    let (r, _) = ui.allocate_exact_size(vec2(112.0, 32.0), Sense::hover());
                    if let Some(c) = widgets::color_field(ui, r, key, cur, false, &p) {
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
            Row::Lead(text) => {
                ui.add(egui::Label::new(egui::RichText::new(t(text)).font(theme::ui(13.0)).color(p.text2)).wrap());
            }
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

    // vývojár = GitHub účet pantr1x prihlásený vo Fluxe (settings.github.user.login, zdieľané s Electron Fluxom);
    // FLUX_DEV=1 na testy
    pub(crate) fn developer(&self) -> bool {
        std::env::var("FLUX_DEV").as_deref() == Ok("1") || self.core.setting("github")["user"]["login"].as_str().map(|l| l.eq_ignore_ascii_case("pantr1x")).unwrap_or(false)
    }

    fn action(&mut self, action: &str, ctx: &egui::Context) {
        match action {
            "tour" => {
                self.settings = None;
                self.tour = Some(0);
            }
            // nový štart (uloží súbory) – napr. pre priehľadné okno živej tapety
            "restart" => {
                for i in 0..self.tabs.len() {
                    if self.tabs[i].dirty() {
                        self.save(i);
                    }
                }
                if let Ok(e) = std::env::current_exe() {
                    if std::process::Command::new(e).args(std::env::args_os().skip(1)).spawn().is_ok() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
            "dev-data" => flux_core::settings::open_external(&flux_core::settings::user_data().to_string_lossy()),
            "dev-exe" => {
                if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
                    flux_core::settings::open_external(&d.to_string_lossy());
                }
            }
            "dev-check" => self.upd.check(ctx, ctx.input(|i| i.time), false),
            "dev-reinstall" => self.upd.reinstall(ctx),
            "dev-trim" => crate::mem::trim_now(),
            "opt-reset" => {
                for k in ["optPyAc", "optFx", "optAnim", "optEditorFx", "optJsLimit", "pyMemory", "lspIdle"] {
                    self.update_settings(|o| {
                        o.remove(k);
                    });
                }
                self.apply_look(ctx);
            }
            // vlastný obrázok na pozadí → userData/backgrounds (settings.bg ako v Electron Fluxe)
            "bg-pick" => {
                if let Some(f) = rfd::FileDialog::new().set_title(t("Background")).add_filter(t("Images"), &["png", "jpg", "jpeg", "webp", "bmp", "gif"]).pick_file() {
                    let dir = flux_core::settings::user_data().join("backgrounds");
                    let _ = std::fs::create_dir_all(&dir);
                    let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "bg.png".into());
                    let file = format!("{}-{name}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
                    if std::fs::copy(&f, dir.join(&file)).is_ok() {
                        self.set("bg", json!({ "type": "image", "file": file, "name": name }), ctx);
                    }
                }
            }
            "bg-reset" => {
                self.update_settings(|o| {
                    o.remove("bg");
                });
                self.apply_look(ctx);
            }
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
                ui.painter().rect_filled(r, CornerRadius::same(14), p.hover);
                ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
                let lr = Rect::from_min_size(pos2(r.left() + 16.0, r.top() + 17.0), vec2(52.0, 52.0));
                widgets::brand_mark(ui, lr, &p);
                ui.painter().text(pos2(lr.right() + 14.0, r.top() + 32.0), Align2::LEFT_CENTER, "Flux Native", theme::bold(17.0), p.text);
                // stav aktualizácie (updatesUI v Electron Fluxe)
                use crate::update::State as U;
                let st = self.upd.state();
                let note = match &st {
                    _ if !crate::update::enabled() => t("Development build – updates are off"),
                    U::Idle => String::new(),
                    U::Checking => t("Checking for updates…"),
                    U::Latest => t("You have the latest version"),
                    U::Available { version, .. } => tf("Version {v} is available", &[("v", version)]),
                    U::Downloading { got, total } => tf("Downloading… {n} %", &[("n", &(got * 100 / (*total).max(1)).min(100).to_string())]),
                    U::Ready { version } => tf("Version {v} is ready", &[("v", version)]),
                    U::Error(e) => e.clone(),
                };
                let mut line = tf("Version {v}", &[("v", env!("CARGO_PKG_VERSION"))]);
                if !note.is_empty() {
                    line = format!("{line} \u{00B7} {note}");
                }
                let col = match st {
                    U::Error(_) => p.red,
                    U::Available { .. } | U::Ready { .. } => p.green,
                    _ => p.text3,
                };
                widgets::text(ui, pos2(lr.right() + 14.0, r.top() + 55.0), Align2::LEFT_CENTER, &line, theme::ui(12.5), col, r.width() - 420.0);
                if let U::Downloading { got, total } = st {
                    let k = (got as f32 / total.max(1) as f32).clamp(0.0, 1.0);
                    let bar = Rect::from_min_size(pos2(r.left() + 16.0, r.bottom() - 9.0), vec2(r.width() - 32.0, 4.0));
                    ui.painter().rect_filled(bar, CornerRadius::same(2), p.hover);
                    ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * k, 4.0)), CornerRadius::same(2), p.accent);
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                }
                let mut b = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_max(pos2(r.right() - 330.0, r.top() + 27.0), pos2(r.right() - 16.0, r.top() + 60.0)))
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                if widgets::button(&mut b, Some("globe"), &t("All versions"), p.card2, p.text, 30.0, &p).clicked() {
                    flux_core::settings::open_external("https://pantr1x.github.io/Flux/#releases");
                }
                if crate::update::enabled() {
                    match self.upd.state() {
                        U::Available { .. } | U::Error(_) => {
                            let avail = matches!(self.upd.state(), U::Available { .. });
                            let (label, icon) = if avail { (t("Download update"), "download") } else { (t("Check for updates"), "refresh") };
                            if widgets::button(&mut b, Some(icon), &label, p.accent, p.accent_fg, 30.0, &p).clicked() {
                                if avail {
                                    self.upd.download(ctx);
                                } else {
                                    self.upd.check(ctx, ctx.input(|i| i.time), false);
                                }
                            }
                        }
                        U::Ready { .. } => {
                            if widgets::button(&mut b, Some("refresh"), &t("Restart to update"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                                self.restart_to_update(ctx);
                            }
                        }
                        U::Idle | U::Latest => {
                            if widgets::button(&mut b, Some("refresh"), &t("Check for updates"), p.card2, p.text, 30.0, &p).clicked() {
                                self.upd.check(ctx, ctx.input(|i| i.time), false);
                            }
                        }
                        _ => {}
                    }
                }
                // poznámky k vydaniam (native/CHANGELOG.md)
                ui.add_space(14.0);
                let (hr, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
                let mut job = egui::text::LayoutJob::default();
                job.append(&t("Release notes").to_uppercase(), 0.0, egui::TextFormat { font_id: theme::bold(11.0), color: p.text3, extra_letter_spacing: 1.0, ..Default::default() });
                let g = ui.fonts_mut(|f| f.layout_job(job));
                ui.painter().galley(pos2(hr.left(), hr.center().y - g.size().y / 2.0), g, p.text3);
                ui.add_space(6.0);
                let start = ui.cursor().top();
                let bg = ui.painter().add(egui::Shape::Noop);
                ui.add_space(4.0);
                // každá verzia je sklopený riadok (verzia, dátum, súhrn); klik ju rozbalí
                // najnovšia verzia rozbalená, staršie len po kliknutí na „Staršie verzie“
                let notes = release_notes();
                let all = self.settings.as_ref().is_some_and(|s| s.notes_all);
                let n = if all { notes.len() } else { notes.len().min(1) };
                for (vi, note) in notes.iter().enumerate().take(n) {
                    let open = self.settings.as_ref().map(|s| s.notes_open.contains(&note.ver) != (vi == 0)).unwrap_or(vi == 0);
                    let (hr, resp) = ui.allocate_exact_size(vec2(w, 46.0), Sense::click());
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
                    if hk > 0.0 {
                        ui.painter().rect_filled(hr.shrink2(vec2(6.0, 2.0)), CornerRadius::same(9), p.hover.gamma_multiply(hk));
                    }
                    let ok = ctx.animate_bool_with_time(resp.id.with("o"), open, 0.15);
                    // šípka sa otočí pri rozbalení
                    let c = pos2(hr.left() + 22.0, hr.center().y);
                    let rot = ok * std::f32::consts::FRAC_PI_2;
                    let pt = |x: f32, y: f32| c + egui::Vec2::angled(rot) * x + egui::Vec2::angled(rot + std::f32::consts::FRAC_PI_2) * y;
                    ui.painter().line_segment([pt(-2.5, -4.5), pt(2.5, 0.0)], Stroke::new(1.5, p.text3));
                    ui.painter().line_segment([pt(2.5, 0.0), pt(-2.5, 4.5)], Stroke::new(1.5, p.text3));
                    let vw = widgets::text_w(ui, &note.ver, theme::bold(14.0));
                    ui.painter().text(pos2(hr.left() + 36.0, hr.center().y), Align2::LEFT_CENTER, &note.ver, theme::bold(14.0), p.text);
                    let mut sx = hr.left() + 36.0 + vw + 14.0;
                    if vi == 0 {
                        let cw = widgets::text_w(ui, &t("Latest"), theme::bold(10.5)) + 14.0;
                        let chip = Rect::from_min_size(pos2(sx - 4.0, hr.center().y - 9.0), vec2(cw, 18.0));
                        ui.painter().rect_filled(chip, CornerRadius::same(9), p.accent);
                        ui.painter().text(chip.center(), Align2::CENTER_CENTER, t("Latest"), theme::bold(10.5), p.accent_fg);
                        sx += cw + 8.0;
                    }
                    ui.painter().text(pos2(hr.right() - 16.0, hr.center().y), Align2::RIGHT_CENTER, &note.date, theme::ui(12.0), p.text3);
                    widgets::text(ui, pos2(sx, hr.center().y), Align2::LEFT_CENTER, &note.summary, theme::ui(12.5), p.text2, (hr.right() - 110.0 - sx).max(20.0));
                    if resp.clicked() {
                        if let Some(st) = self.settings.as_mut() {
                            if !st.notes_open.remove(&note.ver) {
                                st.notes_open.insert(note.ver.clone());
                            }
                        }
                    }
                    if open {
                        ui.scope(|ui| {
                            ui.set_opacity(ok);
                            for line in &note.lines {
                                let (font, color, indent, text) = if let Some(h) = line.strip_prefix("### ") {
                                    ui.add_space(4.0);
                                    (theme::bold(13.0), p.text2, 36.0, h.to_string())
                                } else if let Some(b) = line.strip_prefix("- ") {
                                    (theme::ui(12.5), p.text2, 52.0, b.to_string())
                                } else {
                                    (theme::ui(12.5), p.text2, 36.0, line.to_string())
                                };
                                // **tučné** časti, *kurzíva* bez hviezdičiek
                                let mut job = egui::text::LayoutJob::default();
                                job.wrap.max_width = w - indent - 16.0;
                                for (i, part) in text.split("**").enumerate() {
                                    let f = if i % 2 == 1 { theme::bold(font.size) } else { font.clone() };
                                    job.append(&part.replace('*', ""), 0.0, egui::TextFormat { font_id: f, color: if i % 2 == 1 { p.text } else { color }, ..Default::default() });
                                }
                                let g = ui.fonts_mut(|f| f.layout_job(job));
                                let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 4.0), Sense::hover());
                                if indent > 40.0 {
                                    ui.painter().circle_filled(pos2(r.left() + 42.0, r.top() + 9.0), 2.0, p.text3);
                                }
                                ui.painter().galley(pos2(r.left() + indent, r.top() + 2.0), g, color);
                            }
                            ui.add_space(10.0);
                        });
                    }
                    if vi + 1 < n {
                        let y = ui.cursor().top();
                        ui.painter().hline(ui.min_rect().left() + 16.0..=ui.min_rect().left() + w - 16.0, y, Stroke::new(1.0, p.line));
                    }
                }
                ui.add_space(4.0);
                let rr = Rect::from_min_max(pos2(ui.min_rect().left(), start), pos2(ui.min_rect().left() + w, ui.cursor().top()));
                let older = notes.len().saturating_sub(1);
                ui.painter().set(bg, egui::Shape::rect_filled(rr, CornerRadius::same(14), p.hover));
                ui.painter().rect_stroke(rr, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
                if older > 0 {
                    ui.add_space(8.0);
                    let label = if all { t("Hide older versions") } else { tf("Show older versions ({n})", &[("n", &older.to_string())]) };
                    if widgets::button(ui, Some(if all { "collapse" } else { "chevron" }), &label, p.card2, p.text, 30.0, &p).clicked() {
                        if let Some(st) = self.settings.as_mut() {
                            st.notes_all = !st.notes_all;
                        }
                    }
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
                    ui.painter().rect_filled(r, CornerRadius::same(10), p.card2.lerp_to_gamma(p.hover, ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12)));
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
                ui.painter().set(bg, egui::Shape::rect_filled(r, CornerRadius::same(14), p.hover));
                ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
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
                    ui.painter().rect_filled(r, CornerRadius::same(12), p.card2.lerp_to_gamma(p.hover, ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12)));
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
                ui.painter().rect_filled(r, CornerRadius::same(14), p.hover);
                ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
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
                let col = if cur.starts_with('#') { p.accent } else { Color32::from_rgb(0x8b, 0x7b, 0xff) };
                if cur.starts_with('#') {
                    ui.painter().circle_stroke(cc, 14.0, Stroke::new(2.0, col));
                }
                if let Some(c) = widgets::color_field(ui, Rect::from_center_size(cc, vec2(28.0, 28.0)), "accent", col, true, &p) {
                    pick = Some(format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()));
                }
                if let Some(a) = pick {
                    self.set("accent", json!(a), ctx);
                }
            }
            "tools" => self.tools_ui(ui, w, ctx, None),
            "gh-account" | "gh-git" => self.github_ui(ui, id, w, ctx),
            "ai-key" | "ai-mcp" | "ai-flux" => self.ai_settings_ui(ui, id, w, ctx),
            // zabudované časti ako karty (createPluginsUI({ builtins }))
            "plugins" => {
                let cards = [
                    ("github", "GitHub", t("Open your repositories as projects and sign in with your GitHub account."), Some("github")),
                    ("sparkle", "Claude AI", t("A coding assistant next to your code (Ctrl+I), with MCP connectors."), Some("ai")),
                    ("globe", "Live Server", t("Your web page next to the code, reloaded every time you save."), None),
                    ("download", "Languages", t("Python, Node.js, Java, C/C++ and more, downloaded only when you need them."), Some("tools")),
                ];
                let cw = (w - 12.0) / 2.0;
                let (area, _) = ui.allocate_exact_size(vec2(w, 2.0 * 96.0 + 12.0), Sense::hover());
                let mut go = None;
                // GitHub sa inštaluje ako plugin (settings.githubPlugin), ostatné sú zabudované
                let gh = self.gh_plugin();
                let mut toggle_gh = None;
                for (i, (ic, name, desc, tab)) in cards.iter().enumerate() {
                    let tab = if i == 0 && !gh { &None } else { tab };
                    let r = Rect::from_min_size(area.min + vec2((i % 2) as f32 * (cw + 12.0), (i / 2) as f32 * 108.0), vec2(cw, 96.0));
                    let resp = ui.interact(r, ui.id().with(("plug", i)), Sense::click());
                    let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered() && tab.is_some(), 0.12);
                    ui.painter().rect_filled(r, CornerRadius::same(14), p.hover.lerp_to_gamma(p.active, hk));
                    ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
                    let ir = Rect::from_min_size(r.min + vec2(14.0, 14.0), vec2(34.0, 34.0));
                    ui.painter().rect_filled(ir, CornerRadius::same(10), p.card2);
                    widgets::icon_at(ui, ir.center(), 17.0, ic, p.text);
                    ui.painter().text(pos2(ir.right() + 12.0, ir.top() + 9.0), Align2::LEFT_CENTER, *name, theme::bold(13.5), p.text);
                    let status = if i > 0 {
                        t("Built into Flux")
                    } else if gh {
                        t("Installed")
                    } else {
                        t("Not installed")
                    };
                    ui.painter().text(pos2(ir.right() + 12.0, ir.top() + 26.0), Align2::LEFT_CENTER, status, theme::ui(11.0), if i > 0 || gh { p.green } else { p.text3 });
                    if i == 0 {
                        let label = if gh { t("Remove") } else { t("Install") };
                        let bw = widgets::text_w(ui, &label, theme::bold(13.0)) + 26.0;
                        let br = Rect::from_min_size(pos2(r.right() - 14.0 - bw, r.top() + 14.0), vec2(bw, 28.0));
                        let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(br));
                        if widgets::button(&mut bu, None, &label, if gh { p.card2 } else { p.accent }, if gh { p.text } else { p.accent_fg }, 28.0, &p).clicked() {
                            toggle_gh = Some(!gh);
                        }
                    }
                    let mut job = egui::text::LayoutJob::single_section(desc.clone(), egui::TextFormat { font_id: theme::ui(12.0), color: p.text3, ..Default::default() });
                    job.wrap.max_width = cw - 28.0;
                    job.wrap.max_rows = 2;
                    let g = ui.fonts_mut(|f| f.layout_job(job));
                    ui.painter().galley(pos2(r.left() + 14.0, ir.bottom() + 8.0), g, p.text3);
                    if let (true, Some(tb)) = (resp.on_hover_cursor(if tab.is_some() { egui::CursorIcon::PointingHand } else { egui::CursorIcon::Default }).clicked(), tab) {
                        go = Some(*tb);
                    }
                }
                if let Some(on) = toggle_gh {
                    self.set("githubPlugin", json!(on), ctx);
                    self.note(if on { t("GitHub plugin installed. Find it in Settings → GitHub.") } else { t("GitHub plugin removed.") });
                    go = None;
                }
                if let Some(tb) = go {
                    if let Some(st) = self.settings.as_mut() {
                        st.tab = tb.to_string();
                        st.spy = None;
                    }
                }
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
pub(super) fn switch(ui: &mut egui::Ui, on: &mut bool, p: &crate::theme::Pal, anim: bool) -> bool {
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
