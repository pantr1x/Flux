// Live Server vedľa kódu (panel Live Servera v Electron Fluxe): vpravo na karte lišta s adresou,
// veľkosťou zariadenia, obnovením a „Otvoriť v prehliadači“ a pod ňou stránka. Na Windows je to
// vložený WebView2 (wry, podokno Fluxu); inde karta s odkazom do prehliadača.
use super::App;
use crate::i18n::t;
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Rect, Sense, Stroke};

pub struct Preview {
    pub url: String,
    device: usize, // 0 = celá šírka, 1 = tablet, 2 = mobil
    pub width: f32,
    #[cfg(windows)]
    view: Option<wry::WebView>,
    #[cfg(windows)]
    failed: bool,
    #[cfg(windows)]
    cur: std::sync::Arc<std::sync::Mutex<String>>, // adresa, na ktorej je stránka práve (odkazy)
    #[cfg(windows)]
    go: std::sync::Arc<std::sync::Mutex<Option<String>>>, // odkaz z nového okna (target=_blank) → načítať v paneli
    #[allow(dead_code)] // len Windows (čo je práve načítané vo WebView2)
    shown_url: String,
    reload: bool,
    pub body: Option<Rect>, // kde má byť stránka (v bodoch egui) – nastaví preview_ui
    #[allow(dead_code)]
    covered: bool, // minulý snímok bola stránka zakrytá
}

// (názov, šírka stránky v px; 0 = celý panel)
const DEVICES: [(&str, f32); 3] = [("Full", 0.0), ("Tablet", 768.0), ("Phone", 390.0)];

impl Preview {
    pub fn new(url: String) -> Self {
        Preview {
            url,
            device: 0,
            width: 0.0,
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
            body: None,
            covered: false,
        }
    }

    // vložená stránka funguje (Windows + WebView2)
    pub fn embedded(&self) -> bool {
        #[cfg(windows)]
        {
            !self.failed
        }
        #[cfg(not(windows))]
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
            Some(p) => p.url = url,
            None => {
                let mut p = Preview::new(url.clone());
                // bez vloženého zobrazenia otvoriť aj prehliadač
                if !p.embedded() {
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
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), 40.0));
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
        #[cfg(windows)]
        let shown = pv.cur.lock().map(|c| c.clone()).unwrap_or_default();
        #[cfg(not(windows))]
        let shown = String::new();
        let addr = if shown.is_empty() { pv.url.clone() } else { shown }.trim_start_matches("http://").to_string();
        widgets::text(ui, pos2(bar.left() + 26.0, cy), Align2::LEFT_CENTER, &addr, theme::ui(12.0), p.text2, (rx - bar.left() - 34.0).max(20.0));
        // stránka (šírka podľa zariadenia, v strede)
        let area = Rect::from_min_max(pos2(rect.left() + 1.0, bar.bottom() + 1.0), rect.max);
        let dw = DEVICES[pv.device].1;
        let body = if dw > 0.0 && dw < area.width() { Rect::from_center_size(pos2(area.center().x, area.center().y), vec2(dw, area.height())) } else { area };
        if dw > 0.0 {
            ui.painter().rect_filled(area, 0.0, p.card2);
        }
        pv.body = Some(body);
        if !pv.embedded() {
            // bez vloženého zobrazenia: karta s odkazom
            let c = body.center();
            widgets::icon_at(ui, pos2(c.x, c.y - 44.0), 26.0, "globe", p.text3);
            ui.painter().text(pos2(c.x, c.y - 10.0), Align2::CENTER_CENTER, t("Live Server"), theme::bold(15.0), p.text);
            ui.painter().text(pos2(c.x, c.y + 12.0), Align2::CENTER_CENTER, t("Open the page in the browser – it reloads when you save (F5)"), theme::ui(12.0), p.text3);
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
        let covered = self.settings.is_some() || self.palette.is_some() || self.ask.is_some() || self.tour.is_some() || self.intro.is_some() || self.start || egui::Popup::is_any_open(ctx);
        #[cfg(windows)]
        {
            let zoom = ctx.zoom_factor();
            let Some(pv) = self.preview.as_mut() else { return };
            // zbalené okno: WebView2 zavrieť (jeho procesy berú desiatky MB), po obnovení sa vytvorí znova
            if ctx.input(|i| i.viewport().minimized.unwrap_or(false)) {
                pv.view = None;
                return;
            }
            let Some(b) = pv.body else { return };
            let bounds = wry::Rect {
                position: wry::dpi::LogicalPosition::new((b.left() * zoom) as f64, (b.top() * zoom) as f64).into(),
                size: wry::dpi::LogicalSize::new((b.width() * zoom).max(1.0) as f64, (b.height() * zoom).max(1.0) as f64).into(),
            };
            if pv.view.is_none() && !pv.failed {
                // bez zobratia klávesnice – inak by nešlo písať v editore ani hľadať
                // odkazy: svoje stránky sa otvoria v paneli (aj target=_blank), cudzie webové adresy v prehliadači
                let origin = pv.url.split('/').take(3).collect::<Vec<_>>().join("/");
                let (cur, go) = (pv.cur.clone(), pv.go.clone());
                let (o1, o2) = (origin.clone(), origin);
                let nav = move |u: String| {
                    if u.starts_with(&o1) || !u.starts_with("http") {
                        if let Ok(mut c) = cur.lock() {
                            *c = u;
                        }
                        true
                    } else {
                        flux_core::settings::open_external(&u);
                        false
                    }
                };
                let neww = move |u: String, _f: wry::NewWindowFeatures| {
                    if u.starts_with(&o2) {
                        if let Ok(mut g) = go.lock() {
                            *g = Some(u);
                        }
                    } else if u.starts_with("http") {
                        flux_core::settings::open_external(&u);
                    }
                    wry::NewWindowResponse::Deny
                };
                match wry::WebViewBuilder::new()
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
                // odkaz z nového okna príde z vlákna WebView2 – Flux ho zistí pri najbližšom snímku
                ctx.request_repaint_after(std::time::Duration::from_millis(300));
                let _ = v.set_bounds(bounds);
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
                }
                let link = pv.go.lock().ok().and_then(|mut g| g.take());
                if let Some(u) = link {
                    let _ = v.load_url(&u);
                }
                if pv.reload {
                    pv.reload = false;
                    let _ = v.reload();
                }
            }
        }
    }
}
