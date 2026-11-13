// Klávesové skratky: jedna tabuľka (id príkazu, skupina, názov, predvolená skratka). Vlastné skratky sú
// v settings.keybindings = {id: "Ctrl+Shift+X"} („“ = bez skratky, viac skratiek oddelených čiarkou).
// Nastavenia → Klávesnica ich ukazujú po skupinách, kliknutím sa nahrá nová skratka; skratky pluginov majú vlastnú skupinu.
use super::*;
use super::plugins_ui::parse_key;
use crate::i18n::tf;

pub(super) const KEYMAP: &[(&str, &str, &str, &str)] = &[
    ("new-file", "File", "New file", "Ctrl+N"),
    ("new-project", "File", "New project", "Ctrl+Shift+N"),
    ("open-folder", "File", "Open folder", "Ctrl+O"),
    ("open-file", "File", "Open file", "Ctrl+Alt+O"),
    ("quick-open", "File", "Quick open file", "Ctrl+P"),
    ("save", "File", "Save", "Ctrl+S"),
    ("save-all", "File", "Save all", "Ctrl+Shift+S"),
    ("close", "File", "Close tab", "Ctrl+W"),
    ("reopen-tab", "File", "Restore closed tab", "Ctrl+Shift+T"),
    ("tab-left", "File", "Move tab left", "Ctrl+Shift+PageUp"),
    ("tab-right", "File", "Move tab right", "Ctrl+Shift+PageDown"),
    ("run", "Run", "Run current file", "F5"),
    ("stop", "Run", "Stop", "Shift+F5"),
    ("terminal", "Run", "Terminal", "Ctrl+`"),
    ("panel", "Run", "Toggle output panel", "Ctrl+J"),
    ("search", "View", "Search everything", "Ctrl+Shift+F, Ctrl+Shift+A"),
    ("commands", "View", "Commands", "Ctrl+Shift+P"),
    ("sidebar", "View", "Toggle sidebar", "Ctrl+B"),
    ("view-explorer", "View", "Explorer", "Ctrl+Shift+E"),
    ("view-plugins", "View", "Plugins", "Ctrl+Shift+X"),
    ("settings", "View", "Settings", "Ctrl+Comma"),
    ("home", "View", "Home", ""),
    ("ai", "View", "Claude AI", "Ctrl+I"),
    ("github", "View", "GitHub", "Ctrl+Shift+G"),
    ("theme", "Look", "Toggle light / dark theme", "Ctrl+Shift+L"),
    ("themes", "Look", "Theme store", "Ctrl+Shift+K"),
    ("glass-clear", "Look", "More see-through", "Ctrl+Alt+Z"),
    ("glass-solid", "Look", "Less see-through", "Ctrl+Alt+C"),
];

// pevné skratky editora (len na prečítanie)
pub(super) const FIXED: &[(&str, &str)] = &[
    ("Find", "Ctrl+F"),
    ("Replace", "Ctrl+H"),
    ("Comment line", "Ctrl+/"),
    ("Suggestions", "Ctrl+Space"),
    ("Undo", "Ctrl+Z"),
    ("Redo", "Ctrl+Y"),
    ("Indent / next snippet field", "Tab"),
    ("Go to tab 1–9", "Ctrl+1…9"),
    ("Back / forward", "Alt+← / Alt+→"),
    ("Rename in the Explorer", "F2"),
    ("Zoom the editor", "Ctrl+Wheel"),
];

pub(super) const GROUPS: [&str; 4] = ["File", "Run", "View", "Look"];

// skratka ako text: Ctrl+Shift+K (názvy kláves ako v egui, čiarka = „Comma“, aby sa dala oddeliť)
pub(super) fn fmt_key(m: egui::Modifiers, k: egui::Key) -> String {
    let mut s = String::new();
    if m.command || m.ctrl {
        s.push_str("Ctrl+");
    }
    if m.alt {
        s.push_str("Alt+");
    }
    if m.shift {
        s.push_str("Shift+");
    }
    s.push_str(match k {
        egui::Key::Comma => "Comma",
        egui::Key::Backtick => "`",
        _ => k.name(),
    });
    s
}

