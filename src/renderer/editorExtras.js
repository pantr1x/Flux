// Drobnosti v editore, ktoré šetria čas:
//  - HTML: po napísaní <h1> sa hneď doplní </h1>,
//  - komentár nad riadkom (pravý klik alebo Ctrl+Alt+/),
//  - po premenovaní premennej ponuka „premenovať všade“,
//  - v úvodzovkách napovedá súbory a priečinky projektu,
//  - hover nad obrázkom v HTML/CSS ukáže náhľad, nad odkazom odkaz.
import { t } from './i18n.js';

const VOID_TAGS = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr', '!doctype']);
const IMAGE = /\.(png|jpe?g|gif|webp|svg|ico|bmp|avif)$/i;

const COMMENT = {
  python: ['# ', ''],
  ruby: ['# ', ''],
  shell: ['# ', ''],
  powershell: ['# ', ''],
  yaml: ['# ', ''],
  lua: ['-- ', ''],
  html: ['<!-- ', ' -->'],
  xml: ['<!-- ', ' -->'],
  markdown: ['<!-- ', ' -->'],
  css: ['/* ', ' */'],
  scss: ['// ', ''],
  less: ['// ', ''],
  bat: ['REM ', ''],
};

export function setupEditorExtras({ monaco, editor, flux, getWorkspace, getFilePath, onFsChanged }) {
  autoCloseTags(monaco, editor);
  addCommentAction(monaco, editor);
  renameEverywhere(monaco, editor);
  pathSuggestions(monaco, { flux, getWorkspace, getFilePath, onFsChanged });
  imageHover(monaco, { flux, getFilePath });
}

// ---------- HTML: automatické ukončenie tagu ----------
function autoCloseTags(monaco, editor) {
  editor.onDidChangeModelContent((e) => {
    const model = editor.getModel();
    if (!model || e.isUndoing || e.isRedoing || e.isFlush) return;
    if (!['html', 'php', 'xml'].includes(model.getLanguageId())) return;
    if (e.changes.length !== 1) return;
    const ch = e.changes[0];
    // Návrh tagu prijatý Tabom/Enterom (napr. „<h“ → „h1“) → doplní „>“ aj „</h1>“.
    if (/^[a-zA-Z][\w:.-]*$/.test(ch.text) && ch.text.length > 1) {
      const ln = ch.range.startLineNumber;
      const lineText = model.getLineContent(ln);
      const startCol = ch.range.startColumn;
      const endCol = startCol + ch.text.length;
      if (lineText[startCol - 2] !== '<') return;
      const rest = lineText.slice(endCol - 1);
      if (/^[\w>/]/.test(rest)) return; // už je tam „>“ alebo slovo pokračuje
      const tag = ch.text;
      const isVoid = VOID_TAGS.has(tag.toLowerCase());
      queueMicrotask(() => {
        if (editor.getModel() !== model) return;
        const col = endCol + 1;
        editor.executeEdits('flux-close-tag', [{ range: new monaco.Range(ln, endCol, ln, endCol), text: isVoid ? '>' : `></${tag}>` }], [new monaco.Selection(ln, col, ln, col)]);
      });
      return;
    }
    if (ch.text !== '>') return;
    const line = ch.range.startLineNumber;
    const col = ch.range.startColumn + 1; // za „>“
    const before = model.getLineContent(line).slice(0, col - 1);
    const m = before.match(/<([a-zA-Z][\w:.-]*)(?:\s+[^<>]*?)?>$/);
    if (!m || before.endsWith('/>')) return;
    const tag = m[1];
    if (VOID_TAGS.has(tag.toLowerCase())) return;
    const after = model.getLineContent(line).slice(col - 1);
    if (after.startsWith(`</${tag}`)) return;
    // Po skončení aktuálnej zmeny vložíme </tag> a kurzor necháme medzi.
    queueMicrotask(() => {
      if (editor.getModel() !== model) return;
      editor.executeEdits('flux-close-tag', [{ range: new monaco.Range(line, col, line, col), text: `</${tag}>` }], [new monaco.Selection(line, col, line, col)]);
    });
  });
}

