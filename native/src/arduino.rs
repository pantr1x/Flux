// Arduino (aj Elegoo a iné klony): rozpoznanie dosky na USB, oficiálny arduino-cli (Flux si ho stiahne sám),
// kompilácia + nahratie, sériový monitor, knižnice, firmvér (Blink, StandardFirmata) a piny pre schému zapojenia.
// Všetky príkazy bežia vo Výstupe (pty) – vidno priebeh aj chyby ako pri ▶ Run.
use flux_core::runner::Cmd;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

// typy dosiek na výber (fqbn, názov); Elegoo Uno R3 / Mega 2560 / Nano sú klony týchto
pub const BOARDS: [(&str, &str); 14] = [
    ("arduino:avr:uno", "Arduino Uno (Elegoo Uno R3)"),
    ("arduino:avr:mega", "Arduino Mega 2560 (Elegoo Mega)"),
    ("arduino:avr:nano:cpu=atmega328old", "Arduino Nano – old bootloader (most clones)"),
    ("arduino:avr:nano", "Arduino Nano"),
    ("arduino:avr:leonardo", "Arduino Leonardo"),
    ("arduino:avr:micro", "Arduino Micro"),
    ("arduino:avr:pro:cpu=16MHzatmega328", "Arduino Pro Mini 5V"),
    ("arduino:renesas_uno:unor4wifi", "Arduino UNO R4 WiFi"),
    ("arduino:renesas_uno:minima", "Arduino UNO R4 Minima"),
    ("arduino:megaavr:nona4809", "Arduino Nano Every"),
    ("esp32:esp32:esp32", "ESP32 Dev Module"),
    ("esp32:esp32:esp32s3", "ESP32-S3 Dev Module"),
    ("esp8266:esp8266:nodemcuv2", "ESP8266 NodeMCU"),
    ("rp2040:rp2040:rpipico", "Raspberry Pi Pico (Arduino-Pico)"),
];

// dosky mimo Arduina potrebujú pridať adresu zoznamu balíčkov (arduino-cli --additional-urls)
pub fn extra_url(fqbn: &str) -> Option<&'static str> {
    match fqbn.split(':').next()? {
        "esp32" => Some("https://espressif.github.io/arduino-esp32/package_esp32_index.json"),
        "esp8266" => Some("https://arduino.esp8266.com/stable/package_esp8266com_index.json"),
        "rp2040" => Some("https://github.com/earlephilhower/arduino-pico/releases/download/global/package_rp2040_index.json"),
        _ => None,
    }
}

pub const BAUDS: [u32; 6] = [9600, 19200, 38400, 57600, 115200, 250000];

#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    pub port: String,               // COM3 / /dev/ttyUSB0
    pub name: String,               // čo to je (z arduino-cli alebo z tabuľky USB čipov)
    pub fqbn: Option<String>,       // typ dosky, ak sa dá zistiť
    pub chip: Option<&'static str>, // USB čip klonu (CH340, FTDI, CP210x)
}

// USB VID:PID → (názov, typ dosky, čip). Elegoo a väčšina lacných klonov má čip CH340.
pub fn known(vid: &str, pid: &str) -> Option<(&'static str, Option<&'static str>, Option<&'static str>)> {
    let v = vid.trim_start_matches("0x").to_lowercase();
    let p = pid.trim_start_matches("0x").to_lowercase();
    Some(match (v.as_str(), p.as_str()) {
        ("2341" | "2a03", "0043" | "0001" | "0243" | "6001") => ("Arduino Uno", Some("arduino:avr:uno"), None),
        ("2341" | "2a03", "0010" | "0042" | "0242") => ("Arduino Mega 2560", Some("arduino:avr:mega"), None),
        ("2341" | "2a03", "8036" | "0036") => ("Arduino Leonardo", Some("arduino:avr:leonardo"), None),
        ("2341" | "2a03", "8037" | "0037") => ("Arduino Micro", Some("arduino:avr:micro"), None),
        ("1a86", "7523" | "5523" | "55d4") => ("Elegoo / clone board (CH340)", Some("arduino:avr:uno"), Some("CH340")),
        ("0403", "6001" | "6015") => ("Board with FTDI chip (Nano, Pro Mini…)", Some("arduino:avr:nano"), Some("FTDI")),
        ("10c4", "ea60") => ("ESP32 / ESP8266 board (CP210x)", Some("esp32:esp32:esp32"), Some("CP210x")),
        ("2e8a", _) => ("Raspberry Pi Pico (RP2040)", Some("rp2040:rp2040:rpipico"), None),
        ("303a", _) => ("ESP32-S3 / C3 (native USB)", Some("esp32:esp32:esp32s3"), None),
        _ => return None,
    })
}

// výstup `arduino-cli board list --format json` (verzia 1.x: {detected_ports: […]}, staršie: […])
pub fn parse_board_list(json: &str) -> Vec<Board> {
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    let list = v["detected_ports"].as_array().or(v.as_array()).cloned().unwrap_or_default();
    let mut out = vec![];
    for d in list {
        let port = &d["port"];
        if port["protocol"].as_str().is_some_and(|p| p != "serial") {
            continue;
        }
        let addr = port["address"].as_str().or(d["address"].as_str()).unwrap_or("").to_string();
        if addr.is_empty() {
            continue;
        }
        let props = &port["properties"];
        let (vid, pid) = (props["vid"].as_str().unwrap_or(""), props["pid"].as_str().unwrap_or(""));
        let kn = known(vid, pid);
        let mb = d["matching_boards"].as_array().or(d["boards"].as_array()).and_then(|a| a.first().cloned());
        // bez VID (napr. vstavaný /dev/ttyS0) to nie je USB doska
        if mb.is_none() && kn.is_none() && vid.is_empty() {
            continue;
        }
        let name = mb.as_ref().and_then(|b| b["name"].as_str()).map(String::from).or(kn.map(|k| k.0.to_string())).unwrap_or_else(|| format!("USB serial device {vid}:{pid}"));
        let fqbn = mb.as_ref().and_then(|b| b["fqbn"].as_str()).map(String::from).or(kn.and_then(|k| k.1.map(String::from)));
        out.push(Board { port: addr, name, fqbn, chip: kn.and_then(|k| k.2) });
    }
    out
}

