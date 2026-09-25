# Flux – what's new

Every version with its changes. Flux shows these notes in **Settings → General → About & updates** and on the [website](https://pantr1x.github.io/Flux/#releases).

## 1.4.13 – 2026-09-25

### Python
- **Autocomplete and error checking now also work for a single Python file** you open without a project – for example with a double-click on a `.py` file. Before, they only worked inside an open folder.

## 1.4.12 – 2026-09-24

### Plugins
- **Ratings and comments:**
  - Give a plugin **1 to 5 stars** and write what you think. Open a plugin in **Settings → Plugins** and press **Rate**.
  - You can change or delete your rating later.
  - Everyone can read the comments. To write, sign in with GitHub.
- The average rating is shown on every plugin card and at the top of the plugin page, for example ★ 4.5 (12).
- **Screenshots open full screen:**
  - Click a screenshot to see it big. Use ← → or the mouse wheel to go through them.
  - Click again to zoom in, drag to move around, and press Esc to close.

## 1.4.11 – 2026-09-24

### Fixes and small improvements
- **Restart and update** no longer gets stuck when you have unsaved files and press **Cancel**. The progress window closes, and you can start the update again later.
- The home screen now has the **☰ menu** next to the flux logo too, like the editor.
- Plugin descriptions in **Settings → Plugins** now show two lines instead of being cut off after one.

## 1.4.10 – 2026-09-24

### Rename everywhere
- The **Rename everywhere?** offer has a new, cleaner look: a small card that shows the old and the new name, with two clear buttons. **This file** shows how many places change, and **Whole project** shows how many other files change.

## 1.4.9 – 2026-09-24

### Rename everywhere
- When you change a **folder in the middle of a path**, for example `"C:/Users/admin/Desktop/python/simon.py"` to `".../Documents/python/simon.py"`, Flux now offers to change **every path in that folder**, not only the exact same text. It works in this file and in the other files of the project. A folder with a similar name, such as `Desktop2`, stays as it is.
- The offer shows long paths shortened, so it always fits in the editor.

## 1.4.8 – 2026-09-24

### Rename everywhere
- When you change a name (a variable, a function…), Flux still offers to rename it everywhere in the file. There is now also a button to rename it **in the other files of the project**.
- This also works for **text in quotes**, like a folder path: change `"data/images"` to `"data/pics"` and Flux offers to change the same path everywhere, including longer paths like `"data/images/cat.png"`.
- When you **rename a file or folder** in the sidebar, Flux finds the paths in your code that point to it and asks whether to update them. It works for Python, HTML, CSS, JavaScript and other languages.
- Names inside comments and plain text are left alone.

## 1.4.7 – 2026-09-24

### Faster updates
- **Updates are now much faster.** When only Flux itself changed, Flux downloads just the changed part (about 5 MB instead of the whole 117 MB installer), swaps it and opens again in a few seconds. No installer window appears.
- When something bigger changes (for example the Electron version), Flux still uses the normal installer, like before.
- The normal installer is faster too. Python autocomplete used to be about 5,400 separate files, which Windows had to delete, copy and check on every update. It is now one file that Flux unpacks the first time you use Python.
- This update itself still uses the installer. The fast updates start with the next version.

## 1.4.6 – 2026-09-24

### Settings
- Clicking a suggestion in **Search settings** now jumps to that setting. Before, it closed the settings instead.

## 1.4.5 – 2026-09-24

### Search
- You can turn the magnifier at the top into a **wide search field**. Turn it on in **Settings → Appearance → Window → Wide search field** or in **View → Wide search field**. Click it or press Ctrl+Shift+A to search files, commands, settings and projects.

## 1.4.4 – 2026-09-24

### Updating
- Flux compares versions correctly with test builds (for example 1.5.0-beta.1), so updates never go to an older version by mistake.
- Behind the scenes: the Flux developer can now try test builds before everyone else. They are not public releases. You keep getting only normal versions, and nothing changes for you.

### Website
- The website scrolls much more smoothly.
- It now has a questions-and-answers section and a nicer preview when you share the link.
- The website says that macOS and Linux versions are coming.

## 1.4.3 – 2026-09-24

### Updating
- Updating Flux now shows a **small “Updating Flux” window** with just a progress bar – no “Flux Setup”, no “Installing” and no buttons. Flux opens again by itself when it is done.
- The first installation of Flux looks the same as before.

## 1.4.2 – 2026-09-24

### Fixes
- **Project list:** the highlight sat a little above the selected project (the file counts below the projects load a moment later). It now always lines up – also for file tabs, the file list and the Settings menu.
- **Smooth scrolling in the editor** no longer jumps back a tiny bit at the start.
- **Intro:** the Flux logo was an empty square when animations are off. Now it shows right away.

### Website
- **Scrolling fixed:** the mouse wheel sometimes did nothing or stuttered. Now the page glides and stops right away when you change direction; touchpads scroll as usual.

### Checked on Windows
- Every build now installs Flux on a real Windows machine, runs an update like the app does and takes screenshots: the installer shows only **“Installing – Please wait…” with a progress bar**, then Flux opens again by itself.

## 1.4.1 – 2026-09-24

### Updating shows its progress
- After **Restart and update** the Flux installer opens with **only its progress bar** – no pages to click through – and Flux opens again by itself when it is done. (The small extra window from 1.4.0 did not show reliably.)
- Restart and update no longer waits long on a slow connection before it starts.

### Smoother switching
- Switching **files**: the highlight **slides** to the new tab and the code **slides in** from that side.
- The highlight also glides in the **file list**, the **project list** and the **Settings menu**.
- Turn it off with **Transition animations** in *Settings → Appearance → Window*.

## 1.4.0 – 2026-09-24

### Update progress
- While a new version **installs in the background**, the small update window shows a **real progress bar with percent** – how much of the new version is already installed.
- While a new version **downloads**, the status bar at the bottom shows **Updating 42 %** with a small bar. When it is ready it says **Restart to update** – click it to install.

### Smooth scrolling everywhere
- Not just the editor: **Settings, lists, the sidebar, the project page, AI and every other panel** now glide a little after you stop the mouse wheel.
- Turn it off with **Smooth scrolling with inertia** in *Settings → Appearance → Window* (it moved there from *Editor*).

### Transition animations
- A soft fade when you **switch files**, **Settings pages**, **Output / Terminal** and screens like the project page.
- Turn them off with **Transition animations** in *Settings → Appearance → Window*. *Animations off* in *Memory & speed* turns them off too.

## 1.3.2 – 2026-09-24

### Updating with progress
- **Restart and update** now shows a window with a **progress bar**: first it finishes downloading (in %), then it installs.
- While Flux is closed and the update installs, a small window with a progress bar stays on the screen until Flux opens again.
- In *Settings → General → About & updates* you see a progress bar while a new version downloads.

### More languages
- **Zig**, **R** and **Julia** – install them with one click in *Settings → Languages*, run them with **F5** and start a new project with them (*More languages*).

### GitHub
- *Save this project on GitHub* is just one **Publish** button – the project is private, and you can change that on GitHub any time.
- **Like Flux?** Give it a star: *Settings → General* or *Help → Star Flux on GitHub*.

### Other
- The command *AI: open assistant* is gone – use the **AI** button or **Ctrl+I**.
- **Home screen:** the settings button is no longer hidden under the window buttons (minimize, maximize, close).

### Website
- Brighter colors instead of large black areas, clearer text and a new font.
- **Smooth scrolling** that glides a little after you stop the wheel.
- A **Star** button with the number of stars, and Zig, R and Julia in the list of languages.

## 1.3.1 – 2026-09-24

### Menu
- The **flux logo** takes you to the start screen again. The menu is the **☰ button right next to it**.
- The menu no longer flickers or jumps to another submenu when you move the mouse into it at an angle.

### Search in Settings
- Finds settings by their name, description, section and tab – and by related words: *terminal* also finds *Output* and *Panel position*, *memory* finds *Save memory*.
- Small typos are fine: *fnot size* finds *Font size*, *memroy* finds *Memory*.
- **Suggestions** appear under the search box while you type. Pick one with the arrow keys and Enter (or a click) and Flux jumps right to it.

### Languages
- **Every app language now comes with Flux.** Switching is instant and works without the internet – newer translations still download in the background.

### GitHub
- *Save this project on GitHub* is clearer: choose **Who can see it?** – *Only you* or *Everyone* – and press **Publish on GitHub** (with a rocket).

## 1.3.0 – 2026-09-24

### Menu
- **Click the flux logo** (top left) for the menu: *Home*, **File**, **Edit**, **View**, **Run** and **Help** – every menu opens next to it when you point at it.
- When the sidebar is hidden, the menu is the small **☰** button at the top.
- Prefer a classic menu row? Turn on **Menu bar** in *View* or in *Settings → Appearance → Window*.

### Search
- A **magnifier** at the top next to *AI* searches files, commands, settings and projects (like *Ctrl+Shift+A*). Hide it with **Search button** in *View* or in Settings.

### Move things around
- **Panel position:** put *Output* and the **Terminal** at the **bottom**, on the **right** or on the **left**. Drag its edge to make it wider.
- **Sidebar position:** projects and files on the **left** or on the **right**.
- Both are in the **View** menu and in *Settings → Appearance → Window*.

### Updates in the intro
- The intro now asks if Flux should **update itself automatically** or **ask you first**.

### Fixes
- **Plugins** has its own puzzle icon – it looked the same as **AI**.
- Memory savings (*Save memory*, *Advanced*) now apply right when Flux starts, not only after you change a setting.

## 1.2.3 – 2026-09-24

### Memory & speed
- **Settings → General → Memory & speed** replaces *Performance*. At the top you see **how much memory Flux uses** and can free it.
- **Save memory** (was *Power saving*) turns on every saving at once – good for slower PCs.
- New **Advanced** part: change each saving yourself –
  - **Python autocomplete** on or off (it uses the most memory),
  - **Transparency and blur**, **Animations** and **Extra editor effects** separately,
  - **Limit memory of the window** to 512 MB,
  - how much **memory Python autocomplete** may use (768 MB, 1 GB or 2 GB),
  - when to **stop Python autocomplete** if no Python file is open (after 1, 5 or 15 min, or never).
- *Reset advanced* makes every part follow *Save memory* again.

### Plugins
- Plugins are **no longer installed by themselves**. Error Lens, Bookmarks, Theme Pack and the others are in the store – press **Install** and Flux downloads them from GitHub.
- **GitHub** and **Flux Together** open their own page with a description and everything they do.
- A normal **back arrow** on plugin pages.

### Website
- Shows **how many people downloaded Flux** – in total and for every version.

## 1.2.2 – 2026-09-24

### Flux Together 1.1 – green dots
- A **green dot** now shows where your friends are working: next to the **project**, next to **every folder** on the way to their file (even when the folder is folded) and next to the **file** itself.
- The dot **pulses while someone is typing**. Hover it to see who it is and which file they have open.
- You also see the dot next to your other projects in the sidebar when a friend works in one of them.

### Fixes
- **Sidebar:** no more empty strips with double lines under *N hidden* when the *Files* or *Together* list is empty.
- **Home screen:** *Ctrl+Shift+N*, *Ctrl+O*, *Ctrl+Shift+A* and *Ctrl+,* work there now – before, only Esc did.
- **Home screen:** the gear button (and Ctrl+,) opened Settings *behind* the home screen, so nothing seemed to happen. Esc now closes Settings first.
- **Home screen:** the action cards always fill the whole row, also when *From GitHub* is off.
- **Project page:** numbers like *12 min ago* are shown in full instead of *12 …*, and the page fits when the AI panel is open.
- **Live Server + AI panel:** the editor keeps a usable width instead of shrinking to a few letters.
- **Settings → General:** *All versions* sits in the same box as the other update settings.
- **Search everything (Ctrl+Shift+A)** finds section names like *Accent color* and the update settings (*Update automatically*, *All versions*).

## 1.2.1 – 2026-09-24

### Settings → General
- **General** now shows its parts in the left menu, just like **Appearance** – click one to jump to it, and the part you are looking at is highlighted.
- **About & updates** moved into General as its first part: your version, *Check for updates*, *Update automatically* and the release notes (folded, they load when you open them).
- **Shortcuts** moved into General too – the whole list, search, changing keys and *Edit shortcuts.json* are at the end of the page.
- **All versions** opens the Releases page of the website, where you can download any version of Flux.
- Links that used to open the old *Shortcuts* or *About & updates* tab now jump to the right part of General.

### New website
- A new website that looks like Flux itself: a live editor you can click through and run, all features, shortcuts, and a **Releases** page with every version, its notes and its installer – loaded straight from GitHub Releases.

## 1.2.0 – 2026-09-24

### Flux Together (Wi-Fi)
- A new built-in plugin: work together with friends on the **same Wi-Fi**. Turn it on in **Settings → Plugins → Built into Flux**.
- Everyone enters the same **room code**, and then you see each other in the sidebar under *Together*:
  - who has which file open and on which line,
  - **what they are typing, live**,
  - what they recently opened, edited and saved.
- In the editor you see your friend's line and cursor with their name. A dot in the file list shows which files others have open.
- Click a friend to jump to their file and line.
- It connects directly inside your network – nothing goes to the internet or GitHub, and there are no limits. If Windows asks, allow Flux on private networks.

### More colors
- New section **Settings → Appearance → App colors**. Pick your own colors for text, secondary text, background, panels and borders.
- Choose how see-through the panels are.
- Every color has its own *Reset*, and *Reset all colors* goes back to your theme.

### Sidebar
- **Settings** is now at the bottom left, with its name next to the icon.
- **Hidden projects:** click *N hidden* to unfold them. The eye icon puts a project back into the sidebar.

### Settings → General
- A new first page with your version and *Check for updates*.
- It also holds your name, the language, the intro and tour, and **Performance**:
  - power saving,
  - how much memory Flux uses right now,
  - a *Free memory* button.

### Fixes
- **Mouse pointer:** the crosshair, dot and ring pointers work again, also with *Use it in the whole app*. They are now drawn as images that Windows accepts.
- **GitHub** has its real logo everywhere in Flux.

### Website
- Flux has its own page with a download button for the newest version: https://pantr1x.github.io/Flux/

## 1.1.0 – 2026-09-24

### Useful plugins are built in
- Error Lens, Auto Rename Tag, Bookmarks, Color Highlight, Word Count, Snippet Pack and Theme Pack now come with Flux and are on from the start.
- Turn each of them off in **Settings → Plugins → Built into Flux**.

### Less memory
- Python autocomplete (Pyright) only starts when you open a Python file, and stops when no Python file has been open for a few minutes. That saves several hundred MB, for example in web projects.
- Pyright has a memory limit, so big projects can't eat all your RAM.
- The wallpaper image is freed when you don't use it, and Flux no longer keeps a spare background process around.

### Power saving for slower PCs
- New switch in **Settings → Appearance → Window**. It turns off transparency, blur, the wallpaper, animations, glow and other effects.
- The editor draws less: no code map, no sticky headers and no smooth cursor.
- Flux uses even less memory: Pyright gets a smaller limit and stops sooner.
- On PCs with 4 GB of RAM or less, it is on from the start.

### Looks
- **A new color picker** instead of the Windows one: a color field, a hue slider, preset colors and a hex box.
- The sections of **Appearance** are now in the left menu of the settings, and the one you are looking at is highlighted.

### Updates
- **No more updating twice.** If a newer version came out while one was waiting to install, Flux gets the newest one first.
- *What's new* shows the notes of every version you skipped.

## 1.0.2 – 2026-09-24

### Flux as a Claude extension
- New button **Claude extension (.mcpb)** in Settings → AI → *Use Flux from other AI apps*. It saves *Flux for Claude.mcpb* to your Downloads and opens it. Claude Desktop then offers to install Flux as an extension – just press **Install**.
- This works even when *Add to Claude Desktop* can't find Claude's settings.
- claude.ai in the browser can't connect to apps on your computer, so use Claude Desktop, Claude Code or Cursor.

## 1.0.1 – 2026-09-24

### Claude Desktop
- **Add to Claude Desktop** works with Claude Desktop from the Microsoft Store too (it keeps its settings in a different folder).
- The button turns on the Flux server for AI apps by itself.
- If Flux is closed when Claude needs it, Flux now starts by itself.

## 1.0.0 – 2026-09-24

Flux 1.0 – the first complete version.

### Live Server 2.0
- **Alt + click** any element in the preview and Flux jumps to the line where it is written in your HTML. While you hold Alt, the element under the mouse is outlined and its tag and line are shown.
- **JavaScript errors show up on the page** in a small card with a *Show in Flux* button that opens the file on the right line.
- **Devices:** preview as laptop, tablet or phone in a real device frame, and **rotate** it. Big devices are scaled to fit.
- **Screenshot** of the preview with one click. It is saved to `screenshots/` in your project and copied.
- **Open on your phone:** allow it on your Wi-Fi and scan the QR code. The page reloads on your phone too when you save.

### Make it yours
- **Theme Pack plugin:** 8 more color themes – Gruvbox, Solarized Dark & Light, Rosé Pine & Dawn, Ayu Mirage, Night Owl and Everforest.
- **Glass UI plugin:** see-through glass panels and a soft glow in your accent color.
- Plugins can now add color themes (`flux.themes.add`) and window variables (`flux.ui.setVar`).
- **Density:** *compact* fits more files, tabs and lines on the screen.
- **Focus mode (F11):** only the code, full screen.
- **Sticky headers:** the function or class you are in stays at the top while you scroll.

### More languages
- Run **TypeScript** (with Node.js, nothing else to install), **Perl** and **shell scripts** (on Windows with Git Bash).
- TypeScript and Perl templates and snippets (Snippet Pack 1.1).
- Errors with short file names (Java, Go, Rust, gcc…) are now clickable in the output too.
- Flux speaks two more languages: **Portuguese** and **Ukrainian** – 9 in total.

### Smaller things
- The date on the home screen starts with a capital letter only.
- The *Terminal* tab is translated.

## 0.3.8 – 2026-09-24

### First start: extras
- A new step, *Set up extras*, comes before choosing the look. It has three parts:
  - **Plugins** – pick useful ones like Error Lens or Bookmarks. They install in the background.
  - **GitHub** – install it (with Git) and sign in right away.
  - **Shortcuts** – change the most important ones by clicking them and pressing new keys.
- Both *Set up extras* and *Make it yours* have a *Skip* button.

## 0.3.7 – 2026-09-24

### Updates
- After *Restart now*, Flux installs the update and opens again by itself. This is more reliable from the next update on.

### First start
- A new step asks what Flux should call you, for the greeting on the home screen. You can skip it.

### Looks
- *Brightness of dark areas* goes higher now. It only lightens backgrounds – text and outlines stay the same.

## 0.3.6 – 2026-09-24

### GitHub is now a plugin
- GitHub is built into Flux as a plugin you install in **Settings → Plugins**. Until then, there are no GitHub buttons, settings or status in the app.
- Installing it also installs Git if it is missing, because GitHub needs it.
- If you were already signed in to GitHub, it stays on.

### Projects
- Next to the pin in the sidebar there is now a button to hide a project. It stays on the home screen, and *Undo* brings it right back.

## 0.3.5 – 2026-09-24

### Updates on start
- Flux looks for a new version right after it starts.
- With *Update automatically* on, it downloads the new version in the background and tells you when it is ready – with a *Restart now* button.
- With it off, you get a message that a new version is available, with a *Download* button.

### Terminal
- The panel at the bottom has two tabs: *Output* for your programs and *Terminal* – a real command line (PowerShell on Windows) in your project folder.
- Ctrl+Shift+` opens the terminal. Running a program switches back to *Output*.

### Files outside a project
- Files you open or create outside a project are listed under *Files* in the sidebar, so you can get back to them quickly.

### Projects
- Right-click a project (in the sidebar or on the home screen) for more options:
  - *Hide from the sidebar* – the project stays on the home screen. Hidden projects are one click away at the bottom of the list.
  - *Remove from Flux* – the files stay on your disk.
  - *Delete project* – the folder goes to the Recycle Bin, so you can still restore it.

## 0.3.4 – 2026-09-24

### Back and forward
- Arrows next to the tabs, on the home screen and in Settings take you back to where you were – a file, the project page, home or a settings page.
- The back and forward buttons on your mouse work too, and so do Alt+← and Alt+→.

### Open with Flux
- After installing, Flux shows up in Windows under *Open with* for text and code files (.txt, .md, .py, .html, .js and many more). Your default programs stay the same.

### New file without a project
- *Empty file* on the home screen asks what kind of file you want (.txt, .md, .py, .js, .html…) and where to save it. It is no longer added to the open project.

### Looks
- New sliders with a colored track.
- *Brightness of dark areas* in Settings → Appearance and in the first-start setup – turn it up if your wallpaper is very dark.
- The date on the home screen is in the app language.

## 0.3.3 – 2026-09-24

### Project page
- A new layout: big tiles with coding time, runs, lines of code, files and the last change at the top.
- Files, to-do list and Git side by side; the languages of the project are shown under its name.

### Home screen
- Your statistics across all projects – coding time, runs, lines of code and projects.

### Looks
- New drop-down menus in the Flux style instead of the white Windows lists.
- Switches in the Windows 11 style.
- Switching between light and dark mode works every time (it broke when a remembered theme was deleted).

### First start
- *More languages* – Rust, Ruby, PHP, Lua and more.
- The *Look* step shows a live preview of the code with the theme and accent you pick.

### Updates
- *Restart and update* installs quietly in the background, without the installer windows. Only the changed parts of Flux are downloaded.
- The *What's new* window closes with Esc or Enter.

## 0.3.2 – 2026-09-24

### New home screen
- Two columns: your projects on the left with a search box, *Start something new*, recent files and a daily tip on the right.
- Each project shows its languages, how many files it has and when it was last changed (e.g. *13 h ago*); paths are shorter.

### Settings
- A bigger window, quick jumps inside the long *Appearance* page and smaller theme cards.
- More space between headings and texts; the version is shown in the corner.

### Installer
- The *Start Flux now* box on the last page uses the normal Windows check box with readable text.

## 0.3.1 – 2026-09-24

### Edit any file, no project needed
- **Open file** on the home screen (or Ctrl+Alt+O) opens any file – notes in **.md**, **.txt** and anything else – without a project.
- Drop files onto the Flux window to open them, or a folder to open it as a project. *Open with → Flux* in Windows works too.
- Your **recent files** are on the home screen.

### Markdown preview
- Open a **.md** file and press **Preview** (Ctrl+Shift+V) – the rendered text is shown next to the editor and updates while you type.
- Headings, lists, check lists, tables, quotes, code, links and pictures.

### Fixes
- Release notes and *What's new* show headings properly, with icons instead of emoji.

## 0.3.0 – 2026-09-24

### Use Flux from Claude and other AI apps
Turn it on in **Settings → AI → Use Flux from other AI apps**. Claude Desktop (one click), Claude Code, Cursor and VS Code can then see your Flux projects, read and write files, and update the description and to-do list – changes show up in Flux right away. It works only on your computer and needs a secret key.

### Also new
- **Sign in with GitHub** opens a GitHub window inside Flux and types the code in for you.
- Projects show their **main language plus how many others** they use (e.g. *JavaScript +3*), measured by the amount of code like on GitHub, and a small **GitHub mark** when they come from GitHub.

## 0.2.2 – 2026-09-24

- Installer: the “Start Flux now” text on the last page is readable on the dark background.
- Sign in with GitHub: clearer steps – where the code is and what to press.

## 0.2.1 – 2026-09-24

- **Sign in with GitHub** now works: click it, log in in your browser (or create an account, also with Google), press *Authorize* – and all your repositories are in Flux.
- Release notes are built into Flux, so *Settings → About & updates* always shows what is new.
- Lists and tables in release notes, plugin pages and the AI chat look right.

## 0.2.0 – 2026-09-24

The first public version of Flux – a small, modern code editor for Windows 11.

### Highlights
- **One-click Run** for Python, JavaScript, Java, C/C++, C#, Go, Rust, Ruby, PHP and Lua – missing languages are downloaded for you from their official sources.
- **Live Server** for websites with a preview that reloads on save.
- **Autocomplete and error checking** for Python, HTML, CSS and JavaScript.
- **Claude AI assistant** (Ctrl+I) with your own API key and MCP connectors – it can also read your project and plan your to-do list.
- **GitHub**: open any of your repositories as a project, commit & push, publish new projects.

### Make it yours
- Code themes plus the **Theme studio** – build your own theme with color pickers and see it live.
- Custom text cursor, mouse cursor (also your own image), fonts, app size, corners and background.
- **Every shortcut can be changed**, and you can write your own in `shortcuts.json`.
- 7 app languages with flags: English, Slovak, German, Spanish, French, Italian and Polish.

### Plugins
Error Lens, Auto Rename Tag, Python Docstring, Indent Rainbow, Better Comments, CSS Class Completion, Bookmarks, Color Highlight, Snippet Pack and Word Count – install them in **Settings → Plugins**, or make your own.

### Updates
Flux now updates itself. Turn it off any time in **Settings → About & updates**, where you also find these notes for every version.

### Also new
- **Sign in with GitHub** in your browser – you can log in (or create an account, also with Google) right there.
- **Theme studio**, **all shortcuts editable**, custom mouse cursor and a *Reset* for every look option.
- Plugins store with ratings and screenshots; ten plugins from the Flux team.
- Dark installer with the Flux artwork.

## 0.1.0 – 2026-09-23

The first build of Flux.

- One-click Run for Python and JavaScript, Live Server for websites.
- Python autocomplete and error checking.
- Translucent, Zen-like look with code themes, projects in the sidebar and a code map.
- First-start intro and feature tour.
