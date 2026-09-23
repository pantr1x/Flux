// Spúšťanie pluginov. Každý plugin dostane objekt `flux` s jednoduchým API
// (príkazy, stavový riadok, editor, úryvky, štýly, udalosti, úložisko). Popis API: docs/PLUGINS.md
import { parseKey } from './userShortcuts.js';

const bridge = window.flux;

export function createPluginHost({ monaco, editor, toast, getActiveFile, getSettings, saveSettings, runCommand, isOverridden = () => false, getWorkspace = () => null }) {
  const loaded = new Map(); // id → { module, disposables, commands, name }
  const saveListeners = new Set();
  const openListeners = new Set();
  const keyHandlers = [];

  window.addEventListener(
    'keydown',
    (e) => {
      if (window.fluxRecordingKeys) return;
      const hit = keyHandlers.find((h) => {
        if (isOverridden(h.full)) return false; // skratku si zmenil v Nastaveniach → Skratky
        const k = h.key;
        return e.ctrlKey === k.ctrl && e.shiftKey === k.shift && e.altKey === k.alt && e.metaKey === k.meta && (e.key.toLowerCase() === k.key || e.code.toLowerCase() === `key${k.key}` || e.code.toLowerCase() === `digit${k.key}`);
      });
      if (!hit) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      hit.run();
    },
    true,
  );

  function makeApi(id, name, disposables, commands) {
    const own = (d) => (disposables.push(d), d);
    const data = () => getSettings().pluginData?.[id] || {};
    const api = {
      id,
      version: '1',
      monaco,
      editor,
      toast: (msg, kind = 'info', ms) => toast(String(msg), kind, ms),
      // Čokoľvek, čo treba pri vypnutí pluginu upratať (Monaco disposable alebo funkcia).
      own: (d) => (own(typeof d === 'function' ? d : () => d?.dispose?.()), d),
      activeFile: () => getActiveFile(),
      insertText(text) {
        if (!editor.getModel()) return false;
        editor.focus();
        const snippet = editor.getContribution('snippetController2');
        if (snippet && String(text).includes('$')) snippet.insert(String(text));
        else editor.executeEdits(id, [{ range: editor.getSelection(), text: String(text), forceMoveMarkers: true }]);
        return true;
      },
      commands: {
        register(cmdId, label, run, opts = {}) {
          const full = `${id}.${cmdId}`;
          const cmd = { id: full, label, run, plugin: name, key: opts.key || '' };
          commands.set(full, cmd);
          if (opts.key) {
            const k = parseKey(opts.key);
            if (k) {
              const h = { key: k, run, full };
              keyHandlers.push(h);
              own(() => keyHandlers.splice(keyHandlers.indexOf(h), 1));
            }
          }
          return own(() => commands.delete(full));
        },
        run(cmdId) {
          const cmd = commands.get(`${id}.${cmdId}`) || commands.get(cmdId);
          if (cmd) return cmd.run();
          return runCommand(cmdId);
        },
      },
      statusBar: {
        add({ text = '', title = '', onClick } = {}) {
          const el = document.createElement('button');
          el.className = 'status-item plugin-item';
          el.textContent = text;
          el.title = title || name;
          if (onClick) el.onclick = onClick;
          document.getElementById('st-autosave').before(el);
          own(() => el.remove());
          return {
            set: (t) => (el.textContent = t),
            setTitle: (t) => (el.title = t),
            show: (v = true) => (el.hidden = !v),
            remove: () => el.remove(),
            element: el,
          };
        },
      },
      snippets: {
        add(language, prefix, body, description = '') {
          const d = monaco.languages.registerCompletionItemProvider(language, {
            provideCompletionItems(model, position) {
              const word = model.getWordUntilPosition(position);
              const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn);
              return {
                suggestions: [
                  { label: prefix, kind: monaco.languages.CompletionItemKind.Snippet, insertText: body, insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet, detail: description || name, range },
                ],
              };
            },
          });
          own(() => d.dispose());
          return d;
        },
      },
      ui: {
        addStyle(css) {
          const style = document.createElement('style');
          style.dataset.plugin = id;
          style.textContent = css;
          document.head.append(style);
          own(() => style.remove());
          return { remove: () => style.remove() };
        },
        toggleClass(cls, on) {
          document.body.classList.toggle(cls, on);
          own(() => document.body.classList.remove(cls));
        },
      },
      decorations(options = []) {
        const col = editor.createDecorationsCollection(options);
        own(() => col.clear());
        return col;
      },
      onSave(cb) {
        saveListeners.add(cb);
        return own(() => saveListeners.delete(cb));
      },
      onOpen(cb) {
        openListeners.add(cb);
        return own(() => openListeners.delete(cb));
      },
      onChange(cb) {
        const d = editor.onDidChangeModelContent(() => cb(getActiveFile()));
        own(() => d.dispose());
        return d;
      },
      onSelection(cb) {
        const d = editor.onDidChangeCursorSelection(() => cb(getActiveFile()));
        own(() => d.dispose());
        return d;
      },
      every(ms, cb) {
        const timer = setInterval(cb, Math.max(200, ms));
        own(() => clearInterval(timer));
        return timer;
      },
      // Súbory otvoreného projektu (len na čítanie).
      project: {
        folder: () => getWorkspace(),
        async files() {
          const root = getWorkspace();
          if (!root) return [];
          const all = await bridge.listAll();
          return all.map((p) => p.slice(root.length + 1).replace(/\\/g, '/'));
        },
        async read(rel) {
          const root = getWorkspace();
          if (!root) throw new Error('No folder is open');
          const sep = root.includes('\\') ? '\\' : '/';
          return bridge.read(/^([a-z]:|\/)/i.test(rel) ? rel : `${root}${sep}${String(rel).replace(/[\\/]/g, sep)}`);
        },
      },
      storage: {
        get: (key, fallback) => (key in data() ? data()[key] : fallback),
        async set(key, value) {
          const all = { ...(getSettings().pluginData || {}) };
          all[id] = { ...(all[id] || {}), [key]: value };
          await saveSettings({ pluginData: all });
        },
      },
    };
    return api;
  }

  async function load(p) {
    const disposables = [];
    const commands = new Map();
    try {
      const mod = await import(/* @vite-ignore */ p.url);
      const api = makeApi(p.id, p.name, disposables, commands);
      await mod.activate?.(api);
      loaded.set(p.id, { module: mod, disposables, commands, name: p.name });
    } catch (err) {
      disposables.forEach((d) => d?.());
      console.error('plugin', p.id, err);
      toast(`${p.name}: ${err.message || err}`, 'error', 7000);
    }
  }

  async function unload(id) {
    const p = loaded.get(id);
    if (!p) return;
    try {
      await p.module.deactivate?.();
    } catch {}
    for (const d of p.disposables.reverse()) {
      try {
        d?.();
      } catch {}
    }
    loaded.delete(id);
  }

  async function start() {
    let list = [];
    try {
      list = await bridge.pluginsActive();
    } catch {}
    for (const p of list) await load(p);
  }

  // Po inštalácii / zapnutí / vypnutí: načítať znova len zmenené.
  async function refresh(id) {
    await unload(id);
    const list = await bridge.pluginsActive();
    const p = list.find((x) => x.id === id);
    if (p) await load(p);
  }

  return {
    start,
    refresh,
    unload,
    commands: () => [...loaded.values()].flatMap((p) => [...p.commands.values()]),
    loaded: () => [...loaded.keys()],
    emitSave: (file) => saveListeners.forEach((cb) => cb(file)),
    emitOpen: (file) => openListeners.forEach((cb) => cb(file)),
  };
}
