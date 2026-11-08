// Rozloženie ako vo VS Code: úzky pás ikon (Activity Bar) a vedľa neho bočný panel so zobrazením
// – Prieskumník (projekt ako zbaliteľná sekcia s Nový súbor / Nový priečinok / Obnoviť / Zbaliť,
// nové súbory sa píšu priamo do stromu) a Projekty (pripnuté, zoznam, Nový projekt).
use super::{chrome, drag_ghost, drop_ok, name_field, rename_field, short_dir, App, Rn};
use crate::i18n::t;
use crate::theme::{self, GAP, TOP_H};
use crate::widgets::{self, Lead};
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind};
use flux_core::fsops;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;

pub const ACT_W: f32 = 48.0;
const SEC_H: f32 = 26.0;
const TREE_H: f32 = 24.0; // riadok stromu (VS Code má 22)

// zobrazenia v páse ikon: id, ikona, popis
// (hľadanie je pole v hornej lište, projekty sú sekcia Prieskumníka – ako VS Code)
const VIEWS: [(&str, &str, &str, &str); 2] = [("explorer", "file", "Explorer", "Ctrl+Shift+E"), ("plugins", "puzzle", "Plugins", "Ctrl+Shift+X")];
// tlačidlo ľavého pásu: zaoblená plocha pri prechode, ikona sa pri najazdení/kliknutí „odrazí“ (pružina),
// pri stlačení sa jemne zmenší; vráti (odpoveď, stred, veľkosť a farbu ikony)
fn bar_button(ui: &egui::Ui, b: Rect, id: egui::Id, active: bool, anim: bool, p: &crate::theme::Pal) -> (egui::Response, egui::Pos2, f32, egui::Color32) {
    let ctx = ui.ctx().clone();
    let resp = ui.interact(b, id, Sense::click());
    let t = if anim { 0.14 } else { 0.0 };
    let hk = ctx.animate_bool_with_time(id.with("h"), resp.hovered(), t);
    let ak = ctx.animate_bool_with_time(id.with("a"), active, t);
    let pk = ctx.animate_bool_with_time(id.with("p"), resp.is_pointer_button_down_on(), if anim { 0.07 } else { 0.0 });
    let now = ctx.input(|i| i.time);
    // začiatok „odrazu“: najazdenie alebo kliknutie
    let kick: f64 = ctx.data_mut(|d| {
        let was = d.get_temp::<bool>(id.with("was")).unwrap_or(false);
        d.insert_temp(id.with("was"), resp.hovered());
        if (resp.hovered() && !was) || resp.clicked() {
            d.insert_temp(id.with("kick"), now);
        }
        d.get_temp::<f64>(id.with("kick")).unwrap_or(-10.0)
    });
    let dt = (now - kick) as f32;
    let bounce = if anim && dt < 0.7 {
        ctx.request_repaint();
        (-dt * 7.0).exp() * (dt * 24.0).sin() * 0.16
    } else {
        0.0
    };
    let pill = Rect::from_center_size(b.center(), vec2(38.0, 38.0));
    if hk > 0.0 || ak > 0.0 {
        ui.painter().rect_filled(pill, CornerRadius::same(12), p.hover.gamma_multiply(hk.max(ak * 0.55)));
    }
    if ak > 0.0 {
        ui.painter().rect_filled(pill, CornerRadius::same(12), p.accent.gamma_multiply(0.10 * ak));
    }
    let scale = (1.0 + 0.08 * hk + bounce) * (1.0 - 0.1 * pk);
    let center = b.center() - vec2(0.0, 1.5 * hk * (1.0 - pk));
    let c = p.text3.lerp_to_gamma(p.text, hk.max(ak));
    let c = if ak > 0.5 { c.lerp_to_gamma(p.accent, 0.35 * ak) } else { c };
    (resp, center, scale, c)
}

// bublina s názvom a skratkou (klávesy ako malé čiapočky)
fn key_tip(ui: &mut egui::Ui, name: &str, key: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(name).strong());
        if !key.is_empty() {
            ui.add_space(6.0);
            for part in key.split('+') {
                ui.label(egui::RichText::new(part).monospace().size(11.0).background_color(ui.visuals().widgets.inactive.bg_fill));
            }
        }
    });
}

impl App {
    pub(super) fn load_side(&mut self) {
        if self.side_view.is_empty() {
            self.side_view = "explorer".into();
        }
        if self.side_closed.is_none() {
            let set: HashSet<String> = self.get("sideClosed").as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
            self.side_closed = Some(set);
        }
    }

    // prepne zobrazenie; klik na už otvorené zobrazenie skryje bočný panel (ako VS Code)
    pub(super) fn show_view(&mut self, id: &str, ctx: &egui::Context) {
        self.load_side();
        match id {
            "search" => self.act("search", ctx),
            "plugins" => self.open_settings("plugins", ctx),
            _ => {
                if self.side_open && self.side_view == id {
                    self.side_open = false;
                } else {
                    self.side_open = true;
                    if self.side_view != id {
                        self.side_view = id.to_string();
                        let v = id.to_string();
                        self.update_settings(|o| {
                            o.insert("sideView".into(), json!(v));
                        });
                    }
                }
            }
        }
    }

    pub(super) fn sec_open(&self, id: &str) -> bool {
        !self.side_closed.as_ref().is_some_and(|s| s.contains(id))
    }

    pub(super) fn toggle_sec(&mut self, id: &str) {
        let set = self.side_closed.get_or_insert_with(HashSet::new);
        if !set.remove(id) {
            set.insert(id.to_string());
        }
        let list: Vec<Value> = set.iter().map(|s| json!(s)).collect();
        self.update_settings(|o| {
            o.insert("sideClosed".into(), Value::Array(list));
        });
    }

