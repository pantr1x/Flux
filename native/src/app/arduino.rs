// Arduino panel vpravo na karte (pre .ino súbory): doska na USB (aj Elegoo / klony s CH340), nahratie, sériový monitor,
// schéma zapojenia podľa pinov v kóde, knižnice a firmvér. Logika je v crate::arduino, príkazy bežia vo Výstupe.
use super::App;
use crate::arduino::{self as ard, Board, BAUDS, BOARDS};
use crate::i18n::{t, tf};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, PartialEq)]
pub enum Inst {
    Idle,
    Downloading,
    Failed(String),
    Done(String), // cesta k arduino-cli → v ďalšej snímke sa vo Výstupe nainštaluje jadro dosiek
}

pub struct ArdUi {
    pub open: bool,
    pub width: f32,
    pub seen: HashSet<String>, // .ino, pri ktorých sa panel už raz sám otvoril
    pub section: u8,           // 0 doska, 1 zapojenie, 2 knižnice, 3 firmvér
    boards: Arc<Mutex<Vec<Board>>>,
    scanning: Arc<AtomicBool>,
    last_scan: Option<Instant>,
    cli: Option<String>,
    cli_at: Option<Instant>,
    inst: Arc<Mutex<Inst>>,
    pub port: Option<String>,
    lib_q: String,
    lib_res: Arc<Mutex<Option<Vec<(String, String, String)>>>>,
    lib_busy: Arc<AtomicBool>,
    installed: Arc<Mutex<Option<Vec<(String, String)>>>>,
}

impl Default for ArdUi {
    fn default() -> Self {
        ArdUi {
            open: false,
            width: 0.0,
            seen: HashSet::new(),
            section: 0,
            boards: Arc::new(Mutex::new(vec![])),
            scanning: Arc::new(AtomicBool::new(false)),
            last_scan: None,
            cli: None,
            cli_at: None,
            inst: Arc::new(Mutex::new(Inst::Idle)),
            port: None,
            lib_q: String::new(),
            lib_res: Arc::new(Mutex::new(None)),
            lib_busy: Arc::new(AtomicBool::new(false)),
            installed: Arc::new(Mutex::new(None)),
        }
    }
}

// farba a text súčiastky pre režim pinu
fn mode_info(mode: &str) -> (Color32, &'static str, &'static str) {
    match mode {
        "out" => (Color32::from_rgb(0xff, 0x8a, 0x3d), "OUTPUT", "LED (long leg) here through a 220 Ω resistor; short leg → GND"),
        "pwm" => (Color32::from_rgb(0xf5, 0xc2, 0x42), "PWM", "LED or motor driver – brightness / speed with analogWrite()"),
        "in" => (Color32::from_rgb(0x4d, 0xa3, 0xff), "INPUT", "Button: one leg here and through 10 kΩ to GND, the other leg → 5V"),
        "pull" => (Color32::from_rgb(0x4d, 0xa3, 0xff), "INPUT_PULLUP", "Button: one leg here, the other leg → GND (no resistor needed)"),
        "analog" => (Color32::from_rgb(0x3d, 0xd6, 0x8c), "ANALOG", "Potentiometer or sensor: middle leg here, outer legs → 5V and GND"),
        "servo" => (Color32::from_rgb(0xc0, 0x7b, 0xff), "SERVO", "Servo signal wire (orange / yellow); red → 5V, brown → GND"),
        "tone" => (Color32::from_rgb(0xff, 0x5c, 0x8a), "TONE", "Buzzer (+) here, (−) → GND"),
        _ => (Color32::GRAY, "", ""),
    }
}

impl App {
    pub(super) fn ard_board(&self) -> String {
        self.get("arduinoBoard").as_str().map(String::from).unwrap_or_else(|| BOARDS[0].0.to_string())
    }

    fn ard_baud(&self) -> u32 {
        self.get("arduinoBaud").as_u64().map(|b| b as u32).unwrap_or(9600)
    }

    fn ard_cli(&mut self) -> Option<String> {
        if self.ard.cli_at.is_none_or(|at| at.elapsed() > Duration::from_secs(5)) {
            self.ard.cli = ard::cli();
            self.ard.cli_at = Some(Instant::now());
        }
        self.ard.cli.clone()
    }

