// Obchod s témami (ako obchod pluginov): zoznam themes/store/index.json z GitHubu, ku každej téme jej súbor .conf
// (malý, stiahne sa hneď kvôli ukážke farieb). Inštalácia = súbor do userData/themes, potom sa téma hneď použije.
// Vlastnú tému do obchodu pridá ktokoľvek pull requestom (docs/THEMES.md → Share your theme).
use super::App;
use crate::i18n::{t, tf};
use crate::theme;
use crate::themes::ThemeFile;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const BRANCHES: [&str; 2] = ["main", "claude/optimistic-darwin-5i7m9t"];

#[derive(Clone)]
pub struct StoreTheme {
    pub id: String,
    pub name: String,
    pub author: String,
    pub desc: String,
    pub src: String,
    pub file: Option<ThemeFile>,
}

#[derive(Clone, Default)]
pub enum Load {
    #[default]
    Idle,
    Loading,
    Err(String),
    Ready(Vec<StoreTheme>),
}

#[derive(Default)]
pub struct ThemeStore {
    pub browse: bool,
    pub q: String,
    pub data: Arc<Mutex<Load>>,
}

// súbor z obchodu: FLUX_THEME_REGISTRY = miestny priečinok (testy), inak GitHub (tie isté vetvy ako pluginy)
fn fetch(rel: &str) -> Result<String, String> {
    if let Ok(d) = std::env::var("FLUX_THEME_REGISTRY") {
        return std::fs::read_to_string(std::path::Path::new(&d).join(rel)).map_err(|e| e.to_string());
    }
    let mut last = String::new();
    let own = env!("FLUX_BRANCH");
    let mut branches: Vec<&str> = if own.is_empty() || BRANCHES.contains(&own) { vec![] } else { vec![own] };
    branches.extend(BRANCHES);
    for b in branches {
        let url = format!("https://raw.githubusercontent.com/pantr1x/Flux/{b}/themes/store/{rel}");
        match crate::net::request("GET", &url, &[], None, 30) {
            Ok((200, body)) => return Ok(body),
            Ok((code, _)) => last = format!("HTTP {code}"),
            Err(e) => last = e,
        }
    }
    Err(last)
}

// id témy smie mať len písmená, číslice, - a _ (je to aj meno súboru)
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

// rozbor index.json + súborov tém (pre testy aj pre vlákno)
pub fn load_with(get: impl Fn(&str) -> Result<String, String>) -> Result<Vec<StoreTheme>, String> {
    let v: Value = serde_json::from_str(&get("index.json")?).map_err(|e| e.to_string())?;
    let mut out = vec![];
    for e in v["themes"].as_array().cloned().unwrap_or_default().iter().take(200) {
        let id = e["id"].as_str().unwrap_or("").to_string();
        if !valid_id(&id) {
            continue;
        }
        let src = get(&format!("{id}.conf")).unwrap_or_default();
        let file = crate::themes::parse(&id, &src).filter(|_| !src.is_empty());
        out.push(StoreTheme {
            name: e["name"].as_str().map(String::from).or_else(|| file.as_ref().map(|f| f.name.clone())).unwrap_or_else(|| id.clone()),
            author: e["author"].as_str().unwrap_or("").to_string(),
            desc: e["description"].as_str().unwrap_or("").to_string(),
            id,
            src,
            file,
        });
    }
    Ok(out)
}

