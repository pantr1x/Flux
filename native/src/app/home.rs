// Domov Fluxu (klik na logo) – to isté ako openStart() v app.js: pozdrav s dátumom, súčty za všetky
// projekty, Nový projekt / Otvoriť priečinok / Otvoriť súbor, pripnuté a nedávne projekty s hľadaním,
// nedávne súbory, „Začni niečo nové“ zo šablón (templates.js) a tip dňa.
use super::{format_time, App};
use crate::i18n::{t, tf};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use flux_core::fsops;
use serde_json::Value;
use std::path::Path;

// START_CHOICES v app.js: (id, názov, popis, ikona)
const CHOICES: [(&str, &str, &str, &str); 7] = [
    ("py", "Python", "script – print, input, maths", "main.py"),
    ("py-tkinter", "Python window", "app with buttons (tkinter)", "window.py"),
    ("py-pygame", "Python game", "move with arrow keys (pygame)", "game.py"),
    ("html", "HTML page", "one page with a skeleton", "index.html"),
    ("web", "Web project", "HTML + CSS + JavaScript", "style.css"),
    ("js", "JavaScript", "script that runs with Node.js", "script.js"),
    ("empty", "Empty file", ".txt, .md, .py… saved anywhere", "file.txt"),
];

const TIPS: [&str; 6] = [
    "Press Ctrl+Shift+A to search everything – commands, settings, files and projects.",
    "Drop a file or a folder onto Flux to open it.",
    "Ctrl+I opens Claude, your coding assistant.",
    "Plugins like Error Lens and Bookmarks are in Settings → Plugins.",
    "Every shortcut can be changed in Settings → Shortcuts.",
    "Open a .md file and press Ctrl+Shift+V to see the preview.",
];

// šablóny z templates.js (bez značky kurzora $0)
const HTML_PAGE: &str = "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n  <title>{{title}}</title>\n</head>\n<body>\n  \n</body>\n</html>\n";
const HTML_WITH_ASSETS: &str = "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n  <title>{{title}}</title>\n  <link rel=\"stylesheet\" href=\"style.css\">\n</head>\n<body>\n  <main>\n    <h1>{{title}}</h1>\n    <p>Edit this text in index.html.</p>\n    <button id=\"button\">Click me</button>\n  </main>\n\n  <script src=\"script.js\"></script>\n</body>\n</html>\n";
const CSS_BASE: &str = "* {\n  box-sizing: border-box;\n}\n\nbody {\n  margin: 0;\n  min-height: 100vh;\n  display: grid;\n  place-items: center;\n  font-family: system-ui, sans-serif;\n  background: #f4f4f8;\n  color: #1d1d24;\n}\n\nmain {\n  text-align: center;\n}\n\nbutton {\n  padding: 10px 18px;\n  border: 0;\n  border-radius: 10px;\n  background: #6b5cff;\n  color: white;\n  font-size: 16px;\n  cursor: pointer;\n}\n";
const JS_BASE: &str = "const button = document.querySelector('#button');\nlet count = 0;\n\nbutton.addEventListener('click', () => {\n  count++;\n  button.textContent = `Clicked ${count}×`;\n  console.log('click', count);\n});\n";
const PY_TK: &str = "import tkinter as tk\n\n\ndef on_click():\n    label.config(text=\"Clicked!\")\n\n\nwindow = tk.Tk()\nwindow.title(\"My app\")\nwindow.geometry(\"360x200\")\n\nlabel = tk.Label(window, text=\"Hello!\", font=(\"Segoe UI\", 16))\nlabel.pack(pady=24)\n\ntk.Button(window, text=\"Click me\", command=on_click).pack()\n\nwindow.mainloop()\n";
const PY_GAME: &str = "import pygame\n\npygame.init()\nscreen = pygame.display.set_mode((640, 480))\npygame.display.set_caption(\"My game\")\nclock = pygame.time.Clock()\n\nx, y = 300, 220\nrunning = True\nwhile running:\n    for event in pygame.event.get():\n        if event.type == pygame.QUIT:\n            running = False\n\n    keys = pygame.key.get_pressed()\n    if keys[pygame.K_LEFT]:\n        x -= 5\n    if keys[pygame.K_RIGHT]:\n        x += 5\n    if keys[pygame.K_UP]:\n        y -= 5\n    if keys[pygame.K_DOWN]:\n        y += 5\n\n    screen.fill((30, 30, 40))\n    pygame.draw.rect(screen, (139, 123, 255), (x, y, 40, 40))\n\n    pygame.display.flip()\n    clock.tick(60)\n\npygame.quit()\n";

