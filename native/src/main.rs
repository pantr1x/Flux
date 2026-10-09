// Flux Native – Flux v čistom Ruste (egui), bez Chromia. Logika je v crates/flux-core.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod pins;
mod migrate;
mod single;
mod plugins;
mod glass;
mod lint;
mod complete;
mod colors;
mod perf;

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
mod ws;
mod theme;
mod update;
mod wall;
mod widgets;

// priehľadné okno (material „see“ = čisté, „blur“ = rozmazané): za Fluxom vidno, čo tam je (plocha, Lively, Discord…),
// Flux nič nedekóduje ani nekreslí tapetu. Dá sa zvoliť len pri vytvorení okna, preto sa rozhodne zo settings.json vopred.
pub static TRANSPARENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn see_material(s: &serde_json::Value) -> bool {
    matches!(s["material"].as_str(), Some("see" | "blur"))
}

pub fn want_transparent() -> bool {
    let s = flux_core::settings::load();
    let on = |k: &str, def: bool| s[k].as_bool().unwrap_or(def);
    let lite = on("lite", false);
    see_material(&s) && on("optFx", !lite) && !boot::safe()
}

// 0.9.12: priehľadné okno je nové predvolené – raz prepne tapetu (bez vlastného obrázka) a staré „sklo“ živej tapety
fn see_migrate() {
    let mut s = flux_core::settings::load();
    let Some(o) = s.as_object_mut() else { return };
    // 0.9.13: bez otázky „Vidíš, čo je za Fluxom?“ – staré kľúče pokusu preč
    if o.remove("glassTrial").is_some() | o.remove("glassFailed").is_some() && o.get("seeMigrated").is_some() {
        flux_core::settings::save(&s);
        return;
    }
    if o.get("seeMigrated").and_then(|v| v.as_bool()) == Some(true) {
        return;
    }
    let mat = o.get("material").and_then(|v| v.as_str()).unwrap_or("auto").to_string();
    let own_bg = o.get("bg").map(|b| b["type"].is_string()).unwrap_or(false);
    let glass = o.get("liveWallMode").and_then(|v| v.as_str()) == Some("glass");
    if glass || (matches!(mat.as_str(), "auto" | "wallpaper") && !own_bg) {
        o.insert("material".into(), serde_json::json!("see"));
        if glass {
            o.insert("liveWallMode".into(), serde_json::json!("play"));
        }
    }
    o.insert("seeMigrated".into(), serde_json::json!(true));
    flux_core::settings::save(&s);
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
            .with_app_id("flux")
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
    // flux --version: ktorá zostava beží (kontrola po inštalácii/aktualizácii)
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("Flux Native {} ({})", env!("CARGO_PKG_VERSION"), &update::SHA[..update::SHA.len().min(7)]);
        return Ok(());
    }
    if args.len() == 5 && args[1] == "--make-patch" {
        if let Err(e) = update::make_patch(args[2].as_ref(), args[3].as_ref(), args[4].as_ref()) {
            eprintln!("make-patch: {e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    // aktualizácia: tento program (.new) prepíše Flux-Native.exe po jeho skončení (update.rs)
    if args.len() >= 4 && args[1] == "--apply-update" {
        update::apply_update(&args);
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mcp-bridge") {
        mcp::bridge();
        return Ok(());
    }
    // po aktualizácii: .new preč, .old zatiaľ ostane (návrat, ak by nová verzia hneď spadla)
    update::cleanup(true);
    if args.iter().any(|a| a == "--updated") {
        update::remove_helper();
    }
    // prechod zo starého Fluxu (Electron): odkazy, pripnutie a typy súborov → sem (migrate.rs)
    if args.iter().any(|a| a == "--migrate") {
        migrate::run(&args);
        if args.iter().any(|a| a == "--no-window") {
            return Ok(());
        }
    }
    // Flux beží len raz: súbor z Prieskumníka (alebo ďalšie spustenie) prevezme bežiace okno.
    // Proces bez viditeľného okna („duch“) spustenie neprevezme (single::forward) – iné procesy Flux nikdy neukončuje.
    if !args.iter().any(|a| a == "--background") && single::forward(single::path_arg(&args).as_deref()) {
        return Ok(());
    }
    remember_path();
    let _ = log::set_logger(&Log).map(|_| log::set_max_level(log::LevelFilter::Warn));
    boot::catch_hard_crashes();
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
    perf::init();
    see_migrate();
    let r = run();
    if let Err(e) = &r {
        if !update::rollback() {
            report(&format!("The graphics could not start: {e}"));
        }
    }
    r
}

// cesta k tomuto programu v nastaveniach – starý Flux (Electron) podľa nej prejde na túto kópiu namiesto sťahovania
fn remember_path() {
    let Ok(exe) = std::env::current_exe() else { return };
    let p = exe.to_string_lossy().to_string();
    let mut s = flux_core::settings::load();
    if s["nativePath"].as_str() != Some(p.as_str()) {
        s["nativePath"] = serde_json::json!(p);
        flux_core::settings::save(&s);
    }
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
