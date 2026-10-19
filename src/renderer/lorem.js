// Lorem ipsum – výplňový text. V editore: „lorem“ + Tab (30 slov), „lorem10“ (10 slov),
// „lorem3s“ (3 vety), „lorem2p“ (2 odseky). Príkaz „Insert lorem ipsum…“ v palete vloží text na kurzor.
import { t } from './i18n.js';

const WORDS = (
  'lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua ' +
  'enim ad minim veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea commodo consequat duis aute irure in ' +
  'reprehenderit voluptate velit esse cillum fugiat nulla pariatur excepteur sint occaecat cupidatat non proident sunt culpa ' +
  'qui officia deserunt mollit anim id est laborum praesent elementum facilisis leo vel fringilla nunc mattis pellentesque ' +
  'habitant morbi tristique senectus netus malesuada fames ac turpis egestas integer feugiat scelerisque varius cras semper ' +
  'auctor neque vitae tortor condimentum lacinia viverra accumsan mauris pharetra massa ultricies mi purus faucibus ornare ' +
  'suspendisse sapien nec sagittis aliquam etiam erat velit blandit volutpat maecenas gravida arcu dictum tellus at urna'
).split(' ');
const OPENING = ['lorem', 'ipsum', 'dolor', 'sit', 'amet', 'consectetur', 'adipiscing', 'elit'];

const pick = () => WORDS[Math.floor(Math.random() * WORDS.length)];
const cap = (s) => s[0].toUpperCase() + s.slice(1);

// n slov ako vety (veľké písmeno, čiarky, bodka); prvý text vždy začína „Lorem ipsum dolor sit amet…“
function words(n, first = true) {
  const list = Array.from({ length: n }, (_, i) => (first && i < OPENING.length ? OPENING[i] : pick()));
  let out = '';
  let len = 0;
  let goal = 6 + Math.floor(Math.random() * 9);
  list.forEach((w, i) => {
    const start = len === 0;
    out += (i ? ' ' : '') + (start ? cap(w) : w);
    len++;
    const last = i === list.length - 1;
    if (last || len >= goal) {
      out += '.';
      len = 0;
      goal = 6 + Math.floor(Math.random() * 9);
    } else if (len > 3 && Math.random() < 0.12) out += ',';
  });
  return out;
}
const sentence = (first) => words(6 + Math.floor(Math.random() * 9), first).replace(/\.(?=.)/g, ',');
const sentences = (n, first = true) => Array.from({ length: n }, (_, i) => sentence(first && i === 0)).join(' ');
const paragraphs = (n, first = true) => Array.from({ length: n }, (_, i) => sentences(4 + Math.floor(Math.random() * 3), first && i === 0));

// unit: w = slová, s = vety, p = odseky; lang rozhodne, ako sa oddelia odseky
export function lorem(count, unit = 'w', lang = 'plaintext') {
  if (unit === 's') return sentences(count);
  if (unit !== 'p') return words(count);
  const list = paragraphs(count);
  return lang === 'html' ? list.map((p) => `<p>${p}</p>`).join('\n') : list.join('\n\n');
}

export function setupLorem({ monaco, editor, confirmPalette }) {
  monaco.languages.registerCompletionItemProvider('*', {
    provideCompletionItems(model, position) {
      const word = model.getWordUntilPosition(position);
      const before = model.getLineContent(position.lineNumber).slice(0, position.column - 1);
      const m = before.match(/(?:^|[^\w$])(lorem(\d*)([wsp]?))$/i);
      if (!m) return { suggestions: [] };
      const lang = model.getLanguageId();
      // v HTML samotné lorem / lorem10 rozbalí Emmet – tu len vety a odseky
      if (lang === 'html' && !m[3]) return { suggestions: [] };
      const count = Math.min(200, Number(m[2]) || (m[3] ? 1 : 30));
      const unit = (m[3] || 'w').toLowerCase();
      const what = { w: t('{n} words', { n: count }), s: t('{n} sentences', { n: count }), p: t('{n} paragraphs', { n: count }) }[unit];
      const range = new monaco.Range(position.lineNumber, position.column - m[1].length, position.lineNumber, word.endColumn);
      return {
        suggestions: [
          {
            label: { label: m[1], description: `Lorem ipsum · ${what}` },
            kind: monaco.languages.CompletionItemKind.Text,
            insertText: lorem(count, unit, lang),
            range,
            sortText: '0',
            documentation: t('Placeholder text. lorem10 = 10 words, lorem3s = 3 sentences, lorem2p = 2 paragraphs.'),
          },
        ],
      };
    },
  });

  // príkaz v palete: vloží text na kurzor (alebo nahradí označený)
  return async function insertLorem() {
    const model = editor.getModel();
    if (!model) return;
    const choice = await confirmPalette(t('Insert lorem ipsum'), [
      { label: t('One sentence'), value: [1, 's'] },
      { label: t('{n} words', { n: 30 }), value: [30, 'w'] },
      { label: t('One paragraph'), value: [1, 'p'] },
      { label: t('{n} paragraphs', { n: 3 }), value: [3, 'p'] },
      { label: t('{n} paragraphs', { n: 5 }), value: [5, 'p'] },
    ]);
    if (!choice) return;
    const text = lorem(choice[0], choice[1], model.getLanguageId());
    const sel = editor.getSelection();
    editor.executeEdits('lorem', [{ range: sel, text, forceMoveMarkers: true }]);
    editor.pushUndoStop();
    editor.focus();
  };
}
