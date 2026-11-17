// Náhľad Live Servera bez prehliadača: HTML + CSS sa rozloží (blitz: Stylo + Taffy) a vykreslí do obrázka (vello_cpu),
// ktorý egui ukáže ako textúru. ~5 MB namiesto ~90 MB WebView2 / WebKitGTK; JavaScript sa nespúšťa.
// Súbory číta priamo cez server::Reader (aj neuložený text), obrázky a štýly z iných webov stiahne curl na vlákne.
use anyrender::ImageRenderer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_dom::{local_name, DocumentConfig, StyleThreading};
use blitz_html::HtmlDocument;
use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request};
use blitz_traits::shell::{ColorScheme, Viewport};
use eframe::egui;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::server::Reader;

// súbory stránky: z projektu priamo, z iných webov cez curl (na vlákne – UI nečaká)
struct Net {
    reader: Reader,
    pending: Arc<AtomicUsize>, // koľko súborov sa ešte načítava (po dokončení sa stránka prekreslí)
    done: Arc<AtomicUsize>,
}

impl NetProvider for Net {
    fn fetch(&self, _doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        let url = request.url;
        let local = url.host_str() == Some("127.0.0.1") && url.port() == Some(self.reader.port);
        if local {
            if let Some(b) = self.reader.read(url.path()) {
                handler.bytes(url.to_string(), Bytes::from(b));
                self.done.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }
        if !matches!(url.scheme(), "http" | "https") {
            return;
        }
        // štýly a obrázky z CDN (Bootstrap, Google Fonts…), najviac 8 MB
        let (pending, done) = (self.pending.clone(), self.done.clone());
        pending.fetch_add(1, Ordering::Relaxed);
        std::thread::spawn(move || {
            if let Some(b) = crate::net::bytes(url.as_str()) {
                if b.len() <= 8 << 20 {
                    handler.bytes(url.to_string(), Bytes::from(b));
                }
            }
            pending.fetch_sub(1, Ordering::Relaxed);
            done.fetch_add(1, Ordering::Relaxed);
        });
    }
}

// čo sa stalo po kliknutí do stránky
pub enum Click {
    None,
    Line(usize),          // prvok so zdrojom na riadku N (skok v editore)
    Go(String),           // odkaz na inú stránku projektu
    External(String),     // odkaz na iný web → prehliadač
}

pub struct HtmlView {
    doc: Option<HtmlDocument>,
    key: (String, u64), // adresa + verzia servera, z ktorých je dokument
    vp: (u32, u32, u32), // fyzická šírka, výška, mierka × 1000
    renderer: Option<VelloCpuImageRenderer>,
    used: Instant, // posledné kreslenie – vykresľovač sa po chvíli nečinnosti uvoľní
    tex: Option<egui::TextureHandle>,
    dirty: bool,
    pending: Arc<AtomicUsize>,
    done: Arc<AtomicUsize>,
    seen_done: usize,
    built: Instant,    // kedy sa dokument naposledy zostavil – pri písaní najviac ~5× za sekundu
    // rolovanie bez prekresľovania: nakreslí sa pás ~2 obrazoviek a pri rolovaní sa len posúva obrázok
    band: (f64, f64), // (vrch, výška) nakresleného pásu v CSS px
    fixed: bool,      // stránka má position: fixed/sticky – tá sa musí kresliť pri každom posune
    want_band: bool,  // pás len pri rolovaní; pri písaní a hover stačí to, čo je na obrazovke (menej práce)
    pub waiting: bool, // nová verzia čaká na zostavenie (treba prekresliť o chvíľu)
    pub scripts: bool, // stránka má JavaScript (tu nebeží)
    pub link_hover: bool,
}

impl Default for HtmlView {
    fn default() -> Self {
        HtmlView {
            doc: None,
            key: (String::new(), 0),
            vp: (0, 0, 0),
            renderer: None,
            used: Instant::now(),
            tex: None,
            dirty: false,
            pending: Default::default(),
            done: Default::default(),
            seen_done: 0,
            built: Instant::now(),
            band: (0.0, 0.0),
            fixed: true,
            want_band: false,
            waiting: false,
            scripts: false,
            link_hover: false,
        }
    }
}

fn has_script(html: &str) -> bool {
    let low = html.to_ascii_lowercase();
    let mut i = 0;
    while let Some(k) = low[i..].find("<script") {
        let s = i + k;
        let Some(e) = low[s..].find('>') else { break };
        let open = &low[s..s + e];
        let body_end = low[s + e..].find("</script").map(|x| s + e + x).unwrap_or(low.len());
        if open.contains(" src") || !low[s + e + 1..body_end].trim().is_empty() {
            return true;
        }
        i = body_end.max(s + 1);
    }
    false
}

impl HtmlView {
    // stránka na adrese `url` vo verzii `ver` s veľkosťou (fyzické px) a mierkou (px na CSS px)
    pub fn sync(&mut self, reader: &Reader, url: &str, ver: u64, w: u32, h: u32, scale: f32) {
        let (w, h) = (w.max(16), h.max(16));
        let vp = (w, h, (scale * 1000.0) as u32);
        // písanie (rovnaká stránka, nová verzia): zostaviť najviac raz za 180 ms
        let same_page = self.key.0 == url && self.doc.is_some();
        self.waiting = same_page && self.key.1 != ver && self.built.elapsed().as_millis() < 180;
        if !self.waiting && (self.key.0 != url || self.key.1 != ver || self.doc.is_none()) {
            self.built = Instant::now();
            let path = url::Url::parse(url).map(|u| u.path().to_string()).unwrap_or_else(|_| "/".into());
            let html = reader.read(&path).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_else(|| "<p>404</p>".into());
            self.scripts = has_script(&html);
            // pri písaní (rovnaká adresa) ostane posunutie stránky
            let keep = (self.key.0 == url).then(|| self.doc.as_ref().map(|d| d.viewport_scroll())).flatten();
            let net = Arc::new(Net { reader: reader.clone(), pending: self.pending.clone(), done: self.done.clone() });
            let mut doc = HtmlDocument::from_html(
                &html,
                DocumentConfig {
                    viewport: Some(Viewport::new(w, h, scale, ColorScheme::Light)),
                    base_url: Some(url.to_string()),
                    net_provider: Some(net),
                    style_threading: StyleThreading::Sequential,
                    ..Default::default()
                },
            );
            doc.handle_messages();
            doc.resolve(0.0);
            if let Some(s) = keep {
                doc.set_viewport_scroll(s);
            }
            self.doc = Some(doc);
            self.key = (url.to_string(), ver);
            self.vp = vp;
            self.dirty = true;
        } else if self.vp != vp {
            if let Some(d) = self.doc.as_mut() {
                d.set_viewport(Viewport::new(w, h, scale, ColorScheme::Light));
                d.resolve(0.0);
            }
            self.vp = vp;
            self.dirty = true;
        }
        if self.dirty {
            self.fixed = self.doc.as_ref().is_none_or(has_fixed);
        }
        // dorazili súbory z webu (štýly, obrázky) → znova rozložiť
        let done = self.done.load(Ordering::Relaxed);
        if done != self.seen_done {
            self.seen_done = done;
            if let Some(d) = self.doc.as_mut() {
                d.handle_messages();
                d.resolve(0.0);
                self.fixed = has_fixed(d);
                self.dirty = true;
            }
        }
    }

