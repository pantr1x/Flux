// Vytiahne všetky texty na preklad (t('…')) + texty zo zoznamov (šablóny, odznaky…) do locales/en.keys.json.
import { readFileSync, writeFileSync } from 'node:fs';

const files = ['src/renderer/app.js', 'src/renderer/onboarding.js', 'src/renderer/activity.js', 'src/renderer/tools.js', 'src/renderer/editorExtras.js', 'src/main/toolchains.js', 'src/renderer/aiPanel.js', 'src/renderer/github.js', 'src/renderer/userShortcuts.js', 'src/main/ai.js', 'src/main/github.js', 'src/main/main.js', 'src/main/runner.js', 'src/renderer/keymap.js', 'src/renderer/pluginHost.js', 'src/renderer/pluginsUI.js', 'src/main/plugins.js', 'src/renderer/themeStudio.js', 'src/renderer/updatesUI.js', 'src/main/updater.js', 'src/main/mcpServer.js', 'src/renderer/together.js'];
const keys = new Set();
const lit = (q, body) => body.replace(new RegExp(`\\\\${q}`, 'g'), q).replace(/\\n/g, '\n');
for (const f of files) {
  const src = readFileSync(f, 'utf8');
  for (const m of src.matchAll(/\bt\(\s*(['"])((?:\\.|(?!\1).)*)\1/g)) keys.add(lit(m[1], m[2]));
  // toggle('key', 'Label', 'hint')
  for (const m of src.matchAll(/toggle\('\w+', '([^']+)'(?:, '([^']+)')?\)/g)) {
    keys.add(m[1]);
    if (m[2]) keys.add(m[2]);
  }
  // { … title: '…', sub/text/desc/name/label: '…' } v zoznamoch
  for (const m of src.matchAll(/\b(?:title|sub|text|desc|name|label):\s*(['"])((?:\\.|(?!\1).)*)\1/g)) {
    const v = lit(m[1], m[2]);
    if (/[a-z]/i.test(v) && !/^[\w.-]+\.(py|html|css|js|txt)$/.test(v)) keys.add(v);
  }
}
// keymap.js: ['id', 'kategória', 'Popis', …] a ['id', 'Kategória']
for (const m of readFileSync('src/renderer/keymap.js', 'utf8').matchAll(/^\s*\['\w+', (?:'\w+', )?'([^']+)'/gm)) keys.add(m[1]);
// themeStudio.js: skupiny a popisy farieb
for (const m of readFileSync('src/renderer/themeStudio.js', 'utf8').matchAll(/\['(?:\w+)', '([^']+)'\]/g)) keys.add(m[1]);
for (const m of readFileSync('src/renderer/themeStudio.js', 'utf8').matchAll(/^\s*\['([A-Z]\w+)', \[/gm)) keys.add(m[1]);
const tpl = readFileSync('src/renderer/templates.js', 'utf8');
for (const m of tpl.matchAll(/\b(?:label|detail):\s*(['"])((?:\\.|(?!\1).)*)\1/g)) keys.add(lit(m[1], m[2]));
for (const w of ['compact', 'normal', 'relaxed', 'large']) keys.add(w);
// Mená, ktoré sa neprekladajú.
for (const k of ['Python', 'JavaScript', 'VS Code Dark', 'Consolas', 'Cascadia Code', 'Cascadia Mono', 'Courier New', 'Acrylic (Windows)', 'Mica (Windows)']) keys.delete(k);
const list = [...keys].filter((k) => k.trim()).sort();
writeFileSync('locales/en.keys.json', JSON.stringify(list, null, 1));
console.log(list.length, 'textov');
