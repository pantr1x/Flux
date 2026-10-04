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
    pub detail: Option<String>,        // otvorená stránka pluginu
    pub detail_card: Option<PlugCard>, // jej posledný stav
    pub icons: std::collections::HashMap<String, Arc<Mutex<Option<Option<String>>>>>, // id → SVG ikony
    pub readmes: std::collections::HashMap<String, Arc<Mutex<Option<Option<String>>>>>, // id → README.md
}

// jedna karta na stránke pluginov (zabudovaný GitHub, nainštalovaný plugin alebo položka obchodu)
#[derive(Clone, Debug)]
pub struct PlugCard {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub desc: String,
    pub verified: bool,
    pub tags: Vec<String>,
    pub icon: String,
    pub dir: Option<PathBuf>,
    pub installed: Option<Manifest>,
    pub update: bool,
    pub on: bool,
    pub err: Option<String>,
}

impl PlugCard {
    fn from_entry(e: &Value, man: Option<Manifest>) -> Self {
        let s = |k: &str| e[k].as_str().unwrap_or("").to_string();
        let version = s("version");
        PlugCard {
            id: man.as_ref().map(|m| m.id.clone()).unwrap_or_else(|| s("id")),
            name: s("name"),
            publisher: s("publisher"),
            update: man.as_ref().is_some_and(|m| !version.is_empty() && super::prefs::ver_key(&version) > super::prefs::ver_key(&m.version)),
            version,
            desc: s("description"),
            verified: e["verified"].as_bool() == Some(true),
            tags: e["tags"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
            icon: e["icon"].as_str().unwrap_or("icon.svg").to_string(),
            dir: man.as_ref().map(|m| m.dir.clone()),
            on: true,
            installed: man,
            err: None,
        }
    }
    fn github(on: bool) -> Self {
        PlugCard {
            id: "github".into(),
            name: "GitHub".into(),
            publisher: "Flux".into(),
            version: String::new(),
            desc: t("Open your repositories as projects and sign in with your GitHub account."),
            verified: true,
            tags: vec![t("Built in")],
            icon: String::new(),
            dir: None,
            installed: None,
            update: false,
            on,
            err: None,
        }
    }
    fn sub(&self) -> String {
        let mut v = vec![if self.publisher.is_empty() { "—".to_string() } else { self.publisher.clone() }];
        if !self.version.is_empty() {
            v.push(format!("v{}", self.version));
        }
        if self.installed.as_ref().is_some_and(|m| m.dev) {
            v.push(t("from a folder"));
        }
        v.join(" · ")
    }
}

// <text> z jednoduchej SVG ikony: (text, farba, veľkosť, x, y, šírka viewBoxu)
fn svg_text(svg: &str) -> Option<(String, egui::Color32, f32, f32, f32, f32)> {
    let attr = |tag: &str, name: &str| -> Option<String> {
        let i = tag.find(&format!(" {name}=\""))? + name.len() + 3;
        Some(tag[i..].split('"').next()?.to_string())
    };
    let vb: f32 = attr(svg, "viewBox").and_then(|v| v.split_whitespace().nth(2).and_then(|x| x.parse().ok())).unwrap_or(24.0);
    let s = svg.find("<text")?;
    let open_end = svg[s..].find('>')? + s;
    let tag = &svg[s..open_end];
    let close = svg[open_end..].find("</text>")? + open_end;
    let txt = svg[open_end + 1..close].trim().to_string();
    let fill = attr(tag, "fill").and_then(|f| u32::from_str_radix(f.trim_start_matches('#'), 16).ok()).unwrap_or(0xffffff);
    let col = egui::Color32::from_rgb((fill >> 16) as u8, (fill >> 8) as u8, fill as u8);
    let size: f32 = attr(tag, "font-size").and_then(|v| v.parse().ok()).unwrap_or(vb / 3.0);
    let x: f32 = attr(tag, "x").and_then(|v| v.parse().ok()).unwrap_or(vb / 2.0);
    let y: f32 = attr(tag, "y").and_then(|v| v.parse().ok()).unwrap_or(vb / 2.0);
    Some((txt, col, size, x, y + size * 0.2, vb))
}

// jednoduchý Markdown pre README pluginov: # nadpisy, odrážky, ```kód```, odseky (**tučné**, *kurzíva*, `kód`)
fn markdown(ui: &mut egui::Ui, md: &str, w: f32, p: &theme::Pal) {
    let mut code = false;
    let mut first_h1 = true;
    let mut table: Vec<Vec<String>> = vec![];
    let lines: Vec<&str> = md.lines().chain(std::iter::once("")).collect();
    for line in lines {
        // tabuľka: riadky „| a | b |“ sa zbierajú a nakreslia naraz
        if !code && line.trim_start().starts_with('|') {
            let cells: Vec<String> = line.trim().trim_matches('|').split('|').map(|c| c.trim().to_string()).collect();
            if !cells.iter().all(|c| c.chars().all(|ch| matches!(ch, '-' | ':' | ' ')) ) {
                table.push(cells);
            }
            continue;
        }
        if !table.is_empty() {
            md_table(ui, &std::mem::take(&mut table), w, p);
        }
        if line.trim_start().starts_with("```") {
            code = !code;
            ui.add_space(4.0);
            continue;
        }
        if code {
            let (r, _) = ui.allocate_exact_size(vec2(w, 19.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::ZERO, p.hover);
            widgets::text(ui, pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, line, theme::mono(12.0), p.text, w - 24.0);
            continue;
        }
        if line.trim().is_empty() {
            ui.add_space(6.0);
            continue;
        }
        if line.starts_with("![") {
            continue; // obrázky zatiaľ nie
        }
        if line.starts_with("# ") && std::mem::take(&mut first_h1) {
            continue; // meno pluginu je už v hlavičke
        }
        let (size, head, bullet, text) = if let Some(h) = line.strip_prefix("# ") {
            (20.0, true, false, h)
        } else if let Some(h) = line.strip_prefix("## ") {
            ui.add_space(8.0);
            (16.0, true, false, h)
        } else if let Some(h) = line.strip_prefix("### ") {
            ui.add_space(6.0);
            (14.0, true, false, h)
        } else if let Some(b) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            (13.0, false, true, b)
        } else {
            (13.0, false, false, line)
        };
        let indent = if bullet { 20.0 } else { 0.0 };
        let lh = (size * 1.5f32).round();
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = (w - indent).max(60.0);
        for (part, k) in super::prefs::spans(text) {
            let base = egui::TextFormat { font_id: if head { theme::bold(size) } else { theme::ui(size) }, color: if head { p.text } else { p.text2 }, line_height: Some(lh), ..Default::default() };
            let f = match k {
                super::prefs::Span::Plain => base,
                super::prefs::Span::Bold => egui::TextFormat { font_id: theme::bold(size), color: p.text, ..base },
                super::prefs::Span::Italic => egui::TextFormat { italics: true, ..base },
                super::prefs::Span::Code => egui::TextFormat { font_id: theme::mono(size - 1.0), color: p.text, background: p.hover, ..base },
            };
            job.append(&part, 0.0, f);
        }
        let g = ui.fonts_mut(|f| f.layout_job(job));
        let first = g.rows.first().map(|r| r.rect().height()).unwrap_or(lh);
        let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y + 4.0), Sense::hover());
        if bullet {
            ui.painter().circle_filled(pos2(r.left() + 8.0, r.top() + first / 2.0), 2.4, p.accent.gamma_multiply(0.8));
        }
        ui.painter().galley(pos2(r.left() + indent, r.top()), g, p.text2);
    }
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

    // všetky karty: zabudovaný GitHub, nainštalované pluginy a obchod (podľa karty a hľadania)
    fn plug_cards(&mut self, store: bool) -> Option<Vec<PlugCard>> {
        let installed: Vec<Manifest> = plugins::Host::discover(&self.plug_dev_dirs());
        let cat: Vec<Value> = match self.plug_catalog() {
            Some(Ok(v)) => v,
            _ if store => return None,
            _ => vec![],
        };
        let entry = |id: &str| cat.iter().find(|e| e["id"].as_str() == Some(id)).cloned();
        let mut out = vec![];
        if store {
            for e in cat.iter().filter(|e| e["native"].as_bool() == Some(true)) {
                let id = e["id"].as_str().unwrap_or("").to_string();
                let man = installed.iter().find(|m| m.id == id).cloned();
                out.push(PlugCard::from_entry(e, man));
            }
        } else {
            out.push(PlugCard::github(self.gh_plugin()));
            for m in installed {
                let raw: Value = std::fs::read_to_string(m.dir.join("plugin.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
                let mut c = PlugCard::from_entry(&entry(&m.id).unwrap_or(raw), Some(m.clone()));
                c.name = m.name.clone();
                c.version = m.version.clone();
                c.desc = m.description.clone();
                if c.publisher.is_empty() {
                    c.publisher = m.publisher.clone();
                }
                c.err = self.plug.host.as_ref().and_then(|h| h.list.iter().find(|x| x.man.id == m.id)).and_then(|x| x.error.clone());
                c.on = self.plug_enabled(&m.id);
                out.push(c);
            }
        }
        let q = self.plug.query.to_lowercase();
        if !q.is_empty() {
            out.retain(|c| format!("{} {} {} {}", c.name, c.desc, c.id, c.tags.join(" ")).to_lowercase().contains(&q));
        }
        Some(out)
    }

    fn plugins_body(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        if let Some(id) = self.plug.detail.clone() {
            self.plug_detail_ui(ui, w, ctx, &id);
            return;
        }
        // horný riadok: Nainštalované / Obchod vľavo, hľadanie vpravo (ako rozšírenia vo VS Code)
        let installed_n = plugins::Host::discover(&self.plug_dev_dirs()).len() + usize::from(self.gh_plugin());
        let st = self.plug.tab_store;
        let labels = [(false, "puzzle", tf("Installed ({n})", &[("n", &installed_n.to_string())])), (true, "star", t("Store"))];
        let seg_w: Vec<f32> = labels.iter().map(|l| widgets::text_w(ui, &l.2, theme::bold(13.0)) + 48.0).collect();
        let (row, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
        let bar = Rect::from_min_size(row.min, vec2(seg_w.iter().sum::<f32>() + 8.0, 40.0));
        ui.painter().rect_filled(bar, CornerRadius::same(12), p.hover);
        ui.painter().rect_stroke(bar, CornerRadius::same(12), Stroke::new(1.0, p.line), StrokeKind::Inside);
        let mut x = bar.left() + 4.0;
        for (i, (store, icon, label)) in labels.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, bar.top() + 4.0), vec2(seg_w[i], 32.0));
            x += seg_w[i];
            let resp = ui.interact(r, ui.id().with(("plug-seg", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
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
                self.plug.tab_store = *store;
            }
        }
        let sw = (w - bar.width() - 16.0).clamp(160.0, 320.0);
        let sr = Rect::from_min_size(pos2(row.right() - sw, row.top()), vec2(sw, 40.0));
        ui.painter().rect_filled(sr, CornerRadius::same(12), p.hover);
        ui.painter().rect_stroke(sr, CornerRadius::same(12), Stroke::new(1.0, p.line), StrokeKind::Inside);
        widgets::icon_at(ui, pos2(sr.left() + 18.0, sr.center().y), 14.0, "search", p.text3);
        let mut si = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(sr.left() + 34.0, sr.top() + 11.0), pos2(sr.right() - 10.0, sr.bottom() - 8.0))));
        si.add(egui::TextEdit::singleline(&mut self.plug.query).hint_text(t("Search plugins")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).font(theme::ui(13.0)));
        ui.add_space(16.0);

        let store = self.plug.tab_store;
        match self.plug_cards(store) {
            None => match self.plug_catalog() {
                Some(Err(e)) => {
                    self.plug_line(ui, w, &tf("The store could not be loaded: {e}", &[("e", &e)]), theme::ui(12.5), p.red);
                    let (row, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
                    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
                    if widgets::button(&mut b, Some("refresh"), &t("Try again"), p.card2, p.text, 30.0, &p).clicked() {
                        self.plug.catalog = None;
                    }
                }
                _ => {
                    self.plug_line(ui, w, &t("Loading the store…"), theme::ui(12.5), p.text3);
                    ctx.request_repaint_after(Duration::from_millis(200));
                }
            },
            Some(cards) => {
                if cards.is_empty() {
                    self.plug_empty(ui, w, store);
                } else {
                    self.plug_grid(ui, w, ctx, &cards, store);
                }
            }
        }
        // výpis z console.log pluginov
        let log: Vec<String> = self.plug.host.as_ref().map(|h| h.log.iter().rev().take(8).cloned().collect()).unwrap_or_default();
        if !log.is_empty() && !store {
            ui.add_space(8.0);
            self.plug_line(ui, w, &t("Plugin log").to_uppercase(), theme::bold(11.0), p.text3);
            for l in log {
                self.plug_line(ui, w, &l, theme::mono(11.5), p.text2);
            }
        }
        // pre autorov pluginov: oddelene dole
        ui.add_space(22.0);
        let (hr, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
        ui.painter().hline(hr.x_range(), hr.center().y, Stroke::new(1.0, p.line));
        ui.add_space(14.0);
        self.plug_line(ui, w, &t("Make your own").to_uppercase(), theme::bold(11.0), p.text3);
        self.plug_line(ui, w, &t("Plugins are small JavaScript files. Start from a working example and change it."), theme::ui(12.0), p.text3);
        ui.add_space(6.0);
        let (row, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        let mut b = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
        b.spacing_mut().item_spacing.x = 8.0;
        if widgets::button(&mut b, Some("plus"), &t("Create a plugin"), p.card2, p.text, 32.0, &p).clicked() {
            self.plug_create(ctx);
        }
        if widgets::button(&mut b, Some("folder"), &t("Load from folder"), p.card2, p.text, 32.0, &p).clicked() {
            if let Some(d) = rfd::FileDialog::new().set_title(t("Load from folder")).pick_folder() {
                self.plug_add_dev(&d.to_string_lossy());
            }
        }
        if widgets::button(&mut b, Some("external"), &t("Plugin guide"), p.card2, p.text, 32.0, &p).clicked() {
            flux_core::settings::open_external("https://github.com/pantr1x/Flux/blob/main/docs/PLUGINS.md");
        }
    }

    fn plug_line(&self, ui: &mut egui::Ui, w: f32, text: &str, font: egui::FontId, color: egui::Color32) {
        let (r, _) = ui.allocate_exact_size(vec2(w, font.size + 8.0), Sense::hover());
        widgets::text(ui, pos2(r.left() + 2.0, r.center().y), Align2::LEFT_CENTER, text, font, color, w - 4.0);
    }

    fn plug_empty(&mut self, ui: &mut egui::Ui, w: f32, store: bool) {
        let p = self.pal;
        let (r, _) = ui.allocate_exact_size(vec2(w, 150.0), Sense::hover());
        ui.painter().rect_filled(r, CornerRadius::same(16), p.hover);
        ui.painter().rect_stroke(r, CornerRadius::same(16), Stroke::new(1.0, p.line), StrokeKind::Inside);
        widgets::icon_at(ui, pos2(r.center().x, r.top() + 34.0), 24.0, if store { "search" } else { "puzzle" }, p.text3);
        let (a, b) = if self.plug.query.is_empty() {
            (t("No plugins yet."), t("Open the Store, or create your own plugin."))
        } else {
            (t("Nothing found."), t("Try another word."))
        };
        ui.painter().text(pos2(r.center().x, r.top() + 66.0), Align2::CENTER_CENTER, a, theme::bold(14.0), p.text);
        ui.painter().text(pos2(r.center().x, r.top() + 88.0), Align2::CENTER_CENTER, b, theme::ui(12.5), p.text3);
        if !store && self.plug.query.is_empty() {
            let label = t("Open the Store");
            let bw = widgets::text_w(ui, &label, theme::bold(13.0)) + 46.0;
            let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_center_size(pos2(r.center().x, r.top() + 122.0), vec2(bw, 32.0))));
            if widgets::button(&mut bu, Some("star"), &label, p.accent, p.accent_fg, 32.0, &p).clicked() {
                self.plug.tab_store = true;
            }
        }
    }

    // ikona pluginu (icon.svg z priečinka alebo z obchodu); kým nie je, farebné písmeno
    fn plug_icon(&mut self, ui: &egui::Ui, c: &PlugCard, r: Rect) {
        let p = self.pal;
        if c.id == "github" {
            ui.painter().rect_filled(r, CornerRadius::same((r.width() * 0.28) as u8), p.card2);
            widgets::icon_at(ui, r.center(), r.width() * 0.5, "github", p.text);
            return;
        }
        let slot = self
            .plug
            .icons
            .entry(c.id.clone())
            .or_insert_with(|| {
                let s: Arc<Mutex<Option<Option<String>>>> = Arc::default();
                let (s2, id, file, dir) = (s.clone(), c.id.clone(), c.icon.clone(), c.dir.clone());
                std::thread::spawn(move || {
                    let svg = if file.ends_with(".svg") {
                        dir.and_then(|d| std::fs::read_to_string(d.join(&file)).ok()).or_else(|| fetch_text(&format!("{id}/{file}")).ok())
                    } else {
                        None
                    };
                    *s2.lock().unwrap() = Some(svg.filter(|s| s.contains("<svg")));
                });
                s
            })
            .clone();
        let svg = slot.lock().unwrap().clone();
        match svg {
            Some(Some(svg)) => {
                ui.painter().rect_filled(r, CornerRadius::same((r.width() * 0.28) as u8), p.card2);
                egui::Image::from_bytes(format!("bytes://plugicon-{}.svg", c.id), svg.clone().into_bytes())
                    .corner_radius(CornerRadius::same((r.width() * 0.28) as u8))
                    .paint_at(ui, r);
                // <text> v SVG resvg nekreslí – nakreslí ho egui (napr. „123“ vo Word Count)
                if let Some((txt, fill, size, x, y, vb)) = svg_text(&svg) {
                    let k = r.width() / vb;
                    ui.painter().text(r.min + vec2(x * k, y * k), Align2::CENTER_BOTTOM, txt, theme::bold(size * k * 0.95), fill);
                }
            }
            Some(None) => {
                let hue = c.name.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32)) % 360;
                let col: egui::Color32 = egui::ecolor::Hsva::new(hue as f32 / 360.0, 0.45, 0.75, 1.0).into();
                ui.painter().rect_filled(r, CornerRadius::same((r.width() * 0.28) as u8), col.gamma_multiply(0.22));
                let mono: String = c.name.chars().next().map(|ch| ch.to_uppercase().collect()).unwrap_or_default();
                ui.painter().text(r.center(), Align2::CENTER_CENTER, mono, theme::bold(r.width() * 0.42), col);
            }
            None => {
                ui.painter().rect_filled(r, CornerRadius::same((r.width() * 0.28) as u8), p.card2);
                ui.ctx().request_repaint_after(Duration::from_millis(150));
            }
        }
    }

    // mriežka kariet (2 stĺpce, na úzkom okne 1): ikona, meno ✓, vydavateľ · verzia, popis, štítky, akcia; klik = detail
    fn plug_grid(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context, cards: &[PlugCard], store: bool) {
        let p = self.pal;
        let cols = if w >= 620.0 { 2 } else { 1 };
        let gap = 12.0;
        let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = 148.0;
        let rows = cards.len().div_ceil(cols);
        let (area, _) = ui.allocate_exact_size(vec2(w, rows as f32 * (ch + gap) - gap), Sense::hover());
        let mut act: Option<(String, &str)> = None;
        for (i, c) in cards.iter().enumerate() {
            let r = Rect::from_min_size(area.min + vec2((i % cols) as f32 * (cw + gap), (i / cols) as f32 * (ch + gap)), vec2(cw, ch));
            let resp = ui.interact(r, ui.id().with(("plug-card", &c.id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
            ui.painter().rect_filled(r, CornerRadius::same(16), p.hover.lerp_to_gamma(p.active, hk * 0.6));
            ui.painter().rect_stroke(r, CornerRadius::same(16), Stroke::new(1.0, p.line.lerp_to_gamma(p.line_strong, hk)), StrokeKind::Inside);
            let ir = Rect::from_min_size(r.min + vec2(16.0, 16.0), vec2(44.0, 44.0));
            self.plug_icon(ui, c, ir);
            let tx = ir.right() + 12.0;
            let nw = widgets::text_w(ui, &c.name, theme::bold(14.0)).min(cw - 96.0);
            widgets::text(ui, pos2(tx, ir.top() + 12.0), Align2::LEFT_CENTER, &c.name, theme::bold(14.0), p.text, cw - 96.0);
            if c.verified {
                ui.painter().circle_filled(pos2(tx + nw + 11.0, ir.top() + 12.0), 7.0, p.accent);
                widgets::icon_at(ui, pos2(tx + nw + 11.0, ir.top() + 12.0), 9.0, "check", p.accent_fg);
            }
            widgets::text(ui, pos2(tx, ir.top() + 32.0), Align2::LEFT_CENTER, &c.sub(), theme::ui(11.5), p.text3, cw - 96.0);
            let mut job = egui::text::LayoutJob::single_section(c.desc.clone(), egui::TextFormat { font_id: theme::ui(12.5), color: p.text2, ..Default::default() });
            job.wrap.max_width = cw - 32.0;
            job.wrap.max_rows = 2;
            let g = ui.fonts_mut(|f| f.layout_job(job));
            ui.painter().galley(pos2(r.left() + 16.0, ir.bottom() + 10.0), g, p.text2);
            // dole: štítky / chyba vľavo, akcia vpravo
            let by = r.bottom() - 26.0;
            if let Some(e) = &c.err {
                widgets::text(ui, pos2(r.left() + 16.0, by), Align2::LEFT_CENTER, e, theme::ui(11.5), p.red, cw - 150.0);
            } else {
                let mut tx = r.left() + 16.0;
                for tag in c.tags.iter().take(3) {
                    let tw = widgets::text_w(ui, tag, theme::ui(11.0)) + 16.0;
                    if tx + tw > r.right() - 140.0 {
                        break;
                    }
                    let tr = Rect::from_min_size(pos2(tx, by - 10.0), vec2(tw, 20.0));
                    ui.painter().rect_filled(tr, CornerRadius::same(10), p.card2);
                    ui.painter().text(tr.center(), Align2::CENTER_CENTER, tag, theme::ui(11.0), p.text3);
                    tx += tw + 6.0;
                }
            }
            let ar = Rect::from_min_size(pos2(r.right() - 150.0, by - 15.0), vec2(136.0, 30.0));
            let mut b = ui.new_child(egui::UiBuilder::new().max_rect(ar).layout(egui::Layout::right_to_left(egui::Align::Center)));
            if let Some(a) = self.plug_action(&mut b, c, store) {
                act = Some((c.id.clone(), a));
            } else if resp.clicked() {
                self.plug.detail = Some(c.id.clone());
                self.plug.detail_card = Some(c.clone());
            }
        }
        if let Some((id, a)) = act {
            if let Some(c) = cards.iter().find(|c| c.id == id) {
                self.plug_do(c, a, ctx);
            }
        }
    }

    // tlačidlo / prepínač karty; vráti akciu
    fn plug_action(&self, b: &mut egui::Ui, c: &PlugCard, store: bool) -> Option<&'static str> {
        let p = self.pal;
        let busy = self.plug.installing.as_deref() == Some(c.id.as_str());
        if busy {
            b.label(egui::RichText::new(t("Installing…")).font(theme::ui(12.5)).color(p.text3));
            return None;
        }
        if c.id == "github" {
            let on = c.on;
            return widgets::button(b, None, &if on { t("Remove") } else { t("Install") }, if on { p.card2 } else { p.accent }, if on { p.text } else { p.accent_fg }, 30.0, &p).clicked().then_some("github");
        }
        if store || c.installed.is_none() {
            return match (&c.installed, c.update) {
                (None, _) => widgets::button(b, Some("plus"), &t("Install"), p.accent, p.accent_fg, 30.0, &p).clicked().then_some("install"),
                (Some(_), true) => widgets::button(b, Some("refresh"), &t("Update"), p.accent, p.accent_fg, 30.0, &p).clicked().then_some("install"),
                (Some(_), false) => {
                    let lw = widgets::text_w(b, &t("Installed"), theme::bold(12.5));
                    let x = b.max_rect().right() - lw - 22.0;
                    widgets::icon_at(b, pos2(x, b.max_rect().center().y), 14.0, "check", p.green);
                    b.painter().text(pos2(x + 12.0, b.max_rect().center().y), Align2::LEFT_CENTER, t("Installed"), theme::bold(12.5), p.green);
                    None
                }
            };
        }
        let mut on = c.on;
        if super::prefs::switch(b, &mut on, &p, self.anim_on()) {
            return Some("toggle");
        }
        if c.update && widgets::button(b, Some("refresh"), &t("Update"), p.accent, p.accent_fg, 28.0, &p).clicked() {
            return Some("install");
        }
        None
    }

    // vykoná akciu z karty alebo detailu
    fn plug_do(&mut self, c: &PlugCard, what: &str, ctx: &egui::Context) {
        let id = c.id.clone();
        let man = c.installed.clone();
        match what {
            "github" => {
                let on = !self.gh_plugin();
                self.set("githubPlugin", json!(on), ctx);
                self.note(if on { t("GitHub plugin installed. Find it in Settings → GitHub.") } else { t("GitHub plugin removed.") });
            }
            "install" => {
                self.plug.installing = Some(id.clone());
                match install_plugin(&id, c.verified) {
                    Ok(dir) => {
                        self.plug_unload(&id);
                        let mut name = c.name.clone();
                        if let Some(m) = plugins::read_manifest(&dir, false) {
                            name = m.name.clone();
                            self.plug_load(m);
                        }
                        self.plug.icons.remove(&id);
                        self.note(tf("{name} installed.", &[("name", &name)]));
                    }
                    Err(e) => self.note(tf("Install failed: {e}", &[("e", &e)])),
                }
                self.plug.installing = None;
            }
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
                    self.note(tf("{name} reloaded.", &[("name", &c.name)]));
                }
            }
            _ => {
                // odinštalovať / odobrať priečinok
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
                    self.note(tf("{name} removed.", &[("name", &c.name)]));
                }
            }
        }
    }

    // stránka pluginu (ako rozšírenie vo VS Code): veľká ikona, meno, akcie, štítky a README
    fn plug_detail_ui(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context, id: &str) {
        let p = self.pal;
        // aktuálny stav karty (po inštalácii / vypnutí sa mení)
        let fresh = self.plug_cards(false).into_iter().flatten().chain(self.plug_cards(true).into_iter().flatten()).find(|c| c.id == id);
        let Some(c) = fresh.or_else(|| self.plug.detail_card.clone()) else {
            self.plug.detail = None;
            return;
        };
        let (row, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
        let mut b = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
        if widgets::button(&mut b, None, &format!("← {}", t("All plugins")), p.card2, p.text, 30.0, &p).clicked() || ui.input(|i| i.key_pressed(egui::Key::Backspace) && i.modifiers.alt) {
            self.plug.detail = None;
        }
        ui.add_space(14.0);
        let (hr, _) = ui.allocate_exact_size(vec2(w, 96.0), Sense::hover());
        let ir = Rect::from_min_size(hr.min + vec2(0.0, 4.0), vec2(80.0, 80.0));
        self.plug_icon(ui, &c, ir);
        let tx = ir.right() + 18.0;
        let nw = widgets::text_w(ui, &c.name, theme::bold(22.0));
        ui.painter().text(pos2(tx, ir.top() + 16.0), Align2::LEFT_CENTER, &c.name, theme::bold(22.0), p.text);
        if c.verified {
            ui.painter().circle_filled(pos2(tx + nw + 14.0, ir.top() + 16.0), 9.0, p.accent);
            widgets::icon_at(ui, pos2(tx + nw + 14.0, ir.top() + 16.0), 11.0, "check", p.accent_fg);
        }
        widgets::text(ui, pos2(tx, ir.top() + 42.0), Align2::LEFT_CENTER, &c.sub(), theme::ui(12.5), p.text3, w - tx + hr.left());
        let mut job = egui::text::LayoutJob::single_section(c.desc.clone(), egui::TextFormat { font_id: theme::ui(13.5), color: p.text2, ..Default::default() });
        job.wrap.max_width = w - (tx - hr.left());
        job.wrap.max_rows = 2;
        let g = ui.fonts_mut(|f| f.layout_job(job));
        ui.painter().galley(pos2(tx, ir.top() + 56.0), g, p.text2);
        // akcie
        ui.add_space(4.0);
        let (row, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
        let mut b = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
        b.spacing_mut().item_spacing.x = 8.0;
        let mut act = None;
        if self.plug.installing.as_deref() == Some(id) {
            b.label(egui::RichText::new(t("Installing…")).font(theme::ui(13.0)).color(p.text3));
        } else if c.id == "github" {
            if widgets::button(&mut b, None, &if c.on { t("Remove") } else { t("Install") }, if c.on { p.card2 } else { p.accent }, if c.on { p.text } else { p.accent_fg }, 34.0, &p).clicked() {
                act = Some("github");
            }
        } else if c.installed.is_none() {
            if widgets::button(&mut b, Some("plus"), &t("Install"), p.accent, p.accent_fg, 34.0, &p).clicked() {
                act = Some("install");
            }
        } else {
            if c.update && widgets::button(&mut b, Some("refresh"), &t("Update"), p.accent, p.accent_fg, 34.0, &p).clicked() {
                act = Some("install");
            }
            if widgets::button(&mut b, None, &if c.on { t("Turn off") } else { t("Turn on") }, if c.on { p.card2 } else { p.accent }, if c.on { p.text } else { p.accent_fg }, 34.0, &p).clicked() {
                act = Some("toggle");
            }
            if widgets::button(&mut b, Some("refresh"), &t("Reload"), p.card2, p.text, 34.0, &p).clicked() {
                act = Some("reload");
            }
            if widgets::button(&mut b, None, &if c.installed.as_ref().is_some_and(|m| m.dev) { t("Remove") } else { t("Uninstall") }, p.card2, p.text, 34.0, &p).clicked() {
                act = Some("remove");
            }
        }
        if let Some(e) = &c.err {
            ui.add_space(6.0);
            self.plug_line(ui, w, e, theme::ui(12.5), p.red);
        }
        // štítky
        if !c.tags.is_empty() {
            ui.add_space(10.0);
            let (tr, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
            let mut x = tr.left();
            for tag in &c.tags {
                let tw = widgets::text_w(ui, tag, theme::ui(11.5)) + 18.0;
                let r = Rect::from_min_size(pos2(x, tr.top()), vec2(tw, 22.0));
                ui.painter().rect_filled(r, CornerRadius::same(11), p.card2);
                ui.painter().text(r.center(), Align2::CENTER_CENTER, tag, theme::ui(11.5), p.text3);
                x += tw + 6.0;
            }
        }
        ui.add_space(16.0);
        let (hl, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
        ui.painter().hline(hl.x_range(), hl.center().y, Stroke::new(1.0, p.line));
        ui.add_space(14.0);
        // README (z priečinka alebo z obchodu)
        let slot = self
            .plug
            .readmes
            .entry(c.id.clone())
            .or_insert_with(|| {
                let s: Arc<Mutex<Option<Option<String>>>> = Arc::default();
                let (s2, id, dir, builtin) = (s.clone(), c.id.clone(), c.dir.clone(), c.id == "github");
                std::thread::spawn(move || {
                    let r = if builtin {
                        None
                    } else {
                        dir.and_then(|d| std::fs::read_to_string(d.join("README.md")).ok()).or_else(|| fetch_text(&format!("{id}/README.md")).ok())
                    };
                    *s2.lock().unwrap() = Some(r);
                });
                s
            })
            .clone();
        let readme = slot.lock().unwrap().clone();
        match readme {
            None => {
                self.plug_line(ui, w, &t("Loading…"), theme::ui(12.5), p.text3);
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            Some(None) => self.plug_line(ui, w, &c.desc, theme::ui(13.0), p.text2),
            Some(Some(md)) => markdown(ui, &md, w, &p),
        }
        if let Some(a) = act {
            self.plug_do(&c, a, ctx);
            if a == "remove" {
                self.plug.detail = None;
            }
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

// tabuľka z README: prvý riadok tučne, stĺpce podľa najširšieho textu
fn md_table(ui: &mut egui::Ui, rows: &[Vec<String>], w: f32, p: &theme::Pal) {
    let n = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if n == 0 {
        return;
    }
    let mut cw = vec![0f32; n];
    for (i, r) in rows.iter().enumerate() {
        for (k, c) in r.iter().enumerate() {
            let f = if i == 0 { theme::bold(12.5) } else { theme::ui(12.5) };
            cw[k] = cw[k].max(widgets::text_w(ui, &c.replace(['*', '`'], ""), f) + 24.0);
        }
    }
    let total: f32 = cw.iter().sum();
    if total > w {
        let k = w / total;
        cw.iter_mut().for_each(|x| *x *= k);
    }
    let rh = 30.0;
    let (r, _) = ui.allocate_exact_size(vec2(cw.iter().sum::<f32>().min(w), rh * rows.len() as f32), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::same(10), p.hover);
    ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, p.line), StrokeKind::Inside);
    for (i, row) in rows.iter().enumerate() {
        let y = r.top() + i as f32 * rh;
        if i > 0 {
            ui.painter().hline(r.x_range(), y, Stroke::new(1.0, p.line));
        }
        let mut x = r.left();
        for (k, c) in row.iter().enumerate() {
            let f = if i == 0 { theme::bold(12.5) } else { theme::ui(12.5) };
            widgets::text(ui, pos2(x + 12.0, y + rh / 2.0), Align2::LEFT_CENTER, &c.replace(['*', '`'], ""), f, if i == 0 { p.text } else { p.text2 }, cw[k] - 18.0);
            x += cw[k];
        }
    }
    ui.add_space(6.0);
}