// porty bez arduino-cli (aby sa doska ukázala aj pred inštaláciou)
pub fn scan_ports() -> Vec<Board> {
    let mut out = vec![];
    #[cfg(target_os = "linux")]
    {
        let Ok(rd) = std::fs::read_dir("/sys/class/tty") else { return out };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("rfcomm") {
                out.push(Board { port: format!("/dev/{name}"), name: "Bluetooth serial port (HC-05 / HC-06…)".into(), fqbn: None, chip: Some("Bluetooth") });
                continue;
            }
            if !(name.starts_with("ttyUSB") || name.starts_with("ttyACM")) {
                continue;
            }
            // …/ttyUSB0/device → rozhranie USB; idVendor/idProduct sú o úroveň či dve vyššie
            let dev = e.path().join("device");
            let real = std::fs::canonicalize(&dev).unwrap_or(dev);
            let mut vid = String::new();
            let mut pid = String::new();
            for up in real.ancestors().take(4) {
                if let (Ok(v), Ok(p)) = (std::fs::read_to_string(up.join("idVendor")), std::fs::read_to_string(up.join("idProduct"))) {
                    vid = v.trim().to_string();
                    pid = p.trim().to_string();
                    break;
                }
            }
            let kn = known(&vid, &pid);
            out.push(Board {
                port: format!("/dev/{name}"),
                name: kn.map(|k| k.0.to_string()).unwrap_or_else(|| if vid.is_empty() { "USB serial device".into() } else { format!("USB serial device {vid}:{pid}") }),
                fqbn: kn.and_then(|k| k.1.map(String::from)),
                chip: kn.and_then(|k| k.2),
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // „USB-SERIAL CH340 (COM3)“ + DeviceID s VID_1A86&PID_7523
        let ps = "Get-CimInstance Win32_PnPEntity | Where-Object { $_.Name -match '\\(COM\\d+\\)' } | ForEach-Object { $_.Name + '|' + $_.DeviceID }";
        if let Ok(o) = Command::new("powershell.exe").args(["-NoProfile", "-Command", ps]).creation_flags(0x0800_0000).output() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let (name, id) = line.split_once('|').unwrap_or((line, ""));
                let Some(port) = name.rsplit('(').next().and_then(|s| s.strip_suffix(')')).filter(|p| p.starts_with("COM")) else { continue };
                let up = id.to_uppercase();
                let grab = |k: &str| up.find(k).map(|i| up[i + k.len()..].chars().take(4).collect::<String>()).unwrap_or_default();
                let (vid, pid) = (grab("VID_"), grab("PID_"));
                let kn = known(&vid, &pid);
                out.push(Board {
                    port: port.to_string(),
                    name: kn.map(|k| k.0.to_string()).unwrap_or_else(|| name.rsplit_once(" (").map(|x| x.0).unwrap_or(name).to_string()),
                    fqbn: kn.and_then(|k| k.1.map(String::from)),
                    chip: kn.and_then(|k| k.2),
                });
            }
        }
    }
    out.sort_by(|a, b| a.port.cmp(&b.port));
    out
}

// ---------- arduino-cli ----------
pub fn tools_dir() -> PathBuf {
    flux_core::settings::user_data().join("tools").join("arduino-cli")
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "arduino-cli.exe"
    } else {
        "arduino-cli"
    }
}

// stiahnutý Fluxom, inak z PATH (napr. sudo pacman -S arduino-cli)
pub fn cli() -> Option<String> {
    let own = tools_dir().join(exe_name());
    if own.is_file() {
        return Some(own.to_string_lossy().to_string());
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(exe_name())).find(|p| p.is_file()).map(|p| p.to_string_lossy().to_string())
}

pub fn download_url() -> &'static str {
    if cfg!(windows) {
        "https://downloads.arduino.cc/arduino-cli/arduino-cli_latest_Windows_64bit.zip"
    } else if cfg!(target_os = "macos") {
        "https://downloads.arduino.cc/arduino-cli/arduino-cli_latest_macOS_64bit.tar.gz"
    } else {
        "https://downloads.arduino.cc/arduino-cli/arduino-cli_latest_Linux_64bit.tar.gz"
    }
}

// záloha: najnovšie vydanie arduino-cli na GitHube
fn github_cli() -> Option<Vec<u8>> {
    let v = crate::net::json("GET", "https://api.github.com/repos/arduino/arduino-cli/releases/latest", &["User-Agent: Flux".into()], None).ok()?;
    let suffix = if cfg!(windows) {
        "_Windows_64bit.zip"
    } else if cfg!(target_os = "macos") {
        "_macOS_64bit.tar.gz"
    } else {
        "_Linux_64bit.tar.gz"
    };
    let url = v["assets"].as_array()?.iter().filter_map(|a| a["browser_download_url"].as_str()).find(|u| u.ends_with(suffix))?.to_string();
    crate::net::download(&url, 900)
}

fn tar() -> String {
    if cfg!(windows) {
        let sys = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let p = Path::new(&sys).join("System32").join("tar.exe");
        if p.is_file() {
            return p.to_string_lossy().to_string();
        }
    }
    "tar".into()
}

// stiahne a rozbalí arduino-cli do tools_dir() (≈ 25 MB); beží na vlákne
pub fn install_cli() -> Result<String, String> {
    let dir = tools_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // downloads.arduino.cc, inak to isté vydanie z GitHubu (arduino/arduino-cli)
    let data = crate::net::download(download_url(), 900).or_else(github_cli).ok_or("Download failed – check the internet connection.")?;
    let archive = dir.join(if cfg!(windows) { "cli.zip" } else { "cli.tar.gz" });
    std::fs::write(&archive, data).map_err(|e| e.to_string())?;
    let mut c = Command::new(tar());
    c.arg("-xf").arg(&archive).arg("-C").arg(&dir);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let ok = c.status().map(|s| s.success()).unwrap_or(false);
    let _ = std::fs::remove_file(&archive);
    let exe = dir.join(exe_name());
    if !ok || !exe.is_file() {
        return Err("Could not unpack arduino-cli.".into());
    }
    Ok(exe.to_string_lossy().to_string())
}

fn quiet(cli: &str, args: &[&str]) -> Option<String> {
    let mut c = Command::new(cli);
    c.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let o = c.output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
}

pub fn list_boards(cli: &str) -> Option<Vec<Board>> {
    quiet(cli, &["board", "list", "--format", "json"]).map(|s| parse_board_list(&s))
}

// priečinky arduino-cli: (data = jadrá dosiek, user = Arduino/libraries)
pub fn dirs(cli: &str) -> (PathBuf, PathBuf) {
    let home = dirs::home_dir().unwrap_or_default();
    let mut data = if cfg!(windows) { dirs::data_local_dir().unwrap_or_default().join("Arduino15") } else { home.join(".arduino15") };
    let mut user = if cfg!(windows) { dirs::document_dir().unwrap_or(home.clone()).join("Arduino") } else { home.join("Arduino") };
    if let Some(v) = quiet(cli, &["config", "dump", "--format", "json"]).and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
        let d = if v["config"].is_object() { &v["config"]["directories"] } else { &v["directories"] };
        if let Some(x) = d["data"].as_str().filter(|x| !x.is_empty()) {
            data = PathBuf::from(x);
        }
        if let Some(x) = d["user"].as_str().filter(|x| !x.is_empty()) {
            user = PathBuf::from(x);
        }
    }
    (data, user)
}

// je jadro pre túto dosku nainštalované? (arduino:avr:uno → packages/arduino/hardware/avr)
pub fn core_installed(data: &Path, fqbn: &str) -> bool {
    let mut it = fqbn.split(':');
    match (it.next(), it.next()) {
        (Some(vendor), Some(arch)) => data.join("packages").join(vendor).join("hardware").join(arch).is_dir(),
        _ => false,
    }
}

pub fn core_of(fqbn: &str) -> String {
    fqbn.split(':').take(2).collect::<Vec<_>>().join(":")
}

// knižnice: `lib search <q> --format json` → (meno, verzia, popis)
pub fn lib_search(cli: &str, q: &str) -> Vec<(String, String, String)> {
    let Some(s) = quiet(cli, &["lib", "search", q, "--format", "json"]) else { return vec![] };
    let v: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
    let list = v["libraries"].as_array().cloned().unwrap_or_default();
    list.iter()
        .take(40)
        .map(|l| {
            let latest = if l["latest"].is_object() { l["latest"].clone() } else { l["releases"].as_object().and_then(|r| r.values().last().cloned()).unwrap_or(Value::Null) };
            (l["name"].as_str().unwrap_or("").to_string(), latest["version"].as_str().unwrap_or("").to_string(), latest["sentence"].as_str().unwrap_or("").to_string())
        })
        .filter(|x| !x.0.is_empty())
        .collect()
}

