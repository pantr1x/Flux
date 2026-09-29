// GitHub v Nastaveniach – ako src/renderer/github.js + src/main/github.js v Electron Fluxe:
// prihlásenie cez prehliadač (device flow), alebo token, zoznam repozitárov a otvorenie (klon) ako projekt.
// Token je v secret.rs (DPAPI), do settings.json ide len github.user – ten zdieľa aj Electron Flux.
use super::App;
use crate::i18n::{t, tf};
use crate::{net, secret, theme, widgets};
use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Client ID OAuth aplikácie Flux (package.json → flux.githubClientId); nie je tajný
const CLIENT_ID: &str = "Ov23lizS4t9eZvEEScJ8";

fn api_base() -> String {
    std::env::var("FLUX_GITHUB_API").unwrap_or_else(|_| "https://api.github.com".into())
}

fn web_base() -> String {
    std::env::var("FLUX_GITHUB_URL").unwrap_or_else(|_| "https://github.com".into())
}

#[derive(Clone)]
pub struct Repo {
    pub full: String,
    pub private: bool,
    pub description: String,
    pub language: String,
}

#[derive(Default)]
pub struct GhState {
    pub code: Option<(String, String)>, // prihlasovanie: (kód, adresa)
    pub cancel: bool,
    pub busy: bool,
    pub error: Option<String>,
    pub user: Option<Value>, // nový účet – UI ho uloží do nastavení
    pub repos: Option<Vec<Repo>>,
    pub loading: bool,
    pub cloning: Option<String>,
    pub opened: Option<String>,    // naklonovaný priečinok – UI ho otvorí
    pub published: Option<String>, // adresa nového repozitára na GitHube
    pub publish_err: Option<String>,
    pub avatar: Option<egui::ColorImage>,
}

#[derive(Default)]
pub struct GhUi {
    pub shared: Arc<Mutex<GhState>>,
    token_in: String,
    query: String,
    picking: bool,
    advanced: bool,
    connected: Option<bool>, // je uložený token (zistené raz, nie v každej snímke)
    avatar: Option<egui::TextureHandle>,
    avatar_asked: bool,
}

fn headers(token: &str) -> Vec<String> {
    vec!["Accept: application/vnd.github+json".into(), format!("Authorization: Bearer {token}"), "X-GitHub-Api-Version: 2022-11-28".into()]
}

// overí token (GET /user) a uloží ho
fn connect(sh: &Arc<Mutex<GhState>>, token: &str) -> Result<(), String> {
    let u = net::json("GET", &format!("{}/user", api_base()), &headers(token), None).map_err(|e| if e.contains("Bad credentials") { t("GitHub did not accept the token.") } else { e })?;
    let login = u["login"].as_str().unwrap_or("").to_string();
    if login.is_empty() {
        return Err(t("GitHub did not accept the token."));
    }
    secret::set("github", token);
    let user = json!({ "login": login, "name": u["name"].as_str().unwrap_or(&login), "id": u["id"], "avatar": u["avatar_url"] });
    sh.lock().unwrap().user = Some(user);
    Ok(())
}

fn post_form(url: &str, body: &Value) -> Value {
    let h = vec!["Accept: application/json".to_string(), "Content-Type: application/json".to_string()];
    net::request("POST", url, &h, Some(&body.to_string()), 30).ok().and_then(|(_, b)| serde_json::from_str(&b).ok()).unwrap_or(Value::Null)
}

