// Pluginy v aplikácii: spustenie, efekty (toast, text, príkazy, stavový riadok, úryvky, témy, značky), udalosti,
// časovače a Nastavenia → Pluginy (nainštalované, obchod, vytvoriť plugin, načítať z priečinka).
use super::App;
use crate::i18n::{t, tf};
use crate::plugins::{self, Manifest};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct PlugCmd {
    pub plugin: String,
    pub id: String,
    pub label: String,
    pub key: Option<(egui::Modifiers, egui::Key)>,
}

pub struct PlugStatus {
    pub plugin: String,
    pub k: String,
    pub text: String,
    pub title: String,
    pub show: bool,
}

#[derive(Default)]
pub struct PlugState {
    pub host: Option<plugins::Host>,
    pub cmds: Vec<PlugCmd>,
    pub status: Vec<PlugStatus>,
    pub snips: Vec<(String, String, crate::complete::Item)>, // plugin, jazyk, úryvok
    pub marks: std::collections::HashMap<String, (String, Vec<crate::lint::Diag>)>, // plugin → (cesta, značky)
    pub changed: Option<(String, u64, Instant)>,
    pub sent: Option<(String, u64)>,
    pub sel: Option<(String, usize)>,
    pub tick: Option<Instant>,
    pub catalog: Option<Arc<Mutex<Option<Result<Vec<Value>, String>>>>>,
    pub installing: Option<String>,
    pub query: String,
    pub tab_store: bool,
}

// „Ctrl+Alt+H“ → modifikátory + kláves
fn parse_key(s: &str) -> Option<(egui::Modifiers, egui::Key)> {
    let mut m = egui::Modifiers::NONE;
    let mut key = None;
    for part in s.split('+').map(|x| x.trim()) {
        match part.to_lowercase().as_str() {
            "ctrl" | "cmd" | "control" => m.command = true,
            "alt" => m.alt = true,
            "shift" => m.shift = true,
            p => key = egui::Key::from_name(&p.to_uppercase()).or_else(|| egui::Key::from_name(p)),
        }
    }
    if m.command {
        m.ctrl = !cfg!(target_os = "macos");
    }
    key.map(|k| (m, k))
}

fn file_in(dir: &Path, rel: &str) -> Option<PathBuf> {
    // len súbory v priečinku pluginu (žiadne ../)
    let p = dir.join(rel);
    (!rel.contains("..") && !Path::new(rel).is_absolute()).then_some(p)
}

impl App {
    fn plug_dev_dirs(&self) -> Vec<String> {
        self.get("pluginDevDirs").as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
    }

    fn plug_enabled(&self, id: &str) -> bool {
        self.get("nativePlugins")[id]["enabled"].as_bool() != Some(false)
    }

    // stav pre plugin: aktívny súbor, projekt, jeho uložené dáta
    fn plug_snap(&self, id: &str) -> Value {
        let file = if self.start || self.home {
            Value::Null
        } else {
            self.tabs.get(self.active).map(|tab| {
                let ext = tab.ext();
                let (a, b) = self.sel_chars.unwrap_or((0, 0));
                let (a, b) = (a.min(b), a.max(b));
                let sel: String = tab.text.chars().skip(a).take(b - a).collect();
                json!({
                    "path": tab.path, "name": widgets::file_name(&tab.path), "language": plugins::lang_id(&ext), "text": tab.text,
                    "selection": { "start": a, "end": b, "text": sel }, "cursor": { "line": self.cursor.0, "column": self.cursor.1 }
                })
            }).unwrap_or(Value::Null)
        };
        json!({ "file": file, "folder": self.workspace(), "storage": self.get("pluginData")[id].clone() })
    }

    pub(super) fn plug_start(&mut self) {
        if self.plug.host.is_some() {
            return;
        }
        plugins::set_folder(self.workspace().map(PathBuf::from));
        self.plug.host = Some(plugins::Host::new());
        for m in plugins::Host::discover(&self.plug_dev_dirs()) {
            self.plug_load(m);
        }
    }