// súbory šablóny: (meno, obsah); prvý sa otvorí
pub(super) fn template_files(id: &str) -> Vec<(&'static str, &'static str)> {
    match id {
        "py" => vec![("main.py", "\n")],
        "py-tkinter" => vec![("window.py", PY_TK)],
        "py-pygame" => vec![("game.py", PY_GAME)],
        "html" => vec![("index.html", HTML_PAGE)],
        "web" => vec![("index.html", HTML_WITH_ASSETS), ("style.css", CSS_BASE), ("script.js", JS_BASE)],
        "js" => vec![("script.js", "console.log('Hello!');\n")],
        _ => vec![],
    }
}

// ~\Documents\x a skrátenie dlhých ciest (shortPath v app.js)
fn short_path(p: &str) -> String {
    let home = dirs::home_dir().map(|h| h.to_string_lossy().to_string()).unwrap_or_default();
    let out = if !home.is_empty() && p.starts_with(&home) { format!("~{}", &p[home.len()..]) } else { p.to_string() };
    let sep = if out.contains('\\') { '\\' } else { '/' };
    let parts: Vec<&str> = out.split(['\\', '/']).collect();
    if parts.len() > 4 {
        format!("{0}{3}…{3}{1}{3}{2}", parts[0], parts[parts.len() - 2], parts[parts.len() - 1], sep)
    } else {
        out
    }
}