    // výrez textúry, ktorý je práve na obrazovke (pás je vyšší ako panel)
    pub fn uv(&self) -> egui::Rect {
        let Some(d) = self.doc.as_ref() else { return egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)) };
        let (top, bh) = self.band;
        if bh <= 0.0 {
            return egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        }
        let vh = self.vp.1 as f64 / d.viewport().hidpi_scale as f64;
        let y0 = ((d.viewport_scroll().y - top) / bh).clamp(0.0, 1.0) as f32;
        let y1 = (y0 + (vh / bh) as f32).min(1.0);
        egui::Rect::from_min_max(egui::pos2(0.0, y0), egui::pos2(1.0, y1))
    }

    pub fn loading(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0
    }

    // textúra stránky (prekreslí sa len po zmene)
    pub fn texture(&mut self, ctx: &egui::Context) -> Option<egui::TextureId> {
        if self.dirty {
            self.dirty = false;
            let (w, h, _) = self.vp;
            let fixed = self.fixed;
            let doc = self.doc.as_mut()?;
            let sc = doc.viewport().hidpi_scale as f64;
            // pás nad aj pod obrazovkou (najviac 2048 px – strop textúry v egui); s fixed prvkami len obrazovka
            let bh = if fixed || !self.want_band { h } else { (h * 2).min(2048).max(h) };
            self.want_band = false;
            let scroll = doc.viewport_scroll();
            let top = if bh > h { (scroll.y - (bh - h) as f64 / sc / 2.0).max(0.0) } else { scroll.y };
            doc.set_viewport_scroll(blitz_dom::Point { x: scroll.x, y: top });
            let r = self.renderer.get_or_insert_with(|| VelloCpuImageRenderer::new(w, bh));
            r.resize(w, bh);
            let mut buf = Vec::with_capacity((w * bh * 4) as usize);
            r.render_to_vec(|s| blitz_paint::paint_scene(s, doc, sc, w, bh, 0, 0), &mut buf);
            doc.set_viewport_scroll(scroll);
            self.band = (top, bh as f64 / sc);
            let img = egui::ColorImage::from_rgba_premultiplied([w as usize, bh as usize], &buf);
            drop(buf);
            match self.tex.as_mut() {
                Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                None => self.tex = Some(ctx.load_texture("flux-html-view", img, egui::TextureOptions::LINEAR)),
            }
            self.used = Instant::now();
        } else if self.renderer.is_some() && self.used.elapsed().as_secs() >= 3 {
            // nečinnosť: vykresľovač (vlastný obraz stránky) preč, pamäť späť systému
            self.renderer = None;
            crate::mem::release_heap();
        }
        self.tex.as_ref().map(|t| t.id())
    }

    pub fn scroll(&mut self, dx: f32, dy: f32) {
        if let Some(d) = self.doc.as_mut() {
            // blitz otáča znamienko sám: kladné = obsah ide dole (rolovanie hore), ako delta kolieska v egui
            let x0 = d.viewport_scroll().x;
            if d.scroll_viewport_by_has_changed(dx as f64, dy as f64) {
                // v nakreslenom páse sa len posunie obrázok (uv) – bez prekreslenia
                let s = d.viewport_scroll();
                let vh = self.vp.1 as f64 / d.viewport().hidpi_scale as f64;
                let (top, bh) = self.band;
                if self.fixed || s.x != x0 || s.y < top || s.y + vh > top + bh {
                    self.dirty = true;
                    self.want_band = true;
                }
            }
        }
    }

    // myš nad stránkou (CSS px): :hover štýly a kurzor nad odkazom
    pub fn hover(&mut self, x: f32, y: f32) {
        let Some(d) = self.doc.as_mut() else { return };
        if d.set_hover_to(x, y) {
            d.resolve(0.0);
            self.dirty = true;
        }
        self.link_hover = d.get_hover_node_id().is_some_and(|id| ancestors(d, id).any(|n| is_a(n) && n.attr(local_name!("href")).is_some()));
    }

    pub fn click(&mut self, x: f32, y: f32) -> Click {
        let Some(d) = self.doc.as_ref() else { return Click::None };
        let Some(hit) = d.hit(x, y) else { return Click::None };
        // odkaz má prednosť; inak riadok zdroja najbližšieho prvku s data-flux-l
        if let Some(href) = ancestors(d, hit.node_id).find(|n| is_a(n)).and_then(|n| n.attr(local_name!("href"))) {
            if href.starts_with('#') || href.starts_with("javascript:") {
                return Click::None;
            }
            if let Ok(u) = d.base_url().join(href) {
                let local = u.host_str() == Some("127.0.0.1") && u.port() == d.base_url().port();
                return if local { Click::Go(u.to_string()) } else { Click::External(u.to_string()) };
            }
        }
        let line = ancestors(d, hit.node_id).find_map(|n| n.attr(blitz_dom::LocalName::from("data-flux-l")).and_then(|v| v.parse::<usize>().ok()));
        line.map(Click::Line).unwrap_or(Click::None)
    }
}

