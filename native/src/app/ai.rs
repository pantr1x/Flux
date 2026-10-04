// Claude AI – ako src/main/ai.js + aiPanel.js v Electron Fluxe: vlastný API kľúč Anthropic, model,
// vzdialené MCP konektory a panel s rozhovorom vpravo na karte (Ctrl+I). Odpoveď sa streamuje cez curl (SSE).
// Kľúč a tokeny sú v secret.rs (DPAPI); do settings.json ide len ai.model a ai.mcp [{name, url, enabled}].
use super::github::{divider, group_begin, group_end, line_row};
use super::App;
use crate::i18n::{t, tf};
use crate::{net, secret, theme, widgets};
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use serde_json::{json, Value};
use std::io::BufRead;
use std::sync::{Arc, Mutex};

// hotové MCP konektory (meno, adresa, popis)
// (Notion, Linear a pod. potrebujú prihlásenie OAuth, ktoré API konektor nevie – preto tu nie sú)
const PRESETS: [(&str, &str, &str); 4] = [
    ("Context7", "https://mcp.context7.com/mcp", "Up-to-date docs for libraries and frameworks"),
    ("DeepWiki", "https://mcp.deepwiki.com/mcp", "Ask about any public GitHub repository"),
    ("GitHub", "https://api.githubcopilot.com/mcp/", "Issues, pull requests and code on GitHub (needs a token)"),
    ("Hugging Face", "https://huggingface.co/mcp", "Models, datasets and Spaces"),
];

pub const MODELS: [(&str, &str); 3] = [("claude-opus-5", "Claude Opus 5"), ("claude-sonnet-5", "Claude Sonnet 5"), ("claude-haiku-4-5", "Claude Haiku 4.5")];
const DEFAULT_MODEL: &str = "claude-opus-5";