    // ---------- pás ikon (Activity Bar) ----------
    pub(super) fn activity_bar(&mut self, root: &mut egui::Ui) {
        self.load_side();
        let p = self.pal;
        let right = self.get("sidePos").as_str() == Some("right");
        (if right { egui::Panel::right("activity") } else { egui::Panel::left("activity") })
            .resizable(false)
            .exact_size(ACT_W)
            .show_separator_line(false)
            .frame(Frame::new().fill(self.base_fill()))
            .show(root, |ui| {
                let r = ui.max_rect();
                if p.layer[0].a() > 0 {
                    let k = if self.rounded { 8 } else { 0 };
                    let cr = if right { CornerRadius { ne: k, se: k, ..Default::default() } } else { CornerRadius { nw: k, sw: k, ..Default::default() } };
                    ui.ctx().layer_painter(ui.layer_id()).rect_filled(r, cr, p.layer[0]);
                }
                chrome::drag_area(ui, r, "activity");
                // jemná čiara k bočnému panelu
                if self.side_open {
                    let x = if right { r.left() + 0.5 } else { r.right() - 0.5 };
                    ui.painter().vline(x, r.top() + 10.0..=r.bottom() - 10.0, Stroke::new(1.0, p.line.gamma_multiply(0.7)));
                }
                let cx = r.center().x;
                // logo = domov Fluxu (vpravo by bolo pod − □ ×, tam ho nahrádza ponuka ☰)
                let mut y = r.top();
                if !right {
                    let brand = Rect::from_center_size(pos2(cx, r.top() + TOP_H / 2.0), vec2(36.0, 36.0));
                    self.tour_rects.insert("brand", brand);
                    let resp = ui.interact(brand, ui.id().with("brand"), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(brand, CornerRadius::same(10), p.hover);
                    }
                    widgets::brand_mark(ui, Rect::from_center_size(brand.center(), vec2(22.0, 22.0)), &p);
                    if resp.on_hover_text(t("Home")).clicked() {
                        let now = ui.ctx().input(|i| i.time);
                        self.open_start(now);
                    }
                }
                y += TOP_H + 2.0;
                let ctx = ui.ctx().clone();
                for (id, icon, tip, key) in VIEWS {
                    let b = Rect::from_min_size(pos2(r.left(), y), vec2(ACT_W, 46.0));
                    let active = self.side_open && self.side_view == id;
                    let bid = ui.id().with(("act", id));
                    let (resp, center, scale, c) = bar_button(ui, b, bid, active, self.anim_on(), &p);
                    let resp = resp.on_hover_ui(|ui| key_tip(ui, &t(tip), key)).on_hover_cursor(egui::CursorIcon::PointingHand);
                    let ak = ui.ctx().animate_bool_with_time(bid.with("a"), active, if self.anim_on() { 0.14 } else { 0.0 });
                    if ak > 0.0 {
                        // značka aktívneho zobrazenia pri okraji (ako VS Code), v akcentovej farbe, so žiarou
                        let h = 22.0 * ak;
                        let bx = if right { b.right() - 3.0 } else { b.left() + 0.5 };
                        let bar = Rect::from_min_size(pos2(bx, b.center().y - h / 2.0), vec2(2.5, h));
                        ui.painter().rect_filled(bar.expand2(vec2(2.0, 3.0)), CornerRadius::same(3), p.accent.gamma_multiply(0.18 * ak));
                        ui.painter().rect_filled(bar, CornerRadius::same(2), p.accent);
                    }
                    widgets::icon_scaled(ui, center, 21.0, scale, icon, c);
                    if id == "explorer" {
                        self.tour_rects.insert("files", b);
                    }
                    if resp.clicked() {
                        self.show_view(id, &ctx);
                    }
                    y += 46.0;
                }
                // ikony z pluginov (flux.activityBar.add), pod jemnou čiarou
                let limit = r.bottom() - GAP - 4.0 - 80.0;
                let mut plug_click = None;
                if !self.plug.acts.is_empty() && y + 40.0 <= limit {
                    ui.painter().hline(r.left() + 14.0..=r.right() - 14.0, y + 4.0, Stroke::new(1.0, p.line.gamma_multiply(0.7)));
                    y += 9.0;
                }
                for a in &self.plug.acts {
                    if y + 40.0 > limit {
                        break;
                    }
                    let b = Rect::from_min_size(pos2(r.left(), y), vec2(ACT_W, 44.0));
                    let (resp, center, scale, c) = bar_button(ui, b, ui.id().with(("plug-act", a.plugin.as_str(), a.k.as_str())), false, self.anim_on(), &p);
                    let resp = resp.on_hover_ui(|ui| key_tip(ui, &a.title, &a.key_text)).on_hover_cursor(egui::CursorIcon::PointingHand);
                    if a.icon.trim_start().starts_with("<svg") {
                        // vlastné SVG pluginu, zafarbené ako ostatné ikony
                        let uri = format!("bytes://plugact-{}-{}.svg", a.plugin, a.k);
                        let img = egui::Image::from_bytes(uri, a.icon.as_bytes().to_vec());
                        if let Some(tid) = img.load_for_size(ui.ctx(), vec2(19.0, 19.0)).ok().and_then(|t| t.texture_id()) {
                            ui.painter().image(tid, Rect::from_center_size(center, vec2(19.0, 19.0) * scale), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), c);
                        }
                    } else {
                        widgets::icon_scaled(ui, center, 19.0, scale, &a.icon, c);
                    }
                    if resp.clicked() {
                        plug_click = Some((a.plugin.clone(), a.k.clone()));
                    }
                    y += 44.0;
                }
                if let Some((pl, k)) = plug_click {
                    self.plug_call(&pl, &format!("__click({})", json!(k)));
                }
                // dole: ponuka ☰ a nastavenia (svetlá/tmavá je v Nastaveniach → Vzhľad a v ponuke)
                let mut by = r.bottom() - GAP - 4.0;
                let set_r = Rect::from_min_max(pos2(r.left() + 6.0, by - 36.0), pos2(r.right() - 6.0, by));
                self.tour_rects.insert("settings", set_r);
                if widgets::icon_button_at(ui, set_r, "settings", 20.0, &p, true).on_hover_text(t("Settings")).clicked() {
                    self.open_settings("general", &ctx);
                }
                by -= 40.0;
                let menu_r = Rect::from_min_max(pos2(r.left() + 6.0, by - 36.0), pos2(r.right() - 6.0, by));
                let menu = widgets::icon_button_at(ui, menu_r, "menu", 19.0, &p, true).on_hover_text(t("Menu"));
                self.app_menu(&menu);
            });
    }

    // ---------- bočný panel ----------
    pub(super) fn sidebar(&mut self, root: &mut egui::Ui) {
        self.load_side();
        let p = self.pal;
        let right = self.get("sidePos").as_str() == Some("right");
        // šírka sa dá meniť ťahaním okraja (sideWidth)
        let w = self.side_drag.unwrap_or_else(|| self.get("sideWidth").as_f64().unwrap_or(crate::theme::SIDE_W as f64) as f32).clamp(180.0, 600.0);
        let resp = (if right { egui::Panel::right("side") } else { egui::Panel::left("side") })
            .resizable(false)
            .exact_size(w)
            .show_separator_line(false)
            .frame(Frame::new().fill(self.base_fill()).inner_margin(Margin { left: 6, right: 6, top: 0, bottom: GAP as i8 }))
            .show(root, |ui| {
                if p.layer[0].a() > 0 {
                    let r = ui.max_rect();
                    let full = Rect::from_min_max(pos2(r.left() - 6.0, r.top()), pos2(r.right() + 6.0, r.bottom() + GAP));
                    ui.ctx().layer_painter(ui.layer_id()).rect_filled(full, CornerRadius::ZERO, p.layer[0]);
                }
                self.side_head(ui, &t("Explorer").to_uppercase(), !right);
                self.explorer_view(ui);
            });
        // úchyt na okraji panela
        let r = resp.response.rect;
        let x = if right { r.left() } else { r.right() };
        let grip = Rect::from_min_max(pos2(x - 3.0, r.top() + TOP_H), pos2(x + 3.0, r.bottom()));
        let g = root.interact(grip, egui::Id::new("side-grip"), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if g.hovered() || g.dragged() {
            root.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            root.painter().vline(x, grip.y_range(), Stroke::new(2.0, p.accent.gamma_multiply(0.6)));
        }
        if g.dragged() {
            let d = g.drag_delta().x;
            self.side_drag = Some((w + if right { -d } else { d }).clamp(180.0, 600.0));
        }
        if g.drag_stopped() {
            if let Some(v) = self.side_drag.take() {
                self.update_settings(|o| {
                    o.insert("sideWidth".into(), json!(v.round()));
                });
            }
        }
        if g.double_clicked() {
            self.side_drag = None;
            self.update_settings(|o| {
                o.remove("sideWidth");
            });
        }
    }

    // výška zoznamu (PROJECTS / FILES) – ťahá sa za čiaru pod ním, uloží sa do nastavení
    fn list_height(&mut self, ui: &mut egui::Ui, key: &str, cur: f32, row_h: f32, max_rows: usize) -> f32 {
        let p = self.pal;
        let y = ui.cursor().top();
        let grip = Rect::from_min_max(pos2(ui.max_rect().left(), y - 2.0), pos2(ui.max_rect().right(), y + 5.0));
        let g = ui.interact(grip, ui.id().with(("list-grip", key)), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeVertical);
        let id = egui::Id::new(("list-h", key));
        let mut h = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(cur);
        if g.hovered() || g.dragged() {
            ui.painter().hline(grip.x_range(), y + 1.0, Stroke::new(2.0, p.accent.gamma_multiply(0.6)));
        }
        if g.dragged() {
            h = (h + g.drag_delta().y).clamp(row_h, row_h * max_rows.max(1) as f32);
            ui.ctx().data_mut(|d| d.insert_temp(id, h));
        }
        if g.drag_stopped() {
            let k = key.to_string();
            self.update_settings(|o| {
                o.insert(k, json!(h.round()));
            });
            ui.ctx().data_mut(|d| d.remove::<f32>(id));
        }
        h
    }

    // hlavička zobrazenia: názov („EXPLORER“) a vpravo skrytie panela; prázdne miesto presúva okno
    fn side_head(&mut self, ui: &mut egui::Ui, title: &str, toggle: bool) {
        let p = self.pal;
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), TOP_H), Sense::hover());
        chrome::drag_area(ui, r, "side");
        let mut job = egui::text::LayoutJob::default();
        job.append(title, 0.0, egui::TextFormat { font_id: theme::bold(11.0), color: p.text2, extra_letter_spacing: 1.0, ..Default::default() });
        let g = ui.fonts_mut(|f| f.layout_job(job));
        ui.painter().galley(pos2(r.left() + 10.0, r.center().y - g.size().y / 2.0), g, p.text2);
        if toggle {
            let tr = Rect::from_center_size(pos2(r.right() - 14.0, r.center().y), vec2(28.0, 28.0));
            if widgets::icon_button_at(ui, tr, "sidebar", 16.0, &p, true).on_hover_text(t("Hide sidebar")).clicked() {
                self.side_open = false;
            }
        }
    }

    // zbaliteľná sekcia (ako VS Code): šípka, názov veľkými písmenami, vpravo tlačidlá pri prejdení myšou;
    // vráti (klik na hlavičku, index kliknutého tlačidla)
    fn sec_header(&mut self, ui: &mut egui::Ui, id: &str, title: &str, acts: &[(&str, String)], show_acts: bool) -> (bool, Option<usize>) {
        let p = self.pal;
        let open = self.sec_open(id);
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), SEC_H), Sense::click());
        let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
        if hk > 0.0 {
            ui.painter().rect_filled(r, CornerRadius::same(6), p.hover.gamma_multiply(hk * 0.6));
        }
        let c = pos2(r.left() + 11.0, r.center().y);
        if open {
            let pts = vec![pos2(c.x - 3.5, c.y - 1.5), pos2(c.x, c.y + 2.0), pos2(c.x + 3.5, c.y - 1.5)];
            ui.painter().line(pts, Stroke::new(1.5, p.text2));
        } else {
            widgets::icon_at(ui, c, 12.0, "chevron", p.text2);
        }
        let mut job = egui::text::LayoutJob::default();
        job.append(title, 0.0, egui::TextFormat { font_id: theme::bold(11.0), color: p.text, extra_letter_spacing: 0.6, ..Default::default() });
        job.wrap = egui::text::TextWrapping::truncate_at_width(r.width() - 26.0 - if show_acts { acts.len() as f32 * 24.0 } else { 0.0 });
        let g = ui.fonts_mut(|f| f.layout_job(job));
        ui.painter().galley(pos2(r.left() + 22.0, r.center().y - g.size().y / 2.0), g, p.text);
        let mut hit = None;
        if show_acts {
            let mut x = r.right() - 12.0;
            for (i, (icon, tip)) in acts.iter().enumerate().rev() {
                let b = Rect::from_center_size(pos2(x, r.center().y), vec2(22.0, 22.0));
                if widgets::icon_button_at(ui, b, icon, 14.0, &p, true).on_hover_text(tip.as_str()).clicked() {
                    hit = Some(i);
                }
                x -= 24.0;
            }
        }
        (resp.clicked() && hit.is_none(), hit)
    }

    // kam pôjde nový súbor/priečinok: vybraný priečinok, priečinok vybraného súboru, inak koreň projektu
    pub(super) fn new_target(&self) -> Option<String> {
        let ws = self.workspace()?;
        let in_ws = |p: &str| Path::new(p).starts_with(&ws);
        let pick = self.tree_focus.clone().filter(|f| in_ws(f)).or_else(|| self.tabs.get(self.active).filter(|_| !self.home).map(|t| t.path.clone()).filter(|f| in_ws(f)));
        Some(match pick {
            Some(f) if Path::new(&f).is_dir() => f,
            Some(f) => Path::new(&f).parent().map(|d| d.to_string_lossy().to_string()).filter(|d| in_ws(d)).unwrap_or(ws),
            None => ws,
        })
    }

    // nový súbor/priečinok priamo v strome (ako VS Code): pole sa ukáže v cieľovom priečinku
    pub(super) fn start_new_item(&mut self, is_dir: bool, dir: Option<String>) {
        let Some(dir) = dir.or_else(|| self.new_target()) else {
            if !is_dir {
                self.open_new_file(None);
            }
            return;
        };
        self.side_open = true;
        self.side_view = "explorer".into();
        if !self.sec_open("project") {
            self.toggle_sec("project");
        }
        self.open_dirs.insert(dir.clone());
        self.new_in = Some(dir);
        self.new_item = Some((is_dir, String::new()));
    }

    // ---------- Prieskumník ----------
    fn explorer_view(&mut self, ui: &mut egui::Ui) {
        let p = self.pal;
        self.drop_rects.clear();
        let side_hover = ui.rect_contains_pointer(ui.max_rect());
        // dole sekcia FILES (súbory otvorené mimo projektu); strom projektu vyplní zvyšok
        let n_files = self.recent_files.len();
        let files_open = self.sec_open("files");
        let row_h = 38.0f32;
        let files_h = SEC_H + if files_open { n_files.min(4) as f32 * row_h + 30.0 } else { 0.0 };
        // hore zoznam projektov (VS Code nemá zobrazenie Projekty – len nedávne priečinky; tu je to sekcia Prieskumníka)
        self.projects_section(ui, side_hover, files_h);
        let Some(w) = self.workspace() else {
            // bez otvoreného priečinka: ako VS Code „You have not yet opened a folder.“ + tlačidlá
            let (head, _) = self.sec_header(ui, "project", &t("No folder is open.").trim_end_matches('.').to_uppercase(), &[], false);
            if head {
                self.toggle_sec("project");
            }
            if self.sec_open("project") {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(t("You have not yet opened a folder.")).color(p.text2).size(12.5));
                });
                ui.add_space(8.0);
                let bw = ui.available_width() - 20.0;
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.allocate_ui(vec2(bw, 30.0), |ui| {
                        if widgets::button(ui, Some("folderOpen"), &t("Open folder"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                            if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                                self.open_folder(&d.to_string_lossy());
                            }
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.allocate_ui(vec2(bw, 30.0), |ui| {
                        if widgets::button(ui, Some("plus"), &t("New project"), p.hover, p.text, 30.0, &p).clicked() {
                            self.open_new_project();
                        }
                    });
                });
            }
            self.files_section(ui, files_h, side_hover);
            return;
        };
        // ---- projekt: zbaliteľná sekcia s menom priečinka a tlačidlami ako vo VS Code ----
        let name = widgets::file_name(&w).to_uppercase();
        let acts = [("filePlus", t("New file")), ("folderPlus", t("New folder")), ("refresh", t("Refresh")), ("collapse", t("Collapse all"))];
        let (head, hit) = self.sec_header(ui, "project", &name, &acts, side_hover);
        match hit {
            Some(0) => self.start_new_item(false, None),
            Some(1) => self.start_new_item(true, None),
            Some(2) => self.tree.clear(),
            Some(3) => {
                self.open_dirs.clear();
                self.tree_focus = None;
            }
            _ => {}
        }
        if head {
            self.toggle_sec("project");
        }
        if self.sec_open("project") {
            let h = (ui.max_rect().bottom() - files_h - ui.cursor().top() - ui.spacing().item_spacing.y * 2.0).max(60.0);
            let sk = egui::Id::new("sa-tree");
            let off = self.smooth.begin(ui.ctx(), sk, ui.layer_id(), None);
            let mut sa = egui::ScrollArea::vertical().id_salt("tree").max_height(h).auto_shrink([false, true]);
            if let Some(o) = off {
                sa = sa.vertical_scroll_offset(o);
            }
            let sout = sa.show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                // zvýraznenie otvoreného súboru kĺže k novému riadku (slideIndicator v Electron Fluxe)
                let bg = ui.painter().add(egui::Shape::Noop);
                let origin = ui.min_rect().top();
                self.tree_sel = None;
                self.tree_rows.clear();
                self.tree_ui(ui, &w, 0);
                // pri ťahaní: miesto na vytiahnutie von z priečinkov (do koreňa projektu) – až pod stromom;
                // nad ním by po začatí ťahania posunulo všetky riadky a pod kurzorom by bol iný priečinok
                if egui::DragAndDrop::has_payload_of_type::<String>(ui.ctx()) {
                    let rr = widgets::row_ex(ui, false, false, 6.0, Lead::Line("folderOpen"), &t("Move to the project folder"), None, false, &self.pal);
                    if let Some(src) = rr.dnd_hover_payload::<String>() {
                        if drop_ok(&src, &w) {
                            ui.painter().rect_stroke(rr.rect, CornerRadius::same(8), Stroke::new(1.5, self.pal.accent), StrokeKind::Inside);
                        }
                    }
                    if let Some(src) = rr.dnd_release_payload::<String>() {
                        self.move_item(&src, &w);
                    }
                }
                // prázdne miesto pod stromom: klik zruší výber (nové súbory pôjdu do koreňa), pravý klik = ponuka koreňa
                let rest = 10.0;
                let (_, eresp) = ui.allocate_exact_size(vec2(ui.available_width(), rest), Sense::click());
                if eresp.clicked() {
                    self.tree_focus = None;
                }
                self.tree_menu(&eresp, &w, true);
                if let Some(r) = self.tree_sel {
                    let t = if self.anim_on() { 0.2 } else { 0.0 };
                    let y = ui.ctx().animate_value_with_time(egui::Id::new("tree-glide"), r.top() - origin, t);
                    let gr = Rect::from_min_size(pos2(r.left(), origin + y), r.size());
                    ui.painter().set(bg, egui::Shape::rect_filled(gr, CornerRadius::same(6), self.pal.active));
                }
            });
            self.smooth.end(sk, &sout);
        }
        self.files_section(ui, files_h, side_hover);
    }

    // sekcia FILES: súbory otvorené mimo projektu – hneď pod stromom (ako sekcie vo VS Code)
    fn files_section(&mut self, ui: &mut egui::Ui, _files_h: f32, side_hover: bool) {
        let p = self.pal;
        ui.add_space(4.0);
        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top() - 2.0, Stroke::new(1.0, p.line));
        let acts = [("filePlus", t("New file"))];
        let (head, hit) = self.sec_header(ui, "files", &t("Files").to_uppercase(), &acts, side_hover);
        if head {
            self.toggle_sec("files");
        }
        if hit == Some(0) {
            let d = self.recent_files.first().and_then(|f| Path::new(f).parent().map(|d| d.to_string_lossy().to_string()));
            self.open_new_file(d);
        }
        if !self.sec_open("files") {
            return;
        }
        let mut open_f = None;
        let sk = egui::Id::new("sa-loose");
        let off = self.smooth.begin(ui.ctx(), sk, ui.layer_id(), None);
        let row_h = 38.0 + ui.spacing().item_spacing.y;
        let n = self.recent_files.len();
        let live = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new(("list-h", "filesHeight"))));
        let fh = live.or(self.get("filesHeight").as_f64().map(|v| v as f32)).unwrap_or(n.min(4) as f32 * row_h).min(n.max(1) as f32 * row_h);
        let mut sa = egui::ScrollArea::vertical().id_salt("loose").max_height(fh).auto_shrink([false, true]);
        if let Some(o) = off {
            sa = sa.vertical_scroll_offset(o);
        }
        let sout = sa.show(ui, |ui| {
            for f in &self.recent_files {
                let sel = !self.home && self.tabs.get(self.active).map(|t| &t.path == f).unwrap_or(false);
                let r = widgets::row(ui, sel, 10.0, Lead::File(&widgets::file_name(f)), &widgets::file_name(f), Some(&short_dir(f)), false, &p).on_hover_text(f);
                // voľný súbor sa dá pretiahnuť na projekt alebo priečinok v strome (presunie sa tam)
                if r.drag_started() {
                    egui::DragAndDrop::set_payload(ui.ctx(), f.clone());
                }
                if r.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    drag_ghost(ui.ctx(), &widgets::file_name(f));
                }
                if r.clicked() {
                    open_f = Some(f.clone());
                }
            }
        });
        self.smooth.end(sk, &sout);
        if n > 1 {
            self.list_height(ui, "filesHeight", sout.inner_rect.height(), row_h, n);
        }
        if let Some(f) = open_f {
            self.open_file(&f);
        }
        if self.recent_files.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(22.0);
                if ui.add(egui::Label::new(egui::RichText::new(format!("+ {}", t("New file"))).color(p.text3).size(12.5)).sense(Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.open_new_file(None);
                }
            });
        }
    }

    // ---------- sekcia PROJECTS (pripnuté navrchu, Nový projekt / Otvoriť priečinok v hlavičke) ----------
    fn projects_section(&mut self, ui: &mut egui::Ui, side_hover: bool, files_h: f32) {
        let p = self.pal;
        let ws = self.workspace();
        let mut open = None;
        let mut menu_for: Option<(egui::Response, String, bool)> = None;
        let mut rename: Option<(String, String)> = None;
        let mut projects = self.projects.clone();
        // pripnuté prvé (poradie inak ostáva)
        projects.sort_by_key(|pr| pr["pinned"].as_bool() != Some(true));
        let acts = [("plus", t("New project")), ("folderOpen", t("Open folder"))];
        self.tour_rects.insert("projects", Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), SEC_H)));
        let (head, hit) = self.sec_header(ui, "projects", &t("Projects").to_uppercase(), &acts, side_hover);
        match hit {
            Some(0) => self.open_new_project(),
            Some(1) => {
                if let Some(d) = rfd::FileDialog::new().set_title(t("Open folder")).pick_folder() {
                    self.open_folder(&d.to_string_lossy());
                }
            }
            _ => {}
        }
        if head {
            self.toggle_sec("projects");
        }
        if self.sec_open("projects") {
            // s otvoreným projektom najviac 4 celé riadky (strom má prednosť), bez neho celé miesto
            let row_h = 38.0 + ui.spacing().item_spacing.y;
            let saved = self.get("projectsHeight").as_f64().map(|v| v as f32);
            let max_h = if ws.is_some() {
                let live = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new(("list-h", "projectsHeight"))));
                live.or(saved).unwrap_or(projects.len().min(4) as f32 * row_h).min(projects.len().max(1) as f32 * row_h)
            } else {
                (ui.max_rect().bottom() - files_h - ui.cursor().top() - 150.0).max(row_h * 2.0).min(projects.len() as f32 * row_h)
            };
            let sk = egui::Id::new("sa-projects");
            let off = self.smooth.begin(ui.ctx(), sk, ui.layer_id(), None);
            let mut sa = egui::ScrollArea::vertical().id_salt("projects").max_height(max_h).auto_shrink([false, true]);
            if let Some(o) = off {
                sa = sa.vertical_scroll_offset(o);
            }
            let sout = sa.show(ui, |ui| {
                for pr in &projects {
                    self.project_row(ui, pr, ws.as_deref(), &mut open, &mut menu_for, &mut rename);
                }
            });
            self.smooth.end(sk, &sout);
            if ws.is_some() && projects.len() > 1 {
                let cur = sout.inner_rect.height();
                self.list_height(ui, "projectsHeight", cur, row_h, projects.len());
            }
            if projects.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    if ui.add(egui::Label::new(egui::RichText::new(format!("+ {}", t("New project"))).color(p.text3).size(12.5)).sense(Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.open_new_project();
                    }
                });
            }
        }
        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top() + 1.0, Stroke::new(1.0, p.line));
        ui.add_space(5.0);
        if let Some((r, d, pinned)) = menu_for {
            self.project_menu(&r, &d, pinned);
        }
        if let Some((f, n)) = rename {
            self.finish_rename(&f, &n);
        }
        if let Some(d) = open {
            if ws.as_deref() == Some(d.as_str()) {
                self.home = true;
                self.start = false;
            } else {
                self.open_folder(&d);
            }
        }
    }

    // pole na názov nového súboru/priečinka priamo v strome (v cieľovom priečinku), s ikonou podľa mena;
    // „a/b/c.py“ vytvorí aj priečinky a a b (ako VS Code)
    fn new_item_ui(&mut self, ui: &mut egui::Ui, dir: &str, depth: usize) {
        let target = self.new_in.clone().or_else(|| self.workspace()).unwrap_or_default();
        let Some((is_dir, name)) = &mut self.new_item else { return };
        if target != dir {
            return;
        }
        let p = self.pal;
        let hint = if *is_dir { t("Folder name") } else { t("File name") };
        let mut res = Rn::Editing;
        let indent = 6.0 + depth as f32 * 14.0 + if *is_dir { 16.0 } else { 20.0 };
        ui.horizontal(|ui| {
            ui.add_space(indent);
            let (ir, _) = ui.allocate_exact_size(vec2(16.0, 24.0), Sense::hover());
            let icon_r = Rect::from_center_size(ir.center(), vec2(16.0, 16.0));
            if *is_dir {
                widgets::icon(ui, Rect::from_center_size(ir.center(), vec2(15.0, 15.0)), "folder", p.text2);
            } else {
                widgets::file_icon(ui, icon_r, if name.is_empty() { "file" } else { name.as_str() });
            }
            ui.add_space(3.0);
            // Enter / klik inam potvrdí, Esc zruší (fokus sa žiada len raz – inak by Enter „zjedlo“ rename_field)
            res = name_field(ui, egui::Id::new(("new-item", dir)), name, Margin::symmetric(8, 4), false, &hint);
        });
        if matches!(res, Rn::Editing) {
            return;
        }
        let (d, n) = (*is_dir, name.trim().trim_matches(|c| c == '/' || c == '\\').to_string());
        self.new_item = None;
        self.new_in = None;
        if !matches!(res, Rn::Done) || n.is_empty() {
            return;
        }
        let parts: Vec<&str> = n.split(['/', '\\']).collect();
        if parts.iter().any(|s| s.trim().is_empty() || *s == "." || *s == "..") {
            self.status = t("Invalid name.");
            return;
        }
        let target = parts.iter().fold(Path::new(dir).to_path_buf(), |acc, s| acc.join(s.trim()));
        let target_s = target.to_string_lossy().to_string();
        if target.exists() {
            self.status = t("A file or folder with this name already exists.");
            return;
        }
        match fsops::create(&target_s, d) {
            Ok(_) => {
                self.tree.clear();
                // otvorené všetky priečinky na ceste k novej položke
                let mut cur = Path::new(dir).to_path_buf();
                self.open_dirs.insert(dir.to_string());
                for s in &parts[..parts.len() - 1] {
                    cur = cur.join(s.trim());
                    self.open_dirs.insert(cur.to_string_lossy().to_string());
                }
                self.tree_focus = Some(target_s.clone());
                if d {
                    self.open_dirs.insert(target_s);
                } else {
                    self.open_file(&target_s);
                }
            }
            Err(e) => self.status = e,
        }
    }

    pub(super) fn tree_ui(&mut self, ui: &mut egui::Ui, dir: &str, depth: usize) {
        let p = self.pal;
        self.new_item_ui(ui, dir, depth);
        let items = self.list(dir);
        for (name, path, is_dir) in items {
            let indent = 6.0 + depth as f32 * 14.0;
            if let Some((from, new)) = self.renaming.as_mut().filter(|r| r.0 == path) {
                let mut res = Rn::Editing;
                ui.horizontal(|ui| {
                    // text poľa presne nad menom v riadku (ikona ostáva vľavo)
                    ui.add_space(indent + if is_dir { 27.0 } else { 33.0 });
                    // pri súbore sa označí meno bez prípony
                    res = rename_field(ui, egui::Id::new(("rename", &path)), new, Margin::symmetric(10, 4), !is_dir);
                });
                match res {
                    Rn::Editing => {}
                    Rn::Done => {
                        let (f, n) = (from.clone(), new.clone());
                        self.renaming = None;
                        self.finish_rename(&f, &n);
                    }
                    Rn::Cancel => self.renaming = None,
                }
                continue;
            }
            let open = self.open_dirs.contains(&path);
            let active = !self.home && self.tabs.get(self.active).map(|t| t.path == path).unwrap_or(false);
            let dirty = self.tabs.iter().any(|t| t.path == path && t.dirty());
            let lead = if is_dir { Lead::Folder { open } } else { Lead::File(&name) };
            let ind = if is_dir { indent } else { indent + 20.0 };
            // aktívny riadok bez vlastnej výplne – kreslí ju kĺzavé zvýraznenie v strome
            let r = widgets::row_sized(ui, TREE_H, active, false, ind, lead, &name, None, false, &p);
            if active {
                self.tree_sel = Some(r.rect);
            }
            // vybraný (naposledy kliknutý) priečinok – sem pôjde nový súbor; pri ovládaní klávesnicou aj súbor
            if (is_dir || self.tree_kbd) && self.tree_focus.as_deref() == Some(path.as_str()) {
                ui.painter().rect_stroke(r.rect, CornerRadius::same(6), Stroke::new(1.0, p.accent.gamma_multiply(0.55)), StrokeKind::Inside);
            }
            self.tree_menu(&r, &path, is_dir);
            // ťahanie: riadok sa dá chytiť a pustiť na priečinok (alebo súbor = jeho priečinok); mimo priečinka cez „Move to the project folder“
            if r.drag_started() {
                egui::DragAndDrop::set_payload(ui.ctx(), path.clone());
            }
            if r.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                drag_ghost(ui.ctx(), &name);
            }
            let tgt_dir = if is_dir { path.clone() } else { Path::new(&path).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default() };
            self.drop_rects.push((r.rect.intersect(ui.clip_rect()), tgt_dir.clone()));
            if let Some(src) = r.dnd_hover_payload::<String>() {
                if drop_ok(&src, &tgt_dir) {
                    ui.painter().rect_stroke(r.rect, CornerRadius::same(8), Stroke::new(1.5, p.accent), StrokeKind::Inside);
                }
            }
            if let Some(src) = r.dnd_release_payload::<String>() {
                self.move_item(&src, &tgt_dir);
            }
            if dirty {
                ui.painter().circle_filled(pos2(r.rect.right() - 12.0, r.rect.center().y), 3.0, p.accent);
            }
            self.tree_rows.push((path.clone(), is_dir));
            if r.clicked() || r.secondary_clicked() {
                self.tree_focus = Some(path.clone());
                self.tree_kbd = true;
            }
            if r.clicked() {
                if is_dir {
                    if open {
                        self.open_dirs.remove(&path);
                    } else {
                        self.open_dirs.insert(path.clone());
                    }
                } else {
                    self.open_file(&path);
                }
            }
            if is_dir && self.open_dirs.contains(&path) {
                let top = r.rect.bottom();
                self.tree_ui(ui, &path, depth + 1);
                // vodiaca čiara odsadenia pod otvoreným priečinkom (ako VS Code)
                let bottom = ui.cursor().top() - ui.spacing().item_spacing.y;
                if bottom > top + 2.0 {
                    let x = r.rect.left() + indent + 7.0;
                    ui.painter().vline(x, top..=bottom, Stroke::new(1.0, p.line.gamma_multiply(if ui.rect_contains_pointer(ui.clip_rect()) { 1.0 } else { 0.6 })));
                }
            }
        }
    }
}