// prihlásenie cez prehliadač: kód → používateľ ho potvrdí na github.com/login/device → token
fn sign_in(sh: Arc<Mutex<GhState>>, ctx: egui::Context) {
    {
        let mut s = sh.lock().unwrap();
        s.busy = true;
        s.error = None;
        s.cancel = false;
    }
    std::thread::spawn(move || {
        let fail = |e: String| {
            let mut s = sh.lock().unwrap();
            s.error = Some(e);
            s.code = None;
            s.busy = false;
            ctx.request_repaint();
        };
        let d = post_form(&format!("{}/login/device/code", web_base()), &json!({ "client_id": CLIENT_ID, "scope": "repo read:user" }));
        let (Some(dc), Some(code)) = (d["device_code"].as_str(), d["user_code"].as_str()) else {
            return fail(d["error_description"].as_str().map(String::from).unwrap_or_else(|| t("GitHub did not answer. Check your internet connection.")));
        };
        let url = d["verification_uri"].as_str().unwrap_or("https://github.com/login/device").to_string();
        let mut interval = d["interval"].as_u64().unwrap_or(5);
        let until = Instant::now() + Duration::from_secs(d["expires_in"].as_u64().unwrap_or(900));
        ctx.copy_text(code.to_string());
        flux_core::settings::open_external(&url);
        sh.lock().unwrap().code = Some((code.to_string(), url));
        ctx.request_repaint();
        while Instant::now() < until {
            // čakanie po kúskoch, aby Zrušiť zabralo hneď
            for _ in 0..interval * 4 {
                std::thread::sleep(Duration::from_millis(250));
                if sh.lock().unwrap().cancel {
                    let mut s = sh.lock().unwrap();
                    s.code = None;
                    s.busy = false;
                    return;
                }
            }
            let r = post_form(&format!("{}/login/oauth/access_token", web_base()), &json!({ "client_id": CLIENT_ID, "device_code": dc, "grant_type": "urn:ietf:params:oauth:grant-type:device_code" }));
            if let Some(tok) = r["access_token"].as_str() {
                let res = connect(&sh, tok);
                let mut s = sh.lock().unwrap();
                s.code = None;
                s.busy = false;
                s.error = res.err();
                ctx.request_repaint();
                return;
            }
            match r["error"].as_str() {
                Some("slow_down") => interval += 5,
                Some("access_denied") => return fail(t("You cancelled the sign-in on GitHub.")),
                Some("authorization_pending") | None => {}
                Some(e) => return fail(r["error_description"].as_str().unwrap_or(e).to_string()),
            }
        }
        fail(t("The code expired. Try signing in again."))
    });
}

