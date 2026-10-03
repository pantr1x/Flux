// Pozadie za oknom ako v Electron Fluxe (materiál „wallpaper“): tapeta plochy, vlastný obrázok
// zo settings.bg alebo živá tapeta (Lively / Wallpaper Engine, aj video) nakreslená tak, aby sedela
// s plochou, a cez ňu mierne priesvitné panely. Obrázok sa načíta na pozadí, v pamäti ostane len textúra.
use eframe::egui::{self, pos2, Color32, ColorImage, Rect, TextureHandle, TextureOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// čo kresliť: obrázok, alebo video (s náhľadom, keď sa nedá prehrať)
#[derive(Clone, Debug, PartialEq)]
pub enum Src {
    Image(PathBuf),
    Video(PathBuf, Option<PathBuf>),
    Still(PathBuf, Option<PathBuf>), // z videa len jedna snímka – dekodér sa hneď uvoľní (málo pamäte)
}

// snímka pre UI: obrázok, rozmazaná kópia a jas (pri videu len občas)
struct Loaded {
    img: ColorImage,
    blur: Option<ColorImage>,
    lum: Option<f32>,
}

#[derive(Default)]
pub struct Wall {
    tex: Option<TextureHandle>,
    blur: Option<TextureHandle>, // rozmazaná kópia (1/8) pod panelmi – ako backdrop-filter
    pending: Arc<Mutex<Option<Loaded>>>,
    img_rect: Option<Rect>,
    source: Option<Src>,
    play: Arc<AtomicBool>, // video hrá len pri zameranom okne a zapnutých animáciách
    stop: Arc<AtomicBool>,
    pub lum: Option<f32>, // priemerný jas tapety 0–1 (na prispôsobenie panelov)
    // video v pozadí: po 3 s pauzy sa dekodér aj grafické zariadenie uvoľnia (released), posledná snímka
    // ostane; pri návrate sa video spustí znova od uloženého miesta (resume, 100 ns)
    resume: Arc<std::sync::atomic::AtomicI64>,
    released: Arc<AtomicBool>,
}

// tapeta Windows (SystemParametersInfo), na testy FLUX_WALLPAPER
pub fn desktop_wallpaper() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FLUX_WALLPAPER") {
        return Some(PathBuf::from(p));
    }
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETDESKWALLPAPER};
        let mut buf = [0u16; 520];
        if SystemParametersInfoW(SPI_GETDESKWALLPAPER, buf.len() as u32, buf.as_mut_ptr() as *mut _, 0) != 0 {
            let len = buf.iter().position(|c| *c == 0).unwrap_or(0);
            if len > 0 {
                return Some(PathBuf::from(String::from_utf16_lossy(&buf[..len])));
            }
        }
    }
    None
}

fn luminance(rgba: &[u8]) -> f32 {
    let (mut sum, mut n) = (0f64, 0f64);
    for px in rgba.chunks_exact(4).step_by(16) {
        sum += (0.299 * px[0] as f64 + 0.587 * px[1] as f64 + 0.114 * px[2] as f64) / 255.0;
        n += 1.0;
    }
    (sum / n.max(1.0)) as f32
}

fn load_image(path: &std::path::Path) -> Option<Loaded> {
    let img = image::open(path).ok()?;
    // zmenšiť na najviac 1920 px na šírku – ostrosť na ploche stačí a textúra zaberie ~9 MB
    let img = if img.width() > 1920 { img.resize(1920, 1920 * img.height() / img.width().max(1), image::imageops::FilterType::Triangle) } else { img };
    let rgba = img.to_rgba8();
    drop(img);
    let lum = luminance(rgba.as_raw());
    let size = [rgba.width() as usize, rgba.height() as usize];
    let blur = blurred(&rgba);
    let img = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(Loaded { img, blur: Some(blur), lum: Some(lum) })
}

impl Wall {
    // vývojár: čo tapeta práve drží
    pub fn describe(&self) -> String {
        match &self.source {
            None => String::new(),
            Some(Src::Image(_)) => "wallpaper image".into(),
            Some(Src::Still(..)) => "live wallpaper (still frame)".into(),
            Some(Src::Video(..)) => "live wallpaper video".into(),
        }
    }