impl App {
    fn store_start(&mut self, ctx: &egui::Context) {
        let data = self.theme_store.data.clone();
        *data.lock().unwrap() = Load::Loading;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = load_with(fetch);
            *data.lock().unwrap() = match r {
                Ok(l) => Load::Ready(l),
                Err(e) => Load::Err(e),
            };
            ctx.request_repaint();
        });
    }

    // nainštaluje tému z obchodu a hneď ju použije
    fn store_install(&mut self, th: &StoreTheme, ctx: &egui::Context) {
        if !valid_id(&th.id) || th.file.is_none() {
            return;
        }
        let dir = crate::themes::user_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Err(e) = std::fs::write(dir.join(format!("{}.conf", th.id)), &th.src) {
            self.status = e.to_string();
            return;
        }
        crate::themes::reload();
        self.set_code_theme(&th.id, ctx);
        self.apply_look(ctx);
        self.note(tf("Theme {name} installed", &[("name", &th.name)]));
    }

    // odstráni tému stiahnutú z obchodu (dá sa stiahnuť znova); vlastné témy sa tu nemažú
    fn store_remove(&mut self, id: &str, ctx: &egui::Context) {
        if !valid_id(id) {
            return;
        }
        let _ = std::fs::remove_file(crate::themes::user_dir().join(format!("{id}.conf")));
        crate::themes::reload();
        if self.get("codeTheme").as_str() == Some(id) {
            let dark = self.pal.dark;
            self.set_code_theme(if dark { crate::gen::DEFAULT_THEME } else { "vscode-light" }, ctx);
            self.apply_look(ctx);
        }
    }

    // prepínač Nainštalované / Obchod + hľadanie (len v obchode); vráti true, keď sa má kresliť obchod
    pub(super) fn themes_header(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context, installed_n: usize) -> bool {
        let p = self.pal;
        let st = self.theme_store.browse;
        let labels = [(false, "palette", tf("Installed ({n})", &[("n", &installed_n.to_string())])), (true, "star", t("Theme store"))];
        let seg_w: Vec<f32> = labels.iter().map(|l| widgets::text_w(ui, &l.2, theme::bold(13.0)) + 48.0).collect();
        let (row, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
        let bar = Rect::from_min_size(row.min, vec2(seg_w.iter().sum::<f32>() + 8.0, 40.0));
        ui.painter().rect_filled(bar, CornerRadius::same(12), p.hover);
        ui.painter().rect_stroke(bar, CornerRadius::same(12), Stroke::new(1.0, p.line), StrokeKind::Inside);
        let mut x = bar.left() + 4.0;
        for (i, (store, icon, label)) in labels.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, bar.top() + 4.0), vec2(seg_w[i], 32.0));
            x += seg_w[i];
            let resp = ui.interact(r, ui.id().with(("theme-seg", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = *store == st;
            let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered() && !on, 0.12);
            if on {
                ui.painter().rect_filled(r, CornerRadius::same(9), p.card2);
                ui.painter().rect_stroke(r, CornerRadius::same(9), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            } else if hk > 0.0 {
                ui.painter().rect_filled(r, CornerRadius::same(9), p.active.gamma_multiply(hk));
            }
            let fg = if on { p.text } else { p.text3 };
            widgets::icon_at(ui, pos2(r.left() + 20.0, r.center().y), 14.0, icon, if on { p.accent } else { fg });
            ui.painter().text(pos2(r.left() + 34.0, r.center().y), Align2::LEFT_CENTER, label, theme::bold(13.0), fg);
            if resp.clicked() {
                self.theme_store.browse = *store;
            }
        }
        if st {
            let sw = (w - bar.width() - 16.0).clamp(140.0, 300.0);
            let sr = Rect::from_min_size(pos2(row.right() - sw, row.top()), vec2(sw, 40.0));
            ui.painter().rect_filled(sr, CornerRadius::same(12), p.hover);
            ui.painter().rect_stroke(sr, CornerRadius::same(12), Stroke::new(1.0, p.line), StrokeKind::Inside);
            widgets::icon_at(ui, pos2(sr.left() + 18.0, sr.center().y), 14.0, "search", p.text3);
            let mut si = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(sr.left() + 34.0, sr.top() + 11.0), pos2(sr.right() - 10.0, sr.bottom() - 8.0))));
            si.add(egui::TextEdit::singleline(&mut self.theme_store.q).hint_text(t("Search themes")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(13.0)));
        }
        ui.add_space(14.0);
        st
    }

    pub(super) fn theme_store_ui(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        if matches!(*self.theme_store.data.lock().unwrap(), Load::Idle) {
            self.store_start(ctx);
        }
        let state = self.theme_store.data.lock().unwrap().clone();
        let list = match state {
            Load::Ready(l) => l,
            Load::Err(e) => {
                ui.label(egui::RichText::new(tf("The theme store could not be loaded: {e}", &[("e", &e)])).color(p.red));
                if widgets::button(ui, Some("refresh"), &t("Try again"), p.card2, p.text, 30.0, &p).clicked() {
                    self.store_start(ctx);
                }
                return;
            }
            _ => {
                ui.label(egui::RichText::new(t("Loading the theme store…")).color(p.text3));
                ctx.request_repaint_after(Duration::from_millis(200));
                return;
            }
        };
        let q = self.theme_store.q.trim().to_lowercase();
        let shown: Vec<&StoreTheme> = list.iter().filter(|th| q.is_empty() || format!("{} {} {} {}", th.name, th.author, th.desc, th.id).to_lowercase().contains(&q)).collect();
        let cur = self.get("codeTheme").as_str().unwrap_or("").to_string();
        let have = crate::themes::all();
        let cols = if w < 560.0 { 1 } else { 2 };
        let cw = (w - (cols - 1) as f32 * 12.0) / cols as f32;
        let ch = 170.0;
        let rows = shown.len().div_ceil(cols);
        let (area, _) = ui.allocate_exact_size(vec2(w, rows as f32 * (ch + 12.0)), Sense::hover());
        let (mut install, mut use_id, mut remove) = (None, None, None);
        for (i, th) in shown.iter().enumerate() {
            let r = Rect::from_min_size(area.min + vec2((i % cols) as f32 * (cw + 12.0), (i / cols) as f32 * (ch + 12.0)), vec2(cw, ch));
            let installed = have.iter().any(|h| h.id == th.id && h.user);
            let using = cur == th.id;
            let resp = ui.interact(r, ui.id().with(("ts-card", th.id.as_str())), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
            ui.painter().rect_filled(r, CornerRadius::same(14), p.card2.lerp_to_gamma(p.hover, hk * 0.6));
            ui.painter().rect_stroke(r, CornerRadius::same(14), if using { Stroke::new(1.5, p.accent) } else { Stroke::new(1.0, p.line.lerp_to_gamma(p.line_strong, hk)) }, StrokeKind::Inside);
            // ukážka: kúsok editora vo farbách témy
            let pv = Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(r.width() - 16.0, 86.0));
            if let Some(f) = &th.file {
                preview(ui, pv, f);
            }
            let by = if th.author.is_empty() { String::new() } else { tf("by {name}", &[("name", &th.author)]) };
            let kind = match &th.file {
                Some(f) if !f.dark => t("light"),
                _ => t("dark"),
            };
            // tlačidlo vpravo dole
            let (label, fill, fg) = if using {
                (t("In use"), Color32::TRANSPARENT, p.green)
            } else if installed {
                (t("Use"), p.card, p.text)
            } else {
                (t("Install"), p.accent, p.accent_fg)
            };
            let bw = widgets::text_w(ui, &label, theme::bold(12.5)) + 28.0;
            let br = Rect::from_min_size(pos2(r.right() - 12.0 - bw, r.bottom() - 42.0), vec2(bw, 30.0));
            let tw = br.left() - r.left() - 24.0;
            widgets::text(ui, pos2(r.left() + 14.0, pv.bottom() + 20.0), Align2::LEFT_CENTER, &th.name, theme::bold(14.0), p.text, tw);
            let meta = if by.is_empty() { kind } else { format!("{by} · {kind}") };
            widgets::text(ui, pos2(r.left() + 14.0, pv.bottom() + 39.0), Align2::LEFT_CENTER, &meta, theme::ui(11.5), p.text3, tw);
            widgets::text(ui, pos2(r.left() + 14.0, pv.bottom() + 57.0), Align2::LEFT_CENTER, &th.desc, theme::ui(11.5), p.text2, tw);
            let bresp = ui.interact(br, ui.id().with(("ts-btn", th.id.as_str())), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let bh = ctx.animate_bool_with_time(bresp.id.with("h"), bresp.hovered(), 0.12);
            if fill != Color32::TRANSPARENT {
                ui.painter().rect_filled(br, CornerRadius::same(10), fill.lerp_to_gamma(Color32::WHITE, 0.08 * bh));
                if fill == p.card {
                    ui.painter().rect_stroke(br, CornerRadius::same(10), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                }
            }
            if using {
                widgets::icon_at(ui, pos2(br.left() + 8.0, br.center().y), 12.0, "check", fg);
            }
            ui.painter().text(br.center() + vec2(if using { 6.0 } else { 0.0 }, 0.0), Align2::CENTER_CENTER, &label, theme::bold(12.5), fg);
            // odstrániť (len stiahnutú z obchodu)
            let mut x_clicked = false;
            if installed && (resp.hovered() || bresp.hovered()) {
                let xr = Rect::from_center_size(pos2(br.left() - 20.0, br.center().y), vec2(28.0, 28.0));
                x_clicked = widgets::icon_button_at(ui, xr, "trash", 14.0, &p, true).on_hover_text(t("Remove")).clicked();
            }
            if !th.desc.is_empty() {
                resp.clone().on_hover_text(&th.desc);
            }
            if x_clicked {
                remove = Some(th.id.clone());
            } else if (bresp.clicked() || resp.clicked()) && !using {
                if installed {
                    use_id = Some(th.id.clone());
                } else {
                    install = Some((*th).clone());
                }
            }
        }
        if shown.is_empty() {
            ui.label(egui::RichText::new(t("No themes found")).color(p.text3));
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if widgets::button(ui, Some("github"), &t("Share your theme"), p.card2, p.text, 30.0, &p).on_hover_text(t("add your .conf file to the store with a pull request – the guide explains how")).clicked() {
                flux_core::settings::open_external("https://github.com/pantr1x/Flux/blob/main/docs/THEMES.md#share-your-theme");
            }
            if widgets::button(ui, Some("refresh"), &t("Reload"), p.card2, p.text, 30.0, &p).clicked() {
                self.store_start(ctx);
            }
        });
        if let Some(th) = install {
            self.store_install(&th, ctx);
        }
        if let Some(id) = use_id {
            self.set_code_theme(&id, ctx);
            self.apply_look(ctx);
        }
        if let Some(id) = remove {
            self.store_remove(&id, ctx);
        }
    }
}