fn load_repos(sh: Arc<Mutex<GhState>>, ctx: egui::Context) {
    let Some(token) = secret::get("github") else { return };
    sh.lock().unwrap().loading = true;
    std::thread::spawn(move || {
        let r = net::json("GET", &format!("{}/user/repos?per_page=100&sort=updated&affiliation=owner,collaborator,organization_member", api_base()), &headers(&token), None);
        let mut s = sh.lock().unwrap();
        s.loading = false;
        match r {
            Ok(v) => {
                s.repos = Some(
                    v.as_array()
                        .map(|a| {
                            a.iter()
                                .map(|r| Repo {
                                    full: r["full_name"].as_str().unwrap_or("").into(),
                                    private: r["private"].as_bool().unwrap_or(false),
                                    description: r["description"].as_str().unwrap_or("").into(),
                                    language: r["language"].as_str().unwrap_or("").into(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                )
            }
            Err(e) => s.error = Some(e),
        }
        ctx.request_repaint();
    });
}

fn git() -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c.env("GIT_TERMINAL_PROMPT", "0");
    c
}

fn b64(data: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            o.push(if i <= c.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    o
}

// už naklonované repo v priečinku projektov (findClone v github.js)
fn find_clone(full: &str, root: &Path) -> Option<PathBuf> {
    let want = format!("github.com/{}", full.to_lowercase());
    std::fs::read_dir(root).ok()?.flatten().map(|e| e.path()).find(|d| {
        d.join(".git").exists()
            && git()
                .args(["remote", "get-url", "origin"])
                .current_dir(d)
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_lowercase().trim_end_matches(".git").ends_with(&want))
                .unwrap_or(false)
    })
}

fn clone(sh: Arc<Mutex<GhState>>, full: String, ctx: egui::Context) {
    sh.lock().unwrap().cloning = Some(full.clone());
    std::thread::spawn(move || {
        let root = flux_core::fsops::default_root();
        let res = (|| -> Result<PathBuf, String> {
            if let Some(d) = find_clone(&full, &root) {
                return Ok(d);
            }
            let name = full.rsplit('/').next().unwrap_or("repo").to_string();
            let mut dir = root.join(&name);
            let mut i = 2;
            while dir.exists() {
                dir = root.join(format!("{name}-{i}"));
                i += 1;
            }
            std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
            let out = git_auth()
                .arg("clone")
                .arg(format!("https://github.com/{full}.git"))
                .arg(&dir)
                .current_dir(&root)
                .output()
                .map_err(|_| t("Git is not installed. Install it in Settings → Languages."))?;
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                return Err(err.trim().lines().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"));
            }
            Ok(dir)
        })();
        let mut s = sh.lock().unwrap();
        s.cloning = None;
        match res {
            Ok(d) => s.opened = Some(d.to_string_lossy().to_string()),
            Err(e) => s.error = Some(e),
        }
        ctx.request_repaint();
    });
}

// git s tokenom ako hlavičkou pre github.com (premenné prostredia, nie príkazový riadok ani .git/config)
fn git_auth() -> std::process::Command {
    let mut c = git();
    if let Some(tok) = secret::get("github") {
        c.env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "http.https://github.com/.extraheader")
            .env("GIT_CONFIG_VALUE_0", format!("AUTHORIZATION: basic {}", b64(format!("x-access-token:{tok}").as_bytes())));
    }
    c
}

// nový repozitár na GitHube z priečinka projektu: vytvorí repo, git init, prvý commit, push
fn publish(sh: Arc<Mutex<GhState>>, dir: String, name: String, desc: String, private: bool, author: (String, String), ctx: egui::Context) {
    sh.lock().unwrap().busy = true;
    std::thread::spawn(move || {
        let res = (|| -> Result<String, String> {
            let token = secret::get("github").ok_or_else(|| t("Sign in with GitHub"))?;
            let body = json!({ "name": name, "description": desc, "private": private }).to_string();
            let v = net::json("POST", &format!("{}/user/repos", api_base()), &headers(&token), Some(&body))?;
            let url = v["clone_url"].as_str().ok_or("GitHub did not answer")?.to_string();
            let html = v["html_url"].as_str().unwrap_or(&url).to_string();
            let readme = Path::new(&dir).join("README.md");
            if !readme.exists() {
                let _ = std::fs::write(&readme, format!("# {name}\n\n{desc}\n"));
            }
            let run = |args: &[&str], auth: bool| -> Result<(), String> {
                let mut c = if auth { git_auth() } else { git() };
                let out = c
                    .args(["-c", &format!("user.name={}", author.0), "-c", &format!("user.email={}", author.1)])
                    .args(args)
                    .current_dir(&dir)
                    .output()
                    .map_err(|_| t("Git is not installed. Install it in Settings → Languages."))?;
                if out.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&out.stderr).trim().lines().last().unwrap_or("git").to_string())
                }
            };
            if !Path::new(&dir).join(".git").exists() {
                run(&["init", "-b", "main"], false)?;
            }
            run(&["add", "-A"], false)?;
            let _ = run(&["commit", "-m", "First commit"], false);
            let _ = run(&["remote", "remove", "origin"], false);
            run(&["remote", "add", "origin", &url], false)?;
            run(&["push", "-u", "origin", "HEAD:main"], true)?;
            Ok(html)
        })();
        let mut s = sh.lock().unwrap();
        s.busy = false;
        match res {
            Ok(h) => s.published = Some(h),
            Err(e) => s.publish_err = Some(e),
        }
        ctx.request_repaint();
    });
}

// owner/repo z odkazu (https://github.com/owner/repo(.git), git@github.com:owner/repo, owner/repo)
pub(super) fn repo_of(link: &str) -> Option<String> {
    let l = link.trim().trim_end_matches('/').trim_end_matches(".git");
    let rest = l.rsplit_once("github.com/").map(|x| x.1).or(l.rsplit_once("github.com:").map(|x| x.1)).unwrap_or(l);
    let mut it = rest.split('/').filter(|x| !x.is_empty());
    let (o, r) = (it.next()?, it.next()?);
    let ok = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    (ok(o) && ok(r)).then(|| format!("{o}/{r}"))
}

