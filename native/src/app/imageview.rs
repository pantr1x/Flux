// Obrázky ako súbory: karta s náhľadom (prispôsobiť / 100 %, Ctrl+koliesko, ťahanie) a veľkosťou.
// Rovnaký zdroj (textúra z image) používa aj náhľad pri prejdení myšou nad cestou v kóde.
use super::App;
use crate::i18n::{t, tf};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "svg"];

pub fn is_image(path: &str) -> bool {
    let e = Path::new(path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    IMAGE_EXT.contains(&e.as_str())
}

enum Slot {
    Loading,
    Decoded(egui::ColorImage),
    Ready(egui::TextureHandle),
    Svg(Arc<[u8]>),
    Failed(String),
}

pub struct Img {
    slot: Arc<Mutex<Slot>>,
    mtime: Option<SystemTime>,
    pub size: [usize; 2],
    used: std::time::Instant,
}

#[derive(Default)]
pub struct Images {
    map: HashMap<String, Img>,
    view: HashMap<String, (Option<f32>, egui::Vec2)>, // cesta → (zväčšenie, None = prispôsobiť; posun)
}

impl Images {
    // obrázok (načíta sa na pozadí; pri zmene súboru znova)
    fn get(&mut self, path: &str) -> &mut Img {
        let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        let stale = self.map.get(path).is_some_and(|i| i.mtime != mtime);
        if stale {
            self.map.remove(path);
        }
        if self.map.len() > 8 && !self.map.contains_key(path) {
            // najstarší preč (šetrenie pamäte)
            if let Some(old) = self.map.iter().min_by_key(|(_, v)| v.used).map(|(k, _)| k.clone()) {
                self.map.remove(&old);
            }
        }
        let img = self.map.entry(path.to_string()).or_insert_with(|| {
            let slot = Arc::new(Mutex::new(Slot::Loading));
            let (s2, p) = (slot.clone(), path.to_string());
            std::thread::spawn(move || {
                let r = if p.to_lowercase().ends_with(".svg") {
                    std::fs::read(&p).map(|b| Slot::Svg(b.into())).unwrap_or_else(|e| Slot::Failed(e.to_string()))
                } else {
                    match image::open(&p) {
                        Ok(im) => {
                            // veľké obrázky zmenšiť na 4096 px (textúra GPU)
                            let im = if im.width().max(im.height()) > 4096 { im.thumbnail(4096, 4096) } else { im };
                            let rgba = im.to_rgba8();
                            Slot::Decoded(egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], &rgba))
                        }
                        Err(e) => Slot::Failed(e.to_string()),
                    }
                };
                *s2.lock().unwrap() = r;
            });
            Img { slot, mtime, size: [0, 0], used: std::time::Instant::now() }
        });
        img.used = std::time::Instant::now();
        img
    }

    // nakreslí obrázok do obdĺžnika (zachová pomer); vráti veľkosť obrázka alebo chybu
    pub fn paint(&mut self, ui: &egui::Ui, path: &str, target: Rect, fit: bool) -> Result<Option<[usize; 2]>, String> {
        let img = self.get(path);
        let mut slot = img.slot.lock().unwrap();
        if let Slot::Decoded(ci) = &*slot {
            let size = ci.size;
            let tex = ui.ctx().load_texture(format!("img:{path}"), ci.clone(), egui::TextureOptions::LINEAR);
            img.size = size;
            *slot = Slot::Ready(tex);
        }
        match &*slot {
            Slot::Loading | Slot::Decoded(_) => {
                ui.ctx().request_repaint_after(Duration::from_millis(80));
                Ok(None)
            }
            Slot::Failed(e) => Err(e.clone()),
            Slot::Ready(tex) => {
                let s = tex.size_vec2();
                let r = if fit { fit_rect(s, target) } else { target };
                ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                Ok(Some(img.size))
            }
            Slot::Svg(b) => {
                let im = egui::Image::from_bytes(format!("bytes://svgfile/{path}"), egui::load::Bytes::Shared(b.clone()));
                let s = im.load_and_calc_size(ui, vec2(512.0, 512.0)).unwrap_or(vec2(256.0, 256.0));
                img.size = [s.x as usize, s.y as usize];
                let r = if fit { fit_rect(s, target) } else { target };
                im.paint_at(ui, r);
                Ok(Some(img.size))
            }
        }
    }

    pub fn size_of(&self, path: &str) -> Option<[usize; 2]> {
        self.map.get(path).map(|i| i.size).filter(|s| s[0] > 0)
    }

    pub fn forget(&mut self, path: &str) {
        self.map.remove(path);
        self.view.remove(path);
    }
}