    // video hrá len keď je okno zamerané a animácie zapnuté (syncVideo v Electron Fluxe)
    pub fn set_playing(&mut self, ctx: &egui::Context, on: bool) {
        self.play.store(on, Ordering::Relaxed);
        if on && self.released.swap(false, Ordering::Relaxed) {
            if let Some(Src::Video(path, _)) = self.source.clone() {
                let (slot, play, stop, ctx) = (self.pending.clone(), self.play.clone(), self.stop.clone(), ctx.clone());
                let (resume, released) = (self.resume.clone(), self.released.clone());
                std::thread::spawn(move || {
                    video::play(&path, &slot, &play, &stop, &ctx, false, &resume, &released, true);
                });
            }
        }
    }

    // nastaví zdroj; ak sa zmenil, načíta ho na pozadí
    pub fn want(&mut self, ctx: &egui::Context, src: Option<Src>) {
        if src == self.source {
            return;
        }
        self.source = src.clone();
        self.stop.store(true, Ordering::Relaxed);
        self.stop = Arc::new(AtomicBool::new(false));
        self.tex = None;
        self.blur = None;
        self.lum = None;
        let Some(src) = src else { return };
        // nový slot – oneskorená snímka starého zdroja sa už nezobrazí
        self.pending = Arc::new(Mutex::new(None));
        self.resume = Default::default();
        self.released = Default::default();
        let slot = self.pending.clone();
        let (resume, released) = (self.resume.clone(), self.released.clone());
        let ctx = ctx.clone();
        let (play, stop) = (self.play.clone(), self.stop.clone());
        // jedna snímka alebo celé video
        enum Kind {
            Still,
            Other,
        }
        let src_kind = if matches!(src, Src::Still(..)) { Kind::Still } else { Kind::Other };
        std::thread::spawn(move || match src {
            Src::Image(path) => {
                if let Some(l) = load_image(&path) {
                    *slot.lock().unwrap() = Some(l);
                    ctx.request_repaint();
                }
            }
            Src::Video(path, preview) | Src::Still(path, preview) => {
                let once = matches!(src_kind, Kind::Still);
                if !video::play(&path, &slot, &play, &stop, &ctx, once, &resume, &released, false) {
                    // video sa nedá prehrať (iný systém, chýba kodek) → náhľad
                    if let Some(l) = preview.as_deref().and_then(load_image) {
                        *slot.lock().unwrap() = Some(l);
                        ctx.request_repaint();
                    }
                }
            }
        });
    }

    // kreslí tapetu do pozadia okna, zarovnanú s plochou (výplň „cover“ cez celý monitor)
    // vráti true, keď sa práve zmenil jas tapety (treba prepočítať farby panelov)
    pub fn paint(&mut self, ctx: &egui::Context) -> bool {
        let mut fresh = false;
        if let Some(l) = self.pending.lock().unwrap().take() {
            match &mut self.tex {
                // rovnaká veľkosť (snímky videa) → len prepísať obsah, bez novej textúry v ovládači
                Some(t) if t.size() == l.img.size => t.set_partial([0, 0], l.img, TextureOptions::LINEAR),
                _ => self.tex = Some(ctx.load_texture("flux-wall", l.img, TextureOptions::LINEAR)),
            }
            if let Some(b) = l.blur {
                match &mut self.blur {
                    Some(t) if t.size() == b.size => t.set(b, TextureOptions::LINEAR),
                    _ => self.blur = Some(ctx.load_texture("flux-wall-blur", b, TextureOptions::LINEAR)),
                }
            }
            if let Some(lum) = l.lum {
                if self.lum.map(|o| (o - lum).abs() > 0.03).unwrap_or(true) {
                    self.lum = Some(lum);
                    fresh = true;
                }
            }
        }
        let Some(tex) = &self.tex else {
            self.img_rect = None;
            return fresh;
        };
        let screen = ctx.content_rect();
        let (inner, monitor) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().monitor_size));
        let [iw, ih] = tex.size();
        let (iw, ih) = (iw as f32, ih as f32);
        // oblasť monitora v súradniciach okna; bez údajov o polohe pokryje len okno
        let area = match (inner, monitor) {
            (Some(inner), Some(m)) if m.x > 0.0 && m.y > 0.0 => Rect::from_min_size(pos2(-inner.min.x, -inner.min.y), m),
            _ => screen,
        };
        let k = (area.width() / iw).max(area.height() / ih);
        let img_rect = Rect::from_center_size(area.center(), egui::vec2(iw * k, ih * k));
        self.img_rect = Some(img_rect);
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.image(tex.id(), img_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        fresh
    }
}

