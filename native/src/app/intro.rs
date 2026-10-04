// Úvod pri prvom spustení – tie isté kroky a vzhľad ako onboarding.js v Electron Fluxe:
// úvod → jazyk → meno → čo chceš programovať → doplnky → vzhľad → hotovo. Ukladá do toho istého settings.json.
use super::App;
use crate::i18n::t;
use crate::theme::{self, ACCENTS};
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use serde_json::{json, Value};

pub const STEPS: [&str; 7] = ["splash", "uilang", "name", "code", "extras", "look", "done"];

// CODE_LANGS z onboarding.js: (id, názov, súbor pre ikonu, „viac“)
const CODE_LANGS: [(&str, &str, &str, bool); 17] = [
    ("python", "Python", "a.py", false),
    ("web", "HTML & CSS", "a.html", false),
    ("js", "JavaScript", "a.js", false),
    ("java", "Java", "a.java", false),
    ("cpp", "C / C++", "a.cpp", false),
    ("csharp", "C#", "a.cs", false),
    ("go", "Go", "a.go", false),
    ("rust", "Rust", "a.rs", true),
    ("ruby", "Ruby", "a.rb", true),
    ("php", "PHP", "a.php", true),
    ("lua", "Lua", "a.lua", true),
    ("zig", "Zig", "a.zig", true),
    ("r", "R", "a.r", true),
    ("julia", "Julia", "a.jl", true),
    ("ts", "TypeScript", "a.ts", true),
    ("perl", "Perl", "a.pl", true),
    ("explore", "Just exploring", "", true),
];

const LOOK_THEMES: [(&str, &str); 6] =
    [("vscode-dark", "VS Code Dark"), ("flux", "Flux"), ("tokyo-night", "Tokyo Night"), ("catppuccin", "Catppuccin Mocha"), ("vscode-light", "VS Code Light"), ("github-light", "GitHub Light")];

const SHORTCUTS: [(&str, &str); 6] = [
    ("Run current file", "F5"),
    ("Search everything", "Ctrl+Shift+A"),
    ("Quick open file…", "Ctrl+P"),
    ("AI assistant", "Ctrl+I"),
    ("Live Server: start / stop", "Alt+L"),
    ("Toggle output panel", "Ctrl+J"),
];

const PREVIEW: &str = "# Guess the number\nimport random\n\ndef play(name: str) -> int:\n    secret = random.randint(1, 10)\n    tries = 0\n    while True:\n        tries += 1\n        if int(input(\"Guess: \")) == secret:\n            print(f\"Well done, {name}!\")\n            return tries";

#[derive(Default)]
pub struct Intro {
    pub step: usize,
    name: String,
    more: bool,
    shown: usize, // krok, ktorý je práve na obrazovke (na animáciu prechodu)
    changed: f64, // kedy sa krok zmenil
    dir: f32,     // smer prechodu: 1 dopredu, -1 späť
}

impl Intro {
    pub fn new(name: &str) -> Self {
        Intro { step: 0, name: name.to_string(), more: false, shown: 0, changed: -1.0, dir: 1.0 }
    }
}

impl App {
    fn intro_set(&mut self, key: &str, v: Value) {
        self.update_settings(|o| {
            o.insert(key.into(), v);
        });
    }