pub const CRUMB_H: f32 = 28.0;
pub const TABS_H: f32 = 36.0;

impl App {
    // drobky nad editorom: projekt › priečinok › … › súbor; klik na priečinok ho ukáže v strome
    pub(super) fn breadcrumbs(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = self.pal;
        let Some(tab) = self.tabs.get(self.active) else { return };
        let path = tab.path.clone();
        let ws = self.workspace();
        // časti cesty: (text, cesta k priečinku / súboru)
        let mut parts: Vec<(String, String)> = vec![];
        match ws.as_deref().filter(|w| Path::new(&path).starts_with(w)) {
            Some(w) => {
                parts.push((widgets::file_name(w), w.to_string()));
                let rel = Path::new(&path).strip_prefix(w).map(|x| x.to_path_buf()).unwrap_or_default();
                let mut cur = Path::new(w).to_path_buf();
                for c in rel.components() {
                    cur = cur.join(c);
                    parts.push((c.as_os_str().to_string_lossy().to_string(), cur.to_string_lossy().to_string()));
                }
            }
            None => {
                if let Some(d) = Path::new(&path).parent() {
                    parts.push((widgets::file_name(&d.to_string_lossy()), d.to_string_lossy().to_string()));
                }
                parts.push((widgets::file_name(&path), path.clone()));
            }
        }
        let cy = r.center().y + 2.0;
        let mut x = r.left() + 16.0;
        let n = parts.len();
        let font = theme::ui(12.5);
        let mut clicked: Option<(String, bool)> = None;
        let clip = ui.painter().with_clip_rect(r);
        for (i, (name, full)) in parts.iter().enumerate() {
            let last = i + 1 == n;
            if i > 0 {
                widgets::icon_at(ui, pos2(x + 5.0, cy), 11.0, "chevron", p.text3.gamma_multiply(0.8));
                x += 14.0;
            }
            let icon_w = if last { 20.0 } else { 0.0 };
            let tw = widgets::text_w(ui, name, font.clone());
            let hit = Rect::from_min_size(pos2(x - 4.0, cy - 10.0), vec2(tw + icon_w + 8.0, 20.0));
            if hit.left() > r.right() - 20.0 {
                break;
            }
            let resp = ui.interact(hit, ui.id().with(("crumb", i)), Sense::click());
            if resp.hovered() {
                clip.rect_filled(hit, CornerRadius::same(5), p.hover);
            }
            if last {
                widgets::file_icon(ui, Rect::from_center_size(pos2(x + 7.0, cy), vec2(14.0, 14.0)), name);
            }
            let c = if last || resp.hovered() { p.text } else { p.text3 };
            clip.text(pos2(x + icon_w, cy), Align2::LEFT_CENTER, name, font.clone(), c);
            if resp.on_hover_text(full.as_str()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                clicked = Some((full.clone(), i == 0 && ws.is_some()));
            }
            x += tw + icon_w + 6.0;
        }
        match clicked {
            // projekt → jeho stránka
            Some((_, true)) => self.home = true,
            // priečinok / súbor → ukáž ho v strome
            Some((f, false)) => self.reveal(&f),
            None => {}
        }
    }

