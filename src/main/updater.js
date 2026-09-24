// Aktualizácie Fluxu: nové verzie sa berú z GitHub Releases (pantr1x/Flux).
// Automaticky: stiahne sa na pozadí a nainštaluje pri zatvorení Fluxu (dá sa vypnúť v Nastaveniach).
const { app, net } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const { t } = require('./i18n');

const REPO = 'pantr1x/Flux';
let autoUpdater = null;
try {
  ({ autoUpdater } = require('electron-updater'));
} catch {}

function createUpdater({ getSettings, send }) {
  const state = { version: app.getVersion(), status: 'idle', latest: '', progress: 0, error: '', canUpdate: app.isPackaged && !!autoUpdater };
  let notesCache = { at: 0, list: null };

  const emit = (patch) => {
    Object.assign(state, patch);
    send('update:state', { ...state });
  };
  const auto = () => getSettings().autoUpdate !== false;

  if (state.canUpdate) {
    autoUpdater.autoDownload = auto();
    autoUpdater.autoInstallOnAppQuit = true;
    autoUpdater.allowPrerelease = false;
    autoUpdater.on('checking-for-update', () => emit({ status: 'checking', error: '' }));
    autoUpdater.on('update-available', (info) => emit({ status: autoUpdater.autoDownload ? 'downloading' : 'available', latest: info.version, progress: 0 }));
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
    const list = [...remote];
    for (const n of localNotes()) if (!list.some((r) => r.version === n.version)) list.push(n);
    list.sort((a, b) => (newer(a.version, b.version) ? -1 : newer(b.version, a.version) ? 1 : 0));
    notesCache = { at: Date.now(), list };
    return list;
  }

  async function check() {
    if (state.canUpdate) {
      autoUpdater.autoDownload = auto();
      try {
        await autoUpdater.checkForUpdates();
      } catch (err) {
        emit({ status: 'error', error: String(err?.message || err).split('\n')[0] });
      }
      return { ...state };
    }
    // Vývojová verzia (nie nainštalovaná): len zistí, aká verzia je najnovšia.
    emit({ status: 'checking', error: '' });
    try {
      const latest = (await notes(true)).find((r) => !r.prerelease)?.version || '';
      emit({ status: latest && newer(latest, state.version) ? 'available' : 'latest', latest });
    } catch (err) {
      emit({ status: 'error', error: err.message });
    }
    return { ...state };
  }

  async function download() {
    if (!state.canUpdate) return false;
    emit({ status: 'downloading', progress: 0 });
    await autoUpdater.downloadUpdate();
    return true;
  }

  function install() {
    if (state.canUpdate && state.status === 'ready') autoUpdater.quitAndInstall(true, true);
  }

  // Pri štarte: skontrolovať (a pri automatických aktualizáciách aj stiahnuť).
  function start() {
    setTimeout(check, 4000);
    setInterval(check, 6 * 60 * 60 * 1000);
  }

  return { state: () => ({ ...state }), check, download, install, notes, start };
}

function newer(a, b) {
  const pa = String(a).split('.').map(Number);
  const pb = String(b).split('.').map(Number);
  for (let i = 0; i < 3; i++) if ((pa[i] || 0) !== (pb[i] || 0)) return (pa[i] || 0) > (pb[i] || 0);
  return false;
}

module.exports = { createUpdater };
