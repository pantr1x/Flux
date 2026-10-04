// TODO Highlight – TODO, FIXME, HACK a NOTE v komentároch farebne a s textom za riadkom (flux.marks).
const WORDS = { TODO: '#e5c07b', FIXME: '#e06c75', HACK: '#c678dd', NOTE: '#61afef', BUG: '#e06c75' };

export function activate(flux) {
  const re = /\b(TODO|FIXME|HACK|NOTE|BUG)\b[:\s]*(.*)/;
  let found = [];
  function scan() {
    const f = flux.activeFile();
    if (!f) return flux.marks.clear();
    found = [];
    const marks = [];
    f.text.split('\n').forEach((line, i) => {
      const m = re.exec(line);
      if (!m) return;
      found.push({ line: i, word: m[1], text: m[2].trim() });
      marks.push({ line: i, col: m.index, len: m[1].length, color: WORDS[m[1]], message: m[2].trim() || m[1] });
    });
    flux.marks.set(marks);
    status.set(found.length ? `☑ ${found.length}` : '');
    status.setTitle(found.length ? `${found.length} TODO/FIXME in this file` : '');
  }
  const status = flux.statusBar.add({ text: '', onClick: () => list() });
  function list() {
    if (!found.length) return flux.toast('No TODO in this file.');
    flux.toast(found.slice(0, 5).map((x) => `${x.line + 1}: ${x.word} ${x.text}`).join(' · '));
  }
  flux.onOpen(scan);
  flux.onChange(scan);
  flux.commands.register('list', 'List TODOs in this file', list);
  scan();
}
