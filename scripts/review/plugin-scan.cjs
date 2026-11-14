// Kontrola kódu pluginu podľa pevných pravidiel – bez spúšťania, len čítanie textu.
// Používa ju Flux pred inštaláciou (plugins.js → upozornenie „Tento plugin môže byť nebezpečný“)
// aj kontrola pull requestov na GitHube (scripts/plugin-review.mjs), takže pravidlá sú na jednom mieste.
// block = vážne (pravdepodobne škodlivé alebo zakázané), warn = treba vysvetlenie (napr. internet).

// [regex, úroveň, správa pre človeka (anglicky – preklad robí volajúci cez t())]
const RULES = [
  [/\beval\s*\(/, 'block', 'runs text as code (eval)'],
  [/\bnew\s+Function\s*\(/, 'block', 'runs text as code (new Function)'],
  [/\bwindow\.flux\b|\bglobalThis\.flux\b/, 'block', 'uses the Flux app bridge directly (files and programs on your PC)'],
  [/\brequire\s*\(|\bprocess\.(env|exec|binding)|child_process|ipcRenderer/, 'block', 'uses Node.js or Electron (full access to your PC)'],
  [/\bimport\s*\(\s*['"`]?https?:/, 'block', 'downloads and runs code from the internet'],
  [/\\x[0-9a-f]{2}(\\x[0-9a-f]{2}){7,}/i, 'block', 'contains hidden (encoded) text'],
  [/document\.createElement\s*\(\s*['"`]script/i, 'block', 'adds a script to the window'],
  [/document\.cookie|localStorage|sessionStorage|indexedDB/, 'warn', 'reads or writes browser storage'],
  [/\bfetch\s*\(|XMLHttpRequest|WebSocket|sendBeacon|EventSource/, 'warn', 'connects to the internet'],
  [/\batob\s*\(|String\.fromCharCode\s*\(\s*\d+\s*,\s*\d+/, 'warn', 'decodes hidden text'],
];

const CODE = /\.(m?js)$/i;

// files: [{ path, text }] → [{ level, file, line, text }]
function scanFiles(files) {
  const out = [];
  for (const f of files) {
    if (/\.svg$/i.test(f.path) && /<script|\son\w+\s*=|javascript:/i.test(f.text)) out.push({ level: 'block', file: f.path, line: 0, text: 'image with hidden code (SVG script)' });
    if (!CODE.test(f.path)) continue;
    f.text.split('\n').forEach((line, i) => {
      if (line.length > 800) out.push({ level: 'block', file: f.path, line: i + 1, text: 'minified or hidden code (very long line)' });
      for (const [re, level, text] of RULES) if (re.test(line)) out.push({ level, file: f.path, line: i + 1, text });
    });
  }
  return out;
}

module.exports = { RULES, scanFiles };