// ---------- komentár nad riadkom ----------
function addCommentAction(monaco, editor) {
  editor.addAction({
    id: 'flux.addComment',
    label: t('Add comment above'),
    keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Alt | monaco.KeyCode.Slash],
    contextMenuGroupId: '1_modification',
    contextMenuOrder: 0,
    run(ed) {
      const model = ed.getModel();
      const pos = ed.getPosition();
      if (!model || !pos) return;
      const [open, close] = COMMENT[model.getLanguageId()] || ['// ', ''];
      const text = model.getLineContent(pos.lineNumber);
      const indent = text.match(/^\s*/)[0];
      const insert = `${indent}${open}${close}\n`;
      const col = indent.length + open.length + 1;
      ed.executeEdits(
        'flux-comment',
        [{ range: new monaco.Range(pos.lineNumber, 1, pos.lineNumber, 1), text: insert }],
        [new monaco.Selection(pos.lineNumber, col, pos.lineNumber, col)],
      );
      ed.focus();
    },
  });
}

// ---------- „Premenovať všade?“ ----------
function renameEverywhere(monaco, editor) {
  const IDENT = /^[A-Za-z_$][\w$]*$/;
  let snap = null; // slovo pod kurzorom pred úpravou
  let session = null; // { model, line, start, old }
  let widget = null;
  let hideTimer = null;

  const wordAt = (model, line, col) => model.getWordAtPosition({ lineNumber: line, column: col });

  const occurrences = (model, word) =>
    model.findMatches(`\\b${word.replace(/[$]/g, '\\$')}\\b`, false, true, true, null, false).filter((m) => {
      // Vynechať výskyty v komentároch a reťazcoch (podľa farebného zvýraznenia).
      try {
        const tokens = monaco.editor.tokenize(model.getLineContent(m.range.startLineNumber), model.getLanguageId())[0] || [];
        let type = '';
        for (const tk of tokens) if (tk.offset <= m.range.startColumn - 1) type = tk.type;
        return !/comment|string/.test(type);
      } catch {
        return true;
      }
    });

  function hide() {
    clearTimeout(hideTimer);
    if (widget) editor.removeContentWidget(widget);
    widget = null;
  }

  function offer(model, line, start, oldName, newName, matches) {
    hide();
    const node = document.createElement('div');
    node.className = 'rn-offer';
    node.innerHTML = `<span>${t('Rename {old} → {new} everywhere?', { old: `<code>${oldName}</code>`, new: `<code>${newName}</code>` })} <small>${t('{n} more', { n: matches.length })}</small></span><button class="rn-yes">${t('Rename all')}</button><button class="rn-no" title="${t('Close (Esc)')}">✕</button>`;
    node.onmousedown = (e) => e.preventDefault();
    node.querySelector('.rn-yes').onclick = () => {
      if (editor.getModel() === model) {
        // Znova nájsť (medzitým sa mohlo niečo zmeniť) a nahradiť naraz – jedno Ctrl+Z to vráti.
        const edits = occurrences(model, oldName).map((m) => ({ range: m.range, text: newName }));
        editor.pushUndoStop();
        editor.executeEdits('flux-rename', edits);
        editor.pushUndoStop();
      }
      hide();
      editor.focus();
    };
    node.querySelector('.rn-no').onclick = () => {
      hide();
      editor.focus();
    };
    widget = {
      getId: () => 'flux.renameOffer',
      getDomNode: () => node,
      getPosition: () => ({ position: { lineNumber: line, column: start }, preference: [monaco.editor.ContentWidgetPositionPreference.BELOW, monaco.editor.ContentWidgetPositionPreference.ABOVE] }),
    };
    editor.addContentWidget(widget);
    hideTimer = setTimeout(hide, 9000);
  }

  function finish() {
    const s = session;
    session = null;
    if (!s || editor.getModel() !== s.model) return;
    const now = wordAt(s.model, s.line, s.start);
    const newName = now && now.startColumn === s.start ? now.word : null;
    if (!newName || newName === s.old || !IDENT.test(newName) || !IDENT.test(s.old)) return;
    const matches = occurrences(s.model, s.old);
    if (matches.length) offer(s.model, s.line, s.start, s.old, newName, matches);
  }

  editor.onDidChangeCursorPosition((e) => {
    const model = editor.getModel();
    if (!model) return;
    const { lineNumber, column } = e.position;
    if (session) {
      const w = wordAt(model, session.line, session.start);
      const end = w && w.startColumn === session.start ? w.endColumn : session.start;
      if (lineNumber !== session.line || column < session.start || column > end) finish();
      else return;
    }
    // Len keď kurzor presunieš ty (klik, šípky) – nie počas písania nového slova.
    if (e.reason === monaco.editor.CursorChangeReason.Explicit || e.source === 'mouse') {
      snap = { model, line: lineNumber, word: wordAt(model, lineNumber, column) };
    }
  });

  editor.onDidChangeModelContent((e) => {
    const model = editor.getModel();
    if (!model || e.isUndoing || e.isRedoing || e.isFlush) return;
    if (widget && e.changes.some((c) => c.range.startLineNumber !== widget.getPosition().position.lineNumber)) hide();
    if (session) return;
    const sn = snap;
    snap = null;
    if (!sn?.word || sn.model !== model) return;
    const ch = e.changes[0];
    if (e.changes.length !== 1 || ch.range.startLineNumber !== sn.line || ch.range.endLineNumber !== sn.line || /[^\w$]/.test(ch.text)) return;
    const w = sn.word;
    if (ch.range.startColumn < w.startColumn || ch.range.endColumn > w.endColumn) return;
    session = { model, line: sn.line, start: w.startColumn, old: w.word };
  });

  editor.onDidBlurEditorText(() => session && finish());
  editor.onDidChangeModel(() => {
    session = null;
    hide();
  });
  editor.onKeyDown((e) => {
    if (e.keyCode === monaco.KeyCode.Escape && widget) hide();
  });
}