pub fn lib_installed(cli: &str) -> Vec<(String, String)> {
    let Some(s) = quiet(cli, &["lib", "list", "--format", "json"]) else { return vec![] };
    let v: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
    let list = v["installed_libraries"].as_array().or(v.as_array()).cloned().unwrap_or_default();
    list.iter().map(|l| (l["library"]["name"].as_str().unwrap_or("").to_string(), l["library"]["version"].as_str().unwrap_or("").to_string())).filter(|x| !x.0.is_empty()).collect()
}

// ---------- sketch ----------
// arduino-cli chce priečinok s rovnakým menom ako .ino; inak kópia vedľa (spolu s .h/.cpp)
pub fn sketch_dir(file: &str) -> Result<PathBuf, String> {
    let p = Path::new(file);
    let stem = p.file_stem().ok_or("No file name")?.to_string_lossy().to_string();
    let parent = p.parent().ok_or("No folder")?;
    if parent.file_name().map(|n| n.to_string_lossy() == stem).unwrap_or(false) {
        return Ok(parent.to_path_buf());
    }
    let dir = flux_core::settings::user_data().join("arduino").join("sketches").join(&stem);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::copy(p, dir.join(format!("{stem}.ino"))).map_err(|e| e.to_string())?;
    if let Ok(rd) = std::fs::read_dir(parent) {
        for e in rd.flatten() {
            let ext = e.path().extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            if matches!(ext.as_str(), "h" | "hpp" | "cpp" | "c") {
                let _ = std::fs::copy(e.path(), dir.join(e.file_name()));
            }
        }
    }
    Ok(dir)
}

// pomocný sketch Fluxu (Blink, prázdny program)
pub fn write_sketch(name: &str, code: &str) -> Result<PathBuf, String> {
    let dir = flux_core::settings::user_data().join("arduino").join(name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{name}.ino")), code).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub const BLINK: &str = "// Flux: test dosky – vstavaná LED (pin 13) bliká raz za sekundu\nvoid setup() {\n  pinMode(LED_BUILTIN, OUTPUT);\n  Serial.begin(9600);\n  Serial.println(\"Hello from Flux!\");\n}\n\nvoid loop() {\n  digitalWrite(LED_BUILTIN, HIGH);\n  delay(500);\n  digitalWrite(LED_BUILTIN, LOW);\n  delay(500);\n}\n";
pub const EMPTY: &str = "// Flux: prázdny program – doska nerobí nič\nvoid setup() {}\n\nvoid loop() {}\n";
pub const STARTER: &str = "// Arduino: setup() beží raz po zapnutí, loop() stále dokola.\nconst int led = 13; // vstavaná LED\n\nvoid setup() {\n  pinMode(led, OUTPUT);\n  Serial.begin(9600);\n}\n\nvoid loop() {\n  digitalWrite(led, HIGH);\n  delay(1000);\n  digitalWrite(led, LOW);\n  delay(1000);\n  Serial.println(\"blink\");\n}\n";

// ---------- príkazy ----------
// jeden príkaz → priamo; viac → skript (.cmd / .sh), ktorý skončí pri prvej chybe
pub fn script(cmds: &[Vec<String>]) -> Result<Cmd, String> {
    if cmds.len() == 1 {
        return Ok(Cmd { cmd: cmds[0][0].clone(), args: cmds[0][1..].to_vec() });
    }
    let dir = flux_core::settings::user_data().join("arduino");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if cfg!(windows) {
        let q = |a: &String| if a.is_empty() || a.contains([' ', '&', '(', ')', '^']) { format!("\"{}\"", a.replace('%', "%%")) } else { a.replace('%', "%%") };
        let mut s = String::from("@echo off\r\nchcp 65001 >nul\r\n");
        for c in cmds {
            s += &format!("{} || exit /b 1\r\n", c.iter().map(q).collect::<Vec<_>>().join(" "));
        }
        let f = dir.join("run.cmd");
        std::fs::write(&f, s).map_err(|e| e.to_string())?;
        Ok(Cmd { cmd: "cmd.exe".into(), args: vec!["/d".into(), "/c".into(), f.to_string_lossy().to_string()] })
    } else {
        let q = |a: &String| format!("'{}'", a.replace('\'', "'\\''"));
        let mut s = String::from("set -e\n");
        for c in cmds {
            s += &format!("{}\n", c.iter().map(q).collect::<Vec<_>>().join(" "));
        }
        let f = dir.join("run.sh");
        std::fs::write(&f, s).map_err(|e| e.to_string())?;
        Ok(Cmd { cmd: "sh".into(), args: vec![f.to_string_lossy().to_string()] })
    }
}

pub fn v(cli: &str, args: &[&str]) -> Vec<String> {
    std::iter::once(cli.to_string()).chain(args.iter().map(|s| s.to_string())).collect()
}

// Linux: má používateľ prístup k portu? (Arch: skupina uucp, Ubuntu/Debian: dialout) → None = áno
#[cfg(unix)]
pub fn port_access(port: &str) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    if std::fs::OpenOptions::new().read(true).write(true).open(port).is_ok() {
        return None;
    }
    let gid = std::fs::metadata(port).ok()?.gid();
    let group = std::fs::read_to_string("/etc/group").ok()?.lines().find_map(|l| {
        let f: Vec<&str> = l.split(':').collect();
        (f.len() > 2 && f[2] == gid.to_string()).then(|| f[0].to_string())
    });
    Some(group.unwrap_or_else(|| "dialout".into()))
}
#[cfg(not(unix))]
pub fn port_access(_port: &str) -> Option<String> {
    None
}

// ---------- piny v kóde (schéma zapojenia) ----------
#[derive(Clone, Debug, PartialEq)]
pub struct Pin {
    pub pin: String,          // "13", "A0"
    pub mode: &'static str,   // out, pwm, in, pull, analog, servo, tone
    pub name: Option<String>, // meno premennej v kóde
    pub line: usize,          // riadok (od 0) prvého použitia
}

fn rank(m: &str) -> u8 {
    match m {
        "servo" => 6,
        "tone" => 5,
        "pwm" => 4,
        "out" => 3,
        "pull" => 2,
        "in" => 1,
        _ => 0, // analog
    }
}

fn norm_pin(v: &str) -> Option<String> {
    let v = v.trim();
    if v == "LED_BUILTIN" {
        return Some("13".into());
    }
    if v.chars().all(|c| c.is_ascii_digit()) && !v.is_empty() {
        return Some(v.trim_start_matches('0').to_string()).map(|s| if s.is_empty() { "0".into() } else { s });
    }
    let u = v.to_uppercase();
    if u.len() >= 2 && u.starts_with('A') && u[1..].chars().all(|c| c.is_ascii_digit()) {
        return Some(u);
    }
    None
}

pub fn pins_in(code: &str) -> Vec<Pin> {
    let lines: Vec<&str> = code.lines().map(|l| l.split("//").next().unwrap_or("")).collect();
    // konštanty: #define LED 13 / const int led = 13; / int pot = A0;
    let mut consts: Vec<(String, String)> = vec![];
    for l in &lines {
        let t = l.trim();
        if let Some(rest) = t.strip_prefix("#define ") {
            let mut it = rest.split_whitespace();
            if let (Some(n), Some(val)) = (it.next(), it.next()) {
                if let Some(p) = norm_pin(val) {
                    consts.push((n.to_string(), p));
                }
            }
        } else if let Some(eq) = t.find('=') {
            let left: Vec<&str> = t[..eq].split_whitespace().collect();
            let val = t[eq + 1..].trim().trim_end_matches(';').trim();
            let is_decl = left.len() >= 2 && left.iter().any(|w| matches!(*w, "int" | "byte" | "uint8_t" | "short" | "long" | "char" | "unsigned"));
            if let (true, Some(n), Some(p)) = (is_decl, left.last(), norm_pin(val)) {
                if !n.contains('[') {
                    consts.push((n.to_string(), p));
                }
            }
        }
    }
    let resolve = |arg: &str| -> (Option<String>, Option<String>) {
        let a = arg.trim();
        if let Some(p) = norm_pin(a) {
            return (Some(p), None);
        }
        match consts.iter().rev().find(|c| c.0 == a) {
            Some(c) => (Some(c.1.clone()), Some(a.to_string())),
            None => (None, None),
        }
    };
    let mut out: Vec<Pin> = vec![];
    let mut add = |pin: String, mode: &'static str, name: Option<String>, line: usize| {
        if let Some(x) = out.iter_mut().find(|x| x.pin == pin) {
            if rank(mode) > rank(x.mode) {
                x.mode = mode;
            }
            if x.name.is_none() {
                x.name = name;
            }
        } else {
            out.push(Pin { pin, mode, name, line });
        }
    };
    for (i, l) in lines.iter().enumerate() {
        for (func, mode) in [("pinMode(", ""), ("digitalWrite(", "out"), ("analogWrite(", "pwm"), ("digitalRead(", "in"), ("analogRead(", "analog"), ("tone(", "tone"), (".attach(", "servo")] {
            let mut from = 0;
            while let Some(k) = l[from..].find(func) {
                let s = from + k + func.len();
                // „xdigitalWrite(“ nie je volanie digitalWrite
                let prev = l[..from + k].chars().last();
                from = s;
                if func != ".attach(" && prev.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let args = &l[s..];
                let end = args.find(')').unwrap_or(args.len());
                let mut parts = args[..end].split(',');
                let first = parts.next().unwrap_or("");
                let (Some(pin), name) = resolve(first) else { continue };
                let m = if func == "pinMode(" {
                    match parts.next().map(|x| x.trim()) {
                        Some("OUTPUT") => "out",
                        Some("INPUT_PULLUP") => "pull",
                        _ => "in",
                    }
                } else {
                    mode
                };
                add(pin, m, name, i);
            }
        }
    }
    // analogové piny na Uno sú aj digitálne 14–19; zoradiť: digitálne podľa čísla, potom A0…
    out.sort_by_key(|p| if let Some(a) = p.pin.strip_prefix('A') { 100 + a.parse::<u32>().unwrap_or(0) } else { p.pin.parse::<u32>().unwrap_or(0) });
    out
}

