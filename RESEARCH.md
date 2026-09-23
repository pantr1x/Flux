# Research: moderný mini-IDE (Python run + Live Server) v štýle Zen Browser

> Cieľ: aplikácia, kde otvorím priečinok, kliknem **▶ Run** a Python skript sa spustí (bez `cd ... && python program.py`),
> pre web mám **Live Server** s automatickým reloadom, editor má **suggestions** (autocomplete, chyby, návrhy)
> a celé to vyzerá čisto a moderne ako **Zen Browser**.

---

## 1. Ako to robia ostatní

### VS Code
| Časť | Ako to funguje |
|---|---|
| Shell | **Electron** (Chromium + Node.js), UI v TypeScripte |
| Editor | **Monaco** (samostatne použiteľný balík `monaco-editor`) |
| Run Python | Python extension nájde interpreter (`.venv`, conda, systémový), tlačidlo ▶ spustí `python "cesta/k/suboru.py"` v integrovanom termináli. Konfigurácia v `launch.json` / `tasks.json` |
| Code Runner (extension) | Mapa `prípona → príkaz` (`.py → python -u $fullFileName`), spustí v Output paneli alebo termináli |
| Terminál | **xterm.js** + **node-pty** (skutočný pseudoterminál → funguje `input()`, farby, Ctrl+C) |
| Live Server (extension od Ritwick Dey) | Lokálny HTTP server + file watcher; do každého HTML **vstrekne malý `<script>`**, ktorý cez **WebSocket** počúva a pri zmene spraví `location.reload()` (pri CSS len vymení `<link>` bez reloadu) |
| Suggestions | **LSP** (Language Server Protocol) – Pylance/Pyright pre Python, `vscode-html/css/json-languageserver` pre web, **Emmet** zabudovaný |

### PyCharm (IntelliJ platforma)
- Java/Kotlin, vlastný UI toolkit, vlastné (nie LSP) analyzátory → veľmi presné, ale ťažké.
- **Run Configurations**: každý skript má konfiguráciu (interpreter, working dir, argumenty, env premenné). Zelený ▶ pri `if __name__ == "__main__":` → *gutter run icon* – toto je presne ten pocit, ktorý chceš.
- Automaticky vytvára/deteguje **venv** a ponúkne inštaláciu chýbajúcich balíkov (`import requests` → „Install package requests“).
- Traceback v konzole má **klikateľné odkazy** na súbor:riadok.

### Ďalšie inšpirácie
- **Zed** (Rust, GPU-renderované UI „GPUI“) – extrémne rýchly, *tasks* (`tasks.json`) spúšťané z command palety, runnable ▶ pri funkciách/testoch.
- **Thonny** – IDE pre začiatočníkov: jedno veľké tlačidlo Run (F5), jednoduchá premenná-inšpekcia. Dôkaz, že jednoduchosť predáva.
- **Cursor / Windsurf** – forky VS Code s AI (inline completion, chat). Ukazujú, že AI návrhy sú dnes očakávaná vec.
- **Brackets / Phoenix Code** – live preview webu priamo vedľa kódu, klik na element → skok do kódu.
- **Replit** – Run tlačidlo nad všetkým, konzola + webview vedľa seba, automaticky detekuje typ projektu.

### Zen Browser – čo z dizajnu prebrať
Zen je fork Firefoxu, ktorého sila je hlavne v UI:
- **Vertikálny sidebar** namiesto horných tabov → v IDE: sidebar = súbory + otvorené taby + workspaces.
- **Compact mode** – všetok „chrome“ sa schová, ostane len obsah; zobrazí sa pri nabehnutí myšou k okraju.
- **Obsah ako zaoblená „karta“** s malým okrajom, ktorá „pláva“ nad pozadím s jemným gradientom.
- **Workspaces** s vlastnou farbou/ikonou → v IDE: projekty prepínateľné jedným klikom.
- **Split view** → kód vľavo, live preview / výstup vpravo.
- **Glance** (náhľad odkazu v plávajúcom okne) → v IDE: rýchly náhľad súboru / definície bez otvorenia tabu.
- **Essentials** (pripnuté ikony hore v sidebare) → pripnuté súbory / obľúbené príkazy.
- **Zen Mods** a vlastný gradient → témy cez CSS premenné.

---

## 2. Čo presne treba (funkčné bloky)

### 2.1 ▶ Run pre Python
1. **Nájsť interpreter** v poradí: `./.venv`, `./venv`, `$VIRTUAL_ENV`, `uv`/`poetry` projekt, potom `py -3` (Windows) / `python3` / `python`. Výber zobraziť v status bare (ako VS Code).
2. **Spustiť proces** s `cwd = priečinok súboru` (alebo koreň projektu – nastaviteľné), `python -u súbor.py` (`-u` = nebufferovaný výstup, aby sa text zobrazoval hneď).
3. **PTY (pseudoterminál)**, nie obyčajná rúra – inak nefunguje `input()`, farby a Ctrl+C.
   - Electron: `node-pty` + `xterm.js`; Tauri/Rust: `portable-pty` + `xterm.js`; Python/Qt: `QProcess` (bez PTY) alebo `ptyprocess`/`pywinpty`.
