// Arduino (aj Elegoo a iné klony): rozpoznanie dosky na USB, oficiálny arduino-cli (Flux si ho stiahne sám),
// kompilácia + nahratie, sériový monitor, knižnice, firmvér (Blink, StandardFirmata) a piny pre schému zapojenia.
// Všetky príkazy bežia vo Výstupe (pty) – vidno priebeh aj chyby ako pri ▶ Run.
use flux_core::runner::Cmd;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

// typy dosiek na výber (fqbn, názov); Elegoo Uno R3 / Mega 2560 / Nano sú klony týchto
pub const BOARDS: [(&str, &str); 7] = [
    ("arduino:avr:uno", "Arduino Uno (Elegoo Uno R3)"),
    ("arduino:avr:mega", "Arduino Mega 2560 (Elegoo Mega)"),
    ("arduino:avr:nano:cpu=atmega328old", "Arduino Nano – old bootloader (most clones)"),
    ("arduino:avr:nano", "Arduino Nano"),
    ("arduino:avr:leonardo", "Arduino Leonardo"),
    ("arduino:avr:micro", "Arduino Micro"),
    ("arduino:avr:pro:cpu=16MHzatmega328", "Arduino Pro Mini 5V"),
];

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
        ("10c4", "ea60") => ("ESP32 / ESP8266 board (CP210x)", None, Some("CP210x")),
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

#[cfg(test)]
mod tests {
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
