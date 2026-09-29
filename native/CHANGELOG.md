# Flux Native – what's new

Flux Native is the new Flux written in Rust. It uses much less memory than Flux (one process instead of three) and keeps the same look, settings and projects.

## 0.5.0 – 2026-09-29

### Home screen like Flux
- **Click the logo** (or ☰ → Home) to open Flux's home screen. It shows a greeting with today's date, your coding time, runs, lines of code and projects.
- **New project, Open folder and Open file** are big buttons at the top.
- **Projects**: pinned projects as cards, recent ones as rows, and a search box. Recent files are listed below.
- **Start something new**: Python, Python window, Python game, HTML page, Web project, JavaScript or an empty file, each with one click. Plus a tip of the day.

### Live Server beside your code
- **Live Server** now opens the page on the right side of Flux, next to your code, and reloads it every time you save. You can switch between full width, tablet and phone, reload it, open it in your browser, or close it with ×. Drag its edge to make it wider.

### Tidier sidebar
- **Settings** at the bottom of the sidebar now sits exactly level with the line above the status bar.

## 0.4.0 – 2026-09-29

### Run, Stop and Live Server
- **One button that changes.** ▶ **Run** turns into **■ Stop** while your program runs. There is no separate stop square any more.
- **Live Server for websites.** For HTML and CSS files the button says **Live Server**. It opens the page in your browser, and the page reloads every time you save. The top bar shows the address, and the button turns into **Stop**. *Live preview* on the project page does the same.

### Languages
- **Download programming languages** in Settings → Languages, like in Flux: Python, Node.js, Java, C/C++, Go, C#, Git, Rust and more. You can see what is installed and which version, install a language with one click, and get updates.

### Settings
- **Release notes are folded.** Every version is one line with a short summary. Click it to read more, including older versions.
- **The menu follows your scrolling.** The highlighted item in the left menu moves as you scroll, down to *Shortcuts*.
- **Feature tour**: Settings → General → Welcome shows you the main parts of Flux, step by step.

### Look and speed
- The sidebar now lines up with the card at the bottom, and the thin light line between them is gone.
- **Video wallpapers** (Lively Wallpaper, Wallpaper Engine) are decoded on the graphics card. They no longer run slow and use much less memory.

## 0.3.0 – 2026-09-29

### Updates from Settings
- **Flux Native updates itself.** Settings → General → About & updates shows when a new version is ready. Press **Download update**, then **Restart to update**. With automatic updates on, it downloads by itself and the status bar shows **Restart to update**. Closing Flux also installs a downloaded update.
- This version still has to be downloaded by hand once. After that, updates come from Settings.

### Looks like Flux
- **Flux's own title bar.** The Windows title bar is gone. The top bar has the logo, ☰, the sidebar button, ← →, search, AI and − □ ×. Drag it to move the window and double-click it to maximize.
- **Wide search field**: turn on Settings → Appearance → Window → *Wide search field* to get a **Search Ctrl+Shift+A** box instead of the magnifier.
- **PINNED** projects sit in their own box above the other projects. Hover a project to hide it or pin it.
- **Project page buttons like in Flux**: *Open main file*, *Run it*, *New file* and 🔍. Web projects get *Open page* instead.
- **Color fields like in Flux**: a round color swatch with its hex code, no checkered squares. Click it to pick a color or type a hex code.

### Wallpaper
- **Less see-through.** The card behind the editor now has a blurred copy of the wallpaper under it and is more solid, so text is easier to read.
- **Lively Wallpaper and Wallpaper Engine**: when one of them is running, Flux shows the same wallpaper behind its panels. Videos play only while Flux is in front. Turn it off in Settings → Appearance → Window.

## 0.2.0 – 2026-09-29

### Looks and feels like Flux
- **Smooth scrolling everywhere**: the editor, settings, the sidebar, the project page and search glide like in Flux. Turn it off in Settings → Appearance → Window.
- **Your desktop wallpaper behind Flux**, like the see-through look of Flux. Panels are slightly transparent. Turn it off with **Save memory** or **Transparency and blur**.
- **A livelier intro**: a moving background, the logo draws itself, and every step slides in.
- **Hover and switch animations**: buttons, rows, tabs and cards fade in softly, and switching projects or files fades the content in.
- **Settings controls like in Flux**: drop-down lists, sliders with their value, and number fields with − and +.
- **Release notes** are here in Settings → General → About & updates.
- The Output and Terminal buttons fit their text, and the code map is sharper and fits its column.

## 0.1.0 – 2026-09-29

### The first Flux Native
- The same window as Flux: sidebar with projects and files, file tabs, ▶ Run, one rounded card with the editor, the code map, Output and Terminal, and the status bar.
- The same icons, colors, code themes and translations (9 languages).
- The same intro on the first start, and the project page with stats and to-do list.
- **Settings** (Ctrl+,): General, Appearance, Editor and Running, with search.
- **☰ menu**, **search** (Ctrl+P files, Ctrl+Shift+A everything, Ctrl+Shift+P commands) and **right-click menus** in the file tree and on projects.
- Editor: find and replace (Ctrl+F / Ctrl+H), comments (Ctrl+/), Tab / Shift+Tab indent, auto-closing brackets.
- Runs Python and all other languages of Flux, with input, and a real terminal.
- Frees memory while it is in the background.
