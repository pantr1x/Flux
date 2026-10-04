# Flux plugins

A plugin is a **folder with one small JavaScript file**. Flux loads it and gives it a `flux` object – that is all it can use.
With it you can add commands, status bar buttons, snippets, code themes and colored marks in the code, react when a file is opened,
changed or saved, and read the files of the open project. You need **no Node.js, no npm and no build step** – just a text editor
(Flux itself works fine).

Flux comes in two versions, and a plugin can support both:

| | **Flux** (the new, native app) | Flux Electron (the old app) |
|---|---|---|
| Engine | QuickJS, built into Flux | Chromium |
| Entry in `plugin.json` | `"native": "native.js"` | `"main": "plugin.js"` |
| API | [Native API](#native-api) | [Electron API](#api-reference-electron-flux) |
| Store | Settings → Plugins → **Store** | Settings → Plugins → Community |

This guide is mostly about the new Flux. The [Electron API](#api-reference-electron-flux) is further down.

---

## Quick start (2 minutes)

1. In Flux open **Settings → Plugins** and press **Create a plugin**.
   Flux makes the folder `Documents/Flux Plugins/my-plugin`, opens it as a project and turns the plugin on.
2. The example already works: press **Ctrl+Alt+H**, look at the line count in the status bar, or type `hello` in a Python file and press Enter.
3. Change `native.js`, save, then press **Ctrl+Shift+A → Reload plugins** (or **Reload** next to the plugin in Settings → Plugins).

That's the whole loop: edit → save → reload.

## The folder

```
my-plugin/
  plugin.json      information about the plugin (required)
  native.js        the code for Flux, with activate(flux) (required)
  flux.d.ts        types of the flux object – for autocompletion (optional)
  icon.svg         icon in the store (square, optional)
  README.md        long description (optional)
```

### plugin.json

```json
{
  "id": "yourname.my-plugin",
  "name": "My plugin",
  "publisher": "yourname",
  "version": "1.0.0",
  "description": "One sentence about what it does.",
  "native": "native.js",
  "main": "native.js",
  "files": ["native.js", "README.md"],
  "tags": ["productivity"],
  "license": "MIT"
}
```

| Field | Meaning |
| --- | --- |
| `id` | Unique, lower case, `publisher.name` (letters, numbers, `.` and `-`). |
| `version` | Raise it on every change – people who have the plugin get an **Update** button. |
| `native` | The JavaScript file for the new Flux. `true` means "use `main`" (when one file works in both apps). Without `native` the plugin is not shown in the new Flux. |
| `main` | The file for Flux Electron. |
| `files` | Every file Flux downloads on install (js, json, md, css, svg, txt). |

### native.js

```js
/** @param {import('./flux').Flux} flux */
export function activate(flux) {
  flux.commands.register('hello', 'Say hello', () => flux.toast('Hello!'), { key: 'Ctrl+Alt+H' });
}

// optional – everything you registered is removed automatically anyway
export function deactivate() {}
```

- `activate(flux)` runs when Flux starts, when the plugin is turned on and after **Reload**.
- One file only: there is no `import` / `require`. Put helpers in the same file.
- Plain modern JavaScript works (arrow functions, classes, `const`, template strings, `Map`, regular expressions with `/u`, `JSON`, `Math`, `Date`).
- There is **no** `window`, `document`, `fetch`, `setTimeout` or file system – use `flux.every()` for timers and `flux.project` for files.

## Native API

Everything a plugin adds through `flux` is removed when the plugin is turned off, updated or uninstalled.
`flux.d.ts` in this folder has the exact types.

### Basics

| API | What it does |
| --- | --- |
| `flux.id`, `flux.version` | Your plugin id, and `"native-1"` in the new Flux (Electron says `"1"`). |
| `flux.toast(text)` | Shows a short message. |
| `console.log(...)` | Writes to **Settings → Plugins → Plugin log**. |

### The open file

| API | What it does |
| --- | --- |
| `flux.activeFile()` | `{ path, name, language, text, selection: { start, end, text }, cursor: { line, column } }`, or `null` on the home screen. `language` is `python`, `javascript`, `typescript`, `html`, `css`, `json`, `markdown`, `c`, `cpp`, `csharp`, `java`, `go`, `rust`, `ruby`, `php`, `lua`, `shell` or `plaintext`. |
| `flux.insertText(text)` | Inserts at the cursor. `$0`, `$1`, `${1:name}` place the cursor like in snippets, `\n` keeps the line's indent. |
| `flux.replaceSelection(text)` | Replaces the selected text. |

```js
flux.commands.register('upper', 'Selection to UPPER CASE', () => {
  const f = flux.activeFile();
  if (f && f.selection.text) flux.replaceSelection(f.selection.text.toUpperCase());
});
```

### Commands

| API | What it does |
| --- | --- |
| `flux.commands.register(id, label, run, { key })` | Adds a command to **Ctrl+Shift+A**. `key` is a shortcut like `Ctrl+Alt+H` or `Ctrl+Shift+F7`. |
| `flux.commands.run(id)` | Runs one of your commands, or a built-in one: `run`, `save`, `settings`, `new-file`, `new-project`, `open-folder`, `ai`, `reload-plugins`. |

### Status bar

```js
const item = flux.statusBar.add({ text: '0 words', title: 'Shown on hover', onClick: () => flux.toast('Clicked') });
item.set('12 words');      // change the text
item.setTitle('…');        // change the hover text
item.show(false);          // hide / show
item.remove();             // remove it
```

### Snippets

```js
// language: like activeFile().language, or '*' for every language
flux.snippets.add('python', 'ifmain', 'if __name__ == "__main__":\n\t${1:main()}$0', 'if __name__ == "__main__":');
```

They show up in the suggestions while typing (with the `{}` icon). **Tab** jumps between `$1`, `$2` … and ends at `$0`.
`\t` is one level of indent.

### Code themes

```js
flux.themes.add('my-dark', {
  name: 'My Dark', type: 'dark', basedOn: 'vscode-dark',
  colors: { keyword: '#ff7ab2', string: '#a5d6ff', comment: '#6a737d' }
});
```

The theme appears in **Settings → Appearance → Code theme**. Color keys: `fg, comment, keyword, storage, string, number, type, function,
variable, parameter, property, constant, tag, attr, delimiter, regexp`. Missing keys come from `basedOn` (any built-in theme id).

### Marks in the code

```js
flux.marks.set([
  { line: 4, col: 2, len: 4, color: '#e5c07b', message: 'add the menu' },  // 0-based line and column
  { line: 9, kind: 'error', message: 'this breaks' }                       // len 0 = to the end of the line
]);
flux.marks.clear();
```

Marks are drawn like Flux's own error hints: a wavy line, a dot next to the line number and the message after the line.
They belong to the file that was open when you set them – set them again in `onOpen` / `onChange`.

### Events and timers

| API | When |
| --- | --- |
| `flux.onOpen(cb)` | A file is opened. |
| `flux.onChange(cb)` | The text changed – 300 ms after typing stops, not on every key. |
| `flux.onSelection(cb)` | The cursor or selection moved. |
| `flux.onSave(cb)` | The file was saved. |
| `flux.every(ms, cb)` | Repeats every `ms` milliseconds (at least 250). |

Every callback gets the same object as `flux.activeFile()`.

### Project and storage

| API | What it does |
| --- | --- |
| `flux.project.folder()` | Path of the open project, or `null`. |
| `flux.project.files()` | All file paths in the project, relative and with `/` (skips `node_modules`, `.git`, `target`, …). |
| `flux.project.read(path)` | Text of a file in the project (max 2 MB). Throws if it can't be read. Files outside the project can't be read. |
| `flux.storage.get(key, fallback)` / `flux.storage.set(key, value)` | Small saved data for your plugin (any JSON value). |

### Limits (to keep Flux fast and safe)

- One call into your plugin may take **200 ms**. A longer one (for example an endless loop) is stopped, and the plugin keeps working.
- All plugins together may use **64 MB** of memory.
- No network, no files outside the project, no running programs.

## Examples

All in [`plugins/`](../plugins), each just a few lines:

| Plugin | Shows |
| --- | --- |
| [`flux.word-count`](../plugins/flux.word-count/native.js) | status bar, `onChange`/`onSelection`, a command |
| [`flux.todo-highlight`](../plugins/flux.todo-highlight/native.js) | `flux.marks`, status bar with click |
| [`flux.snippet-pack`](../plugins/flux.snippet-pack/native.js) | many snippets for many languages |
| [`flux.theme-pack`](../plugins/flux.theme-pack/plugin.js) | code themes (one file for both apps) |

### Walk-through: TODO highlight

```js
export function activate(flux) {
  const re = /\b(TODO|FIXME)\b[:\s]*(.*)/;
  function scan() {
    const f = flux.activeFile();
    if (!f) return flux.marks.clear();
    const marks = [];
    f.text.split('\n').forEach((line, i) => {
      const m = re.exec(line);
      if (m) marks.push({ line: i, col: m.index, len: m[1].length, color: '#e5c07b', message: m[2] });
    });
    flux.marks.set(marks);
  }
  flux.onOpen(scan);
  flux.onChange(scan);
  scan();
}
```

## Debugging

- If `activate` throws, the error is shown on the plugin's card in **Settings → Plugins** and as a message.
- `console.log()` goes to **Plugin log** under the installed plugins.
- After every change press **Reload** – Flux starts the plugin fresh.

---

### API reference (Electron Flux)

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


Electron plugins use `plugin.js` as an ES module and may use `flux.editor`/`flux.monaco`, CSS (`flux.ui.addStyle`) and Monaco decorations. These do not exist in the new Flux.

## Publish your plugin

Plugins are published with a **fork** and a **pull request**. You never need write access to Flux.

1. **Fork** [pantr1x/Flux](https://github.com/pantr1x/Flux) on GitHub (button *Fork* in the top right).
2. In your fork, add your folder as `plugins/<your id>/`. The folder name must be the same as `id`, for example `plugins/anna.todo-highlight/`.
3. Add a screenshot. It is what people look at first.
4. Add an entry to `plugins/index.json`. Use the same fields as your `plugin.json`, but leave out `main`, `files` and `readme`, and add `"verified": false`. For the new Flux add `"native": true`; a plugin that only works there also gets `"electron": false`.
5. Open a **pull request** from your fork to `pantr1x/Flux`.

### What happens after you open the pull request

- **Automatic review.** The *Plugin review* check runs within a minute or two and writes its result as a comment in the pull request.
  - **Rules.** It checks the files against the rules below: forbidden APIs, obfuscated code, files outside `plugins/`, and `plugin.json` against `index.json`.
  - **AI review.** An AI reads the whole plugin and looks for malware: stealing code, files or tokens, hidden network requests, remote code, keyloggers, and plugins that do something different than they say.
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

People rate a plugin with **1 to 5 stars** and can write a comment. Open the plugin in **Settings → Plugins** (Flux Electron) and press **Rate**.
Reading ratings works for everyone. To write one, you sign in with GitHub. Each plugin has its own GitHub issue in `pantr1x/Flux` where the ratings are stored.
