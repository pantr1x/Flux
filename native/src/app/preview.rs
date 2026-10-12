// Live Server vedľa kódu (panel Live Servera v Electron Fluxe): vpravo na karte lišta s adresou,
// veľkosťou zariadenia, obnovením a „Otvoriť v prehliadači“ a pod ňou stránka. Na Windows je to
// vložený WebView2 (wry, podokno Fluxu); inde karta s odkazom do prehliadača.
use super::App;
use crate::i18n::t;
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Rect, Sense, Stroke};

pub struct Preview {
    pub url: String,           // adresa aktívnej karty
    pub follow: String,        // adresa otvoreného súboru, ktorú panel naposledy prevzal
    pub tabs: Vec<String>,         // adresy kariet (tabs[active] == url)
    pub active: usize,
    device: usize, // 0 = celá šírka, 1 = tablet, 2 = mobil
    pub width: f32,
    pub dragging: bool, // ťahá sa okraj panela (stránka sa pritom mení naživo)
    #[cfg(windows)]
    view: Option<wry::WebView>,
    #[cfg(windows)]
    failed: bool,
    #[cfg(windows)]
    cur: std::sync::Arc<std::sync::Mutex<String>>, // adresa, na ktorej je stránka práve (odkazy)
    #[cfg(windows)]
    go: std::sync::Arc<std::sync::Mutex<Vec<String>>>, // odkazy z nových okien (target=_blank) → nová karta
    #[allow(dead_code)] // len Windows (čo je práve načítané vo WebView2)
    shown_url: String,
    reload: bool,
    #[cfg(windows)]
    zoomed: f64, // zväčšenie, ktoré má WebView2 práve nastavené
    #[cfg(target_os = "linux")]
    helper: Option<crate::lpreview::Helper>,
    #[cfg(target_os = "linux")]
    pub lfail: Option<String>, // prečo náhľad vo Fluxe nejde (chýba webkit2gtk, Wayland…) – zobrazí sa karta
    pub body: Option<Rect>, // kde má byť stránka (v bodoch egui) – nastaví preview_ui
    #[allow(dead_code)]
    covered: bool, // minulý snímok bola stránka zakrytá
    away_since: Option<f64>, // od kedy je stránka zakrytá (nastavenia, hľadanie…)
    active_at: f64,          // kedy mal Flux naposledy fokus alebo vstup od používateľa
    asleep: bool,            // stránka je dočasne zavretá (šetrí pamäť), po návrate sa otvorí znova
    #[cfg(windows)]
    low_mem: bool, // WebView2 je v režime nízkej pamäte (Flux bez fokusu)
}

// názov karty: súbor z adresy (index.html pre koreň)
fn tab_title(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or("").trim_start_matches("http://").trim_start_matches("https://");
    let host = path.split('/').next().unwrap_or("");
    let rest = path.split_once('/').map(|x| x.1).unwrap_or("");
    let name = rest.rsplit('/').next().unwrap_or("");
    let local = host.starts_with("127.0.0.1") || host.starts_with("localhost:5") || host == "localhost";
    if name.is_empty() && !local && !host.is_empty() {
        // iný web: názov domény
        host.trim_start_matches("www.").into()
    } else if name.is_empty() {
        "index.html".into()
    } else {
        name.replace("%20", " ")
    }
}

// (názov, šírka stránky v px; 0 = celý panel)
// šírka −1 = „16:9“: stránka sa rozloží na 1920 px široký (stolový) displej a zmenší sa, aby sa vošla do panela
const DEVICES: [(&str, f32); 4] = [("Full", 0.0), ("Tablet", 768.0), ("Phone", 390.0), ("16:9", -1.0)];
#[allow(dead_code)] // len Windows (WebView2 zoom)
const DESKTOP_W: f32 = 1920.0;

impl Preview {
    pub fn new(url: String) -> Self {
        Preview {
            tabs: vec![url.clone()],
            follow: url.clone(),
            active: 0,
            url,
            device: 0,
            width: 0.0,
            dragging: false,
            #[cfg(windows)]
            view: None,
            #[cfg(windows)]
            failed: false,
            #[cfg(windows)]
            cur: Default::default(),
            #[cfg(windows)]
            go: Default::default(),
            shown_url: String::new(),
            reload: false,
            #[cfg(windows)]
            zoomed: 1.0,
            #[cfg(target_os = "linux")]
            helper: None,
            #[cfg(target_os = "linux")]
            lfail: None,
            body: None,
            covered: false,
            away_since: None,
            active_at: -1.0,
            asleep: false,
            #[cfg(windows)]
            low_mem: false,
        }
    }