    // ukáže položku v Prieskumníkovi: otvorí priečinky k nej a vyberie ju
    pub(super) fn reveal(&mut self, path: &str) {
        let Some(ws) = self.workspace() else { return };
        self.side_open = true;
        self.side_view = "explorer".into();
        let mut d = if Path::new(path).is_dir() { Some(Path::new(path)) } else { Path::new(path).parent() };
        while let Some(x) = d.filter(|x| x.starts_with(&ws) && *x != Path::new(&ws)) {
            self.open_dirs.insert(x.to_string_lossy().to_string());
            d = x.parent();
        }
        self.tree_focus = Some(path.to_string());
        self.tree_kbd = true;
    }

    // klávesy v strome (keď nič nepíšeš): ↑/↓ výber, → / ← rozbaliť / zbaliť, Enter otvoriť, F2 premenovať, Delete do Koša
    pub(super) fn tree_keys(&mut self, ctx: &egui::Context) {
        if !self.tree_kbd || !self.side_open || self.start || self.settings.is_some() || self.palette.is_some() || self.ask.is_some() {
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            self.tree_kbd = false;
            return;
        }
        let Some(cur) = self.tree_focus.clone() else { return };
        let none = egui::Modifiers::NONE;
        let (up, down, right, left, enter, f2, del, esc) = ctx.input_mut(|i| {
            (
                i.consume_key(none, egui::Key::ArrowUp),
                i.consume_key(none, egui::Key::ArrowDown),
                i.consume_key(none, egui::Key::ArrowRight),
                i.consume_key(none, egui::Key::ArrowLeft),
                i.consume_key(none, egui::Key::Enter),
                i.consume_key(none, egui::Key::F2),
                i.consume_key(none, egui::Key::Delete),
                i.consume_key(none, egui::Key::Escape),
            )
        });
        let idx = self.tree_rows.iter().position(|r| r.0 == cur);
        let is_dir = Path::new(&cur).is_dir();
        if up || down {
            let n = self.tree_rows.len();
            if n > 0 {
                let i = match idx {
                    Some(i) if up => i.saturating_sub(1),
                    Some(i) => (i + 1).min(n - 1),
                    None => 0,
                };
                self.tree_focus = Some(self.tree_rows[i].0.clone());
            }
        } else if right && is_dir {
            self.open_dirs.insert(cur);
        } else if left {
            if is_dir && self.open_dirs.contains(&cur) {
                self.open_dirs.remove(&cur);
            } else if let Some(par) = Path::new(&cur).parent().map(|x| x.to_string_lossy().to_string()).filter(|x| self.tree_rows.iter().any(|r| &r.0 == x)) {
                self.tree_focus = Some(par);
            }
        } else if enter {
            if is_dir {
                if !self.open_dirs.remove(&cur) {
                    self.open_dirs.insert(cur);
                }
            } else {
                self.open_file(&cur);
            }
        } else if f2 && self.workspace().as_deref() != Some(cur.as_str()) {
            self.renaming = Some((cur.clone(), widgets::file_name(&cur)));
        } else if del && self.workspace().as_deref() != Some(cur.as_str()) && self.get("confirmDelete").as_bool() == Some(false) {
            self.act(&format!("trash:{cur}"), ctx);
        } else if del && self.workspace().as_deref() != Some(cur.as_str()) {
            self.ask = Some(super::menus::Ask {
                text: crate::i18n::tf("Delete “{name}”? It goes to the Recycle Bin, so you can still restore it.", &[("name", &widgets::file_name(&cur))]),
                yes: t("Move to Recycle Bin"),
                action: format!("trash:{cur}"),
            });
        } else if esc {
            self.tree_kbd = false;
        }
        if up || down || right || left || enter {
            ctx.request_repaint();
        }
    }
}