// ---------- Bluetooth (Linux: bluetoothctl) ----------
// spárované zariadenia: (meno, MAC, pripojené)
pub fn bt_devices() -> Vec<(String, String, bool)> {
    let Some(list) = quiet("bluetoothctl", &["devices", "Paired"]).or_else(|| quiet("bluetoothctl", &["paired-devices"])) else { return vec![] };
    list.lines()
        .filter_map(|l| {
            let mut it = l.trim().splitn(3, ' ');
            (it.next()? == "Device").then_some(())?;
            let mac = it.next()?.to_string();
            let name = it.next().unwrap_or(&mac).to_string();
            let conn = quiet("bluetoothctl", &["info", &mac]).is_some_and(|i| i.contains("Connected: yes"));
            Some((name, mac, conn))
        })
        .collect()
}

// ---------- piny v Pythone (MicroPython, CircuitPython, pyFirmata) ----------
fn merge_pin(out: &mut Vec<Pin>, pin: String, mode: &'static str, name: Option<String>, line: usize) {
    if let Some(x) = out.iter_mut().find(|x| x.pin == pin) {
        if rank(mode) > rank(x.mode) {
            x.mode = mode;
        }
        if x.name.is_none() {
            x.name = name;
        }
    } else {
        out.push(Pin { pin, mode, name, line });
    }
}

fn sort_pins(out: &mut [Pin]) {
    out.sort_by_key(|p| if let Some(a) = p.pin.strip_prefix('A') { 100 + a.parse::<u32>().unwrap_or(0) } else { p.pin.parse::<u32>().unwrap_or(0) });
}

// prvý argument volania `Pin(` / `PWMOut(` …: číslo, názov konštanty alebo board.GP15 / board.D5 / board.A0
fn py_pin_arg(a: &str, consts: &[(String, String)]) -> Option<String> {
    let a = a.trim().trim_matches(|c| c == '\'' || c == '"');
    if let Some(b) = a.strip_prefix("board.") {
        let u = b.to_uppercase();
        if let Some(n) = u.strip_prefix("GP").or_else(|| u.strip_prefix('D')).filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
            return Some(n.to_string());
        }
        return norm_pin(&u).filter(|p| p.starts_with('A'));
    }
    norm_pin(a).or_else(|| consts.iter().rev().find(|c| c.0 == a).map(|c| c.1.clone()))
}

pub fn py_pins(code: &str) -> Vec<Pin> {
    let lines: Vec<&str> = code.lines().map(|l| l.split('#').next().unwrap_or("")).collect();
    // konštanty: LED = 2
    let mut consts: Vec<(String, String)> = vec![];
    for l in &lines {
        if let Some((n, v)) = l.split_once('=') {
            let (n, v) = (n.trim(), v.trim());
            if !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_') && v.chars().all(|c| c.is_ascii_digit()) && !v.is_empty() {
                consts.push((n.to_string(), v.to_string()));
            }
        }
    }
    let mut out: Vec<Pin> = vec![];
    for (i, l) in lines.iter().enumerate() {
        let name = l.split_once('=').map(|(n, _)| n.trim()).filter(|n| !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_')).map(String::from);
        let arg_of = |key: &str| -> Option<String> {
            let s = l.find(key)? + key.len();
            let rest = &l[s..];
            let end = rest.find([',', ')']).unwrap_or(rest.len());
            py_pin_arg(&rest[..end], &consts)
        };
        // MicroPython
        if let Some(pin) = arg_of("Pin(") {
            let mode = if l.contains("PWM(") {
                "pwm"
            } else if l.contains("ADC(") {
                "analog"
            } else if l.contains("Pin.OUT") {
                "out"
            } else if l.contains("PULL_UP") {
                "pull"
            } else {
                "in"
            };
            merge_pin(&mut out, pin, mode, name.clone(), i);
        } else if let Some(pin) = arg_of("ADC(") {
            merge_pin(&mut out, pin, "analog", name.clone(), i);
        }
        // CircuitPython
        for (key, mode) in [("digitalio.DigitalInOut(", "out"), ("pwmio.PWMOut(", "pwm"), ("analogio.AnalogIn(", "analog")] {
            if let Some(pin) = arg_of(key) {
                let mut m = mode;
                if key.starts_with("digital") {
                    // smer o pár riadkov nižšie: Direction.INPUT / Pull.UP
                    let tail: String = lines.iter().skip(i + 1).take(4).copied().collect::<Vec<_>>().join(" ");
                    if tail.contains("Direction.INPUT") {
                        m = if tail.contains("Pull.UP") { "pull" } else { "in" };
                    }
                }
                merge_pin(&mut out, pin, m, name.clone(), i);
            }
        }
        // pyFirmata: get_pin('d:8:o'), board.digital[13], board.analog[0]
        if let Some(a) = l.find("get_pin(") {
            let spec = l[a + 8..].trim_start().trim_start_matches(|c| c == '\'' || c == '"');
            let f: Vec<&str> = spec.split(|c| c == ':' || c == '\'' || c == '"').collect();
            if f.len() >= 3 {
                let m = match f[2] {
                    "o" => "out",
                    "p" => "pwm",
                    "s" => "servo",
                    _ => "in",
                };
                if f[0] == "a" {
                    merge_pin(&mut out, format!("A{}", f[1]), "analog", name.clone(), i);
                } else if f[1].chars().all(|c| c.is_ascii_digit()) && !f[1].is_empty() {
                    merge_pin(&mut out, f[1].to_string(), m, name.clone(), i);
                }
            }
        }
        for (key, analog) in [("digital[", false), ("analog[", true)] {
            if let Some(a) = l.find(key) {
                let rest = &l[a + key.len()..];
                if let Some(n) = rest.split(']').next().filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
                    if analog {
                        merge_pin(&mut out, format!("A{n}"), "analog", None, i);
                    } else {
                        merge_pin(&mut out, n.to_string(), if l.contains(".read(") { "in" } else { "out" }, None, i);
                    }
                }
            }
        }
    }
    sort_pins(&mut out);
    out
}