    // vložená stránka funguje (Windows + WebView2)
    pub fn embedded(&self) -> bool {
        #[cfg(windows)]
        {
            !self.failed
        }
        #[cfg(target_os = "linux")]
        {
            self.lfail.is_none()
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            false
        }
    }
}

impl App {
    // otvorí panel pre aktívny súbor (spustí Live Server, ak nebeží)
    pub(super) fn open_preview(&mut self) {
        let Some(url) = self.live_url() else { return };
        match &mut self.preview {
            Some(p) => {
                p.url = url.clone();
                let a = p.active.min(p.tabs.len().saturating_sub(1));
                if let Some(t) = p.tabs.get_mut(a) {
                    *t = url;
                }
            }
            None => {
                let mut p = Preview::new(url.clone());
                // bez vloženého zobrazenia otvoriť aj prehliadač (Linux: rozhodne sa až pri prvom snímku)
                if !p.embedded() && !cfg!(target_os = "linux") {
                    flux_core::settings::open_external(&url);
                }
                p.width = 0.0;
                self.preview = Some(p);
            }
        }
    }

    pub(super) fn close_preview(&mut self) {
        self.preview = None;
        self.server = None;
    }

    // lišta panela + miesto pre stránku; rect = celý panel na karte
    pub(super) fn preview_ui(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = self.pal;
        let Some(pv) = self.preview.as_mut() else { return };
        ui.painter().vline(rect.left(), rect.y_range(), Stroke::new(1.0, p.line));
        // ---- karty stránok (odkaz s target=_blank sa otvorí v novej karte) ----
        const TABS_H: f32 = 32.0;
        let tabs_r = Rect::from_min_size(rect.min, vec2(rect.width(), TABS_H));
        ui.painter().hline(tabs_r.x_range(), tabs_r.bottom(), Stroke::new(1.0, p.line));
        let (mut switch, mut close_tab, mut add_tab) = (None, None, false);
        {
            let n = pv.tabs.len().max(1);
            let aw = (tabs_r.width() - 40.0).max(60.0);
            let tw = (aw / n as f32).clamp(70.0, 170.0);
            for (i, u) in pv.tabs.iter().enumerate() {
                let r = Rect::from_min_size(pos2(tabs_r.left() + 6.0 + i as f32 * (tw + 2.0), tabs_r.top() + 4.0), vec2(tw, TABS_H - 7.0));
                if r.right() > tabs_r.right() - 30.0 {
                    break;
                }
                let on = i == pv.active;
                let resp = ui.interact(r, ui.id().with(("pv-tab", i)), Sense::click()).on_hover_text(u);
                if on || resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(8), if on { p.active } else { p.hover });
                }
                let can_close = pv.tabs.len() > 1;
                let xr = Rect::from_center_size(pos2(r.right() - 13.0, r.center().y), vec2(18.0, 18.0));
                let tw_text = r.width() - 14.0 - if can_close { 22.0 } else { 0.0 };
                widgets::text(ui, pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, &tab_title(u), theme::ui(12.0), if on { p.text } else { p.text2 }, tw_text.max(10.0));
                let mut x_clicked = false;
                if can_close && (on || resp.hovered()) {
                    x_clicked = widgets::icon_button_at(ui, xr, "x", 11.0, &p, true).clicked();
                }
                if x_clicked {
                    close_tab = Some(i);
                } else if resp.clicked() {
                    switch = Some(i);
                }
            }
            let plus_x = (tabs_r.left() + 6.0 + pv.tabs.len() as f32 * (tw + 2.0) + 12.0).min(tabs_r.right() - 16.0);
            if widgets::icon_button_at(ui, Rect::from_center_size(pos2(plus_x, tabs_r.center().y), vec2(24.0, 24.0)), "plus", 13.0, &p, true).on_hover_text(t("New tab")).clicked() {
                add_tab = true;
            }
        }
        if let Some(i) = close_tab {
            pv.tabs.remove(i);
            if pv.active >= i && pv.active > 0 {
                pv.active -= 1;
            }
            pv.url = pv.tabs[pv.active].clone();
        } else if let Some(i) = switch {
            pv.active = i;
            pv.url = pv.tabs[i].clone();
        } else if add_tab {
            pv.tabs.push(pv.url.clone());
            pv.active = pv.tabs.len() - 1;
        }
        let bar = Rect::from_min_size(pos2(rect.left(), rect.top() + TABS_H), vec2(rect.width(), 40.0));
        let cy = bar.center().y;
        ui.painter().hline(bar.x_range(), bar.bottom(), Stroke::new(1.0, p.line));
        // vpravo: ×, prehliadač, obnoviť
        let mut rx = bar.right() - 10.0;
        let mut close = false;
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx - 13.0, cy), vec2(26.0, 26.0)), "x", 14.0, &p, true).on_hover_text(t("Stop Live Server")).clicked() {
            close = true;
        }
        rx -= 30.0;
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx - 13.0, cy), vec2(26.0, 26.0)), "external", 14.0, &p, true).on_hover_text(t("Open in browser")).clicked() {
            flux_core::settings::open_external(&pv.url);
        }
        rx -= 30.0;
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx - 13.0, cy), vec2(26.0, 26.0)), "refresh", 14.0, &p, true).on_hover_text(t("Reload")).clicked() {
            pv.reload = true;
        }
        rx -= 34.0;
        // zariadenia (celá šírka / tablet / mobil)
        for (i, (name, _)) in DEVICES.iter().enumerate().rev() {
            let w = widgets::text_w(ui, &t(name), theme::ui(11.5)) + 16.0;
            let r = Rect::from_min_size(pos2(rx - w, cy - 11.0), vec2(w, 22.0));
            let resp = ui.interact(r, ui.id().with(("pv-dev", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = pv.device == i;
            if on || resp.hovered() {
                ui.painter().rect_filled(r, CornerRadius::same(7), if on { p.active } else { p.hover });
            }
            ui.painter().text(r.center(), Align2::CENTER_CENTER, t(name), theme::ui(11.5), if on { p.text } else { p.text2 });
            if resp.clicked() {
                pv.device = i;
            }
            rx = r.left() - 2.0;
        }
        // vľavo: zelená bodka + adresa
        ui.painter().circle_filled(pos2(bar.left() + 16.0, cy), 3.5, p.green);
        let addr = pv.url.trim_start_matches("http://").to_string();
        widgets::text(ui, pos2(bar.left() + 26.0, cy), Align2::LEFT_CENTER, &addr, theme::ui(12.0), p.text2, (rx - bar.left() - 34.0).max(20.0));
        // stránka (šírka podľa zariadenia, v strede)
        // vľavo 6 px voľných – tam sa chytá okraj na zmenu šírky (WebView2 je nad egui a zakrýval by ho)
        let area = Rect::from_min_max(pos2(rect.left() + 6.0, bar.bottom() + 1.0), rect.max);
        let dw = DEVICES[pv.device].1;
        let body = if dw < 0.0 {
            // najväčší obdĺžnik 16:9, ktorý sa vojde do panela (na stred)
            let m = area.shrink(8.0);
            let w = m.width().min(m.height() * 16.0 / 9.0).max(40.0);
            Rect::from_center_size(m.center(), vec2(w, w * 9.0 / 16.0))
        } else if dw > 0.0 && dw < area.width() { Rect::from_center_size(pos2(area.center().x, area.center().y), vec2(dw, area.height())) } else { area };
        if dw != 0.0 {
            ui.painter().rect_filled(area, 0.0, p.card2);
        }
        pv.body = Some(body);
        if !pv.embedded() {
            // bez vloženého zobrazenia: karta s odkazom
            let c = body.center();
            widgets::icon_at(ui, pos2(c.x, c.y - 44.0), 26.0, "globe", p.text3);
            ui.painter().text(pos2(c.x, c.y - 10.0), Align2::CENTER_CENTER, t("Live Server"), theme::bold(15.0), p.text);
            ui.painter().text(pos2(c.x, c.y + 12.0), Align2::CENTER_CENTER, t("Open the page in the browser – it reloads when you save (F5)"), theme::ui(12.0), p.text3);
            #[cfg(target_os = "linux")]
            match pv.lfail.as_deref() {
                Some("webkit") => {
                    ui.painter().text(pos2(c.x, c.y + 90.0), Align2::CENTER_CENTER, t("To see the page here, install webkit2gtk (libwebkit2gtk-4.1)"), theme::ui(11.5), p.text3);
                }
                Some("helper") => {
                    ui.painter().text(pos2(c.x, c.y + 90.0), Align2::CENTER_CENTER, t("The preview could not start – details are in flux-preview.log in the settings folder"), theme::ui(11.5), p.text3);
                }
                _ => {}
            }
            let mut b = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_center_size(pos2(c.x, c.y + 48.0), vec2(220.0, 32.0))).layout(egui::Layout::top_down(egui::Align::Center)));
            if widgets::button(&mut b, Some("external"), &t("Open in browser"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                flux_core::settings::open_external(&pv.url);
            }
        }
        if close {
            self.close_preview();
        }
    }

    // vložený WebView2: vytvorí ho, drží ho na mieste panela a skryje, keď je nad ním niečo z egui
    #[allow(unused_variables)]
    pub(super) fn sync_preview(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let covered = self.new_file.is_some() || self.new_project.is_some() || self.whats_new.is_some() || self.color_pick.is_some() || self.settings.is_some() || self.palette.is_some() || self.ask.is_some() || self.tour.is_some() || self.intro.is_some() || self.start || egui::Popup::is_any_open(ctx);
        // stránka (WebView2 / WebKit) berie desiatky až stovky MB: keď ju nevidno, zavrie sa a po návrate sa otvorí znova –
        // zbalené okno hneď, Flux bez fokusu po 45 s, zakrytá stránka (nastavenia…) po 20 s; karty a adresy ostanú
        let (now, focused, minimized, input) = ctx.input(|i| (i.time, i.viewport().focused.unwrap_or(true), i.viewport().minimized.unwrap_or(false), !i.events.is_empty() || i.pointer.is_moving()));
        if let Some(pv) = self.preview.as_mut() {
            // „používaný“ = má fokus alebo doň ide myš/klávesnica (niektoré prostredia fokus nehlásia spoľahlivo)
            if focused || input || pv.active_at < 0.0 {
                pv.active_at = now;
            }
            if !covered {
                pv.away_since = None;
            } else if pv.away_since.is_none() {
                pv.away_since = Some(now);
            }
            let idle = now - pv.active_at > 45.0;
            pv.asleep = minimized || idle || pv.away_since.is_some_and(|t| now - t > 20.0);
            if (covered || !focused) && !pv.asleep {
                // bez fokusu sa nekreslí – zobudiť sa, aby sa stránka po limite naozaj zavrela
                ctx.request_repaint_after(std::time::Duration::from_secs(5));
            }
        }
        #[cfg(windows)]
        {
            let zoom = ctx.zoom_factor();
            let Some(pv) = self.preview.as_mut() else { return };
            // nevidno ju: WebView2 zavrieť (jeho procesy berú desiatky MB), po návrate sa vytvorí znova
            if pv.asleep {
                if pv.view.take().is_some() {
                    pv.shown_url.clear();
                }
                return;
            }
            // bez fokusu: WebView2 uvoľní, čo sa dá (MemoryUsageTargetLevel Low), kým sa celkom nezavrie
            if let Some(v) = &pv.view {
                use wry::WebViewExtWindows;
                if pv.low_mem == focused {
                    pv.low_mem = !focused;
                    let _ = v.set_memory_usage_level(if focused { wry::MemoryUsageLevel::Normal } else { wry::MemoryUsageLevel::Low });
                }
            }
            let Some(b) = pv.body else { return };
            let bounds = wry::Rect {
                position: wry::dpi::LogicalPosition::new((b.left() * zoom) as f64, (b.top() * zoom) as f64).into(),
                size: wry::dpi::LogicalSize::new((b.width() * zoom).max(1.0) as f64, (b.height() * zoom).max(1.0) as f64).into(),
            };
            if pv.view.is_none() && !pv.failed {
                // bez zobratia klávesnice – inak by nešlo písať v editore ani hľadať
                // odkazy: target=_blank aj odkaz na iný web sa otvorí v novej karte panela (nie v prehliadači)
                let origin = pv.url.split('/').take(3).collect::<Vec<_>>().join("/");
                let go = pv.go.clone();
                let (go2, cur2) = (go.clone(), pv.cur.clone());
                let nav = move |u: String| {
                    let here = cur2.lock().map(|c| c.clone()).unwrap_or_default();
                    let base = if here.is_empty() { origin.clone() } else { here.split('/').take(3).collect::<Vec<_>>().join("/") };
                    if here.is_empty() || !u.starts_with("http") || u.starts_with(&base) {
                        if let Ok(mut c) = cur2.lock() {
                            *c = u;
                        }
                        true
                    } else {
                        // iná doména: stránka karty ostane, odkaz ide do novej karty
                        if let Ok(mut g) = go2.lock() {
                            g.push(u);
                        }
                        false
                    }
                };
                let neww = move |u: String, _f: wry::NewWindowFeatures| {
                    if u.starts_with("http") {
                        if let Ok(mut g) = go.lock() {
                            g.push(u);
                        }
                    }
                    wry::NewWindowResponse::Deny
                };
                use wry::WebViewBuilderExtWindows;
                // štíhlejší WebView2: jeden proces pre stránky, bez prekladača, synchronizácie a služieb na pozadí
                // (predvolené argumenty wry sa musia zopakovať – tento zoznam ich nahrádza)
                let args = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,Translate,AutofillServerCommunication,OptimizationHints,MediaRouter,SpareRendererForSitePerProcess,msEdgeShoppingAssistant \
                            --renderer-process-limit=1 --disable-background-networking --disable-component-update --disable-sync --js-flags=--max-old-space-size=128";
                match wry::WebViewBuilder::new()
                    .with_additional_browser_args(args)
                    .with_url(&pv.url)
                    .with_bounds(bounds)
                    .with_focused(false)
                    .with_navigation_handler(nav)
                    .with_new_window_req_handler(neww)
                    .build_as_child(frame)
                {
                    Ok(v) => {
                        let _ = v.focus_parent();
                        pv.shown_url = pv.url.clone();
                        pv.zoomed = 1.0; // nový WebView2 začína na 100 %
                        pv.low_mem = false;
                        pv.view = Some(v);
                    }
                    Err(e) => {
                        eprintln!("WebView2: {e}");
                        pv.failed = true;
                        flux_core::settings::open_external(&pv.url);
                    }
                }
            }
            if let Some(v) = &pv.view {
                // odkaz z nového okna a poloha myši v stránke prídu z iných vlákien – Flux ich zistí pri najbližšom snímku
                ctx.request_repaint_after(std::time::Duration::from_millis(80));
                let _ = v.set_bounds(bounds);
                // 16:9: stránka sa rozloží na 1920 px a zmenší na veľkosť panela (WebView2 zoom), inak 100 %
                let want = if pv.device == 3 { (b.width() * zoom) as f64 / DESKTOP_W as f64 } else { 1.0 };
                if (want - pv.zoomed).abs() > 0.002 {
                    pv.zoomed = want;
                    let _ = v.zoom(want);
                }
                let _ = v.set_visible(!covered);
                // klik kamkoľvek do Fluxu (mimo stránky) alebo zakrytá stránka → klávesnica späť Fluxu;
                // WebView2 je samostatné okno a inak by si písanie nechal
                let press_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !b.contains(p)));
                if press_outside || (covered && !pv.covered) {
                    let _ = v.focus_parent();
                }
                pv.covered = covered;
                if pv.shown_url != pv.url {
                    pv.shown_url = pv.url.clone();
                    let _ = v.load_url(&pv.url);
                    if let Ok(mut c) = pv.cur.lock() {
                        c.clear();
                    }
                } else {
                    // stránka sa v karte posunula na iný odkaz → karta a adresa ho nasledujú
                    let cur = pv.cur.lock().map(|c| c.clone()).unwrap_or_default();
                    if !cur.is_empty() && cur != pv.url {
                        pv.url = cur.clone();
                        pv.shown_url = cur.clone();
                        let a = pv.active.min(pv.tabs.len() - 1);
                        pv.tabs[a] = cur;
                    }
                }
                // odkazy z nových okien → nové karty
                let links: Vec<String> = pv.go.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
                for u in links {
                    pv.tabs.push(u.clone());
                    pv.active = pv.tabs.len() - 1;
                    pv.url = u;
                }
                if pv.reload {
                    pv.reload = false;
                    let _ = v.reload();
                }
            }
        }

        #[cfg(target_os = "linux")]
        self.sync_preview_linux(ctx, frame, covered);
    }
}

