// Auto Rename Tag – premenuješ <div> a zmení sa aj </div> (a naopak).
const LANGS = new Set(['html', 'xml', 'php', 'vue', 'svelte', 'javascriptreact', 'typescriptreact', 'markdown']);
const NAME = /[\w:.-]/;

export function activate(flux) {
  const { monaco, editor } = flux;
  let tracked = null;
  let applying = false;

  // Tag, v ktorého názve je kurzor: { lt, closing, name, nameStart, nameEnd }
  function tagAt(model, pos) {
    const text = model.getValue();
    const off = model.getOffsetAt(pos);
    let i = off - 1;
    while (i >= 0 && off - i < 80 && NAME.test(text[i])) i--;
    let closing = false;
    if (text[i] === '/') {
      closing = true;
      i--;
    }
    if (i < 0 || text[i] !== '<') return null;
    const nameStart = i + 1 + (closing ? 1 : 0);
    let j = nameStart;
    while (j < text.length && NAME.test(text[j])) j++;
    if (off < nameStart || off > j) return null;
    return { lt: i, closing, name: text.slice(nameStart, j), nameStart, nameEnd: j, text };
  }

  // Nájde párový tag s pôvodným názvom (počíta vnorenie rovnakých tagov).
  function findPair(tag, oldName) {
    const re = /<(\/?)([\w:.-]+)(?:"[^"]*"|'[^']*'|[^'">])*?(\/?)>/g;
    const all = [];
    let m;
    while ((m = re.exec(tag.text))) if (m[2] === oldName && m.index !== tag.lt) all.push({ at: m.index, closing: !!m[1], self: !!m[3] });
    let depth = 0;
    if (!tag.closing) {
      for (const x of all.filter((x) => x.at > tag.lt)) {
        if (x.self) continue;
        if (!x.closing) depth++;
        else if (depth === 0) return x.at + 2;
        else depth--;
      }
    } else {
      for (const x of all.filter((x) => x.at < tag.lt).reverse()) {
        if (x.self) continue;
        if (x.closing) depth++;
        else if (depth === 0) return x.at + 1;
        else depth--;
      }
    }
    return -1;
  }

  const active = () => {
    const model = editor.getModel();
    return model && LANGS.has(model.getLanguageId()) ? model : null;
  };

  flux.own(
    editor.onDidChangeCursorPosition(() => {
      if (applying) return;
      const model = active();
      tracked = model ? tagAt(model, editor.getPosition()) : null;
    }),
  );

  flux.own(
    editor.onDidChangeModelContent((e) => {
      const model = active();
      if (applying || !model || e.isUndoing || e.isRedoing || e.changes.length !== 1) return;
      const cur = tagAt(model, editor.getPosition());
      const before = tracked;
      tracked = cur;
      if (!before || !cur || before.lt !== cur.lt || before.closing !== cur.closing || before.name === cur.name || !before.name) return;
      const at = findPair(cur, before.name);
      if (at < 0) return;
      const start = model.getPositionAt(at);
      const end = model.getPositionAt(at + before.name.length);
      applying = true;
      editor.executeEdits('auto-rename-tag', [{ range: new monaco.Range(start.lineNumber, start.column, end.lineNumber, end.column), text: cur.name }]);
      applying = false;
      tracked = tagAt(model, editor.getPosition());
    }),
  );
}
