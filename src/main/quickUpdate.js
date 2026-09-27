// Rýchla aktualizácia: keď sa medzi verziami zmenil len kód Fluxu (app.asar), stiahne sa iba ten
// (~6 MB, .gz) namiesto celého inštalátora a pri reštarte sa vymení – bez inštalácie, za pár sekúnd.
// Ku každému vydaniu patrí quick.json (scripts/quick-pack.mjs): verzia, sha512 a „base“ = odtlačok všetkého
// ostatného (Electron, Pyright, natívne moduly; scripts/after-pack.cjs → resources/quick-base.txt).
// Rýchla cesta ide len keď base sedí s nainštalovaným Fluxom a priečinok je zapisovateľný – inak celý inštalátor.
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const zlib = require('node:zlib');
const crypto = require('node:crypto');
const { spawn } = require('node:child_process');

// Pomocník beží ako samostatný Node (Electron s ELECTRON_RUN_AS_NODE) až po zatvorení Fluxu:
// počká, kým Flux skončí, vymení app.asar a (ak treba) Flux znova spustí.
const HELPER = `
const fs = require('fs'), path = require('path'), { spawn } = require('child_process');
const [pid, dir, exe, relaunch, args] = process.argv.slice(2);
const alive = () => { try { process.kill(Number(pid), 0); return true; } catch { return false; } };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
(async () => {
  for (let i = 0; i < 150 && alive(); i++) await sleep(200);
  await sleep(300);
  const cur = path.join(dir, 'app.asar'), nw = cur + '.new', old = cur + '.old';
  for (let i = 0; i < 50 && fs.existsSync(nw); i++) {
    try {
      fs.rmSync(old, { force: true });
      fs.renameSync(cur, old);
      try { fs.renameSync(nw, cur); } catch (e) { fs.renameSync(old, cur); throw e; }
    } catch { await sleep(200); }
  }
  try { fs.rmSync(old, { force: true }); } catch {}
  if (relaunch === '1') {
    const env = { ...process.env };
    delete env.ELECTRON_RUN_AS_NODE;
    spawn(exe, JSON.parse(args || '[]'), { detached: true, stdio: 'ignore', env, windowsHide: false }).unref();
  }
})();
`;

function createQuickUpdate({ repo, fetch, resourcesDir, execPath, baseUrl }) {
  const asar = path.join(resourcesDir, 'app.asar');
  const url = (v, file) => `${baseUrl || `https://github.com/${repo}/releases/download`}/v${v}/${file}`;
  let ready = null; // verzia pripravená v app.asar.new
  let pending = null; // { version, promise }

  const base = () => {
    try {
      return fs.readFileSync(path.join(resourcesDir, 'quick-base.txt'), 'utf8').trim();
    } catch {
      return '';
    }
  };
  // Inštalácia pre všetkých (Program Files) nie je zapisovateľná bez správcu – tam ide inštalátor.
  function writable() {
    const probe = path.join(resourcesDir, `.flux-write-${process.pid}`);
    try {
      fs.writeFileSync(probe, '');
      fs.rmSync(probe, { force: true });
      return true;
    } catch {
      return false;
    }
  }
  const possible = () => !!base() && fs.existsSync(asar) && writable();

  // Pozostatky po nedokončenej výmene (starší ako 2 min – pomocník medzitým mohol ešte bežať).
  function cleanup() {
    for (const f of ['app.asar.new', 'app.asar.old']) {
      const p = path.join(resourcesDir, f);
      try {
        if (Date.now() - fs.statSync(p).mtimeMs > 2 * 60 * 1000) fs.rmSync(p, { force: true });
      } catch {}
    }
  }

  // Stiahne a overí app.asar pre verziu v. true = pripravené, false = treba celý inštalátor.
  async function fetchVersion(v, onProgress = () => {}) {
    if (ready === v) return true;
    if (pending?.version === v) return pending.promise;
    const promise = (async () => {
      if (!possible()) return false;
      const res = await fetch(url(v, 'quick.json'));
      if (!res.ok) return false;
      const info = await res.json();
      if (info.version !== v || !info.base || info.base !== base() || !info.file || !info.sha512) return false;
      const dl = await fetch(url(v, info.file));
      if (!dl.ok || !dl.body) return false;
      const total = Number(dl.headers.get('content-length')) || info.gzSize || 0;
      const chunks = [];
      let got = 0;
      const reader = dl.body.getReader();
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(Buffer.from(value));
        got += value.length;
        if (total) onProgress(Math.min(99, Math.round((got / total) * 100)));
      }
      const data = zlib.gunzipSync(Buffer.concat(chunks));
      if (crypto.createHash('sha512').update(data).digest('base64') !== info.sha512) return false;
      const tmp = `${asar}.new.part`;
      fs.writeFileSync(tmp, data);
      fs.renameSync(tmp, `${asar}.new`);
      ready = v;
      return true;
    })()
      .catch(() => false)
      .finally(() => {
        if (pending?.promise === promise) pending = null;
      });
    pending = { version: v, promise };
    return promise;
  }

  // Spustí pomocníka; Flux sa potom musí zavrieť (app.quit()).
  function apply(relaunch) {
    if (!ready) return false;
    const script = path.join(os.tmpdir(), 'flux-quick-update.js');
    fs.writeFileSync(script, HELPER);
    // znova spustiť s tým istým profilom a sandboxom (ostatné parametre – napr. otvorený súbor – nie)
    const keep = process.argv.slice(1).filter((a) => a === '--no-sandbox' || a.startsWith('--user-data-dir='));
    const child = spawn(execPath, [script, String(process.pid), resourcesDir, execPath, relaunch ? '1' : '0', JSON.stringify(keep)], {
      detached: true,
      stdio: 'ignore',
      windowsHide: true,
      env: { ...process.env, ELECTRON_RUN_AS_NODE: '1' },
    });
    child.unref();
    ready = null;
    return true;
  }

  return { possible, fetch: fetchVersion, apply, cleanup, ready: () => ready };
}

module.exports = { createQuickUpdate };
