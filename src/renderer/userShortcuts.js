// Vlastné skratky zo súboru shortcuts.json (Nastavenia → Skratky → Upraviť).
// Každá skratka môže spustiť príkaz/skript vo výstupe, vložiť text, spustiť príkaz Fluxu alebo editora,
// otvoriť web, alebo viac krokov za sebou.
import { t } from './i18n.js';

const flux = window.flux;

const KEY_ALIASES = { esc: 'escape', del: 'delete', ins: 'insert', up: 'arrowup', down: 'arrowdown', left: 'arrowleft', right: 'arrowright', space: ' ', plus: '+', return: 'enter', pgup: 'pageup', pgdn: 'pagedown' };

// „Ctrl+Shift+K“ → { ctrl, shift, alt, key: 'k' }
export function parseKey(spec) {
  const parts = String(spec).split('+').map((x) => x.trim()).filter(Boolean);
  if (String(spec).trim().endsWith('++')) parts.push('+');
  const k = { ctrl: false, shift: false, alt: false, meta: false, key: '' };
  for (const p of parts) {
    const l = p.toLowerCase();
    if (l === 'ctrl' || l === 'control' || l === 'cmd') k.ctrl = true;
    else if (l === 'shift') k.shift = true;
    else if (l === 'alt' || l === 'option') k.alt = true;
    else if (l === 'win' || l === 'meta' || l === 'super') k.meta = true;
    else k.key = KEY_ALIASES[l] || l;
  }
  return k.key ? k : null;
}

function matches(e, k) {
  if (e.ctrlKey !== k.ctrl || e.shiftKey !== k.shift || e.altKey !== k.alt || e.metaKey !== k.meta) return false;
  const key = e.key.toLowerCase();
  if (key === k.key) return true;
  // So Shiftom/Altom dáva e.key iný znak – porovnáme aj fyzickú klávesu.
  const code = e.code.toLowerCase();
  if (/^[a-z]$/.test(k.key)) return code === `key${k.key}`;
  if (/^[0-9]$/.test(k.key)) return code === `digit${k.key}` || code === `numpad${k.key}`;
  const codes = { '/': 'slash', '\\': 'backslash', ',': 'comma', '.': 'period', ';': 'semicolon', "'": 'quote', '[': 'bracketleft', ']': 'bracketright', '-': 'minus', '=': 'equal', '`': 'backquote' };
  return codes[k.key] === code;
}

export function createUserShortcuts({ commands, editor, getContext, saveAll, toast }) {
  let list = [];
  let file = null;

  async function load(showErrors = false) {
    try {
      const res = await flux.shortcutsFile();
      file = res.path;
      const data = JSON.parse(res.text);
      const items = Array.isArray(data) ? data : data.shortcuts || [];
      list = items
        .map((s) => ({ ...s, parsed: parseKey(s.key) }))
        .filter((s) => s.parsed);
      if (showErrors) toast(t('{n} custom shortcut(s) loaded.', { n: list.length }), 'ok');
    } catch (err) {
      if (showErrors) toast(t('shortcuts.json has an error: {msg}', { msg: String(err.message || err) }), 'error', 8000);
    }
    return list;
  }

  const expand = (text) => {
    const c = getContext();
    return String(text).replace(/\$\{(\w+)\}/g, (m, v) => (c[v] ?? m));
  };

  async function exec(step) {
    if (!step || typeof step !== 'object') return;
    if (step.sequence) {
      for (const s of step.sequence) await exec(s);
      return;
    }
    if (step.run) {
      await saveAll();
      const c = getContext();
      const res = await flux.runShell(expand(step.run), step.cwd ? expand(step.cwd) : c.workspace || c.dir);
      if (!res?.ok) toast(t('Could not run: {cmd}', { cmd: step.run }), 'error');
      return;
    }
    if (step.insert != null) {
      if (!editor.getModel()) return;
      editor.focus();
      const snippet = editor.getContribution('snippetController2');
      const text = expand(step.insert);
      if (snippet) snippet.insert(text);
      else editor.trigger('flux', 'type', { text: text.replace('$0', '') });
      return;
    }
    if (step.url) {
      flux.openExternal(encodeURI(expand(step.url)).replace(/%25/g, '%'));
      return;
    }
    if (step.command) {
      const fn = commands[step.command];
      if (fn) return fn();
      // Akákoľvek akcia editora (napr. editor.action.commentLine).
      const action = editor.getAction(step.command);
      if (action) return action.run();
      editor.trigger('flux', step.command, step.args ?? null);
    }
  }

  window.addEventListener(
    'keydown',
    (e) => {
      if (!list.length || e.repeat || window.fluxRecordingKeys) return;
      const hit = list.find((s) => matches(e, s.parsed));
      if (!hit) return;
      e.preventDefault();
      e.stopImmediatePropagation(); // aby nezbehla aj vstavaná skratka (napr. Ctrl+P pri Ctrl+Alt+P)
      exec(hit).catch((err) => toast(String(err.message || err), 'error'));
    },
    true,
  );

  load();
  return { load, list: () => list, file: () => file, exec };
}
