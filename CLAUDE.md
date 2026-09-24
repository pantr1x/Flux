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
- The *Search settings* box filters `.s-row` rows across all tabs.

## Translations

The app ships English only; other languages are downloaded from GitHub when picked (`src/main/i18n.js`, branches listed in `BRANCHES` – currently `main` and `claude/optimistic-darwin-5i7m9t`). **New strings reach users only after they are on one of those branches.**

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

The same release notes appear in the app (Settings → General → About & updates, plus the bundled `CHANGELOG.md` when offline) and on the website.

## Website (GitHub Pages)

- Source: `site/index.html` (+ `site/icon.png`, `site/images/*.png`). Plain HTML/CSS/JS, no build, no dependencies.
- Deploy: `pages.yml` runs on every push that touches `site/**` (any branch) or by hand (*Actions → Website → Run workflow*). It force-pushes the contents of `site/` to the **`gh-pages`** branch. GitHub Pages must be set once to *Settings → Pages → Deploy from a branch → `gh-pages` / (root)*. Address: https://pantr1x.github.io/Flux/
- The page is styled like the Flux app itself (top bar = window tabs, home-screen stats and action cards, a live editor window, status bar). Colors are CSS variables in `:root`.
- **Everything version-related is loaded in the browser from the GitHub API** (`/repos/pantr1x/Flux/releases`): the download buttons (`[data-dl]`) point to the newest `.exe`, the version/size stats, and the *Releases* section (`#releases`) lists every release with its notes (small markdown renderer `md()`) and installer. Nothing on the page has to be changed for a new release. Without the API it falls back to links to GitHub Releases.
- Content lives in arrays in the script at the bottom: `FILES` (files in the demo editor: tokens per line + what *Run* prints), `FEATURES`, `LANGS`, `KEYS` (shortcuts). Edit those to change the page.
- The app links to `https://pantr1x.github.io/Flux/#releases` from Settings → General → *All versions*.

## Plugins

See `docs/PLUGINS.md`. Built-in plugins are in `plugins/flux.*`, listed in `plugins/index.json`; the app downloads plugin files from the same branches as translations.