4. **Stop** = zabiť celý strom procesov (na Windows `taskkill /T /F`, inak process group `kill(-pgid)`).
5. **Re-run** (Ctrl+Enter / F5), **argumenty a env** v jednoduchom „run profile“ (ekvivalent `launch.json`, ale s UI formulárom).
6. **Klikateľný traceback**: regex `File "(.+)", line (\d+)` → odkaz, ktorý otvorí súbor na riadku.
7. **Chýbajúci balík**: zachytiť `ModuleNotFoundError: No module named 'x'` → tlačidlo „Nainštalovať x“ (`pip install x` / `uv add x`).
8. Bonus: ▶ ikona v gutteri pri `if __name__ == "__main__":` a pri `def test_*` (pytest).

To isté zovšeobecniť na **runner mapu** ako Code Runner: `.js → node`, `.ts → tsx/bun`, `.c → gcc && ./a.out`, `.sh → bash`, `package.json → npm run dev`.

### 2.2 Live Server
1. Statický HTTP server nad priečinkom (voľný port od 5500).
2. **File watcher** (Rust `notify`, Node `chokidar`, Python `watchfiles`) s debounce ~100 ms.
3. Pri servovaní `.html` **vstreknúť pred `</body>`** skript s WebSocket klientom.
4. Pri zmene: `.css` → hot-swap `<link href="...?v=timestamp">` (bez straty stavu), inak `location.reload()`.
5. **Preview panel priamo v aplikácii** (webview v split view) + tlačidlo „Otvoriť v prehliadači“.
6. Extra, ktoré ostatní nemajú dobre:
   - **QR kód** s LAN adresou → otvoríš na mobile a reloaduje sa tiež.
   - **Konzola z preview** (`console.log`/chyby z webovej stránky preposlať do panelu IDE).
   - Prepínač veľkosti zariadenia (mobil/tablet/desktop).
   - Ak projekt má `package.json` so skriptom `dev` (Vite, Next…), namiesto vlastného servera spustiť ten a len zobraziť preview.

### 2.3 Suggestions (inteligentný editor)
| Jazyk | Odporúčaný server | Poznámka |
|---|---|---|
| Python – completion, go-to-def, typy | **basedpyright** alebo **Pyright** (LSP) | Rovnaký engine ako Pylance, open-source |
| Python – lint + format | **Ruff** (`ruff server`, má vstavaný LSP) | Veľmi rýchly, nahrádza flake8/black/isort |
| HTML / CSS / JSON | `vscode-langservers-extracted` | Tie isté servery ako vo VS Code |
| JS / TS | `typescript-language-server` (alebo `vtsls`) | |
| Emmet | `emmet-monaco-es` / `@emmetio/codemirror6-plugin` | `div.card>p*3` + Tab |

- Integrácia: pre Monaco `monaco-languageclient`, pre CodeMirror 6 `@codemirror/lsp-client` (alebo `codemirror-languageserver`).
- **AI návrhy (voliteľné)**: inline „ghost text“ completion cez API (Claude, …) alebo lokálne cez **Ollama**. Plus „Vysvetli chybu“ tlačidlo pri tracebacku – pre začiatočníkov veľmi užitočné.
- **Kontextové návrhy v UI** (nie len v kóde): „Tento projekt nemá venv – vytvoriť?“, „Našiel som `index.html` – spustiť Live Server?“, „Našiel som `requirements.txt` – nainštalovať?“.

---

## 3. Voľba technológie

| Možnosť | + | − | Veľkosť aplikácie |
|---|---|---|---|
| **Tauri 2** (Rust backend + web UI) | malé, rýchle, moderné, systémový webview, výborné pre „Zen“ vzhľad (CSS) | treba trochu Rustu; webview sa líši medzi OS (WebKit na macOS/Linux) | ~5–15 MB |
| **Electron** (Node + Chromium) | najviac hotových knižníc (node-pty, xterm.js, Monaco – presne ako VS Code), všade rovnaký render | ťažký, viac RAM | ~100+ MB |
| **Python + PySide6 (Qt)** | celé v Pythone, ktorý poznáš | moderný „Zen“ vzhľad je v Qt ťažší, editor (QScintilla) slabší než Monaco, LSP treba písať ručne | ~60+ MB |
| **Rozšírenie do VS Code / fork (Code-OSS)** | run, terminál, LSP zadarmo | vzhľad obmedzený, nie „vlastná appka“ | – |

### Editor
- **Monaco** – to isté čo VS Code, najbohatšie funkcie, ale ťažšie na theming a mobil.
- **CodeMirror 6** – ľahší, modulárny, veľmi dobre sa štýluje CSS-kom → lepší pre vlastný moderný vzhľad.

