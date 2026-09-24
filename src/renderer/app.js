import * as monaco from 'monaco-editor';
import { emmetHTML, emmetCSS } from 'emmet-monaco-es';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { StandaloneServices } from 'monaco-editor/editor/standalone/browser/standaloneServices.js';
import { IStorageService } from 'monaco-editor/platform/storage/common/storage.js';
import { IKeybindingService } from 'monaco-editor/platform/keybinding/common/keybinding.js';
import { icon, fileIcon } from './icons.js';
import { flag } from './flags.js';
import { PythonLanguageClient } from './pyLsp.js';
import { THEMES, DEFAULT_THEME, themeOf, defineMonacoTheme, themeSwatch } from './themes.js';
import { TEMPLATES, PY_SNIPPETS, HTML_PAGE } from './templates.js';
import { createCodeMap } from './codemap.js';
import { t, setLocale, translateDom } from './i18n.js';
import { createActivity } from './activity.js';
import { createTools, TOOL_FILE } from './tools.js';
import { setupEditorExtras } from './editorExtras.js';
import { createUserShortcuts } from './userShortcuts.js';
import { createAIPanel, markdown } from './aiPanel.js';
import { createGitHub } from './github.js';
import { BINDINGS, CATEGORIES, createKeymap, kbdHtml } from './keymap.js';
import { createPluginHost } from './pluginHost.js';
import { createPluginsUI } from './pluginsUI.js';
import { createThemeStudio } from './themeStudio.js';
import { setupFancySelects } from './fselect.js';
import { createTogether } from './together.js';
import { createUpdatesUI } from './updatesUI.js';
import { createOnboarding } from './onboarding.js';
import { createMenubar } from './menubar.js';

const flux = window.flux;
const $ = (sel) => document.querySelector(sel);

// Web workery Monaco editora (autocomplete pre HTML/CSS/JS/JSON beží v nich).
self.MonacoEnvironment = {
  getWorker(_id, label) {
    const map = { json: 'json', css: 'css', scss: 'css', less: 'css', html: 'html', handlebars: 'html', razor: 'html', typescript: 'ts', javascript: 'ts' };
    return new Worker(`./workers/${map[label] || 'editor'}.js`);
  },
};

const ACCENTS = {
  mono: 'mono', // čiernobiela ako Zen – biela v tmavej téme, čierna vo svetlej
  violet: '#8b7bff',
  indigo: '#6366f1',
  blue: '#4f9dff',
  sky: '#38bdf8',
  teal: '#2ec4b6',
  green: '#3ecf8e',
  lime: '#a3d635',
  yellow: '#f5c542',
  orange: '#ff9f5a',
  red: '#ff5f6d',
  pink: '#ff6fb1',
  gray: '#9ca3af',
};

const FONTS = [
  { id: 'Consolas', label: 'Consolas (like VS Code)', css: "Consolas, 'Courier New', monospace" },
  { id: 'Cascadia Code', label: 'Cascadia Code', css: "'Cascadia Code', Consolas, monospace" },
  { id: 'Cascadia Mono', label: 'Cascadia Mono', css: "'Cascadia Mono', Consolas, monospace" },
  { id: 'JetBrains Mono', label: 'JetBrains Mono (if installed)', css: "'JetBrains Mono', Consolas, monospace" },
  { id: 'Fira Code', label: 'Fira Code (if installed)', css: "'Fira Code', Consolas, monospace" },
  { id: 'Courier New', label: 'Courier New', css: "'Courier New', monospace" },
];

// Predvolené nastavenia (podobné VS Code).
const DEFAULTS = {
  codeTheme: DEFAULT_THEME,
  lastDark: DEFAULT_THEME,
  lastLight: 'vscode-light',
  accent: 'mono',
  translucent: true,
  fontFamily: 'Consolas',
  fontSize: 14,
  lineHeight: 1.45,
  ligatures: false,
  minimap: true,
  stickyScroll: true,
  wordWrap: false,
  autosave: true,
  autoUpdateLangs: true,
  caretStyle: 'line',
  caretBlink: 'smooth',
  caretWidth: 2,
  caretSmooth: true,
  caretColor: '',
  caretAccent: false,
  pointer: 'text',
  pointerColor: '',
  pointerImage: '',
  pointerHotspot: 'tip',
  pointerEverywhere: false,
  fontCustom: '',
  uiFont: '',
  uiZoom: 100,
  lineNumbers: 'on',
  whitespace: 'selection',
  letterSpacing: 0,
  bracketColors: true,
  cornerRadius: 14,
  wallBlur: 40,
  wallOpacity: 55,
  darkLift: 0,
  density: 'comfortable',
  lite: false,
  uiText: '',
  uiText2: '',
  uiBase: '',
  uiCard: '',
  uiLine: '',
  cardAlpha: 74,
  userName: '',
  clearOnRun: true,
  terminalFontSize: 13,
  suggestDetails: true,
  inertia: true,
  menuBar: false,
  showSearch: true,
  panelPos: 'bottom',
  sidePos: 'left',
};
const setting = (key) => state.settings[key] ?? DEFAULTS[key];
// Pamäť a rýchlosť: každú časť si dá zapnúť/vypnúť v Nastaveniach → Všeobecné → Advanced.
// Kým ju nezmeníš, riadi sa hlavným prepínačom „Save memory“ (lite). true = funkcia je zapnutá.
const optOn = (key) => state.settings[key] ?? !setting('lite');
const OPT_KEYS = ['optFx', 'optAnim', 'optEditorFx', 'optPyAc', 'optJsLimit', 'pyMemory', 'lspIdle'];
const lspIdleMin = () => Number(state.settings.lspIdle ?? (setting('lite') ? 1 : 5));

const state = {
  platform: 'win32',
  settings: {},
  workspace: null,
  python: null,
  tabs: [],
  active: null,
  expanded: new Set(),
  dirCache: new Map(),
  selected: null,
  running: false,
  runOutput: '',
  stoppedByUser: false,
  live: null,
  lspStatus: 'off',
};

// ---------- cesty ----------
const sep = () => (state.platform === 'win32' ? '\\' : '/');
const basename = (p) => p.split(/[\\/]/).pop();
const dirname = (p) => p.slice(0, Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))) || p;
const extOf = (p) => (basename(p).includes('.') ? basename(p).split('.').pop().toLowerCase() : '');
const keyOf = (p) => (state.platform === 'win32' ? p.toLowerCase() : p);
const join = (...parts) => parts.join(sep()).replace(/[\\/]+/g, sep());
function inside(p, dir = state.workspace) {
  if (!dir || !p) return false;
  const a = keyOf(p.replace(/[\\/]+/g, sep()));
  const b = keyOf(dir.replace(/[\\/]+$/, ''));
  return a === b || a.startsWith(b + sep());
}
function relative(p) {
  return state.workspace && inside(p) ? p.slice(state.workspace.replace(/[\\/]+$/, '').length + 1) : p;
}

const LANGS = {
  py: 'python', pyi: 'python', pyw: 'python',
  js: 'javascript', mjs: 'javascript', cjs: 'javascript', jsx: 'javascript',
  ts: 'typescript', tsx: 'typescript', mts: 'typescript',
  html: 'html', htm: 'html', css: 'css', scss: 'scss', less: 'less',
  json: 'json', jsonc: 'json', md: 'markdown', xml: 'xml', svg: 'xml',
  yml: 'yaml', yaml: 'yaml', toml: 'ini', ini: 'ini', cfg: 'ini', env: 'ini',
  sh: 'shell', bat: 'bat', cmd: 'bat', ps1: 'powershell', sql: 'sql',
  c: 'c', h: 'c', cpp: 'cpp', hpp: 'cpp', cs: 'csharp', java: 'java', go: 'go', rs: 'rust',
  php: 'php', rb: 'ruby', lua: 'lua', kt: 'kotlin', swift: 'swift', dart: 'dart', r: 'r',
};
const LANG_NAMES = { python: 'Python', javascript: 'JavaScript', typescript: 'TypeScript', html: 'HTML', css: 'CSS', json: 'JSON', markdown: 'Markdown', plaintext: 'Text' };
const langFor = (p) => LANGS[extOf(p)] || 'plaintext';

// Súbor bez prípony (napr. „test“) – jazyk odhadneme podľa obsahu, aby mal farby.
function guessLang(text) {
  const head = text.slice(0, 4000);
  if (/^#!.*python/.test(head)) return 'python';
  if (/^#!.*\b(ba|z)?sh\b/.test(head)) return 'shell';
  if (/^\s*<(!doctype|html|head|body|div)\b/i.test(head)) return 'html';
  if (/^\s*(import \w|from [\w.]+ import |def \w+\(|class \w+[:(]|print\(|if __name__|for \w+ in |while .+:\s*$)/m.test(head)) return 'python';
  return 'plaintext';
}
const langOf = (path, text) => (extOf(path) ? langFor(path) : guessLang(text));

const RUNNABLE = new Set(['py', 'pyw', 'js', 'mjs', 'cjs', 'bat', 'cmd', 'ps1', 'sh', 'java', 'go', 'cs', 'c', 'cpp', 'cc', 'cxx', 'rs', 'rb', 'php', 'lua', 'ts', 'mts', 'cts', 'pl']);
const WEB = new Set(['html', 'htm', 'css']);

// ---------- drobnosti UI ----------
// Upozornenie vpravo dole: ikona, text, voliteľné tlačidlo a ✕. Pod myšou nezmizne.
function toast(message, kind = 'info', ms = 4200, action = null) {
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  const ic = { error: 'x', ok: 'check', info: 'sparkle' }[kind] || 'sparkle';
  el.innerHTML = `<span class="toast-ic">${icon(ic, 13)}</span><span class="toast-msg"></span>${
    action ? `<button class="toast-act"></button>` : ''
  }<button class="toast-x" title="${t('Close')}">${icon('x', 13)}</button>`;
  el.querySelector('.toast-msg').textContent = message;
  // Rovnaké upozornenie znova → nahradí staré, nehromadia sa.
  for (const old of $('#toasts').children) if (old.dataset.msg === message) old.remove();
  el.dataset.msg = message;
  const close = () => {
    el.classList.add('out');
    setTimeout(() => el.remove(), 220);
  };
  if (action) {
    const b = el.querySelector('.toast-act');
    b.textContent = action.label;
    b.onclick = () => {
      el.remove();
      action.run();
    };
  }
  el.querySelector('.toast-x').onclick = close;
  let timer = setTimeout(close, ms);
  el.onmouseenter = () => clearTimeout(timer);
  el.onmouseleave = () => (timer = setTimeout(close, 1800));
  $('#toasts').append(el);
  while ($('#toasts').children.length > 4) $('#toasts').firstElementChild.remove();
}

function errorText(err) {
  // Chyby z hlavného procesu prichádzajú ako „Error invoking remote method '…': Error: text“.
  return String(err?.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, '');
}

function setIcons() {
  document.querySelectorAll('[data-cmd="new-file"]').forEach((b) => (b.innerHTML = icon('filePlus')));
  document.querySelectorAll('[data-cmd="new-folder"]').forEach((b) => (b.innerHTML = icon('folderPlus')));
  document.querySelectorAll('[data-cmd="refresh"]').forEach((b) => (b.innerHTML = icon('refresh')));
  document.querySelectorAll('[data-cmd="collapse"]').forEach((b) => (b.innerHTML = icon('collapse')));
  $('#btn-compact').innerHTML = icon('sidebar');
  $('#btn-expand').innerHTML = icon('sidebar');
  $('#btn-palette').innerHTML = icon('command');
  $('#btn-settings').innerHTML = `${icon('settings', 15)}<span>${t('Settings')}</span>`;
  $('#brand-mark').innerHTML = icon('code', 13);
  $('#btn-stop').innerHTML = icon('stop', 14);
  $('#btn-ai').innerHTML = `${icon('sparkle', 14)}<span>AI</span>`;
  $('#btn-clear').innerHTML = icon('trash', 15);
  document.querySelector('[data-ptab="shell"]').textContent = t('Terminal');
  $('#btn-panel').innerHTML = icon('panel', 15);
  $('#btn-preview-reload').innerHTML = icon('refresh', 15);
  $('#btn-preview-external').innerHTML = icon('external', 15);
  $('#btn-preview-close').innerHTML = icon('x', 15);
  const devices = $('#devices').children;
  devices[0].innerHTML = icon('monitor', 14);
  devices[1].innerHTML = icon('laptop', 14);
  devices[2].innerHTML = icon('tablet', 14);
  devices[3].innerHTML = icon('phone', 14);
  $('#btn-preview-rotate').innerHTML = icon('rotate', 15);
  $('#btn-preview-shot').innerHTML = icon('camera', 15);
  $('#btn-preview-phone').innerHTML = icon('qr', 15);
}

// ---------- téma ----------
function currentAccent() {
  const value = (state.workspace && state.settings.accents?.[state.workspace]) || setting('accent');
  if (value === 'mono') return monoColor();
  if (ACCENTS[value]) return ACCENTS[value];
  return /^#[0-9a-f]{6}$/i.test(value) ? value : monoColor();
}

const isDark = () => themeOf(setting('codeTheme')).type === 'dark';
const monoColor = () => (isDark() ? '#e6e6ea' : '#26262c');
const accentHex = (name) => (name === 'mono' ? monoColor() : ACCENTS[name]);

// Farba textu na farebnom tlačidle: čierna na svetlej farbe, biela na tmavej.
function readableOn(hex) {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  return 0.299 * r + 0.587 * g + 0.114 * b > 160 ? '#16161a' : '#ffffff';
}

function applyTheme() {
  const dark = isDark();
  const accent = currentAccent();
  document.body.classList.toggle('theme-dark', dark);
  document.body.classList.toggle('theme-light', !dark);
  document.body.classList.toggle('no-mica', state.material === 'none');
  document.body.classList.toggle('wallpaper', state.material === 'wallpaper');
  document.documentElement.style.setProperty('--accent', accent);
  document.documentElement.style.setProperty('--accent-fg', readableOn(accent));
  monaco.editor.setTheme(defineMonacoTheme(monaco, setting('codeTheme'), accent));
  $('#btn-theme').innerHTML = icon(dark ? 'sun' : 'moon');
  if (term) term.options.theme = terminalTheme();
  if (shTerm) shTerm.options.theme = terminalTheme();
  codemap?.refresh();
}

// Nastavenia editora a terminálu (písmo, veľkosť, minimapa…).
function applyEditorSettings() {
  const font = FONTS.find((f) => f.id === setting('fontFamily')) || FONTS[0];
  // Vlastné písmo (ľubovoľné nainštalované) má prednosť pred zoznamom.
  const custom = String(setting('fontCustom') || '').trim();
  const fontCss = custom ? `'${custom.replace(/'/g, '')}', ${font.css}` : font.css;
  editor?.updateOptions({
    fontFamily: fontCss,
    fontSize: setting('fontSize'),
    lineHeight: setting('lineHeight'),
    fontLigatures: setting('ligatures'),
    minimap: { enabled: false },
    stickyScroll: { enabled: setting('stickyScroll') !== false && optOn('optEditorFx'), maxLineCount: 4 },
    wordWrap: setting('wordWrap') ? 'on' : 'off',
    cursorStyle: setting('caretStyle'),
    cursorBlinking: setting('caretBlink'),
    cursorWidth: Number(setting('caretWidth')) || 2,
    cursorSmoothCaretAnimation: setting('caretSmooth') && optOn('optEditorFx') ? 'on' : 'off',
    // Bez efektov editora: menej prekresľovania.
    occurrencesHighlight: optOn('optEditorFx') ? 'singleFile' : 'off',
    renderLineHighlightOnlyWhenFocus: !optOn('optEditorFx'),
    matchBrackets: optOn('optEditorFx') ? 'always' : 'near',
    lineNumbers: setting('lineNumbers'),
    renderWhitespace: setting('whitespace'),
    letterSpacing: Number(setting('letterSpacing')) || 0,
    bracketPairColorization: { enabled: setting('bracketColors') },
  });
  applyCustomization();
  codemap?.setVisible(setting('minimap') && optOn('optEditorFx'));
  if (term) {
    term.options.fontFamily = font.css;
    term.options.fontSize = setting('terminalFontSize');
    try {
      fit.fit();
    } catch {}
  }
}

// Vzhľad podľa tvojho gusta: farba kurzora, ukazovateľ myši (aj crosshair), zaoblenie, písmo rozhrania, veľkosť, pozadie.
const POINTERS = {
  text: 'text',
  default: 'default',
  pointer: 'pointer',
  crosshair: 'crosshair',
};
function pointerCss(kind, color) {
  if (POINTERS[kind]) return POINTERS[kind];
  const c = encodeURIComponent(color || '#ffffff');
  const svg = {
    'flux-cross': `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24'><path d='M12 2v7M12 15v7M2 12h7M15 12h7' stroke='${c}' stroke-width='2' stroke-linecap='round'/><circle cx='12' cy='12' r='1.6' fill='${c}'/></svg>`,
    'flux-dot': `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24'><circle cx='12' cy='12' r='5' fill='${c}' fill-opacity='.85' stroke='black' stroke-opacity='.4'/></svg>`,
    'flux-ring': `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24'><circle cx='12' cy='12' r='7' fill='none' stroke='${c}' stroke-width='2'/><circle cx='12' cy='12' r='1.5' fill='${c}'/></svg>`,
  }[kind];
  if (kind === 'custom' && setting('pointerImage')) {
    const [w, h] = setting('pointerImageSize') || [32, 32];
    const [x, y] = setting('pointerHotspot') === 'center' ? [Math.round(w / 2), Math.round(h / 2)] : [0, 0];
    return `url("${setting('pointerImage')}") ${x} ${y}, auto`;
  }
  if (!svg) return 'text';
  // Windows spoľahlivo berie len bitmapové kurzory → SVG sa raz prekreslí do PNG.
  const key = `${kind}|${color}`;
  if (pngCursors[key]) return `url("${pngCursors[key]}") 12 12, crosshair`;
  const img = new Image();
  img.onload = () => {
    const cv = document.createElement('canvas');
    cv.width = cv.height = 24;
    cv.getContext('2d').drawImage(img, 0, 0, 24, 24);
    pngCursors[key] = cv.toDataURL('image/png');
    applyCustomization();
  };
  img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(decodeURIComponent(svg))}`;
  return `url("data:image/svg+xml;charset=utf-8,${encodeURIComponent(decodeURIComponent(svg))}") 12 12, crosshair`;
}
const pngCursors = {};

// Vlastné farby okna (text, pozadie, panely, čiary) – prepíšu farby témy.
function applyAppColors() {
  const b = document.body.style;
  const rgba = (hex, a) => {
    const n = parseInt(String(hex).slice(1), 16);
    return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
  };
  const set = (name, v) => (v ? b.setProperty(name, v) : b.removeProperty(name));
  const text = setting('uiText');
  const text2 = setting('uiText2');
  set('--text', text);
  set('--text-2', text2);
  set('--text-3', text2 ? rgba(text2, 0.78) : '');
  const base = setting('uiBase');
  set('--base', base);
  set('--base-alpha', base ? rgba(base, 0.45) : '');
  const card = setting('uiCard');
  const alpha = Math.max(0.3, Math.min(1, (Number(setting('cardAlpha')) || 74) / 100));
  set('--card-solid', card);
  set('--card', card ? rgba(card, alpha) : alpha !== 0.74 ? (isDark() ? `rgba(46, 46, 54, ${alpha})` : `rgba(255, 255, 255, ${alpha})`) : '');
  const line = setting('uiLine');
  set('--line-strong', line ? rgba(line, 0.5) : '');
  set('--line', line ? rgba(line, 0.28) : '');
}

function applyCustomization() {
  const root = document.documentElement;
  const caret = setting('caretColor');
  root.style.setProperty('--caret', caret || 'var(--accent)');
  document.body.classList.toggle('custom-caret', !!caret || setting('caretAccent'));
  root.style.setProperty('--editor-pointer', pointerCss(setting('pointer'), setting('pointerColor') || currentAccent()));
  document.body.classList.toggle('pointer-everywhere', !!setting('pointerEverywhere') && setting('pointer') !== 'text');
  root.style.setProperty('--radius', `${Number(setting('cornerRadius'))}px`);
  const uiFont = String(setting('uiFont') || '').trim();
  if (uiFont) root.style.setProperty('--ui-font', `'${uiFont.replace(/'/g, '')}', 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif`);
  else root.style.removeProperty('--ui-font');
  root.style.setProperty('--wall-blur', `${Number(setting('wallBlur'))}px`);
  root.style.setProperty('--wall-opacity', String(Number(setting('wallOpacity')) / 100));
  document.body.classList.toggle('density-compact', setting('density') === 'compact');
  // Rozloženie: menu a hľadanie hore, kde je panel s terminálom a bočný panel.
  document.body.classList.toggle('no-menubar', !setting('menuBar'));
  document.body.classList.toggle('no-topsearch', !setting('showSearch'));
  for (const pos of ['right', 'left']) document.body.classList.toggle(`panel-${pos}`, setting('panelPos') === pos);
  document.body.classList.toggle('panel-side', setting('panelPos') !== 'bottom');
  document.body.classList.toggle('side-right', setting('sidePos') === 'right');
  menubar?.render();
  document.body.classList.toggle('no-fx', !optOn('optFx'));
  document.body.classList.toggle('no-anim', !optOn('optAnim'));
  applyAppColors();
  root.style.setProperty('--dark-lift', String((Number(setting('darkLift')) || 0) * 0.0038));
  flux.setZoom?.(Number(setting('uiZoom')) / 100);
}

let liftTimer = null;
async function saveSettings(patch) {
  state.settings = await flux.setSettings(patch);
}

// ---------- tapeta za oknom ----------
// Rozmazaná tapeta Windows nakreslená priamo vo Fluxe, posúva sa spolu s oknom.
let wallpaperUrl = null;
async function setupWallpaper() {
  const layer = document.createElement('div');
  layer.id = 'wall';
  layer.innerHTML = '<div class="wall-img"></div>';
  document.body.prepend(layer);
  const img = layer.firstChild;
  const load = async () => {
    if (state.material !== 'wallpaper') return;
    const url = await flux.wallpaper();
    if (url && url !== wallpaperUrl) {
      wallpaperUrl = url;
      img.style.backgroundImage = `url("${url}")`;
    }
  };
  flux.onBounds(({ x, y, dw, dh }) => {
    img.style.width = `${dw}px`;
    img.style.height = `${dh}px`;
    img.style.transform = `translate(${-x}px, ${-y}px)`;
  });
  flux.onMaterial((m) => {
    state.material = m;
    // Bez tapety uvoľniť obrázok z pamäte.
    if (m !== 'wallpaper') {
      img.style.backgroundImage = 'none';
      wallpaperUrl = null;
    }
    applyTheme();
    load();
  });
  window.addEventListener('focus', load); // tapeta sa mohla zmeniť (hlavný proces vráti kópiu z cache)
  // Animácie pozadia stoja, keď okno nie je aktívne.
  window.addEventListener('blur', () => document.body.classList.add('idle'));
  window.addEventListener('focus', () => document.body.classList.remove('idle'));
  await load();
  flux.requestBounds();
}

// Čas v projekte: každých 30 s, ak je okno aktívne a za posledné 2 minúty si niečo robil.
let lastActivity = Date.now();
function trackTime() {
  for (const ev of ['keydown', 'mousedown', 'mousemove', 'wheel']) window.addEventListener(ev, () => (lastActivity = Date.now()), { passive: true, capture: true });
  setInterval(() => {
    if (state.workspace && document.hasFocus() && Date.now() - lastActivity < 120000) flux.addProjectTime(state.workspace, 30);
  }, 30000);
}

function formatTime(secs) {
  const m = Math.round(secs / 60);
  if (m < 1) return '< 1 min';
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return `${h} h ${m % 60 ? `${m % 60} min` : ''}`.trim();
}

const fmtNum = (n) => n.toLocaleString();

// ---------- editor ----------
let editor;
let codemap;
let activity;
let tools;
let userKeys;
let aiPanel;
let gh;
let together;
let menubar = null;
// GitHub je voliteľný vstavaný plugin (Nastavenia → Plugins); potrebuje Git.
const ghOn = () => setting('githubPlugin') === true;
let keymap;
let pluginHost;
let pluginsUI;
let updatesUI;
let userKeysCommands = {};
let onboarding;
function createEditor() {
  editor = monaco.editor.create($('#editor'), {
    model: null,
    fontFamily: (FONTS.find((f) => f.id === setting('fontFamily')) || FONTS[0]).css,
    fontLigatures: setting('ligatures'),
    fontSize: setting('fontSize'),
    lineHeight: setting('lineHeight'),
    automaticLayout: true,
    minimap: { enabled: false },
    wordWrap: setting('wordWrap') ? 'on' : 'off',
    smoothScrolling: false,
    cursorBlinking: 'smooth',
    cursorSmoothCaretAnimation: 'on',
    renderLineHighlight: 'all',
    roundedSelection: true,
    padding: { top: 10, bottom: 10 },
    scrollBeyondLastLine: false,
    bracketPairColorization: { enabled: true },
    guides: { bracketPairs: 'active', indentation: true },
    stickyScroll: { enabled: false },
    glyphMargin: true,
    lineDecorationsWidth: 6,
    lineNumbersMinChars: 3,
    mouseWheelZoom: true,
    quickSuggestions: { other: true, comments: false, strings: true },
    suggest: { preview: true, showStatusBar: false, selectionMode: 'always', showIcons: true },
    placeholder: '',
    inlayHints: { enabled: 'off' },
    inlineSuggest: { showToolbar: 'never' },
    scrollbar: { verticalScrollbarSize: 10, horizontalScrollbarSize: 10, useShadows: false },
    overviewRulerLanes: 2,
    tabSize: 4,
    fixedOverflowWidgets: true,
    'semanticHighlighting.enabled': true,
  });
  showSuggestDetails(setting('suggestDetails'));
  codemap = createCodeMap(monaco, editor, $('#codemap'));
  codemap.setVisible(setting('minimap'));

  inertiaScroll();
  monaco.editor.onDidChangeMarkers((uris) => {
    const m = editor.getModel();
    if (m && uris.some((u) => u.toString() === m.uri.toString())) updateProblems();
  });

  editor.onDidChangeCursorPosition((e) => {
    $('#st-pos').textContent = t('Ln {line}, Col {col}', { line: e.position.lineNumber, col: e.position.column });
  });

  // Klik na zelený ▶ pri „if __name__ == '__main__':“ spustí súbor (ako v PyCharme).
  editor.onMouseDown((e) => {
    if (e.target.type === monaco.editor.MouseTargetType.GUTTER_GLYPH_MARGIN) {
      const line = e.target.position?.lineNumber;
      const tab = activeTab();
      if (tab && tab.runLines?.includes(line)) run();
    }
  });

  monaco.editor.registerEditorOpener({
    openCodeEditor(_source, resource, selection) {
      openUri(resource, selection);
      return true;
    },
  });

  setupEditorExtras({
    monaco,
    editor,
    flux,
    getWorkspace: () => state.workspace,
    getFilePath: (model) => state.tabs.find((x) => x.model === model)?.path || null,
    onFsChanged: flux.onFsChanged,
  });

  // HTML / CSS: Emmet (napr. „div.card>p*3“ + Tab).
  emmetHTML(monaco, ['html'], { tokenizer: 'standard' });
  emmetCSS(monaco, ['css', 'scss', 'less'], { tokenizer: 'standard' });

  // JavaScript: nápoveda áno, ale bez falošných chýb o chýbajúcich moduloch.
  monaco.typescript?.javascriptDefaults.setDiagnosticsOptions({ noSemanticValidation: true, noSyntaxValidation: false });
  monaco.typescript?.javascriptDefaults.setCompilerOptions({
    allowJs: true,
    allowNonTsExtensions: true,
    target: monaco.typescript.ScriptTarget.ESNext,
    lib: ['esnext', 'dom', 'dom.iterable'],
  });
}

// ---------- chyby v kóde ----------
const problemsOf = (model) =>
  monaco.editor
    .getModelMarkers({ resource: model.uri })
    .filter((m) => m.severity >= monaco.MarkerSeverity.Warning)
    .sort((a, b) => b.severity - a.severity || a.startLineNumber - b.startLineNumber);

// Červená / žltá značka pri riadku s chybou + počet chýb dole v stavovom riadku.
function updateProblems() {
  const tab = activeTab();
  const el = $('#st-problems');
  if (!tab) {
    el.hidden = true;
    return;
  }
  const list = problemsOf(tab.model);
  const errors = list.filter((m) => m.severity === monaco.MarkerSeverity.Error).length;
  const warns = list.length - errors;
  const byLine = new Map();
  for (const m of list) if (!byLine.has(m.startLineNumber)) byLine.set(m.startLineNumber, m);
  const decos = [...byLine.values()].map((m) => ({
    range: new monaco.Range(m.startLineNumber, 1, m.startLineNumber, 1),
    options: {
      // Jemne ako vo VS Code: tenká čiarka pri riadku a farebné číslo riadku (žiadna bodka).
      linesDecorationsClassName: m.severity === monaco.MarkerSeverity.Error ? 'line-err' : 'line-warn',
      lineNumberClassName: m.severity === monaco.MarkerSeverity.Error ? 'ln-err' : 'ln-warn',
      overviewRuler: { color: m.severity === monaco.MarkerSeverity.Error ? '#ff6b7a' : '#f5b94a', position: monaco.editor.OverviewRulerLane.Left },
    },
  }));
  if (tab.problemDecos) tab.problemDecos.clear();
  tab.problemDecos = editor.createDecorationsCollection(decos);
  el.hidden = false;
  el.className = `status-item${errors ? ' err' : warns ? ' warn' : ' ok'}`;
  el.innerHTML = errors || warns
    ? `${errors ? `<span class="pb pb-e">●</span>${errors}` : ''} ${warns ? `<span class="pb pb-w">▲</span>${warns}` : ''}`.trim()
    : `${icon('check', 13)}${t('No problems')}`;
  el.title = errors || warns ? t('Click to see the list') : t('No problems in this file');
}

function showProblems() {
  const tab = activeTab();
  if (!tab) return;
  const list = problemsOf(tab.model);
  if (!list.length) return toast(t('No problems in this file. 👍'));
  openPalette({
    placeholder: t('Problems in {file}', { file: basename(tab.path) }),
    items: list.map((m) => ({
      label: `${t('Line {n}', { n: m.startLineNumber })}: ${m.message.split('\n')[0]}`,
      icon: `<span class="pb ${m.severity === monaco.MarkerSeverity.Error ? 'pb-e' : 'pb-w'}">${m.severity === monaco.MarkerSeverity.Error ? '●' : '▲'}</span>`,
      m,
    })),
    onPick: ({ m }) => {
      editor.setSelection(new monaco.Selection(m.startLineNumber, m.startColumn, m.endLineNumber, m.endColumn));
      editor.revealRangeInCenter(new monaco.Range(m.startLineNumber, m.startColumn, m.endLineNumber, m.endColumn));
      editor.focus();
    },
  });
}

// Po uložení (Ctrl+S) upozorní na chybu – klik na upozornenie ťa na ňu prenesie.
function announceProblems() {
  const tab = activeTab();
  if (!tab) return;
  setTimeout(() => {
    const errors = problemsOf(tab.model).filter((m) => m.severity === monaco.MarkerSeverity.Error);
    if (!errors.length) return;
    const m = errors[0];
    toast(`${t('Error on line {n}', { n: m.startLineNumber })}: ${m.message.split('\n')[0]}${errors.length > 1 ? `  (+${errors.length - 1})` : ''}`, 'error', 6000, {
      label: t('Go to error'),
      run: () => {
        editor.setPosition({ lineNumber: m.startLineNumber, column: m.startColumn });
        editor.revealLineInCenter(m.startLineNumber);
        editor.focus();
      },
    });
  }, 700);
}

// ---------- plynulé posúvanie so zotrvačnosťou („klzne ako na ľade“) ----------
function inertiaScroll() {
  const node = $('#editor');
  let velocity = 0;
  let frame = 0;
  let last = 0;
  const step = (t) => {
    const dt = Math.min(34, t - last) / 16.67;
    last = t;
    editor.setScrollTop(editor.getScrollTop() + velocity * dt);
    velocity *= Math.pow(0.9, dt);
    if (Math.abs(velocity) < 0.25) {
      velocity = 0;
      frame = 0;
      return;
    }
    frame = requestAnimationFrame(step);
  };
  node.addEventListener(
    'wheel',
    (e) => {
      if (!setting('inertia') || e.ctrlKey || e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
      if (e.target.closest('.suggest-widget, .monaco-hover, .parameter-hints-widget, .find-widget')) return;
      e.preventDefault();
      e.stopPropagation();
      const dy = e.deltaMode === 1 ? e.deltaY * editor.getOption(monaco.editor.EditorOption.lineHeight) : e.deltaY;
      // Zmena smeru zastaví pohyb hneď.
      if (Math.sign(dy) !== Math.sign(velocity)) velocity = 0;
      velocity += dy * 0.14;
      if (!frame) {
        last = performance.now();
        frame = requestAnimationFrame(step);
      }
    },
    { capture: true, passive: false },
  );
}

// Popis vybraného návrhu vedľa zoznamu (ako vo VS Code). Monaco si to pamätá vo svojom úložisku.
function showSuggestDetails(show) {
  try {
    StandaloneServices.get(IStorageService).store('expandSuggestionDocs', !!show, 0, 0);
  } catch {}
}

// Úryvky (snippety) pre Python: „main“, „for“, „def“… + Tab.
function registerSnippets() {
  monaco.languages.registerCompletionItemProvider('python', {
    provideCompletionItems(model, position) {
      const word = model.getWordUntilPosition(position);
      if (!word.word) return { suggestions: [] };
      const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn);
      return {
        suggestions: PY_SNIPPETS.map((sn) => ({
          label: { label: sn.label, description: t('snippet') },
          kind: monaco.languages.CompletionItemKind.Snippet,
          detail: sn.detail,
          documentation: { value: '```python\n' + sn.body.replace(/\$\{\d+:?([^}]*)\}/g, '$1').replace(/\t/g, '    ') + '\n```' },
          insertText: sn.body,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          sortText: '~' + sn.label,
          range,
        })),
      };
    },
  });
}

let lsp;
function createLanguageClient() {
  lsp = new PythonLanguageClient(monaco, {
    isLibrary: (uri) => !inside(monaco.Uri.parse(uri).fsPath),
    onStatus: (status) => {
      state.lspStatus = status;
      renderStatus();
    },
  });
}

// ---------- taby ----------
const activeTab = () => state.tabs.find((t) => t === state.active) || null;
const findTab = (p) => state.tabs.find((t) => keyOf(t.path) === keyOf(p));
const isDirty = (t) => !t.readonly && t.model.getAlternativeVersionId() !== t.savedVersion;

function createTab(path, model, readonly) {
  const lang = model.getLanguageId();
  model.updateOptions({ tabSize: ['python', 'java', 'csharp', 'rust', 'go', 'c', 'cpp'].includes(lang) ? 4 : 2, insertSpaces: true });
  const tab = { isNew: true, path, model, readonly, viewState: null, savedVersion: model.getAlternativeVersionId(), runLines: [], decorations: null };
  model.onDidChangeContent(() => {
    if (!extOf(path) && model.getLanguageId() === 'plaintext') {
      const lang = guessLang(model.getValue());
      if (lang !== 'plaintext') {
        monaco.editor.setModelLanguage(model, lang);
        renderStatus();
      }
    }
    renderTabs();
    reportDirty();
    updateRunGlyphs(tab);
    scheduleAutosave(tab);
    if (tab === state.active && !$('#mdview').hidden) scheduleMdPreview();
    if (/[\\/]config[\\/]themes[\\/][^\\/]+\.json$/i.test(path)) liveThemeFromText(path, model.getValue());
  });
  state.tabs.push(tab);
  return tab;
}

// Prázdny .html súbor dostane sám základnú kostru stránky (ako „!“ + Tab).
function fillEmptyHtml(tab) {
  if (tab.readonly || tab.model.getLanguageId() !== 'html' || tab.model.getValue().trim()) return;
  const title = basename(tab.path).replace(/\.[^.]+$/, '');
  const text = HTML_PAGE.replaceAll('{{title}}', title);
  const marker = text.indexOf('$0');
  const clean = text.replace('$0', '');
  tab.model.setValue(clean);
  const before = clean.slice(0, marker).split('\n');
  tab.viewState = null;
  tab.initialCursor = { lineNumber: before.length, column: before[before.length - 1].length + 1 };
}

const IMAGES = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'ico', 'bmp', 'avif']);

