# Plugin API

Everything the `flux` object gives your plugin. For a step-by-step start see [[Making plugins]].

Everything a plugin adds through `flux` is removed automatically when the plugin is turned off, updated or uninstalled.

| API | What it does |
| --- | --- |
| `flux.id`, `flux.version` | Your plugin id and the API version (`"1"`). |
| `flux.toast(text, kind?, ms?)` | Small message in the corner. `kind`: `info`, `ok`, `warn`, `error`. |
| `flux.activeFile()` | `{ path, name, language, text, selection }` of the open file, or `null`. |
| `flux.insertText(text)` | Inserts text at the cursor. `$1`, `${1:name}` work like in snippets. |
| `flux.commands.register(id, label, run, { key })` | Adds a command to the command palette (Ctrl+Shift+P) and search (Ctrl+Shift+A), optionally with a shortcut like `Ctrl+Alt+H`. |
| `flux.commands.run(id)` | Runs your command or a built-in one (e.g. `run`, `save`, `settings`). |
| `flux.statusBar.add({ text, title, onClick })` | Adds a button to the status bar. Returns `{ set(text), setTitle(text), show(bool), remove(), element }`. |
| `flux.snippets.add(language, prefix, body, description?)` | Adds a snippet, e.g. `('html', 'card', '<div class="card">$0</div>')`. |
| `flux.ui.addStyle(css)` | Adds CSS to Flux. Returns `{ remove() }`. |
| `flux.ui.setVar(name, value)` | Sets a CSS variable of the window, e.g. `setVar('--radius', '20px')`. Undone when the plugin is turned off. |
| `flux.themes.add(key, { name, type, colors, basedOn?, italicComments? })` | Adds a code color theme to the theme list. `type` is `'dark'` or `'light'`; `colors` uses the keys `fg, comment, keyword, storage, string, number, type, function, variable, parameter, property, constant, tag, attr, delimiter, regexp` (hex, with or without `#`). Missing keys come from `basedOn` or the default theme. See the Theme Pack plugin. |
| `flux.ui.toggleClass(name, on)` | Toggles a class on `<body>`. |
| `flux.decorations(list)` | Monaco decorations in the editor. Returns a collection with `.set(list)` and `.clear()`. |
| `flux.onSave(cb)`, `flux.onOpen(cb)` | Called with the file (`{ path, name, language, text }`). |
| `flux.onChange(cb)`, `flux.onSelection(cb)` | Called when the text or the cursor changes. |
| `flux.every(ms, cb)` | Like `setInterval`, stopped automatically. |
| `flux.project.folder()`, `await flux.project.files()`, `await flux.project.read(path)` | The open project: its folder, all file paths (relative, with `/`) and the text of a file. Read-only. |
| `flux.storage.get(key, fallback)`, `await flux.storage.set(key, value)` | Small saved settings for your plugin (JSON values). |
| `flux.own(disposable)` | Cleans up anything you create with Monaco directly (providers, listeners) when the plugin is turned off. Returns it. |
| `flux.editor`, `flux.monaco` | The Monaco editor and API for anything advanced. |


Complete examples: the official plugins in [`plugins/`](https://github.com/pantr1x/Flux/tree/main/plugins).
