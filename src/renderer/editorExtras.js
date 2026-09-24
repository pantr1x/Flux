// Drobnosti v editore, ktoré šetria čas:
//  - HTML: po napísaní <h1> sa hneď doplní </h1>,
//  - komentár nad riadkom (pravý klik alebo Ctrl+Alt+/),
//  - po premenovaní premennej alebo textu v úvodzovkách ponuka „premenovať všade“ (aj v ostatných súboroch),
//  - v úvodzovkách napovedá súbory a priečinky projektu,
//  - hover nad obrázkom v HTML/CSS ukáže náhľad, nad odkazom odkaz.
import { t } from './i18n.js';
import { createRefactor, stringsIn } from './refactor.js';
import { icon } from './icons.js';

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);

const VOID_TAGS = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr', '!doctype']);
const IMAGE = /\.(png|jpe?g|gif|webp|svg|ico|bmp|avif)$/i;

const COMMENT = {
  python: ['# ', ''],
  ruby: ['# ', ''],
  shell: ['# ', ''],
  powershell: ['# ', ''],
  yaml: ['# ', ''],
  lua: ['-- ', ''],
  r: ['# ', ''],
  julia: ['# ', ''],
  html: ['<!-- ', ' -->'],
  xml: ['<!-- ', ' -->'],
  markdown: ['<!-- ', ' -->'],
  css: ['/* ', ' */'],
  scss: ['// ', ''],
  less: ['// ', ''],
  bat: ['REM ', ''],
};

