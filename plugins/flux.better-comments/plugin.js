// Better Comments – farebné komentáre: ! dôležité, ? otázka, TODO úloha, * zvýraznené, // zrušené.
const MARK = {
  python: '#', ruby: '#', shell: '#', powershell: '#', r: '#', perl: '#', yaml: '#', ini: '#', dockerfile: '#',
  lua: '--', sql: '--',
  html: '<!--', xml: '<!--', markdown: '<!--',
  css: '/*', scss: '//', less: '//',
};
const TAGS = [
  [/^!/, 'bc-alert'],
  [/^\?/, 'bc-question'],
  [/^(TODO|FIXME|HACK|XXX)\b/i, 'bc-todo'],
  [/^\*(?!\/)/, 'bc-highlight'],
  [/^\/\//, 'bc-strike'],
];

export function activate(flux) {
  const { monaco, editor } = flux;
  flux.ui.addStyle(`
    .bc-alert { color: #ff5f5f !important; }
    .bc-question { color: #3fa9f5 !important; }
    .bc-todo { color: #ff9d3b !important; font-weight: 600; }
    .bc-highlight { color: #98c379 !important; }
    .bc-strike { color: #7b7b87 !important; text-decoration: line-through; }
  `);
  const col = flux.decorations([]);
  let timer = 0;

  function update() {
    const model = editor.getModel();
    if (!model) return col.clear();
    const mark = MARK[model.getLanguageId()] || '//';
    const esc = mark.replace(/[*/?!.-]/g, '\\$&');
    const re = new RegExp(`${esc}\\s?`);
    const decos = [];
    const n = Math.min(model.getLineCount(), 20000);
    for (let line = 1; line <= n; line++) {
      const text = model.getLineContent(line);
      const m = re.exec(text);
      if (!m) continue;
      // len skutočné komentáre (nie # vnútri reťazca)
      const before = text.slice(0, m.index);
      if ((before.match(/["'`]/g) || []).length % 2) continue;
      const rest = text.slice(m.index + m[0].length);
      const tag = TAGS.find(([r]) => r.test(rest));
      if (tag) decos.push({ range: new monaco.Range(line, m.index + 1, line, text.length + 1), options: { inlineClassName: tag[1] } });
    }
    col.set(decos);
  }
  const later = () => {
    clearTimeout(timer);
    timer = setTimeout(update, 120);
  };
  flux.own(editor.onDidChangeModel(later));
  flux.onChange(later);
  update();
}
