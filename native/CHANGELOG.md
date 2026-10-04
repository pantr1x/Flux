# Flux Native – what's new

Flux Native is the new Flux written in Rust. It uses much less memory than Flux (one process instead of three) and keeps the same look, settings and projects.

## 0.9.16 – 2026-10-04

### Programs end by themselves
- When your program finishes, Flux now shows *Finished* (or the exit code) right away and the **Stop** button turns back into ▶ Run. On Windows it used to keep saying *Running* after the last line.
- The same goes for the Terminal tab when you type `exit`.

### Nicer release notes
- *What's new* and Settings → About & updates now show code like `input()` in a code font, *italic* text, more space between lines and a clear version badge.

### A bit lighter
- The home screen's soft background glow moves only for a few seconds after it opens and only while Flux is in front – then Flux draws nothing until you do something.
- The Output and Terminal panels are drawn with far fewer pieces, so typing in the editor with a full Output panel costs less.

## 0.9.15 – 2026-10-04

### Programs work again on Windows
- ▶ Run showed only the file name and nothing else: Windows was waiting for an answer from Flux before letting the program print anything. Flux now answers, so you see the program's output, and `input()` (or `cin`, `Scanner`…) gets what you type. The Terminal tab is fixed the same way.
- The red *Nothing to run* message no longer stays in the status bar after you switch to another file.

## 0.9.14 – 2026-10-04

### Mistakes are shown where they are
- Forgot a colon, a bracket, a quote or a `;`? Flux now underlines the spot and **explains the mistake right next to the line** – for example *Missing colon (:) at the end of this line*.
- Python uses the real Python check, JSON is checked exactly, and C, C++, Java, C#, JavaScript, Rust, Go, CSS and HTML get checks for brackets, quotes, comments and missing `;`.
- The status bar shows *N problems*; click it to jump to the first one. Turn it off in Settings → Editor → *Error hints*.

### Suggestions while typing
- Start typing and Flux suggests keywords, built-in functions and names from your file. ↑/↓ to choose, Enter or Tab to insert, Esc to close, Ctrl+Space to show them yourself.
- Turn it off in Settings → Editor → *Suggestions while typing*.
- Typing a closing bracket or quote that Flux already added now just steps over it instead of adding a second one.

### Programs that ask for input
- After ▶ Run, your keyboard goes straight to the program, so `input()` (and `cin`, `Scanner`…) gets what you type without clicking the output first.
- If the Terminal can't start, it now says so instead of staying empty.

### Run only when there is something to run
- Files without a `main` (for example Windhawk mods `*.wh.cpp` or header files) no longer offer ▶ Run – the button is greyed out and tells you why.
- The Python version in the status bar shows only for Python files.

### A calmer home screen
- The home screen is now simple: logo, greeting, one big search box and your recent projects. Type to find a project or a recent file, ↑/↓ and Enter to open it.
- The ☰ menu is much narrower.

### What's new
- After an update, Flux shows what changed once at start (this card).

## 0.9.13 – 2026-10-04

### Big files stay smooth
- The editor now colors and draws **only the code you see** (plus a few hundred lines below it). The rest is colored when you scroll there, and typing recolors only from the changed line on.
- Measured on a file with 30,000 lines: opening took **0.3 s instead of 5 s**, and one frame while scrolling takes **3 ms instead of 47 ms** – smooth scrolling stays smooth even in huge files.
- The minimap and the word count in the status bar no longer go through the whole file on every frame either.

### Fixes
- The window buttons (− □ ×) no longer look doubled when Flux is see-through.
- The question *Can you see what is behind Flux?* is gone.
- Home screen: a long coding time (for example *4 h 26 min*) gets smaller instead of running under the clock icon.
- **Back** (mouse button or Alt+←) inside Settings goes to the previous settings page instead of closing Settings.

### GitHub only when you want it
- Settings → Plugins now shows only real plugins – GitHub for now. Claude AI, Live Server and Languages are simply part of Flux.
- The intro asks whether you want GitHub. Without it, Flux doesn't show GitHub anywhere – install it any time in Settings → Plugins.

## 0.9.12 – 2026-10-04

