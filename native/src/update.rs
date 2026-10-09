// Aktualizácie Flux Native z vetvy ci-native (native.yml tam dáva Flux-Native.exe a native.json):
// kontrola → stiahnutie vedľa programu (Flux-Native.new) → kontrola sha256 → výmena a nový štart.
// Spustený .exe sa na Windows nedá prepísať: stiahnutý program (.new) počká, kým Flux skončí, a zapíše sa na jeho miesto
// (apply_update); predošlá verzia ostane ako kópia .old a zmaže sa 10 s po štarte novej.
// Sťahuje systémový curl (súčasť Windows 10/11), takže Flux nepotrebuje vlastného HTTP klienta.
use eframe::egui;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// commit, z ktorého je tento program (native.yml → GITHUB_SHA); „dev“ = vlastná zostava
pub const SHA: &str = env!("FLUX_SHA");

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Idle,
    Checking,
    Latest,
    Available { version: String, sha: String },
    Downloading { got: u64, total: u64 },
    Ready { version: String },
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
struct Info {
    version: String,
    sha: String,
    sha256: String,
    size: u64,
    // rozdiel oproti predošlej zostave (Flux-Native.patch): stiahne sa len zmena, nie celý program
    patch_from: String,
    patch_sha256: String,
    patch_size: u64,
}

pub struct Updater {
    pub state: Arc<Mutex<State>>,
    info: Arc<Mutex<Option<Info>>>,
    pub last_check: f64,
    ctx: Option<egui::Context>, // pre opätovné stiahnutie z install()
}

// Windows: vetva ci-native (Flux-Native.exe); Linux: ci-native-linux (Flux-Native) – ci-native sa pri každom vydaní zakladá odznova
fn base() -> String {
    let branch = if cfg!(windows) { "ci-native" } else { "ci-native-linux" };
    std::env::var("FLUX_UPDATE_URL").unwrap_or_else(|_| format!("https://raw.githubusercontent.com/pantr1x/Flux/{branch}/"))
}

// názov programu na serveri aktualizácií
fn asset() -> &'static str {
    if cfg!(windows) { "Flux-Native.exe" } else { "Flux-Native" }
}

fn curl() -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new("curl");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // bez okna konzoly
    }
    c
}

fn exe() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

// po aktualizácii: starý program (.old) ostane, kým nová verzia nebeží aspoň 20 s (keep_old = true pri štarte);
// ak nová verzia hneď spadne, rollback() vráti starú
pub fn cleanup(keep_old: bool) {
    if let Some(e) = exe() {
        if !keep_old {
            let _ = std::fs::remove_file(e.with_extension("old"));
            // staršie kópie, ktoré pri aktualizácii ešte bežali (Flux-Native.old-<čas>)
            let stem = e.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(Ok(rd)) = e.parent().map(std::fs::read_dir) {
                for f in rd.flatten() {
                    let n = f.file_name().to_string_lossy().to_string();
                    if n.starts_with(&format!("{stem}.old-")) {
                        let _ = std::fs::remove_file(f.path());
                    }
                }
            }
        } else {
            // len pri štarte: nedokončené stiahnutie z minula. Neskôr (keep_old = false) už .new môže byť
            // čerstvo stiahnutá aktualizácia – zmazať ju bola chyba, pre ktorú sa aktualizácie nedali nainštalovať.
            let _ = std::fs::remove_file(e.with_extension("new"));
        }
    }
}

// nová verzia spadla hneď po štarte a vedľa je predošlá (.old): tá sa po skončení tohto procesu zapíše späť a spustí
// len hneď po aktualizácii (--updated) a nie po návrate (--rolledback): inak by chyba, ktorú nespôsobila nová verzia
// (napr. grafika), menila programy dokola
pub fn rollback() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == "--updated") || args.iter().any(|a| a == "--rolledback") {
        return false;
    }
    let Some(e) = exe() else { return false };
    let old = e.with_extension("old");
    if !old.exists() {
        return false;
    }
    let mut c = std::process::Command::new(&old);
    c.arg("--apply-update").arg(std::process::id().to_string()).arg(&e).arg("--relaunch").arg("--rolledback");
    spawn_detached(&mut c).is_ok()
}

