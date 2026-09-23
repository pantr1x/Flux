# Flux

Moderný mini-editor pre Windows 11: **▶ jedno tlačidlo na spustenie Pythonu**, **Live Server** pre weby
a **autocomplete**. Vzhľad je inšpirovaný Zen Browserom.

![ikona](build/icon.png)

## Čo vie

| | |
|---|---|
| **▶ Spustiť (F5)** | Spustí otvorený `.py` súbor – žiadne `cd` ani `python program.py`. Funguje `input()`, farby aj okná (tkinter, turtle, pygame). Pred spustením sa všetko uloží. |
| **■ Zastaviť (Shift+F5)** | Ukončí program aj všetko, čo spustil. |
| **Zelený ▶ v okraji** | Pri `if __name__ == "__main__":` – klik spustí súbor (ako v PyCharme). |
| **Chyby** | Riadky `File "…", line 12` vo výstupe sú klikateľné. Pri chybe sa ukáže tlačidlo „Chyba: main.py, riadok 12“. |
| **Chýbajúci balík** | Pri `ModuleNotFoundError` ponúkne „Nainštalovať requests“ (vie aj `cv2 → opencv-python`, `PIL → pillow`…). |
| **Python** | Sám nájde `.venv` / `venv` v projekte, potom `py` launcher alebo Python v PATH. Dá sa vybrať aj ručne (klik na „Python 3.x“ dole). |
| **Autocomplete – Python** | Pyright (to isté čo Pylance vo VS Code): návrhy, dokumentácia, parametre funkcií, automatické importy, podčiarknuté chyby, Ctrl+klik = prejsť na definíciu. |
| **Autocomplete – web** | HTML, CSS, JavaScript, JSON + **Emmet** (`ul>li*3` + Tab). |
| **Live Server (Alt+L)** | Náhľad webu priamo vedľa kódu. Po uložení sa stránka obnoví, CSS sa vymení bez reloadu. `console.log` zo stránky sa zobrazí vo výstupe. Náhľad ako mobil / tablet / PC. |
| **Auto-ukladanie** | Zapnuté – ukladá samo po chvíli nepísania (vypneš dole v stavovom riadku). |
| **Vzhľad** | Zen štýl: plávajúca karta, zvislý panel, farba pre každý priečinok, svetlá/tmavá téma, kompaktný režim (Ctrl+B), efekt Mica na Windows 11. |

## Skratky

| Klávesy | Akcia |
|---|---|
| `F5` alebo `Ctrl+Enter` | Spustiť súbor / otvoriť náhľad webu |
| `Shift+F5` | Zastaviť |
| `Alt+L` | Live Server zap./vyp. |
| `Ctrl+P` | Nájsť súbor |
| `Ctrl+Shift+P` | Všetky príkazy |
| `Ctrl+S` / `Ctrl+Shift+S` | Uložiť / uložiť všetko |
| `Ctrl+N` / `Ctrl+W` | Nový súbor / zavrieť súbor |
| `Ctrl+B` | Skryť bočný panel (vysunie sa pri nabehnutí myšou k ľavému okraju) |
| `Ctrl+J` | Skryť/zobraziť výstup |
| `Ctrl+Tab` | Ďalší otvorený súbor |
| `Ctrl+koliesko`, `Ctrl+=`, `Ctrl+-` | Veľkosť písma |

## Inštalácia

### Možnosť A – hotový inštalátor
1. Na GitHube otvor záložku **Actions** → **Windows build** → posledný zelený beh.
2. Dole v časti **Artifacts** stiahni **Flux-Windows** a rozbaľ ho.
3. Spusti `Flux-Setup-0.1.0.exe`. Windows môže ukázať „Windows ochránil váš počítač“, pretože
   inštalátor nie je podpísaný → **Ďalšie informácie** → **Spustiť aj tak**.

### Možnosť B – spustenie zo zdrojového kódu
Potrebuješ [Node.js LTS](https://nodejs.org) a [Python](https://www.python.org/downloads/)
(pri inštalácii zaškrtni **Add python.exe to PATH**). Najrýchlejšie v PowerShelli:

```powershell
winget install OpenJS.NodeJS.LTS
winget install Python.Python.3.13
```

Potom v priečinku projektu:

```powershell
npm install
npm start
```

alebo len dvojklik na **`Flux.bat`**.

Vlastný inštalátor zostavíš príkazom `npm run dist` (výsledok je v priečinku `release/`).

## Ako je to postavené

```
src/
  main/            hlavný proces (Node.js)
    main.js        okno, súbory, nastavenia, prepojenie s UI
    runner.js      spúšťanie programov v pseudoterminále (ConPTY)
    python.js      hľadanie Python interpretera
    liveServer.js  HTTP server + automatický reload
    lsp.js         spustenie Pyrightu (autocomplete pre Python)
  preload.js       bezpečný most medzi UI a hlavným procesom
  renderer/        rozhranie
    app.js         editor (Monaco), taby, strom, terminál (xterm.js), náhľad, paleta
    pyLsp.js       prepojenie Pyrightu s editorom
    styles.css     vzhľad
scripts/build.mjs  zabalenie rozhrania cez esbuild
```

Technológie: Electron, Monaco Editor (editor z VS Code), xterm.js, node-pty, Pyright, Emmet.
Prieskum a plán ďalších krokov je v [RESEARCH.md](RESEARCH.md).