### See-through window
- Flux is now **see-through**: whatever is behind it – your desktop, Lively or Wallpaper Engine, Discord or any other window – shows through its panels. Flux no longer draws its own copy of the wallpaper, so the blocky look is gone and it uses less memory and power.
- Settings → Appearance → Window → **Window background**: *See-through – clear*, *See-through – blurred* (Windows blurs what is behind Flux) or the old *Wallpaper picture*.
- **Background dimming** sets how dark the background behind the panels is, so text stays easy to read.
- The first time, Flux asks *Can you see what is behind Flux?*. If you don't answer within 25 seconds (for example because the window stays invisible), it goes back to the wallpaper picture by itself.

### Scrolling that glides
- **Mouse wheel:** each turn gives the page a push, it moves smoothly while you turn and glides out and slows down after you stop – like on an iPad. (The glide announced in 0.9.9 never actually made it into the program – sorry. It is in now.)
- **Touchpad:** the page follows your fingers exactly and keeps gliding after you lift them, slowing down smoothly. The extra jump of one line at the end of a swipe is gone.

## 0.9.11 – 2026-10-04

### Taskbar pin repaired
- If the last update broke your taskbar pin (*"The item … has changed or moved"*), Flux now fixes it by itself a few seconds after it starts. Choose **No** in that message, start Flux once from its folder, and the pin works again.
- Every new build is now tested on Windows before it is published: it updates itself, must stay the same file, keep the pin and open again with a window.
- If Flux ever fails right after an update, it goes back to the previous version only once, never in a loop.

## 0.9.10 – 2026-10-04

### Updates keep your taskbar pin
- *Restart to update* now replaces Flux **in the same file**. Flux pinned to the taskbar stays pinned, and the new version opens on the same icon instead of a separate one.
- The swap takes a moment: Flux closes, the new version writes itself over the old one and starts again.
- Updating from 0.9.9 still uses the old way one last time, so if the pin breaks now, pin Flux again – it stays from now on.

## 0.9.9 – 2026-10-03

### Scrolling like on a tablet
- **Mouse wheel:** every notch gives the page a push and it glides and slows down smoothly, like on an iPad. Turning the wheel faster throws the page further.
- Smooth scrolling now works even when *Transition animations* are off – it has its own switch (*Smooth scrolling with inertia*).
- **Touchpad:** the page no longer stops and then jumps one more line at the end of a swipe.

### Faster updates
- An update now downloads **only what changed** since your version (often a few hundred kB instead of 13 MB). If that is not possible, the whole program is downloaded as before.

## 0.9.8 – 2026-10-03

