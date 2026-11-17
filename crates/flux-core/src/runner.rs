// ▶ Run: ktorý program spustí súbor (ako src/main/runner.js → commandFor) a spustenie v pseudoterminále.
use crate::pty::Pty;
use serde_json::{json, Value};
use std::path::Path;
use crate::Emit;

pub struct Cmd {
    pub cmd: String,
    pub args: Vec<String>,
}

fn c(cmd: &str, args: &[&str]) -> Option<Cmd> {
    Some(Cmd { cmd: cmd.into(), args: args.iter().map(|s| s.to_string()).collect() })
}

fn git_bash() -> Option<String> {
    ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .chain(std::env::var("LOCALAPPDATA").ok().map(|l| format!("{l}\\Programs")))
        .map(|r| format!("{r}\\Git\\bin\\bash.exe"))
        .find(|p| Path::new(p).exists())
}

// C/C++/Rust: najprv preložiť, potom spustiť (ako Code Runner vo VS Code).
fn compile_and_run(src: &str, compiler: &str) -> Option<Cmd> {
    let stem = Path::new(src).file_stem()?.to_string_lossy().to_string();
    if cfg!(windows) {
        let out = format!("{stem}.exe");
        let q = |s: &str| format!("'{}'", s.replace('\'', "''"));
        c("powershell.exe", &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &format!("& {compiler} {} -o {}; if ($?) {{ & {} }}", q(src), q(&out), q(&format!(".\\{out}")))])
    } else {
        c("bash", &["-c", &format!("{compiler} \"$1\" -o \"$2\" && \"./$2\""), "flux", src, &stem])
    }
}

pub fn command_for(file: &str, python: &str, lang: &str) -> Option<Cmd> {
    let p = Path::new(file);
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let base = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if ext.is_empty() && lang == "python" {
        return (!python.is_empty()).then(|| Cmd { cmd: python.into(), args: vec!["-u".into(), file.into()] });
    }
    match ext.as_str() {
        "py" | "pyw" => (!python.is_empty()).then(|| Cmd { cmd: python.into(), args: vec!["-u".into(), file.into()] }),
        "js" | "mjs" | "cjs" => c("node", &[file]),
        "ts" | "mts" | "cts" => c("node", &["--experimental-strip-types", "--no-warnings", file]),
        "pl" => c("perl", &[file]),
        "java" => c("java", &[&base]),
        "go" => c("go", &["run", &base]),
        "cs" => c("dotnet", &["run", &base]),
        "rs" => compile_and_run(&base, "rustc"),
        "rb" => c("ruby", &[&base]),
        "php" => c("php", &[&base]),
        "lua" => c("lua", &[&base]),
        "zig" => c("zig", &["run", &base]),
        "r" => c("Rscript", &[&base]),
        "jl" => c("julia", &[&base]),
        "dart" => c("dart", &["run", &base]),
        "swift" => c("swift", &[&base]),
        "kts" => c("kotlinc", &["-script", &base]),
        "c" => compile_and_run(&base, "gcc"),
        "cpp" | "cc" | "cxx" => compile_and_run(&base, "g++"),
        "ino" | "pde" => arduino(file),
        "bat" | "cmd" if cfg!(windows) => c("cmd.exe", &["/d", "/c", file]),
        "ps1" => c(if cfg!(windows) { "powershell.exe" } else { "pwsh" }, &["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", file]),
        "sh" if !cfg!(windows) => c("bash", &[file]),
        "sh" => git_bash().and_then(|b| c(&b, &[file])),
        _ => None,
    }
}

// Spustí príkaz a pošle run:start (rozhranie potom čaká na run:data / run:exit).
pub fn start(pty: &Pty, emit: &Emit, cmd: &Cmd, cwd: &str, label: &str) -> Value {
    match pty.spawn(emit, &cmd.cmd, &cmd.args, cwd, &[("PYTHONIOENCODING", "utf-8"), ("PYTHONUTF8", "1")], "run:data", "run:exit") {
        Ok(pid) => {
            emit("run:start", json!({ "label": label, "cwd": cwd, "pid": pid, "pty": true }));
            json!({ "ok": true })
        }
        Err(e) => {
            emit("run:start", json!({ "label": label, "cwd": cwd, "pid": 0 }));
            emit("run:exit", json!({ "code": -1, "error": e, "ms": 0 }));
            json!({ "ok": true })
        }
    }
}

