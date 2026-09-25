// Aktualizácie Fluxu: nové verzie sa berú z GitHub Releases (pantr1x/Flux).
// Automaticky: stiahne sa na pozadí a nainštaluje pri zatvorení Fluxu (dá sa vypnúť v Nastaveniach).
const { app, net } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const { t } = require('./i18n');
const { createQuickUpdate } = require('./quickUpdate');

const REPO = 'pantr1x/Flux';
const DEV_LOGIN = 'pantr1x';
let autoUpdater = null;
try {
  ({ autoUpdater } = require('electron-updater'));
} catch {}

function createUpdater({ getSettings, send }) {
  // Linux (AppImage) je zatiaľ len vývojárska verzia: aktualizuje sa iba pri zapnutých vývojárskych aktualizáciách
  // a len keď beží ako AppImage (electron-updater ho vymení za nový).
  const linux = process.platform === 'linux';
  const packaged = app.isPackaged && !!autoUpdater && (!linux || !!process.env.APPIMAGE);
  const useUpdater = () => packaged && (!linux || dev());
  const state = { version: app.getVersion(), status: 'idle', latest: '', progress: 0, error: '' };
  let notesCache = { at: 0, list: null };

  const snap = () => ({ ...state, canUpdate: useUpdater(), devAllowed: devAllowed(), dev: dev() });
  const emit = (patch) => {
    Object.assign(state, patch);
    send('update:state', snap());
  };
  // Vydanie bez linuxovej verzie (napr. stabilné pre Windows) nie je chyba – pre Linux jednoducho nič nové.
  const noLinuxBuild = (err) => linux && (err?.code === 'ERR_UPDATER_CHANNEL_FILE_NOT_FOUND' || /latest-linux\.yml/.test(String(err?.message)));
  const failed = (err) => emit(noLinuxBuild(err) ? { status: 'latest', latest: state.version, error: '' } : { status: 'error', error: String(err?.message || err).split('\n')[0] });
  const auto = () => getSettings().autoUpdate !== false;
  // Vývojárske aktualizácie: predbežné verzie (napr. 1.5.0-beta.1, na GitHube ako „pre-release“).
  // Dostane ich len vývojár – keď je vo Fluxe prihlásený GitHub účet DEV_LOGIN (overený tokenom);
  // vypnúť sa dajú v Nastaveniach. Ostatní ich nikdy neuvidia.
  const devAllowed = () => String(getSettings().github?.user?.login || '').toLowerCase() === DEV_LOGIN;
  const dev = () => devAllowed() && getSettings().devUpdates !== false;

  // Rýchla aktualizácia (len app.asar) – quickUpdate.js. FLUX_QUICK_URL = iný zdroj na testovanie.
  const quick = packaged
    ? createQuickUpdate({ repo: REPO, fetch: (u) => net.fetch(u, { headers: { 'User-Agent': 'Flux' } }), resourcesDir: process.resourcesPath, execPath: process.execPath, baseUrl: process.env.FLUX_QUICK_URL })
    : null;

  // Sťahovanie riadime sami: najprv skúsi rýchlu aktualizáciu, až potom celý inštalátor.
  const downloads = new Map();
  function startDownload(v) {
    if (downloads.has(v)) return downloads.get(v);
    const job = (async () => {
      emit({ status: 'downloading', latest: v, progress: 0, quick: false });
      if (await quick.fetch(v, (p) => emit({ status: 'downloading', latest: v, progress: p }))) {
        emit({ status: 'ready', latest: v, progress: 100, quick: true });
        return;
      }
      await autoUpdater.downloadUpdate();
    })()
      .catch(failed)
      .finally(() => downloads.delete(v));
    downloads.set(v, job);
    return job;
  }

  if (packaged) {
    quick.cleanup();
    autoUpdater.autoDownload = false;
    autoUpdater.autoInstallOnAppQuit = true;
    autoUpdater.allowPrerelease = dev();
    autoUpdater.allowDowngrade = false;
    autoUpdater.on('checking-for-update', () => state.status !== 'ready' && emit({ status: 'checking', error: '' }));
    autoUpdater.on('update-available', (info) => {
      if (state.status === 'ready' && state.latest === info.version) return;
      if (auto()) startDownload(info.version);
      else emit({ status: 'available', latest: info.version, progress: 0 });
    });
    // Rýchla aktualizácia sa vymení, až keď sa Flux naozaj zatvára (zrušené zatvorenie = nič sa nedeje).
    // Po „Reštartovať a aktualizovať“ sa Flux potom sám znova otvorí, pri bežnom zatvorení nie.
    app.on('will-quit', () => {
      if (state.quick && quick.ready()) quick.apply(quitRelaunch);
    });
    // Pri inštalácii pri zatvorení sa vždy použije posledná stiahnutá (najnovšia) verzia.
    autoUpdater.on('update-not-available', (info) => emit({ status: 'latest', latest: info?.version || state.version }));
    autoUpdater.on('download-progress', (p) => emit({ status: 'downloading', progress: Math.round(p.percent || 0) }));
    autoUpdater.on('update-downloaded', (info) => emit({ status: 'ready', latest: info.version, progress: 100, quick: false }));
    autoUpdater.on('error', failed);
  }

  // Poznámky pribalené v aplikácii (CHANGELOG.md) – fungujú aj bez internetu.
  function localNotes() {
    try {
      const log = fs.readFileSync(path.join(__dirname, '../../CHANGELOG.md'), 'utf8');
      return log
        .split(/^## /m)
        .slice(1)
        .map((part) => {
          const [head, ...rest] = part.split('\n');
          const [version, date] = head.split(/\s+[–-]\s+/);
          return { version: version.trim(), name: `Flux ${version.trim()}`, date: date ? new Date(date.trim()).toISOString() : '', body: rest.join('\n').trim(), url: '', local: true };
        });
    } catch {
      return [];
    }
  }

  // Poznámky k vydaniam: z GitHub Releases (novšie verzie) + pribalené v aplikácii.
  async function notes(force = false) {
    if (!force && notesCache.list && Date.now() - notesCache.at < 10 * 60 * 1000) return notesCache.list;
    let remote = [];
    try {
      const res = await net.fetch(`https://api.github.com/repos/${REPO}/releases?per_page=15`, { headers: { Accept: 'application/vnd.github+json', 'User-Agent': 'Flux' } });
      if (res.ok)
        remote = (await res.json())
          .filter((r) => !r.draft)
          .map((r) => ({ version: String(r.tag_name || '').replace(/^v/, ''), name: r.name || r.tag_name, date: r.published_at, body: r.body || '', url: r.html_url, prerelease: r.prerelease }));
    } catch {}
    // predbežné verzie vidí len ten, kto má zapnuté vývojárske aktualizácie (alebo jednu z nich má nainštalovanú)
    const list = remote.filter((r) => !r.prerelease || dev() || r.version === state.version);
    for (const n of localNotes()) if (!list.some((r) => r.version === n.version)) list.push(n);
    list.sort((a, b) => (newer(a.version, b.version) ? -1 : newer(b.version, a.version) ? 1 : 0));
    notesCache = { at: Date.now(), list };
    return list;
  }

  async function check() {
    if (useUpdater()) {
      autoUpdater.allowPrerelease = dev();
      try {
        await autoUpdater.checkForUpdates();
      } catch (err) {
        failed(err);
      }
      return snap();
    }
    // Vývojová verzia (nie nainštalovaná) alebo Linux bez vývojárskych aktualizácií: len zistí, aká verzia je najnovšia.
    emit({ status: 'checking', error: '' });
    try {
      const latest = (await notes(true)).find((r) => dev() || !r.prerelease)?.version || '';
      emit({ status: latest && newer(latest, state.version) ? 'available' : 'latest', latest });
    } catch (err) {
      emit({ status: 'error', error: err.message });
    }
    return snap();
  }

  async function download() {
    if (!useUpdater() || !state.latest) return false;
    await startDownload(state.latest);
    return true;
  }

  // „Reštartovať teraz“: inštalácia s pruhom priebehu a Flux sa sám znova otvorí.
  // Najprv sa ešte pozrie, či medzitým nevyšla novšia verzia – inak by si musel aktualizovať dvakrát.
  // Značka v TEMP je poistka – inštalátor podľa nej Flux spustí, aj keby --force-run nezabral.
  let installing = false;
  let quitRelaunch = false;
  // Zatvorenie sa dá zrušiť (neuložené súbory → Zrušiť): ak Flux o chvíľu stále beží, vrátiť stav späť,
  // nech sa okno s priebehom zavrie a aktualizácia sa dá spustiť znova.
  function watchCancel() {
    setTimeout(() => {
      installing = false;
      quitRelaunch = false;
      emit({ status: 'ready', progress: 100, canceled: Date.now() });
    }, 10000);
  }
  async function install() {
    if (!(useUpdater() && state.status === 'ready') || installing) return;
    installing = true;
    try {
      const ready = state.latest;
      autoUpdater.allowPrerelease = dev();
      // najviac 4 s – pomalé pripojenie nesmie zdržať reštart
      const r = await Promise.race([autoUpdater.checkForUpdates().catch(() => null), new Promise((ok) => setTimeout(() => ok(null), 4000))]);
      const newest = r?.updateInfo?.version;
      if (newest && newer(newest, ready)) await startDownload(newest);
    } catch {}
    // Rýchla aktualizácia: pomocník po zatvorení vymení app.asar a Flux hneď znova otvorí.
    if (state.status === 'ready' && state.quick && quick.ready() === state.latest) {
      emit({ status: 'installing', progress: 100, canceled: 0 });
      quitRelaunch = true;
      setTimeout(() => app.quit(), 300);
      watchCancel();
      return;
    }
    if (state.status !== 'ready') {
      installing = false;
      return;
    }
    try {
      fs.writeFileSync(path.join(require('node:os').tmpdir(), 'flux-relaunch-after-update'), String(Date.now()));
    } catch {}
    emit({ status: 'installing', progress: 100, canceled: 0 });
    // Inštalátor beží viditeľne, ale len s pruhom priebehu (uvítanie, výber a koniec preskočí
    // – build/installer.nsh); po inštalácii Flux otvorí značka v TEMP (customInstall).
    setTimeout(() => autoUpdater.quitAndInstall(false, true), 400);
    watchCancel();
  }

  // Pri štarte: skontrolovať (a pri automatických aktualizáciách aj stiahnuť).
  function start() {
    setTimeout(check, 4000);
    setInterval(check, 6 * 60 * 60 * 1000);
  }

  // Prepnutie vývojárskych aktualizácií: zahodí zoznam poznámok a hneď skontroluje znova.
  function setDev() {
    notesCache = { at: 0, list: null };
    return check();
  }

  return { state: snap, check, download, install, notes, start, setDev };
}

// a > b podľa semver: 1.5.0 > 1.5.0-beta.2 > 1.5.0-beta.1 > 1.4.9
function newer(a, b) {
  const [ma, pa = ''] = String(a).split('-');
  const [mb, pb = ''] = String(b).split('-');
  const na = ma.split('.').map(Number);
  const nb = mb.split('.').map(Number);
  for (let i = 0; i < 3; i++) if ((na[i] || 0) !== (nb[i] || 0)) return (na[i] || 0) > (nb[i] || 0);
  if (!pa || !pb) return !pa && !!pb;
  const xa = pa.split('.');
  const xb = pb.split('.');
  for (let i = 0; i < Math.max(xa.length, xb.length); i++) {
    if (xa[i] === undefined) return false;
    if (xb[i] === undefined) return true;
    if (xa[i] === xb[i]) continue;
    const da = /^\d+$/.test(xa[i]);
    const db = /^\d+$/.test(xb[i]);
    if (da && db) return Number(xa[i]) > Number(xb[i]);
    if (da !== db) return db;
    return xa[i] > xb[i];
  }
  return false;
}

module.exports = { createUpdater };
