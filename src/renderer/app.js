import * as monaco from 'monaco-editor';
import { emmetHTML, emmetCSS } from 'emmet-monaco-es';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { icon, fileBadge } from './icons.js';
import { PythonLanguageClient } from './pyLsp.js';

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
  violet: '#8b7bff',
  blue: '#4f9dff',
  teal: '#2ec4b6',
  green: '#3ecf8e',
  orange: '#ff9f5a',
  pink: '#ff6fb1',
};

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
function toast(message, kind = 'info', ms = 4200) {
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.textContent = message;
  $('#toasts').append(el);
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
function defineThemes(accent) {
  const a = accent.replace('#', '');
  monaco.editor.defineTheme('flux-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '6f6f86', fontStyle: 'italic' },
      { token: 'keyword', foreground: 'c792ea' },
      { token: 'string', foreground: '9ece6a' },
      { token: 'number', foreground: 'ff9e64' },
      { token: 'type', foreground: '7dcfff' },
      { token: 'type.identifier', foreground: '7dcfff' },
      { token: 'delimiter', foreground: '9aa5ce' },
      { token: 'tag', foreground: 'f7768e' },
      { token: 'attribute.name', foreground: 'e0af68' },
      { token: 'attribute.value', foreground: '9ece6a' },
      { token: 'metatag', foreground: 'f7768e' },
    ],
    colors: {
      'editor.background': '#00000000',
      'editor.foreground': '#e2e2ea',
      'editorGutter.background': '#00000000',
      'editor.lineHighlightBackground': '#ffffff07',
      'editor.lineHighlightBorder': '#00000000',
      'editorLineNumber.foreground': '#5e5e6a',
      'editorLineNumber.activeForeground': '#a4a4b3',
      'editorCursor.foreground': accent,
      'editor.selectionBackground': `#${a}44`,
      'editor.inactiveSelectionBackground': `#${a}22`,
      'editor.wordHighlightBackground': '#ffffff10',
      'editorIndentGuide.background1': '#ffffff0b',
      'editorIndentGuide.activeBackground1': '#ffffff22',
      'editorWidget.background': '#34343c',
      'editorWidget.border': '#ffffff14',
      'editorSuggestWidget.background': '#34343c',
      'editorSuggestWidget.border': '#ffffff14',
      'editorSuggestWidget.selectedBackground': `#${a}38`,
      'editorSuggestWidget.highlightForeground': accent,
      'editorHoverWidget.background': '#34343c',
      'editorHoverWidget.border': '#ffffff14',
      'scrollbarSlider.background': '#ffffff12',
      'scrollbarSlider.hoverBackground': '#ffffff20',
      'scrollbarSlider.activeBackground': '#ffffff2a',
      'editorOverviewRuler.border': '#00000000',
      'focusBorder': '#00000000',
      'editorStickyScroll.background': '#2f2f37',
      'editorStickyScrollHover.background': '#383840',
    },
  });
  monaco.editor.defineTheme('flux-light', {
    base: 'vs',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '8e8e9c', fontStyle: 'italic' },
      { token: 'keyword', foreground: '7c3aed' },
      { token: 'string', foreground: '2f855a' },
      { token: 'number', foreground: 'c2410c' },
      { token: 'type', foreground: '0369a1' },
      { token: 'type.identifier', foreground: '0369a1' },
      { token: 'tag', foreground: 'be123c' },
      { token: 'attribute.name', foreground: 'b45309' },
      { token: 'attribute.value', foreground: '2f855a' },
    ],
    colors: {
      'editor.background': '#00000000',
      'editorGutter.background': '#00000000',
      'editor.lineHighlightBackground': '#00000006',
      'editor.lineHighlightBorder': '#00000000',
      'editorLineNumber.foreground': '#c0c0cc',
      'editorLineNumber.activeForeground': '#5c5c6b',
      'editorCursor.foreground': accent,
      'editor.selectionBackground': `#${a}33`,
      'editor.inactiveSelectionBackground': `#${a}1a`,
      'editorIndentGuide.background1': '#0000000c',
      'editorWidget.background': '#ffffff',
      'editorSuggestWidget.background': '#ffffff',
      'editorSuggestWidget.selectedBackground': `#${a}26`,
      'editorSuggestWidget.highlightForeground': accent,
      'scrollbarSlider.background': '#00000012',
      'scrollbarSlider.hoverBackground': '#0000001f',
      'editorOverviewRuler.border': '#00000000',
      'focusBorder': '#00000000',
      'editorStickyScroll.background': '#f8f8fb',
    },
  });
}

