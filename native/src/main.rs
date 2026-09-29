// Flux Native – Flux v čistom Ruste (egui), bez Chromia. Logika je v crates/flux-core.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod term;
mod theme;
mod widgets;

fn options(renderer: eframe::Renderer) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_title("Flux").with_inner_size([1400.0, 900.0]).with_min_inner_size([760.0, 480.0]),
        renderer,
        ..Default::default()
    }
}

fn main() -> eframe::Result {
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
