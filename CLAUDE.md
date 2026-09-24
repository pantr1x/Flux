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
| `.github/workflows/` | `release.yml`, `pages.yml`, `windows-build.yml`. |

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

`release.yml` runs when `package.json` or `CHANGELOG.md` change. If `v<version>` has no complete release (installer + `latest.yml`), it builds on Windows, takes the notes for that version from `CHANGELOG.md` (`scripts/release-notes.mjs`) and publishes `Flux-Setup-<v>.exe`, its `.blockmap` and `latest.yml` with tag `v<version>`. A broken, incomplete release is deleted and built again. If the version was already released, nothing happens – so a fix always needs a new version.

**Developer builds** (only for the Flux developer):
- Set `"devBuild": "<version>.N"` in `package.json`, for example `"devBuild": "1.4.4.1"` next to `"version": "1.4.4"`, and push.
- `release.yml` turns it into the semver `1.4.5-beta.1`: newer than 1.4.4, older than 1.4.5. It sets it with `npm pkg set` before building.
- The app shows it as `1.4.4.1` (`showVer()` in `updatesUI.js`).
- For the next build, raise N. Once you raise `version`, the old `devBuild` no longer matches, and a normal release is built.
- A manual `-beta.N` version also works. Never use another id such as `dev`: electron-updater treats those as custom channels, and a user on one of them would never move back to stable versions.
- The CHANGELOG section is optional (`## 1.4.4.1` works). Without it, the notes are the last commit messages. `release.yml` publishes the build as a GitHub **pre-release** with `beta.yml` (not `--latest`). Only a Flux signed in to the GitHub account **`pantr1x`** gets these builds (`DEV_LOGIN` / `devAllowed()` in `updater.js`, which sets `autoUpdater.allowPrerelease`). That user also sees the *Developer updates* switch in About & updates (`devUpdates`, on by default) and the pre-releases in the release notes. Everyone else, and the website, ignore pre-releases completely. Version comparisons (`newer()` in `updater.js` and `updatesUI.js`) understand pre-release suffixes. To end a beta, release the plain version (`1.5.0` > `1.5.0-beta.N`).

The same release notes appear in the app (Settings → General → About & updates, plus the bundled `CHANGELOG.md` when offline) and on the website.

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

*Built into Flux* in the store are features that live in the app code, not in `plugins/`: **GitHub** and **Flux Together** (list in `app.js`, `createPluginsUI({ builtins })`). Each has `description` (card) and `details` (bullets on its own page, `renderBuiltinDetail` in `pluginsUI.js`).

## Layout, menu and search

- **Menu** (`src/renderer/menubar.js`, items in `appMenus()` in `app.js`): opens from `#btn-appmenu` (☰ next to the `.brand` logo – the logo itself opens the start screen) and from `#btn-menu` (☰ in the top bar, shown only when the sidebar is hidden). Submenus switch with a short delay so moving the mouse diagonally does not jump to another one. Items are `[label, action, shortcut, { checked, radio, disabled }]`, `'-'` = separator, a plain string = caption; a top-level entry with `run` (Home) is a direct action. `menuBar: true` also shows the classic `#menubar` row in the top bar (default off – it takes too much room).
- **Search** is the `#topsearch` magnifier (opens `searchEverything()`); `showSearch: false` hides it, `searchWide: true` (`body.search-wide`) turns it into a wide search field (same button, text + shortcut shown).
- **Panel position** `panelPos` = `bottom` | `right` | `left` (body classes `panel-side`, `panel-right`, `panel-left`; `#card` is a CSS grid). Width `panelWidth` / `--panel-w`, height `panelHeight`. **Sidebar** `sidePos` = `left` | `right` (`body.side-right`, `#app` row-reverse; on Windows the window buttons then sit above the sidebar, so `.side-top` gets the caption padding). Change them with `setLayout(patch)` – it re-lays out Monaco and xterm.
- Body classes from settings are set in `applyCustomization()`, which also runs once at start (after `layoutEvents()`).
- The intro (`onboarding.js`, step *extras*) asks about `autoUpdate` (*Automatically* / *Ask me first*).

## Programming languages

A language needs: an entry in `TOOLCHAINS` (`src/main/toolchains.js` – `probe` command, `winget` package id, sizes, `url`), how to run a file in `commandFor()` and the extension in `toolchainFor()` (`src/main/runner.js`), and in the UI: `LANGS`/`RUNNABLE`/`KIND_FILE`/`KIND_LANG_NAMES`/`PROJECT_KINDS`/`MAIN_EXT` (`app.js`), `TOOL_FILE` (`tools.js`), `CODE_LANGS` (`onboarding.js`), a template (`templates.js`), an icon + `EXT_ICON` (`icons.js`), `KIND_EXT` and `TEXT_EXT` (`main.js`), comment tokens (`editorExtras.js`) and the website `LANGS`. Only use winget ids the language's own docs name (Zig `zig.zig`, R `RProject.R`, Julia `9NJNWW8PVKMN` = Juliaup from the Store). R does not add itself to PATH – `addRPath()` in `toolchains.js` does it.

## Updates

`updater.js` → `install()`: re-checks for a newer version (max 4 s), writes the TEMP marker `flux-relaunch-after-update`, emits `status: 'installing'` and calls `quitAndInstall(false, true)` – the installer runs **visibly**, but on updates (`${isUpdated}`) `build/installer.nsh` skips the welcome page (`skipPageIfUpdated`), the install-mode page (`customInstallMode` keeps the previous per-user / per-machine mode), the folder page (electron-builder) and the finish page (`FluxFinishPre`), so only the progress bar shows – and `customPageAfterChangeDir` → `FluxInstShow` turns that page into a small **“Updating Flux”** window (caption and header text, Back/Next/Cancel and *Show details* hidden, window cut below the progress bar); `customInstall` then starts Flux because of the marker. The installer UI always comes from the **new** version, so installer changes show up on the very next update. Closing Flux normally with a downloaded update still installs silently (`autoInstallOnAppQuit`). The *Windows build* workflow tests exactly this flow (install, then `--updated --force-run`) and fails if the installer waits for a click or Flux does not reopen – **keep it green before releasing installer changes**. It also takes a screenshot every second during the update and force-pushes them (plus `windows.txt`, the visible window titles) to the **`ci-screens`** branch – `git fetch origin ci-screens && git show origin/ci-screens:update-020.png > x.png` to look at them. In the app, `updatesUI.startInstall()` shows the in-app progress overlay; settings show `.up-bar` and the status bar `#st-update` (Updating N % / Restart to update) while a version downloads.

## Smooth scrolling and transitions

- `inertia` (*Smooth scrolling with inertia*, Appearance → Window) controls both the editor (`inertiaScroll()` in `app.js`, a capture wheel listener on `#editor`) and every other scrollable element (`src/renderer/smoothScroll.js` – a global wheel listener that moves the nearest scrollable parent with velocity + friction). Elements that must keep native scrolling: add class `no-smooth` (Monaco, xterm, iframes, selects and range inputs are skipped already).
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

Changing `lite` or pressing *Reset advanced* clears all these keys. Use `optOn()` for new savings, never `setting('lite')` directly.