export function setupEditorExtras({ monaco, editor, flux, getWorkspace, getFilePath, onFsChanged, langFor, toast }) {
  const refactor = createRefactor({ monaco, flux, getWorkspace, langFor });
  autoCloseTags(monaco, editor);
  addCommentAction(monaco, editor);
  renameEverywhere(monaco, editor, { refactor, getFilePath, toast });
  pathSuggestions(monaco, { flux, getWorkspace, getFilePath, onFsChanged });
  imageHover(monaco, { flux, getFilePath });
  return { refactor };
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
// Prepíšeš názov (premenná, funkcia…) alebo text v úvodzovkách (napr. cestu "data/images") a Flux ponúkne
// zmenu aj na ostatných miestach – v tomto súbore a aj v ďalších súboroch projektu (refactor.js).
function renameEverywhere(monaco, editor, { refactor, getFilePath, toast }) {
  const IDENT = /^[\p{L}_$][\p{L}\p{N}_$]*$/u;
  let snap = null; // čo bolo pod kurzorom pred úpravou: { model, line, word?, str? }
  let session = null; // { kind: 'ident' | 'string', model, line, start, old, q? }
  let widget = null;
  let hideTimer = null;
  let offerId = 0;

  const wordAt = (model, line, col) => model.getWordAtPosition({ lineNumber: line, column: col });
  // reťazec, v ktorom je kurzor (stĺpce od 1; start = prvý znak obsahu)
  const stringAt = (model, line, col) => stringsIn(model.getLineContent(line)).find((s) => col - 1 >= s.start && col - 1 <= s.end);

  // dlhá cesta → len jej koniec („…/Desktop“), celá je v title
  const short = (v) => (v.length > 16 && /[\\/]/.test(v) ? `…/${v.split(/[\\/]/).filter(Boolean).pop()}` : v.length > 32 ? `${v.slice(0, 30)}…` : v);
  const identMatches = (model, name) => refactor.identRanges(model.getValue(), model.getLanguageId(), name);
  // bez samotného upraveného reťazca („data“ → „data/sub“ by sa inak zmenil znova)
  const stringMatches = (model, old, neu, self) => refactor.stringRanges(model.getValue(), old, neu).filter((r) => !(self && r.line === self.line && r.start === self.start));

  function hide() {
    clearTimeout(hideTimer);
    offerId++;
    if (widget) editor.removeContentWidget(widget);
    widget = null;
  }

  function offer(s, neu) {
    hide();
    const id = offerId;
    const { model, line, start, old, kind } = s;
    const here = kind === 'ident' ? identMatches(model, old) : stringMatches(model, old, neu, s);
    let others = [];
    const node = document.createElement('div');
    node.className = 'rn-offer';
    const draw = () => {
      const files = others.length;
      node.title = `${old} → ${neu}`;
      const btns = [
        here.length ? `<button class="rn-yes" data-scope="file">${t('This file')}<span class="rn-n">${here.length}</span></button>` : '',
        files ? `<button class="rn-yes" data-scope="all">${t('Whole project')}<span class="rn-n">${files === 1 ? t('+1 file') : t('+{n} files', { n: files })}</span></button>` : '',
      ].join('');
      node.innerHTML = `<div class="rn-body"><div class="rn-head">${icon('edit', 14)}<span>${t('Rename everywhere?')}</span><button class="rn-no" title="${t('Close (Esc)')}">${icon('x', 13)}</button></div>
        <div class="rn-change"><code class="rn-old">${esc(short(old))}</code>${icon('arrowRight', 13)}<code class="rn-new">${esc(short(neu))}</code></div>
        <div class="rn-actions">${btns}</div></div>`;
    };
    const show = () => {
      if (id !== offerId) return;
      draw();
      if (widget) return editor.layoutContentWidget(widget);
      widget = {
        getId: () => 'flux.renameOffer',
        getDomNode: () => node,
        getPosition: () => ({ position: { lineNumber: line, column: start }, preference: [monaco.editor.ContentWidgetPositionPreference.BELOW, monaco.editor.ContentWidgetPositionPreference.ABOVE] }),
      };
      editor.addContentWidget(widget);
    };
    node.onmousedown = (e) => e.preventDefault();
    node.onclick = async (e) => {
      if (e.target.closest('.rn-no')) {
        hide();
        return editor.focus();
      }
      const b = e.target.closest('[data-scope]');
      if (!b) return;
      hide();
      // Znova nájsť (medzitým sa mohlo niečo zmeniť) a nahradiť naraz – jedno Ctrl+Z to vráti.
      const now = kind === 'ident' ? identMatches(model, old) : stringMatches(model, old, neu, s);
      if (now.length) {
        editor.pushUndoStop();
        editor.executeEdits('flux-rename', now.map((r) => ({ range: new monaco.Range(r.line, r.start, r.line, r.end), text: r.text ?? neu })));
        editor.pushUndoStop();
      }
      if (b.dataset.scope === 'all' && others.length) {
        try {
          const n = await refactor.apply(others, neu);
          toast(others.length === 1 ? t('Changed {n} places in 1 other file.', { n }) : t('Changed {n} places in {f} other files.', { n, f: others.length }), 'ok');
        } catch (err) {
          toast(String(err?.message || err), 'error');
        }
      }
      editor.focus();
    };
    if (here.length) show();
    hideTimer = setTimeout(hide, 15000);
    // ostatné súbory projektu – dohľadá sa na pozadí a tlačidlo pribudne
    const path = getFilePath(model);
    if (path)
      refactor
        .scanOthers(kind, old, neu, { skip: path, lang: model.getLanguageId() })
        .then((list) => {
          others = list;
          if (list.length) show();
        })
        .catch(() => {});
  }

  function finish() {
    const s = session;
    session = null;
    if (!s || editor.getModel() !== s.model) return;
    if (s.kind === 'ident') {
      const now = wordAt(s.model, s.line, s.start);
      const neu = now && now.startColumn === s.start ? now.word : null;
      if (!neu || neu === s.old || !IDENT.test(neu) || !IDENT.test(s.old)) return;
      return offer(s, neu);
    }
    // text v úvodzovkách: obsah medzi tou istou otváracou a zatváracou úvodzovkou
    const line = s.model.getLineContent(s.line);
    if (line[s.start - 2] !== s.q) return;
    const close = line.indexOf(s.q, s.start - 1);
    if (close < 0) return;
    const neu = line.slice(s.start - 1, close);
    if (!neu || neu === s.old || s.old.length < 2) return;
    // Cesta: zmenil si priečinok v strede („…/Desktop/python/a.py“ → „…/Documents/python/a.py“)?
    // Potom sa ponúkne zmena všetkých ciest, ktoré začínajú tým priečinkom, nielen presne rovnakých.
    const [from, to] = changedPrefix(s.old, neu);
    offer({ ...s, old: from }, to);
  }

  // Najkratší začiatok cesty (po koniec zmeneného úseku), ktorý pokryje celú zmenu.
  function changedPrefix(a, b) {
    if (!/[\\/]/.test(a + b)) return [a, b];
    let p = 0;
    while (p < a.length && p < b.length && a[p] === b[p]) p++;
    let s = 0;
    while (s < a.length - p && s < b.length - p && a[a.length - 1 - s] === b[b.length - 1 - s]) s++;
    // koniec zmeny v starej ceste → po najbližší oddeľovač
    const sep = a.slice(a.length - s).search(/[\\/]/);
    if (sep < 0) return [a, b];
    const endA = a.length - s + sep;
    const endB = b.length - s + sep;
    const from = a.slice(0, endA);
    const to = b.slice(0, endB);
    return from.length >= 2 && to ? [from, to] : [a, b];
  }

  // koniec „úpravy“: kurzor opustil slovo / reťazec
  const outside = (model, pos) => {
    if (pos.lineNumber !== session.line) return true;
    if (session.kind === 'ident') {
      const w = wordAt(model, session.line, session.start);
      const end = w && w.startColumn === session.start ? w.endColumn : session.start;
      return pos.column < session.start || pos.column > end;
    }
    const line = model.getLineContent(session.line);
    const close = line.indexOf(session.q, session.start - 1);
    return pos.column < session.start || close < 0 || pos.column > close + 1;
  };

  editor.onDidChangeCursorPosition((e) => {
    const model = editor.getModel();
    if (!model) return;
    const { lineNumber, column } = e.position;
    if (session) {
      if (outside(model, e.position)) finish();
      else return;
    }
    // Len keď kurzor presunieš ty (klik, šípky) – nie počas písania nového slova.
    if (e.reason === monaco.editor.CursorChangeReason.Explicit || e.source === 'mouse') {
      snap = { model, line: lineNumber, word: wordAt(model, lineNumber, column), str: stringAt(model, lineNumber, column) };
    }
  });

  editor.onDidChangeModelContent((e) => {
    const model = editor.getModel();
    if (!model || e.isUndoing || e.isRedoing || e.isFlush) return;
    if (widget && e.changes.some((c) => c.range.startLineNumber !== widget.getPosition().position.lineNumber)) hide();
    if (session) return;
    const sn = snap;
    snap = null;
    if (!sn || sn.model !== model) return;
    const ch = e.changes[0];
    if (e.changes.length !== 1 || ch.range.startLineNumber !== sn.line || ch.range.endLineNumber !== sn.line || ch.text.includes('\n')) return;
    // v reťazci: úprava vnútri úvodzoviek (bez samotných úvodzoviek)
    if (sn.str && ch.range.startColumn - 1 >= sn.str.start && ch.range.endColumn - 1 <= sn.str.end && !ch.text.includes(sn.str.q)) {
      session = { kind: 'string', model, line: sn.line, start: sn.str.start + 1, old: sn.str.text, q: sn.str.q };
      return;
    }
    const w = sn.word;
    if (!w || /[^\p{L}\p{N}_$]/u.test(ch.text)) return;
    if (ch.range.startColumn < w.startColumn || ch.range.endColumn > w.endColumn) return;
    session = { kind: 'ident', model, line: sn.line, start: w.startColumn, old: w.word };
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
