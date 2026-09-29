// Vygeneruje desktop/src/bridge.js – window.flux pre Tauri verziu Fluxu.
// Zoznam funkcií sa berie zo src/preload.js (Electron), takže obe verzie majú vždy rovnaké API:
//   invoke('kanál') → Rust príkaz „ipc“ s odpoveďou, send('kanál') → bez odpovede, on('kanál') → udalosť z Rustu.
import { readFileSync, writeFileSync } from 'node:fs';

const src = readFileSync('src/preload.js', 'utf8');
const api = {};
for (const m of src.matchAll(/^\s{2}(\w+):\s*(.+?),?$/gm)) {
  const [, name, body] = m;
  const ch = body.match(/ipcRenderer\.(invoke|send)\('([^']+)'/);
  const ev = body.match(/(?:^on\('|ipcRenderer\.on\(')([^']+)'/);
  if (ch) api[name] = [ch[1], ch[2]];
  else if (ev) api[name] = ['on', ev[1]];
}

const out = `// VYGENEROVANÉ scripts/tauri-bridge.mjs zo src/preload.js – neupravuj ručne.
(() => {
  const T = () => window.__TAURI__;
  const call = (ch, args) => T().core.invoke('ipc', { ch, args });
  const on = (ch) => (cb) => {
    let off = null;
    let dead = false;
    T().event.listen(ch, (e) => cb(e.payload)).then((u) => (dead ? u() : (off = u)));
    return () => (dead = true, off?.());
  };
  const API = ${JSON.stringify(api)};
  const flux = {};
  for (const [name, [kind, ch]] of Object.entries(API)) {
    if (kind === 'on') flux[name] = on(ch);
    else if (kind === 'send') flux[name] = (...a) => void call(ch, a).catch(() => {});
    else flux[name] = (...a) => call(ch, a);
  }
  // súbor pretiahnutý do okna – WebView dáva cestu cez udalosť Tauri, nie cez File
  flux.pathForFile = (f) => f?.path || '';
  flux.tauri = true;
  window.flux = flux;
  // chyby okna do výstupu programu (ľahšie ladenie Tauri verzie)
  const log = (msg) => call('log', [String(msg)]).catch(() => {});
  addEventListener('error', (e) => log(\`error: \${e.message} @ \${e.filename}:\${e.lineno}\`));
  addEventListener('unhandledrejection', (e) => log(\`rejection: \${e.reason?.message || e.reason}\`));
})();
`;
writeFileSync('desktop/src/bridge.js', out);
console.log(`bridge.js: ${Object.keys(api).length} functions`);
