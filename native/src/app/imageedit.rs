// Obrázok v karte: listovanie obrázkov v priečinku (← →, pás náhľadov), úpravy (otočiť, prevrátiť,
// orezať, zmeniť veľkosť, späť), uloženie a prevod do PNG / JPG / WebP / ICO (favikona v 6 veľkostiach).
// Po prevode do nového súboru ponúkne opraviť cesty v kóde („logo.png“ → „logo.webp“), aby stránka fungovala ďalej.
use super::menus::Ask;
use super::App;
use crate::i18n::{t, tf};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// formáty, do ktorých Flux vie zapisovať
pub const FORMATS: [(&str, &str); 4] =
    [("png", "PNG – sharp, keeps transparency"), ("jpg", "JPG – small, good for photos"), ("webp", "WebP – lossless, for websites"), ("ico", "ICO – favicon for websites (16–256 px)")];
const ICO_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];
pub const ASPECTS: [(&str, f32); 5] = [("Free", 0.0), ("1:1", 1.0), ("16:9", 16.0 / 9.0), ("4:3", 4.0 / 3.0), ("3:2", 1.5)];

pub fn ext(path: &str) -> String {
    Path::new(path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

// upravovať sa dá všetko okrem SVG (vektor) – výsledok sa uloží ako bitmapa
pub fn editable(path: &str) -> bool {
    ext(path) != "svg"
}

pub struct Edit {
    pub path: String,
    pub img: image::DynamicImage,
    pub undo: Vec<image::DynamicImage>,
    pub tex: Option<egui::TextureHandle>,
}

pub struct Crop {
    pub r: Rect,                          // výrez v pixeloch obrázka
    pub aspect: usize,                    // index do ASPECTS
    drag: Option<(u8, Rect, egui::Pos2)>, // (0 = nový, 1 = posun, 2–5 = roh), výrez a bod na začiatku ťahania
}

pub struct Convert {
    pub fmt: usize,
    pub name: String,
    pub focus: bool,
}

enum Thumb {
    Loading,
    Decoded(egui::ColorImage),
    Ready(egui::TextureHandle),
    Failed,
}

#[derive(Default)]
pub struct ImgEdit {
    pub edit: Option<Edit>,
    pub crop: Option<Crop>,
    pub convert: Option<Convert>,
    pub resize: Option<(String, bool)>, // text poľa „šírka“ a fokus
    folder: Option<(String, Instant, Vec<String>)>,
    thumbs: HashMap<String, Arc<Mutex<Thumb>>>,
    pub refs: Vec<(String, String)>, // čakajúca oprava ciest: (súbor, nový text)
}

impl ImgEdit {
    // obrázky v rovnakom priečinku (zoznam sa obnoví najviac raz za 2 s)
    pub fn siblings(&mut self, path: &str) -> Vec<String> {
        let dir = Path::new(path).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        let fresh = self.folder.as_ref().is_some_and(|(d, at, _)| *d == dir && at.elapsed() < Duration::from_secs(2));
        if !fresh {
            let mut v: Vec<String> = std::fs::read_dir(&dir)
                .map(|rd| rd.flatten().filter(|e| e.path().is_file()).map(|e| e.path().to_string_lossy().to_string()).filter(|p| super::imageview::is_image(p)).collect())
                .unwrap_or_default();
            v.sort_by_key(|p| widgets::file_name(p).to_lowercase());
            self.folder = Some((dir, Instant::now(), v));
        }
        let v = self.folder.as_ref().map(|f| f.2.clone()).unwrap_or_default();
        if v.iter().any(|p| p == path) {
            v
        } else {
            vec![path.to_string()]
        }
    }

    pub fn dirty(&self, path: &str) -> bool {
        self.edit.as_ref().is_some_and(|e| e.path == path && !e.undo.is_empty())
    }

    // náhľad do pásu (dekóduje sa na pozadí, ≤ 96 px)
    pub fn thumb(&mut self, ui: &egui::Ui, path: &str, r: Rect) {
        if self.thumbs.len() > 200 {
            self.thumbs.clear();
        }
        let slot = self
            .thumbs
            .entry(path.to_string())
            .or_insert_with(|| {
                let s = Arc::new(Mutex::new(Thumb::Loading));
                let (s2, p) = (s.clone(), path.to_string());
                std::thread::spawn(move || {
                    let r = if ext(&p) == "svg" {
                        Thumb::Failed
                    } else {
                        match image::open(&p) {
                            Ok(im) => {
                                let th = im.thumbnail(128, 96).to_rgba8();
                                Thumb::Decoded(egui::ColorImage::from_rgba_unmultiplied([th.width() as usize, th.height() as usize], &th))
                            }
                            Err(_) => Thumb::Failed,
                        }
                    };
                    *s2.lock().unwrap() = r;
                });
                s
            })
            .clone();
        let mut g = slot.lock().unwrap();
        if let Thumb::Decoded(ci) = &*g {
            let tex = ui.ctx().load_texture(format!("thumb:{path}"), ci.clone(), egui::TextureOptions::LINEAR);
            *g = Thumb::Ready(tex);
        }
        match &*g {
            Thumb::Ready(tex) => {
                // vyplniť štvorček (orezať okraje), ako object-fit: cover
                let s = tex.size_vec2();
                let k = (r.width() / s.x).max(r.height() / s.y);
                let (uw, uh) = ((r.width() / (s.x * k)).min(1.0), (r.height() / (s.y * k)).min(1.0));
                let uv = Rect::from_center_size(pos2(0.5, 0.5), vec2(uw, uh));
                ui.painter().image(tex.id(), r, uv, Color32::WHITE);
            }
            Thumb::Loading => {
                ui.ctx().request_repaint_after(Duration::from_millis(120));
            }
            _ => {
                let e = ext(path).to_uppercase();
                ui.painter().text(r.center(), Align2::CENTER_CENTER, e, theme::bold(10.0), Color32::from_gray(140));
            }
        }
    }
}

// ---------- úpravy (čisté funkcie, testované) ----------
pub fn rotate(img: &image::DynamicImage, right: bool) -> image::DynamicImage {
    if right {
        img.rotate90()
    } else {
        img.rotate270()
    }
}

pub fn crop(img: &image::DynamicImage, r: Rect) -> image::DynamicImage {
    let x = r.left().max(0.0).round() as u32;
    let y = r.top().max(0.0).round() as u32;
    let w = (r.width().round() as u32).clamp(1, img.width().saturating_sub(x).max(1));
    let h = (r.height().round() as u32).clamp(1, img.height().saturating_sub(y).max(1));
    img.crop_imm(x, y, w, h)
}

pub fn resize(img: &image::DynamicImage, width: u32) -> image::DynamicImage {
    let h = ((img.height() as f64 * width as f64 / img.width().max(1) as f64).round() as u32).max(1);
    img.resize_exact(width, h, image::imageops::FilterType::Lanczos3)
}

// obrázok → bajty súboru vo formáte fmt (png, jpg/jpeg, webp, ico, bmp)
pub fn encode(img: &image::DynamicImage, fmt: &str) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut out: Vec<u8> = vec![];
    let err = |e: image::ImageError| e.to_string();
    match fmt {
        "png" => {
            let rgba = img.to_rgba8();
            image::codecs::png::PngEncoder::new(&mut out).write_image(&rgba, rgba.width(), rgba.height(), image::ExtendedColorType::Rgba8).map_err(err)?;
        }
        "jpg" | "jpeg" => {
            // JPG nemá priehľadnosť – priehľadné miesta budú biele (nie čierne)
            let rgba = img.to_rgba8();
            let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
            for (x, y, p) in rgba.enumerate_pixels() {
                let a = p[3] as u32;
                let mix = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
                rgb.put_pixel(x, y, image::Rgb([mix(p[0]), mix(p[1]), mix(p[2])]));
            }
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90).write_image(&rgb, rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8).map_err(err)?;
        }
        "webp" => {
            let rgba = img.to_rgba8();
            image::codecs::webp::WebPEncoder::new_lossless(&mut out).write_image(&rgba, rgba.width(), rgba.height(), image::ExtendedColorType::Rgba8).map_err(err)?;
        }
        "bmp" => {
            let rgba = img.to_rgba8();
            image::codecs::bmp::BmpEncoder::new(&mut out).write_image(&rgba, rgba.width(), rgba.height(), image::ExtendedColorType::Rgba8).map_err(err)?;
        }
        "ico" => {
            // štvorec v každej veľkosti (obrázok v strede), PNG vnútri ICO
            let max = img.width().max(img.height());
            let mut frames = vec![];
            for (i, s) in ICO_SIZES.iter().enumerate() {
                if i >= 2 && *s > max {
                    break;
                }
                let fit = img.resize(*s, *s, image::imageops::FilterType::Lanczos3).to_rgba8();
                let mut sq = image::RgbaImage::new(*s, *s);
                image::imageops::overlay(&mut sq, &fit, ((s - fit.width()) / 2) as i64, ((s - fit.height()) / 2) as i64);
                frames.push(image::codecs::ico::IcoFrame::as_png(&sq, *s, *s, image::ExtendedColorType::Rgba8).map_err(err)?);
            }
            image::codecs::ico::IcoEncoder::new(&mut out).encode_images(&frames).map_err(err)?;
        }
        _ => return Err(t("This format cannot be saved here.")),
    }
    Ok(out)
}

