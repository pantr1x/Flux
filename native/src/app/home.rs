// Domov Fluxu (klik na logo), minimalisticky: logo, pozdrav s dátumom, veľké hľadanie (projekty a nedávne
// súbory, ↑/↓ + Enter), pripnuté a nedávne projekty a tri tiché tlačidlá Nový projekt / Otvoriť priečinok / súbor.
use super::App;
use crate::i18n::t;
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use flux_core::fsops;
use serde_json::Value;
use std::path::Path;

// šablóny z templates.js (bez značky kurzora $0)
const HTML_PAGE: &str = "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>{{title}}</title>\n</head>\n<body>\n    \n</body>\n</html>\n";
const HTML_WITH_ASSETS: &str = "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>{{title}}</title>\n    <link rel=\"stylesheet\" href=\"style.css\">\n</head>\n<body>\n    <main>\n        <h1>{{title}}</h1>\n        <p>Edit this text in index.html.</p>\n        <button id=\"button\">Click me</button>\n    </main>\n\n    <script src=\"script.js\"></script>\n</body>\n</html>\n";
const CSS_BASE: &str = "* {\n    box-sizing: border-box;\n}\n\nbody {\n    margin: 0;\n    min-height: 100vh;\n    display: grid;\n    place-items: center;\n    font-family: system-ui, sans-serif;\n    background: #f4f4f8;\n    color: #1d1d24;\n}\n\nmain {\n    text-align: center;\n}\n\nbutton {\n    padding: 10px 18px;\n    border: 0;\n    border-radius: 10px;\n    background: #6b5cff;\n    color: white;\n    font-size: 16px;\n    cursor: pointer;\n}\n";
const JS_BASE: &str = "const button = document.querySelector('#button');\nlet count = 0;\n\nbutton.addEventListener('click', () => {\n    count++;\n    button.textContent = `Clicked ${count}×`;\n    console.log('click', count);\n});\n";
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
        ui.painter().rect_filled(full, if self.rounded { CornerRadius::same(8) } else { CornerRadius::ZERO }, p.base);
        // pohyblivé žiary ako v úvode (ob-aurora)
        // pohyb len 8 s po otvorení a len keď je okno aktívne – potom stoja a nič sa neprekresľuje (CPU)
        let glide_end = self.start_opened + 8.0;
        let tt = if anim { now.min(glide_end) as f32 } else { 0.0 };
        let moving = anim && now < glide_end && ctx.input(|i| i.focused);
        let (w, h) = (full.width(), full.height());
        let glow_c =
            |a: u8| if p.accent == p.text || p.accent.r() == p.accent.g() { Color32::from_white_alpha(a) } else { Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), a) };
        super::intro::glow(ui, full.left_top() + vec2(w * (0.2 + 0.05 * (tt * 0.11).sin()), h * (0.25 + 0.06 * (tt * 0.09).cos())), w * 0.42, glow_c(if p.dark { 14 } else { 44 }));
        super::intro::glow(ui, full.left_top() + vec2(w * (0.78 + 0.04 * (tt * 0.08).cos()), h * (0.75 + 0.05 * (tt * 0.1).sin())), w * 0.38, glow_c(if p.dark { 10 } else { 34 }));
        if moving {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
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
        let mut menu_for: Option<(egui::Response, String, bool)> = None;
        let tpl: Option<&str> = None;
        let mut act: Option<&str> = None;
        let mut file: Option<String> = None;
        // ---- minimalistická domovská obrazovka: logo, pozdrav, hľadanie, posledné projekty ----
        let shown: Vec<Value> = self.projects.clone();
        let q = self.start_q.trim().to_lowercase();
        let desc = |app: &App, d: &str| app.core.setting("projectMeta")[d]["description"].as_str().unwrap_or("").to_string();
        // zoznam: pripnuté, potom nedávne; pri hľadaní aj nedávne súbory
        enum Entry {
            Proj(Value),
            File(String),
        }
        let mut list: Vec<Entry> = vec![];
        {
            let hit = |app: &App, pr: &Value| q.is_empty() || format!("{} {} {}", pr["name"].as_str().unwrap_or(""), pr["dir"].as_str().unwrap_or(""), desc(app, pr["dir"].as_str().unwrap_or(""))).to_lowercase().contains(&q);
            let mut pinned: Vec<Value> = shown.iter().filter(|pr| pr["pinned"].as_bool() == Some(true) && hit(self, pr)).cloned().collect();
            let mut recent: Vec<Value> = shown.iter().filter(|pr| pr["pinned"].as_bool() != Some(true) && hit(self, pr)).cloned().collect();
            recent.sort_by_key(|pr| pr["recent"].as_i64().filter(|r| *r >= 0).unwrap_or(999));
            pinned.truncate(4);
            recent.truncate(if q.is_empty() { 6 } else { 8 });
            list.extend(pinned.into_iter().chain(recent).map(Entry::Proj));
            if !q.is_empty() {
                for f in self.recent_files.iter().filter(|f| f.to_lowercase().contains(&q)).take(6) {
                    list.push(Entry::File(f.clone()));
                }
            }
        }
        // výber šípkami (pamätá sa v egui, nový dopyt = od začiatku)
        let sel_id = egui::Id::new("start-sel");
        let (mut sel, last_q) = ctx.data(|d| d.get_temp::<(usize, String)>(sel_id)).unwrap_or((0, String::new()));
        if last_q != q {
            sel = 0;
        }
        let (up, down, enter) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                !list.is_empty() && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            )
        });
        if !list.is_empty() {
            if up {
                sel = (sel + list.len() - 1) % list.len();
            }
            if down {
                sel = (sel + 1) % list.len();
            }
            sel = sel.min(list.len() - 1);
        }
        ctx.data_mut(|d| d.insert_temp(sel_id, (sel, q.clone())));
        let mut pick: Option<usize> = enter.then_some(sel);
        let sout = sa.show(&mut bui, |ui| {
            let cw = (ui.available_width() - 48.0).min(620.0);
            let left = ui.max_rect().left() + (ui.available_width() - cw) / 2.0;
            // obsah zvisle v strede, kým sa zmestí
            let est = 300.0 + list.len().max(1) as f32 * 54.0;
            ui.add_space(((body.height() - est) / 2.0).max(28.0) + (1.0 - k) * 16.0);
            // logo + pozdrav + dátum
            let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 130.0), Sense::hover());
            let cx = left + cw / 2.0;
            widgets::brand_mark(ui, Rect::from_center_size(pos2(cx, hr.top() + 22.0), vec2(40.0, 40.0)), &p);
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
            ui.painter().text(pos2(cx, hr.top() + 76.0), Align2::CENTER_CENTER, &greet, theme::bold(28.0), p.text);
            ui.painter().text(pos2(cx, hr.top() + 108.0), Align2::CENTER_CENTER, date_line(&lang), theme::ui(13.0), p.text3);
            ui.add_space(14.0);
            // veľké hľadanie
            let (sr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
            let sr = Rect::from_min_size(pos2(left, sr.top()), vec2(cw, 50.0));
            let qid = egui::Id::new("start-q");
            let foc = ctx.memory(|m| m.has_focus(qid));
            ui.painter().rect_filled(sr, CornerRadius::same(16), p.solid.gamma_multiply(0.85));
            ui.painter().rect_stroke(sr, CornerRadius::same(16), Stroke::new(if foc { 1.5 } else { 1.0 }, if foc { p.accent.gamma_multiply(0.7) } else { p.line_strong }), StrokeKind::Inside);
            widgets::icon_at(ui, pos2(sr.left() + 24.0, sr.center().y), 16.0, "search", p.text3);
            let mut qc = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(sr.left() + 44.0, sr.top() + 14.0), pos2(sr.right() - 16.0, sr.bottom() - 12.0))));
            let te = qc.add(egui::TextEdit::singleline(&mut self.start_q).id(qid).hint_text(t("Open a project or file…")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(15.0)));
            if now - self.start_opened < 0.3 && !te.has_focus() {
                te.request_focus();
            }
            ui.add_space(18.0);
            // zoznam
            if list.is_empty() {
                let (er, _) = ui.allocate_exact_size(vec2(ui.available_width(), 70.0), Sense::hover());
                let msg = if !q.is_empty() {
                    t("Nothing found")
                } else if self.gh_plugin() {
                    t("Start a new project, open a folder or get one from GitHub.")
                } else {
                    t("Start a new project or open a folder.")
                };
                ui.painter().text(pos2(cx, er.center().y), Align2::CENTER_CENTER, msg, theme::ui(13.0), p.text3);
            }
            let ws = self.workspace();
            for (i, e) in list.iter().enumerate() {
                let (rr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
                let r = Rect::from_min_size(pos2(left, rr.top()), vec2(cw, 48.0));
                let key = match e {
                    Entry::Proj(pr) => pr["dir"].as_str().unwrap_or("").to_string(),
                    Entry::File(f) => f.clone(),
                };
                let resp = ui.interact(r, ui.id().with(("hm-row", &key)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(&key);
                let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered() || i == sel && !list.is_empty() && (up || down || !q.is_empty() || sel > 0), 0.12);
                let cur = matches!(e, Entry::Proj(_)) && ws.as_deref() == Some(key.as_str());
                if hk > 0.0 || cur {
                    ui.painter().rect_filled(r, CornerRadius::same(12), if cur { p.active.lerp_to_gamma(p.hover, hk) } else { p.hover.gamma_multiply(hk) });
                }
                let ic = Rect::from_min_size(pos2(r.left() + 14.0, r.center().y - 11.0), vec2(22.0, 22.0));
                let (title, sub, right) = match e {
                    Entry::Proj(pr) => {
                        super::newproj::paint_icon(ui, ic, &self.project_icon(&key), &p);
                        let d = desc(self, &key);
                        (pr["name"].as_str().unwrap_or("").to_string(), if d.is_empty() { short_path(&key) } else { d }, self.project_sub(&key))
                    }
                    Entry::File(f) => {
                        let n = Path::new(f).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        widgets::file_icon(ui, ic, &n);
                        (n, short_path(&Path::new(f).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default()), t("file"))
                    }
                };
                let tx = r.left() + 50.0;
                let rw = widgets::text_w(ui, &right, theme::ui(12.0)).min(cw * 0.32);
                widgets::text(ui, pos2(tx, r.center().y - 9.0), Align2::LEFT_CENTER, &title, theme::bold(14.0), p.text, cw - 80.0 - rw);
                widgets::text(ui, pos2(tx, r.center().y + 10.0), Align2::LEFT_CENTER, &sub, theme::ui(12.0), p.text3, cw - 80.0 - rw);
                widgets::text(ui, pos2(r.right() - 14.0, r.center().y), Align2::RIGHT_CENTER, &right, theme::ui(12.0), p.text3, rw);
                if let Entry::Proj(pr) = e {
                    if pr["pinned"].as_bool() == Some(true) {
                        widgets::icon_at(ui, pos2(r.left() + 40.0, r.top() + 12.0), 9.0, "pin", p.accent);
                    }
                    if resp.secondary_clicked() {
                        menu_for = Some((resp.clone(), key.clone(), pr["pinned"].as_bool() == Some(true)));
                    }
                }
                if resp.clicked() {
                    pick = Some(i);
                }
            }
            ui.add_space(16.0);
            // tiché tlačidlá
            let btns = [("newproject", "plus", t("New project")), ("open", "folderOpen", t("Open folder")), ("openfile", "file", t("Open file"))];
            let ws_ = btns.iter().map(|(_, _, l)| widgets::text_w(ui, l, theme::bold(13.0)) + 52.0).sum::<f32>() + 16.0;
            let (br, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
            let mut x = cx - ws_ / 2.0;
            for (i, (id, ic, label)) in btns.iter().enumerate() {
                let bw = widgets::text_w(ui, label, theme::bold(13.0)) + 44.0;
                let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(x, br.top()), vec2(bw, 34.0))));
                if widgets::button(&mut bu, Some(ic), label, if i == 0 { p.accent } else { p.card2 }, if i == 0 { p.accent_fg } else { p.text }, 34.0, &p).clicked() {
                    act = Some(id);
                }
                x += bw + 8.0;
            }
            // späť do editora
            if self.workspace().is_some() || !self.tabs.is_empty() {
                ui.add_space(14.0);
                ui.vertical_centered(|ui| {
                    if widgets::button(ui, None, &format!("{}  Esc", t("Back to editor")), p.hover, p.text3, 28.0, &p).clicked() {
                        act = Some("back");
                    }
                });
            }
            ui.add_space(40.0);
        });
        if let Some(i) = pick {
            match list.get(i) {
                Some(Entry::Proj(pr)) => go = pr["dir"].as_str().map(String::from),
                Some(Entry::File(f)) => file = Some(f.clone()),
                None => {}
            }
        }
        self.smooth.end(sk, &sout);
        if let Some((r, d, pinned)) = menu_for {
            self.project_menu(&r, &d, pinned);
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
