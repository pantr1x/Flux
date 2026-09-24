# Flux plugins

A plugin is a folder with a small JavaScript file. Flux loads it at start-up and gives it a `flux` object
with everything it can use: commands, the status bar, the editor, snippets, styles, events and storage.

## Quick start (2 minutes)

1. In Flux open **Settings → Plugins → Create a plugin**. Flux creates a `flux-plugin-my-plugin` folder and opens it.
2. Open **Settings → Plugins → Load from folder…** and pick that folder. The plugin runs right away.
3. Change `plugin.js`, then press **Reload** next to the plugin in **Loaded from folders**.

## The folder

```
my-plugin/
  plugin.json      manifest (required)
  plugin.js        code with activate(flux) (required)
  icon.svg         icon shown in the store (square, svg or png)
  screenshot.png   pictures shown on the plugin page (optional, any number)
  README.md        long description shown on the plugin page
```

### plugin.json

```json
{
  "id": "yourname.my-plugin",
  "name": "My plugin",
  "publisher": "yourname",
  "version": "1.0.0",
  "description": "One sentence about what it does.",
  "main": "plugin.js",
  "icon": "icon.svg",
  "screenshots": ["screenshot.png"],
  "readme": "README.md",
  "files": ["plugin.js", "icon.svg", "README.md"],
  "tags": ["productivity"],
  "license": "MIT",
  "issue": null
}
```

| Field | Meaning |
| --- | --- |
| `id` | Unique, lower case, `publisher.name` (letters, numbers, `.` and `-`). |
| `version` | Raise it on every change – users get an **Update** button. |
| `main` | The JavaScript file to load (an ES module). |
| `files` | Every file Flux downloads on install (text files: js, css, json, svg, md). Screenshots are only shown, not downloaded. |
| `issue` | Number of the GitHub issue used for ratings (👍 / 👎). The Flux team fills it in when your plugin is accepted. |

## plugin.js

```js
export function activate(flux) {
  flux.commands.register('hello', 'Say hello', () => flux.toast('Hello!'), { key: 'Ctrl+Alt+H' });
}

// optional – everything registered through `flux` is cleaned up automatically anyway
export function deactivate() {}
```

## API reference

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

Look at the plugins in [`plugins/`](../plugins) for complete examples – they are short.

## Publish your plugin

1. **Fork** [pantr1x/Flux](https://github.com/pantr1x/Flux) on GitHub.
2. Copy your folder to `plugins/<your id>/` (the folder name must be the same as `id`).
3. Add a screenshot – it is what people look at first.
4. Add an entry to `plugins/index.json` (the same fields as your `plugin.json`, without `main`, `files` and `readme`, and with `"verified": false`).
5. Open a **pull request**. After a quick review for safety it is merged and shows up in **Settings → Plugins → Community** for everyone.

To publish a new version, raise `version` in both files and open another pull request.

### Rules

- No hidden network requests, no reading files outside the open project, no obfuscated code.
- Keep it small and readable – reviewers must be able to understand it.
- Plugins made by the Flux team have a ✓ next to the publisher name.

## Ratings

Each plugin has a GitHub issue. 👍 and 👎 reactions on it are the rating shown as stars in the store,
and comments are the reviews. Users can rate from the plugin page in Flux once they connect GitHub in **Settings → GitHub**.