// „Ctrl+Comma“ → „Ctrl+,“ na zobrazenie
pub(super) fn show_key(s: &str) -> String {
    s.split('+').map(|p| if p == "Comma" { "," } else { p }).collect::<Vec<_>>().join("+")
}

fn parse_list(s: &str) -> Vec<(egui::Modifiers, egui::Key)> {
    s.split(", ").filter_map(|x| parse_key(x.trim())).collect()
}

impl App {
    // aktuálna skratka príkazu (vlastná, inak predvolená); „“ = žiadna
    pub(super) fn key_of(&self, id: &str) -> String {
        if let Some(v) = self.get("keybindings")[id].as_str() {
            return v.to_string();
        }
        KEYMAP.iter().find(|k| k.0 == id).map(|k| k.3.to_string()).unwrap_or_default()
    }

    // prvá skratka na zobrazenie v ponukách
    pub(super) fn key_label(&self, id: &str) -> String {
        show_key(self.key_of(id).split(", ").next().unwrap_or(""))
    }

    fn set_key(&mut self, id: &str, v: Option<String>) {
        self.update_settings(|o| {
            let mut kb = o.get("keybindings").and_then(|v| v.as_object().cloned()).unwrap_or_default();
            match v {
                Some(s) => {
                    kb.insert(id.into(), json!(s));
                }
                None => {
                    kb.remove(id);
                }
            }
            o.insert("keybindings".into(), Value::Object(kb));
        });
    }

    // všetky skratky z tabuľky; vráti id príkazov, ktorých skratka bola stlačená
    pub(super) fn keymap_hits(&self, ctx: &egui::Context) -> Vec<&'static str> {
        if self.key_rec.is_some() {
            return vec![];
        }
        // dlhšie kombinácie najprv (Ctrl+Shift+S skôr než Ctrl+S – consume_key porovnáva modifikátory voľne)
        let mut all: Vec<(&'static str, egui::Modifiers, egui::Key)> = KEYMAP.iter().flat_map(|k| parse_list(&self.key_of(k.0)).into_iter().map(move |(m, key)| (k.0, m, key))).collect();
        all.sort_by_key(|(_, m, _)| std::cmp::Reverse(m.shift as u8 + m.alt as u8 + m.command as u8));
        let mut hits = vec![];
        for (id, m, k) in all {
            if ctx.input_mut(|i| i.consume_key(m, k)) {
                hits.push(id);
            }
        }
        hits
    }

    // kto už má túto skratku (príkaz alebo plugin)
    fn key_owner(&self, s: &str, not: &str) -> Option<String> {
        let want = parse_key(s)?;
        for k in KEYMAP {
            if k.0 != not && parse_list(&self.key_of(k.0)).contains(&want) {
                return Some(t(k.2));
            }
        }
        for c in &self.plug.cmds {
            if c.key == Some(want) && c.id != not {
                return Some(c.label.clone());
            }
        }
        None
    }

    // nahrávanie novej skratky: Esc = zrušiť, Backspace/Delete = bez skratky
    fn key_record(&mut self, ctx: &egui::Context) {
        let Some(id) = self.key_rec.clone() else { return };
        let mut done: Option<Option<String>> = None;
        let mut cancel = false;
        ctx.input_mut(|i| {
            let mut keep = vec![];
            for e in i.events.drain(..) {
                if let egui::Event::Key { key, pressed: true, modifiers, .. } = &e {
                    if done.is_none() && !cancel {
                        match key {
                            egui::Key::Escape => cancel = true,
                            egui::Key::Backspace | egui::Key::Delete if modifiers.is_none() => done = Some(Some(String::new())),
                            k => done = Some(Some(fmt_key(*modifiers, *k))),
                        }
                        continue;
                    }
                }
                if !matches!(e, egui::Event::Text(_)) {
                    keep.push(e);
                }
            }
            i.events = keep;
        });
        if cancel {
            self.key_rec = None;
        } else if let Some(Some(s)) = done {
            self.key_rec = None;
            let def = KEYMAP.iter().find(|k| k.0 == id).map(|k| k.3).unwrap_or("");
            self.set_key(&id, if s == def { None } else { Some(s) });
        }
    }