// ---------- Arduino ----------
// arduino-cli: v PATH, inak kam ho dá winget (ArduinoSA.CLI)
fn arduino_cli() -> Option<String> {
    let ok = |c: &str| {
        let mut cmd = std::process::Command::new(c);
        cmd.arg("version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.status().is_ok_and(|s| s.success())
    };
    if ok("arduino-cli") {
        return Some("arduino-cli".into());
    }
    ["ProgramFiles", "LOCALAPPDATA"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .flat_map(|r| [format!("{r}\\Arduino CLI\\arduino-cli.exe"), format!("{r}\\Programs\\Arduino CLI\\arduino-cli.exe")])
        .find(|p| Path::new(p).exists())
}

// rýchlosť sériovej linky zo Serial.begin(N) v náčrte (inak 9600)
pub fn arduino_baud(src: &str) -> u32 {
    src.find("Serial.begin(").and_then(|i| src[i + 13..].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()).unwrap_or(9600)
}

// náčrt musí byť v priečinku s rovnakým menom (blink/blink.ino); inak sa skopíruje do dočasného aj so svojimi .h/.cpp
pub fn arduino_sketch_dir(file: &str) -> Option<String> {
    let p = Path::new(file);
    let stem = p.file_stem()?.to_string_lossy().to_string();
    let parent = p.parent()?;
    if parent.file_name().is_some_and(|n| n.to_string_lossy() == stem) {
        return Some(parent.to_string_lossy().to_string());
    }
    let dir = std::env::temp_dir().join("flux-arduino").join(&stem);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::copy(p, dir.join(format!("{stem}.ino"))).ok()?;
    for e in std::fs::read_dir(parent).ok()?.flatten() {
        let f = e.path();
        let ext = f.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
        if matches!(ext.as_str(), "h" | "hpp" | "c" | "cpp") {
            let _ = std::fs::copy(&f, dir.join(e.file_name()));
        }
    }
    Some(dir.to_string_lossy().to_string())
}

// ▶ Run: nájde pripojenú dosku, doinštaluje jej podporu (raz), skompiluje a nahrá; potom sériový monitor vo Výstupe.
// Bez dosky len skontroluje, či sa kód skompiluje (doska z nastavenia arduinoBoard, predvolene Arduino Uno).
fn arduino(file: &str) -> Option<Cmd> {
    let src = std::fs::read_to_string(file).unwrap_or_default();
    let baud = arduino_baud(&src).to_string();
    let def = crate::settings::load()["arduinoBoard"].as_str().unwrap_or("arduino:avr:uno").to_string();
    let Some(cli) = arduino_cli() else {
        let msg = "Arduino CLI is not installed. Install it in Settings > Languages > Arduino (or from https://arduino.github.io/arduino-cli/).";
        return if cfg!(windows) { c("powershell.exe", &["-NoProfile", "-Command", &format!("Write-Host '{msg}'; exit 1")]) } else { c("bash", &["-c", &format!("echo '{msg}'; exit 1")]) };
    };
    let dir = arduino_sketch_dir(file)?;
    if cfg!(windows) {
        let q = |s: &str| format!("'{}'", s.replace('\'', "''"));
        let script = format!(
            "$cli={}; $dir={}; $fqbn=''; $port=''; $baud={};\
             Write-Host ([char]0x203A + ' Looking for a connected board...');\
             $j = & $cli board list --format json 2>$null | ConvertFrom-Json;\
             $ps = if ($j.detected_ports) {{ $j.detected_ports }} else {{ $j }};\
             foreach ($d in $ps) {{ if ($d.matching_boards) {{ $port = $d.port.address; $fqbn = $d.matching_boards[0].fqbn; break }} }};\
             if (-not $port) {{ foreach ($d in $ps) {{ if ($d.port.protocol -eq 'serial' -and $d.port.address -match 'COM') {{ $port = $d.port.address; break }} }} }};\
             if (-not $fqbn) {{ $fqbn = {} }};\
             $core = ($fqbn -split ':')[0..1] -join ':';\
             if (-not (& $cli core list 2>$null | Select-String -SimpleMatch $core)) {{ Write-Host ([char]0x203A + \" Installing support for $core (only once)...\"); & $cli core update-index; & $cli core install $core; if (-not $?) {{ exit 1 }} }};\
             if ($port) {{ Write-Host ([char]0x203A + \" $fqbn on $port - compiling and uploading...\"); & $cli compile --fqbn $fqbn --upload -p $port $dir; if (-not $?) {{ exit 1 }};\
               Write-Host ([char]0x203A + \" Serial monitor ($baud baud) - press Stop to end\"); Start-Sleep -Milliseconds 1200; & $cli monitor -p $port --config \"baudrate=$baud\" }}\
             else {{ Write-Host ([char]0x203A + \" No board connected - only checking the code ($fqbn)\"); & $cli compile --fqbn $fqbn $dir }}",
            q(&cli),
            q(&dir),
            baud,
            q(&def)
        );
        c("powershell.exe", &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
    } else {
        let script = r#"cli="$1"; dir="$2"; def="$3"; baud="$4"
echo "› Looking for a connected board…"
list=$("$cli" board list 2>/dev/null)
read -r port fqbn <<EOF
$(printf '%s\n' "$list" | awk 'NR>1{for(i=2;i<=NF;i++) if($i ~ /^[A-Za-z0-9_.-]+:[A-Za-z0-9_.-]+:[A-Za-z0-9_.-]+/){print $1, $i; exit}}')
EOF
[ -z "$port" ] && port=$(printf '%s\n' "$list" | awk 'NR>1 && $1 ~ /tty(USB|ACM)|cu\./ {print $1; exit}')
[ -z "$fqbn" ] && fqbn="$def"
core="${fqbn%:*}"
if ! "$cli" core list 2>/dev/null | grep -q "^$core "; then
  echo "› Installing support for $core (only once)…"
  "$cli" core update-index && "$cli" core install "$core" || exit 1
fi
if [ -n "$port" ]; then
  echo "› $fqbn on $port – compiling and uploading…"
  "$cli" compile --fqbn "$fqbn" --upload -p "$port" "$dir" || exit 1
  echo "› Serial monitor ($baud baud) – press Stop to end"
  sleep 1.2
  exec "$cli" monitor -p "$port" --config "baudrate=$baud"
else
  echo "› No board connected – only checking the code ($fqbn)"
  exec "$cli" compile --fqbn "$fqbn" "$dir"
fi"#;
        c("bash", &["-c", script, "flux-arduino", &cli, &dir, &def, &baud])
    }
}

pub fn run_file(pty: &Pty, emit: &Emit, file: &str, python: &str, lang: &str) -> Value {
    let cwd = Path::new(file).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let label = Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    match command_for(file, python, lang) {
        Some(cmd) => start(pty, emit, &cmd, &cwd, &label),
        None => json!({ "ok": false, "error": "Can't run this type of file yet." }),
    }
}

// Terminál: PowerShell na Windows, inak shell používateľa.
pub fn shell_command() -> Cmd {
    if cfg!(windows) {
        return Cmd { cmd: "powershell.exe".into(), args: vec!["-NoLogo".into()] };
    }
    let sh = std::env::var("SHELL").ok().filter(|s| Path::new(s).exists()).unwrap_or_else(|| "/bin/bash".into());
    Cmd { cmd: sh, args: vec!["-i".into()] }
}

#[cfg(test)]
mod arduino_tests {
    use super::*;

    #[test]
    fn arduino_baud_and_sketch() {
        assert_eq!(arduino_baud("void setup(){ Serial.begin(115200); }"), 115200);
        assert_eq!(arduino_baud("void setup(){}"), 9600);
        let d = std::env::temp_dir().join("flux-ino-test");
        let _ = std::fs::create_dir_all(d.join("blink"));
        std::fs::write(d.join("blink/blink.ino"), "void setup(){}").unwrap();
        std::fs::write(d.join("sketch.ino"), "void setup(){}").unwrap();
        std::fs::write(d.join("pins.h"), "#define X 1").unwrap();
        // v správnom priečinku ostane, inak sa skopíruje (aj s .h)
        assert_eq!(arduino_sketch_dir(&d.join("blink/blink.ino").to_string_lossy()).unwrap(), d.join("blink").to_string_lossy());
        let t = arduino_sketch_dir(&d.join("sketch.ino").to_string_lossy()).unwrap();
        assert!(Path::new(&t).join("sketch.ino").exists() && Path::new(&t).join("pins.h").exists());
    }
}