    pub(super) fn intro_ui(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        ui.painter().rect_filled(rect, 0.0, p.base);
        let anim = self.anim_on();
        let now = ctx.input(|i| i.time);
        // pohyblivé pozadie: mäkké žiary vo farbe zvýraznenia pomaly krúžia (ako živé .ob pozadie)
        let tt = if anim { now as f32 } else { 0.0 };
        let tint = |a: u8| if p.accent == p.text || p.accent.r() == p.accent.g() { Color32::from_white_alpha(a) } else { Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), a) };
        let (w, h) = (rect.width(), rect.height());
        glow(ui, rect.left_top() + vec2(w * (0.22 + 0.06 * (tt * 0.13).sin()), h * (0.28 + 0.07 * (tt * 0.11).cos())), w * 0.46, tint(if p.dark { 16 } else { 50 }));
        glow(ui, rect.left_top() + vec2(w * (0.74 + 0.05 * (tt * 0.09).cos()), h * (0.78 + 0.06 * (tt * 0.12).sin())), w * 0.4, tint(if p.dark { 11 } else { 38 }));
        glow(ui, rect.left_top() + vec2(w * (0.55 + 0.08 * (tt * 0.07).sin()), h * (0.12 + 0.05 * (tt * 0.1).sin())), w * 0.28, Color32::from_white_alpha(if p.dark { 7 } else { 30 }));
        // jemná bodková mriežka
        let gap = 28.0;
        let mut y = rect.top() + (tt * 3.0) % gap;
        while y < rect.bottom() {
            let mut x = rect.left() + 10.0;
            while x < rect.right() {
                ui.painter().circle_filled(pos2(x, y), 0.9, if p.dark { Color32::from_white_alpha(10) } else { Color32::from_black_alpha(14) });
                x += gap;
            }
            y += gap;
        }
        if anim {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
        let Some(intro) = self.intro.as_mut() else { return };
        // prechod medzi krokmi: nový krok sa vysunie zboku a zosilnie
        if intro.shown != intro.step || intro.changed < 0.0 {
            intro.dir = if intro.step >= intro.shown { 1.0 } else { -1.0 };
            intro.shown = intro.step;
            intro.changed = now;
        }
        let k = if anim { ((now - intro.changed) / 0.35).clamp(0.0, 1.0) as f32 } else { 1.0 };
        let ease = 1.0 - (1.0 - k).powi(3);
        if k < 1.0 {
            ctx.request_repaint();
        }
        ui.set_opacity(ease);
        let rect = rect.translate(vec2((1.0 - ease) * 36.0 * intro.dir, 0.0));
        let step = STEPS[intro.step];
        let cx = rect.center().x;
        let mut next = false;
        let mut back = false;
        let mut finish = false;
        match step {
            "splash" | "done" => {
                let top = rect.center().y - 140.0;
                logo(ui, Rect::from_center_size(pos2(cx, top + 40.0), vec2(82.0, 82.0)), ease);
                if step == "splash" {
                    ui.painter().text(pos2(cx, top + 122.0), Align2::CENTER_CENTER, "flux", theme::bold(34.0), p.text);
                    ui.painter().text(pos2(cx, top + 159.0), Align2::CENTER_CENTER, t("Code. Run. Create."), theme::ui(14.5), p.text2);
                    let w = widgets::text_w(ui, &t("Get started"), theme::bold(13.0)) + 30.0;
                    next = primary(ui, Rect::from_center_size(pos2(cx, top + 204.0), vec2(w, 32.0)), &t("Get started"), &p);
                } else {
                    ui.painter().text(pos2(cx, top + 136.0), Align2::CENTER_CENTER, t("You're all set!"), theme::bold(28.0), p.text);
                    ui.painter().text(pos2(cx, top + 172.0), Align2::CENTER_CENTER, t("Open a project folder and run your code with one click."), theme::ui(14.0), p.text2);
                    let w = widgets::text_w(ui, &t("Start coding"), theme::bold(13.0)) + 30.0;
                    finish = primary(ui, Rect::from_center_size(pos2(cx, top + 222.0), vec2(w, 32.0)), &t("Start coding"), &p);
                }
            }
            _ => {
                // rozšírený zoznam jazykov – aj keď je už vybraný jazyk z „ďalších“ (to isté ako v intro_code)
                let chosen: Vec<String> = self.core.setting("codeLangs").as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
                let more = intro.more || CODE_LANGS.iter().any(|c| c.3 && c.0 != "explore" && chosen.iter().any(|x| x == c.0));
                let w = if step == "extras" {
                    645.0
                } else if step == "code" && more {
                    600.0
                } else {
                    510.0
                };
                // výška obsahu kroku – mriežka jazykov sa musí zmestiť nad navigáciu (Späť/Pokračovať)
                let (content_h, skip) = match step {
                    "uilang" => (190.0, false),
                    "name" => (100.0, true),
                    "code" => (if more { 4.0 * (84.0 + 9.0) + 26.0 } else { 2.0 * (112.0 + 9.0) + 26.0 }, false),
                    "extras" => (330.0, true),
                    _ => (370.0, true),
                };
                let x0 = cx - w / 2.0;
                let top = rect.center().y - (content_h + 120.0) / 2.0;
                let (title, sub) = match step {
                    "uilang" => ("Choose your language", "Every language comes with Flux – switching is instant."),
                    "name" => ("What should Flux call you?", "Just for the greeting on the home screen. You can skip this or change it later."),
                    "code" => ("What do you want to code?", "Flux will show the right templates and buttons first. Pick as many as you like."),
                    "extras" => ("Set up extras", "All optional – you can skip this and do it later in Settings."),
                    _ => ("Make it yours", "You can change all of this later in Settings."),
                };
                ui.painter().text(pos2(x0, top + 14.0), Align2::LEFT_CENTER, t(title), theme::bold(22.0), p.text);
                widgets::text(ui, pos2(x0, top + 42.0), Align2::LEFT_CENTER, &t(sub), theme::ui(12.0), p.text2, w);
                let body = Rect::from_min_size(pos2(x0, top + 66.0), vec2(w, content_h));
                match step {
                    "uilang" => self.intro_langs(ui, body, &ctx),
                    "name" => {
                        let intro = self.intro.as_mut().unwrap();
                        let r = Rect::from_min_size(body.min, vec2(314.0, 36.0));
                        ui.painter().rect_filled(r, CornerRadius::same(10), p.card2);
                        ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.5, p.text2), StrokeKind::Inside);
                        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(12.0, 9.0))));
                        let te = child
                            .add(egui::TextEdit::singleline(&mut intro.name).hint_text(t("Your name")).frame(egui::Frame::NONE).char_limit(40).font(theme::bold(14.0)).desired_width(f32::INFINITY));
                        if !te.has_focus() && !te.lost_focus() {
                            te.request_focus();
                        }
                        if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            next = true;
                        }
                    }
                    "code" => self.intro_code(ui, body),
                    "extras" => self.intro_extras(ui, body),
                    _ => self.intro_look(ui, body, &ctx),
                }
                // navigácia: Späť · bodky · (Preskočiť) Pokračovať
                let ny = body.bottom() + 24.0;
                let bw = widgets::text_w(ui, &t("Back"), theme::bold(12.5)) + 26.0;
                if ghost(ui, Rect::from_min_size(pos2(x0, ny - 15.0), vec2(bw, 30.0)), &t("Back"), &p) {
                    back = true;
                }
                let n = STEPS.len() - 2;
                let cur = self.intro.as_ref().map(|i| i.step).unwrap_or(1);
                let mut dx = cx - (n as f32 * 9.0 + 10.0) / 2.0;
                for i in 1..=n {
                    let on = i == cur;
                    let dw = if on { 15.0 } else { 4.0 };
                    ui.painter().rect_filled(Rect::from_min_size(pos2(dx, ny - 2.0), vec2(dw, 4.0)), CornerRadius::same(2), if on { p.text } else { p.text3.gamma_multiply(0.6) });
                    dx += dw + 5.0;
                }
                let cw = widgets::text_w(ui, &t("Continue"), theme::bold(13.0)) + 28.0;
                let cr = Rect::from_min_size(pos2(x0 + w - cw, ny - 15.0), vec2(cw, 30.0));
                if primary(ui, cr, &t("Continue"), &p) {
                    next = true;
                }
                if skip {
                    let sw = widgets::text_w(ui, &t("Skip"), theme::bold(12.5)) + 26.0;
                    if ghost(ui, Rect::from_min_size(pos2(cr.left() - sw - 6.0, ny - 15.0), vec2(sw, 30.0)), &t("Skip"), &p) {
                        if step == "name" {
                            self.intro.as_mut().unwrap().name.clear();
                        }
                        next = true;
                    }
                }
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) && matches!(step, "splash" | "uilang" | "code" | "look" | "extras") {
            next = true;
        }
        if next {
            if step == "name" {
                let n = self.intro.as_ref().unwrap().name.trim().to_string();
                self.intro_set("userName", json!(n));
            }
            let intro = self.intro.as_mut().unwrap();
            intro.step = (intro.step + 1).min(STEPS.len() - 1);
        }
        if back {
            let intro = self.intro.as_mut().unwrap();
            intro.step = intro.step.saturating_sub(1);
        }
        if finish {
            self.intro = None;
            self.intro_set("onboarded", json!(true));
        }
    }

    fn intro_langs(&mut self, ui: &mut egui::Ui, body: Rect, ctx: &egui::Context) {
        let p = self.pal;
        let langs: Vec<Value> = serde_json::from_str(crate::i18n::LANGS).unwrap_or_default();
        let cur = self.core.setting("language").as_str().unwrap_or("en").to_string();
        let (cw, ch, gap) = ((body.width() - 16.0) / 3.0, 42.0, 7.0);
        let mut pick = None;
        for (i, l) in langs.iter().enumerate() {
            let code = l["code"].as_str().unwrap_or("en");
            let r = Rect::from_min_size(body.min + vec2((i % 3) as f32 * (cw + 8.0), (i / 3) as f32 * (ch + gap)), vec2(cw, ch));
            let resp = ui.interact(r, ui.id().with(("lang", code)), Sense::click());
            let on = code == cur;
            ui.painter().rect_filled(r, CornerRadius::same(10), p.card2.lerp_to_gamma(p.hover, ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12)));
            ui.painter().rect_stroke(r, CornerRadius::same(10), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
            if let Some(svg) = crate::gen::flag(code) {
                egui::Image::from_bytes(format!("bytes://flag/{code}.svg"), svg.as_bytes()).paint_at(ui, Rect::from_min_size(pos2(r.left() + 10.0, r.center().y - 7.0), vec2(21.0, 14.0)));
            }
            ui.painter().text(pos2(r.left() + 39.0, r.top() + 15.0), Align2::LEFT_CENTER, l["native"].as_str().unwrap_or(code), theme::bold(12.0), p.text);
            ui.painter().text(pos2(r.left() + 39.0, r.top() + 29.0), Align2::LEFT_CENTER, l["name"].as_str().unwrap_or(""), theme::ui(10.0), p.text3);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                pick = Some(code.to_string());
            }
        }
        if let Some(code) = pick {
            crate::i18n::set_language(&code);
            self.intro_set("language", json!(code));
            ctx.request_repaint();
        }
    }

    fn intro_code(&mut self, ui: &mut egui::Ui, body: Rect) {
        let p = self.pal;
        let chosen: Vec<String> = self.core.setting("codeLangs").as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let more = self.intro.as_ref().map(|i| i.more).unwrap_or(false) || CODE_LANGS.iter().any(|c| c.3 && c.0 != "explore" && chosen.iter().any(|x| x == c.0));
        let mut list: Vec<(&str, &str, &str, bool)> = CODE_LANGS.iter().filter(|c| more || !c.3).cloned().collect();
        if !more {
            list.push(("+more", "More languages", "", false));
        }
        // viac jazykov = 5 stĺpcov, aby sa 17 kariet zmestilo do 4 riadkov
        let cols = if more { 5 } else { 4 };
        let (cw, ch) = ((body.width() - (cols - 1) as f32 * 9.0) / cols as f32, if more { 84.0 } else { 112.0 });
        // jazyky nie sú súčasťou Fluxu – stiahnu sa až keď ich treba
        ui.painter().text(
            pos2(body.left(), body.bottom() - 8.0),
            Align2::LEFT_CENTER,
            t("Languages are not part of Flux – they download only when you need them (Settings → Languages)."),
            theme::ui(11.0),
            p.text3,
        );
        // čo už je na počítači (Nastavenia → Jazyky) – odznak „Installed“ hore na karte
        super::tools::refresh(&self.tools, ui.ctx(), false);
        let installed: Vec<&str> = {
            let s = self.tools.lock().unwrap();
            s.status.as_ref().map(|(l, _)| flux_core::toolchains::TOOLCHAINS.iter().zip(l.iter()).filter(|(_, st)| st.installed).map(|(tc, _)| tc.id).collect()).unwrap_or_default()
        };
        let mut toggle = None;
        for (i, (id, label, file, _)) in list.iter().enumerate() {
            let r = Rect::from_min_size(body.min + vec2((i % cols) as f32 * (cw + 9.0), (i / cols) as f32 * (ch + 9.0)), vec2(cw, ch));
            let resp = ui.interact(r, ui.id().with(("code", *id)), Sense::click());
            let on = chosen.iter().any(|x| x == id);
            ui.painter().rect_filled(r, CornerRadius::same(12), p.card2.lerp_to_gamma(p.hover, ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12)));
            ui.painter().rect_stroke(r, CornerRadius::same(12), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
            let ic = Rect::from_center_size(pos2(r.center().x, r.top() + if more { 30.0 } else { 38.0 }), if more { vec2(40.0, 40.0) } else { vec2(44.0, 44.0) });
            ui.painter().rect_filled(ic, CornerRadius::same(10), p.hover);
            if *id == "+more" {
                widgets::icon_at(ui, ic.center(), 22.0, "plus", p.text2);
            } else if file.is_empty() {
                widgets::icon_at(ui, ic.center(), 22.0, "sparkle", p.text2);
            } else {
                widgets::file_icon(ui, Rect::from_center_size(ic.center(), vec2(26.0, 26.0)), file);
            }
            ui.painter().text(pos2(r.center().x, ic.bottom() + 16.0), Align2::CENTER_CENTER, t(label), theme::bold(12.0), p.text);
            if *id == "+more" {
                widgets::text(ui, pos2(r.center().x, ic.bottom() + 33.0), Align2::CENTER_CENTER, "Rust, TypeScript, PHP, Lua\u{2026}", theme::ui(10.5), p.text3, cw - 16.0);
            }
            let tool = match *id {
                "js" | "ts" => "node",
                "web" | "explore" | "+more" => "",
                x => x,
            };
            if !tool.is_empty() && installed.contains(&tool) {
                if more {
                    // málo miesta: len zelená fajka v rohu
                    let c = pos2(r.left() + 15.0, r.top() + 15.0);
                    ui.painter().circle_filled(c, 8.0, p.green.gamma_multiply(0.22));
                    widgets::icon_at(ui, c, 10.0, "check", p.green);
                } else {
                    let txt = t("Installed");
                    let tw = widgets::text_w(ui, &txt, theme::bold(10.5)) + 14.0;
                    let y = ic.bottom() + 34.0;
                    widgets::icon_at(ui, pos2(r.center().x - tw / 2.0 + 5.0, y), 10.0, "check", p.green);
                    ui.painter().text(pos2(r.center().x - tw / 2.0 + 14.0, y), Align2::LEFT_CENTER, txt, theme::bold(10.5), p.green);
                }
            }
            if on {
                let cb = Rect::from_min_size(pos2(r.right() - 24.0, r.top() + 8.0), vec2(16.0, 16.0));
                ui.painter().rect_filled(cb, CornerRadius::same(8), p.text);
                widgets::icon_at(ui, cb.center(), 11.0, "check", p.card);
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                toggle = Some(id.to_string());
            }
        }
        if let Some(id) = toggle {
            if id == "+more" {
                self.intro.as_mut().unwrap().more = true;
            } else {
                let mut c = chosen.clone();
                if let Some(i) = c.iter().position(|x| *x == id) {
                    c.remove(i);
                } else {
                    c.push(id);
                }
                self.intro_set("codeLangs", json!(c));
            }
        }
    }

    fn intro_extras(&mut self, ui: &mut egui::Ui, body: Rect) {
        let p = self.pal;
        // Skratky
        let half = (body.width() - 10.0) / 2.0;
        let sk = Rect::from_min_size(body.min, vec2(half, 180.0));
        section_card(ui, sk, "command", &t("Shortcuts"), &p);
        for (i, (label, key)) in SHORTCUTS.iter().enumerate() {
            let y = sk.top() + 50.0 + i as f32 * 21.0;
            ui.painter().text(pos2(sk.left() + 12.0, y), Align2::LEFT_CENTER, t(label), theme::ui(11.5), p.text);
            let kw = widgets::text_w(ui, key, theme::mono(10.5)) + 14.0;
            let kr = Rect::from_min_size(pos2(sk.right() - 12.0 - kw.max(52.0), y - 8.0), vec2(kw.max(52.0), 16.0));
            ui.painter().rect_stroke(kr, CornerRadius::same(4), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            ui.painter().text(kr.center(), Align2::CENTER_CENTER, *key, theme::mono(10.5), p.text);
        }
        // GitHub je plugin: bez neho sa o GitHube nikde nič neukazuje
        let gh = Rect::from_min_size(pos2(body.left() + half + 10.0, body.top()), vec2(half, 180.0));
        section_card(ui, gh, "github", "GitHub", &p);
        let mut job = egui::text::LayoutJob::single_section(
            t("Open your repositories as projects and save your work online. You can change this later in Settings → Plugins."),
            egui::TextFormat { font_id: theme::ui(11.5), color: p.text2, ..Default::default() },
        );
        job.wrap.max_width = half - 24.0;
        let g = ui.fonts_mut(|f| f.layout_job(job));
        ui.painter().galley(pos2(gh.left() + 12.0, gh.top() + 40.0), g, p.text2);
        let on = self.core.setting("githubPlugin").as_bool() == Some(true);
        for (i, (val, label)) in [(true, "Install GitHub"), (false, "Not now")].iter().enumerate() {
            let r = Rect::from_min_size(pos2(gh.left() + 12.0, gh.top() + 100.0 + i as f32 * 36.0), vec2(half - 24.0, 30.0));
            let resp = ui.interact(r, ui.id().with(("gh", i)), Sense::click());
            let sel = on == *val;
            ui.painter().rect_filled(r, CornerRadius::same(8), p.card2.lerp_to_gamma(p.hover, ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12)));
            ui.painter().rect_stroke(r, CornerRadius::same(8), if sel { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
            let c = pos2(r.left() + 16.0, r.center().y);
            ui.painter().circle_stroke(c, 5.5, Stroke::new(1.2, if sel { p.text } else { p.text3 }));
            if sel {
                ui.painter().circle_filled(c, 2.8, p.text);
            }
            ui.painter().text(pos2(r.left() + 29.0, r.center().y), Align2::LEFT_CENTER, t(label), theme::bold(12.0), p.text);
            if resp.clicked() {
                self.intro_set("githubPlugin", json!(*val));
            }
        }
        // Aktualizácie
        let up = Rect::from_min_size(pos2(body.left(), body.top() + 192.0), vec2(body.width(), 118.0));
        section_card(ui, up, "refresh", &t("Updates"), &p);
        widgets::text(
            ui,
            pos2(up.left() + 12.0, up.top() + 44.0),
            Align2::LEFT_CENTER,
            &t("Should Flux keep itself up to date? You can change this later in Settings → General."),
            theme::ui(11.5),
            p.text2,
            up.width() - 24.0,
        );
        let auto = self.core.setting("autoUpdate").as_bool() != Some(false);
        let ow = (up.width() - 34.0) / 2.0;
        for (i, (val, title, sub)) in
            [(true, "Automatically", "in the background, installed when you close Flux"), (false, "Ask me first", "Flux tells you when a new version is out")].iter().enumerate()
        {
            let r = Rect::from_min_size(pos2(up.left() + 12.0 + i as f32 * (ow + 10.0), up.top() + 60.0), vec2(ow, 44.0));
            let resp = ui.interact(r, ui.id().with(("upd", i)), Sense::click());
            let on = auto == *val;
            ui.painter().rect_filled(r, CornerRadius::same(8), p.card2.lerp_to_gamma(p.hover, ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12)));
            ui.painter().rect_stroke(r, CornerRadius::same(8), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
            let c = pos2(r.left() + 16.0, r.top() + 15.0);
            ui.painter().circle_stroke(c, 5.5, Stroke::new(1.2, if on { p.text } else { p.text3 }));
            if on {
                ui.painter().circle_filled(c, 2.8, p.text);
            }
            ui.painter().text(pos2(r.left() + 29.0, r.top() + 15.0), Align2::LEFT_CENTER, t(title), theme::bold(12.0), p.text);
            widgets::text(ui, pos2(r.left() + 29.0, r.top() + 31.0), Align2::LEFT_CENTER, &t(sub), theme::ui(10.5), p.text3, ow - 40.0);
            if resp.clicked() {
                self.intro_set("autoUpdate", json!(*val));
            }
        }
    }

    fn intro_look(&mut self, ui: &mut egui::Ui, body: Rect, ctx: &egui::Context) {
        let p = self.pal;
        let cur = self.core.setting("codeTheme").as_str().unwrap_or(crate::gen::DEFAULT_THEME).to_string();
        let lw = 246.0;
        let (tw, th) = ((lw - 8.0) / 2.0, 46.0);
        let mut set_theme = None;
        for (i, (id, name)) in LOOK_THEMES.iter().enumerate() {
            let r = Rect::from_min_size(body.min + vec2((i % 2) as f32 * (tw + 8.0), (i / 2) as f32 * (th + 8.0)), vec2(tw, th));
            let resp = ui.interact(r, ui.id().with(("look", *id)), Sense::click());
            let on = *id == cur;
            ui.painter().rect_filled(r, CornerRadius::same(10), p.card2.lerp_to_gamma(p.hover, ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12)));
            ui.painter().rect_stroke(r, CornerRadius::same(10), if on { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
            let (_, c, _) = crate::code::theme_of(id);
            for (k, idx) in [2usize, 7, 4, 5, 6].iter().enumerate() {
                ui.painter().rect_filled(Rect::from_min_size(pos2(r.left() + 9.0 + k as f32 * 7.0, r.top() + 9.0), vec2(5.0, 11.0)), CornerRadius::same(1), theme::hex(c[*idx]));
            }
            ui.painter().text(pos2(r.left() + 9.0, r.top() + 32.0), Align2::LEFT_CENTER, *name, theme::bold(11.5), p.text);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                set_theme = Some(id.to_string());
            }
        }
        // farba zvýraznenia
        let accent = self.core.setting("accent").as_str().unwrap_or("mono").to_string();
        let ay = body.top() + 3.0 * (th + 8.0) + 14.0;
        let mut set_accent = None;
        for (i, (name, hex)) in ACCENTS.iter().take(8).enumerate() {
            let c = pos2(body.left() + 10.0 + i as f32 * 27.0, ay);
            let resp = ui.interact(Rect::from_center_size(c, vec2(22.0, 22.0)), ui.id().with(("acc", *name)), Sense::click());
            let col = if *hex == 0 {
                if p.dark {
                    p.text
                } else {
                    Color32::from_rgb(0x1c, 0x1b, 0x18)
                }
            } else {
                theme::hex(*hex)
            };
            if *name == accent {
                ui.painter().circle_stroke(c, 11.5, Stroke::new(1.5, col));
            }
            ui.painter().circle_filled(c, 8.5, col);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                set_accent = Some(name.to_string());
            }
        }
        // jas tmavých plôch
        let ly = ay + 30.0;
        ui.painter().text(pos2(body.left(), ly), Align2::LEFT_CENTER, t("Brightness of dark areas"), theme::bold(11.5), p.text);
        let g = ui.painter().layout(t("Only backgrounds get lighter – text and outlines stay the same. Turn it up if your wallpaper is very dark."), theme::ui(10.0), p.text3, lw);
        let gh = g.size().y;
        ui.painter().galley(pos2(body.left(), ly + 9.0), g, p.text3);
        let mut lift = self.core.setting("darkLift").as_f64().unwrap_or(0.0) as f32;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(body.left(), ly + 16.0 + gh), vec2(lw, 20.0))));
        child.spacing_mut().slider_width = lw - 10.0;
        let sr = child.add(egui::Slider::new(&mut lift, 0.0..=100.0).show_value(false));
        // ukážka editora v zvolenej téme
        let pv = Rect::from_min_size(pos2(body.left() + lw + 16.0, body.top()), vec2(body.width() - lw - 16.0, 204.0));
        ui.painter().rect_filled(pv, CornerRadius::same(12), p.card);
        ui.painter().rect_stroke(pv, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        for k in 0..3 {
            ui.painter().circle_filled(pos2(pv.left() + 12.0 + k as f32 * 11.0, pv.top() + 14.0), 3.0, p.text3.gamma_multiply(0.5));
        }
        let tab = Rect::from_min_size(pos2(pv.left() + 50.0, pv.top() + 6.0), vec2(72.0, 17.0));
        ui.painter().rect_filled(tab, CornerRadius::same(5), p.hover);
        widgets::file_icon(ui, Rect::from_min_size(pos2(tab.left() + 5.0, tab.top() + 3.0), vec2(11.0, 11.0)), "main.py");
        ui.painter().text(pos2(tab.left() + 20.0, tab.center().y), Align2::LEFT_CENTER, "main.py", theme::ui(10.5), p.text);
        let run = Rect::from_min_size(pos2(pv.right() - 50.0, pv.top() + 6.0), vec2(42.0, 17.0));
        ui.painter().rect_filled(run, CornerRadius::same(5), p.accent);
        ui.painter().text(run.center(), Align2::CENTER_CENTER, format!("\u{25B6} {}", t("Run")), theme::bold(10.0), p.accent_fg);
        let mut job = self.code.highlight(ctx, ui.style(), PREVIEW, "py");
        for s in job.sections.iter_mut() {
            s.format.font_id = theme::mono(10.5);
            s.format.line_height = Some(14.0);
        }
        job.wrap.max_width = f32::INFINITY;
        let g = ui.fonts_mut(|f| f.layout_job(job));
        ui.painter().with_clip_rect(pv.shrink(2.0)).galley(pv.min + vec2(10.0, 36.0), g, p.text);
        if let Some(id) = set_theme {
            self.set_code_theme(&id, ctx);
        }
        if let Some(a) = set_accent {
            self.intro_set("accent", json!(a));
            self.apply_look(ctx);
        }
        if sr.changed() {
            self.intro_set("darkLift", json!(lift.round()));
            self.apply_look(ctx);
        }
        // pomalší počítač = Šetriť pamäť (lite): hneď vidno, ako Flux vyzerá, a koľko pamäte práve berie
        let lite = self.core.setting("lite").as_bool() == Some(true);
        let lr = Rect::from_min_size(pos2(body.left(), ly + 46.0 + gh), vec2(lw, 50.0));
        let resp = ui.interact(lr, ui.id().with("intro-lite"), Sense::click());
        ui.painter().rect_filled(lr, CornerRadius::same(10), p.card2.lerp_to_gamma(p.hover, ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12)));
        ui.painter().rect_stroke(lr, CornerRadius::same(10), if lite { Stroke::new(1.5, p.text) } else { Stroke::new(1.0, p.line) }, StrokeKind::Inside);
        let cb = Rect::from_min_size(pos2(lr.left() + 12.0, lr.center().y - 8.0), vec2(16.0, 16.0));
        if lite {
            ui.painter().rect_filled(cb, CornerRadius::same(4), p.text);
            widgets::icon_at(ui, cb.center(), 11.0, "check", p.card);
        } else {
            ui.painter().rect_stroke(cb, CornerRadius::same(4), Stroke::new(1.2, p.text3), StrokeKind::Inside);
        }
        ui.painter().text(pos2(lr.left() + 38.0, lr.top() + 17.0), Align2::LEFT_CENTER, t("My computer is slower"), theme::bold(12.0), p.text);
        widgets::text(ui, pos2(lr.left() + 38.0, lr.top() + 34.0), Align2::LEFT_CENTER, &t("less memory: no blur, wallpaper or animations"), theme::ui(10.5), p.text3, lw - 46.0);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.intro_set("lite", json!(!lite));
            self.apply_look(ctx);
        }
        // pamäť práve teraz (pod ukážkou)
        if let Some(mb) = crate::mem::used_mb() {
            let my = pv.bottom() + 20.0;
            widgets::icon_at(ui, pos2(pv.left() + 8.0, my), 13.0, "rocket", p.text3);
            let line = crate::i18n::tf("Flux uses {n} MB right now", &[("n", &format!("{mb:.0}"))]);
            ui.painter().text(pos2(pv.left() + 22.0, my), Align2::LEFT_CENTER, line, theme::bold(12.0), p.text);
            let note = if lite { t("Save memory is on – effects and animations are off.") } else { t("Effects and animations are on.") };
            widgets::text(ui, pos2(pv.left() + 22.0, my + 18.0), Align2::LEFT_CENTER, &note, theme::ui(11.0), p.text3, pv.width() - 24.0);
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
    }
}

