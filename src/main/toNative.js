// Jeden Flux: tento (Electron) Flux sa presunie na Flux Native – rýchlejší, s menšou pamäťou.
// Nastavenia sú spoločné (%APPDATA%\Flux\settings.json), takže projekty aj všetko ostatné ostane.
// Flux Native (`--migrate`) presmeruje odkazy a pripnutie na paneli úloh. Tento program ostáva ako spúšťač:
// typy súborov („Otvoriť vo Fluxe“) ďalej ukazujú sem a súbor sa hneď odovzdá Native. Native nič nepíše do registrov
// ani nemaže iné programy – s tým ho Windows Defender v 0.9.18 zmazal ako trójsky kôň.
// Postup: ak už Native je (settings.nativePath alebo stiahnutý predtým) → odovzdať pri štarte; inak stiahnuť
// na pozadí a odovzdať pri zavretí (bez okna). Chyba (offline, zlý súčet) = Flux beží ďalej ako doteraz.
const { app } = require('electron');
const { spawn } = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const https = require('node:https');
const http = require('node:http');
const path = require('node:path');

const BASE = process.env.FLUX_NATIVE_URL || 'https://raw.githubusercontent.com/pantr1x/Flux/ci-native/';
const DAY = 24 * 3600 * 1000;

function target() {
  return path.join(process.env.LOCALAPPDATA || app.getPath('userData'), 'Programs', 'Flux Native', 'Flux-Native.exe');
}

function enabled() {
  if (process.env.FLUX_NO_NATIVE === '1') return false;
  if (process.env.FLUX_NATIVE_URL) return true; // test
  return process.platform === 'win32' && app.isPackaged;
}

function get(url, redirects = 5) {
  return new Promise((resolve, reject) => {
    const lib = url.startsWith('http:') ? http : https;
    const req = lib.get(url, { headers: { 'User-Agent': 'Flux' } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location && redirects > 0) {
        res.resume();
        resolve(get(new URL(res.headers.location, url).toString(), redirects - 1));
        return;
      }
      if (res.statusCode !== 200) {
        res.resume();
        reject(new Error(`HTTP ${res.statusCode}`));
        return;
      }
      const parts = [];
      res.on('data', (c) => parts.push(c));
      res.on('end', () => resolve(Buffer.concat(parts)));
      res.on('error', reject);
    });
    req.setTimeout(60000, () => req.destroy(new Error('timeout')));
    req.on('error', reject);
  });
}

// cesta k použiteľnému Flux Native, alebo null
function ready(settings) {
  for (const p of [settings.nativePath, target()]) {
    try {
      if (p && fs.statSync(p).size > 1e6) return p;
    } catch {}
  }
  return null;
}

async function download(settings, saveSettings) {
  settings.nativeTry = Date.now();
  saveSettings();
  const info = JSON.parse((await get(BASE + 'native.json')).toString('utf8'));
  const bin = await get(BASE + 'Flux-Native.exe');
  const sum = crypto.createHash('sha256').update(bin).digest('hex');
  if (bin.length !== info.size || sum !== String(info.sha256).toLowerCase()) throw new Error('checksum');
  const out = target();
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out + '.part', bin);
  fs.renameSync(out + '.part', out);
  return out;
}

// súbory z príkazového riadka (dvojklik na súbor v Prieskumníkovi)
function files() {
  return process.argv.slice(1).filter((a) => !a.startsWith('-') && a !== '.' && fs.existsSync(a) && path.resolve(a) !== path.resolve(app.getAppPath()));
}

function handOver(exe, quiet, settings) {
  // už prevedené: len spustiť Native so súborom; inak prvé prevedenie (odkazy, pripnutie)
  const args = settings && settings.migratedFromElectron ? files().slice(0, 1) : ['--migrate', String(process.pid), process.execPath];
  if (quiet) args.push('--no-window');
  if (process.env.FLUX_NATIVE_URL && process.platform !== 'win32') {
    console.log('[toNative] would run', exe, args.join(' '));
    return true;
  }
  try {
    const child = spawn(exe, args, { detached: true, stdio: 'ignore', windowsHide: true });
    child.unref();
    return true;
  } catch {
    return false;
  }
}

// volá sa na začiatku app.whenReady(); true = odovzdané, tento Flux sa zavrie
function start({ settings, saveSettings }) {
  if (!enabled() || process.argv.includes('--background')) return false;
  const exe = ready(settings);
  if (exe && handOver(exe, false, settings)) {
    app.quit();
    return true;
  }
  // stiahnuť na pozadí (najviac raz za deň) a odovzdať pri zavretí
  let got = null;
  if (!settings.nativeTry || Date.now() - settings.nativeTry > DAY || process.env.FLUX_NATIVE_URL) {
    setTimeout(() => {
      download(settings, saveSettings)
        .then((p) => {
          got = p;
          console.log('[toNative] downloaded', p);
        })
        .catch((e) => console.log('[toNative] download failed:', e.message));
    }, process.env.FLUX_NATIVE_URL ? 500 : 20000);
  }
  app.on('will-quit', () => {
    if (got && !settings.migratedFromElectron) handOver(got, true, null);
  });
  return false;
}

module.exports = { start, ready, target };
