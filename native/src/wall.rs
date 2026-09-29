// Pozadie za oknom ako v Electron Fluxe (materiál „wallpaper“): tapeta plochy (alebo vlastný obrázok
// zo settings.bg) nakreslená tak, aby sedela s plochou, a cez ňu mierne priesvitné panely.
// Obrázok sa načíta na pozadí a zmenší na veľkosť obrazovky, v pamäti ostane len textúra.
use eframe::egui::{self, pos2, Color32, ColorImage, Rect, TextureHandle, TextureOptions};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Wall {
    tex: Option<TextureHandle>,
    pending: Arc<Mutex<Option<(ColorImage, f32)>>>,
    source: Option<PathBuf>,
    pub lum: Option<f32>, // priemerný jas tapety 0–1 (na prispôsobenie panelov)
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

impl Wall {
    // nastaví zdroj; ak sa zmenil, načíta ho na pozadí
    pub fn want(&mut self, ctx: &egui::Context, src: Option<PathBuf>) {
        if src == self.source {
            return;
        }
        self.source = src.clone();
        self.tex = None;
        self.lum = None;
        let Some(path) = src else { return };
        let slot = self.pending.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let Ok(img) = image::open(&path) else { return };
            // zmenšiť na najviac 1920 px na šírku – ostrosť na ploche stačí a textúra zaberie ~9 MB
            let img = if img.width() > 1920 { img.resize(1920, 1920 * img.height() / img.width().max(1), image::imageops::FilterType::Triangle) } else { img };
            let rgba = img.to_rgba8();
            drop(img);
            // priemerný jas (každý 16. bod stačí)
            let (mut sum, mut n) = (0f64, 0f64);
            for px in rgba.pixels().step_by(16) {
                sum += (0.299 * px[0] as f64 + 0.587 * px[1] as f64 + 0.114 * px[2] as f64) / 255.0;
                n += 1.0;
            }
            let size = [rgba.width() as usize, rgba.height() as usize];
            let ci = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            drop(rgba);
            *slot.lock().unwrap() = Some((ci, (sum / n.max(1.0)) as f32));
            ctx.request_repaint();
        });
    }

    // kreslí tapetu do pozadia okna, zarovnanú s plochou (výplň „cover“ cez celý monitor)
    // vráti true, keď sa práve načítala nová tapeta (treba prepočítať farby panelov)
    pub fn paint(&mut self, ctx: &egui::Context) -> bool {
        let mut fresh = false;
        if let Some((img, lum)) = self.pending.lock().unwrap().take() {
            self.tex = Some(ctx.load_texture("flux-wall", img, TextureOptions::LINEAR));
            self.lum = Some(lum);
            fresh = true;
        }
        let Some(tex) = &self.tex else { return fresh };
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
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.image(tex.id(), img_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        fresh
    }
}
