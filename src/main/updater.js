// Aktualizácie Fluxu: nové verzie sa berú z GitHub Releases (pantr1x/Flux).
// Automaticky: stiahne sa na pozadí a nainštaluje pri zatvorení Fluxu (dá sa vypnúť v Nastaveniach).
const { app, net } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const { t } = require('./i18n');

const REPO = 'pantr1x/Flux';
const DEV_LOGIN = 'pantr1x';
let autoUpdater = null;
try {
  ({ autoUpdater } = require('electron-updater'));
} catch {}

function createUpdater({ getSettings, send }) {
  const state = { version: app.getVersion(), status: 'idle', latest: '', progress: 0, error: '', canUpdate: app.isPackaged && !!autoUpdater };
  let notesCache = { at: 0, list: null };

  const emit = (patch) => {
    Object.assign(state, patch);
    send('update:state', { ...state, devAllowed: devAllowed(), dev: dev() });
  };
  const auto = () => getSettings().autoUpdate !== false;
  // Vývojárske aktualizácie: predbežné verzie (napr. 1.5.0-beta.1, na GitHube ako „pre-release“).
  // Dostane ich len vývojár – keď je vo Fluxe prihlásený GitHub účet DEV_LOGIN (overený tokenom);
  // vypnúť sa dajú v Nastaveniach. Ostatní ich nikdy neuvidia.
  const devAllowed = () => String(getSettings().github?.user?.login || '').toLowerCase() === DEV_LOGIN;
  const dev = () => devAllowed() && getSettings().devUpdates !== false;

  if (state.canUpdate) {
    autoUpdater.autoDownload = auto();
    autoUpdater.autoInstallOnAppQuit = true;
    autoUpdater.allowPrerelease = dev();
    autoUpdater.allowDowngrade = false;
    autoUpdater.on('checking-for-update', () => emit({ status: 'checking', error: '' }));
    autoUpdater.on('update-available', (info) => emit({ status: autoUpdater.autoDownload ? 'downloading' : 'available', latest: info.version, progress: 0 }));
    // Pri inštalácii pri zatvorení sa vždy použije posledná stiahnutá (najnovšia) verzia.
    autoUpdater.on('update-not-available', (info) => emit({ status: 'latest', latest: info?.version || state.version }));
    autoUpdater.on('download-progress', (p) => emit({ status: 'downloading', progress: Math.round(p.percent || 0) }));
    autoUpdater.on('update-downloaded', (info) => emit({ status: 'ready', latest: info.version, progress: 100 }));
    autoUpdater.on('error', (err) => emit({ status: 'error', error: String(err?.message || err).split('\n')[0] }));
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
    if (state.canUpdate) {
      autoUpdater.autoDownload = auto();
      autoUpdater.allowPrerelease = dev();
      try {
        await autoUpdater.checkForUpdates();
      } catch (err) {
        emit({ status: 'error', error: String(err?.message || err).split('\n')[0] });
      }
      return { ...state, devAllowed: devAllowed(), dev: dev() };
    }
    // Vývojová verzia (nie nainštalovaná): len zistí, aká verzia je najnovšia.
    emit({ status: 'checking', error: '' });
    try {
      const latest = (await notes(true)).find((r) => dev() || !r.prerelease)?.version || '';
      emit({ status: latest && newer(latest, state.version) ? 'available' : 'latest', latest });
    } catch (err) {
      emit({ status: 'error', error: err.message });
    }
    return { ...state, devAllowed: devAllowed(), dev: dev() };
  }

  async function download() {
    if (!state.canUpdate) return false;
    emit({ status: 'downloading', progress: 0 });
    await autoUpdater.downloadUpdate();
    return true;
  }

  // „Reštartovať teraz“: inštalácia s pruhom priebehu a Flux sa sám znova otvorí.
  // Najprv sa ešte pozrie, či medzitým nevyšla novšia verzia – inak by si musel aktualizovať dvakrát.
  // Značka v TEMP je poistka – inštalátor podľa nej Flux spustí, aj keby --force-run nezabral.
  let installing = false;
  async function install() {
    if (!(state.canUpdate && state.status === 'ready') || installing) return;
    installing = true;
    try {
      const ready = state.latest;
      autoUpdater.autoDownload = true;
      autoUpdater.allowPrerelease = dev();
      // najviac 4 s – pomalé pripojenie nesmie zdržať reštart
      const r = await Promise.race([autoUpdater.checkForUpdates().catch(() => null), new Promise((ok) => setTimeout(() => ok(null), 4000))]);
      const newest = r?.updateInfo?.version;
      if (newest && newer(newest, ready)) {
        emit({ status: 'downloading', latest: newest, progress: 0 });
        await r.downloadPromise;
      }
    } catch {}
    try {
      fs.writeFileSync(path.join(require('node:os').tmpdir(), 'flux-relaunch-after-update'), String(Date.now()));
    } catch {}
    emit({ status: 'installing', progress: 100 });
    // Inštalátor beží viditeľne, ale len s pruhom priebehu (uvítanie, výber a koniec preskočí
    // – build/installer.nsh); po inštalácii Flux otvorí značka v TEMP (customInstall).
    setTimeout(() => autoUpdater.quitAndInstall(false, true), 400);
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

  return { state: () => ({ ...state, devAllowed: devAllowed(), dev: dev() }), check, download, install, notes, start, setDev };
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
