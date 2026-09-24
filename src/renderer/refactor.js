// Zmeny naprieč projektom: premenovanie názvu alebo textu v úvodzovkách vo všetkých súboroch
// a po premenovaní súboru/priečinka oprava ciest, ktoré naň v kóde ukazujú.
// Otvorené súbory sa menia v editore (dá sa to vrátiť Ctrl+Z), ostatné priamo na disku.

// súbory, v ktorých má zmysel hľadať (text, kód)
const TEXT = /\.(py|pyw|js|mjs|cjs|jsx|ts|tsx|html?|css|scss|less|json|md|txt|php|rb|lua|pl|java|kt|c|h|cpp|hpp|cc|cs|go|rs|zig|r|jl|swift|sh|bat|ps1|ya?ml|toml|ini|cfg|xml|svg|vue|svelte)$/i;
const MAX_FILES = 3000;
const MAX_SIZE = 1024 * 1024;

// reťazce v úvodzovkách na riadku: { start (index obsahu), end (index zatváracej úvodzovky), q, text }
export function stringsIn(line) {
  const out = [];
  const re = /(["'`])((?:\\.|(?!\1)[^\\\n])*)\1/g;
  for (let m; (m = re.exec(line)); ) out.push({ start: m.index + 1, end: m.index + 1 + m[2].length, q: m[1], text: m[2] });
  return out;
}

const escRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

// Cesty: porovnávame s '/', bez rozdielu veľkých písmen (Windows) a s vyriešeným . a ..
function normPath(p) {
  const parts = [];
  for (const seg of String(p).replace(/\\/g, '/').split('/')) {
    if (seg === '.' || (seg === '' && parts.length)) continue;
    if (seg === '..' && parts.length > 1) parts.pop();
    else parts.push(seg);
  }
  return parts.join('/').toLowerCase();
}
const dirOf = (p) => String(p).replace(/[\\/][^\\/]*$/, '');

export function createRefactor({ monaco, flux, getWorkspace, langFor }) {
  const modelOf = (file) => monaco.editor.getModel(monaco.Uri.file(file));

  async function files(filter = () => true) {
    const ws = getWorkspace();
    if (!ws) return [];
    const all = (await flux.listAll().catch(() => [])).filter((f) => TEXT.test(f) && filter(f)).slice(0, MAX_FILES);
    const out = [];
    await Promise.all(
      all.map(async (path) => {
        const model = modelOf(path);
        if (model) return out.push({ path, text: model.getValue(), model });
        const text = await flux.read(path).catch(() => null);
        if (typeof text === 'string' && text.length <= MAX_SIZE) out.push({ path, text });
      }),
    );
    return out;
  }

  // Výskyty názvu mimo komentárov a reťazcov (podľa farebného zvýraznenia jazyka).
  function identRanges(text, lang, name) {
    if (!text.includes(name)) return [];
    const lines = text.split('\n');
    let tokens = null;
    const re = new RegExp(`(?<![\\w$])${escRe(name)}(?![\\w$])`, 'g');
    const out = [];
    lines.forEach((line, i) => {
      if (!line.includes(name)) return;
      for (let m; (m = re.exec(line)); ) {
        tokens ||= safeTokenize(text, lang);
        let type = '';
        for (const tk of tokens[i] || []) if (tk.offset <= m.index) type = tk.type;
        if (!/comment|string/.test(type)) out.push({ line: i + 1, start: m.index + 1, end: m.index + 1 + name.length, text: null });
      }
    });
    return out;
  }
  function safeTokenize(text, lang) {
    try {
      return monaco.editor.tokenize(text, lang);
    } catch {
      return [];
    }
  }

  // Reťazce s rovnakým obsahom – aj dlhšie cesty, ktoré ním začínajú („data/img“ → aj „data/img/cat.png“).
  function stringRanges(text, old, neu) {
    if (!text.includes(old)) return [];
    const out = [];
    text.split('\n').forEach((line, i) => {
      if (!line.includes(old)) return;
      for (const s of stringsIn(line)) {
        if (s.text === old || s.text.startsWith(`${old}/`) || s.text.startsWith(`${old}\\`))
          out.push({ line: i + 1, start: s.start + 1, end: s.start + 1 + old.length, text: neu });
      }
    });
    return out;
  }

  // Zmeny v ostatných súboroch projektu: [{ path, model?, text, ranges }]
  async function scanOthers(kind, old, neu, { skip, lang }) {
    const list = await files((f) => f !== skip && (kind !== 'ident' || langFor(f) === lang));
    return list
      .map((f) => ({ ...f, ranges: kind === 'ident' ? identRanges(f.text, lang, old) : stringRanges(f.text, old, neu) }))
      .filter((f) => f.ranges.length);
  }

  // Zapíše zmeny: otvorené súbory cez editor (Ctrl+Z), zatvorené na disk.
  async function apply(changes, neu) {
    let places = 0;
    for (const c of changes) {
      const edits = c.ranges.map((r) => ({ range: new monaco.Range(r.line, r.start, r.line, r.end), text: r.text ?? neu }));
      places += edits.length;
      const model = modelOf(c.path);
      if (model) {
        model.pushStackElement();
        model.pushEditOperations([], edits, () => null);
        model.pushStackElement();
        continue;
      }
      const lines = c.text.split('\n');
      for (const r of [...c.ranges].sort((a, b) => b.line - a.line || b.start - a.start)) {
        const l = lines[r.line - 1];
        lines[r.line - 1] = l.slice(0, r.start - 1) + (r.text ?? neu) + l.slice(r.end - 1);
      }
      await flux.write(c.path, lines.join('\n'));
    }
    return places;
  }

  // Po premenovaní súboru/priečinka: reťazce, ktoré naň ukazujú (relatívne k súboru alebo k projektu, aj „/…“ ako web).
  async function pathRefs(oldPath, newPath) {
    const ws = getWorkspace();
    const oldName = oldPath.replace(/^.*[\\/]/, '');
    const newName = newPath.replace(/^.*[\\/]/, '');
    const oldN = normPath(oldPath);
    const newN = normPath(newPath);
    const hits = (resolved, target) => resolved === target || resolved.startsWith(`${target}/`);
    const resolveAll = (file, s) => {
      if (/^[a-z][\w+.-]*:/i.test(s)) return []; // http:, data:, C:… – nie relatívna cesta
      const bases = s.startsWith('/') ? [ws] : [dirOf(file), ws];
      return bases.map((b) => normPath(`${b}/${s.replace(/^\/+/, '')}`));
    };
    const list = await files();
    const out = [];
    for (const f of list) {
      const ranges = [];
      f.text.split('\n').forEach((line, i) => {
        if (!line.includes(oldName)) return;
        for (const s of stringsIn(line)) {
          if (!s.text.includes(oldName) || !resolveAll(f.path, s.text).some((r) => hits(r, oldN))) continue;
          // nahradiť ten úsek cesty, po ktorého zmene cesta ukazuje na nové miesto
          const segs = s.text.split(/([\\/])/);
          for (let k = 0; k < segs.length; k += 2) {
            if (segs[k] !== oldName) continue;
            const next = [...segs];
            next[k] = newName;
            const cand = next.join('');
            if (resolveAll(f.path, cand).some((r) => hits(r, newN))) {
              ranges.push({ line: i + 1, start: s.start + 1, end: s.end + 1, text: cand });
              break;
            }
          }
        }
      });
      if (ranges.length) out.push({ ...f, ranges });
    }
    return out;
  }

  return { scanOthers, apply, pathRefs, identRanges, stringRanges };
}