// ---------- napovedanie súborov a priečinkov v úvodzovkách ----------
function pathSuggestions(monaco, { flux, getWorkspace, getFilePath, onFsChanged }) {
  let files = null;
  let loading = null;
  onFsChanged(() => (files = null));
  const load = async () => {
    if (files) return files;
    if (!loading) loading = flux.listAll().then((f) => (files = f)).finally(() => (loading = null));
    return loading;
  };
  const langs = ['python', 'html', 'css', 'scss', 'less', 'javascript', 'typescript', 'php', 'markdown', 'json', 'ruby', 'lua', 'java', 'go', 'rust', 'csharp', 'cpp', 'c'];
  for (const lang of langs) {
    monaco.languages.registerCompletionItemProvider(lang, {
      triggerCharacters: ['"', "'", '/', '('],
      async provideCompletionItems(model, position) {
        const ws = getWorkspace();
        const filePath = getFilePath(model);
        if (!ws || !filePath) return { suggestions: [] };
        const before = model.getLineContent(position.lineNumber).slice(0, position.column - 1);
        const m = before.match(/(["'`])([^"'`]*)$/) || (['css', 'scss', 'less'].includes(lang) && before.match(/url\(()([^)"']*)$/));
        if (!m) return { suggestions: [] };
        const typed = m[2].replace(/\\/g, '/');
        if (/^[a-z]+:/i.test(typed) || typed.includes('//')) return { suggestions: [] };
        const sep = ws.includes('\\') ? '\\' : '/';
        const norm = (p) => p.split(sep).join('/');
        const fileDir = norm(filePath).split('/').slice(0, -1).join('/');
        const dirPart = typed.includes('/') ? typed.slice(0, typed.lastIndexOf('/') + 1) : '';
        const partial = typed.slice(dirPart.length);
        // Relatívne k súboru; „/“ na začiatku = od koreňa projektu.
        const base = resolve(dirPart.startsWith('/') ? norm(ws) : fileDir, dirPart.replace(/^\//, ''));
        const all = (await load()).map(norm);
        const seen = new Map();
        for (const f of all) {
          if (!f.startsWith(`${base}/`)) continue;
          const rest = f.slice(base.length + 1);
          const [head, ...tail] = rest.split('/');
          if (!head || seen.has(head) || head.startsWith('.')) continue;
          seen.set(head, tail.length > 0);
        }
        const range = new monaco.Range(position.lineNumber, position.column - partial.length, position.lineNumber, position.column);
        const suggestions = [...seen].map(([name, isDir]) => ({
          label: isDir ? `${name}/` : name,
          kind: isDir ? monaco.languages.CompletionItemKind.Folder : monaco.languages.CompletionItemKind.File,
          insertText: isDir ? `${name}/` : name,
          range,
          sortText: `${isDir ? 0 : 1}${name}`,
          detail: isDir ? t('folder') : t('file'),
          command: isDir ? { id: 'editor.action.triggerSuggest', title: '' } : undefined,
        }));
        return { suggestions: suggestions.filter((s) => s.label.toLowerCase().startsWith(partial.toLowerCase())) };
      },
    });
  }
}

function resolve(dir, rel) {
  const parts = dir.split('/');
  for (const p of rel.split('/')) {
    if (!p || p === '.') continue;
    if (p === '..') parts.pop();
    else parts.push(p);
  }
  return parts.join('/');
}

// ---------- náhľad obrázka / odkazu pri hoveri ----------
function imageHover(monaco, { flux, getFilePath }) {
  const cache = new Map();
  for (const lang of ['html', 'css', 'scss', 'less', 'markdown', 'php', 'javascript']) {
    monaco.languages.registerHoverProvider(lang, {
      async provideHover(model, position) {
        const line = model.getLineContent(position.lineNumber);
        const re = /(["'])([^"']+?)\1|url\(\s*([^)"']+)\s*\)|\]\(([^)]+)\)/g;
        let hit = null;
        for (const m of line.matchAll(re)) {
          const start = m.index + 1;
          const end = m.index + m[0].length + 1;
          if (position.column >= start && position.column <= end) {
            hit = { value: (m[2] || m[3] || m[4]).trim(), range: new monaco.Range(position.lineNumber, start, position.lineNumber, end) };
            break;
          }
        }
        if (!hit) return null;
        const v = hit.value;
        if (/^https?:\/\//i.test(v)) {
          const img = IMAGE.test(v.split('?')[0]);
          return {
            range: hit.range,
            contents: [{ value: img ? `![](${v})\n\n[${t('Open in browser')}](${v})` : `🔗 [${v}](${v})`, isTrusted: true }],
          };
        }
        if (!IMAGE.test(v.split('?')[0])) return null;
        const file = getFilePath(model);
        if (!file) return null;
        const sep = file.includes('\\') ? '\\' : '/';
        const dir = file.split(sep).slice(0, -1).join('/');
        const full = resolve(dir.split(sep).join('/'), v.split('?')[0]).split('/').join(sep);
        let data = cache.get(full);
        if (!data) {
          try {
            data = await flux.readImage(full);
            cache.set(full, data);
            setTimeout(() => cache.delete(full), 30000);
          } catch {
            return { range: hit.range, contents: [{ value: `⚠️ ${t('Image not found: {path}', { path: v })}` }] };
          }
        }
        const kb = data.size > 1024 * 1024 ? `${(data.size / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(data.size / 1024))} KB`;
        return {
          range: hit.range,
          contents: [{ value: `<img src="${data.url}" style="max-width:240px;max-height:180px">\n\n${v.split('/').pop()} · ${kb}`, supportHtml: true, isTrusted: true }],
        };
      },
    });
  }
}
