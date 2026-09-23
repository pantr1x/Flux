// CSS Class Completion – v class="" ti Flux ponúkne triedy zo všetkých .css súborov projektu
// (a z <style> v otvorenej stránke). Ako „HTML CSS Support“ vo VS Code.
const CLASS = /\.(-?[_a-zA-Z][\w-]*)(?=[^{}]*\{)/g;

export function activate(flux) {
  const { monaco } = flux;
  let cache = { at: 0, classes: new Map() };

  async function projectClasses() {
    if (Date.now() - cache.at < 5000) return cache.classes;
    const classes = new Map(); // názov → súbor
    try {
      const files = (await flux.project.files()).filter((f) => /\.(css|scss|less)$/i.test(f)).slice(0, 200);
      for (const f of files) {
        try {
          const text = (await flux.project.read(f)).replace(/\/\*[\s\S]*?\*\//g, '');
          for (const m of text.matchAll(CLASS)) if (!classes.has(m[1])) classes.set(m[1], f);
        } catch {}
      }
    } catch {}
    cache = { at: Date.now(), classes };
    return classes;
  }

  flux.onSave((file) => {
    if (/\.(css|scss|less)$/i.test(file.name)) cache.at = 0;
  });

  const provider = {
    triggerCharacters: ['"', "'", ' '],
    async provideCompletionItems(model, position) {
      const before = model.getValueInRange({ startLineNumber: position.lineNumber, startColumn: 1, endLineNumber: position.lineNumber, endColumn: position.column });
      if (!/\bclass(Name)?\s*=\s*["'{][^"'}]*$/.test(before)) return { suggestions: [] };
      const classes = new Map(await projectClasses());
      // aj <style> priamo v stránke
      for (const block of model.getValue().matchAll(/<style[^>]*>([\s\S]*?)<\/style>/gi)) for (const m of block[1].matchAll(CLASS)) if (!classes.has(m[1])) classes.set(m[1], 'this page');
      const used = new Set(before.split(/["'\s{]+/));
      const word = model.getWordUntilPosition(position);
      const range = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn);
      return {
        suggestions: [...classes]
          .filter(([name]) => !used.has(name))
          .map(([name, file]) => ({ label: name, kind: monaco.languages.CompletionItemKind.Class, insertText: name, detail: file, range, sortText: `0${name}` })),
      };
    },
  };
  for (const lang of ['html', 'php', 'javascriptreact', 'typescriptreact', 'vue', 'svelte']) flux.own(monaco.languages.registerCompletionItemProvider(lang, provider));
}
