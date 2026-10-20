// Arduino panel vpravo na karte (pre .ino súbory): doska na USB (aj Elegoo / klony s CH340), nahratie, sériový monitor,
// schéma zapojenia podľa pinov v kóde, knižnice a firmvér. Logika je v crate::arduino, príkazy bežia vo Výstupe.
use super::boardart::{self, ESP_L, ESP_R, NANO_L, NANO_R, PICO_L, PICO_R};
use super::dash;
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

// nástroje pre Python na doske (zisťujú sa na pozadí každých 10 s)
#[derive(Clone, Default)]
pub struct PyTools {
    mp: Option<Vec<String>>, // mpremote
    es: Option<Vec<String>>, // esptool
    serial: bool,            // pyserial (konzola CircuitPython)
    firmata: bool,           // pyfirmata2
}

pub struct ArdUi {
    pub open: bool,
    tools: Arc<Mutex<PyTools>>,
    last_tools: Option<Instant>,
    esp: usize,           // čip pre nahratie MicroPythonu
    pub setup_open: bool, // okno „Getting started“
    setup_mode: String,
    setup_h: f32,
    guide_due: Option<Instant>, // kedy sa po prvom otvorení panela posúdi, či ukázať sprievodcu
    guide_checked: bool,
    scroll_top: bool,
    sketch: Option<(String, Instant, Sketch)>,                 // piny celého sketchu (kľúč, kedy)
    proj: Option<(String, Instant, Option<(String, String)>)>, // stránka projektu: (priečinok, kedy, (súbor dosky, jazyk))
    flipped: HashSet<&'static str>,                            // sekcie, ktoré používateľ prepol oproti predvolenému stavu
    undo: Vec<(String, String)>,                               // naposledy odobraný pin: (súbor, pôvodný text)
    undo_pin: String,
    pub zoom: f32,     // priblíženie dosky (Ctrl + koleso)
    pan: egui::Vec2,   // posun priblíženej dosky (px)
    dlg_new: bool,     // v okne pinu: vytvoriť nový súbor namiesto pridania do otvoreného
    pub pin_dlg: bool, // okno „nový súbor pre pin“
    dlg_mode: String,
    dlg_kind: String, // predvoľba (prázdne = prázdny súbor)
    dlg_h: f32,
    ctx: (String, String, String),               // priečinok, jazyk a súbor, pre ktorý sa kreslí doska
    plain: bool,                                 // bez rámikov (hlavná stránka projektu)
    new_name: String,                            // meno nového súboru z vlastného kódu (prázdne = pin_<n>)
    sel: Option<String>,                         // pin vybraný kliknutím v schéme
    bt: Arc<Mutex<Vec<(String, String, bool)>>>, // spárované Bluetooth zariadenia (meno, MAC, pripojené)
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
            tools: Arc::new(Mutex::new(PyTools::default())),
            last_tools: None,
            esp: 0,
            setup_open: false,
            setup_mode: String::new(),
            setup_h: 420.0,
            guide_due: None,
            guide_checked: false,
            scroll_top: false,
            sketch: None,
            proj: None,
            flipped: HashSet::new(),
            undo: vec![],
            undo_pin: String::new(),
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            dlg_new: false,
            pin_dlg: false,
            dlg_mode: String::new(),
            dlg_kind: String::new(),
            dlg_h: 400.0,
            ctx: (String::new(), String::new(), String::new()),
            plain: false,
            new_name: String::new(),
            sel: None,
            bt: Arc::new(Mutex::new(vec![])),
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

// jeden krok prípravy (nástroje, doska, práva k portu…)
struct Row {
    ok: bool,
    title: String,
    detail: String,
    red: bool,
    busy: bool,
    act: Option<(&'static str, String)>,
    copy: Option<String>,
}

// farba a text súčiastky pre režim pinu
// sketch = všetky .ino v priečinku (Arduino ich pri kompilácii spojí): piny s kontrolou, kde začína ktorý súbor, knižnice
#[derive(Clone, Default)]
pub(super) struct Sketch {
    pub pins: Vec<ard::Pin>,
    pub files: Vec<(usize, String)>, // (prvý riadok v spojenom texte, cesta)
    pub includes: Vec<(String, Option<String>)>,
}

// text problému s pinom (pwm = zoznam PWM pinov dosky, ak je známy)
pub(super) fn warn_text(kind: &str, pin: &str, pwm: &str) -> String {
    match kind {
        "unused" => tf("Pin {pin} is set up with pinMode() but nothing in the code uses it.", &[("pin", pin)]),
        "missing" => tf("Pin {pin} doesn't exist on this board.", &[("pin", pin)]),
        "nopwm" if !pwm.is_empty() => tf("analogWrite() on pin {pin} only switches it on or off – use a PWM pin: {list}.", &[("pin", pin), ("list", pwm)]),
        "nopwm" => tf("analogWrite() on pin {pin} only switches it on or off – use a PWM pin (~).", &[("pin", pin)]),
        "noanalog" => tf("analogRead() needs an analog pin (A0, A1…) – pin {pin} is digital.", &[("pin", pin)]),
        "analogonly" => tf("{pin} on this board can only be read with analogRead().", &[("pin", pin)]),
        "serial" => tf("Pin {pin} carries the USB connection (Serial) – uploading and the Serial Monitor can fail. Use another pin.", &[("pin", pin)]),
        "servo_pwm" => tf("With the Servo library, analogWrite() on pin {pin} stops working – use 3, 5, 6 or 11.", &[("pin", pin)]),
        "tone_pwm" => tf("tone() stops analogWrite() on pin {pin} – use another PWM pin.", &[("pin", pin)]),
        _ => String::new(),
    }
}

// čo kúpiť / zapojiť pre súčiastku
fn part_label(part: &str) -> String {
    t(match part {
        "led" => "LED + 220 Ω resistor",
        "button" => "Push button",
        "sensor" => "Potentiometer or analog sensor",
        "servo" => "Servo motor",
        "buzzer" => "Piezo buzzer",
        "neo" => "NeoPixel LED strip",
        "dht" => "DHT11 / DHT22 sensor + 10 kΩ resistor",
        "onewire" => "DS18B20 sensor + 4.7 kΩ resistor",
        "lcd" => "16×2 LCD + 10 kΩ potentiometer",
        "serial" => "Serial module (Bluetooth HC-05, GPS…)",
        _ => "",
    })
}

// výber dosky z dizajnu: dva riadky (model, čip + režim) a šípka
fn dash_select(ui: &mut egui::Ui, id: egui::Id, title: &str, sub: &str, options: &[String], width: f32, p: &theme::Pal) -> Option<usize> {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 54.0), Sense::click());
    let hk = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered(), 0.12);
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
    ui.painter().rect_filled(r, CornerRadius::same(10), dash::field().lerp_to_gamma(p.hover, hk));
    let (w, col) = if open || resp.has_focus() { (2.0, dash::cyan()) } else { (1.0, if hk > 0.0 { p.line_strong } else { dash::border() }) };
    ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(w, col), StrokeKind::Inside);
    widgets::text(ui, pos2(r.left() + 14.0, r.top() + 19.0), Align2::LEFT_CENTER, title, theme::bold(13.5), dash::text(), width - 54.0);
    widgets::text(ui, pos2(r.left() + 14.0, r.top() + 37.0), Align2::LEFT_CENTER, sub, theme::ui(12.0), dash::text3(), width - 54.0);
    let c = pos2(r.right() - 20.0, r.center().y);
    ui.painter().line(vec![pos2(c.x - 4.5, c.y - 2.0), pos2(c.x, c.y + 2.5), pos2(c.x + 4.5, c.y - 2.0)], Stroke::new(1.8, dash::text2()));
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let mut pick = None;
    egui::Popup::menu(&resp).width(width).show(|ui| {
        ui.set_min_width(width - 12.0);
        for (i, o) in options.iter().enumerate() {
            let (rr, rs) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
            if rs.hovered() {
                ui.painter().rect_filled(rr, CornerRadius::same(6), p.hover);
            }
            widgets::text(ui, pos2(rr.left() + 10.0, rr.center().y), Align2::LEFT_CENTER, o, theme::ui(13.0), p.text, rr.width() - 20.0);
            if rs.clicked() {
                pick = Some(i);
            }
        }
    });
    pick
}

pub(super) fn mode_info(mode: &str) -> (Color32, &'static str, &'static str) {
    match mode {
        "out" => (dash::cyan(), "OUTPUT", "LED (long leg) here through a 220 Ω resistor; short leg → GND"),
        "pwm" => (Color32::from_rgb(0xf5, 0xc2, 0x42), "PWM", "LED or motor driver – brightness / speed with analogWrite()"),
        "in" => (Color32::from_rgb(0x4d, 0xa3, 0xff), "INPUT", "Button: one leg here and through 10 kΩ to GND, the other leg → 5V"),
        "pull" => (Color32::from_rgb(0x4d, 0xa3, 0xff), "INPUT_PULLUP", "Button: one leg here, the other leg → GND (no resistor needed)"),
        "analog" => (Color32::from_rgb(0x3d, 0xd6, 0x8c), "ANALOG", "Potentiometer or sensor: middle leg here, outer legs → 5V and GND"),
        "servo" => (Color32::from_rgb(0xc0, 0x7b, 0xff), "SERVO", "Servo signal wire (orange / yellow); red → 5V, brown → GND"),
        "tone" => (Color32::from_rgb(0xff, 0x5c, 0x8a), "TONE", "Buzzer (+) here, (−) → GND"),
        "read" => (Color32::from_rgb(0x4d, 0xa3, 0xff), "READ", "Button: one leg here and through 10 kΩ to GND, the other leg → 5V"),
        "neo" => (Color32::from_rgb(0xff, 0x7a, 0xd9), "NEOPIXEL", "LED strip data (DIN) here through 330 Ω; + → 5V, − → GND"),
        "dht" => (Color32::from_rgb(0x5e, 0xd4, 0xc8), "DHT", "DHT sensor data pin here (10 kΩ to 5V); + → 5V, − → GND"),
        "onewire" => (Color32::from_rgb(0x5e, 0xd4, 0xc8), "1-WIRE", "DS18B20 data pin here with 4.7 kΩ to 5V; + → 5V, − → GND"),
        "lcd" => (Color32::from_rgb(0x9d, 0xb4, 0xff), "LCD", "One of the LCD wires (RS, E, D4–D7) – follow the order in your code"),
        "serial" => (Color32::from_rgb(0xff, 0xb8, 0x6b), "SERIAL", "RX / TX of a module (Bluetooth, GPS…) – cross them: module TX → this RX"),
        _ => (Color32::GRAY, "", ""),
    }
}

