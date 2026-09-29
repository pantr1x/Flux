// Programovacie jazyky na stiahnutie (to isté ako src/main/toolchains.js): overenie príkazom,
// inštalácia cez winget s priebehom, aktualizácie. Po inštalácii sa načíta nová PATH z registra.
use std::io::Read;
use std::process::{Command, Stdio};

pub struct Toolchain {
    pub id: &'static str,
    pub name: &'static str,
    pub detail: &'static str,
    pub probe: &'static [(&'static str, &'static [&'static str])],
    pub winget: &'static str,
    pub download: u32, // MB
    pub disk: u32,     // MB
    pub url: &'static str,
}

const fn tc(
    id: &'static str,
    name: &'static str,
    detail: &'static str,
    probe: &'static [(&'static str, &'static [&'static str])],
    winget: &'static str,
    download: u32,
    disk: u32,
    url: &'static str,
) -> Toolchain {
    Toolchain { id, name, detail, probe, winget, download, disk, url }
}

#[cfg(windows)]
const PY: &[(&str, &[&str])] = &[("py", &["-3", "--version"]), ("python", &["--version"])];
#[cfg(not(windows))]
const PY: &[(&str, &[&str])] = &[("python3", &["--version"])];

pub const TOOLCHAINS: [Toolchain; 15] = [
    tc("python", "Python", "", PY, "Python.Python.3.13", 30, 110, "https://www.python.org/downloads/"),
    tc("node", "Node.js", "JavaScript", &[("node", &["--version"])], "OpenJS.NodeJS.LTS", 30, 100, "https://nodejs.org/"),
    tc("java", "Java", "OpenJDK 21", &[("java", &["-version"])], "Microsoft.OpenJDK.21", 180, 320, "https://learn.microsoft.com/java/openjdk/download"),
    tc("cpp", "C / C++", "GCC (WinLibs)", &[("g++", &["--version"])], "BrechtSanders.WinLibs.POSIX.UCRT", 250, 1000, "https://winlibs.com/"),
    tc("go", "Go", "", &[("go", &["version"])], "GoLang.Go", 75, 250, "https://go.dev/dl/"),
    tc("csharp", "C#", ".NET 10 SDK", &[("dotnet", &["--list-sdks"])], "Microsoft.DotNet.SDK.10", 220, 750, "https://dotnet.microsoft.com/download"),
    tc("git", "Git", "GitHub & version control", &[("git", &["--version"])], "Git.Git", 65, 350, "https://git-scm.com/downloads"),
    tc("rust", "Rust", "GNU toolchain", &[("rustc", &["--version"])], "Rustlang.Rust.GNU", 300, 1100, "https://www.rust-lang.org/tools/install"),
    tc("ruby", "Ruby", "", &[("ruby", &["--version"])], "RubyInstallerTeam.Ruby.3.3", 30, 120, "https://rubyinstaller.org/"),
    tc("php", "PHP", "", &[("php", &["--version"])], "PHP.PHP.8.4", 32, 110, "https://windows.php.net/download/"),
    tc("perl", "Perl", "Strawberry Perl", &[("perl", &["-v"])], "StrawberryPerl.StrawberryPerl", 170, 600, "https://strawberryperl.com/"),
    tc("lua", "Lua", "", &[("lua", &["-v"])], "DEVCOM.Lua", 2, 5, "https://www.lua.org/download.html"),
    tc("zig", "Zig", "", &[("zig", &["version"])], "zig.zig", 50, 300, "https://ziglang.org/download/"),
    tc("r", "R", "statistics & data", &[("Rscript", &["--version"])], "RProject.R", 85, 300, "https://cran.r-project.org/bin/windows/base/"),
    // Juliaup z Microsoft Store – oficiálny spôsob inštalácie Julie vo Windows
    tc("julia", "Julia", "Juliaup", &[("julia", &["--version"])], "9NJNWW8PVKMN", 180, 600, "https://julialang.org/downloads/"),
];

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub installed: bool,
    pub version: String,
}

// príkaz bez okna konzoly
fn cmd(c: &str) -> Command {
    #[allow(unused_mut)]
    let mut k = Command::new(c);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        k.creation_flags(0x0800_0000);
    }
    k
}