const SYSTEM: &str = "You are the coding assistant built into Flux, a small code editor for beginners.
Explain things simply and briefly, and prefer short, complete, runnable code.
Put code in fenced blocks with the language name (e.g. ```python) so the user can insert it into the file with one click.
When the user shares a file, refer to it by name and line numbers.
Answer in the language the user writes in.";

fn api_url() -> String {
    std::env::var("FLUX_AI_URL").unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".into())
}

pub struct Msg {
    pub user: bool,
    pub text: String,
    pub tools: Vec<String>, // použité nástroje MCP („server · nástroj“)
}

#[derive(Default)]
pub struct AiState {
    pub msgs: Vec<Msg>,
    pub history: Vec<Value>, // správy pre API (aj bloky nástrojov MCP)
    pub running: bool,
    pub child: Option<std::process::Child>,
    pub error: Option<String>,
    pub test: Option<Result<String, String>>,
    pub testing: bool,
}

#[derive(Default)]
pub struct AiUi {
    pub shared: Arc<Mutex<AiState>>,
    pub open: bool,
    pub width: f32,
    pub input: String,
    key_in: String,
    mcp_name: String,
    mcp_url: String,
    mcp_token: String,
    mcp_add_now: bool,                // hotový konektor bez tokenu – pridať hneď
    key_hint: Option<Option<String>>, // „…abcd“ (zistené raz)
    share_file: Option<bool>,
}

fn headers(key: &str, mcp: bool) -> Vec<String> {
    let mut h = vec![format!("x-api-key: {key}"), "anthropic-version: 2023-06-01".into(), "content-type: application/json".into()];
    if mcp {
        h.push("anthropic-beta: mcp-client-2025-11-20".into());
    }
    h
}

// chyba API → veta pre používateľa (ako catch v ai.js)
fn nice_error(code: u16, msg: &str) -> String {
    match code {
        401 => t("The API key was rejected. Check it in Settings → AI."),
        429 => t("Too many requests – wait a moment and try again."),
        0 => t("Could not reach the AI service. Check your internet connection."),
        _ => format!("{} {code}: {msg}", t("AI error")),
    }
}

impl App {
    fn ai_model(&self) -> String {
        self.core.setting("ai")["model"].as_str().filter(|m| MODELS.iter().any(|x| x.0 == *m)).unwrap_or(DEFAULT_MODEL).to_string()
    }

    fn ai_mcp(&self) -> Vec<Value> {
        self.core.setting("ai")["mcp"].as_array().cloned().unwrap_or_default()
    }

    fn ai_save(&self, f: impl FnOnce(&mut serde_json::Map<String, Value>)) {
        self.update_settings(|o| {
            let mut ai = o.get("ai").and_then(|v| v.as_object().cloned()).unwrap_or_default();
            f(&mut ai);
            o.insert("ai".into(), Value::Object(ai));
        });
    }

    // Nastavenia → AI: kľúč + model, MCP konektory
    pub(super) fn ai_settings_ui(&mut self, ui: &mut egui::Ui, id: &str, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        match id {
            "ai-key" => {
                let hint = self.ai.key_hint.get_or_insert_with(|| secret::hint("ai")).clone();
                let g = group_begin(ui);
                let mut key = std::mem::take(&mut self.ai.key_in);
                let mut save = false;
                let sub = match &hint {
                    Some(h) => tf("Saved ({hint}). Paste a new one to replace it.", &[("hint", h)]),
                    None => t("Get one at console.anthropic.com"),
                };
                line_row(self, ui, w, "Anthropic API key", &sub, |ui| {
                    if widgets::button(ui, None, &t("Save"), p.card2, p.text, 30.0, &p).clicked() {
                        save = true;
                    }
                    let r = ui.add(egui::TextEdit::singleline(&mut key).password(true).hint_text("sk-ant-…").desired_width(190.0).margin(egui::Margin::symmetric(10, 6)));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        save = true;
                    }
                });
                if save && !key.trim().is_empty() {
                    secret::set("ai", key.trim());
                    key.clear();
                    self.ai.key_hint = None;
                    self.note(t("Saved."));
                }
                self.ai.key_in = key;
                divider(ui, w, &p);
                let cur = self.ai_model();
                let mut pick = None;
                line_row(self, ui, w, &t("Model"), &t("Opus is the smartest, Haiku the fastest and cheapest."), |ui| {
                    let labels: Vec<String> = MODELS.iter().map(|m| m.1.to_string()).collect();
                    let cur_l = MODELS.iter().find(|m| m.0 == cur).map(|m| m.1).unwrap_or(MODELS[0].1);
                    if let Some(i) = widgets::select(ui, egui::Id::new("ai-model"), cur_l, &labels, 200.0, &p) {
                        pick = Some(MODELS[i].0);
                    }
                });
                if let Some(m) = pick {
                    self.ai_save(|a| {
                        a.insert("model".into(), json!(m));
                    });
                }
                if hint.is_some() {
                    divider(ui, w, &p);
                    let (testing, res) = {
                        let s = self.ai.shared.lock().unwrap();
                        (s.testing, s.test.clone())
                    };
                    let (line, err) = match &res {
                        Some(Ok(m)) => (tf("Works – {model} answered.", &[("model", m)]), false),
                        Some(Err(e)) => (e.clone(), true),
                        None => (t("Send a tiny question to check the key."), false),
                    };
                    let mut go = false;
                    ui.horizontal(|ui| {
                        ui.set_min_height(54.0);
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.set_width((w - 32.0) * 0.62);
                            ui.add_space(9.0);
                            ui.label(egui::RichText::new(t("Test the connection")).font(theme::bold(13.0)).color(p.text));
                            ui.add(
                                egui::Label::new(egui::RichText::new(line).font(theme::ui(12.0)).color(if err {
                                    p.red
                                } else if res.is_some() {
                                    p.green
                                } else {
                                    p.text3
                                }))
                                .wrap(),
                            );
                            ui.add_space(9.0);
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(16.0);
                            if testing {
                                ui.add(egui::Spinner::new().size(16.0).color(p.text2));
                            } else if widgets::button(ui, Some("sparkle"), &t("Test"), p.card2, p.text, 30.0, &p).clicked() {
                                go = true;
                            }
                        });
                    });
                    if go {
                        self.ai_test(ctx);
                    }
                }
                group_end(ui, g, w, &p);
            }
            "ai-mcp" => {
                let list = self.ai_mcp();
                let g = group_begin(ui);
                let mut toggle = None;
                let mut del = None;
                if list.is_empty() {
                    ui.horizontal(|ui| {
                        ui.set_min_height(48.0);
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new(t("No connectors yet.")).font(theme::ui(12.5)).color(p.text3));
                    });
                }
                for (i, m) in list.iter().enumerate() {
                    if i > 0 {
                        divider(ui, w, &p);
                    }
                    let name = m["name"].as_str().unwrap_or("").to_string();
                    let has_token = secret::get(&format!("mcp:{name}")).is_some() || m["token"].as_str().is_some_and(|s| !s.is_empty() && !s.starts_with("enc:"));
                    let sub = format!("{}{}", m["url"].as_str().unwrap_or(""), if has_token { format!(" \u{00B7} {}", t("token saved")) } else { String::new() });
                    let mut on = m["enabled"].as_bool() != Some(false);
                    line_row(self, ui, w, &name, &sub, |ui| {
                        if widgets::icon_button(ui, "trash", &p, true).on_hover_text(t("Remove")).clicked() {
                            del = Some(i);
                        }
                        ui.add_space(6.0);
                        if super::prefs::switch(ui, &mut on, &p, true) {
                            toggle = Some((i, on));
                        }
                    });
                }
                group_end(ui, g, w, &p);
                // hotové konektory: jeden klik vyplní meno a adresu
                ui.add_space(10.0);
                ui.label(egui::RichText::new(t("Ready-made connectors")).font(theme::bold(12.0)).color(p.text2));
                ui.add_space(4.0);
                let mut preset = None;
                // bez pluginu GitHub sa GitHub nikde neponúka
                let gh = self.gh_plugin();
                ui.horizontal_wrapped(|ui| {
                    for (i, (name, _, _)) in PRESETS.iter().enumerate() {
                        if *name == "GitHub" && !gh {
                            continue;
                        }
                        let have = list.iter().any(|m| m["name"].as_str() == Some(name));
                        if widgets::button(ui, Some(if have { "check" } else { "plus" }), name, p.card2, if have { p.text3 } else { p.text }, 28.0, &p).on_hover_text(t(PRESETS[i].2)).clicked() {
                            preset = Some(i);
                        }
                    }
                });
                if let Some(i) = preset {
                    let (name, url, _) = PRESETS[i];
                    self.ai.mcp_name = name.to_lowercase().replace(' ', "-");
                    self.ai.mcp_url = url.to_string();
                    if name == "GitHub" {
                        self.note(t("GitHub needs a token: paste a personal access token and press Add."));
                        flux_core::settings::open_external("https://github.com/settings/personal-access-tokens/new");
                    } else {
                        self.ai.mcp_token.clear();
                        self.ai.mcp_add_now = true;
                    }
                }
                // přidanie (form.s-mcp-add)
                ui.add_space(10.0);
                let mut add = false;
                ui.horizontal(|ui| {
                    let fw = ((w - 110.0) / 3.0).max(80.0);
                    ui.add(egui::TextEdit::singleline(&mut self.ai.mcp_name).hint_text(t("Name, e.g. github")).desired_width(fw).margin(egui::Margin::symmetric(10, 6)));
                    ui.add(egui::TextEdit::singleline(&mut self.ai.mcp_url).hint_text("https://…/mcp").desired_width(fw).margin(egui::Margin::symmetric(10, 6)));
                    ui.add(egui::TextEdit::singleline(&mut self.ai.mcp_token).password(true).hint_text(t("Token (optional)")).desired_width(fw).margin(egui::Margin::symmetric(10, 6)));
                    if widgets::button(ui, Some("plus"), &t("Add"), p.card2, p.text, 30.0, &p).clicked() {
                        add = true;
                    }
                });
                let add = add || std::mem::take(&mut self.ai.mcp_add_now);
                let (name, url) = (self.ai.mcp_name.trim().to_string(), self.ai.mcp_url.trim().to_string());
                if add && !name.is_empty() && url.starts_with("http") {
                    if !self.ai.mcp_token.trim().is_empty() {
                        secret::set(&format!("mcp:{name}"), self.ai.mcp_token.trim());
                    }
                    self.ai_save(|a| {
                        let mut l: Vec<Value> = a.get("mcp").and_then(|v| v.as_array().cloned()).unwrap_or_default();
                        l.retain(|m| m["name"].as_str() != Some(&name));
                        l.push(json!({ "name": name, "url": url, "enabled": true }));
                        a.insert("mcp".into(), json!(l));
                    });
                    self.ai.mcp_name.clear();
                    self.ai.mcp_url.clear();
                    self.ai.mcp_token.clear();
                }
                if let Some((i, on)) = toggle {
                    self.ai_save(|a| {
                        if let Some(m) = a.get_mut("mcp").and_then(|v| v.as_array_mut()).and_then(|l| l.get_mut(i)) {
                            m["enabled"] = json!(on);
                        }
                    });
                }
                if let Some(i) = del {
                    if let Some(n) = list[i]["name"].as_str() {
                        secret::remove(&format!("mcp:{n}"));
                    }
                    self.ai_save(|a| {
                        if let Some(l) = a.get_mut("mcp").and_then(|v| v.as_array_mut()) {
                            if i < l.len() {
                                l.remove(i);
                            }
                        }
                    });
                }
            }
            "ai-flux" => self.mcp_ui(ui, w),
            _ => {}
        }
    }

    fn ai_test(&mut self, ctx: &egui::Context) {
        let Some(key) = secret::get("ai") else { return };
        let model = self.ai_model();
        let sh = self.ai.shared.clone();
        let c = ctx.clone();
        {
            let mut s = sh.lock().unwrap();
            s.testing = true;
            s.test = None;
        }
        std::thread::spawn(move || {
            let body = json!({ "model": model, "max_tokens": 16, "messages": [{ "role": "user", "content": "Say OK." }] }).to_string();
            let res = match net::request("POST", &api_url(), &headers(&key, false), Some(&body), 60) {
                Ok((200, b)) => Ok(serde_json::from_str::<Value>(&b).ok().and_then(|v| v["model"].as_str().map(String::from)).unwrap_or(model)),
                Ok((code, b)) => Err(nice_error(code, serde_json::from_str::<Value>(&b).ok().and_then(|v| v["error"]["message"].as_str().map(String::from)).as_deref().unwrap_or(""))),
                Err(_) => Err(nice_error(0, "")),
            };
            let mut s = sh.lock().unwrap();
            s.testing = false;
            s.test = Some(res);
            c.request_repaint();
        });
    }

    // pošle otázku; odpoveď prichádza po kúskoch (content_block_delta)
    fn ai_send(&mut self, text: String, ctx: &egui::Context) {
        let Some(key) = secret::get("ai") else { return };
        let model = self.ai_model();
        let servers: Vec<(String, String)> =
            self.ai_mcp().iter().filter(|m| m["enabled"].as_bool() != Some(false)).filter_map(|m| Some((m["name"].as_str()?.to_string(), m["url"].as_str()?.to_string()))).collect();
        // otvorený súbor ako kontext (ak je zapnuté „Pridať otvorený súbor“)
        let mut content = text.clone();
        if self.ai.share_file != Some(false) && !self.home {
            if let Some(tab) = self.tabs.get(self.active) {
                let body: String = tab.text.chars().take(40_000).collect();
                content = format!("{text}\n\n<file name=\"{}\" cursor_line=\"{}\">\n{body}\n</file>", tab.name(), self.cursor.0);
            }
        }
        let sh = self.ai.shared.clone();
        {
            let mut s = sh.lock().unwrap();
            s.msgs.push(Msg { user: true, text, tools: vec![] });
            s.msgs.push(Msg { user: false, text: String::new(), tools: vec![] });
            s.history.push(json!({ "role": "user", "content": content }));
            s.running = true;
            s.error = None;
        }
        let c = ctx.clone();
        std::thread::spawn(move || {
            let mut params = json!({ "model": model, "max_tokens": 64000, "stream": true, "system": SYSTEM });
            if model != "claude-haiku-4-5" {
                params["thinking"] = json!({ "type": "adaptive" });
                params["output_config"] = json!({ "effort": "medium" });
            }
            if !servers.is_empty() {
                params["mcp_servers"] = json!(servers
                    .iter()
                    .map(|(n, u)| {
                        let mut m = json!({ "type": "url", "url": u, "name": n });
                        if let Some(tk) = secret::get(&format!("mcp:{n}")) {
                            m["authorization_token"] = json!(tk);
                        }
                        m
                    })
                    .collect::<Vec<_>>());
                params["tools"] = json!(servers.iter().map(|(n, _)| json!({ "type": "mcp_toolset", "mcp_server_name": n })).collect::<Vec<_>>());
            }
            // pause_turn = dlhšia práca nástrojov MCP → pokračovať (najviac 8×)
            for _ in 0..8 {
                params["messages"] = json!(sh.lock().unwrap().history.clone());
                let mut child = match net::stream(&api_url(), &headers(&key, !servers.is_empty()), &params.to_string()) {
                    Ok(ch) => ch,
                    Err(e) => {
                        let mut s = sh.lock().unwrap();
                        s.error = Some(e);
                        s.running = false;
                        c.request_repaint();
                        return;
                    }
                };
                let out = child.stdout.take();
                sh.lock().unwrap().child = Some(child);
                let mut blocks: Vec<Value> = vec![];
                let mut partial: Vec<String> = vec![];
                let mut stop = String::new();
                let mut raw = String::new();
                for line in std::io::BufReader::new(out.unwrap()).lines() {
                    let Ok(line) = line else { break };
                    let Some(data) = line.strip_prefix("data:") else {
                        if !line.starts_with("event:") && !line.trim().is_empty() {
                            raw.push_str(&line);
                        }
                        continue;
                    };
                    let Ok(ev) = serde_json::from_str::<Value>(data.trim()) else { continue };
                    match ev["type"].as_str().unwrap_or("") {
                        "content_block_start" => {
                            let b = ev["content_block"].clone();
                            if b["type"] == "mcp_tool_use" {
                                if let Some(m) = sh.lock().unwrap().msgs.last_mut() {
                                    m.tools.push(format!("{} \u{00B7} {}", b["server_name"].as_str().unwrap_or(""), b["name"].as_str().unwrap_or("")));
                                }
                            }
                            blocks.push(b);
                            partial.push(String::new());
                        }
                        "content_block_delta" => {
                            let i = ev["index"].as_u64().unwrap_or(0) as usize;
                            let d = &ev["delta"];
                            let Some(b) = blocks.get_mut(i) else { continue };
                            match d["type"].as_str().unwrap_or("") {
                                "text_delta" => {
                                    let s = d["text"].as_str().unwrap_or("");
                                    b["text"] = json!(format!("{}{s}", b["text"].as_str().unwrap_or("")));
                                    if let Some(m) = sh.lock().unwrap().msgs.last_mut() {
                                        m.text.push_str(s);
                                    }
                                    c.request_repaint();
                                }
                                "thinking_delta" => b["thinking"] = json!(format!("{}{}", b["thinking"].as_str().unwrap_or(""), d["thinking"].as_str().unwrap_or(""))),
                                "signature_delta" => b["signature"] = json!(d["signature"].as_str().unwrap_or("")),
                                "input_json_delta" => partial[i].push_str(d["partial_json"].as_str().unwrap_or("")),
                                _ => {}
                            }
                        }
                        "content_block_stop" => {
                            let i = ev["index"].as_u64().unwrap_or(0) as usize;
                            if let (Some(b), Some(pj)) = (blocks.get_mut(i), partial.get(i)) {
                                if !pj.is_empty() {
                                    b["input"] = serde_json::from_str(pj).unwrap_or(json!({}));
                                }
                            }
                        }
                        "message_delta" => stop = ev["delta"]["stop_reason"].as_str().unwrap_or("").to_string(),
                        "error" => raw = ev["error"]["message"].as_str().unwrap_or("").to_string(),
                        _ => {}
                    }
                }
                // Zastaviť: z nedokončenej odpovede ostane len text (myslenie bez podpisu by API odmietlo)
                let aborted = {
                    let mut s = sh.lock().unwrap();
                    if let Some(mut ch) = s.child.take() {
                        let _ = ch.wait();
                    }
                    let aborted = !s.running;
                    if aborted {
                        let txt: String = blocks.iter().filter_map(|b| b["text"].as_str()).collect();
                        s.history.push(json!({ "role": "assistant", "content": if txt.trim().is_empty() { "(stopped)".to_string() } else { txt } }));
                    } else if !blocks.is_empty() {
                        s.history.push(json!({ "role": "assistant", "content": blocks }));
                    }
                    aborted
                };
                if aborted {
                    c.request_repaint();
                    return;
                }
                if blocks.is_empty() && !raw.is_empty() {
                    // chyba API prišla ako obyčajný JSON (napr. 401)
                    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
                    let kind = v["error"]["type"].as_str().unwrap_or("");
                    let code = match kind {
                        "authentication_error" => 401,
                        "rate_limit_error" => 429,
                        _ => 1,
                    };
                    let mut s = sh.lock().unwrap();
                    s.error = Some(nice_error(code, v["error"]["message"].as_str().unwrap_or(&raw)));
                    s.history.pop(); // otázka bez odpovede – nech sa dá poslať znova
                    s.msgs.pop();
                    break;
                }
                if stop == "refusal" {
                    sh.lock().unwrap().error = Some(t("Claude could not answer this request."));
                }
                if stop != "pause_turn" {
                    break;
                }
            }
            let mut s = sh.lock().unwrap();
            s.running = false;
            c.request_repaint();
        });
    }

    fn ai_stop(&mut self) {
        let mut s = self.ai.shared.lock().unwrap();
        s.running = false;
        if let Some(ch) = s.child.as_mut() {
            let _ = ch.kill();
        }
    }

    // FLUX_TEST ask=<otázka>
    pub(super) fn ai_ask(&mut self, q: &str, ctx: &egui::Context) {
        self.ai.open = true;
        self.ai_send(q.to_string(), ctx);
    }

    pub(super) fn toggle_ai(&mut self) {
        self.ai.open = !self.ai.open;
    }

    // panel vpravo na karte (#ai-panel)
    pub(super) fn ai_panel(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = self.pal;
        let ctx = ui.ctx().clone();
        ui.painter().vline(r.left(), r.y_range(), Stroke::new(1.0, p.line));
        // hlavička
        let head = Rect::from_min_size(r.min, vec2(r.width(), 44.0));
        widgets::icon_at(ui, pos2(head.left() + 20.0, head.center().y), 15.0, "sparkle", p.accent);
        ui.painter().text(pos2(head.left() + 34.0, head.center().y), Align2::LEFT_CENTER, "Claude", theme::bold(14.0), p.text);
        let model = MODELS.iter().find(|m| m.0 == self.ai_model()).map(|m| m.1).unwrap_or("");
        ui.painter().text(pos2(head.left() + 92.0, head.center().y), Align2::LEFT_CENTER, model, theme::ui(11.5), p.text3);
        let close = Rect::from_center_size(pos2(head.right() - 22.0, head.center().y), vec2(28.0, 28.0));
        if widgets::icon_button_at(ui, close, "x", 14.0, &p, true).on_hover_text(t("Close")).clicked() {
            self.ai.open = false;
        }
        let clear = Rect::from_center_size(pos2(head.right() - 54.0, head.center().y), vec2(28.0, 28.0));
        if widgets::icon_button_at(ui, clear, "plus", 14.0, &p, true).on_hover_text(t("New chat")).clicked() {
            self.ai_stop();
            let mut s = self.ai.shared.lock().unwrap();
            s.msgs.clear();
            s.history.clear();
            s.error = None;
        }
        ui.painter().hline(r.x_range(), head.bottom(), Stroke::new(1.0, p.line));
        let has_key = self.ai.key_hint.get_or_insert_with(|| secret::hint("ai")).is_some();
        // vstup dole
        let input_h = 96.0;
        let inp = Rect::from_min_max(pos2(r.left() + 12.0, r.bottom() - input_h), pos2(r.right() - 12.0, r.bottom() - 12.0));
        let body = Rect::from_min_max(pos2(r.left(), head.bottom() + 1.0), pos2(r.right(), inp.top() - 8.0));
        let mut bui = ui.new_child(egui::UiBuilder::new().max_rect(body));
        bui.set_clip_rect(body);
        let (running, err) = {
            let s = self.ai.shared.lock().unwrap();
            (s.running, s.error.clone())
        };
        let mut insert = None;
        if !has_key {
            bui.add_space(40.0);
            bui.vertical_centered(|ui| {
                ui.set_width(body.width() - 48.0);
                widgets::icon_at(ui, pos2(body.center().x, ui.cursor().top() + 14.0), 26.0, "sparkle", p.text3);
                ui.add_space(38.0);
                ui.add(egui::Label::new(egui::RichText::new(t("Add your Anthropic API key in Settings → AI first.")).font(theme::ui(13.0)).color(p.text2)).wrap());
                ui.add_space(12.0);
            });
            let mut b = bui.new_child(
                egui::UiBuilder::new().max_rect(Rect::from_center_size(pos2(body.center().x, bui.cursor().top() + 16.0), vec2(200.0, 32.0))).layout(egui::Layout::top_down(egui::Align::Center)),
            );
            if widgets::button(&mut b, Some("settings"), &t("Open settings"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                self.open_settings("ai", &ctx);
            }
        } else {
            let mut sa = egui::ScrollArea::vertical().id_salt("ai-chat").auto_shrink(false).stick_to_bottom(true);
            sa = sa.max_height(body.height());
            sa.show(&mut bui, |ui| {
                ui.add_space(10.0);
                let s = self.ai.shared.lock().unwrap();
                if s.msgs.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.set_width(body.width() - 32.0);
                            ui.label(egui::RichText::new(t("Ask Claude about your code.")).font(theme::bold(14.0)).color(p.text));
                            ui.add(egui::Label::new(egui::RichText::new(t("It sees the open file. Code in answers can be inserted with one click.")).font(theme::ui(12.5)).color(p.text3)).wrap());
                        });
                    });
                }
                let bw = body.width() - 32.0;
                for (mi, m) in s.msgs.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.set_width(bw);
                            if m.user {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                    egui::Frame::new().fill(p.active).corner_radius(12).inner_margin(egui::Margin::symmetric(12, 8)).show(ui, |ui| {
                                        ui.set_max_width(bw * 0.85);
                                        ui.add(egui::Label::new(egui::RichText::new(&m.text).font(theme::ui(13.0)).color(p.text)).wrap());
                                    });
                                });
                            } else {
                                for tl in &m.tools {
                                    ui.label(egui::RichText::new(format!("🔧 {tl}")).font(theme::ui(11.5)).color(p.text3));
                                }
                                if m.text.is_empty() && running && mi + 1 == s.msgs.len() {
                                    ui.add(egui::Spinner::new().size(14.0).color(p.text3));
                                }
                                // text a bloky kódu (```jazyk … ```)
                                for (bi, part) in m.text.split("```").enumerate() {
                                    if bi % 2 == 0 {
                                        let txt = part.trim_matches('\n');
                                        if !txt.is_empty() {
                                            let mut job = egui::text::LayoutJob::default();
                                            job.wrap.max_width = bw;
                                            for (k, seg) in txt.split("**").enumerate() {
                                                for (j, s2) in seg.split('`').enumerate() {
                                                    let font = if j % 2 == 1 {
                                                        theme::mono(12.5)
                                                    } else if k % 2 == 1 {
                                                        theme::bold(13.0)
                                                    } else {
                                                        theme::ui(13.0)
                                                    };
                                                    job.append(s2, 0.0, egui::TextFormat { font_id: font, color: p.text, line_height: Some(19.0), ..Default::default() });
                                                }
                                            }
                                            ui.label(job);
                                        }
                                    } else {
                                        let (lang, code) = part.split_once('\n').unwrap_or(("", part));
                                        let code = code.trim_end_matches('\n');
                                        egui::Frame::new().fill(p.card2).stroke(Stroke::new(1.0, p.line)).corner_radius(10).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                                            ui.set_width(bw - 20.0);
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(lang.trim()).font(theme::ui(11.0)).color(p.text3));
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.add(egui::Button::new(egui::RichText::new(t("Insert")).font(theme::ui(11.5))).frame(false)).on_hover_text(t("Insert at the cursor")).clicked()
                                                    {
                                                        insert = Some(code.to_string());
                                                    }
                                                    if ui.add(egui::Button::new(egui::RichText::new(t("Copy")).font(theme::ui(11.5))).frame(false)).clicked() {
                                                        ui.ctx().copy_text(code.to_string());
                                                    }
                                                });
                                            });
                                            egui::ScrollArea::horizontal().id_salt(("ai-code", mi, bi)).show(ui, |ui| {
                                                ui.add(egui::Label::new(egui::RichText::new(code).font(theme::mono(12.0)).color(p.text)).extend());
                                            });
                                        });
                                    }
                                }
                            }
                        });
                    });
                    ui.add_space(12.0);
                }
                if let Some(e) = &err {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.add(egui::Label::new(egui::RichText::new(e).font(theme::ui(12.5)).color(p.red)).wrap());
                    });
                }
                ui.add_space(8.0);
            });
        }
        if let Some(code) = insert {
            self.insert_at_cursor(&code);
        }
        // pole na otázku
        ui.painter().rect_filled(inp, CornerRadius::same(12), p.card2);
        ui.painter().rect_stroke(inp, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let mut iu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(inp.min + vec2(10.0, 8.0), pos2(inp.right() - 10.0, inp.bottom() - 36.0))));
        let te = iu.add_enabled(
            has_key,
            egui::TextEdit::multiline(&mut self.ai.input).hint_text(t("Ask Claude…")).frame(egui::Frame::NONE).desired_width(f32::INFINITY).desired_rows(2).font(theme::ui(13.0)),
        );
        let enter = te.has_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        // spodok poľa: „otvorený súbor“ a Poslať / Zastaviť
        let by = inp.bottom() - 20.0;
        let share = self.ai.share_file != Some(false);
        let fname = if self.home { None } else { self.tabs.get(self.active).map(|t| t.name()) };
        if let Some(f) = fname {
            let chip = Rect::from_min_size(pos2(inp.left() + 10.0, by - 11.0), vec2((widgets::text_w(ui, &f, theme::ui(11.5)) + 28.0).min(inp.width() - 110.0), 22.0));
            let cr = ui.interact(chip, ui.id().with("ai-share"), Sense::click()).on_hover_text(t("Send the open file with the question"));
            ui.painter().rect_filled(chip, CornerRadius::same(7), if share { p.active } else { p.hover });
            widgets::icon_at(ui, pos2(chip.left() + 11.0, chip.center().y), 11.0, if share { "check" } else { "file" }, if share { p.text } else { p.text3 });
            widgets::text(ui, pos2(chip.left() + 21.0, chip.center().y), Align2::LEFT_CENTER, &f, theme::ui(11.5), if share { p.text } else { p.text3 }, chip.width() - 25.0);
            if cr.clicked() {
                self.ai.share_file = Some(!share);
            }
        }
        let send_r = Rect::from_center_size(pos2(inp.right() - 24.0, by), vec2(30.0, 26.0));
        let sr = ui.interact(send_r, ui.id().with("ai-send"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        ui.painter().rect_filled(send_r, CornerRadius::same(8), if running { p.red.gamma_multiply(0.85) } else { p.accent });
        widgets::icon_at(ui, send_r.center(), 13.0, if running { "stop" } else { "play" }, p.accent_fg);
        if running {
            if sr.on_hover_text(t("Stop")).clicked() {
                self.ai_stop();
            }
        } else if (sr.on_hover_text(t("Send")).clicked() || enter) && has_key {
            let q = self.ai.input.trim().to_string();
            if !q.is_empty() {
                self.ai.input.clear();
                self.ai_send(q, &ctx);
            }
        }
    }

    // kód z odpovede na miesto kurzora v otvorenom súbore
    fn insert_at_cursor(&mut self, code: &str) {
        if self.home || self.tabs.is_empty() {
            self.status = t("Open a file first.");
            return;
        }
        let (line, col) = self.cursor;
        let tab = &mut self.tabs[self.active];
        let mut ci = 0usize;
        for (i, l) in tab.text.split('\n').enumerate() {
            if i + 1 == line {
                ci += (col - 1).min(l.chars().count());
                break;
            }
            ci += l.chars().count() + 1;
        }
        let bi = tab.text.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(tab.text.len());
        tab.text.insert_str(bi, code);
        self.last_edit = Some(std::time::Instant::now());
        self.note(t("Inserted."));
    }
}