    fn plug_load(&mut self, m: Manifest) {
        let id = m.id.clone();
        let on = self.plug_enabled(&id);
        let snap = self.plug_snap(&id);
        let fx = self.plug.host.as_mut().map(|h| h.load(m, &snap, on)).unwrap_or_default();
        self.plug_apply(fx);
        if let Some(e) = self.plug.host.as_ref().and_then(|h| h.list.iter().find(|p| p.man.id == id)).and_then(|p| p.error.clone()) {
            self.note(tf("Plugin {name} failed: {e}", &[("name", &id), ("e", &e)]));
        }
    }

    pub(super) fn plug_unload(&mut self, id: &str) {
        if let Some(h) = self.plug.host.as_mut() {
            h.unload(id);
        }
        self.plug.cmds.retain(|c| c.plugin != id);
        self.plug.status.retain(|c| c.plugin != id);
        self.plug.snips.retain(|c| c.0 != id);
        self.plug.marks.remove(id);
    }

    pub(super) fn plug_reload_all(&mut self) {
        let ids: Vec<String> = self.plug.host.as_ref().map(|h| h.list.iter().map(|p| p.man.id.clone()).collect()).unwrap_or_default();
        for id in ids {
            self.plug_unload(&id);
        }
        self.plug.host = None;
        self.plug_start();
        self.note(t("Plugins reloaded."));
    }

    fn plug_call(&mut self, id: &str, code: &str) {
        plugins::set_folder(self.workspace().map(PathBuf::from));
        let snap = self.plug_snap(id);
        let fx = self.plug.host.as_mut().map(|h| h.call(id, code, &snap)).unwrap_or_default();
        self.plug_apply(fx);
    }

    // udalosť pre všetky bežiace pluginy
    pub(super) fn plug_emit(&mut self, ev: &str) {
        let ids = self.plug.host.as_ref().map(|h| h.running()).unwrap_or_default();
        for id in ids {
            self.plug_call(&id, &format!("__emit('{ev}')"));
        }
    }

    pub(super) fn plug_command(&mut self, plugin: &str, id: &str) {
        let code = format!("__cmd({})", json!(id));
        self.plug_call(plugin, &code);
    }