### Odporúčanie
**Tauri 2 + Svelte (alebo React) + CodeMirror 6 + xterm.js**, backend v Ruste (`portable-pty`, `notify`, `axum`/`tiny_http` + WebSocket pre Live Server), LSP servery spúšťané ako podprocesy a prepojené cez stdio ↔ frontend.

Ak chceš čo najrýchlejšie výsledky bez Rustu → **Electron + Monaco + node-pty + xterm.js** (najviac hotových riešení, najmenej prekvapení).

---

## 4. Architektúra (návrh pre Tauri)

```
┌──────────────────────── Frontend (Svelte + CSS) ────────────────────────┐
│ Sidebar (súbory, workspaces) │ Editor (CodeMirror 6) │ Preview / Output │
│ Command palette (Ctrl+K)     │ LSP klient            │ xterm.js         │
└───────────────▲─────────────────────────▲─────────────────────▲─────────┘
                │ Tauri IPC (commands + events)                 │
┌───────────────┴───────────── Backend (Rust) ──────────────────┴─────────┐
│ fs + watcher (notify) │ runner (portable-pty, kill tree) │ live server  │
│ interpreter detection │ LSP proxy (stdio ↔ IPC)          │ (HTTP + WS)  │
└─────────────────────────────────────────────────────────────────────────┘
          │                         │                           │
     python/.venv            pyright, ruff,               prehliadač / mobil
                              html/css/ts LSP
```

---

## 5. Dizajn (Zen-like) – konkrétne pravidlá
- Pozadie: jemný **gradient** (farba workspace), editor ako **karta** so `border-radius: 12px`, okraj 8 px, jemný tieň.
- **Vertikálny sidebar** vľavo, šírka meniteľná, skrývateľný; hore „essentials“ (pripnuté súbory/akcie).
- **Compact mode** (Ctrl+Shift+B?): skryje sidebar aj toolbar, ukážu sa pri hoveri na okraj.
- **Jedna horná lišta** len s: názov projektu · ▶ Run · ⏹ Stop · 🌐 Live · výber interpretera.
- **Command palette** (Ctrl+K / Ctrl+Shift+P) na všetko – „Run“, „Open preview“, „Create venv“, „Change theme“.
- Typografia: UI `Inter`/systémové písmo, kód `JetBrains Mono` / `Geist Mono`; veľa prázdneho priestoru, žiadne zbytočné ikony.
- Svetlá/tmavá téma + akcentová farba na workspace; animácie krátke (150–200 ms), rešpektovať `prefers-reduced-motion`.

---

## 6. Roadmapa (MVP → ďalej)

**Fáza 1 – MVP (spustiteľné)**
- Otvoriť priečinok, strom súborov, taby, CodeMirror so zvýrazňovaním.
- ▶ Run aktuálneho `.py` s detekciou `.venv`, výstup v xterm.js, Stop, Re-run, klikateľný traceback.

**Fáza 2 – Web**
- Live Server s injektovaným reloadom + CSS hot-swap, preview v split view, „Otvoriť v prehliadači“.

**Fáza 3 – Suggestions**
- LSP: basedpyright + ruff, HTML/CSS/JS servery, Emmet. Diagnostiky (podčiarknutie chýb), hover, go-to-definition.

**Fáza 4 – Zen polish**
- Workspaces s farbami, compact mode, command palette, témy, glance náhľad.

**Fáza 5 – Extra**
- Run profily (argumenty, env), runner mapa pre ďalšie jazyky, „Install missing package“, QR pre mobil, konzola z preview, AI návrhy / „Vysvetli chybu“, debugger (`debugpy` cez DAP).

---

## 7. Riziká a pasce
- **`input()` nefunguje bez PTY** – od začiatku použiť PTY.
- **Windows**: `python` môže byť Store alias, ktorý nič nespustí → preferovať `py -3`; zabíjanie procesov cez `taskkill /T`.
- **Webview rozdiely** (Tauri na macOS/Linux = WebKit) – testovať CSS na všetkých OS.
- **LSP inštalácia**: servery treba pribaliť alebo stiahnuť pri prvom spustení (pyright potrebuje Node, ruff je jeden binárny súbor).
- **Veľké priečinky** (`node_modules`, `.venv`) – ignorovať vo watcheri aj v strome súborov.
- Rozsah: IDE je obrovský projekt – držať MVP malé a dizajn silný.

---

## 8. Otázky pre teba (rozhodnú ďalší krok)
1. **Tauri (malé, Rust)** alebo **Electron (ľahšie, väčšie)** – alebo radšej čisto **Python/Qt**?
2. Ktoré OS: Windows, macOS, Linux?
3. Chceš **AI suggestions** (API / lokálne Ollama), alebo stačí klasický LSP autocomplete?
4. Primárne pre seba, alebo aj pre začiatočníkov (Thonny štýl – jednoduchosť na prvom mieste)?
5. Názov projektu? (repo sa volá `cad`)
