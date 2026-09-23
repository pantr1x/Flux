// Python Docstring – pod "def" napíš """ a Flux doplní popis funkcie s parametrami (ako autoDocstring vo VS Code).
export function activate(flux) {
  const { editor } = flux;
  let busy = false;

  function splitTop(s) {
    const out = [];
    let depth = 0;
    let cur = '';
    for (const ch of s) {
      if ('([{'.includes(ch)) depth++;
      if (')]}'.includes(ch)) depth--;
      if (ch === ',' && depth === 0) {
        out.push(cur);
        cur = '';
      } else cur += ch;
    }
    if (cur.trim()) out.push(cur);
    return out.map((x) => x.trim()).filter(Boolean);
  }

  // Hlavička funkcie nad riadkom `line` (aj na viac riadkov).
  function signatureAbove(model, line) {
    let text = '';
    for (let l = line - 1; l >= Math.max(1, line - 15); l--) {
      text = `${model.getLineContent(l)}\n${text}`;
      const m = text.match(/^\s*(?:async\s+)?def\s+(\w+)\s*\(([\s\S]*)\)\s*(?:->\s*([^:]+))?:\s*$/);
      if (m) return { name: m[1], params: m[2], ret: (m[3] || '').trim() };
      if (/^\s*(class|def|async def)\b/.test(model.getLineContent(l))) return null;
    }
    return null;
  }

  function docstring(sig) {
    let n = 1;
    const lines = ['"""${' + n++ + ':Short description of ' + sig.name + '.}'];
    const params = splitTop(sig.params.replace(/\n/g, ' '))
      .map((p) => p.replace(/=.*$/, '').trim())
      .filter((p) => p && !/^(self|cls|\*|\/)$/.test(p));
    if (params.length) {
      lines.push('', 'Args:');
      for (const p of params) {
        const [name, type] = p.split(':').map((x) => x.trim());
        lines.push(`    ${name.replace(/^\*+/, '')}${type ? ` (${type})` : ''}: \${${n++}:description}`);
      }
    }
    if (sig.ret && sig.ret !== 'None') lines.push('', 'Returns:', `    ${sig.ret}: \${${n++}:description}`);
    lines.push('"""');
    // odsadenie ďalších riadkov doplní editor sám (podľa riadku, kde sa vkladá)
    return lines.join('\n');
  }

  function insertAt(line) {
    const model = editor.getModel();
    if (!model || model.getLanguageId() !== 'python') return false;
    const sig = signatureAbove(model, line);
    if (!sig) return false;
    const content = model.getLineContent(line);
    const indent = content.match(/^\s*/)[0] || `${model.getLineContent(line - 1).match(/^\s*/)[0]}    `;
    busy = true;
    editor.setSelection({ startLineNumber: line, startColumn: indent.length + 1, endLineNumber: line, endColumn: content.length + 1 });
    if (!content.trim()) editor.executeEdits('docstring', [{ range: editor.getSelection(), text: '' }]);
    editor.getContribution('snippetController2').insert(docstring(sig));
    busy = false;
    return true;
  }

  flux.own(
    editor.onDidChangeModelContent((e) => {
      const model = editor.getModel();
      if (busy || !model || model.getLanguageId() !== 'python' || e.isUndoing || !e.changes.some((c) => c.text.includes('"'))) return;
      const line = editor.getPosition().lineNumber;
      if (/^\s*"{3,6}\s*$/.test(model.getLineContent(line))) setTimeout(() => insertAt(line), 0);
    }),
  );

  flux.commands.register(
    'generate',
    'Python: generate docstring',
    () => {
      const line = editor.getPosition()?.lineNumber;
      const model = editor.getModel();
      if (!line || !model) return;
      // Kurzor na riadku s "def": vloží nový riadok pod ním.
      if (/^\s*(async\s+)?def\b/.test(model.getLineContent(line)) && /:\s*$/.test(model.getLineContent(line))) {
        const indent = `${model.getLineContent(line).match(/^\s*/)[0]}    `;
        const end = model.getLineMaxColumn(line);
        editor.executeEdits('docstring', [{ range: { startLineNumber: line, startColumn: end, endLineNumber: line, endColumn: end }, text: `\n${indent}` }]);
        return insertAt(line + 1) || flux.toast('Put the cursor on a line with "def".');
      }
      if (!insertAt(line)) flux.toast('Put the cursor on a line with "def".');
    },
    { key: 'Ctrl+Shift+2' },
  );
}