fn run(c: &str, args: &[&str]) -> Option<String> {
    let o = cmd(c).args(args).stdin(Stdio::null()).output().ok()?;
    if !o.status.success() {
        return None;
    }
    Some(format!("{}\n{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)).trim().to_string())
}

// prvé číslo verzie: „version "21.0.2"“ má prednosť (java vypíše aj iné riadky), inak prvé x.y(.z)
fn version_of(out: &str) -> String {
    let low = out.to_lowercase();
    let grab = |s: &str| -> String {
        let s = s.trim_start_matches(['"', 'v', ' ']);
        let v: String = s.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        v.trim_end_matches('.').to_string()
    };
    if let Some(i) = low.find("version") {
        let v = grab(&out[i + 7..]);
        if v.contains('.') {
            return v;
        }
    }
    let b = out.as_bytes();
    for i in 0..b.len() {
        let prev_ok = i == 0 || !(b[i - 1].is_ascii_digit() || b[i - 1] == b'.');
        if prev_ok && b[i].is_ascii_digit() {
            let v = grab(&out[i..]);
            if v.matches('.').count() >= 1 && v.split('.').all(|p| !p.is_empty()) {
                return v.split('.').take(3).collect::<Vec<_>>().join(".");
            }
        }
    }
    String::new()
}

pub fn probe_one(t: &Toolchain) -> Status {
    for (c, args) in t.probe {
        if let Some(out) = run(c, args) {
            // Windows „Store“ alias pre python vypíše len návod – ten neberieme
            let low = out.to_lowercase();
            if low.contains("microsoft store") || low.contains("was not found") || (t.id == "csharp" && version_of(&out).is_empty()) {
                continue;
            }
            return Status { installed: true, version: version_of(&out) };
        }
    }
    Status::default()
}

// stav všetkých jazykov (paralelne) + či je k dispozícii winget
pub fn status() -> (Vec<Status>, bool) {
    let handles: Vec<_> = (0..TOOLCHAINS.len()).map(|i| std::thread::spawn(move || probe_one(&TOOLCHAINS[i]))).collect();
    let list = handles.into_iter().map(|h| h.join().unwrap_or_default()).collect();
    (list, can_install())
}

pub fn can_install() -> bool {
    cfg!(windows) && run("winget", &["--version"]).is_some()
}

// Po inštalácii má Windows novú PATH len v registri – načítame ju, aby Flux nový jazyk hneď našiel.
pub fn refresh_path() {
    if !cfg!(windows) {
        return;
    }
    if let Some(out) = run("powershell.exe", &["-NoProfile", "-Command", "[Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')"]) {
        let cur = std::env::var("PATH").unwrap_or_default();
        let mut seen: Vec<String> = vec![];
        for p in out.lines().next().unwrap_or("").split(';').chain(cur.split(';')) {
            if !p.is_empty() && !seen.iter().any(|s| s.eq_ignore_ascii_case(p)) {
                seen.push(p.to_string());
            }
        }
        std::env::set_var("PATH", seen.join(";"));
    }
    add_r_path();
}

// Inštalátor R nepridáva R do PATH – pridáme priečinok bin najnovšej verzie.
pub fn add_r_path() {
    if !cfg!(windows) {
        return;
    }
    let roots = [std::env::var("ProgramFiles").ok(), std::env::var("LOCALAPPDATA").ok().map(|l| format!("{l}\\Programs"))];
    for root in roots.into_iter().flatten() {
        let Ok(rd) = std::fs::read_dir(std::path::Path::new(&root).join("R")) else { continue };
        let mut dirs: Vec<String> = rd.flatten().map(|d| d.file_name().to_string_lossy().to_string()).filter(|d| d.starts_with("R-")).collect();
        dirs.sort_by_key(|d| d.trim_start_matches("R-").split('.').map(|n| n.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>());
        if let Some(newest) = dirs.last() {
            let bin = std::path::Path::new(&root).join("R").join(newest).join("bin");
            let path = std::env::var("PATH").unwrap_or_default();
            if bin.join("Rscript.exe").exists() && !path.contains(&*bin.to_string_lossy()) {
                std::env::set_var("PATH", format!("{path};{}", bin.to_string_lossy()));
            }
        }
        return;
    }
}

// Inštalácia (alebo aktualizácia) cez winget; priebeh = (percentá, posledný riadok).
// Úspech rozhoduje nové overenie – winget vráti chybu aj keď už je jazyk nainštalovaný.
pub fn install(id: &str, upgrade: bool, mut progress: impl FnMut(Option<u32>, &str)) -> Result<Status, String> {
    let t = TOOLCHAINS.iter().find(|t| t.id == id).ok_or("Unknown language")?;
    if !cfg!(windows) {
        return Err(format!("Automatic install works on Windows. Download it from {}", t.url));
    }
    let verb = if upgrade { "upgrade" } else { "install" };
    let mut child = cmd("winget")
        .args([verb, "--id", t.winget, "-e", "--silent", "--accept-source-agreements", "--accept-package-agreements", "--disable-interactivity"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| format!("winget was not found. Download it from {}", t.url))?;
    let mut out = child.stdout.take().ok_or("winget")?;
    let mut buf = [0u8; 4096];
    let mut last = String::new();
    while let Ok(n) = out.read(&mut buf) {
        if n == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&buf[..n]).to_string();
        // posledné „NN %“ v kúsku výstupu
        let pct = text.match_indices('%').filter_map(|(i, _)| text[..i].trim_end().rsplit(|c: char| !c.is_ascii_digit()).next().and_then(|d| d.parse::<u32>().ok())).filter(|p| *p <= 100).last();
        // riadok bez pruhu priebehu (█▒)
        if let Some(l) =
            text.split(['\r', '\n']).map(|l| l.split(['█', '▒', '░']).next().unwrap_or("").trim().to_string()).filter(|l| l.chars().count() > 3 && !l.chars().all(|c| "-\\|/ ".contains(c))).last()
        {
            last = l;
        }
        progress(pct, &last);
    }
    let code = child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    refresh_path();
    let st = probe_one(t);
    if st.installed {
        Ok(st)
    } else if last.is_empty() {
        Err(format!("Installation failed (code {code})."))
    } else {
        Err(last)
    }
}

// ktoré nainštalované jazyky majú novšiu verziu (winget upgrade vypíše tabuľku): id → (teraz, nová)
pub fn check_updates() -> Vec<(&'static str, String, String)> {
    if !cfg!(windows) {
        return vec![];
    }
    let Some(out) = run("winget", &["upgrade", "--accept-source-agreements", "--disable-interactivity"]) else { return vec![] };
    let mut found = vec![];
    for line in out.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        for t in &TOOLCHAINS {
            if let Some(i) = cols.iter().position(|c| c.eq_ignore_ascii_case(t.winget)) {
                if let (Some(a), Some(b)) = (cols.get(i + 1), cols.get(i + 2)) {
                    found.push((t.id, a.to_string(), b.to_string()));
                }
            }
        }
    }
    found
}
