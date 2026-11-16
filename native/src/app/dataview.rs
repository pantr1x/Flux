// Dáta vo Výstupe: slovníky a zoznamy, ktoré program vypísal (print(data), JSON), ako strom vedľa výstupu –
// kľúč, hodnota farbou podľa typu, počet položiek; klik rozbalí, klik na hodnotu skopíruje cestu ["a"][0]["b"].
use super::*;
use crate::i18n::tf;
use crate::pydata::{self, D};

#[derive(Default)]
pub(super) struct DataView {
    key: (u64, u64), // (karta, počet zápisov do Výstupu) – pri zmene sa výstup prečíta znova
    checked: f64,
    pub items: Vec<(usize, D)>,
    pub sel: usize,
    toggled: HashSet<String>, // cesty rozbalené/zbalené oproti predvolenému (prvé dve úrovne otvorené)
}

struct Row<'a> {
    depth: usize,
    key: String,
    is_index: bool,
    d: &'a D,
    path: String,
    open: bool,
}

const ROW_H: f32 = 21.0;
const MAX_ROWS: usize = 4000;

fn flatten<'a>(d: &'a D, key: String, is_index: bool, path: String, depth: usize, toggled: &HashSet<String>, out: &mut Vec<Row<'a>>) {
    if out.len() >= MAX_ROWS {
        return;
    }
    let open = d.is_container() && ((depth < 2) != toggled.contains(&path));
    out.push(Row { depth, key, is_index, d, path: path.clone(), open });
    if !open {
        return;
    }
    match d {
        D::Map(v) => {
            for (i, (k, c)) in v.iter().enumerate() {
                flatten(c, k.clone(), false, format!("{path}{}", pydata::step(k, true, i)), depth + 1, toggled, out);
            }
        }
        D::List(v, _) => {
            for (i, c) in v.iter().enumerate() {
                flatten(c, i.to_string(), true, format!("{path}{}", pydata::step("", false, i)), depth + 1, toggled, out);
            }
        }
        _ => {}
    }
}

impl App {
    // nové dáta vo Výstupe aktívnej karty (najviac raz za 0,3 s; celý text len keď pribudol výstup)
    pub(super) fn data_refresh(&mut self, now: f64) {
        let Some(s) = self.tabs.get(self.active).and_then(|t| t.sess.as_ref()) else {
            self.data.items.clear();
            return;
        };
        let key = (s.id, s.out.fed);
        if key == self.data.key || now - self.data.checked < 0.3 {
            return;
        }
        let text = s.out.all_text();
        self.data.key = key;
        self.data.checked = now;
        let items = pydata::find_all(&text);
        // nové dáta → ukázať posledné
        if items.len() != self.data.items.len() || items.last() != self.data.items.last() {
            self.data.sel = items.len().saturating_sub(1);
            if key.0 != self.data.key.0 {
                self.data.toggled.clear();
            }
        }
        self.data.items = items;
    }

    pub(super) fn data_on(&self) -> bool {
        self.get("dataView").as_bool() != Some(false)
    }

