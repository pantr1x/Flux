// Aktualizácie Flux Native z vetvy ci-native (native.yml tam dáva Flux-Native.exe a native.json):
// kontrola → stiahnutie vedľa programu (Flux-Native.new) → kontrola sha256 → výmena a nový štart.
// Spustený .exe sa na Windows nedá prepísať, ale dá sa premenovať – starý ostane ako .old a zmaže sa pri ďalšom štarte.
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
}

pub struct Updater {
    pub state: Arc<Mutex<State>>,
    info: Arc<Mutex<Option<Info>>>,
    pub last_check: f64,
    ctx: Option<egui::Context>, // pre opätovné stiahnutie z install()
}

fn base() -> String {
    std::env::var("FLUX_UPDATE_URL").unwrap_or_else(|_| "https://raw.githubusercontent.com/pantr1x/Flux/ci-native/".into())
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

// nová verzia spadla hneď po štarte a vedľa je predošlá (.old): vymeniť späť a spustiť ju
pub fn rollback() -> bool {
    let Some(e) = exe() else { return false };
    let old = e.with_extension("old");
    if !old.exists() {
        return false;
    }
    let bad = e.with_extension("bad");
    let _ = std::fs::remove_file(&bad);
    if std::fs::rename(&e, &bad).is_err() {
        return false;
    }
    if std::fs::rename(&old, &e).is_err() {
        let _ = std::fs::rename(&bad, &e);
        return false;
    }
    std::process::Command::new(&e).spawn().is_ok()
}

// vlastná zostava bez adresy na testy sa neaktualizuje (prepísala by sa verziou z CI)
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
        let old = e.with_extension("old");
        // .old môže ešte bežať (starý Flux, most MCP pre Claude) → vtedy sa nedá zmazať; program ide inam
        let old = if std::fs::remove_file(&old).is_ok() || !old.exists() {
            old
        } else {
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            e.with_extension(format!("old-{t}"))
        };
        std::fs::rename(&e, &old).map_err(|x| x.to_string())?;
        if let Err(x) = std::fs::rename(&new, &e) {
            let _ = std::fs::rename(&old, &e);
            return Err(x.to_string());
        }
        *self.state.lock().unwrap() = State::Idle;
        if relaunch {
            let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).filter(|a| a != "--background" && a != "--minimized").chain(extra.iter().map(|a| a.into())).collect();
            std::process::Command::new(&e).args(args).spawn().map_err(|x| x.to_string())?;
        }
        Ok(())
    }
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
    set(State::Downloading { got: 0, total: i.size });
    let child = curl().args(["-fsSL", "--max-time", "600", "-o"]).arg(&new).arg(format!("{}Flux-Native.exe", base())).spawn();
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
