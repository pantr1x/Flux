# Plugins for the new Flux

The new Flux (the fast native app) runs plugins written in **JavaScript** – one file with `activate(flux)`, no Node.js and no build step.

**Start here:** Settings → Plugins → **Create a plugin**. Flux makes a working example and opens it. Edit it, then **Ctrl+Shift+A → Reload plugins**.

What a plugin can do:

- commands in Ctrl+Shift+A, with shortcuts
- buttons in the status bar
- snippets in the suggestions (with Tab stops)
- code color themes
- colored marks with a message right in the code (like error hints)
- react when a file is opened, changed or saved, read the project's files, save small settings

➡ The full guide with the API reference and examples: **[docs/PLUGINS.md](https://github.com/pantr1x/Flux/blob/main/docs/PLUGINS.md)**
➡ Types for autocompletion: [docs/flux.d.ts](https://github.com/pantr1x/Flux/blob/main/docs/flux.d.ts)
➡ Publishing: [[Publishing plugins]] (add `"native": true` to your entry in `index.json`)
