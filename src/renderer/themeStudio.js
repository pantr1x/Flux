// Štúdio vlastnej témy: farby cez jednoduché okienka, zmeny vidno hneď v editore.
// „Advanced“ otvorí JSON súbor témy – aj tam sa farby menia už počas písania.
import { t } from './i18n.js';
import { icon } from './icons.js';

const flux = window.flux;

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);

export const THEME_GROUPS = [
  ['Basics', [['fg', 'Text'], ['comment', 'Comments'], ['delimiter', 'Brackets and symbols']]],
  ['Code', [['keyword', 'Keywords (if, for, return)'], ['storage', 'def, class, function'], ['string', 'Text in quotes'], ['number', 'Numbers'], ['constant', 'Constants (True, None)'], ['regexp', 'Regular expressions']]],
  ['Names', [['function', 'Functions'], ['type', 'Classes and types'], ['variable', 'Variables'], ['parameter', 'Parameters'], ['property', 'Properties']]],
  ['Web', [['tag', 'HTML tags'], ['attr', 'HTML attributes']]],
];
const KEYS = THEME_GROUPS.flatMap(([, rows]) => rows.map(([k]) => k));

const SAMPLES = {
  python: `# Catch the stars ⭐
import random

class Player:
    def __init__(self, name, speed=5):
        self.name = name
        self.score = 0

def roll(sides: int) -> int:
    return random.randint(1, sides)

if roll(6) > 3 and True:
    print(f"Lucky {roll(6)}!")`,
  html: `<!-- My page -->
<section class="hero" id="top">
  <h1>Hello, world</h1>
  <a href="about.html">About me</a>
</section>
<style>
  .hero { color: #6b5cff; margin: 12px; }
</style>`,
  javascript: `// Count the clicks
const button = document.querySelector('#go');
let clicks = 0;

function onClick(event) {
  clicks += 1;
  button.textContent = \`Clicked \${clicks} times\`;
  return /\\d+/.test(button.textContent);
}`,
};

// JSON súboru témy (rovnaký tvar, aký načíta loadCustomThemes).
export function themeJson(theme) {
  const colors = {};
  for (const k of KEYS) if (theme.t[k]) colors[k] = `#${theme.t[k]}`;
  return `${JSON.stringify(
    {
      '//': 'Your own color theme. Colors are hex (#RRGGBB). Changes show up while you type; Ctrl+S saves.',
      name: theme.name,
      type: theme.type,
      italicComments: !!theme.t.italicComments,
      colors,
    },
    null,
    2,
  )}\n`;
}

