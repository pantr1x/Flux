// Rozloženie ako vo VS Code: úzky pás ikon (Activity Bar) a vedľa neho bočný panel so zobrazením
// – Prieskumník (projekt ako zbaliteľná sekcia s Nový súbor / Nový priečinok / Obnoviť / Zbaliť,
// nové súbory sa píšu priamo do stromu) a Projekty (pripnuté, zoznam, Nový projekt).
use super::{chrome, drag_ghost, drop_ok, name_field, rename_field, short_dir, App, Rn};
use crate::i18n::t;
use crate::theme::{self, GAP, TOP_H};
use crate::widgets::{self, Lead};
use eframe::egui::{self, pos2, vec2, CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind};
use flux_core::fsops;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;

pub const ACT_W: f32 = 48.0;
const SEC_H: f32 = 26.0;

// zobrazenia v páse ikon: id, ikona, popis
// (hľadanie je pole v hornej lište, projekty sú sekcia Prieskumníka – ako VS Code)
const VIEWS: [(&str, &str, &str); 2] = [("explorer", "file", "Explorer"), ("plugins", "puzzle", "Plugins")];

impl App {
    fn load_side(&mut self) {
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

    fn sec_open(&self, id: &str) -> bool {
        !self.side_closed.as_ref().is_some_and(|s| s.contains(id))
    }

    fn toggle_sec(&mut self, id: &str) {
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
                for (id, icon, tip) in VIEWS {
                    let b = Rect::from_min_size(pos2(r.left(), y), vec2(ACT_W, 46.0));
                    let active = self.side_open && self.side_view == id;
                    let resp = ui.interact(b, ui.id().with(("act", id)), Sense::click()).on_hover_text(t(tip));
                    let hk = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
                    let ak = ui.ctx().animate_bool_with_time(resp.id.with("a"), active, if self.anim_on() { 0.15 } else { 0.0 });
                    if ak > 0.0 {
                        // značka aktívneho zobrazenia pri okraji (ako VS Code), v akcentovej farbe
                        let h = 22.0 * ak;
                        let bx = if right { b.right() - 2.5 } else { b.left() + 0.5 };
                        ui.painter().rect_filled(Rect::from_min_size(pos2(bx, b.center().y - h / 2.0), vec2(2.0, h)), CornerRadius::same(1), p.accent);
                    }
                    let c = p.text3.lerp_to_gamma(p.text, hk.max(ak));
                    widgets::icon_at(ui, b.center(), 21.0, icon, c);
                    if id == "explorer" {
                        self.tour_rects.insert("files", b);
                    }
                    if resp.clicked() {
                        self.show_view(id, &ctx);
                    }
                    y += 46.0;
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
        (if right { egui::Panel::right("side") } else { egui::Panel::left("side") })
            .resizable(false)
            .exact_size(crate::theme::SIDE_W)
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
        let side_hover = ui.rect_contains_pointer(ui.max_rect());
        // dole sekcia FILES (súbory otvorené mimo projektu); strom projektu vyplní zvyšok
        let n_files = self.recent_files.len();
        let files_open = self.sec_open("files");
        let row_h = 38.0f32;
        let files_h = SEC_H + if files_open { (n_files as f32 * row_h).min(row_h * 3.5) + 30.0 } else { 0.0 };
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
                // zvýraznenie otvoreného súboru kĺže k novému riadku (slideIndicator v Electron Fluxe)
                let bg = ui.painter().add(egui::Shape::Noop);
                let origin = ui.min_rect().top();
                self.tree_sel = None;
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
                let rest = 24.0;
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
        let mut sa = egui::ScrollArea::vertical().id_salt("loose").max_height((self.recent_files.len() as f32 * 38.0).min(38.0 * 3.5)).auto_shrink([false, true]);
        if let Some(o) = off {
            sa = sa.vertical_scroll_offset(o);
        }
        let sout = sa.show(ui, |ui| {
            for f in &self.recent_files {
                let sel = !self.home && self.tabs.get(self.active).map(|t| &t.path == f).unwrap_or(false);
                if widgets::row(ui, sel, 10.0, Lead::File(&widgets::file_name(f)), &widgets::file_name(f), Some(&short_dir(f)), false, &p).on_hover_text(f).clicked() {
                    open_f = Some(f.clone());
                }
            }
        });
        self.smooth.end(sk, &sout);
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
            // s otvoreným projektom najviac ~3,5 riadka (strom má prednosť), bez neho celé miesto
            let row_h = 38.0f32;
            let max_h = if ws.is_some() {
                (projects.len() as f32 * row_h).min(row_h * 3.5)
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
                    ui.add_space(indent);
                    // pri súbore sa označí meno bez prípony
                    res = rename_field(ui, egui::Id::new(("rename", &path)), new, Margin::symmetric(10, 5), !is_dir);
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
            let r = widgets::row_ex(ui, active, false, ind, lead, &name, None, false, &p);
            if active {
                self.tree_sel = Some(r.rect);
            }
            // vybraný (naposledy kliknutý) priečinok: jemný rámik – sem pôjde nový súbor
            if is_dir && self.tree_focus.as_deref() == Some(path.as_str()) {
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
            if r.clicked() || r.secondary_clicked() {
                self.tree_focus = Some(path.clone());
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