impl App {
    // typ dosky: ručne zvolený, alebo (automaticky) pripojená doska; klon s CH340 sa nedá rozpoznať → uložená voľba
    pub(super) fn ard_board(&self) -> String {
        let saved = self.get("arduinoBoard").as_str().map(String::from).unwrap_or_else(|| BOARDS[0].0.to_string());
        if self.get("arduinoBoardAuto").as_bool() == Some(false) {
            return saved;
        }
        let port = self.ard_port();
        let boards = self.ard.boards.lock().unwrap();
        boards.iter().find(|b| Some(&b.port) == port.as_ref()).filter(|b| b.chip != Some("CH340")).and_then(|b| b.fqbn.clone()).unwrap_or(saved)
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
        let do_tools = self.ard.last_tools.is_none_or(|at| at.elapsed() > Duration::from_secs(10));
        if do_tools {
            self.ard.last_tools = Some(Instant::now());
        }
        let tools = self.ard.tools.clone();
        let bt = self.ard.bt.clone();
        let (slot, flag, cli, ctx) = (self.ard.boards.clone(), self.ard.scanning.clone(), self.ard_cli(), ctx.clone());
        std::thread::spawn(move || {
            if do_tools {
                let t = PyTools { mp: ard::find_tool("mpremote"), es: ard::find_tool("esptool"), serial: ard::py_has("serial"), firmata: ard::py_has("pyfirmata2") };
                *tools.lock().unwrap() = t;
                *bt.lock().unwrap() = ard::bt_devices();
            }
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
            self.ard_need_setup();
            self.status = t("Install the Arduino tools first (Arduino panel → Install).");
            return None;
        };
        let Some(port) = self.ard_port() else {
            self.ard_need_setup();
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
                self.ard_need_setup();
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
            let extra: Vec<&str> = ard::extra_url(&fqbn).map(|u| vec!["--additional-urls", u]).unwrap_or_default();
            let (mut upd, mut inst) = (ard::v(&cli, &["core", "update-index"]), ard::v(&cli, &["core", "install", &ard::core_of(&fqbn)]));
            upd.extend(extra.iter().map(|s| s.to_string()));
            inst.extend(extra.iter().map(|s| s.to_string()));
            cmds.push(upd);
            cmds.push(inst);
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

    // doska z arduino-cli (Uno, Mega, Nano…): Python sa tam nahráva ako preložené C++
    pub(super) fn ard_cpp_board(&self) -> bool {
        self.ard_board().starts_with("arduino:")
    }

    // je pre tento jazyk a dosku k dispozícii kompilácia (Verify) / nahratie cez C++?
    pub(super) fn ard_can_build(&self, mode: &str) -> bool {
        mode == "cpp" || mode == "firmata" || self.ard_cpp_board()
    }

    // nahrať (upload) alebo len skontrolovať (verify) program otvoreného súboru: C++ priamo, Python po preklade do C++
    pub(super) fn ard_build(&mut self, upload: bool) {
        match self.ard_mode().as_deref() {
            Some("cpp") => self.ard_upload(upload),
            Some(_) => self.ard_py_build(upload),
            None => {}
        }
    }

    // Python → C++ (src/pytoc.rs); chyba ukáže riadok v Pythone
    fn ard_translate(&mut self) -> Option<(String, String)> {
        let tab = self.tabs.get(self.active)?;
        let (path, text) = (tab.path.clone(), tab.text.clone());
        let name = widgets::file_name(&path);
        match crate::pytoc::translate(&text, &name) {
            Ok(cpp) => Some((path, cpp)),
            Err(e) => {
                let n = e.strip_prefix("line ").and_then(|r| r.split(':').next()).and_then(|n| n.trim().parse::<usize>().ok());
                if let Some(n) = n {
                    self.lint_jump = Some(n.saturating_sub(1));
                }
                let msg = tf("Can't convert {name} to C++ – {err}", &[("name", &name), ("err", &e)]);
                if let Some(s) = self.sess_mut() {
                    s.out.feed(&format!("\r\n\x1b[31m{msg}\x1b[0m\r\n"));
                }
                self.status = msg;
                None
            }
        }
    }

    fn ard_py_build(&mut self, upload: bool) {
        if !self.ard_cpp_board() && self.ard_mode().as_deref() != Some("firmata") {
            self.status = t("Pick an Arduino board first (Board select).");
            return;
        }
        let Some((path, cpp)) = self.ard_translate() else { return };
        let stem = std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "sketch".into());
        let dir = flux_core::settings::user_data().join("arduino").join("sketches").join(&stem);
        let _ = std::fs::remove_dir_all(&dir);
        if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join(format!("{stem}.ino")), cpp)) {
            self.status = e.to_string();
            return;
        }
        let name = widgets::file_name(&path);
        let label = if upload { tf("Converting {name} to C++ and uploading", &[("name", &name)]) } else { tf("Converting {name} to C++ and checking", &[("name", &name)]) };
        self.ard_compile(&label, &dir, upload, vec![]);
    }

