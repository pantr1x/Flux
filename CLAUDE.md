# Flux – notes for Claude

Flux is a small code editor for Windows 10/11, built with Electron and Monaco. It has one-click Run, Live Server, Claude AI, GitHub, plugins, and it updates itself from GitHub Releases. The repository also holds the website (`site/`), translations (`locales/`) and plugins (`plugins/`).

## Layout

| Path | What it is |
|---|---|
| `src/main/` | Electron main process. `main.js` = window, settings, IPC. One module per area: `updater.js` (GitHub Releases + electron-updater), `runner.js`/`toolchains.js` (running code, downloading languages), `liveServer.js`, `lsp.js` (Pyright), `ai.js`, `github.js`, `plugins.js`, `i18n.js`, `lan.js` (Flux Together), `mcpServer.js`. |
| `src/preload.js` | The only bridge between UI and Node: everything the UI can do is `window.flux.*` here. A new IPC call needs a handler in `main.js` **and** a line here. |
| `src/renderer/` | UI. `app.js` is the core (editor, sidebar, home screen, **settings**, commands). Bigger parts have their own module: `updatesUI.js`, `pluginsUI.js`, `keymap.js`, `userShortcuts.js`, `themes.js`, `themeStudio.js`, `onboarding.js`, `together.js`, `aiPanel.js`… `styles.css` holds all styles. |
| `locales/` | Translations downloaded by the app at runtime (see *Translations*). |
| `plugins/` | Built-in and community plugins + `index.json`. Guide: `docs/PLUGINS.md`. |
| `site/` | The website (a single `index.html`, no build step). |
| `build/` | Installer assets; `release-notes.md` is generated during a release. |
| `scripts/` | `build.mjs` (esbuild → `dist/renderer`), `release-notes.mjs`, `extract-strings.mjs`, `build-locales.py` + `locales_*.py`. |
| `.github/workflows/` | `release.yml`, `pages.yml`, `windows-build.yml`, `plugin-review.yml` (safety review of plugin pull requests: `scripts/plugin-review.mjs`, rules + Claude with the `ANTHROPIC_API_KEY` secret (without it the check asks for a manual review; GitHub Models only answered `OK` in 2026-09, so there is no free fallback); manual run for one plugin via *Run workflow*; PR code is only read, never run), `wiki.yml` (publishes `docs/wiki/*.md` to the GitHub wiki – edit those files, not the wiki). |

## Flux Native (`native/`, pure Rust + egui) – in progress, the chosen direction

- **Why:** in a measurement on Windows CI, Electron Flux took ~120 MB and the Tauri/WebView2 build ~109 MB, because the Chromium UI engine is almost all of it. So the user chose a **native** app with a different look, and Flux Native is the way forward. Electron Flux keeps shipping until Flux Native covers the important features.
- **Layout:** Cargo workspace at the repo root (`Cargo.toml`) with these members:
  - `crates/flux-core`: shared logic, used by both Rust builds, with the modules `settings` (same `settings.json` as Electron), `fsops` (incl. `summary(dir)`: files, lines, languages, last change), `python`, `runner` (`command_for`) and `pty`. Events go through an `Emit` callback (`Arc<dyn Fn(&str, Value)>`).
  - `desktop` (Tauri),
  - `native`.