// malé okno editora vo farbách témy: pozadie panela, čísla riadkov a „kód“ z farieb tokenov
fn preview(ui: &egui::Ui, r: Rect, f: &ThemeFile) {
    let bg = f.ui.get("panel").or_else(|| f.ui.get("background")).map(|c| theme::hex(*c)).unwrap_or(if f.dark { Color32::from_rgb(0x1a, 0x1a, 0x1f) } else { Color32::from_rgb(0xfb, 0xfb, 0xfc) });
    ui.painter().rect_filled(r, CornerRadius::same(10), bg);
    if let Some(b) = f.ui.get("border") {
        ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, theme::hex(*b)), StrokeKind::Inside);
    }
    let c = |i: usize| theme::hex(f.code[i]);
    let num = c(1).gamma_multiply(0.7);
    // (index farby, šírka) – def name(param): / if x == 42: / return "text"
    let lines: [&[(usize, f32)]; 4] = [&[(2, 18.0), (7, 34.0), (14, 4.0), (9, 26.0), (14, 6.0)], &[(2, 10.0), (8, 16.0), (14, 8.0), (5, 14.0), (14, 4.0)], &[(2, 30.0), (4, 46.0)], &[(1, 70.0)]];
    let indent = [0.0, 12.0, 24.0, 0.0];
    for (li, row) in lines.iter().enumerate() {
        let y = r.top() + 16.0 + li as f32 * 17.0;
        ui.painter().rect_filled(Rect::from_min_size(pos2(r.left() + 10.0, y - 2.0), vec2(6.0, 4.0)), CornerRadius::same(2), num);
        let mut x = r.left() + 26.0 + indent[li];
        for (idx, w) in row.iter() {
            ui.painter().rect_filled(Rect::from_min_size(pos2(x, y - 3.0), vec2(*w, 6.0)), CornerRadius::same(3), c(*idx));
            x += w + 5.0;
        }
    }
    // pruh s farbou zvýraznenia dole
    if let Some(a) = f.ui.get("accent") {
        ui.painter().rect_filled(Rect::from_min_size(pos2(r.right() - 44.0, r.bottom() - 16.0), vec2(32.0, 8.0)), CornerRadius::same(4), theme::hex(*a));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_store_index() {
        // každá téma v themes/store má záznam v index.json a dá sa prečítať (a naopak)
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../themes/store");
        let list = load_with(|rel| std::fs::read_to_string(dir.join(rel)).map_err(|e| e.to_string())).unwrap();
        assert!(!list.is_empty());
        for th in &list {
            assert!(th.file.is_some(), "theme {} has no readable .conf", th.id);
            assert!(!th.author.is_empty() && !th.desc.is_empty(), "theme {} needs author and description", th.id);
        }
        for f in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "conf") {
                let id = p.file_stem().unwrap().to_string_lossy().to_string();
                assert!(list.iter().any(|t| t.id == id), "{id}.conf is missing in index.json");
            }
        }
        assert!(!valid_id("../x") && !valid_id("") && valid_id("night-owl"));
    }
}