function currentAccent() {
  const name = (state.workspace && state.settings.accents?.[state.workspace]) || state.settings.accent || 'violet';
  return ACCENTS[name] ? name : 'violet';
}

function applyTheme() {
  const dark = state.settings.theme !== 'light';
  const accentName = currentAccent();
  const accent = ACCENTS[accentName];
  document.body.classList.toggle('theme-dark', dark);
  document.body.classList.toggle('theme-light', !dark);
  document.documentElement.style.setProperty('--accent', accent);
  defineThemes(accent);
  monaco.editor.setTheme(dark ? 'flux-dark' : 'flux-light');
  $('#btn-theme').innerHTML = icon(dark ? 'sun' : 'moon');
  $('#accents').innerHTML = Object.entries(ACCENTS)
    .map(([name, c]) => `<button data-accent="${name}" style="--c:${c}" class="${name === accentName ? 'active' : ''}" title="${name}"></button>`)
    .join('');
  if (term) term.options.theme = terminalTheme();
}

async function saveSettings(patch) {
  state.settings = await flux.setSettings(patch);
}

// ---------- editor ----------
let editor;
function createEditor() {
  editor = monaco.editor.create($('#editor'), {
    model: null,
    theme: 'flux-dark',
    fontFamily: "'Cascadia Code', 'Cascadia Mono', Consolas, 'JetBrains Mono', monospace",
    fontLigatures: true,
    fontSize: state.settings.fontSize || 14,
    lineHeight: 1.6,
    automaticLayout: true,
    minimap: { enabled: false },
    smoothScrolling: true,
    cursorBlinking: 'smooth',
    cursorSmoothCaretAnimation: 'on',
    renderLineHighlight: 'all',
    roundedSelection: true,
    padding: { top: 10, bottom: 10 },
    scrollBeyondLastLine: false,
    bracketPairColorization: { enabled: true },
    guides: { bracketPairs: 'active', indentation: true },
    stickyScroll: { enabled: true },
    glyphMargin: true,
    lineDecorationsWidth: 6,
    lineNumbersMinChars: 3,
    mouseWheelZoom: true,
    quickSuggestions: { other: true, comments: false, strings: true },
    suggest: { preview: true, showStatusBar: false, selectionMode: 'always' },
    inlayHints: { enabled: 'off' },
    scrollbar: { verticalScrollbarSize: 10, horizontalScrollbarSize: 10, useShadows: false },
    overviewRulerLanes: 2,
    tabSize: 4,
    fixedOverflowWidgets: true,
    'semanticHighlighting.enabled': true,
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
  const tab = { path, model, readonly, viewState: null, savedVersion: model.getAlternativeVersionId(), runLines: [], decorations: null };
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

async function openFile(path, { line, column, focus = true } = {}) {
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

function activate(tab) {
  const prev = activeTab();
  if (prev && prev !== tab) prev.viewState = editor.saveViewState();
  state.active = tab;
  editor.setModel(tab.model);
  editor.updateOptions({ readOnly: tab.readonly });
  if (tab.viewState) editor.restoreViewState(tab.viewState);
  updateRunGlyphs(tab);
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
  if (!state.settings.autosave || tab.readonly) return;
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
    div.className = `tab${tab === state.active ? ' active' : ''}${isDirty(tab) ? ' dirty' : ''}`;
    div.title = tab.path;
    div.innerHTML = `${fileBadge(tab.path)}<span class="name"></span>${tab.readonly ? '<span class="readonly">iba čítanie</span>' : ''}<button class="close" title="Zavrieť (Ctrl+W)">${icon('x', 13)}</button>`;
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
            `${fileBadge(e.name)}<span class="name">${name}</span>${dirtyKeys.has(k) ? '<span class="dirty-dot"></span>' : ''}</div>`,
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

async function newFile(dir = targetDir()) {
  if (!dir) return openFolderDialog();
  let name = await promptPalette({
    placeholder: 'napr. main.py, index.html, styles/app.css',
    note: `Nový súbor v: ${relative(dir) || basename(dir)}  ·  bez prípony sa pridá .py`,
  });
  if (!name) return;
  name = name.trim();
  if (!basename(name).includes('.')) name += '.py';
  try {
    const path = await flux.create(join(dir, name), false);
    await revealInTree(path);
    await refreshTree();
    openFile(path);
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
  $('#ws-name').textContent = basename(opened);
  applyTheme();
  await refreshTree();
  renderWelcome();
  await detectPython();
  lsp.start(opened, state.python?.path);
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
  auto.innerHTML = `${icon('save', 13)}${state.settings.autosave ? 'Auto-ukladanie' : 'Ukladanie: Ctrl+S'}`;
  auto.className = `status-item${state.settings.autosave ? '' : ''}`;
}

// ---------- terminál / výstup ----------
let term, fit;
let inputBuffer = '';

function terminalTheme() {
  const dark = state.settings.theme !== 'light';
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

function onRunStart({ label, cwd, pty }) {
  state.running = true;
  state.pty = !!pty;
  state.runOutput = '';
  state.stoppedByUser = false;
  inputBuffer = '';
  showPanel(true);
  setHint([]);
  term.reset();
  term.write('\x1b[?25h');
  term.writeln(`\x1b[38;2;139;123;255m▶\x1b[0m \x1b[1m${label}\x1b[0m  \x1b[2m${cwd}\x1b[0m`);
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

function renderRunButton() {
  const tab = activeTab();
  const btn = $('#btn-run');
  const web = tab && WEB.has(extOf(tab.path));
  btn.innerHTML = `${icon(web ? 'globe' : 'play', 14)}<span>${state.running ? 'Znova' : web ? 'Náhľad' : 'Spustiť'}</span>`;
  btn.title = web ? 'Otvoriť živý náhľad (F5)' : 'Spustiť aktuálny súbor (F5)';
  btn.classList.toggle('running', state.running);
  $('#btn-stop').disabled = !state.running;
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

async function toggleLive() {
  if (state.live) {
    await flux.liveStop();
    state.live = null;
    closePreview();
    renderLive();
  } else {
    const tab = activeTab();
    await openPreview(tab && inside(tab.path) ? tab.path : null);
  }
}

function renderLive() {
  const btn = $('#btn-live');
  btn.classList.toggle('on', !!state.live);
  btn.innerHTML = `${icon('globe', 14)}<span>${state.live ? `Live :${state.live.port}` : 'Live Server'}</span>`;
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

function openPalette({ items = null, placeholder = '', value = '', note = '', select = null, onPick, onCancel }) {
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
  palette = { items, onPick, onCancel, sel: 0, shown: [] };
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
    items: files.map((f) => ({ label: relative(f), icon: fileBadge(f), path: f })),
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
    c('Rýchlo otvoriť súbor…', quickOpen, 'Ctrl+P', 'filePlus'),
    c('Nový súbor…', () => newFile(), 'Ctrl+N', 'filePlus'),
    c('Nový priečinok…', () => newFolder(), '', 'folderPlus'),
    c('Uložiť', () => saveTab(activeTab()), 'Ctrl+S', 'save'),
    c('Uložiť všetko', saveAll, '', 'save'),
    c('Zavrieť súbor', () => state.active && closeTab(state.active), 'Ctrl+W', 'x'),
    c('Formátovať dokument', () => editor.getAction('editor.action.formatDocument')?.run(), 'Shift+Alt+F', 'sparkle'),
    c('Prepnúť bočný panel (kompaktný režim)', toggleCompact, 'Ctrl+B', 'sidebar'),
    c('Prepnúť panel s výstupom', () => showPanel($('#panel').classList.contains('collapsed')), 'Ctrl+J', 'panel'),
    c('Prepnúť svetlú / tmavú tému', toggleTheme, '', state.settings.theme === 'light' ? 'moon' : 'sun', 'téma theme dark light farby vzhľad'),
    c(state.settings.autosave ? 'Vypnúť automatické ukladanie' : 'Zapnúť automatické ukladanie', toggleAutosave, '', 'save'),
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
  await saveSettings({ theme: state.settings.theme === 'light' ? 'dark' : 'light' });
  applyTheme();
}

async function toggleAutosave() {
  await saveSettings({ autosave: !state.settings.autosave });
  renderStatus();
  toast(state.settings.autosave ? 'Automatické ukladanie je zapnuté.' : 'Automatické ukladanie je vypnuté – ukladaj cez Ctrl+S.');
}

async function setFontSize(delta) {
  const size = Math.min(28, Math.max(10, (state.settings.fontSize || 14) + delta));
  editor.updateOptions({ fontSize: size });
  await saveSettings({ fontSize: size });
}

async function toggleCompact() {
  const compact = !document.body.classList.contains('compact');
  document.body.classList.toggle('compact', compact);
  document.body.classList.remove('peek');
  await saveSettings({ compact });
}

// ---------- uvítanie ----------
function renderWelcome() {
  if (state.active) return;
  const w = $('#welcome');
  w.hidden = false;
  const recent = (state.settings.recent || []).filter((d) => d !== state.workspace).slice(0, 5);
  const recentHtml = recent.length
    ? `<p class="welcome-section">Nedávne</p><div class="recent">${recent
        .map((d) => `<button data-dir="${escapeAttr(d)}"><span>${escapeHtml(basename(d))}</span><span class="path">${escapeHtml(d)}</span></button>`)
        .join('')}</div>`
    : '';
  const ws = state.workspace;
  w.innerHTML = `
    <div class="welcome-inner">
      <h1><span class="brand-mark"></span>${ws ? escapeHtml(basename(ws)) : 'flux'}</h1>
      <p class="sub">${ws ? 'Vyber súbor vľavo alebo vytvor nový.' : 'Otvor priečinok s projektom a spúšťaj kód jedným klikom.'}</p>
      <div class="welcome-actions">
        ${ws
          ? `<button class="primary" data-act="new">${icon('filePlus')}Nový súbor</button><button data-act="quick">${icon('command')}Nájsť súbor</button>`
          : `<button class="primary" data-act="open">${icon('folderOpen')}Otvoriť priečinok</button>`}
      </div>
      ${recentHtml}
      <p class="welcome-section">Skratky</p>
      <div class="shortcuts">
        <span>Spustiť súbor / živý náhľad webu</span><span><kbd>F5</kbd></span>
        <span>Zastaviť program</span><span><kbd>Shift</kbd> <kbd>F5</kbd></span>
        <span>Live Server</span><span><kbd>Alt</kbd> <kbd>L</kbd></span>
        <span>Nájsť súbor</span><span><kbd>Ctrl</kbd> <kbd>P</kbd></span>
        <span>Všetky príkazy</span><span><kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>P</kbd></span>
        <span>Skryť bočný panel</span><span><kbd>Ctrl</kbd> <kbd>B</kbd></span>
      </div>
    </div>`;
  w.onclick = (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    if (b.dataset.dir) setWorkspace(b.dataset.dir);
    else if (b.dataset.act === 'open') openFolderDialog();
    else if (b.dataset.act === 'new') newFile();
    else if (b.dataset.act === 'quick') quickOpen();
  };
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
      const ctrl = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();
      let handled = true;
      if (e.key === 'F5' && e.shiftKey) stop();
      else if (e.key === 'F5' || (ctrl && e.key === 'Enter')) run();
      else if (ctrl && e.shiftKey && key === 'p') openCommandPalette();
      else if (ctrl && !e.shiftKey && key === 'p') quickOpen();
      else if (ctrl && key === 's') e.shiftKey ? saveAll() : saveTab(activeTab());
      else if (ctrl && key === 'o') openFolderDialog();
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
  });

  // Kompaktný režim: panel sa vysunie pri nabehnutí k ľavému okraju.
  $('#peek-zone').addEventListener('mouseenter', () => document.body.classList.add('peek'));
  $('#sidebar').addEventListener('mouseleave', () => document.body.classList.remove('peek'));
  if (state.settings.compact) document.body.classList.add('compact');

  $('#btn-compact').onclick = toggleCompact;
  $('#btn-expand').onclick = toggleCompact;
  $('#btn-theme').onclick = toggleTheme;
  $('#btn-palette').onclick = openCommandPalette;
  $('#btn-workspace').onclick = workspaceSwitcher;
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
  $('#st-live').onclick = () => state.live && flux.openExternal($('#preview-frame').src || state.live.url);
  $('#btn-preview-reload').onclick = () => {
    const f = $('#preview-frame');
    if (f.src) f.src = f.src;
  };
  $('#btn-preview-external').onclick = () => flux.openExternal($('#preview-frame').src || state.live?.url);
  $('#btn-preview-close').onclick = closePreview;
  $('#devices').onclick = (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    for (const x of $('#devices').children) x.classList.toggle('active', x === b);
    const w = b.dataset.w;
    $('#preview-frame').style.width = w ? `${w}px` : '100%';
    $('#preview-frame').style.flex = 'none';
    document.querySelector('.preview-stage').classList.toggle('framed', !!w);
  };
  $('#accents').onclick = async (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    const accents = { ...(state.settings.accents || {}) };
    if (state.workspace) accents[state.workspace] = b.dataset.accent;
    await saveSettings(state.workspace ? { accents } : { accent: b.dataset.accent });
    applyTheme();
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
  state.settings = { autosave: true, ...init.settings };
  if (init.settings.autosave === undefined) await saveSettings({ autosave: true });
  document.body.classList.add(`platform-${state.platform}`);
  if (!init.mica) document.body.classList.add('no-mica');

  setIcons();
  createEditor();
  createLanguageClient();
  applyTheme();
  createTerminal();
  treeEvents();
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
  else detectPython();
}

main();
