// Flux Native – okno v rovnakom rozložení ako Electron Flux: bočný panel (logo, projekty, voľné súbory, strom, Nastavenia),
// horná lišta (späť/dopredu, karty súborov, hľadanie, AI, ▶ Run), zaoblená karta s editorom + minimapou,
// Výstupom/Terminálom a stavovým riadkom; bez otvoreného súboru stránka projektu. Logika je v flux-core.
mod ai;
mod chrome;
mod editing;
mod github;
mod home;
mod intro;
mod menus;
mod newfile;
mod newproj;
mod prefs;
mod preview;
mod tools;
mod tour;

use crate::code::Code;
use crate::i18n::t;
use crate::term::TermView;
use crate::theme::{self, Pal, GAP, SIDE_W, TOP_H};
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
    last_go: Option<Instant>,                           // posledný krok späť/dopredu (tlačidlá myši niekedy pošlú dva)
    tree: HashMap<String, Vec<(String, String, bool)>>, // priečinok → (meno, cesta, je priečinok)
    open_dirs: HashSet<String>,
    projects: Vec<Value>,
    summaries: HashMap<String, Value>,
    recent_files: Vec<String>,
    show_hidden: bool,
    new_project: Option<newproj::NewProj>,
    new_file: Option<newfile::NewFile>, // okno Nový súbor
    new_item: Option<(bool, String)>,   // (priečinok?, meno) pre nový súbor/priečinok v strome
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
    find: Option<editing::Find>,
    find_goto: bool,
    palette: Option<menus::Palette>,
    ask: Option<menus::Ask>,
    renaming: Option<(String, String)>,
    desc_edit: Option<(String, String, bool)>, // (projekt, text, fokus) – úprava popisu na stránke projektu
    new_in: Option<String>,
    settings: Option<prefs::SettingsUi>,
    switched: f64, // čas poslednej zmeny obsahu karty (animácia prechodu)
    nav_dir: f32,
    tree_sel: Option<Rect>,
    unfocused_at: Option<Instant>, // od kedy je okno v pozadí (tichá aktualizácia)     // riadok otvoreného súboru v strome (kĺzavé zvýraznenie)
    live_push: Option<Instant>,    // posledné poslanie neuloženého textu Live Serveru  // späť (-1) / dopredu (+1): obsah sa vysunie z tej strany
    shown: String,
    panel_w: f32,
    trim: crate::mem::Trim,
    smooth: crate::smooth::Smooth,
    wall: crate::wall::Wall,
    live: Arc<std::sync::Mutex<Option<crate::live::Found>>>, // Lively / Wallpaper Engine (zisťuje sa na pozadí)
    live_at: f64,
    tex_info: String, // vývojár: počet a veľkosť textúr (raz za sekundu)
    start_trimmed: bool,
    boot_ok: bool,                                 // záznam štartu „ok“ zapísaný
    live_done: Arc<std::sync::atomic::AtomicBool>, // prvé zisťovanie živej tapety skončilo
    live_saved: Option<crate::live::Found>,        // čo je uložené v settings.liveCache
    wall_at: f64,                                  // kedy sa naposledy zisťoval zdroj tapety
    pub(crate) upd: crate::update::Updater,
    server: Option<crate::server::Server>, // Live Server (HTML)
    ok_status: String,                     // hlásenie o úspechu (zelené); ostatné v status sú chyby
    ok_at: Option<Instant>,                // kedy sa hlásenie o úspechu ukázalo (po 4 s zmizne)
    ai: ai::AiUi,                          // Claude: nastavenia a panel
    gh: github::GhUi,                      // GitHub účet a repozitáre
    mcp: Option<crate::mcp::Server>,       // Flux pre iné AI aplikácie (Nastavenia → AI)
    mcp_err: Option<String>,
    tools: tools::Shared, // programovacie jazyky na stiahnutie
    start: bool,          // domov Fluxu (logo)
    start_q: String,      // hľadanie projektu na domove
    start_opened: f64,
    preview: Option<preview::Preview>,       // Live Server vedľa kódu
    tour: Option<usize>,                     // krok prehliadky funkcií
    tour_rects: HashMap<&'static str, Rect>, // kde sú časti okna (pre prehliadku)
    frames: std::collections::VecDeque<f64>, // časy snímok (vývojárske počítadlo)
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let core = Arc::new(Core::load());
        // naposledy zistená živá tapeta – hneď od štartu, bez prebliknutia tapety Windows
        let lc = core.setting("liveCache");
        let live_cache = lc["file"].as_str().map(std::path::PathBuf::from).filter(|p| p.is_file()).map(|file| crate::live::Found {
            file,
            video: lc["video"].as_bool().unwrap_or(false),
            preview: lc["preview"].as_str().map(std::path::PathBuf::from),
            source: if lc["source"].as_str() == Some("Wallpaper Engine") { "Wallpaper Engine" } else { "Lively Wallpaper" },
        });
        // svetlá/tmavá podľa témy kódu – ako isDark() v Electron Fluxe
        let code = Code::new(core.setting("codeTheme").as_str().unwrap_or(""));
        let pal = theme::palette_for(code.dark, core.setting("accent").as_str().unwrap_or("mono"), core.setting("darkLift").as_f64().unwrap_or(0.0));
        theme::fonts(&cc.egui_ctx, core.setting("fontFamily").as_str().unwrap_or("Consolas"));
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
            last_go: None,
            tree: HashMap::new(),
            open_dirs: HashSet::new(),
            projects: vec![],
            summaries: HashMap::new(),
            recent_files: vec![],
            show_hidden: false,
            new_project: None,
            new_item: None,
            new_file: None,
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
            find: None,
            find_goto: false,
            palette: None,
            ask: None,
            renaming: None,
            desc_edit: None,
            new_in: None,
            settings: None,
            switched: 0.0,
            nav_dir: 0.0,
            tree_sel: None,
            unfocused_at: None,
            live_push: None,
            shown: String::new(),
            panel_w: 420.0,
            trim: Default::default(),
            smooth: Default::default(),
            wall: Default::default(),
            live: Arc::new(std::sync::Mutex::new(live_cache.clone())),
            live_done: Default::default(),
            tex_info: String::new(),
            start_trimmed: false,
            boot_ok: false,
            live_saved: live_cache,
            live_at: -100.0,
            wall_at: -100.0,
            upd: Default::default(),
            server: None,
            ok_status: String::new(),
            ok_at: None,
            ai: Default::default(),
            gh: Default::default(),
            mcp: None,
            mcp_err: None,
            tools: Default::default(),
            start: false,
            start_q: String::new(),
            start_opened: 0.0,
            preview: None,
            tour: None,
            tour_rects: HashMap::new(),
            frames: Default::default(),
        };
        crate::boot::window();
        crate::i18n::set_language(app.core.setting("language").as_str().unwrap_or("en"));
        if app.core.setting("onboarded").as_bool() != Some(true) || std::env::var("FLUX_INTRO").is_ok() {
            app.intro = Some(intro::Intro::new(app.core.setting("userName").as_str().unwrap_or("")));
        }
        app.out.feed(&format!("\x1b[90m{}\x1b[0m\r\n", t("Program output appears here. Press F5 or ▶ Run.")));
        app.gh_plugin_migrate();
        if app.core.setting("glassFailed").as_bool() == Some(true) {
            app.update_settings(|o| {
                o.remove("glassFailed");
            });
            app.status = t("The see-through glass did not work on this computer, so Flux went back to the moving wallpaper.");
        }
        if crate::mcp::enabled(&app.core) {
            app.mcp_toggle(true);
        }
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
        app.apply_look(&cc.egui_ctx);
        // tichá aktualizácia zbaleného okna: nový Flux tiež zbalený
        if std::env::args().any(|a| a == "--minimized") {
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        app
    }

    fn palette(&self) -> Pal {
        let mut p = theme::palette_for(self.code.dark, self.core.setting("accent").as_str().unwrap_or("mono"), self.core.setting("darkLift").as_f64().unwrap_or(0.0));
        // vlastné farby aplikácie (Nastavenia → Vzhľad → Farby aplikácie)
        let c = |k: &str| self.core.setting(k).as_str().filter(|s| s.len() == 7 && s.starts_with('#')).and_then(|s| u32::from_str_radix(&s[1..], 16).ok()).map(theme::hex);
        if let Some(x) = c("uiText") {
            p.text = x;
        }
        if let Some(x) = c("uiText2") {
            p.text2 = x;
            p.text3 = x.gamma_multiply(0.8);
        }
        if let Some(x) = c("uiBase") {
            p.base = x;
        }
        if let Some(x) = c("uiCard") {
            p.card = x;
        }
        if let Some(x) = c("uiLine") {
            p.line = x.gamma_multiply(0.6);
            p.line_strong = x;
        }
        p.solid = p.solid.lerp_to_gamma(p.card, 0.5);
        // tapeta za oknom: pozadie a karta sú priesvitné (--base-alpha, --card v Electron Fluxe)
        if self.wall_on() {
            let alpha = (self.get("cardAlpha").as_f64().unwrap_or(74.0) / 100.0).clamp(0.3, 1.0) as f32;
            // adaptColors: pri svetlej tapete v tmavej téme (a naopak) sú panely menej priesvitné, aby bol text čitateľný
            let lum = self.wall.lum.unwrap_or(0.3);
            let clash = if p.dark { (lum - 0.25).max(0.0) } else { (0.65 - lum).max(0.0) };
            let base_a = if p.dark { 0.62 } else { 0.66 } + (clash * 0.9).min(0.3);
            // priehľadné okno (živá tapeta): pod kartou nie je rozmazaná kópia → karta plnšia
            let card_def = if self.see_through() {
                0.94
            } else if p.dark {
                0.9
            } else {
                0.88
            };
            let card_a = if (alpha - 0.74).abs() > 0.001 { alpha } else { (card_def + clash * 0.2).min(0.96) };
            p.base = p.base.gamma_multiply(base_a);
            p.card = p.card.gamma_multiply(card_a);
        }
        p
    }

    // tapeta za oknom: materiál nie je „none“ a efekty sú zapnuté (optFx)
    fn wall_on(&self) -> bool {
        !crate::boot::safe() && self.get("material").as_str() != Some("none") && self.core.setting("optFx").as_bool().unwrap_or(self.get("lite").as_bool() != Some(true))
    }

    fn wall_source(&self) -> Option<crate::wall::Src> {
        use crate::wall::Src;
        if !self.wall_on() {
            return None;
        }
        let bg = self.core.setting("bg");
        if bg["type"].as_str() == Some("image") {
            if let Some(f) = bg["file"].as_str() {
                let p = settings::user_data().join("backgrounds").join(f);
                if p.exists() {
                    return Some(Src::Image(p));
                }
            }
        }
        // živá tapeta (Lively Wallpaper / Wallpaper Engine), ak beží – bez vlastného pozadia a keď nie je vypnutá
        if self.get("liveWallpaper").as_bool() != Some(false) {
            if let Some(f) = self.live.lock().unwrap().clone() {
                // priehľadné okno: plocha (aj živá tapeta) presvitá sama, Flux nič nekreslí ani nedekóduje
                if self.see_through() {
                    return None;
                }
                let mode = self.get("liveWallMode").as_str().unwrap_or("see").to_string();
                return Some(match (f.video, mode.as_str()) {
                    (true, "play" | "see" | "glass") => Src::Video(f.file, f.preview), // „see“ z 0.7–0.8.3 = pohyblivá
                    (true, _) => Src::Still(f.file, f.preview),                        // jedna snímka, potom sa dekodér uvoľní
                    _ => Src::Image(f.file),
                });
            }
            // prvé zisťovanie ešte beží (bez uloženej živej tapety): zatiaľ nič, nech tapeta Windows nepreblikne
            if !self.live_done.load(std::sync::atomic::Ordering::Relaxed) && self.started.elapsed() < Duration::from_secs(2) {
                return None;
            }
        }
        crate::wall::desktop_wallpaper().map(Src::Image)
    }

    fn glass_ui(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let left = (25.0 - self.started.elapsed().as_secs_f32()).max(0.0).ceil();
        let w = 440.0;
        let card = Rect::from_center_size(full.center(), vec2(w, 150.0));
        ui.painter().add(egui::Shadow { offset: [0, 14], blur: 40, spread: 0, color: Color32::from_black_alpha(110) }.as_shape(card, CornerRadius::same(14)));
        ui.painter().rect_filled(card, CornerRadius::same(14), p.solid);
        ui.painter().rect_stroke(card, CornerRadius::same(14), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        ui.painter().text(card.min + vec2(24.0, 30.0), Align2::LEFT_CENTER, t("Do you see your live wallpaper through Flux?"), theme::bold(14.0), p.text);
        let g = ui.painter().layout(crate::i18n::tf("If you don't answer, Flux goes back to the moving wallpaper in {n} s.", &[("n", &format!("{left:.0}"))]), theme::ui(12.5), p.text3, w - 48.0);
        ui.painter().galley(card.min + vec2(24.0, 48.0), g, p.text3);
        let mut b = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(pos2(card.left() + 20.0, card.bottom() - 52.0), pos2(card.right() - 20.0, card.bottom() - 18.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        b.spacing_mut().item_spacing.x = 8.0;
        let keep = widgets::button(&mut b, Some("check"), &t("Yes, keep it"), p.accent, p.accent_fg, 32.0, &p).clicked();
        let back = widgets::button(&mut b, None, &t("Go back"), p.card2, p.text, 32.0, &p).clicked();
        if keep {
            crate::GLASS_OK.store(true, std::sync::atomic::Ordering::Relaxed);
            self.update_settings(|o| {
                o.remove("glassTrial");
            });
        } else if back {
            crate::GLASS_OK.store(true, std::sync::atomic::Ordering::Relaxed);
            self.update_settings(|o| {
                o.insert("liveWallMode".into(), json!("play"));
                o.remove("glassTrial");
            });
            if let Ok(e) = std::env::current_exe() {
                if std::process::Command::new(e).spawn().is_ok() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }

    // priehľadné okno práve ukazuje živú tapetu (okno vytvorené ako priehľadné + tapeta beží + režim „see“)
    fn see_through(&self) -> bool {
        crate::TRANSPARENT.load(std::sync::atomic::Ordering::Relaxed)
            && self.wall_on()
            && matches!(self.get("liveWallMode").as_str().unwrap_or("see"), "see" | "glass")
            && self.get("liveWallpaper").as_bool() != Some(false)
            && self.live.lock().unwrap().is_some()
    }

    // „Restart to update“: uloží súbory, vymení program a spustí nový
    pub(crate) fn restart_to_update(&mut self, ctx: &egui::Context) {
        for i in 0..self.tabs.len() {
            if self.tabs[i].dirty() {
                self.save(i);
            }
        }
        match self.upd.install(true) {
            Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(e) => self.status = e,
        }
    }

    // zistí živú tapetu na pozadí (tasklist + súbory nastavení), najviac raz za 10 s pri zameranom okne
    fn check_live(&mut self, ctx: &egui::Context, now: f64) {
        if now - self.live_at < 10.0 || !self.wall_on() || self.get("liveWallpaper").as_bool() == Some(false) || self.core.setting("bg")["type"].is_string() {
            return;
        }
        self.live_at = now;
        let slot = self.live.clone();
        let done = self.live_done.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let found = crate::live::detect();
            let mut cur = slot.lock().unwrap();
            if *cur != found {
                *cur = found;
                ctx.request_repaint();
            }
            done.store(true, std::sync::atomic::Ordering::Relaxed);
        });
    }

    // posledná zistená živá tapeta sa pamätá v settings.liveCache – pri ďalšom štarte hneď bez prebliknutia
    // vráti true, keď sa živá tapeta zmenila (treba prepočítať vzhľad)
    fn sync_live_cache(&mut self) -> bool {
        let cur = self.live.lock().unwrap().clone();
        if cur == self.live_saved || !self.live_done.load(std::sync::atomic::Ordering::Relaxed) {
            return false;
        }
        self.live_saved = cur.clone();
        let v = cur.map(|f| json!({ "file": f.file.to_string_lossy(), "video": f.video, "preview": f.preview.map(|p| p.to_string_lossy().to_string()), "source": f.source }));
        self.update_settings(|o| match v {
            Some(v) => {
                o.insert("liveCache".into(), v);
            }
            None => {
                o.remove("liveCache");
            }
        });
        // tapeta sa zmenila → nový zdroj hneď
        self.wall_at = -100.0;
        true
    }

    // znova načíta vzhľad z nastavení (farby, veľkosť, hustota, plynulé posúvanie, písmo výstupu)
    fn apply_look(&mut self, ctx: &egui::Context) {
        let src = self.wall_source();
        self.wall.want(ctx, src);
        self.pal = self.palette();
        theme::apply(ctx, &self.pal);
        ctx.set_zoom_factor((self.get("uiZoom").as_f64().unwrap_or(100.0) / 100.0).clamp(0.8, 1.4) as f32);
        widgets::set_dense(self.get("density").as_str() == Some("compact"));
        let inertia = self.get("inertia").as_bool() != Some(false) && self.anim_on();
        self.smooth.on = inertia;
        ctx.all_styles_mut(|s| {
            s.scroll_animation = if inertia { egui::style::ScrollAnimation::new(1600.0, egui::Rangef::new(0.12, 0.3)) } else { egui::style::ScrollAnimation::none() };
            s.animation_time = if self.anim_on() { 0.12 } else { 0.0 };
        });
        let fs = self.get("terminalFontSize").as_f64().unwrap_or(13.0) as f32;
        self.out.font_size = fs;
        self.sh.font_size = fs;
    }

    fn radius(&self) -> u8 {
        self.get("cornerRadius").as_f64().unwrap_or(14.0).clamp(0.0, 26.0) as u8
    }

    fn open_settings(&mut self, tab: &str, ctx: &egui::Context) {
        self.settings = Some(prefs::SettingsUi::new(tab, ctx.input(|i| i.time)));
    }

    // hlásenie o úspechu v stavovom riadku (zelené)
    fn note(&mut self, s: String) {
        self.ok_at = Some(Instant::now());
        self.ok_status = s.clone();
        self.status = s;
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
        // prepnutie karty: obsah príde zo strany, kde karta leží (späť/dopredu má svoj smer z go())
        if self.nav_dir == 0.0 && (i != self.active || self.home) {
            self.nav_dir = if self.home || i > self.active { 0.6 } else { -0.6 };
        }
        self.active = i;
        self.home = false;
    }

    // aktuálne miesto pre históriu späť/dopredu: domov, stránka projektu alebo súbor
    fn place(&self) -> String {
        if self.start {
            "home".into()
        } else if self.home || self.tabs.is_empty() {
            format!("proj:{}", self.workspace().unwrap_or_default())
        } else {
            format!("file:{}", self.tabs[self.active].path)
        }
    }

    // na konci snímky: nové miesto → do histórie (miesta sa menia z veľa miest, takto sa nič nevynechá)
    fn track_place(&mut self) {
        let p = self.place();
        if p == "proj:" {
            return;
        }
        if self.hist.get(self.hist_i) != Some(&p) {
            if !self.hist.is_empty() {
                self.hist.truncate(self.hist_i + 1);
            }
            self.hist.push(p);
            self.hist_i = self.hist.len() - 1;
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
        // najviac jeden krok za 150 ms (dvojitá udalosť tlačidla myši, druhý prechod egui)
        if self.last_go.is_some_and(|t| t.elapsed() < Duration::from_millis(150)) {
            return;
        }
        self.last_go = Some(Instant::now());
        self.nav_dir = if back { -1.0 } else { 1.0 };
        let mut i = self.hist_i;
        loop {
            i = match if back { i.checked_sub(1) } else { Some(i + 1).filter(|i| *i < self.hist.len()) } {
                Some(i) => i,
                None => return,
            };
            // preskočiť miesta, ktoré už neexistujú (zmazaný súbor alebo projekt)
            let p = self.hist[i].clone();
            let gone = p.strip_prefix("file:").or(p.strip_prefix("proj:")).is_some_and(|x| !Path::new(x).exists());
            if gone || p == self.place() {
                continue;
            }
            self.hist_i = i;
            if p == "home" {
                let now = self.switched.max(0.0);
                self.open_start(now);
            } else if let Some(dir) = p.strip_prefix("proj:") {
                self.start = false;
                if self.workspace().as_deref() != Some(dir) {
                    self.open_folder(dir);
                }
                self.home = true;
            } else if let Some(f) = p.strip_prefix("file:") {
                self.start = false;
                // súbor z iného projektu: najprv prepnúť projekt, aby sedel bočný panel
                let ws = self.workspace().unwrap_or_default();
                if ws.is_empty() || !f.starts_with(&ws) {
                    let owner = self.projects.iter().filter_map(|p| p["dir"].as_str()).filter(|d| f.starts_with(d)).max_by_key(|d| d.len()).map(String::from);
                    if let Some(d) = owner {
                        self.open_folder(&d);
                    }
                }
                if let Some(t) = self.tabs.iter().position(|t| t.path == f) {
                    self.active = t;
                    self.home = false;
                } else {
                    self.open_file(f);
                }
            }
            return;
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
        // Live Server: otvorené stránky sa znova načítajú (už z disku)
        if let Some(s) = &self.server {
            if let Some(t) = self.tabs.get(i) {
                s.clear_live(&t.path);
            }
            s.bump();
        }
    }

    // HTML, CSS a JS webovej stránky sa „spúšťajú“ cez Live Server (JS s index.html vedľa)
    fn is_web_file(&self) -> bool {
        let Some(tab) = self.tabs.get(self.active) else { return false };
        match tab.ext().as_str() {
            "html" | "htm" | "css" => true,
            "js" => Path::new(&tab.path).parent().map(|d| d.join("index.html").exists()).unwrap_or(false),
            _ => false,
        }
    }

    // spustí Live Server nad projektom (alebo priečinkom súboru) a otvorí stránku v prehliadači
    // Live Server → panel so stránkou vedľa kódu
    fn live_server(&mut self) {
        self.open_preview();
    }

    // spustí server nad projektom (alebo priečinkom súboru) a vráti adresu stránky aktívneho súboru
    fn live_url(&mut self) -> Option<String> {
        let tab = self.tabs.get(self.active)?;
        let file = tab.path.clone();
        let root = self.workspace().filter(|w| file.starts_with(w.as_str())).map(std::path::PathBuf::from).or_else(|| Path::new(&file).parent().map(|p| p.to_path_buf())).unwrap_or_default();
        if self.server.as_ref().map(|s| s.root != root).unwrap_or(true) {
            match crate::server::Server::start(&root) {
                Ok(s) => self.server = Some(s),
                Err(e) => {
                    self.status = e.to_string();
                    return None;
                }
            }
        }
        // CSS/JS → stránka index.html v tom istom priečinku
        let page = if matches!(Path::new(&file).extension().and_then(|e| e.to_str()), Some("html" | "htm")) {
            file
        } else {
            Path::new(&file).parent().map(|d| d.join("index.html").to_string_lossy().to_string()).unwrap_or(file)
        };
        self.server.as_ref().map(|s| s.url(&page))
    }

    fn run(&mut self) {
        if self.tabs.is_empty() || self.home {
            return;
        }
        self.save(self.active);
        if self.is_web_file() {
            return self.live_server();
        }
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
        if self.get("clearOnRun").as_bool() != Some(false) {
            self.out.clear();
        } else {
            self.out.feed("\r\n");
        }
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
                    if let Some(s) = &self.server {
                        s.bump();
                    }
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
                "find" => {
                    self.find = Some(editing::Find { q: arg.to_string(), repl: String::new(), replace: true, idx: 0, focus: false });
                    self.find_goto = true;
                }
                "palette" => self.open_palette(
                    match arg {
                        "all" => menus::PaletteMode::Everything,
                        "commands" => menus::PaletteMode::Commands,
                        _ => menus::PaletteMode::Files,
                    },
                    ctx,
                ),
                "sfind" => self.settings_find(arg),
                // edit=<text>: pripíše text do otvoreného súboru bez uloženia (testy Live Servera)
                "edit" => {
                    if let Some(tab) = self.tabs.get_mut(self.active) {
                        tab.text.push_str(&arg.replace("\\n", "\n"));
                        self.last_edit = Some(Instant::now());
                    }
                }
                "pq" => {
                    if let Some(pl) = self.palette.as_mut() {
                        pl.set_query(arg);
                    }
                }
                "settings" => self.open_settings(if arg.is_empty() { "general" } else { arg }, ctx),
                "ai" => self.toggle_ai(),
                "newproj" => self.open_new_project(),
                "newfile" => self.open_new_file(None),
                "panic" => panic!("test crash"),
                "gh-signin" => self.gh_sign_in(ctx),
                "gh-pick" => self.gh_pick(ctx),
                "ask" => self.ai_ask(arg, ctx),
                "set" => {
                    // set=kľúč:hodnota (JSON), napr. set=panelPos:"right"
                    if let Some((k, v)) = arg.split_once(':') {
                        let v = serde_json::from_str(v).unwrap_or(json!(v));
                        self.set(k, v, ctx);
                    }
                }
                "tour" => self.tour = Some(arg.parse().unwrap_or(0)),
                "start" => self.open_start(ctx.input(|i| i.time)),
                "hist" => eprintln!("[hist] {} {:?}", self.hist_i, self.hist),
                "back" => self.go(true),
                "fwd" => self.go(false),
                "preview" => self.open_preview(),
                "update" => self.upd.check(ctx, 0.0, false),
                "update-get" => self.upd.download(ctx),
                "update-install" => self.restart_to_update(ctx),
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
        chrome::drag_area(ui, r, "side");
        // bočný panel vpravo: vpravo hore sú − □ ×, prepínač panela je v hornej lište
        let with_toggle = with_toggle && self.get("sidePos").as_str() != Some("right");
        let brand = Rect::from_min_size(pos2(r.left() + 2.0, r.center().y - 14.0), vec2(70.0, 28.0));
        self.tour_rects.insert("brand", brand);
        let resp = ui.interact(brand, ui.id().with("brand"), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(brand, CornerRadius::same(8), p.hover);
        }
        // logo otvorí domov Fluxu (ako .brand v Electron Fluxe)
        if resp.on_hover_text(t("Home")).clicked() {
            let now = ui.ctx().input(|i| i.time);
            self.open_start(now);
        }
        widgets::brand_mark(ui, Rect::from_min_size(pos2(brand.left() + 4.0, r.center().y - 10.0), vec2(20.0, 20.0)), &p);
        ui.painter().text(pos2(brand.left() + 32.0, r.center().y), Align2::LEFT_CENTER, "flux", theme::bold(15.0), p.text);
        let menu = widgets::icon_button_at(ui, Rect::from_center_size(pos2(brand.right() + 17.0, r.center().y), vec2(28.0, 28.0)), "menu", 16.0, &p, true).on_hover_text(t("Menu"));
        self.app_menu(&menu);
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

    // riadok projektu (.pr-row): ikona jazyka, meno, štatistiky; pri prejdení myšou skryť a pripnúť
    fn project_row(
        &mut self,
        ui: &mut egui::Ui,
        pr: &Value,
        ws: Option<&str>,
        open: &mut Option<String>,
        menu_for: &mut Option<(egui::Response, String, bool, bool)>,
        rename: &mut Option<(String, String)>,
    ) {
        let p = self.pal;
        let dir = pr["dir"].as_str().unwrap_or("").to_string();
        let name = pr["name"].as_str().unwrap_or("").to_string();
        let pinned = pr["pinned"].as_bool() == Some(true);
        let hidden = pr["hidden"].as_bool() == Some(true);
        let sel = ws == Some(dir.as_str());
        let sub = self.project_sub(&dir);
        let spec = self.project_icon(&dir);
        let lead = Lead::Project(&spec);
        if let Some((from, new)) = self.renaming.as_mut().filter(|r| r.0 == dir) {
            let te = ui.add(egui::TextEdit::singleline(new).desired_width(f32::INFINITY).margin(Margin::symmetric(10, 8)));
            te.request_focus();
            if te.lost_focus() {
                let (f, n) = (from.clone(), new.clone());
                self.renaming = None;
                // Enter aj klik inam uloží, Esc zruší
                if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    *rename = Some((f, n));
                }
            }
            return;
        }
        let r = widgets::row(ui, sel, 10.0, lead, &name, Some(&sub), true, &p).on_hover_text(&dir);
        if r.secondary_clicked() || r.context_menu_opened() {
            *menu_for = Some((r.clone(), dir.clone(), pinned, hidden));
        }
        let over = ui.rect_contains_pointer(r.rect);
        let k = ui.ctx().animate_bool_with_time(r.id.with("acts"), over, 0.15);
        let cy = r.rect.center().y;
        if k > 0.0 || pinned {
            // špendlík (pripnutý = vždy viditeľný a vyplnený accentom), vedľa neho skryť
            let pin_r = Rect::from_center_size(pos2(r.rect.right() - 16.0, cy), vec2(24.0, 24.0));
            let pr_ = ui.interact(pin_r, ui.id().with(("pin", &dir)), Sense::click()).on_hover_text(t(if pinned { "Unpin" } else { "Pin to top" }));
            if pr_.hovered() {
                ui.painter().rect_filled(pin_r, CornerRadius::same(7), p.active);
            }
            let pc = if pinned { p.accent } else { p.text3.lerp_to_gamma(p.text, if pr_.hovered() { 1.0 } else { 0.0 }) };
            widgets::icon_at(ui, pin_r.center(), 13.0 * (0.8 + 0.2 * if pinned { 1.0 } else { k }), "pin", pc.gamma_multiply(if pinned { 1.0 } else { k }));
            if pr_.clicked() {
                fsops::pin(&self.core, &dir, !pinned);
                self.reload_projects();
            }
            if k > 0.0 {
                let hide_r = Rect::from_center_size(pos2(r.rect.right() - 40.0, cy), vec2(24.0, 24.0));
                let hr = ui.interact(hide_r, ui.id().with(("hide", &dir)), Sense::click()).on_hover_text(t(if hidden { "Show in the sidebar" } else { "Hide from the sidebar" }));
                if hr.hovered() {
                    ui.painter().rect_filled(hide_r, CornerRadius::same(7), p.active);
                }
                widgets::icon_at(ui, hide_r.center(), 13.0 * (0.8 + 0.2 * k), if hidden { "eye" } else { "eyeOff" }, (if hr.hovered() { p.text } else { p.text3 }).gamma_multiply(k));
                if hr.clicked() {
                    fsops::hide(&self.core, &dir, !hidden);
                    self.reload_projects();
                }
            }
        }
        if r.clicked() {
            *open = Some(dir);
        }
    }

    fn sidebar(&mut self, root: &mut egui::Ui) {
        let p = self.pal;
        (if self.get("sidePos").as_str() == Some("right") { egui::Panel::right("side") } else { egui::Panel::left("side") })
            .resizable(false)
            .exact_size(SIDE_W)
            .show_separator_line(false)
            // spodok ako karta: päta končí GAP nad okrajom a je v rovine so stavovým riadkom
            .frame(Frame::new().fill(p.base).inner_margin(Margin { left: 8, right: 6, top: 0, bottom: GAP as i8 }))
            .show(root, |ui| {
                self.side_top(ui, true);
                let ws = self.workspace();
                let mut open = None;
                let mut menu_for: Option<(egui::Response, String, bool, bool)> = None;
                let mut rename: Option<(String, String)> = None;
                let hidden = self.projects.iter().filter(|pr| pr["hidden"].as_bool() == Some(true)).count();
                let projects = self.projects.clone();
                // ---- PINNED (pripnuté projekty v jemnom rámčeku, .pr-pinned) ----
                let pinned: Vec<Value> = projects.iter().filter(|pr| pr["pinned"].as_bool() == Some(true) && pr["hidden"].as_bool() != Some(true)).cloned().collect();
                if !pinned.is_empty() {
                    let (cap, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
                    widgets::icon_at(ui, pos2(cap.left() + 13.0, cap.center().y), 11.0, "pin", p.text3);
                    let mut job = egui::text::LayoutJob::default();
                    job.append(&t("Pinned").to_uppercase(), 0.0, egui::TextFormat { font_id: theme::bold(11.0), color: p.text3, extra_letter_spacing: 1.0, ..Default::default() });
                    let g = ui.fonts_mut(|f| f.layout_job(job));
                    ui.painter().galley(pos2(cap.left() + 22.0, cap.center().y - g.size().y / 2.0), g, p.text3);
                    let box_top = ui.cursor().top();
                    let where_ = ui.painter().add(egui::Shape::Noop);
                    ui.add_space(3.0);
                    let mut inner = ui
                        .new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(ui.max_rect().left() + 3.0, ui.cursor().top()), pos2(ui.max_rect().right() - 3.0, ui.max_rect().bottom()))));
                    for pr in &pinned {
                        self.project_row(&mut inner, pr, ws.as_deref(), &mut open, &mut menu_for, &mut rename);
                    }
                    let used = inner.min_rect().height();
                    ui.add_space(used + 3.0);
                    let br = Rect::from_min_max(pos2(ui.max_rect().left(), box_top), pos2(ui.max_rect().right(), ui.cursor().top()));
                    let tint = p.hover.lerp_to_gamma(p.accent.gamma_multiply(0.35), 0.2);
                    ui.painter().set(where_, egui::epaint::RectShape::new(br, CornerRadius::same(12), tint, Stroke::new(1.0, p.line.lerp_to_gamma(p.accent, 0.22)), StrokeKind::Inside));
                    ui.add_space(10.0);
                }
                // ---- PROJECTS ----
                if widgets::section(ui, &t("Projects").to_uppercase(), Some("folderOpen"), &p).map(|r| r.on_hover_text(t("Open folder")).clicked()) == Some(true) {
                    if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                        self.open_folder(&d.to_string_lossy());
                    }
                }
                let sk = egui::Id::new("sa-projects");
                let off = self.smooth.begin(ui.ctx(), sk, ui.layer_id(), None);
                let mut sa = egui::ScrollArea::vertical().id_salt("projects").max_height(200.0).auto_shrink([false, true]);
                if let Some(o) = off {
                    sa = sa.vertical_scroll_offset(o);
                }
                let sout = sa.show(ui, |ui| {
                    for pr in projects.iter().filter(|pr| pr["pinned"].as_bool() != Some(true)) {
                        if pr["hidden"].as_bool() == Some(true) && !self.show_hidden {
                            continue;
                        }
                        self.project_row(ui, pr, ws.as_deref(), &mut open, &mut menu_for, &mut rename);
                    }
                });
                self.smooth.end(sk, &sout);
                self.tour_rects.insert("projects", Rect::from_min_size(sout.inner_rect.min, vec2(sout.inner_rect.width(), sout.content_size.y.min(sout.inner_rect.height()))));
                if let Some((r, d, pinned, hid)) = menu_for {
                    self.project_menu(&r, &d, pinned, hid);
                }
                if let Some((f, n)) = rename {
                    self.finish_rename(&f, &n);
                }
                if let Some(d) = open {
                    if ws.as_deref() == Some(d.as_str()) {
                        self.home = true;
                    } else {
                        self.open_folder(&d);
                    }
                }
                if widgets::row(ui, false, 10.0, Lead::Line("plus"), &t("New project"), None, false, &p).clicked() {
                    self.open_new_project();
                }
                if hidden > 0
                    && widgets::row(ui, false, 10.0, Lead::Line(if self.show_hidden { "eyeOff" } else { "eye" }), &crate::i18n::tf("{n} hidden", &[("n", &hidden.to_string())]), None, false, &p)
                        .clicked()
                {
                    self.show_hidden = !self.show_hidden;
                }
                // ---- FILES (súbory otvorené mimo projektu) ----
                if !self.recent_files.is_empty() {
                    widgets::separator(ui, &p);
                    if widgets::section(ui, &t("Files").to_uppercase(), Some("filePlus"), &p).map(|r| r.on_hover_text(t("New file")).clicked()) == Some(true) {
                        let d = self.recent_files.first().and_then(|f| Path::new(f).parent().map(|d| d.to_string_lossy().to_string()));
                        self.open_new_file(d);
                    }
                    let mut open_f = None;
                    let sk = egui::Id::new("sa-loose");
                    let off = self.smooth.begin(ui.ctx(), sk, ui.layer_id(), None);
                    let mut sa = egui::ScrollArea::vertical().id_salt("loose").max_height(170.0).auto_shrink([false, true]);
                    if let Some(o) = off {
                        sa = sa.vertical_scroll_offset(o);
                    }
                    let sout = sa.show(ui, |ui| {
                        for f in &self.recent_files {
                            let sel = !self.home && self.tabs.get(self.active).map(|t| &t.path == f).unwrap_or(false);
                            if widgets::row(ui, sel, 10.0, Lead::File(&widgets::file_name(f)), &widgets::file_name(f), Some(&short_dir(f)), false, &p).on_hover_text(f).clicked() {
                                open_f = Some(f.clone());
                            }
                        }
                    });
                    self.smooth.end(sk, &sout);
                    if let Some(f) = open_f {
                        self.open_file(&f);
                    }
                }
                // ---- strom súborov projektu ----
                let footer_h = 28.0; // výška stavového riadka karty
                if let Some(w) = ws.clone() {
                    widgets::separator(ui, &p);
                    let tb = ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        ui.add_space(4.0);
                        if widgets::icon_button(ui, "filePlus", &p, true).on_hover_text(t("New file")).clicked() {
                            self.open_new_file(None);
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
                    self.tour_rects.insert("files", Rect::from_min_size(tb.response.rect.min, vec2(4.0 * 30.0 + 4.0, tb.response.rect.height())));
                    ui.add_space(4.0);
                    // strom končí presne nad pätou (aj s medzerou medzi prvkami)
                    let h = (ui.max_rect().bottom() - footer_h - ui.cursor().top() - ui.spacing().item_spacing.y - 2.0).max(40.0);
                    let sk = egui::Id::new("sa-tree");
                    let off = self.smooth.begin(ui.ctx(), sk, ui.layer_id(), None);
                    let mut sa = egui::ScrollArea::vertical().id_salt("tree").max_height(h).auto_shrink([false, false]);
                    if let Some(o) = off {
                        sa = sa.vertical_scroll_offset(o);
                    }
                    let sout = sa.show(ui, |ui| {
                        // zvýraznenie otvoreného súboru kĺže k novému riadku (slideIndicator v Electron Fluxe)
                        let bg = ui.painter().add(egui::Shape::Noop);
                        let origin = ui.min_rect().top();
                        self.tree_sel = None;
                        self.tree_ui(ui, &w, 0);
                        if let Some(r) = self.tree_sel {
                            let t = if self.anim_on() { 0.2 } else { 0.0 };
                            let y = ui.ctx().animate_value_with_time(egui::Id::new("tree-glide"), r.top() - origin, t);
                            let gr = Rect::from_min_size(pos2(r.left(), origin + y), r.size());
                            ui.painter().set(bg, egui::Shape::rect_filled(gr, CornerRadius::same(8), self.pal.active));
                        }
                    });
                    self.smooth.end(sk, &sout);
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
                }
                // ---- päta: Nastavenia, skratky, svetlá/tmavá ----
                // pevne na spodku panela: čiara = horná čiara stavového riadka karty, text v jeho strede
                // (nie podľa toku rozloženia – medzery medzi prvkami ju posúvali nižšie)
                let mr = ui.max_rect();
                let r = Rect::from_min_max(pos2(mr.left(), mr.bottom() - footer_h), pos2(mr.right(), mr.bottom()));
                ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, p.line));
                let s = Rect::from_min_size(pos2(r.left(), r.center().y - 12.0), vec2(96.0, 24.0));
                self.tour_rects.insert("settings", r);
                let sr = ui.interact(s, ui.id().with("settings"), Sense::click());
                if sr.hovered() {
                    ui.painter().rect_filled(s, CornerRadius::same(8), p.hover);
                }
                widgets::icon_at(ui, pos2(s.left() + 15.0, s.center().y), 15.0, "settings", p.text2);
                ui.painter().text(pos2(s.left() + 31.0, s.center().y), Align2::LEFT_CENTER, t("Settings"), theme::bold(13.0), p.text2);
                if sr.on_hover_text(t("Settings")).clicked() {
                    self.open_settings("general", ui.ctx());
                }
                let sun = Rect::from_center_size(pos2(r.right() - 12.0, s.center().y), vec2(26.0, 26.0));
                let cmd = sun.translate(vec2(-26.0, 0.0));
                widgets::icon_button_at(ui, cmd, "command", 15.0, &p, true).on_hover_text(t("F5 Run · Ctrl+S Save · Ctrl+O Open folder · Ctrl+W Close"));
                if widgets::icon_button_at(ui, sun, if p.dark { "sun" } else { "moon" }, 15.0, &p, true).on_hover_text(t("Light / dark")).clicked() {
                    self.set_theme(!p.dark, ui.ctx());
                }
            });
    }

    // pole na názov nového súboru/priečinka priamo v strome (v cieľovom priečinku)
    fn new_item_ui(&mut self, ui: &mut egui::Ui, dir: &str, indent: f32) {
        let target = self.new_in.clone().or_else(|| self.workspace()).unwrap_or_default();
        let Some((is_dir, name)) = &mut self.new_item else { return };
        if target != dir {
            return;
        }
        let mut r = None;
        ui.horizontal(|ui| {
            ui.add_space(indent);
            r = Some(ui.add(egui::TextEdit::singleline(name).hint_text(if *is_dir { t("Folder name") } else { t("File name") }).desired_width(f32::INFINITY).margin(Margin::symmetric(10, 5))));
        });
        let r = r.unwrap();
        r.request_focus();
        if r.lost_focus() {
            let (d, n) = (*is_dir, name.trim().to_string());
            self.new_item = None;
            self.new_in = None;
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !n.is_empty() {
                let target = Path::new(dir).join(&n).to_string_lossy().to_string();
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

    fn tree_ui(&mut self, ui: &mut egui::Ui, dir: &str, depth: usize) {
        let p = self.pal;
        self.new_item_ui(ui, dir, depth as f32 * 14.0);
        let items = self.list(dir);
        for (name, path, is_dir) in items {
            let indent = 6.0 + depth as f32 * 14.0;
            if let Some((from, new)) = self.renaming.as_mut().filter(|r| r.0 == path) {
                let mut te = None;
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    te = Some(ui.add(egui::TextEdit::singleline(new).desired_width(f32::INFINITY).margin(Margin::symmetric(10, 5))));
                });
                let te = te.unwrap();
                te.request_focus();
                if te.lost_focus() {
                    let (f, n) = (from.clone(), new.clone());
                    self.renaming = None;
                    if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.finish_rename(&f, &n);
                    }
                }
                continue;
            }
            let open = self.open_dirs.contains(&path);
            let active = !self.home && self.tabs.get(self.active).map(|t| t.path == path).unwrap_or(false);
            let dirty = self.tabs.iter().any(|t| t.path == path && t.dirty());
            let lead = if is_dir { Lead::Folder { open } } else { Lead::File(&name) };
            let ind = if is_dir { indent } else { indent + 20.0 };
            // aktívny riadok bez vlastnej výplne – kreslí ju kĺzavé zvýraznenie v strome
            let r = widgets::row_ex(ui, active, false, ind, lead, &name, None, false, &p);
            if active {
                self.tree_sel = Some(r.rect);
            }
            self.tree_menu(&r, &path, is_dir);
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
        // prázdna lišta presúva okno (ako -webkit-app-region: drag); tlačidlá nad ňou majú prednosť
        chrome::drag_area(ui, r, "top");
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
        // vpravo: − □ × okna, Stop, ▶ Run, AI, hľadanie
        let full_r = ui.ctx().content_rect().right();
        let side_right = self.side_open && self.get("sidePos").as_str() == Some("right");
        let mut rx = if side_right { r.right() } else { r.right().min(full_r - chrome::CONTROLS_W - 6.0) };
        if side_right {
            if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx - 14.0, cy), vec2(28.0, 28.0)), "sidebar", 16.0, &p, true).on_hover_text(t("Hide sidebar")).clicked() {
                self.side_open = false;
            }
            rx -= 36.0;
        }
        let can_run = !self.tabs.is_empty() && !self.home;
        // na stránke projektu ▶ Run nie je (ako v Electron Fluxe)
        // jedno tlačidlo: ▶ Run ↔ ■ Stop; pri HTML/CSS/JS webu Live Server (ako v Electron Fluxe)
        let web = can_run && self.is_web_file();
        let live_on = self.server.is_some();
        if can_run || self.running || live_on {
            let stopping = self.running || (web && live_on);
            let (icon, label, tip) = if stopping {
                ("stop", t("Stop"), if self.running { t("Stop (Shift+F5)") } else { t("Stop Live Server") })
            } else if web {
                ("globe", t("Live Server"), t("Open the page in the browser – it reloads when you save (F5)"))
            } else {
                ("play", t("Run"), t("Run (F5)"))
            };
            let run_w = widgets::text_w(ui, &label, theme::bold(13.5)) + 46.0;
            let aw = ui.ctx().animate_value_with_time(egui::Id::new("run-w"), run_w, if self.anim_on() { 0.15 } else { 0.0 });
            let run = Rect::from_min_size(pos2(rx - aw, cy - 15.0), vec2(aw, 30.0));
            self.tour_rects.insert("run", run);
            let rr = ui.interact(run, ui.id().with("run"), Sense::click());
            let hk = ui.ctx().animate_bool_with_time(rr.id.with("h"), rr.hovered(), 0.12);
            let (bg, fg) = if stopping { (p.red.gamma_multiply(0.16 + 0.08 * hk), p.red) } else { (p.accent.lerp_to_gamma(Color32::WHITE, 0.08 * hk), p.accent_fg) };
            ui.painter().rect_filled(run, CornerRadius::same(10), bg);
            if stopping {
                ui.painter().rect_stroke(run, CornerRadius::same(10), Stroke::new(1.0, p.red.gamma_multiply(0.45)), StrokeKind::Inside);
            }
            let clip = ui.painter().with_clip_rect(run);
            widgets::icon_at(ui, pos2(run.left() + 18.0, cy), if stopping { 12.0 } else { 14.0 }, icon, fg);
            clip.text(pos2(run.left() + 32.0, cy), Align2::LEFT_CENTER, &label, theme::bold(13.5), fg);
            if rr.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                if self.running {
                    self.out.pty.kill();
                } else if web && live_on {
                    self.close_preview();
                } else {
                    self.run();
                }
            }
            rx -= aw + 7.0;
            // adresa bežiaceho Live Servera (klik ju otvorí znova)
            if let Some(s) = &self.server {
                let url = format!("127.0.0.1:{}", s.port);
                let w = widgets::text_w(ui, &url, theme::ui(12.0)) + 34.0;
                let r = Rect::from_min_size(pos2(rx - w, cy - 13.0), vec2(w, 26.0));
                let resp = ui.interact(r, ui.id().with("live-url"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                ui.painter().rect_filled(r, CornerRadius::same(9), if resp.hovered() { p.active } else { p.hover });
                ui.painter().circle_filled(pos2(r.left() + 13.0, cy), 3.5, p.green);
                ui.painter().text(pos2(r.left() + 22.0, cy), Align2::LEFT_CENTER, &url, theme::ui(12.0), p.text2);
                if resp.on_hover_text(t("Open in browser")).clicked() {
                    let u = self.tabs.get(self.active).map(|t| s.url(&t.path)).unwrap_or_else(|| format!("http://127.0.0.1:{}/", s.port));
                    settings::open_external(&u);
                }
                rx -= w + 7.0;
            }
        }
        let ai = Rect::from_min_size(pos2(rx - 56.0, cy - 15.0), vec2(56.0, 30.0));
        let ar = ui.interact(ai, ui.id().with("ai"), Sense::click());
        ui.painter().rect_filled(ai, CornerRadius::same(10), if ar.hovered() { p.active } else { p.hover });
        widgets::icon_at(ui, pos2(ai.left() + 17.0, cy), 14.0, "sparkle", p.text2);
        ui.painter().text(pos2(ai.left() + 29.0, cy), Align2::LEFT_CENTER, "AI", theme::bold(13.0), p.text2);
        if self.ai.open {
            ui.painter().rect_stroke(ai, CornerRadius::same(10), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        }
        if ar.on_hover_text(t("Claude AI (Ctrl+I)")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.toggle_ai();
        }
        rx -= 63.0;
        if self.get("showSearch").as_bool() != Some(false) && self.get("searchWide").as_bool() == Some(true) {
            // široké pole hľadania (body.search-wide #topsearch): 🔍 Search … Ctrl+Shift+A
            let w = (rx - x - 160.0).clamp(120.0, 280.0);
            let sr = Rect::from_min_size(pos2(rx - w - 6.0, cy - 15.0), vec2(w, 30.0));
            let resp = ui.interact(sr, ui.id().with("topsearch"), Sense::click()).on_hover_cursor(egui::CursorIcon::Text);
            let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
            ui.painter().rect_filled(sr, CornerRadius::same(9), p.card2);
            ui.painter().rect_stroke(sr, CornerRadius::same(9), Stroke::new(1.0, p.line.lerp_to_gamma(p.line_strong, hk)), StrokeKind::Inside);
            let col = p.text3.lerp_to_gamma(p.text2, hk);
            widgets::icon_at(ui, pos2(sr.left() + 18.0, cy), 14.0, "search", col);
            let key_w = if w > 200.0 { widgets::text_w(ui, "Ctrl+Shift+A", theme::ui(10.5)) + 10.0 } else { 0.0 };
            widgets::text(ui, pos2(sr.left() + 32.0, cy), Align2::LEFT_CENTER, &t("Search"), theme::ui(13.0), col, w - 40.0 - key_w);
            if key_w > 0.0 {
                ui.painter().text(pos2(sr.right() - 8.0, cy), Align2::RIGHT_CENTER, "Ctrl+Shift+A", theme::ui(10.5), p.text3);
            }
            if resp.on_hover_text(t("Search files, commands, settings and projects")).clicked() {
                self.act("search", ui.ctx());
            }
            rx -= w + 12.0;
        } else if self.get("showSearch").as_bool() != Some(false) {
            if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx - 14.0, cy), vec2(30.0, 30.0)), "search", 16.0, &p, true)
                .on_hover_text(format!("{} (Ctrl+Shift+A)", t("Search files, commands, settings and projects")))
                .clicked()
            {
                self.act("search", ui.ctx());
            }
            rx -= 36.0;
        }
        // karty súborov
        let mut close = None;
        let mut act = None;
        let tabs_clip = Rect::from_x_y_ranges(x..=rx, r.y_range());
        let painter = ui.painter().with_clip_rect(tabs_clip);
        // zvýraznenie aktívnej karty sa presúva plynulo (slideIndicator v Electron Fluxe)
        {
            let mut xx = x;
            for (i, t) in self.tabs.iter().enumerate() {
                let w = (widgets::text_w(ui, &t.name(), theme::ui(13.0)) + 16.0 + 7.0 + 20.0 + 16.0).min(220.0);
                if i == self.active && !self.home {
                    let t_anim = if self.anim_on() { 0.18 } else { 0.0 };
                    let ax = ui.ctx().animate_value_with_time(egui::Id::new("tab-ind-x"), xx, t_anim);
                    let aw = ui.ctx().animate_value_with_time(egui::Id::new("tab-ind-w"), w, t_anim);
                    let tr = Rect::from_min_size(pos2(ax, cy - 15.0), vec2(aw, 30.0));
                    painter.add(egui::Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(if p.dark { 70 } else { 18 }) }.as_shape(tr, CornerRadius::same(9)));
                    painter.rect_filled(tr, CornerRadius::same(9), p.card);
                    painter.rect_stroke(tr, CornerRadius::same(9), Stroke::new(1.0, p.line), StrokeKind::Inside);
                }
                xx += w + 4.0;
            }
        }
        for (i, t) in self.tabs.iter().enumerate() {
            let sel = i == self.active && !self.home;
            let name = t.name();
            let w = (widgets::text_w(ui, &name, theme::ui(13.0)) + 16.0 + 7.0 + 20.0 + 16.0).min(220.0);
            let tr = Rect::from_min_size(pos2(x, cy - 15.0), vec2(w, 30.0));
            if tr.left() > rx {
                break;
            }
            let resp = ui.interact(tr.intersect(tabs_clip), ui.id().with(("tab", i)), Sense::click());
            let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered() && !sel, 0.12);
            // každá karta má svoj jemný rámik (aktívna ho má výraznejší – kĺzavý indikátor vyššie)
            if !sel {
                painter.rect_filled(tr, CornerRadius::same(9), p.card2.lerp_to_gamma(p.hover, hk));
                painter.rect_stroke(tr, CornerRadius::same(9), Stroke::new(1.0, p.line), StrokeKind::Inside);
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
        // veľkosť písma a výška riadku z nastavení (Monaco: 14 px × 1.45 ≈ 20 px)
        let fsz = self.get("fontSize").as_f64().unwrap_or(14.0).clamp(9.0, 32.0) as f32;
        let lh: f32 = (fsz * self.get("lineHeight").as_f64().unwrap_or(1.45) as f32).round();
        const PAD: f32 = 16.0;
        let nums = self.get("lineNumbers").as_str().unwrap_or("on").to_string();
        let gutter: f32 = if nums == "off" { 26.0 } else { 67.0 };
        let mini_w: f32 = if self.get("minimap").as_bool() != Some(false) { 112.0 } else { 0.0 };
        let wrap = self.get("wordWrap").as_bool() == Some(true);
        let font = theme::mono(fsz);
        let ed_rect = Rect::from_min_max(rect.min, pos2(rect.right() - mini_w, rect.bottom()));
        let mini = Rect::from_min_max(pos2(rect.right() - mini_w, rect.top()), rect.max);
        // ---- klávesy editora pred TextEdit: Tab/Shift+Tab, Ctrl+/, Ctrl+F/H, písané zátvorky ----
        let ed_id = egui::Id::new(("editor", self.tabs[self.active].path.clone()));
        let ctx = ui.ctx().clone();
        let focused = ctx.memory(|m| m.has_focus(ed_id));
        let (find_k, repl_k) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::COMMAND, egui::Key::F), i.consume_key(egui::Modifiers::COMMAND, egui::Key::H)));
        if find_k || repl_k {
            let sel_text = egui::TextEdit::load_state(&ctx, ed_id).and_then(|s| s.cursor.char_range()).map(|r| {
                let (a, b) = (usize::from(r.primary.index).min(usize::from(r.secondary.index)), usize::from(r.primary.index).max(usize::from(r.secondary.index)));
                self.tabs[self.active].text.chars().skip(a).take(b - a).collect::<String>()
            });
            let prev = self.find.take();
            let q = sel_text.filter(|s| !s.is_empty() && !s.contains('\n')).or(prev.as_ref().map(|f| f.q.clone())).unwrap_or_default();
            self.find = Some(editing::Find { q, repl: prev.map(|f| f.repl).unwrap_or_default(), replace: repl_k, idx: 0, focus: true });
        }
        let mut typed: Option<char> = None;
        let mut new_sel: Option<(usize, usize)> = None;
        if focused {
            let (tab_k, back_k, comment_k) = ctx.input_mut(|i| {
                (i.consume_key(egui::Modifiers::NONE, egui::Key::Tab), i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab), i.consume_key(egui::Modifiers::COMMAND, egui::Key::Slash))
            });
            typed = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Text(t) = e {
                        let mut c = t.chars();
                        match (c.next(), c.next()) {
                            (Some(ch), None) if "([{\"'".contains(ch) => Some(ch),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
            });
            if tab_k || back_k || comment_k {
                let (a, b) = egui::TextEdit::load_state(&ctx, ed_id).and_then(|s| s.cursor.char_range()).map(|r| (usize::from(r.secondary.index), usize::from(r.primary.index))).unwrap_or((0, 0));
                let ext = self.tabs[self.active].ext();
                let text = &mut self.tabs[self.active].text;
                new_sel = Some(if comment_k { editing::toggle_comment(text, a, b, &ext) } else { editing::indent(text, a, b, back_k) });
                self.last_edit = Some(Instant::now());
            }
        }
        // nálezy hľadania
        let found = self.find.as_ref().map(|f| editing::matches(&self.tabs[self.active].text, &f.q)).unwrap_or_default();
        let cur_found = self.find.as_ref().map(|f| if found.is_empty() { 0 } else { f.idx % found.len() }).unwrap_or(0);
        if self.find_goto {
            if let Some(&(a, b)) = found.get(cur_found) {
                new_sel = Some((a, b));
            }
        }
        if let Some((a, b)) = new_sel {
            let mut st = egui::TextEdit::load_state(&ctx, ed_id).unwrap_or_default();
            st.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(a), egui::text::CCursor::new(b))));
            st.store(&ctx, ed_id);
        }
        let find_goto = std::mem::take(&mut self.find_goto);
        let code = &self.code;
        let tab = &mut self.tabs[self.active];
        let ext = tab.ext();
        let lang = if ext.is_empty() { "txt".to_string() } else { ext };
        let cur_line = self.cursor.0;
        let mut edited = false;
        let mut cursor = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(ed_rect));
        child.set_clip_rect(ed_rect);
        let mut sa = if wrap { egui::ScrollArea::vertical() } else { egui::ScrollArea::both() }.auto_shrink(false).id_salt(&tab.path);
        // plynulé posúvanie (koliesko aj skoky z minimapy)
        let sk = egui::Id::new(("sa-ed", tab.path.clone()));
        if let Some(y) = self.smooth.begin(&child.ctx().clone(), sk, child.layer_id(), self.scroll_to.take()) {
            sa = sa.vertical_scroll_offset(y.max(0.0));
        }
        let out = sa.show(&mut child, |ui| {
            ui.add_space(PAD);
            ui.horizontal_top(|ui| {
                ui.add_space(gutter);
                let hl = ui.painter().add(egui::Shape::Noop);
                let found_shapes = ui.painter().add(egui::Shape::Noop);
                let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_w: f32| {
                    let mut job = code.highlight(ui.ctx(), ui.style(), buf.as_str(), &lang);
                    job.wrap.max_width = if wrap { wrap_w } else { f32::INFINITY };
                    for s in job.sections.iter_mut() {
                        s.format.font_id = font.clone();
                        s.format.line_height = Some(lh);
                        s.format.valign = egui::Align::Center;
                    }
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let lines = tab.text.lines().count().max(1) + usize::from(tab.text.ends_with('\n'));
                let te = egui::TextEdit::multiline(&mut tab.text)
                    .id(ed_id)
                    .code_editor()
                    .font(font.clone())
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .desired_width(if wrap { ui.available_width() - 12.0 } else { f32::INFINITY })
                    .desired_rows(lines.max(1))
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .show(ui);
                edited = te.response.changed() || new_sel.is_some();
                // automatické zatváranie zátvoriek a úvodzoviek
                if te.response.changed() {
                    if let (Some(ch), Some(r)) = (typed, te.cursor_range) {
                        let c = usize::from(r.primary.index);
                        if editing::auto_close(&mut tab.text, c, ch) {
                            let mut st = egui::TextEdit::load_state(ui.ctx(), ed_id).unwrap_or_default();
                            st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(c))));
                            st.store(ui.ctx(), ed_id);
                        }
                    }
                }
                // zvýraznenie nálezov (za textom) a posun na aktuálny
                if !found.is_empty() {
                    let mut shapes = vec![];
                    let clip = ui.clip_rect();
                    for (i, &(a, b)) in found.iter().enumerate() {
                        let ra = te.galley.pos_from_cursor(egui::text::CCursor::new(a)).translate(te.galley_pos.to_vec2());
                        if ra.bottom() < clip.top() - 200.0 || ra.top() > clip.bottom() + 200.0 {
                            if !(find_goto && i == cur_found) {
                                continue;
                            }
                        }
                        let rb = te.galley.pos_from_cursor(egui::text::CCursor::new(b)).translate(te.galley_pos.to_vec2());
                        let r = Rect::from_min_max(pos2(ra.left(), ra.top() + 1.0), pos2(rb.right().max(ra.left() + 2.0), ra.bottom() - 1.0));
                        let on = i == cur_found;
                        shapes.push(egui::Shape::rect_filled(r, 3.0, if on { Color32::from_rgba_unmultiplied(245, 185, 74, 110) } else { Color32::from_rgba_unmultiplied(245, 185, 74, 45) }));
                        if on && find_goto {
                            ui.scroll_to_rect(r.expand(60.0), None);
                        }
                    }
                    ui.painter().set(found_shapes, egui::Shape::Vec(shapes));
                }
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
                let num_x = te.galley_pos.x - gutter + 45.0;
                // riadok → číslo riadku (pri zalamovaní má jeden riadok viac radov)
                let mut line_no = 1usize;
                let mut starts = true;
                for row in g.rows.iter() {
                    let i = line_no - 1;
                    let n = line_no;
                    let first = starts;
                    starts = row.ends_with_newline;
                    if row.ends_with_newline {
                        line_no += 1;
                    }
                    let rr = row.rect().translate(te.galley_pos.to_vec2());
                    if rr.bottom() < clip.top() || rr.top() > clip.bottom() {
                        continue;
                    }
                    if n == cur_line && first {
                        let band = Rect::from_x_y_ranges(clip.left()..=clip.right(), rr.top()..=rr.top() + lh);
                        ui.painter().set(hl, egui::Shape::rect_filled(band, 0.0, if p.dark { Color32::from_white_alpha(7) } else { Color32::from_black_alpha(8) }));
                    }
                    let c = if n == cur_line { p.text } else { p.text3.gamma_multiply(0.75) };
                    // čísla riadkov: on / relative (vzdialenosť od kurzora) / off
                    let label = match nums.as_str() {
                        "off" => String::new(),
                        "relative" if n != cur_line => n.abs_diff(cur_line).to_string(),
                        _ => n.to_string(),
                    };
                    if first && !label.is_empty() {
                        ui.painter().text(pos2(num_x, rr.top() + lh / 2.0), Align2::RIGHT_CENTER, label, theme::mono((fsz - 1.0).max(9.0)), c);
                    }
                    if let (true, Some(l)) = (first, src_lines.get(i)) {
                        let spaces = l.chars().take_while(|c| *c == ' ').count();
                        for k in 1..=(spaces / 4) {
                            let gx = te.galley_pos.x + ((k - 1) * 4) as f32 * cw + 0.5;
                            if k * 4 <= spaces && k > 0 {
                                ui.painter().vline(gx, rr.top()..=rr.top() + lh, Stroke::new(1.0, p.line));
                            }
                        }
                    }
                }
                let _ = lines;
            });
            ui.add_space(PAD);
        });
        self.smooth.end(sk, &out);
        if edited {
            self.last_edit = Some(Instant::now());
        }
        if let Some(c) = cursor {
            self.cursor = c;
        }
        // ---- lišta hľadania ----
        if self.find.is_some() {
            match self.find_bar(ui, ed_rect, found.len()) {
                Some("close") => self.find = None,
                Some("next") => {
                    if let Some(f) = self.find.as_mut() {
                        f.idx = (cur_found + 1) % found.len().max(1);
                    }
                    self.find_goto = true;
                }
                Some("prev") => {
                    if let Some(f) = self.find.as_mut() {
                        f.idx = (cur_found + found.len().max(1) - 1) % found.len().max(1);
                    }
                    self.find_goto = true;
                }
                Some("goto") => self.find_goto = true,
                Some("replace") => {
                    if let (Some(&(a, b)), Some(f)) = (found.get(cur_found), self.find.as_ref()) {
                        let repl = f.repl.clone();
                        let text = &mut self.tabs[self.active].text;
                        let (ab, bb) = (text.char_indices().nth(a).map(|x| x.0).unwrap_or(text.len()), text.char_indices().nth(b).map(|x| x.0).unwrap_or(text.len()));
                        text.replace_range(ab..bb, &repl);
                        self.last_edit = Some(Instant::now());
                        self.find_goto = true;
                    }
                }
                Some("replace-all") => {
                    if let Some(f) = self.find.as_ref() {
                        let (q, repl) = (f.q.clone(), f.repl.clone());
                        let text = &mut self.tabs[self.active].text;
                        for &(a, b) in found.iter().rev() {
                            let (ab, bb) = (text.char_indices().nth(a).map(|x| x.0).unwrap_or(text.len()), text.char_indices().nth(b).map(|x| x.0).unwrap_or(text.len()));
                            text.replace_range(ab..bb, &repl);
                        }
                        let _ = q;
                        self.last_edit = Some(Instant::now());
                    }
                }
                _ => {}
            }
            ctx.request_repaint();
        }
        // ---- minimapa (ako v Monacu: znak = 1 px, riadok = 2 px) ----
        if mini_w <= 0.0 {
            return;
        }
        ui.painter().vline(mini.left(), mini.y_range(), Stroke::new(1.0, p.line));
        let tab = &self.tabs[self.active];
        let job = self.code.highlight(ui.ctx(), ui.style(), &tab.text, &lang);
        let painter = ui.painter().with_clip_rect(mini);
        // minimapa ako v Monacu: riadok = 2 px, znak = 1 px; pri dlhom súbore sa posúva spolu s editorom
        let ppp = ui.ctx().pixels_per_point();
        let snap = |v: f32| (v * ppp).round() / ppp;
        let total = tab.text.lines().count().max(1);
        let pitch = 2.0f32;
        let view_h = out.inner_rect.height();
        let content_h = (out.content_size.y - view_h).max(1.0);
        let frac = (out.state.offset.y / content_h).clamp(0.0, 1.0);
        let mm_h = mini.height() - 8.0;
        let mm_off = ((total as f32 * pitch - mm_h).max(0.0) * frac).round();
        let first = (mm_off / pitch) as usize;
        let max_lines = first + (mm_h / pitch) as usize + 1;
        let (x0, y0) = (mini.left() + 10.0, mini.top() + 4.0 - mm_off);
        let max_cols = ((mini_w - 18.0).max(10.0)) as usize;
        let (mut line, mut col) = (0usize, 0usize);
        let draw = |line: usize, st: usize, end: usize, color: Color32| {
            if line < first || st >= max_cols {
                return;
            }
            let end = end.min(max_cols);
            let y = snap(y0 + line as f32 * pitch);
            painter.rect_filled(Rect::from_min_size(pos2(snap(x0 + st as f32), y), vec2(snap((end - st) as f32).max(1.0 / ppp), (1.0f32).max(1.0 / ppp).max(snap(1.3)))), 0.0, color);
        };
        'outer: for s in &job.sections {
            let color = s.format.color.gamma_multiply(0.7);
            let mut run: Option<usize> = None;
            for ch in job.text[s.byte_range.start.0..s.byte_range.end.0].chars() {
                if ch == '\n' || ch == ' ' || ch == '\t' {
                    if let Some(st) = run.take() {
                        draw(line, st, col, color);
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
                draw(line, st, col, color);
            }
        }
        // posuvník minimapy: viditeľná časť; klik/ťahanie posúva editor
        let mr = ui.interact(mini, ui.id().with("minimap"), Sense::click_and_drag());
        let top_line = ((out.state.offset.y - PAD).max(0.0)) / lh;
        let slider = Rect::from_min_size(pos2(mini.left() + 1.0, y0 + top_line * pitch), vec2(mini_w - 1.0, view_h / lh * pitch));
        let hk = ui.ctx().animate_bool_with_time(ui.id().with("mm-h"), mr.hovered() || mr.dragged(), 0.15);
        if hk > 0.0 {
            painter.rect_filled(slider, 0.0, p.hover.gamma_multiply(hk));
        }
        if let Some(pos) = mr.interact_pointer_pos() {
            if mr.clicked() || mr.dragged() {
                let target_line = (pos.y - y0) / pitch;
                self.scroll_to = Some(target_line * lh + PAD - view_h / 2.0);
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
                self.open_new_project();
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
        let sk = egui::Id::new("sa-project-page");
        let off = self.smooth.begin(&child.ctx().clone(), sk, child.layer_id(), None);
        let mut sa = egui::ScrollArea::vertical().id_salt("project-page").auto_shrink(false);
        if let Some(o) = off {
            sa = sa.vertical_scroll_offset(o);
        }
        let sout = sa.show(&mut child, |ui| {
            let full = ui.available_width();
            let inner_w = (full - 60.0).min(960.0);
            let left = rect.left() + (full - inner_w) / 2.0;
            ui.add_space(40.0);
            // ---- hlavička: ikona, meno, popis, jazyky, akcie ----
            let (head, _) = ui.allocate_exact_size(vec2(full, 104.0), Sense::hover());
            let icon_box = Rect::from_min_size(pos2(left, head.top()), vec2(64.0, 64.0));
            ui.painter().rect_filled(icon_box, CornerRadius::same(14), p.card2);
            ui.painter().rect_stroke(icon_box, CornerRadius::same(14), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            // klik na ikonu = zmeniť ikonu projektu
            let ir = ui.interact(icon_box, ui.id().with("proj-icon"), Sense::click()).on_hover_text(t("Change icon")).on_hover_cursor(egui::CursorIcon::PointingHand);
            if ir.hovered() {
                ui.painter().rect_stroke(icon_box, CornerRadius::same(14), Stroke::new(1.5, p.text3), StrokeKind::Inside);
            }
            let spec = self.project_icon(&ws);
            newproj::paint_icon(ui, Rect::from_center_size(icon_box.center(), vec2(36.0, 36.0)), &spec, &p);
            egui::Popup::from_toggle_button_response(&ir).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                ui.set_width(300.0);
                let lang = langs.first().map(|k| kind_file(k)).unwrap_or("");
                let cur = self.core.setting("projectMeta")[&ws]["icon"].as_str().unwrap_or("").to_string();
                if let Some(ic) = newproj::icon_picker(ui, &cur, lang, 300.0, &p) {
                    self.set_project_meta(&ws, json!({ "icon": if ic.is_empty() { Value::Null } else { json!(ic) } }));
                }
            });
            let tx = icon_box.right() + 19.0;
            let tr = ui.painter().text(pos2(tx, head.top() + 20.0), Align2::LEFT_CENTER, widgets::file_name(&ws), theme::bold(30.0), p.text);
            ui.interact(tr, ui.id().with("proj-title"), Sense::hover()).on_hover_text(&ws);
            // popis: klik = upraviť; prázdny → „Pridať krátky popis…“
            let dr = Rect::from_min_size(pos2(tx - 4.0, head.top() + 45.0), vec2((inner_w - 420.0).max(160.0), 24.0));
            let desc = meta["description"].as_str().unwrap_or("").replace('\n', " ");
            if let Some((_, text, focus)) = self.desc_edit.as_mut().filter(|e| e.0 == ws) {
                let mut du = ui.new_child(egui::UiBuilder::new().max_rect(dr));
                let te =
                    du.add(egui::TextEdit::singleline(text).hint_text(t("Add a short description…")).desired_width(dr.width()).char_limit(160).font(theme::ui(14.0)).margin(Margin::symmetric(4, 3)));
                if std::mem::take(focus) {
                    te.request_focus();
                }
                if te.lost_focus() {
                    let v = text.trim().to_string();
                    self.desc_edit = None;
                    self.set_project_meta(&ws, json!({ "description": v }));
                }
            } else {
                let empty = desc.trim().is_empty();
                let shown = if empty { t("Add a short description…") } else { desc.clone() };
                let r = widgets::text(ui, pos2(tx, head.top() + 57.0), Align2::LEFT_CENTER, &shown, theme::ui(14.0), if empty { p.text3 } else { p.text2 }, inner_w - 420.0);
                let resp = ui.interact(r.expand(4.0), ui.id().with("proj-desc"), Sense::click()).on_hover_cursor(egui::CursorIcon::Text).on_hover_text(t("Click to edit"));
                if resp.hovered() {
                    ui.painter().rect_stroke(r.expand(4.0), CornerRadius::same(6), Stroke::new(1.0, p.line), StrokeKind::Inside);
                }
                if resp.clicked() {
                    self.desc_edit = Some((ws.clone(), desc, true));
                }
            }
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
                    .max_rect(Rect::from_min_max(pos2(left + inner_w - 520.0, head.top() + 6.0), pos2(left + inner_w, head.top() + 40.0)))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            acts.spacing_mut().item_spacing.x = 6.0;
            // ako acts v app.js: web → Open page · Live preview, ostatné → Open main file · Run it; potom New file a 🔍
            if widgets::icon_button(&mut acts, "search", &p, true).on_hover_text(format!("{} (Ctrl+P)", t("Find file"))).clicked() {
                self.act("quick-open", ui.ctx());
            }
            let has_main = main_file.is_some();
            let (nf_bg, nf_fg) = if has_main { (p.card2, p.text) } else { (p.accent, p.accent_fg) };
            if widgets::button(&mut acts, Some("filePlus"), &t("New file"), nf_bg, nf_fg, 32.0, &p).clicked() {
                self.open_new_file(None);
            }
            if let Some((name, path)) = &main_file {
                let web = name.ends_with(".html");
                if web {
                    // Live preview = Live Server (stránka sa znova načíta pri uložení)
                    if widgets::button(&mut acts, Some("monitor"), &t("Live preview"), p.card2, p.text, 32.0, &p)
                        .on_hover_text(t("Open the page in the browser – it reloads when you save (F5)"))
                        .clicked()
                    {
                        self.open_file(path);
                        self.live_server();
                    }
                    if widgets::button(&mut acts, Some("globe"), &t("Open page"), p.accent, p.accent_fg, 32.0, &p).on_hover_text(name).clicked() {
                        settings::open_external(path);
                    }
                } else {
                    if widgets::button(&mut acts, Some("play"), &t("Run it"), p.card2, p.text, 32.0, &p).on_hover_text(name).clicked() {
                        self.open_file(path);
                        self.run();
                    }
                    if widgets::button(&mut acts, Some("code"), &t("Open main file"), p.accent, p.accent_fg, 32.0, &p).on_hover_text(name).clicked() {
                        self.open_file(path);
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
                ("refresh", summary["last"].as_u64().filter(|l| *l > 0).map(ago).unwrap_or("\u{2013}".into()), "last change"),
            ];
            let (row, _) = ui.allocate_exact_size(vec2(full, 84.0), Sense::hover());
            let tw = (inner_w - 4.0 * 11.0) / 5.0;
            for (i, (ic, val, label)) in tiles.iter().enumerate() {
                let t = Rect::from_min_size(pos2(left + i as f32 * (tw + 11.0), row.top()), vec2(tw, 83.0));
                // dlaždica sa pri prejdení myšou jemne nadvihne
                let tresp = ui.interact(t, ui.id().with(("tile", i)), Sense::hover());
                let hk = ui.ctx().animate_bool_with_time(tresp.id, tresp.hovered(), 0.15);
                let t = t.translate(vec2(0.0, -2.0 * hk));
                if hk > 0.0 {
                    ui.painter().add(egui::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha((60.0 * hk) as u8) }.as_shape(t, CornerRadius::same(12)));
                }
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
                    let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
                    if hk > 0.0 {
                        ui.painter().rect_filled(r, CornerRadius::same(8), p.hover.gamma_multiply(hk));
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
        self.smooth.end(sk, &sout);
    }

    // ---------- karta: Výstup / Terminál ----------
    fn bottom_panel(&mut self, ui: &mut egui::Ui, rect: Rect, pos: &str) {
        let p = self.pal;
        match pos {
            "right" => ui.painter().vline(rect.left(), rect.y_range(), Stroke::new(1.0, p.line)),
            "left" => ui.painter().vline(rect.right(), rect.y_range(), Stroke::new(1.0, p.line)),
            _ => ui.painter().hline(rect.x_range(), rect.top(), Stroke::new(1.0, p.line)),
        };
        // pilulky Output | Terminal
        let gw = 4.0 + [t("Output"), t("Terminal")].iter().map(|l| widgets::text_w(ui, l, theme::bold(12.5)) + 22.0).sum::<f32>();
        let group = Rect::from_min_size(pos2(rect.left() + 15.0, rect.top() + 7.0), vec2(gw, 26.0));
        ui.painter().rect_filled(group, CornerRadius::same(8), p.card2);
        let mut x = group.left() + 2.0;
        for (b, label) in [(Bottom::Output, "Output"), (Bottom::Terminal, "Terminal")] {
            let w = widgets::text_w(ui, &t(label), theme::bold(12.5)) + 22.0;
            let r = Rect::from_min_size(pos2(x, group.top() + 2.0), vec2(w, 22.0));
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
        // vývojárske počítadlo: snímky za sekundu (pri nečinnosti má byť ~0)
        if self.developer() && self.get("devFps").as_bool() == Some(true) {
            let r = widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, &format!("{} fps", self.frames.len()), theme::mono(11.0), p.text3, 80.0);
            x = r.right() + 18.0;
        }
        // hlásenie o úspechu po 4 s zmizne
        if !self.status.is_empty() && self.status == self.ok_status {
            match self.ok_at.map(|t| t.elapsed()) {
                Some(e) if e >= Duration::from_secs(4) => {
                    self.status.clear();
                    self.ok_status.clear();
                }
                Some(e) => ui.ctx().request_repaint_after(Duration::from_secs(4) - e),
                None => {}
            }
        }
        if !self.status.is_empty() {
            widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, &self.status, small.clone(), if self.status == self.ok_status { p.green } else { p.red }, (rect.right() - x - 320.0).max(40.0));
        }
        // vpravo: aktualizácia (#st-update), Ln/Col, slová, Auto save, jazyk
        let mut rx = rect.right() - 16.0;
        use crate::update::State as U;
        let chip = match self.upd.state() {
            U::Available { .. } => Some((t("Update available"), false)),
            U::Downloading { got, total } => Some((crate::i18n::tf("Updating {n} %", &[("n", &(got * 100 / total.max(1)).min(100).to_string())]), false)),
            U::Ready { .. } => Some((t("Restart to update"), true)),
            _ => None,
        };
        if let Some((label, ready)) = chip {
            let w = widgets::text_w(ui, &label, theme::bold(11.5)) + 30.0;
            let cr = Rect::from_min_size(pos2(rx - w, cy - 10.0), vec2(w, 20.0));
            let resp = ui.interact(cr, ui.id().with("st-update"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            ui.painter().rect_filled(cr, CornerRadius::same(10), if resp.hovered() { p.accent.lerp_to_gamma(Color32::WHITE, 0.1) } else { p.accent });
            widgets::icon_at(ui, pos2(cr.left() + 12.0, cy), 11.0, "download", p.accent_fg);
            ui.painter().text(pos2(cr.left() + 22.0, cy), Align2::LEFT_CENTER, &label, theme::bold(11.5), p.accent_fg);
            if resp.clicked() {
                if ready {
                    self.restart_to_update(ui.ctx());
                } else {
                    self.open_settings("general", ui.ctx());
                    self.upd.download(ui.ctx());
                }
            }
            rx = cr.left() - 16.0;
        }
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
        let (sets, panel, side, dark) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::J),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::B),
                i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::L),
            )
        });
        // ďalšie skratky z Electron Fluxu
        let more = ctx.input_mut(|i| {
            let c = egui::Modifiers::COMMAND;
            let cs = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
            [
                (i.consume_key(c, egui::Key::P), "quick-open"),
                (i.consume_key(c, egui::Key::I), "ai"),
                (i.consume_key(cs, egui::Key::A), "search"),
                (i.consume_key(cs, egui::Key::P), "commands"),
                (i.consume_key(cs, egui::Key::N), "new-project"),
                (i.consume_key(c, egui::Key::N), "new-file"),
                (i.consume_key(cs, egui::Key::S), "save-all"),
                (i.consume_key(egui::Modifiers::SHIFT, egui::Key::F5), "stop"),
                (i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::ALT, egui::Key::O), "open-file"),
            ]
        });
        for (hit, id) in more {
            if hit {
                self.act(id, ctx);
            }
        }
        if sets {
            if self.settings.is_some() {
                self.settings = None;
            } else {
                self.open_settings("general", ctx);
            }
        }
        if panel {
            self.panel_open = !self.panel_open;
        }
        if side {
            self.side_open = !self.side_open;
        }
        if dark {
            self.set_theme(!self.pal.dark, ctx);
        }
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
    // ako ďaleko zájde jedno otočenie kolieska (Vzhľad → Okno → Rýchlosť posúvania); Ctrl+koliesko = zoom ostáva
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        let k = (self.core.setting("scrollSpeed").as_f64().unwrap_or(100.0) / 100.0).clamp(0.5, 3.0) as f32;
        if (k - 1.0).abs() < 0.01 {
            return;
        }
        for e in raw.events.iter_mut() {
            if let egui::Event::MouseWheel { delta, modifiers, .. } = e {
                if !modifiers.ctrl && !modifiers.command {
                    *delta *= k;
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        crate::boot::closed();
    }

    // priehľadné okno: kde Flux nič nenakreslí, presvitá plocha (živá tapeta)
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // FLUX_TR_OPAQUE=1: priehľadné okno, ale nepriehľadné pozadie (diagnostika v CI)
        if crate::TRANSPARENT.load(std::sync::atomic::Ordering::Relaxed) && std::env::var("FLUX_TR_OPAQUE").is_err() {
            [0.0, 0.0, 0.0, 0.0]
        } else {
            visuals.panel_fill.to_normalized_gamma_f32()
        }
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let ctx = &ctx;
        self.events();
        self.run_tests(ctx);
        // tapeta pod všetkým (panely sú nad ňou priesvitné)
        if self.wall.paint(ctx) {
            self.apply_look(ctx);
        }
        // pamäť na pozadí (trimMemory, predvolene zapnuté)
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        // aktualizácie: pri štarte a potom každých 6 h; autoUpdate = hneď stiahnuť (inak len ponúknuť)
        let now_t = ctx.input(|i| i.time);
        if self.intro.is_none() && now_t - self.upd.last_check > 3600.0 {
            let auto = self.get("autoUpdate").as_bool() != Some(false);
            self.upd.check(ctx, now_t, auto);
        }
        // tichá aktualizácia (autoUpdate): stiahnutá verzia sa nasadí, keď je Flux 1 min v pozadí a nič nerobí
        // (žiadne neuložené súbory, bežiaci program, Live Server ani odpoveď AI); nový Flux nezoberie fokus
        if focused {
            self.unfocused_at = None;
        } else if self.unfocused_at.is_none() {
            self.unfocused_at = Some(Instant::now());
        }
        if matches!(self.upd.state(), crate::update::State::Ready { .. })
            && self.get("autoUpdate").as_bool() != Some(false)
            && self.unfocused_at.is_some_and(|t| t.elapsed() > Duration::from_secs(60))
            && !self.running
            && self.server.is_none()
            && !self.tabs.iter().any(|t| t.dirty())
            && !self.ai.shared.lock().unwrap().running
            && self.settings.is_none()
            && self.new_project.is_none()
            && self.new_file.is_none()
        {
            let extra: &[&str] = if minimized { &["--background", "--minimized"] } else { &["--background"] };
            if self.upd.install_with(true, extra).is_ok() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if !focused {
            ctx.request_repaint_after(Duration::from_secs(20));
        }
        // pri zavretí s pripravenou aktualizáciou sa program len vymení (bez nového štartu)
        if ctx.input(|i| i.viewport().close_requested()) {
            let _ = self.upd.install(false);
        }
        // živá tapeta: pri štarte a potom pri zameranom okne
        if focused || self.live_at < 0.0 {
            self.check_live(ctx, ctx.input(|i| i.time));
        }
        if self.sync_live_cache() {
            self.apply_look(ctx);
        }
        // zdroj tapety (súbory, tapeta Windows, živá tapeta) stačí zistiť raz za 3 s, nie pri každom snímku
        let now_w = ctx.input(|i| i.time);
        // prvé sekundy častejšie (kým dobehne zisťovanie živej tapety), potom raz za 3 s
        let every = if self.started.elapsed() < Duration::from_secs(4) { 0.25 } else { 3.0 };
        if now_w - self.wall_at > every {
            self.wall_at = now_w;
            let src = self.wall_source();
            self.wall.want(ctx, src);
        }
        self.wall.set_playing(focused && !minimized && self.core.setting("optAnim").as_bool().unwrap_or(self.get("lite").as_bool() != Some(true)));
        if let Some(d) = self.trim.tick(focused, self.core.setting("trimMemory").as_bool() != Some(false)) {
            ctx.request_repaint_after(d);
        }
        if self.intro.is_some() {
            let r = root.max_rect();
            egui::CentralPanel::default().frame(Frame::new().fill(self.pal.base)).show(root, |ui| {
                chrome::drag_area(ui, Rect::from_min_size(r.min, vec2(r.width(), TOP_H)), "intro");
                self.intro_ui(ui, r)
            });
            self.window_chrome(ctx);
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
        self.tour_rects.clear();
        // po štarte (načítanie písma, tapety, zvýraznenia) raz uvoľniť nepotrebnú pamäť
        // Live Server počas písania: neuložený text najviac každých 120 ms
        if let (Some(s), Some(tab), Some(edit)) = (&self.server, self.tabs.get(self.active), self.last_edit) {
            let web = matches!(tab.ext().as_str(), "html" | "htm" | "css" | "js");
            let due = self.live_push.is_none_or(|p| p < edit);
            if web && due && tab.dirty() {
                if self.live_push.is_none_or(|p| p.elapsed() >= Duration::from_millis(120)) {
                    s.set_live(&tab.path, &tab.text);
                    self.live_push = Some(Instant::now());
                } else {
                    ctx.request_repaint_after(Duration::from_millis(120));
                }
            }
        }
        // výsledky GitHubu (prihlásenie, import, nový repozitár) aj keď okná nie sú otvorené
        self.gh_poll(&ctx);
        // Flux beží a kreslí → ďalší štart normálny (boot.rs)
        if !self.boot_ok && self.started.elapsed() > Duration::from_secs(3) {
            self.boot_ok = true;
            crate::boot::ok();
        } else if !self.boot_ok {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        if !self.start_trimmed && self.started.elapsed() > Duration::from_secs(10) {
            self.start_trimmed = true;
            // nová verzia beží → predošlá (.old) už netreba
            crate::update::cleanup(false);
            if self.core.setting("trimMemory").as_bool() != Some(false) {
                crate::mem::trim_now();
            }
        } else if !self.start_trimmed {
            ctx.request_repaint_after(Duration::from_secs(10).saturating_sub(self.started.elapsed()));
        }
        // rozpis textúr pre vývojára (len keď sú otvorené nastavenia)
        if self.settings.is_some() && self.developer() && ctx.input(|i| i.time).fract() < 0.05 {
            let tm = ctx.tex_manager();
            let tm = tm.read();
            let (mut n, mut bytes, mut big) = (0, 0usize, vec![]);
            for (_, m) in tm.allocated() {
                let b = m.size[0] * m.size[1] * m.bytes_per_pixel;
                n += 1;
                bytes += b;
                if b > 2 << 20 {
                    big.push(format!("{} {}×{}", m.name, m.size[0], m.size[1]));
                }
            }
            self.tex_info = format!("{n} · {:.1} MB{}", bytes as f64 / 1048576.0, if big.is_empty() { String::new() } else { format!(" ({})", big.join(", ")) });
        }
        // vývojárske počítadlo snímok: časy za poslednú sekundu
        let t_now = ctx.input(|i| i.time);
        self.frames.push_back(t_now);
        while self.frames.front().is_some_and(|f| t_now - f > 1.0) {
            self.frames.pop_front();
        }
        // domov Fluxu namiesto bočného panela a karty
        if self.start {
            egui::CentralPanel::default().frame(Frame::new()).show(root, |ui| {
                let r = ui.max_rect();
                self.start_ui(ui, r);
            });
        }
        if self.side_open && !self.start {
            self.sidebar(root);
        }
        let p = self.pal;
        let radius = self.radius();
        let side_right = self.get("sidePos").as_str() == Some("right");
        let now = ctx.input(|i| i.time);
        // panel Live Servera sleduje aktívnu stránku
        if self.preview.is_some() && self.is_web_file() {
            if let Some(u) = self.live_url() {
                if let Some(pv) = self.preview.as_mut() {
                    pv.url = u;
                }
            }
        }
        if !self.start {
            egui::CentralPanel::default().frame(Frame::new().fill(p.base)).show(root, |ui| {
                let full = ui.max_rect();
                let (lpad, rpad) = if !self.side_open {
                    (GAP, GAP)
                } else if side_right {
                    (GAP, 0.0)
                } else {
                    (0.0, GAP)
                };
                let top = Rect::from_min_max(pos2(full.left() + if self.side_open && !side_right { 0.0 } else { GAP }, full.top()), pos2(full.right() - rpad, full.top() + TOP_H));
                self.top_bar(ui, top);
                // jedna zaoblená karta: editor/stránka projektu, Výstup/Terminál, stavový riadok (#card)
                let card_r = Rect::from_min_max(pos2(full.left() + lpad, top.bottom()), pos2(full.right() - rpad, full.bottom() - GAP));
                ui.painter().add(egui::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(if p.dark { 110 } else { 25 }) }.as_shape(card_r, CornerRadius::same(radius)));
                // pod kartou rozmazaná tapeta (ako backdrop-filter) – priesvitná karta potom neruší textom tapety
                self.wall.paint_blurred(ui.painter(), card_r, radius);
                ui.painter().rect_filled(card_r, CornerRadius::same(radius), p.card);
                ui.painter().rect_stroke(card_r, CornerRadius::same(radius), Stroke::new(1.0, p.line), StrokeKind::Outside);
                let mut card_ui = ui.new_child(egui::UiBuilder::new().max_rect(card_r));
                card_ui.set_clip_rect(card_r.shrink(1.0));
                let status = Rect::from_min_max(pos2(card_r.left(), card_r.bottom() - 28.0), card_r.max);
                self.tour_rects.insert("status", status);
                self.status_bar(&mut card_ui, status);
                let mut main = Rect::from_min_max(card_r.min, pos2(card_r.right(), status.top()));
                // Live Server vpravo na karte (šírka sa dá ťahať)
                if self.preview.is_some() {
                    let pw = {
                        let pv = self.preview.as_mut().unwrap();
                        if pv.width <= 0.0 {
                            pv.width = (main.width() * 0.45).round();
                        }
                        pv.width = pv.width.clamp(280.0, (main.width() - 320.0).max(280.0));
                        pv.width
                    };
                    let pr = Rect::from_min_max(pos2(main.right() - pw, main.top()), main.max);
                    let grip = Rect::from_min_max(pos2(pr.left() - 3.0, pr.top()), pos2(pr.left() + 3.0, pr.bottom()));
                    let gr = card_ui.interact(grip, card_ui.id().with("pv-grip"), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeColumn);
                    if gr.dragged() {
                        if let Some(pv) = self.preview.as_mut() {
                            pv.width = (pv.width - gr.drag_delta().x).max(280.0);
                        }
                    }
                    self.preview_ui(&mut card_ui, pr);
                    main.max.x = pr.left();
                }
                // Claude vpravo na karte (šírka sa dá ťahať)
                if self.ai.open {
                    if self.ai.width <= 0.0 {
                        self.ai.width = 380.0;
                    }
                    self.ai.width = self.ai.width.clamp(300.0, (main.width() - 320.0).max(300.0));
                    let ar = Rect::from_min_max(pos2(main.right() - self.ai.width, main.top()), main.max);
                    let grip = Rect::from_min_max(pos2(ar.left() - 3.0, ar.top()), pos2(ar.left() + 3.0, ar.bottom()));
                    let gr = card_ui.interact(grip, card_ui.id().with("ai-grip"), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeColumn);
                    if gr.dragged() {
                        self.ai.width = (self.ai.width - gr.drag_delta().x).max(300.0);
                    }
                    self.ai_panel(&mut card_ui, ar);
                    main.max.x = ar.left();
                }
                let show_editor = !self.home && !self.tabs.is_empty();
                let pos = self.get("panelPos").as_str().unwrap_or("bottom").to_string();
                if self.panel_open && show_editor {
                    // Výstup/Terminál dole, vpravo alebo vľavo (panelPos) s ťahadlom
                    let panel = if pos == "bottom" {
                        let h = self.panel_h.clamp(90.0, (main.height() - 120.0).max(90.0));
                        Rect::from_min_max(pos2(main.left(), main.bottom() - h), main.max)
                    } else {
                        let w = self.panel_w.clamp(220.0, (main.width() - 300.0).max(220.0));
                        if pos == "right" {
                            Rect::from_min_max(pos2(main.right() - w, main.top()), main.max)
                        } else {
                            Rect::from_min_max(main.min, pos2(main.left() + w, main.bottom()))
                        }
                    };
                    let grip = match pos.as_str() {
                        "bottom" => Rect::from_min_max(pos2(panel.left(), panel.top() - 3.0), pos2(panel.right(), panel.top() + 3.0)),
                        "right" => Rect::from_min_max(pos2(panel.left() - 3.0, panel.top()), pos2(panel.left() + 3.0, panel.bottom())),
                        _ => Rect::from_min_max(pos2(panel.right() - 3.0, panel.top()), pos2(panel.right() + 3.0, panel.bottom())),
                    };
                    let gr =
                        card_ui.interact(grip, card_ui.id().with("grip"), Sense::drag()).on_hover_cursor(if pos == "bottom" { egui::CursorIcon::ResizeRow } else { egui::CursorIcon::ResizeColumn });
                    if gr.dragged() {
                        let d = gr.drag_delta();
                        match pos.as_str() {
                            "bottom" => self.panel_h = (panel.height() - d.y).max(90.0),
                            "right" => self.panel_w = (panel.width() - d.x).max(220.0),
                            _ => self.panel_w = (panel.width() + d.x).max(220.0),
                        }
                    }
                    self.bottom_panel(&mut card_ui, panel, &pos);
                    match pos.as_str() {
                        "bottom" => main.max.y = panel.top(),
                        "right" => main.max.x = panel.left(),
                        _ => main.min.x = panel.right(),
                    }
                } else if show_editor {
                    // skrytý panel: malé tlačidlo na jeho vrátenie
                    let b = Rect::from_center_size(pos2(main.right() - 136.0, main.bottom() - 18.0), vec2(26.0, 26.0));
                    if widgets::icon_button_at(&mut card_ui, b, "panel", 15.0, &p, true).on_hover_text(t("Show Output")).clicked() {
                        self.panel_open = true;
                    }
                }
                // prechod pri zmene súboru/stránky: obsah jemne „vybledne“ dnu (transitions)
                let key = if show_editor { self.tabs[self.active].path.clone() } else { format!("home:{:?}", self.workspace()) };
                if key != self.shown {
                    self.shown = key;
                    self.switched = now;
                }
                // späť/dopredu: obsah prichádza zo strany (24 px, doznie s prelínaním)
                let mut area = main;
                if self.anim_on() && self.nav_dir != 0.0 {
                    let k = ((now - self.switched) / 0.24).clamp(0.0, 1.0) as f32;
                    if k < 1.0 {
                        area = main.translate(vec2(self.nav_dir * 28.0 * (1.0 - k) * (1.0 - k), 0.0));
                    } else {
                        self.nav_dir = 0.0;
                    }
                }
                if show_editor {
                    self.editor(&mut card_ui, area);
                } else {
                    self.project_page(&mut card_ui, area);
                }
                if self.anim_on() {
                    let k = ((now - self.switched) / 0.24).clamp(0.0, 1.0) as f32;
                    if k < 1.0 {
                        card_ui.painter().rect_filled(main.shrink(1.0), 0.0, p.card.gamma_multiply(1.0 - k * k));
                        ctx.request_repaint();
                    }
                }
            });
        }
        // paleta a potvrdenie nad všetkým
        if self.palette.is_some() {
            let full = ctx.content_rect();
            egui::Area::new(egui::Id::new("palette-layer")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
                ui.set_min_size(full.size());
                self.palette_ui(ui, full);
            });
        }
        // pokus so sklom: potvrdiť, že Flux vidno (inak sa main.rs po 25 s vráti sám)
        if crate::TRANSPARENT.load(std::sync::atomic::Ordering::Relaxed) && !crate::GLASS_OK.load(std::sync::atomic::Ordering::Relaxed) && self.core.setting("glassTrial").as_str() == Some("running") {
            let full = ctx.content_rect();
            egui::Area::new(egui::Id::new("glass-layer")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
                ui.set_min_size(full.size());
                self.glass_ui(ui, full);
            });
        }
        if self.ask.is_some() {
            let full = ctx.content_rect();
            egui::Area::new(egui::Id::new("ask-layer")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
                ui.set_min_size(full.size());
                self.ask_ui(ui, full);
            });
        }
        // okno Nový súbor
        if self.new_file.is_some() {
            let full = ctx.content_rect();
            egui::Area::new(egui::Id::new("newfile-layer")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
                ui.set_min_size(full.size());
                self.new_file_ui(ui, full);
            });
        }
        // okno Nový projekt
        if self.new_project.is_some() {
            let full = ctx.content_rect();
            egui::Area::new(egui::Id::new("newproj-layer")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
                ui.set_min_size(full.size());
                self.new_project_ui(ui, full);
            });
        }
        // Nastavenia nad všetkým (vlastná vrstva)
        if self.settings.is_some() {
            let full = ctx.content_rect();
            egui::Area::new(egui::Id::new("settings-layer")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
                ui.set_min_size(full.size());
                self.settings_ui(ui, full);
            });
        }
        // prehliadka funkcií nad všetkým
        if self.tour.is_some() {
            let full = ctx.content_rect();
            egui::Area::new(egui::Id::new("tour-layer")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
                ui.set_min_size(full.size());
                self.tour_ui(ui, full);
            });
        }
        self.window_chrome(ctx);
        // späť/dopredu: tlačidlá myši 4/5 a Alt+←/→ na každej obrazovke (nie v intre)
        if self.intro.is_none() {
            let (b, f) = ctx.input_mut(|i| {
                (
                    // ako prehliadač: až pri pustení tlačidla myši
                    i.pointer.button_released(egui::PointerButton::Extra1) || i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft),
                    i.pointer.button_released(egui::PointerButton::Extra2) || i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowRight),
                )
            });
            if b {
                self.go(true);
            } else if f {
                self.go(false);
            }
            self.track_place();
        }
        // vložený WebView2 na mieste panela Live Servera
        self.sync_preview(ctx, _frame);
    }
}
