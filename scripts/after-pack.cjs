// electron-builder afterPack: odtlačok všetkého okrem kódu Fluxu (app.asar) → resources/quick-base.txt.
// Rýchla aktualizácia (src/main/quickUpdate.js) sa smie použiť, len keď sa odtlačok medzi verziami nezmenil
// (rovnaký Electron, Pyright, natívne moduly…). Hlavné .exe sa nepočíta – mení sa v ňom len číslo verzie.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

const SKIP = new Set(['resources/app.asar', 'resources/quick-base.txt', 'resources/app-update.yml']);

function files(dir, rel = '') {
  return fs
    .readdirSync(path.join(dir, rel), { withFileTypes: true })
    .flatMap((e) => {
      const r = rel ? `${rel}/${e.name}` : e.name;
      return e.isDirectory() ? files(dir, r) : [r];
    })
    .sort();
}

function quickBase(appOutDir, exeName) {
  const h = crypto.createHash('sha256');
  for (const f of files(appOutDir)) {
    if (SKIP.has(f) || f === exeName) continue;
    h.update(f + '\0');
    h.update(crypto.createHash('sha256').update(fs.readFileSync(path.join(appOutDir, f))).digest());
  }
  return h.digest('hex');
}

module.exports = async (context) => {
  const name = context.packager.appInfo.productFilename;
  const exe = context.electronPlatformName === 'win32' ? `${name}.exe` : context.electronPlatformName === 'darwin' ? '' : name.toLowerCase();
  const base = quickBase(context.appOutDir, exe);
  const res = context.electronPlatformName === 'darwin' ? path.join(context.appOutDir, `${name}.app/Contents/Resources`) : path.join(context.appOutDir, 'resources');
  fs.writeFileSync(path.join(res, 'quick-base.txt'), base + '\n');
  console.log(`  • quick update base ${base.slice(0, 12)}…`);
};
module.exports.quickBase = quickBase;