// dátum ako „utorok 29. septembra“ v jazyku Fluxu
fn date_line(lang: &str) -> String {
    use chrono::Locale;
    let now = chrono::Local::now();
    let (loc, fmt) = match lang {
        "sk" => (Locale::sk_SK, "%A %-d. %B"),
        "de" => (Locale::de_DE, "%A, %-d. %B"),
        "es" => (Locale::es_ES, "%A, %-d de %B"),
        "fr" => (Locale::fr_FR, "%A %-d %B"),
        "it" => (Locale::it_IT, "%A %-d %B"),
        "pl" => (Locale::pl_PL, "%A, %-d %B"),
        "pt" => (Locale::pt_PT, "%A, %-d de %B"),
        "uk" => (Locale::uk_UA, "%A, %-d %B"),
        _ => (Locale::en_GB, "%A, %-d %B"),
    };
    let s = now.format_localized(fmt, loc).to_string();
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

impl App {
    pub(super) fn open_start(&mut self, now: f64) {
        self.start = true;
        self.start_opened = now;
        self.start_q.clear();
        self.reload_projects();
        // štatistiky pre karty a súčty
        for pr in self.projects.clone() {
            if let Some(d) = pr["dir"].as_str() {
                if !self.summaries.contains_key(d) {
                    self.summarize(d);
                }
            }
        }
    }

    fn close_start(&mut self) {
        self.start = false;
    }

    // vytvorí súbory šablóny v otvorenom projekte, inak nový projekt; otvorí hlavný súbor
    fn start_template(&mut self, id: &str, ctx: &egui::Context) {
        if id == "empty" {
            self.close_start();
            self.open_new_file(None);
            return;
        }
        let files = template_files(id);
        let dir = match self.workspace() {
            Some(w) => w,
            None => {
                let base = match id {
                    "html" | "web" => "my-website",
                    "js" => "js-project",
                    _ => "python-project",
                };
                let made = (1..100).find_map(|i| {
                    let n = if i == 1 { base.to_string() } else { format!("{base}-{i}") };
                    fsops::create_project(&n, "").ok().and_then(|v| v.as_str().map(String::from))
                });
                let Some(d) = made else {
                    self.status = t("Could not create the project.");
                    return;
                };
                self.open_folder(&d);
                d
            }
        };
        let title = Path::new(&dir).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut first = None;
        for (name, content) in files {
            // voľné meno: main.py, main-2.py…
            let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
            let path = (1..100).map(|i| Path::new(&dir).join(if i == 1 { name.to_string() } else { format!("{stem}-{i}.{ext}") })).find(|p| !p.exists()).unwrap_or_else(|| Path::new(&dir).join(name));
            let p = path.to_string_lossy().to_string();
            if fsops::write(&p, &content.replace("{{title}}", &title)).is_ok() && first.is_none() {
                first = Some(p);
            }
        }
        self.close_start();
        self.tree.clear();
        if let Some(f) = first {
            self.open_file(&f);
        }
        let _ = ctx;
    }

    pub(super) fn start_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        let anim = self.anim_on();
        let now = ctx.input(|i| i.time);
        ui.painter().rect_filled(full, 0.0, p.base);
        // pohyblivé žiary ako v úvode (ob-aurora)
        let tt = if anim { now as f32 } else { 0.0 };
        let (w, h) = (full.width(), full.height());
        let glow_c =
            |a: u8| if p.accent == p.text || p.accent.r() == p.accent.g() { Color32::from_white_alpha(a) } else { Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), a) };
        super::intro::glow(ui, full.left_top() + vec2(w * (0.2 + 0.05 * (tt * 0.11).sin()), h * (0.25 + 0.06 * (tt * 0.09).cos())), w * 0.42, glow_c(if p.dark { 14 } else { 44 }));
        super::intro::glow(ui, full.left_top() + vec2(w * (0.78 + 0.04 * (tt * 0.08).cos()), h * (0.75 + 0.05 * (tt * 0.1).sin())), w * 0.38, glow_c(if p.dark { 10 } else { 34 }));
        if anim {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        // ---- horná lišta: logo, ☰, ⚙ (okno sa ťahá za prázdne miesto) ----
        let top = Rect::from_min_size(full.min, vec2(full.width(), theme::TOP_H));
        super::chrome::drag_area(ui, top, "start");
        let cy = top.center().y;
        widgets::brand_mark(ui, Rect::from_min_size(pos2(full.left() + 16.0, cy - 10.0), vec2(20.0, 20.0)), &p);
        ui.painter().text(pos2(full.left() + 44.0, cy), Align2::LEFT_CENTER, "flux", theme::bold(15.0), p.text);
        let menu = widgets::icon_button_at(ui, Rect::from_center_size(pos2(full.left() + 98.0, cy), vec2(28.0, 28.0)), "menu", 16.0, &p, true).on_hover_text(t("Menu"));
        self.app_menu(&menu);
        // ← → ako v hornej lište (navButtons v app.js)
        let (back, fwd) = (self.hist_i > 0, self.hist_i + 1 < self.hist.len());
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(full.left() + 132.0, cy), vec2(28.0, 28.0)), "arrowLeft", 16.0, &p, back).on_hover_text(t("Back")).clicked() {
            self.go(true);
        }
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(full.left() + 162.0, cy), vec2(28.0, 28.0)), "arrowRight", 16.0, &p, fwd).on_hover_text(t("Forward")).clicked() {
            self.go(false);
        }
        let set_r = Rect::from_center_size(pos2(full.right() - super::chrome::CONTROLS_W - 24.0, cy), vec2(30.0, 30.0));
        if widgets::icon_button_at(ui, set_r, "settings", 16.0, &p, true).on_hover_text(t("Settings")).clicked() {
            self.open_settings("general", &ctx);
        }
        // ---- obsah (posúva sa) ----
        let body = Rect::from_min_max(pos2(full.left(), top.bottom()), full.max);
        let mut bui = ui.new_child(egui::UiBuilder::new().max_rect(body));
        bui.set_clip_rect(body);
        // vstup: jemné vysunutie + zobrazenie
        let k = if anim { (((now - self.start_opened) / 0.3).clamp(0.0, 1.0)) as f32 } else { 1.0 };
        let k = 1.0 - (1.0 - k).powi(3);
        if k < 1.0 {
            ctx.request_repaint();
        }
        bui.set_opacity(k);
        let sk = egui::Id::new("sa-start");
        let off = self.smooth.begin(&ctx, sk, bui.layer_id(), None);
        let mut sa = egui::ScrollArea::vertical().id_salt("start").auto_shrink(false);
        if let Some(o) = off {
            sa = sa.vertical_scroll_offset(o);
        }
        let mut go: Option<String> = None;
        let mut menu_for: Option<(egui::Response, String, bool, bool)> = None;
        let mut tpl: Option<&str> = None;
        let mut act: Option<&str> = None;
        let mut file: Option<String> = None;
        let sout = sa.show(&mut bui, |ui| {
            let cw = (ui.available_width() - 64.0).min(1060.0);
            let left = ui.max_rect().left() + (ui.available_width() - cw) / 2.0;
            ui.add_space(26.0 + (1.0 - k) * 16.0);
            // ---- hero: dátum, pozdrav, súčty ----
            let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
            let hour = chrono::Timelike::hour(&chrono::Local::now());
            let base = if hour < 5 {
                t("Good night")
            } else if hour < 12 {
                t("Good morning")
            } else if hour < 18 {
                t("Good afternoon")
            } else {
                t("Good evening")
            };
            let name = self.get("userName").as_str().unwrap_or("").trim().to_string();
            let greet = if name.is_empty() { base } else { format!("{base}, {name}") };
            let lang = self.core.setting("language").as_str().unwrap_or("en").to_string();
            ui.painter().text(pos2(left, hr.top() + 10.0), Align2::LEFT_CENTER, date_line(&lang), theme::ui(12.5), p.text3);
            ui.painter().text(pos2(left, hr.top() + 44.0), Align2::LEFT_CENTER, &greet, theme::bold(30.0), p.text);
            ui.painter().text(pos2(left, hr.top() + 78.0), Align2::LEFT_CENTER, t("What do you want to work on?"), theme::ui(14.0), p.text2);
            if !self.projects.is_empty() {
                let secs: u64 = self.core.setting("projectTime").as_object().map(|o| o.values().filter_map(|v| v.as_u64()).sum()).unwrap_or(0);
                let runs: u64 = self.core.setting("activity").as_object().map(|o| o.values().filter_map(|v| v["runs"].as_u64()).sum()).unwrap_or(0);
                let lines: u64 = self.summaries.values().filter_map(|s| s["lines"].as_u64()).sum();
                let tiles = [
                    ("clock", if secs >= 60 { format_time(secs) } else { "0 min".into() }, "coding time"),
                    ("play", runs.to_string(), "runs"),
                    ("code", lines.to_string(), "lines of code"),
                    ("folder", self.projects.len().to_string(), "projects"),
                ];
                let tw = 118.0;
                for (i, (ic, val, label)) in tiles.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(left + cw - (4 - i) as f32 * (tw + 8.0) + 8.0, hr.top() + 18.0), vec2(tw, 70.0));
                    ui.painter().rect_filled(r, CornerRadius::same(14), p.card.gamma_multiply(0.8));
                    ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
                    widgets::icon_at(ui, pos2(r.right() - 18.0, r.top() + 18.0), 13.0, ic, p.text3);
                    ui.painter().text(pos2(r.left() + 14.0, r.top() + 27.0), Align2::LEFT_CENTER, val, theme::bold(20.0), p.text);
                    widgets::text(ui, pos2(r.left() + 14.0, r.top() + 52.0), Align2::LEFT_CENTER, &t(label), theme::ui(11.5), p.text3, tw - 20.0);
                }
            }
            ui.add_space(22.0);
            // ---- akcie ----
            let (ar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 58.0), Sense::hover());
            let mut x = left;
            for (i, (id, ic, title, sub)) in [
                ("newproject", "plus", t("New project"), "Ctrl+Shift+N".to_string()),
                ("open", "folderOpen", t("Open folder"), "Ctrl+O".to_string()),
                ("openfile", "file", t("Open file"), t(".md, .txt, any file")),
            ]
            .into_iter()
            .enumerate()
            {
                let bw = widgets::text_w(ui, &title, theme::bold(13.5)).max(widgets::text_w(ui, &sub, theme::ui(11.5))) + 76.0;
                let r = Rect::from_min_size(pos2(x, ar.top()), vec2(bw, 58.0));
                let resp = ui.interact(r, ui.id().with(("hm-act", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
                let primary = i == 0;
                let (bg, fg, fg2) = if primary {
                    (p.accent.lerp_to_gamma(Color32::WHITE, 0.08 * hk), p.accent_fg, p.accent_fg.gamma_multiply(0.7))
                } else {
                    (p.solid.gamma_multiply(0.6).lerp_to_gamma(p.hover, hk), p.text, p.text3)
                };
                ui.painter().rect_filled(r.translate(vec2(0.0, -2.0 * hk)), CornerRadius::same(14), bg);
                if !primary {
                    ui.painter().rect_stroke(r.translate(vec2(0.0, -2.0 * hk)), CornerRadius::same(14), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                }
                let ib = Rect::from_center_size(pos2(r.left() + 30.0, r.center().y - 2.0 * hk), vec2(34.0, 34.0));
                ui.painter().rect_filled(ib, CornerRadius::same(10), if primary { p.accent_fg.gamma_multiply(0.15) } else { p.hover });
                widgets::icon_at(ui, ib.center(), 17.0, ic, fg);
                ui.painter().text(pos2(r.left() + 58.0, r.center().y - 9.0 - 2.0 * hk), Align2::LEFT_CENTER, &title, theme::bold(13.5), fg);
                ui.painter().text(pos2(r.left() + 58.0, r.center().y + 10.0 - 2.0 * hk), Align2::LEFT_CENTER, &sub, theme::ui(11.5), fg2);
                if resp.clicked() {
                    act = Some(id);
                }
                x += bw + 10.0;
            }
            ui.add_space(22.0);
            // ---- dva stĺpce ----
            let gap = 22.0;
            let lw = ((cw - gap) * 1.55 / 2.55).floor();
            let rw = cw - gap - lw;
            let col_top = ui.cursor().top();
            let bg_l = ui.painter().add(egui::Shape::Noop);
            let bg_r = ui.painter().add(egui::Shape::Noop);
            // ľavý: projekty
            let mut lu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(left + 18.0, col_top + 18.0), vec2(lw - 36.0, 10000.0))));
            let lwi = lw - 36.0;
            {
                let ui = &mut lu;
                let (hr, _) = ui.allocate_exact_size(vec2(lwi, 30.0), Sense::hover());
                ui.painter().text(pos2(hr.left(), hr.center().y), Align2::LEFT_CENTER, t("Projects"), theme::bold(15.0), p.text);
                let shown: Vec<Value> = self.projects.iter().filter(|pr| pr["hidden"].as_bool() != Some(true)).cloned().collect();
                if !shown.is_empty() {
                    // hľadanie projektu
                    let sr = Rect::from_min_size(pos2(hr.right() - 220.0, hr.top()), vec2(220.0, 30.0));
                    ui.painter().rect_filled(sr, CornerRadius::same(9), p.card2);
                    ui.painter().rect_stroke(sr, CornerRadius::same(9), Stroke::new(1.0, p.line), StrokeKind::Inside);
                    widgets::icon_at(ui, pos2(sr.left() + 16.0, sr.center().y), 13.0, "search", p.text3);
                    let mut qc = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(sr.left() + 30.0, sr.top() + 6.0), pos2(sr.right() - 8.0, sr.bottom() - 5.0))));
                    qc.add(egui::TextEdit::singleline(&mut self.start_q).hint_text(t("Find a project…")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(12.5)));
                }
                ui.add_space(10.0);
                let q = self.start_q.trim().to_lowercase();
                let desc = |app: &App, d: &str| app.core.setting("projectMeta")[d]["description"].as_str().unwrap_or("").to_string();
                let hit = |app: &App, pr: &Value| q.is_empty() || format!("{} {}", pr["name"].as_str().unwrap_or(""), desc(app, pr["dir"].as_str().unwrap_or(""))).to_lowercase().contains(&q);
                let ws = self.workspace();
                let pinned: Vec<Value> = shown.iter().filter(|pr| pr["pinned"].as_bool() == Some(true) && hit(self, pr)).cloned().collect();
                let mut recent: Vec<Value> = shown.iter().filter(|pr| pr["pinned"].as_bool() != Some(true) && hit(self, pr)).cloned().collect();
                recent.sort_by_key(|pr| pr["recent"].as_i64().filter(|r| *r >= 0).unwrap_or(999));
                recent.truncate(6);
                if shown.is_empty() {
                    let (er, _) = ui.allocate_exact_size(vec2(lwi, 110.0), Sense::hover());
                    widgets::icon_at(ui, pos2(er.center().x, er.top() + 26.0), 22.0, "folder", p.text3);
                    ui.painter().text(pos2(er.center().x, er.top() + 58.0), Align2::CENTER_CENTER, t("No projects yet"), theme::bold(14.0), p.text);
                    ui.painter().text(pos2(er.center().x, er.top() + 80.0), Align2::CENTER_CENTER, t("Start a new project, open a folder or get one from GitHub."), theme::ui(12.0), p.text3);
                }
                // karta/riadok projektu
                let mut project = |app: &mut App, ui: &mut egui::Ui, r: Rect, pr: &Value, card: bool| {
                    let dir = pr["dir"].as_str().unwrap_or("").to_string();
                    let resp = ui.interact(r, ui.id().with(("hm-p", &dir, card)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(&dir);
                    let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
                    let cur = ws.as_deref() == Some(dir.as_str());
                    if card {
                        ui.painter().rect_filled(r, CornerRadius::same(16), p.card.gamma_multiply(0.7).lerp_to_gamma(p.hover, hk));
                        ui.painter().rect_stroke(r, CornerRadius::same(16), Stroke::new(1.0, if cur { p.accent.gamma_multiply(0.6) } else { p.line_strong }), StrokeKind::Inside);
                    } else if hk > 0.0 || cur {
                        ui.painter().rect_filled(r, CornerRadius::same(10), if cur { p.active } else { p.hover.gamma_multiply(hk) });
                    }
                    let isz = if card { 24.0 } else { 18.0 };
                    let ic = if card {
                        Rect::from_min_size(pos2(r.left() + 16.0, r.top() + 16.0), vec2(isz, isz))
                    } else {
                        Rect::from_min_size(pos2(r.left() + 12.0, r.center().y - isz / 2.0), vec2(isz, isz))
                    };
                    super::newproj::paint_icon(ui, ic, &app.project_icon(&dir), &p);
                    let name = pr["name"].as_str().unwrap_or("");
                    let d = desc(app, &dir);
                    let sub = if d.is_empty() { short_path(&dir) } else { d };
                    let stat = app.project_sub(&dir);
                    if card {
                        let tx = r.left() + 16.0;
                        widgets::text(ui, pos2(tx, r.top() + 56.0), Align2::LEFT_CENTER, name, theme::bold(14.0), p.text, r.width() - 32.0);
                        widgets::text(ui, pos2(tx, r.top() + 76.0), Align2::LEFT_CENTER, &sub, theme::ui(12.0), p.text3, r.width() - 32.0);
                        widgets::text(ui, pos2(tx, r.top() + 96.0), Align2::LEFT_CENTER, &stat, theme::ui(11.5), p.text3, r.width() - 32.0);
                        widgets::icon_at(ui, pos2(r.right() - 20.0, r.top() + 22.0), 12.0, "pin", p.accent);
                    } else {
                        let tx = r.left() + 42.0;
                        let nw = widgets::text_w(ui, name, theme::bold(13.0)).min(r.width() * 0.4);
                        widgets::text(ui, pos2(tx, r.center().y), Align2::LEFT_CENTER, name, theme::bold(13.0), p.text, nw);
                        let sw = widgets::text_w(ui, &stat, theme::ui(11.5)).min(r.width() * 0.35);
                        widgets::text(ui, pos2(tx + nw + 10.0, r.center().y), Align2::LEFT_CENTER, &sub, theme::ui(12.0), p.text3, (r.width() - 42.0 - nw - sw - 30.0).max(10.0));
                        widgets::text(ui, pos2(r.right() - 12.0, r.center().y), Align2::RIGHT_CENTER, &stat, theme::ui(11.5), p.text3, sw);
                    }
                    if resp.secondary_clicked() {
                        menu_for = Some((resp.clone(), dir.clone(), pr["pinned"].as_bool() == Some(true), pr["hidden"].as_bool() == Some(true)));
                    }
                    if resp.clicked() {
                        go = Some(dir);
                    }
                };
                if !pinned.is_empty() {
                    ui.add_space(4.0);
                    let (sr, _) = ui.allocate_exact_size(vec2(lwi, 22.0), Sense::hover());
                    widgets::icon_at(ui, pos2(sr.left() + 5.0, sr.center().y), 11.0, "pin", p.text3);
                    ui.painter().text(pos2(sr.left() + 16.0, sr.center().y), Align2::LEFT_CENTER, t("Pinned").to_uppercase(), theme::bold(11.0), p.text3);
                    ui.add_space(4.0);
                    let per = if lwi > 520.0 { 3 } else { 2 };
                    let cwid = (lwi - (per - 1) as f32 * 10.0) / per as f32;
                    for row in pinned.chunks(per) {
                        let (rr, _) = ui.allocate_exact_size(vec2(lwi, 116.0), Sense::hover());
                        for (i, pr) in row.iter().enumerate() {
                            let r = Rect::from_min_size(pos2(rr.left() + i as f32 * (cwid + 10.0), rr.top()), vec2(cwid, 112.0));
                            project(self, ui, r, pr, true);
                        }
                    }
                }
                if !recent.is_empty() {
                    ui.add_space(10.0);
                    let (sr, _) = ui.allocate_exact_size(vec2(lwi, 22.0), Sense::hover());
                    ui.painter().text(pos2(sr.left(), sr.center().y), Align2::LEFT_CENTER, t("Recent").to_uppercase(), theme::bold(11.0), p.text3);
                    for pr in &recent {
                        let (r, _) = ui.allocate_exact_size(vec2(lwi, 40.0), Sense::hover());
                        project(self, ui, r, pr, false);
                    }
                }
                if !q.is_empty() && pinned.is_empty() && recent.is_empty() && !shown.is_empty() {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(t("Nothing found")).font(theme::ui(12.5)).color(p.text3));
                }
                // nedávne súbory
                let files: Vec<String> = self.recent_files.iter().take(5).cloned().collect();
                if !files.is_empty() {
                    ui.add_space(16.0);
                    let (sr, _) = ui.allocate_exact_size(vec2(lwi, 26.0), Sense::hover());
                    ui.painter().text(pos2(sr.left(), sr.center().y), Align2::LEFT_CENTER, t("Recent files"), theme::bold(15.0), p.text);
                    for f in files {
                        let (r, resp) = ui.allocate_exact_size(vec2(lwi, 38.0), Sense::click());
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
                        if hk > 0.0 {
                            ui.painter().rect_filled(r, CornerRadius::same(10), p.hover.gamma_multiply(hk));
                        }
                        let name = Path::new(&f).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        widgets::file_icon(ui, Rect::from_min_size(pos2(r.left() + 12.0, r.center().y - 9.0), vec2(18.0, 18.0)), &name);
                        let nw = widgets::text_w(ui, &name, theme::bold(13.0)).min(r.width() * 0.45);
                        widgets::text(ui, pos2(r.left() + 42.0, r.center().y), Align2::LEFT_CENTER, &name, theme::bold(13.0), p.text, nw);
                        let parent = Path::new(&f).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
                        widgets::text(ui, pos2(r.left() + 52.0 + nw, r.center().y), Align2::LEFT_CENTER, &short_path(&parent), theme::ui(12.0), p.text3, r.width() - 64.0 - nw);
                        if resp.clicked() {
                            file = Some(f);
                        }
                    }
                }
            }
            let left_h = lu.min_rect().height() + 36.0;
            // pravý: začni niečo nové + tip
            let mut ru = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(left + lw + gap + 18.0, col_top + 18.0), vec2(rw - 36.0, 10000.0))));
            let rwi = rw - 36.0;
            {
                let ui = &mut ru;
                let (hr, _) = ui.allocate_exact_size(vec2(rwi, 22.0), Sense::hover());
                ui.painter().text(pos2(hr.left(), hr.center().y), Align2::LEFT_CENTER, t("Start something new"), theme::bold(15.0), p.text);
                let sub = match self.workspace() {
                    Some(w) => tf("new file in {dir}", &[("dir", &Path::new(&w).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())]),
                    None => t("creates a new project"),
                };
                ui.label(egui::RichText::new(sub).font(theme::ui(12.0)).color(p.text3));
                ui.add_space(8.0);
                // jazyky z úvodu idú prvé (codeLangs)
                let prefs: Vec<String> = self.core.setting("codeLangs").as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
                let lang_of = |id: &str| match id {
                    "py" | "py-tkinter" | "py-pygame" => "python",
                    "html" | "web" => "web",
                    "js" => "js",
                    _ => "",
                };
                let mut choices = CHOICES.to_vec();
                choices.sort_by_key(|c| !prefs.iter().any(|l| l == lang_of(c.0)));
                for (id, title, sub, icon) in choices {
                    let (r, resp) = ui.allocate_exact_size(vec2(rwi, 46.0), Sense::click());
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
                    if hk > 0.0 {
                        ui.painter().rect_filled(r, CornerRadius::same(10), p.hover.gamma_multiply(hk));
                    }
                    widgets::file_icon(ui, Rect::from_min_size(pos2(r.left() + 12.0, r.center().y - 9.0), vec2(18.0, 18.0)), icon);
                    ui.painter().text(pos2(r.left() + 42.0, r.center().y - 8.0), Align2::LEFT_CENTER, t(title), theme::bold(13.0), p.text);
                    widgets::text(ui, pos2(r.left() + 42.0, r.center().y + 10.0), Align2::LEFT_CENTER, &t(sub), theme::ui(11.5), p.text3, r.width() - 50.0);
                    if resp.clicked() {
                        tpl = Some(id);
                    }
                }
                ui.add_space(12.0);
                // tip dňa
                let tip = t(TIPS[chrono::Datelike::day(&chrono::Local::now()) as usize % TIPS.len()]);
                let g = ui.painter().layout(tip, theme::ui(12.0), p.text2, rwi - 58.0);
                let (tr, _) = ui.allocate_exact_size(vec2(rwi, g.size().y + 40.0), Sense::hover());
                ui.painter().rect_filled(tr, CornerRadius::same(12), p.hover);
                widgets::icon_at(ui, pos2(tr.left() + 22.0, tr.top() + 22.0), 14.0, "sparkle", p.accent);
                ui.painter().text(pos2(tr.left() + 42.0, tr.top() + 16.0), Align2::LEFT_CENTER, t("Tip"), theme::bold(12.5), p.text);
                ui.painter().galley(pos2(tr.left() + 42.0, tr.top() + 28.0), g, p.text2);
            }
            let right_h = ru.min_rect().height() + 36.0;
            let hh = left_h.max(right_h);
            for (slot, x, ww) in [(bg_l, left, lw), (bg_r, left + lw + gap, rw)] {
                let r = Rect::from_min_size(pos2(x, col_top), vec2(ww, hh));
                ui.painter().set(slot, egui::epaint::RectShape::new(r, CornerRadius::same(18), p.solid.gamma_multiply(0.45), Stroke::new(1.0, p.line), StrokeKind::Inside));
            }
            ui.allocate_exact_size(vec2(ui.available_width(), hh), Sense::hover());
            // späť do editora
            if self.workspace().is_some() || !self.tabs.is_empty() {
                ui.add_space(18.0);
                ui.vertical_centered(|ui| {
                    if widgets::button(ui, None, &format!("{}  Esc", t("Back to editor")), p.card2, p.text2, 30.0, &p).clicked() {
                        act = Some("back");
                    }
                });
            }
            ui.add_space(40.0);
        });
        self.smooth.end(sk, &sout);
        if let Some((r, d, pinned, hid)) = menu_for {
            self.project_menu(&r, &d, pinned, hid);
        }
        // Esc = späť (ak je kam)
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.settings.is_none() && self.palette.is_none() && (self.workspace().is_some() || !self.tabs.is_empty()) {
            act = Some("back");
        }
        if let Some(d) = go {
            self.close_start();
            if self.workspace().as_deref() == Some(d.as_str()) {
                self.home = true;
            } else {
                self.open_folder(&d);
            }
        }
        if let Some(f) = file {
            self.close_start();
            self.open_file(&f);
        }
        if let Some(id) = tpl {
            self.start_template(id, &ctx);
        }
        match act {
            Some("back") => self.close_start(),
            Some("newproject") => {
                self.close_start();
                self.side_open = true;
                self.open_new_project();
            }
            Some("open") => {
                if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                    self.close_start();
                    self.open_folder(&d.to_string_lossy());
                }
            }
            Some("openfile") => {
                self.close_start();
                self.act("open-file", &ctx);
            }
            _ => {}
        }
    }
}
