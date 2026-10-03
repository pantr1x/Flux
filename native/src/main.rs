// Flux Native – Flux v čistom Ruste (egui), bez Chromia. Logika je v crates/flux-core.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

// počítadlo haldy (Nastavenia → Vývojár): koľko pamäte drží kód Fluxu, bez grafického ovládača a dekodérov
struct Count;
pub static HEAP: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
unsafe impl std::alloc::GlobalAlloc for Count {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        HEAP.fetch_add(l.size(), std::sync::atomic::Ordering::Relaxed);
        unsafe { std::alloc::System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        HEAP.fetch_sub(l.size(), std::sync::atomic::Ordering::Relaxed);
        unsafe { std::alloc::System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        let r = unsafe { std::alloc::System.realloc(p, l, n) };
        if !r.is_null() {
            HEAP.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
            HEAP.fetch_sub(l.size(), std::sync::atomic::Ordering::Relaxed);
        }
        r
    }
}
#[global_allocator]
static ALLOC: Count = Count;

pub fn heap_mb() -> f64 {
    HEAP.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1048576.0
}
mod boot;
mod code;
mod gen;
mod i18n;
mod live;
mod mcp;
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
        && (s["liveWallMode"].as_str() == Some("glass") || (s["liveWallMode"].as_str() == Some("see") && std::env::var("FLUX_SEE_THROUGH").as_deref() == Ok("1")))
        && !s["bg"]["type"].is_string()
        && s["liveCache"]["file"].is_string()
        && !boot::safe()
}

// „sklo“ (liveWallMode: glass) je pokus: po prepnutí sa Flux spýta, či ho vidno. Bez potvrdenia do 25 s
// (okno neviditeľné, zaseknuté, spadnuté) sa vráti na pohyblivú tapetu a spustí znova.
pub static GLASS_OK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn glass_revert() {
    let mut s = flux_core::settings::load();
    if let Some(o) = s.as_object_mut() {
        o.insert("liveWallMode".into(), serde_json::json!("play"));
        o.remove("glassTrial");
    }
    flux_core::settings::save(&s);
}

fn glass_trial() {
    let mut s = flux_core::settings::load();
    match s["glassTrial"].as_str() {
        // minulý pokus sa nepotvrdil (okno nebolo vidno) → späť
        Some("running") => {
            glass_revert();
            notice("The see-through glass did not work on this computer, so Flux went back to the moving wallpaper.");
        }
        Some("pending") if want_transparent() => {
            s["glassTrial"] = serde_json::json!("running");
            flux_core::settings::save(&s);
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(25));
                if !GLASS_OK.load(std::sync::atomic::Ordering::Relaxed) {
                    glass_revert();
                    // nový štart nie je „zlyhaný“ (bez núdzového režimu), len ukáže správu
                    let mut s = flux_core::settings::load();
                    s["glassFailed"] = serde_json::json!(true);
                    flux_core::settings::save(&s);
                    boot::closed();
                    if let Ok(e) = std::env::current_exe() {
                        let _ = std::process::Command::new(e).spawn();
                    }
                    std::process::exit(3);
                }
            });
        }
        _ => {}
    }
}

fn options(renderer: eframe::Renderer) -> eframe::NativeOptions {
    let transparent = want_transparent();
    eprintln!("[flux] renderer {renderer:?}, transparent {transparent}");
    TRANSPARENT.store(transparent, std::sync::atomic::Ordering::Relaxed);
    // FLUX_SIZE=1280x780 – veľkosť okna pre testy a porovnanie so screenshotmi Electron Fluxu
    let size = std::env::var("FLUX_SIZE").ok().and_then(|s| s.split_once('x').and_then(|(w, h)| Some([w.parse().ok()?, h.parse().ok()?]))).unwrap_or([1400.0, 900.0]);
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Flux")
            // vlastná titulná lišta ako v Electron Fluxe (− □ × kreslí app/chrome.rs)
            .with_decorations(false)
            // spustený mostom MCP (--background): nekradnúť fokus
            .with_active(!std::env::args().any(|a| a == "--background"))
            .with_transparent(transparent)
            .with_inner_size(size)
            .with_min_inner_size([760.0, 480.0])
            .with_icon(std::sync::Arc::new(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon64.png")).unwrap_or_default())),
        renderer,
        ..Default::default()
    }
}

// chyba pri štarte: záznam do userData/flux-native-crash.log a okno s hlásením (bez konzoly by inak nebolo nič vidieť)
// informačné okno (núdzový režim); mimo Windows len na stderr
pub fn notice(msg: &str) {
    eprintln!("[flux] {msg}");
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
        let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (text, title) = (w(msg), w("Flux"));
        unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION) };
    }
}

// varovania a chyby z eframe/winit/glutin na stderr (v CI presmerované do súboru)
struct Log;
impl log::Log for Log {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Warn
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            eprintln!("[{}] {}: {}", r.level(), r.target(), r.args());
        }
    }
    fn flush(&self) {}
}

pub fn report(msg: &str) {
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
    // most pre Claude Desktop: stdin/stdout ↔ MCP server bežiaceho Fluxu, bez okna
    // CI: rozdiel oproti predošlej zostave (update.rs)
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 5 && args[1] == "--make-patch" {
        if let Err(e) = update::make_patch(args[2].as_ref(), args[3].as_ref(), args[4].as_ref()) {
            eprintln!("make-patch: {e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mcp-bridge") {
        mcp::bridge();
        return Ok(());
    }
    // po aktualizácii: .new preč, .old zatiaľ ostane (návrat, ak by nová verzia hneď spadla)
    update::cleanup(true);
    let _ = log::set_logger(&Log).map(|_| log::set_max_level(log::LevelFilter::Warn));
    boot::catch_hard_crashes();
    boot::kill_ghosts();
    boot::begin();
    boot::watchdog();
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
    glass_trial();
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
    // núdzový režim po dvoch neúspešných štartoch: rovno wgpu
    let forced = std::env::var("FLUX_RENDERER").ok().or_else(|| (boot::FAILS.load(std::sync::atomic::Ordering::Relaxed) >= 2).then(|| "wgpu".to_string()));
    if forced.as_deref() != Some("wgpu") {
        match eframe::run_native("Flux", options(eframe::Renderer::Glow), Box::new(|cc| Ok(Box::new(app::App::new(cc))))) {
            Ok(()) => return Ok(()),
            Err(e) => eprintln!("OpenGL nefunguje ({e}), skúšam wgpu"),
        }
    }
    eframe::run_native("Flux", options(eframe::Renderer::Wgpu), Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
