# Publishing plugins

Plugins are published with a **fork** and a **pull request**. You do not need write access to Flux. Every plugin is checked by an **automatic safety review** before anyone can install it.

## Step by step

1. **Fork** [pantr1x/Flux](https://github.com/pantr1x/Flux). Press *Fork* in the top right on GitHub.
2. In your fork, add your plugin folder as `plugins/<your id>/`. The folder name must be the same as `id`, for example `plugins/anna.todo-highlight/`.
   - On github.com: open the `plugins` folder, press *Add file → Upload files* and drag your folder in.
   - Or with Git: clone your fork, copy the folder, then commit and push.
3. Add a **screenshot**. It is the first thing people see in the store.
4. Add your plugin to `plugins/index.json`. Use the same fields as your `plugin.json`, leave out `main`, `files` and `readme`, and set `"verified": false`:

   ```json
   {
     "id": "anna.todo-highlight",
     "name": "TODO Highlight",
     "publisher": "anna",
     "verified": false,
     "version": "1.0.0",
     "description": "Highlights TODO and FIXME in every file.",
     "icon": "icon.svg",
     "screenshots": ["screenshot.png"],
     "tags": ["productivity"]
   }
   ```

5. Open a **pull request** from your fork to `pantr1x/Flux`, with a short description of what the plugin does.

## The automatic review

Within a minute or two the **Plugin review** check runs and writes its result as a comment in your pull request. Your code is **only read, never run or installed**.

**Rules.** These checks run without AI:

- Only `plugins/` may change. Changes anywhere else are blocked.
- No `eval`, `new Function`, `window.flux`, Node.js (`require`, `process`) or Electron.
- No code loaded from the internet and no injected `<script>` tags.
- No obfuscated or minified code: very long lines and long encoded strings are blocked.
- Only code, JSON, Markdown, CSS, SVG and image files, and none bigger than 300 KB.
- `plugin.json` must match the folder name and `index.json`.
- The `flux.` prefix, the publisher *Flux* and `"verified": true` are reserved for the Flux team.
- Network requests and browser storage give a warning. Explain them in your README.

**AI review.** An AI reads the whole plugin and looks for these things. It uses Claude when an API key is set, and otherwise the free GitHub Models, so it always runs:

- stealing code, files, passwords or tokens,
- hidden network requests and keyloggers,
- remote code and crypto mining,
- plugins that do something different than their description says.

It answers **safe**, **suspicious** or **malicious** and lists what it found with file and line.

**Result:**

| Check | Meaning |
| --- | --- |
| ✅ green | No problems found. A maintainer gives the final OK and merges. |
| ❌ red | Something must be fixed, or a person has to look first. The comment says what. |

Push a fix to the same branch and the review runs again. The comment is updated, not duplicated.

## After it is merged

- Your plugin appears in **Settings → Plugins → Community** for everyone.
- People can rate it with 1–5 stars and write comments.
- **New version:** raise `version` in both `plugin.json` and `index.json` and open another pull request. It is reviewed the same way, and users get an **Update** button.