- **`native/src/`:**
  - `app.rs`: the window. It has a top bar with tabs and ▶ Run, a sidebar with projects and a file tree, and one rounded card holding the editor plus Output/Terminal, followed by a status bar. Editing uses `egui::TextEdit` with syntect colors (`egui_extras::syntax_highlighting`) and a line number gutter. It also covers autosave after 1 s, F5, Ctrl+S/O/W and reloading on outside changes via `notify`.
  - `term.rs`: Alacritty's VT emulator (`alacritty_terminal`) fed by `flux_core::pty`, drawn as a colored grid, with keys → bytes.
  - `theme.rs`: Flux colors, plus Segoe UI and Cascadia Mono from `%WINDIR%\Fonts`.
  - `widgets.rs`: painted sidebar rows, icon buttons, section captions, text with `…`, the logo.
  - `gen.rs` is **generated** by `node scripts/native-gen.mjs` from `src/renderer/icons.js` (the same file and line icons as SVG, drawn with resvg via egui_extras `svg`; `<text>` in an icon is drawn by egui on top) and `src/renderer/themes.js` (the code themes). Run it again after changing icons or themes.
  - `i18n.rs`: `t("…")` / `tf("…", &[("n", "3")])` with the same `locales/*.json` (bundled with `include_str!`, a newer `userData/locales` copy wins). `scripts/extract-strings.mjs` also scans `native/src/**/*.rs`, so native strings are translated like all others (`locales_v20.py`).
  - `app/intro.rs`: the first-run intro (same steps and look as `onboarding.js`: splash, language with flags, name, what to code, extras, look with theme/accent/brightness and a live preview, done). Shown until `settings.onboarded`; `FLUX_INTRO=1` forces it, the `FLUX_TEST` action `next` moves on.
  - `smooth.rs`: smooth wheel scrolling for every `ScrollArea` (fixed-duration Hermite glide, see *Wheel input*): call `smooth.begin()` before `show()` (it takes the wheel over the area's layer and returns the offset to set) and `smooth.end()` after. Off with `inertia` or animations off.
  - `wall.rs`: the Electron look of material `wallpaper`: the desktop wallpaper (`SystemParametersInfoW(SPI_GETDESKWALLPAPER)`, or an own `settings.bg` image, `FLUX_WALLPAPER` in tests) drawn in the background layer, lined up with the desktop, under translucent `--base-alpha` / `--card` panels. Off with `material: none`, `optFx` or `lite`. Overlays (settings, search, menus) use the opaque `Pal.solid`.
  - Animations follow `anim_on()` (`transitions` + `optAnim`/`lite`): hover fades via `animate_bool_with_time`, the gliding active-tab indicator, content fade on file/project switch, the settings open, and the intro (moving glows, logo draw-in, steps slide in).
  - `native/CHANGELOG.md`: release notes of Flux Native, shown in Settings → General → About & updates. Add a section with each bigger change and raise `version` in `native/Cargo.toml` (and `assets/flux.rc`).
  - `mem.rs`: `trimMemory` for Flux Native (K32EmptyWorkingSet 20 s after losing focus, then every 2 min).
  - `build.rs` + `assets/flux.rc`: Windows icon and "Flux" description (Task Manager, taskbar); the window icon is `assets/icon64.png`.
  - `app/chrome.rs`: Flux's own title bar. The window has no Windows frame (`with_decorations(false)`). `drag_area()` makes empty parts of the top bar move the window (`StartDrag`, double-click = maximize). `window_chrome()` draws − □ × (46 px each, `CONTROLS_W`) in an `Order::Tooltip` area at the top right, above settings and the intro, plus 5 px resize edges (`BeginResize`). The top bar reserves `CONTROLS_W` on the right (with `sidePos: right` the sidebar toggle moves into the top bar).
  - `live.rs`: Lively Wallpaper / Wallpaper Engine detection, ported from `src/main/liveWallpaper.js` (same files, `tasklist`, `LOCALAPPDATA` / `FLUX_STEAM_PATH` for tests). `App::check_live()` runs it on a thread at most every 10 s while focused; `wall_source()` → `wall::Src::Image` or `Src::Video(file, preview)`. Off with `liveWallpaper: false` or an own `settings.bg`.
  - `wall.rs` video: `Src::Video` plays through Windows Media Foundation (`windows` crate, `IMFSourceReader` with a D3D11 device manager = hardware decoding via `hw_decoder()`, RGB32 scaled to ≤ 960 px, ≤ 24 fps; late frames are skipped without copying and a lag over 0.8 s seeks to the real time) into one texture updated in place; it plays only while focused and `optAnim` is on (`set_playing`). If MF fails (or off Windows) the preview image is shown. Every image also gets a 1/8 box-blurred copy (`blurred()`), drawn under the card with `paint_blurred()` like `backdrop-filter`.
  - Baked copy: `Src::Video` never decodes the original. `baked_copy()` / `bake_now()` make `userData/wallcache/<sha16>.mp4` once (key = path + size + mtime; `video::bake()` = SourceReader RGB32 ≤ 640 px → SinkWriter H.264 1.5 Mbit/s, ≤ 60 s, below-normal thread priority; the newest 3 are kept). While baking, one still frame is shown; if baking fails the still frame stays. Files in `wallcache` are decoded without the D3D11 device. On iGPU laptops the full-size decoder's surface pool was counted as Flux memory (hundreds of MB).
  - `main.rs` has a counting global allocator (`HEAP`, `heap_mb()`), shown in Developer → Memory as *Flux code N MB*. Big files (> 300 KB) get a TextEdit undoer with 20 steps (`undo_capped`).
  - Video in the background: after 3 s paused (`set_playing(ctx, false)`), the MF thread stores the position in `Wall.resume` (100 ns), sets `released` and exits (reader, D3D device and `MFShutdown` freed; the last frame stays). `set_playing(ctx, true)` respawns `video::play(…, resumed: true)` from that position.
  - **Live wallpaper modes** (`liveWallMode`):
    - `glass` (0.9.2, *See-through glass (try it)*): the same transparent window as `see`, chosen in Settings. It is a trial: choosing it sets `glassTrial: pending`; `main.rs` → `glass_trial()` turns it into `running` and starts a 25 s guard thread. The app shows *Do you see your live wallpaper through Flux?* (`glass_ui()`); *Yes, keep it* sets `GLASS_OK` and removes `glassTrial`. No answer (invisible/hung window) → `glass_revert()` (`liveWallMode: play`, `glassFailed` note), `boot::closed()`, relaunch. A start that still finds `running` reverts too.
    - `see` (**disabled** since 0.8.4: on Windows the `with_transparent` window was invisible – the process ran without any window; only with `FLUX_SEE_THROUGH=1` for experiments; the default is `still`): no decoding. `main.rs` → `want_transparent()` reads `settings.json` before the window exists and creates it with `with_transparent` when a live wallpaper is cached. `App::clear_color` is then transparent, `wall_source()` returns `None`, and the real desktop (Lively's animation) shows through the translucent base; the card gets alpha 0.94 because there is no blur under it. Changing the mode needs a restart (*Restart Flux* row in Appearance).
    - `still`: `Src::Still` decodes one frame, then drops the reader and D3D.
    - `play`: the MF video path.
  - `settings.liveCache` stores the last detection (`sync_live_cache()`). It is used from the first frame, and without a cache the desktop wallpaper waits up to 2 s for the first detection, so it never flashes. `FLUX_LIVE_TEST=1` skips the "is Lively running" check (the CI memory runs with a fake Lively folder).
  - **Back/forward:** `place()` = `home` / `proj:<dir>` / `file:<path>`; `track_place()` records changes at the end of each frame, `go()` restores them. Mouse buttons 4/5 and Alt+←/→ work everywhere; `FLUX_TEST` actions `back` / `fwd`.
  - `update.rs`: self-update. `native.yml` writes `native.json` (`version`, `sha`, `sha256`, `size`) next to `Flux-Native.exe` in `ci-native`; `build.rs` bakes `GITHUB_SHA` as `FLUX_SHA` (`dev` locally = updates off). A different `sha` = newer build. `curl` downloads to `<exe>.new`, the sha256 is checked, *Restart to update* renames the running exe to `.old`, moves `.new` in and starts it; closing with a ready update only swaps. `cleanup(true)` at start removes only `.new`; `.old` stays until the new build has run 10 s (`cleanup(false)` from `ui()`). The panic hook in `main.rs` writes `userData/flux-native-crash.log`, shows a MessageBox, and within 20 s of start `rollback()` swaps `.old` back and starts it (same when both renderers fail). `FLUX_TEST` action `panic` tests it. Checks at start and every hour (`autoUpdate` = download by itself, and install by itself once the window has been unfocused for 60 s with nothing dirty/running: `install_with(true, ["--background"(, "--minimized")])`). `cleanup(false)` (10 s after start) must never delete `.new` – before 0.9.2 it did, so fast downloads were lost. If `.old` is still running (old Flux, MCP bridge), the exe moves to `.old-<secs>` instead. `run_check()` also runs while `Ready`: without `.new` it checks normally, with `.new` it downloads a newer `sha` over it (before 0.9.6 a Ready state blocked every later check). **Delta updates:** `native.yml` downloads the previous `ci-native` build and runs `Flux-Native.exe --make-patch <old> <new> out\Flux-Native.patch` (zstd with the old exe as ref prefix, level 19, window 2^26, long-distance matching); `native.json.patch = {from: <old sha>, sha256, size}`. `try_patch()` uses it when `patch.from == SHA`, checks the result's sha256 and otherwise falls back to the full exe. Test: `cargo test -p flux-native patch_roundtrip`. UI: About & updates + status bar chip. Test with `FLUX_UPDATE_URL=http://localhost:PORT/` (a folder with `native.json` + `Flux-Native.exe`) and `FLUX_TEST` actions `update`, `update-get`, `update-install`.
  - `server.rs`: Live Server. It is a tiny std-only HTTP server on `127.0.0.1:5500+` over the project, or the file's folder.
    - Every HTML page gets a script that polls `/__flux/ver` and reloads when the number changes. `save()` and `fs:changed` call `bump()`.
    - HTML/CSS files (and JS with an `index.html` next to it) show **Live Server** instead of ▶ Run (`is_web_file()`), and running starts `live_server()`.
    - The single top-bar button turns into **Stop** while a program or the server runs, and the address chip reopens the page.
    - Test: `FLUX_TEST='1:open=index.html;2:run'` + `curl 127.0.0.1:5500/`.
  - `app/preview.rs`: the Live Server panel on the right of the card (`App.preview`, draggable width; toolbar with address, Full/Tablet/Phone, reload, open in browser, ×).
    - On Windows the page is an embedded WebView2 child window (`wry`, `build_as_child(frame)`). `sync_preview()` runs at the end of `ui()`: it sets the bounds (egui points × `zoom_factor`) and hides the web view while anything egui draws on top of it (settings, palette, menus, tour, intro, home screen).
    - Elsewhere, or if WebView2 fails, the panel shows an *Open in browser* card and opens the browser. `FLUX_TEST` action `preview`.
  - `app/home.rs`: Flux's home screen (`openStart()` in `app.js`), opened by the logo, ☰ → Home and `FLUX_TEST` action `start`.
    - It shows a greeting with a localized date (`chrono` `unstable-locales`), total stats, action cards, pinned cards, recent rows with search, recent files, the `START_CHOICES` templates (texts copied from `templates.js`) and the tip of the day.
    - A template writes files into the open project, or creates `python-project` / `my-website` / `js-project` (with `-2`… if taken).
  - Sidebar footer: an absolute rect `[panel bottom − 28, panel bottom]` (the panel has `GAP` bottom margin), so its line is the status bar's top line. Don't place it through the layout flow: item spacing pushed it lower.
  - `app/tools.rs` + `crates/flux-core/src/toolchains.rs`: Settings → Languages. This is a 1:1 port of `src/main/toolchains.js`: probe, `winget install|upgrade` with progress, a PATH refresh from the registry, `addRPath`, and `winget upgrade` for updates. State lives in `App.tools` (a shared `Mutex`); installs run one after another from a queue.
  - `app/tour.rs`: the feature tour (the same 6 steps as `TOUR` in `onboarding.js`). The targets are `App.tour_rects`, filled while drawing (`projects`, `files`, `brand`, `run`, `status`, `settings`); missing ones are skipped. `FLUX_TEST` action `tour=<step>`.
  - `app/newproj.rs`: the *Create a project* window: no language choice; `Where` = *On this computer* / *New on GitHub* (`gh_publish()` → `POST /user/repos`, `git init -b main`, README, first commit, push with the token header; `private` switch) / *Import from GitHub* (`gh_repos()` list or a link parsed by `github::repo_of()`, then `clone()`). Icon picker (`icon_picker`: automatic = main language, line icons incl. `robot`, emoji that egui's default font has), name, description, location. `gh_poll()` runs every frame, so results arrive with the window closed. The icon is `projectMeta[dir].icon` = `line:<name>` / `emoji:<char>` (empty = language); `project_icon()` + `paint_icon()` / `Lead::Project` draw it in the sidebar, home and project page (click the big icon to change it). `FLUX_TEST` action `newproj`.
  - Back/forward: `go()` runs at most once per 150 ms, the mouse buttons act on release, missing places are skipped and a file from another project switches the project first. Disabled icon buttons still swallow clicks (else they fell through to the title-bar drag area).
  - `boot.rs`: `userData/flux-native-start.json` = `{stage, pid, fails}` (`starting` → `window` in `App::new` → `ok` after 3 s of frames → `closed` in `on_exit`, only if the pid is ours). A start after a record stuck at `starting`/`window` kills that pid if it is still a Flux Native process, shows a MessageBox and runs in **safe mode** (`boot::safe()`: no transparent window, `wall_on()` false; `fails >= 2` → `wgpu`). `FLUX_SAFE=1` forces it. `kill_ghosts()` at start terminates other `flux-native*.exe` processes without a visible window; `watchdog()` restarts Flux with `FLUX_SAFE=1` (stage `hidden`) when its own process has no visible window 12 s after start. The CI smoke test also checks `IsWindowVisible`. `catch_hard_crashes()` = `SetUnhandledExceptionFilter` → `report()` (code + address in the crash log). `build.rs` links the Windows main thread with an 8 MB stack.
  - CI `native.yml` has a **smoke test** with real-world settings (home, projects with icons, `pantr1x`, `enc:` keys, `liveCache`, diacritics) and `FLUX_TEST` steps through settings/new project/AI; if the process died, has no window or wrote a crash log, the job fails and `ci-native` (= the update channel) is **not** published.
  - `mem::used_mb()` = private working set (`PROCESS_MEMORY_COUNTERS_EX2`), the Task Manager number; `PrivateUsage` counted GPU-driver commit (~400 MB).
  - Settings (`app/prefs.rs`) copy Electron's `openSettings()`: card `min(1000, 94 %) × min(740, 90 %)` radius 20, nav 210 px (active tab icon in accent), groups = `p.hover` fill radius 14, uppercase 11 px section titles, `Row::Lead` for `.s-lead` paragraphs. Tabs: General, Appearance, Editor, Running, Languages, Plugins (built-in cards), AI, GitHub, Developer.
  - `net.rs`: HTTP through the system `curl`; URL, headers and body go in as a config on stdin (`-K -`), so tokens never show in the process list. `request` / `json` / `stream` (SSE, `-N`) / `bytes`.
  - `secret.rs`: GitHub token (`github`), Anthropic key (`ai`) and MCP tokens (`mcp:<name>`) live in the Windows Credential Manager (`CredWriteW`, targets `Flux Native/<key>`; DPAPI's `CryptUnprotectData` looks like a password stealer to antivirus heuristics), elsewhere in `userData/flux-native-secrets.json` (0600). Electron's `enc:` values can't be decrypted in Rust, so Native never reads or writes `github.token` / `ai.key`; it shares only `github.user`, `ai.model` and `ai.mcp [{name, url, enabled}]`.
  - `app/github.rs`: GitHub tab = `github.js` `renderSettings`: device-flow sign-in (client id from `package.json` → `flux.githubClientId`), token field, avatar, *Open a repository* (repo list, `git clone` into `Documents/Flux Projects` with the token as `http.extraheader` via `GIT_CONFIG_*` env, or reuse an existing clone), plus the Git toolchain row (`tools_ui(…, Some("git"))`). Tests: `FLUX_GITHUB_API` / `FLUX_GITHUB_URL` point at a fake server; `FLUX_TEST` actions `gh-signin`, `gh-pick`.
  - `app/ai.rs`: AI tab (key + *Test*, model = `MODELS` from `ai.js`, MCP connector list/add) and the Claude panel on the right of the card (Ctrl+I, the top-bar AI button, `act("ai")`). Requests stream from `/v1/messages` (`FLUX_AI_URL` for tests) with the same params as `ai.js` (adaptive thinking except Haiku, `mcp_servers` + `mcp_toolset` with the `mcp-client-2025-11-20` beta); content blocks are rebuilt from SSE so `pause_turn` can continue. The open file goes along as `<file>` unless the chip is off; code blocks have Copy / Insert. `FLUX_TEST` actions `ai`, `ask=<question>`.
  - `mcp.rs`: Flux for other AI apps (port of `mcpServer.js`): std `TcpListener` on `127.0.0.1:39217+` (`settings.mcpServer = {enabled, token, port}`, shared with Electron), `/mcp` POST JSON-RPC with `Authorization: Bearer`, Origin check, tools `flux_project_info` / `flux_read_file` / `flux_add_todos` / `flux_update_todo` / `flux_set_description` on `core.workspace`. UI in Settings → AI (`mcp_ui()` in `app/ai.rs`: switch, *Add to Claude Desktop* → `claude_desktop_config.json` `mcpServers.flux = {command: exe, args: ["--mcp-bridge"]}`, copy buttons, *New key*). `Flux-Native.exe --mcp-bridge` (in `main()` before anything else) pipes stdin lines to the server and starts Flux with `--background` (no focus, null stdio, breakaway from the job) when it is not running; it writes `userData/mcp-bridges/<pid>` so `boot::kill_ghosts()` leaves it alone. Ready-made connectors (`PRESETS` in `app/ai.rs`) fill the remote MCP list.
  - `app/newfile.rs`: the *New file* window (language grid `LANGS` with extension + starter code, name, location); every New file entry opens it via `open_new_file(dir)`. `FLUX_TEST` action `newfile`.
  - GitHub is a plugin: `settings.githubPlugin` (same key as Electron; `gh_plugin_migrate()` turns it on once for signed-in users). Off = no GitHub settings tab and no GitHub places in New project. Card with *Install* / *Remove* in Settings → Plugins.
  - Wheel input: `raw_input_hook()` → `Smooth::feed()` scales by `scrollSpeed` (50–300 %) and splits it: whole-line deltas (mouse wheel notches) go to `notches` (raw, ×1.5) and drive a fling (each notch adds `notches × 1.5 / TAU` velocity, the view glides and slows down exponentially with `TAU` = 0.32 s, stops at the edges; jumps from the minimap/search use a fixed 220 ms Hermite segment, `retarget()`); a whole-line delta within 400 ms of touchpad input counts as touchpad too (Windows sends one at the end of a swipe); smooth scrolling depends only on `inertia` and `optAnim`/`lite`, not on `transitions`; fractional deltas (precision touchpad, free-spin wheels) become `Point` units and are applied 1:1 without our spring (Windows sends its own momentum). egui's smoothed `smooth_scroll_delta` over our areas is always taken and dropped, so nothing is smoothed twice.
  - `code.rs`: the syntect `SyntectSettings` is created on the first `highlight()` and dropped by `release_if_idle(120 s)` (≈11 MB of syntax definitions). `reduce_texture_memory` is on (no RAM copy of SVG icons), and `raw_input_hook()` caps `max_texture_side` at 2048 (egui's font atlas is as wide as the GPU's max texture side). `Updater::install_with` re-downloads when `.new` is missing (antivirus).
  - Memory: `Tab { seen, unloaded, stale }` – `unload_idle_tabs()` frees the text of non-dirty tabs unseen for 10 min (`FLUX_UNLOAD_SECS` in tests), `reload_tab()` (every frame for the active tab, cheap) reads it back; `fs:changed` reloads only the active tab and marks the rest `stale`. Project summaries are cached in `userData/native-summaries.json`; at start only the open project and projects without a cache entry are scanned. The editor gutter loop draws only rows inside the clip rect and reads indent from the galley row (no per-frame copies of the text).
  - Live Server serves unsaved text: `Server::set_live(file, text)` (pushed every 120 ms from a dirty html/css/js tab), `clear_live` on save; `/__flux/ver` answers `N:css` or `N:page` and the page swaps only stylesheets for CSS.
  - Settings → **Developer** (its own tab in the settings menu, with sub-items): shown only when `settings.github.user.login` is `pantr1x` (the same shared `settings.json` Electron writes after GitHub sign-in), or with `FLUX_DEV=1`.
    - It has: build + commit, Intro, Tour, *Check now*, *Reinstall the latest build* (`Updater::reinstall`, also on `dev` builds), memory + *Trim*, `devFps` (fps in the status bar), and buttons that open the settings and program folders.
  - Settings menu: the release notes are folded per version (`release_notes()`, parsed once).
    - The sub-items follow scrolling (`SettingsUi.spy`, taken from each section's top; at the end it is the last one). After a click, `spy_hold` keeps the choice for 0.6 s.
    - `scripts/extract-strings.mjs` also collects `Row::…("key", "Title", "Hint")` strings and section titles from the Rust code.
  - `code.rs`: turns the Flux code theme (`settings.codeTheme`, same as Electron; light/dark follows it like `isDark()`) into a syntect theme mapped to TextMate scopes.
- **Look = Electron Flux.** The layout copies `styles.css`: sidebar 248 px (logo, PROJECTS with language icon + "HTML/CSS +3 · 22 files", New project, FILES = loose files, tree toolbar, Settings footer with ⌘ and ☀/☾), top bar 44 px (back/forward, file tabs, search, AI, Run pill, Stop), one card (radius 14, 8 px gap) with the editor (line height 20, gutter, current line, indent guides, minimap), Output/Terminal pills, and the status bar inside the card. Without an open file the card shows the project page (header, language chips, stat tiles, files by kind, TO-DO from `settings.projectMeta`). Colors are the CSS tokens in `theme.rs`. Compare with an Electron screenshot of the same `settings.json`: `FLUX_SIZE=1280x780` sets the window size, and extra `FLUX_TEST` actions are `home`, `light`, `dark`, `theme=<id>`.
- **Renderer:** OpenGL (`glow`) first. If that fails, for example in a VM or on CI, it retries with `wgpu`. `FLUX_RENDERER=wgpu` forces `wgpu`.
- **Test on Linux:** needs `libxkbcommon-dev libxkbcommon-x11-0 libgl1-mesa-dev libgl1-mesa-dri` and Rust ≥ 1.95. Windows-only code (Media Foundation, windows-sys) can be checked with `rustup target add x86_64-pc-windows-gnu && cargo check -p flux-native --target x86_64-pc-windows-gnu`. Run `cargo build -p flux-native`, then `FLUX_USER_DATA=<dir> FLUX_TEST='2:open=main.py;4:run;6:type=Rust\r' LIBGL_ALWAYS_SOFTWARE=1 xvfb-run ./target/debug/flux-native` and take a screenshot with `import -window root x.png`. `FLUX_TEST` steps are `<seconds>:<action>`, where the action is `open=<file in project>`, `run`, `type=<text for the program>`, `terminal` or `shell=<text>`.
- **Windows:** `.github/workflows/native.yml` builds a release and measures the Task Manager memory. The result goes to the job summary and to the branch `ci-native` (`native.png`, `memory.txt`, `native.json` for updates, and the program itself as `Flux-Native.exe`, downloadable at https://github.com/pantr1x/Flux/raw/ci-native/Flux-Native.exe – portable, runs next to Electron Flux with the same settings). Flux Native is deliberately **not** a GitHub release: a non-semver pre-release would be picked by electron-updater (`allowPrerelease`) and break updates.
- **Next phases:** Python autocomplete (an LSP client talking to Pyright), Live Server, Git/GitHub, AI, plugins, themes, updates, installer.

## Flux in Rust (Tauri, `desktop/`) – paused (WebView2 saved only ~10 % RAM)

- **Goal:** Flux with the same UI and much less memory. The window is the system WebView (WebView2 on Windows) showing the unchanged `dist/renderer`, and everything the Electron main process does moves to Rust.
- **Bridge:** `desktop/src/bridge.js` is **generated** from `src/preload.js` by `node scripts/tauri-bridge.mjs`, so both builds share one `window.flux` API. Every call is the Rust command `ipc(ch, args)` with the same channel names as in Electron (`desktop/src/main.rs`). Events use `app.emit(channel, payload)`. After adding an IPC call, regenerate the bridge and add the channel to `main.rs`. Channels that are not ported yet return safe defaults (`null` / `[]`).
- **Ported:**
  - settings: the same `settings.json` as Electron (`FLUX_USER_DATA` overrides the folder),
  - translations,
  - projects, file tree and file operations (dialogs via `rfd`, Recycle Bin via `trash`, a watcher via `notify` → `fs:changed`),
  - finding Python,
  - ▶ Run for all languages (`runner.rs` = `commandFor`), in a PTY (`pty.rs`, `portable-pty`),
  - Terminal.
- **Not yet:** Pyright/LSP (needs a Node runtime), updates, GitHub, plugins, AI, MCP, Live Server, backgrounds/wallpaper, toolchains, Flux Together, memory stats.
- **Test on Linux:** `cargo build` in `desktop/` (needs `libwebkit2gtk-4.1-dev`), then `FLUX_USER_DATA=<dir> FLUX_TEST_JS="<js>" xvfb-run ./target/debug/flux`. `FLUX_TEST_JS` runs in the window 8 s after start (debug builds only). Log with `window.__TAURI__.core.invoke('ipc', { ch: 'log', args: [msg] })`, which prints `[okno] …` to stderr.
- **Windows:** `.github/workflows/tauri.yml` builds it and measures its Task Manager memory next to the Electron build (job summary + artifact `Flux-Tauri`).

## Build and check

```bash
npm install
npm start        # build the renderer and start Flux
npm run dist     # Windows installer into release/
```

There are no tests or linter. Before committing, at least check syntax:

```bash
node --input-type=module --check < src/renderer/app.js   # renderer files are ES modules
node --check src/main/main.js                            # main process is CommonJS
```

**Check the UI in the real app, not only the code.** In a cloud session Electron runs headless:

```bash
npm ci --ignore-scripts && node node_modules/electron/install.js && node scripts/build.mjs
xvfb-run -a -s "-screen 0 1280x800x24" node test.mjs   # Playwright `_electron.launch()`
```

Launch with `executablePath: 'node_modules/electron/dist/electron'`, `args: ['--no-sandbox', '--user-data-dir=<tmp>', '.']` and put a `settings.json` in that folder (`onboarded: true`, `projects`, `lastFolder`, `togetherPlugin`…) so the app opens straight into a project. Take screenshots of every screen you touched and also check:

- no element with the `hidden` attribute is still displayed (`[hidden]` is forced to `display: none` globally – keep it that way),
- text does not run under icons or get cut to `…` in cards (`.pj-tile`),
- layouts at narrow widths (AI panel open, Live Server open),
- shortcuts on every screen (home screen, settings, dialogs) – the global key handler in `app.js` returns early per screen, so order matters (settings are checked before the home screen because they can open on top of it).

Flux Together peers can be faked from the test: `app.evaluate(({ BrowserWindow }, p) => BrowserWindow.getAllWindows()[0].webContents.send('lan:peers', p), peers)` with `peers = [{ id, name, state: { project, file, line, col, text, typing, recent } }]` (send again before each check – the real LAN module re-emits every 2 s).

For the website, open `site/index.html` in a browser (or Playwright with `/opt/pw-browsers/chromium` in cloud sessions) and check desktop and phone width (no horizontal scroll).

## Conventions

- **UI text is English and always goes through `t('…')`** (`src/renderer/i18n.js`, `src/main/i18n.js`). Placeholders: `t('Version {v}', { v })`.
- **Code comments are in Slovak**, short, like the surrounding code.
- HTML is built with template strings; always escape user data with `escapeHtml` / `escapeAttr` (or `esc` in smaller modules).
- Settings live in `state.settings`; read with `setting(key)`, write with `saveSettings({...})`. Inputs with `data-key` in the settings panel are saved automatically.
- Icons: `icon(name, size)` from `src/renderer/icons.js`.

## Settings panel

`openSettings()` in `src/renderer/app.js` builds the whole panel:

- `tabs` = the left menu (`[id, icon, label]`), each tab is a `<section data-pane="id">`.
- **General** is the first tab. Its parts (each an `<h3>` directly inside the section): *About & updates* (`#g-updates`, filled by `updatesUI.render(el, { compact: true })`), *You*, *Performance*, *Language*, *Welcome*, *Shortcuts* (`#g-keys`, list from `shortcutRows()`).
- **General** and **Appearance** show their `<h3>` parts as sub-items in the left menu (the `.s-sub` loop after `showTab`). A new `<h3>` directly in those sections automatically becomes a menu item.
- The old tab ids `keys` and `about` still work: `state.settingsTab = 'keys'` opens General and scrolls to *Shortcuts* (`jumpTo`).
- The *Search settings* box (`#s-find`) filters rows across all tabs and shows suggestions (`.s-suggest`, arrow keys + Enter jump to the row). Matching: every word must hit the row's name, description, section (`h3`) or tab; related words come from `SYN` (phrases separated by `|`), and one typo (incl. swapped letters) is allowed via `near()`/`fuzzy()`. Add a `SYN` entry when people would search a setting by another word.

## Translations

All app languages are **bundled** (`package.json` → `build.files` ships `locales/*.json`), so switching is instant and works offline. `src/main/i18n.js` merges the bundled file with a newer copy in `userData/locales`, which is downloaded in the background from GitHub (branches in `BRANCHES` – currently `main` and `claude/optimistic-darwin-5i7m9t`). New strings reach users with the next release, or earlier once they are on one of those branches.

Adding strings:

1. Use `t('New text')` in code (add the file to the list in `scripts/extract-strings.mjs` if it is new).
2. `node scripts/extract-strings.mjs` → updates `locales/en.keys.json`.
3. Add translations for all 8 languages (sk, de, es, fr, it, pl, pt, uk) in a new `scripts/locales_vNN.py` (copy the shape of `locales_v11.py`) and import it in `scripts/build-locales.py`.
4. `python3 scripts/build-locales.py` → regenerates `locales/*.json`; it prints missing strings and placeholder mismatches.

Never edit `locales/*.json` by hand – the next build overwrites them.

## Releases (GitHub Releases)

Flux updates itself from **GitHub Releases** of `pantr1x/Flux` (`src/main/updater.js`, electron-updater reads `latest.yml`). To release a version:

1. Raise `version` in `package.json` (semver, e.g. `1.2.1`).
2. Add a `## 1.2.1 – YYYY-MM-DD` section at the top of `CHANGELOG.md`, written for users (simple English, what changed and where to find it).
3. Commit and push to `main` or a `claude/**` branch.

`release.yml` runs when `package.json` or `CHANGELOG.md` change. If `v<version>` has no complete release (installer + `latest.yml`), it builds on Windows, takes the notes for that version from `CHANGELOG.md` (`scripts/release-notes.mjs`) and publishes these assets with the tag `v<version>`:
- `Flux-Setup-<v>.exe` and its `.blockmap`,
- `latest.yml`,
- the **quick update** files `Flux-<v>.asar.gz` and `quick.json` (`scripts/quick-pack.mjs`; see *Updates*). A broken, incomplete release is deleted and built again. If the version was already released, nothing happens – so a fix always needs a new version.

**Developer builds** (only for the Flux developer):
- Set `"devBuild": "<version>.N"` in `package.json`, for example `"devBuild": "1.4.4.1"` next to `"version": "1.4.4"`, and push.
- `release.yml` turns it into the semver `1.4.5-beta.1`: newer than 1.4.4, older than 1.4.5. It sets it with `npm pkg set` before building.
- The app shows it as `1.4.4.1` (`showVer()` in `updatesUI.js`).
- For the next build, raise N. Once you raise `version`, the old `devBuild` no longer matches, and a normal release is built.
- A manual `-beta.N` version also works. Never use another id such as `dev`: electron-updater treats those as custom channels, and a user on one of them would never move back to stable versions.
- The CHANGELOG section is optional (`## 1.4.4.1` works, but the bundled CHANGELOG is shown to everyone offline, so prefer commit messages). Without it, the notes are the last commit messages. `release.yml` publishes the build as a GitHub **pre-release** (not `--latest`). electron-builder writes `latest.yml` even for a beta with the GitHub provider; electron-updater with `allowPrerelease` tries `beta.yml` and falls back to that `latest.yml`, so the workflow uploads `latest.yml`. Only a Flux signed in to the GitHub account **`pantr1x`** gets these builds (`DEV_LOGIN` / `devAllowed()` in `updater.js`, which sets `autoUpdater.allowPrerelease`). That user also sees the *Developer updates* switch in About & updates (`devUpdates`, on by default) and the pre-releases in the release notes. Everyone else, and the website, ignore pre-releases completely. Version comparisons (`newer()` in `updater.js` and `updatesUI.js`) understand pre-release suffixes. To end a beta, release the plain version (`1.5.0` > `1.5.0-beta.N`).

The same release notes appear in the app (Settings → General → About & updates, plus the bundled `CHANGELOG.md` when offline) and on the website.

**Linux (developer builds only, for now)**:
- The `linux` job in `release.yml` runs only for pre-releases. It builds on Ubuntu, because `npm ci` installs the node-pty prebuilt for the runner's platform. It adds `Flux-<v>.AppImage` and `latest-linux.yml` to the same release.
- In the app (`updater.js`), Linux updates only run from an AppImage (`APPIMAGE`) and only while developer updates are on (`useUpdater()`). Other Linux users just see the newest version, like a source checkout.
- A release without `latest-linux.yml` (a Windows-only stable release) counts as "up to date" on Linux, not as an error (`noLinuxBuild()`).
- `package.json` → `build.linux` sets the AppImage target, `desktopName` and `syncDesktopName` (window ↔ `.desktop` entry). The quick update never applies there: the AppImage is read-only, so it falls back to the full AppImage.
- One-command install: `curl -fsSL https://pantr1x.github.io/Flux/install.sh | bash` (`site/install.sh`, deployed with the website but not linked from it). It installs FUSE 2 with the distro's package manager (pacman/apt/dnf/zypper), downloads the newest release that has an AppImage to `~/.local/share/flux/Flux.AppImage` (a name without a version, so electron-updater replaces it in place), and adds `~/.local/bin/flux` plus a `.desktop` entry and icon. `--uninstall` removes them. Without FUSE the `flux` wrapper sets `APPIMAGE_EXTRACT_AND_RUN=1`.
- The see-through background (material `wallpaper`) needs the desktop wallpaper: `src/main/wallpaper.js` → `linuxWallpaper()` reads it from KDE Plasma (`plasma-org.kde.plasma.desktop-appletsrc`, also wallpaper packages), GNOME/Cinnamon/MATE (`gsettings`), XFCE (`xfconf-query`), swww, hyprpaper, feh and nitrogen. On Wayland the window position is unknown (always 0,0), so the wallpaper lines up only when the window is maximized.
- `install.sh` takes the icon out of the AppImage (`--appimage-extract usr/share/icons/hicolor/512x512/apps/flux.png`) and writes the full path into `Icon=`.
- The website still says Linux is *coming soon*.
- Test it locally: `npx electron-builder --linux AppImage --publish never`, then run it with `APPIMAGE_EXTRACT_AND_RUN=1 … --no-sandbox` in a container without FUSE.

## Website (GitHub Pages)

- Source: `site/index.html` (+ `site/icon.png`, `site/images/*.png`). Plain HTML/CSS/JS, no build, no dependencies.
- Deploy: `pages.yml` runs on every push that touches `site/**` (any branch) or by hand (*Actions → Website → Run workflow*). It force-pushes the contents of `site/` to the **`gh-pages`** branch. GitHub Pages must be set once to *Settings → Pages → Deploy from a branch → `gh-pages` / (root)*. Address: https://pantr1x.github.io/Flux/
- The page is styled like the Flux app itself (top bar = window tabs, home-screen stats and action cards, a live editor window, status bar). Colors are CSS variables in `:root`.
- The *downloads* stat and the count per release are the `download_count` of each release's `.exe` (GitHub counts every download, including self-updates).
- **Everything version-related is loaded in the browser from the GitHub API** (`/repos/pantr1x/Flux/releases`): the download buttons (`[data-dl]`) point to the newest `.exe`, the version/size stats, and the *Releases* section (`#releases`) lists every release with its notes (small markdown renderer `md()`) and installer. Nothing on the page has to be changed for a new release. Without the API it falls back to links to GitHub Releases.
- **SEO / GEO**:
  - `<head>` carries the description, canonical, Open Graph and Twitter tags, plus JSON-LD (`SoftwareApplication`, `WebSite`, and a `FAQPage` whose Q&As must match the `#faq` section).
  - `site/images/og.png` (1200×630) is the link preview, and its URLs are absolute.
  - The site also serves `robots.txt`, `sitemap.xml` and `llms.txt`, a plain summary for AI assistants.
  - Platforms: Windows is available, macOS and Linux are *coming soon* (the `.platforms` chips, FAQ, footer and og.png). Update them when a new platform ships.
- **Scrolling**:
  - The wheel moves a target, and the page eases toward it exponentially (`1 - exp(-dt/95)`). Touchpad and keyboard scrolling stay native.
  - Keep scrolling cheap: the background `.wall` is plain radial gradients on its own layer, and cards have no `backdrop-filter`. Blur filters there made scrolling stutter. Only the sticky top bar blurs.
- Content lives in arrays in the script at the bottom: `FILES` (files in the demo editor: tokens per line + what *Run* prints), `FEATURES`, `LANGS`, `KEYS` (shortcuts). Edit those to change the page.
- The app links to `https://pantr1x.github.io/Flux/#releases` from Settings → General → *All versions*.

## Plugins

See `docs/PLUGINS.md`. The Flux team's plugins are in `plugins/flux.*`, listed in `plugins/index.json`. **Nothing is bundled or turned on by itself**: `BUILTIN_IDS` in `src/main/plugins.js` is empty and `package.json` → `build.files` does not ship `plugins/`. A plugin is downloaded from GitHub (same branches as translations) only when the user presses **Install** in Settings → Plugins. For local testing, `FLUX_PLUGIN_REGISTRY=/path/to/plugins` makes the store read that folder instead of GitHub.

**Install check**: `src/main/pluginScan.js` holds the code rules (eval, `window.flux`, Node/Electron, remote code, obfuscation, network…). `plugins.install()` scans community plugins before installing and returns `needsConfirm` with the findings, `pluginsUI.js` shows an `askDialog` warning (*Cancel* is the default) and installs with `force` only on *Install anyway*. `scripts/plugin-review.mjs` uses the same rules for pull requests.

**Ratings and comments**: 1–5 stars plus text.
- **Storage.** Each plugin has a GitHub issue in `pantr1x/Flux` (`issue` in `index.json`). A review is a comment on that issue, starting with `<!-- flux-review stars=N -->` and ★★★★☆.
- **Rules.** One rating per user: the latest counts, and `review()` PATCHes your own comment. A comment without stars is a plain comment.
- **Access.** Reading works without a login (60 requests/h, so the catalog is cached for 10 min); writing needs GitHub sign-in (`githubApi`).
- **Code.**
  - `plugins.js`: `reviews` / `review` / `deleteComment` / `summary`.
  - `pluginsUI.js`: `renderReviews`, `rateSum`, `starRow` (half stars via `--f`).
- **Screenshots.** They open in `openShots()`, a lightbox: ← → / wheel to switch, click to zoom and drag to pan, Esc to close. The global key handler in `app.js` returns early while `.pl-lightbox` is open, so Esc does not close the settings.
- **New plugins.** Add a new issue titled `Plugin reviews: <name> (<id>)` and put its number in `index.json`.

*Built into Flux* in the store are features that live in the app code, not in `plugins/`: **GitHub** and **Flux Together** (list in `app.js`, `createPluginsUI({ builtins })`). Each has `description` (card) and `details` (bullets on its own page, `renderBuiltinDetail` in `pluginsUI.js`).

## Layout, menu and search

- **Menu** (`src/renderer/menubar.js`, items in `appMenus()` in `app.js`): opens from `#btn-appmenu` (☰ next to the `.brand` logo – the logo itself opens the start screen), from `#btn-menu` (☰ in the top bar, shown only when the sidebar is hidden) and from the home screen's ☰ (`data-act="menu"`, re-rendered each time, so it uses `menubar.openAt(el)`; `data-menu-trigger` keeps the outside-click handler from closing it). Submenus switch with a short delay so moving the mouse diagonally does not jump to another one. Items are `[label, action, shortcut, { checked, radio, disabled }]`, `'-'` = separator, a plain string = caption; a top-level entry with `run` (Home) is a direct action. `menuBar: true` also shows the classic `#menubar` row in the top bar (default off – it takes too much room).
- **Search** is the `#topsearch` magnifier (opens `searchEverything()`); `showSearch: false` hides it, `searchWide: true` (`body.search-wide`) turns it into a wide search field (same button, text + shortcut shown).
- **Panel position** `panelPos` = `bottom` | `right` | `left` (body classes `panel-side`, `panel-right`, `panel-left`; `#card` is a CSS grid). Width `panelWidth` / `--panel-w`, height `panelHeight`. **Sidebar** `sidePos` = `left` | `right` (`body.side-right`, `#app` row-reverse; on Windows the window buttons then sit above the sidebar, so `.side-top` gets the caption padding). Change them with `setLayout(patch)` – it re-lays out Monaco and xterm.
- Body classes from settings are set in `applyCustomization()`, which also runs once at start (after `layoutEvents()`).
- The intro (`onboarding.js`, step *extras*) asks about `autoUpdate` (*Automatically* / *Ask me first*).

## Rename everywhere

- **Editor offer**: `renameEverywhere()` in `editorExtras.js`.
  - Edit an identifier, or the text inside quotes, after moving the cursor there yourself. When the cursor leaves it, the offer appears: *Rename all* (this file) and *Also in N other files*.
  - Identifiers skip comments and strings; this uses Monaco tokens.
  - Strings match the same content or longer paths that start with it (`old/…`).
  - For paths, `changedPrefix()` first cuts both values to the end of the changed segment. Changing `Desktop` in `C:/…/Desktop/python/a.py` then renames every path under `C:/…/Desktop`.
- **Sidebar rename**: `offerPathRefs()` in `app.js`, after `renameItem()`.
  - It finds quoted paths that really resolve to the renamed file or folder: relative to the file, to the project, or `/…` as web root.
  - It then offers to update them.
- **Shared logic**: `src/renderer/refactor.js`, which contains `scanOthers`, `pathRefs` and `apply`.
  - Open files are changed through their Monaco model, so Ctrl+Z works. Closed files are written to disk.

## Programming languages

A language needs: an entry in `TOOLCHAINS` (`src/main/toolchains.js` – `probe` command, `winget` package id, sizes, `url`), how to run a file in `commandFor()` and the extension in `toolchainFor()` (`src/main/runner.js`), and in the UI: `LANGS`/`RUNNABLE`/`KIND_FILE`/`KIND_LANG_NAMES`/`PROJECT_KINDS`/`MAIN_EXT` (`app.js`), `TOOL_FILE` (`tools.js`), `CODE_LANGS` (`onboarding.js`), a template (`templates.js`), an icon + `EXT_ICON` (`icons.js`), `KIND_EXT` and `TEXT_EXT` (`main.js`), comment tokens (`editorExtras.js`) and the website `LANGS`. Only use winget ids the language's own docs name (Zig `zig.zig`, R `RProject.R`, Julia `9NJNWW8PVKMN` = Juliaup from the Store). R does not add itself to PATH – `addRPath()` in `toolchains.js` does it.

## Updates

**Quick update** (`src/main/quickUpdate.js`) – the normal path when only Flux's own code changed:
- **How it works.**
  - `updater.js` sets `autoDownload = false` and `startDownload(v)` first tries `quick.fetch(v)`.
  - That reads `quick.json` from the release, downloads `Flux-<v>.asar.gz` (~5 MB), checks the sha512 and writes `resources/app.asar.new`.
  - The helper starts only in `will-quit`, when Flux really closes. `quick.apply(quitRelaunch)` writes it to TEMP; it is `Flux.exe` run as Node (`ELECTRON_RUN_AS_NODE`). It waits for Flux to exit, swaps `app.asar`, then starts Flux again, keeping `--user-data-dir`/`--no-sandbox`.
  - *Restart and update* sets `quitRelaunch = true` and calls `app.quit()`. A normal quit swaps the file without the restart.
  - Closing can be canceled: unsaved files → *Cancel*. `watchCancel()` handles this for both paths: if Flux is still running after 10 s, the state goes back to `ready` with `canceled`, and `updatesUI` closes its overlay.
- **When it is used.** Only when the release's `base` equals the installed `resources/quick-base.txt` and the install folder is writable (per-user install, not Program Files).
  - `base` is a fingerprint of everything except `app.asar`, the main `.exe` and `app-update.yml`: Electron, the Pyright tar, native modules.
  - `scripts/after-pack.cjs` (electron-builder `afterPack`) computes it.
  - A new Electron, a new basedpyright or a new native module changes `base`. Then Flux silently falls back to the full installer below, as it also does on any error.
  - Do not make those files differ between builds without a reason: e.g. the Pyright tar is written deterministically in `build.mjs`.
- **Testing it.** `FLUX_QUICK_URL` points `quick.json` downloads at another server. On Linux: build `--linux dir`, point `resources/app-update.yml` at a generic provider and set `APPIMAGE`.

**Pyright is one file**:
- `build.mjs` packs `node_modules/basedpyright` into `build/pyright/pyright-<ver>.tar` (deterministic ustar, no `.map` files).
- It is shipped via `extraResources` and excluded from `files`.
- `lsp.js` unpacks it into `userData/pyright/<name>` on first use (Windows `System32\tar.exe`) and queues LSP messages meanwhile.
- **Root.** In the renderer, `lspRoot()` returns the project, or the folder of a loose Python file when no project is open (double-click on a `.py`). `isLibrary` compares against `lsp.root`, so such a file is sent to Pyright, while typeshed and other library files are not.
- The installer used to delete and copy its ~5,400 files on every update. That made updates take about a minute.

**Full installer** – `updater.js` → `install()`: re-checks for a newer version (max 4 s), writes the TEMP marker `flux-relaunch-after-update`, emits `status: 'installing'` and calls `quitAndInstall(false, true)` – the installer runs **visibly**, but on updates (`${isUpdated}`) `build/installer.nsh` skips the welcome page (`skipPageIfUpdated`), the install-mode page (`customInstallMode` keeps the previous per-user / per-machine mode), the folder page (electron-builder) and the finish page (`FluxFinishPre`), so only the progress bar shows – and `customPageAfterChangeDir` → `FluxInstShow` turns that page into a small **“Updating Flux”** window (caption and header text, Back/Next/Cancel and *Show details* hidden, window cut below the progress bar); `customInstall` then starts Flux because of the marker. The installer UI always comes from the **new** version, so installer changes show up on the very next update. Closing Flux normally with a downloaded update still installs silently (`autoInstallOnAppQuit`). The *Windows build* workflow tests exactly this flow (install, then `--updated --force-run`) and fails if the installer waits for a click or Flux does not reopen – **keep it green before releasing installer changes**. It also takes a screenshot every second during the update and force-pushes them (plus `windows.txt`, the visible window titles) to the **`ci-screens`** branch – `git fetch origin ci-screens && git show origin/ci-screens:update-020.png > x.png` to look at them. In the app, `updatesUI.startInstall()` shows the in-app progress overlay; settings show `.up-bar` and the status bar `#st-update` (Updating N % / Restart to update) while a version downloads.

## Backgrounds

- **Own background:** Settings → Appearance → Window → *Background*.
  - **Files:** `settings.bg = { type: 'image' | 'video' | 'youtube', file | id, name }`. Files are copied to `userData/backgrounds` and served as `app://flux/bg/<file>`. The protocol answers Range requests, so videos can loop and seek.
  - **History:** `settings.bgHistory` keeps the last 12 backgrounds. A file is deleted when it drops out of the list.
  - **Migration:** the old `settings.bgImage` is moved into this format at start (`migrateBackground()`).
- **Main process** (`main.js`):
  - `backgroundInfo()`, `useBackground()` and the `app:background*` IPC handlers.
  - A video or YouTube background counts as a wallpaper for `materialMode()` (`movingBackground()`).
  - YouTube embeds need a `Referer`: `onBeforeSendHeaders` sets the website address for `youtube-nocookie.com`.
- **Renderer:**
  - `setupWallpaper()` → `setMedia()` puts a `<video>` or a YouTube `<iframe>` into `#wall .wall-media` (`#wall.moving` hides the image layer). The CSP allows `frame-src https://www.youtube-nocookie.com` and `media-src 'self' blob:`.
  - `renderBgHistory()` draws the thumbnails.
  - **Moving backgrounds:**
    - A video is not shown directly: it stays a hidden 1×1 `.wall-src` and is drawn into a small `<canvas>` (1/8 of the display, at most 15 fps, `requestVideoFrameCallback`).
    - `body.moving-bg` turns off every `backdrop-filter`, because the panels would re-blur on each frame.
    - **Baked video:** `bake()` in `setupWallpaper()` records one loop of the *live* video (no second decoder) into a MediaRecorder on a blurred 1/8-size canvas (same cover crop as the live canvas, `blur(wallBlur/8) saturate(1.35)` = the `.wall-media` CSS). It waits for `bounds` (`pendingBake`), so the copy has the screen's size; recording pauses with the video. It saves the result with `bg:save-baked` to `userData/backgrounds/baked/<key>.webm` (the newest 3 are kept, key starts with `v2|`), served as `app://flux/baked/…`. `showBaked()` fades a new `.wall-media.baked` box in, then removes the live one.
      - `useBaked()` plays that small webm directly, with `.wall-media.baked` = no CSS filter.
      - The key is a hash of the video URL, `wallBlur` and the display size.
    - The *Windows wallpaper* button (`app:reset-background`) also sets `liveWallpaper: false`.
  - **Performance:** both wall layers (`.wall-img`, `.wall-media`) are laid out at 1/4 of the display size and scaled up 4× (`place()` in `setupWallpaper()`), with the blur divided by 4 in CSS. A background video plays only while the window is visible and focused (`syncVideo()`). Without `optAnim` it shows a still frame, and YouTube shows its thumbnail.
- **Default material `auto`** (`materialMode()` in `main.js`) = `wallpaper`, drawn by Flux. It shows only the desktop, including Lively or Wallpaper Engine, not the windows behind Flux, which Acrylic would show. The user did not want that. `migrateMaterial()` switches the old saved `wallpaper` to `auto` once (`materialAuto`). Live wallpaper detection runs only when Flux draws the wall (`drawsWall()`).
- **Sharp wallpaper (default, `wallBlur: 0`)**: `sharpWall()` (blur < 4) lays both wall layers out at full size (`scaleOf()` = 1 instead of 4). A video plays directly as `.wall-direct` (hardware decoded), without a canvas or baking. With a blur, the 1/4 layer, canvas and baked copy are used as before. `placeWall()` re-lays out the layers when the blur changes.
- **Acrylic stays on when inactive:** `src/main/keepActive.js`.
  - Windows greys Acrylic/Mica when the window loses focus. On `blur`, `restore` and `show`, `poke()` sends the window `WM_NCACTIVATE(TRUE)`, so DWM keeps the active backdrop. The focus does not change.
  - The message is sent by a tiny C# helper, `userData/bin/flux-keepactive.exe`. `ensure()` compiles it once with the `csc.exe` of .NET Framework 4 in `%WINDIR%`, so there is no native module, and it changes nothing in the quick-update `base`.
  - Without `csc`, Acrylic simply greys as before. The `keepAcrylic: false` setting turns it off.
  - `FLUX_FAKE_MICA=1` makes Linux tests resolve `auto` to `acrylic`.
- **MCP bridge never steals focus:** when `mcp-bridge.js` (run by Claude Desktop) has to start Flux, it passes `--background`. `second-instance` ignores such starts, and a first start with it uses `showInactive()`. The bridge file in `userData` is refreshed at every start while the MCP server is on.
- **Live wallpapers:** `src/main/liveWallpaper.js` → `detect()`.
  - **When it runs:** only when there is no own background and `liveWallpaper` is not off. The result is cached for 10 s.
  - **Lively Wallpaper:** it reads `%LOCALAPPDATA%\Lively Wallpaper\WallpaperLayout.json`, or the Store package path, and then each `LivelyInfo.json`.
  - **Wallpaper Engine:** it reads `steamapps\common\wallpaper_engine\config.json` (Steam path from the registry and `libraryfolders.vdf`), then `project.json` (a video file, or else the `preview`).
  - **Running check:** only while the program runs, checked with `tasklist`: `lively.exe`, `wallpaper32/64.exe`.
  - **Serving the file:** videos and GIFs go through `app://flux/live/…`, which serves only the currently detected file. Images go through `wallpaperPath()`.
  - **Testing off Windows:** set `LOCALAPPDATA` and `FLUX_STEAM_PATH` to fake folders.

## Smooth scrolling and transitions

- `inertia` (*Smooth scrolling with inertia*, Appearance → Window) controls both the editor (`inertiaScroll()` in `app.js`, a capture wheel listener on `#editor`) and every other scrollable element (`src/renderer/smoothScroll.js` – a global wheel listener on the nearest scrollable parent). Both move toward a **target** that the wheel sets, as a critically damped spring (`springStep()`, the exact solution for a step `dt`, `w = 0.016`/ms), with `dt` taken from the `requestAnimationFrame` timestamp. The speed never jumps on the next wheel notch. Both older models felt choppy: velocity + friction, and plain `1 - exp(-dt/τ)` easing, which is sawtooth-like with notches. The editor also calls `editor.render()` right after `setScrollTop()`, so Monaco does not draw a frame late. In the editor, while the sticky header is shown, the target is rounded to a whole line (from `getTopForLineNumber(1)`, since lines have a top padding), and `stickyFix()` → `snap()` skips while `scrollAnimating`, so the view does not jump once more at the end. Elements that must keep native scrolling: add class `no-smooth` (Monaco, xterm, iframes, selects and range inputs are skipped already).
- `transitions` (*Transition animations*) sets `body.fx-trans`; CSS plays `fx-in`/`fx-fade` when settings panes, Output/Terminal or the project page appear, and `playTransition(el, dir)` restarts the animation (in `activate()` the editor slides in from the side of the new tab). `slideIndicator(box, activeEl, key)` draws a `.slide-ind` highlight that glides to the selected item (file tabs, tree, projects, settings menu); call it after re-rendering such a list. Its position comes from the `offsetLeft/Top` chain (not affected by running animations) and a `ResizeObserver` re-aligns it when rows change size later (e.g. project stats load async).
- Anything that only becomes visible through an animation (`forwards`, e.g. the intro logo `.ob-draw`) needs a `body.no-anim` rule with its final state – `no-anim` removes all animations.
- Animation loops: the `requestAnimationFrame` timestamp can be older than the `performance.now()` taken at the last wheel event – clamp `dt` so it is never negative (see `inertiaScroll()` and the website). `body.no-anim` (Memory & speed) wins over it.

## Memory & speed

Settings → General → *Memory & speed*. `lite` (*Save memory*) is the master switch. Each part in *Advanced* has its own key and, while unset, follows `lite`: `optOn(key)` = `settings[key] ?? !lite` (same helper in `app.js` and `main.js`).

| Key | Off means |
|---|---|
| `optPyAc` | Pyright is never started (`ensureLsp`) |
| `optFx` | `body.no-fx` (no backdrop blur, no wallpaper) and window material `none` |
| `optAnim` | `body.no-anim` (no CSS animations/transitions), no smooth scrolling |
| `optEditorFx` | no sticky scroll, code map, smooth caret, word highlight |
| `optJsLimit` (on = limit) | `--max-old-space-size=512` for the window, after restart |
| `pyMemory` | MB for Pyright (default 768 with `lite`, else 2048) |
| `lspIdle` | minutes until Pyright stops without a Python file (0 = never) |

Always on, whatever the settings:
- **Network service in the main process:** `enable-features=NetworkServiceInProcess2`, so there is no separate Utility process.
- **GPU:**
  - Do **not** use `in-process-gpu`. On Windows it left the window blank in 1.4.35.3, although it worked under xvfb.
  - If the GPU process dies (`child-process-gone`, `type: 'GPU'`), Flux sets `settings.gpuSafe = true`, and later starts use `--disable-gpu-compositing`.
- **Code splitting:** `scripts/build.mjs` builds the renderer with esbuild `splitting: true` (`dist/renderer/chunks/`). Every `import()` becomes its own file, loaded only when needed, e.g. Monaco language tokenizers and modes. Emmet (`emmet-monaco-es`) is imported at the first HTML/CSS model (`needEmmet`). Measured: start 1.9 s → 1.6 s, window process −11 MB.
- **V8:** `js-flags=--optimize-for-size` (plus `--max-old-space-size=512` with `optJsLimit`). One `js-flags` switch only, because a second `appendSwitch` would replace the first.
- **Working-set trim (Windows, `trimMemory`, default on):**
  - When the window has been unfocused for 20 s, or minimized for 1 s, and then every 2 min, `keepActive.trim()` runs the helper as `flux-keepactive.exe trim <pids>`. The pids are all of `getAppMetrics()` plus Pyright.
  - The helper calls `K32EmptyWorkingSet` (VERSION 2), and Windows moves unused pages out of RAM, as Chromium does on minimize. The Task Manager number drops well below 100 MB in the background.
  - An active Electron window cannot get near 100 MB: main, GPU and window each have a Chromium baseline, and Flux's own JS is only ~30 MB of it.
  - The helper is compiled ~10 s after start on Windows, whatever the material.
- **Idle trimming:** 2 minutes after the window loses focus, the session cache is cleared and `webFrame.clearCache()` runs (`app:trim`).
- **Pyright** stops after 5 minutes of the window being unfocused.
- **Project stats:** `projectStats()` in `main.js` scans only the open project (`workspace`). Other projects get the numbers saved at their last visit (`userData/project-stats.json`, `files: null` when never scanned).
  - Inside the open project, a file is read only when its mtime or size changed. The per-file lines and chars live in `project-stats.json` → `files[dir]`.
  - `FLUX_STATS_DEBUG=1` adds a `reads` counter to the result, for tests.
- **File list:** `fs:list-all` is cached (`listCache`) until the project watcher or a Flux file operation (`staleStats`) sees a change.
- **Pyright** runs with `diagnosticMode: 'openFilesOnly'` and `indexing: false`. It reads the open files and what they import, and does not index the whole project.
- **Memory breakdown:** `app:memory` returns a breakdown by process type, which the settings show.

Changing `lite` or pressing *Reset advanced* clears all these keys. Use `optOn()` for new savings, never `setting('lite')` directly.