// Náhľad obrázka v plávajúcom okne (ako „Glance“ v Zene) – Esc alebo klik mimo ho zavrie.
async function glanceImage(path) {
  let img;
  try {
    img = await flux.readImage(path);
  } catch (err) {
    return toast(errorText(err), 'error');
  }
  const el = $('#glance');
  const kb = img.size < 1024 * 1024 ? `${Math.max(1, Math.round(img.size / 1024))} kB` : `${(img.size / 1024 / 1024).toFixed(1)} MB`;
  el.innerHTML = `<div class="gl-card"><div class="gl-bar"><span class="gl-name"></span><span class="gl-meta"></span><div class="grow"></div><button class="icon-btn" data-close title="${t('Close (Esc)')}">${icon('x', 15)}</button></div><div class="gl-stage"><img alt=""></div></div>`;
  el.querySelector('.gl-name').textContent = basename(path);
  const image = el.querySelector('img');
  image.onload = () => (el.querySelector('.gl-meta').textContent = `${image.naturalWidth} × ${image.naturalHeight} px · ${kb}`);
  image.src = img.url;
  el.hidden = false;
  el.onclick = (e) => {
    if (e.target === el || e.target.closest('[data-close]')) el.hidden = true;
  };
}

async function openFile(path, { line, column, focus = true } = {}) {
  if (IMAGES.has(extOf(path))) return glanceImage(path);
  let tab = findTab(path);
  if (!tab) {
    let model = monaco.editor.getModel(monaco.Uri.file(path));
    if (!model) {
      let text;
      try {
        text = await flux.read(path);
      } catch (err) {
        toast(errorText(err), 'error');
        return null;
      }
      // Pri súbežnom otvorení mohol model medzitým vzniknúť.
      model = monaco.editor.getModel(monaco.Uri.file(path)) || monaco.editor.createModel(text, langOf(path, text), monaco.Uri.file(path));
    }
    tab = findTab(path) || createTab(path, model, false);
    fillEmptyHtml(tab);
  }
  activate(tab);
  if (line) {
    const pos = { lineNumber: line, column: column || 1 };
    editor.setPosition(pos);
    editor.revealPositionInCenter(pos);
    if (!column) editor.setSelection(new monaco.Selection(line, 1, line, tab.model.getLineMaxColumn(line)));
  }
  if (focus) editor.focus();
  return tab;
}

function openUri(uri, selection) {
  const path = uri.fsPath;
  let tab = findTab(path);
  if (!tab) {
    const model = monaco.editor.getModel(uri);
    if (!model) return;
    tab = createTab(path, model, !inside(path));
  }
  activate(tab);
  if (selection) {
    const range = 'startLineNumber' in selection ? selection : new monaco.Range(selection.lineNumber, selection.column, selection.lineNumber, selection.column);
    editor.setSelection(range);
    editor.revealRangeInCenter(range);
  }
  editor.focus();
}

// Nápoveda v prázdnom súbore.
function placeholderFor(tab) {
  const lang = tab.model.getLanguageId();
  if (lang === 'html') return t('Type ! and press Tab for an HTML page skeleton');
  if (lang === 'python') return t('Start typing, e.g. print("Hello"), then press F5.  Snippets: main, for, def, input + Tab');
  if (lang === 'css') return 'Napr. body { background: #f4f4f8; }';
  return '';
}

function activate(tab) {
  const prev = activeTab();
  if (prev && prev !== tab) prev.viewState = editor.saveViewState();
  state.active = tab;
  editor.setModel(tab.model);
  editor.updateOptions({ readOnly: tab.readonly, placeholder: placeholderFor(tab) });
  if (tab.viewState) editor.restoreViewState(tab.viewState);
  else if (tab.initialCursor) {
    editor.setPosition(tab.initialCursor);
    tab.initialCursor = null;
  }
  if (isPythonTab(tab)) ensureLsp();
  updateRunGlyphs(tab);
  updateProblems();
  renderMdButton(tab);
  if (!$('#mdview').hidden) renderMdPreview();
  $('#welcome').hidden = true;
  if (inside(tab.path)) {
    state.selected = tab.path;
    revealInTree(tab.path);
  }
  renderTabs();
  renderTree();
  renderRunButton();
  renderStatus();
  followPreview(tab);
  document.title = `${basename(tab.path)} — Flux`;
  navNote();
  pluginHost?.emitOpen({ path: tab.path, name: basename(tab.path), language: tab.model.getLanguageId() });
}

// Stránka projektu: súbory ostanú otvorené v kartách, len sa žiadna nezobrazí.
function showProjectPage() {
  const prev = activeTab();
  if (prev) prev.viewState = editor.saveViewState();
  state.active = null;
  editor.setModel(null);
  renderTabs();
  renderRunButton();
  renderStatus();
  renderWelcome();
  document.title = 'Flux';
  navNote();
}

async function closeTab(tab, { force = false } = {}) {
  if (!force && isDirty(tab)) {
    const ok = await confirmPalette(t('“{file}” has unsaved changes.', { file: basename(tab.path) }), [
      { label: t('Save and close'), value: 'save' },
      { label: t('Close without saving'), value: 'discard' },
      { label: t('Cancel'), value: null },
    ]);
    if (!ok) return;
    if (ok === 'save' && !(await saveTab(tab))) return;
  }
  const i = state.tabs.indexOf(tab);
  state.tabs.splice(i, 1);
  clearTimeout(tab.autosaveTimer);
  if (state.active === tab) {
    const next = state.tabs[i] || state.tabs[i - 1];
    if (next) activate(next);
    else {
      state.active = null;
      editor.setModel(null);
      renderWelcome();
      document.title = 'Flux';
    }
  }
  tab.model.dispose();
  renderTabs();
  renderRunButton();
  renderStatus();
  reportDirty();
}

async function saveTab(tab) {
  if (!tab || tab.readonly) return true;
  if (!isDirty(tab)) return true;
  const version = tab.model.getAlternativeVersionId();
  try {
    await flux.write(tab.path, tab.model.getValue());
  } catch (err) {
    toast(`${t('Could not save')}: ${errorText(err)}`, 'error');
    return false;
  }
  tab.savedVersion = version;
  lsp.didSave(tab.model);
  together?.onSave(tab.path);
  pluginHost?.emitSave({ path: tab.path, name: basename(tab.path), language: tab.model.getLanguageId(), text: tab.model.getValue() });
  // Uložil si vlastné skratky alebo tému → hneď sa použijú.
  if (/[\\/]config[\\/]shortcuts\.json$/i.test(tab.path)) userKeys?.load(true);
  if (/[\\/]config[\\/]themes[\\/][^\\/]+\.json$/i.test(tab.path)) loadCustomThemes(true);
  renderTabs();
  reportDirty();
  return true;
}

async function saveAll() {
  for (const tab of state.tabs) if (isDirty(tab)) await saveTab(tab);
}

function scheduleAutosave(tab) {
  if (!setting('autosave') || tab.readonly) return;
  clearTimeout(tab.autosaveTimer);
  tab.autosaveTimer = setTimeout(() => saveTab(tab), 700);
}

function reportDirty() {
  flux.setDirty(state.tabs.filter(isDirty).length);
}

// ---------- súbory mimo projektu v bočnom paneli ----------
let looseRecent = [];
async function refreshLooseFiles() {
  try {
    looseRecent = await flux.recentFiles();
  } catch {}
  renderLooseFiles();
}
function renderLooseFiles() {
  const el = $('#loose');
  if (!el) return;
  const open = state.tabs.filter((tb) => !inside(tb.path) && !tb.readonly).map((tb) => tb.path);
  const seen = new Set(open.map(keyOf));
  const recent = looseRecent.filter((f) => !inside(f) && !seen.has(keyOf(f))).slice(0, Math.max(0, 5 - open.length));
  const files = [...open, ...recent];
  el.hidden = !files.length;
  if (!files.length) return (el.innerHTML = '');
  const cur = state.active ? keyOf(state.active.path) : '';
  el.innerHTML =
    `<div class="pr-head"><span>${t('Files')}</span><button class="icon-btn" data-act="newloose" title="${t('New file outside a project')}">${icon('filePlus', 15)}</button></div>` +
    files
      .map(
        (f) =>
          `<div class="pr-row lf-row${keyOf(f) === cur ? ' active' : ''}${seen.has(keyOf(f)) ? ' open' : ''}" data-file="${escapeAttr(f)}" title="${escapeAttr(f)}">${fileIcon(basename(f))}<span class="pr-text"><span class="pr-name">${escapeHtml(basename(f))}</span><small class="pr-sub">${escapeHtml(shortPath(dirname(f)))}</small></span><button class="pr-pin lf-x" data-forget title="${t('Remove from the list')}">${icon('x', 13)}</button></div>`,
      )
      .join('');
}
function setupLooseFiles() {
  const el = $('#loose');
  el.onclick = async (e) => {
    if (e.target.closest('[data-act="newloose"]')) return newStandaloneFile();
    const row = e.target.closest('[data-file]');
    if (!row) return;
    const f = row.dataset.file;
    if (e.target.closest('[data-forget]')) {
      const tab = state.tabs.find((tb) => keyOf(tb.path) === keyOf(f));
      if (tab) await closeTab(tab);
      if (state.tabs.some((tb) => keyOf(tb.path) === keyOf(f))) return;
      await flux.forgetRecentFile(f);
      return refreshLooseFiles();
    }
    const tab = state.tabs.find((tb) => keyOf(tb.path) === keyOf(f));
    if (tab) return activate(tab);
    const ok = await flux.openRecentFile(f);
    if (ok) return openStandalone([ok]);
    await flux.forgetRecentFile(f);
    toast(t('The file no longer exists.'), 'error');
    refreshLooseFiles();
  };
  refreshLooseFiles();
}

function renderTabs() {
  renderLooseFiles();
  const el = $('#tabs');
  el.innerHTML = '';
  for (const tab of state.tabs) {
    const div = document.createElement('div');
    div.className = `tab${tab === state.active ? ' active' : ''}${isDirty(tab) ? ' dirty' : ''}${tab.isNew ? ' enter' : ''}`;
    tab.isNew = false;
    div.title = tab.path;
    div.innerHTML = `${fileIcon(basename(tab.path), tab.model.getLanguageId())}<span class="name"></span>${tab.readonly ? `<span class="readonly">${t('read-only')}</span>` : ''}<button class="close" title="${t('Close (Ctrl+W)')}">${icon('x', 13)}</button>`;
    div.querySelector('.name').textContent = basename(tab.path);
    div.addEventListener('mousedown', (e) => {
      if (e.button === 1) {
        e.preventDefault();
        closeTab(tab);
      }
    });
    div.addEventListener('click', (e) => {
      if (e.target.closest('.close')) closeTab(tab);
      else activate(tab);
    });
    div.addEventListener('dblclick', (e) => {
      if (!e.target.closest('.close') && !tab.readonly) renameItem({ path: tab.path, dir: false });
    });
    div.title = `${tab.path}\n${t('Double-click to rename')}`;
    el.append(div);
    if (tab === state.active) requestAnimationFrame(() => div.scrollIntoView({ block: 'nearest', inline: 'nearest' }));
  }
}