    // otvorí preložený C++ kód, aby bolo vidieť, čo sa nahrá
    fn ard_show_cpp(&mut self) {
        let Some((path, cpp)) = self.ard_translate() else { return };
        let stem = std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "sketch".into());
        let dir = flux_core::settings::user_data().join("arduino").join("sketches").join(format!("{stem}-preview"));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join(format!("{stem}.ino"));
        if std::fs::write(&f, cpp).is_ok() {
            let fp = f.to_string_lossy().to_string();
            flux_core::fsops::allow_file(&self.core, &fp);
            self.open_file(&fp);
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

    // stav nástrojov pre intro: 0 nie sú, 1 sťahujú sa, 2 pripravené
    pub(super) fn ard_tools_state(&mut self) -> u8 {
        if self.ard_cli().is_some() {
            2
        } else if matches!(*self.ard.inst.lock().unwrap(), Inst::Downloading) {
            1
        } else {
            0
        }
    }

    pub(super) fn ard_install_tools(&mut self, ctx: &egui::Context) {
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

    // jazyk aktívneho súboru: .ino = C++, .py podľa importov (alebo ručne zvolený); inak žiadna doska
    pub(super) fn ard_mode(&self) -> Option<String> {
        let tab = self.tabs.get(self.active)?;
        match tab.ext().as_str() {
            "ino" => Some("cpp".into()),
            "py" => ard::guess_py(&tab.text).map(String::from),
            _ => None,
        }
    }

    // Python + Firmata: je nainštalovaný pyFirmata2 a nájdená doska? Inak otvorí Getting started a povie, čo chýba
    pub(super) fn ard_firmata_ready(&mut self) -> bool {
        if !self.ard.tools.lock().unwrap().firmata {
            self.ard_need_setup();
            self.status = t("Install pyFirmata2 first (board panel → Install).");
            return false;
        }
        if self.ard_port().is_none() {
            self.ard_need_setup();
            self.status = t("No board found – plug it in with a USB cable.");
            return false;
        }
        true
    }

    fn ard_cp_copy(&mut self, path: &str) {
        self.status = match ard::circuitpy_drive() {
            Some(d) => match std::fs::copy(path, d.join("code.py")) {
                Ok(_) => tf("Copied to {drive} as code.py – the board restarts by itself", &[("drive", &d.to_string_lossy())]),
                Err(e) => e.to_string(),
            },
            None => {
                self.ard_need_setup();
                t("The CIRCUITPY drive was not found – plug the board in and wait a moment.")
            }
        };
    }

    // ▶ (F5) pri Pythone na doske: MicroPython `mpremote run` (save = uložiť ako main.py), CircuitPython skopíruje súbor ako code.py
    pub(super) fn ard_py_run(&mut self, save: bool) {
        let Some(path) = self.tabs.get(self.active).map(|t| t.path.clone()) else { return };
        if self.ard_mode().as_deref() == Some("circuitpython") {
            return self.ard_cp_copy(&path);
        }
        let Some(mp) = self.ard.tools.lock().unwrap().mp.clone() else {
            self.ard_need_setup();
            self.status = t("Install mpremote first (board panel → Install).");
            return;
        };
        let Some(port) = self.ard_port() else {
            self.ard_need_setup();
            self.status = t("No board found – plug it in with a USB cable.");
            return;
        };
        let name = widgets::file_name(&path);
        let mut cmd = mp;
        cmd.extend(["connect".to_string(), port]);
        if save {
            cmd.extend(["cp".into(), path, ":main.py".into(), "+".into(), "reset".into()]);
            self.ard_run(&tf("Saving {name} on the board as main.py", &[("name", &name)]), vec![cmd]);
        } else {
            cmd.extend(["run".into(), path]);
            self.ard_run(&tf("Running {name} on the board (Stop = Ctrl+C)", &[("name", &name)]), vec![cmd]);
        }
    }

    fn ard_install_py(&mut self, label: &str, pkg: &str, app: bool) {
        self.ard.last_tools = None;
        self.ard_run(label, vec![ard::pip_install(pkg, app)]);
    }

    // prepnutie jazyka: otvorí (alebo vytvorí) súbor v tom jazyku vedľa aktuálneho
    fn ard_switch_lang(&mut self, lang: &str) {
        let Some(path) = self.tabs.get(self.active).map(|t| t.path.clone()) else { return };
        let pp = std::path::Path::new(&path);
        let Some(dir) = pp.parent() else { return };
        let stem = pp.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "sketch".into());
        let port = self.ard_port().unwrap_or_else(|| "/dev/ttyUSB0".into());
        let (name, content): (String, String) = match lang {
            "cpp" => {
                // už existujúci .ino v priečinku, inak nový
                let found = std::fs::read_dir(dir).ok().and_then(|rd| rd.flatten().map(|e| e.path()).find(|f| f.extension().is_some_and(|x| x == "ino")));
                match found {
                    Some(f) => (f.file_name().unwrap_or_default().to_string_lossy().to_string(), String::new()),
                    None => (format!("{stem}.ino"), ard::STARTER.to_string()),
                }
            }
            "micropython" => ("main.py".into(), ard::MICROPY_BLINK.to_string()),
            "circuitpython" => ("code.py".into(), ard::CIRCUITPY_BLINK.to_string()),
            _ => ("firmata.py".into(), ard::firmata_blink(&port)),
        };
        let target = dir.join(&name);
        if !target.exists() {
            if let Err(e) = std::fs::write(&target, content) {
                self.status = e.to_string();
                return;
            }
        }
        self.open_file(&target.to_string_lossy());
    }

    // jazyk programu: C++ alebo Python na doske; kliknutie na iný otvorí (alebo vytvorí) súbor v ňom
    fn ard_lang_picker(&mut self, ui: &mut egui::Ui, mode: &str) {
        let p = self.pal;
        let w = ui.available_width();
        let opts: [(&str, &str); 4] = [("cpp", "C++"), ("micropython", "MicroPython"), ("circuitpython", "CircuitPython"), ("firmata", "Firmata")];
        let (area, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
        ui.painter().rect_filled(area, CornerRadius::same(10), p.hover);
        // šírka podľa dĺžky názvu, nech sa „MicroPython“ nezreže
        let weights: Vec<f32> = opts.iter().map(|o| (o.1.len().max(5) + 2) as f32).collect();
        let total: f32 = weights.iter().sum();
        let mut x = 3.0;
        let mut go = None;
        for (i, (id, name)) in opts.iter().enumerate() {
            let bw = (w - 6.0) * weights[i] / total;
            let r = Rect::from_min_size(area.min + vec2(x, 3.0), vec2(bw, 26.0));
            x += bw;
            let resp = ui.interact(r, ui.id().with(("ard-lang", i)), Sense::click());
            let on = *id == mode;
            if on {
                ui.painter().rect_filled(r, CornerRadius::same(8), p.active);
            } else if resp.hovered() {
                ui.painter().rect_filled(r, CornerRadius::same(8), p.card2);
            }
            widgets::text(ui, r.center(), Align2::CENTER_CENTER, name, if on { theme::bold(12.0) } else { theme::ui(12.0) }, if on { p.text } else { p.text2 }, bw - 6.0);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !on {
                go = Some(*id);
            }
        }
        self.ard_hint(ui, &t("Click another language to open (or create) a file in it next to this one."));
        if let Some(l) = go {
            self.ard_switch_lang(l);
        }
    }

    // odznak stavu pripojenia (USB / Bluetooth / Offline), zarovnaný doprava na `right`
    fn ard_pill(&self, ui: &egui::Ui, right: f32, cy: f32) {
        let p = self.pal;
        let (kind, lname, lport) = self.ard_link();
        let short_port = lport.rsplit('/').next().unwrap_or("").to_string();
        let (col, label, tip) = match kind {
            1 => (p.green, format!("USB · {short_port}"), format!("{lname} ({lport})")),
            2 => (Color32::from_rgb(0x4d, 0xa3, 0xff), format!("Bluetooth · {}", lname.chars().take(14).collect::<String>()), format!("{lname} ({lport})")),
            _ => (p.text3, t("Offline"), t("Offline – no board connected")),
        };
        let pw = widgets::text_w(ui, &label, theme::bold(11.5)) + 28.0;
        let pill = Rect::from_center_size(pos2(right - pw / 2.0, cy), vec2(pw, 24.0));
        ui.painter().rect_filled(pill, CornerRadius::same(12), p.hover);
        ui.painter().circle_filled(pos2(pill.left() + 12.0, pill.center().y), 4.0, col);
        ui.painter().text(pos2(pill.left() + 22.0, pill.center().y), Align2::LEFT_CENTER, &label, theme::bold(11.5), if kind == 0 { p.text2 } else { p.text });
        ui.interact(pill, ui.id().with(("ard-link", (right * 10.0) as i32)), Sense::hover()).on_hover_text(tip);
    }

    // niečo chýba (nástroje, doska…): otvor panel na karte Doska a zvýrazni zoznam „Getting ready“
    fn ard_need_setup(&mut self) {
        self.ard.open = true;
        self.ard.section = 0;
        let mode = self.ard_mode().unwrap_or_else(|| "cpp".into());
        self.ard_open_setup(&mode);
    }

    // ako je doska pripojená: (0 offline, 1 USB, 2 Bluetooth, meno, port)
    fn ard_link(&self) -> (u8, String, String) {
        let port = self.ard_port();
        let boards = self.ard.boards.lock().unwrap();
        match boards.iter().find(|b| Some(&b.port) == port.as_ref()) {
            Some(b) => {
                let bt = b.chip == Some("Bluetooth") || b.port.contains("rfcomm") || b.name.to_lowercase().contains("bluetooth");
                (if bt { 2 } else { 1 }, b.name.clone(), b.port.clone())
            }
            None => (0, String::new(), String::new()),
        }
    }

    // Firmware tab pri MicroPythone / CircuitPythone: nahratie interpretera na dosku
    fn ard_py_fw_ui(&mut self, ui: &mut egui::Ui, mode: &str) {
        let p = self.pal;
        let tools = self.ard.tools.lock().unwrap().clone();
        let mut act: Option<&str> = None;
        let name = if mode == "micropython" { "MicroPython" } else { "CircuitPython" };
        self.ard_card(ui, |ui| {
            ui.label(egui::RichText::new(tf("Get {name} for your board", &[("name", name)])).font(theme::bold(13.0)).color(p.text));
            ui.add(
                egui::Label::new(
                    egui::RichText::new(tf("Download the firmware file for your exact board (.uf2 for Raspberry Pi Pico and Adafruit boards, .bin for ESP32 / ESP8266), then install it below.", &[]))
                        .font(theme::ui(12.0))
                        .color(p.text2),
                )
                .wrap(),
            );
            if widgets::button(ui, Some("external"), &tf("Open the {name} downloads", &[("name", name)]), p.card2, p.text, 28.0, &p).clicked() {
                act = Some("get");
            }
        });
        // .uf2 (Pico a spol.): bootloader sa ukáže ako disk
        let uf2 = ard::uf2_drive();
        self.ard_card(ui, |ui| {
            ui.label(egui::RichText::new(t("Raspberry Pi Pico and UF2 boards")).font(theme::bold(13.0)).color(p.text));
            let hint = match &uf2 {
                Some(d) => tf("Bootloader drive found: {d}", &[("d", &d.to_string_lossy())]),
                None => t("Hold the BOOTSEL button while plugging the board in (or double-tap reset). A drive like RPI-RP2 appears."),
            };
            ui.add(egui::Label::new(egui::RichText::new(hint).font(theme::ui(12.0)).color(if uf2.is_some() { p.green } else { p.text2 })).wrap());
            if widgets::button(ui, Some("upload"), &t("Choose .uf2 and install"), if uf2.is_some() { p.accent } else { p.card2 }, if uf2.is_some() { p.accent_fg } else { p.text }, 28.0, &p).clicked()
            {
                act = Some("uf2");
            }
        });
        if mode == "micropython" {
            let chips = [("ESP32", "0x1000"), ("ESP32-S3 / C3 / C6", "0x0"), ("ESP8266", "0x0")];
            let mut esp = self.ard.esp.min(2);
            self.ard_card(ui, |ui| {
                ui.label(egui::RichText::new(t("ESP32 / ESP8266")).font(theme::bold(13.0)).color(p.text));
                let opts: Vec<String> = chips.iter().map(|c| c.0.to_string()).collect();
                if let Some(i) = widgets::select(ui, egui::Id::new("ard-esp"), chips[esp].0, &opts, ui.available_width(), &p) {
                    esp = i;
                }
                if tools.es.is_none() {
                    ui.add(egui::Label::new(egui::RichText::new(t("esptool is the official tool for flashing ESP chips.")).font(theme::ui(12.0)).color(p.text2)).wrap());
                    if widgets::button(ui, Some("download"), &t("Install esptool"), p.accent, p.accent_fg, 28.0, &p).clicked() {
                        act = Some("install-es");
                    }
                } else if widgets::button(ui, Some("upload"), &t("Choose .bin and flash (erases the board)"), p.accent, p.accent_fg, 28.0, &p).clicked() {
                    act = Some("bin");
                }
            });
            self.ard.esp = esp;
        }
        match act {
            Some("get") => flux_core::settings::open_external(if mode == "micropython" { "https://micropython.org/download/" } else { "https://circuitpython.org/downloads" }),
            Some("install-es") => self.ard_install_py(&t("Installing esptool"), "esptool", true),
            Some("uf2") => {
                let Some(dir) = ard::uf2_drive() else {
                    self.status = t("Bootloader drive not found – hold BOOTSEL while plugging the board in.");
                    return;
                };
                if let Some(f) = rfd::FileDialog::new().set_title(t("Choose a .uf2 file")).add_filter("UF2", &["uf2"]).pick_file() {
                    let to = dir.join(f.file_name().unwrap_or_default());
                    self.status = match std::fs::copy(&f, to) {
                        Ok(_) => t("Installed – the board restarts by itself."),
                        Err(e) => e.to_string(),
                    };
                }
            }
            Some("bin") => {
                let (Some(es), Some(port)) = (tools.es.clone(), self.ard_port()) else {
                    self.status = t("No board found – plug it in with a USB cable.");
                    return;
                };
                if let Some(f) = rfd::FileDialog::new().set_title(t("Choose a .bin file")).add_filter("Firmware", &["bin"]).pick_file() {
                    let off = ["0x1000", "0x0", "0x0"][self.ard.esp.min(2)];
                    let mut erase = es.clone();
                    erase.extend(["--port".to_string(), port.clone(), "erase_flash".into()]);
                    let mut write = es;
                    write.extend(["--port".to_string(), port, "--baud".into(), "460800".into(), "write_flash".into(), "-z".into(), off.into(), f.to_string_lossy().to_string()]);
                    self.ard_run(&t("Flashing MicroPython (hold BOOT if it waits at “Connecting…”)"), vec![erase, write]);
                }
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
        // prvýkrát po otvorení panela: ak niečo chýba, ukáže sa okno Getting started (až keď sa zistia nástroje)
        if !self.ard.guide_checked {
            let due = *self.ard.guide_due.get_or_insert(Instant::now() + Duration::from_millis(1800));
            if Instant::now() >= due {
                self.ard.guide_checked = true;
                if let Some(m) = self.ard_mode() {
                    if self.get("arduinoGuideSeen").as_bool() != Some(true) && !self.ard_setup_rows(&m).iter().all(|r| r.ok) {
                        self.ard_open_setup(&m);
                    }
                }
            }
        }
        ui.painter().vline(r.left(), r.y_range(), Stroke::new(1.0, p.line));
        let head = Rect::from_min_size(r.min, vec2(r.width(), 44.0));
        widgets::icon_at(ui, pos2(head.left() + 20.0, head.center().y), 15.0, "chip", p.accent);
        ui.painter().text(
            pos2(head.left() + 34.0, head.center().y),
            Align2::LEFT_CENTER,
            if self.ard_mode().as_deref().is_some_and(|m| m != "cpp") { "Board" } else { "Arduino" },
            theme::bold(14.0),
            p.text,
        );
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(head.right() - 22.0, head.center().y), vec2(28.0, 28.0)), "x", 14.0, &p, true).on_hover_text(t("Close")).clicked() {
            self.ard.open = false;
        }
        self.ard_pill(ui, head.right() - 44.0, head.center().y);
        ui.painter().hline(r.x_range(), head.bottom(), Stroke::new(1.0, p.line));
        // sekcie (pri Pythone na doske len Doska a Firmvér)
        let mode = self.ard_mode().unwrap_or_else(|| "cpp".into());
        let sections: Vec<(u8, String)> = if mode == "cpp" { vec![(0, t("Board")), (2, t("Libraries")), (3, t("Firmware"))] } else { vec![(0, t("Board")), (3, t("Firmware"))] };
        if !sections.iter().any(|x| x.0 == self.ard.section) {
            self.ard.section = 0;
        }
        let tabs_r = Rect::from_min_size(pos2(r.left() + 12.0, head.bottom() + 8.0), vec2(r.width() - 24.0, 30.0));
        let tw = tabs_r.width() / sections.len() as f32;
        for (i, (id, n)) in sections.iter().enumerate() {
            let tr = Rect::from_min_size(pos2(tabs_r.left() + i as f32 * tw, tabs_r.top()), vec2(tw - 4.0, 28.0));
            let resp = ui.interact(tr, ui.id().with(("ard-tab", i)), Sense::click());
            let on = self.ard.section == *id;
            if on || resp.hovered() {
                ui.painter().rect_filled(tr, CornerRadius::same(8), if on { p.active } else { p.hover });
            }
            widgets::text(ui, tr.center(), Align2::CENTER_CENTER, n, if on { theme::bold(12.5) } else { theme::ui(12.5) }, if on { p.text } else { p.text2 }, tw - 8.0);
            if resp.clicked() {
                self.ard.section = *id;
            }
        }
        // spodná lišta s hlavnou akciou: len na karte Doska
        const FOOT_H: f32 = 54.0;
        let foot_on = self.ard.section == 0;
        let mut body = Rect::from_min_max(pos2(r.left(), tabs_r.bottom() + 6.0), r.max);
        if foot_on {
            body.max.y -= FOOT_H;
            let foot = Rect::from_min_max(pos2(r.left(), r.bottom() - FOOT_H), r.max);
            self.ard_footer(ui, foot, &mode);
        }
        let mut bui = ui.new_child(egui::UiBuilder::new().max_rect(body.shrink2(vec2(14.0, 4.0))));
        bui.set_clip_rect(body);
        let mut sa = egui::ScrollArea::vertical().id_salt("ard-body").auto_shrink(false);
        if std::mem::take(&mut self.ard.scroll_top) {
            sa = sa.vertical_scroll_offset(0.0);
        }
        sa.show(&mut bui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            match (self.ard.section, mode.as_str()) {
                (0, m) => self.ard_board_tab(ui, m),
                (2, _) => self.ard_libs_ui(ui, &ctx),
                (_, "micropython" | "circuitpython") => self.ard_py_fw_ui(ui, &mode),
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
        let (fill, m) = if self.ard.plain { (Color32::TRANSPARENT, 0) } else { (p.hover, 12) };
        egui::Frame::NONE.fill(fill).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(m)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    }

    // zoznam pripojených dosiek (výber portu) a spárované Bluetooth zariadenia
    fn ard_ports_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.pal;
        let w = ui.available_width();
        let boards = self.ard.boards.lock().unwrap().clone();
        let cur = self.ard_port();
        if boards.is_empty() {
            let hint = if cfg!(windows) {
                t("Plug the board in with a USB cable (a data cable, not only charging). Elegoo and other clones use the CH340 chip – if no COM port appears, install its driver from wch-ic.com.")
            } else {
                t("Plug the board in with a USB cable (a data cable, not only charging). It shows up as /dev/ttyACM0 or /dev/ttyUSB0 (Elegoo and other clones with the CH340 chip).")
            };
            self.ard_hint(ui, &hint);
        }
        let bt = self.ard.bt.lock().unwrap().clone();
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
        // spárované Bluetooth zariadenia (HC-05 / HC-06 a pod.): doska cez rfcomm je sériový port
        if !bt.is_empty() {
            let mut copy: Option<String> = None;
            for (name, mac, conn) in &bt {
                self.ard_card(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (cr, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                        ui.painter().circle_filled(cr.center(), 4.0, if *conn { Color32::from_rgb(0x4d, 0xa3, 0xff) } else { p.text3 });
                        ui.label(egui::RichText::new(name).font(theme::bold(12.5)).color(p.text));
                        ui.label(egui::RichText::new(mac).font(theme::mono(11.0)).color(p.text3));
                    });
                    if !boards.iter().any(|b| b.port.contains("rfcomm")) && !cfg!(windows) {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("sudo rfcomm bind /dev/rfcomm0 {mac}")).font(theme::mono(11.0)).color(p.text2));
                            if widgets::icon_button(ui, "copy", &p, true).on_hover_text(t("Copy")).clicked() {
                                copy = Some(format!("sudo rfcomm bind /dev/rfcomm0 {mac}"));
                            }
                        });
                    }
                });
            }
            if let Some(c) = copy {
                ui.ctx().copy_text(c);
            }
            self.ard_hint(
                ui,
                &t("Run the command once, and the Bluetooth module shows up here as a serial port (serial monitor, console). Uploading code over Bluetooth works only on boards that support it."),
            );
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
    }

    // čo ešte chýba, aby to fungovalo (nástroje, doska, práva k portu): odškrtávací zoznam; nič, keď je všetko hotové
    fn ard_setup_rows(&mut self, mode: &str) -> Vec<Row> {
        let tools = self.ard.tools.lock().unwrap().clone();
        let inst = self.ard.inst.lock().unwrap().clone();
        let port = self.ard_port();
        let cur = self.ard.boards.lock().unwrap().iter().find(|b| Some(&b.port) == port.as_ref()).cloned();
        let mut rows: Vec<Row> = vec![];
        let row = |ok: bool, title: String, detail: String| Row { ok, title, detail, red: false, busy: false, act: None, copy: None };
        match mode {
            "micropython" => {
                let mut r = row(tools.mp.is_some(), "mpremote".into(), t("The official MicroPython tool for sending files to the board and opening its REPL (a small Python package)."));
                if tools.mp.is_none() {
                    r.act = Some(("install-mp", t("Install")));
                } else {
                    r.detail.clear();
                }
                rows.push(r);
            }
            "circuitpython" => {
                let d = ard::circuitpy_drive();
                let detail = match &d {
                    Some(d) => d.to_string_lossy().to_string(),
                    None => t("Plug the board in. If it never shows up, CircuitPython is not on it yet – see the Firmware tab."),
                };
                rows.push(row(d.is_some(), t("CIRCUITPY drive"), detail));
            }
            _ => {
                let ok = self.ard_cli().is_some();
                let mut r = row(ok, t("Arduino tools"), String::new());
                if !ok {
                    match &inst {
                        Inst::Downloading => {
                            r.busy = true;
                            r.detail = t("Downloading arduino-cli…");
                        }
                        Inst::Failed(e) => {
                            r.red = true;
                            r.detail = e.clone();
                            r.act = Some(("install-tools", t("Try again")));
                        }
                        _ => {
                            r.detail = t("Flux downloads the official arduino-cli (about 25 MB) and the board package for Uno, Mega, Nano and Elegoo boards (about 150 MB). Nothing else is needed.");
                            r.act = Some(("install-tools", t("Install")));
                        }
                    }
                }
                rows.push(r);
            }
        }
        if mode == "firmata" {
            let mut r = row(tools.firmata, "pyFirmata2".into(), t("The Python library that talks to StandardFirmata."));
            if !tools.firmata {
                r.act = Some(("install-firmata", t("Install")));
            } else {
                r.detail.clear();
            }
            rows.push(r);
        }
        let board = match &cur {
            Some(b) => row(true, t("Board"), format!("{}  ·  {}", b.name, b.port)),
            None => row(false, t("Board"), t("Plug it in with a USB data cable (not a charge-only one).")),
        };
        rows.push(board);
        if let Some(port) = &port {
            if let Some(group) = ard::port_access(port) {
                let cmd = format!("sudo usermod -aG {group} $USER");
                let mut r = row(
                    false,
                    t("Port access"),
                    format!("{}\n{}", tf("No permission for {port}", &[("port", port)]), tf("Add yourself to the {group} group once, then log out and back in:", &[("group", &group)])),
                );
                r.red = true;
                r.copy = Some(cmd);
                rows.push(r);
            }
        }
        rows
    }

    // malý pruh v paneli: čo ešte chýba + tlačidlo na sprievodcu (okno „Getting started“); nič, keď je všetko hotové
    fn ard_setup_ui(&mut self, ui: &mut egui::Ui, mode: &str) -> Option<&'static str> {
        let p = self.pal;
        let rows = self.ard_setup_rows(mode);
        if rows.iter().all(|r| r.ok) {
            return None;
        }
        let (done, total) = (rows.iter().filter(|r| r.ok).count(), rows.len());
        let mut open = false;
        egui::Frame::NONE
            .fill(p.hover.lerp_to_gamma(p.accent, 0.08))
            .stroke(Stroke::new(1.5, p.accent.gamma_multiply(0.6)))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(t("Getting started")).font(theme::bold(13.5)).color(p.text));
                ui.label(egui::RichText::new(tf("{done} of {total} steps done", &[("done", &done.to_string()), ("total", &total.to_string())])).font(theme::ui(12.0)).color(p.text2));
                let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
                ui.painter().rect_filled(bar, CornerRadius::same(2), p.card2);
                ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * done as f32 / total.max(1) as f32, 4.0)), CornerRadius::same(2), p.accent);
                ui.add_space(2.0);
                open = widgets::button(ui, Some("rocket"), &t("Open the guide"), p.accent, p.accent_fg, 28.0, &p).clicked();
            });
        if open {
            self.ard_open_setup(mode);
        }
        None
    }

    pub(super) fn ard_open_setup(&mut self, mode: &str) {
        self.ard.setup_open = true;
        self.ard.setup_mode = mode.to_string();
    }

    // okno „Getting started“ (ako Nastavenia): kroky, ktoré treba urobiť raz, aby nahrávanie na dosku fungovalo
    pub(super) fn ard_setup_window(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        let mode = self.ard.setup_mode.clone();
        let rows = self.ard_setup_rows(&mode);
        let all_ok = rows.iter().all(|r| r.ok);
        ctx.request_repaint_after(Duration::from_millis(500));
        let bg = ui.interact(full, ui.id().with("ards-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(80));
        let w = (full.width() - 48.0).min(560.0);
        let top = full.top() + ((full.height() - self.ard.setup_h) / 2.0).max(16.0);
        let card = Rect::from_min_size(pos2(full.center().x - w / 2.0, top), vec2(w, self.ard.setup_h.min(full.bottom() - 16.0 - top)));
        let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if bg.clicked() && !card.contains(bg.interact_pointer_pos().unwrap_or_default()) {
            close = true;
        }
        ui.painter().add(egui::Shadow { offset: [0, 30], blur: 90, spread: 0, color: Color32::from_black_alpha(128) }.as_shape(card, CornerRadius::same(20)));
        ui.painter().rect_filled(card, CornerRadius::same(20), p.solid);
        ui.painter().rect_stroke(card, CornerRadius::same(20), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let inner = card.shrink2(vec2(26.0, 22.0));
        let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(inner.min, vec2(inner.width(), 2000.0))));
        cu.set_clip_rect(card);
        cu.spacing_mut().item_spacing.y = 8.0;
        let mut act: Option<&'static str> = None;
        let mut copy: Option<String> = None;
        let head = match mode.as_str() {
            "micropython" => "MicroPython",
            "circuitpython" => "CircuitPython",
            "firmata" => "Python + Firmata",
            _ => "Arduino",
        };
        cu.horizontal(|ui| {
            widgets::icon_at(ui, ui.cursor().min + vec2(10.0, 14.0), 18.0, "chip", p.accent);
            ui.add_space(26.0);
            ui.label(egui::RichText::new(tf("Getting started with {what}", &[("what", head)])).font(theme::bold(20.0)).color(p.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::icon_button(ui, "x", &p, true).on_hover_text(t("Close (Esc)")).clicked() {
                    close = true;
                }
            });
        });
        cu.label(
            egui::RichText::new(if all_ok { t("Everything is ready – you can upload to your board now.") } else { t("Do these steps once – after that Upload to board just works.") })
                .font(theme::ui(13.0))
                .color(if all_ok { p.green } else { p.text2 }),
        );
        cu.add_space(6.0);
        let first = rows.iter().position(|r| !r.ok);
        for (i, r) in rows.iter().enumerate() {
            egui::Frame::NONE
                .fill(p.card2)
                .stroke(Stroke::new(1.0, if Some(i) == first { p.accent.gamma_multiply(0.7) } else { p.line }))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(14))
                .show(&mut cu, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (cr, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                        if r.ok {
                            ui.painter().circle_filled(cr.center(), 13.0, p.green);
                            widgets::icon_at(ui, cr.center(), 15.0, "check", Color32::BLACK);
                        } else if Some(i) == first {
                            ui.painter().circle_filled(cr.center(), 13.0, p.accent);
                            ui.painter().text(cr.center(), Align2::CENTER_CENTER, (i + 1).to_string(), theme::bold(14.0), p.accent_fg);
                        } else {
                            ui.painter().circle_stroke(cr.center(), 12.5, Stroke::new(1.5, p.text3));
                            ui.painter().text(cr.center(), Align2::CENTER_CENTER, (i + 1).to_string(), theme::bold(14.0), p.text3);
                        }
                        ui.label(egui::RichText::new(&r.title).font(theme::bold(15.0)).color(if r.ok { p.text2 } else { p.text }));
                        if r.busy {
                            ui.spinner();
                        }
                        if r.ok {
                            ui.label(egui::RichText::new(t("Done")).font(theme::ui(12.0)).color(p.green));
                        }
                    });
                    if !r.detail.is_empty() {
                        egui::Frame::NONE.inner_margin(egui::Margin { left: 38, right: 0, top: 0, bottom: 0 }).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(egui::Label::new(egui::RichText::new(&r.detail).font(theme::ui(12.5)).color(if r.red { p.red } else { p.text2 })).wrap());
                            if let Some(c) = &r.copy {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(c).font(theme::mono(12.5)).color(p.text));
                                    if widgets::icon_button(ui, "copy", &p, true).on_hover_text(t("Copy")).clicked() {
                                        copy = Some(c.clone());
                                    }
                                });
                            }
                            if let Some((a, label)) = &r.act {
                                if widgets::button(ui, Some("download"), label, p.accent, p.accent_fg, 30.0, &p).clicked() {
                                    act = Some(*a);
                                }
                            }
                        });
                    }
                });
        }
        cu.add_space(6.0);
        cu.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if all_ok {
                    if widgets::button(ui, Some("check"), &t("Done"), p.accent, p.accent_fg, 32.0, &p).clicked() {
                        close = true;
                    }
                } else if widgets::button(ui, None, &t("Close"), p.card2, p.text, 32.0, &p).clicked() {
                    close = true;
                }
            });
        });
        self.ard.setup_h = (cu.min_rect().height() + 44.0).max(220.0);
        if let Some(c) = copy {
            ctx.copy_text(c);
        }
        if let Some(a) = act {
            self.ard_act(a, &ctx);
        }
        if close {
            self.ard.setup_open = false;
            self.update_settings(|o| {
                o.insert("arduinoGuideSeen".into(), json!(true));
            });
        }
    }

    // jedno telo karty Doska pre všetky jazyky
    fn ard_board_tab(&mut self, ui: &mut egui::Ui, mode: &str) {
        let p = self.pal;
        let w = ui.available_width();
        let mut act = self.ard_setup_ui(ui, mode);
        self.ard_head_ui(ui, mode);
        let boards = self.ard.boards.lock().unwrap().clone();
        // Uno, Nano, Mega a klony (ATmega) majú 2–8 KB RAM: MicroPython na nich nebeží
        if matches!(mode, "micropython" | "circuitpython") {
            let cur = self.ard_port();
            let small = boards.iter().find(|b| Some(&b.port) == cur.as_ref()).filter(|b| b.fqbn.as_deref().is_some_and(|f| f.starts_with("arduino:avr:")));
            if let Some(b) = small {
                let ch = b.chip == Some("CH340");
                let mut to_firmata = false;
                self.ard_card(ui, |ui| {
                    ui.label(egui::RichText::new(t("MicroPython can't run on this board – Flux converts it to C++")).font(theme::bold(13.0)).color(p.text));
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(t("Arduino Uno, Nano, Mega and Elegoo boards have only 2–8 KB of memory, so Python can't run on them. When you upload, Flux converts your Python to C++ (simple programs: pins, delays, loops, if, print…). Use Python + Firmata to run any Python live from your computer."))
                                .font(theme::ui(12.0))
                                .color(p.text2),
                        )
                        .wrap(),
                    );
                    if ch {
                        ui.add(egui::Label::new(egui::RichText::new(t("If this is an ESP32 board with a CH340 chip, ignore this.")).font(theme::ui(11.5)).color(p.text3)).wrap());
                    }
                    to_firmata = widgets::button(ui, None, &t("Switch to Python + Firmata"), p.card2, p.text, 28.0, &p).clicked();
                });
                if to_firmata {
                    self.ard_switch_lang("firmata");
                }
            }
        }
        // jazyk
        let lang_name = match mode {
            "micropython" => "MicroPython",
            "circuitpython" => "CircuitPython",
            "firmata" => "Firmata",
            _ => "C++",
        };
        if self.ard_sec(ui, "lang", &t("Programming language"), true, Some(lang_name.into())) {
            self.ard_lang_picker(ui, mode);
            let desc = match mode {
                "micropython" => "Python runs on the board itself (ESP32, ESP8266, Raspberry Pi Pico…). Flux sends your file with mpremote.",
                "circuitpython" => "Python runs on the board itself (Pico, Adafruit, Seeed…). The board shows up as a CIRCUITPY drive – Flux copies your file there as code.py.",
                "firmata" => "The board runs StandardFirmata and your Python program runs on the computer (pyFirmata2) – works with Uno, Mega, Nano and Elegoo boards.",
                _ => "Arduino C++ is compiled and uploaded with the official arduino-cli (Uno, Mega, Nano, ESP32, Pico…).",
            };
            self.ard_hint(ui, &t(desc));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                if mode == "firmata" && widgets::button(ui, Some("copy"), &t("Copy example with this port"), p.card2, p.text, 28.0, &p).clicked() {
                    act = Some("example");
                }
                if mode == "micropython" && widgets::button(ui, Some("folder"), &t("Files on board"), p.card2, p.text, 28.0, &p).clicked() {
                    act = Some("ls");
                }
            });
        }
        // dosky a Bluetooth
        let nb = boards.len();
        if self.ard_sec(ui, "boards", &t("Connected boards"), nb != 1, Some(nb.to_string())) {
            self.ard_ports_ui(ui);
            if boards.iter().any(|b| b.chip == Some("CH340")) && mode == "cpp" {
                self.ard_hint(ui, &t("Clone boards (CH340) can't tell which board they are – pick it here. Nano clones usually need “old bootloader”."));
            }
        }
        // sériový monitor (C++)
        if mode == "cpp" && self.ard_sec(ui, "serial", &t("Serial monitor"), false, None) {
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
        }
        if matches!(mode, "micropython" | "circuitpython") {
            self.ard_hint(ui, &t("Close the REPL or console (Stop) before running a file – only one program can use the port."));
        }
        if let Some(a) = act {
            let ctx = ui.ctx().clone();
            self.ard_act(a, &ctx);
        }
    }

    // všetky akcie panela na jednom mieste
    fn ard_act(&mut self, act: &str, ctx: &egui::Context) {
        let tools = self.ard.tools.lock().unwrap().clone();
        let port = self.ard_port();
        match act {
            "upload" | "upload-cpp" => self.ard_build(true),
            "verify" => self.ard_build(false),
            "show-cpp" => self.ard_show_cpp(),
            "monitor" => self.ard_monitor(),
            "install-tools" => self.ard_install_tools(ctx),
            "install-mp" => self.ard_install_py(&t("Installing mpremote"), "mpremote", true),
            "install-serial" => self.ard_install_py(&t("Installing pyserial"), "pyserial", false),
            "install-firmata" => self.ard_install_py(&t("Installing pyFirmata2"), "pyfirmata2", false),
            "run" => self.ard_py_run(false),
            "save" => self.ard_py_run(true),
            "repl" | "ls" => {
                let (Some(mp), Some(port)) = (tools.mp.clone(), port.clone()) else {
                    self.status = t("No board found – plug it in with a USB cable.");
                    return;
                };
                let mut cmd = mp;
                cmd.extend(["connect".to_string(), port]);
                if act == "repl" {
                    cmd.push("repl".into());
                    self.ard_run(&t("MicroPython REPL (Stop to close)"), vec![cmd]);
                } else {
                    cmd.extend(["fs".to_string(), "ls".to_string()]);
                    self.ard_run(&t("Files on the board"), vec![cmd]);
                }
            }
            "console" => {
                if !tools.serial {
                    return self.ard_install_py(&t("Installing pyserial"), "pyserial", false);
                }
                match port {
                    Some(port) => self.ard_run(&t("CircuitPython console (Stop to close)"), vec![vec![ard::py().into(), "-m".into(), "serial.tools.miniterm".into(), port, "115200".into()]]),
                    None => self.status = t("No board found – plug it in with a USB cable."),
                }
            }
            "firmata" => self.ard_firmware("firmata"),
            "pyrun" => self.run(),
            "example" => ctx.copy_text(ard::firmata_blink(&port.unwrap_or_else(|| "/dev/ttyUSB0".into()))),
            _ => {}
        }
    }

    // spodná lišta s hlavnou akciou (vždy na očiach): Nahrať na dosku + 1–2 vedľajšie
    fn ard_footer(&mut self, ui: &mut egui::Ui, foot: Rect, mode: &str) {
        let p = self.pal;
        let tools = self.ard.tools.lock().unwrap().clone();
        let cli = self.ard_cli().is_some();
        ui.painter().hline(foot.x_range(), foot.top(), Stroke::new(1.0, p.line));
        // (ikona, text, akcia) hlavného tlačidla + vedľajšie (ikona, tip, akcia)
        let (main, side): ((&str, String, &str), Vec<(&str, String, &str)>) = match mode {
            "micropython" | "circuitpython" if self.ard_cpp_board() && !cli => (("download", t("Install Arduino tools"), "install-tools"), vec![]),
            "micropython" | "circuitpython" if self.ard_cpp_board() => {
                (("upload", t("Upload to board"), "upload-cpp"), vec![("check", t("Verify (compile only)"), "verify"), ("file", t("Show the C++ that gets uploaded"), "show-cpp")])
            }
            "firmata" if cli => {
                (("upload", t("Upload to board"), "upload-cpp"), vec![("play", t("Run on board without uploading (Firmata)"), "pyrun"), ("check", t("Verify (compile only)"), "verify")])
            }
            "micropython" if tools.mp.is_none() => (("download", t("Install mpremote"), "install-mp"), vec![]),
            "micropython" => (("upload", t("Upload to board"), "save"), vec![("play", t("Run without saving"), "run"), ("terminal", t("REPL"), "repl")]),
            "circuitpython" => (("upload", t("Copy to board"), "run"), vec![("terminal", t("Serial console"), "console")]),
            "firmata" if !tools.firmata => (("download", t("Install pyFirmata2"), "install-firmata"), vec![("upload", t("Upload StandardFirmata to the board"), "firmata")]),
            "firmata" => (("play", t("Run the script"), "pyrun"), vec![("upload", t("Upload StandardFirmata to the board"), "firmata")]),
            _ if !cli => (("download", t("Install Arduino tools"), "install-tools"), vec![]),
            _ => (("upload", t("Upload to board"), "upload"), vec![("check", t("Verify (compile only)"), "verify"), ("terminal", t("Serial monitor"), "monitor")]),
        };
        let mut act: Option<&str> = None;
        let inner = foot.shrink2(vec2(14.0, 9.0));
        let mut right = inner.right();
        for (i, (icon, tip, a)) in side.iter().enumerate().rev() {
            let r = Rect::from_min_size(pos2(right - 36.0, inner.top()), vec2(36.0, inner.height()));
            ui.painter().rect_filled(r, CornerRadius::same(10), p.card2);
            ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            let _ = i;
            if widgets::icon_button_at(ui, r, icon, 16.0, &p, true).on_hover_text(tip.as_str()).clicked() {
                act = Some(a);
            }
            right -= 42.0;
        }
        let pr = Rect::from_min_max(inner.min, pos2(right + 4.0, inner.bottom()));
        let resp = ui.interact(pr, ui.id().with("ard-main"), Sense::click());
        let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.1);
        ui.painter().rect_filled(pr, CornerRadius::same(10), p.accent.lerp_to_gamma(Color32::WHITE, 0.12 * hk));
        let tw = widgets::text_w(ui, &main.1, theme::bold(13.5));
        let x0 = pr.center().x - (tw + 22.0) / 2.0;
        widgets::icon_at(ui, pos2(x0 + 8.0, pr.center().y), 15.0, main.0, p.accent_fg);
        ui.painter().text(pos2(x0 + 22.0, pr.center().y), Align2::LEFT_CENTER, &main.1, theme::bold(13.5), p.accent_fg);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            act = Some(main.2);
        }
        if let Some(a) = act {
            let ctx = ui.ctx().clone();
            self.ard_act(a, &ctx);
        }
    }

    // obrázok dosky: 0 = Uno / Nano / Mega, 1 = ESP32 DevKit, 2 = Raspberry Pi Pico (zvolená alebo pripojená doska)
    fn ard_layout(&self, mode: &str) -> u8 {
        let by_fqbn = |f: &str| match f.split(':').nth(1) {
            _ if f.starts_with("esp32:") || f.starts_with("esp8266:") => Some(1),
            _ if f.starts_with("rp2040:") => Some(2),
            Some("avr") if f.contains(":mega") => Some(3),
            Some("avr") if f.contains(":nano") => Some(4),
            Some(_) => Some(0),
            None => None,
        };
        let fq = self.ard_board();
        if self.get("arduinoBoardAuto").as_bool() == Some(false) {
            return by_fqbn(&fq).unwrap_or(0);
        }
        let port = self.ard_port();
        let boards = self.ard.boards.lock().unwrap().clone();
        match boards.iter().find(|b| Some(&b.port) == port.as_ref()) {
            Some(b) => {
                let n = b.name.to_lowercase();
                if let Some(l) = b.fqbn.as_deref().and_then(by_fqbn).filter(|l| *l != 0) {
                    l
                } else if n.contains("esp") || (mode == "micropython" && b.chip == Some("CH340")) {
                    1
                } else if n.contains("pico") || n.contains("rp2") {
                    2
                } else if n.contains("mega") {
                    3
                } else if b.chip == Some("CH340") {
                    // klon bez rozpoznania: podľa uloženej voľby (Uno, Mega, Nano…)
                    by_fqbn(&fq).unwrap_or(0)
                } else {
                    0
                }
            }
            None if mode == "cpp" => by_fqbn(&fq).unwrap_or(0),
            None if mode == "micropython" => 1,
            None => 0,
        }
    }

    // obrázok dosky (klik na pin ho vyberie)
    fn ard_picture(&mut self, ui: &mut egui::Ui, mode: &str, pins: &[ard::Pin]) {
        let p = self.pal;
        let layout = self.ard_layout(mode);
        let w = ui.available_width();
        let sel = self.ard.sel.clone();
        let avr = matches!(layout, 0 | 3);
        let draw = |ui: &mut egui::Ui, w: f32| match layout {
            1 => boardart::draw_vertical(ui, &p, w, "ESP32", &ESP_L, &ESP_R, false, Color32::from_rgb(0x23, 0x25, 0x2b), pins, &sel),
            2 => boardart::draw_vertical(ui, &p, w, "Pico", &PICO_L, &PICO_R, true, Color32::from_rgb(0x1c, 0x7a, 0x44), pins, &sel),
            4 => boardart::draw_vertical(ui, &p, w, "Nano", &NANO_L, &NANO_R, true, Color32::from_rgb(0x00, 0x87, 0x8f), pins, &sel),
            3 => boardart::draw_avr(ui, &p, w, true, pins, &sel),
            _ => boardart::draw_avr(ui, &p, w, false, pins, &sel),
        };
        let clicked = if avr {
            // Uno / Mega: okno pevnej veľkosti; Ctrl + koleso približuje k miestu pod myšou, ťahanie posúva, dvojklik vráti späť
            let mega = layout == 3;
            let h0 = boardart::avr_h(w, mega);
            let (vp, resp) = ui.allocate_exact_size(vec2(w, h0), Sense::click_and_drag());
            if let Some(pt) = resp.hover_pos() {
                let zd = ui.input(|i| i.zoom_delta());
                if (zd - 1.0).abs() > 0.0001 {
                    let old = self.ard.zoom;
                    let new = (old * zd.powf(0.6)).clamp(1.0, 3.0);
                    let rel = pt - vp.min;
                    self.ard.pan = (rel + self.ard.pan) / old * new - rel;
                    self.ard.zoom = new;
                }
                // touchpad: posun do strán dvoma prstami
                let sd = ui.input(|i| i.smooth_scroll_delta);
                if self.ard.zoom > 1.001 && sd.x.abs() > 0.0 {
                    self.ard.pan.x -= sd.x;
                }
            }
            if resp.dragged() && self.ard.zoom > 1.001 {
                self.ard.pan -= resp.drag_delta();
            }
            if resp.double_clicked() {
                self.ard.zoom = 1.0;
                self.ard.pan = egui::Vec2::ZERO;
            }
            let z = self.ard.zoom;
            let hz = boardart::avr_h(w * z, mega);
            self.ard.pan.x = self.ard.pan.x.clamp(0.0, (w * z - w).max(0.0));
            self.ard.pan.y = self.ard.pan.y.clamp(0.0, (hz - h0).max(0.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(vp.min - self.ard.pan, vec2(w * z, hz))));
            child.set_clip_rect(vp.intersect(ui.clip_rect()));
            let c = draw(&mut child, w * z);
            if z > 1.001 {
                // malá mapa: kde v doske práve sme
                let mini = Rect::from_min_size(pos2(vp.right() - 74.0, vp.bottom() - 46.0), vec2(64.0, 36.0));
                ui.painter().rect_filled(mini, CornerRadius::same(5), p.card2.gamma_multiply(0.9));
                ui.painter().rect_stroke(mini, CornerRadius::same(5), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                let view = Rect::from_min_size(mini.min + vec2(self.ard.pan.x / (w * z) * mini.width(), self.ard.pan.y / hz * mini.height()), vec2(mini.width() / z, mini.height() * h0 / hz));
                ui.painter().rect_stroke(view, CornerRadius::same(2), Stroke::new(1.5, p.accent), StrokeKind::Inside);
                resp.on_hover_cursor(if ui.input(|i| i.pointer.primary_down()) { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
            }
            let hint = if z > 1.001 {
                tf("Zoom {z} % – drag to move, double-click to reset", &[("z", &((z * 100.0).round() as i32).to_string())])
            } else {
                t("Ctrl + mouse wheel zooms where the mouse is")
            };
            ui.label(egui::RichText::new(hint).font(theme::ui(11.0)).color(p.text3));
            c
        } else {
            self.ard.zoom = 1.0;
            self.ard.pan = egui::Vec2::ZERO;
            draw(ui, w)
        };
        if let Some(c) = clicked {
            self.ard.sel = Some(c);
            self.ard.pin_dlg = true;
            self.ard.dlg_mode = self.ard.ctx.1.clone();
            self.ard.dlg_kind.clear();
            self.ard.dlg_new = false;
        }
    }

    // text súboru: z otvorenej karty, inak z disku
    fn ard_target_text(&self, path: &str) -> String {
        self.tabs.iter().find(|t| t.path == path).map(|t| t.text.clone()).or_else(|| std::fs::read_to_string(path).ok()).unwrap_or_default()
    }

    // upraví súbor `path` (otvorí ho, ak ešte nie je); vráti jeho meno, ak sa niečo zmenilo
    fn ard_edit_target(&mut self, path: &str, f: impl FnOnce(&str) -> Option<String>) -> Option<String> {
        if !self.tabs.iter().any(|t| t.path == path) {
            self.open_file(path);
        }
        let i = self.tabs.iter().position(|t| t.path == path)?;
        let new = f(&self.tabs[i].text)?;
        self.tabs[i].text = new;
        Some(self.tabs[i].name())
    }

    // prepíše súbor: ak je otvorený, jeho kartu, inak súbor na disku; vráti pôvodný text
    fn ard_rewrite(&mut self, path: &str, new: &str) -> Option<String> {
        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
            let old = std::mem::replace(&mut self.tabs[i].text, new.to_string());
            return Some(old);
        }
        let old = std::fs::read_to_string(path).ok()?;
        std::fs::write(path, new).ok()?;
        Some(old)
    }

    // odoberie pin z kódu všetkých súborov sketchu (opak „pridať do kódu“); jeho odobratie sa dá vrátiť
    pub(super) fn ard_remove_pin(&mut self, pin: &str, mode: &str, files: &[String]) {
        let mut undo: Vec<(String, String)> = vec![];
        let mut hard = false;
        for f in files {
            let text = self.ard_target_text(f);
            let new = if mode == "cpp" {
                match ard::remove_pin_cpp(&text, pin) {
                    Ok(n) => n,
                    Err(_) => {
                        hard = true;
                        None
                    }
                }
            } else {
                ard::remove_pin_py(&text, pin)
            };
            if let Some(n) = new {
                if let Some(old) = self.ard_rewrite(f, &n) {
                    undo.push((f.clone(), old));
                }
            }
        }
        self.status = if !undo.is_empty() {
            let n = undo.len();
            self.ard.undo = undo;
            self.ard.undo_pin = pin.to_string();
            self.ard.sketch = None;
            tf("Removed pin {pin} from the code ({n} file(s))", &[("pin", pin), ("n", &n.to_string())])
        } else if hard {
            t("This pin comes from a loop or an array – remove it by hand in the code.")
        } else {
            t("Nothing to remove for this pin.")
        };
    }

    fn ard_undo_remove(&mut self) {
        for (path, old) in std::mem::take(&mut self.ard.undo) {
            self.ard_rewrite(&path, &old);
        }
        self.ard.sketch = None;
        self.status = tf("Pin {pin} is back in the code", &[("pin", &std::mem::take(&mut self.ard.undo_pin))]);
    }

    // výber dosky: automaticky podľa pripojenej, alebo vlastná zo zoznamu
    fn ard_board_select(&mut self, ui: &mut egui::Ui) {
        let p = self.pal;
        let w = ui.available_width();
        let auto = self.get("arduinoBoardAuto").as_bool() != Some(false);
        let fq = self.ard_board();
        let mut opts: Vec<String> = vec![t("Automatic (detected)")];
        opts.extend(BOARDS.iter().map(|b| t(b.1)));
        let cur_name = BOARDS.iter().find(|b| b.0 == fq).map(|b| t(b.1)).unwrap_or(fq.clone());
        let chip = dash::chip_of(&fq);
        let how = if auto { t("Automatic (detected)") } else { t("Custom") };
        let sub = if chip.is_empty() { format!("{} · {how}", t("Board")) } else { format!("{} · {how} · {chip}", t("Board")) };
        let picked = dash_select(ui, egui::Id::new("ard-board"), &cur_name, &sub, &opts, w, &p);
        if let Some(i) = picked {
            self.ard.sel = None;
            if i == 0 {
                self.update_settings(|o| {
                    o.insert("arduinoBoardAuto".into(), json!(true));
                });
            } else {
                let f = BOARDS[i - 1].0;
                self.update_settings(|o| {
                    o.insert("arduinoBoardAuto".into(), json!(false));
                    o.insert("arduinoBoard".into(), json!(f));
                });
            }
        }
    }

    // kód používa piny, ktoré má len Mega (22–53, A8+), ale doska je Uno/Nano: ponúkni Megu
    fn ard_mega_hint(&mut self, ui: &mut egui::Ui, mode: &str, pins: &[ard::Pin]) {
        if mode != "cpp" || self.ard_layout(mode) == 3 {
            return;
        }
        let mega_only = pins.iter().any(|x| x.pin.parse::<u32>().is_ok_and(|n| n >= 20) || x.pin.strip_prefix('A').and_then(|a| a.parse::<u32>().ok()).is_some_and(|a| a >= 8));
        if !mega_only {
            return;
        }
        let p = self.pal;
        let mut go = false;
        self.ard_card(ui, |ui| {
            ui.label(egui::RichText::new(t("Your code uses pins that only exist on the Mega 2560.")).font(theme::ui(12.0)).color(p.text2));
            go = widgets::button(ui, None, &t("Use Arduino Mega 2560"), p.accent, p.accent_fg, 28.0, &p).clicked();
        });
        if go {
            self.update_settings(|o| {
                o.insert("arduinoBoardAuto".into(), json!(false));
                o.insert("arduinoBoard".into(), json!("arduino:avr:mega"));
            });
        }
    }

    // výber dosky, tipy a karta vybraného pinu: hotové ukážky alebo vlastný kód (do nového súboru)
    fn ard_pin_panel(&mut self, ui: &mut egui::Ui, mode: &str, pins: &[ard::Pin], with_select: bool) {
        let layout = self.ard_layout(mode);
        if with_select {
            self.ard_board_select(ui);
        }
        self.ard_mega_hint(ui, mode, pins);
        if layout == 1 || layout == 2 {
            self.ard_hint(ui, &t("ESP32 and Pico pins work at 3.3 V – connect parts to 3V3, not 5V."));
        }
        self.ard_hint(ui, &t("Click a pin on the board to add code for it."));
    }

    // okno po kliku na pin: jazyk, predvoľba, meno → vytvorí súbor v projekte
    pub(super) fn ard_pin_window(&mut self, ui: &mut egui::Ui, full: Rect) {
        dash::set(&self.pal);
        let p = self.pal;
        let Some(sel) = self.ard.sel.clone() else {
            self.ard.pin_dlg = false;
            return;
        };
        let (dir, ctx_mode, target) = self.ard.ctx.clone();
        if self.ard.dlg_mode.is_empty() {
            self.ard.dlg_mode = ctx_mode.clone();
        }
        let layout = self.ard_layout(&ctx_mode);
        let bg = ui.interact(full, ui.id().with("ardp-dim"), Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(80));
        let w = (full.width() - 48.0).min(520.0);
        let top = full.top() + ((full.height() - self.ard.dlg_h) / 2.0).max(16.0);
        let card = Rect::from_min_size(pos2(full.center().x - w / 2.0, top), vec2(w, self.ard.dlg_h.min(full.bottom() - 16.0 - top)));
        let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if bg.clicked() && !card.contains(bg.interact_pointer_pos().unwrap_or_default()) {
            close = true;
        }
        ui.painter().add(egui::Shadow { offset: [0, 30], blur: 90, spread: 0, color: Color32::from_black_alpha(128) }.as_shape(card, CornerRadius::same(20)));
        ui.painter().rect_filled(card, CornerRadius::same(20), p.solid);
        ui.painter().rect_stroke(card, CornerRadius::same(20), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let inner = card.shrink2(vec2(26.0, 22.0));
        let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(inner.min, vec2(inner.width(), 2000.0))));
        cu.set_clip_rect(card);
        cu.spacing_mut().item_spacing.y = 10.0;
        let sk0 = self.ard_sketch(&target, &ctx_mode);
        let pins = sk0.pins.clone();
        let sk_files = sk0.files.clone();
        let used = pins.iter().position(|x| x.pin == sel);
        cu.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{} {sel}", t("Pin"))).font(theme::bold(20.0)).color(p.text));
            if let Some(k) = used {
                for (i, pt) in pins[k].parts.iter().enumerate() {
                    let m = pins[k].uses.iter().find(|u| ard::part_of(u.0) == *pt).map(|u| u.0).unwrap_or(pins[k].mode);
                    let (col, label, _) = mode_info(m);
                    let txt = if i == 0 { format!("#{} · {label}", k + 1) } else { format!("+ {label}") };
                    ui.label(egui::RichText::new(txt).font(theme::mono(12.0)).color(col));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::icon_button(ui, "x", &p, true).on_hover_text(t("Close (Esc)")).clicked() {
                    close = true;
                }
            });
        });
        if let Some(k) = used.filter(|k| pins[*k].conflict()) {
            let _ = k;
            cu.add(
                egui::Label::new(
                    egui::RichText::new(t("Two different parts use this pin in your code – they would get in each other's way. Pick a free pin for the new one.")).font(theme::ui(12.5)).color(p.red),
                )
                .wrap(),
            );
        }
        cu.label(egui::RichText::new(t("Pick a language and a starting point – Flux creates the file in your project.")).font(theme::ui(13.0)).color(p.text2));
        cu.add_space(4.0);
        cu.label(egui::RichText::new(t("LANGUAGE")).font(theme::bold(11.0)).color(p.text3));
        let langs = [("cpp", "C++ (.ino)"), ("micropython", "MicroPython"), ("circuitpython", "CircuitPython"), ("firmata", "Python + Firmata")];
        cu.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for (m, label) in langs {
                let on = self.ard.dlg_mode == m;
                if widgets::button(ui, None, label, if on { p.active } else { p.card2 }, p.text, 30.0, &p).clicked() {
                    self.ard.dlg_mode = m.to_string();
                }
            }
        });
        let hint = match self.ard.dlg_mode.as_str() {
            "firmata" => Some(t(
                "Python + Firmata: Run on board changes only this pin live (the board needs StandardFirmata once); Upload converts the program to C++ so it keeps running without the computer.",
            )),
            "micropython" | "circuitpython" if self.ard_cpp_board() => Some(t("On Arduino boards Flux converts the Python to C++ when you upload (simple programs only).")),
            "cpp" => Some(t("C++ replaces the whole program on the board when you upload.")),
            _ => None,
        };
        if let Some(h) = hint {
            cu.add(egui::Label::new(egui::RichText::new(h).font(theme::ui(12.0)).color(p.text3)).wrap());
        }
        cu.label(egui::RichText::new(t("START WITH")).font(theme::bold(11.0)).color(p.text3));
        let kinds: Vec<(&str, String)> =
            [("", "Empty file"), ("led", "Blink an LED"), ("button", "Read a button"), ("pwm", "Fade (PWM)"), ("analog", "Analog sensor"), ("servo", "Servo"), ("buzzer", "Buzzer")]
                .into_iter()
                .filter(|(k, _)| match (layout, *k) {
                    (0 | 3 | 4, "analog") => sel.starts_with('A'),
                    (0 | 4, "pwm") => ["3", "5", "6", "9", "10", "11"].contains(&sel.as_str()),
                    (3, "pwm") => sel.parse::<u32>().is_ok_and(|n| (2..=13).contains(&n) || (44..=46).contains(&n)),
                    _ => true,
                })
                .map(|(k, l)| (k, t(l)))
                .collect();
        cu.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for (k, label) in &kinds {
                let glyph = if k.is_empty() { "file" } else { k };
                if dash::preset(ui, glyph, label, self.ard.dlg_kind == *k).clicked() {
                    self.ard.dlg_kind = k.to_string();
                }
            }
        });
        let ext = ard::base_text(&self.ard.dlg_mode, "").0;
        let can_add = !target.is_empty() && !self.ard.dlg_kind.is_empty() && ctx_mode == self.ard.dlg_mode;
        let new_open = self.ard.dlg_new || !can_add;
        let mut new_name = std::mem::take(&mut self.ard.new_name);
        if new_open {
            cu.label(egui::RichText::new(t("FILE NAME")).font(theme::bold(11.0)).color(p.text3));
            cu.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let tw = (ui.available_width() - 60.0).max(90.0);
                ui.add(egui::TextEdit::singleline(&mut new_name).hint_text(format!("pin_{}", sel.to_lowercase())).font(theme::ui(13.5)).desired_width(tw).margin(egui::Margin::symmetric(10, 8)));
                ui.label(egui::RichText::new(format!(".{ext}")).font(theme::mono(13.0)).color(p.text3));
            });
        } else {
            cu.add(
                egui::Label::new(egui::RichText::new(tf("The code goes into {name}, which you already have.", &[("name", &widgets::file_name(&target))])).font(theme::ui(12.5)).color(p.text2)).wrap(),
            );
        }
        self.ard.new_name = new_name.clone();
        let (mut create, mut add, mut remove) = (false, false, false);
        cu.add_space(4.0);
        cu.horizontal(|ui| {
            if can_add {
                let add_label = tf("Add to {name}", &[("name", &widgets::file_name(&target))]);
                add = widgets::button(ui, Some("plus"), &add_label, if new_open { p.card2 } else { p.accent }, if new_open { p.text } else { p.accent_fg }, 34.0, &p).clicked();
            }
            if new_open {
                create = widgets::button(ui, Some("filePlus"), &t("Create file"), if can_add { p.card2 } else { p.accent }, if can_add { p.text } else { p.accent_fg }, 34.0, &p).clicked();
            } else if widgets::button(ui, Some("filePlus"), &t("New file instead"), p.card2, p.text, 34.0, &p).clicked() {
                self.ard.dlg_new = true;
            }
            if widgets::button(ui, None, &t("Cancel"), p.card2, p.text, 34.0, &p).clicked() {
                close = true;
            }
            if used.is_some() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::button(ui, Some("trash"), &t("Remove from code"), p.red.gamma_multiply(0.18), p.red, 34.0, &p)
                        .on_hover_text(t("Deletes the lines of this pin from your code (you can undo it)"))
                        .clicked()
                    {
                        remove = true;
                    }
                });
            }
        });
        let h = cu.min_rect().height() + 44.0;
        if (h - self.ard.dlg_h).abs() > 0.5 {
            self.ard.dlg_h = h;
            ui.ctx().request_repaint();
        }
        let mode = self.ard.dlg_mode.clone();
        let kind = self.ard.dlg_kind.clone();
        if remove {
            let fs: Vec<String> = sk_files.iter().map(|f| f.1.clone()).collect();
            self.ard_remove_pin(&sel, &ctx_mode, &fs);
            close = true;
        } else if add {
            self.status = match self.ard_edit_target(&target, |txt| ard::add_snippet(&mode, txt, &sel, &kind, layout == 2)) {
                Some(name) => tf("Added to {name} – the code is in the editor", &[("name", &name)]),
                None => t("That is already in the code."),
            };
            close = true;
        } else if create {
            let port = self.ard_port().unwrap_or_else(|| "/dev/ttyUSB0".into());
            let (ext, content) = ard::new_for_pin(&mode, &sel, &kind, &port, layout == 2);
            let typed = new_name.trim().trim_end_matches(&format!(".{ext}")).to_string();
            let clean: String = typed.chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '.') { c } else { '_' }).collect();
            let base = if clean.trim().is_empty() { format!("pin_{}", sel.to_lowercase()) } else { clean.trim().to_string() };
            let dir = std::path::PathBuf::from(dir);
            let mut path = dir.join(format!("{base}.{ext}"));
            let mut n = 2;
            while path.exists() {
                path = dir.join(format!("{base}-{n}.{ext}"));
                n += 1;
            }
            match std::fs::write(&path, content) {
                Ok(()) => {
                    self.reload_projects();
                    self.open_file(&path.to_string_lossy());
                    self.ard.new_name.clear();
                    self.status = tf("Created {name}", &[("name", &widgets::file_name(&path.to_string_lossy()))]);
                    close = true;
                }
                Err(e) => self.status = e.to_string(),
            }
        }
        if close {
            self.ard.pin_dlg = false;
            self.ard.sel = None;
            self.ard.dlg_mode.clear();
            self.ard.dlg_new = false;
        }
    }

    fn ard_head_ui(&mut self, ui: &mut egui::Ui, mode: &str) {
        let target = self.tabs.get(self.active).map(|t| t.path.clone()).unwrap_or_default();
        let sk = self.ard_sketch(&target, mode);
        let pins = sk.pins.clone();
        let dir = std::path::Path::new(&target).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        self.ard.ctx = (dir, mode.to_string(), target.clone());
        self.ard_picture(ui, mode, &pins);
        self.ard_pin_panel(ui, mode, &pins, true);
        // použité piny: kompaktný zoznam
        let n = pins.len();
        let open = self.ard_sec(ui, "pins", &t("Pins in your code"), true, (n > 0).then(|| n.to_string()));
        if open {
            self.ard_pins_rows(ui, mode, &pins, &sk.files);
        }
    }

    // riadky použitých pinov (hover = čo k nemu pripojiť, klik = skok na riadok v kóde)
    fn ard_pins_rows(&mut self, ui: &mut egui::Ui, mode: &str, pins: &[ard::Pin], files: &[(usize, String)]) {
        let p = self.pal;
        if pins.is_empty() {
            let hint = if mode == "cpp" {
                "Use pinMode(), digitalWrite(), analogRead()… in your code and the wiring appears here."
            } else {
                "Use Pin(), PWM(), ADC() (or digitalio, get_pin…) in your code and the wiring appears here."
            };
            self.ard_hint(ui, &t(hint));
        }
        if !self.ard.undo.is_empty() {
            let mut undo = false;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tf("Pin {pin} removed from the code", &[("pin", &self.ard.undo_pin)])).font(theme::ui(12.0)).color(p.text2));
                undo = widgets::button(ui, Some("undo"), &t("Undo"), p.card2, p.text, 24.0, &p).clicked();
            });
            if undo {
                self.ard_undo_remove();
            }
        }
        let mut jump: Option<usize> = None;
        let mut remove: Option<String> = None;
        for (k, pin) in pins.iter().enumerate() {
            let (col, pmode, part) = mode_info(pin.mode);
            let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rr, CornerRadius::same(8), p.hover);
            }
            let c = pos2(rr.left() + 14.0, rr.center().y);
            boardart::bubble(ui.painter(), c, 10.0, col, pin, k + 1);
            let mut title = match &pin.name {
                Some(nm) => format!("{} {}  ·  {nm}", t("Pin"), pin.pin),
                None => format!("{} {}", t("Pin"), pin.pin),
            };
            // viac súborov v sketchi: odkiaľ pin je
            if files.len() > 1 {
                if let Some(f) = files.iter().rev().find(|f| f.0 <= pin.line) {
                    title += &format!("  ·  {}", widgets::file_name(&f.1));
                }
            }
            // poradové číslo (rovnaké ako na doske) pred názvom
            ui.painter().text(pos2(rr.left() + 34.0, rr.center().y), Align2::LEFT_CENTER, format!("{}.", k + 1), theme::bold(12.5), col);
            let tr = widgets::text(ui, pos2(rr.left() + 50.0, rr.center().y), Align2::LEFT_CENTER, &title, theme::bold(12.5), p.text, rr.width() - 164.0);
            // ďalšie súčiastky na tom istom pine: malé ikony, konflikt červený výkričník
            let mut x = tr.right() + 10.0;
            for extra in pin.parts.iter().skip(1) {
                let m = pin.uses.iter().find(|u| ard::part_of(u.0) == *extra).map(|u| u.0).unwrap_or("");
                dash::glyph(ui.painter(), dash::mode_glyph(m), pos2(x + 6.0, rr.center().y), mode_info(m).0, 0.62);
                x += 16.0;
            }
            if pin.conflict() || pin.warns.iter().any(|w| *w != "unused") {
                ui.painter().text(pos2(x + 2.0, rr.center().y), Align2::LEFT_CENTER, "!", theme::bold(13.0), p.red);
            }
            let _ = part;
            ui.painter().text(pos2(rr.right() - 8.0, rr.center().y), Align2::RIGHT_CENTER, pmode, theme::mono(11.0), col);
            // × = odobrať pin z kódu (zobrazí sa pri prejdení myšou)
            let xr = Rect::from_center_size(pos2(rr.right() - 112.0, rr.center().y), vec2(22.0, 22.0));
            let xh = ui.interact(xr, resp.id.with("x"), Sense::click());
            let shown = resp.hovered() || xh.hovered();
            if shown {
                if xh.hovered() {
                    ui.painter().rect_filled(xr, CornerRadius::same(6), p.red.gamma_multiply(0.2));
                }
                widgets::icon_at(ui, xr.center(), 12.0, "x", if xh.hovered() { p.red } else { p.text3 });
            }
            if xh.on_hover_text(tf("Remove pin {pin} from the code", &[("pin", &pin.pin)])).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                remove = Some(pin.pin.clone());
            } else if resp.on_hover_text(boardart::bubble_tip(pin)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                jump = Some(pin.line);
            }
        }
        if let Some(pn) = remove {
            let fs: Vec<String> = if files.is_empty() { self.tabs.get(self.active).map(|t| vec![t.path.clone()]).unwrap_or_default() } else { files.iter().map(|f| f.1.clone()).collect() };
            self.ard_remove_pin(&pn, mode, &fs);
        }
        if let Some(l) = jump {
            // riadok v spojenom sketchi → súbor a riadok v ňom
            match files.iter().rev().find(|f| f.0 <= l) {
                Some((start, f)) => {
                    if self.tabs.get(self.active).map(|t| &t.path) != Some(f) {
                        self.open_file(f);
                    }
                    self.lint_jump = Some(l - start);
                }
                None => self.lint_jump = Some(l),
            }
        }
    }

    // piny celého sketchu (C++: všetky .ino v priečinku, hlavný súbor prvý; Python: len súbor) s kontrolou podľa dosky; kešuje sa 1 s
    pub(super) fn ard_sketch(&mut self, path: &str, mode: &str) -> Sketch {
        let fq = self.ard_board();
        let key = format!("{path}|{mode}|{fq}");
        if let Some((k, at, sk)) = &self.ard.sketch {
            if *k == key && at.elapsed() < Duration::from_millis(1000) {
                return sk.clone();
            }
        }
        let mut sk = Sketch::default();
        if !path.is_empty() {
            let mut files = vec![path.to_string()];
            if mode == "cpp" {
                if let Some(dir) = std::path::Path::new(path).parent() {
                    let mut more: Vec<String> =
                        std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path().to_string_lossy().to_string()).filter(|f| f.ends_with(".ino") && f != path).collect();
                    more.sort();
                    files.extend(more);
                }
            }
            let mut text = String::new();
            let mut line = 0;
            for f in &files {
                let t = self.ard_target_text(f);
                sk.files.push((line, f.clone()));
                for l in t.lines() {
                    text.push_str(l);
                    text.push('\n');
                    line += 1;
                }
            }
            sk.pins = ard::pins_for(mode, &text);
            if mode == "cpp" {
                ard::check(&mut sk.pins, &fq, text.contains("Serial."));
                sk.includes = ard::includes(&text);
            }
        }
        self.ard.sketch = Some((key, Instant::now(), sk.clone()));
        sk
    }

    // projekt s doskou: súbor, ktorý sa nahráva na dosku (prvé .ino, inak Python s importmi pre dosku) a jeho jazyk; kešuje sa 2 s
    pub(super) fn ard_project_board(&mut self, ws: &str, items: &[(String, String, bool)]) -> Option<(String, String)> {
        if let Some((w, at, r)) = &self.ard.proj {
            if w == ws && at.elapsed() < Duration::from_secs(2) {
                return r.clone();
            }
        }
        let mut files: Vec<&(String, String, bool)> = items.iter().filter(|i| !i.2).collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let mut found = files.iter().find(|i| i.0.ends_with(".ino")).map(|i| (i.1.clone(), "cpp".to_string()));
        if found.is_none() {
            let order = ["main.py", "code.py", "firmata.py"];
            let mut py: Vec<&&(String, String, bool)> = files.iter().filter(|i| i.0.ends_with(".py")).collect();
            py.sort_by_key(|i| order.iter().position(|o| *o == i.0).unwrap_or(9));
            found = py.iter().find_map(|i| {
                let text = std::fs::read(&i.1).ok().map(|b| String::from_utf8_lossy(&b[..b.len().min(16384)]).to_string())?;
                ard::guess_py(&text).map(|m| (i.1.clone(), m.to_string()))
            });
        }
        self.ard.proj = Some((ws.to_string(), Instant::now(), found.clone()));
        found
    }

    // názov modelu dosky (zvolenej alebo rozpoznanej)
    fn ard_model_name(&self) -> String {
        let fq = self.ard_board();
        BOARDS.iter().find(|b| b.0 == fq).map(|b| t(b.1)).unwrap_or(fq)
    }

    // pravý stĺpec stránky projektu: kontrola, piny, súčiastky, knižnice
    fn ard_project_sections(&mut self, ui: &mut egui::Ui, mode: &str, path: &str, sk: &Sketch) {
        let p = self.pal;
        let cap = |ui: &mut egui::Ui, s: String| {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(s).font(theme::bold(10.5)).color(p.text3));
        };
        let fq = self.ard_board();
        let pwm = ard::pwm_list(&fq);
        // kontrola pinov podľa dosky
        if mode == "cpp" && !sk.pins.is_empty() {
            let mut probs: Vec<(String, String)> = vec![];
            for pin in &sk.pins {
                if pin.conflict() {
                    probs.push((pin.pin.clone(), tf("Pin {pin}: two different parts use it.", &[("pin", &pin.pin)])));
                }
                for w in &pin.warns {
                    probs.push((pin.pin.clone(), warn_text(w, &pin.pin, &pwm)));
                }
            }
            cap(ui, t("CHECK"));
            if probs.is_empty() {
                if ard::caps(&fq).is_some() {
                    ui.label(egui::RichText::new(format!("✓ {}", t("The pins fit this board."))).font(theme::ui(12.5)).color(p.green));
                } else {
                    self.ard_hint(ui, &t("Pin checks are only available for Arduino boards (Uno, Nano, Mega…)."));
                }
            }
            for (pin, text) in probs {
                ui.horizontal_top(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(26.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(5), p.red.gamma_multiply(0.85));
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, &pin, theme::bold(10.5), Color32::WHITE);
                    ui.add(egui::Label::new(egui::RichText::new(text).font(theme::ui(12.5)).color(p.text)).wrap());
                });
            }
        }
        // piny
        if !sk.pins.is_empty() {
            let title = if sk.files.len() > 1 { tf("PINS IN {name}", &[("name", &t("THE SKETCH"))]) } else { tf("PINS IN {name}", &[("name", &widgets::file_name(path).to_uppercase())]) };
            cap(ui, title);
            self.ard_pins_rows(ui, mode, &sk.pins, &sk.files);
        }
        // súčiastky
        let parts = ard::parts_list(&sk.pins);
        if !parts.is_empty() {
            cap(ui, t("YOU NEED"));
            for (pt, n) in &parts {
                ui.label(egui::RichText::new(format!("{n} × {}", part_label(pt))).font(theme::ui(12.5)).color(p.text2));
            }
            ui.label(egui::RichText::new(format!("1 × {}", t("Breadboard and jumper wires"))).font(theme::ui(12.5)).color(p.text2));
        }
        // knižnice z #include
        if !sk.includes.is_empty() {
            cap(ui, t("LIBRARIES"));
            let cli = self.ard_cli();
            if let Some(c) = &cli {
                if self.ard.installed.lock().unwrap().is_none() {
                    let (slot, c2) = (self.ard.installed.clone(), c.clone());
                    *slot.lock().unwrap() = Some(vec![]);
                    std::thread::spawn(move || {
                        let r = ard::lib_installed(&c2);
                        *slot.lock().unwrap() = Some(r);
                    });
                }
            }
            let installed = self.ard.installed.lock().unwrap().clone().unwrap_or_default();
            let mut install: Option<String> = None;
            for (h, lib) in &sk.includes {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(h).font(theme::mono(12.0)).color(p.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match lib {
                        None => {
                            ui.label(egui::RichText::new(t("built in")).font(theme::ui(12.0)).color(p.text3));
                        }
                        Some(l) if installed.iter().any(|x| x.0.eq_ignore_ascii_case(l)) => {
                            ui.label(egui::RichText::new(format!("✓ {}", t("Installed"))).font(theme::ui(12.0)).color(p.green));
                        }
                        Some(l) => {
                            if cli.is_some() {
                                if widgets::button(ui, Some("download"), &t("Install"), p.card2, p.text, 26.0, &p).on_hover_text(l).clicked() {
                                    install = Some(l.clone());
                                }
                            } else {
                                ui.label(egui::RichText::new(t("Needs the Arduino tools")).font(theme::ui(12.0)).color(p.text3));
                            }
                        }
                    });
                });
            }
            if let (Some(name), Some(c)) = (install, cli) {
                self.ard_run(&tf("Installing the {name} library", &[("name", &name)]), vec![ard::v(&c, &["lib", "install", &name])]);
                *self.ard.installed.lock().unwrap() = None;
            }
        }
    }

    // Doska na stránke projektu bez rámikov: meno a pripojenie hore, veľká doska, vpravo výber dosky a použité piny.
    // Bez súboru (`path` prázdna) ukáže „Board is empty“ s tlačidlom na prvý program.
    pub(super) fn ard_project_card(&mut self, ui: &mut egui::Ui, path: &str, mode: &str, ws: &str) {
        dash::set(&self.pal);
        let p = self.pal;
        let ctx = ui.ctx().clone();
        self.ard_scan(&ctx);
        ctx.request_repaint_after(Duration::from_millis(1000));
        let sk = self.ard_sketch(path, mode);
        let pins = sk.pins.clone();
        let dir = if path.is_empty() { ws.to_string() } else { std::path::Path::new(path).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default() };
        self.ard.ctx = (dir, mode.to_string(), path.to_string());
        let name = self.ard_model_name();
        let (kind, _, lport) = self.ard_link();
        ui.set_width(ui.available_width());
        let (tr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
        let boards_empty = self.ard.boards.lock().unwrap().is_empty();
        let model = if mode == "cpp" { name } else { self.ard_model_name_py(mode) };
        ui.painter().text(pos2(tr.left(), tr.top() + 16.0), Align2::LEFT_CENTER, &model, theme::bold(26.0), p.text);
        let (col, status) = match kind {
            1 => (p.green, tf("Connected via USB · {port}", &[("port", &lport)])),
            2 => (Color32::from_rgb(0x4d, 0xa3, 0xff), tf("Connected via Bluetooth · {port}", &[("port", &lport)])),
            _ => (p.text3, if boards_empty { t("Offline – plug the board in with a USB cable") } else { t("Offline") }),
        };
        ui.painter().circle_filled(pos2(tr.left() + 5.0, tr.top() + 42.0), 4.0, col);
        ui.painter().text(pos2(tr.left() + 16.0, tr.top() + 42.0), Align2::LEFT_CENTER, status, theme::ui(13.0), p.text2);
        let rows = self.ard_setup_rows(mode);
        if !rows.iter().all(|r| r.ok) {
            let (done, total) = (rows.iter().filter(|r| r.ok).count(), rows.len());
            let mut open = false;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t("Getting started")).font(theme::bold(13.0)).color(p.accent));
                ui.label(egui::RichText::new(tf("{done} of {total} steps done", &[("done", &done.to_string()), ("total", &total.to_string())])).font(theme::ui(12.5)).color(p.text2));
                open = widgets::button(ui, Some("rocket"), &t("Open the guide"), p.accent, p.accent_fg, 28.0, &p).clicked();
            });
            if open {
                self.ard_open_setup(mode);
            }
            ui.add_space(6.0);
        }
        let total = ui.available_width();
        let narrow = total < 760.0;
        let lw = if narrow { total } else { (total * 0.64).min(700.0) };
        let rw = if narrow { total } else { total - lw - 28.0 };
        let empty = path.is_empty();
        let layout = self.ard_layout(mode);
        let right = |this: &mut Self, ui: &mut egui::Ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            this.ard_board_select(ui);
            if empty {
                egui::Frame::NONE.fill(p.hover).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(14)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(t("Board is empty")).font(theme::bold(15.0)).color(p.text));
                    ui.add(egui::Label::new(egui::RichText::new(t("Add your first program: click a pin on the board, or start with the button.")).font(theme::ui(12.5)).color(p.text2)).wrap());
                    if widgets::button(ui, Some("plus"), &t("Add something"), p.accent, p.accent_fg, 32.0, &p).clicked() {
                        this.ard.sel = Some(
                            match layout {
                                1 => "2",
                                2 => "25",
                                _ => "13",
                            }
                            .to_string(),
                        );
                        this.ard.pin_dlg = true;
                        this.ard.dlg_mode = this.ard.ctx.1.clone();
                        this.ard.dlg_kind = "led".into();
                        this.ard.dlg_new = false;
                    }
                });
            } else {
                this.ard_pin_panel(ui, mode, &pins, false);
                this.ard_project_sections(ui, mode, path, &sk);
            }
        };
        if narrow {
            self.ard_picture(ui, mode, &pins);
            ui.add_space(8.0);
            right(self, ui);
        } else {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 28.0;
                ui.allocate_ui_with_layout(vec2(lw, 10.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(lw);
                    self.ard_picture(ui, mode, &pins);
                });
                ui.allocate_ui_with_layout(vec2(rw, 10.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(rw);
                    right(self, ui);
                });
            });
        }
    }

    // názov pre Python na doske (model dosky je tu menej dôležitý ako jazyk)
    fn ard_model_name_py(&self, mode: &str) -> String {
        let lang = match mode {
            "micropython" => "MicroPython",
            "circuitpython" => "CircuitPython",
            _ => "Python + Firmata",
        };
        let n = self.ard_model_name();
        if self.ard_layout(mode) == 0 && mode != "firmata" {
            lang.to_string()
        } else {
            format!("{lang} · {n}")
        }
    }

    // zbaliteľná sekcia: hlavička s šípkou, nadpisom a odznakom; vráti, či je rozbalená
    fn ard_sec(&mut self, ui: &mut egui::Ui, id: &'static str, title: &str, default_open: bool, badge: Option<String>) -> bool {
        let p = self.pal;
        let open = default_open != self.ard.flipped.contains(id);
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(8), p.hover);
        }
        // šípka (kreslená, aby sa dala otáčať)
        let c = pos2(r.left() + 11.0, r.center().y);
        let (a, b, d) = if open { (vec2(-3.5, -1.8), vec2(0.0, 1.8), vec2(3.5, -1.8)) } else { (vec2(-1.8, -3.5), vec2(1.8, 0.0), vec2(-1.8, 3.5)) };
        let st = Stroke::new(1.6, p.text3);
        ui.painter().line_segment([c + a, c + b], st);
        ui.painter().line_segment([c + b, c + d], st);
        ui.painter().text(pos2(r.left() + 24.0, r.center().y), Align2::LEFT_CENTER, title.to_uppercase(), theme::bold(11.0), p.text2);
        if let Some(bd) = badge {
            let w = widgets::text_w(ui, &bd, theme::bold(10.5)) + 12.0;
            let br = Rect::from_center_size(pos2(r.right() - 8.0 - w / 2.0, r.center().y), vec2(w, 17.0));
            ui.painter().rect_filled(br, CornerRadius::same(8), p.card2);
            ui.painter().text(br.center(), Align2::CENTER_CENTER, bd, theme::bold(10.5), p.text2);
        }
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !self.ard.flipped.remove(id) {
            self.ard.flipped.insert(id);
        }
        open
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
