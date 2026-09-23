import * as monaco from 'monaco-editor';
import { emmetHTML, emmetCSS } from 'emmet-monaco-es';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { StandaloneServices } from 'monaco-editor/editor/standalone/browser/standaloneServices.js';
import { IStorageService } from 'monaco-editor/platform/storage/common/storage.js';
import { icon, fileIcon } from './icons.js';
import { PythonLanguageClient } from './pyLsp.js';
import { THEMES, DEFAULT_THEME, themeOf, defineMonacoTheme, themeSwatch } from './themes.js';
import { TEMPLATES, PY_SNIPPETS, HTML_PAGE } from './templates.js';
import { createCodeMap } from './codemap.js';

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
  { id: 'Consolas', label: 'Consolas (ako VS Code)', css: "Consolas, 'Courier New', monospace" },
  { id: 'Cascadia Code', label: 'Cascadia Code', css: "'Cascadia Code', Consolas, monospace" },
  { id: 'Cascadia Mono', label: 'Cascadia Mono', css: "'Cascadia Mono', Consolas, monospace" },
  { id: 'JetBrains Mono', label: 'JetBrains Mono (ak je nainštalované)', css: "'JetBrains Mono', Consolas, monospace" },
  { id: 'Fira Code', label: 'Fira Code (ak je nainštalované)', css: "'Fira Code', Consolas, monospace" },
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
  wordWrap: false,
  autosave: true,
  clearOnRun: true,
  terminalFontSize: 13,
  suggestDetails: true,
  inertia: true,
};
const setting = (key) => state.settings[key] ?? DEFAULTS[key];

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

const RUNNABLE = new Set(['py', 'pyw', 'js', 'mjs', 'cjs', 'bat', 'cmd', 'ps1', 'sh']);
const WEB = new Set(['html', 'htm', 'css']);

// ---------- drobnosti UI ----------
function toast(message, kind = 'info', ms = 4200, action = null) {
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.textContent = message;
  if (action) {
    el.classList.add('clickable');
    el.title = action.label;
    el.onclick = () => {
      el.remove();
      action.run();
    };
  }
  $('#toasts').append(el);
  setTimeout(() => el.classList.add('out'), ms - 250);
  setTimeout(() => el.remove(), ms);
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
  $('#btn-settings').innerHTML = icon('settings');
  $('#brand-mark').innerHTML = icon('code', 13);
  $('#btn-stop').innerHTML = icon('stop', 14);
  $('#btn-clear').innerHTML = icon('trash', 15);
  $('#btn-panel').innerHTML = icon('panel', 15);
  $('#btn-preview-reload').innerHTML = icon('refresh', 15);
  $('#btn-preview-external').innerHTML = icon('external', 15);
  $('#btn-preview-close').innerHTML = icon('x', 15);
  const devices = $('#devices').children;
  devices[0].innerHTML = icon('monitor', 14);
  devices[1].innerHTML = icon('tablet', 14);
  devices[2].innerHTML = icon('phone', 14);
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
  const accentName = Object.keys(ACCENTS).find((k) => accentHex(k) === accent);
  $('#accents').innerHTML =
    Object.keys(ACCENTS)
      .slice(0, 6)
      .map((name) => `<button data-accent="${name}" style="--c:${accentHex(name)}" class="${name === accentName ? 'active' : ''}" title="${name}"></button>`)
      .join('') + `<button class="more" data-accent="more" title="Ďalšie farby a témy">${icon('palette', 13)}</button>`;
  if (term) term.options.theme = terminalTheme();
  codemap?.refresh();
}

// Nastavenia editora a terminálu (písmo, veľkosť, minimapa…).
function applyEditorSettings() {
  const font = FONTS.find((f) => f.id === setting('fontFamily')) || FONTS[0];
  editor?.updateOptions({
    fontFamily: font.css,
    fontSize: setting('fontSize'),
    lineHeight: setting('lineHeight'),
    fontLigatures: setting('ligatures'),
    minimap: { enabled: false },
    wordWrap: setting('wordWrap') ? 'on' : 'off',
  });
  codemap?.setVisible(setting('minimap'));
  if (term) {
    term.options.fontFamily = font.css;
    term.options.fontSize = setting('terminalFontSize');
    try {
      fit.fit();
    } catch {}
  }
}

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
    applyTheme();
    load();
  });
  window.addEventListener('focus', load); // tapeta sa mohla zmeniť
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
  if (m < 1) return 'menej ako minúta';
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return `${h} h ${m % 60 ? `${m % 60} min` : ''}`.trim();
}

const fmtNum = (n) => n.toLocaleString('sk-SK');

// ---------- editor ----------
let editor;
let codemap;
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
    $('#st-pos').textContent = `Riadok ${e.position.lineNumber} · Stĺpec ${e.position.column}`;
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
      glyphMarginClassName: m.severity === monaco.MarkerSeverity.Error ? 'glyph-error' : 'glyph-warn',
      glyphMarginHoverMessage: { value: m.message },
      overviewRuler: { color: m.severity === monaco.MarkerSeverity.Error ? '#ff6b7a' : '#f5b94a', position: monaco.editor.OverviewRulerLane.Left },
    },
  }));
  if (tab.problemDecos) tab.problemDecos.clear();
  tab.problemDecos = editor.createDecorationsCollection(decos);
  el.hidden = false;
  el.className = `status-item${errors ? ' err' : warns ? ' warn' : ' ok'}`;
  el.innerHTML = errors || warns
    ? `${errors ? `<span class="pb pb-e">●</span>${errors}` : ''} ${warns ? `<span class="pb pb-w">▲</span>${warns}` : ''}`.trim()
    : `${icon('check', 13)}Bez chýb`;
  el.title = errors || warns ? 'Klikni pre zoznam chýb' : 'V tomto súbore nie sú chyby';
}