    // dosky na USB: každé 2 s na pozadí (arduino-cli, ak je; inak porty systému)
    fn ard_scan(&mut self, ctx: &egui::Context) {
        if self.ard.scanning.load(Ordering::Relaxed) || self.ard.last_scan.is_some_and(|at| at.elapsed() < Duration::from_secs(2)) {
            return;
        }
        self.ard.last_scan = Some(Instant::now());
        self.ard.scanning.store(true, Ordering::Relaxed);
        let (slot, flag, cli, ctx) = (self.ard.boards.clone(), self.ard.scanning.clone(), self.ard_cli(), ctx.clone());
        std::thread::spawn(move || {
            let mut list = cli.as_deref().and_then(ard::list_boards).unwrap_or_default();
            // arduino-cli niekedy klon nevypíše – doplniť porty zo systému
            for b in ard::scan_ports() {
                if !list.iter().any(|x| x.port == b.port) {
                    list.push(b);
                }
            }
            *slot.lock().unwrap() = list;
            flag.store(false, Ordering::Relaxed);
            ctx.request_repaint();
        });
    }

    // príkazy vo Výstupe aktívnej karty
    fn ard_run(&mut self, label: &str, cmds: Vec<Vec<String>>) {
        let cmd = match ard::script(&cmds) {
            Ok(c) => c,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        self.panel_open = true;
        let base = self.emit.clone();
        let cwd = self.tabs.get(self.active).and_then(|t| std::path::Path::new(&t.path).parent().map(|d| d.to_string_lossy().to_string())).unwrap_or_default();
        let Some(s) = self.sess_mut() else { return };
        s.bottom = super::Bottom::Output;
        s.out.clear();
        flux_core::runner::start(&s.out.pty, &s.emit(&base), &cmd, &cwd, label);
        self.focus_out = true;
    }

    // zvolený port: ručne vybraný, inak prvá nájdená doska
    fn ard_port(&self) -> Option<String> {
        let list = self.ard.boards.lock().unwrap();
        self.ard.port.clone().filter(|p| list.iter().any(|b| &b.port == p)).or_else(|| list.first().map(|b| b.port.clone()))
    }

    fn ard_ready(&mut self) -> Option<(String, String, String)> {
        let Some(cli) = self.ard_cli() else {
            self.ard.open = true;
            self.ard.section = 0;
            self.status = t("Install the Arduino tools first (Arduino panel → Install).");
            return None;
        };
        let Some(port) = self.ard_port() else {
            self.ard.open = true;
            self.status = t("No board found – plug it in with a USB cable.");
            return None;
        };
        Some((cli, port, self.ard_board()))
    }

    // kompilácia (+ nahratie) sketchu z dir; jadro dosky sa doinštaluje, ak chýba
    fn ard_compile(&mut self, label: &str, dir: &std::path::Path, upload: bool, pre: Vec<Vec<String>>) {
        let cli = match self.ard_cli() {
            Some(c) => c,
            None => {
                self.status = t("Install the Arduino tools first (Arduino panel → Install).");
                return;
            }
        };
        let fqbn = self.ard_board();
        let port = if upload {
            match self.ard_ready() {
                Some((_, p, _)) => Some(p),
                None => return,
            }
        } else {
            None
        };
        let (data, _) = ard::dirs(&cli);
        let mut cmds = pre;
        if !ard::core_installed(&data, &fqbn) {
            cmds.push(ard::v(&cli, &["core", "update-index"]));
            cmds.push(ard::v(&cli, &["core", "install", &ard::core_of(&fqbn)]));
        }
        let d = dir.to_string_lossy().to_string();
        match &port {
            Some(p) => cmds.push(ard::v(&cli, &["compile", "--upload", "-b", &fqbn, "-p", p, &d])),
            None => cmds.push(ard::v(&cli, &["compile", "-b", &fqbn, &d])),
        }
        self.ard_run(label, cmds);
    }

    // ▶ (F5) pri .ino: skompilovať a nahrať do dosky
    pub(super) fn ard_upload(&mut self, upload: bool) {
        let Some(path) = self.tabs.get(self.active).map(|t| t.path.clone()) else { return };
        match ard::sketch_dir(&path) {
            Ok(dir) => {
                let name = widgets::file_name(&path);
                let label = if upload { tf("Uploading {name}", &[("name", &name)]) } else { tf("Checking {name}", &[("name", &name)]) };
                self.ard_compile(&label, &dir, upload, vec![]);
            }
            Err(e) => self.status = e,
        }
    }

    fn ard_monitor(&mut self) {
        let Some((cli, port, _)) = self.ard_ready() else { return };
        let baud = self.ard_baud().to_string();
        self.ard_run(
            &tf("Serial monitor {port} ({baud} baud) – type to send, Stop to close", &[("port", &port), ("baud", &baud)]),
            vec![ard::v(&cli, &["monitor", "-p", &port, "-c", &format!("baudrate={baud}")])],
        );
    }

    fn ard_install_tools(&mut self, ctx: &egui::Context) {
        let slot = self.ard.inst.clone();
        *slot.lock().unwrap() = Inst::Downloading;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = ard::install_cli();
            *slot.lock().unwrap() = match r {
                Ok(p) => Inst::Done(p),
                Err(e) => Inst::Failed(e),
            };
            ctx.request_repaint();
        });
    }

    fn ard_firmware(&mut self, kind: &str) {
        let Some(cli) = self.ard_cli() else { return };
        match kind {
            "blink" | "empty" => match ard::write_sketch(if kind == "blink" { "FluxBlink" } else { "FluxEmpty" }, if kind == "blink" { ard::BLINK } else { ard::EMPTY }) {
                Ok(dir) => self.ard_compile(&t(if kind == "blink" { "Uploading the Blink test" } else { "Uploading an empty program" }), &dir, true, vec![]),
                Err(e) => self.status = e,
            },
            "firmata" => {
                let (_, user) = ard::dirs(&cli);
                let dir = user.join("libraries").join("Firmata").join("examples").join("StandardFirmata");
                self.ard_compile(&t("Uploading StandardFirmata"), &dir, true, vec![ard::v(&cli, &["lib", "install", "Firmata"])]);
            }
            _ => {}
        }
    }

    // ---------- panel ----------
    pub(super) fn ard_panel(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        // stiahnuté arduino-cli → jadro dosiek (AVR = Uno, Mega, Nano, Elegoo…) vo Výstupe
        let done = match &*self.ard.inst.lock().unwrap() {
            Inst::Done(path) => Some(path.clone()),
            _ => None,
        };
        if let Some(cli) = done {
            *self.ard.inst.lock().unwrap() = Inst::Idle;
            self.ard.cli = Some(cli.clone());
            self.ard.cli_at = Some(Instant::now());
            self.ard_run(&t("Installing the Arduino board package (AVR: Uno, Mega, Nano, Elegoo…)"), vec![ard::v(&cli, &["core", "update-index"]), ard::v(&cli, &["core", "install", "arduino:avr"])]);
        }
        self.ard_scan(&ctx);
        ctx.request_repaint_after(Duration::from_millis(1000));
        ui.painter().vline(r.left(), r.y_range(), Stroke::new(1.0, p.line));
        let head = Rect::from_min_size(r.min, vec2(r.width(), 44.0));
        widgets::icon_at(ui, pos2(head.left() + 20.0, head.center().y), 15.0, "chip", p.accent);
        ui.painter().text(pos2(head.left() + 34.0, head.center().y), Align2::LEFT_CENTER, "Arduino", theme::bold(14.0), p.text);
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(head.right() - 22.0, head.center().y), vec2(28.0, 28.0)), "x", 14.0, &p, true).on_hover_text(t("Close")).clicked() {
            self.ard.open = false;
        }
        ui.painter().hline(r.x_range(), head.bottom(), Stroke::new(1.0, p.line));
        // sekcie
        let tabs_r = Rect::from_min_size(pos2(r.left() + 12.0, head.bottom() + 8.0), vec2(r.width() - 24.0, 30.0));
        let names = [t("Board"), t("Wiring"), t("Libraries"), t("Firmware")];
        let tw = tabs_r.width() / 4.0;
        for (i, n) in names.iter().enumerate() {
            let tr = Rect::from_min_size(pos2(tabs_r.left() + i as f32 * tw, tabs_r.top()), vec2(tw - 4.0, 28.0));
            let resp = ui.interact(tr, ui.id().with(("ard-tab", i)), Sense::click());
            let on = self.ard.section == i as u8;
            if on || resp.hovered() {
                ui.painter().rect_filled(tr, CornerRadius::same(8), if on { p.active } else { p.hover });
            }
            widgets::text(ui, tr.center(), Align2::CENTER_CENTER, n, if on { theme::bold(12.5) } else { theme::ui(12.5) }, if on { p.text } else { p.text2 }, tw - 8.0);
            if resp.clicked() {
                self.ard.section = i as u8;
            }
        }
        let body = Rect::from_min_max(pos2(r.left(), tabs_r.bottom() + 6.0), r.max);
        let mut bui = ui.new_child(egui::UiBuilder::new().max_rect(body.shrink2(vec2(14.0, 4.0))));
        bui.set_clip_rect(body);
        egui::ScrollArea::vertical().id_salt("ard-body").auto_shrink(false).show(&mut bui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            match self.ard.section {
                0 => self.ard_board_ui(ui, &ctx),
                1 => self.ard_wiring_ui(ui),
                2 => self.ard_libs_ui(ui, &ctx),
                _ => self.ard_firmware_ui(ui),
            }
            ui.add_space(12.0);
        });
    }

    fn ard_hint(&self, ui: &mut egui::Ui, text: &str) {
        let p = self.pal;
        ui.add(egui::Label::new(egui::RichText::new(text).font(theme::ui(12.0)).color(p.text3)).wrap());
    }

    fn ard_card(&self, ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
        let p = self.pal;
        egui::Frame::NONE.fill(p.hover).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    }

    fn ard_board_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = self.pal;
        let w = ui.available_width();
        // nástroje
        let cli = self.ard_cli();
        let inst = self.ard.inst.lock().unwrap().clone();
        if cli.is_none() {
            let mut go = false;
            self.ard_card(ui, |ui| {
                ui.label(egui::RichText::new(t("Arduino tools are not installed")).font(theme::bold(13.5)).color(p.text));
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(t(
                            "Flux downloads the official arduino-cli (about 25 MB) and the board package for Uno, Mega, Nano and Elegoo boards (about 150 MB). Nothing else is needed.",
                        ))
                        .font(theme::ui(12.0))
                        .color(p.text2),
                    )
                    .wrap(),
                );
                match &inst {
                    Inst::Downloading => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(egui::RichText::new(t("Downloading arduino-cli…")).font(theme::ui(12.5)).color(p.text2));
                        });
                    }
                    _ => {
                        if let Inst::Failed(e) = &inst {
                            ui.label(egui::RichText::new(e).font(theme::ui(12.0)).color(p.red));
                        }
                        go = widgets::button(ui, Some("download"), &t("Install Arduino tools"), p.accent, p.accent_fg, 30.0, &p).clicked();
                    }
                }
            });
            if go {
                self.ard_install_tools(ctx);
            }
        }
        // dosky
        ui.label(egui::RichText::new(t("CONNECTED BOARDS")).font(theme::bold(11.0)).color(p.text3));
        let boards = self.ard.boards.lock().unwrap().clone();
        let cur = self.ard_port();
        if boards.is_empty() {
            self.ard_card(ui, |ui| {
                ui.label(egui::RichText::new(t("No board found")).font(theme::bold(13.0)).color(p.text));
                let hint = if cfg!(windows) {
                    t("Plug the board in with a USB cable (a data cable, not only charging). Elegoo and other clones use the CH340 chip – if no COM port appears, install its driver from wch-ic.com.")
                } else {
                    t("Plug the board in with a USB cable (a data cable, not only charging). It shows up as /dev/ttyACM0 or /dev/ttyUSB0 (Elegoo and other clones with the CH340 chip).")
                };
                ui.add(egui::Label::new(egui::RichText::new(hint).font(theme::ui(12.0)).color(p.text2)).wrap());
            });
        }
        let mut pick: Option<(String, Option<String>)> = None;
        for b in &boards {
            let on = cur.as_deref() == Some(b.port.as_str());
            let (rr, resp) = ui.allocate_exact_size(vec2(w, 46.0), Sense::click());
            ui.painter().rect_filled(
                rr,
                CornerRadius::same(10),
                if on {
                    p.active
                } else if resp.hovered() {
                    p.hover
                } else {
                    Color32::TRANSPARENT
                },
            );
            if on {
                ui.painter().rect_stroke(rr, CornerRadius::same(10), Stroke::new(1.0, p.accent), StrokeKind::Inside);
            }
            ui.painter().circle_filled(pos2(rr.left() + 16.0, rr.center().y), 4.5, p.green);
            widgets::text(ui, pos2(rr.left() + 30.0, rr.top() + 15.0), Align2::LEFT_CENTER, &b.name, theme::bold(13.0), p.text, w - 40.0);
            let sub = match b.chip {
                Some(c) => format!("{}  ·  {c}", b.port),
                None => b.port.clone(),
            };
            widgets::text(ui, pos2(rr.left() + 30.0, rr.top() + 32.0), Align2::LEFT_CENTER, &sub, theme::mono(11.5), p.text3, w - 40.0);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                pick = Some((b.port.clone(), b.fqbn.clone()));
            }
        }
        if let Some((port, fqbn)) = pick {
            self.ard.port = Some(port);
            // rozpoznaný typ (Uno, Mega…) nastaviť; pri klone (CH340) nechať, čo si vybral
            if let Some(f) = fqbn.filter(|f| f != "arduino:avr:uno" || !self.get("arduinoBoard").is_string()) {
                self.update_settings(|o| {
                    o.insert("arduinoBoard".into(), json!(f));
                });
            }
        }
        // prístup k portu (Linux): skupina uucp / dialout
        if let Some(port) = cur.clone() {
            if let Some(group) = ard::port_access(&port) {
                let cmd = format!("sudo usermod -aG {group} $USER");
                let mut copy = false;
                self.ard_card(ui, |ui| {
                    ui.label(egui::RichText::new(tf("No permission for {port}", &[("port", &port)])).font(theme::bold(13.0)).color(p.red));
                    ui.add(
                        egui::Label::new(egui::RichText::new(tf("Add yourself to the {group} group once, then log out and back in:", &[("group", &group)])).font(theme::ui(12.0)).color(p.text2))
                            .wrap(),
                    );
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&cmd).font(theme::mono(12.0)).color(p.text));
                        copy = widgets::icon_button(ui, "copy", &p, true).on_hover_text(t("Copy")).clicked();
                    });
                });
                if copy {
                    ui.ctx().copy_text(cmd);
                }
            }
        }
        // typ dosky
        ui.add_space(4.0);
        ui.label(egui::RichText::new(t("BOARD TYPE")).font(theme::bold(11.0)).color(p.text3));
        let fq = self.ard_board();
        let opts: Vec<String> = BOARDS.iter().map(|b| t(b.1)).collect();
        let cur_label = BOARDS.iter().find(|b| b.0 == fq).map(|b| t(b.1)).unwrap_or(fq.clone());
        if let Some(i) = widgets::select(ui, egui::Id::new("ard-board"), &cur_label, &opts, w, &p) {
            let f = BOARDS[i].0;
            self.update_settings(|o| {
                o.insert("arduinoBoard".into(), json!(f));
            });
        }
        if boards.iter().any(|b| b.chip == Some("CH340")) {
            self.ard_hint(ui, &t("Clone boards (CH340) can't tell which board they are – pick it here. Nano clones usually need “old bootloader”."));
        }
        // akcie
        ui.add_space(4.0);
        let mut act: Option<&str> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            if widgets::button(ui, Some("upload"), &t("Upload"), p.accent, p.accent_fg, 30.0, &p).on_hover_text(t("Compile and upload to the board (F5)")).clicked() {
                act = Some("upload");
            }
            if widgets::button(ui, Some("check"), &t("Verify"), p.card2, p.text, 30.0, &p).on_hover_text(t("Only compile – find errors without a board")).clicked() {
                act = Some("verify");
            }
        });
        ui.add_space(4.0);
        ui.label(egui::RichText::new(t("SERIAL MONITOR")).font(theme::bold(11.0)).color(p.text3));
        let baud = self.ard_baud();
        let bopts: Vec<String> = BAUDS.iter().map(|b| format!("{b} baud")).collect();
        ui.horizontal(|ui| {
            if let Some(i) = widgets::select(ui, egui::Id::new("ard-baud"), &format!("{baud} baud"), &bopts, (w - 110.0).max(120.0), &p) {
                let b = BAUDS[i];
                self.update_settings(|o| {
                    o.insert("arduinoBaud".into(), json!(b));
                });
            }
            if widgets::button(ui, Some("terminal"), &t("Open"), p.card2, p.text, 32.0, &p).on_hover_text(t("Show what the board prints with Serial.println()")).clicked() {
                act = Some("monitor");
            }
        });
        self.ard_hint(ui, &t("Use the same speed as Serial.begin() in your code. Close the monitor (Stop) before uploading."));
        match act {
            Some("upload") => self.ard_upload(true),
            Some("verify") => self.ard_upload(false),
            Some("monitor") => self.ard_monitor(),
            _ => {}
        }
    }

    // schéma: doska Uno, piny použité v kóde (číslo v krúžku) a čo k nim pripojiť
    fn ard_wiring_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.pal;
        let code = self.tabs.get(self.active).map(|t| t.text.clone()).unwrap_or_default();
        let pins = ard::pins_in(&code);
        let w = ui.available_width();
        let h = w * 0.74 + 24.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let pa = ui.painter_at(rect);
        let board = Rect::from_min_size(rect.min + vec2(4.0, 44.0), vec2(w - 8.0, h - 88.0));
        let teal = Color32::from_rgb(0x00, 0x87, 0x8f);
        pa.rect_filled(board, CornerRadius::same(10), teal);
        pa.rect_stroke(board, CornerRadius::same(10), Stroke::new(1.0, Color32::from_black_alpha(80)), StrokeKind::Inside);
        // USB, napájanie, čip, nápis
        let bw = board.width();
        let bh = board.height();
        let at = |fx: f32, fy: f32| pos2(board.left() + bw * fx, board.top() + bh * fy);
        pa.rect_filled(Rect::from_min_max(at(-0.03, 0.12), at(0.13, 0.36)), CornerRadius::same(3), Color32::from_gray(190));
        pa.rect_filled(Rect::from_min_max(at(-0.02, 0.66), at(0.12, 0.86)), CornerRadius::same(3), Color32::from_gray(30));
        pa.rect_filled(Rect::from_min_max(at(0.40, 0.58), at(0.80, 0.74)), CornerRadius::same(2), Color32::from_gray(25));
        pa.text(at(0.30, 0.42), Align2::CENTER_CENTER, "UNO", theme::bold((bh * 0.11).clamp(10.0, 22.0)), Color32::from_white_alpha(220));
        pa.circle_filled(at(0.205, 0.22), 4.0, Color32::from_rgb(0xff, 0xb3, 0x47)); // vstavaná LED (pin 13)
                                                                                     // hlavičky pinov: hore digitálne, dole napájanie + analógové
        let top: Vec<&str> = vec!["SCL", "SDA", "AREF", "GND", "13", "12", "11", "10", "9", "8", "", "7", "6", "5", "4", "3", "2", "1", "0"];
        let bottom: Vec<&str> = vec!["", "IOREF", "RST", "3V3", "5V", "GND", "GND", "VIN", "", "A0", "A1", "A2", "A3", "A4", "A5"];
        let used = |name: &str| pins.iter().position(|x| x.pin == name);
        let power = pins.iter().any(|x| matches!(x.mode, "in" | "analog" | "servo"));
        let draw_row = |labels: &[&str], y: f32, x0: f32, x1: f32, up: bool| {
            let n = labels.len() as f32;
            for (i, l) in labels.iter().enumerate() {
                if l.is_empty() {
                    continue;
                }
                let x = board.left() + bw * (x0 + (x1 - x0) * i as f32 / (n - 1.0));
                let c = pos2(x, board.top() + bh * y);
                let s = (bw / 48.0).clamp(4.0, 8.0);
                pa.rect_filled(Rect::from_center_size(c, vec2(s, s)), 1.0, Color32::from_gray(20));
                let ty = if up { c.y + s + 6.0 } else { c.y - s - 6.0 };
                // úzky panel: dlhé popisy skrátiť (GND → G), nech sa neprekrývajú
                let step = bw * (x1 - x0) / (n - 1.0);
                let short = match *l {
                    "SCL" | "SDA" | "AREF" | "IOREF" | "RST" | "VIN" if step < 22.0 => "",
                    "GND" if step < 22.0 => "G",
                    "3V3" if step < 22.0 => "3V",
                    other => other,
                };
                pa.text(pos2(x, ty), Align2::CENTER_CENTER, short, theme::ui((step * 0.62).clamp(7.0, 10.0)), Color32::from_white_alpha(200));
                let idx = used(l);
                let pwr = power && (*l == "5V" || *l == "GND");
                if let Some(k) = idx {
                    let (col, _, _) = mode_info(pins[k].mode);
                    // čiara von z dosky a krúžok s číslom
                    // susedné piny: krúžky striedavo vyššie/nižšie, nech sa neprekrývajú
                    let lift = if i % 2 == 1 { 20.0 } else { 0.0 };
                    let out = pos2(x, if up { rect.top() + 10.0 + lift } else { rect.bottom() - 10.0 - lift });
                    pa.line_segment([c, out], Stroke::new(2.0, col));
                    pa.circle_filled(out, 9.0, col);
                    pa.text(out, Align2::CENTER_CENTER, (k + 1).to_string(), theme::bold(10.5), Color32::BLACK);
                    pa.circle_stroke(c, s * 0.9, Stroke::new(2.0, col));
                } else if pwr {
                    pa.circle_stroke(c, s * 0.9, Stroke::new(1.5, if *l == "5V" { Color32::from_rgb(0xff, 0x5c, 0x5c) } else { Color32::from_gray(230) }));
                }
            }
        };
        draw_row(&top, 0.07, 0.30, 0.97, true);
        draw_row(&bottom, 0.93, 0.40, 0.97, false);
        let fq = self.ard_board();
        if fq.contains("mega") {
            self.ard_hint(ui, &t("The picture shows the Uno layout – pins 0–13 and A0–A5 are in the same places on the Mega; other pins are listed below."));
        } else if fq.contains("nano") || fq.contains("pro") {
            self.ard_hint(ui, &t("The picture shows the Uno layout – the Nano and Pro Mini have the same pin numbers, printed next to each pin."));
        }
        if pins.is_empty() {
            self.ard_card(ui, |ui| {
                ui.label(egui::RichText::new(t("No pins used yet")).font(theme::bold(13.0)).color(p.text));
                ui.add(egui::Label::new(egui::RichText::new(t("Use pinMode(), digitalWrite(), analogRead()… in your code and the wiring appears here.")).font(theme::ui(12.0)).color(p.text2)).wrap());
            });
            return;
        }
        let mut jump: Option<usize> = None;
        for (k, pin) in pins.iter().enumerate() {
            let (col, mode, part) = mode_info(pin.mode);
            let resp = egui::Frame::NONE
                .fill(p.hover)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (cr, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                        ui.painter().circle_filled(cr.center(), 9.0, col);
                        ui.painter().text(cr.center(), Align2::CENTER_CENTER, (k + 1).to_string(), theme::bold(10.5), Color32::BLACK);
                        let title = match &pin.name {
                            Some(n) => format!("{} {}  ·  {n}", t("Pin"), pin.pin),
                            None => format!("{} {}", t("Pin"), pin.pin),
                        };
                        ui.label(egui::RichText::new(title).font(theme::bold(13.0)).color(p.text));
                        ui.label(egui::RichText::new(mode).font(theme::mono(11.0)).color(col));
                    });
                    ui.add(egui::Label::new(egui::RichText::new(t(part)).font(theme::ui(12.0)).color(p.text2)).wrap());
                })
                .response;
            if ui.interact(resp.rect, ui.id().with(("ard-pin", k)), Sense::click()).on_hover_text(t("Show in code")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                jump = Some(pin.line);
            }
        }
        if let Some(l) = jump {
            self.lint_jump = Some(l);
        }
    }

    fn ard_libs_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = self.pal;
        let w = ui.available_width();
        let Some(cli) = self.ard_cli() else {
            self.ard_hint(ui, &t("Install the Arduino tools first (Board tab)."));
            return;
        };
        let te = egui::Frame::NONE
            .fill(p.card2)
            .stroke(Stroke::new(1.0, p.line_strong))
            .corner_radius(CornerRadius::same(9))
            .inner_margin(egui::Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.ard.lib_q)
                        .id(egui::Id::new("ard-lib-q"))
                        .hint_text(t("Search libraries – e.g. Servo, DHT, NeoPixel"))
                        .frame(egui::Frame::NONE)
                        .desired_width(w - 20.0)
                        .font(theme::ui(13.0)),
                )
            })
            .inner;
        let busy = self.ard.lib_busy.load(Ordering::Relaxed);
        if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.ard.lib_q.trim().is_empty() && !busy {
            let (q, slot, flag, c2, ctx2) = (self.ard.lib_q.trim().to_string(), self.ard.lib_res.clone(), self.ard.lib_busy.clone(), cli.clone(), ctx.clone());
            flag.store(true, Ordering::Relaxed);
            std::thread::spawn(move || {
                let r = ard::lib_search(&c2, &q);
                *slot.lock().unwrap() = Some(r);
                flag.store(false, Ordering::Relaxed);
                ctx2.request_repaint();
            });
        }
        // nainštalované (raz pri otvorení; po inštalácii znova)
        if self.ard.installed.lock().unwrap().is_none() && !busy {
            let (slot, c2) = (self.ard.installed.clone(), cli.clone());
            *slot.lock().unwrap() = Some(vec![]);
            std::thread::spawn(move || {
                let r = ard::lib_installed(&c2);
                *slot.lock().unwrap() = Some(r);
            });
        }
        if busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(egui::RichText::new(t("Searching…")).font(theme::ui(12.5)).color(p.text2));
            });
        }
        let installed = self.ard.installed.lock().unwrap().clone().unwrap_or_default();
        let res = self.ard.lib_res.lock().unwrap().clone();
        let mut install: Option<String> = None;
        if let Some(list) = res {
            if list.is_empty() && !busy {
                self.ard_hint(ui, &t("Nothing found."));
            }
            for (name, ver, sentence) in list {
                let have = installed.iter().any(|x| x.0 == name);
                self.ard_card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&name).font(theme::bold(13.0)).color(p.text));
                        ui.label(egui::RichText::new(&ver).font(theme::ui(11.5)).color(p.text3));
                    });
                    if !sentence.is_empty() {
                        ui.add(egui::Label::new(egui::RichText::new(&sentence).font(theme::ui(12.0)).color(p.text2)).wrap());
                    }
                    if have {
                        ui.label(egui::RichText::new(format!("✓ {}", t("Installed"))).font(theme::ui(12.0)).color(p.green));
                    } else if widgets::button(ui, Some("download"), &t("Install"), p.card2, p.text, 28.0, &p).clicked() {
                        install = Some(name.clone());
                    }
                });
            }
        }
        if !installed.is_empty() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(t("INSTALLED")).font(theme::bold(11.0)).color(p.text3));
            for (n, v) in &installed {
                ui.label(egui::RichText::new(format!("{n}  {v}")).font(theme::ui(12.5)).color(p.text2));
            }
        }
        if let Some(name) = install {
            self.ard_run(&tf("Installing the {name} library", &[("name", &name)]), vec![ard::v(&cli, &["lib", "install", &name])]);
            *self.ard.installed.lock().unwrap() = None;
        }
    }

    fn ard_firmware_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.pal;
        if self.ard_cli().is_none() {
            self.ard_hint(ui, &t("Install the Arduino tools first (Board tab)."));
            return;
        }
        let items = [
            ("blink", "Test the board (Blink)", "The built-in LED blinks once a second and the board says hello in the serial monitor – a quick check that upload works."),
            ("firmata", "StandardFirmata", "Firmware for controlling the board from your computer: Python (pyFirmata), Node.js (Johnny-Five) or Processing – no Arduino code needed."),
            ("empty", "Empty program", "Stops whatever the board is doing (motors, LEDs, sounds)."),
        ];
        let mut go: Option<&str> = None;
        for (id, title, desc) in items {
            self.ard_card(ui, |ui| {
                ui.label(egui::RichText::new(t(title)).font(theme::bold(13.0)).color(p.text));
                ui.add(egui::Label::new(egui::RichText::new(t(desc)).font(theme::ui(12.0)).color(p.text2)).wrap());
                if widgets::button(ui, Some("upload"), &t("Upload"), p.card2, p.text, 28.0, &p).clicked() {
                    go = Some(id);
                }
            });
        }
        self.ard_hint(ui, &t("Uploading replaces the program on the board. Your sketch stays in Flux – upload it again with F5."));
        if let Some(id) = go {
            self.ard_firmware(id);
        }
        let _ = Value::Null;
    }
}
