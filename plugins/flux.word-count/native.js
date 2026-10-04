// Word Count pre Flux Native – slová, znaky a čas čítania súboru alebo výberu v stavovom riadku.
export function activate(flux) {
  const item = flux.statusBar.add({ text: '', onClick: () => show() });
  const count = (text) => {
    const words = (text.match(/[\p{L}\p{N}'’-]+/gu) || []).length;
    const lines = text ? text.split('\n').length : 0;
    return { words, chars: text.length, lines, minutes: Math.max(1, Math.round(words / 200)) };
  };
  const pick = (f) => (f.selection && f.selection.text ? f.selection.text : f.text);
  function update() {
    const f = flux.activeFile();
    if (!f) return item.show(false);
    item.show(true);
    const sel = f.selection && f.selection.text;
    const c = count(pick(f));
    item.set(`${sel ? '✂ ' : ''}${c.words} words`);
    item.setTitle(`${c.chars} characters · ${c.lines} lines · ~${c.minutes} min read`);
  }
  function show() {
    const f = flux.activeFile();
    if (!f) return;
    const c = count(pick(f));
    flux.toast(`${f.selection && f.selection.text ? 'Selection' : f.name}: ${c.words} words, ${c.chars} characters, ${c.lines} lines, about ${c.minutes} min to read.`);
  }
  flux.onChange(update);
  flux.onSelection(update);
  flux.onOpen(update);
  flux.commands.register('show', 'Show word count', show);
  update();
}