function showProblems() {
  const tab = activeTab();
  if (!tab) return;
  const list = problemsOf(tab.model);
  if (!list.length) return toast('V tomto súbore nie sú žiadne chyby. 👍');
  openPalette({
    placeholder: `Chyby v ${basename(tab.path)}`,
    items: list.map((m) => ({
      label: `Riadok ${m.startLineNumber}: ${m.message.split('\n')[0]}`,
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
    toast(`Chyba na riadku ${m.startLineNumber}: ${m.message.split('\n')[0]}${errors.length > 1 ? `  (+${errors.length - 1} ďalšie)` : ''}`, 'error', 6000, {
      label: 'Prejsť na chybu',
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
          label: { label: sn.label, description: 'úryvok' },
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
  el.innerHTML = `<div class="gl-card"><div class="gl-bar"><span class="gl-name"></span><span class="gl-meta"></span><div class="grow"></div><button class="icon-btn" data-close title="Zavrieť (Esc)">${icon('x', 15)}</button></div><div class="gl-stage"><img alt=""></div></div>`;
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
  if (lang === 'html') return 'Napíš ! a stlač Tab – vytvorí sa kostra HTML stránky';
  if (lang === 'python') return 'Začni písať, napr. print("Ahoj") a stlač F5.  Skratky: main, for, def, input + Tab';
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
  updateRunGlyphs(tab);
  updateProblems();
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
}

async function closeTab(tab, { force = false } = {}) {
  if (!force && isDirty(tab)) {
    const ok = await confirmPalette(`„${basename(tab.path)}“ má neuložené zmeny.`, [
      { label: 'Uložiť a zavrieť', value: 'save' },
      { label: 'Zavrieť bez uloženia', value: 'discard' },
      { label: 'Zrušiť', value: null },
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
    toast(`Nepodarilo sa uložiť: ${errorText(err)}`, 'error');
    return false;
  }
  tab.savedVersion = version;
  lsp.didSave(tab.model);
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

function renderTabs() {
  const el = $('#tabs');
  el.innerHTML = '';
  for (const tab of state.tabs) {
    const div = document.createElement('div');
    div.className = `tab${tab === state.active ? ' active' : ''}${isDirty(tab) ? ' dirty' : ''}${tab.isNew ? ' enter' : ''}`;
    tab.isNew = false;
    div.title = tab.path;
    div.innerHTML = `${fileIcon(basename(tab.path), tab.model.getLanguageId())}<span class="name"></span>${tab.readonly ? '<span class="readonly">iba čítanie</span>' : ''}<button class="close" title="Zavrieť (Ctrl+W)">${icon('x', 13)}</button>`;
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
    div.title = `${tab.path}\nDvojklik = premenovať`;
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
    options: { glyphMarginClassName: 'run-glyph', glyphMarginHoverMessage: { value: 'Spustiť (F5)' } },
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

function renderTree() {
  const el = $('#tree');
  if (!state.workspace) {
    el.innerHTML = `<div class="tree-empty">Nie je otvorený žiadny priečinok.<br><button id="tree-open">Otvoriť priečinok</button></div>`;
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
  el.innerHTML = html.join('') || `<div class="tree-empty">Priečinok je prázdny.<br><button id="tree-new">Vytvoriť súbor</button></div>`;
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
      placeholder: 'Vyber šablónu…',
      note: `Nový súbor v: ${where}`,
      items: TEMPLATES.map((t) => ({
        label: t.label,
        detail: t.detail,
        icon: t.project ? icon('folderPlus', 16) : t.name ? fileIcon(t.name) : icon('filePlus', 16),
        tpl: t,
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
    placeholder: tpl.project ? 'Názov priečinka projektu' : 'napr. main.py, index.html, styles/app.css',
    note: tpl.project ? `Nový projekt v: ${where}` : `Názov súboru (${tpl.label})${tpl.files ? '' : ' · bez prípony sa pridá .py'}`,
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
  const name = await promptPalette({ placeholder: 'Názov priečinka', note: `Nový priečinok v: ${relative(dir) || basename(dir)}` });
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
  const name = await promptPalette({ value: old, select: [0, dot > 0 ? dot : old.length], note: 'Nový názov' });
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
    toast('Priečinok už neexistuje.', 'error');
    state.settings = await flux.setSettings({});
    return renderWelcome();
  }
  // Pred prepnutím projektu všetko ulož a zavri.
  await saveAll();
  for (const tab of [...state.tabs]) await closeTab(tab, { force: true });
  state.workspace = opened;
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
  await detectPython();
  lsp.start(opened, state.python?.path);
}

// ---------- projekty (ako „workspaces“ v Zene) ----------
// Zoznam nedávnych priečinkov s ikonou podľa obsahu – prepnutie jedným klikom.
async function renderProjects() {
  const el = $('#projects');
  let list = [];
  try {
    list = await flux.projects();
  } catch {}
  const kindIcon = (k) => (k === 'python' ? fileIcon('a.py') : k === 'web' ? fileIcon('a.html') : icon('folder', 16));
  el.innerHTML =
    `<div class="pr-head"><span>Projekty</span><button class="icon-btn" data-act="open" title="Otvoriť existujúci priečinok (Ctrl+O)">${icon('folderOpen', 15)}</button></div>` +
    list
      .map(
        (p) =>
          `<div class="pr-row${keyOf(p.dir) === keyOf(state.workspace || '') ? ' active' : ''}${p.pinned ? ' pinned' : ''}" data-dir="${escapeAttr(p.dir)}" data-pinned="${p.pinned ? 1 : ''}" title="${escapeAttr(p.dir)}\nPravý klik = premenovať, odstrániť">${kindIcon(p.kind)}<span class="pr-text"><span class="pr-name">${escapeHtml(p.name)}</span><small class="pr-sub" data-stats="${escapeAttr(p.dir)}"></small></span><button class="pr-pin" data-pin title="${p.pinned ? 'Odopnúť' : 'Pripnúť hore'}">${icon('pin', 13)}</button></div>`,
      )
      .join('') +
    `<button class="pr-row pr-new" data-act="new">${icon('plus', 16)}<span>Nový projekt</span></button>`;
  const current = list.find((p) => keyOf(p.dir) === keyOf(state.workspace || ''));
  if (current && state.projectKind !== current.kind) {
    state.projectKind = current.kind;
    if (!state.active) renderWelcome();
  }
  // Pod každým projektom: čas a počet súborov.
  for (const p of list) {
    flux.projectStats(p.dir).then((st) => {
      const sub = [...el.querySelectorAll('.pr-sub')].find((x) => x.dataset.stats === p.dir);
      if (sub) sub.textContent = `${st.time >= 60 ? formatTime(st.time) + ' · ' : ''}${st.files} ${st.files === 1 ? 'súbor' : st.files >= 2 && st.files <= 4 ? 'súbory' : 'súborov'}`;
    });
  }
}

// Nový projekt: typ → názov → vytvorí priečinok (Dokumenty\Flux projekty) so základným súborom.
async function newProject() {
  const kind = await new Promise((resolve) =>
    openPalette({
      placeholder: 'Aký projekt?',
      items: [
        { label: 'Python projekt', detail: 'priečinok + main.py', icon: fileIcon('a.py'), v: 'python' },
        { label: 'Web projekt', detail: 'priečinok + index.html, style.css, script.js', icon: fileIcon('a.html'), v: 'web' },
        { label: 'Prázdny projekt', detail: 'len priečinok', icon: icon('folder', 16), v: 'empty' },
        { label: 'Otvoriť existujúci priečinok…', icon: icon('folderOpen', 16), v: 'open' },
      ],
      onPick: (it) => resolve(it.v),
      onCancel: () => resolve(null),
    }),
  );
  if (!kind) return;
  if (kind === 'open') return openFolderDialog();
  const root = await flux.projectRoot();
  const name = await promptPalette({
    value: kind === 'web' ? 'moj-web' : kind === 'python' ? 'moj-program' : 'novy-projekt',
    placeholder: 'Názov projektu',
    note: `Vytvorí sa v: ${root}`,
  });
  if (!name) return;
  let dir;
  try {
    dir = await flux.createProject(name.trim());
  } catch (err) {
    return toast(errorText(err), 'error');
  }
  await setWorkspace(dir);
  const tplId = kind === 'python' ? 'py-main' : kind === 'web' ? 'web' : null;
  const tpl = TEMPLATES.find((t) => t.id === tplId);
  if (!tpl) return renderProjects();
  let first = null;
  for (const f of tpl.files) {
    const path = join(dir, f.name.replace('{{name}}', 'main.py'));
    await flux.create(path, false);
    await flux.write(path, f.content.replaceAll('{{title}}', name.trim()).replace('$0', ''));
    if (!first || f.open) first = path;
  }
  await refreshTree();
  await renderProjects();
  openFile(first);
}

async function renameProject(dir) {
  const old = basename(dir);
  const name = await promptPalette({ value: old, note: 'Nový názov projektu (premenuje aj priečinok na disku)' });
  if (!name || name.trim() === old) return;
  const wasOpen = keyOf(dir) === keyOf(state.workspace || '');
  if (wasOpen) await saveAll();
  let res;
  try {
    res = await flux.renameProject(dir, name.trim());
  } catch (err) {
    return toast(errorText(err), 'error');
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
    if (b.dataset.dir && keyOf(b.dataset.dir) !== keyOf(state.workspace || '')) await setWorkspace(b.dataset.dir);
  };
  el.oncontextmenu = async (e) => {
    const b = e.target.closest('[data-dir]');
    if (!b) return;
    e.preventDefault();
    const dir = b.dataset.dir;
    const pinned = !!b.dataset.pinned;
    const choice = await confirmPalette(basename(dir), [
      { label: pinned ? 'Odopnúť' : 'Pripnúť hore', value: 'pin', icon: icon('pin', 15) },
      { label: 'Premenovať…', value: 'rename', icon: icon('edit', 15) },
      { label: 'Odstrániť zo zoznamu (súbory ostanú na disku)', value: 'forget', icon: icon('x', 15) },
    ]);
    if (choice === 'pin') await flux.pinProject(dir, !pinned);
    if (choice === 'rename') return renameProject(dir);
    if (choice === 'forget') await flux.forgetProject(dir);
    state.settings = await flux.setSettings({});
    renderProjects();
  };
}

// ---------- Python ----------
async function detectPython() {
  state.python = await flux.findPython();
  renderStatus();
  if (lsp.ready) lsp.setPython(state.python?.path);
}

function renderStatus() {
  const py = $('#st-python');
  if (state.python) {
    py.className = 'status-item';
    py.innerHTML = `${icon('python', 13)}<span>Python ${state.python.version}</span><span style="opacity:.6">${escapeHtml(state.python.source)}</span>`;
    py.title = state.python.path;
  } else {
    py.className = 'status-item warn';
    py.innerHTML = `${icon('python', 13)}<span>Python sa nenašiel</span>`;
    py.title = 'Klikni a vyber Python alebo ho nainštaluj';
  }
  const lspEl = $('#st-lsp');
  const lang = state.active?.model.getLanguageId();
  lspEl.hidden = lang !== 'python' || !inside(state.active?.path);
  const s = { off: ['', ''], starting: ['Autocomplete sa načítava…', ''], ready: ['Autocomplete', 'ok'], error: ['Autocomplete nebeží', 'warn'] }[state.lspStatus];
  lspEl.className = `status-item ${s[1]}`;
  lspEl.innerHTML = s[0] ? `<span class="dot"></span>${s[0]}` : '';
  $('#st-lang').textContent = state.active ? LANG_NAMES[lang] || lang : '';
  if (!state.active) $('#st-pos').textContent = '';
  const auto = $('#st-autosave');
  auto.innerHTML = `${icon('save', 13)}${setting('autosave') ? 'Auto-ukladanie' : 'Ukladanie: Ctrl+S'}`;
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
      callback(links);
    },
  });
}

function greet() {
  term.writeln('\x1b[2mTu sa zobrazí výstup programu. Stlač \x1b[0m\x1b[1mF5\x1b[0m\x1b[2m alebo ▶ Spustiť.\x1b[0m');
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
    hints.push({ label: `Nainštalovať ${pkg}`, icon: 'download', run: () => pipInstall(pkg) });
  }
  if (code !== 0) {
    const frames = [...out.matchAll(/File "([^"]+)", line (\d+)/g)].filter((m) => inside(m[1]));
    const last = frames.pop();
    if (last) {
      hints.push({ label: `Chyba: ${basename(last[1])}, riadok ${last[2]}`, icon: 'chevron', run: () => openFile(last[1], { line: Number(last[2]) }) });
    }
  }
  setHint(hints);
}

// ---------- spúšťanie ----------
async function run() {
  const tab = activeTab();
  if (!tab) return toast('Otvor súbor, ktorý chceš spustiť.');
  const ext = extOf(tab.path);
  const lang = tab.model.getLanguageId();
  if (WEB.has(ext)) return openPreview(tab.path);
  if (!RUNNABLE.has(ext) && !(!ext && lang === 'python')) {
    return toast(ext ? `Súbory .${ext} zatiaľ neviem spustiť. Skús .py, .js alebo .html.` : 'Súbor nemá príponu – premenuj ho napr. na test.py.');
  }
  if (!inside(tab.path)) return toast('Tento súbor je iba na čítanie.');
  if (lang === 'python' && !state.python) {
    showPanel(true);
    setHint([
      { label: 'Stiahnuť Python', icon: 'download', run: () => flux.openExternal('https://www.python.org/downloads/') },
      { label: 'Vybrať python.exe…', icon: 'python', run: choosePython },
      { label: 'Hľadať znova', icon: 'refresh', run: detectPython },
    ]);
    return toast('Python sa nenašiel. Nainštaluj ho z python.org (zaškrtni „Add python.exe to PATH“).', 'error', 7000);
  }
  await saveAll();
  const res = await flux.runFile(tab.path, state.python?.path, lang).catch((err) => ({ ok: false, error: errorText(err) }));
  if (!res.ok) toast(res.error || 'Nepodarilo sa spustiť.', 'error');
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
  updatePanelVisibility();
  state.pty = !!pty;
  state.runOutput = '';
  state.stoppedByUser = false;
  inputBuffer = '';
  showPanel(true);
  setHint([]);
  if (setting('clearOnRun')) term.reset();
  else term.writeln('');
  term.write('\x1b[?25h');
  term.writeln(`\x1b[2m▶ ${label}\x1b[0m`);
  $('#run-state').className = 'run-state running';
  $('#run-state').textContent = 'Beží…';
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
    term.writeln(`\r\n\x1b[2m── Zastavené po ${secs} ──\x1b[0m`);
    status.className = 'run-state';
    status.textContent = 'Zastavené';
  } else if (code === 0) {
    term.writeln(`\r\n\x1b[2m── Hotovo za ${secs} ──\x1b[0m`);
    status.className = 'run-state ok';
    status.textContent = 'Hotovo';
  } else {
    term.writeln(`\r\n\x1b[31m── Skončilo s chybou (kód ${code ?? '?'}) ──\x1b[0m`);
    status.className = 'run-state fail';
    status.textContent = 'Chyba';
  }
  analyzeOutput(code);
  renderRunButton();
  // Po pip install alebo vytvorení venv znova zistiť Python.
  if (/^(pip install|python -m venv)/.test(state.lastLabel || '')) detectPython().then(() => lsp.start(state.workspace, state.python?.path));
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
  const show = !!state.active || state.running;
  $('#panel').hidden = !show;
  $('#panel-resizer').hidden = !show;
}

function renderRunButton() {
  updatePanelVisibility();
  const kind = fileKind(activeTab());
  const btn = $('#btn-run');
  const runnable = kind === 'script' || kind === 'script-web';
  btn.hidden = !runnable && !state.running;
  btn.innerHTML = `${icon('play', 14)}<span>${state.running ? 'Znova' : 'Spustiť'}</span>`;
  btn.title = 'Spustiť aktuálny súbor (F5)';
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
  $('#btn-panel').title = show ? 'Skryť panel (Ctrl+J)' : 'Zobraziť panel (Ctrl+J)';
  if (show) requestAnimationFrame(() => fit.fit());
}

// ---------- Live Server a náhľad ----------
function liveUrlFor(path) {
  const rel = relative(path).split(/[\\/]/).map(encodeURIComponent).join('/');
  return `${state.live.url}/${rel}`;
}

async function startLive() {
  if (!state.workspace) {
    toast('Najprv otvor priečinok s webom.');
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
  btn.title = state.live ? 'Zastaviť Live Server (Alt+L)' : 'Spustiť Live Server (Alt+L)';
  const st = $('#st-live');
  st.hidden = !state.live;
  if (state.live) {
    st.innerHTML = `${icon('globe', 13)}${state.live.url.replace('http://', '')}`;
    st.title = 'Otvoriť v prehliadači';
  }
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
      : '<div class="p-empty">Nič sa nenašlo</div>';
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
    openPalette({ note, placeholder: 'Vyber možnosť…', items: options, onPick: (o) => resolve(o.value), onCancel: () => resolve(null) }),
  );
}

async function quickOpen() {
  if (!state.workspace) return openFolderDialog();
  const files = await flux.listAll();
  openPalette({
    placeholder: 'Hľadať súbor podľa názvu…',
    items: files.map((f) => ({ label: relative(f), icon: fileIcon(basename(f)), path: f })),
    onPick: (it) => openFile(it.path),
  });
}

function commands() {
  const c = (label, run, kbd, ic, keywords = '') => ({ label, run, kbd, keywords, icon: ic ? icon(ic, 15) : '' });
  const list = [
    c('Spustiť aktuálny súbor', run, 'F5', 'play'),
    c('Zastaviť program', stop, 'Shift+F5', 'stop'),
    c(state.live ? 'Live Server: zastaviť' : 'Live Server: spustiť', toggleLive, 'Alt+L', 'globe', 'web náhľad preview html prehliadač'),
    c('Otvoriť priečinok…', openFolderDialog, 'Ctrl+O', 'folderOpen'),
    c('Nový projekt…', newProject, 'Ctrl+Shift+N', 'plus', 'projekt priečinok folder'),
    c('Rýchlo otvoriť súbor…', quickOpen, 'Ctrl+P', 'filePlus'),
    c('Nový súbor…', () => newFile(), 'Ctrl+N', 'filePlus'),
    c('Nový priečinok…', () => newFolder(), '', 'folderPlus'),
    c('Uložiť', () => saveTab(activeTab()), 'Ctrl+S', 'save'),
    c('Uložiť všetko', saveAll, '', 'save'),
    c('Zavrieť súbor', () => state.active && closeTab(state.active), 'Ctrl+W', 'x'),
    c('Formátovať dokument', () => editor.getAction('editor.action.formatDocument')?.run(), 'Shift+Alt+F', 'sparkle'),
    c('Prepnúť bočný panel (kompaktný režim)', toggleCompact, 'Ctrl+B', 'sidebar'),
    c('Prepnúť panel s výstupom', () => showPanel($('#panel').classList.contains('collapsed')), 'Ctrl+J', 'panel'),
    c('Prepnúť svetlú / tmavú tému', toggleTheme, '', isDark() ? 'sun' : 'moon', 'téma theme dark light farby vzhľad'),
    c('Farebná téma kódu…', chooseTheme, '', 'palette', 'téma theme farby vs code dracula one dark'),
    c('Nastavenia', openSettings, 'Ctrl+,', 'settings', 'settings nastavenia písmo font'),
    c('Nový súbor zo šablóny…', () => newFile(), 'Ctrl+N', 'template', 'šablóna template html python web projekt'),
    c(setting('autosave') ? 'Vypnúť automatické ukladanie' : 'Zapnúť automatické ukladanie', toggleAutosave, '', 'save'),
    c('Python: vybrať interpreter…', choosePython, '', 'python'),
    c('Python: zistiť interpreter automaticky', async () => {
      await flux.resetPython();
      await detectPython();
      lsp.start(state.workspace, state.python?.path);
    }, '', 'python'),
    c('Python: vytvoriť virtuálne prostredie (.venv)', createVenv, '', 'python'),
    c('Python: nainštalovať balík (pip)…', async () => {
      const pkg = await promptPalette({ placeholder: 'napr. requests, numpy, pygame', note: 'pip install' });
      if (pkg) pipInstall(pkg);
    }, '', 'download'),
    c('Python: reštartovať autocomplete', () => lsp.start(state.workspace, state.python?.path), '', 'refresh'),
    c('Zobraziť chyby v súbore', showProblems, '', 'x', 'problémy errors chyby'),
    c('Zväčšiť písmo', () => setFontSize(1), 'Ctrl+=', ''),
    c('Zmenšiť písmo', () => setFontSize(-1), 'Ctrl+-', ''),
  ];
  return list;
}

function openCommandPalette() {
  openPalette({ placeholder: 'Napíš príkaz…', items: commands(), onPick: (it) => it.run() });
}

function workspaceSwitcher() {
  const items = [
    { label: 'Otvoriť priečinok…', icon: icon('folderOpen', 15), kbd: 'Ctrl+O', run: openFolderDialog },
    ...(state.settings.recent || [])
      .filter((d) => d !== state.workspace)
      .map((d) => ({ label: basename(d), detail: d, icon: icon('folder', 15), run: () => setWorkspace(d) })),
  ];
  openPalette({ placeholder: 'Prepnúť na priečinok…', items, onPick: (it) => it.run() });
}

function pythonMenu() {
  const items = commands().filter((c) => c.label.startsWith('Python'));
  openPalette({
    placeholder: state.python ? `${state.python.path}` : 'Python sa nenašiel',
    items: state.python ? items : [{ label: 'Stiahnuť Python z python.org', icon: icon('download', 15), run: () => flux.openExternal('https://www.python.org/downloads/') }, ...items],
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
    toast(`Používam Python ${info.version}`);
  } catch (err) {
    toast(errorText(err), 'error');
  }
}

async function createVenv() {
  if (!state.python) return toast('Najprv treba mať nainštalovaný Python.', 'error');
  if (!state.workspace) return toast('Najprv otvor priečinok.', 'error');
  state.lastLabel = 'python -m venv';
  await flux.createVenv(state.python.path);
}

// ---------- nastavenia ----------
async function toggleTheme() {
  // Prepne medzi naposledy použitou tmavou a svetlou témou kódu.
  const next = isDark() ? setting('lastLight') : setting('lastDark');
  await setCodeTheme(next);
  applyTheme();
}

async function toggleAutosave() {
  await saveSettings({ autosave: !setting('autosave') });
  renderStatus();
  toast(setting('autosave') ? 'Automatické ukladanie je zapnuté.' : 'Automatické ukladanie je vypnuté – ukladaj cez Ctrl+S.');
}

async function setFontSize(delta) {
  const size = Math.min(28, Math.max(10, setting('fontSize') + delta));
  editor.updateOptions({ fontSize: size });
  await saveSettings({ fontSize: size });
}

async function toggleCompact() {
  const compact = !document.body.classList.contains('compact');
  document.body.classList.toggle('compact', compact);
  document.body.classList.remove('peek');
  await saveSettings({ compact });
}

async function setCodeTheme(id) {
  const t = themeOf(id);
  await saveSettings({ codeTheme: id, theme: t.type, [t.type === 'dark' ? 'lastDark' : 'lastLight']: id });
  applyTheme();
}

// Výber témy kódu – pri prechádzaní šípkami sa téma hneď ukazuje.
function chooseTheme() {
  const original = setting('codeTheme');
  const items = Object.entries(THEMES).map(([id, t]) => ({
    id,
    label: t.name,
    detail: t.type === 'dark' ? 'tmavá' : 'svetlá',
    icon: `<span class="swatch">${themeSwatch(id).map((c) => `<i style="background:${c}"></i>`).join('')}</span>`,
  }));
  openPalette({
    placeholder: 'Farebná téma kódu…',
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
    `<label class="s-row"><span><b>${label}</b>${hint ? `<small>${hint}</small>` : ''}</span><input type="checkbox" class="switch" data-key="${key}"${setting(key) ? ' checked' : ''}></label>`;
  panel.innerHTML = `
    <div class="s-card" role="dialog" aria-label="Nastavenia">
      <header><h2>${icon('settings', 18)}Nastavenia</h2><button class="icon-btn" data-close title="Zavrieť (Esc)">${icon('x', 16)}</button></header>
      <div class="s-body">
        <section>
          <h3>Vzhľad</h3>
          <div class="s-label">Téma kódu</div>
          <div class="theme-grid">${Object.entries(THEMES)
            .map(
              ([id, t]) =>
                `<button class="theme-card${id === setting('codeTheme') ? ' active' : ''}" data-theme="${id}"><span class="swatch">${themeSwatch(id)
                  .map((c) => `<i style="background:${c}"></i>`)
                  .join('')}</span><span>${escapeHtml(t.name)}</span><small>${t.type === 'dark' ? 'tmavá' : 'svetlá'}</small></button>`,
            )
            .join('')}</div>
          <div class="s-label">Farba zvýraznenia${state.workspace ? ` <small>(pre priečinok ${escapeHtml(basename(state.workspace))})</small>` : ''}</div>
          <div class="accent-grid">${Object.keys(ACCENTS)
            .map((name) => `<button data-accent="${name}" style="--c:${accentHex(name)}" class="${accentHex(name) === accent ? 'active' : ''}" title="${name === 'mono' ? 'čiernobiela (ako Zen)' : name}"></button>`)
            .join('')}<label class="custom-color" title="Vlastná farba"><input type="color" value="${accent}" data-custom-accent></label></div>
          ${state.platform === 'win32' ? `<label class="s-row"><span><b>Priesvitnosť okna</b><small>„Tapeta“ ostane priesvitná aj keď okno nie je aktívne. Acrylic/Mica robí Windows a vtedy okno zosivie.</small></span><select data-key="material">${[state.hasWallpaper && ['wallpaper', 'Tapeta (odporúčané)'], state.mica && ['acrylic', 'Acrylic (Windows)'], state.mica && ['mica', 'Mica (Windows)'], ['none', 'Vypnutá']].filter(Boolean).map(([v, l]) => opt(v, l, state.material)).join('')}</select></label>` : ''}
        </section>
        <section>
          <h3>Editor</h3>
          <label class="s-row"><span><b>Písmo</b></span><select data-key="fontFamily">${FONTS.map((f) => opt(f.id, f.label, setting('fontFamily'))).join('')}</select></label>
          <label class="s-row"><span><b>Veľkosť písma</b></span><input type="number" min="9" max="32" data-key="fontSize" value="${setting('fontSize')}"></label>
          <label class="s-row"><span><b>Výška riadku</b></span><select data-key="lineHeight">${[1.3, 1.45, 1.6, 1.8].map((v) => opt(v, { 1.3: 'kompaktná', 1.45: 'normálna', 1.6: 'voľnejšia', 1.8: 'veľká' }[v], setting('lineHeight'))).join('')}</select></label>
          ${toggle('ligatures', 'Ligatúry', 'spojené znaky ako => a != (napr. v Cascadia Code)')}
          ${toggle('minimap', 'Minimapa', 'zmenšený náhľad kódu vpravo')}
          ${toggle('wordWrap', 'Zalamovať dlhé riadky')}
          ${toggle('inertia', 'Plynulé posúvanie so zotrvačnosťou', 'po pustení kolieska text ešte chvíľu dokĺže')}
          ${toggle('suggestDetails', 'Popis návrhov vedľa zoznamu', 'dokumentácia vybranej funkcie ako vo VS Code')}
          ${toggle('autosave', 'Automatické ukladanie', 'uloží súbor chvíľu po písaní')}
        </section>
        <section>
          <h3>Spúšťanie</h3>
          ${toggle('clearOnRun', 'Vyčistiť výstup pred spustením')}
          <label class="s-row"><span><b>Veľkosť písma výstupu</b></span><input type="number" min="9" max="28" data-key="terminalFontSize" value="${setting('terminalFontSize')}"></label>
          <div class="s-row"><span><b>Python</b><small>${state.python ? `${escapeHtml(state.python.version)} · ${escapeHtml(state.python.path)}` : 'nenašiel sa'}</small></span><button class="s-btn" data-action="python">Zmeniť…</button></div>
        </section>
        <section>
          <h3>Skratky</h3>
          <div class="shortcuts">
            <span>Spustiť</span><span><kbd>F5</kbd></span>
            <span>Live Server</span><span><kbd>Alt</kbd> <kbd>L</kbd></span>
            <span>Nový súbor zo šablóny</span><span><kbd>Ctrl</kbd> <kbd>N</kbd></span>
            <span>Všetky príkazy</span><span><kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>P</kbd></span>
            <span>HTML kostra</span><span><kbd>!</kbd> + <kbd>Tab</kbd></span>
            <span>Nastavenia</span><span><kbd>Ctrl</kbd> <kbd>,</kbd></span>
          </div>
        </section>
      </div>
    </div>`;
  panel.hidden = false;

  panel.onclick = async (e) => {
    if (e.target === panel || e.target.closest('[data-close]')) return closeSettings();
    const themeBtn = e.target.closest('[data-theme]');
    if (themeBtn) {
      await setCodeTheme(themeBtn.dataset.theme);
      return openSettings();
    }
    const accentBtn = e.target.closest('[data-accent]');
    if (accentBtn) {
      await setAccent(accentBtn.dataset.accent);
      return openSettings();
    }
    if (e.target.closest('[data-action="python"]')) {
      closeSettings();
      pythonMenu();
    }
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
    if (el.type === 'number' || key === 'lineHeight') value = Number(value);
    await saveSettings({ [key]: value });
    if (key === 'suggestDetails') showSuggestDetails(value);
    if (key === 'material') await saveSettings({ translucent: true });
    applyEditorSettings();
    renderStatus();
  };
}

function closeSettings() {
  $('#settings').hidden = true;
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

// ---------- úvodná obrazovka na celé okno (klik na logo) ----------
const START_CHOICES = [
  { id: 'py', title: 'Python', sub: 'skript – print, input, výpočty', icon: 'main.py' },
  { id: 'py-tkinter', title: 'Python okno', sub: 'aplikácia s tlačidlami (tkinter)', icon: 'okno.py' },
  { id: 'py-pygame', title: 'Python hra', sub: 'pohyb šípkami (pygame)', icon: 'hra.py' },
  { id: 'html', title: 'HTML stránka', sub: 'jedna stránka s kostrou', icon: 'index.html' },
  { id: 'web', title: 'Web projekt', sub: 'HTML + CSS + JavaScript', icon: 'style.css' },
  { id: 'empty', title: 'Prázdny súbor', sub: 'vlastný názov a prípona', icon: 'subor.txt' },
];

function openStart() {
  const el = $('#start');
  const recent = (state.settings.recent || []).filter((d) => d !== state.workspace).slice(0, 4);
  el.innerHTML = `
    <div class="st-top drag"><div class="brand-mark">${icon('code', 15)}</div><span>flux</span></div>
    <div class="st-inner">
      <h1>Čo ideš robiť?</h1>
      <p class="st-sub">${state.workspace ? `Nový súbor sa vytvorí v priečinku <b>${escapeHtml(basename(state.workspace))}</b>.` : 'Najprv si vyberieš priečinok, kam sa súbor uloží.'}</p>
      <div class="st-grid">${START_CHOICES.map(
        (c) => `<button class="st-card" data-tpl="${c.id}"><span class="st-ic">${fileIcon(c.icon).replace(/width="16" height="16"/, 'width="30" height="30"')}</span><b>${c.title}</b><small>${c.sub}</small></button>`,
      ).join('')}</div>
      <div class="st-row">
        <button class="st-link" data-act="open">${icon('folderOpen', 16)}Otvoriť priečinok…</button>
        ${recent.map((d) => `<button class="st-link" data-dir="${escapeAttr(d)}" title="${escapeAttr(d)}">${icon('folder', 16)}${escapeHtml(basename(d))}</button>`).join('')}
      </div>
      ${state.workspace ? `<button class="st-back" data-act="back">Späť do editora <kbd>Esc</kbd></button>` : ''}
    </div>`;
  el.hidden = false;
  el.onclick = async (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    if (b.dataset.act === 'back') return closeStart();
    if (b.dataset.act === 'open') {
      await openFolderDialog();
      if (state.workspace) openStart();
      return;
    }
    if (b.dataset.dir) {
      await setWorkspace(b.dataset.dir);
      return closeStart();
    }
    const tpl = TEMPLATES.find((t) => t.id === b.dataset.tpl);
    if (!tpl) return;
    if (!state.workspace) {
      await openFolderDialog();
      if (!state.workspace) return;
    }
    closeStart();
    newFile(state.workspace, tpl);
  };
}

function closeStart() {
  $('#start').hidden = true;
  if (state.active) editor.focus();
}

// ---------- uvítanie ----------
function renderWelcome() {
  if (state.active) return;
  updateProblems();
  const w = $('#welcome');
  w.hidden = false;
  const ws = state.workspace;
  const projectIcon = (kind) =>
    (kind === 'python' ? fileIcon('a.py') : kind === 'web' ? fileIcon('a.html') : icon('folder', 16)).replace(/width="16" height="16"/, 'width="40" height="40"');
  w.innerHTML = `
    <div class="welcome-inner">
      <div class="wl-head">
        <span class="wl-icon">${ws ? projectIcon(state.projectKind) : `<span class="brand-mark big">${icon('code', 22)}</span>`}</span>
        <div><h1>${ws ? escapeHtml(basename(ws)) : 'flux'}</h1>
        <p class="sub">${ws ? escapeHtml(ws) : 'Otvor priečinok s projektom a spúšťaj kód jedným klikom.'}</p></div>
      </div>
      ${ws ? `<div class="stats" id="wl-stats">${['Čas v projekte', 'Súbory', 'Riadky', 'Znaky'].map((l) => `<div class="stat"><b>–</b><span>${l}</span></div>`).join('')}</div>` : ''}
      <div class="welcome-actions">
        ${ws
          ? `<button class="primary" data-act="new">${icon('filePlus')}Nový súbor</button><button data-act="quick">${icon('command')}Nájsť súbor</button><button data-act="start">${icon('template')}Šablóny</button>`
          : `<button class="primary" data-act="open">${icon('folderOpen')}Otvoriť priečinok</button><button data-act="newproject">${icon('plus')}Nový projekt</button>`}
      </div>
      <p class="welcome-section">Skratky</p>
      <div class="shortcuts">
        <span>Spustiť súbor / živý náhľad webu</span><span><kbd>F5</kbd></span>
        <span>Nový súbor zo šablóny</span><span><kbd>Ctrl</kbd> <kbd>N</kbd></span>
        <span>Nájsť súbor</span><span><kbd>Ctrl</kbd> <kbd>P</kbd></span>
        <span>Všetky príkazy</span><span><kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>P</kbd></span>
      </div>
    </div>`;
  w.onclick = (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    if (b.dataset.act === 'open') openFolderDialog();
    else if (b.dataset.act === 'new') newFile();
    else if (b.dataset.act === 'quick') quickOpen();
    else if (b.dataset.act === 'start') openStart();
    else if (b.dataset.act === 'newproject') newProject();
  };
  if (ws) {
    flux.projectStats(ws).then((st) => {
      const el = $('#wl-stats');
      if (!el || state.workspace !== ws) return;
      const vals = [formatTime(st.time), fmtNum(st.files), fmtNum(st.lines), fmtNum(st.chars)];
      el.querySelectorAll('.stat b').forEach((b, i) => (b.textContent = vals[i]));
    });
  }
}

// ---------- klávesové skratky ----------
function keybindings() {
  window.addEventListener(
    'keydown',
    (e) => {
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
      if (!$('#glance').hidden) {
        if (e.key === 'Escape') {
          e.preventDefault();
          $('#glance').hidden = true;
        }
        return;
      }
      if (!$('#start').hidden) {
        if (e.key === 'Escape' && state.workspace) {
          e.preventDefault();
          closeStart();
        }
        return;
      }
      if (!$('#settings').hidden) {
        if (e.key === 'Escape') {
          e.preventDefault();
          closeSettings();
        }
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
      else if (ctrl && !e.shiftKey && key === 'p') quickOpen();
      else if (ctrl && key === 's') e.shiftKey ? saveAll() : saveTab(activeTab()).then(() => announceProblems());
      else if (ctrl && key === 'o') openFolderDialog();
      else if (ctrl && e.shiftKey && key === 'n') newProject();
      else if (ctrl && key === 'n') newFile();
      else if (ctrl && key === 'w') state.active && closeTab(state.active);
      else if (ctrl && key === 'b') toggleCompact();
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
    (e) => root.style.setProperty('--side-w', `${Math.min(480, Math.max(180, e.clientX))}px`),
    () => saveSettings({ sideWidth: parseInt(getComputedStyle(root).getPropertyValue('--side-w')) }),
  );
  resizer($('#preview-resizer'), (e) => {
    const rect = $('#workarea').getBoundingClientRect();
    const w = Math.min(rect.width - 200, Math.max(240, rect.right - e.clientX));
    $('#preview').style.width = `${w}px`;
  });
  resizer($('#panel-resizer'), (e) => {
    const rect = $('#card').getBoundingClientRect();
    const h = Math.min(rect.height - 120, Math.max(90, rect.bottom - 28 - e.clientY));
    showPanel(true);
    $('#panel').style.height = `${h}px`;
  }, () => saveSettings({ panelHeight: parseInt($('#panel').style.height) }));
  if (state.settings.panelHeight) $('#panel').style.height = `${state.settings.panelHeight}px`;

  // Kompaktný režim: panel sa vysunie pri nabehnutí k ľavému okraju.
  $('#peek-zone').addEventListener('mouseenter', () => document.body.classList.add('peek'));
  $('#sidebar').addEventListener('mouseleave', () => document.body.classList.remove('peek'));
  if (state.settings.compact) document.body.classList.add('compact');

  $('#btn-compact').onclick = toggleCompact;
  $('#btn-expand').onclick = toggleCompact;
  $('#btn-theme').onclick = toggleTheme;
  $('#btn-palette').onclick = openCommandPalette;
  $('#btn-settings').onclick = openSettings;
  $('.brand').onclick = openStart;
  $('.brand').title = 'Úvodná obrazovka';
  $('#btn-run').onclick = run;
  $('#btn-stop').onclick = stop;
  $('#btn-live').onclick = toggleLive;
  $('#btn-clear').onclick = () => {
    term.reset();
    term.write(state.running ? '' : '\x1b[?25l');
    setHint([]);
  };
  $('#btn-panel').onclick = () => showPanel($('#panel').classList.contains('collapsed'));
  $('#st-python').onclick = pythonMenu;
  $('#st-autosave').onclick = toggleAutosave;
  $('#st-problems').onclick = showProblems;
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
    const w = b.dataset.w;
    $('#preview-frame').style.width = w ? `${w}px` : '100%';
    $('#preview-frame').style.flex = 'none';
    document.querySelector('.preview-stage').classList.toggle('framed', !!w);
  };
  $('#accents').onclick = (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    if (b.dataset.accent === 'more') openSettings();
    else setAccent(b.dataset.accent);
  };
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
  state.mica = !!init.mica;
  state.material = init.material;
  state.hasWallpaper = !!init.hasWallpaper;
  state.settings = init.settings;
  // Jednorazovo: staré verzie ukladali fialovú ako predvolenú – nová predvolená je čiernobiela (ako Zen).
  if (!state.settings.monoMigrated) await saveSettings({ monoMigrated: true, accent: 'mono', accents: {} });
  document.body.classList.add(`platform-${state.platform}`);

  setIcons();
  setupWallpaper();
  createEditor();
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
  flux.onFsChanged(async () => {
    await refreshTree();
    syncOpenTabs();
  });
  flux.onSaveAllAndClose(async () => {
    await saveAll();
    flux.close();
  });

  if (init.lastFolder) await setWorkspace(init.lastFolder);
  else {
    detectPython();
    openStart();
  }
}

main();
