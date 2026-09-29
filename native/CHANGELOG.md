# Flux Native – what's new

Flux Native is the new Flux written in Rust. It uses much less memory than Flux (one process instead of three) and keeps the same look, settings and projects.

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