// reťazce v úvodzovkách / url(...) na riadku: (začiatok, koniec) v bajtoch obsahu
fn quoted(line: &str) -> Vec<(usize, usize)> {
    let b = line.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'"' || c == b'\'' || c == b'`' {
            if let Some(end) = line[i + 1..].find(c as char) {
                out.push((i + 1, i + 1 + end));
                i += end + 2;
                continue;
            }
        } else if c == b'u' && line.is_char_boundary(i) && line[i..].starts_with("url(") {
            if let Some(end) = line[i + 4..].find(')') {
                let (s, e) = (i + 4, i + 4 + end);
                if !line[s..e].starts_with(['"', '\'']) {
                    out.push((s, e));
                }
                i = e + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn norm(p: &Path) -> String {
    let mut parts: Vec<String> = vec![];
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::CurDir => {}
            other => parts.push(other.as_os_str().to_string_lossy().to_string()),
        }
    }
    let s = parts.join("/").replace('\\', "/").replace("//", "/");
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

// Cesty v textových súboroch projektu, ktoré ukazujú na `old` (relatívne k súboru, k projektu alebo „/…“ od koreňa).
// Vráti (súbor, nový text, počet ciest) – meno súboru v reťazci sa nahradí menom `new`.
pub fn path_refs(files: &[String], ws: &str, old: &str, new: &str) -> Vec<(String, String, usize)> {
    let old_name = widgets::file_name(old);
    let new_name = widgets::file_name(new);
    let target = norm(Path::new(old));
    const TEXT: &[&str] = &["html", "htm", "css", "scss", "less", "js", "mjs", "cjs", "jsx", "ts", "tsx", "json", "md", "py", "php", "vue", "svelte", "xml", "svg", "txt"];
    let mut out = vec![];
    for f in files {
        if !TEXT.contains(&ext(f).as_str()) || f == old {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        if text.len() > 1_000_000 || !text.contains(&old_name) {
            continue;
        }
        let dir = Path::new(f).parent().unwrap_or(Path::new(""));
        let mut n = 0;
        let mut res = String::with_capacity(text.len());
        for (li, line) in text.split('\n').enumerate() {
            if li > 0 {
                res.push('\n');
            }
            let mut last = 0;
            for (s, e) in quoted(line) {
                let v = &line[s..e];
                if !v.contains(&old_name) || v.contains("://") || v.starts_with("data:") {
                    continue;
                }
                let clean = v.split(['?', '#']).next().unwrap_or(v);
                let hit =
                    if let Some(abs) = clean.strip_prefix('/') { norm(&Path::new(ws).join(abs)) == target } else { norm(&dir.join(clean)) == target || norm(&Path::new(ws).join(clean)) == target };
                if !hit {
                    continue;
                }
                // nahradiť posledný výskyt mena súboru v ceste
                if let Some(pos) = v.rfind(&old_name) {
                    res.push_str(&line[last..s + pos]);
                    res.push_str(&new_name);
                    last = s + pos + old_name.len();
                    n += 1;
                }
            }
            res.push_str(&line[last..]);
        }
        if n > 0 {
            out.push((f.clone(), res, n));
        }
    }
    out
}

impl App {
    fn img_path(&self) -> Option<String> {
        self.tabs.get(self.active).filter(|t| t.image).map(|t| t.path.clone())
    }

    // načíta obrázok na úpravy (raz; ďalšie úpravy pracujú s ním)
    fn img_begin(&mut self) -> bool {
        let Some(path) = self.img_path() else { return false };
        if self.imgs.ed.edit.as_ref().is_some_and(|e| e.path == path) {
            return true;
        }
        if !editable(&path) {
            self.status = t("SVG is a vector image – convert it to PNG first.");
            return false;
        }
        match image::open(&path) {
            Ok(img) => {
                self.imgs.ed.edit = Some(Edit { path, img, undo: vec![], tex: None });
                true
            }
            Err(e) => {
                self.status = e.to_string();
                false
            }
        }
    }

    pub(super) fn img_begin_pub(&mut self) -> bool {
        self.img_begin()
    }

    pub(super) fn img_apply(&mut self, op: &str) {
        if !self.img_begin() {
            return;
        }
        let crop_r = self.imgs.ed.crop.as_ref().map(|c| c.r);
        let width = self.imgs.ed.resize.as_ref().and_then(|(s, _)| s.trim().to_string().into());
        let Some(e) = self.imgs.ed.edit.as_mut() else { return };
        let next = match op {
            "rotl" => Some(rotate(&e.img, false)),
            "rotr" => Some(rotate(&e.img, true)),
            "flip" => Some(e.img.fliph()),
            "crop" => crop_r.map(|r| crop(&e.img, r)),
            "resize" => {
                let w = width.unwrap_or_default();
                let n = if let Some(p) = w.strip_suffix('%') { (e.img.width() as f64 * p.trim().parse::<f64>().unwrap_or(0.0) / 100.0).round() as u32 } else { w.parse::<u32>().unwrap_or(0) };
                if !(1..=16384).contains(&n) {
                    self.status = t("Enter a width between 1 and 16384.");
                    return;
                }
                (n != e.img.width()).then(|| resize(&e.img, n))
            }
            "undo" => {
                if let Some(prev) = e.undo.pop() {
                    e.img = prev;
                    e.tex = None;
                }
                None
            }
            _ => None,
        };
        if let Some(n) = next {
            e.undo.push(std::mem::replace(&mut e.img, n));
            if e.undo.len() > 12 {
                e.undo.remove(0);
            }
            e.tex = None;
        }
        if op == "crop" {
            self.imgs.ed.crop = None;
        }
        if op == "resize" {
            self.imgs.ed.resize = None;
        }
        let path = self.img_path().unwrap_or_default();
        self.imgs.view.insert(path, (None, egui::Vec2::ZERO));
    }

    pub(super) fn img_crop_start(&mut self) {
        if !self.img_begin() {
            return;
        }
        let Some(e) = &self.imgs.ed.edit else { return };
        let (w, h) = (e.img.width() as f32, e.img.height() as f32);
        self.imgs.ed.crop = Some(Crop { r: Rect::from_min_size(pos2(w * 0.1, h * 0.1), vec2(w * 0.8, h * 0.8)), aspect: 0, drag: None });
        let path = e.path.clone();
        self.imgs.view.insert(path, (None, egui::Vec2::ZERO));
    }

    // ďalší / predošlý obrázok v priečinku (karta ostane tá istá)
    pub(super) fn img_go(&mut self, to: String) {
        let Some(cur) = self.img_path() else { return };
        if to == cur {
            return;
        }
        if self.imgs.ed.dirty(&cur) {
            self.status = t("The image has unsaved changes – save or undo them first.");
            return;
        }
        self.imgs.ed.edit = None;
        self.imgs.ed.crop = None;
        self.imgs.ed.resize = None;
        if let Some(i) = self.tabs.iter().position(|t| t.path == to) {
            self.activate(i);
        } else {
            self.imgs.forget(&cur);
            self.tabs[self.active].path = to;
        }
    }

    // Ctrl+S na karte s obrázkom: uložiť úpravy (formát, ktorý Flux nevie zapísať → prevod)
    pub(super) fn img_save(&mut self) {
        let Some(path) = self.img_path() else { return };
        if !self.imgs.ed.dirty(&path) {
            return;
        }
        let fmt = ext(&path);
        if !matches!(fmt.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp" | "ico") {
            self.img_convert_open();
            return;
        }
        let res = self.imgs.ed.edit.as_ref().map(|e| encode(&e.img, &fmt)).unwrap_or(Err(String::new()));
        match res.and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string())) {
            Ok(_) => {
                self.imgs.ed.edit = None;
                self.imgs.forget(&path);
                self.note(tf("Saved {name}.", &[("name", &widgets::file_name(&path))]));
            }
            Err(e) => self.status = e,
        }
    }

    pub(super) fn img_convert_open(&mut self) {
        let Some(path) = self.img_path() else { return };
        let stem = Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        // predvolený formát iný ako terajší (PNG → WebP, ostatné → PNG)
        let fmt = if ext(&path) == "png" { 2 } else { 0 };
        self.imgs.ed.convert = Some(Convert { fmt, name: format!("{stem}.{}", FORMATS[fmt].0), focus: true });
    }

    fn img_convert_do(&mut self) {
        let Some(path) = self.img_path() else { return };
        let Some(cv) = self.imgs.ed.convert.take() else { return };
        let fmt = FORMATS[cv.fmt].0;
        let mut name = cv.name.trim().to_string();
        if name.is_empty() {
            return;
        }
        if Path::new(&name).extension().is_none() {
            name = format!("{name}.{fmt}");
        }
        let target = Path::new(&path).parent().unwrap_or(Path::new(".")).join(&name).to_string_lossy().to_string();
        // obrázok: upravený, alebo načítaný zo súboru (SVG cez egui nevieme – len bitmapy)
        let img = match self.imgs.ed.edit.as_ref().filter(|e| e.path == path) {
            Some(e) => Ok(e.img.clone()),
            None => image::open(&path).map_err(|e| e.to_string()),
        };
        let fe = ext(&name);
        let use_fmt = if matches!(fe.as_str(), "png" | "jpg" | "jpeg" | "webp" | "ico" | "bmp") { fe.clone() } else { fmt.to_string() };
        match img.and_then(|im| encode(&im, &use_fmt)).and_then(|b| std::fs::write(&target, &b).map(|_| b.len()).map_err(|e| e.to_string())) {
            Ok(n) => {
                let kb = (n as f64 / 1024.0).ceil() as u64;
                self.note(tf("Saved {name} ({size}).", &[("name", &name), ("size", &format!("{kb} KB"))]));
                if target != path {
                    self.imgs.ed.edit = None;
                    self.imgs.ed.folder = None;
                    self.img_go(target.clone());
                    self.img_after_convert(&path, &target);
                } else {
                    self.imgs.ed.edit = None;
                    self.imgs.forget(&path);
                }
            }
            Err(e) => self.status = e,
        }
    }

    // po prevode: opraviť cesty v kóde; nová .ico → ponúknuť <link rel="icon"> v index.html
    fn img_after_convert(&mut self, old: &str, new: &str) {
        let Some(ws) = self.workspace() else { return };
        if !new.starts_with(&ws) {
            return;
        }
        if ext(new) == "ico" {
            let index = Path::new(&ws).join("index.html");
            let Ok(html) = std::fs::read_to_string(&index) else { return };
            let low = html.to_lowercase();
            if low.contains("rel=\"icon\"") || low.contains("rel='icon'") || low.contains("shortcut icon") || !low.contains("</head>") {
                return;
            }
            let href = new.strip_prefix(&ws).unwrap_or(new).trim_start_matches(['/', '\\']).replace('\\', "/");
            self.ask = Some(Ask { text: tf("Use {name} as the favicon of index.html?", &[("name", &widgets::file_name(new))]), yes: t("Add"), action: format!("img-favicon:{href}") });
            return;
        }
        let files: Vec<String> = flux_core::fsops::list_all(&self.core).as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let refs = path_refs(&files, &ws, old, new);
        if refs.is_empty() {
            return;
        }
        let n: usize = refs.iter().map(|r| r.2).sum();
        self.imgs.ed.refs = refs.into_iter().map(|(f, s, _)| (f, s)).collect();
        let f = self.imgs.ed.refs.len();
        let name = widgets::file_name(old);
        let text = if n == 1 {
            tf("Update the path to {name}?", &[("name", &name)])
        } else if f == 1 {
            tf("Update {n} paths to {name} in 1 file?", &[("n", &n.to_string()), ("name", &name)])
        } else {
            tf("Update {n} paths to {name} in {f} files?", &[("n", &n.to_string()), ("name", &name), ("f", &f.to_string())])
        };
        self.ask = Some(Ask { text, yes: t("Update"), action: "img-refs".into() });
    }

    // potvrdené otázky (act → img-refs / img-favicon:<href>)
    pub(super) fn img_ask_done(&mut self, id: &str) {
        if id == "img-refs" {
            let refs = std::mem::take(&mut self.imgs.ed.refs);
            let mut done = 0;
            for (f, text) in refs {
                // otvorený neupravený súbor: zmeniť aj kartu; upravený nechať (používateľ ho práve mení)
                if let Some(tab) = self.tabs.iter_mut().find(|x| x.path == f) {
                    if tab.text != tab.saved {
                        continue;
                    }
                    tab.text = text.clone();
                    tab.saved = text.clone();
                }
                if std::fs::write(&f, &text).is_ok() {
                    done += 1;
                }
            }
            self.note(tf("Updated paths in {n} files.", &[("n", &done.to_string())]));
        } else if let Some(href) = id.strip_prefix("img-favicon:") {
            let Some(ws) = self.workspace() else { return };
            let index = Path::new(&ws).join("index.html").to_string_lossy().to_string();
            let Ok(html) = std::fs::read_to_string(&index) else { return };
            let at = html.to_lowercase().find("</head>").unwrap_or(html.len());
            let line_start = html[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let indent: String = html[line_start..at].chars().take_while(|c| c.is_whitespace()).collect();
            let tag = format!("{indent}  <link rel=\"icon\" href=\"{href}\">\n");
            let mut out = html.clone();
            out.insert_str(line_start, &tag);
            if let Some(tab) = self.tabs.iter_mut().find(|x| x.path == index) {
                tab.text = out.clone();
                tab.saved = out.clone();
            }
            match std::fs::write(&index, out) {
                Ok(_) => self.note(t("Favicon added to index.html.")),
                Err(e) => self.status = e.to_string(),
            }
        }
    }

    // ---------- kreslenie ----------
    // obrázok s úpravami (textúra z upraveného obrázka); vráti rozmery
    pub(super) fn img_paint_edit(&mut self, ui: &egui::Ui, target: Rect) -> Option<[usize; 2]> {
        let e = self.imgs.ed.edit.as_mut()?;
        if e.tex.is_none() {
            let shown = if e.img.width().max(e.img.height()) > 4096 { e.img.thumbnail(4096, 4096) } else { e.img.clone() };
            let rgba = shown.to_rgba8();
            let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], &rgba);
            e.tex = Some(ui.ctx().load_texture(format!("imgedit:{}", e.path), ci, egui::TextureOptions::LINEAR));
        }
        let tex = e.tex.as_ref()?;
        ui.painter().image(tex.id(), target, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        Some([e.img.width() as usize, e.img.height() as usize])
    }

    // orezanie: stmavenie mimo výrezu, mriežka tretín, rohy; ťahanie = nový výrez / posun / zmena rohom
    pub(super) fn img_crop_ui(&mut self, ui: &mut egui::Ui, target: Rect, size: [usize; 2]) {
        let Some(c) = self.imgs.ed.crop.as_mut() else { return };
        let k = target.width() / size[0].max(1) as f32;
        let to_s = |r: Rect| Rect::from_min_max(target.min + r.min.to_vec2() * k, target.min + r.max.to_vec2() * k);
        let resp = ui.interact(target, ui.id().with("img-crop"), Sense::click_and_drag());
        let ratio = ASPECTS[c.aspect].1;
        let (iw, ih) = (size[0] as f32, size[1] as f32);
        if let Some(pos) = resp.interact_pointer_pos() {
            let px = ((pos - target.min) / k).to_pos2();
            let px = pos2(px.x.clamp(0.0, iw), px.y.clamp(0.0, ih));
            if resp.drag_started() {
                let s = to_s(c.r);
                let corners = [s.left_top(), s.right_top(), s.left_bottom(), s.right_bottom()];
                let kind = corners.iter().position(|q| q.distance(pos) < 14.0).map(|i| i as u8 + 2).unwrap_or(if s.contains(pos) { 1 } else { 0 });
                c.drag = Some((kind, c.r, px));
            }
            if let (true, Some((kind, start, from))) = (resp.dragged(), c.drag) {
                if kind == 1 {
                    let d = px - from;
                    let mut r = start.translate(d);
                    r = r.translate(vec2((-r.left()).max(0.0) - (r.right() - iw).max(0.0), (-r.top()).max(0.0) - (r.bottom() - ih).max(0.0)));
                    c.r = r;
                } else {
                    // pevný bod: oproti ťahanému rohu, pri novom výreze miesto kliknutia
                    let anchor = match kind {
                        2 => start.right_bottom(),
                        3 => start.left_bottom(),
                        4 => start.right_top(),
                        5 => start.left_top(),
                        _ => from,
                    };
                    let mut w = (px.x - anchor.x).abs().max(4.0);
                    let mut h = (px.y - anchor.y).abs().max(4.0);
                    if ratio > 0.0 {
                        if w / h > ratio {
                            w = h * ratio;
                        } else {
                            h = w / ratio;
                        }
                    }
                    let x = if px.x < anchor.x { anchor.x - w } else { anchor.x };
                    let y = if px.y < anchor.y { anchor.y - h } else { anchor.y };
                    c.r = Rect::from_min_size(pos2(x, y), vec2(w, h)).intersect(Rect::from_min_size(pos2(0.0, 0.0), vec2(iw, ih)));
                }
            }
        }
        if resp.drag_stopped() {
            c.drag = None;
        }
        let s = to_s(c.r);
        let dim = Color32::from_black_alpha(140);
        let pa = ui.painter_at(target);
        pa.rect_filled(Rect::from_min_max(target.min, pos2(target.right(), s.top())), 0.0, dim);
        pa.rect_filled(Rect::from_min_max(pos2(target.left(), s.bottom()), target.max), 0.0, dim);
        pa.rect_filled(Rect::from_min_max(pos2(target.left(), s.top()), pos2(s.left(), s.bottom())), 0.0, dim);
        pa.rect_filled(Rect::from_min_max(pos2(s.right(), s.top()), pos2(target.right(), s.bottom())), 0.0, dim);
        let thin = Stroke::new(1.0, Color32::from_white_alpha(90));
        for i in 1..3 {
            let f = i as f32 / 3.0;
            pa.vline(s.left() + s.width() * f, s.y_range(), thin);
            pa.hline(s.x_range(), s.top() + s.height() * f, thin);
        }
        pa.rect_stroke(s, 0.0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Inside);
        for (q, dx, dy) in [(s.left_top(), 1.0, 1.0), (s.right_top(), -1.0, 1.0), (s.left_bottom(), 1.0, -1.0), (s.right_bottom(), -1.0, -1.0)] {
            let w = Stroke::new(3.0, Color32::WHITE);
            pa.line_segment([q, q + vec2(14.0 * dx, 0.0)], w);
            pa.line_segment([q, q + vec2(0.0, 14.0 * dy)], w);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
    }

    // pás náhľadov nad spodnou lištou; klik = ten obrázok
    pub(super) fn img_strip(&mut self, ui: &mut egui::Ui, r: Rect, list: &[String], cur: &str) -> Option<String> {
        let p = self.pal;
        ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, p.line));
        let (tw, th, gap) = (64.0, 46.0, 6.0);
        let total = list.len() as f32 * (tw + gap);
        let idx = list.iter().position(|x| x == cur).unwrap_or(0) as f32;
        // aktuálny obrázok v strede pásu (keď sa nezmestia všetky)
        let off = if total <= r.width() - 24.0 { (r.width() - total) / 2.0 } else { (r.width() / 2.0 - idx * (tw + gap) - tw / 2.0).clamp(r.width() - 12.0 - total, 12.0) };
        let clip = ui.painter_at(r);
        let mut picked = None;
        for (i, path) in list.iter().enumerate() {
            let x = r.left() + off + i as f32 * (tw + gap);
            let tr = Rect::from_min_size(pos2(x, r.center().y - th / 2.0), vec2(tw, th));
            if tr.right() < r.left() || tr.left() > r.right() {
                continue;
            }
            clip.rect_filled(tr, CornerRadius::same(7), p.hover);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(tr));
            child.set_clip_rect(tr.intersect(r));
            self.imgs.ed.thumb(&child, path, tr.shrink(1.0));
            let on = path == cur;
            let resp = ui.interact(tr.intersect(r), ui.id().with(("thumb", i)), Sense::click()).on_hover_text(widgets::file_name(path));
            let stroke = if on {
                Stroke::new(2.0, p.accent)
            } else if resp.hovered() {
                Stroke::new(1.0, p.line_strong)
            } else {
                Stroke::new(1.0, p.line)
            };
            clip.rect_stroke(tr, CornerRadius::same(7), stroke, StrokeKind::Outside);
            if resp.clicked() {
                picked = Some(path.clone());
            }
        }
        picked
    }

    // okno „Previesť / uložiť ako“ (formát + meno súboru)
    pub(super) fn img_convert_ui(&mut self, ctx: &egui::Context, full: Rect) {
        let p = self.pal;
        let Some(path) = self.img_path() else {
            self.imgs.ed.convert = None;
            return;
        };
        let mut go = false;
        let mut close = false;
        egui::Area::new(egui::Id::new("img-convert-dim")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
            let bg = ui.interact(full, ui.id().with("dim"), Sense::click());
            ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(70));
            if bg.clicked() {
                close = true;
            }
        });
        egui::Area::new(egui::Id::new("img-convert")).order(egui::Order::Foreground).anchor(Align2::CENTER_CENTER, vec2(0.0, -20.0)).show(ctx, |ui| {
            let Some(cv) = self.imgs.ed.convert.as_mut() else { return };
            egui::Frame::NONE.fill(p.solid).stroke(Stroke::new(1.0, p.line_strong)).corner_radius(CornerRadius::same(16)).inner_margin(egui::Margin::same(18)).show(ui, |ui| {
                ui.set_width(380.0);
                ui.label(egui::RichText::new(tf("Convert “{name}” to…", &[("name", &widgets::file_name(&path))])).font(theme::bold(15.0)).color(p.text));
                ui.add_space(10.0);
                for (i, (fmt, label)) in FORMATS.iter().enumerate() {
                    let on = cv.fmt == i;
                    let (r, resp) = ui.allocate_exact_size(vec2(380.0, 34.0), Sense::click());
                    if on || resp.hovered() {
                        ui.painter().rect_filled(r, CornerRadius::same(8), if on { p.active } else { p.hover });
                    }
                    ui.painter().text(pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, t(label), theme::ui(13.0), p.text);
                    if resp.clicked() {
                        cv.fmt = i;
                        let stem = Path::new(&cv.name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        cv.name = format!("{stem}.{fmt}");
                    }
                }
                ui.add_space(10.0);
                ui.label(egui::RichText::new(t("File name")).font(theme::ui(12.0)).color(p.text3));
                let te = ui.add(egui::TextEdit::singleline(&mut cv.name).desired_width(380.0).margin(egui::Margin::symmetric(10, 7)).font(theme::ui(13.5)));
                if cv.focus {
                    cv.focus = false;
                    te.request_focus();
                }
                if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    go = true;
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if widgets::button(ui, Some("imageEdit"), &t("Convert"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                        go = true;
                    }
                    if widgets::button(ui, None, &t("Cancel"), p.card2, p.text, 30.0, &p).clicked() {
                        close = true;
                    }
                });
            });
        });
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            close = true;
        }
        if close {
            self.imgs.ed.convert = None;
        } else if go {
            self.img_convert_do();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> image::DynamicImage {
        let mut im = image::RgbaImage::new(40, 20);
        for (x, _, p) in im.enumerate_pixels_mut() {
            *p = image::Rgba([if x < 20 { 255 } else { 0 }, 0, 0, if x < 30 { 255 } else { 0 }]);
        }
        image::DynamicImage::ImageRgba8(im)
    }

    #[test]
    fn imgedit_ops() {
        let im = sample();
        let r = rotate(&im, true);
        assert_eq!((r.width(), r.height()), (20, 40));
        let c = crop(&im, Rect::from_min_size(pos2(5.0, 5.0), vec2(10.0, 8.0)));
        assert_eq!((c.width(), c.height()), (10, 8));
        let s = resize(&im, 20);
        assert_eq!((s.width(), s.height()), (20, 10));
        let f = im.fliph().to_rgba8();
        assert_eq!(f.get_pixel(39, 0)[0], 255);
    }

    #[test]
    fn imgedit_encode_formats() {
        let im = sample();
        for fmt in ["png", "jpg", "webp", "bmp", "ico"] {
            let b = encode(&im, fmt).unwrap();
            let back = image::load_from_memory(&b).unwrap();
            assert!(back.width() > 0, "{fmt}");
        }
        // JPG: priehľadné miesto je biele
        let j = image::load_from_memory(&encode(&im, "jpg").unwrap()).unwrap().to_rgb8();
        assert!(j.get_pixel(38, 10)[1] > 200);
        // ICO: hlavička a 2 veľkosti (16, 32) pre malý obrázok
        let ico = encode(&im, "ico").unwrap();
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 2);
    }

    #[test]
    fn imgedit_path_refs() {
        let dir = std::env::temp_dir().join(format!("flux-refs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("img")).unwrap();
        std::fs::create_dir_all(dir.join("css")).unwrap();
        let ws = dir.to_string_lossy().to_string();
        let html = dir.join("index.html");
        std::fs::write(&html, "<title>Moja stránka – úvod</title>\n<img src=\"img/logo.png\">\n<img src='/img/logo.png?v=2'>\n<a href=\"other/logo.png\">").unwrap();
        let css = dir.join("css/a.css");
        std::fs::write(&css, "body{background:url(../img/logo.png)}").unwrap();
        let files = vec![html.to_string_lossy().to_string(), css.to_string_lossy().to_string()];
        let old = dir.join("img/logo.png").to_string_lossy().to_string();
        let new = dir.join("img/logo.webp").to_string_lossy().to_string();
        let r = path_refs(&files, &ws, &old, &new);
        assert_eq!(r.len(), 2);
        let h = r.iter().find(|x| x.0.ends_with("index.html")).unwrap();
        assert_eq!(h.2, 2);
        assert!(h.1.contains("img/logo.webp\"") && h.1.contains("/img/logo.webp?v=2") && h.1.contains("other/logo.png"));
        assert!(r.iter().any(|x| x.1.contains("url(../img/logo.webp)")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