// piny podľa jazyka: C++ alebo Python
pub fn pins_for(mode: &str, code: &str) -> Vec<Pin> {
    if mode == "cpp" {
        pins_in(code)
    } else {
        py_pins(code)
    }
}

// ---------- „pridať do kódu“: kliknutie na pin v schéme vloží ukážku ----------
fn lines_of(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}

fn join_lines(v: &[String], trailing_nl: bool) -> String {
    let mut s = v.join("\n");
    if trailing_nl || !s.is_empty() {
        s.push('\n');
    }
    s
}

// riadok za otváracou zátvorkou funkcie (`void setup() {`), alebo None, ak funkcia v kóde nie je
fn after_fn(lines: &[String], head: &str) -> Option<usize> {
    let i = lines.iter().position(|l| l.trim_start().starts_with(head))?;
    Some(if lines[i].contains('{') { i + 1 } else { i + 2 })
}

fn add_cpp(text: &str, pin: &str, kind: &str) -> Option<String> {
    let id = pin.to_lowercase();
    let mut top: Vec<String> = vec![];
    let mut setup: Vec<String> = vec![];
    let mut lp: Vec<String> = vec![];
    let ser = !text.contains("Serial.begin(");
    match kind {
        "led" => {
            setup.push(format!("pinMode({pin}, OUTPUT);"));
            lp.extend([format!("digitalWrite({pin}, HIGH);"), "delay(500);".into(), format!("digitalWrite({pin}, LOW);"), "delay(500);".into()]);
        }
        "button" => {
            setup.push(format!("pinMode({pin}, INPUT_PULLUP);"));
            lp.extend([format!("if (digitalRead({pin}) == LOW) {{"), format!("  Serial.println(\"Button on pin {pin} pressed\");"), "  delay(200);".into(), "}".into()]);
        }
        "pwm" => {
            setup.push(format!("pinMode({pin}, OUTPUT);"));
            lp.extend([format!("for (int v = 0; v <= 255; v += 5) {{"), format!("  analogWrite({pin}, v);"), "  delay(20);".into(), "}".into()]);
        }
        "analog" => lp.extend([format!("int value_{id} = analogRead({pin});"), format!("Serial.println(value_{id});"), "delay(100);".into()]),
        "servo" => {
            top.extend(["#include <Servo.h>".into(), format!("Servo servo_{id};")]);
            setup.push(format!("servo_{id}.attach({pin});"));
            lp.push(format!("servo_{id}.write(90);"));
        }
        "buzzer" => {
            setup.push(format!("pinMode({pin}, OUTPUT);"));
            lp.extend([format!("tone({pin}, 1000, 200);"), "delay(500);".into()]);
        }
        _ => return None,
    }
    if matches!(kind, "button" | "analog") && ser {
        setup.insert(0, "Serial.begin(9600);".into());
    }
    if text.contains(&setup.iter().chain(lp.iter()).next()?.clone()) {
        return None; // už pridané
    }
    Some(place_cpp(text, top, setup, lp))
}

