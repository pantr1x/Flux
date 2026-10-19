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
    esp: usize,                     // čip pre nahratie MicroPythonu
    flipped: HashSet<&'static str>, // sekcie, ktoré používateľ prepol oproti predvolenému stavu
    custom: String,                 // vlastný kód pre pin
    custom_open: bool,
    custom_repeat: bool,                         // do loop() / slučky (inak raz, do setup())
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
            flipped: HashSet::new(),
            custom: String::new(),
            custom_open: false,
            custom_repeat: true,
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

// pin dosky na obrázku: popisok, kľúč pre kód (číslo GPIO; prázdny = nejde použiť), druh: 0 GPIO, 1 GND, 2 napájanie, 3 iné
#[derive(Clone, Copy)]
struct Pad {
    label: &'static str,
    key: &'static str,
    kind: u8,
}

const fn pd(label: &'static str, key: &'static str, kind: u8) -> Pad {
    Pad { label, key, kind }
}

// ESP32 DevKitC (38 pinov), USB dole
const ESP_L: [Pad; 19] = [
    pd("3V3", "", 2),
    pd("EN", "", 3),
    pd("VP", "36", 0),
    pd("VN", "39", 0),
    pd("34", "34", 0),
    pd("35", "35", 0),
    pd("32", "32", 0),
    pd("33", "33", 0),
    pd("25", "25", 0),
    pd("26", "26", 0),
    pd("27", "27", 0),
    pd("14", "14", 0),
    pd("12", "12", 0),
    pd("GND", "", 1),
    pd("13", "13", 0),
    pd("SD2", "", 3),
    pd("SD3", "", 3),
    pd("CMD", "", 3),
    pd("5V", "", 2),
];
const ESP_R: [Pad; 19] = [
    pd("GND", "", 1),
    pd("23", "23", 0),
    pd("22", "22", 0),
    pd("TX0", "1", 0),
    pd("RX0", "3", 0),
    pd("21", "21", 0),
    pd("GND", "", 1),
    pd("19", "19", 0),
    pd("18", "18", 0),
    pd("5", "5", 0),
    pd("17", "17", 0),
    pd("16", "16", 0),
    pd("4", "4", 0),
    pd("0", "0", 0),
    pd("2", "2", 0),
    pd("15", "15", 0),
    pd("SD1", "", 3),
    pd("SD0", "", 3),
    pd("CLK", "", 3),
];
// Raspberry Pi Pico, USB hore
const PICO_L: [Pad; 20] = [
    pd("GP0", "0", 0),
    pd("GP1", "1", 0),
    pd("GND", "", 1),
    pd("GP2", "2", 0),
    pd("GP3", "3", 0),
    pd("GP4", "4", 0),
    pd("GP5", "5", 0),
    pd("GND", "", 1),
    pd("GP6", "6", 0),
    pd("GP7", "7", 0),
    pd("GP8", "8", 0),
    pd("GP9", "9", 0),
    pd("GND", "", 1),
    pd("GP10", "10", 0),
    pd("GP11", "11", 0),
    pd("GP12", "12", 0),
    pd("GP13", "13", 0),
    pd("GND", "", 1),
    pd("GP14", "14", 0),
    pd("GP15", "15", 0),
];
const PICO_R: [Pad; 20] = [
    pd("VBUS", "", 2),
    pd("VSYS", "", 2),
    pd("GND", "", 1),
    pd("3V3_EN", "", 3),
    pd("3V3", "", 2),
    pd("VREF", "", 3),
    pd("GP28", "28", 0),
    pd("GND", "", 1),
    pd("GP27", "27", 0),
    pd("GP26", "26", 0),
    pd("RUN", "", 3),
    pd("GP22", "22", 0),
    pd("GND", "", 1),
    pd("GP21", "21", 0),
    pd("GP20", "20", 0),
    pd("GP19", "19", 0),
    pd("GP18", "18", 0),
    pd("GND", "", 1),
    pd("GP17", "17", 0),
    pd("GP16", "16", 0),
];

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

    fn ard_cp_copy(&mut self, path: &str) {
        self.status = match ard::circuitpy_drive() {
            Some(d) => match std::fs::copy(path, d.join("code.py")) {
                Ok(_) => tf("Copied to {drive} as code.py – the board restarts by itself", &[("drive", &d.to_string_lossy())]),
                Err(e) => e.to_string(),
            },
            None => t("The CIRCUITPY drive was not found – plug the board in and wait a moment."),
        };
    }

    // ▶ (F5) pri Pythone na doske: MicroPython `mpremote run` (save = uložiť ako main.py), CircuitPython skopíruje súbor ako code.py
    pub(super) fn ard_py_run(&mut self, save: bool) {
        let Some(path) = self.tabs.get(self.active).map(|t| t.path.clone()) else { return };
        if self.ard_mode().as_deref() == Some("circuitpython") {
            return self.ard_cp_copy(&path);
        }
        let Some(mp) = self.ard.tools.lock().unwrap().mp.clone() else {
            self.ard.open = true;
            self.ard.section = 0;
            self.status = t("Install mpremote first (board panel → Install).");
            return;
        };
        let Some(port) = self.ard_port() else {
            self.ard.open = true;
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
        // stav pripojenia: USB / Bluetooth / offline
        let (kind, lname, lport) = self.ard_link();
        let short_port = lport.rsplit('/').next().unwrap_or("").to_string();
        let (col, label, tip) = match kind {
            1 => (p.green, format!("USB · {short_port}"), format!("{lname} ({lport})")),
            2 => (Color32::from_rgb(0x4d, 0xa3, 0xff), format!("Bluetooth · {}", lname.chars().take(14).collect::<String>()), format!("{lname} ({lport})")),
            _ => (p.text3, t("Offline"), t("Offline – no board connected")),
        };
        let pw = widgets::text_w(ui, &label, theme::bold(11.5)) + 28.0;
        let pill = Rect::from_center_size(pos2(head.right() - 44.0 - pw / 2.0, head.center().y), vec2(pw, 24.0));
        ui.painter().rect_filled(pill, CornerRadius::same(12), p.hover);
        ui.painter().circle_filled(pos2(pill.left() + 12.0, pill.center().y), 4.0, col);
        ui.painter().text(pos2(pill.left() + 22.0, pill.center().y), Align2::LEFT_CENTER, &label, theme::bold(11.5), if kind == 0 { p.text2 } else { p.text });
        ui.interact(pill, ui.id().with("ard-link"), Sense::hover()).on_hover_text(tip);
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
        egui::ScrollArea::vertical().id_salt("ard-body").auto_shrink(false).show(&mut bui, |ui| {
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
        egui::Frame::NONE.fill(p.hover).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
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
    fn ard_setup_ui(&mut self, ui: &mut egui::Ui, mode: &str) -> Option<&'static str> {
        let p = self.pal;
        struct Row {
            ok: bool,
            title: String,
            detail: String,
            red: bool,
            busy: bool,
            act: Option<(&'static str, String)>,
            copy: Option<String>,
        }
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
        if rows.iter().all(|r| r.ok) {
            return None;
        }
        let mut act: Option<&'static str> = None;
        let mut copy: Option<String> = None;
        self.ard_card(ui, |ui| {
            ui.label(egui::RichText::new(t("GETTING READY")).font(theme::bold(11.0)).color(p.text3));
            for r in &rows {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let (cr, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                    if r.ok {
                        ui.painter().circle_filled(cr.center(), 7.5, p.green);
                        widgets::icon_at(ui, cr.center(), 10.0, "check", Color32::BLACK);
                    } else if r.busy {
                        ui.painter().circle_stroke(cr.center(), 7.0, Stroke::new(1.5, p.accent));
                    } else {
                        ui.painter().circle_stroke(cr.center(), 7.0, Stroke::new(1.5, if r.red { p.red } else { p.text3 }));
                    }
                    ui.label(egui::RichText::new(&r.title).font(theme::bold(12.5)).color(if r.ok { p.text2 } else { p.text }));
                    if r.busy {
                        ui.spinner();
                    }
                });
                if !r.detail.is_empty() {
                    egui::Frame::NONE.inner_margin(egui::Margin { left: 24, right: 0, top: 0, bottom: 0 }).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(egui::Label::new(egui::RichText::new(&r.detail).font(theme::ui(11.5)).color(if r.red { p.red } else { p.text2 })).wrap());
                        if let Some(c) = &r.copy {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(c).font(theme::mono(11.5)).color(p.text));
                                if widgets::icon_button(ui, "copy", &p, true).on_hover_text(t("Copy")).clicked() {
                                    copy = Some(c.clone());
                                }
                            });
                        }
                        if let Some((a, label)) = &r.act {
                            if widgets::button(ui, Some("download"), label, p.accent, p.accent_fg, 26.0, &p).clicked() {
                                act = Some(*a);
                            }
                        }
                    });
                }
            }
        });
        if let Some(c) = copy {
            ui.ctx().copy_text(c);
        }
        act
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
                    ui.label(egui::RichText::new(t("This board is too small for MicroPython")).font(theme::bold(13.0)).color(p.red));
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(t("Arduino Uno, Nano, Mega and Elegoo boards have only 2–8 KB of memory – Python can't run on them. Use Python + Firmata (Python runs on your computer and controls the board) or C++."))
                                .font(theme::ui(12.0))
                                .color(p.text2),
                        )
                        .wrap(),
                    );
                    if ch {
                        ui.add(egui::Label::new(egui::RichText::new(t("If this is an ESP32 board with a CH340 chip, ignore this.")).font(theme::ui(11.5)).color(p.text3)).wrap());
                    }
                    to_firmata = widgets::button(ui, None, &t("Switch to Python + Firmata"), p.accent, p.accent_fg, 28.0, &p).clicked();
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
            "upload" => self.ard_upload(true),
            "verify" => self.ard_upload(false),
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
        let by_fqbn = |f: &str| match f.split(':').next() {
            Some("esp32" | "esp8266") => Some(1),
            Some("rp2040") => Some(2),
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
                } else {
                    0
                }
            }
            None if mode == "cpp" => by_fqbn(&fq).unwrap_or(0),
            None if mode == "micropython" => 1,
            None => 0,
        }
    }

    // hlavička panela: obrázok dosky so stavom pripojenia, výber dosky (automaticky / vlastná), piny, vlastný kód a jazyk
    fn ard_head_ui(&mut self, ui: &mut egui::Ui, mode: &str) {
        let p = self.pal;
        let code = self.tabs.get(self.active).map(|t| t.text.clone()).unwrap_or_default();
        let pins = ard::pins_for(mode, &code);
        let layout = self.ard_layout(mode);
        let w = ui.available_width();
        let sel = self.ard.sel.clone();
        let clicked = match layout {
            1 => self.ard_draw_vertical(ui, w, "ESP32", &ESP_L, &ESP_R, false, Color32::from_rgb(0x23, 0x25, 0x2b), &pins, &sel),
            2 => self.ard_draw_vertical(ui, w, "Pico", &PICO_L, &PICO_R, true, Color32::from_rgb(0x1c, 0x7a, 0x44), &pins, &sel),
            _ => self.ard_draw_uno(ui, w, &pins, &sel),
        };
        if let Some(c) = clicked {
            self.ard.sel = if sel.as_deref() == Some(c.as_str()) { None } else { Some(c) };
        }
        // doska: automaticky podľa pripojenej, alebo vlastná
        let auto = self.get("arduinoBoardAuto").as_bool() != Some(false);
        let fq = self.ard_board();
        let mut opts: Vec<String> = vec![t("Automatic (detected)")];
        opts.extend(BOARDS.iter().map(|b| t(b.1)));
        let cur_name = BOARDS.iter().find(|b| b.0 == fq).map(|b| t(b.1)).unwrap_or(fq.clone());
        let cur_label = if auto { format!("{}: {} · {cur_name}", t("Board"), t("Automatic")) } else { format!("{}: {cur_name}", t("Board")) };
        if let Some(i) = widgets::select(ui, egui::Id::new("ard-board"), &cur_label, &opts, w, &p) {
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
        if layout != 0 {
            self.ard_hint(ui, &t("ESP32 and Pico pins work at 3.3 V – connect parts to 3V3, not 5V."));
        } else if mode == "cpp" && fq.contains("mega") {
            self.ard_hint(ui, &t("The picture shows the Uno layout – pins 0–13 and A0–A5 are in the same places on the Mega; other pins are listed below."));
        } else if mode == "cpp" && (fq.contains("nano") || fq.contains("pro")) {
            self.ard_hint(ui, &t("The picture shows the Uno layout – the Nano and Pro Mini have the same pin numbers, printed next to each pin."));
        }
        self.ard_hint(ui, &t("Click a pin to add LED, button, servo… code for it."));
        // vybraný pin → hotové ukážky alebo vlastný kód
        if let Some(sel) = self.ard.sel.clone() {
            let used = pins.iter().find(|x| x.pin == sel).map(|x| mode_info(x.mode).1);
            let kinds: Vec<(&str, String)> = [("led", "LED"), ("button", "Button"), ("pwm", "Fade (PWM)"), ("analog", "Analog sensor"), ("servo", "Servo"), ("buzzer", "Buzzer")]
                .into_iter()
                .filter(|(k, _)| match (layout, *k) {
                    (0, "analog") => sel.starts_with('A'),
                    (0, "pwm") => ["3", "5", "6", "9", "10", "11"].contains(&sel.as_str()),
                    _ => true,
                })
                .map(|(k, l)| (k, t(l)))
                .collect();
            let mut add: Option<&str> = None;
            let mut custom = std::mem::take(&mut self.ard.custom);
            let (mut open, mut repeat, mut insert) = (self.ard.custom_open, self.ard.custom_repeat, false);
            self.ard_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{} {sel}", t("Pin"))).font(theme::bold(13.5)).color(p.text));
                    if let Some(u) = used {
                        ui.label(egui::RichText::new(u).font(theme::mono(11.0)).color(p.text3));
                    }
                });
                ui.label(egui::RichText::new(t("Presets – add to your code:")).font(theme::ui(12.0)).color(p.text2));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    for (k, label) in &kinds {
                        if widgets::button(ui, None, label, p.card2, p.text, 28.0, &p).clicked() {
                            add = Some(k);
                        }
                    }
                    if widgets::button(ui, Some("code"), &t("Custom code…"), if open { p.active } else { p.card2 }, p.text, 28.0, &p).clicked() {
                        open = !open;
                    }
                });
                if open {
                    ui.add(
                        egui::TextEdit::multiline(&mut custom)
                            .hint_text(if mode == "cpp" { "digitalWrite({pin}, HIGH);" } else { "pin = {pin}" })
                            .font(theme::mono(12.0))
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin::symmetric(10, 8)),
                    );
                    ui.add(egui::Label::new(egui::RichText::new(tf("{pin} becomes the pin number (here {n}).", &[("n", &sel)])).font(theme::ui(11.5)).color(p.text3)).wrap());
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        let (once_t, rep_t) = if mode == "cpp" { (t("Once (setup)"), t("Repeat (loop)")) } else { (t("Once (before the loop)"), t("Repeat (in the loop)")) };
                        if widgets::button(ui, None, &once_t, if !repeat { p.active } else { p.card2 }, p.text, 26.0, &p).clicked() {
                            repeat = false;
                        }
                        if widgets::button(ui, None, &rep_t, if repeat { p.active } else { p.card2 }, p.text, 26.0, &p).clicked() {
                            repeat = true;
                        }
                        if widgets::button(ui, Some("check"), &t("Insert"), p.accent, p.accent_fg, 26.0, &p).clicked() {
                            insert = true;
                        }
                    });
                }
            });
            self.ard.custom = custom.clone();
            self.ard.custom_open = open;
            self.ard.custom_repeat = repeat;
            if add.is_some() || insert {
                let Some(i) = (!self.tabs.is_empty()).then_some(self.active) else { return };
                let name = self.tabs[i].name();
                let new = match add {
                    Some(kind) => ard::add_snippet(mode, &self.tabs[i].text, &sel, kind, layout == 2),
                    None => ard::add_custom(mode, &self.tabs[i].text, &sel, &custom, repeat),
                };
                match new {
                    Some(new) => {
                        self.tabs[i].text = new;
                        self.status = tf("Added to {name} – the code is in the editor", &[("name", &name)]);
                    }
                    None => self.status = t("That is already in the code."),
                }
            }
        }
        // použité piny: kompaktný zoznam
        let n = pins.len();
        let open = self.ard_sec(ui, "pins", &t("Pins in your code"), true, (n > 0).then(|| n.to_string()));
        if open {
            if pins.is_empty() {
                let hint = if mode == "cpp" {
                    "Use pinMode(), digitalWrite(), analogRead()… in your code and the wiring appears here."
                } else {
                    "Use Pin(), PWM(), ADC() (or digitalio, get_pin…) in your code and the wiring appears here."
                };
                self.ard_hint(ui, &t(hint));
            }
            let mut jump: Option<usize> = None;
            for (k, pin) in pins.iter().enumerate() {
                let (col, pmode, part) = mode_info(pin.mode);
                let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rr, CornerRadius::same(8), p.hover);
                }
                let c = pos2(rr.left() + 14.0, rr.center().y);
                ui.painter().circle_filled(c, 9.0, col);
                ui.painter().text(c, Align2::CENTER_CENTER, (k + 1).to_string(), theme::bold(10.5), Color32::BLACK);
                let title = match &pin.name {
                    Some(nm) => format!("{} {}  ·  {nm}", t("Pin"), pin.pin),
                    None => format!("{} {}", t("Pin"), pin.pin),
                };
                widgets::text(ui, pos2(rr.left() + 32.0, rr.center().y), Align2::LEFT_CENTER, &title, theme::bold(12.5), p.text, rr.width() - 130.0);
                ui.painter().text(pos2(rr.right() - 8.0, rr.center().y), Align2::RIGHT_CENTER, pmode, theme::mono(11.0), col);
                if resp.on_hover_text(t(part)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    jump = Some(pin.line);
                }
            }
            if let Some(l) = jump {
                self.lint_jump = Some(l);
            }
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

    // Uno / Nano / Mega: hore digitálne piny, dole napájanie a analógové
    fn ard_draw_uno(&self, ui: &mut egui::Ui, w: f32, pins: &[ard::Pin], sel: &Option<String>) -> Option<String> {
        let p = self.pal;
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
        let top: Vec<&str> = vec!["SCL", "SDA", "AREF", "GND", "13", "12", "11", "10", "9", "8", "", "7", "6", "5", "4", "3", "2", "1", "0"];
        let bottom: Vec<&str> = vec!["", "IOREF", "RST", "3V3", "5V", "GND", "GND", "VIN", "", "A0", "A1", "A2", "A3", "A4", "A5"];
        let used = |name: &str| pins.iter().position(|x| x.pin == name);
        let power = pins.iter().any(|x| matches!(x.mode, "in" | "analog" | "servo"));
        let clicked = std::cell::Cell::new(None::<String>);
        let draw_row = |labels: &[&str], y: f32, x0: f32, x1: f32, up: bool, row: u8| {
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
                let pickable = l.chars().all(|c| c.is_ascii_digit()) || (l.starts_with('A') && l.len() == 2);
                if pickable {
                    let hit = Rect::from_center_size(pos2(x, c.y + if up { 7.0 } else { -7.0 }), vec2(step.max(10.0), 26.0));
                    let resp = ui.interact(hit, ui.id().with(("ard-pad", row, i)), Sense::click());
                    if resp.hovered() {
                        pa.circle_stroke(c, s * 1.5, Stroke::new(1.5, Color32::from_white_alpha(160)));
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("{} {l}", t("Pin"))).clicked() {
                        clicked.set(Some(l.to_string()));
                    }
                }
                if sel.as_deref() == Some(*l) {
                    pa.circle_stroke(c, s * 1.7, Stroke::new(2.0, p.accent));
                }
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
        draw_row(&top, 0.07, 0.30, 0.97, true, 0);
        draw_row(&bottom, 0.93, 0.40, 0.97, false, 1);
        clicked.take()
    }

    // ESP32 / Pico: dva stĺpce pinov po stranách dosky
    #[allow(clippy::too_many_arguments)]
    fn ard_draw_vertical(&self, ui: &mut egui::Ui, w: f32, title: &str, left: &[Pad], right: &[Pad], usb_top: bool, body: Color32, pins: &[ard::Pin], sel: &Option<String>) -> Option<String> {
        let p = self.pal;
        let n = left.len().max(right.len());
        let row_h = 15.0;
        let h = n as f32 * row_h + 70.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let pa = ui.painter_at(rect);
        let bw = 128.0f32.min(w * 0.5);
        let bx = rect.center().x;
        let board = Rect::from_min_max(pos2(bx - bw / 2.0, rect.top() + 8.0), pos2(bx + bw / 2.0, rect.bottom() - 8.0));
        pa.rect_filled(board, CornerRadius::same(8), body);
        pa.rect_stroke(board, CornerRadius::same(8), Stroke::new(1.0, Color32::from_black_alpha(90)), StrokeKind::Inside);
        let usb = Rect::from_center_size(pos2(bx, if usb_top { board.top() + 4.0 } else { board.bottom() - 4.0 }), vec2(34.0, 14.0));
        pa.rect_filled(usb, CornerRadius::same(3), Color32::from_gray(190));
        pa.text(board.center(), Align2::CENTER_CENTER, title, theme::bold(15.0), Color32::from_white_alpha(70));
        let y0 = if usb_top { board.top() + 30.0 } else { board.top() + 14.0 };
        let clicked = std::cell::Cell::new(None::<String>);
        for (side, pads) in [(0u8, left), (1u8, right)] {
            for (i, pad) in pads.iter().enumerate() {
                let y = y0 + i as f32 * row_h + row_h / 2.0;
                let px = if side == 0 { board.left() + 9.0 } else { board.right() - 9.0 };
                let c = pos2(px, y);
                let col = match pad.kind {
                    0 => Color32::from_gray(25),
                    1 => Color32::from_gray(150),
                    2 => Color32::from_rgb(0xc0, 0x40, 0x40),
                    _ => Color32::from_gray(70),
                };
                pa.rect_filled(Rect::from_center_size(c, vec2(7.0, 7.0)), 1.0, col);
                let (anchor, tx) = if side == 0 { (Align2::LEFT_CENTER, px + 8.0) } else { (Align2::RIGHT_CENTER, px - 8.0) };
                pa.text(pos2(tx, y), anchor, pad.label, theme::ui(8.5), Color32::from_white_alpha(if pad.kind == 0 { 215 } else { 130 }));
                if pad.kind == 0 && !pad.key.is_empty() {
                    let hit = if side == 0 {
                        Rect::from_min_max(pos2(board.left() - 4.0, y - row_h / 2.0), pos2(board.center().x, y + row_h / 2.0))
                    } else {
                        Rect::from_min_max(pos2(board.center().x, y - row_h / 2.0), pos2(board.right() + 4.0, y + row_h / 2.0))
                    };
                    let resp = ui.interact(hit, ui.id().with(("ard-vpad", side, i)), Sense::click());
                    if resp.hovered() {
                        pa.circle_stroke(c, 6.0, Stroke::new(1.5, Color32::from_white_alpha(160)));
                    }
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("{} {}", t("Pin"), pad.key)).clicked() {
                        clicked.set(Some(pad.key.to_string()));
                    }
                    if sel.as_deref() == Some(pad.key) {
                        pa.circle_stroke(c, 7.5, Stroke::new(2.0, p.accent));
                    }
                    if let Some(k) = pins.iter().position(|x| x.pin == pad.key) {
                        let (mc, _, _) = mode_info(pins[k].mode);
                        let lift = if i % 2 == 1 { 22.0 } else { 0.0 };
                        let ox = if side == 0 { board.left() - 16.0 - lift } else { board.right() + 16.0 + lift };
                        let out = pos2(ox, y);
                        pa.line_segment([c, out], Stroke::new(2.0, mc));
                        pa.circle_filled(out, 7.5, mc);
                        pa.text(out, Align2::CENTER_CENTER, (k + 1).to_string(), theme::bold(9.5), Color32::BLACK);
                        pa.circle_stroke(c, 5.0, Stroke::new(2.0, mc));
                    }
                }
            }
        }
        clicked.take()
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