function updateRunGlyphs(tab) {
  if (!tab || tab.model.isDisposed()) return;
  const lines = [];
  if (tab.model.getLanguageId() === 'python') {
    const count = tab.model.getLineCount();
    for (let i = 1; i <= count; i++) {
      if (/^if\s+__name__\s*==\s*["']__main__["']\s*:/.test(tab.model.getLineContent(i))) lines.push(i);
    }
  }
  tab.runLines = lines;
  if (tab !== state.active) return;
  const decos = lines.map((line) => ({
    range: new monaco.Range(line, 1, line, 1),
    options: { glyphMarginClassName: 'run-glyph', glyphMarginHoverMessage: { value: t('Run (F5)') } },
  }));
  if (tab.decorations) tab.decorations.clear();
  tab.decorations = editor.createDecorationsCollection(decos);
}

// ---------- strom súborov ----------
async function loadDir(dir) {
  try {
    const entries = await flux.list(dir);
    state.dirCache.set(keyOf(dir), entries);
    return entries;
  } catch {
    state.dirCache.delete(keyOf(dir));
    state.expanded.delete(keyOf(dir));
    return [];
  }
}

async function refreshTree() {
  if (!state.workspace) return renderTree();
  const dirs = [state.workspace, ...[...state.expanded].map((k) => state.expandedPaths?.get(k)).filter(Boolean)];
  await Promise.all(dirs.map(loadDir));
  renderTree();
}

function setExpanded(dir, open) {
  state.expandedPaths ??= new Map();
  if (open) {
    state.expanded.add(keyOf(dir));
    state.expandedPaths.set(keyOf(dir), dir);
  } else {
    state.expanded.delete(keyOf(dir));
  }
}

async function revealInTree(path) {
  let dir = dirname(path);
  const toOpen = [];
  while (inside(dir) && keyOf(dir) !== keyOf(state.workspace)) {
    if (!state.expanded.has(keyOf(dir))) toOpen.unshift(dir);
    dir = dirname(dir);
  }
  if (!toOpen.length) return;
  for (const d of toOpen) {
    setExpanded(d, true);
    await loadDir(d);
  }
  renderTree();
}

// Dlhé hodnoty v kartách štatistík („12 min ago“, „3 h 20 min“) menším písmom, aby sa zmestili celé.
function fitTiles(box) {
  for (const b of box?.querySelectorAll('.pj-tile b') || []) b.classList.toggle('pj-long', b.textContent.trim().length > 6);
}

function renderTree() {
  const el = $('#tree');
  if (!state.workspace) {
    el.innerHTML = `<div class="tree-empty">${t('No folder is open.')}<br><button id="tree-open">${t('Open folder')}</button></div>`;
    $('#tree-open').onclick = openFolderDialog;
    return;
  }
  const activeKey = state.active ? keyOf(state.active.path) : null;
  const dirtyKeys = new Set(state.tabs.filter(isDirty).map((t) => keyOf(t.path)));
  const html = [];
  const walk = (dir, depth) => {
    const entries = state.dirCache.get(keyOf(dir)) || [];
    for (const e of entries) {
      const k = keyOf(e.path);
      const open = e.dir && state.expanded.has(k);
      const cls = ['row'];
      if (k === activeKey) cls.push('active');
      if (state.selected && k === keyOf(state.selected)) cls.push('selected');
      const pad = 6 + depth * 14;
      const name = escapeHtml(e.name);
      if (e.dir) {
        html.push(
          `<div class="${cls.join(' ')}" data-path="${escapeAttr(e.path)}" data-dir="1" style="padding-left:${pad}px">` +
            `<span class="chev${open ? ' open' : ''}">${icon('chevron', 12)}</span>` +
            `<span class="folder-ic">${icon(open ? 'folderOpen' : 'folder', 15)}</span><span class="name">${name}</span></div>`,
        );
        if (open) walk(e.path, depth + 1);
      } else {
        html.push(
          `<div class="${cls.join(' ')}" data-path="${escapeAttr(e.path)}" style="padding-left:${pad + 20}px">` +
            `${fileIcon(e.name, findTab(e.path)?.model.getLanguageId())}<span class="name">${name}</span>${dirtyKeys.has(k) ? '<span class="dirty-dot"></span>' : ''}</div>`,
        );
      }
    }
  };
  walk(state.workspace, 0);
  el.innerHTML = html.join('') || `<div class="tree-empty">${t('This folder is empty.')}<br><button id="tree-new">${t('Create a file')}</button></div>`;
  together?.markTree();
  const newBtn = $('#tree-new');
  if (newBtn) newBtn.onclick = () => newFile();
}

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
}
const escapeAttr = escapeHtml;

function treeEvents() {
  const el = $('#tree');
  el.addEventListener('click', async (e) => {
    const row = e.target.closest('.row');
    if (!row) return;
    const path = row.dataset.path;
    state.selected = path;
    if (row.dataset.dir) {
      const open = !state.expanded.has(keyOf(path));
      setExpanded(path, open);
      if (open) await loadDir(path);
      renderTree();
    } else {
      openFile(path);
    }
  });
  el.addEventListener('contextmenu', async (e) => {
    e.preventDefault();
    if (!state.workspace) return;
    const row = e.target.closest('.row');
    const item = row ? { path: row.dataset.path, dir: !!row.dataset.dir } : { path: state.workspace, dir: true };
    state.selected = item.path;
    renderTree();
    const action = await flux.treeMenu(item);
    const baseDir = item.dir ? item.path : dirname(item.path);
    if (action === 'new-file') newFile(baseDir);
    else if (action === 'new-folder') newFolder(baseDir);
    else if (action === 'rename') renameItem(item);
    else if (action === 'delete') deleteItem(item);
    else if (action === 'copy-path') navigator.clipboard.writeText(item.path);
    else if (action === 'reveal') flux.reveal(item.path);
    else if (action === 'run') {
      await openFile(item.path);
      run();
    }
  });
}

function targetDir() {
  if (!state.workspace) return null;
  const sel = state.selected;
  if (sel && inside(sel)) {
    const entry = [...state.dirCache.values()].flat().find((e) => keyOf(e.path) === keyOf(sel));
    if (entry) return entry.dir ? entry.path : dirname(entry.path);
  }
  return state.active && inside(state.active.path) ? dirname(state.active.path) : state.workspace;
}

// Nový súbor: najprv šablóna (Python, HTML, web projekt…), potom názov.
async function newFile(dir = targetDir(), preset = null) {
  if (!dir) return openFolderDialog();
  const where = relative(dir) || basename(dir);
  const tpl = preset || await new Promise((resolve) =>
    openPalette({
      placeholder: t('Choose a template…'),
      note: t('New file in: {dir}', { dir: where }),
      items: TEMPLATES.map((tp) => ({
        label: t(tp.label),
        detail: t(tp.detail),
        icon: tp.project ? icon('folderPlus', 16) : tp.name ? fileIcon(tp.name) : icon('filePlus', 16),
        tpl: tp,
      })),
      onPick: (it) => resolve(it.tpl),
      onCancel: () => resolve(null),
    }),
  );
  if (!tpl) return;
  const dot = tpl.name.lastIndexOf('.');
  let name = await promptPalette({
    value: tpl.name,
    select: [0, dot > 0 ? dot : tpl.name.length],
    placeholder: tpl.project ? t('Project folder name') : t('e.g. main.py, index.html, styles/app.css'),
    note: tpl.project ? t('New project in: {dir}', { dir: where }) : `${t('File name')} (${t(tpl.label)})${tpl.files ? '' : ` · ${t('.py is added when there is no extension')}`}`,
  });
  if (!name) return;
  name = name.trim();
  try {
    if (!tpl.files) {
      if (!basename(name).includes('.')) name += '.py';
      const path = await flux.create(join(dir, name), false);
      await revealInTree(path);
      await refreshTree();
      return openFile(path);
    }
    const base = tpl.project ? join(dir, name) : dir;
    const title = tpl.project ? name : basename(name).replace(/\.[^.]+$/, '');
    let toOpen = null;
    let cursor = null;
    for (const f of tpl.files) {
      const fileName = f.name.replace('{{name}}', name);
      const path = join(base, fileName);
      let content = f.content.replaceAll('{{title}}', title);
      const marker = content.indexOf('$0');
      content = content.replace('$0', '');
      await flux.create(path, false);
      await flux.write(path, content);
      if (!toOpen || f.open) {
        toOpen = path;
        if (marker >= 0) {
          const before = content.slice(0, marker).split('\n');
          cursor = { line: before.length, column: before[before.length - 1].length + 1 };
        }
      }
    }
    if (tpl.project) setExpanded(base, true);
    renderProjects();
    await revealInTree(toOpen);
    await refreshTree();
    const tab = await openFile(toOpen, cursor || {});
    if (tab && cursor) editor.setSelection(new monaco.Selection(cursor.line, cursor.column, cursor.line, cursor.column));
  } catch (err) {
    toast(errorText(err), 'error');
  }
}

async function newFolder(dir = targetDir()) {
  if (!dir) return openFolderDialog();
  const name = await promptPalette({ placeholder: t('Folder name'), note: t('New folder in: {dir}', { dir: relative(dir) || basename(dir) }) });
  if (!name) return;
  try {
    const path = await flux.create(join(dir, name.trim()), true);
    setExpanded(path, true);
    await refreshTree();
  } catch (err) {
    toast(errorText(err), 'error');
  }
}

async function renameItem(item) {
  const old = basename(item.path);
  const dot = item.dir ? -1 : old.lastIndexOf('.');
  const name = await promptPalette({ value: old, select: [0, dot > 0 ? dot : old.length], note: t('New name') });
  if (!name || name === old) return;
  const target = join(dirname(item.path), name.trim());
  try {
    await flux.rename(item.path, target);
  } catch (err) {
    return toast(errorText(err), 'error');
  }
  // Otvorené taby presunúť na novú cestu.
  for (const tab of [...state.tabs]) {
    if (keyOf(tab.path) === keyOf(item.path) || inside(tab.path, item.path)) {
      const newPath = target + tab.path.slice(item.path.length);
      const wasActive = tab === state.active;
      const text = tab.model.getValue();
      const dirty = isDirty(tab);
      await closeTab(tab, { force: true });
      const model = monaco.editor.createModel(text, langFor(newPath), monaco.Uri.file(newPath));
      const nt = createTab(newPath, model, false);
      if (dirty) nt.savedVersion = -1;
      if (wasActive) activate(nt);
    }
  }
  state.selected = target;
  await refreshTree();
  renderTabs();
}

async function deleteItem(item) {
  let ok;
  try {
    ok = await flux.trash(item.path);
  } catch (err) {
    return toast(errorText(err), 'error');
  }
  if (!ok) return;
  for (const tab of [...state.tabs]) {
    if (keyOf(tab.path) === keyOf(item.path) || inside(tab.path, item.path)) await closeTab(tab, { force: true });
  }
  await refreshTree();
}

// Súbor sa zmenil mimo editora (napr. ho prepísal tvoj program) → načítať novú verziu.
async function syncOpenTabs() {
  for (const tab of state.tabs) {
    if (tab.readonly || isDirty(tab)) continue;
    let text;
    try {
      text = await flux.read(tab.path);
    } catch {
      continue;
    }
    if (tab.model.isDisposed() || isDirty(tab) || text === tab.model.getValue()) continue;
    const view = tab === state.active ? editor.saveViewState() : null;
    tab.model.pushEditOperations([], [{ range: tab.model.getFullModelRange(), text }], () => null);
    tab.savedVersion = tab.model.getAlternativeVersionId();
    if (view) editor.restoreViewState(view);
    renderTabs();
  }
}

// ---------- priečinok ----------
async function openFolderDialog() {
  const dir = await flux.openFolderDialog();
  if (dir) await setWorkspace(dir);
}

async function setWorkspace(dir) {
  const opened = await flux.openFolder(dir);
  if (!opened) {
    toast(t('The folder no longer exists.'), 'error');
    state.settings = await flux.setSettings({});
    return renderWelcome();
  }
  // Pred prepnutím projektu všetko ulož a zavri.
  await saveAll();
  for (const tab of [...state.tabs]) await closeTab(tab, { force: true });
  state.workspace = opened;
  if (shellAlive) startShell(true); // terminál sa presunie do nového projektu
  state.expanded.clear();
  state.dirCache.clear();
  state.selected = null;
  state.settings = await flux.setSettings({});
  closePreview();
  state.live = null;
  renderLive();
  renderProjects();
  applyTheme();
  $('#tree').classList.add('switching');
  await refreshTree();
  setTimeout(() => $('#tree').classList.remove('switching'), 600);
  renderWelcome();
  if (ghOn()) gh?.refreshStatus();
  else renderGitStatus(null);
  if (ghOn()) gh?.syncOnOpen().then(async (pulled) => {
    if (!pulled) return;
    await refreshTree();
    gh.refreshStatus();
  });
  await detectPython();
  // Pyright (doplňovanie pre Python) beží len keď treba – šetrí stovky MB pamäte.
  if (state.tabs.some(isPythonTab)) ensureLsp(true);
  else lsp.stop();
  together?.onProject();
}

// Po zmene pamäte/rýchlosti: vzhľad, editor a Python autocomplete hneď podľa nastavení.
function applyPerformance() {
  applyCustomization();
  applyEditorSettings();
  if (!optOn('optPyAc')) lsp?.stop();
  else if (state.tabs.some(isPythonTab)) ensureLsp(true);
  renderStatus();
}

// ---------- Python autocomplete len na požiadanie ----------
const isPythonTab = (tab) => tab?.model?.getLanguageId() === 'python' && inside(tab.path);
let lspIdleSince = 0;
function ensureLsp(force = false) {
  if (!state.workspace || !lsp || !optOn('optPyAc')) return;
  lspIdleSince = 0;
  if (!force && lsp.root === state.workspace) return;
  lsp.start(state.workspace, state.python?.path);
}
// Keď dlho nie je otvorený žiadny Python súbor, server sa vypne (v úspornom režime skôr).
setInterval(() => {
  if (!lsp?.root) return;
  if (state.tabs.some(isPythonTab)) {
    lspIdleSince = 0;
    return;
  }
  lspIdleSince ||= Date.now();
  if (lspIdleMin() && Date.now() - lspIdleSince > lspIdleMin() * 60e3) lsp.stop();
}, 30e3);

// ---------- projekty (ako „workspaces“ v Zene) ----------
// Zoznam nedávnych priečinkov s ikonou podľa obsahu – prepnutie jedným klikom.
// Ikona typu projektu (Python, web, Java…).
const KIND_FILE = { python: 'a.py', web: 'a.html', node: 'a.js', java: 'a.java', cpp: 'a.cpp', c: 'a.c', go: 'a.go', csharp: 'a.cs', rust: 'a.rs', ruby: 'a.rb', php: 'a.php', lua: 'a.lua' };
function kindIcon(kind, size = 16, github = false) {
  const svg = KIND_FILE[kind] ? fileIcon(KIND_FILE[kind]) : icon('folder', 16);
  const out = size === 16 ? svg : svg.replace(/width="16" height="16"/, `width="${size}" height="${size}"`);
  // projekt z GitHubu: malý znak v rohu ikony
  return github ? `<span class="kind-ic" title="GitHub">${out}<i class="kind-gh">${icon('github', Math.max(8, Math.round(size * 0.55)))}</i></span>` : out;
}

const KIND_LANG_NAMES = { python: 'Python', web: 'HTML/CSS', node: 'JavaScript', java: 'Java', cpp: 'C/C++', go: 'Go', csharp: 'C#', rust: 'Rust', ruby: 'Ruby', php: 'PHP', lua: 'Lua' };
// „JavaScript +3“ – hlavný jazyk a koľko ďalších má projekt
function langLine(p) {
  const langs = (p.langs || []).filter((l) => KIND_LANG_NAMES[l]);
  const main = KIND_LANG_NAMES[p.kind] || KIND_LANG_NAMES[langs[0]];
  if (!main) return '';
  const more = langs.filter((l) => l !== p.kind).length;
  return more ? `${main} +${more}` : main;
}
const langTitle = (p) => (p.langs || []).map((l) => KIND_LANG_NAMES[l]).filter(Boolean).join(', ');

async function renderProjects() {
  const el = $('#projects');
  let list = [];
  try {
    list = await flux.projects();
  } catch {}
  state.projectList = list;
  const row = (p) =>
    `<div class="pr-row${keyOf(p.dir) === keyOf(state.workspace || '') ? ' active' : ''}${p.pinned ? ' pinned' : ''}" data-dir="${escapeAttr(p.dir)}" data-pinned="${p.pinned ? 1 : ''}" title="${escapeAttr(p.dir)}${langTitle(p) ? `\n${escapeAttr(langTitle(p))}` : ''}\n${t('Right-click for more – rename, hide, delete')}">${kindIcon(p.kind, 16, p.github)}<span class="pr-text"><span class="pr-name">${escapeHtml(p.name)}</span><small class="pr-sub" data-stats="${escapeAttr(p.dir)}"></small></span><button class="pr-pin pr-hide" data-hide title="${p.hidden ? t('Show in the sidebar') : t('Hide from the sidebar')}">${icon(p.hidden ? 'eye' : 'eyeOff', 13)}</button><button class="pr-pin" data-pin title="${p.pinned ? t('Unpin') : t('Pin to top')}">${icon('pin', 13)}</button></div>`;
  const shown = (p) => !p.hidden;
  const pinned = list.filter((p) => p.pinned && shown(p));
  const rest = list.filter((p) => !p.pinned && shown(p));
  const hiddenCount = list.filter((p) => !shown(p)).length;
  el.innerHTML =
    (pinned.length ? `<div class="pr-head pr-head-pin"><span>${icon('pin', 11)}${t('Pinned')}</span></div><div class="pr-pinned">${pinned.map(row).join('')}</div>` : '') +
    `<div class="pr-head"><span>${t('Projects')}</span><button class="icon-btn" data-act="open" title="${t('Open an existing folder (Ctrl+O)')}">${icon('folderOpen', 15)}</button></div>` +
    rest.map(row).join('') +
    `<button class="pr-row pr-new" data-act="new">${icon('plus', 16)}<span>${t('New project')}</span></button>` +
    (hiddenCount
      ? `<button class="pr-row pr-new pr-hidden${state.showHidden ? ' open' : ''}" data-act="hidden">${icon('chevron', 13)}<span>${t('{n} hidden', { n: hiddenCount })}</span></button>` +
        (state.showHidden ? `<div class="pr-hidden-list">${list.filter((p) => !shown(p)).map(row).join('')}</div>` : '')
      : '');
  together?.markTree();
  const current = list.find((p) => keyOf(p.dir) === keyOf(state.workspace || ''));
  if (current && state.projectKind !== current.kind) {
    state.projectKind = current.kind;
    if (!state.active) renderWelcome();
  }
  // Pod každým projektom: čas a počet súborov.
  for (const p of list) {
    flux.projectStats(p.dir).then((st) => {
      const sub = [...el.querySelectorAll('.pr-sub')].find((x) => x.dataset.stats === p.dir);
      if (sub) sub.textContent = [langLine(p), `${st.files} ${st.files === 1 ? t('file') : t('files')}`, st.time >= 60 ? formatTime(st.time) : ''].filter(Boolean).join(' · ');
    });
  }
}

// Nový projekt: pekné okno – typ, názov, umiestnenie, „Create“.
const PROJECT_KINDS = [
  { id: 'python', title: 'Python', text: 'Scripts, games and apps', icon: 'a.py', tpl: 'py-main', tool: 'python' },
  { id: 'web', title: 'Website', text: 'HTML, CSS and JavaScript', icon: 'a.html', tpl: 'web' },
  { id: 'node', title: 'JavaScript', text: 'Scripts with Node.js', icon: 'a.js', tpl: 'js', tool: 'node' },
  { id: 'empty', title: 'Empty', text: 'Start from scratch', icon: null, tpl: null },
  // Ďalšie jazyky – stiahnu sa, až keď ich chceš.
  { id: 'java', title: 'Java', icon: 'a.java', tpl: 'java-main', tool: 'java', more: true },
  { id: 'cpp', title: 'C++', icon: 'a.cpp', tpl: 'cpp-main', tool: 'cpp', more: true },
  { id: 'c', title: 'C', icon: 'a.c', tpl: 'c-main', tool: 'cpp', more: true },
  { id: 'go', title: 'Go', icon: 'a.go', tpl: 'go-main', tool: 'go', more: true },
  { id: 'csharp', title: 'C#', icon: 'a.cs', tpl: 'cs-main', tool: 'csharp', more: true },
  { id: 'rust', title: 'Rust', icon: 'a.rs', tpl: 'rs-main', tool: 'rust', more: true },
  { id: 'ruby', title: 'Ruby', icon: 'a.rb', tpl: 'rb-main', tool: 'ruby', more: true },
  { id: 'php', title: 'PHP', icon: 'a.php', tpl: 'php-main', tool: 'php', more: true },
  { id: 'lua', title: 'Lua', icon: 'a.lua', tpl: 'lua-main', tool: 'lua', more: true },
  { id: 'ts', title: 'TypeScript', icon: 'a.ts', tpl: 'ts-main', tool: 'node', more: true },
  { id: 'perl', title: 'Perl', icon: 'a.pl', tpl: 'pl-main', tool: 'perl', more: true },
];

// Popis a zoznam úloh projektu (ukladá sa v nastaveniach podľa priečinka).
const projectMeta = (dir) => state.settings.projectMeta?.[dir] || {};
async function saveProjectMeta(dir, patch) {
  const all = { ...(state.settings.projectMeta || {}) };
  all[dir] = { ...all[dir], ...patch };
  await saveSettings({ projectMeta: all });
}

// opts.tpl: šablóna zo štartovacej obrazovky – projekt sa založí rovno s ňou.
async function newProject(opts = {}) {
  const el = $('#newproj');
  const prefs = setting('codeLangs') || [];
  let kind = opts.kind || (prefs.includes('web') && !prefs.includes('python') ? 'web' : 'python');
  let root = await flux.projectRoot();
  let nameTouched = false;
  const kindOf = (id) => PROJECT_KINDS.find((k) => k.id === id);
  const defaultName = () => ({ python: 'my-program', web: 'my-website', node: 'my-script', empty: 'new-project' })[kind] || `my-${kind}-app`;
  const big = (k, n) => (k.icon ? fileIcon(k.icon).replace(/width="16" height="16"/, `width="${n}" height="${n}"`) : icon('folder', n - 2));
  el.innerHTML = `
    <div class="np-card" role="dialog">
      <header><h2>${t('Create a project')}</h2><button class="icon-btn" data-close title="${t('Close (Esc)')}">${icon('x', 16)}</button></header>
      <div class="np-kinds">${PROJECT_KINDS.filter((k) => !k.more)
        .map((k) => `<button class="np-kind" data-kind="${k.id}"><span class="np-ic">${big(k, 30)}</span><b>${t(k.title)}</b><small>${t(k.text)}</small></button>`)
        .join('')}</div>
      <div class="np-more"><span>${t('More languages')}</span>${PROJECT_KINDS.filter((k) => k.more)
        .map((k) => `<button class="np-chip" data-kind="${k.id}">${big(k, 15)}${k.title}</button>`)
        .join('')}</div>
      <div id="np-tool"></div>
      <label class="np-field"><span>${t('Name')}</span><input id="np-name" spellcheck="false" autocomplete="off"></label>
      <label class="np-field"><span>${t('Short description')} <small>${t('optional')}</small></span><input id="np-desc" spellcheck="false" autocomplete="off" maxlength="160" placeholder="${t('e.g. A game where you catch falling stars')}"></label>
      <div class="np-field"><span>${t('Location')}</span><div class="np-loc"><code id="np-root"></code><button class="s-btn" data-root>${t('Change…')}</button></div></div>
      <footer>${ghOn() ? `<button class="ob-ghost np-gh" data-gh>${icon('github', 15)}${t('Open from GitHub…')}</button>` : ''}<div class="grow"></div><button class="ob-ghost" data-close>${t('Cancel')}</button><button class="ob-primary" data-create>${icon('plus', 15)}${t('Create project')}</button></footer>
    </div>`;
  const input = $('#np-name');
  tools.bind($('#np-tool'));
  // Pri jazyku, ktorý ešte nemáš, ukáže veľkosť a tlačidlo Inštalovať.
  const showTool = async () => {
    const box = $('#np-tool');
    const id = kindOf(kind)?.tool;
    if (!id) return (box.innerHTML = '');
    await tools.status();
    if (kindOf(kind)?.tool !== id) return;
    const tc = tools.info(id);
    box.innerHTML = tc && !tc.installed ? `<p class="np-note">${t('You will need {name} to run this project:', { name: escapeHtml(tc.name) })}</p>${tools.row(tc)}` : '';
  };
  const sync = () => {
    for (const b of el.querySelectorAll('[data-kind]')) b.classList.toggle('on', b.dataset.kind === kind);
    if (!nameTouched) input.value = defaultName();
    $('#np-root').textContent = `${root}${sep()}${input.value.trim() || '…'}`;
  };
  input.oninput = () => {
    nameTouched = true;
    sync();
  };
  sync();
  showTool();
  el.hidden = false;
  requestAnimationFrame(() => {
    input.focus();
    input.select();
  });
  const close = () => (el.hidden = true);
  const create = async () => {
    const name = input.value.trim();
    if (!name) return input.focus();
    const description = $('#np-desc').value.trim();
    let dir;
    try {
      dir = await flux.createProject(name, root);
    } catch (err) {
      return toast(errorText(err), 'error');
    }
    el.querySelector('.np-card').classList.add('done');
    setTimeout(close, 260);
    await saveProjectMeta(dir, { description, kind, todos: [] });
    await setWorkspace(dir);
    const kindTpl = TEMPLATES.find((x) => x.id === kindOf(kind).tpl);
    const tpl = opts.tpl && kind === opts.kind ? opts.tpl : kindTpl;
    if (!tpl || !tpl.files?.length) {
      await renderProjects();
      if (tpl) newFile(dir, tpl);
      return;
    }
    let first = null;
    for (const f of tpl.files) {
      const path = join(dir, f.name.replace('{{name}}', tpl.name || 'main.py'));
      await flux.create(path, false);
      await flux.write(path, f.content.replaceAll('{{title}}', name).replace('$0', kind === 'python' ? `print("Hello from ${name}!")` : ''));
      if (!first || f.open) first = path;
    }
    await refreshTree();
    await renderProjects();
    openFile(first);
  };
  el.onclick = async (e) => {
    if (e.target === el || e.target.closest('[data-close]')) return close();
    const k = e.target.closest('[data-kind]');
    if (k) {
      kind = k.dataset.kind;
      sync();
      return showTool();
    }
    if (e.target.closest('[data-root]')) {
      const picked = await flux.chooseProjectRoot();
      if (picked) root = picked;
      return sync();
    }
    if (e.target.closest('[data-create]')) create();
    if (e.target.closest('[data-gh]')) {
      close();
      gh.pickRepo();
    }
  };
  el.onkeydown = (e) => {
    if (e.key === 'Enter' && e.target.tagName === 'INPUT') create();
    if (e.key === 'Escape') close();
  };
}

async function renameProject(dir) {
  const old = basename(dir);
  const name = await promptPalette({ value: old, note: t('New project name (also renames the folder on disk)') });
  if (!name || name.trim() === old) return;
  const wasOpen = keyOf(dir) === keyOf(state.workspace || '');
  if (wasOpen) await saveAll();
  let res;
  try {
    res = await flux.renameProject(dir, name.trim());
  } catch (err) {
    return toast(errorText(err), 'error');
  }
  // Popis a úlohy idú s projektom.
  if (state.settings.projectMeta?.[dir]) {
    const all = { ...state.settings.projectMeta, [res.dir]: state.settings.projectMeta[dir] };
    delete all[dir];
    await saveSettings({ projectMeta: all });
  }
  if (wasOpen) {
    for (const tab of [...state.tabs]) await closeTab(tab, { force: true });
    state.workspace = null;
    await setWorkspace(res.dir);
  }
  renderProjects();
}

function projectEvents() {
  const el = $('#projects');
  el.onclick = async (e) => {
    const hide = e.target.closest('[data-hide]');
    if (hide) {
      const dir = hide.closest('[data-dir]').dataset.dir;
      const p = (state.projectList || []).find((x) => x.dir === dir);
      await flux.hideProject(dir, !p?.hidden);
      if (!p?.hidden) toast(t('“{name}” is hidden from the sidebar. You find it on the home screen.', { name: basename(dir) }), 'ok', 5000, { label: t('Undo'), run: () => flux.hideProject(dir, false).then(renderProjects) });
      return renderProjects();
    }
    const pin = e.target.closest('[data-pin]');
    if (pin) {
      const row = pin.closest('[data-dir]');
      await flux.pinProject(row.dataset.dir, !row.dataset.pinned);
      state.settings = await flux.setSettings({});
      return renderProjects();
    }
    const b = e.target.closest('button, .pr-row');
    if (!b) return;
    if (b.dataset.act === 'open') return openFolderDialog();
    if (b.dataset.act === 'new') return newProject();
    if (b.dataset.act === 'hidden') {
      state.showHidden = !state.showHidden;
      return renderProjects();
    }
    if (!b.dataset.dir) return;
    if (keyOf(b.dataset.dir) !== keyOf(state.workspace || '')) await setWorkspace(b.dataset.dir);
    else showProjectPage(); // už otvorený projekt → jeho stránka (popis, úlohy, štatistiky)
  };
  el.oncontextmenu = async (e) => {
    const b = e.target.closest('[data-dir]');
    if (!b) return;
    e.preventDefault();
    await projectMenu(b.dataset.dir);
    renderProjects();
  };
}

// Skryté projekty: otvoriť alebo vrátiť do bočného panela.
async function pickHiddenProject() {
  const hidden = (state.projectList || []).filter((p) => p.hidden);
  const it = await new Promise((resolve) =>
    openPalette({
      placeholder: t('Hidden projects'),
      note: t('Right-click a project to show it in the sidebar again.'),
      items: hidden.map((p) => ({ label: p.name, detail: shortPath(p.dir), icon: kindIcon(p.kind, 16, p.github), dir: p.dir })),
      onPick: resolve,
      onCancel: () => resolve(null),
    }),
  );
  if (it) await setWorkspace(it.dir);
}

// Ponuka projektu (pravý klik v bočnom paneli aj na domovskej obrazovke).
async function projectMenu(dir) {
  const p = (state.projectList || []).find((x) => keyOf(x.dir) === keyOf(dir)) || {};
  const choice = await confirmPalette(basename(dir), [
    { label: p.pinned ? t('Unpin') : t('Pin to top'), value: 'pin', icon: icon('pin', 15) },
    { label: t('Rename…'), value: 'rename', icon: icon('edit', 15) },
    { label: p.hidden ? t('Show in the sidebar') : t('Hide from the sidebar (stays on the home screen)'), value: 'hide', icon: icon('sidebar', 15) },
    { label: t('Remove from Flux (files stay on disk)'), value: 'forget', icon: icon('x', 15) },
    { label: t('Delete project… (moves the folder to the Recycle Bin)'), value: 'trash', icon: icon('trash', 15) },
  ]);
  if (choice === 'pin') await flux.pinProject(dir, !p.pinned);
  if (choice === 'rename') return renameProject(dir);
  if (choice === 'hide') {
    await flux.hideProject(dir, !p.hidden);
    if (!p.hidden) toast(t('“{name}” is hidden from the sidebar. You find it on the home screen.', { name: basename(dir) }), 'ok');
  }
  if (choice === 'forget') await flux.forgetProject(dir);
  if (choice === 'trash') {
    const sure = await confirmPalette(t('Delete “{name}”? The folder goes to the Recycle Bin, so you can still restore it.', { name: basename(dir) }), [
      { label: t('Delete project'), value: 'yes', icon: icon('trash', 15) },
      { label: t('Cancel'), value: null },
    ]);
    if (sure !== 'yes') return;
    const current = keyOf(dir) === keyOf(state.workspace || '');
    if (current) for (const tab of [...state.tabs]) if (inside(tab.path)) await closeTab(tab, { force: true });
    try {
      await flux.trashProject(dir);
    } catch (err) {
      return toast(errorText(err), 'error');
    }
    toast(t('“{name}” was moved to the Recycle Bin.', { name: basename(dir) }), 'ok');
    if (current) return location.reload();
  }
  state.settings = await flux.setSettings({});
  await renderProjects();
  if (!$('#start').hidden) openStart();
}

// ---------- Python ----------
async function detectPython() {
  state.python = await flux.findPython();
  renderStatus();
  if (lsp.ready) lsp.setPython(state.python?.path);
}

function renderStatus() {
  const py = $('#st-python');
  // Python dole len pri Python súbore alebo Python projekte (nie pri HTML a pod.).
  const activeLang = state.active?.model.getLanguageId();
  py.hidden = !(activeLang === 'python' || (!state.active && state.projectKind === 'python'));
  if (state.python) {
    py.className = 'status-item';
    py.innerHTML = `${icon('python', 13)}<span>Python ${state.python.version}</span><span style="opacity:.6">${escapeHtml(state.python.source)}</span>`;
    py.title = state.python.path;
  } else {
    py.className = 'status-item warn';
    py.innerHTML = `${icon('python', 13)}<span>${t('Python not found')}</span>`;
    py.title = t('Click to choose Python or install it');
  }
  const lspEl = $('#st-lsp');
  const lang = state.active?.model.getLanguageId();
  lspEl.hidden = lang !== 'python' || !inside(state.active?.path);
  const s = { off: ['', ''], starting: [t('Autocomplete loading…'), ''], ready: ['Autocomplete', 'ok'], error: [t('Autocomplete not running'), 'warn'] }[state.lspStatus];
  lspEl.className = `status-item ${s[1]}`;
  lspEl.innerHTML = s[0] ? `<span class="dot"></span>${s[0]}` : '';
  $('#st-lang').textContent = state.active ? LANG_NAMES[lang] || lang : '';
  if (!state.active) $('#st-pos').textContent = '';
  const auto = $('#st-autosave');
  auto.innerHTML = `${icon('save', 13)}${setting('autosave') ? t('Auto save') : t('Save: Ctrl+S')}`;
}

// ---------- terminál / výstup ----------
let term, fit;
let inputBuffer = '';

function terminalTheme() {
  const dark = isDark();
  const accent = ACCENTS[currentAccent()];
  return dark
    ? { background: '#00000000', foreground: '#dcdce6', cursor: accent, cursorAccent: '#2d2d34', selectionBackground: accent + '55', black: '#2a2a35', brightBlack: '#6f6f86', red: '#ff6b7a', green: '#3ecf8e', yellow: '#f5b94a', blue: '#6ea8ff', magenta: '#c792ea', cyan: '#5ccfe6', white: '#dcdce6' }
    : { background: '#00000000', foreground: '#1d1d24', cursor: accent, cursorAccent: '#f8f8fb', selectionBackground: accent + '44', black: '#1d1d24', brightBlack: '#8e8e9c', red: '#e0364a', green: '#17a86b', yellow: '#b7791f', blue: '#2563eb', magenta: '#7c3aed', cyan: '#0e7490', white: '#5c5c6b' };
}

function createTerminal() {
  term = new Terminal({
    fontFamily: "'Cascadia Mono', 'Cascadia Code', Consolas, monospace",
    fontSize: 13,
    lineHeight: 1.25,
    convertEol: true,
    cursorBlink: true,
    cursorStyle: 'bar',
    scrollback: 5000,
    allowProposedApi: true,
    allowTransparency: true,
    theme: terminalTheme(),
  });
  fit = new FitAddon();
  term.loadAddon(fit);
  term.open($('#terminal'));
  term.write('\x1b[?25l');
  flux.resize(term.cols, term.rows);
  greet();
  new ResizeObserver(() => {
    try {
      fit.fit();
    } catch {}
  }).observe($('#terminal'));

  // Písanie do bežiaceho programu (input()).
  term.onData((data) => {
    if (!state.running) return;
    if (state.pty) {
      // Skutočný terminál – klávesy idú priamo programu (Ctrl+C ho preruší).
      flux.input(data);
      return;
    }
    // Záložný režim bez pseudoterminálu: riadok sa odošle Enterom.
    for (const ch of data.replace(/\r\n/g, '\r')) {
      if (ch === '\r' || ch === '\n') {
        term.write('\r\n');
        flux.input(inputBuffer + '\n');
        inputBuffer = '';
      } else if (ch === '\x7f' || ch === '\b') {
        if (inputBuffer) {
          inputBuffer = [...inputBuffer].slice(0, -1).join('');
          term.write('\b \b');
        }
      } else if (ch === '\x03') {
        stop();
      } else if (ch >= ' ') {
        inputBuffer += ch;
        term.write(ch);
      }
    }
  });
  term.onResize(({ cols, rows }) => flux.resize(cols, rows));

  term.attachCustomKeyEventHandler((e) => {
    if (e.type === 'keydown' && e.ctrlKey && e.key.toLowerCase() === 'c' && term.hasSelection()) {
      navigator.clipboard.writeText(term.getSelection());
      return false;
    }
    return true;
  });

  // Klikateľné chyby: File "C:\...\main.py", line 12  /  C:\...\app.js:12:5
  term.registerLinkProvider({
    provideLinks(y, callback) {
      const line = term.buffer.active.getLine(y - 1)?.translateToString(true) || '';
      const links = [];
      const add = (start, text, path, lineNo, col) => {
        if (!inside(path)) return;
        links.push({
          range: { start: { x: start + 1, y }, end: { x: start + text.length, y } },
          text,
          decorations: { underline: true, pointerCursor: true },
          activate: () => openFile(path, { line: lineNo, column: col }),
        });
      };
      for (const m of line.matchAll(/File "([^"]+)", line (\d+)/g)) add(m.index, m[0], m[1], Number(m[2]));
      for (const m of line.matchAll(/((?:[A-Za-z]:\\|\/)[^\s:()'"]+\.\w+):(\d+)(?::(\d+))?/g)) add(m.index, m[0], m[1], Number(m[2]), m[3] ? Number(m[3]) : undefined);
      // Relatívne cesty (Java „Main.java:5“, Go, Rust „src/main.rs:3:5“, gcc…) – voči priečinku spusteného súboru.
      if (state.runDir)
        for (const m of line.matchAll(/(?<![\w\\/:.])((?:[\w.-]+[\\/])*[\w.-]+\.(?:java|go|rs|c|cc|cpp|h|hpp|cs|rb|php|lua|ts|pl|py|js|kt|dart)):(\d+)(?::(\d+))?/g)) {
          const p = join(state.runDir, m[1]);
          if (!links.some((l) => l.range.start.x === m.index + 1)) add(m.index, m[0], p, Number(m[2]), m[3] ? Number(m[3]) : undefined);
        }
      callback(links);
    },
  });
}

// ---------- terminál (príkazový riadok) vedľa výstupu ----------
let shTerm = null;
let shFit = null;
let shellAlive = false;
async function startShell(fresh = false) {
  if (!shTerm) {
    shTerm = new Terminal({
      fontFamily: "'Cascadia Mono', 'Cascadia Code', Consolas, monospace",
      fontSize: setting('terminalFontSize') || 13,
      lineHeight: 1.2,
      cursorBlink: true,
      cursorStyle: 'bar',
      scrollback: 5000,
      allowProposedApi: true,
      allowTransparency: true,
      theme: terminalTheme(),
    });
    shFit = new FitAddon();
    shTerm.loadAddon(shFit);
    shTerm.open($('#shell'));
    shTerm.onData((d) => flux.shellInput(d));
    shTerm.onResize(({ cols, rows }) => flux.shellResize(cols, rows));
    shTerm.attachCustomKeyEventHandler((e) => {
      if (e.type === 'keydown' && e.ctrlKey && e.key.toLowerCase() === 'c' && shTerm.hasSelection()) {
        navigator.clipboard.writeText(shTerm.getSelection());
        shTerm.clearSelection();
        return false;
      }
      if (e.type === 'keydown' && e.ctrlKey && e.key.toLowerCase() === 'v') {
        navigator.clipboard.readText().then((x) => x && flux.shellInput(x));
        return false;
      }
      return true;
    });
    flux.onShellData((d) => shTerm.write(d));
    flux.onShellExit(() => {
      shellAlive = false;
      shTerm.writeln(`\r\n\x1b[2m${t('The terminal has ended. Press Enter to start a new one.')}\x1b[0m`);
      const sub = shTerm.onData((d) => {
        if (d !== '\r') return;
        sub.dispose();
        startShell(true);
      });
    });
    new ResizeObserver(() => {
      if ($('#shell').hidden) return;
      try {
        shFit.fit();
      } catch {}
    }).observe($('#shell'));
  }
  try {
    shFit.fit();
  } catch {}
  flux.shellResize(shTerm.cols, shTerm.rows);
  if (!shellAlive || fresh) {
    if (fresh) shTerm.reset();
    shellAlive = await flux.shellStart(fresh);
    if (!shellAlive) shTerm.writeln(t('The terminal could not start on this computer.'));
  }
}
function showPanelTab(id) {
  state.panelTab = id;
  document.querySelectorAll('.panel-tab').forEach((b) => b.classList.toggle('on', b.dataset.ptab === id));
  $('#terminal').hidden = id !== 'output';
  $('#shell').hidden = id !== 'shell';
  $('#run-state').hidden = id !== 'output';
  $('#hint').hidden = id !== 'output';
  updatePanelVisibility();
  showPanel(true);
  if (id === 'shell') startShell().then(() => shTerm.focus());
  else requestAnimationFrame(() => fit.fit());
}

function greet() {
  term.writeln(`\x1b[2m${t('Program output appears here. Press')} \x1b[0m\x1b[1mF5\x1b[0m\x1b[2m ${t('or ▶ Run.')}\x1b[0m`);
}

function setHint(items) {
  const el = $('#hint');
  el.innerHTML = '';
  for (const item of items) {
    const b = document.createElement('button');
    b.className = 'hint';
    b.innerHTML = `${icon(item.icon || 'sparkle', 13)}<span></span>`;
    b.querySelector('span').textContent = item.label;
    b.onclick = item.run;
    el.append(b);
  }
}

const PIP_NAMES = {
  cv2: 'opencv-python', PIL: 'pillow', sklearn: 'scikit-learn', skimage: 'scikit-image', bs4: 'beautifulsoup4',
  yaml: 'pyyaml', dotenv: 'python-dotenv', serial: 'pyserial', usb: 'pyusb', win32api: 'pywin32', win32con: 'pywin32',
  win32gui: 'pywin32', attr: 'attrs', dateutil: 'python-dateutil', Crypto: 'pycryptodome', telegram: 'python-telegram-bot',
  discord: 'discord.py', docx: 'python-docx', pptx: 'python-pptx', fitz: 'pymupdf', jwt: 'PyJWT', OpenGL: 'PyOpenGL',
  magic: 'python-magic', gi: 'PyGObject', wx: 'wxPython', mpl_toolkits: 'matplotlib', google: 'google-api-python-client',
};

function analyzeOutput(code) {
  const out = state.runOutput;
  const hints = [];
  const missing = /ModuleNotFoundError: No module named '([^']+)'/.exec(out);
  if (missing && state.python) {
    const mod = missing[1].split('.')[0];
    const pkg = PIP_NAMES[mod] || mod;
    hints.push({ label: t('Install {pkg}', { pkg }), icon: 'download', run: () => pipInstall(pkg) });
  }
  if (code !== 0) {
    const frames = [...out.matchAll(/File "([^"]+)", line (\d+)/g)].filter((m) => inside(m[1]));
    const last = frames.pop();
    if (last) {
      hints.push({ label: t('Error: {file}, line {n}', { file: basename(last[1]), n: last[2] }), icon: 'chevron', run: () => openFile(last[1], { line: Number(last[2]) }) });
    }
  }
  setHint(hints);
}

// ---------- spúšťanie ----------
async function run() {
  const tab = activeTab();
  if (!tab) return toast(t('Open the file you want to run.'));
  const ext = extOf(tab.path);
  const lang = tab.model.getLanguageId();
  if (WEB.has(ext)) return openPreview(tab.path);
  if (!RUNNABLE.has(ext) && !(!ext && lang === 'python')) {
    return toast(ext ? t("Can't run .{ext} files yet. Try .py, .js or .html.", { ext }) : t('The file has no extension – rename it, e.g. to test.py.'));
  }
  if (!inside(tab.path)) return toast(t('This file is read-only.'));
  if (lang === 'python' && !state.python) {
    showPanel(true);
    setHint([
      { label: t('Download Python'), icon: 'download', run: () => flux.openExternal('https://www.python.org/downloads/') },
      { label: t('Choose python.exe…'), icon: 'python', run: choosePython },
      { label: t('Search again'), icon: 'refresh', run: detectPython },
    ]);
    return toast(t('Python not found. Install it from python.org (tick “Add python.exe to PATH”).'), 'error', 7000);
  }
  await saveAll();
  state.runDir = dirname(tab.path);
  const res = await flux.runFile(tab.path, state.python?.path, lang).catch((err) => ({ ok: false, error: errorText(err) }));
  // Jazyk nie je nainštalovaný → okno s veľkosťou a tlačidlom Inštalovať; po inštalácii hneď spustí.
  if (res.missing) {
    if (await tools.ask(res.missing, res.canInstall)) run();
    return;
  }
  if (!res.ok) toast(res.error || t('Could not run the file.'), 'error');
}

async function pipInstall(pkg) {
  if (!state.python) return;
  await flux.pipInstall(state.python.path, pkg);
}

function stop() {
  if (!state.running) return;
  state.stoppedByUser = true;
  flux.stop();
}

function onRunStart({ label, pty }) {
  state.running = true;
  if (!/^(pip install|python -m venv)/.test(label)) activity.run(state.workspace);
  updatePanelVisibility();
  state.pty = !!pty;
  state.runOutput = '';
  state.stoppedByUser = false;
  inputBuffer = '';
  if (state.panelTab === 'shell') showPanelTab('output');
  showPanel(true);
  setHint([]);
  if (setting('clearOnRun')) term.reset();
  else term.writeln('');
  term.write('\x1b[?25h');
  term.writeln(`\x1b[2m▶ ${label}\x1b[0m`);
  $('#run-state').className = 'run-state running';
  $('#run-state').textContent = t('Running…');
  renderRunButton();
}

function onRunData(text) {
  state.runOutput = (state.runOutput + text).slice(-20000);
  term.write(text);
  // Program pravdepodobne čaká na vstup (input bez nového riadku) → kurzor do terminálu.
  if (!text.endsWith('\n')) term.focus();
}

function onRunExit({ code, error, ms }) {
  state.running = false;
  term.write('\x1b[?25l');
  const secs = ms >= 1000 ? `${(ms / 1000).toFixed(1)} s` : `${ms} ms`;
  if (error) term.writeln(`\r\n\x1b[31m${error}\x1b[0m`);
  const status = $('#run-state');
  if (state.stoppedByUser) {
    term.writeln(`\r\n\x1b[2m── ${t('Stopped after {t}', { t: secs })} ──\x1b[0m`);
    status.className = 'run-state';
    status.textContent = t('Stopped');
  } else if (code === 0) {
    term.writeln(`\r\n\x1b[2m── ${t('Done in {t}', { t: secs })} ──\x1b[0m`);
    status.className = 'run-state ok';
    status.textContent = t('Done');
  } else {
    term.writeln(`\r\n\x1b[31m── ${t('Exited with an error (code {code})', { code: code ?? '?' })} ──\x1b[0m`);
    status.className = 'run-state fail';
    status.textContent = t('Error');
  }
  analyzeOutput(code);
  renderRunButton();
  // Po pip install alebo vytvorení venv znova zistiť Python.
  if (/^(pip install|python -m venv)/.test(state.lastLabel || '')) detectPython().then(() => ensureLsp(true));
}

// Tlačidlá sa ukazujú len keď dávajú zmysel: ▶ Spustiť pri Pythone/JS…, Live Server pri webe.
function fileKind(tab) {
  if (!tab) return 'none';
  const ext = extOf(tab.path);
  if (WEB.has(ext)) return 'web';
  if (ext === 'js' || ext === 'mjs') return 'script-web';
  if (RUNNABLE.has(ext) || (!ext && tab.model.getLanguageId() === 'python')) return 'script';
  return 'none';
}

// Výstup sa ukazuje len keď je otvorený súbor (alebo niečo beží) – na úvodnej obrazovke nie.
function updatePanelVisibility() {
  const show = !!state.active || state.running || state.panelTab === 'shell';
  $('#panel').hidden = !show;
  $('#panel-resizer').hidden = !show;
}

function renderRunButton() {
  updatePanelVisibility();
  const kind = fileKind(activeTab());
  const btn = $('#btn-run');
  const runnable = kind === 'script' || kind === 'script-web';
  btn.hidden = !runnable && !state.running;
  btn.innerHTML = `${icon('play', 14)}<span>${state.running ? t('Restart') : t('Run')}</span>`;
  btn.title = t('Run the current file (F5)');
  btn.classList.toggle('running', state.running);
  $('#btn-stop').hidden = !runnable && !state.running;
  $('#btn-stop').disabled = !state.running;
  const live = $('#btn-live');
  live.hidden = !(kind === 'web' || kind === 'script-web' || state.live);
  live.classList.toggle('primary', kind === 'web' && !state.live);
}

// ---------- panel ----------
function showPanel(show) {
  const panel = $('#panel');
  panel.classList.toggle('collapsed', !show);
  $('#btn-panel').innerHTML = icon('panel', 15);
  $('#btn-panel').title = show ? t('Hide panel (Ctrl+J)') : t('Show panel (Ctrl+J)');
  if (show)
    requestAnimationFrame(() => {
      if (state.panelTab === 'shell') shFit?.fit();
      else fit.fit();
    });
}

// ---------- Live Server a náhľad ----------
function liveUrlFor(path) {
  const rel = relative(path).split(/[\\/]/).map(encodeURIComponent).join('/');
  return `${state.live.url}/${rel}`;
}

async function startLive() {
  if (!state.workspace) {
    toast(t('Open a folder with your website first.'));
    return false;
  }
  try {
    state.live = await flux.liveStart();
  } catch (err) {
    toast(errorText(err), 'error');
    return false;
  }
  renderLive();
  return true;
}

async function openPreview(path) {
  if (!state.live && !(await startLive())) return;
  let url = `${state.live.url}/`;
  if (path && ['html', 'htm'].includes(extOf(path)) && inside(path)) url = liveUrlFor(path);
  else if ($('#preview-frame').src && !$('#preview').hidden) url = $('#preview-frame').src;
  await saveAll();
  $('#preview').hidden = false;
  $('#preview-resizer').hidden = false;
  if ($('#preview-frame').src !== url) $('#preview-frame').src = url;
  $('#preview-url').textContent = url.replace(/^http:\/\//, '');
  renderLive();
}

function followPreview(tab) {
  if ($('#preview').hidden || !state.live || !['html', 'htm'].includes(extOf(tab.path)) || !inside(tab.path)) return;
  const url = liveUrlFor(tab.path);
  if ($('#preview-frame').src !== url) {
    $('#preview-frame').src = url;
    $('#preview-url').textContent = url.replace(/^http:\/\//, '');
  }
}

function closePreview() {
  $('#preview').hidden = true;
  $('#preview-resizer').hidden = true;
  $('#preview-frame').removeAttribute('src');
}

async function stopLive() {
  if (state.live) await flux.liveStop();
  state.live = null;
  closePreview();
  renderLive();
}

// Live Server: ak beží a náhľad je zatvorený, len ho znova ukáže; inak zapne/vypne.
async function toggleLive() {
  if (state.live && $('#preview').hidden) {
    const tab = activeTab();
    await openPreview(tab && inside(tab.path) ? tab.path : null);
  } else if (state.live) {
    await stopLive();
  } else {
    const tab = activeTab();
    await openPreview(tab && inside(tab.path) ? tab.path : null);
  }
}

const renderRunButtonSoon = () => queueMicrotask(renderRunButton);

function renderLive() {
  const btn = $('#btn-live');
  btn.classList.toggle('on', !!state.live);
  btn.innerHTML = `${icon('globe', 14)}<span>${state.live ? `Live :${state.live.port}` : 'Live Server'}</span>`;
  renderRunButtonSoon();
  btn.title = state.live ? t('Stop Live Server (Alt+L)') : t('Start Live Server (Alt+L)');
  const st = $('#st-live');
  st.hidden = !state.live;
  if (state.live) {
    st.innerHTML = `${icon('globe', 13)}${state.live.url.replace('http://', '')}`;
    st.title = t('Open in browser');
  }
}

// ---------- náhľad: zariadenia, otočenie, snímka, mobil ----------
const DEVICES = { laptop: [1280, 800], tablet: [820, 1180], phone: [390, 844] };
const previewDevice = { id: 'full', landscape: false };
function layoutPreview() {
  const stage = document.querySelector('.preview-stage');
  const box = $('#pv-dev');
  const frame = $('#preview-frame');
  if (!stage || !box) return;
  const dev = DEVICES[previewDevice.id];
  $('#btn-preview-rotate').hidden = !dev;
  stage.classList.toggle('framed', !!dev);
  if (!dev) {
    box.style.cssText = '';
    frame.style.cssText = '';
    $('#pv-hint').textContent = '';
    return;
  }
  const [w, h] = previewDevice.landscape ? [dev[1], dev[0]] : dev;
  const sw = stage.clientWidth - 28;
  const sh = stage.clientHeight - 40;
  const k = Math.min(1, sw / w, sh / h);
  box.style.cssText = `width:${Math.round(w * k)}px;height:${Math.round(h * k)}px`;
  frame.style.cssText = `width:${w}px;height:${h}px;transform:scale(${k})`;
  $('#pv-hint').textContent = `${w} × ${h}${k < 1 ? ` · ${Math.round(k * 100)} %` : ''}`;
}

async function previewScreenshot() {
  const r = $('#preview-frame').getBoundingClientRect();
  const z = (Number(setting('uiZoom')) || 100) / 100;
  try {
    const res = await flux.liveScreenshot({ x: r.left * z, y: r.top * z, width: r.width * z, height: r.height * z });
    if (!res?.file) return;
    await refreshTree();
    const msg = res.copied ? t('Screenshot saved to {file} and copied.', { file: relative(res.file) }) : t('Screenshot saved to {file}.', { file: relative(res.file) });
    toast(msg, 'ok', 6000, { label: t('Open'), run: () => openFile(res.file) });
  } catch (err) {
    toast(errorText(err), 'error');
  }
}

// Karta „Otvoriť v mobile“: povolí prístup z Wi-Fi a ukáže QR kód.
async function togglePhonePopover() {
  const old = document.querySelector('.pv-pop');
  if (old) return old.remove();
  const pop = document.createElement('div');
  pop.className = 'pv-pop';
  document.body.append(pop);
  const place = () => {
    const r = $('#btn-preview-phone').getBoundingClientRect();
    pop.style.top = `${r.bottom + 8}px`;
    pop.style.left = `${Math.max(8, Math.min(innerWidth - 300, r.right - 290))}px`;
  };
  const draw = async () => {
    const lanUrl = state.live?.lanUrl;
    if (!state.live?.lan || !lanUrl) {
      pop.innerHTML = `<h4>${icon('phone', 15)}${t('Open on your phone')}</h4>
        <p>${t('Your phone must be on the same Wi-Fi. While this is on, anyone on that Wi-Fi can open the files of this project.')}</p>
        <button class="s-btn primary" data-lan-on>${t('Allow on my Wi-Fi')}</button>
        ${state.live?.lan && !lanUrl ? `<p class="pv-warn">${t('No Wi-Fi or network connection found.')}</p>` : ''}`;
    } else {
      const page = $('#preview-frame').src ? new URL($('#preview-frame').src).pathname : '/';
      const url = lanUrl + page;
      let qr = '';
      try {
        qr = await flux.liveQr(url);
      } catch {}
      pop.innerHTML = `<h4>${icon('phone', 15)}${t('Scan with your phone camera')}</h4>
        ${qr ? `<img src="${qr}" width="190" height="190" alt="QR">` : ''}
        <code>${escapeHtml(url)}</code>
        <p>${t('If it does not open, allow Flux in the Windows firewall message.')}</p>
        <button class="s-btn" data-lan-off>${t('Stop sharing on Wi-Fi')}</button>`;
    }
    place();
  };
  pop.onclick = async (e) => {
    e.stopPropagation();
    const on = e.target.closest('[data-lan-on]');
    const off = e.target.closest('[data-lan-off]');
    if (!on && !off) return;
    try {
      const info = await flux.liveLan(!!on);
      if (info) {
        state.live = info;
        const f = $('#preview-frame');
        if (f.src) f.src = f.src.replace(/:\d+\//, `:${info.port}/`);
        renderLive();
      }
    } catch (err) {
      toast(errorText(err), 'error');
    }
    draw();
  };
  const close = (e) => {
    if (!pop.contains(e.target) && !e.target.closest('#btn-preview-phone')) {
      pop.remove();
      document.removeEventListener('pointerdown', close, true);
    }
  };
  document.addEventListener('pointerdown', close, true);
  await draw();
}

function onLiveLog({ level, text }) {
  const color = { error: '31', warn: '33', info: '36', log: '36' }[level] || '36';
  term.writeln(`\x1b[${color}m[web]\x1b[0m ${text}`);
  if (level === 'error') showPanel(true);
}

// ---------- paleta príkazov / dialógy ----------
let palette = null;

// Vyhľadávanie bez ohľadu na diakritiku a veľkosť písmen („tema“ nájde „tému“).
const fold = (s) => s.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();

function fuzzy(query, text, keywords = '') {
  if (!query) return { score: 0, marks: [] };
  const q = fold(query);
  const t = fold(text);
  const direct = t.indexOf(q);
  if (direct >= 0) return { score: 1000 - direct - t.length * 0.01, marks: [...Array(q.length).keys()].map((i) => i + direct) };
  if (keywords && fold(keywords).includes(q)) return { score: 500, marks: [] };
  let ti = 0;
  const marks = [];
  for (const ch of q) {
    ti = t.indexOf(ch, ti);
    if (ti < 0) return null;
    marks.push(ti++);
  }
  return { score: 100 - (marks[marks.length - 1] - marks[0]) - t.length * 0.01, marks };
}

function highlight(text, marks) {
  const set = new Set(marks);
  return [...text].map((c, i) => (set.has(i) ? `<b>${escapeHtml(c)}</b>` : escapeHtml(c))).join('');
}

function openPalette({ items = null, placeholder = '', value = '', note = '', select = null, onPick, onCancel, onSelect }) {
  closePalette();
  const overlay = $('#overlay');
  const input = $('#palette-input');
  const list = $('#palette-list');
  overlay.hidden = false;
  input.value = value;
  input.placeholder = placeholder;
  let noteEl = $('#palette .p-note');
  if (noteEl) noteEl.remove();
  if (note) {
    noteEl = document.createElement('div');
    noteEl.className = 'p-note';
    noteEl.textContent = note;
    $('#palette').insertBefore(noteEl, list);
  }
  palette = { items, onPick, onCancel, onSelect, sel: 0, shown: [], lastSel: null };
  const render = () => {
    if (!items) {
      list.innerHTML = '';
      return;
    }
    const q = input.value.trim();
    palette.shown = items
      .map((it) => ({ it, m: fuzzy(q, it.label, it.keywords) }))
      .filter((x) => x.m)
      .sort((a, b) => b.m.score - a.m.score)
      .slice(0, 80);
    if (!q) palette.shown.sort((a, b) => items.indexOf(a.it) - items.indexOf(b.it));
    palette.sel = Math.min(palette.sel, Math.max(0, palette.shown.length - 1));
    list.innerHTML = palette.shown.length
      ? palette.shown
          .map(
            ({ it, m }, i) =>
              `<div class="p-item${i === palette.sel ? ' sel' : ''}" data-i="${i}">${it.icon || ''}<span class="label">${highlight(it.label, m.marks)}</span>` +
              `${it.kbd ? `<kbd>${it.kbd}</kbd>` : it.detail ? `<span class="detail">${escapeHtml(it.detail)}</span>` : ''}</div>`,
          )
          .join('')
      : `<div class="p-empty">${t('Nothing found')}</div>`;
    list.querySelector('.sel')?.scrollIntoView({ block: 'nearest' });
    const current = palette.shown[palette.sel]?.it;
    if (current && current !== palette.lastSel) {
      palette.lastSel = current;
      palette.onSelect?.(current);
    }
  };
  palette.render = render;
  input.oninput = () => {
    palette.sel = 0;
    render();
  };
  list.onmousemove = (e) => {
    const row = e.target.closest('.p-item');
    if (row && Number(row.dataset.i) !== palette.sel) {
      palette.sel = Number(row.dataset.i);
      render();
    }
  };
  list.onclick = (e) => {
    const row = e.target.closest('.p-item');
    if (row) pick(Number(row.dataset.i));
  };
  render();
  input.focus();
  if (select) input.setSelectionRange(select[0], select[1]);
  else input.select();
}

function pick(i) {
  if (!palette) return;
  const p = palette;
  const value = $('#palette-input').value;
  const chosen = p.items ? p.shown[i]?.it : value;
  if (p.items && !chosen) return;
  closePalette(true);
  p.onPick?.(chosen);
}

function closePalette(picked = false) {
  if (!palette) return;
  const p = palette;
  palette = null;
  $('#overlay').hidden = true;
  if (!picked) p.onCancel?.();
  if (state.active) editor.focus();
}

function promptPalette(opts) {
  return new Promise((resolve) => openPalette({ ...opts, items: null, onPick: (v) => resolve(v.trim() || null), onCancel: () => resolve(null) }));
}

function confirmPalette(note, options) {
  return new Promise((resolve) =>
    openPalette({ note, placeholder: t('Choose an option…'), items: options, onPick: (o) => resolve(o.value), onCancel: () => resolve(null) }),
  );
}

async function quickOpen() {
  if (!state.workspace) return openFolderDialog();
  const files = await flux.listAll();
  openPalette({
    placeholder: t('Search files by name…'),
    items: files.map((f) => ({ label: relative(f), icon: fileIcon(basename(f)), path: f })),
    onPick: (it) => openFile(it.path),
  });
}

// Aktuálna skratka vstavanej akcie (aj keď si ju zmenil v Nastaveniach → Skratky).
const keymapKey = (id) => {
  const b = BINDINGS.find((x) => x.id === id);
  return b ? keymap?.current(b) || '' : '';
};

// ---------- náhľad Markdownu (.md) vedľa editora ----------
let mdTimer = 0;
function renderMdPreview() {
  const tab = activeTab();
  const box = $('#mdview');
  if (!box || box.hidden) return;
  if (!tab || !['md', 'markdown'].includes(extOf(tab.path))) return closeMdPreview();
  $('#mdview-name').textContent = basename(tab.path);
  $('#mdview-body').innerHTML = markdown(tab.model.getValue(), { preview: true });
}
function toggleMdPreview(show) {
  const tab = activeTab();
  const box = $('#mdview');
  const want = show ?? box.hidden;
  if (want && (!tab || !['md', 'markdown'].includes(extOf(tab.path)))) return toast(t('Open a Markdown (.md) file first.'), 'info');
  box.hidden = !want;
  $('#md-resizer').hidden = !want;
  document.body.classList.toggle('md-open', want);
  if (want) renderMdPreview();
}
// Tlačidlo „Náhľad“ v rohu editora pri .md súboroch.
function renderMdButton(tab = activeTab()) {
  const b = $('#md-toggle');
  if (b) b.hidden = !tab || !['md', 'markdown'].includes(extOf(tab.path));
}
function closeMdPreview() {
  toggleMdPreview(false);
}
function scheduleMdPreview() {
  clearTimeout(mdTimer);
  mdTimer = setTimeout(renderMdPreview, 150);
}

function commands() {
  const c = (label, run, kbd, ic, keywords = '') => ({ label, run, kbd, keywords, icon: ic ? icon(ic, 15) : '' });
  const list = [
    c(t('Run current file'), run, 'F5', 'play'),
    c(t('Stop program'), stop, 'Shift+F5', 'stop'),
    c(state.live ? t('Live Server: stop') : t('Live Server: start'), toggleLive, 'Alt+L', 'globe', 'web preview html browser'),
    c(t('Open folder…'), openFolderDialog, 'Ctrl+O', 'folderOpen'),
    c(t('Open file…'), openFileDialog, keymapKey('openFile'), 'file', 'markdown md txt single file'),
    c(t('Markdown: show preview'), () => toggleMdPreview(), keymapKey('mdPreview'), 'file', 'md readme'),
    c(t('New project…'), newProject, 'Ctrl+Shift+N', 'plus', 'project folder'),
    c(t('Quick open file…'), quickOpen, 'Ctrl+P', 'filePlus'),
    c(t('New file…'), () => newFile(), 'Ctrl+N', 'filePlus'),
    c(t('New folder…'), () => newFolder(), '', 'folderPlus'),
    c(t('Save'), () => saveTab(activeTab()), 'Ctrl+S', 'save'),
    c(t('Save all'), saveAll, '', 'save'),
    c(t('Close file'), () => state.active && closeTab(state.active), 'Ctrl+W', 'x'),
    c(t('Format document'), () => editor.getAction('editor.action.formatDocument')?.run(), 'Shift+Alt+F', 'sparkle'),
    c(t('Toggle sidebar (compact mode)'), toggleCompact, 'Ctrl+B', 'sidebar'),
    c(t('Focus mode (only the code)'), toggleFocus, 'F11', 'monitor', 'zen fullscreen distraction free'),
    c(t('Toggle output panel'), () => showPanel($('#panel').classList.contains('collapsed')), 'Ctrl+J', 'panel'),
    c(t('Toggle light / dark theme'), toggleTheme, '', isDark() ? 'sun' : 'moon', 'theme dark light colors appearance'),
    c(t('Color theme…'), chooseTheme, '', 'palette', 'theme colors vs code dracula one dark'),
    c(t('Settings'), openSettings, 'Ctrl+,', 'settings', 'settings preferences font'),
    c(t('Search everything…'), () => searchEverything(), 'Ctrl+Shift+A', 'command', 'search find all settings'),
    ghOn() && c(t('GitHub: open a repository…'), () => gh.pickRepo(), '', 'github', 'github clone repo'),
    c(t('AI: open assistant'), () => aiPanel.show(), 'Ctrl+I', 'sparkle', 'ai claude chat assistant'),
    c(t('AI: explain this file'), () => aiPanel.ask(t('Explain this file.')), '', 'sparkle', 'ai claude explain'),
    c(t('AI: fix the errors in this file'), () => aiPanel.ask(t('Find and fix the errors in this file. Show the corrected code.')), '', 'sparkle', 'ai claude fix bug error'),
    c(t('New file from template…'), () => newFile(), 'Ctrl+N', 'template', 'template html python web project'),
    c(setting('autosave') ? t('Turn off auto save') : t('Turn on auto save'), toggleAutosave, '', 'save'),
    c(t('Python: select interpreter…'), choosePython, '', 'python'),
    c(t('Python: detect interpreter automatically'), async () => {
      await flux.resetPython();
      await detectPython();
      lsp.start(state.workspace, state.python?.path);
    }, '', 'python'),
    c(t('Python: create virtual environment (.venv)'), createVenv, '', 'python'),
    c(t('Python: install package (pip)…'), async () => {
      const pkg = await promptPalette({ placeholder: t('e.g. requests, numpy, pygame'), note: 'pip install' });
      if (pkg) pipInstall(pkg);
    }, '', 'download'),
    c(t('Python: restart autocomplete'), () => lsp.start(state.workspace, state.python?.path), '', 'refresh'),
    c(t('Replay the intro'), () => onboarding.open(), '', 'sparkle', 'onboarding welcome intro'),
    c(t('Feature tour'), () => onboarding.startTour(), '', 'sparkle', 'tour help'),
    c(t('Show problems in file'), showProblems, '', 'x', 'problems errors'),
    c(t('Increase font size'), () => setFontSize(1), 'Ctrl+=', ''),
    c(t('Decrease font size'), () => setFontSize(-1), 'Ctrl+-', ''),
  ];
  return list.filter(Boolean);
}

function pluginCommands() {
  return (pluginHost?.commands() || []).map((c) => ({ label: `${c.plugin}: ${c.label}`, run: c.run, kbd: c.key, icon: icon('puzzle', 15), keywords: 'plugin' }));
}

function openCommandPalette() {
  openPalette({ placeholder: t('Type a command…'), items: [...commands(), ...pluginCommands()], onPick: (it) => it.run() });
}

function workspaceSwitcher() {
  const items = [
    { label: t('Open folder…'), icon: icon('folderOpen', 15), kbd: 'Ctrl+O', run: openFolderDialog },
    ...(state.settings.recent || [])
      .filter((d) => d !== state.workspace)
      .map((d) => ({ label: basename(d), detail: d, icon: icon('folder', 15), run: () => setWorkspace(d) })),
  ];
  openPalette({ placeholder: t('Switch to folder…'), items, onPick: (it) => it.run() });
}

function pythonMenu() {
  const items = commands().filter((c) => c.label.startsWith('Python'));
  openPalette({
    placeholder: state.python ? `${state.python.path}` : t('Python not found'),
    items: state.python ? items : [{ label: t('Download Python from python.org'), icon: icon('download', 15), run: () => flux.openExternal('https://www.python.org/downloads/') }, ...items],
    onPick: (it) => it.run(),
  });
}

async function choosePython() {
  try {
    const info = await flux.choosePython();
    if (!info) return;
    state.python = info;
    renderStatus();
    lsp.start(state.workspace, info.path);
    toast(t('Using Python {v}', { v: info.version }));
  } catch (err) {
    toast(errorText(err), 'error');
  }
}

async function createVenv() {
  if (!state.python) return toast(t('Python needs to be installed first.'), 'error');
  if (!state.workspace) return toast(t('Open a folder first.'), 'error');
  state.lastLabel = 'python -m venv';
  await flux.createVenv(state.python.path);
}

// ---------- nastavenia ----------
async function toggleTheme() {
  // Prepne medzi naposledy použitou tmavou a svetlou témou kódu.
  // Ak si zapamätaná téma medzitým zmizla alebo zmenila typ (vlastná téma), použije sa predvolená.
  const want = isDark() ? 'light' : 'dark';
  let next = want === 'light' ? setting('lastLight') : setting('lastDark');
  if (!THEMES[next] || THEMES[next].type !== want) next = want === 'light' ? 'vscode-light' : DEFAULT_THEME;
  await setCodeTheme(next);
}

async function toggleAutosave() {
  await saveSettings({ autosave: !setting('autosave') });
  renderStatus();
  toast(setting('autosave') ? t('Auto save is on.') : t('Auto save is off – save with Ctrl+S.'));
}

async function setFontSize(delta) {
  const size = Math.min(28, Math.max(10, setting('fontSize') + delta));
  editor.updateOptions({ fontSize: size });
  await saveSettings({ fontSize: size });
}

// Režim sústredenia: len kód na celej obrazovke (bez bočného panela, výstupu a stavového riadku).
async function toggleFocus() {
  const on = !document.body.classList.contains('focus-mode');
  document.body.classList.toggle('focus-mode', on);
  document.body.classList.toggle('compact', on || !!setting('compact'));
  document.body.classList.remove('peek');
  flux.setFullScreen?.(on);
  if (on) toast(t('Focus mode – press F11 to go back.'), 'info', 2500);
  requestAnimationFrame(() => editor?.layout());
}

async function toggleCompact() {
  const compact = !document.body.classList.contains('compact');
  document.body.classList.toggle('compact', compact);
  document.body.classList.remove('peek');
  await saveSettings({ compact });
}

// ---------- vlastné farebné témy (config/themes/*.json) ----------
const THEME_KEYS = ['fg', 'comment', 'keyword', 'storage', 'string', 'number', 'type', 'function', 'variable', 'parameter', 'property', 'constant', 'tag', 'attr', 'delimiter', 'regexp'];
let customThemeFiles = {};

// Téma zo súboru JSON → objekt témy (farby bez #).
function parseTheme(data, fallbackName) {
  const type = data.type === 'light' ? 'light' : 'dark';
  const base = THEMES[data.basedOn] || THEMES[type === 'light' ? 'vscode-light' : 'vscode-dark'];
  const colors = {};
  for (const [k, v] of Object.entries(data.colors || {})) if (typeof v === 'string' && /^#?([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(v)) colors[k] = v.replace('#', '').toUpperCase();
  return { name: data.name || fallbackName, type, custom: true, t: { ...base.t, ...colors, italicComments: data.italicComments ?? base.t.italicComments } };
}

// Téma z pluginu (flux.themes.add). Ak je práve vybraná, hneď sa použije.
function addPluginTheme(id, def) {
  const th = parseTheme({ name: def.name, type: def.type, basedOn: def.basedOn, colors: def.colors || {}, italicComments: def.italicComments }, id);
  THEMES[id] = { ...th, custom: false, plugin: def.plugin };
  if (setting('codeTheme') === id) applyTheme();
  return () => {
    delete THEMES[id];
    if (setting('codeTheme') === id) setCodeTheme(isDark() ? DEFAULT_THEME : 'vscode-light');
  };
}

async function loadCustomThemes(announce = false) {
  let files = [];
  try {
    files = await flux.customThemes();
  } catch {}
  for (const id of Object.keys(THEMES)) if (id.startsWith('custom-')) delete THEMES[id];
  customThemeFiles = {};
  for (const f of files) {
    try {
      const id = `custom-${f.id}`;
      THEMES[id] = parseTheme(JSON.parse(f.text), f.id);
      customThemeFiles[id] = f.path;
    } catch (err) {
      if (announce) toast(t('Theme {file} has an error: {msg}', { file: `${f.id}.json`, msg: err.message }), 'error', 8000);
    }
  }
  if (announce) {
    if (setting('codeTheme')?.startsWith('custom-')) applyTheme();
    toast(t('Themes reloaded.'), 'ok');
  }
}

// Advanced: JSON témy v editore – farby sa menia už počas písania (bez uloženia).
let themeLiveTimer = null;
function liveThemeFromText(file, text) {
  const id = Object.keys(customThemeFiles).find((k) => keyOf(customThemeFiles[k]) === keyOf(file));
  if (!id) return;
  clearTimeout(themeLiveTimer);
  themeLiveTimer = setTimeout(() => {
    let data;
    try {
      data = JSON.parse(text);
    } catch {
      return; // rozpísaný JSON – počkáme na ďalšiu zmenu
    }
    const th = parseTheme(data, THEMES[id]?.name || id);
    const typeChanged = THEMES[id]?.type !== th.type;
    THEMES[id] = th;
    if (setting('codeTheme') !== id || typeChanged) return setCodeTheme(id);
    monaco.editor.setTheme(defineMonacoTheme(monaco, id, currentAccent()));
    codemap?.refresh();
  }, 200);
}

// Nová téma = kópia aktuálnej; hneď sa otvorí štúdio s farbami.
async function createCustomTheme() {
  const cur = themeOf(setting('codeTheme'));
  const colors = {};
  for (const k of THEME_KEYS) if (cur.t[k]) colors[k] = `#${cur.t[k]}`;
  const file = await flux.newTheme(t('My theme'), cur.type, colors);
  await loadCustomThemes();
  const id = Object.keys(customThemeFiles).find((k) => customThemeFiles[k] === file);
  if (!id) return;
  await openThemeStudio(id);
}

let themeStudio = null;
async function openThemeStudio(id) {
  closeSettings();
  if (setting('codeTheme') !== id) await setCodeTheme(id);
  themeStudio ||= createThemeStudio({
    monaco,
    THEMES,
    builtIns: () => Object.entries(THEMES).filter(([bid]) => !bid.startsWith('custom-')),
    getFile: (tid) => customThemeFiles[tid],
    apply: (tid) => {
      monaco.editor.setTheme(defineMonacoTheme(monaco, tid, currentAccent()));
      codemap?.refresh();
    },
    setType: (tid) => setCodeTheme(tid),
    openAdvanced: async (tid) => {
      await openFile(customThemeFiles[tid]);
      toast(t('Colors change while you type. Ctrl+S saves the theme.'), 'info', 6000);
    },
    remove: async (tid) => {
      if (!(await flux.trash(customThemeFiles[tid]))) return false;
      const type = THEMES[tid]?.type;
      await loadCustomThemes();
      await setCodeTheme(type === 'light' ? 'vscode-light' : 'flux');
      return true;
    },
  });
  themeStudio.open(id);
}

async function setCodeTheme(id) {
  const t = themeOf(id);
  await saveSettings({ codeTheme: id, theme: t.type, [t.type === 'dark' ? 'lastDark' : 'lastLight']: id });
  applyTheme();
}

// Výber témy kódu – pri prechádzaní šípkami sa téma hneď ukazuje.
function chooseTheme() {
  const original = setting('codeTheme');
  const items = Object.entries(THEMES).map(([id, th]) => ({
    id,
    label: th.name,
    detail: th.type === 'dark' ? t('dark') : t('light'),
    icon: `<span class="swatch">${themeSwatch(id).map((c) => `<i style="background:${c}"></i>`).join('')}</span>`,
  }));
  openPalette({
    placeholder: t('Color theme…'),
    items,
    onSelect: (it) => monaco.editor.setTheme(defineMonacoTheme(monaco, it.id, currentAccent())),
    onPick: (it) => setCodeTheme(it.id),
    onCancel: () => monaco.editor.setTheme(defineMonacoTheme(monaco, original, currentAccent())),
  });
}

// ---------- nastavenia ----------
function openSettings() {
  const panel = $('#settings');
  const accent = currentAccent();
  const opt = (value, label, current) => `<option value="${escapeAttr(String(value))}"${String(value) === String(current) ? ' selected' : ''}>${escapeHtml(label)}</option>`;
  const toggle = (key, label, hint = '') =>
    `<label class="s-row"><span><b>${t(label)}</b>${hint ? `<small>${t(hint)}</small>` : ''}</span><input type="checkbox" class="switch" data-key="${key}"${setting(key) ? ' checked' : ''}></label>`;
  const materials = [
    state.hasWallpaper && ['wallpaper', t('Wallpaper (recommended)')],
    state.mica && ['acrylic', 'Acrylic (Windows)'],
    state.mica && ['mica', 'Mica (Windows)'],
    ['none', t('Off')],
  ].filter(Boolean);
  const tabs = [
    ['general', 'settings', t('General')],
    ['appearance', 'palette', t('Appearance')],
    ['editor', 'code', t('Editor')],
    ['running', 'play', t('Running')],
    ['tools', 'download', t('Languages')],
    ['plugins', 'puzzle', t('Plugins')],
    ['ai', 'sparkle', t('AI')],
    ghOn() && ['github', 'github', 'GitHub'],
  ].filter(Boolean);
  if (state.settingsTab === 'custom') state.settingsTab = 'appearance';
  // Skratky a aktualizácie sú teraz časti Všeobecných – staré odkazy skočia na ne.
  const jumpTo = { keys: 'g-keys', about: 'g-updates' }[state.settingsTab];
  if (jumpTo) state.settingsTab = 'general';
  const tab = tabs.some(([id]) => id === state.settingsTab) ? state.settingsTab : 'general';
  panel.innerHTML = `
    <div class="s-card" role="dialog" aria-label="${t('Settings')}">
      <nav class="s-nav">
        <div class="s-nav-title">${t('Settings')}</div>
        <label class="s-find">${icon('search', 14)}<input id="s-find" placeholder="${t('Search settings…')}" spellcheck="false" autocomplete="off"></label>
        ${tabs.map(([id, ic, label]) => `<button class="s-tab${id === tab ? ' on' : ''}" data-tab="${id}">${icon(ic, 16)}<span>${label}</span></button>`).join('')}
        <div class="grow"></div>
        <div class="s-ver">Flux ${escapeHtml(state.version || '')}</div>
      </nav>
      <div class="s-main">
        <header class="s-head">${navButtons()}<h2 id="s-title"></h2><button class="icon-btn s-close" data-close title="${t('Close (Esc)')}">${icon('x', 16)}</button></header>
        <div class="s-body">
          <section data-pane="appearance">
            <h3>${t('Code theme')}</h3>
            <div class="theme-grid">${Object.entries(THEMES)
              .map(
                ([id, th]) =>
                  `<button class="theme-card${id === setting('codeTheme') ? ' active' : ''}" data-theme="${id}"><span class="swatch">${themeSwatch(id)
                    .map((c) => `<i style="background:${c}"></i>`)
                    .join('')}</span><span>${escapeHtml(th.name)}</span><small>${th.custom ? t('your theme') : th.type === 'dark' ? t('dark') : t('light')}</small>${th.custom ? `<span class="tc-edit" data-edit-theme="${id}" title="${t('Edit')}">${icon('edit', 12)}</span>` : ''}</button>`,
              )
              .join('')}<button class="theme-card theme-new" data-action="new-theme">${icon('plus', 16)}<span>${t('Create your own')}</span><small>${t('from the current theme')}</small></button></div>
            <h3>${t('Accent color')}${state.workspace ? ` <small>${t('for folder {name}', { name: escapeHtml(basename(state.workspace)) })}</small>` : ''}</h3>
            <div class="s-group"><div class="accent-grid">${Object.keys(ACCENTS)
              .map((name) => `<button data-accent="${name}" style="--c:${accentHex(name)}" class="${accentHex(name) === accent ? 'active' : ''}" title="${name === 'mono' ? t('black & white (like Zen)') : name}"></button>`)
              .join('')}<label class="custom-color" title="${t('Custom color')}"><input type="color" value="${accent}" data-custom-accent></label></div></div>
            <h3>${t('App colors')}</h3>
            <div class="s-group">
              ${[
                ['uiText', t('Text'), t('menus, file names and buttons')],
                ['uiText2', t('Secondary text'), t('hints and small text')],
                ['uiBase', t('Background'), t('behind the panels')],
                ['uiCard', t('Panels'), t('editor, sidebar and windows')],
                ['uiLine', t('Borders'), t('lines between parts')],
              ]
                .map(
                  ([k, label, hint]) =>
                    `<label class="s-row"><span><b>${label}</b><small>${hint}${setting(k) ? '' : ` · ${t('from the current theme')}`}</small></span><span class="s-inline"><input type="color" data-key="${k}" value="${setting(k) || getComputedStyle(document.body).getPropertyValue({ uiText: '--text', uiText2: '--text-2', uiBase: '--base', uiCard: '--card-solid', uiLine: '--line-strong' }[k]).trim().replace(/^rgba?\(.*$/, '#888888') || '#888888'}"><button class="s-btn" data-color-reset="${k}">${t('Reset')}</button></span></label>`,
                )
                .join('')}
              <label class="s-row"><span><b>${t('Panel transparency')}</b><small>${t('how much the background shines through the panels')}</small></span><input type="range" min="30" max="100" data-key="cardAlpha" value="${setting('cardAlpha')}"></label>
              <div class="s-row"><span><b>${t('Reset all colors')}</b><small>${t('back to the colors of your theme')}</small></span><button class="s-btn" data-action="colors-reset">${icon('refresh', 13)}${t('Reset')}</button></div>
            </div>
            <h3>${t('Text cursor')}</h3>
            <div class="s-group">
              <label class="s-row"><span><b>${t('Cursor shape')}</b></span><select data-key="caretStyle">${[['line', t('line')], ['line-thin', t('thin line')], ['block', t('block')], ['block-outline', t('block outline')], ['underline', t('underline')], ['underline-thin', t('thin underline')]].map(([v, l]) => opt(v, l, setting('caretStyle'))).join('')}</select></label>
              <label class="s-row"><span><b>${t('Blinking')}</b></span><select data-key="caretBlink">${[['smooth', t('smooth')], ['blink', t('blink')], ['phase', t('fade')], ['expand', t('expand')], ['solid', t('no blinking')]].map(([v, l]) => opt(v, l, setting('caretBlink'))).join('')}</select></label>
              <label class="s-row"><span><b>${t('Cursor width')}</b><small>${t('for the line shape')}</small></span><input type="range" min="1" max="6" data-key="caretWidth" value="${setting('caretWidth')}"></label>
              ${toggle('caretSmooth', 'Smooth cursor movement', 'the cursor glides when it moves')}
              <label class="s-row"><span><b>${t('Cursor color')}</b><small>${t('empty = color of the theme')}</small></span><span class="s-inline"><input type="color" data-key="caretColor" value="${setting('caretColor') || accent}"><button class="s-btn" data-action="caret-reset">${t('Reset')}</button></span></label>
            </div>
            <h3>${t('Mouse pointer in the editor')}</h3>
            <div class="s-group">
              <div class="s-row"><span><b>${t('Pointer')}</b><small>${t('Crosshair and dot use your pointer color.')}</small></span><span class="ptr-pick">${[['text', 'I'], ['default', '↖'], ['crosshair', '+'], ['flux-cross', '⌖'], ['flux-dot', '●'], ['flux-ring', '◎']]
                .map(([v, l]) => `<button class="ptr${setting('pointer') === v ? ' on' : ''}" data-pointer="${v}" title="${v}">${l}</button>`)
                .join('')}${setting('pointerImage') ? `<button class="ptr ptr-img${setting('pointer') === 'custom' ? ' on' : ''}" data-pointer="custom" title="${t('Your own cursor')}"><img src="${escapeAttr(setting('pointerImage'))}" alt=""></button>` : ''}</span></div>
              <div class="s-row"><span><b>${t('Your own cursor')}</b><small>${t('PNG, SVG, CUR or ICO – bigger pictures are made smaller (max 64 px).')}</small></span><span class="s-inline"><button class="s-btn" data-action="cursor-pick">${icon('download', 13)}${t('Upload…')}</button>${setting('pointerImage') ? `<select data-key="pointerHotspot" title="${t('Where the click happens')}">${opt('tip', t('click at top-left'), setting('pointerHotspot'))}${opt('center', t('click in the middle'), setting('pointerHotspot'))}</select>` : ''}</span></div>
              <label class="s-row"><span><b>${t('Pointer color')}</b></span><input type="color" data-key="pointerColor" value="${setting('pointerColor') || '#ffffff'}"></label>
              ${toggle('pointerEverywhere', 'Use it in the whole app', 'not only over the code')}
              <div class="s-row"><span><b>${t('Back to the normal cursor')}</b></span><button class="s-btn" data-action="pointer-reset">${icon('refresh', 13)}${t('Reset')}</button></div>
            </div>
            <h3>${t('Text & fonts')}</h3>
            <div class="s-group">
              <label class="s-row"><span><b>${t('Custom code font')}</b><small>${t('any font installed on your computer, e.g. Hack or Iosevka')}</small></span><input class="s-text" data-key="fontCustom" value="${escapeAttr(setting('fontCustom'))}" placeholder="${t('font name')}"></label>
              <label class="s-row"><span><b>${t('App font')}</b><small>${t('font of menus and buttons')}</small></span><input class="s-text" data-key="uiFont" value="${escapeAttr(setting('uiFont'))}" placeholder="Segoe UI"></label>
              <label class="s-row"><span><b>${t('Letter spacing')}</b></span><input type="range" min="-1" max="3" step="0.5" data-key="letterSpacing" value="${setting('letterSpacing')}"></label>
              <label class="s-row"><span><b>${t('Line numbers')}</b></span><select data-key="lineNumbers">${[['on', t('on')], ['relative', t('relative')], ['off', t('off')]].map(([v, l]) => opt(v, l, setting('lineNumbers'))).join('')}</select></label>
              <label class="s-row"><span><b>${t('Show spaces and tabs')}</b></span><select data-key="whitespace">${[['none', t('never')], ['selection', t('in selection')], ['boundary', t('between words')], ['all', t('always')]].map(([v, l]) => opt(v, l, setting('whitespace'))).join('')}</select></label>
              ${toggle('bracketColors', 'Colored brackets', 'matching brackets get the same color')}
            </div>
            <h3>${t('Window')}</h3>
            <div class="s-group">
              ${state.platform === 'win32' ? `<label class="s-row"><span><b>${t('Window translucency')}</b><small>${t('“Wallpaper” stays translucent even when the window is not active. With Acrylic/Mica, Windows turns the window grey when inactive.')}</small></span><select data-key="material">${materials.map(([v, l]) => opt(v, l, state.material)).join('')}</select></label>` : ''}
              ${toggle('menuBar', 'Menu bar', 'File, Edit, View, Run and Help as a row at the top – otherwise they open from the flux logo')}
              ${toggle('showSearch', 'Search button', 'a magnifier at the top that finds files, commands and settings')}
              <label class="s-row"><span><b>${t('Panel position')}</b><small>${t('where output and the terminal are')}</small></span><select data-key="panelPos">${[['bottom', t('Bottom')], ['right', t('Right')], ['left', t('Left')]].map(([v, l]) => opt(v, l, setting('panelPos'))).join('')}</select></label>
              <label class="s-row"><span><b>${t('Sidebar position')}</b><small>${t('projects and files')}</small></span><select data-key="sidePos">${[['left', t('Left')], ['right', t('Right')]].map(([v, l]) => opt(v, l, setting('sidePos'))).join('')}</select></label>
              <label class="s-row"><span><b>${t('Size of everything')}</b><small id="s-zoom-v">${setting('uiZoom')} %</small></span><input type="range" min="80" max="140" step="5" data-key="uiZoom" value="${setting('uiZoom')}"></label>
              <label class="s-row"><span><b>${t('Density')}</b><small>${t('Compact fits more files, tabs and lines on the screen.')}</small></span><select data-key="density">${opt('comfortable', t('comfortable'), setting('density'))}${opt('compact', t('compact'), setting('density'))}</select></label>
              <label class="s-row"><span><b>${t('Rounded corners')}</b></span><input type="range" min="0" max="26" data-key="cornerRadius" value="${setting('cornerRadius')}"></label>
              <label class="s-row"><span><b>${t('Background blur')}</b></span><input type="range" min="0" max="100" data-key="wallBlur" value="${setting('wallBlur')}"></label>
              <label class="s-row"><span><b>${t('Background strength')}</b></span><input type="range" min="10" max="100" data-key="wallOpacity" value="${setting('wallOpacity')}"></label>
              <label class="s-row"><span><b>${t('Brightness of dark areas')}</b><small>${t('Only backgrounds get lighter – text and outlines stay the same. Turn it up if your wallpaper is very dark.')}</small></span><input type="range" min="0" max="100" data-key="darkLift" value="${setting('darkLift')}"></label>
              <div class="s-row"><span><b>${t('Background image')}</b><small>${t('your own picture instead of the Windows wallpaper')}</small></span><span class="s-inline"><button class="s-btn" data-action="bg-pick">${t('Choose…')}</button><button class="s-btn" data-action="bg-reset">${t('Reset')}</button></span></div>
            </div>
            <div class="s-group s-reset-all"><div class="s-row"><span><b>${t('Reset the look')}</b><small>${t('cursor, pointer, fonts, size, corners and background go back to default – your theme and colors stay')}</small></span><button class="s-btn" data-action="look-reset">${icon('refresh', 13)}${t('Reset all')}</button></div></div>
          </section>
          <section data-pane="editor">
            <h3>${t('Text')}</h3>
            <div class="s-group">
              <label class="s-row"><span><b>${t('Font')}</b></span><select data-key="fontFamily">${FONTS.map((f) => opt(f.id, t(f.label), setting('fontFamily'))).join('')}</select></label>
              <label class="s-row"><span><b>${t('Font size')}</b></span><input type="number" min="9" max="32" data-key="fontSize" value="${setting('fontSize')}"></label>
              <label class="s-row"><span><b>${t('Line height')}</b></span><select data-key="lineHeight">${[1.3, 1.45, 1.6, 1.8].map((v) => opt(v, t({ 1.3: 'compact', 1.45: 'normal', 1.6: 'relaxed', 1.8: 'large' }[v]), setting('lineHeight'))).join('')}</select></label>
              ${toggle('ligatures', 'Ligatures', 'joined characters like => and != (e.g. in Cascadia Code)')}
              ${toggle('wordWrap', 'Wrap long lines')}
            </div>
            <h3>${t('Behaviour')}</h3>
            <div class="s-group">
              ${toggle('minimap', 'Code map', 'small preview of the code on the right')}
              ${toggle('stickyScroll', 'Sticky headers', 'the function or class you are in stays at the top while you scroll')}
              ${toggle('inertia', 'Smooth scrolling with inertia', 'text keeps gliding a bit after you stop the wheel')}
              ${toggle('suggestDetails', 'Show docs next to suggestions', 'documentation of the selected function, like in VS Code')}
              ${toggle('autosave', 'Auto save', 'saves the file shortly after you stop typing')}
            </div>
          </section>
          <section data-pane="running">
            <h3>Python</h3>
            <div class="s-group">
              <div class="s-row"><span><b>${t('Interpreter')}</b><small>${state.python ? `${escapeHtml(state.python.version)} · ${escapeHtml(state.python.path)}` : t('not found')}</small></span><button class="s-btn" data-action="python">${t('Change…')}</button></div>
            </div>
            <h3>${t('Output')}</h3>
            <div class="s-group">
              ${toggle('clearOnRun', 'Clear output before running')}
              <label class="s-row"><span><b>${t('Output font size')}</b></span><input type="number" min="9" max="28" data-key="terminalFontSize" value="${setting('terminalFontSize')}"></label>
            </div>
          </section>
          <section data-pane="tools">
            <p class="s-lead">${t('Flux keeps its installer small. Programming languages are downloaded from their official sources only when you need them.')}</p>
            <div class="s-group s-mb">
              ${toggle('autoUpdateLangs', 'Update languages automatically', 'once a day in the background, from the official sources')}
              <div class="s-row"><span><b>${t('Updates')}</b><small id="s-upd-note">${t('See which languages have a newer version.')}</small></span><button class="s-btn" data-action="check-updates">${t('Check now')}</button></div>
            </div>
            <div class="tc-list" id="s-tools"><div class="s-loading">${t('Checking what is installed…')}</div></div>
          </section>
          <section data-pane="ai">
            <p class="s-lead">${t('Flux can use Claude as a coding assistant (Ctrl+I). You pay Anthropic directly with your own API key – it is stored encrypted on this computer.')}</p>
            <h3>${t('API key')}</h3>
            <div class="s-group">
              <div class="s-row"><span><b>Anthropic API key</b><small id="s-ai-keyhint">${t('Get one at console.anthropic.com')}</small></span><span class="s-inline"><input type="password" id="s-ai-key" placeholder="sk-ant-…" autocomplete="off" spellcheck="false"><button class="s-btn" data-action="ai-key">${t('Save')}</button></span></div>
              <label class="s-row"><span><b>${t('Model')}</b><small>${t('Opus is the smartest, Haiku the fastest and cheapest.')}</small></span><select id="s-ai-model"></select></label>
            </div>
            <h3>${t('MCP connectors')}</h3>
            <p class="s-lead">${t('Connect remote MCP servers (for example GitHub, Linear or your own) – Claude can then use their tools while answering.')}</p>
            <div class="s-group" id="s-mcp"></div>
            <form class="s-mcp-add" id="s-mcp-add">
              <input id="s-mcp-name" placeholder="${t('Name, e.g. github')}" autocomplete="off" spellcheck="false" required>
              <input id="s-mcp-url" placeholder="https://…/mcp" autocomplete="off" spellcheck="false" required>
              <input id="s-mcp-token" type="password" placeholder="${t('Token (optional)')}" autocomplete="off">
              <button class="s-btn">${icon('plus', 13)}${t('Add')}</button>
            </form>
            <h3>${t('Use Flux from other AI apps')}</h3>
            <p class="s-lead">${t('Claude Desktop, Claude Code, Cursor and other apps that support MCP can see and change your Flux projects – files, description and to-do list. It works only on this computer and needs a secret key.')}</p>
            <div id="s-flux-mcp"></div>
          </section>
          ${ghOn() ? '<section data-pane="github" id="s-github"></section>' : ''}
          <section data-pane="plugins" id="s-plugins"></section>
          <section data-pane="general">
            <h3 id="g-updates">${t('About & updates')}</h3>
            <div id="s-about" class="s-about-inline"></div>
            <h3>${t('You')}</h3>
            <div class="s-group">
              <label class="s-row"><span><b>${t('Your name')}</b><small>${t('for the greeting on the home screen')}</small></span><input class="s-text" data-key="userName" value="${escapeAttr(setting('userName'))}" placeholder="${t('e.g. Šimon')}"></label>
            </div>
            <h3>${t('Memory & speed')}</h3>
            <div class="s-group">
              <div class="s-row s-mem-row"><span><b>${t('Memory used by Flux')}</b><small id="s-mem">${t('Loading…')}</small></span><button class="s-btn" data-action="mem-free">${icon('refresh', 13)}${t('Free memory')}</button></div>
              <label class="s-row s-lite"><span><b>${t('Save memory')}</b><small>${t('Uses less memory and turns off effects – good for slower PCs. You can change each part under Advanced.')}</small></span><input type="checkbox" class="switch" data-key="lite"${setting('lite') ? ' checked' : ''}></label>
            </div>
            <details class="s-adv"${OPT_KEYS.some((k) => state.settings[k] !== undefined) ? ' open' : ''}><summary>${icon('chevron', 12)}${t('Advanced')}</summary>
              <div class="s-group">
                ${[
                  ['optPyAc', t('Python autocomplete'), t('suggestions and error checking for Python (Pyright) – uses the most memory')],
                  ['optFx', t('Transparency and blur'), t('see-through panels and your blurred wallpaper')],
                  ['optAnim', t('Animations'), t('windows, menus and cards glide in and out')],
                  ['optEditorFx', t('Extra editor effects'), t('sticky headers, code map, smooth cursor and highlighting the word under the cursor')],
                  ['optJsLimit', t('Limit memory of the window'), t('at most 512 MB for the app window – applies after a restart')],
                ]
                  .map(([k, label, hint]) => `<label class="s-row"><span><b>${label}</b><small>${hint}</small></span><input type="checkbox" class="switch" data-key="${k}"${(k === 'optJsLimit' ? state.settings[k] ?? !!setting('lite') : optOn(k)) ? ' checked' : ''}></label>`)
                  .join('')}
                <label class="s-row"><span><b>${t('Memory for Python autocomplete')}</b><small>${t('more memory helps with big projects')}</small></span><select data-key="pyMemory">${[768, 1024, 2048].map((v) => opt(v, v < 1024 ? `${v} MB` : `${v / 1024} GB`, state.settings.pyMemory ?? (setting('lite') ? 768 : 2048))).join('')}</select></label>
                <label class="s-row"><span><b>${t('Stop Python autocomplete when not used')}</b><small>${t('frees its memory when no Python file is open')}</small></span><select data-key="lspIdle">${[1, 5, 15, 0].map((v) => opt(v, v ? t('after {n} min', { n: v }) : t('never'), lspIdleMin())).join('')}</select></label>
                <div class="s-row"><span><b>${t('Reset advanced')}</b><small>${t('every part follows “Save memory” again')}</small></span><button class="s-btn" data-action="opt-reset">${icon('refresh', 13)}${t('Reset')}</button></div>
              </div>
            </details>
            <h3>${t('Language')}</h3>
            <div class="s-group">
              <div class="s-row"><span><b>${t('App language')}</b><small>${t('Languages are downloaded from GitHub when you pick them.')}</small></span><div class="lang-pick" id="s-lang"><button class="s-btn lang-cur" type="button">${flag(setting('language') || 'en', 20)}<span>${escapeHtml(setting('language') || 'en')}</span>${icon('chevron', 12)}</button></div></div>
            </div>
            <h3>${t('Welcome')}</h3>
            <div class="s-group">
              <div class="s-row"><span><b>${t('Intro and tour')}</b><small>${t('Replay the first-start intro or the feature tour.')}</small></span><span class="s-btns"><button class="s-btn" data-action="intro">${t('Intro')}</button><button class="s-btn" data-action="tour">${t('Tour')}</button></span></div>
            </div>
            <h3 id="g-keys">${t('Shortcuts')}</h3>
            <div class="s-group s-mb">
              <div class="s-row"><span><b>${t('Your own shortcuts')}</b><small>${t('Run any script or command, insert text, chain steps – all in one file.')}</small></span><button class="s-btn" data-action="edit-keys">${icon('edit', 13)}${t('Edit shortcuts.json')}</button></div>
            </div>
            <input class="s-search" id="s-keys-q" placeholder="${t('Search shortcuts…')}" spellcheck="false">
            <div id="s-keys">${shortcutRows()}</div>
          </section>
        </div>
      </div>
    </div>`;
  tools.bind($('#s-tools'));
  tools.status().then((st) => {
    const box = $('#s-tools');
    if (box) box.innerHTML = st.list.map((tc) => tools.row(tc)).join('');
  });
  $('#s-keys-q').oninput = (e) => {
    const q = e.target.value.toLowerCase().trim();
    // .s-row má display:flex, preto skrývame cez style (atribút hidden by nezabral).
    for (const r of panel.querySelectorAll('#s-keys .s-key')) r.style.display = !q || r.textContent.toLowerCase().includes(q) ? '' : 'none';
    for (const c of panel.querySelectorAll('#s-keys .s-keycat')) c.style.display = [...c.querySelectorAll('.s-key')].some((r) => r.style.display !== 'none') ? '' : 'none';
  };
  // AI: kľúč, model, MCP servery
  let aiCfg = null;
  const renderAI = () => {
    if (!aiCfg || !$('#s-mcp')) return;
    $('#s-ai-keyhint').textContent = aiCfg.hasKey ? t('Saved ({hint}). Paste a new one to replace it.', { hint: aiCfg.keyHint }) : t('Get one at console.anthropic.com');
    $('#s-ai-model').innerHTML = aiCfg.models.map((m) => opt(m.id, m.name, aiCfg.model)).join('');
    $('#s-mcp').innerHTML = aiCfg.mcp.length
      ? aiCfg.mcp
          .map(
            (m, i) =>
              `<div class="s-row"><span><b>${escapeHtml(m.name)}</b><small>${escapeHtml(m.url)}${m.hasToken ? ` · ${t('token saved')}` : ''}</small></span><span class="s-inline"><input type="checkbox" class="switch" data-mcp-on="${i}"${m.enabled ? ' checked' : ''}><button class="icon-btn" data-mcp-del="${i}" title="${t('Remove')}">${icon('trash', 14)}</button></span></div>`,
          )
          .join('')
      : `<div class="s-row"><span><small>${t('No connectors yet.')}</small></span></div>`;
  };
  flux.aiConfig().then((c) => {
    aiCfg = c;
    renderAI();
  });
  const saveMcp = async (list) => {
    aiCfg = await flux.aiUpdate({ mcp: list });
    renderAI();
    aiPanel?.refreshConfig();
  };
  // Flux ako MCP server pre iné AI aplikácie
  const renderFluxMcp = async () => {
    const box = $('#s-flux-mcp');
    if (!box) return;
    const m = await flux.mcpInfo();
    const on = m.enabled && m.running;
    box.innerHTML = `<div class="s-group">
        <label class="s-row"><span><b>${t('Let AI apps use Flux')}</b><small>${on ? t('Running – AI apps can connect now.') : t('Off')}</small></span><input type="checkbox" class="switch" id="s-fmcp-on"${on ? ' checked' : ''}></label>
        ${
          on
            ? `<div class="s-row"><span><b>${t('Link')}</b><small class="mono">${escapeHtml(m.url)}</small></span><button class="s-btn" data-copy-mcp="url">${icon('file', 13)}${t('Copy')}</button></div>
               <div class="s-row"><span><b>${t('Secret key')}</b><small>${t('Anyone with the key can change your projects – do not share it.')}</small></span><span class="s-inline"><button class="s-btn" data-copy-mcp="key">${icon('file', 13)}${t('Copy')}</button><button class="s-btn" data-action="mcp-new-key">${icon('refresh', 13)}${t('New key')}</button></span></div>
               <div class="s-row s-mcp-apps"><span><b>${t('Connect an app')}</b><small>${t('One click for Claude Desktop (restart it afterwards); the others get a ready command to paste.')}</small></span><span class="s-inline wrap"><button class="s-btn primary" data-action="mcp-claude">${icon('sparkle', 13)}${t('Add to Claude Desktop')}</button><button class="s-btn" data-action="mcp-ext" title="${t('A file you double-click – Claude Desktop installs Flux as an extension.')}">${icon('download', 13)}${t('Claude extension (.mcpb)')}</button><button class="s-btn" data-copy-mcp="claude-code">${t('Claude Code')}</button><button class="s-btn" data-copy-mcp="cursor">${t('Cursor / VS Code')}</button></span></div>`
            : ''
        }
      </div>
      <p class="s-note">${t('ChatGPT and claude.ai in the browser can only connect to servers on the internet, not to apps on your computer – use Claude Desktop, Claude Code or Cursor.')}</p>`;
    box.onchange = async (e) => {
      if (e.target.id !== 's-fmcp-on') return;
      try {
        await flux.mcpEnable(e.target.checked);
      } catch (err) {
        toast(errorText(err), 'error', 7000);
      }
      renderFluxMcp();
    };
    box.onclick = async (e) => {
      const c = e.target.closest('[data-copy-mcp]');
      if (c) {
        const kind = c.dataset.copyMcp;
        const text = {
          url: m.url,
          key: m.token,
          'claude-code': `claude mcp add --transport http flux ${m.url} --header "Authorization: Bearer ${m.token}"`,
          cursor: JSON.stringify({ mcpServers: { flux: { url: m.url, headers: { Authorization: `Bearer ${m.token}` } } } }, null, 2),
        }[kind];
        await navigator.clipboard.writeText(text);
        return toast(kind === 'claude-code' ? t('Copied – paste it into a terminal.') : kind === 'cursor' ? t('Copied – paste it into mcp.json of Cursor or VS Code.') : t('Copied.'), 'ok', 5000);
      }
      if (e.target.closest('[data-action="mcp-new-key"]')) {
        await flux.mcpNewKey();
        toast(t('New key made. Connect your AI apps again.'), 'ok', 6000);
        return renderFluxMcp();
      }
      if (e.target.closest('[data-action="mcp-ext"]')) {
        try {
          const r = await flux.mcpExtension();
          toast(r.opened ? t('Claude Desktop opens the extension – press Install.') : t('Saved to {file}. Double-click it to install it in Claude Desktop.', { file: r.file }), 'ok', 10000);
        } catch (err) {
          toast(errorText(err), 'error', 8000);
        }
        return;
      }
      if (e.target.closest('[data-action="mcp-claude"]')) {
        try {
          await flux.mcpAddToClaude();
          toast(t('Added to Claude Desktop. Restart Claude Desktop – then ask it about your Flux projects.'), 'ok', 9000);
        } catch (err) {
          toast(errorText(err), 'error', 8000);
        }
      }
    };
  };
  renderFluxMcp();
  $('#s-mcp-add').onsubmit = async (e) => {
    e.preventDefault();
    const name = $('#s-mcp-name').value.trim().replace(/\s+/g, '-');
    const url = $('#s-mcp-url').value.trim();
    if (!/^https:\/\//.test(url)) return toast(t('The MCP server address must start with https://'), 'error');
    if (aiCfg.mcp.some((m) => m.name === name)) return toast(t('A connector with this name already exists.'), 'error');
    await saveMcp([...aiCfg.mcp, { name, url, enabled: true, token: $('#s-mcp-token').value.trim() }]);
    $('#s-mcp-add').reset();
  };
  $('#s-ai-model').onchange = async (e) => {
    e.stopPropagation();
    aiCfg = await flux.aiUpdate({ model: e.target.value });
    aiPanel?.refreshConfig();
  };
  renderGitHubSettings();
  const showTab = (id) => {
    state.settingsTab = id;
    navNote();
    $('#s-title').textContent = tabs.find(([x]) => x === id)?.[2] || '';
    if (id === 'plugins' && !$('#s-plugins').childElementCount) pluginsUI.render($('#s-plugins'));
    if (id === 'general') {
      showMemory();
      if (!$('#s-about').childElementCount) updatesUI.render($('#s-about'), { compact: true });
    }
    panel.querySelectorAll('[data-tab]').forEach((b) => b.classList.toggle('on', b.dataset.tab === id));
    panel.querySelectorAll('[data-pane]').forEach((p) => (p.hidden = p.dataset.pane !== id));
  };
  // Dlhé karty (Vzhľad, Všeobecné): časti ako podpoložky v ľavom menu (zvýrazní sa tá, ktorú práve vidíš).
  for (const pane of panel.querySelectorAll('[data-pane="appearance"], [data-pane="general"]')) {
    const heads = [...pane.querySelectorAll(':scope > h3')];
    const tabBtn = panel.querySelector(`.s-tab[data-tab="${pane.dataset.pane}"]`);
    if (heads.length < 4 || !tabBtn) continue;
    const sub = document.createElement('div');
    sub.className = 's-sub';
    sub.innerHTML = heads.map((h, i) => `<button type="button" data-jump="${i}">${escapeHtml(h.childNodes[0]?.textContent?.trim() || h.textContent.trim())}</button>`).join('');
    tabBtn.after(sub);
    sub.onclick = (e) => {
      const b = e.target.closest('[data-jump]');
      if (!b) return;
      e.stopPropagation();
      if (state.settingsTab !== pane.dataset.pane) tabBtn.click();
      heads[Number(b.dataset.jump)].scrollIntoView({ behavior: optOn('optAnim') ? 'smooth' : 'auto', block: 'start' });
    };
    const scroller = pane.closest('.s-body') || pane.parentElement;
    const spy = () => {
      if (pane.hidden) return;
      const top = scroller.getBoundingClientRect().top + 40;
      let cur = 0;
      heads.forEach((h, i) => {
        if (h.getBoundingClientRect().top <= top) cur = i;
      });
      sub.querySelectorAll('[data-jump]').forEach((b, i) => b.classList.toggle('on', i === cur));
    };
    scroller.addEventListener('scroll', spy, { passive: true });
    tabBtn.addEventListener('click', () => requestAnimationFrame(spy));
    requestAnimationFrame(spy);
  }
  showTab(tab);
  if (jumpTo) requestAnimationFrame(() => $(`#${jumpTo}`)?.scrollIntoView({ block: 'start' }));
  // Hľadanie naprieč všetkými záložkami: ukáže len riadky, ktoré sedia.
  const ROWS = '.s-row, .theme-card, .tc-row, .ob-row';
  $('#s-find').oninput = (e) => {
    const q = e.target.value.toLowerCase().trim();
    const sections = panel.querySelectorAll('[data-pane]');
    for (const el of panel.querySelectorAll(ROWS + ', .s-group, .s-body h3, .s-lead, .s-keycat, .s-jump')) el.style.display = '';
    if (!q) return showTab(state.settingsTab || 'appearance');
    panel.querySelectorAll('details.s-adv').forEach((d) => (d.open = true));
    $('#s-title').textContent = t('Search results');
    panel.querySelectorAll('[data-tab]').forEach((b) => b.classList.remove('on'));
    let any = false;
    for (const sec of sections) {
      const tabName = tabs.find(([x]) => x === sec.dataset.pane)?.[2]?.toLowerCase() || '';
      let hits = 0;
      for (const r of sec.querySelectorAll(ROWS)) {
        const ok = tabName.includes(q) || r.textContent.toLowerCase().includes(q);
        r.style.display = ok ? '' : 'none';
        if (ok) hits++;
      }
      for (const g of sec.querySelectorAll('.s-group, .theme-grid, .tc-list, .s-keycat')) {
        const vis = [...g.querySelectorAll(ROWS)].some((r) => r.style.display !== 'none');
        g.style.display = vis || !g.querySelector(ROWS) ? '' : 'none';
      }
      for (const h of sec.querySelectorAll('h3, .s-lead, .s-jump')) h.style.display = 'none';
      sec.hidden = !hits;
      any ||= !!hits;
    }
    $('#s-noresult')?.remove();
    if (!any) panel.querySelector('.s-body').insertAdjacentHTML('beforeend', `<div class="s-loading" id="s-noresult">${t('Nothing found')}</div>`);
  };
  requestAnimationFrame(() => $('#s-find')?.focus());
  // Otvorené z domovskej obrazovky (koliesko, Ctrl+,): domov ide pod nastavenia, inak by ich zakryl.
  $('#start').classList.add('under-settings');
  // Zoznam jazykov z GitHubu.
  flux.i18nList().then((list) => {
    const box = $('#s-lang');
    if (!box || !Array.isArray(list)) return;
    const cur = setting('language') || 'en';
    const me = list.find((l) => l.code === cur) || { code: cur, name: cur };
    box.innerHTML = `<button class="s-btn lang-cur" type="button">${flag(me.code, 20)}<span>${escapeHtml(me.native || me.name)}</span>${icon('chevron', 12)}</button>
      <div class="lang-menu" hidden>${list
        .map((l) => `<button type="button" class="lang-opt${l.code === cur ? ' on' : ''}" data-lang-code="${l.code}">${flag(l.code, 22)}<b>${escapeHtml(l.native || l.name)}</b><small>${escapeHtml(l.name)}</small></button>`)
        .join('')}</div>`;
    const menu = box.querySelector('.lang-menu');
    box.onclick = async (e) => {
      e.preventDefault();
      e.stopPropagation();
      const pick = e.target.closest('[data-lang-code]');
      if (!pick) return (menu.hidden = !menu.hidden);
      menu.hidden = true;
      if (pick.dataset.langCode === cur) return;
      box.querySelector('.lang-cur').disabled = true;
      try {
        await flux.i18nUse(pick.dataset.langCode);
        flux.reload();
      } catch {
        box.querySelector('.lang-cur').disabled = false;
        toast(t('Could not download the language. Check your internet connection.'), 'error');
      }
    };
    panel.addEventListener('pointerdown', (e) => {
      if (!box.contains(e.target)) menu.hidden = true;
    });
  });
  panel.hidden = false;

  panel.onclick = async (e) => {
    if (e.target === panel || e.target.closest('[data-close]')) return closeSettings();
    const editTheme = e.target.closest('[data-edit-theme]');
    if (editTheme) {
      e.stopPropagation();
      return openThemeStudio(editTheme.dataset.editTheme);
    }
    const tabBtn = e.target.closest('[data-tab]');
    if (tabBtn) return showTab(tabBtn.dataset.tab);
    // Výber sa len označí – bez prekreslenia (žiadne blikanie ani skok na začiatok).
    const themeBtn = e.target.closest('[data-theme]');
    if (themeBtn) {
      panel.querySelectorAll('[data-theme]').forEach((b) => b.classList.toggle('active', b === themeBtn));
      return setCodeTheme(themeBtn.dataset.theme);
    }
    const accentBtn = e.target.closest('[data-accent]');
    if (accentBtn) {
      panel.querySelectorAll('[data-accent]').forEach((b) => b.classList.toggle('active', b === accentBtn));
      return setAccent(accentBtn.dataset.accent);
    }
    if (e.target.closest('[data-action="python"]')) {
      closeSettings();
      pythonMenu();
    }
    if (e.target.closest('[data-action="ai-key"]')) {
      const key = $('#s-ai-key').value.trim();
      if (!key) return;
      aiCfg = await flux.aiUpdate({ key });
      $('#s-ai-key').value = '';
      renderAI();
      aiPanel?.refreshConfig();
      return toast(t('API key saved.'), 'ok');
    }
    const mdel = e.target.closest('[data-mcp-del]');
    if (mdel) return saveMcp(aiCfg.mcp.filter((_, i) => i !== Number(mdel.dataset.mcpDel)));
    const mon = e.target.closest('[data-mcp-on]');
    if (mon) return saveMcp(aiCfg.mcp.map((m, i) => (i === Number(mon.dataset.mcpOn) ? { ...m, enabled: mon.checked } : m)));
    const rebind = e.target.closest('[data-rebind]');
    if (rebind) {
      const b = allBindings().find((x) => x.id === rebind.dataset.rebind);
      if (!b) return;
      return keymap.record(rebind, b, () => {
        $('#s-keys').innerHTML = shortcutRows();
        $('#s-keys-q').dispatchEvent(new Event('input'));
      });
    }
    const resetKey = e.target.closest('[data-reset-key]');
    if (resetKey) {
      e.preventDefault();
      const over = { ...(setting('keymap') || {}) };
      delete over[resetKey.dataset.resetKey];
      await saveSettings({ keymap: over });
      $('#s-keys').innerHTML = shortcutRows();
      return;
    }
    const ptr = e.target.closest('[data-pointer]');
    if (ptr) {
      panel.querySelectorAll('[data-pointer]').forEach((b) => b.classList.toggle('on', b === ptr));
      await saveSettings({ pointer: ptr.dataset.pointer });
      return applyCustomization();
    }
    if (e.target.closest('[data-action="mem-free"]')) {
      if (!state.tabs.some(isPythonTab)) lsp.stop();
      await flux.freeMemory?.();
      return showMemory();
    }
    if (e.target.closest('[data-action="opt-reset"]')) {
      await saveSettings(Object.fromEntries(OPT_KEYS.map((k) => [k, undefined])));
      applyPerformance();
      return rerenderSettings();
    }
    const cr = e.target.closest('[data-color-reset]');
    if (cr) {
      await saveSettings({ [cr.dataset.colorReset]: '' });
      applyCustomization();
      return rerenderSettings();
    }
    if (e.target.closest('[data-action="colors-reset"]')) {
      await saveSettings({ uiText: '', uiText2: '', uiBase: '', uiCard: '', uiLine: '', cardAlpha: 74 });
      applyCustomization();
      return rerenderSettings();
    }
    if (e.target.closest('[data-action="caret-reset"]')) {
      await saveSettings({ caretColor: '' });
      return applyCustomization();
    }
    if (e.target.closest('[data-action="cursor-pick"]')) {
      try {
        const img = await flux.chooseCursor();
        if (!img) return;
        await saveSettings({ pointerImage: img.url, pointerImageSize: [img.width, img.height], pointer: 'custom' });
        applyCustomization();
        toast(t('Your cursor is set.'), 'ok');
        return rerenderSettings();
      } catch (err) {
        return toast(errorText(err), 'error', 6000);
      }
    }
    if (e.target.closest('[data-action="pointer-reset"]')) {
      await saveSettings({ pointer: 'text', pointerColor: '', pointerEverywhere: false });
      applyCustomization();
      toast(t('The normal cursor is back.'), 'ok');
      return rerenderSettings();
    }
    if (e.target.closest('[data-action="look-reset"]')) {
      const keys = ['caretStyle', 'caretBlink', 'caretWidth', 'caretSmooth', 'caretColor', 'pointer', 'pointerColor', 'pointerEverywhere', 'pointerHotspot', 'fontCustom', 'uiFont', 'uiZoom', 'letterSpacing', 'lineNumbers', 'whitespace', 'bracketColors', 'cornerRadius', 'wallBlur', 'wallOpacity', 'darkLift', 'density', 'uiText', 'uiText2', 'uiBase', 'uiCard', 'uiLine', 'cardAlpha'];
      await saveSettings(Object.fromEntries(keys.map((k) => [k, DEFAULTS[k]])));
      applyEditorSettings();
      toast(t('Everything looks like new again.'), 'ok');
      return rerenderSettings();
    }
    if (e.target.closest('[data-action="bg-pick"]')) {
      const ok = await flux.chooseBackground();
      if (ok) {
        wallpaperUrl = null;
        window.dispatchEvent(new Event('focus'));
        toast(t('Background changed.'), 'ok');
      }
      return;
    }
    if (e.target.closest('[data-action="bg-reset"]')) {
      await flux.resetBackground();
      wallpaperUrl = null;
      window.dispatchEvent(new Event('focus'));
      return;
    }
    if (e.target.closest('[data-action="edit-keys"]')) {
      closeSettings();
      const f = await flux.shortcutsFile();
      return openFile(f.path);
    }
    if (e.target.closest('[data-action="new-theme"]')) {
      closeSettings();
      return createCustomTheme();
    }
    const upd = e.target.closest('[data-action="check-updates"]');
    if (upd) {
      upd.disabled = true;
      $('#s-upd-note').textContent = t('Checking…');
      const found = await tools.checkUpdates();
      const n = Object.keys(found).length;
      if ($('#s-upd-note')) $('#s-upd-note').textContent = n ? t('{n} update(s) available.', { n }) : t('Everything is up to date.');
      upd.disabled = false;
      return;
    }
    if (e.target.closest('[data-action="intro"]')) {
      closeSettings();
      onboarding.open();
    }
    if (e.target.closest('[data-action="tour"]')) {
      closeSettings();
      onboarding.startTour();
    }
  };
  // Posuvníky a farby sa prejavia hneď pri ťahaní.
  panel.oninput = (e) => {
    const el = e.target;
    const key = el.dataset?.key;
    if (!key || !['range', 'color'].includes(el.type)) return;
    state.settings = { ...state.settings, [key]: el.type === 'range' ? Number(el.value) : el.value };
    if (key === 'uiZoom') $('#s-zoom-v').textContent = `${el.value} %`;
    applyEditorSettings();
  };
  panel.onchange = async (e) => {
    const el = e.target;
    if (el.dataset.customAccent !== undefined) {
      await setAccent(el.value);
      return;
    }
    const key = el.dataset.key;
    if (!key) return;
    let value = el.type === 'checkbox' ? el.checked : el.value;
    if (el.type === 'number' || el.type === 'range' || ['lineHeight', 'pyMemory', 'lspIdle'].includes(key)) value = Number(value);
    await saveSettings({ [key]: value });
    // Hlavný prepínač pamäte: všetky časti v Advanced sa zase riadia ním.
    if (key === 'lite') {
      await saveSettings(Object.fromEntries(OPT_KEYS.map((k) => [k, undefined])));
      applyPerformance();
      rerenderSettings();
    }
    if (OPT_KEYS.includes(key)) applyPerformance();
    if (['menuBar', 'showSearch', 'panelPos', 'sidePos'].includes(key)) setLayout({});
    if (key === 'suggestDetails') showSuggestDetails(value);
    if (key === 'material') await saveSettings({ translucent: true });
    applyEditorSettings();
    renderStatus();
  };
}

// Plávajúce hľadanie všetkého (Ctrl+Shift+A): príkazy, nastavenia, súbory, projekty.
let settingsIndexCache = null;
function settingsIndex() {
  if (settingsIndexCache) return settingsIndexCache;
  const wasHidden = $('#settings').hidden;
  openSettings();
  const out = [];
  for (const sec of $('#settings').querySelectorAll('[data-pane]')) {
    // Skratky sú v hľadaní ako príkazy, nie ako nastavenia.
    // aj nadpisy častí (Accent color, Code theme…), nielen riadky
    for (const el of sec.querySelectorAll(':scope > h3, .s-row:not(.s-key) > span:first-child > b, .theme-card > span:nth-child(2), .tc-text > b')) {
      const label = el.firstChild?.textContent?.trim() || el.textContent.trim();
      if (label && !out.some((o) => o.label === label)) out.push({ tab: sec.dataset.pane, label });
    }
  }
  // Aktualizácie sa do Všeobecných dokresľujú až neskôr (updatesUI) – ich riadky pridáme ručne.
  for (const label of [t('Check for updates'), t('Update automatically'), t('All versions'), t('Release notes')])
    if (!out.some((o) => o.label === label)) out.push({ tab: 'general', label });
  if (wasHidden) {
    $('#settings').hidden = true;
    $('#start').classList.remove('under-settings');
  }
  settingsIndexCache = out;
  return out;
}

async function searchEverything() {
  const cmds = [...commands(), ...pluginCommands()].map((c) => ({ ...c, group: t('Command') }));
  const cmdLabels = new Set(cmds.map((c) => c.label));
  // Riadky zo Skratiek sú už medzi príkazmi.
  const sets = settingsIndex().filter((x) => !cmdLabels.has(x.label)).map((x) => ({
    label: x.label,
    icon: icon('settings', 15),
    detail: t('Setting'),
    keywords: 'settings',
    run: () => {
      state.settingsTab = x.tab;
      openSettings();
      // Nájdený riadok zablikne (riadky aktualizácií sa dokreslia o chvíľu – skúsi sa znova).
      const find = (retry) => {
        const pane = $(`#settings [data-pane="${x.tab}"]`);
        const row = [...(pane || $('#settings')).querySelectorAll('.s-row, .theme-card, .tc-row, h3, .up-notes-fold')].find((r) => r.textContent.includes(x.label));
        if (!row) return retry && setTimeout(() => find(false), 500);
        const fold = row.closest('details');
        if (fold) fold.open = true;
        row.scrollIntoView({ block: row.tagName === 'H3' ? 'start' : 'center' });
        row.classList.add('flash');
        setTimeout(() => row.classList.remove('flash'), 1400);
      };
      find(true);
    },
  }));
  const projects = (state.projectList || []).map((p) => ({
    label: p.name,
    icon: kindIcon(p.kind),
    detail: t('Project'),
    run: () => (keyOf(p.dir) === keyOf(state.workspace || '') ? showProjectPage() : setWorkspace(p.dir)),
  }));
  let files = [];
  try {
    files = state.workspace ? await flux.listAll() : [];
  } catch {}
  const fileItems = files.slice(0, 3000).map((f) => ({ label: relPath(f), icon: fileIcon(basename(f)), detail: t('File'), run: () => openFile(f) }));
  openPalette({
    placeholder: t('Search commands, settings, files and projects…'),
    items: [...cmds, ...sets, ...projects, ...fileItems],
    onPick: (it) => it.run(),
  });
}

// Skratky po kategóriách; klik na skratku = nahrať novú (Backspace vráti pôvodnú).
// Všetky skratky: vstavané + každý príkaz editora + pluginy + vlastné zo shortcuts.json.
let editorBindingsCache = null;
function editorBindings() {
  if (editorBindingsCache) return editorBindingsCache;
  const known = new Set(BINDINGS.map((b) => b.editorAction).filter(Boolean));
  let kb = null;
  try {
    kb = StandaloneServices.get(IKeybindingService);
  } catch {}
  const keyOf = (id) => {
    try {
      return (kb?.lookupKeybinding(id)?.getLabel() || '').replace(/UpArrow/g, 'Up').replace(/DownArrow/g, 'Down').replace(/LeftArrow/g, 'Left').replace(/RightArrow/g, 'Right');
    } catch {
      return '';
    }
  };
  editorBindingsCache = editor
    .getSupportedActions()
    .filter((a) => a.label && !known.has(a.id))
    .map((a) => ({ id: `ed:${a.id}`, cat: 'more', label: a.label, key: keyOf(a.id), editorAction: a.id }))
    .sort((a, b) => a.label.localeCompare(b.label));
  return editorBindingsCache;
}

function allBindings() {
  const plugins = (pluginHost?.commands() || []).map((c) => ({ id: `plugin:${c.id}`, cat: 'plugins', label: `${c.plugin}: ${c.label}`, key: c.key || '', run: c.run }));
  const custom = (userKeys?.list() || []).map((k) => ({
    id: `custom:${k.index}`,
    cat: 'custom',
    label: k.label || k.run || k.insert || k.command || k.url || t('Custom'),
    key: k.key || '',
    save: (combo) => userKeys.setKey(k.index, combo),
  }));
  return [...BINDINGS, ...custom, ...plugins, ...(editor ? editorBindings() : [])];
}

function shortcutRows() {
  const all = allBindings();
  return CATEGORIES.map(([cat, name]) => {
    const rows = all.filter((b) => b.cat === cat);
    if (!rows.length) return '';
    return `<div class="s-keycat" data-cat="${cat}"><h3>${t(name)}</h3><div class="s-group">${rows
      .map(
        (b) =>
          `<div class="s-row s-key" data-key-row="${escapeAttr(b.id)}"><span><b>${escapeHtml(t(b.label))}</b>${keymap.isChanged(b) ? `<small class="s-changed">${t('changed')} · <a href="#" data-reset-key="${escapeAttr(b.id)}">${t('reset')}</a></small>` : ''}</span><button class="kbds kbd-btn" data-rebind="${escapeAttr(b.id)}" title="${t('Click and press a new shortcut')}">${kbdHtml(keymap.current(b), escapeHtml)}</button></div>`,
      )
      .join('')}</div></div>`;
  }).join('');
}

// Nastavenia nakresliť znova (napr. po nahratí kurzora) – bez skoku na začiatok.
// Koľko pamäte práve používa Flux (všetky jeho procesy).
async function showMemory() {
  const el = $('#s-mem');
  if (!el) return;
  try {
    const m = await flux.memory();
    el.textContent = `${m.total} MB${m.lsp ? ` · ${t('Python autocomplete: {mb} MB', { mb: m.lsp })}` : ` · ${t('Python autocomplete is off')}`}`;
  } catch {
    el.textContent = '';
  }
}

function rerenderSettings() {
  const top = $('#settings .s-body')?.scrollTop || 0;
  openSettings();
  const body = $('#settings .s-body');
  if (body) body.scrollTop = top;
}

function closeSettings() {
  $('#settings').hidden = true;
  $('#start').classList.remove('under-settings');
  navNote();
  if (state.active) editor.focus();
}

async function setAccent(value) {
  if (state.workspace) {
    await saveSettings({ accents: { ...(state.settings.accents || {}), [state.workspace]: value } });
  } else {
    await saveSettings({ accent: value });
  }
  applyTheme();
}

// ---------- späť / dopredu (šípky, bočné tlačidlá myši, Alt+←/→) ----------
// Miesto = domov, stránka projektu, súbor alebo karta nastavení.
const nav = { back: [], fwd: [], cur: null, busy: false, timer: null };
function currentPlace() {
  if (!$('#settings').hidden) return { type: 'settings', id: state.settingsTab || 'appearance' };
  if (!$('#start').hidden) return { type: 'home', id: '' };
  if (state.active) return { type: 'file', id: state.active.path, ws: state.workspace };
  if (state.workspace) return { type: 'project', id: state.workspace };
  return { type: 'home', id: '' };
}
const samePlace = (a, b) => !!a && !!b && a.type === b.type && keyOf(a.id || '') === keyOf(b.id || '');
// Volá sa po každej zmene obrazovky; krátke prechodné stavy sa zlúčia.
function navNote() {
  clearTimeout(nav.timer);
  nav.timer = setTimeout(() => {
    if (nav.busy) return;
    const p = currentPlace();
    if (samePlace(p, nav.cur)) return;
    if (nav.cur) nav.back.push(nav.cur);
    if (nav.back.length > 60) nav.back.shift();
    nav.fwd = [];
    nav.cur = p;
    renderNavButtons();
  }, 60);
}
function renderNavButtons() {
  document.querySelectorAll('.nav-back').forEach((b) => (b.disabled = !nav.back.length));
  document.querySelectorAll('.nav-fwd').forEach((b) => (b.disabled = !nav.fwd.length));
}
const navButtons = () =>
  `<span class="nav-btns no-drag"><button class="icon-btn nav-back" title="${t('Back')} (Alt+←)"${nav.back.length ? '' : ' disabled'}>${icon('arrowLeft', 15)}</button><button class="icon-btn nav-fwd" title="${t('Forward')} (Alt+→)"${nav.fwd.length ? '' : ' disabled'}>${icon('arrowRight', 15)}</button></span>`;
async function goPlace(p) {
  nav.busy = true;
  try {
    if (p.type === 'settings') {
      if ($('#settings').hidden) {
        state.settingsTab = p.id;
        openSettings();
      } else $(`#settings [data-tab="${p.id}"]`)?.click();
      return;
    }
    if (!$('#settings').hidden) closeSettings();
    if (p.type === 'home') return await openStart();
    closeStart();
    const ws = p.type === 'project' ? p.id : p.ws;
    if (ws && keyOf(ws) !== keyOf(state.workspace || '')) await setWorkspace(ws);
    if (p.type === 'project') showProjectPage();
    else await openFile(p.id);
  } catch (err) {
    toast(errorText(err), 'error');
  } finally {
    clearTimeout(nav.timer);
    setTimeout(() => {
      nav.cur = currentPlace();
      nav.busy = false;
      renderNavButtons();
    }, 120);
  }
}
function goBack() {
  if (nav.busy || !nav.back.length) return;
  nav.fwd.push(nav.cur);
  goPlace(nav.back.pop());
}
function goForward() {
  if (nav.busy || !nav.fwd.length) return;
  nav.back.push(nav.cur);
  goPlace(nav.fwd.pop());
}
function setupNav() {
  // Bočné tlačidlá myši (4 = späť, 5 = dopredu); Chromium by inak skúsil ísť späť v stránke.
  const aux = (e) => {
    if (e.button !== 3 && e.button !== 4) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.type === 'mouseup') e.button === 3 ? goBack() : goForward();
  };
  window.addEventListener('mousedown', aux, true);
  window.addEventListener('mouseup', aux, true);
  document.addEventListener('click', (e) => {
    const b = e.target.closest?.('.nav-back, .nav-fwd');
    if (!b) return;
    e.preventDefault();
    e.stopPropagation();
    b.classList.contains('nav-back') ? goBack() : goForward();
  }, true);
  $('#topnav').outerHTML = navButtons().replace('class="nav-btns no-drag"', 'class="nav-btns no-drag" id="topnav"');
  navNote();
}

// ---------- úvodná obrazovka na celé okno (klik na logo) ----------
const START_CHOICES = [
  { id: 'py', lang: 'python', title: 'Python', sub: 'script – print, input, maths', icon: 'main.py' },
  { id: 'py-tkinter', lang: 'python', title: 'Python window', sub: 'app with buttons (tkinter)', icon: 'window.py' },
  { id: 'py-pygame', lang: 'python', title: 'Python game', sub: 'move with arrow keys (pygame)', icon: 'game.py' },
  { id: 'html', lang: 'web', title: 'HTML page', sub: 'one page with a skeleton', icon: 'index.html' },
  { id: 'web', lang: 'web', title: 'Web project', sub: 'HTML + CSS + JavaScript', icon: 'style.css' },
  { id: 'js', lang: 'js', title: 'JavaScript', sub: 'script that runs with Node.js', icon: 'script.js' },
  { id: 'empty', lang: '', title: 'Empty file', sub: '.txt, .md, .py… saved anywhere', icon: 'file.txt' },
];

// ---------- samostatné súbory (aj bez projektu) ----------
async function openStandalone(files) {
  if (!files?.length) return;
  closeStart();
  for (const f of files) await openFile(f);
  refreshLooseFiles();
}
async function openFileDialog() {
  openStandalone(await flux.openFileDialog());
}
// Prázdny súbor mimo projektu: typ → kam uložiť → otvorí sa.
const NEW_FILE_TYPES = () => [
  ['txt', 'Text', t('notes, plain text')],
  ['md', 'Markdown', t('notes with headings and lists')],
  ['py', 'Python', t('script')],
  ['js', 'JavaScript', t('script')],
  ['html', 'HTML', t('web page')],
  ['css', 'CSS', t('styles')],
  ['json', 'JSON', t('data')],
  ['csv', 'CSV', t('table')],
];
async function newStandaloneFile() {
  const ext = await new Promise((resolve) =>
    openPalette({
      placeholder: t('What kind of file?'),
      note: t('The file is not part of a project – you choose where to save it.'),
      items: NEW_FILE_TYPES().map(([e, label, detail]) => ({ label: `${label}  .${e}`, detail, icon: fileIcon(`a.${e}`), ext: e })),
      onPick: (it) => resolve(it.ext),
      onCancel: () => resolve(null),
    }),
  );
  if (!ext) return;
  try {
    const f = await flux.newFileDialog(ext);
    if (f) openStandalone([f]);
  } catch (err) {
    toast(errorText(err), 'error');
  }
}
// Pretiahnutie súborov alebo priečinka do okna.
function setupDrop() {
  window.addEventListener('dragover', (e) => {
    if ([...(e.dataTransfer?.types || [])].includes('Files')) e.preventDefault();
  });
  window.addEventListener('drop', async (e) => {
    const files = [...(e.dataTransfer?.files || [])];
    if (!files.length) return;
    e.preventDefault();
    const ok = [];
    for (const f of files) {
      const p = flux.pathForFile(f);
      if (!p) continue;
      const allowed = await flux.allowFile(p);
      if (allowed?.dir) await setWorkspace(allowed.dir); // pretiahnutý priečinok = projekt
      else if (allowed) ok.push(allowed);
    }
    openStandalone(ok);
  });
}

// Domov Fluxu (klik na logo): pripnuté a nedávne projekty, nový/otvoriť a rýchly štart zo šablóny.
// Kratšia cesta: C:\\Users\\meno\\Documents\\x → ~\\Documents\\x
function shortPath(p) {
  const home = state.home || '';
  let out = home && keyOf(p).startsWith(keyOf(home)) ? `~${p.slice(home.length)}` : p;
  const parts = out.split(/[\\/]/);
  if (parts.length > 4) out = [parts[0], '…', ...parts.slice(-2)].join(state.platform === 'win32' ? '\\' : '/');
  return out;
}
// „pred 2 h“, „včera“…
function timeAgo(ms) {
  const s = Math.max(0, (Date.now() - ms) / 1000);
  if (s < 60) return t('just now');
  if (s < 3600) return t('{n} min ago', { n: Math.round(s / 60) });
  if (s < 86400) return t('{n} h ago', { n: Math.round(s / 3600) });
  if (s < 2 * 86400) return t('yesterday');
  if (s < 30 * 86400) return t('{n} days ago', { n: Math.round(s / 86400) });
  return new Date(ms).toLocaleDateString();
}

async function openStart() {
  const el = $('#start');
  const prefs = setting('codeLangs') || [];
  const choices = [...START_CHOICES].sort((a, b) => (prefs.includes(b.lang) ? 1 : 0) - (prefs.includes(a.lang) ? 1 : 0));
  let list = state.projectList || [];
  try {
    list = await flux.projects();
  } catch {}
  let recentFiles = [];
  try {
    recentFiles = (await flux.recentFiles()).slice(0, 5);
  } catch {}
  const pinned = list.filter((p) => p.pinned);
  const recent = list
    .filter((p) => !p.pinned)
    .sort((a, b) => (a.recent < 0 ? 999 : a.recent) - (b.recent < 0 ? 999 : b.recent))
    .slice(0, 6);
  const h = new Date().getHours();
  const name = String(setting('userName') || '').trim();
  const greetBase = h < 5 ? t('Good night') : h < 12 ? t('Good morning') : h < 18 ? t('Good afternoon') : t('Good evening');
  const greet = name ? `${greetBase}, ${escapeHtml(name)}` : greetBase;
  const desc = (p) => projectMeta(p.dir).description;
  const cur = (p) => (keyOf(p.dir) === keyOf(state.workspace || '') ? ' current' : '');
  const card = (p) =>
    `<button class="hm-card${cur(p)}" data-dir="${escapeAttr(p.dir)}" data-find="${escapeAttr(`${p.name} ${desc(p) || ''}`.toLowerCase())}"><span class="hm-ic">${kindIcon(p.kind, 24, p.github)}</span><span class="hm-text"><b>${escapeHtml(p.name)}</b><small>${escapeHtml(desc(p) || shortPath(p.dir))}</small><small class="hm-stat" data-stats="${escapeAttr(p.dir)}"></small></span></button>`;
  const line = (p) =>
    `<button class="hm-line${cur(p)}" data-dir="${escapeAttr(p.dir)}" data-find="${escapeAttr(`${p.name} ${desc(p) || ''}`.toLowerCase())}"><span class="hm-ic sm">${kindIcon(p.kind, 18, p.github)}</span><span class="hm-text"><b>${escapeHtml(p.name)}</b><small>${escapeHtml(desc(p) || shortPath(p.dir))}</small></span><small class="hm-stat" data-stats="${escapeAttr(p.dir)}"></small></button>`;
  const fileLine = (f) =>
    `<button class="hm-line" data-file="${escapeAttr(f)}"><span class="hm-ic sm">${fileIcon(basename(f)).replace(/width="16" height="16"/, 'width="18" height="18"')}</span><span class="hm-text"><b>${escapeHtml(basename(f))}</b><small>${escapeHtml(shortPath(f.slice(0, f.length - basename(f).length - 1)))}</small></span></button>`;
  const dateRaw = new Date().toLocaleDateString(setting('language') || 'en', { weekday: 'long', day: 'numeric', month: 'long' });
  const date = dateRaw.charAt(0).toUpperCase() + dateRaw.slice(1);
  const tips = [
    t('Press Ctrl+Shift+A to search everything – commands, settings, files and projects.'),
    t('Drop a file or a folder onto Flux to open it.'),
    t('Ctrl+I opens Claude, your coding assistant.'),
    t('Plugins like Error Lens and Bookmarks are in Settings → Plugins.'),
    t('Every shortcut can be changed in Settings → Shortcuts.'),
    t('Open a .md file and press Ctrl+Shift+V to see the preview.'),
  ];
  const tip = tips[new Date().getDate() % tips.length];
  const noProjects = !pinned.length && !recent.length;
  el.innerHTML = `
    <div class="ob-aurora"><i></i><i></i><i></i></div><div class="ob-grain"></div>
    <div class="st-top drag"><div class="brand-mark">${icon('code', 15)}</div><span>flux</span>${navButtons()}<div class="grow"></div><button class="icon-btn no-drag hm-top-btn" data-act="settings" title="${t('Settings')}">${icon('settings', 16)}</button></div>
    <div class="st-scroll"><div class="st-inner hm">
      <header class="hm-hero">
        <div><small class="hm-date">${escapeHtml(date)}</small><h1>${greet}</h1><p class="st-sub">${t('What do you want to work on?')}</p></div>
        ${
          list.length
            ? `<div class="hm-stats" id="hm-stats">
          <div class="pj-tile"><span class="pj-tile-ic">${icon('clock', 14)}</span><b data-k="time">–</b><small>${t('coding time')}</small></div>
          <div class="pj-tile"><span class="pj-tile-ic">${icon('play', 13)}</span><b data-k="runs">–</b><small>${t('runs')}</small></div>
          <div class="pj-tile"><span class="pj-tile-ic">${icon('code', 14)}</span><b data-k="lines">–</b><small>${t('lines of code')}</small></div>
          <div class="pj-tile"><span class="pj-tile-ic">${icon('folder', 14)}</span><b data-k="projects">${list.length}</b><small>${t('projects')}</small></div>
        </div>`
            : ''
        }
      </header>
      <div class="hm-actions">
        <button class="hm-act primary" data-act="newproject"><span class="hm-act-ic">${icon('plus', 17)}</span><span><b>${t('New project')}</b><small>Ctrl+Shift+N</small></span></button>
        <button class="hm-act" data-act="open"><span class="hm-act-ic">${icon('folderOpen', 17)}</span><span><b>${t('Open folder')}</b><small>Ctrl+O</small></span></button>
        <button class="hm-act" data-act="openfile"><span class="hm-act-ic">${icon('file', 17)}</span><span><b>${t('Open file')}</b><small>${t('.md, .txt, any file')}</small></span></button>
        ${ghOn() ? `<button class="hm-act" data-act="github"><span class="hm-act-ic">${icon('github', 17)}</span><span><b>${t('From GitHub')}</b><small>${t('open a repository')}</small></span></button>` : ''}
      </div>
      <div class="hm-cols">
        <section class="hm-main">
          <div class="hm-h-row"><h3 class="hm-h">${t('Projects')}</h3>${noProjects ? '' : `<label class="hm-search">${icon('search', 14)}<input id="hm-q" placeholder="${t('Find a project…')}" spellcheck="false" autocomplete="off"></label>`}</div>
          ${
            noProjects
              ? `<div class="hm-empty">${icon('folder', 22)}<b>${t('No projects yet')}</b><small>${t('Start a new project, open a folder or get one from GitHub.')}</small></div>`
              : `${pinned.length ? `<div class="hm-sub">${icon('pin', 11)}${t('Pinned')}</div><div class="hm-cards">${pinned.map(card).join('')}</div>` : ''}
                 ${recent.length ? `<div class="hm-sub">${t('Recent')}</div><div class="hm-lines">${recent.map(line).join('')}</div>` : ''}
                 <div class="hm-none" hidden>${t('Nothing found')}</div>`
          }
        </section>
        <aside class="hm-side">
          <h3 class="hm-h">${t('Start something new')}<small>${state.workspace ? t('new file in {dir}', { dir: escapeHtml(basename(state.workspace)) }) : t('creates a new project')}</small></h3>
          <div class="hm-quick">${choices
            .map((c) => `<button class="hm-q" data-tpl="${c.id}"><span class="hm-ic sm">${fileIcon(c.icon).replace(/width="16" height="16"/, 'width="18" height="18"')}</span><span class="hm-text"><b>${t(c.title)}</b><small>${t(c.sub)}</small></span></button>`)
            .join('')}</div>
          ${recentFiles.length ? `<h3 class="hm-h">${t('Recent files')}</h3><div class="hm-lines">${recentFiles.map(fileLine).join('')}</div>` : ''}
          <div class="hm-tip"><span class="hm-tip-ic">${icon('sparkle', 14)}</span><span><b>${t('Tip')}</b><small>${escapeHtml(tip)}</small></span></div>
        </aside>
      </div>
      ${state.workspace ? `<button class="st-back" data-act="back">${t('Back to editor')} <kbd>Esc</kbd></button>` : ''}
    </div></div>`;
  const q = el.querySelector('#hm-q');
  if (q)
    q.oninput = () => {
      const v = q.value.trim().toLowerCase();
      let any = false;
      for (const x of el.querySelectorAll('.hm-main [data-find]')) {
        const show = !v || x.dataset.find.includes(v);
        x.style.display = show ? '' : 'none';
        any ||= show;
      }
      el.querySelector('.hm-none').hidden = any;
    };
  // Súčty za všetky projekty: čas, spustenia, riadky.
  Promise.all(list.map((p) => flux.projectStats(p.dir).catch(() => null))).then((all) => {
    const box = el.querySelector('#hm-stats');
    if (!box) return;
    const ok = all.filter(Boolean);
    const time = ok.reduce((n, st) => n + (st.time || 0), 0);
    box.querySelector('[data-k="time"]').textContent = time >= 60 ? formatTime(time) : '0 min';
    fitTiles(box);
    countUp(box.querySelector('[data-k="lines"]'), ok.reduce((n, st) => n + (st.lines || 0), 0));
    countUp(box.querySelector('[data-k="runs"]'), list.reduce((n, p) => n + (activity.runs(p.dir) || 0), 0));
  });
  for (const p of [...pinned, ...recent]) {
    flux.projectStats(p.dir).then((st) => {
      for (const x of el.querySelectorAll('.hm-stat')) {
        if (x.dataset.stats === p.dir) x.textContent = [langLine(p), `${st.files} ${st.files === 1 ? t('file') : t('files')}`, st.lastModified ? timeAgo(st.lastModified) : ''].filter(Boolean).join(' · ');
      }
    });
  }
  el.hidden = false;
  document.body.classList.add('start-open');
  navNote();
  el.oncontextmenu = async (e) => {
    const b = e.target.closest('[data-dir]');
    if (!b) return;
    e.preventDefault();
    await projectMenu(b.dataset.dir);
  };
  el.onclick = async (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    if (b.dataset.act === 'back') return closeStart();
    if (b.dataset.act === 'settings') return openSettings();
    if (b.dataset.dir) {
      closeStart();
      if (keyOf(b.dataset.dir) !== keyOf(state.workspace || '')) await setWorkspace(b.dataset.dir);
      else showProjectPage();
      return;
    }
    if (b.dataset.act === 'newfile') {
      closeStart();
      return newFile();
    }
    if (b.dataset.act === 'github') {
      closeStart();
      return gh.pickRepo();
    }
    if (b.dataset.act === 'newproject') {
      closeStart();
      return newProject();
    }
    if (b.dataset.act === 'openfile') return openFileDialog();
    if (b.dataset.file) {
      const f = await flux.openRecentFile(b.dataset.file);
      if (f) return openStandalone([f]);
      await flux.forgetRecentFile(b.dataset.file);
      return openStart();
    }
    if (b.dataset.act === 'open') {
      await openFolderDialog();
      if (state.workspace) openStart();
      return;
    }
    if (b.dataset.tpl === 'empty') return newStandaloneFile();
    const tpl = TEMPLATES.find((x) => x.id === b.dataset.tpl);
    if (!tpl) return;
    closeStart();
    // Bez otvoreného projektu: rovno nový projekt s touto šablónou (žiadne hľadanie priečinka).
    if (!state.workspace) {
      const lang = START_CHOICES.find((c) => c.id === tpl.id)?.lang;
      return newProject({ kind: lang === 'python' ? 'python' : lang === 'web' ? 'web' : 'empty', tpl });
    }
    newFile(state.workspace, tpl);
  };
}

function closeStart() {
  $('#start').hidden = true;
  navNote();
  document.body.classList.remove('start-open');
  if (state.active) editor.focus();
}

// ---------- uvítanie ----------
// Čísla, ktoré „nabehnú“ od nuly.
function countUp(el, to, format = fmtNum) {
  const start = performance.now();
  const dur = 700;
  const tick = (now) => {
    const k = Math.min(1, (now - start) / dur);
    el.textContent = format(Math.round(to * (1 - Math.pow(1 - k, 3))));
    if (k < 1) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

function renderWelcome() {
  if (state.active) return;
  navNote();
  updateProblems();
  const w = $('#welcome');
  w.hidden = false;
  const ws = state.workspace;
  const big = (svg, size) => svg.replace(/width="16" height="16"/, `width="${size}" height="${size}"`);
  const projectIcon = (kind) => big(kind === 'python' ? fileIcon('a.py') : kind === 'web' ? fileIcon('a.html') : icon('folder', 16), 40);
  if (!ws) {
    w.innerHTML = `
      <div class="welcome-inner">
        <div class="wl-head"><span class="wl-icon"><span class="brand-mark big">${icon('code', 22)}</span></span>
        <div><h1>flux</h1><p class="sub">${t('Open a project folder and run your code with one click.')}</p></div></div>
        <div class="welcome-actions">
          <button class="primary" data-act="newproject">${icon('plus')}${t('New project')}</button>
          <button data-act="open">${icon('folderOpen')}${t('Open folder')}</button>
          <button data-act="openfile">${icon('file')}${t('Open file')}</button>
        </div>
      </div>`;
  } else {
    const meta = projectMeta(ws);
    const kind = state.projectKind || meta.kind || 'folder';
    const runs = activity.runs(ws);
    // Čo sa dá v projekte robiť – podľa typu (web má náhľad, Python spúšťanie…).
    const acts =
      kind === 'web'
        ? [['open-main', 'globe', t('Open page')], ['live', 'monitor', t('Live preview')]]
        : kind === 'folder'
          ? []
          : [['open-main', 'code', t('Open main file')], ['run-main', 'play', t('Run it')]];
    const info = (state.projectList || []).find((p) => keyOf(p.dir) === keyOf(ws)) || {};
    const langs = (info.langs || []).filter((l) => KIND_LANG_NAMES[l]);
    const tile = (k, ic, labelText, value = '–') => `<div class="pj-tile"><span class="pj-tile-ic">${icon(ic, 15)}</span><b data-k="${k}">${value}</b><small>${labelText}</small></div>`;
    w.innerHTML = `
      <div class="welcome-inner pj pj2">
        <header class="pj-top">
          <span class="wl-icon">${kindIcon(kind, 40, info.github)}</span>
          <div class="wl-title">
            <h1>${escapeHtml(basename(ws))}</h1>
            <p class="pj-desc${meta.description ? '' : ' empty'}" data-edit-desc title="${t('Click to edit')}">${escapeHtml(meta.description || t('Add a short description…'))}</p>
            ${langs.length ? `<div class="pj-langs">${langs.map((l) => `<span class="pj-lang">${fileIcon(KIND_FILE[l] || 'a.txt')}${escapeHtml(KIND_LANG_NAMES[l])}</span>`).join('')}${info.github ? `<span class="pj-lang">${icon('git', 12)}GitHub</span>` : ''}</div>` : ''}
          </div>
          <div class="pj-actions">
            ${acts.map(([act, ic, label], i) => `<button class="s-btn${i ? '' : ' primary'}" data-act="${act}">${icon(ic, 14)}${label}</button>`).join('')}
            <button class="s-btn${acts.length ? '' : ' primary'}" data-act="new">${icon('filePlus', 14)}${t('New file')}</button>
            <button class="icon-btn" data-act="quick" title="${t('Find file')} (Ctrl+P)">${icon('search', 16)}</button>
          </div>
        </header>
        <div class="pj-tiles" id="wl-stats">
          ${tile('time', 'clock', t('coding time'))}
          ${tile('runs', 'play', t('runs'), runs)}
          ${tile('lines', 'code', t('lines of code'))}
          ${tile('files', 'file', t('files'))}
          ${tile('changed', 'refresh', t('last change'))}
        </div>
        <div class="pj-cols">
          <section class="pj-panel"><div class="pj-files" id="pj-files"><div class="pj-h"><span>${t('Files')}</span></div><div class="s-loading"><span class="spin"></span></div></div></section>
          <div class="pj-side">
            <section class="pj-panel pj-todo">
              <div class="pj-h"><span>${t('To-do')}</span><small id="pj-count"></small></div>
              <form class="pj-add" id="pj-add"><span class="pj-plus">${icon('plus', 14)}</span><input id="pj-new" placeholder="${t('Add a task and press Enter…')}" autocomplete="off" spellcheck="false" maxlength="200"></form>
              <ul class="pj-list" id="pj-list"></ul>
            </section>
            ${ghOn() ? '<section class="pj-panel pj-git" id="pj-git"></section>' : ''}
          </div>
        </div>
      </div>`;
    renderTodos(ws);
    if (ghOn()) gh?.renderCard($('#pj-git'));
    $('#pj-add').onsubmit = async (e) => {
      e.preventDefault();
      const input = $('#pj-new');
      const text = input.value.trim();
      if (!text) return;
      input.value = '';
      await saveProjectMeta(ws, { todos: [...(projectMeta(ws).todos || []), { id: Date.now(), text, done: false }] });
      renderTodos(ws, true);
    };
    flux.projectStats(ws).then((st) => {
      if (state.workspace !== ws || !$('#wl-stats')) return;
      const q = (k) => $(`#wl-stats [data-k="${k}"]`);
      q('time').textContent = st.time >= 60 ? formatTime(st.time) : '0 min';
      countUp(q('lines'), st.lines);
      countUp(q('files'), st.files);
      countUp(q('runs'), runs);
      q('changed').textContent = st.lastModified ? timeAgo(st.lastModified) : '–';
      fitTiles($('#wl-stats'));
    });
    // Hlavné súbory projektu (pri webe stránky .html) na jeden klik.
    flux.listAll().then((files) => {
      if (state.workspace !== ws || !$('#pj-files')) return;
      state.projectFiles = files;
      const main = mainFileOf(files, kind);
      const row = (f) => {
        const rel = relPath(f);
        const dir = rel.slice(0, rel.length - basename(f).length).replace(/[\\/]$/, '');
        return `<button class="pj-row${f === main ? ' main' : ''}" data-open="${escapeAttr(f)}">${fileIcon(basename(f))}<b>${escapeHtml(basename(f))}</b><small>${escapeHtml(dir)}</small>${f === main ? `<span class="pj-main">${t('main')}</span>` : ''}</button>`;
      };
      // Web: stránky, štýly a skripty zvlášť; inak súbory jazykov projektu.
      const exts = [...new Set([...(MAIN_EXT[kind] || []), ...langs.flatMap((l) => MAIN_EXT[l] || [])])];
      const groups =
        kind === 'web'
          ? [[t('Pages'), /\.html?$/i], [t('Styles'), /\.(css|scss|sass|less)$/i], [t('Scripts'), /\.(m?js|ts)$/i]]
          : [[t('Files'), new RegExp(`\\.(${(exts.length ? exts : ['py', 'html', 'js', 'md', 'txt']).join('|')})$`, 'i')]];
      const html = groups
        .map(([label, re]) => [label, files.filter((f) => re.test(f))])
        .filter(([, list]) => list.length)
        .map(([label, list]) => `<div class="pj-group"><div class="pj-h"><span>${label}</span><small>${list.length}</small></div><div class="pj-rows">${list.slice(0, 12).map(row).join('')}</div>${list.length > 12 ? `<button class="pj-more" data-act="quick">${t('{n} more', { n: list.length - 12 })}</button>` : ''}</div>`)
        .join('');
      if (!html) $('#pj-files').innerHTML = `<div class="pj-h"><span>${t('Files')}</span></div><div class="pj-empty">${t('This folder is empty.')}</div>`;
      if (html) $('#pj-files').innerHTML = `<div class="pj-groups">${html}</div>`;
    });
  }
  w.onclick = async (e) => {
    const desc = e.target.closest('[data-edit-desc]');
    if (desc) return editDescription(desc);
    const todo = e.target.closest('[data-todo]');
    if (todo) return toggleTodo(state.workspace, Number(todo.dataset.todo), e.target.closest('[data-del]'));
    const b = e.target.closest('button');
    if (!b) return;
    if (b.dataset.open) return openFile(b.dataset.open);
    if (b.dataset.act === 'open-main' || b.dataset.act === 'run-main' || b.dataset.act === 'live') {
      const files = state.projectFiles || (await flux.listAll());
      const main = mainFileOf(files, state.projectKind);
      if (!main) return newFile();
      await openFile(main);
      if (b.dataset.act === 'run-main') run();
      if (b.dataset.act === 'live' && !state.live) toggleLive();
      return;
    }
    if (b.dataset.act === 'open') openFolderDialog();
    else if (b.dataset.act === 'openfile') openFileDialog();
    else if (b.dataset.act === 'new') newFile();
    else if (b.dataset.act === 'quick') quickOpen();
    else if (b.dataset.act === 'start') openStart();
    else if (b.dataset.act === 'newproject') newProject();
  };
}

// Hlavný súbor projektu: index.html, main.py, Main.java…
const MAIN_EXT = { python: ['py'], web: ['html', 'htm'], node: ['js', 'mjs'], java: ['java'], cpp: ['cpp', 'c'], go: ['go'], csharp: ['cs'], rust: ['rs'], ruby: ['rb'], php: ['php'], lua: ['lua'] };
function mainFileOf(files, kind) {
  const exts = MAIN_EXT[kind];
  if (!exts) return null;
  const mine = files.filter((f) => exts.includes(extOf(f)));
  const pref = /^(index|main|program|app)\./i;
  return mine.find((f) => pref.test(basename(f)) && !relPath(f).includes('/') && !relPath(f).includes('\\')) || mine.find((f) => pref.test(basename(f))) || mine[0] || null;
}

function relPath(f) {
  return state.workspace && f.startsWith(state.workspace) ? f.slice(state.workspace.length + 1) : basename(f);
}

// Zoznam úloh projektu – odškrtnuté idú dole.
function renderTodos(ws, animateFirst = false) {
  const list = $('#pj-list');
  if (!list) return;
  const todos = projectMeta(ws).todos || [];
  const done = todos.filter((x) => x.done).length;
  $('#pj-count').textContent = todos.length ? `${done} / ${todos.length}` : '';
  const sorted = [...todos.filter((x) => !x.done), ...todos.filter((x) => x.done)];
  list.innerHTML = sorted.length
    ? sorted
        .map(
          (x, i) =>
            `<li class="pj-item${x.done ? ' done' : ''}${animateFirst && i === todos.filter((y) => !y.done).length - 1 && !x.done ? ' new' : ''}" data-todo="${x.id}"><span class="pj-check">${icon('check', 12)}</span><span class="pj-text">${escapeHtml(x.text)}</span><button class="pj-del" data-del title="${t('Remove')}">${icon('x', 13)}</button></li>`,
        )
        .join('')
    : `<li class="pj-empty">${t('Nothing here yet. Plan your next steps!')}</li>`;
}

async function toggleTodo(ws, id, del) {
  let todos = projectMeta(ws).todos || [];
  if (del) {
    const li = document.querySelector(`[data-todo="${id}"]`);
    li?.classList.add('leaving');
    await new Promise((r) => setTimeout(r, 180));
    todos = todos.filter((x) => x.id !== id);
  } else {
    todos = todos.map((x) => (x.id === id ? { ...x, done: !x.done } : x));
  }
  await saveProjectMeta(ws, { todos });
  renderTodos(ws);
}

// Popis projektu – klik a píš, Enter uloží.
function editDescription(el) {
  const ws = state.workspace;
  const input = document.createElement('input');
  input.className = 'pj-desc-input';
  input.value = projectMeta(ws).description || '';
  input.maxLength = 160;
  input.placeholder = t('Add a short description…');
  el.replaceWith(input);
  input.focus();
  let done = false;
  const save = async (keep) => {
    if (done) return;
    done = true;
    if (keep) await saveProjectMeta(ws, { description: input.value.trim() });
    renderWelcome();
  };
  input.onkeydown = (e) => {
    if (e.key === 'Enter') save(true);
    if (e.key === 'Escape') save(false);
    e.stopPropagation();
  };
  input.onblur = () => save(true);
}

// Vetva a počet zmien v stavovom riadku (klik → stránka projektu s kartou GitHub).
function renderGitStatus(st) {
  const el = $('#st-git');
  if (!el) return;
  if (!ghOn()) st = null;
  el.hidden = !st?.repo;
  if (!st?.repo) return;
  el.innerHTML = `${icon('git', 13)}<span>${escapeHtml(st.branch || 'main')}</span>${st.changes ? `<span class="git-n">${st.changes}</span>` : ''}${st.ahead ? `<span class="git-s">↑${st.ahead}</span>` : ''}`;
  el.title = st.changes ? t('{n} changed file(s) – click to save them to GitHub', { n: st.changes }) : t('Git: everything is saved');
}

// Zapnutie / vypnutie GitHub pluginu. Git sa pri zapnutí doinštaluje, ak chýba.
async function setGitHubPlugin(on) {
  if (on) {
    const st = await tools.status(true);
    const git = st?.list?.find((x) => x.id === 'git');
    if (git && !git.installed) {
      if (!tools.canInstall()) {
        toast(t('GitHub needs Git. Install it from git-scm.com, then try again.'), 'error', 9000, { label: 'git-scm.com', run: () => flux.openExternal('https://git-scm.com/downloads') });
        return false;
      }
      toast(t('Installing Git – GitHub needs it…'), 'info', 6000);
      if (!(await tools.install('git'))) return false;
    }
  }
  await saveSettings({ githubPlugin: on });
  toast(on ? t('GitHub is installed. Sign in under Settings → GitHub.') : t('GitHub was removed. Your projects and files stay.'), 'ok', 6000);
  if (on) gh?.refreshStatus();
  else renderGitStatus(null);
  if (!state.active) renderWelcome();
  // Karta GitHub v nastaveniach sa objaví / zmizne.
  if (!$('#settings').hidden) {
    state.settingsTab = 'plugins';
    openSettings();
  }
  return true;
}

function renderGitHubSettings() {
  if (ghOn() && $('#s-github')) gh?.renderSettings($('#s-github'));
}

// ---------- klávesové skratky ----------
function keybindings() {
  window.addEventListener(
    'keydown',
    (e) => {
      if (window.fluxRecordingKeys) return;
      if (e.altKey && !e.ctrlKey && !e.shiftKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight') && !palette && $('#onboard').hidden) {
        e.preventDefault();
        e.stopPropagation();
        return e.key === 'ArrowLeft' ? goBack() : goForward();
      }
      if (palette) {
        if (e.key === 'Escape') {
          e.preventDefault();
          closePalette();
        } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
          e.preventDefault();
          const n = palette.shown.length;
          if (n) {
            palette.sel = (palette.sel + (e.key === 'ArrowDown' ? 1 : n - 1)) % n;
            palette.render();
          }
        } else if (e.key === 'Enter') {
          e.preventDefault();
          pick(palette.sel);
        }
        return;
      }
      if (!$('#newproj').hidden || !$('#onboard').hidden) return;
      if (!$('#tour').hidden) {
        if (e.key === 'Escape') $('#tour').hidden = true;
        return;
      }
      if (!$('#glance').hidden) {
        if (e.key === 'Escape') {
          e.preventDefault();
          $('#glance').hidden = true;
        }
        return;
      }
      // Nastavenia môžu byť otvorené aj nad domovskou obrazovkou – Esc zavrie najprv ich.
      if (!$('#settings').hidden) {
        if (e.key === 'Escape') {
          e.preventDefault();
          closeSettings();
        }
        return;
      }
      if (!$('#start').hidden) {
        // Na domovskej obrazovke fungujú skratky, ktoré sú na nej napísané.
        const ctrl = e.ctrlKey || e.metaKey;
        const key = e.key.toLowerCase();
        let used = true;
        if (e.key === 'Escape' && state.workspace) closeStart();
        else if (ctrl && e.shiftKey && key === 'n') {
          closeStart();
          newProject();
        } else if (ctrl && e.altKey && e.code === 'KeyO') openFileDialog();
        else if (ctrl && !e.shiftKey && key === 'o') openFolderDialog();
        else if (ctrl && e.shiftKey && key === 'a') searchEverything();
        else if (ctrl && e.shiftKey && key === 'p') openCommandPalette();
        else if (ctrl && e.key === ',') openSettings();
        else used = false;
        if (used) e.preventDefault();
        return;
      }
      const ctrl = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();
      let handled = true;
      if (ctrl && e.key === ',') openSettings();
      else if (e.key === 'F2' && !e.target.closest?.('.monaco-editor')) {
        const p = state.selected || state.active?.path;
        const entry = p && [...state.dirCache.values()].flat().find((x) => keyOf(x.path) === keyOf(p));
        if (entry) renameItem(entry);
      }
      else if (e.key === 'F5' && e.shiftKey) stop();
      else if (e.key === 'F5' || (ctrl && e.key === 'Enter')) run();
      else if (ctrl && e.shiftKey && key === 'p') openCommandPalette();
      else if (ctrl && e.shiftKey && key === 'a') searchEverything();
      else if (ctrl && !e.shiftKey && key === 'p') quickOpen();
      else if (ctrl && key === 's') e.shiftKey ? saveAll() : saveTab(activeTab()).then(() => {
          announceProblems();
          const tb = activeTab();
          if (!tb) return;
        });
      else if (ctrl && e.altKey && e.code === 'KeyO') openFileDialog();
      else if (ctrl && e.shiftKey && e.code === 'KeyV' && state.active && ['md', 'markdown'].includes(extOf(state.active.path))) toggleMdPreview();
      else if (ctrl && key === 'o') openFolderDialog();
      else if (ctrl && e.shiftKey && key === 'n') newProject();
      else if (ctrl && key === 'n') newFile();
      else if (ctrl && key === 'w') state.active && closeTab(state.active);
      else if (ctrl && key === 'b') toggleCompact();
      else if (e.key === 'F11' && !ctrl && !e.shiftKey && !e.altKey) toggleFocus();
      else if (ctrl && !e.shiftKey && key === 'i') aiPanel.toggle();
      else if (ctrl && e.shiftKey && e.code === 'Backquote') showPanelTab('shell');
      else if (ctrl && (key === 'j' || e.key === '`' || e.key === ';')) showPanel($('#panel').classList.contains('collapsed'));
      else if (ctrl && e.key === 'Tab' && state.tabs.length > 1) {
        const i = state.tabs.indexOf(state.active);
        activate(state.tabs[(i + (e.shiftKey ? state.tabs.length - 1 : 1)) % state.tabs.length]);
      } else if (e.altKey && key === 'l') toggleLive();
      else if (ctrl && (e.key === '=' || e.key === '+')) setFontSize(1);
      else if (ctrl && e.key === '-') setFontSize(-1);
      else handled = false;
      if (handled) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
    true,
  );
  $('#overlay').addEventListener('mousedown', (e) => {
    if (e.target.id === 'overlay') closePalette();
  });
}

// ---------- ponuka a hľadanie v hornej lište ----------
async function setLayout(patch) {
  await saveSettings(patch);
  applyCustomization();
  // editor, terminál a náhľad sa prispôsobia novému miestu
  requestAnimationFrame(() => {
    editor?.layout();
    showPanel(!$('#panel').classList.contains('collapsed'));
  });
}
function appMenus() {
  const panelOpen = !$('#panel').classList.contains('collapsed');
  const ed = (id) => () => {
    editor.focus();
    editor.getAction(id)?.run();
  };
  return [
    { id: 'home', label: t('Home'), icon: 'template', run: () => openStart() },
    {
      id: 'file',
      label: t('File'),
      items: [
        [t('New file…'), () => newFile(), 'Ctrl+N'],
        [t('New project…'), () => (closeStart(), newProject()), 'Ctrl+Shift+N'],
        [t('New folder…'), () => newFolder()],
        '-',
        [t('Open folder…'), openFolderDialog, 'Ctrl+O'],
        [t('Open file…'), openFileDialog, 'Ctrl+Alt+O'],
        [t('Quick open file…'), quickOpen, 'Ctrl+P'],
        '-',
        [t('Save'), () => saveTab(activeTab()), 'Ctrl+S', { disabled: !state.active }],
        [t('Save all'), saveAll, 'Ctrl+Shift+S'],
        [t('Close file'), () => state.active && closeTab(state.active), 'Ctrl+W', { disabled: !state.active }],
        '-',
        [t('Home'), () => openStart()],
        [t('Settings'), openSettings, 'Ctrl+,'],
      ],
    },
    {
      id: 'edit',
      label: t('Edit'),
      items: [
        [t('Undo'), ed('undo'), 'Ctrl+Z', { disabled: !state.active }],
        [t('Redo'), ed('redo'), 'Ctrl+Y', { disabled: !state.active }],
        '-',
        [t('Find'), ed('actions.find'), 'Ctrl+F', { disabled: !state.active }],
        [t('Replace'), ed('editor.action.startFindReplaceAction'), 'Ctrl+H', { disabled: !state.active }],
        '-',
        [t('Toggle comment'), ed('editor.action.commentLine'), 'Ctrl+/', { disabled: !state.active }],
        [t('Format document'), ed('editor.action.formatDocument'), 'Shift+Alt+F', { disabled: !state.active }],
      ],
    },
    {
      id: 'view',
      label: t('View'),
      items: [
        [t('Search everything…'), () => searchEverything(), 'Ctrl+Shift+A'],
        [t('Commands'), openCommandPalette, 'Ctrl+Shift+P'],
        '-',
        [t('Sidebar'), toggleCompact, 'Ctrl+B', { checked: !document.body.classList.contains('compact') }],
        [t('Panel'), () => showPanel(!panelOpen), 'Ctrl+J', { checked: panelOpen }],
        [t('AI assistant'), () => aiPanel.toggle(), 'Ctrl+I', { checked: !$('#ai').hidden }],
        [t('Focus mode (only the code)'), toggleFocus, 'F11'],
        '-',
        t('Panel position'),
        ...[['bottom', t('Bottom')], ['right', t('Right')], ['left', t('Left')]].map(([v, l]) => [l, () => setLayout({ panelPos: v }), '', { checked: setting('panelPos') === v, radio: true }]),
        t('Sidebar position'),
        ...[['left', t('Left')], ['right', t('Right')]].map(([v, l]) => [l, () => setLayout({ sidePos: v }), '', { checked: setting('sidePos') === v, radio: true }]),
        '-',
        [t('Menu bar'), () => setLayout({ menuBar: !setting('menuBar') }), '', { checked: !!setting('menuBar') }],
        [t('Search button'), () => setLayout({ showSearch: !setting('showSearch') }), '', { checked: !!setting('showSearch') }],
        '-',
        [t('Toggle light / dark theme'), toggleTheme],
      ],
    },
    {
      id: 'run',
      label: t('Run'),
      items: [
        [t('Run current file'), run, 'F5', { disabled: !state.active }],
        [t('Stop program'), stop, 'Shift+F5'],
        '-',
        [t('Live Server'), toggleLive, 'Alt+L'],
        [t('Terminal'), () => showPanelTab('shell')],
      ],
    },
    {
      id: 'help',
      label: t('Help'),
      items: [
        [t('Feature tour'), () => onboarding.startTour()],
        [t('Shortcuts'), () => ((state.settingsTab = 'keys'), openSettings())],
        [t('Release notes'), () => ((state.settingsTab = 'about'), openSettings())],
        '-',
        [t('Website'), () => flux.openExternal('https://pantr1x.github.io/Flux/')],
        [t('Report a problem'), () => flux.openExternal('https://github.com/pantr1x/Flux/issues/new')],
      ],
    },
  ];
}
function setupMenubar() {
  // Menu je v logu „flux“ (a v ☰, keď je bočný panel skrytý); riadok File/Edit/View… je voliteľný.
  const brand = $('.brand');
  brand.insertAdjacentHTML('beforeend', `<span class="brand-chev">${icon('chevron', 11)}</span>`);
  brand.title = t('Menu');
  $('#btn-menu').innerHTML = icon('menu', 16);
  $('#btn-menu').title = t('Menu');
  menubar = createMenubar({ bar: $('#menubar'), triggers: [brand, $('#btn-menu')], icon, esc: escapeHtml, getMenus: appMenus });
  menubar.render();
  const s = $('#topsearch');
  s.innerHTML = icon('search', 16);
  s.title = `${t('Search files, commands, settings and projects')} (Ctrl+Shift+A)`;
  s.onclick = () => searchEverything();
}

// ---------- zmena veľkosti panelov ----------
function resizer(handle, onMove, onEnd) {
  handle.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    document.body.classList.add('resizing');
    const move = (ev) => onMove(ev);
    const up = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
      document.body.classList.remove('resizing');
      onEnd?.();
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
  });
}

function layoutEvents() {
  const root = document.documentElement;
  if (state.settings.sideWidth) root.style.setProperty('--side-w', `${state.settings.sideWidth}px`);
  resizer(
    $('#side-resizer'),
    (e) => root.style.setProperty('--side-w', `${Math.min(480, Math.max(180, setting('sidePos') === 'right' ? innerWidth - e.clientX : e.clientX))}px`),
    () => saveSettings({ sideWidth: parseInt(getComputedStyle(root).getPropertyValue('--side-w')) }),
  );
  resizer($('#preview-resizer'), (e) => {
    const rect = $('#workarea').getBoundingClientRect();
    const w = Math.min(rect.width - 200, Math.max(240, rect.right - e.clientX));
    $('#preview').style.width = `${w}px`;
  });
  resizer($('#md-resizer'), (e) => {
    const rect = $('#workarea').getBoundingClientRect();
    $('#mdview').style.width = `${Math.min(rect.width - 200, Math.max(240, rect.right - e.clientX))}px`;
  });
  $('#md-toggle').innerHTML = `${icon('file', 13)}<span>${t('Preview')}</span>`;
  $('#md-toggle').onclick = () => toggleMdPreview();
  $('#mdview-close').innerHTML = icon('x', 15);
  $('#mdview-close').onclick = () => closeMdPreview();
  $('#mdview-body').addEventListener('click', (e) => {
    const copy = e.target.closest('[data-copy]');
    if (copy) {
      navigator.clipboard.writeText(copy.closest('.ai-code').querySelector('code').textContent);
      return toast(t('Copied.'), 'ok', 1500);
    }
    const a = e.target.closest('a[href]');
    if (a) {
      e.preventDefault();
      flux.openExternal(a.href);
    }
  });
  resizer($('#ai-resizer'), (e) => {
    const rect = $('#workarea').getBoundingClientRect();
    const w = Math.min(rect.width - 240, Math.max(300, rect.right - e.clientX));
    $('#ai').style.width = `${w}px`;
  }, () => saveSettings({ aiWidth: parseInt($('#ai').style.width) }));
  if (state.settings.aiWidth) $('#ai').style.width = `${state.settings.aiWidth}px`;
  resizer($('#panel-resizer'), (e) => {
    const rect = $('#card').getBoundingClientRect();
    showPanel(true);
    // Panel vpravo / vľavo: mení sa šírka, dole výška.
    if (setting('panelPos') !== 'bottom') {
      const w = Math.min(rect.width - 320, Math.max(240, setting('panelPos') === 'right' ? rect.right - e.clientX : e.clientX - rect.left));
      return document.documentElement.style.setProperty('--panel-w', `${w}px`);
    }
    const h = Math.min(rect.height - 120, Math.max(90, rect.bottom - 28 - e.clientY));
    $('#panel').style.height = `${h}px`;
  }, () => (setting('panelPos') !== 'bottom' ? saveSettings({ panelWidth: parseInt(getComputedStyle(root).getPropertyValue('--panel-w')) }) : saveSettings({ panelHeight: parseInt($('#panel').style.height) })));
  if (state.settings.panelHeight) $('#panel').style.height = `${state.settings.panelHeight}px`;
  if (state.settings.panelWidth) root.style.setProperty('--panel-w', `${state.settings.panelWidth}px`);
  setupMenubar();

  // Kompaktný režim: panel sa vysunie pri nabehnutí k ľavému okraju.
  $('#peek-zone').addEventListener('mouseenter', () => document.body.classList.add('peek'));
  $('#sidebar').addEventListener('mouseleave', () => document.body.classList.remove('peek'));
  if (state.settings.compact) document.body.classList.add('compact');

  $('#btn-compact').onclick = toggleCompact;
  $('#btn-expand').onclick = toggleCompact;
  $('#btn-theme').onclick = toggleTheme;
  $('#btn-palette').onclick = openCommandPalette;
  $('#btn-settings').onclick = openSettings;
  $('#btn-run').onclick = run;
  $('#btn-stop').onclick = stop;
  $('#btn-live').onclick = toggleLive;
  $('#btn-ai').onclick = () => aiPanel.toggle();
  document.querySelectorAll('.panel-tab').forEach((b) => (b.onclick = () => showPanelTab(b.dataset.ptab)));
  $('#btn-clear').onclick = () => {
    if (state.panelTab === 'shell') return shTerm?.clear();
    term.reset();
    term.write(state.running ? '' : '\x1b[?25l');
    setHint([]);
    // Po vymazaní už nesvieti „Error“ ani „Done“.
    if (!state.running) {
      $('#run-state').className = 'run-state';
      $('#run-state').textContent = '';
    }
  };
  $('#btn-panel').onclick = () => showPanel($('#panel').classList.contains('collapsed'));
  $('#st-python').onclick = pythonMenu;
  $('#st-autosave').onclick = toggleAutosave;
  $('#st-problems').onclick = showProblems;
  $('#st-git').onclick = () => showProjectPage();
  $('#st-install').onclick = () => {
    state.settingsTab = 'tools';
    openSettings();
  };
  $('#st-live').onclick = () => state.live && flux.openExternal($('#preview-frame').src || state.live.url);
  $('#btn-preview-reload').onclick = () => {
    const f = $('#preview-frame');
    if (f.src) f.src = f.src;
  };
  $('#btn-preview-external').onclick = () => flux.openExternal($('#preview-frame').src || state.live?.url);
  $('#btn-preview-close').onclick = stopLive;
  $('#devices').onclick = (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    for (const x of $('#devices').children) x.classList.toggle('active', x === b);
    previewDevice.id = b.dataset.dev;
    previewDevice.landscape = false;
    layoutPreview();
  };
  $('#btn-preview-rotate').onclick = () => {
    previewDevice.landscape = !previewDevice.landscape;
    layoutPreview();
  };
  $('#btn-preview-shot').onclick = previewScreenshot;
  $('#btn-preview-phone').onclick = (e) => {
    e.stopPropagation();
    togglePhonePopover();
  };
  new ResizeObserver(() => layoutPreview()).observe(document.querySelector('.preview-stage'));
  $('#essentials').onclick = (e) => {
    const cmd = e.target.closest('button')?.dataset.cmd;
    if (cmd === 'new-file') newFile();
    else if (cmd === 'new-folder') newFolder();
    else if (cmd === 'refresh') refreshTree();
    else if (cmd === 'collapse') {
      state.expanded.clear();
      renderTree();
    }
  };
}

// ---------- štart ----------
async function main() {
  const init = await flux.init();
  state.platform = init.platform;
  setupFancySelects();
  state.home = init.home || '';
  state.version = init.version || '';
  state.mica = !!init.mica;
  state.material = init.material;
  state.hasWallpaper = !!init.hasWallpaper;
  state.settings = init.settings;
  // GitHub je odteraz plugin: kto už bol prihlásený, má ho rovno zapnutý.
  if (state.settings.githubPlugin === undefined) {
    try {
      await saveSettings({ githubPlugin: !!(await flux.ghInfo()).connected });
    } catch {}
  }
  // Jazyk rozhrania (stiahnutý z GitHubu) ešte pred vykreslením.
  try {
    setLocale(await flux.i18nCurrent());
  } catch {}
  translateDom();
  // Jednorazovo: staré verzie ukladali fialovú ako predvolenú – nová predvolená je čiernobiela (ako Zen).
  if (!state.settings.monoMigrated) await saveSettings({ monoMigrated: true, accent: 'mono', accents: {} });
  document.body.classList.add(`platform-${state.platform}`);

  setIcons();
  setupWallpaper();
  activity = createActivity({ getSettings: () => state.settings, saveSettings });
  tools = createTools({ toast });
  onboarding = createOnboarding({
    colorize: (code, lang) => monaco.editor.colorize(code, lang, { tabSize: 4 }),
    fileIcon,
    icon,
    setCodeTheme,
    setAccent,
    setDarkLift: (v) => {
      state.settings = { ...state.settings, darkLift: v };
      applyCustomization();
      clearTimeout(liftTimer);
      liftTimer = setTimeout(() => saveSettings({ darkLift: v }), 300);
    },
    accents: Object.keys(ACCENTS).slice(0, 8),
    accentHex,
    currentAccent,
    getSettings: () => state.settings,
    saveSettings,
    toast,
    tools,
    keymap: () => keymap,
    githubOn: ghOn,
    setGitHubPlugin,
    installPlugin: async (id) => {
      await flux.pluginInstall(id);
      await pluginHost?.refresh(id);
    },
  });
  await loadCustomThemes();
  createEditor();
  userKeys = createUserShortcuts({
    editor,
    toast,
    saveAll,
    // Príkazy Fluxu, ktoré sa dajú dať na vlastnú skratku ("command": "run").
    commands: (userKeysCommands = {
      run,
      stop,
      save: () => saveTab(activeTab()),
      saveAll,
      live: toggleLive,
      newFile: () => newFile(),
      newProject: () => newProject(),
      openFolder: openFolderDialog,
      openFile: openFileDialog,
      mdPreview: () => toggleMdPreview(),
      quickOpen,
      palette: openCommandPalette,
      settings: openSettings,
      home: openStart,
      format: () => editor.getAction('editor.action.formatDocument')?.run(),
      sidebar: toggleCompact,
      focus: toggleFocus,
      panel: () => showPanel($('#panel').classList.contains('collapsed')),
      theme: toggleTheme,
      projectPage: showProjectPage,
      searchAll: () => searchEverything(),
      ai: () => aiPanel.toggle(),
    }),
    getContext: () => {
      const tab = activeTab();
      const file = tab?.path || '';
      const pos = editor.getPosition();
      const sel = editor.getModel() && editor.getSelection() ? editor.getModel().getValueInRange(editor.getSelection()) : '';
      const word = editor.getModel() && pos ? editor.getModel().getWordAtPosition(pos)?.word || '' : '';
      return { file, fileName: basename(file), dir: file ? file.slice(0, file.length - basename(file).length - 1) : state.workspace || '', workspace: state.workspace || '', selection: sel, word, line: pos?.lineNumber ?? '' };
    },
  });
  // Pluginy (Nastavenia → Plugins)
  const activeFileInfo = () => {
    const tab = activeTab();
    if (!tab) return null;
    return { path: tab.path, name: basename(tab.path), language: tab.model.getLanguageId(), text: tab.model.getValue(), selection: editor.getModel() ? tab.model.getValueInRange(editor.getSelection()) : '' };
  };
  pluginHost = createPluginHost({
    monaco,
    editor,
    toast,
    getActiveFile: activeFileInfo,
    getSettings: () => state.settings,
    saveSettings,
    runCommand: (id) => userKeysCommands[id]?.(),
    isOverridden: (full) => !!keymap?.overridden(`plugin:${full}`),
    getWorkspace: () => state.workspace,
    addTheme: addPluginTheme,
  });
  updatesUI = createUpdatesUI({ toast, getSetting: setting, saveSettings });
  pluginsUI = createPluginsUI({
    host: pluginHost,
    toast,
    openProject: (dir) => (closeSettings(), setWorkspace(dir)),
    builtins: [
      {
        id: 'github',
        name: 'GitHub',
        icon: 'github',
        needs: t('needs Git'),
        description: t('Sign in with GitHub, open your repositories as projects, save changes with commit & push and publish new projects. Installs Git if it is missing.'),
        details: [
          t('Sign in with your GitHub account – Flux shows a short code you enter on github.com.'),
          t('Open any of your repositories as a project with one click.'),
          t('Save your changes with commit & push right from the sidebar.'),
          t('Publish a new project to GitHub with one click.'),
          t('If Git is missing on your computer, Flux installs it for you.'),
        ],
        enabled: ghOn,
        set: setGitHubPlugin,
      },
      {
        id: 'together',
        name: 'Flux Together (Wi-Fi)',
        icon: 'globe',
        needs: t('same Wi-Fi'),
        description: t('Work together with friends on the same Wi-Fi: see who has which file open, where their cursor is and what they are typing – live. Nothing goes to the internet.'),
        details: [
          t('Everyone on the same Wi-Fi enters the same room code – that is all.'),
          t('A green dot shows where your friends are working: next to the project, every folder on the way and the file.'),
          t('See which file each friend has open, on which line and what they are typing – live.'),
          t('Click a friend to jump to their file and line.'),
          t('It connects directly inside your network – nothing goes to the internet and there are no limits.'),
        ],
        enabled: () => setting('togetherPlugin') === true,
        set: async (on) => {
          if (!on) await together?.stop();
          await saveSettings({ togetherPlugin: on });
          together?.refresh();
          toast(on ? t('Together is on – find it in the sidebar under your files.') : t('Together was removed.'), 'ok', 5000);
          return true;
        },
      },
    ],
  });

  // Zmenené vstavané skratky (Nastavenia → Skratky).
  keymap = createKeymap({
    toast,
    getBindings: allBindings,
    getOverrides: () => setting('keymap') || {},
    saveOverrides: (keymapObj) => saveSettings({ keymap: keymapObj }),
    runAction: (b) => {
      if (b.run) return b.run();
      if (b.editorAction) {
        const action = editor.getAction(b.editorAction);
        return action ? action.run() : editor.trigger('keyboard', b.editorAction, {});
      }
      const extra = {
        closeFile: () => state.active && closeTab(state.active),
        searchAll: () => searchEverything(),
        ai: () => aiPanel.toggle(),
        zoomIn: () => setFontSize(1),
        zoomOut: () => setFontSize(-1),
        focus: toggleFocus,
      };
      return (extra[b.id] || userKeysCommands[b.id])?.();
    },
  });
  gh = createGitHub({
    toast,
    tools,
    getWorkspace: () => state.workspace,
    openProject: (dir) => setWorkspace(dir),
    openSettingsTab: (id) => {
      state.settingsTab = id;
      openSettings();
    },
    onStatus: renderGitStatus,
  });
  // Flux Together cez Wi-Fi (vstavaný plugin, zapína sa v Nastaveniach → Plugins)
  together = createTogether({
    monaco,
    editor,
    t,
    icon,
    esc: escapeHtml,
    toast,
    getWorkspace: () => state.workspace,
    getName: () => String(setting('userName') || '').trim(),
    isOn: () => setting('togetherPlugin') === true,
    openFile,
    relative,
    join,
    basename,
    promptPalette,
  });
  together.refresh();
  aiPanel = createAIPanel({
    toast,
    getContext: () => {
      const tab = activeTab();
      const model = editor.getModel();
      const sel = model && editor.getSelection() ? model.getValueInRange(editor.getSelection()) : '';
      return { file: tab?.path || '', fileName: tab ? basename(tab.path) : '', text: model?.getValue() || '', selection: sel, language: model?.getLanguageId() || '' };
    },
    // Kód z odpovede: nahradí výber, inak sa vloží na kurzor.
    insertCode: (code) => {
      if (!editor.getModel() || activeTab()?.readonly) return toast(t('Open a file first.'));
      const sel = editor.getSelection();
      editor.pushUndoStop();
      editor.executeEdits('flux-ai', [{ range: sel, text: code, forceMoveMarkers: true }]);
      editor.pushUndoStop();
      editor.focus();
    },
    openSettingsAI: () => {
      state.settingsTab = 'ai';
      openSettings();
    },
  });
  editor.onDidChangeCursorSelection(() => aiPanel.updateContext());
  // AI zmenila popis alebo úlohy projektu → hneď to vidno na stránke projektu.
  flux.onProjectMetaChanged(async () => {
    state.settings = await flux.setSettings({});
    if (!state.active) renderWelcome();
  });
  editor.addAction({
    id: 'flux.askAI',
    label: t('Ask AI about this'),
    contextMenuGroupId: '1_modification',
    contextMenuOrder: 1,
    run: () => aiPanel.ask(editor.getModel()?.getValueInRange(editor.getSelection()) ? t('Explain this code.') : t('Explain this file.')),
  });
  registerSnippets();
  createLanguageClient();
  applyTheme();
  createTerminal();
  treeEvents();
  projectEvents();
  renderProjects();
  trackTime();
  keybindings();
  layoutEvents();
  // Hneď pri štarte: rozloženie (menu, panel, bočný panel) a úspory pamäte – nielen po zmene nastavení.
  applyCustomization();
  renderRunButton();
  renderLive();
  renderStatus();
  renderTree();
  renderWelcome();

  flux.onRunStart((info) => {
    state.lastLabel = info.label;
    onRunStart(info);
  });
  flux.onRunData(onRunData);
  flux.onRunExit(onRunExit);
  flux.onLiveLog(onLiveLog);
  // Alt + klik v náhľade / „Show in Flux“ pri chybe → súbor na správnom riadku.
  flux.onLiveOpen(async ({ file, line }) => {
    const tab = await openFile(file, { line });
    if (tab) {
      editor.revealLineInCenter(line);
      editor.setSelection(new monaco.Selection(line, 1, line, editor.getModel().getLineMaxColumn(line)));
    }
  });
  let gitTimer = 0;
  flux.onFsChanged(async () => {
    await refreshTree();
    syncOpenTabs();
    // Počet zmien pre Git – nie pri každom uložení hneď, stačí raz za chvíľu.
    clearTimeout(gitTimer);
    if (ghOn()) gitTimer = setTimeout(() => gh?.refreshStatus(), 1500);
  });
  // Iná AI aplikácia (cez MCP) otvorila projekt / súbor alebo zapísala súbor.
  flux.onMcpOpenProject((dir) => setWorkspace(dir));
  flux.onMcpOpenFile((file) => openFile(file));
  flux.onMcpFileWritten(async () => {
    await refreshTree();
    syncOpenTabs();
  });
  // Súbory otvorené cez „Otvoriť v programe → Flux“ alebo pretiahnuté do okna.
  setupDrop();
  setupNav();
  setupLooseFiles();
  flux.onOpenFiles((files) => openStandalone(files));
  flux.startupFiles().then((files) => files.length && setTimeout(() => openStandalone(files), 600));
  flux.onSaveAllAndClose(async () => {
    await saveAll();
    flux.close();
  });

  if (!state.settings.onboarded) onboarding.open();
  pluginHost.start(); // nainštalované pluginy
  updatesUI.whatsNew();
  if (init.lastFolder) await setWorkspace(init.lastFolder);
  else {
    detectPython();
    if (state.settings.onboarded) openStart();
  }
}

main();