#[cfg(target_os = "linux")]
impl App {
    // Linux: pomocník flux-preview (WebKitGTK) ako podokno nášho X11 okna; príkazy cez stdin (lpreview.rs)
    fn sync_preview_linux(&mut self, ctx: &egui::Context, frame: &eframe::Frame, covered: bool) {
        use crate::lpreview::{get, Ev, Get, Helper};
        let zoom = ctx.zoom_factor();
        let Some(pv) = self.preview.as_mut() else { return };
        if pv.lfail.is_some() {
            return;
        }
        if pv.asleep {
            if pv.helper.take().is_some() {
                pv.shown_url.clear();
            }
            return;
        }
        let Some(b) = pv.body else { return };
        let bounds = [(b.left() * zoom) as f64, (b.top() * zoom) as f64, (b.width() * zoom).max(1.0) as f64, (b.height() * zoom).max(1.0) as f64];
        if pv.helper.is_none() {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let xid = match frame.window_handle().map(|h| h.as_raw()) {
                Ok(RawWindowHandle::Xlib(h)) => h.window as u64,
                Ok(RawWindowHandle::Xcb(h)) => h.window.get() as u64,
                _ => 0,
            };
            if xid == 0 {
                pv.lfail = Some("wayland".into());
                flux_core::settings::open_external(&pv.url);
                return;
            }
            match get() {
                Get::Busy => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                    return;
                }
                Get::Failed(e) => {
                    eprintln!("[flux] preview helper: {e}");
                    pv.lfail = Some("helper".into());
                    flux_core::settings::open_external(&pv.url);
                    return;
                }
                Get::Ready(path) => match Helper::spawn(&path, xid, &pv.url, bounds) {
                    Ok(h) => {
                        pv.helper = Some(h);
                        pv.shown_url = pv.url.clone();
                    }
                    Err(e) => {
                        eprintln!("[flux] preview helper: {e}");
                        pv.lfail = Some("helper".into());
                        flux_core::settings::open_external(&pv.url);
                        return;
                    }
                },
            }
        }
        let Some(h) = pv.helper.as_mut() else { return };
        if h.exited() {
            let err = h.err.lock().map(|e| e.clone()).unwrap_or_default();
            eprintln!("[flux] preview helper exited: {err}");
            // pre hlásenie chyby: čo pomocník vypísal (chýbajúca knižnica, GTK, WebKit…)
            let _ = std::fs::write(flux_core::settings::user_data().join("flux-preview.log"), format!("flux-preview exited\n{err}"));
            pv.lfail = Some(if err.contains("libwebkit") || err.contains("shared librar") { "webkit".into() } else { "helper".into() });
            pv.helper = None;
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(80));
        let want = if pv.device == 3 { (b.width() * zoom) as f64 / DESKTOP_W as f64 } else { 1.0 };
        h.state(bounds, !covered, want);
        if pv.shown_url != pv.url {
            pv.shown_url = pv.url.clone();
            h.send(serde_json::json!({ "url": pv.url }));
        }
        if pv.reload {
            pv.reload = false;
            h.send(serde_json::json!({ "reload": true }));
        }
        let evs: Vec<Ev> = h.rx.try_iter().collect();
        for ev in evs {
            match ev {
                // stránka v karte prešla na iný odkaz → karta a adresa ho nasledujú
                Ev::Nav(u) => {
                    if u != pv.url && u.starts_with("http") {
                        pv.url = u.clone();
                        pv.shown_url = u.clone();
                        let a = pv.active.min(pv.tabs.len().saturating_sub(1));
                        if let Some(t) = pv.tabs.get_mut(a) {
                            *t = u;
                        }
                    }
                }
                Ev::Tab(u) => {
                    pv.tabs.push(u.clone());
                    pv.active = pv.tabs.len() - 1;
                    pv.url = u;
                }
            }
        }
    }
}