// ---------- Flux pre iné AI aplikácie (MCP server, mcp.rs) ----------
impl App {
    pub(super) fn mcp_toggle(&mut self, on: bool) {
        crate::mcp::set_enabled(&self.core, on);
        self.mcp = None;
        self.mcp_err = None;
        if on {
            match crate::mcp::start(self.core.clone(), self.emit.clone()) {
                Ok(s) => self.mcp = Some(s),
                Err(e) => self.mcp_err = Some(e),
            }
        }
    }

    fn mcp_ui(&mut self, ui: &mut egui::Ui, w: f32) {
        let p = self.pal;
        let g = group_begin(ui);
        let mut on = self.mcp.is_some();
        let status = match (&self.mcp, &self.mcp_err) {
            (Some(s), _) => tf("Running at {url}", &[("url", &crate::mcp::url(s.port))]),
            (None, Some(e)) => e.clone(),
            _ => t("Off"),
        };
        let mut toggled = None;
        line_row(self, ui, w, &t("Flux for other AI apps"), &status, |ui| {
            if super::prefs::switch(ui, &mut on, &p, true) {
                toggled = Some(on);
            }
        });
        if let Some(v) = toggled {
            self.mcp_toggle(v);
        }
        if let Some(port) = self.mcp.as_ref().map(|s| s.port) {
            let url = crate::mcp::url(port);
            let key = crate::mcp::token(&self.core);
            let mut copy: Option<(String, String)> = None;
            let mut desktop = false;
            divider(ui, w, &p);
            line_row(self, ui, w, "Claude Desktop", &t("Adds Flux to Claude Desktop's settings. Restart Claude Desktop afterwards."), |ui| {
                if widgets::button(ui, Some("plus"), &t("Add to Claude Desktop"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                    desktop = true;
                }
            });
            let code_cmd = format!("claude mcp add --transport http flux {url} --header \"Authorization: Bearer {key}\"");
            let cursor = serde_json::to_string_pretty(&json!({ "mcpServers": { "flux": { "url": url, "headers": { "Authorization": format!("Bearer {key}") } } } })).unwrap_or_default();
            let rows = [
                ("Claude Code", t("Run this command in a terminal."), code_cmd),
                ("Cursor / VS Code", t("Paste into mcp.json."), cursor),
                ("Link", t("For any other app that supports MCP over HTTP."), url.clone()),
                ("Key", t("Sent as Authorization: Bearer <key>. Keep it secret."), key.clone()),
            ];
            for (title, hint, value) in rows {
                divider(ui, w, &p);
                line_row(self, ui, w, &t(title), &hint, |ui| {
                    if widgets::button(ui, None, &t("Copy"), p.card2, p.text, 30.0, &p).clicked() {
                        copy = Some((value.clone(), title.to_string()));
                    }
                });
            }
            divider(ui, w, &p);
            let mut renew = false;
            line_row(self, ui, w, &t("New key"), &t("Apps that use the old key stop working until you paste the new one."), |ui| {
                if widgets::button(ui, Some("refresh"), &t("New key"), p.card2, p.text, 30.0, &p).clicked() {
                    renew = true;
                }
            });
            if let Some((v, title)) = copy {
                ui.ctx().copy_text(v);
                self.note(tf("{what} copied.", &[("what", &t(&title))]));
            }
            if desktop {
                match crate::mcp::add_to_claude_desktop() {
                    Ok(f) => self.note(tf("Added to Claude Desktop ({file}). Restart Claude Desktop.", &[("file", &f)])),
                    Err(e) => self.status = e,
                }
            }
            if renew {
                crate::mcp::new_key(&self.core);
                self.note(t("New key created."));
            }
        }
        group_end(ui, g, w, &p);
    }
}