// position: fixed / sticky niekde na stránke (lepiaca hlavička…) – taká stránka sa pri rolovaní kreslí celá
fn has_fixed(d: &HtmlDocument) -> bool {
    use style::computed_values::position::T as P;
    let mut stack = vec![d.root_node().id];
    let mut n = 0;
    while let Some(id) = stack.pop() {
        n += 1;
        if n > 20_000 {
            return true;
        }
        let Some(node) = d.get_node(id) else { continue };
        if node.primary_styles().is_some_and(|s| matches!(s.clone_position(), P::Fixed | P::Sticky)) {
            return true;
        }
        stack.extend(node.children.iter().copied());
    }
    false
}

fn is_a(n: &blitz_dom::Node) -> bool {
    n.element_data().is_some_and(|e| e.name.local == local_name!("a"))
}

fn ancestors<'a>(d: &'a blitz_dom::BaseDocument, id: blitz_dom::NodeId) -> impl Iterator<Item = &'a blitz_dom::Node> + 'a {
    let mut cur = d.get_node(id);
    std::iter::from_fn(move || {
        let n = cur?;
        cur = n.parent.and_then(|p| d.get_node(p));
        Some(n)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn htmlview_scripts() {
        assert!(!has_script("<p>hi</p>"));
        assert!(has_script("<script src=\"a.js\"></script>"));
        assert!(has_script("<script>alert(1)</script>"));
        assert!(!has_script("<script>  </script>"));
    }
}