impl App {
    // karty otvorených súborov nad editorom (ako VS Code): riadok na vrchu karty, aktívna karta má farbu editora
    // a akcentový prúžok hore; koliesko myši posúva karty, prostredné tlačidlo zatvára
    pub(super) fn tab_strip(&mut self, ui: &mut egui::Ui, r: Rect, radius: u8) {
        let p = self.pal;
        let strip_bg = if p.dark { p.card.lerp_to_gamma(egui::Color32::BLACK, 0.22) } else { p.card.lerp_to_gamma(p.hover, 0.7) };
        ui.painter().rect_filled(r, CornerRadius { nw: radius, ne: radius, ..Default::default() }, strip_bg);
        // spodná čiara pod kartami – pod aktívnou kartou sa preruší (karta „vteká“ do editora)
        let base_line = ui.painter().add(egui::Shape::Noop);
        let font = theme::ui(13.0);
        let bold = self.get("tabBold").as_bool() == Some(true);
        let big_dot = self.get("dirtyDot").as_str() == Some("large");
        let widths: Vec<f32> = self.tabs.iter().map(|t| (widgets::text_w(ui, &t.name(), font.clone()) + 64.0).clamp(90.0, 240.0)).collect();
        let total: f32 = widths.iter().sum();
        // posun kariet: koliesko nad riadkom; aktívna karta sa vždy ukáže
        let id = egui::Id::new("tab-strip-off");
        let mut off: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(0.0);
        let left0 = r.left() + if radius > 0 { 4.0 } else { 0.0 };
        let view_w = r.right() - left0 - 4.0;
        if ui.rect_contains_pointer(r) {
            let d = ui.input(|i| i.smooth_scroll_delta());
            off -= if d.x.abs() > d.y.abs() { d.x } else { d.y };
        }
        if !self.home {
            let a = widths[..self.active.min(widths.len())].iter().sum::<f32>();
            let b = a + widths.get(self.active).copied().unwrap_or(0.0);
            if a < off {
                off = a;
            } else if b > off + view_w {
                off = b - view_w;
            }
        }
        off = off.clamp(0.0, (total - view_w).max(0.0));
        ui.ctx().data_mut(|d| d.insert_temp(id, off));
        let clip = Rect::from_min_max(pos2(left0, r.top()), pos2(r.right() - 4.0, r.bottom()));
        let painter = ui.painter().with_clip_rect(clip);
        let mut x = left0 - off;
        let (mut close, mut act) = (None, None);
        let mut active_x: Option<(f32, f32)> = None;
        for (i, t) in self.tabs.iter().enumerate() {
            let w = widths[i];
            // zaoblené karty: aktívna má zaoblený vrch a splýva s editorom, ostatné sú pri prejdení pilulky
            let tr = Rect::from_min_max(pos2(x + 2.0, r.top() + 5.0), pos2(x + w - 2.0, r.bottom()));
            x += w;
            if tr.right() < clip.left() || tr.left() > clip.right() {
                continue;
            }
            let sel = i == self.active && !self.home;
            let name = t.name();
            let resp = ui.interact(tr.intersect(clip), ui.id().with(("tab", i)), Sense::click());
            let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered() && !sel, 0.12);
            if sel {
                let top = CornerRadius { nw: 10, ne: 10, ..Default::default() };
                let ta = if self.anim_on() { 0.16 } else { 0.0 };
                let ax = ui.ctx().animate_value_with_time(egui::Id::new("tabbar-x"), tr.left(), ta);
                let aw = ui.ctx().animate_value_with_time(egui::Id::new("tabbar-w"), tr.width(), ta);
                let ar = Rect::from_min_size(pos2(ax, tr.top()), vec2(aw, tr.height() + 1.0));
                painter.rect_filled(ar, top, p.card);
                painter.rect_stroke(ar, top, Stroke::new(1.0, p.line), StrokeKind::Inside);
                // prekrytie spodnej hrany rámika, nech karta plynule prejde do editora
                painter.rect_filled(Rect::from_min_max(pos2(ar.left() + 1.0, ar.bottom() - 2.0), pos2(ar.right() - 1.0, ar.bottom() + 1.0)), CornerRadius::ZERO, p.card);
                // malý akcentový prúžok navrchu
                let bar = Rect::from_center_size(pos2(ar.center().x, ar.top() + 1.5), vec2((ar.width() * 0.35).min(36.0), 3.0));
                painter.rect_filled(bar, CornerRadius::same(2), p.accent);
                active_x = Some((ar.left(), ar.right()));
            } else if hk > 0.0 {
                let pill = Rect::from_min_max(pos2(tr.left(), tr.top() + 1.0), pos2(tr.right(), tr.bottom() - 4.0));
                painter.rect_filled(pill, CornerRadius::same(8), p.hover.gamma_multiply(hk));
            }
            let cy = tr.center().y - 1.0;
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(clip));
            child.set_clip_rect(clip.intersect(tr));
            widgets::file_icon(&child, Rect::from_min_size(pos2(tr.left() + 12.0, cy - 8.0), vec2(16.0, 16.0)), &name);
            let f = if sel && bold { theme::bold(13.0) } else { font.clone() };
            widgets::text(&child, pos2(tr.left() + 34.0, cy), egui::Align2::LEFT_CENTER, &name, f, if sel { p.text } else if resp.hovered() { p.text2 } else { p.text3 }, w - 62.0);
            // farba karty podľa druhu súboru (Visual Studio: Tab colorization)
            if let Some(c) = self.tab_color(&name) {
                painter.rect_filled(Rect::from_min_max(pos2(tr.left() + 10.0, tr.bottom() - 4.0), pos2(tr.right() - 10.0, tr.bottom() - 2.0)), CornerRadius::same(1), c);
            }
            let cr = Rect::from_center_size(pos2(tr.right() - 17.0, cy), vec2(20.0, 20.0));
            let c = ui.interact(cr.intersect(clip), ui.id().with(("tabx", i)), Sense::click());
            if t.dirty() && !c.hovered() {
                painter.circle_filled(cr.center(), if big_dot { 5.0 } else { 3.5 }, if big_dot { p.accent } else { p.text2 });
            } else {
                if c.hovered() {
                    painter.rect_filled(cr, CornerRadius::same(5), p.active);
                }
                if sel || resp.hovered() || c.hovered() {
                    widgets::icon_at(&child, cr.center(), 12.0, "x", if c.hovered() { p.text } else { p.text3 });
                }
            }
            let resp = resp.on_hover_text(t.path.as_str());
            if c.clicked() || resp.middle_clicked() {
                close = Some(i);
            } else if resp.clicked() {
                act = Some(i);
            }
        }
        let line = match active_x {
            Some((a, b)) => egui::Shape::Vec(vec![
                egui::Shape::hline(r.left()..=a.max(r.left()), r.bottom() - 0.5, Stroke::new(1.0, p.line)),
                egui::Shape::hline(b.min(r.right())..=r.right(), r.bottom() - 0.5, Stroke::new(1.0, p.line)),
            ]),
            None => egui::Shape::hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, p.line)),
        };
        ui.painter().set(base_line, line);
        // prázdne miesto za kartami: dvojklik = nový súbor (ako VS Code)
        if x < clip.right() {
            let rest = Rect::from_min_max(pos2(x.max(clip.left()), r.top()), pos2(clip.right(), r.bottom()));
            if ui.interact(rest, ui.id().with("tabs-rest"), Sense::click()).double_clicked() {
                self.open_new_file(None);
            }
        }
        if let Some(i) = close {
            self.close_tab(i);
        } else if let Some(i) = act {
            self.activate(i);
        }
    }
}
