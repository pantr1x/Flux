// Flux Native – Flux v čistom Ruste (egui), bez Chromia. Logika je v crates/flux-core.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod code;
mod gen;
mod i18n;
mod live;
mod mem;
mod smooth;
mod term;
mod theme;
mod update;
mod wall;
mod widgets;

fn options(renderer: eframe::Renderer) -> eframe::NativeOptions {
    // FLUX_SIZE=1280x780 – veľkosť okna pre testy a porovnanie so screenshotmi Electron Fluxu
    let size = std::env::var("FLUX_SIZE").ok().and_then(|s| s.split_once('x').and_then(|(w, h)| Some([w.parse().ok()?, h.parse().ok()?]))).unwrap_or([1400.0, 900.0]);
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Flux")
            // vlastná titulná lišta ako v Electron Fluxe (− □ × kreslí app/chrome.rs)
            .with_decorations(false)
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