### Moving live wallpaper with much less memory
- Flux now makes a **small copy** of your live wallpaper video once (at most 640 px wide, saved in Flux's data folder) and plays that copy. Before, it decoded the original video – often 4K – and on laptops with built-in graphics that alone could take a few hundred MB.
- While the copy is being made (usually a few seconds, only the first time), Flux shows a still frame of the wallpaper.

### Also
- A very big file keeps fewer undo steps (20 instead of 100), because every step is a full copy of the text.
- Developer: the memory row also shows how much Flux's own code uses.

## 0.9.7 – 2026-10-03

### Less memory with a moving live wallpaper
- When Flux has been in the background for 3 seconds, the video decoder of your live wallpaper is **freed completely** (the last frame stays). When you come back, the video continues from the same spot. Flux in the background now uses much less memory.
- The text atlas (all letters Flux has drawn) is smaller.

### Scrolling without the slow tail
- After a turn of the mouse wheel the page glides to its place in a fixed short time and stops exactly there. Before, after scrolling a lot it slowed down almost to a stop and then still moved a little.

## 0.9.6 – 2026-10-03

### Updates never get stuck again
- **Fixed:** once an update was downloaded (*Restart to update*), Flux stopped looking for newer ones. If that download was lost, Flux never found any update again. Now it keeps checking every hour: a lost download is fetched again, and a newer build replaces the one waiting.
- *About & updates* has a check button next to *Restart to update*.

## 0.9.5 – 2026-10-03

### Smoother scrolling, also on a touchpad
- **Touchpad:** the page now follows your fingers exactly. Before, Flux added its own glide on top of the one Windows already does, so the page kept moving after you stopped.
- **Mouse wheel:** each notch is smoothed only once (before, egui smoothed it and then Flux again, which felt mushy). The glide is the same spring as in Flux.

## 0.9.4 – 2026-10-03

### Restart to update
- **Fixed:** *Restart to update* showed "The system cannot find the file specified" when the downloaded update was gone (removed by an antivirus or by the bug fixed in 0.9.2). Flux now downloads it again by itself and offers the restart once it is ready.

### Less memory
- Code coloring (the definitions of all languages, about 11 MB) is loaded only when code is on screen and freed after 2 minutes without code.
- Icons are no longer kept twice (in RAM and on the graphics card).

## 0.9.3 – 2026-10-03

### New file, like New project
- **+ New file** is now always in the sidebar under FILES, the same way as *+ New project* under PROJECTS. It opens the New file window (language, name, location).

### Less memory and work
- Files you haven't looked at for **10 minutes** are freed from memory (the tab stays). Clicking the tab loads the file again – unsaved files are never freed.
- When a file changes on disk, Flux reloads only the one you are looking at; other tabs reload when you open them.
- The editor draws line numbers and indent guides **only for the lines you see**, and no longer copies the text on every frame.
- Flux remembers each project's stats (files, lines, languages) and at start reads only the open project, not every file of every project.

### Smooth scrolling
- One turn of the mouse wheel scrolls a bit further and still glides smoothly. *Scroll distance* in Appearance → Window changes it further.

## 0.9.2 – 2026-10-03

### Updates install by themselves again
- **Fixed:** when an update finished downloading in the first 10 seconds after start (a fast connection), Flux deleted it again, so updates never installed. Now they do.
- With *Install updates automatically* on, a downloaded update installs **by itself** once Flux has been in the background for a minute and nothing is running or unsaved. Flux comes back without stealing focus.
- Flux checks for updates every hour instead of every 6 hours.
- An update no longer fails when an older Flux (or the MCP bridge for Claude) is still running.

### See-through glass (try it)
- New choice in Settings → Appearance → Window → Live wallpaper: **See-through glass**. Flux doesn't play its own copy of the live wallpaper – the real desktop (Lively, Wallpaper Engine) shows through, almost without extra memory. Windows behind Flux show through too.
- It is a test: after the restart Flux asks whether you can see it. Without an answer in 25 seconds (for example when the window stays invisible) it goes back to the moving wallpaper by itself.

## 0.9.1 – 2026-10-03

### Smoother switching between files
- The highlight in the file tree **glides** to the file you click.
- Switching tabs or files slides the code in from the side where the tab is (left or right) while it fades in.

## 0.9.0 – 2026-10-03

### Projects and files
- **Renaming a project works**, also the one that is open. Flux closes the program and terminal that held the folder, renames it and opens it again with its description, to-dos and tabs. The name is also saved when you click away.
- **Deleting a project takes you to the home screen**, and the status bar no longer keeps saying "… is ready". Short messages like that now disappear after a few seconds.
- **New file** has its own window, like New project: pick the **language**, type a name and Flux adds the right extension and a small starter code. It works from the file tree, the project page, the menu, Ctrl+N and the right-click menu.
- When you create a project you can write **what it is about**; on the project page click the description to change it.

### Live Server
- **The page changes while you type**, without saving. CSS changes swap only the styles, so the page does not flash.
- **You can type in the editor again** while the page is open next to it (the page no longer keeps the keyboard). The same fixes search in Settings and the search box.

### Back and forward
- The arrows (and the mouse side buttons) now **slide the view** in from the side you go to.

### Settings
- **Every setting has a short explanation** under its name.
- **Search in Settings** also finds words in your language.
- **Scroll distance** (Appearance → Window): how far one turn of the mouse wheel scrolls, 50–300 %.
- **Release notes** show only the newest version; *Show older versions* opens the rest.

### GitHub is a plugin
- GitHub is now in **Settings → Plugins** with *Install* / *Remove*. Without it there is no GitHub tab and no GitHub choices in New project. If you were already signed in, it stays installed.

### Claude and MCP
- **Ready-made connectors** in Settings → AI: Context7, DeepWiki, GitHub and Hugging Face in one click.
- **Use Flux from other AI apps**: turn it on and Claude Desktop, Claude Code, Cursor and others can see and change your open project (files, description, to-dos). *Add to Claude Desktop* sets it up for you; for the others copy the command or the settings. Flux starts in the background by itself when Claude needs it.

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
