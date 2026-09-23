// Word Count – počet slov a znakov (celý súbor alebo výber) v stavovom riadku.
export function activate(flux) {
  const item = flux.statusBar.add({ onClick: () => show() });
  const count = (text) => {
    const words = (text.match(/[\p{L}\p{N}'’-]+/gu) || []).length;
    const chars = text.length;
    const lines = text ? text.split('\n').length : 0;
    return { words, chars, lines, minutes: Math.max(1, Math.round(words / 200)) };
  };
  function update() {
    const f = flux.activeFile();
    if (!f) return item.show(false);
    item.show(true);
    const sel = f.selection;
    const c = count(sel || f.text);
    item.set(`${sel ? '✂ ' : ''}${c.words.toLocaleString()} words`);
    item.setTitle(`${c.chars.toLocaleString()} characters · ${c.lines} lines · ~${c.minutes} min read`);
  }
  function show() {
    const f = flux.activeFile();
    if (!f) return;
    const c = count(f.selection || f.text);
    flux.toast(`${f.selection ? 'Selection' : f.name}: ${c.words} words, ${c.chars} characters, ${c.lines} lines, about ${c.minutes} min to read.`, 'info', 6000);
  }
  let timer = 0;
  const later = () => {
    clearTimeout(timer);
    timer = setTimeout(update, 200);
  };
  flux.onChange(later);
  flux.onSelection(later);
  flux.onOpen(later);
  flux.commands.register('show', 'Show word count', show);
  update();
}
