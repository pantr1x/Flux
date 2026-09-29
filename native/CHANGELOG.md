# Flux Native – what's new

Flux Native is the new Flux written in Rust. It uses much less memory than Flux (one process instead of three) and keeps the same look, settings and projects.

## 0.8.5 – 2026-09-29

### Live wallpaper moves again
- Lively Wallpaper and Wallpaper Engine **play behind Flux's panels again**, like before 0.7. Flux shows your wallpaper right from the start. If you want to save memory, pick *Still image* in Settings → Appearance → Window → Live wallpaper.

### New project window
- No more choosing a language: pick an **icon**, a **name**, an optional description and **where** it goes.
- **On this computer**: a new folder in Flux Projects.
- **New on GitHub**: Flux creates the folder, a new repository on your GitHub account (private or public) and uploads the first version.
- **Import from GitHub**: pick one of your repositories, or paste a link to any public one. Flux downloads it and opens it.

## 0.8.4 – 2026-09-29

### Fixed: Flux Native did not open with Lively Wallpaper or Wallpaper Engine
- Since 0.7.0 Flux made its window see-through when you use a live wallpaper. On Windows that window stayed **invisible**: Flux ran only in the background. The see-through window is off now; Flux shows a still frame of your live wallpaper (or plays it with *Play inside Flux*).
- Invisible Flux processes that are still running from before are closed when you start Flux, so you can rename or replace the program again.
- If Flux has no visible window 12 seconds after start, it restarts itself in safe mode.

## 0.8.3 – 2026-09-29

### Flux Native starts again
- **Safe mode.** If Flux did not start properly last time, it now tells you and starts without the see-through window and wallpaper (after two failed starts also with simpler graphics). An invisible Flux that got stuck is closed first. Once a start works, the next one is normal again.
- More room for Flux's main thread on Windows, and crashes outside Flux's own code are written to `flux-native-crash.log` too.
- Your GitHub token and API keys are now kept in the **Windows Credential Manager**. Please sign in to GitHub and paste your API key again once.
- New builds are tested with real-world settings before they are offered as an update.

## 0.8.2 – 2026-09-29

### Safer updates
- If a new version of Flux Native does not start, it **goes back to the previous version by itself** and tells you what happened.
- When Flux cannot start, you now see a message instead of nothing. The details are saved in `flux-native-crash.log` in the settings folder.

## 0.8.1 – 2026-09-29

### New project window
- **New project** opens a proper window, like in Flux: pick Python, Website, JavaScript, Empty or one of 14 more languages, then choose an **icon**: the language icon, a line icon (robot, rocket, globe, star…) or an emoji (🎮 🚀 🎨 🔥…). Add a name, a short description and where to save it.
- If the language is not on your computer yet, the window offers **Install** right there.
- **Change the icon later**: click the big icon on the project page.

### Fixes
- **Back and forward** with the mouse buttons now go exactly one step and switch to the right project. The ← → buttons at the top no longer maximize the window when you click them while they are grey.
- The intro shows **Installed** on languages that are already on your computer.
- The memory number ("Flux uses … MB right now") now matches Task Manager. It used to count memory the graphics driver only reserves, so it showed about 400 MB.

## 0.8.0 – 2026-09-29

### Settings like in Flux
- **Settings look like Flux again.** The window has the same size and shape, sits in the middle of the screen and no longer reaches the bottom edge. The menu, the sections and the rows look the same too.
- **New rows:** *Panel transparency* and your own **Background** picture (Appearance), and *Reset advanced* (General → Memory & speed).
- **Plugins** shows what is built into Flux Native: GitHub, Claude AI, Live Server and Languages.

### GitHub
- Settings → **GitHub**: press **Sign in**, confirm the code in your browser, and you are connected. You can also paste a token.
- **Open a repository**: pick one of your repositories and Flux downloads it and opens it as a project.
- Your GitHub key is stored encrypted on this computer.

### Claude AI
- Settings → **AI**: paste your Anthropic API key, choose the model and press **Test** to check it.
- **MCP connectors**: add remote MCP servers (name, address and an optional token), and Claude can use their tools while it answers.
- **Ctrl+I** or the **AI** button opens Claude on the right side. Claude sees the open file, and code in answers has **Copy** and **Insert** buttons.

## 0.7.0 – 2026-09-29

### Much less memory with Lively Wallpaper and Wallpaper Engine
- **See-through window.** Flux no longer plays its own copy of your video wallpaper. The window is see-through, so your live wallpaper shows behind the panels as it is, moving, and Flux does not use extra memory for it. The first time, Flux offers **Restart Flux** in Settings → Appearance to turn this on.
- Settings → Appearance → Window → **Live wallpaper**: *See-through* (lightest, windows behind Flux show too), *Still image* or *Play inside Flux* (uses more memory).
- **No more flash at start.** Flux remembers your live wallpaper, so the Windows wallpaper no longer shows for a moment first.

### Back and forward
- **← →** in the top bar, the back/forward buttons on your mouse and **Alt+← / Alt+→** now go back and forth between files, project pages and the home screen.

### For the developer
- Settings → Developer → Memory shows what uses memory: textures, wallpaper, preview and the program.

## 0.6.0 – 2026-09-29

- **Intro**:
  - *What do you want to code?* now fits on the screen, even with all languages. It also tells you that languages are not part of Flux: they download only when you need them.
  - The *Make it yours* step has **My computer is slower**. It turns on Save memory, so you see right away how Flux looks with it, and it shows how much memory Flux uses right now.
- **File tabs** each sit in their own box.
- **Developer** is now its own section in Settings, with parts for builds and updates, intro and tour, memory and folders. Only the Flux developer sees it.

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
