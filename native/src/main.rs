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

fn main() -> eframe::Result {
    // po aktualizácii zmaže starý program
    update::cleanup();
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