// veľké logo (LOGO z onboarding.js)
// veľké logo: pri objavení „vyskočí“ a znak </> sa nakreslí zľava doprava (.ob-draw)
fn logo(ui: &egui::Ui, r: Rect, k: f32) {
    let s = 0.86 + 0.14 * k;
    let r = Rect::from_center_size(r.center(), r.size() * s);
    ui.painter().add(egui::Shadow { offset: [0, 10], blur: 30, spread: 0, color: Color32::from_black_alpha(90) }.as_shape(r, CornerRadius::same(22)));
    ui.painter().rect_filled(r, CornerRadius::same(22), Color32::from_rgb(0x11, 0x11, 0x14));
    ui.painter().rect_stroke(r, CornerRadius::same(22), Stroke::new(1.0, Color32::from_white_alpha(36)), StrokeKind::Inside);
    let ir = Rect::from_center_size(r.center(), egui::Vec2::splat(r.width() * 0.62));
    let reveal = Rect::from_min_max(ir.min, pos2(egui::lerp(ir.left()..=ir.right(), k), ir.bottom()));
    // odhalenie cez orezanie maliara
    let painter = ui.painter().with_clip_rect(reveal.expand(1.0));
    if let Some(svg) = crate::gen::line("code") {
        let img = egui::Image::from_bytes("bytes://li/code.svg", svg.as_bytes()).tint(Color32::WHITE);
        if let Some(tex) = img.load_for_size(ui.ctx(), ir.size()).ok().and_then(|p| p.texture_id()) {
            painter.image(tex, ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        } else {
            ui.ctx().request_repaint();
        }
    }
}

fn primary(ui: &mut egui::Ui, r: Rect, label: &str, p: &crate::theme::Pal) -> bool {
    let resp = ui.interact(r, ui.id().with(("primary", label)), Sense::click());
    let fill = if resp.hovered() { p.accent.lerp_to_gamma(Color32::WHITE, 0.08) } else { p.accent };
    ui.painter().rect_filled(r, CornerRadius::same(10), fill);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, theme::bold(13.0), p.accent_fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn ghost(ui: &mut egui::Ui, r: Rect, label: &str, p: &crate::theme::Pal) -> bool {
    let resp = ui.interact(r, ui.id().with(("ghost", label)), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(10), p.hover);
    }
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, theme::bold(12.5), if resp.hovered() { p.text } else { p.text2 });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn section_card(ui: &egui::Ui, r: Rect, icon: &str, title: &str, p: &crate::theme::Pal) {
    ui.painter().rect_filled(r, CornerRadius::same(12), p.card2);
    ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
    widgets::icon_at(ui, pos2(r.left() + 18.0, r.top() + 20.0), 13.0, icon, p.text);
    ui.painter().text(pos2(r.left() + 30.0, r.top() + 20.0), Align2::LEFT_CENTER, title, theme::bold(12.5), p.text);
}

// mäkká kruhová žiara: stred farby → okraj priehľadný (mesh s prechodom farieb)
pub(super) fn glow(ui: &egui::Ui, c: egui::Pos2, r: f32, color: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, color);
    let n = 64;
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(c + vec2(a.cos(), a.sin()) * r, Color32::TRANSPARENT);
    }
    for i in 1..=n as u32 {
        mesh.add_triangle(0, i, i + 1);
    }
    ui.painter().add(mesh);
}