export function createThemeStudio({ monaco, THEMES, apply, getFile, setType, openAdvanced, remove, builtIns }) {
  let el = null;
  let id = null;
  let saveTimer = null;
  let lang = 'python';

  const theme = () => THEMES[id];

  function save() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => flux.write(getFile(id), themeJson(theme())), 350);
  }

  async function preview() {
    const pre = el?.querySelector('.ts-preview');
    if (!pre) return;
    pre.innerHTML = await monaco.editor.colorize(SAMPLES[lang], lang, { tabSize: 4 });
  }

  function row(k, label) {
    const hex = `#${theme().t[k] || 'FFFFFF'}`.toLowerCase();
    return `<label class="ts-row"><span>${t(label)}</span><input type="text" class="ts-hex" data-hex="${k}" value="${hex}" spellcheck="false" maxlength="7"><input type="color" data-color="${k}" value="${hex}"></label>`;
  }

  function render() {
    const th = theme();
    el.innerHTML = `
      <header class="ts-head">${icon('palette', 16)}<b>${t('Theme studio')}</b><div class="grow"></div><button class="icon-btn" data-ts-close title="${t('Close (Esc)')}">${icon('x', 15)}</button></header>
      <div class="ts-body">
        <label class="ts-field"><span>${t('Name')}</span><input id="ts-name" value="${esc(th.name)}" spellcheck="false"></label>
        <div class="ts-field"><span>${t('Mode')}</span><div class="ts-seg">${['dark', 'light'].map((m) => `<button data-ts-type="${m}" class="${th.type === m ? 'on' : ''}">${icon(m === 'dark' ? 'moon' : 'sun', 13)}${t(m)}</button>`).join('')}</div></div>
        <label class="ts-field"><span>${t('Start from')}</span><select id="ts-base"><option value="">${t('Choose…')}</option>${builtIns()
          .map(([bid, b]) => `<option value="${bid}">${esc(b.name)}</option>`)
          .join('')}</select></label>
        <div class="ts-tabs">${Object.keys(SAMPLES)
          .map((l) => `<button data-ts-lang="${l}" class="${l === lang ? 'on' : ''}">${{ python: 'Python', html: 'HTML', javascript: 'JavaScript' }[l]}</button>`)
          .join('')}</div>
        <pre class="ts-preview monaco-editor"></pre>
        ${THEME_GROUPS.map(([g, rows]) => `<h4>${t(g)}</h4><div class="ts-group">${rows.map(([k, l]) => row(k, l)).join('')}</div>`).join('')}
        <label class="ts-row ts-italic"><span>${t('Italic comments')}</span><input type="checkbox" class="switch" id="ts-italic"${th.t.italicComments ? ' checked' : ''}></label>
      </div>
      <footer class="ts-foot">
        <button class="s-btn" data-ts-advanced title="${t('Edit the theme file – colors change while you type')}">${icon('code', 13)}${t('Advanced')}</button>
        <button class="icon-btn" data-ts-delete title="${t('Delete theme')}">${icon('trash', 14)}</button>
        <div class="grow"></div>
        <button class="s-btn primary" data-ts-close>${t('Done')}</button>
      </footer>`;
    preview();
  }

  function setColor(k, hex) {
    if (!/^#[0-9a-f]{6}$/i.test(hex)) return;
    theme().t[k] = hex.slice(1).toUpperCase();
    const c = el.querySelector(`[data-color="${k}"]`);
    const h = el.querySelector(`[data-hex="${k}"]`);
    if (c && c.value !== hex.toLowerCase()) c.value = hex.toLowerCase();
    if (h && document.activeElement !== h) h.value = hex.toLowerCase();
    apply(id);
    save();
  }

  function bind() {
    el.oninput = (e) => {
      const c = e.target.dataset.color || e.target.dataset.hex;
      if (c) return setColor(c, e.target.value.startsWith('#') ? e.target.value : `#${e.target.value}`);
      if (e.target.id === 'ts-name') {
        theme().name = e.target.value || t('My theme');
        return save();
      }
      if (e.target.id === 'ts-italic') {
        theme().t.italicComments = e.target.checked;
        apply(id);
        return save();
      }
    };
    el.onchange = (e) => {
      if (e.target.id !== 'ts-base' || !e.target.value) return;
      const base = THEMES[e.target.value];
      Object.assign(theme().t, { ...base.t });
      theme().type = base.type;
      setType(id, base.type);
      apply(id);
      save();
      render();
    };
    el.onclick = async (e) => {
      if (e.target.closest('[data-ts-close]')) return close();
      const ty = e.target.closest('[data-ts-type]');
      if (ty) {
        theme().type = ty.dataset.tsType;
        el.querySelectorAll('[data-ts-type]').forEach((b) => b.classList.toggle('on', b === ty));
        await setType(id, theme().type);
        return save();
      }
      const lg = e.target.closest('[data-ts-lang]');
      if (lg) {
        lang = lg.dataset.tsLang;
        el.querySelectorAll('[data-ts-lang]').forEach((b) => b.classList.toggle('on', b === lg));
        return preview();
      }
      if (e.target.closest('[data-ts-advanced]')) {
        clearTimeout(saveTimer);
        await flux.write(getFile(id), themeJson(theme()));
        close();
        return openAdvanced(id);
      }
      if (e.target.closest('[data-ts-delete]')) {
        if (await remove(id)) close();
      }
    };
    el.onkeydown = (e) => {
      if (e.key === 'Escape') close();
    };
  }

  function open(themeId) {
    id = themeId;
    if (!THEMES[id]) return;
    if (!el) {
      el = document.createElement('aside');
      el.id = 'theme-studio';
      document.body.append(el);
    }
    el.hidden = false;
    render();
    bind();
    el.querySelector('#ts-name').focus();
  }

  function close() {
    if (!el || el.hidden) return;
    clearTimeout(saveTimer);
    if (id && THEMES[id]) flux.write(getFile(id), themeJson(theme()));
    el.hidden = true;
  }

  return { open, close, isOpen: () => !!el && !el.hidden };
}