fn fetch_avatar(sh: Arc<Mutex<GhState>>, url: String, ctx: egui::Context) {
    std::thread::spawn(move || {
        let sep = if url.contains('?') { '&' } else { '?' };
        let Some(b) = net::bytes(&format!("{url}{sep}s=80")) else { return };
        let Ok(img) = image::load_from_memory(&b) else { return };
        let img = img.resize_exact(80, 80, image::imageops::FilterType::Triangle).to_rgba8();
        sh.lock().unwrap().avatar = Some(egui::ColorImage::from_rgba_unmultiplied([80, 80], img.as_raw()));
        ctx.request_repaint();
    });
}

// skupina s pozadím ako .s-group – začiatok a koniec okolo vlastného obsahu
pub(super) fn group_begin(ui: &mut egui::Ui) -> (f32, egui::layers::ShapeIdx) {
    (ui.cursor().top(), ui.painter().add(egui::Shape::Noop))
}

pub(super) fn group_end(ui: &mut egui::Ui, g: (f32, egui::layers::ShapeIdx), w: f32, p: &theme::Pal) {
    let r = Rect::from_min_max(pos2(ui.min_rect().left(), g.0), pos2(ui.min_rect().left() + w, ui.cursor().top()));
    ui.painter().set(g.1, egui::Shape::rect_filled(r, CornerRadius::same(14), p.hover));
    ui.painter().rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0, p.line), StrokeKind::Inside);
}

