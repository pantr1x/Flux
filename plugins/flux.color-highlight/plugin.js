// Color Highlight – farby ako #6b5cff alebo rgb(…) ukáže ako malý štvorček v akomkoľvek súbore
// (Python, JavaScript, JSON…); klik naň otvorí výber farby.
const LANGS = ['python', 'javascript', 'typescript', 'json', 'markdown', 'plaintext', 'rust', 'go', 'java', 'cpp', 'c', 'csharp', 'php', 'ruby', 'lua', 'yaml', 'xml', 'shell', 'ini'];
const RE = /#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})\b|rgba?\(\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3}\s*(?:,\s*(?:0|1|0?\.\d+)\s*)?\)/g;

function parse(s) {
  if (s[0] === '#') {
    let h = s.slice(1);
    if (h.length <= 4) h = [...h].map((c) => c + c).join('');
    const n = (i) => parseInt(h.slice(i, i + 2), 16) / 255;
    return { red: n(0), green: n(2), blue: n(4), alpha: h.length === 8 ? n(6) : 1 };
  }
  const [r, g, b, a = 1] = s.match(/[\d.]+/g).map(Number);
  if (r > 255 || g > 255 || b > 255) return null;
  return { red: r / 255, green: g / 255, blue: b / 255, alpha: a };
}

const hex = (c) => {
  const p = (x) => Math.round(x * 255).toString(16).padStart(2, '0');
  return `#${p(c.red)}${p(c.green)}${p(c.blue)}${c.alpha < 1 ? p(c.alpha) : ''}`;
};

export function activate(flux) {
  const { monaco } = flux;
  const provider = {
    provideDocumentColors(model) {
      const out = [];
      const text = model.getValue();
      if (text.length > 1_000_000) return out;
      for (const m of text.matchAll(RE)) {
        const color = parse(m[0]);
        if (!color) continue;
        const a = model.getPositionAt(m.index);
        const b = model.getPositionAt(m.index + m[0].length);
        out.push({ color, range: new monaco.Range(a.lineNumber, a.column, b.lineNumber, b.column) });
      }
      return out;
    },
    provideColorPresentations(model, info) {
      const c = info.color;
      const old = model.getValueInRange(info.range);
      const rgb = `rgb${c.alpha < 1 ? 'a' : ''}(${[c.red, c.green, c.blue].map((x) => Math.round(x * 255)).join(', ')}${c.alpha < 1 ? `, ${+c.alpha.toFixed(2)}` : ''})`;
      return old.startsWith('rgb') ? [{ label: rgb }, { label: hex(c) }] : [{ label: hex(c) }, { label: rgb }];
    },
  };
  for (const lang of LANGS) flux.own(monaco.languages.registerColorProvider(lang, provider));
}