// pomocník mimo úlohy (job) volajúceho, aby ho zavretie Fluxu neukončilo
fn spawn_detached(c: &mut std::process::Command) -> std::io::Result<std::process::Child> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_BREAKAWAY_FROM_JOB; úloha, ktorá to nedovolí, vráti chybu → bez neho
        c.creation_flags(0x0100_0000);
        if let Ok(ch) = c.spawn() {
            return Ok(ch);
        }
        c.creation_flags(0);
    }
    c.spawn()
}

#[cfg(windows)]
pub(crate) fn wait_exit(pid: u32, max: Duration) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !h.is_null() {
            WaitForSingleObject(h, max.as_millis() as u32);
            CloseHandle(h);
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn wait_exit(pid: u32, max: Duration) {
    let t = std::time::Instant::now();
    while std::path::Path::new(&format!("/proc/{pid}")).exists() && t.elapsed() < max {
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn write_in_place(target: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().write(true).truncate(true).open(target)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    if f.metadata()?.len() != bytes.len() as u64 {
        return Err(std::io::Error::other("short write"));
    }
    Ok(())
}

// Flux-Native.new --apply-update <pid> <Flux-Native.exe> [--relaunch <argumenty…>]
// (aj Flux-Native.old pri návrate): počká na koniec Fluxu, zálohuje ho do .old, zapíše sa na jeho miesto a spustí ho
pub fn apply_update(args: &[String]) {
    let (Some(pid), Some(target)) = (args.get(2).and_then(|p| p.parse::<u32>().ok()), args.get(3)) else { return };
    let target = PathBuf::from(target);
    let relaunch = args.get(4).is_some_and(|a| a == "--relaunch");
    wait_exit(pid, Duration::from_secs(60));
    let Some(me) = exe() else { return };
    let Ok(bytes) = std::fs::read(&me) else { return };
    let old = target.with_extension("old");
    // záloha pre rollback – kópia, nie premenovanie (pri návrate je zálohou tento program sám)
    if me.file_name() != old.file_name() {
        let _ = std::fs::copy(&target, &old);
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut ok = false;
    while !ok {
        ok = write_in_place(&target, &bytes).is_ok();
        if !ok {
            if std::time::Instant::now() > deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(200)); // antivírus, Flux ešte končí
        }
    }
    // stále použitý (starý most MCP z tohto súboru) → ako predtým: premenovať nabok a skopírovať sa
    if !ok {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let aside = target.with_extension(format!("old-{t}"));
        if std::fs::rename(&target, &aside).is_ok() && std::fs::copy(&me, &target).is_err() {
            let _ = std::fs::rename(&aside, &target);
        }
    }
    if relaunch {
        let mut c = std::process::Command::new(&target);
        // testovacie kroky (FLUX_TEST) patria len pôvodnému štartu, inak by sa nová verzia aktualizovala dokola
        c.args(&args[5..]).arg("--updated").env_remove("FLUX_TEST");
        for _ in 0..10 {
            if spawn_detached(&mut c).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(300));
        }
    }
}

// nový štart po aktualizácii: pomocník (.new) možno ešte končí, takže ho zmazať o chvíľu
pub fn remove_helper() {
    let Some(new) = exe().map(|e| e.with_extension("new")) else { return };
    std::thread::spawn(move || {
        for _ in 0..25 {
            if std::fs::remove_file(&new).is_ok() || !new.exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
}

pub fn enabled() -> bool {
    SHA != "dev" || std::env::var("FLUX_UPDATE_URL").is_ok()
}

impl Default for Updater {
    fn default() -> Self {
        Self { state: Arc::new(Mutex::new(State::Idle)), info: Default::default(), last_check: -1e9, ctx: None }
    }
}

impl Updater {
    pub fn state(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    fn set(&self, s: State, ctx: &egui::Context) {
        *self.state.lock().unwrap() = s;
        ctx.request_repaint();
    }

    // zistí, či je v ci-native novší program (iný commit); auto = po stiahnutí hneď pripraviť
    pub fn check(&mut self, ctx: &egui::Context, now: f64, auto_download: bool) {
        self.run_check(ctx, now, auto_download, false);
    }

    // vývojár: stiahne aktuálnu zostavu z ci-native aj keď je to ten istý commit (aj do vlastnej zostavy)
    pub fn reinstall(&mut self, ctx: &egui::Context) {
        if matches!(self.state(), State::Ready { .. }) {
            *self.state.lock().unwrap() = State::Idle;
        }
        self.run_check(ctx, self.last_check, true, true);
    }

    fn run_check(&mut self, ctx: &egui::Context, now: f64, auto_download: bool, force: bool) {
        if (!enabled() && !force) || matches!(self.state(), State::Checking | State::Downloading { .. }) {
            return;
        }
        // pripravená aktualizácia: bez súboru (.new zmizol) → normálna kontrola; so súborom sa hľadá ešte novšia
        // zostava (predtým sa v tomto stave už nikdy nekontrolovalo a novšie verzie zostali neviditeľné)
        let ready = matches!(self.state(), State::Ready { .. });
        let have_new = exe().is_some_and(|e| e.with_extension("new").exists());
        let ready_sha = if ready && have_new { self.info.lock().unwrap().as_ref().map(|i| i.sha.clone()) } else { None };
        if ready && ready_sha.is_none() && have_new {
            return;
        }
        self.last_check = now;
        self.ctx = Some(ctx.clone());
        if ready_sha.is_none() {
            self.set(State::Checking, ctx);
        }
        let (state, info, ctx) = (self.state.clone(), self.info.clone(), ctx.clone());
        std::thread::spawn(move || {
            let out = curl().args(["-fsSL", "--max-time", "20", &format!("{}native.json", base())]).output();
            let parsed = out.ok().filter(|o| o.status.success()).and_then(|o| serde_json::from_slice::<Value>(&o.stdout).ok());
            let Some(j) = parsed else {
                if ready_sha.is_some() {
                    return; // pripravená verzia ostáva
                }
                *state.lock().unwrap() = State::Error(crate::i18n::t("Could not check for updates"));
                ctx.request_repaint();
                return;
            };
            let i = Info {
                version: j["version"].as_str().unwrap_or("").into(),
                sha: j["sha"].as_str().unwrap_or("").into(),
                sha256: j["sha256"].as_str().unwrap_or("").to_lowercase(),
                size: j["size"].as_u64().unwrap_or(0),
                patch_from: j["patch"]["from"].as_str().unwrap_or("").into(),
                patch_sha256: j["patch"]["sha256"].as_str().unwrap_or("").to_lowercase(),
                patch_size: j["patch"]["size"].as_u64().unwrap_or(0),
            };
            if let Some(rs) = &ready_sha {
                // stiahnutá je stále najnovšia → nič; inak stiahnuť novšiu namiesto nej
                if i.sha.is_empty() || &i.sha == rs {
                    return;
                }
                *info.lock().unwrap() = Some(i);
                download_now(&state, &info, &ctx);
                return;
            }
            let newer = !i.sha.is_empty() && (i.sha != SHA || force);
            *state.lock().unwrap() = if newer { State::Available { version: i.version.clone(), sha: i.sha.clone() } } else { State::Latest };
            *info.lock().unwrap() = Some(i);
            ctx.request_repaint();
            if newer && auto_download {
                download_now(&state, &info, &ctx);
            }
        });
    }

    pub fn download(&self, ctx: &egui::Context) {
        if !matches!(self.state(), State::Available { .. } | State::Error(_)) {
            return;
        }
        let (state, info, ctx) = (self.state.clone(), self.info.clone(), ctx.clone());
        std::thread::spawn(move || download_now(&state, &info, &ctx));
    }

    // vymení program a spustí nový (s rovnakými argumentmi); volajúci potom zavrie okno.
    // relaunch = false: pri bežnom zavretí Fluxu sa len vymení (autoInstallOnAppQuit v Electron Fluxe)
    pub fn install(&self, relaunch: bool) -> Result<(), String> {
        self.install_with(relaunch, &[])
    }

    // extra: argumenty navyše pre nový štart (--background pri tichej aktualizácii)
    pub fn install_with(&self, relaunch: bool, extra: &[&str]) -> Result<(), String> {
        if !matches!(self.state(), State::Ready { .. }) {
            return Err("not ready".into());
        }
        let e = exe().ok_or("no exe")?;
        let new = e.with_extension("new");
        // stiahnutý súbor zmizol (antivírus, upratovanie starej verzie) → stiahnuť znova, potom znova „Restart to update“
        if !new.exists() {
            let (state, info) = (self.state.clone(), self.info.clone());
            let has = info.lock().unwrap().is_some();
            if let (Some(ctx), true) = (self.ctx.clone(), has) {
                std::thread::spawn(move || download_now(&state, &info, &ctx));
            } else {
                *self.state.lock().unwrap() = State::Idle;
            }
            return Err(crate::i18n::t("The update file was missing, so Flux is downloading it again."));
        }
        // stiahnutý program po zavretí Fluxu prepíše Flux-Native.exe na mieste (apply_update) – súbor ostane ten istý,
        // takže pripnutie na paneli úloh platí ďalej (premenovanie na .old ho presunulo na .old, ktorý sa potom zmazal)
        let mut c = std::process::Command::new(&new);
        c.arg("--apply-update").arg(std::process::id().to_string()).arg(&e);
        if relaunch {
            let skip = ["--background", "--minimized", "--updated", "--rolledback"];
            c.arg("--relaunch").args(std::env::args_os().skip(1).filter(|a| !skip.iter().any(|s| a == s))).args(extra);
        }
        spawn_detached(&mut c).map_err(|x| x.to_string())?;
        *self.state.lock().unwrap() = State::Idle;
        Ok(())
    }
}

// ---------- rozdielové aktualizácie (zstd „patch-from“: nová zostava skomprimovaná voči predošlej) ----------

const PATCH_WINDOW: u32 = 26; // 64 MB okno – pokryje starý aj nový program

// CI: Flux-Native.exe --make-patch <stará> <nová> <výstup>
pub fn make_patch(old: &std::path::Path, new: &std::path::Path, out: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;
    let (old_b, new_b) = (std::fs::read(old)?, std::fs::read(new)?);
    let mut enc = zstd::stream::write::Encoder::with_ref_prefix(std::fs::File::create(out)?, 19, &old_b)?;
    enc.window_log(PATCH_WINDOW)?;
    enc.long_distance_matching(true)?;
    enc.include_checksum(true)?;
    enc.write_all(&new_b)?;
    enc.finish()?;
    Ok(())
}

pub fn apply_patch(old: &[u8], patch: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut dec = zstd::stream::read::Decoder::with_ref_prefix(patch, old)?;
    dec.window_log_max(PATCH_WINDOW)?;
    let mut out = Vec::new();
    dec.read_to_end(&mut out)?;
    Ok(out)
}

// stiahne súbor do `to` cez curl s priebehom (veľkosť na disku)
fn fetch(url: &str, to: &std::path::Path, total: u64, set: &dyn Fn(State)) -> bool {
    let child = curl().args(["-fsSL", "--max-time", "600", "-o"]).arg(to).arg(url).spawn();
    let Ok(mut child) = child else { return false };
    loop {
        match child.try_wait() {
            Ok(Some(st)) => return st.success(),
            Ok(None) => {
                let got = std::fs::metadata(to).map(|m| m.len()).unwrap_or(0);
                set(State::Downloading { got, total });
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(_) => return false,
        }
    }
}

// rozdiel pre túto zostavu → hotový nový program v `new`; false = treba stiahnuť celý
fn try_patch(i: &Info, e: &std::path::Path, new: &std::path::Path, set: &dyn Fn(State)) -> bool {
    if i.patch_from.is_empty() || i.patch_from != SHA || i.patch_sha256.is_empty() {
        return false;
    }
    let pf = e.with_extension("patch");
    let _ = std::fs::remove_file(&pf);
    let ok = fetch(&format!("{}Flux-Native.patch", base()), &pf, i.patch_size, set);
    let patch = if ok { std::fs::read(&pf).ok() } else { None };
    let _ = std::fs::remove_file(&pf);
    let Some(patch) = patch.filter(|p| format!("{:x}", Sha256::digest(p)) == i.patch_sha256) else { return false };
    let Ok(old) = std::fs::read(e) else { return false };
    let Ok(out) = apply_patch(&old, &patch) else { return false };
    if format!("{:x}", Sha256::digest(&out)) != i.sha256 {
        return false;
    }
    std::fs::write(new, out).is_ok()
}

fn download_now(state: &Arc<Mutex<State>>, info: &Arc<Mutex<Option<Info>>>, ctx: &egui::Context) {
    let Some(i) = info.lock().unwrap().clone() else { return };
    let Some(e) = exe() else { return };
    let new = e.with_extension("new");
    let _ = std::fs::remove_file(&new);
    let set = |s: State| {
        *state.lock().unwrap() = s;
        ctx.request_repaint();
    };
    // najprv len rozdiel (desiatky až stovky kB), inak celý program
    if try_patch(&i, &e, &new, &set) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755));
        }
        set(State::Ready { version: i.version });
        return;
    }
    let _ = std::fs::remove_file(&new);
    set(State::Downloading { got: 0, total: i.size });
    let child = curl().args(["-fsSL", "--max-time", "600", "-o"]).arg(&new).arg(format!("{}{}", base(), asset())).spawn();
    let Ok(mut child) = child else {
        return set(State::Error(crate::i18n::t("Download failed")));
    };
    // priebeh = veľkosť súboru na disku
    let ok = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st.success(),
            Ok(None) => {
                let got = std::fs::metadata(&new).map(|m| m.len()).unwrap_or(0);
                set(State::Downloading { got, total: i.size });
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(_) => break false,
        }
    };
    let bytes = if ok { std::fs::read(&new).ok() } else { None };
    let good = bytes.map(|b| i.sha256.is_empty() || format!("{:x}", Sha256::digest(&b)) == i.sha256).unwrap_or(false);
    if !good {
        let _ = std::fs::remove_file(&new);
        return set(State::Error(crate::i18n::t("Download failed")));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755));
    }
    set(State::Ready { version: i.version });
}

#[cfg(test)]
mod tests {
    // rozdiel tam a späť: z „starého“ programu a rozdielu vznikne presne nový
    #[test]
    fn patch_roundtrip() {
        let dir = std::env::temp_dir().join(format!("flux-patch-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let old: Vec<u8> = (0..3_000_000u32).map(|i| (i.wrapping_mul(2654435761) >> 24) as u8).collect();
        let mut new = old.clone();
        for i in (1_000_000..1_050_000).step_by(37) {
            new[i] ^= 0x5a;
        }
        new.splice(2_000_000..2_000_000, b"inserted ".repeat(500));
        std::fs::write(dir.join("old"), &old).unwrap();
        std::fs::write(dir.join("new"), &new).unwrap();
        super::make_patch(&dir.join("old"), &dir.join("new"), &dir.join("p")).unwrap();
        let patch = std::fs::read(dir.join("p")).unwrap();
        assert!(patch.len() < 100_000, "patch {} B", patch.len());
        assert_eq!(super::apply_patch(&old, &patch).unwrap(), new);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
