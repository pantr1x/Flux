// Error Lens – chyby a varovania priamo na riadku, kde sú (ako rozšírenie Error Lens vo VS Code).
export function activate(flux) {
  const { monaco, editor } = flux;
  const S = monaco.MarkerSeverity;
  flux.ui.addStyle(`
    .el-line-error { background: rgba(255, 90, 90, 0.07); }
    .el-line-warn { background: rgba(255, 196, 0, 0.05); }
    .el-msg { font-style: italic; margin-left: 2.5em; opacity: 0.9; }
    .el-msg-error { color: #ff7b7b !important; }
    .el-msg-warn { color: #e2b93b !important; }
  `);
  const col = flux.decorations([]);
  let on = flux.storage.get('on', true);
  let warnings = flux.storage.get('warnings', true);

  function update() {
    const model = editor.getModel();
    if (!model || !on) return col.clear();
    const min = warnings ? S.Warning : S.Error;
    const byLine = new Map();
    const markers = monaco.editor.getModelMarkers({ resource: model.uri }).filter((m) => m.severity >= min);
    for (const m of markers.sort((a, b) => b.severity - a.severity)) if (!byLine.has(m.startLineNumber)) byLine.set(m.startLineNumber, m);
    col.set(
      [...byLine.values()].flatMap((m) => {
        const err = m.severity === S.Error;
        const line = Math.min(m.startLineNumber, model.getLineCount());
        const end = model.getLineMaxColumn(line);
        const text = m.message.split('\n')[0].slice(0, 160);
        return [
          { range: new monaco.Range(line, 1, line, 1), options: { isWholeLine: true, className: err ? 'el-line-error' : 'el-line-warn' } },
          { range: new monaco.Range(line, end, line, end), options: { showIfCollapsed: true, after: { content: `${err ? '✖' : '⚠'} ${text}`, inlineClassName: `el-msg ${err ? 'el-msg-error' : 'el-msg-warn'}` } } },
        ];
      }),
    );
  }

  flux.own(monaco.editor.onDidChangeMarkers(update));
  flux.own(editor.onDidChangeModel(update));
  flux.onChange(() => setTimeout(update, 50));
  flux.commands.register('toggle', 'Error Lens: turn on / off', () => {
    on = !on;
    flux.storage.set('on', on);
    update();
    flux.toast(on ? 'Error Lens is on.' : 'Error Lens is off.');
  });
  flux.commands.register('warnings', 'Error Lens: show / hide warnings', () => {
    warnings = !warnings;
    flux.storage.set('warnings', warnings);
    update();
  });
  update();
}
