// Indent Rainbow – každá úroveň odsadenia má jemnú farbu; zlé odsadenie svieti na červeno.
// Najviac pomôže v Pythone, kde odsadenie rozhoduje o tom, čo patrí do bloku.
const COLORS = ['rgba(255, 214, 102, 0.07)', 'rgba(127, 219, 138, 0.07)', 'rgba(255, 126, 219, 0.07)', 'rgba(94, 197, 255, 0.07)'];

export function activate(flux) {
  const { monaco, editor } = flux;
  flux.ui.addStyle(COLORS.map((c, i) => `.ir-${i} { background: ${c}; }`).join('\n') + '\n.ir-bad { background: rgba(255, 70, 70, 0.22); }');
  const col = flux.decorations([]);
  let timer = 0;

  function update() {
    const model = editor.getModel();
    if (!model) return col.clear();
    const { tabSize, insertSpaces } = model.getOptions();
    const unit = insertSpaces ? tabSize : 1;
    const decos = [];
    for (const r of editor.getVisibleRanges()) {
      for (let line = Math.max(1, r.startLineNumber - 5); line <= Math.min(model.getLineCount(), r.endLineNumber + 5); line++) {
        const text = model.getLineContent(line);
        const lead = text.match(/^[ \t]*/)[0];
        if (!lead.length || lead.length === text.length) continue;
        if (insertSpaces && (lead.includes('\t') || lead.length % unit)) {
          decos.push({ range: new monaco.Range(line, 1, line, lead.length + 1), options: { inlineClassName: 'ir-bad', hoverMessage: { value: 'Indentation does not match the rest of the file' } } });
          continue;
        }
        for (let i = 0; i * unit < lead.length; i++) decos.push({ range: new monaco.Range(line, i * unit + 1, line, Math.min(lead.length, (i + 1) * unit) + 1), options: { inlineClassName: `ir-${i % COLORS.length}` } });
      }
    }
    col.set(decos);
  }
  const later = () => {
    clearTimeout(timer);
    timer = setTimeout(update, 60);
  };
  flux.own(editor.onDidScrollChange(later));
  flux.own(editor.onDidChangeModel(later));
  flux.onChange(later);
  update();
}
