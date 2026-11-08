// Prispôsobenie prostredia ako vo Visual Studiu (docs/ide/personalizing-the-visual-studio-ide.md):
// export / import / obnovenie nastavení, nastavenia ako JSON, uložené rozloženia okna, externé nástroje,
// karty (Ctrl+1–9, presun, obnovenie zavretej) a rýchle ovládanie priehľadnosti (ako GlassIt vo VS Code).
use super::{menus::Ask, App, Bottom};
use crate::i18n::{t, tf};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Rect, Sense};
use serde_json::{json, Map, Value};
use std::path::Path;

// kľúče, ktoré patria k nastaveniam prostredia (exportujú sa / obnovujú) – nie projekty, účty, tokeny ani stav okna
fn portable(k: &str) -> bool {
    !super::prefs::default_of(k).is_null()
        || matches!(
            k,
            "codeTheme" | "lastDark" | "lastLight" | "accent" | "material" | "language" | "keybindings" | "externalTools" | "layouts" | "tabColors" | "dirtyDot" | "rulers" | "filesExclude"
        )
}

// rozloženie okna = tieto kľúče (+ či je otvorený bočný panel a Výstup)
const LAYOUT_KEYS: [&str; 10] = ["sidePos", "panelPos", "showTabs", "statusBar", "activityBar", "breadcrumbs", "minimap", "uiZoom", "density", "searchWide"];

impl App {
    // ---------- nastavenia: export, import, obnovenie, JSON ----------
    pub(super) fn settings_export(&mut self) {
        let Some(f) = rfd::FileDialog::new().set_title(t("Export settings")).set_file_name("flux-settings.json").add_filter("JSON", &["json"]).save_file() else { return };
        let set = self.core.settings.lock().unwrap().clone();
        let mut out = Map::new();
        if let Some(o) = set.as_object() {
            for (k, v) in o.iter().filter(|(k, _)| portable(k)) {
                out.insert(k.clone(), v.clone());
            }
        }
        let doc = json!({ "flux": "settings", "version": env!("CARGO_PKG_VERSION"), "settings": out });
        match std::fs::write(&f, serde_json::to_string_pretty(&doc).unwrap_or_default()) {
            Ok(_) => self.note(tf("Settings saved to {name}.", &[("name", &widgets::file_name(&f.to_string_lossy()))])),
            Err(e) => self.status = e.to_string(),
        }
    }

