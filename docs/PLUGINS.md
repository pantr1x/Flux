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
| `issue` | Number of the GitHub issue that stores the ratings. The Flux team fills it in when your plugin is accepted. |

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

Plugins are published with a **fork** and a **pull request**. You never need write access to Flux.

1. **Fork** [pantr1x/Flux](https://github.com/pantr1x/Flux) on GitHub (button *Fork* in the top right).
2. In your fork, add your folder as `plugins/<your id>/`. The folder name must be the same as `id`, for example `plugins/anna.todo-highlight/`.
3. Add a screenshot. It is what people look at first.
4. Add an entry to `plugins/index.json`. Use the same fields as your `plugin.json`, but leave out `main`, `files` and `readme`, and add `"verified": false`.
5. Open a **pull request** from your fork to `pantr1x/Flux`.

### What happens after you open the pull request

- **Automatic review.** The *Plugin review* check runs within a minute or two and writes its result as a comment in the pull request.
  - **Rules.** It checks the files against the rules below: forbidden APIs, obfuscated code, files outside `plugins/`, and `plugin.json` against `index.json`.
  - **AI review.** Claude reads the whole plugin and looks for malware: stealing code, files or tokens, hidden network requests, remote code, keyloggers, and plugins that do something different than they say.
  - **Only read, never run.** Your code is never run and never installed during the review.
- **Fixing problems.** If something is wrong, the check is red and the comment says what to fix. Push a fix to the same branch and the review runs again.
- **Final OK.** When the check is green, a maintainer looks at it once more and merges it. From then on it shows up in **Settings → Plugins → Community** for everyone.

To publish a new version, raise `version` in both files and open another pull request. It is reviewed the same way.

### Rules

- **Use only the `flux` object your plugin gets.** Never use `window.flux`, Node.js (`require`, `process`) or Electron.
- **No `eval`, `new Function`, and no code loaded from the internet.**
- **No hidden network requests.** If your plugin needs the internet, say so in the README and show it to the user.
- **No obfuscated or minified code.** Keep it small and readable, so reviewers can understand it. Files over 300 KB and very long lines are rejected.
- **Only these files:** code, JSON, Markdown, CSS, SVG and images.
- **Reserved names:** the `flux.` prefix, the publisher name *Flux* and `"verified": true` are for plugins made by the Flux team. These have a ✓ next to the publisher name.

## Ratings

People rate a plugin with **1 to 5 stars** and can write a comment. Open the plugin in **Settings → Plugins** and press **Rate**.
Reading ratings works for everyone. To write one, you sign in with GitHub. Each plugin has its own GitHub issue in `pantr1x/Flux` where the ratings are stored.