impl Wall {
    // rozmazaná tapeta pod panelom (rect so zaoblením), zarovnaná s ostrou – panel nad ňou je pokojnejší
    pub fn paint_blurred(&self, painter: &egui::Painter, r: Rect, radius: u8) {
        let (Some(tex), Some(img)) = (&self.blur, self.img_rect) else { return };
        let uv = Rect::from_min_max(pos2((r.min.x - img.min.x) / img.width(), (r.min.y - img.min.y) / img.height()), pos2((r.max.x - img.min.x) / img.width(), (r.max.y - img.min.y) / img.height()));
        painter.add(egui::epaint::RectShape::filled(r, egui::CornerRadius::same(radius), Color32::WHITE).with_texture(tex.id(), uv));
    }
}

// 1/8 veľkosti, 3× box blur (≈ Gaussov blur ~28 px) a saturate(1.35) ako .wall-media v Electron Fluxe
fn blurred(src: &image::RgbaImage) -> ColorImage {
    let (w, h) = ((src.width() / 8).max(1), (src.height() / 8).max(1));
    let small = image::imageops::resize(src, w, h, image::imageops::FilterType::Triangle);
    let (w, h) = (w as usize, h as usize);
    let mut px: Vec<[f32; 3]> = small.pixels().map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]).collect();
    let r = 3i64;
    let mut tmp = px.clone();
    for _ in 0..3 {
        for (horiz, (len, lines)) in [(true, (w, h)), (false, (h, w))] {
            for line in 0..lines {
                let at = |i: usize| if horiz { line * w + i } else { i * w + line };
                let mut acc = [0f32; 3];
                let get = |i: i64| px[at(i.clamp(0, len as i64 - 1) as usize)];
                for i in -r..=r {
                    let c = get(i);
                    acc = [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]];
                }
                for i in 0..len {
                    let n = (2 * r + 1) as f32;
                    tmp[at(i)] = [acc[0] / n, acc[1] / n, acc[2] / n];
                    let (a, b) = (get(i as i64 + r + 1), get(i as i64 - r));
                    acc = [acc[0] + a[0] - b[0], acc[1] + a[1] - b[1], acc[2] + a[2] - b[2]];
                }
            }
            std::mem::swap(&mut px, &mut tmp);
        }
    }
    let rgba: Vec<u8> = px
        .iter()
        .flat_map(|c| {
            let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let s = |v: f32| (l + (v - l) * 1.35).clamp(0.0, 255.0) as u8;
            [s(c[0]), s(c[1]), s(c[2]), 255]
        })
        .collect();
    ColorImage::from_rgba_unmultiplied([w, h], &rgba)
}