    fn plug_apply(&mut self, fx: Vec<Value>) {
        for f in fx {
            let plugin = f["plugin"].as_str().unwrap_or("").to_string();
            let s = |k: &str| f[k].as_str().unwrap_or("").to_string();
            match f["t"].as_str().unwrap_or("") {
                "toast" | "error" => self.note(s("text")),
                "log" => {
                    if let Some(h) = self.plug.host.as_mut() {
                        h.log.push(format!("[{plugin}] {}", s("text")));
                        let n = h.log.len();
                        if n > 300 {
                            h.log.drain(..n - 300);
                        }
                    }
                }
                "insert" | "replace" => self.plug_edit(&s("text"), f["t"] == "replace"),
                "cmd" => {
                    let id = s("id");
                    self.plug.cmds.retain(|c| !(c.plugin == plugin && c.id == id));
                    self.plug.cmds.push(PlugCmd { plugin, id, label: s("label"), key: parse_key(&s("key")) });
                }
                "run" => self.plug_run.push(s("id")),
                "status" => {
                    let k = s("k");
                    if let Some(x) = self.plug.status.iter_mut().find(|x| x.plugin == plugin && x.k == k) {
                        if let Some(t) = f["text"].as_str() {
                            x.text = t.into();
                        }
                        if let Some(t) = f["title"].as_str() {
                            x.title = t.into();
                        }
                        if let Some(b) = f["show"].as_bool() {
                            x.show = b;
                        }
                    } else {
                        self.plug.status.push(PlugStatus { plugin, k, text: s("text"), title: s("title"), show: f["show"].as_bool() != Some(false) });
                    }
                }
                "status-rm" => {
                    let k = s("k");
                    self.plug.status.retain(|x| !(x.plugin == plugin && x.k == k));
                }
                "snippet" => {
                    let body = s("body");
                    let prefix = s("prefix");
                    let desc = s("desc");
                    let preview = if desc.is_empty() { body.lines().next().unwrap_or("").replace("$0", "").to_string() } else { desc };
                    let item = crate::complete::Item { label: prefix, kind: crate::complete::Kind::Snippet, insert: Some(body), detail: Some(preview), doc: None };
                    self.plug.snips.push((plugin, s("lang"), item));
                }
                "theme" => plugins::add_theme(&plugin, &s("key"), &f["def"]),
                "marks" => {
                    let path = self.tabs.get(self.active).map(|t| t.path.clone()).unwrap_or_default();
                    let list: Vec<crate::lint::Diag> = f["list"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|m| {
                                    let line = m["line"].as_u64()? as usize;
                                    let color = m["color"].as_str().and_then(|h| u32::from_str_radix(h.trim_start_matches('#'), 16).ok()).map(theme::hex);
                                    Some(crate::lint::Diag {
                                        line,
                                        col: m["col"].as_u64().unwrap_or(0) as usize,
                                        len: m["len"].as_u64().unwrap_or(0) as usize,
                                        err: m["kind"] == "error",
                                        msg: m["message"].as_str().unwrap_or("").to_string(),
                                        color,
                                    })
                                })
                                .take(2000)
                                .collect()
                        })
                        .unwrap_or_default();
                    self.plug.marks.insert(plugin, (path, list));
                }
                "store" => {
                    let (k, v) = (s("key"), f["value"].clone());
                    self.update_settings(|o| {
                        let mut all = o.get("pluginData").cloned().unwrap_or(json!({}));
                        all[&plugin][&k] = v;
                        o.insert("pluginData".into(), all);
                    });
                }
                _ => {}
            }
        }
    }

    // vloženie / nahradenie výberu v aktívnom súbore
    fn plug_edit(&mut self, text: &str, replace: bool) {
        if self.start || self.home {
            return;
        }
        let Some(tab) = self.tabs.get_mut(self.active) else { return };
        let (a, b) = self.sel_chars.unwrap_or((tab.text.chars().count(), tab.text.chars().count()));
        let (a, b) = if replace { (a.min(b), a.max(b)) } else { (a.max(b), a.max(b)) };
        let bi = |c: usize| tab.text.char_indices().nth(c).map(|(i, _)| i).unwrap_or(tab.text.len());
        let (ba, bb) = (bi(a), bi(b));
        // $0 / $1 ako v úryvkoch
        let line_start = tab.text[..ba].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let indent: String = tab.text[line_start..ba].chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        let (ins, stops) = crate::complete::expand(text, &indent, "    ");
        tab.text.replace_range(ba..bb, &ins);
        let c = a + stops.first().map(|s| s.1).unwrap_or(ins.chars().count());
        self.plug_cursor = Some(c);
        self.last_edit = Some(Instant::now());
    }

    // každú snímku: zmena textu (300 ms po písaní), výber, časovače, klávesové skratky príkazov, commands.run
    pub(super) fn plug_frame(&mut self, ctx: &egui::Context) {
        if self.plug.host.is_none() {
            return;
        }
        for id in std::mem::take(&mut self.plug_run) {
            if let Some(c) = self.plug.cmds.iter().find(|c| c.id == id).map(|c| (c.plugin.clone(), c.id.clone())) {
                self.plug_command(&c.0, &c.1);
            } else {
                self.act(&id, ctx);
            }
        }
        // skratky
        let hits: Vec<(String, String)> = self.plug.cmds.iter().filter_map(|c| c.key.filter(|(m, k)| ctx.input_mut(|i| i.consume_key(*m, *k))).map(|_| (c.plugin.clone(), c.id.clone()))).collect();
        for (p, id) in hits {
            self.plug_command(&p, &id);
        }
        let cur = (!self.start && !self.home).then(|| self.tabs.get(self.active).map(|t| (t.path.clone(), self.hls.get(&t.path).map(|h| h.gen).unwrap_or(0)))).flatten();
        if let Some((path, gen)) = cur.clone() {
            if self.plug.changed.as_ref().map(|c| (&c.0, c.1)) != Some((&path, gen)) {
                self.plug.changed = Some((path.clone(), gen, Instant::now()));
            }
            let due = self.plug.changed.as_ref().is_some_and(|c| c.2.elapsed() >= Duration::from_millis(300));
            if due && self.plug.sent.as_ref() != Some(&(path.clone(), gen)) {
                let first = self.plug.sent.as_ref().map(|s| &s.0) != Some(&path);
                self.plug.sent = Some((path.clone(), gen));
                if !first {
                    self.plug_emit("change");
                }
            } else if !due {
                ctx.request_repaint_after(Duration::from_millis(320));
            }
            let sel = (path, self.cursor.0 * 100_000 + self.cursor.1);
            if self.plug.sel.as_ref() != Some(&sel) {
                self.plug.sel = Some(sel);
                self.plug_emit("selection");
            }
        }
        let timers = self.plug.host.as_ref().map(|h| h.ids_with_timers()).unwrap_or_default();
        if !timers.is_empty() {
            if self.plug.tick.is_none_or(|t| t.elapsed() >= plugins::timer_period()) {
                self.plug.tick = Some(Instant::now());
                let ms = ctx.input(|i| i.time * 1000.0) as u64;
                for id in timers {
                    self.plug_call(&id, &format!("__tick({ms})"));
                }
            }
            ctx.request_repaint_after(plugins::timer_period());
        }
    }

    // ---------- Nastavenia → Pluginy ----------
    fn plug_catalog(&mut self) -> Option<Result<Vec<Value>, String>> {
        let slot = self.plug.catalog.get_or_insert_with(|| {
            let s: Arc<Mutex<Option<Result<Vec<Value>, String>>>> = Arc::default();
            let s2 = s.clone();
            std::thread::spawn(move || {
                let r = fetch_index();
                *s2.lock().unwrap() = Some(r);
            });
            s
        });
        slot.lock().unwrap().clone()
    }

    pub(super) fn plugins_ui(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        // riadok nastavení je vodorovný – obsah pod sebou
        ui.vertical(|ui| self.plugins_body(ui, w, ctx));
    }

    fn plugins_body(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        // riadok 1: Nainštalované / Obchod vľavo, Vytvoriť plugin + návod vpravo
        let (row, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
        bar.spacing_mut().item_spacing.x = 6.0;
        let st = self.plug.tab_store;
        if widgets::button(&mut bar, Some("puzzle"), &t("Installed"), if !st { p.accent } else { p.card2 }, if !st { p.accent_fg } else { p.text }, 30.0, &p).clicked() {
            self.plug.tab_store = false;
        }
        if widgets::button(&mut bar, Some("star"), &t("Store"), if st { p.accent } else { p.card2 }, if st { p.accent_fg } else { p.text }, 30.0, &p).clicked() {
            self.plug.tab_store = true;
        }
        let mut right = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)));
        right.spacing_mut().item_spacing.x = 6.0;
        if widgets::button(&mut right, Some("external"), &t("Plugin guide"), p.card2, p.text, 30.0, &p).clicked() {
            flux_core::settings::open_external("https://github.com/pantr1x/Flux/blob/main/docs/PLUGINS.md");
        }
        if widgets::button(&mut right, Some("plus"), &t("Create a plugin"), p.card2, p.text, 30.0, &p).clicked() {
            self.plug_create(ctx);
        }
        ui.add_space(10.0);
        if self.plug.tab_store {
            self.plug_store_ui(ui, w, ctx);
        } else {
            self.plug_installed_ui(ui, w, ctx);
        }
    }

    fn plug_line(&self, ui: &mut egui::Ui, w: f32, text: &str, font: egui::FontId, color: egui::Color32) {
        let (r, _) = ui.allocate_exact_size(vec2(w, font.size + 8.0), Sense::hover());
        widgets::text(ui, pos2(r.left() + 2.0, r.center().y), Align2::LEFT_CENTER, text, font, color, w - 4.0);
    }

    fn plug_card(&self, ui: &mut egui::Ui, w: f32, title: &str, sub: &str, desc: &str, err: Option<&str>) -> Rect {
        let p = self.pal;
        let h = if err.is_some() { 96.0 } else { 78.0 };
        let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        ui.painter().rect_filled(r, CornerRadius::same(14), p.hover);
        ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
        let ir = Rect::from_min_size(r.min + vec2(14.0, 14.0), vec2(34.0, 34.0));
        ui.painter().rect_filled(ir, CornerRadius::same(10), p.card2);
        widgets::icon_at(ui, ir.center(), 17.0, "puzzle", p.text);
        ui.painter().text(pos2(ir.right() + 12.0, ir.top() + 8.0), Align2::LEFT_CENTER, title, theme::bold(13.5), p.text);
        ui.painter().text(pos2(ir.right() + 12.0, ir.top() + 25.0), Align2::LEFT_CENTER, sub, theme::ui(11.0), p.text3);
        widgets::text(ui, pos2(ir.right() + 12.0, ir.top() + 44.0), Align2::LEFT_CENTER, desc, theme::ui(12.0), p.text2, w - 330.0);
        if let Some(e) = err {
            widgets::text(ui, pos2(ir.right() + 12.0, ir.top() + 64.0), Align2::LEFT_CENTER, e, theme::ui(12.0), p.red, w - 90.0);
        }
        ui.add_space(8.0);
        r
    }

    fn plug_installed_ui(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        let list: Vec<(Manifest, Option<String>, bool)> = plugins::Host::discover(&self.plug_dev_dirs())
            .into_iter()
            .map(|m| {
                let err = self.plug.host.as_ref().and_then(|h| h.list.iter().find(|x| x.man.id == m.id)).and_then(|x| x.error.clone());
                let on = self.plug_enabled(&m.id);
                (m, err, on)
            })
            .collect();
        if list.is_empty() {
            let (r, _) = ui.allocate_exact_size(vec2(w, 70.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::same(14), p.hover);
            ui.painter().text(r.center() - vec2(0.0, 9.0), Align2::CENTER_CENTER, t("No plugins yet."), theme::bold(13.0), p.text2);
            ui.painter().text(r.center() + vec2(0.0, 11.0), Align2::CENTER_CENTER, t("Open the Store, or create your own plugin."), theme::ui(12.0), p.text3);
            ui.add_space(8.0);
        }
        let mut act: Option<(String, &str)> = None;
        for (m, err, on) in &list {
            let sub = format!("{} · v{}{}", if m.publisher.is_empty() { "—" } else { &m.publisher }, m.version, if m.dev { format!(" · {}", t("from a folder")) } else { String::new() });
            let r = self.plug_card(ui, w, &m.name, &sub, &m.description, err.as_deref());
            let mut b = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(r.right() - 330.0, r.top() + 14.0), vec2(316.0, 30.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
            b.spacing_mut().item_spacing.x = 6.0;
            if widgets::button(&mut b, None, &if m.dev { t("Remove") } else { t("Uninstall") }, p.card2, p.text, 28.0, &p).clicked() {
                act = Some((m.id.clone(), "remove"));
            }
            if widgets::button(&mut b, Some("refresh"), &t("Reload"), p.card2, p.text, 28.0, &p).clicked() {
                act = Some((m.id.clone(), "reload"));
            }
            if widgets::button(&mut b, None, &if *on { t("Turn off") } else { t("Turn on") }, if *on { p.card2 } else { p.accent }, if *on { p.text } else { p.accent_fg }, 28.0, &p).clicked() {
                act = Some((m.id.clone(), "toggle"));
            }
        }
        // pre autorov: plugin z vlastného priečinka
        let (row, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        let mut b = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
        if widgets::button(&mut b, Some("folder"), &t("Load from folder"), p.card2, p.text, 30.0, &p).clicked() {
            if let Some(d) = rfd::FileDialog::new().set_title(t("Load from folder")).pick_folder() {
                self.plug_add_dev(&d.to_string_lossy());
            }
        }
        // výpis z console.log pluginov
        let log: Vec<String> = self.plug.host.as_ref().map(|h| h.log.iter().rev().take(8).cloned().collect()).unwrap_or_default();
        if !log.is_empty() {
            ui.add_space(6.0);
            self.plug_line(ui, w, &t("Plugin log").to_uppercase(), theme::bold(11.0), p.text3);
            for l in log {
                self.plug_line(ui, w, &l, theme::mono(11.5), p.text2);
            }
        }
        let Some((id, what)) = act else { return };
        let man = list.iter().find(|x| x.0.id == id).map(|x| x.0.clone());
        match what {
            "toggle" => {
                let on = !self.plug_enabled(&id);
                self.update_settings(|o| {
                    let mut all = o.get("nativePlugins").cloned().unwrap_or(json!({}));
                    all[&id]["enabled"] = json!(on);
                    o.insert("nativePlugins".into(), all);
                });
                self.plug_unload(&id);
                if let (true, Some(m)) = (on, man) {
                    self.plug_load(m);
                }
            }
            "reload" => {
                self.plug_unload(&id);
                if let Some(m) = man {
                    self.plug_load(m);
                    self.note(tf("{name} reloaded.", &[("name", &id)]));
                }
            }
            _ => {
                self.plug_unload(&id);
                if let Some(m) = man {
                    if m.dev {
                        let d = m.dir.to_string_lossy().to_string();
                        self.update_settings(|o| {
                            let mut a = o.get("pluginDevDirs").cloned().unwrap_or(json!([]));
                            if let Some(v) = a.as_array_mut() {
                                v.retain(|x| x.as_str() != Some(d.as_str()));
                            }
                            o.insert("pluginDevDirs".into(), a);
                        });
                    } else if m.dir.starts_with(plugins::installed_dir()) {
                        let _ = std::fs::remove_dir_all(&m.dir);
                    }
                }
                let _ = ctx;
            }
        }
    }

    fn plug_store_ui(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        let (sr, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        ui.painter().rect_filled(sr, CornerRadius::same(10), p.hover);
        ui.painter().rect_stroke(sr, CornerRadius::same(10), Stroke::new(1.0, p.line), StrokeKind::Inside);
        widgets::icon_at(ui, pos2(sr.left() + 16.0, sr.center().y), 14.0, "search", p.text3);
        let mut si = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(sr.left() + 32.0, sr.top() + 8.0), pos2(sr.right() - 10.0, sr.bottom() - 6.0))));
        si.add(egui::TextEdit::singleline(&mut self.plug.query).hint_text(t("Search plugins")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(13.0)));
        ui.add_space(8.0);
        let cat = self.plug_catalog();
        let list = match cat {
            None => {
                self.plug_line(ui, w, &t("Loading the store…"), theme::ui(12.5), p.text3);
                ctx.request_repaint_after(Duration::from_millis(200));
                return;
            }
            Some(Err(e)) => {
                self.plug_line(ui, w, &tf("The store could not be loaded: {e}", &[("e", &e)]), theme::ui(12.5), p.red);
                let (row, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
                let mut b = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
                if widgets::button(&mut b, Some("refresh"), &t("Try again"), p.card2, p.text, 28.0, &p).clicked() {
                    self.plug.catalog = None;
                }
                return;
            }
            Some(Ok(v)) => v,
        };
        let have: std::collections::HashMap<String, String> = plugins::Host::discover(&[]).into_iter().map(|m| (m.id, m.version)).collect();
        let q = self.plug.query.to_lowercase();
        let mut install = None;
        for e in list.iter().filter(|e| e["native"].as_bool() == Some(true)) {
            let name = e["name"].as_str().unwrap_or("").to_string();
            let id = e["id"].as_str().unwrap_or("").to_string();
            let desc = e["description"].as_str().unwrap_or("").to_string();
            if !q.is_empty() && !format!("{name} {desc} {id}").to_lowercase().contains(&q) {
                continue;
            }
            let ver = e["version"].as_str().unwrap_or("").to_string();
            let verified = e["verified"].as_bool() == Some(true);
            let sub = format!("{}{} · v{ver}", e["publisher"].as_str().unwrap_or(""), if verified { " ✓" } else { "" });
            let r = self.plug_card(ui, w, &name, &sub, &desc, None);
            let mut b = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(r.right() - 200.0, r.top() + 14.0), vec2(186.0, 30.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
            let busy = self.plug.installing.as_deref() == Some(id.as_str());
            match have.get(&id) {
                _ if busy => {
                    b.label(egui::RichText::new(t("Installing…")).font(theme::ui(12.5)).color(p.text3));
                }
                Some(v) if *v == ver => {
                    b.label(egui::RichText::new(t("Installed")).font(theme::bold(12.5)).color(p.green));
                }
                Some(_) => {
                    if widgets::button(&mut b, Some("refresh"), &t("Update"), p.accent, p.accent_fg, 28.0, &p).clicked() {
                        install = Some((id.clone(), verified));
                    }
                }
                None => {
                    if widgets::button(&mut b, Some("plus"), &t("Install"), p.accent, p.accent_fg, 28.0, &p).clicked() {
                        install = Some((id.clone(), verified));
                    }
                }
            }
        }
        if let Some((id, verified)) = install {
            self.plug.installing = Some(id.clone());
            match install_plugin(&id, verified) {
                Ok(dir) => {
                    self.plug_unload(&id);
                    if let Some(m) = plugins::read_manifest(&dir, false) {
                        self.plug_load(m);
                    }
                    self.note(tf("{name} installed.", &[("name", &id)]));
                }
                Err(e) => self.note(tf("Install failed: {e}", &[("e", &e)])),
            }
            self.plug.installing = None;
        }
    }

    pub(super) fn plug_test_install(&mut self, id: &str) {
        match install_plugin(id, true) {
            Ok(dir) => {
                self.plug_unload(id);
                if let Some(m) = plugins::read_manifest(&dir, false) {
                    self.plug_load(m);
                }
            }
            Err(e) => eprintln!("[flux] plugin install {id}: {e}"),
        }
    }

    pub(super) fn plug_add_dev(&mut self, d: &str) {
        let Some(m) = plugins::read_manifest(Path::new(d), true) else {
            self.note(t("This folder has no plugin.json with a \"native\" entry."));
            return;
        };
        let dd = d.to_string();
        self.update_settings(|o| {
            let mut a = o.get("pluginDevDirs").cloned().unwrap_or(json!([]));
            if let Some(v) = a.as_array_mut() {
                if !v.iter().any(|x| x.as_str() == Some(dd.as_str())) {
                    v.push(json!(dd));
                }
            }
            o.insert("pluginDevDirs".into(), a);
        });
        let id = m.id.clone();
        self.plug_unload(&id);
        self.plug_start();
        self.plug_load(m);
        self.note(tf("{name} loaded.", &[("name", &id)]));
    }

    // nový plugin: priečinok s plugin.json, native.js (príklad s komentármi), flux.d.ts, README – otvorí sa ako projekt
    fn plug_create(&mut self, ctx: &egui::Context) {
        let base = dirs::document_dir().unwrap_or_else(std::env::temp_dir).join("Flux Plugins");
        let mut dir = base.join("my-plugin");
        let mut n = 2;
        while dir.exists() {
            dir = base.join(format!("my-plugin-{n}"));
            n += 1;
        }
        let name = dir.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        let user = self.get("github")["user"]["login"].as_str().unwrap_or("me").to_lowercase();
        if std::fs::create_dir_all(&dir).is_err() {
            self.note(t("Could not create the folder."));
            return;
        }
        let man = json!({
            "id": format!("{user}.{name}"), "name": "My plugin", "publisher": user, "version": "1.0.0",
            "description": "One sentence about what it does.", "main": "native.js", "native": "native.js",
            "files": ["native.js", "README.md"], "tags": [], "license": "MIT"
        });
        let _ = std::fs::write(dir.join("plugin.json"), serde_json::to_string_pretty(&man).unwrap_or_default());
        let _ = std::fs::write(dir.join("native.js"), TEMPLATE);
        let _ = std::fs::write(dir.join("flux.d.ts"), include_str!("../../../docs/flux.d.ts"));
        let _ = std::fs::write(dir.join("README.md"), "# My plugin\n\nWhat it does and how to use it.\n");
        let d = dir.to_string_lossy().to_string();
        self.plug_add_dev(&d);
        self.open_folder(&d);
        self.open_file(&dir.join("native.js").to_string_lossy());
        self.settings = None;
        let _ = ctx;
    }
}

const TEMPLATE: &str = r#"// My plugin for Flux. Guide: https://github.com/pantr1x/Flux/blob/main/docs/PLUGINS.md
// Change this file, then press Reload next to the plugin in Settings → Plugins (or Ctrl+Shift+A → Reload plugins).

/** @param {import('./flux').Flux} flux */
export function activate(flux) {
  // 1. A command in Ctrl+Shift+A, with a shortcut
  flux.commands.register('hello', 'Say hello', () => {
    const file = flux.activeFile();
    flux.toast(file ? `Hello from ${file.name}!` : 'Hello!');
  }, { key: 'Ctrl+Alt+H' });

  // 2. A button in the status bar that shows the number of lines
  const lines = flux.statusBar.add({ text: '', title: 'Lines in this file' });
  const update = () => {
    const f = flux.activeFile();
    lines.set(f ? `${f.text.split('\n').length} lines` : '');
  };
  flux.onOpen(update);
  flux.onChange(update);
  update();

  // 3. A snippet: type "hello" in a Python file and press Enter
  flux.snippets.add('python', 'hello', 'print("Hello, ${1:world}!")$0', 'print("Hello, world!")');
}

export function deactivate() {}
"#;

fn registry_local() -> Option<PathBuf> {
    std::env::var("FLUX_PLUGIN_REGISTRY").ok().map(PathBuf::from)
}

const BRANCHES: [&str; 2] = ["main", "claude/optimistic-darwin-5i7m9t"];

fn fetch_text(rel: &str) -> Result<String, String> {
    if let Some(d) = registry_local() {
        return std::fs::read_to_string(d.join(rel)).map_err(|e| e.to_string());
    }
    let mut last = String::new();
    for b in BRANCHES {
        let url = format!("https://raw.githubusercontent.com/pantr1x/Flux/{b}/plugins/{rel}");
        match crate::net::request("GET", &url, &[], None, 30) {
            Ok((200, body)) => return Ok(body),
            Ok((code, _)) => last = format!("HTTP {code}"),
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn fetch_index() -> Result<Vec<Value>, String> {
    let v: Value = serde_json::from_str(&fetch_text("index.json")?).map_err(|e| e.to_string())?;
    Ok(v["plugins"].as_array().cloned().unwrap_or_default())
}

// stiahne súbory pluginu do userData/native-plugins/<id> (najprv do .tmp, potom výmena)
fn install_plugin(id: &str, verified: bool) -> Result<PathBuf, String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("bad id".into());
    }
    let man: Value = serde_json::from_str(&fetch_text(&format!("{id}/plugin.json"))?).map_err(|e| e.to_string())?;
    let entry = man["native"].as_str().map(String::from).or_else(|| (man["native"] == true).then(|| man["main"].as_str().unwrap_or("plugin.js").to_string())).ok_or("not a Flux Native plugin")?;
    let mut files: Vec<String> = man["files"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    if !files.contains(&entry) {
        files.push(entry.clone());
    }
    let tmp = plugins::installed_dir().join(format!("{id}.tmp"));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    std::fs::write(tmp.join("plugin.json"), serde_json::to_string_pretty(&man).unwrap_or_default()).map_err(|e| e.to_string())?;
    for f in &files {
        let ok_ext = [".js", ".json", ".md", ".css", ".svg", ".txt"].iter().any(|x| f.ends_with(x));
        let Some(dst) = file_in(&tmp, f).filter(|_| ok_ext) else { continue };
        let text = fetch_text(&format!("{id}/{f}"))?;
        if f.ends_with(".js") && !verified {
            let warn = plugins::scan(&text);
            if !warn.is_empty() {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(format!("{f}: {}", warn.join(", ")));
            }
        }
        if let Some(parent) = dst.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&dst, text).map_err(|e| e.to_string())?;
    }
    let dir = plugins::installed_dir().join(id);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&tmp, &dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

impl App {
    // stavový riadok: tlačidlá z pluginov (pred Auto save); vráti šírku, ktorú zabrali
    pub(super) fn plug_status_ui(&mut self, ui: &mut egui::Ui, right: f32, y: f32) -> f32 {
        let p = self.pal;
        let mut x = right;
        let mut click = None;
        for (i, s) in self.plug.status.iter().enumerate().rev() {
            if !s.show || s.text.is_empty() {
                continue;
            }
            let w = widgets::text_w(ui, &s.text, theme::ui(12.0)) + 14.0;
            let r = Rect::from_min_size(pos2(x - w, y - 10.0), vec2(w, 20.0));
            let resp = ui.interact(r, ui.id().with(("plug-st", i)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, CornerRadius::same(6), p.hover);
            }
            ui.painter().text(r.center(), Align2::CENTER_CENTER, &s.text, theme::ui(12.0), p.text2);
            let resp = if s.title.is_empty() { resp } else { resp.on_hover_text(&s.title) };
            if resp.clicked() {
                click = Some((s.plugin.clone(), s.k.clone()));
            }
            x -= w + 4.0;
        }
        if let Some((pl, k)) = click {
            self.plug_call(&pl, &format!("__click({})", json!(k)));
        }
        right - x
    }
}