// riadok s názvom a popisom vľavo; vpravo obsah z `right`
pub(super) fn line_row(app: &App, ui: &mut egui::Ui, w: f32, title: &str, hint: &str, right: impl FnOnce(&mut egui::Ui)) {
    let p = app.pal;
    ui.horizontal(|ui| {
        ui.set_min_height(54.0);
        ui.add_space(16.0);
        ui.allocate_ui_with_layout(vec2(w - 32.0, 54.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.vertical(|ui| {
                ui.set_width((w - 32.0) * 0.55);
                ui.add_space(9.0);
                ui.label(egui::RichText::new(title).font(theme::bold(13.0)).color(p.text));
                if !hint.is_empty() {
                    ui.add(egui::Label::new(egui::RichText::new(hint).font(theme::ui(12.0)).color(p.text3)).wrap());
                }
                ui.add_space(9.0);
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
        });
    });
}

pub(super) fn divider(ui: &mut egui::Ui, w: f32, p: &theme::Pal) {
    let y = ui.cursor().top();
    ui.painter().hline(ui.min_rect().left() + 16.0..=ui.min_rect().left() + w - 16.0, y - 2.0, Stroke::new(1.0, p.line));
}

impl App {
    // FLUX_TEST gh-signin / gh-pick
    pub(super) fn gh_sign_in(&mut self, ctx: &egui::Context) {
        sign_in(self.gh.shared.clone(), ctx.clone());
    }

    pub(super) fn gh_pick(&mut self, ctx: &egui::Context) {
        self.gh.picking = true;
        load_repos(self.gh.shared.clone(), ctx.clone());
    }

    // pre okno Nový projekt
    pub(super) fn gh_connected(&mut self) -> bool {
        *self.gh.connected.get_or_insert_with(|| secret::get("github").is_some())
    }

    pub(super) fn gh_state(&self) -> (bool, Option<String>, Option<String>) {
        let s = self.gh.shared.lock().unwrap();
        (s.busy, s.error.clone(), s.cloning.clone())
    }

    pub(super) fn gh_import(&mut self, full: &str, ctx: &egui::Context) {
        self.gh.shared.lock().unwrap().error = None;
        clone(self.gh.shared.clone(), full.to_string(), ctx.clone());
    }

    pub(super) fn gh_repos(&mut self, ui: &mut egui::Ui, w: f32, ctx: &egui::Context) {
        let (loaded, loading, cloning) = {
            let s = self.gh.shared.lock().unwrap();
            (s.repos.is_some(), s.loading, s.cloning.clone())
        };
        if !loaded && !loading {
            load_repos(self.gh.shared.clone(), ctx.clone());
        }
        self.repo_list(ui, w, cloning.as_deref(), ctx);
    }

    pub(super) fn gh_publish(&mut self, dir: &str, name: &str, desc: &str, private: bool, ctx: &egui::Context) {
        let u = self.core.setting("github")["user"].clone();
        let login = u["login"].as_str().unwrap_or("flux").to_string();
        let author = (u["name"].as_str().unwrap_or(&login).to_string(), format!("{}+{login}@users.noreply.github.com", u["id"].as_u64().unwrap_or(0)));
        self.gh.shared.lock().unwrap().error = None;
        publish(self.gh.shared.clone(), dir.into(), name.into(), desc.into(), private, author, ctx.clone());
    }

    // výsledky z vlákien: nový účet do nastavení, naklonovaný projekt otvoriť
    pub(super) fn gh_poll(&mut self, ctx: &egui::Context) {
        let (user, opened, avatar) = {
            let mut s = self.gh.shared.lock().unwrap();
            (s.user.take(), s.opened.take(), s.avatar.take())
        };
        let (published, perr) = {
            let mut s = self.gh.shared.lock().unwrap();
            (s.published.take(), s.publish_err.take())
        };
        if let Some(h) = published {
            self.note(tf("Published on GitHub: {url}", &[("url", &h)]));
        }
        if let Some(e) = perr {
            self.status = format!("GitHub: {e}");
        }
        if let Some(u) = user {
            self.update_settings(|o| {
                o.insert("github".into(), json!({ "user": u }));
            });
            self.gh.connected = Some(true);
            self.gh.avatar = None;
            self.gh.avatar_asked = false;
            self.note(tf("Connected as {user}.", &[("user", u["login"].as_str().unwrap_or(""))]));
        }
        if let Some(img) = avatar {
            self.gh.avatar = Some(ctx.load_texture("gh-avatar", img, egui::TextureOptions::LINEAR));
        }
        if let Some(d) = opened {
            self.settings = None;
            self.new_project = None;
            self.start = false;
            self.open_folder(&d);
        }
    }

    pub(super) fn github_ui(&mut self, ui: &mut egui::Ui, id: &str, w: f32, ctx: &egui::Context) {
        let p = self.pal;
        self.gh_poll(ctx);
        let connected = *self.gh.connected.get_or_insert_with(|| secret::get("github").is_some());
        match id {
            "gh-account" => {
                let (code, busy, err, cloning) = {
                    let s = self.gh.shared.lock().unwrap();
                    (s.code.clone(), s.busy, s.error.clone(), s.cloning.clone())
                };
                let g = group_begin(ui);
                if let Some((code, url)) = code {
                    // kód na potvrdenie v prehliadači (.gh-signin)
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.set_width(w - 32.0);
                            ui.label(egui::RichText::new(t("A GitHub window opened and Flux types this code into it for you:")).font(theme::ui(13.0)).color(p.text2));
                            ui.add_space(10.0);
                            let (r, resp) = ui.allocate_exact_size(vec2(220.0, 52.0), Sense::click());
                            ui.painter().rect_filled(r, CornerRadius::same(12), p.card2);
                            ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, &code, theme::mono(26.0), p.text);
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(t("Click to copy")).clicked() {
                                ctx.copy_text(code.clone());
                                self.note(t("Copied."));
                            }
                            ui.add_space(10.0);
                            for (i, s) in ["Log in to GitHub if it asks (also with Google).", "Press Continue, then Authorize."].iter().enumerate() {
                                ui.label(egui::RichText::new(format!("{}. {}", i + 1, t(s))).font(theme::ui(13.0)).color(p.text));
                            }
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(t("If the code is not filled in, click the first box and press Ctrl+V – it is already copied.")).font(theme::ui(12.0)).color(p.text3));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.add(egui::Spinner::new().size(14.0).color(p.text2));
                                ui.label(egui::RichText::new(t("Waiting for GitHub…")).font(theme::ui(12.5)).color(p.text2));
                            });
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if widgets::button(ui, Some("external"), &t("Use my browser instead"), p.card2, p.text, 30.0, &p).clicked() {
                                    flux_core::settings::open_external(&url);
                                }
                                if widgets::button(ui, None, &t("Cancel"), p.card2, p.text, 30.0, &p).clicked() {
                                    self.gh.shared.lock().unwrap().cancel = true;
                                }
                            });
                        });
                    });
                    ui.add_space(14.0);
                    ctx.request_repaint_after(Duration::from_millis(300));
                } else if connected {
                    let user = self.core.setting("github")["user"].clone();
                    let login = user["login"].as_str().unwrap_or("").to_string();
                    let name = user["name"].as_str().unwrap_or(&login).to_string();
                    if !self.gh.avatar_asked {
                        self.gh.avatar_asked = true;
                        if let Some(u) = user["avatar"].as_str() {
                            fetch_avatar(self.gh.shared.clone(), u.to_string(), ctx.clone());
                        }
                    }
                    let avatar = self.gh.avatar.clone();
                    let mut disconnect = false;
                    ui.horizontal(|ui| {
                        ui.set_min_height(60.0);
                        ui.add_space(16.0);
                        let (ar, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                        match &avatar {
                            Some(tex) => {
                                egui::Image::from_texture(tex).corner_radius(17).paint_at(ui, ar);
                            }
                            None => {
                                ui.painter().circle_filled(ar.center(), 17.0, p.card2);
                                widgets::icon_at(ui, ar.center(), 18.0, "github", p.text2);
                            }
                        }
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new(&name).font(theme::bold(13.0)).color(p.text));
                            ui.label(egui::RichText::new(format!("@{login}")).font(theme::ui(12.0)).color(p.text3));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(16.0);
                            if widgets::button(ui, None, &t("Disconnect"), p.card2, p.text, 30.0, &p).clicked() {
                                disconnect = true;
                            }
                        });
                    });
                    divider(ui, w, &p);
                    let mut pick = false;
                    line_row(self, ui, w, &t("Open a repository"), &t("Pick one of your repositories – Flux opens it as a project."), |ui| {
                        if widgets::button(ui, Some("github"), &t("Choose…"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                            pick = true;
                        }
                    });
                    if pick {
                        self.gh.picking = !self.gh.picking;
                        if self.gh.picking && self.gh.shared.lock().unwrap().repos.is_none() {
                            load_repos(self.gh.shared.clone(), ctx.clone());
                        }
                    }
                    if self.gh.picking {
                        divider(ui, w, &p);
                        self.repo_list(ui, w, cloning.as_deref(), ctx);
                    }
                    if disconnect {
                        secret::remove("github");
                        self.update_settings(|o| {
                            o.insert("github".into(), json!({}));
                        });
                        self.gh = GhUi { shared: self.gh.shared.clone(), connected: Some(false), ..Default::default() };
                        self.gh.shared.lock().unwrap().repos = None;
                    }
                } else {
                    let mut go = false;
                    line_row(self, ui, w, &t("Sign in with GitHub"), &t("Opens your browser – log in or create an account (also with Google) and press Authorize."), |ui| {
                        if busy {
                            ui.add(egui::Spinner::new().size(16.0).color(p.text2));
                        } else if widgets::button(ui, Some("github"), &t("Sign in"), p.accent, p.accent_fg, 30.0, &p).clicked() {
                            go = true;
                        }
                    });
                    if go {
                        sign_in(self.gh.shared.clone(), ctx.clone());
                    }
                    divider(ui, w, &p);
                    // Pokročilé: token (details.gh-more)
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
                    widgets::icon_at(ui, pos2(r.left() + 24.0, r.center().y), 12.0, "chevron", p.text3);
                    ui.painter().text(pos2(r.left() + 36.0, r.center().y), Align2::LEFT_CENTER, t("Advanced: use a token"), theme::ui(12.5), if resp.hovered() { p.text } else { p.text2 });
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.gh.advanced = !self.gh.advanced;
                    }
                    if self.gh.advanced {
                        let mut token = std::mem::take(&mut self.gh.token_in);
                        let mut go = false;
                        let mut make = false;
                        line_row(self, ui, w, &t("Personal access token"), &t("Create one on GitHub with the “repo” permission, then paste it here."), |ui| {
                            if widgets::button(ui, None, &t("Connect"), p.card2, p.text, 30.0, &p).clicked() {
                                go = true;
                            }
                            let r = ui.add(egui::TextEdit::singleline(&mut token).password(true).hint_text("ghp_… / github_pat_…").desired_width(170.0).margin(egui::Margin::symmetric(10, 6)));
                            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                go = true;
                            }
                            if ui.link(egui::RichText::new(t("Create token")).font(theme::ui(12.0))).clicked() {
                                make = true;
                            }
                        });
                        if make {
                            flux_core::settings::open_external("https://github.com/settings/tokens/new?scopes=repo&description=Flux");
                        }
                        if go && !token.trim().is_empty() {
                            let sh = self.gh.shared.clone();
                            let tok = token.trim().to_string();
                            let c = ctx.clone();
                            sh.lock().unwrap().busy = true;
                            std::thread::spawn(move || {
                                let r = connect(&sh, &tok);
                                let mut s = sh.lock().unwrap();
                                s.busy = false;
                                s.error = r.err();
                                c.request_repaint();
                            });
                            token.clear();
                        }
                        self.gh.token_in = token;
                    }
                }
                group_end(ui, g, w, &p);
                if let Some(e) = err {
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(egui::RichText::new(e).font(theme::ui(12.5)).color(p.red)).wrap());
                }
            }
            // Git – ten istý riadok ako v Jazykoch (inštalácia / aktualizácia)
            _ => self.tools_ui(ui, w, ctx, Some("git")),
        }
    }

    fn repo_list(&mut self, ui: &mut egui::Ui, w: f32, cloning: Option<&str>, ctx: &egui::Context) {
        let p = self.pal;
        let (repos, loading) = {
            let s = self.gh.shared.lock().unwrap();
            (s.repos.clone(), s.loading)
        };
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.add(egui::TextEdit::singleline(&mut self.gh.query).hint_text(t("Search repositories…")).desired_width(w - 32.0).margin(egui::Margin::symmetric(10, 6)));
        });
        ui.add_space(6.0);
        let Some(repos) = repos else {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                if loading {
                    ui.add(egui::Spinner::new().size(14.0).color(p.text2));
                }
                ui.label(egui::RichText::new(t("Loading…")).font(theme::ui(12.5)).color(p.text3));
            });
            ui.add_space(10.0);
            return;
        };
        let q = self.gh.query.trim().to_lowercase();
        let list: Vec<&Repo> = repos.iter().filter(|r| q.is_empty() || r.full.to_lowercase().contains(&q) || r.description.to_lowercase().contains(&q)).take(40).collect();
        if list.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(egui::RichText::new(t("No repositories found.")).font(theme::ui(12.5)).color(p.text3));
            });
        }
        let mut open = None;
        for r in list {
            let (rr, resp) = ui.allocate_exact_size(vec2(w, 46.0), Sense::click());
            let hk = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
            if hk > 0.0 {
                ui.painter().rect_filled(rr.shrink2(vec2(8.0, 2.0)), CornerRadius::same(9), p.active.gamma_multiply(hk));
            }
            widgets::icon_at(ui, pos2(rr.left() + 28.0, rr.center().y), 15.0, if r.private { "lock" } else { "github" }, p.text2);
            let x = rr.left() + 46.0;
            widgets::text(ui, pos2(x, rr.top() + 15.0), Align2::LEFT_CENTER, &r.full, theme::bold(13.0), p.text, w - 190.0);
            let sub = [r.language.as_str(), r.description.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" \u{00B7} ");
            widgets::text(ui, pos2(x, rr.top() + 32.0), Align2::LEFT_CENTER, &sub, theme::ui(11.5), p.text3, w - 190.0);
            let label = if cloning == Some(r.full.as_str()) { t("Cloning…") } else { t("Open") };
            ui.painter().text(pos2(rr.right() - 24.0, rr.center().y), Align2::RIGHT_CENTER, label, theme::bold(12.5), if resp.hovered() { p.text } else { p.text2 });
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && cloning.is_none() {
                open = Some(r.full.clone());
            }
        }
        ui.add_space(6.0);
        if let Some(full) = open {
            clone(self.gh.shared.clone(), full, ctx.clone());
        }
    }
}
