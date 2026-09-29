// Flux Native – Flux v čistom Ruste (egui), bez Chromia. Logika je v crates/flux-core.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod code;
mod gen;
mod i18n;
mod live;
mod mem;
mod net;
mod secret;
mod server;
mod smooth;
mod term;
mod theme;
mod update;
mod wall;
mod widgets;

// priehľadné okno pre živú tapetu (Lively / Wallpaper Engine): pracovná plocha presvitá priamo,
// Flux nič nedekóduje. Dá sa zvoliť len pri vytvorení okna, preto sa rozhodne zo settings.json vopred.
pub static TRANSPARENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn want_transparent() -> bool {
    let s = flux_core::settings::load();
    let on = |k: &str, def: bool| s[k].as_bool().unwrap_or(def);
    let lite = on("lite", false);
    s["material"].as_str() != Some("none")
        && on("optFx", !lite)
        && on("liveWallpaper", true)
        && s["liveWallMode"].as_str().unwrap_or("see") == "see"
        && !s["bg"]["type"].is_string()
        && s["liveCache"]["file"].is_string()
}

fn options(renderer: eframe::Renderer) -> eframe::NativeOptions {
    let transparent = want_transparent();
    TRANSPARENT.store(transparent, std::sync::atomic::Ordering::Relaxed);
    // FLUX_SIZE=1280x780 – veľkosť okna pre testy a porovnanie so screenshotmi Electron Fluxu
    let size = std::env::var("FLUX_SIZE").ok().and_then(|s| s.split_once('x').and_then(|(w, h)| Some([w.parse().ok()?, h.parse().ok()?]))).unwrap_or([1400.0, 900.0]);
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Flux")
            // vlastná titulná lišta ako v Electron Fluxe (− □ × kreslí app/chrome.rs)
            .with_decorations(false)
            .with_transparent(transparent)
            .with_inner_size(size)
            .with_min_inner_size([760.0, 480.0])
            .with_icon(std::sync::Arc::new(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon64.png")).unwrap_or_default())),
        renderer,
        ..Default::default()
    }
}

// chyba pri štarte: záznam do userData/flux-native-crash.log a okno s hlásením (bez konzoly by inak nebolo nič vidieť)
fn report(msg: &str) {
    let log = flux_core::settings::user_data().join("flux-native-crash.log");
    let line = format!("[{}] Flux Native {} ({}): {msg}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), env!("CARGO_PKG_VERSION"), update::SHA);
    let _ = std::fs::create_dir_all(log.parent().unwrap_or(std::path::Path::new(".")));
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&log).and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    eprintln!("{line}");
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let text = w(&format!("Flux Native could not start.\n\n{msg}\n\nDetails were saved to:\n{}", log.display()));
        let title = w("Flux");
        unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR) };
    }
}

fn main() -> eframe::Result {
    // po aktualizácii: .new preč, .old zatiaľ ostane (návrat, ak by nová verzia hneď spadla)
    update::cleanup(true);
    let started = std::time::Instant::now();
    std::panic::set_hook(Box::new(move |info| {
        let at = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        let what = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        // pád hneď po štarte novej verzie → späť na predošlú
        if started.elapsed() < std::time::Duration::from_secs(20) && update::rollback() {
            report(&format!("{what}{at}\n\nFlux went back to the previous version."));
            std::process::exit(1);
        }
        report(&format!("{what}{at}"));
    }));
    let r = run();
    if let Err(e) = &r {
        if !update::rollback() {
            report(&format!("The graphics could not start: {e}"));
        }
    }
    r
}

fn run() -> eframe::Result {
    // OpenGL (glow) je najúspornejšie; bez ovládača OpenGL (virtuálny stroj, server) záloha cez wgpu (DirectX/Vulkan).
    // FLUX_RENDERER=wgpu vynúti zálohu.
    let forced = std::env::var("FLUX_RENDERER").ok();
    if forced.as_deref() != Some("wgpu") {
        match eframe::run_native("Flux", options(eframe::Renderer::Glow), Box::new(|cc| Ok(Box::new(app::App::new(cc))))) {
            Ok(()) => return Ok(()),
            Err(e) => eprintln!("OpenGL nefunguje ({e}), skúšam wgpu"),
        }
    }
    eframe::run_native("Flux", options(eframe::Renderer::Wgpu), Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