    pub(super) fn data_ui(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = self.pal;
        if self.data.items.is_empty() {
            return;
        }
        let sel = self.data.sel.min(self.data.items.len() - 1);
        let theme_id = self.get("codeTheme").as_str().unwrap_or(crate::gen::DEFAULT_THEME).to_string();
        let (_, c, _) = crate::code::theme_of(&theme_id);
        let col = |k: &str| {
            let x = c[crate::gen::THEME_KEYS.iter().position(|n| *n == k).unwrap_or(0)];
            Color32::from_rgb((x >> 16) as u8, (x >> 8) as u8, x as u8)
        };
        // hlavička: Data · riadok N · ‹ 2/3 › · ×
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), 28.0));
        let cy = head.center().y;
        let (line_no, root) = &self.data.items[sel];
        let mut x = head.left() + 10.0;
        widgets::icon_at(ui, pos2(x + 7.0, cy), 13.0, "code", p.text3);
        x += 20.0;
        let r = widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, &t("Data"), theme::bold(12.5), p.text, 120.0);
        x = r.right() + 8.0;
        let what = match root {
            D::Map(v) => format!("dict · {}", keys_n(v.len())),
            other => format!("{} · {}", other.kind(), items_n(other.len())),
        };
        let info = format!("{what} · {}", tf("line {n}", &[("n", &line_no.to_string())]));
        widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, &info, theme::ui(11.5), p.text3, (head.right() - x - 110.0).max(20.0));
        let mut rx = head.right() - 16.0;
        if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx, cy), vec2(22.0, 22.0)), "x", 12.0, &p, true).on_hover_text(t("Hide data")).clicked() {
            self.set("dataView", json!(false), ui.ctx());
        }
        rx -= 26.0;
        let n = self.data.items.len();
        if n > 1 {
            if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx, cy), vec2(22.0, 22.0)), "arrowRight", 12.0, &p, sel + 1 < n).on_hover_text(t("Next")).clicked() {
                self.data.sel = sel + 1;
            }
            rx -= 13.0;
            let lbl = format!("{}/{}", sel + 1, n);
            let w = widgets::text_w(ui, &lbl, theme::ui(11.5));
            ui.painter().text(pos2(rx - w / 2.0, cy), Align2::CENTER_CENTER, &lbl, theme::ui(11.5), p.text2);
            rx -= w + 13.0;
            if widgets::icon_button_at(ui, Rect::from_center_size(pos2(rx, cy), vec2(22.0, 22.0)), "arrowLeft", 12.0, &p, sel > 0).on_hover_text(t("Previous")).clicked() {
                self.data.sel = sel - 1;
            }
        }
        ui.painter().hline(rect.x_range(), head.bottom(), Stroke::new(1.0, p.line));
        // strom
        let body = Rect::from_min_max(pos2(rect.left(), head.bottom() + 1.0), rect.max);
        let mut rows = vec![];
        let (_, root) = &self.data.items[sel];
        let root_key = String::new();
        flatten(root, root_key, false, String::new(), 0, &self.data.toggled, &mut rows);
        let mut toggle: Option<String> = None;
        let mut copy: Option<String> = None;
        let font = egui::FontId::monospace((self.term_fs - 0.5).max(10.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
        child.set_clip_rect(body.intersect(ui.clip_rect()));
        egui::ScrollArea::vertical().id_salt(("data-tree", sel)).auto_shrink([false, false]).show_rows(&mut child, ROW_H, rows.len(), |ui, range| {
            for i in range {
                let row = &rows[i];
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r.shrink2(vec2(4.0, 0.0)), CornerRadius::same(5), p.hover);
                }
                let mut x = r.left() + 10.0 + row.depth as f32 * 14.0;
                let cy = r.center().y;
                // vodiace čiary úrovní
                for d in 1..=row.depth {
                    let lx = r.left() + 10.0 + (d as f32 - 1.0) * 14.0 + 5.0;
                    ui.painter().vline(lx, r.y_range(), Stroke::new(1.0, p.line));
                }
                if row.d.is_container() {
                    // trojuholník ▸ / ▾
                    let c0 = pos2(x + 5.0, cy);
                    let pts = if row.open {
                        vec![pos2(c0.x - 3.5, c0.y - 2.0), pos2(c0.x + 3.5, c0.y - 2.0), pos2(c0.x, c0.y + 2.5)]
                    } else {
                        vec![pos2(c0.x - 2.0, c0.y - 3.5), pos2(c0.x + 2.5, c0.y), pos2(c0.x - 2.0, c0.y + 3.5)]
                    };
                    ui.painter().add(egui::Shape::convex_polygon(pts, p.text3, Stroke::NONE));
                }
                x += 14.0;
                // kľúč (pri koreni nič)
                if row.depth > 0 {
                    let kc = if row.is_index { col("number").gamma_multiply(0.75) } else { col("property") };
                    let kt = if row.is_index { row.key.clone() } else { row.key.clone() };
                    let kr = widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, &kt, font.clone(), kc, (r.right() - x - 60.0).max(20.0) * 0.5);
                    let cr = ui.painter().text(pos2(kr.right(), cy), Align2::LEFT_CENTER, ":", font.clone(), p.text3);
                    x = cr.right() + 8.0;
                }
                // hodnota
                let (txt, vc) = match row.d {
                    D::Map(v) => (if row.open { format!("{{ {} }}", keys_n(v.len())) } else { preview(row.d) }, p.text3),
                    D::List(..) => (if row.open { format!("[ {} ]", items_n(row.d.len())) } else { preview(row.d) }, p.text3),
                    D::Str(_) => (row.d.short(), col("string")),
                    D::Num(_) => (row.d.short(), col("number")),
                    D::Bool(_) | D::Null => (row.d.short(), col("storage")),
                    D::Raw(_) => (row.d.short(), col("type")),
                };
                let kind = row.d.kind();
                let kw = widgets::text_w(ui, kind, theme::ui(10.5));
                widgets::text(ui, pos2(x, cy), Align2::LEFT_CENTER, &txt, font.clone(), vc, (r.right() - x - kw - 22.0).max(20.0));
                // typ vpravo, len pri prejdení (tichý zoznam)
                if resp.hovered() {
                    ui.painter().text(pos2(r.right() - 10.0, cy), Align2::RIGHT_CENTER, kind, theme::ui(10.5), p.text3);
                }
                let resp = resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(if row.path.is_empty() { t("Click to fold or unfold") } else if row.d.is_container() { format!("{}\n{}", row.path, t("Click to fold or unfold")) } else { format!("{}\n{}", row.path, t("Click to copy the path")) });
                if resp.clicked() {
                    if row.d.is_container() {
                        toggle = Some(row.path.clone());
                    } else {
                        copy = Some(row.path.clone());
                    }
                }
                if resp.secondary_clicked() && !row.path.is_empty() {
                    copy = Some(row.path.clone());
                }
            }
        });
        if let Some(path) = toggle {
            if !self.data.toggled.remove(&path) {
                self.data.toggled.insert(path);
            }
        }
        if let Some(path) = copy {
            ui.ctx().copy_text(path.clone());
            self.note(tf("Copied {path}", &[("path", &path)]));
        }
    }
}

// zbalený slovník / zoznam: začiatok obsahu – {'lat': 48.4, 'lon': 17.8, …}
fn preview(d: &D) -> String {
    let mut s = String::new();
    match d {
        D::Map(v) => {
            s.push('{');
            for (i, (k, c)) in v.iter().take(4).enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&format!("{k}: {}", c.short()));
            }
            if v.len() > 4 {
                s.push_str(", …");
            }
            s.push('}');
        }
        D::List(v, _) => {
            s.push('[');
            for (i, c) in v.iter().take(5).enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&c.short());
            }
            if v.len() > 5 {
                s.push_str(", …");
            }
            s.push(']');
        }
        other => s = other.short(),
    }
    s
}

fn keys_n(n: usize) -> String {
    if n == 1 { t("1 key") } else { tf("{n} keys", &[("n", &n.to_string())]) }
}

fn items_n(n: usize) -> String {
    if n == 1 { t("1 item") } else { tf("{n} items", &[("n", &n.to_string())]) }
}