fn place_cpp(text: &str, top: Vec<String>, setup: Vec<String>, lp: Vec<String>) -> String {
    let mut top = top;
    let mut lines = lines_of(text);
    let ind = |v: &[String]| v.iter().map(|l| format!("  {l}")).collect::<Vec<_>>();
    match (after_fn(&lines, "void setup"), after_fn(&lines, "void loop")) {
        (Some(s), Some(l)) => {
            // najprv to, čo je neskôr v súbore, nech sa indexy nepoplietli
            let (first, second) = if s > l { ((s, ind(&setup)), (l, ind(&lp))) } else { ((l, ind(&lp)), (s, ind(&setup))) };
            for (at, add) in [first, second] {
                for (k, line) in add.into_iter().enumerate() {
                    lines.insert(at + k, line);
                }
            }
        }
        _ => {
            if !lines.is_empty() && !lines.last().is_some_and(|l| l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push("void setup() {".into());
            lines.extend(ind(&setup));
            lines.push("}".into());
            lines.push(String::new());
            lines.push("void loop() {".into());
            lines.extend(ind(&lp));
            lines.push("}".into());
        }
    }
    if !top.is_empty() {
        top.push(String::new());
        for (k, line) in top.into_iter().enumerate() {
            lines.insert(k, line);
        }
    }
    join_lines(&lines, text.ends_with('\n') || text.is_empty())
}

fn add_py(mode: &str, text: &str, pin: &str, kind: &str, pico: bool) -> Option<String> {
    let analog = pin.starts_with('A');
    let num = pin.trim_start_matches('A');
    let id = pin.to_lowercase();
    let (mut imports, mut defs, mut body): (Vec<String>, Vec<String>, Vec<String>) = (vec![], vec![], vec![]);
    match mode {
        "micropython" => {
            let pe = num;
            match kind {
                "led" => {
                    imports.extend(["from machine import Pin".into(), "from time import sleep".into()]);
                    defs.push(format!("led_{id} = Pin({pe}, Pin.OUT)"));
                    body.extend([format!("led_{id}.toggle()"), "sleep(0.5)".into()]);
                }
                "button" => {
                    imports.push("from machine import Pin".into());
                    defs.push(format!("button_{id} = Pin({pe}, Pin.IN, Pin.PULL_UP)"));
                    body.extend([format!("if button_{id}.value() == 0:"), format!("    print(\"Button on pin {pin} pressed\")")]);
                }
                "pwm" => {
                    imports.extend(["from machine import Pin, PWM".into(), "from time import sleep".into()]);
                    defs.push(format!("pwm_{id} = PWM(Pin({pe}), freq=1000)"));
                    body.extend([format!("pwm_{id}.duty_u16(32768)"), "sleep(0.5)".into()]);
                }
                "analog" => {
                    imports.extend(["from machine import Pin, ADC".into(), "from time import sleep".into()]);
                    defs.push(format!("adc_{id} = ADC(Pin({pe}))"));
                    body.extend([format!("print(adc_{id}.read_u16())"), "sleep(0.1)".into()]);
                }
                "servo" => {
                    imports.extend(["from machine import Pin, PWM".into(), "from time import sleep".into()]);
                    defs.push(format!("servo_{id} = PWM(Pin({pe}), freq=50)"));
                    body.extend([format!("servo_{id}.duty_u16(4915)  # about 90 degrees"), "sleep(1)".into()]);
                }
                "buzzer" => {
                    imports.extend(["from machine import Pin, PWM".into(), "from time import sleep".into()]);
                    defs.push(format!("buzzer_{id} = PWM(Pin({pe}))"));
                    body.extend([format!("buzzer_{id}.freq(1000)"), format!("buzzer_{id}.duty_u16(32768)"), "sleep(0.2)".into(), format!("buzzer_{id}.duty_u16(0)"), "sleep(0.5)".into()]);
                }
                _ => return None,
            }
        }
        "circuitpython" => {
            let pe = if pico {
                format!("board.GP{num}")
            } else if analog {
                format!("board.{pin}")
            } else {
                format!("board.D{num}")
            };
            imports.extend(["import time".into(), "import board".into()]);
            match kind {
                "led" => {
                    imports.push("import digitalio".into());
                    defs.extend([format!("led_{id} = digitalio.DigitalInOut({pe})"), format!("led_{id}.direction = digitalio.Direction.OUTPUT")]);
                    body.extend([format!("led_{id}.value = not led_{id}.value"), "time.sleep(0.5)".into()]);
                }
                "button" => {
                    imports.push("import digitalio".into());
                    defs.extend([format!("button_{id} = digitalio.DigitalInOut({pe})"), format!("button_{id}.direction = digitalio.Direction.INPUT"), format!("button_{id}.pull = digitalio.Pull.UP")]);
                    body.extend([format!("if not button_{id}.value:"), format!("    print(\"Button on pin {pin} pressed\")")]);
                }
                "pwm" => {
                    imports.push("import pwmio".into());
                    defs.push(format!("pwm_{id} = pwmio.PWMOut({pe}, frequency=1000)"));
                    body.extend([format!("pwm_{id}.duty_cycle = 32768"), "time.sleep(0.5)".into()]);
                }
                "analog" => {
                    imports.push("import analogio".into());
                    defs.push(format!("adc_{id} = analogio.AnalogIn({pe})"));
                    body.extend([format!("print(adc_{id}.value)"), "time.sleep(0.1)".into()]);
                }
                "servo" => {
                    imports.push("import pwmio".into());
                    defs.push(format!("servo_{id} = pwmio.PWMOut({pe}, frequency=50)"));
                    body.extend([format!("servo_{id}.duty_cycle = 4915  # about 90 degrees"), "time.sleep(1)".into()]);
                }
                "buzzer" => {
                    imports.push("import pwmio".into());
                    defs.push(format!("buzzer_{id} = pwmio.PWMOut({pe}, frequency=1000, variable_frequency=True)"));
                    body.extend([format!("buzzer_{id}.duty_cycle = 32768"), "time.sleep(0.2)".into(), format!("buzzer_{id}.duty_cycle = 0"), "time.sleep(0.5)".into()]);
                }
                _ => return None,
            }
        }
        _ => {
            // pyFirmata2: objekt `board` z Arduino("port")
            imports.push("import time".into());
            let sampling = "board.samplingOn()".to_string();
            match kind {
                "led" => {
                    defs.push(format!("led_{id} = board.get_pin('d:{num}:o')"));
                    body.extend([format!("led_{id}.write(1)"), "time.sleep(0.5)".into(), format!("led_{id}.write(0)"), "time.sleep(0.5)".into()]);
                }
                "button" => defs.extend([
                    format!("button_{id} = board.get_pin('d:{num}:i')"),
                    format!("button_{id}.register_callback(lambda v: print(\"Pin {pin}:\", v))"),
                    format!("button_{id}.enable_reporting()"),
                    sampling,
                ]),
                "pwm" | "buzzer" => {
                    defs.push(format!("pwm_{id} = board.get_pin('d:{num}:p')"));
                    body.extend([format!("pwm_{id}.write(0.5)"), "time.sleep(0.5)".into()]);
                }
                "analog" => {
                    defs.extend([format!("board.analog[{num}].register_callback(lambda v: print(\"A{num}:\", v))"), format!("board.analog[{num}].enable_reporting()"), sampling]);
                    body.push("time.sleep(0.1)".into());
                }
                "servo" => {
                    defs.push(format!("servo_{id} = board.get_pin('d:{num}:s')"));
                    body.extend([format!("servo_{id}.write(90)"), "time.sleep(1)".into()]);
                }
                _ => return None,
            }
        }
    }
    if text.contains(defs.first()?.as_str()) {
        return None; // už pridané
    }
    Some(place_py(text, imports, defs, body))
}

fn place_py(text: &str, imports: Vec<String>, defs: Vec<String>, body: Vec<String>) -> String {
    let mut lines = lines_of(text);
    let is_loop = |l: &String| l.trim_end() == "while True:";
    // telo do slučky, definície pred ňu (alebo na koniec)
    match lines.iter().position(is_loop) {
        Some(lp) => {
            for (k, b) in body.iter().enumerate() {
                lines.insert(lp + 1 + k, format!("    {b}"));
            }
            for (k, d) in defs.iter().enumerate() {
                if !text.contains(d.as_str()) {
                    lines.insert(lp + k, d.clone());
                }
            }
        }
        None => {
            if !lines.is_empty() && !lines.last().is_some_and(|l| l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.extend(defs.iter().filter(|d| !text.contains(d.as_str())).cloned());
            if !body.is_empty() {
                lines.push(String::new());
                lines.push("while True:".into());
                lines.extend(body.iter().map(|b| format!("    {b}")));
            }
        }
    }
    // importy navrch (za úvodné komentáre a existujúce importy)
    let mut at = 0;
    for (i, l) in lines.iter().enumerate().take(30) {
        let t = l.trim_start();
        if t.starts_with("import ") || t.starts_with("from ") || (t.starts_with('#') && at == i) || (t.is_empty() && at == i) {
            at = i + 1;
        }
    }
    let new: Vec<String> = imports.into_iter().filter(|im| !text.lines().any(|l| l.trim() == im.as_str())).collect();
    for (k, im) in new.iter().enumerate() {
        lines.insert(at + k, im.clone());
    }
    join_lines(&lines, true)
}

// vlastný kód z panela: {pin} sa nahradí číslom pinu; repeat = do loop() / do slučky, inak do setup() / pred slučku
pub fn add_custom(mode: &str, text: &str, pin: &str, code: &str, repeat: bool) -> Option<String> {
    let code = code.replace("{pin}", pin);
    let mut lines: Vec<String> = code.lines().map(|l| l.trim_end().to_string()).collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    if lines.is_empty() || text.contains(lines.join("\n").as_str()) {
        return None;
    }
    let (once, each) = if repeat { (vec![], lines) } else { (lines, vec![]) };
    Some(if mode == "cpp" { place_cpp(text, vec![], once, each) } else { place_py(text, vec![], once, each) })
}

// nový skript s vlastným kódom pre pin: (prípona, obsah); hlavička podľa jazyka, potom kód (raz / opakovane)
pub fn new_script(mode: &str, pin: &str, code: &str, repeat: bool, port: &str) -> Option<(&'static str, String)> {
    let (ext, head) = match mode {
        "cpp" => ("ino", String::new()),
        "micropython" => ("py", "from machine import Pin\nfrom time import sleep\n".to_string()),
        "circuitpython" => ("py", "import time\nimport board\nimport digitalio\n".to_string()),
        _ => ("py", format!("import time\nfrom pyfirmata2 import Arduino\n\nboard = Arduino(\"{port}\")\n")),
    };
    add_custom(mode, &head, pin, code, repeat).map(|c| (ext, c))
}

// pin + druh súčiastky → upravený text súboru (None = nič nepridané / už tam je)
pub fn add_snippet(mode: &str, text: &str, pin: &str, kind: &str, pico: bool) -> Option<String> {
    match mode {
        "cpp" => add_cpp(text, pin, kind),
        "micropython" | "circuitpython" | "firmata" => add_py(mode, text, pin, kind, pico),
        _ => None,
    }
}

// ---------- Python na doske (MicroPython, CircuitPython, Python + Firmata) ----------
// podľa importov v .py súbore: ide o kód pre dosku?
pub fn guess_py(text: &str) -> Option<&'static str> {
    let has = |k: &str| {
        text.lines().take(300).any(|l| {
            let l = l.trim_start();
            l.strip_prefix("import ").or_else(|| l.strip_prefix("from ")).is_some_and(|r| r.split(|c: char| !c.is_alphanumeric() && c != '_').next() == Some(k))
        })
    };
    if has("pyfirmata") || has("pyfirmata2") {
        Some("firmata")
    } else if has("machine") || has("utime") || has("micropython") || has("network") && text.contains("WLAN") {
        Some("micropython")
    } else if has("board") || has("digitalio") || has("analogio") || has("busio") || has("neopixel") || has("adafruit_hid") {
        Some("circuitpython")
    } else {
        None
    }
}

pub fn py() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

fn works(cmd: &[String]) -> bool {
    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]).arg("--version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c.status().map(|s| s.success()).unwrap_or(false)
}

