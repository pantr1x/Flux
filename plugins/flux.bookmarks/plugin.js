// Bookmarks – označ si dôležité riadky a skáč medzi nimi (Ctrl+Alt+K / J / L), aj medzi súbormi.
export function activate(flux) {
  const { monaco, editor } = flux;
  flux.ui.addStyle(`
    .bm-glyph::before { content: ''; display: block; width: 9px; height: 12px; margin: 3px 0 0 5px; background: #4ea1ff;
      clip-path: polygon(0 0, 100% 0, 100% 100%, 50% 72%, 0 100%); }
    .bm-line { background: rgba(78, 161, 255, 0.08); }
  `);
  const col = flux.decorations([]);
  let marks = flux.storage.get('marks', {}); // cesta → [čísla riadkov]
  const item = flux.statusBar.add({ title: 'Bookmarks – click to jump to the next one', onClick: () => jump(1) });

  const path = () => flux.activeFile()?.path;
  const lines = () => (path() ? marks[path()] || [] : []);

  function draw() {
    const model = editor.getModel();
    if (!model) return col.clear();
    col.set(
      lines()
        .filter((l) => l <= model.getLineCount())
        .map((l) => ({ range: new monaco.Range(l, 1, l, 1), options: { isWholeLine: true, glyphMarginClassName: 'bm-glyph', className: 'bm-line', overviewRuler: { color: '#4ea1ff', position: monaco.editor.OverviewRulerLane.Left }, stickiness: 1 } })),
    );
    const total = Object.values(marks).reduce((n, l) => n + l.length, 0);
    item.set(`🔖 ${total}`);
    item.show(total > 0);
  }

  function save() {
    // riadky sa pri písaní posúvajú spolu s dekoráciami
    const model = editor.getModel();
    if (model && path()) {
      const now = [...new Set(col.getRanges().map((r) => r.startLineNumber))].sort((a, b) => a - b);
      if (now.length) marks[path()] = now;
      else delete marks[path()];
    }
    flux.storage.set('marks', marks);
  }

  function toggle() {
    const p = path();
    const line = editor.getPosition()?.lineNumber;
    if (!p || !line) return;
    const list = new Set(marks[p] || []);
    list.has(line) ? list.delete(line) : list.add(line);
    marks[p] = [...list].sort((a, b) => a - b);
    if (!marks[p].length) delete marks[p];
    draw();
    flux.storage.set('marks', marks);
  }

  function jump(dir) {
    const here = editor.getPosition()?.lineNumber || 0;
    const list = lines();
    const next = dir > 0 ? list.find((l) => l > here) ?? list[0] : [...list].reverse().find((l) => l < here) ?? list[list.length - 1];
    if (!next) return flux.toast(Object.keys(marks).length ? 'No bookmarks in this file.' : 'No bookmarks yet – press Ctrl+Alt+K on a line.');
    editor.setPosition({ lineNumber: next, column: 1 });
    editor.revealLineInCenter(next);
    editor.focus();
  }

  let timer = 0;
  flux.onChange(() => {
    clearTimeout(timer);
    timer = setTimeout(save, 400);
  });
  flux.own(editor.onDidChangeModel(draw));
  flux.onOpen(draw);
  flux.commands.register('toggle', 'Bookmarks: add / remove on this line', toggle, { key: 'Ctrl+Alt+K' });
  flux.commands.register('next', 'Bookmarks: next', () => jump(1), { key: 'Ctrl+Alt+L' });
  flux.commands.register('prev', 'Bookmarks: previous', () => jump(-1), { key: 'Ctrl+Alt+J' });
  flux.commands.register('clear', 'Bookmarks: remove all in this file', () => {
    if (path()) delete marks[path()];
    draw();
    flux.storage.set('marks', marks);
  });
  draw();
}