    // Nastavenia → Klávesnica
    pub(super) fn keys_ui(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        self.key_record(ctx);
        if self.key_rec.is_some() {
            ctx.request_repaint();
        }
        // filter
        let fid = egui::Id::new("keys-filter");
        let mut q: String = ctx.data(|d| d.get_temp(fid)).unwrap_or_default();
        ui.horizontal(|ui| {
            self.field(ui, &mut q, &t("Filter shortcuts"), (w - 170.0).max(160.0), false);
            let custom = self.get("keybindings").as_object().is_some_and(|o| !o.is_empty());
            if custom && widgets::button(ui, Some("refresh"), &t("Reset all"), p.card2, p.text, 32.0, &p).clicked() {
                self.update_settings(|o| {
                    o.remove("keybindings");
                });
            }
        });
        ctx.data_mut(|d| d.insert_temp(fid, q.clone()));
        let ql = q.to_lowercase();
        let hit = |label: &str, key: &str| ql.is_empty() || t(label).to_lowercase().contains(&ql) || label.to_lowercase().contains(&ql) || key.to_lowercase().contains(&ql);
        ui.add_space(6.0);
        ui.label(egui::RichText::new(t("Click a shortcut to change it, then press the new keys. Esc cancels, Backspace removes it.")).font(theme::ui(12.0)).color(p.text3));
        ui.add_space(4.0);

        for g in GROUPS {
            let rows: Vec<(&str, &str, String)> = KEYMAP.iter().filter(|k| k.1 == g).map(|k| (k.0, k.2, self.key_of(k.0))).filter(|(_, l, k)| hit(l, k)).collect();
            if rows.is_empty() {
                continue;
            }
            self.keys_group(ui, w, g, rows.len(), |me, ui, i| {
                let (id, label, key) = &rows[i];
                me.key_row(ui, w, id, label, key, true);
            });
        }
        // skratky pluginov: vlastná skupina
        let names: HashMap<String, String> = if self.plug.cmds.iter().any(|c| c.key.is_some()) || self.plug.acts.iter().any(|a| a.key.is_some()) {
            self.plug_installed().into_iter().map(|m| (m.0.id, m.0.name)).collect()
        } else {
            HashMap::new()
        };
        let name = |id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_string());
        let mut plug: Vec<(String, String)> = self.plug.cmds.iter().filter_map(|c| c.key.map(|(m, k)| (format!("{} – {}", name(&c.plugin), c.label), fmt_key(m, k)))).collect();
        plug.extend(self.plug.acts.iter().filter(|a| a.key.is_some()).map(|a| (format!("{} – {}", name(&a.plugin), a.title), a.key_text.clone())));
        plug.retain(|(l, k)| hit(l, k));
        if !plug.is_empty() {
            self.keys_group(ui, w, "Plugins", plug.len(), |me, ui, i| {
                let (l, k) = &plug[i];
                me.key_row(ui, w, "", l, k, false);
            });
        }
        let fixed: Vec<&(&str, &str)> = FIXED.iter().filter(|(l, k)| hit(l, k)).collect();
        if !fixed.is_empty() {
            self.keys_group(ui, w, "Editor", fixed.len(), |me, ui, i| {
                let (l, k) = fixed[i];
                me.key_row(ui, w, "", l, k, false);
            });
        }
        ui.add_space(8.0);
    }

    fn keys_group(&mut self, ui: &mut egui::Ui, w: f32, title: &str, n: usize, mut row: impl FnMut(&mut Self, &mut egui::Ui, usize)) {
        let p = self.pal;
        ui.add_space(10.0);
        ui.label(egui::RichText::new(t(title).to_uppercase()).font(theme::bold(11.0)).color(p.text3));
        ui.add_space(4.0);
        let start = ui.cursor().top();
        let bg = ui.painter().add(egui::Shape::Noop);
        for i in 0..n {
            row(self, ui, i);
            if i + 1 < n {
                let y = ui.cursor().top();
                ui.painter().hline(ui.min_rect().left() + 16.0..=ui.min_rect().left() + w - 16.0, y, Stroke::new(1.0, p.line));
            }
        }
        let r = Rect::from_min_max(pos2(ui.min_rect().left(), start), pos2(ui.min_rect().left() + w, ui.cursor().top()));
        ui.painter().set(bg, egui::Shape::rect_filled(r, CornerRadius::same(14), p.hover));
        ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
    }

    fn key_row(&mut self, ui: &mut egui::Ui, w: f32, id: &str, label: &str, key: &str, edit: bool) {
        let p = self.pal;
        let (r, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
        ui.painter().text(pos2(r.left() + 16.0, r.center().y), Align2::LEFT_CENTER, t(label), theme::ui(13.0), p.text);
        let rec = edit && self.key_rec.as_deref() == Some(id);
        let mut right = r.right() - 16.0;
        // vrátiť predvolenú
        let custom = edit && self.get("keybindings").get(id).is_some();
        if custom && !rec {
            let b = Rect::from_center_size(pos2(right - 11.0, r.center().y), vec2(22.0, 22.0));
            let resp = ui.interact(b, ui.id().with(("key-reset", id)), Sense::click()).on_hover_text(t("Reset to default"));
            if resp.hovered() {
                ui.painter().rect_filled(b, CornerRadius::same(6), p.card2);
            }
            widgets::icon(ui, b.shrink(4.0), "refresh", if resp.hovered() { p.text } else { p.text3 });
            if resp.clicked() {
                self.set_key(id, None);
            }
            right -= 30.0;
        }
        // klávesy ako „čiapočky“
        let parts: Vec<String> = if rec {
            vec![t("Press keys…")]
        } else if key.is_empty() {
            vec![if edit { t("Add") } else { String::new() }]
        } else {
            key.split(", ").map(show_key).collect()
        };
        let font = theme::mono(11.5);
        let mut x = right;
        let mut hit_rect = Rect::NOTHING;
        for part in parts.iter().rev() {
            let caps: Vec<&str> = if rec || key.is_empty() { vec![part.as_str()] } else { part.split('+').collect() };
            for cap in caps.iter().rev() {
                let cw = widgets::text_w(ui, cap, font.clone()) + 14.0;
                let kr = Rect::from_min_size(pos2(x - cw, r.center().y - 11.0), vec2(cw, 22.0));
                let (fill, stroke, col) = if rec { (p.accent.gamma_multiply(0.18), p.accent, p.text) } else if key.is_empty() { (Color32::TRANSPARENT, p.line, p.text3) } else { (p.card2, p.line_strong, p.text) };
                ui.painter().rect(kr, CornerRadius::same(6), fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
                ui.painter().text(kr.center(), Align2::CENTER_CENTER, *cap, font.clone(), col);
                hit_rect = hit_rect.union(kr);
                x = kr.left() - 4.0;
            }
            x -= 8.0;
        }
        if edit {
            let resp = ui.interact(hit_rect.expand(3.0), ui.id().with(("key-edit", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.hovered() && !rec {
                ui.painter().rect_stroke(hit_rect.expand(3.0), CornerRadius::same(8), Stroke::new(1.0, p.accent), StrokeKind::Inside);
            }
            if resp.clicked() {
                self.key_rec = if rec { None } else { Some(id.to_string()) };
            }
            // konflikt s iným príkazom
            if !rec && !key.is_empty() {
                if let Some(o) = key.split(", ").find_map(|k| self.key_owner(k, id)) {
                    let msg = tf("Also used by {name}", &[("name", &o)]);
                    ui.painter().text(pos2(x - 6.0, r.center().y), Align2::RIGHT_CENTER, msg, theme::ui(11.5), p.red);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_defaults_parse() {
        for k in KEYMAP {
            for s in k.3.split(", ").filter(|s| !s.is_empty()) {
                let (m, key) = parse_key(s).unwrap_or_else(|| panic!("{} {}", k.0, s));
                assert_eq!(parse_key(&fmt_key(m, key)), Some((m, key)), "{s}");
            }
        }
        assert_eq!(show_key("Ctrl+Comma"), "Ctrl+,");
    }
}