// Video tapeta cez Windows Media Foundation: IMFSourceReader dekóduje (aj hardvérovo), zmenší na
// ≤ 960 px a dá RGB32; vlákno posiela snímky (≤ 24 fps) do UI, ktoré prepíše jednu textúru.
mod video {
    use super::Loaded;
    use eframe::egui;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, AtomicI64};
    use std::sync::{Arc, Mutex};

    // vráti false, keď sa video nedá otvoriť ani raz (vtedy sa ukáže náhľad)
    #[cfg(not(windows))]
    #[allow(clippy::too_many_arguments)]
    pub fn play(_: &Path, _: &Arc<Mutex<Option<Loaded>>>, _: &AtomicBool, _: &AtomicBool, _: &egui::Context, _: bool, _: &AtomicI64, _: &AtomicBool, _: bool) -> bool {
        false
    }

    #[cfg(windows)]
    #[allow(clippy::too_many_arguments)]
    pub fn play(
        path: &Path,
        slot: &Arc<Mutex<Option<Loaded>>>,
        play: &AtomicBool,
        stop: &AtomicBool,
        ctx: &egui::Context,
        once: bool,
        resume: &AtomicI64,
        released: &AtomicBool,
        resumed: bool,
    ) -> bool {
        // resumed: snímka už je na obrazovke (pokračovanie po uvoľnení v pozadí)
        let mut shown = resumed;
        let r = unsafe { run(path, slot, play, stop, ctx, &mut shown, once, resume, released) };
        if let Err(e) = r {
            eprintln!("video tapeta: {e}");
        }
        shown
    }

    // D3D11 zariadenie + správca pre Media Foundation; None = zostane dekódovanie v procesore
    #[cfg(windows)]
    unsafe fn hw_decoder(
        attrs: &windows::Win32::Media::MediaFoundation::IMFAttributes,
    ) -> Option<(windows::Win32::Graphics::Direct3D11::ID3D11Device, windows::Win32::Media::MediaFoundation::IMFDXGIDeviceManager)> {
        use windows::core::Interface;
        use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
        use windows::Win32::Graphics::Direct3D10::ID3D10Multithread;
        use windows::Win32::Graphics::Direct3D11::*;
        use windows::Win32::Media::MediaFoundation::*;
        let mut dev = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            Default::default(),
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT | D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut dev),
            None,
            None,
        )
        .ok()?;
        let dev = dev?;
        if let Ok(mt) = dev.cast::<ID3D10Multithread>() {
            let _ = mt.SetMultithreadProtected(true);
        }
        let (mut token, mut mgr) = (0u32, None);
        MFCreateDXGIDeviceManager(&mut token, &mut mgr).ok()?;
        let mgr = mgr?;
        mgr.ResetDevice(&dev, token).ok()?;
        attrs.SetUnknown(&MF_SOURCE_READER_D3D_MANAGER, &mgr).ok()?;
        Some((dev, mgr))
    }

    #[cfg(windows)]
    #[allow(clippy::too_many_arguments)]
    unsafe fn run(
        path: &Path,
        slot: &Arc<Mutex<Option<Loaded>>>,
        play: &AtomicBool,
        stop: &AtomicBool,
        ctx: &egui::Context,
        shown: &mut bool,
        once: bool,
        resume: &AtomicI64,
        released: &AtomicBool,
    ) -> windows::core::Result<()> {
        use super::{blurred, luminance};
        use egui::ColorImage;
        use std::sync::atomic::Ordering;
        use std::time::{Duration, Instant};
        use windows::core::{GUID, HSTRING};
        use windows::Win32::Media::MediaFoundation::*;
        use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        use windows::Win32::System::Variant::VT_I8;
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        MFStartup(MF_VERSION, MFSTARTUP_LITE)?;
        // MFShutdown pri každom konci (aj pri chybe) – inak by Media Foundation ostala v pamäti
        struct Mf;
        impl Drop for Mf {
            fn drop(&mut self) {
                unsafe {
                    let _ = MFShutdown();
                }
            }
        }
        let _mf = Mf;
        let mut attrs = None;
        MFCreateAttributes(&mut attrs, 2)?;
        let attrs = attrs.ok_or_else(windows::core::Error::empty)?;
        attrs.SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1)?;
        attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)?;
        // dekódovanie na grafickej karte (D3D11): bez neho sa 4K video dekóduje v procesore – pomaly
        // (tapeta sa oneskoruje) a so stovkami MB snímok v RAM
        let _d3d = hw_decoder(&attrs);
        let reader = MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attrs)?;
        let vs = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
        let _ = reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false);
        reader.SetStreamSelection(vs, true)?;
        let native = reader.GetNativeMediaType(vs, 0)?;
        let fs = native.GetUINT64(&MF_MT_FRAME_SIZE)?;
        let (nw, nh) = ((fs >> 32) as u32, (fs & 0xffff_ffff) as u32);
        let w = nw.min(960) & !1;
        let h = ((nh as u64 * w as u64 / nw.max(1) as u64) as u32) & !1;
        let mt = MFCreateMediaType()?;
        mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
        mt.SetUINT64(&MF_MT_FRAME_SIZE, ((w as u64) << 32) | h as u64)?;
        if reader.SetCurrentMediaType(vs, None, &mt).is_err() {
            // bez zmenšenia (niektoré dekodéry to nevedia)
            let _ = mt.DeleteItem(&MF_MT_FRAME_SIZE);
            reader.SetCurrentMediaType(vs, None, &mt)?;
        }
        let cur = reader.GetCurrentMediaType(vs)?;
        let fs = cur.GetUINT64(&MF_MT_FRAME_SIZE)?;
        let (w, h) = ((fs >> 32) as usize, (fs & 0xffff_ffff) as usize);
        let stride = cur.GetUINT32(&MF_MT_DEFAULT_STRIDE).map(|s| s as i32).unwrap_or((w * 4) as i32);
        let mut clock = Instant::now();
        let mut first_ts: Option<i64> = None;
        let mut last_shown = Instant::now() - Duration::from_secs(1);
        let mut last_blur = Instant::now() - Duration::from_secs(10);
        let mut loops = 0;
        // pokračovanie po uvoľnení: od uloženého miesta
        let start = resume.load(Ordering::Relaxed);
        if start > 0 {
            let mut pv = PROPVARIANT::default();
            (*pv.Anonymous.Anonymous).vt = VT_I8;
            (*pv.Anonymous.Anonymous).Anonymous.hVal = start;
            let _ = reader.SetCurrentPosition(&GUID::zeroed(), &pv);
        }
        let mut last_ts = start;
        let mut paused: Option<Instant> = None;
        while !stop.load(Ordering::Relaxed) {
            // pauza: prvá snímka už je na obrazovke, ďalšie až keď okno zas hrá
            if *shown && !play.load(Ordering::Relaxed) {
                // dlhšie v pozadí → uvoľniť dekodér a D3D (Flux v pozadí zaberá oveľa menej); posledná snímka ostáva
                if paused.get_or_insert_with(Instant::now).elapsed() > Duration::from_secs(3) {
                    resume.store(last_ts, Ordering::Relaxed);
                    released.store(true, Ordering::Relaxed);
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(150));
                first_ts = None;
                continue;
            }
            paused = None;
            let (mut flags, mut ts, mut sample) = (0u32, 0i64, None);
            reader.ReadSample(vs, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut sample))?;
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                loops += 1;
                if loops > 1 && !*shown {
                    return Ok(());
                }
                // znova od začiatku
                let mut pv = PROPVARIANT::default();
                (*pv.Anonymous.Anonymous).vt = VT_I8;
                (*pv.Anonymous.Anonymous).Anonymous.hVal = 0;
                reader.SetCurrentPosition(&GUID::zeroed(), &pv)?;
                first_ts = None;
                continue;
            }
            let Some(sample) = sample else { continue };
            last_ts = ts;
            // tempo podľa časových značiek videa (100 ns)
            match first_ts {
                None => {
                    first_ts = Some(ts);
                    clock = Instant::now();
                }
                Some(t0) => {
                    let due = Duration::from_nanos(((ts - t0).max(0) as u64) * 100);
                    let el = clock.elapsed();
                    if due > el {
                        std::thread::sleep(due - el);
                    } else if el > due + Duration::from_millis(800) {
                        // dekódovanie nestíha: skok na aktuálny čas, nech video nebeží spomalene
                        let mut pv = PROPVARIANT::default();
                        (*pv.Anonymous.Anonymous).vt = VT_I8;
                        (*pv.Anonymous.Anonymous).Anonymous.hVal = t0 + (el.as_nanos() / 100) as i64 + 2_000_000;
                        let _ = reader.SetCurrentPosition(&GUID::zeroed(), &pv);
                        continue;
                    } else if *shown && el > due + Duration::from_millis(60) {
                        continue; // oneskorená snímka – preskočiť bez kopírovania
                    }
                }
            }
            if *shown && last_shown.elapsed() < Duration::from_millis(41) {
                continue; // najviac ~24 snímok za sekundu
            }
            let buf = sample.ConvertToContiguousBuffer()?;
            let (mut ptr, mut len) = (std::ptr::null_mut(), 0u32);
            buf.Lock(&mut ptr, None, Some(&mut len))?;
            let data = std::slice::from_raw_parts(ptr, len as usize);
            // riadok: podľa typu média, pri snímkach z GPU podľa dĺžky bufferu
            let row = if stride.unsigned_abs() as usize >= w * 4 && stride.unsigned_abs() as usize * h <= len as usize { stride.unsigned_abs() as usize } else { (len as usize / h.max(1)).max(w * 4) };
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..h {
                let sy = if stride < 0 { h - 1 - y } else { y };
                let Some(src) = data.get(sy * row..sy * row + w * 4) else { break };
                for (d, s) in rgba[y * w * 4..(y + 1) * w * 4].chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                    d.copy_from_slice(&[s[2], s[1], s[0], 255]);
                }
            }
            let _ = buf.Unlock();
            // rozmazaná kópia a jas len raz za ~2 s (stačí na pokojné pozadie panelov)
            let (blur, lum) = if last_blur.elapsed() > Duration::from_secs(2) {
                last_blur = Instant::now();
                let img = image::RgbaImage::from_raw(w as u32, h as u32, rgba.clone());
                (img.map(|i| blurred(&i)), Some(luminance(&rgba)))
            } else {
                (None, None)
            };
            *slot.lock().unwrap() = Some(Loaded { img: ColorImage::from_rgba_unmultiplied([w, h], &rgba), blur, lum });
            ctx.request_repaint();
            *shown = true;
            last_shown = Instant::now();
            // jedna snímka stačí → koniec: čítačka, dekodér aj D3D sa uvoľnia
            if once {
                return Ok(());
            }
        }
        Ok(())
    }
}
