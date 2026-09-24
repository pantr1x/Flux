// Nastavenia → O aplikácii: verzia, aktualizácie (aj automatické) a poznámky k vydaniam.
import { t } from './i18n.js';
import { icon } from './icons.js';
import { markdown } from './aiPanel.js';

const flux = window.flux;

// Vývojárske verzie sa zobrazujú ako 1.4.4.1: interne (semver) sú to 1.4.5-beta.1 – novšie ako 1.4.4, staršie ako 1.4.5.
export function showVer(v) {
  const m = /^(\d+)\.(\d+)\.(\d+)-beta\.(\d+)$/.exec(String(v || ''));
  return m && Number(m[3]) > 0 ? `${m[1]}.${m[2]}.${Number(m[3]) - 1}.${m[4]}` : String(v || '');
}
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);

export function createUpdatesUI({ toast, getSetting, saveSettings }) {
  let box = null;
  let state = null;

  function statusText(s) {
    switch (s.status) {
      case 'checking':
        return `<span class="spin"></span>${t('Checking for updates…')}`;
      case 'latest':
        return `${icon('check', 13)}${t('You have the newest version.')}`;
      case 'available':
        return t('Version {v} is available.', { v: esc(showVer(s.latest)) });
      case 'downloading':
        return `<span class="spin"></span>${t('Downloading version {v}… {p} %', { v: esc(showVer(s.latest)), p: s.progress })}`;
      case 'installing':
        return `<span class="spin"></span>${t('Installing Flux {v}…', { v: esc(showVer(s.latest)) })}`;
      case 'ready':
        return `${icon('check', 13)}${t('Version {v} is ready – it installs when you restart Flux.', { v: esc(showVer(s.latest)) })}`;
      case 'error':
        return t('Could not check for updates: {msg}', { msg: esc(s.error) });
      default:
        return t('Not checked yet.');
    }
  }

  function actions(s) {
    if (s.status === 'ready') return `<button class="s-btn primary" data-up-install>${icon('refresh', 13)}${t('Restart and update')}</button>`;
    if (s.status === 'available' && s.canUpdate) return `<button class="s-btn primary" data-up-download>${icon('download', 13)}${t('Download')}</button>`;
    if (s.status === 'available') return `<button class="s-btn primary" data-up-open>${icon('external', 13)}${t('Download')}</button>`;
    return `<button class="s-btn" data-up-check ${s.status === 'checking' || s.status === 'downloading' ? 'disabled' : ''}>${icon('refresh', 13)}${t('Check for updates')}</button>`;
  }

  function drawStatus() {
    if (!box || !state) return;
    const el = box.querySelector('#up-status');
    const bar = state.status === 'downloading' || state.status === 'installing' ? `<span class="up-bar${state.status === 'installing' ? ' busy' : ''}"><i style="width:${Number(state.progress) || 0}%"></i></span>` : '';
    if (el) el.innerHTML = `<span><b>${t('Updates')}</b><small class="up-line">${statusText(state)}</small>${bar}</span>${actions(state)}`;
    drawOverlay();
  }

  async function drawNotes(force = false) {
    const el = box?.querySelector('#up-notes');
    if (!el) return;
    try {
      const list = await flux.updateNotes(force);
      el.innerHTML = list.length
        ? list
            .map(
              (r, i) => `<details class="up-rel"${i === 0 || r.version === state?.version ? ' open' : ''}>
              <summary><b>${esc(r.name || `v${showVer(r.version)}`)}</b>${r.version === state?.version ? `<span class="up-badge">${t('installed')}</span>` : ''}${i === 0 ? `<span class="up-badge new">${t('newest')}</span>` : ''}${r.prerelease ? `<span class="up-badge dev">${t('dev build')}</span>` : ''}<small>${r.date ? new Date(r.date).toLocaleDateString() : ''}</small></summary>
              <div class="ai-body">${markdown(r.body || t('No notes for this version.'), { icons: true })}</div></details>`,
            )
            .join('')
        : `<div class="s-loading">${t('No releases yet.')}</div>`;
    } catch (err) {
      el.innerHTML = `<div class="s-loading">${esc(String(err.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, ''))}</div>`;
    }
  }

  // Vývojárske aktualizácie: riadok vidí len vývojár (prihlásený GitHub účet pantr1x – kontroluje updater.js).
  const devRow = () =>
    state?.devAllowed
      ? `<label class="s-row" id="up-dev-row"><span><b>${icon('flask', 13)}${t('Developer updates')}</b><small>${t('test builds that are not public releases – only you get them')}</small></span><input type="checkbox" class="switch" id="up-dev"${state.dev ? ' checked' : ''}></label>`
      : '';

  // compact: vložené do Všeobecných – poznámky k vydaniam sú zbalené a načítajú sa až po otvorení.
  async function render(el, { compact = false } = {}) {
    box = el;
    state = await flux.updateState();
    const notes = `<div id="up-notes"><div class="s-loading"><span class="spin"></span> ${t('Loading…')}</div></div>`;
    box.innerHTML = `
      <div class="up-hero"><div class="brand-mark big">${icon('code', 26)}</div><div><h2>Flux</h2><small>${t('Version {v}', { v: esc(showVer(state.version)) })}</small></div></div>
      <div class="s-group">
        <div class="s-row" id="up-status"></div>
        <label class="s-row"><span><b>${t('Update automatically')}</b><small>${t('downloads new versions in the background and installs them when you close Flux')}</small></span><input type="checkbox" class="switch" id="up-auto"${getSetting('autoUpdate') !== false ? ' checked' : ''}></label>
        ${devRow()}
        ${compact ? `<div class="s-row"><span><b>${t('Like Flux?')}</b><small>${t('A star on GitHub helps other people find it.')}</small></span><button class="s-btn" data-up-star>${icon('star', 13)}${t('Star on GitHub')}</button></div>` : ''}
        ${compact ? `<div class="s-row"><span><b>${t('All versions')}</b><small>${t('Download any version of Flux from GitHub Releases.')}</small></span><button class="s-btn" data-up-releases>${icon('external', 13)}${t('Open')}</button></div>` : ''}
      </div>
      ${compact ? `<details class="up-notes-fold"><summary>${icon('chevron', 12)}${t('Release notes')}</summary>${notes}</details>` : `<h3>${t('Release notes')}</h3>${notes}`}`;
    drawStatus();
    const fold = box.querySelector('.up-notes-fold');
    if (fold) fold.addEventListener('toggle', () => fold.open && !fold.dataset.loaded && ((fold.dataset.loaded = '1'), drawNotes()));
    else drawNotes();
    if (state.status === 'idle') flux.updateCheck();
    box.onchange = async (e) => {
      if (e.target.id === 'up-dev') {
        await saveSettings({ devUpdates: e.target.checked });
        state = await flux.updateDev(e.target.checked);
        box.querySelector('#up-notes') && drawNotes(true);
        return;
      }
      if (e.target.id === 'up-auto') {
        await saveSettings({ autoUpdate: e.target.checked });
        if (e.target.checked) flux.updateCheck();
      }
    };
    box.onclick = async (e) => {
      if (e.target.closest('[data-up-check]')) return flux.updateCheck();
      if (e.target.closest('[data-up-download]')) return flux.updateDownload().catch((err) => toast(String(err.message || err), 'error'));
      if (e.target.closest('[data-up-install]')) return startInstall();
      if (e.target.closest('[data-up-star]')) return flux.openExternal('https://github.com/pantr1x/Flux');
      if (e.target.closest('[data-up-releases]')) return flux.openExternal('https://pantr1x.github.io/Flux/#releases');
      if (e.target.closest('[data-up-open]')) return flux.openExternal('https://github.com/pantr1x/Flux/releases/latest');
    };
  }

  // „Reštartovať a aktualizovať“: okno s priebehom (dosťahovanie novšej verzie, potom inštalácia).
  // Po zatvorení Fluxu priebeh ukazuje malé okno Windows, kým sa Flux znova neotvorí.
  let overlay = null;
  function drawOverlay() {
    if (!overlay || !state) return;
    const pct = state.status === 'downloading' ? Number(state.progress) || 0 : 100;
    overlay.querySelector('.up-ov-title').textContent = t('Updating Flux to {v}…', { v: showVer(state.latest) });
    overlay.querySelector('.up-ov-step').textContent =
      state.status === 'downloading' ? t('Downloading… {p} %', { p: pct }) : state.status === 'error' ? t('Could not check for updates: {msg}', { msg: state.error }) : t('Installing… Flux closes and opens again by itself.');
    overlay.querySelector('.up-bar').classList.toggle('busy', state.status !== 'downloading');
    overlay.querySelector('.up-bar i').style.width = `${pct}%`;
    if (state.status === 'error') setTimeout(() => ((overlay?.remove(), (overlay = null))), 4000);
  }
  function startInstall() {
    overlay?.remove();
    overlay = document.createElement('div');
    overlay.className = 'up-overlay';
    overlay.innerHTML = `<div class="up-ov-card"><div class="brand-mark big">${icon('code', 22)}</div><b class="up-ov-title"></b><small class="up-ov-step"></small><span class="up-bar busy"><i></i></span></div>`;
    document.body.append(overlay);
    drawOverlay();
    return flux.updateInstall();
  }

  // Zmeny stavu z hlavného procesu (aj keď nastavenia nie sú otvorené).
  // Upozornenia pri štarte: s automatickými aktualizáciami sa nová verzia rovno sťahuje,
  // bez nich príde správa s tlačidlom na stiahnutie.
  const announced = {};
  const once = (key, fn) => {
    if (announced[key]) return;
    announced[key] = true;
    fn();
  };
  // Stavový riadok: „Updating 42 %“ s malým pruhom, kým sa nová verzia sťahuje na pozadí.
  function drawStatusbar() {
    const el = document.getElementById('st-update');
    if (!el || !state) return;
    const on = state.status === 'downloading' || state.status === 'ready';
    el.hidden = !on;
    if (!on) return;
    el.title = state.status === 'ready' ? t('Flux {v} is ready – it installs when you restart.', { v: showVer(state.latest) }) : t('Downloading Flux {v} in the background…', { v: showVer(state.latest) });
    el.innerHTML =
      state.status === 'ready'
        ? `${icon('refresh', 12)}<span>${t('Restart to update')}</span>`
        : `${icon('download', 12)}<span>${t('Updating {p} %', { p: Number(state.progress) || 0 })}</span><span class="st-upbar"><i style="width:${Number(state.progress) || 0}%"></i></span>`;
    el.onclick = () => (state.status === 'ready' ? startInstall() : null);
  }

  flux.onUpdateState((s) => {
    state = s;
    drawStatusbar();
    if (box?.isConnected) drawStatus();
    else drawOverlay();
    // počas inštalácie ukazuje priebeh okno – bez ďalších správ
    if (!s.latest || overlay) return;
    if (s.status === 'available')
      once(`a${s.latest}`, () =>
        toast(t('Flux {v} is available.', { v: showVer(s.latest) }), 'info', 20000, {
          label: t('Download'),
          run: () => (s.canUpdate ? flux.updateDownload().catch((err) => toast(String(err.message || err), 'error')) : flux.openExternal('https://github.com/pantr1x/Flux/releases/latest')),
        }),
      );
    if (s.status === 'downloading') once(`d${s.latest}`, () => toast(t('Downloading Flux {v} in the background…', { v: showVer(s.latest) }), 'info', 5000));
    if (s.status === 'ready')
      once(`r${s.latest}`, () =>
        toast(t('Flux {v} is ready – it installs when you restart.', { v: showVer(s.latest) }), 'ok', 20000, { label: t('Restart now'), run: () => startInstall() }),
      );
  });

  // Po aktualizácii: „Čo je nové“.
  async function whatsNew() {
    const st = await flux.updateState();
    const seen = getSetting('lastSeenVersion');
    await saveSettings({ lastSeenVersion: st.version });
    if (!seen || seen === st.version) return;
    let rel = null;
    try {
      // Všetky verzie od poslednej, ktorú si videl (ak si niektoré preskočil).
      const notes = await flux.updateNotes();
      // semver: 1.5.0 > 1.5.0-beta.2 > 1.5.0-beta.1 > 1.4.9
      const newer = (a, b) => {
        const [ma, xa = ''] = String(a).split('-');
        const [mb, xb = ''] = String(b).split('-');
        const pa = ma.split('.').map(Number);
        const pb = mb.split('.').map(Number);
        for (let i = 0; i < 3; i++) if ((pa[i] || 0) !== (pb[i] || 0)) return (pa[i] || 0) > (pb[i] || 0);
        if (!xa || !xb) return !xa && !!xb;
        return xa.localeCompare(xb, 'en', { numeric: true }) > 0;
      };
      const range = notes.filter((r) => newer(r.version, seen) && !newer(r.version, st.version));
      if (range.length > 1) rel = { body: range.map((r) => `## ${showVer(r.version)}\n\n${r.body}`).join('\n\n') };
      else rel = notes.find((r) => r.version === st.version);
    } catch {}
    const el = document.createElement('div');
    el.className = 'up-whatsnew';
    el.innerHTML = `<div class="np-card"><header><h2>${icon('sparkle', 18)}${t('What’s new in Flux {v}', { v: esc(showVer(st.version)) })}</h2><button class="icon-btn" data-close>${icon('x', 16)}</button></header>
      <div class="ai-body up-wn-body">${markdown(rel?.body || t('Flux was updated to version {v}.', { v: showVer(st.version) }), { icons: true })}</div>
      <footer><div class="grow"></div><button class="ob-primary" data-close>${t('Continue')}</button></footer></div>`;
    document.body.append(el);
    const close = () => {
      el.remove();
      window.removeEventListener('keydown', onKey, true);
    };
    const onKey = (e) => {
      if (e.key === 'Escape' || e.key === 'Enter') {
        e.preventDefault();
        e.stopPropagation();
        close();
      }
    };
    window.addEventListener('keydown', onKey, true);
    el.onclick = (e) => {
      if (e.target === el || e.target.closest('[data-close]')) close();
    };
  }

  return { render, whatsNew, startInstall };
}
