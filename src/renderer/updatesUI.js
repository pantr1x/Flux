// Nastavenia → O aplikácii: verzia, aktualizácie (aj automatické) a poznámky k vydaniam.
import { t } from './i18n.js';
import { icon } from './icons.js';
import { markdown } from './aiPanel.js';

const flux = window.flux;
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
        return t('Version {v} is available.', { v: esc(s.latest) });
      case 'downloading':
        return `<span class="spin"></span>${t('Downloading version {v}… {p} %', { v: esc(s.latest), p: s.progress })}`;
      case 'ready':
        return `${icon('check', 13)}${t('Version {v} is ready – it installs when you restart Flux.', { v: esc(s.latest) })}`;
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
    if (el) el.innerHTML = `<span><b>${t('Updates')}</b><small class="up-line">${statusText(state)}</small></span>${actions(state)}`;
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
              <summary><b>${esc(r.name || `v${r.version}`)}</b>${r.version === state?.version ? `<span class="up-badge">${t('installed')}</span>` : ''}${i === 0 ? `<span class="up-badge new">${t('newest')}</span>` : ''}<small>${r.date ? new Date(r.date).toLocaleDateString() : ''}</small></summary>
              <div class="ai-body">${markdown(r.body || t('No notes for this version.'), { icons: true })}</div></details>`,
            )
            .join('')
        : `<div class="s-loading">${t('No releases yet.')}</div>`;
    } catch (err) {
      el.innerHTML = `<div class="s-loading">${esc(String(err.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, ''))}</div>`;
    }
  }

  async function render(el) {
    box = el;
    state = await flux.updateState();
    box.innerHTML = `
      <div class="up-hero"><div class="brand-mark big">${icon('code', 26)}</div><div><h2>Flux</h2><small>${t('Version {v}', { v: esc(state.version) })}</small></div></div>
      <div class="s-group">
        <div class="s-row" id="up-status"></div>
        <label class="s-row"><span><b>${t('Update automatically')}</b><small>${t('downloads new versions in the background and installs them when you close Flux')}</small></span><input type="checkbox" class="switch" id="up-auto"${getSetting('autoUpdate') !== false ? ' checked' : ''}></label>
      </div>
      <h3>${t('Release notes')}</h3>
      <div id="up-notes"><div class="s-loading"><span class="spin"></span> ${t('Loading…')}</div></div>`;
    drawStatus();
    drawNotes();
    if (state.status === 'idle') flux.updateCheck();
    box.onchange = async (e) => {
      if (e.target.id === 'up-auto') {
        await saveSettings({ autoUpdate: e.target.checked });
        if (e.target.checked) flux.updateCheck();
      }
    };
    box.onclick = async (e) => {
      if (e.target.closest('[data-up-check]')) return flux.updateCheck();
      if (e.target.closest('[data-up-download]')) return flux.updateDownload().catch((err) => toast(String(err.message || err), 'error'));
      if (e.target.closest('[data-up-install]')) return flux.updateInstall();
      if (e.target.closest('[data-up-open]')) return flux.openExternal('https://github.com/pantr1x/Flux/releases/latest');
    };
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
  flux.onUpdateState((s) => {
    state = s;
    if (box?.isConnected) drawStatus();
    if (!s.latest) return;
    if (s.status === 'available')
      once(`a${s.latest}`, () =>
        toast(t('Flux {v} is available.', { v: s.latest }), 'info', 20000, {
          label: t('Download'),
          run: () => (s.canUpdate ? flux.updateDownload().catch((err) => toast(String(err.message || err), 'error')) : flux.openExternal('https://github.com/pantr1x/Flux/releases/latest')),
        }),
      );
    if (s.status === 'downloading') once(`d${s.latest}`, () => toast(t('Downloading Flux {v} in the background…', { v: s.latest }), 'info', 5000));
    if (s.status === 'ready')
      once(`r${s.latest}`, () =>
        toast(t('Flux {v} is ready – it installs when you restart.', { v: s.latest }), 'ok', 20000, { label: t('Restart now'), run: () => flux.updateInstall() }),
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
      rel = (await flux.updateNotes()).find((r) => r.version === st.version);
    } catch {}
    const el = document.createElement('div');
    el.className = 'up-whatsnew';
    el.innerHTML = `<div class="np-card"><header><h2>${icon('sparkle', 18)}${t('What’s new in Flux {v}', { v: esc(st.version) })}</h2><button class="icon-btn" data-close>${icon('x', 16)}</button></header>
      <div class="ai-body up-wn-body">${markdown(rel?.body || t('Flux was updated to version {v}.', { v: st.version }), { icons: true })}</div>
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

  return { render, whatsNew };
}