// obdĺžnik s pomerom strán obrázka vnútri cieľa (nikdy nezväčší nad 100 %)
fn fit_rect(s: egui::Vec2, target: Rect) -> Rect {
    if s.x <= 0.0 || s.y <= 0.0 {
        return target;
    }
    let k = (target.width() / s.x).min(target.height() / s.y).min(1.0);
    Rect::from_center_size(target.center(), s * k)
}

fn human(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1048576.0)
    } else {
        format!("{} KB", (bytes as f64 / 1024.0).ceil() as u64)
    }
}

impl App {
    // karta s obrázkom namiesto editora
    pub(super) fn image_view(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = self.pal;
        let path = self.tabs[self.active].path.clone();
        let bar_h = 40.0;
        let area = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - bar_h));
        // šachovnica (priehľadnosť)
        let painter = ui.painter_at(area);
        let cell = 12.0;
        let (c1, c2) = if p.dark { (Color32::from_gray(38), Color32::from_gray(46)) } else { (Color32::from_gray(236), Color32::from_gray(250)) };
        painter.rect_filled(area, 0.0, c1);
        let mut y = area.top();
        let mut row = 0;
        while y < area.bottom() {
            let mut x = area.left() + if row % 2 == 0 { 0.0 } else { cell };
            while x < area.right() {
                painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(cell, cell)), 0.0, c2);
                x += cell * 2.0;
            }
            y += cell;
            row += 1;
        }
        let resp = ui.interact(area, ui.id().with(("imgview", &path)), Sense::click_and_drag());
        let (mut zoom, mut pan) = self.imgs.view.get(&path).copied().unwrap_or((None, egui::Vec2::ZERO));
        let size = self.imgs.size_of(&path);
        let fit_k = size.map(|s| (area.shrink(24.0).width() / s[0] as f32).min(area.shrink(24.0).height() / s[1] as f32).min(1.0)).unwrap_or(1.0);
        // Ctrl+koliesko (alebo koliesko bez Ctrl) = zväčšenie okolo kurzora; ťahanie = posun; dvojklik = prispôsobiť ↔ 100 %
        if resp.hovered() {
            let dy: f32 = ui.input(|i| {
                i.raw
                    .events
                    .iter()
                    .map(|e| match e {
                        egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta, .. } => delta.y,
                        egui::Event::MouseWheel { delta, .. } => delta.y * 40.0,
                        _ => 0.0,
                    })
                    .sum()
            });
            let zd = ui.input(|i| i.zoom_delta());
            if zd != 1.0 || dy.abs() > 0.1 {
                let k = zoom.unwrap_or(fit_k);
                let f = if zd != 1.0 { zd } else { (dy * 0.0025).exp() };
                let nk = (k * f).clamp(0.05, 16.0);
                if let (Some(pos), Some(s)) = (resp.hover_pos(), size) {
                    let c = area.center() + pan;
                    let rel = (pos - c) / (s[0] as f32 * k).max(1.0);
                    pan += (pos - c) - rel * (s[0] as f32 * nk);
                }
                zoom = Some(nk);
            }
        }
        if resp.dragged() {
            pan += resp.drag_delta();
            if zoom.is_none() {
                zoom = Some(fit_k);
            }
        }
        if resp.double_clicked() {
            if zoom.is_some() {
                zoom = None;
            } else {
                zoom = Some(1.0);
            }
            pan = egui::Vec2::ZERO;
        }
        if zoom.is_none() {
            pan = egui::Vec2::ZERO;
        }
        let k = zoom.unwrap_or(fit_k);
        let target = match size {
            Some(s) => Rect::from_center_size(area.center() + pan, vec2(s[0] as f32 * k, s[1] as f32 * k)),
            None => area.shrink(24.0),
        };
        let mut clip = ui.new_child(egui::UiBuilder::new().max_rect(area));
        clip.set_clip_rect(area);
        match self.imgs.paint(&clip, &path, target, size.is_none()) {
            Ok(None) => {
                ui.painter().text(area.center(), Align2::CENTER_CENTER, t("Loading…"), theme::ui(13.0), p.text3);
            }
            Err(e) => {
                ui.painter().text(area.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, t("This image cannot be shown."), theme::bold(14.0), p.text);
                ui.painter().text(area.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, e, theme::ui(12.0), p.text3);
            }
            Ok(Some(_)) => {}
        }
        self.imgs.view.insert(path.clone(), (zoom, pan));
        // spodný pás: rozmery · veľkosť · typ, zväčšenie, tlačidlá
        let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - bar_h), rect.max);
        ui.painter().hline(bar.x_range(), bar.top(), Stroke::new(1.0, p.line));
        let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let ext = Path::new(&path).extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default();
        let info = match self.imgs.size_of(&path) {
            Some(s) => format!("{} × {}  ·  {}  ·  {}", s[0], s[1], human(bytes), ext),
            None => format!("{}  ·  {}", human(bytes), ext),
        };
        ui.painter().text(pos2(bar.left() + 16.0, bar.center().y), Align2::LEFT_CENTER, info, theme::ui(12.5), p.text2);
        let mut b = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(12.0, 5.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
        b.spacing_mut().item_spacing.x = 6.0;
        if widgets::button(&mut b, Some("folder"), &t("Show in folder"), p.card2, p.text, 28.0, &p).clicked() {
            if let Some(d) = Path::new(&path).parent() {
                flux_core::settings::open_external(&d.to_string_lossy());
            }
        }
        if widgets::button(&mut b, None, "100 %", if zoom == Some(1.0) { p.active } else { p.card2 }, p.text, 28.0, &p).clicked() {
            self.imgs.view.insert(path.clone(), (Some(1.0), egui::Vec2::ZERO));
        }
        if widgets::button(&mut b, None, &t("Fit"), if zoom.is_none() { p.active } else { p.card2 }, p.text, 28.0, &p).clicked() {
            self.imgs.view.insert(path.clone(), (None, egui::Vec2::ZERO));
        }
        b.label(egui::RichText::new(format!("{:.0} %", k * 100.0)).font(theme::ui(12.5)).color(p.text3));
        let _ = tf;
    }

    // náhľad cesty / odkazu pri prejdení myšou (obrázok, začiatok súboru, priečinok alebo adresa)
    pub(super) fn link_tip(&mut self, ctx: &egui::Context, at: egui::Pos2, target: &str) {
        let p = self.pal;
        let url = target.starts_with("http://") || target.starts_with("https://");
        egui::Area::new(egui::Id::new("link-tip")).order(egui::Order::Tooltip).fixed_pos(at + vec2(14.0, 18.0)).interactable(false).show(ctx, |ui| {
            egui::Frame::NONE.fill(p.solid).stroke(Stroke::new(1.0, p.line_strong)).corner_radius(CornerRadius::same(10)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                ui.set_max_width(420.0);
                let name = if url { target.to_string() } else { widgets::file_name(target) };
                if !url && is_image(target) {
                    let (r, _) = ui.allocate_exact_size(vec2(240.0, 160.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(6), p.hover);
                    match self.imgs.paint(ui, target, r.shrink(4.0), true) {
                        Ok(None) => {
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, t("Loading…"), theme::ui(12.0), p.text3);
                        }
                        Err(_) => {
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, t("This image cannot be shown."), theme::ui(12.0), p.text3);
                        }
                        Ok(Some(_)) => {}
                    }
                    let dims = self.imgs.size_of(target).map(|s| format!("  ·  {} × {}", s[0], s[1])).unwrap_or_default();
                    ui.label(egui::RichText::new(format!("{name}{dims}")).font(theme::bold(12.5)).color(p.text));
                } else if url {
                    ui.label(egui::RichText::new(t("Link")).font(theme::bold(12.5)).color(p.text));
                    ui.label(egui::RichText::new(&name).font(theme::mono(12.0)).color(p.accent));
                } else if Path::new(target).is_dir() {
                    ui.label(egui::RichText::new(format!("📁 {name}")).font(theme::bold(12.5)).color(p.text));
                    let mut items: Vec<String> = std::fs::read_dir(target).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
                    items.sort();
                    let n = items.len();
                    for it in items.iter().take(10) {
                        ui.label(egui::RichText::new(it).font(theme::mono(11.5)).color(p.text2));
                    }
                    if n > 10 {
                        ui.label(egui::RichText::new(tf("and {n} more", &[("n", &(n - 10).to_string())])).font(theme::ui(11.5)).color(p.text3));
                    }
                } else {
                    ui.label(egui::RichText::new(&name).font(theme::bold(12.5)).color(p.text));
                    let head: String = std::fs::read(target)
                        .ok()
                        .filter(|b| !b.iter().take(4000).any(|c| *c == 0))
                        .map(|b| String::from_utf8_lossy(&b[..b.len().min(4000)]).lines().take(12).collect::<Vec<_>>().join("\n"))
                        .unwrap_or_default();
                    if !head.is_empty() {
                        egui::Frame::NONE.fill(p.hover).corner_radius(CornerRadius::same(6)).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                            ui.label(egui::RichText::new(head).font(theme::mono(11.5)).color(p.text2));
                        });
                    }
                }
                ui.add_space(2.0);
                ui.label(egui::RichText::new(t("Ctrl+click to open")).font(theme::ui(11.0)).color(p.text3));
            });
        });
        let _ = StrokeKind::Inside;
    }
}