    pub(super) fn settings_import(&mut self, ctx: &egui::Context) {
        let Some(f) = rfd::FileDialog::new().set_title(t("Import settings")).add_filter("JSON", &["json"]).pick_file() else { return };
        let Some(doc) = std::fs::read_to_string(&f).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) else {
            self.status = t("This file is not a Flux settings file.");
            return;
        };
        // súbor z exportu ({flux, settings}) alebo priamo objekt nastavení
        let src = doc.get("settings").cloned().unwrap_or(doc);
        let Some(o) = src.as_object().cloned() else {
            self.status = t("This file is not a Flux settings file.");
            return;
        };
        let mut n = 0;
        self.update_settings(|set| {
            for (k, v) in o.into_iter().filter(|(k, _)| portable(k)) {
                set.insert(k, v);
                n += 1;
            }
        });
        self.after_settings_change(ctx);
        self.note(tf("Imported {n} settings.", &[("n", &n.to_string())]));
    }

    // obnoví všetky nastavenia prostredia na predvolené (projekty, účty a kľúče ostanú)
    pub(super) fn settings_reset(&mut self, ctx: &egui::Context) {
        self.update_settings(|set| {
            set.retain(|k, _| !portable(k) || k == "language");
        });
        self.after_settings_change(ctx);
        self.note(t("All settings are back to default."));
    }

    pub(super) fn ask_settings_reset(&mut self) {
        self.ask = Some(Ask {
            text: t("Reset all settings to default? Your projects, accounts and keys stay. Tip: export your settings first, then you can bring them back."),
            yes: t("Reset all settings"),
            action: "settings-reset".into(),
        });
    }

    // „Edit user settings as JSON“ – otvorí settings.json v editore
    pub(super) fn settings_json(&mut self) {
        let f = flux_core::settings::user_data().join("settings.json");
        let path = f.to_string_lossy().to_string();
        self.settings = None;
        flux_core::fsops::allow_file(&self.core, &path);
        self.reload_projects();
        self.open_file(&path);
    }

    fn after_settings_change(&mut self, ctx: &egui::Context) {
        crate::i18n::set_language(self.get("language").as_str().unwrap_or("en"));
        theme::fonts(ctx, self.get("fontFamily").as_str().unwrap_or("Consolas"));
        let id = self.get("codeTheme").as_str().unwrap_or(crate::gen::DEFAULT_THEME).to_string();
        self.set_code_theme(&id, ctx);
        self.tree.clear();
        self.apply_look(ctx);
    }

    // ---------- rozloženia okna (Window → Save Window Layout / Apply / Reset) ----------
    pub(super) fn layouts(&self) -> Vec<Value> {
        self.get("layouts").as_array().cloned().unwrap_or_default()
    }

    pub(super) fn layout_save(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let mut vals = Map::new();
        for k in LAYOUT_KEYS {
            vals.insert(k.into(), self.get(k));
        }
        vals.insert("sideOpen".into(), json!(self.side_open));
        vals.insert("panelOpen".into(), json!(self.panel_open));
        let mut list = self.layouts();
        list.retain(|l| l["name"].as_str() != Some(name));
        list.push(json!({ "name": name, "values": vals }));
        self.update_settings(|o| {
            o.insert("layouts".into(), Value::Array(list));
        });
        self.note(tf("Layout “{name}” saved.", &[("name", name)]));
    }

    pub(super) fn layout_apply(&mut self, name: &str, ctx: &egui::Context) {
        let Some(l) = self.layouts().into_iter().find(|l| l["name"].as_str() == Some(name)) else { return };
        let vals = l["values"].as_object().cloned().unwrap_or_default();
        self.update_settings(|o| {
            for k in LAYOUT_KEYS {
                match vals.get(k) {
                    Some(v) if !v.is_null() => {
                        o.insert(k.into(), v.clone());
                    }
                    _ => {
                        o.remove(k);
                    }
                }
            }
        });
        self.side_open = vals.get("sideOpen").and_then(|v| v.as_bool()).unwrap_or(true);
        self.panel_open = vals.get("panelOpen").and_then(|v| v.as_bool()).unwrap_or(true);
        self.apply_look(ctx);
    }

    pub(super) fn layout_delete(&mut self, name: &str) {
        let mut list = self.layouts();
        list.retain(|l| l["name"].as_str() != Some(name));
        self.update_settings(|o| {
            o.insert("layouts".into(), Value::Array(list));
        });
    }

    // Window → Reset Window Layout
    pub(super) fn layout_reset(&mut self, ctx: &egui::Context) {
        self.update_settings(|o| {
            for k in LAYOUT_KEYS {
                o.remove(k);
            }
        });
        self.side_open = true;
        self.panel_open = true;
        self.apply_look(ctx);
    }

    // ---------- externé nástroje (Tools → External Tools) ----------
    pub(super) fn ext_tools(&self) -> Vec<(String, String)> {
        self.get("externalTools")
            .as_array()
            .map(|a| a.iter().map(|x| (x["name"].as_str().unwrap_or("").to_string(), x["command"].as_str().unwrap_or("").to_string())).filter(|x| !x.0.is_empty() && !x.1.is_empty()).collect())
            .unwrap_or_default()
    }

    pub(super) fn ext_tools_set(&mut self, list: Vec<(String, String)>) {
        let arr: Vec<Value> = list.into_iter().map(|(n, c)| json!({ "name": n, "command": c })).collect();
        self.update_settings(|o| {
            o.insert("externalTools".into(), Value::Array(arr));
        });
    }

    // spustí nástroj v Termináli; $(FilePath), $(FileDir), $(FileName), $(ProjectDir) ako vo Visual Studiu
    pub(super) fn ext_tool_run(&mut self, i: usize) {
        let Some((_, cmd)) = self.ext_tools().get(i).cloned() else { return };
        let file = self.tabs.get(self.active).filter(|_| !self.home).map(|t| t.path.clone()).unwrap_or_default();
        let q = |s: &str| if s.contains(' ') { format!("\"{s}\"") } else { s.to_string() };
        let dir = Path::new(&file).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        let line = cmd
            .replace("$(FilePath)", &q(&file))
            .replace("$(FileDir)", &q(&dir))
            .replace("$(FileName)", &widgets::file_name(&file))
            .replace("$(ProjectDir)", &q(&self.workspace().unwrap_or_default()));
        if self.sess_mut().is_none() {
            self.status = t("Open a file first – the tool runs in its Terminal.");
            return;
        }
        self.panel_open = true;
        if let Some(s) = self.sess_mut() {
            s.bottom = Bottom::Terminal;
        }
        self.start_shell();
        if let Some(s) = self.sess_mut() {
            s.sh.pty.write(&format!("{line}\r"));
        }
    }

    // ---------- karty ----------
    // Ctrl+Shift+PgUp / PgDn: posunie aktívnu kartu doľava / doprava
    pub(super) fn move_tab(&mut self, right: bool) {
        let i = self.active;
        if self.home || self.tabs.len() < 2 {
            return;
        }
        let j = if right { (i + 1).min(self.tabs.len() - 1) } else { i.saturating_sub(1) };
        if i != j {
            self.tabs.swap(i, j);
            self.active = j;
        }
    }

    // obnoví naposledy zavretú kartu (Ctrl+Shift+T, Window → Restore Closed Tab)
    pub(super) fn reopen_tab(&mut self) {
        while let Some(p) = self.closed_tabs.pop() {
            if Path::new(&p).is_file() {
                self.open_file(&p);
                return;
            }
        }
    }

    // farba karty podľa prípony (Tab colorization: File extension)
    pub(super) fn tab_color(&self, name: &str) -> Option<egui::Color32> {
        if self.get("tabColors").as_str() != Some("extension") {
            return None;
        }
        let ext = Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let h = ext.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
        let hue = (h % 360) as f32 / 360.0;
        Some(egui::ecolor::Hsva::new(hue, 0.55, 0.85, 1.0).into())
    }

    // ---------- priehľadnosť pod rukou ----------
    // Ctrl+Alt+Z = priehľadnejšie, Ctrl+Alt+C = plnšie (o 5 %) – okno, editor aj Výstup naraz
    pub(super) fn glass_step(&mut self, more_clear: bool, ctx: &egui::Context) {
        let d = if more_clear { -5.0 } else { 5.0 };
        let cur = |k: &str, def: f64| self.get(k).as_f64().unwrap_or(def);
        let vals = [("alphaSide", cur("alphaSide", 78.0), 10.0), ("alphaEditor", cur("alphaEditor", 96.0), 20.0), ("alphaPanel", cur("alphaPanel", 96.0), 20.0)];
        self.update_settings(|o| {
            for (k, v, min) in vals {
                o.insert(k.into(), json!(((v + d).clamp(min, 100.0)).round()));
            }
        });
        self.apply_look(ctx);
        let side = self.get("alphaSide").as_f64().unwrap_or(78.0);
        self.note(tf("Opacity {n} %", &[("n", &format!("{side:.0}"))]));
    }

    // čip v stavovom riadku: aktuálna nepriehľadnosť, klik = posuvníky
    pub(super) fn glass_chip(&mut self, ui: &mut egui::Ui, rx: f32, cy: f32) -> f32 {
        if self.get("material").as_str() == Some("none") || self.get("glassChip").as_bool() == Some(false) {
            return 0.0;
        }
        let p = self.pal;
        let v = self.get("alphaSide").as_f64().unwrap_or(78.0);
        let label = format!("{v:.0} %");
        let w = widgets::text_w(ui, &label, theme::ui(12.0)) + 26.0;
        let r = Rect::from_min_size(pos2(rx - w, cy - 10.0), vec2(w, 20.0));
        let resp = ui.interact(r, ui.id().with("glass-chip"), Sense::click()).on_hover_text(t("Transparency – Ctrl+Alt+Z clearer, Ctrl+Alt+C more solid"));
        if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(10), p.hover);
        }
        widgets::icon_at(ui, pos2(r.left() + 10.0, cy), 12.0, "eye", p.text2);
        ui.painter().text(pos2(r.left() + 20.0, cy), Align2::LEFT_CENTER, &label, theme::ui(12.0), p.text2);
        let ctx = ui.ctx().clone();
        egui::Popup::menu(&resp).show(|ui| {
            ui.set_min_width(260.0);
            ui.label(egui::RichText::new(t("Transparency")).strong());
            ui.add_space(4.0);
            for (k, label, def, min) in [("alphaSide", "Window", 78.0, 10.0), ("alphaEditor", "Editor", 96.0, 20.0), ("alphaPanel", "Output and Terminal", 96.0, 20.0)] {
                let mut x = self.get(k).as_f64().unwrap_or(def);
                ui.label(egui::RichText::new(t(label)).color(p.text2).size(12.0));
                if ui.add(egui::Slider::new(&mut x, min..=100.0).suffix(" %").integer()).changed() {
                    self.update_settings(|o| {
                        o.insert(k.into(), json!(x.round()));
                    });
                    self.apply_look(&ctx);
                }
            }
            ui.add_space(4.0);
            ui.label(egui::RichText::new(t("Ctrl+Alt+Z clearer · Ctrl+Alt+C more solid")).color(p.text3).size(11.5));
        });
        w + 10.0
    }
}