// nástroj z pip/pipx (mpremote, esptool): na PATH, v ~/.local/bin, alebo ako `python -m <modul>`
pub fn find_tool(name: &str) -> Option<Vec<String>> {
    let direct = vec![name.to_string()];
    if works(&direct) {
        return Some(direct);
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        for sub in [".local/bin", "Scripts"] {
            let f = Path::new(&home).join(sub).join(if cfg!(windows) { format!("{name}.exe") } else { name.to_string() });
            if f.is_file() {
                return Some(vec![f.to_string_lossy().to_string()]);
            }
        }
    }
    let m = vec![py().to_string(), "-m".to_string(), name.to_string()];
    works(&m).then_some(m)
}

// inštalácia balíčka: aplikácie (mpremote, esptool) cez pipx, knižnice (pyserial, pyfirmata2) cez pip --user;
// novšie Ubuntu/Debian chránia systémový Python, preto posledný pokus s --break-system-packages
pub fn pip_install(pkg: &str, app: bool) -> Vec<String> {
    if cfg!(windows) {
        return vec![py().into(), "-m".into(), "pip".into(), "install".into(), "--user".into(), pkg.into()];
    }
    let p = py();
    // Ubuntu / Debian nemajú pip od začiatku: stiahne sa get-pip.py (bez sudo); aplikácie najprv skúsia pipx
    let pipx = if app { format!("if command -v pipx >/dev/null 2>&1; then pipx install {pkg} && exit 0; fi\n") } else { String::new() };
    let line = format!(
        "{pipx}if ! {p} -m pip --version >/dev/null 2>&1; then\n\
         echo 'pip is missing - downloading get-pip.py'\n\
         T=$(mktemp -d)\n\
         curl -fsSL https://bootstrap.pypa.io/get-pip.py -o \"$T/get-pip.py\" || {{ echo 'Download failed. Try: sudo apt install python3-pip'; exit 1; }}\n\
         {p} \"$T/get-pip.py\" --user --break-system-packages || {p} \"$T/get-pip.py\" --user || {{ echo 'Could not install pip. Try: sudo apt install python3-pip'; exit 1; }}\n\
         fi\n\
         {p} -m pip install --user {pkg} || {p} -m pip install --user --break-system-packages {pkg}"
    );
    vec!["sh".into(), "-c".into(), line]
}

// CircuitPython sa po pripojení ukáže ako disk CIRCUITPY
pub fn circuitpy_drive() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        for l in 'D'..='Z' {
            let d = PathBuf::from(format!("{l}:\\"));
            if d.join("boot_out.txt").is_file() {
                return Some(d);
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let user = std::env::var("USER").unwrap_or_default();
        for base in [format!("/media/{user}"), format!("/run/media/{user}"), "/media".into(), "/mnt".into(), "/Volumes".into()] {
            let d = Path::new(&base).join("CIRCUITPY");
            if d.is_dir() {
                return Some(d);
            }
        }
        None
    }
}

// disk bootloadera UF2 (Raspberry Pi Pico RPI-RP2, dosky Adafruit…): obsahuje INFO_UF2.TXT
pub fn uf2_drive() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        for l in 'D'..='Z' {
            let d = PathBuf::from(format!("{l}:\\"));
            if d.join("INFO_UF2.TXT").is_file() {
                return Some(d);
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let user = std::env::var("USER").unwrap_or_default();
        for base in [format!("/media/{user}"), format!("/run/media/{user}"), "/media".into(), "/mnt".into(), "/Volumes".into()] {
            let Ok(rd) = std::fs::read_dir(&base) else { continue };
            for e in rd.flatten() {
                if e.path().join("INFO_UF2.TXT").is_file() {
                    return Some(e.path());
                }
            }
        }
        None
    }
}

