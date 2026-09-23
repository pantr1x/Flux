// TODO Highlight – zvýrazní TODO, FIXME, HACK a NOTE v kóde a spočíta ich.
const TAGS = { TODO: 'todo', FIXME: 'fixme', HACK: 'hack', NOTE: 'note', BUG: 'fixme' };
const RE = /\b(TODO|FIXME|HACK|NOTE|BUG)\b[:\s]?/g;

export function activate(flux) {
  const { monaco, editor } = flux;
  flux.ui.addStyle(`
    .fx-todo { background: rgba(245, 197, 66, .22); color: #f5c542 !important; border-radius: 3px; font-weight: 700; }
    .fx-fixme { background: rgba(255, 107, 122, .22); color: #ff6b7a !important; border-radius: 3px; font-weight: 700; }
    .fx-hack { background: rgba(255, 158, 100, .22); color: #ff9e64 !important; border-radius: 3px; font-weight: 700; }
    .fx-note { background: rgba(125, 211, 252, .20); color: #7dd3fc !important; border-radius: 3px; font-weight: 700; }
  `);
  const deco = flux.decorations([]);
  const item = flux.statusBar.add({ onClick: () => next() });
  let found = [];

  function scan() {
    const model = editor.getModel();
    found = [];
    if (!model) {
      deco.clear();
      item.show(false);
      return;
    }
    const items = [];
    const lines = model.getLineCount();
    for (let ln = 1; ln <= lines; ln++) {
      const text = model.getLineContent(ln);
      for (const m of text.matchAll(RE)) {
        const range = new monaco.Range(ln, m.index + 1, ln, m.index + 1 + m[1].length);
        found.push(range);
        items.push({ range, options: { inlineClassName: `fx-${TAGS[m[1]]}`, overviewRuler: { color: '#f5c542', position: monaco.editor.OverviewRulerLane.Right }, hoverMessage: { value: `**${m[1]}** – ${text.slice(m.index + m[0].length).trim()}` } } });
      }
    }
    deco.set(items);
    item.show(found.length > 0);
    item.set(`☑ ${found.length} TODO${found.length === 1 ? '' : 's'}`);
    item.setTitle('Click to jump to the next one');
  }

  function next() {
    if (!found.length) return flux.toast('No TODOs in this file 🎉');
    const line = editor.getPosition()?.lineNumber || 0;
    const target = found.find((r) => r.startLineNumber > line) || found[0];
    editor.setSelection(target);
    editor.revealRangeInCenter(target);
    editor.focus();
  }

  let timer = 0;
  const later = () => {
    clearTimeout(timer);
    timer = setTimeout(scan, 250);
  };
  flux.onChange(later);
  flux.onOpen(later);
  flux.commands.register('next', 'Go to next TODO', next, { key: 'Ctrl+Alt+N' });
  scan();
}
