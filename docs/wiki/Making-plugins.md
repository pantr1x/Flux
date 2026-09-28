# Making plugins

A plugin is a **folder with a small JavaScript file**. Flux loads it and gives it a `flux` object. It uses that object to add commands, buttons, snippets, styles and themes. You do not need Node.js, npm or any build step.

## 1. Create the plugin

1. In Flux open **Settings → Plugins** and press **Create a plugin**.
2. Flux creates a folder `flux-plugin-my-plugin` with a working example you can change.
3. Press **Load from folder…** in **Settings → Plugins** and pick that folder. The plugin runs right away.

Every time you change the code, press **Reload** next to the plugin in **Loaded from folders**.

## 2. The folder

```
my-plugin/
  plugin.json      information about the plugin (required)
  plugin.js        the code, with activate(flux) (required)
  icon.svg         icon in the store (square)
  screenshot.png   pictures on the plugin page (optional)
  README.md        long description on the plugin page
```

`plugin.json`:

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
  "license": "MIT"
}
```

- `id` is unique and lower case: `yourname.plugin-name`.
- Raise `version` on every change. Users then get an **Update** button.
- `files` lists every file Flux downloads when someone installs the plugin.

## 3. Your first code

`plugin.js` exports `activate(flux)`. Flux calls it when the plugin starts:

```js
export function activate(flux) {
  flux.commands.register('hello', 'Say hello', () => flux.toast('Hello from my plugin!'), { key: 'Ctrl+Alt+H' });
}
```

Press **Ctrl+Alt+H**, or find *Say hello* in the command palette (**Ctrl+Shift+P**).

You do not have to clean anything up. Everything you add through `flux` is removed automatically when the plugin is turned off, updated or uninstalled.

## 4. Examples

### A button in the status bar

Shows the number of lines of the open file and updates while you type:

```js
export function activate(flux) {
  const item = flux.statusBar.add({ title: 'Lines in this file' });
  const update = () => {
    const file = flux.activeFile();
    item.show(!!file);
    if (file) item.set(`${file.text.split('\n').length} lines`);
  };
  flux.onChange(update);
  flux.onOpen(update);
  update();
}
```

### Snippets

Type the prefix and press **Tab**:

```js
export function activate(flux) {
  flux.snippets.add('html', 'card', '<div class="card">\n  <h2>${1:Title}</h2>\n  $0\n</div>', 'Card');
  flux.snippets.add('python', 'readfile', 'with open("${1:file.txt}") as f:\n    ${2:text} = f.read()', 'Read a file');
}
```

### A new look (CSS)

```js
export function activate(flux) {
  flux.ui.setVar('--radius', '20px');
  flux.ui.addStyle(`
    #tabs .tab.active { box-shadow: inset 0 -2px 0 var(--accent); }
  `);
}
```

### A color theme

```js
export function activate(flux) {
  flux.themes.add('sunset', {
    name: 'Sunset',
    type: 'dark',
    colors: { keyword: 'ff7a90', string: 'ffd28a', function: '7ad7ff', comment: '8a8398', number: 'ffb86b' },
  });
}
```

The theme appears in **Settings → Appearance → Code theme**.

### Doing something when you save

```js
export function activate(flux) {
  flux.onSave((file) => {
    if (file.text.includes('TODO')) flux.toast(`${file.name} still has a TODO`, 'warn');
  });
}
```

### Saving settings

```js
export function activate(flux) {
  let count = flux.storage.get('count', 0);
  flux.commands.register('count', 'Count clicks', async () => {
    count += 1;
    await flux.storage.set('count', count);
    flux.toast(`Clicked ${count} times`);
  });
}
```

## 5. Tips

- **Look at the official plugins.** The ones in [`plugins/`](https://github.com/pantr1x/Flux/tree/main/plugins) are short and show the whole API in real use.
- **Keep the code readable.** Minified or obfuscated code is rejected when you publish.
- **Use only the `flux` object.** `window.flux`, Node.js and Electron are off limits. The automatic review blocks them.
- **Next step:** [[Plugin API]] for everything `flux` can do, then [[Publishing plugins]].