// je Python modul nainštalovaný (pyserial = serial, pyfirmata2)?
pub fn py_has(module: &str) -> bool {
    let mut c = Command::new(py());
    c.args(["-c", &format!("import {module}")]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c.status().map(|s| s.success()).unwrap_or(false)
}

pub const MICROPY_BLINK: &str = "# MicroPython: blikanie LED na doske\nfrom machine import Pin\nfrom time import sleep\n\nled = Pin(2, Pin.OUT)  # ESP32: 2, Raspberry Pi Pico: \"LED\"\n\nwhile True:\n    led.toggle()\n    sleep(0.5)\n";
pub const CIRCUITPY_BLINK: &str = "# CircuitPython: blikanie LED na doske\nimport time\nimport board\nimport digitalio\n\nled = digitalio.DigitalInOut(board.LED)\nled.direction = digitalio.Direction.OUTPUT\n\nwhile True:\n    led.value = not led.value\n    time.sleep(0.5)\n";
pub fn firmata_blink(port: &str) -> String {
    format!("# Python + Firmata: na dosku sa raz nahrá StandardFirmata, program beží v počítači\nimport time\nfrom pyfirmata2 import Arduino\n\nboard = Arduino(\"{port}\")\n\nwhile True:\n    board.digital[13].write(1)\n    time.sleep(0.5)\n    board.digital[13].write(0)\n    time.sleep(0.5)\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn arduino_py_pins_and_snippets() {
        let mp = "from machine import Pin, PWM, ADC\nLED = 2\nled = Pin(LED, Pin.OUT)\nbtn = Pin(4, Pin.IN, Pin.PULL_UP)\nm = PWM(Pin(5))\nadc = ADC(Pin(34))\n";
        let p = super::py_pins(mp);
        let get = |n: &str| p.iter().find(|x| x.pin == n).map(|x| x.mode);
        assert_eq!((get("2"), get("4"), get("5"), get("34")), (Some("out"), Some("pull"), Some("pwm"), Some("analog")));
        let cp = "import board, digitalio, analogio\nled = digitalio.DigitalInOut(board.GP15)\nled.direction = digitalio.Direction.OUTPUT\na = analogio.AnalogIn(board.A0)\n";
        let p = super::py_pins(cp);
        assert!(p.iter().any(|x| x.pin == "15" && x.mode == "out") && p.iter().any(|x| x.pin == "A0" && x.mode == "analog"));
        let fm = "board = Arduino('x')\nled = board.get_pin('d:8:o')\nboard.analog[1].enable_reporting()\n";
        let p = super::py_pins(fm);
        assert!(p.iter().any(|x| x.pin == "8" && x.mode == "out") && p.iter().any(|x| x.pin == "A1"));
        // C++: LED na pin 8 do setup() a loop()
        let out = super::add_snippet("cpp", super::STARTER, "8", "led", false).unwrap();
        assert!(out.contains("pinMode(8, OUTPUT);") && out.contains("digitalWrite(8, HIGH);"));
        assert!(super::add_snippet("cpp", &out, "8", "led", false).is_none());
        let pins = super::pins_in(&out);
        assert!(pins.iter().any(|x| x.pin == "8"));
        // servo: include navrchu
        assert!(super::add_snippet("cpp", super::STARTER, "9", "servo", false).unwrap().starts_with("#include <Servo.h>"));
        // MicroPython: definícia pred slučkou, telo v slučke, importy hore
        let out = super::add_snippet("micropython", super::MICROPY_BLINK, "5", "pwm", false).unwrap();
        let l: Vec<&str> = out.lines().collect();
        let w = l.iter().position(|x| *x == "while True:").unwrap();
        assert!(l[..w].contains(&"pwm_5 = PWM(Pin(5), freq=1000)") && l[w + 1] == "    pwm_5.duty_u16(32768)");
        assert!(l.contains(&"from machine import Pin, PWM"));
        // CircuitPython na Pico: board.GP15
        assert!(super::add_snippet("circuitpython", super::CIRCUITPY_BLINK, "15", "led", true).unwrap().contains("board.GP15"));
        // vlastný kód: {pin} sa nahradí, do loop() alebo do setup()
        let c = super::add_custom("cpp", super::STARTER, "8", "digitalWrite({pin}, HIGH);", true).unwrap();
        assert!(c.contains("  digitalWrite(8, HIGH);"));
        let c = super::add_custom("micropython", super::MICROPY_BLINK, "8", "x = {pin}", false).unwrap();
        assert!(c.lines().position(|l| l == "x = 8").unwrap() < c.lines().position(|l| l == "while True:").unwrap());
        assert!(super::add_custom("cpp", super::STARTER, "8", "  \n", true).is_none());
        // nový skript s vlastným kódom
        let (ext, c) = super::new_script("cpp", "13", "digitalWrite({pin}, HIGH);", true, "").unwrap();
        assert!(ext == "ino" && c.contains("void setup()") && c.contains("  digitalWrite(13, HIGH);"));
        let (ext, c) = super::new_script("firmata", "8", "board.digital[{pin}].write(1)", true, "/dev/ttyUSB0").unwrap();
        assert!(ext == "py" && c.contains("Arduino(\"/dev/ttyUSB0\")") && c.contains("    board.digital[8].write(1)"));
        // pyFirmata
        assert!(super::add_snippet("firmata", &super::firmata_blink("/dev/ttyUSB0"), "9", "servo", false).unwrap().contains("get_pin('d:9:s')"));
    }

    #[test]
    fn arduino_py_modes() {
        assert_eq!(super::guess_py("from machine import Pin\n"), Some("micropython"));
        assert_eq!(super::guess_py("import board\nimport digitalio"), Some("circuitpython"));
        assert_eq!(super::guess_py("from pyfirmata2 import Arduino"), Some("firmata"));
        assert_eq!(super::guess_py("import os\nprint(1)"), None);
        assert!(super::extra_url("esp32:esp32:esp32").is_some() && super::extra_url("arduino:avr:uno").is_none());
        assert!(super::firmata_blink("/dev/ttyUSB0").contains("/dev/ttyUSB0"));
    }

    use super::*;

    #[test]
    fn arduino_known_boards() {
        assert_eq!(known("0x2341", "0x0043").unwrap().1, Some("arduino:avr:uno"));
        assert_eq!(known("1A86", "7523").unwrap().2, Some("CH340"));
        assert!(known("dead", "beef").is_none());
    }

    #[test]
    fn arduino_board_list_json() {
        let v1 = r#"{"detected_ports":[{"matching_boards":[{"name":"Arduino Uno","fqbn":"arduino:avr:uno"}],"port":{"address":"/dev/ttyACM0","label":"/dev/ttyACM0","protocol":"serial","properties":{"pid":"0x0043","vid":"0x2341"}}},
            {"port":{"address":"/dev/ttyUSB0","protocol":"serial","properties":{"pid":"0x7523","vid":"0x1A86"}}},
            {"port":{"address":"/dev/ttyS0","protocol":"serial","properties":{}}}]}"#;
        let b = parse_board_list(v1);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].name, "Arduino Uno");
        assert_eq!(b[1].chip, Some("CH340"));
        assert_eq!(b[1].fqbn.as_deref(), Some("arduino:avr:uno"));
        // staršie arduino-cli: pole
        let old = r#"[{"address":"COM3","protocol":"serial","port":{"address":"COM3","protocol":"serial","properties":{"vid":"0x2341","pid":"0x0042"}}}]"#;
        assert_eq!(parse_board_list(old)[0].fqbn.as_deref(), Some("arduino:avr:mega"));
    }

    #[test]
    fn arduino_pins_from_code() {
        let code = "#define BUTTON 2\nconst int led = 13;\nint pot = A0; // potenciometer\nServo s;\nvoid setup(){ pinMode(led, OUTPUT); pinMode(BUTTON, INPUT_PULLUP); s.attach(9); }\nvoid loop(){ int v = analogRead(pot); analogWrite(6, v/4); digitalWrite(LED_BUILTIN, HIGH); tone(8, 440); // digitalWrite(4, HIGH)\n}";
        let p = pins_in(code);
        let get = |n: &str| p.iter().find(|x| x.pin == n).map(|x| (x.mode, x.name.clone()));
        assert_eq!(get("13"), Some(("out", Some("led".into()))));
        assert_eq!(get("2"), Some(("pull", Some("BUTTON".into()))));
        assert_eq!(get("9"), Some(("servo", None)));
        assert_eq!(get("6"), Some(("pwm", None)));
        assert_eq!(get("8"), Some(("tone", None)));
        assert_eq!(get("A0"), Some(("analog", Some("pot".into()))));
        assert!(get("4").is_none(), "commented-out code is ignored");
        assert_eq!(p.last().unwrap().pin, "A0");
    }

    #[test]
    fn arduino_sketch_dir_and_script() {
        let base = std::env::temp_dir().join(format!("flux-ard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("Blink")).unwrap();
        std::fs::write(base.join("Blink/Blink.ino"), "void setup(){}").unwrap();
        assert_eq!(sketch_dir(&base.join("Blink/Blink.ino").to_string_lossy()).unwrap(), base.join("Blink"));
        let c = core_of("arduino:avr:nano:cpu=atmega328old");
        assert_eq!(c, "arduino:avr");
        let _ = std::fs::remove_dir_all(&base);
    }
}
