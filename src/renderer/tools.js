// Programovacie jazyky na stiahnutie (Java, C/C++, Go, C#…): stav, veľkosť, inštalácia s priebehom.
// Riadok jazyka (.tc-row) sa dá vložiť kamkoľvek – do úvodu, nového projektu aj nastavení.
import { t } from './i18n.js';
import { icon, fileIcon } from './icons.js';

const flux = window.flux;

export const TOOL_FILE = { git: 'a.git', python: 'a.py', node: 'a.js', java: 'a.java', cpp: 'a.cpp', go: 'a.go', csharp: 'a.cs', rust: 'a.rs', ruby: 'a.rb', php: 'a.php', lua: 'a.lua' };

const mb = (n) => (n >= 1000 ? `${(n / 1000).toFixed(1).replace('.0', '')} GB` : `${n} MB`);

export function createTools({ toast }) {
  let cache = null;
  let pending = null;
  const progress = new Map(); // id → { percent, line }
  const installing = new Set();
  let queue = []; // jazyky čakajúce na inštaláciu (po úvode sa inštalujú jeden po druhom)

  async function status(force = false) {
    if (cache && !force) return cache;
    if (!pending || force) pending = flux.toolchains().then((s) => (cache = s)).finally(() => (pending = null));
    return pending;
  }

  const info = (id) => cache?.list.find((x) => x.id === id);

  function meta(tc) {
    if (installing.has(tc.id)) {
      const p = progress.get(tc.id) || {};
      return `<span class="tc-bar"><i style="width:${p.percent ?? 4}%"></i></span><small class="tc-line">${escapeHtml(p.line || t('Starting…'))}</small>`;
    }
    if (tc.installed && tc.update) return `<small class="tc-upd">${icon('download', 12)}${t('Update available: {from} → {to}', { from: escapeHtml(tc.version || tc.update.current), to: escapeHtml(tc.update.available) })}</small>`;
    if (tc.installed) return `<small class="tc-ok">${icon('check', 12)}${t('Installed')}${tc.version ? ` · ${escapeHtml(tc.version)}` : ''}</small>`;
    if (queue.includes(tc.id)) return `<small>${t('Waiting…')}</small>`;
    return `<small>${t('{download} download · {disk} on disk', { download: mb(tc.download), disk: mb(tc.disk) })}</small>`;
  }

  function action(tc) {
    if (installing.has(tc.id)) return `<button class="s-btn" disabled>${t('Installing…')}</button>`;
    if (tc.installed && tc.update && cache?.canInstall) return `<button class="s-btn tc-install" data-tc-update="${tc.id}">${icon('download', 13)}${t('Update')}</button>`;
    if (tc.installed) return '';
    if (queue.includes(tc.id)) return '';
    if (!cache?.canInstall) return `<button class="s-btn" data-tc-web="${tc.id}">${icon('external', 13)}${t('Download')}</button>`;
    return `<button class="s-btn tc-install" data-tc-install="${tc.id}">${icon('download', 13)}${t('Install')}</button>`;
  }

  function row(tc) {
    const ic = fileIcon(TOOL_FILE[tc.id] || 'a.txt').replace(/width="16" height="16"/, 'width="22" height="22"');
    return `<div class="tc-row${tc.installed ? ' ok' : ''}" data-tc="${tc.id}"><span class="tc-ic">${ic}</span><span class="tc-text"><b>${escapeHtml(tc.name)}${tc.detail ? ` <em>${escapeHtml(tc.detail)}</em>` : ''}</b><span class="tc-meta">${meta(tc)}</span></span><span class="tc-act">${action(tc)}</span></div>`;
  }

  // Prekreslí všetky riadky daného jazyka, nech sú kdekoľvek v okne.
  function refresh(id) {
    const tc = info(id);
    if (!tc) return;
    for (const el of document.querySelectorAll(`.tc-row[data-tc="${id}"]`)) {
      el.classList.toggle('ok', !!tc.installed);
      el.querySelector('.tc-meta').innerHTML = meta(tc);
      el.querySelector('.tc-act').innerHTML = action(tc);
    }
    for (const cb of listeners) cb(id, tc);
  }

  const listeners = new Set();
  const onChange = (cb) => (listeners.add(cb), () => listeners.delete(cb));

  flux.onToolchainProgress((p) => {
    progress.set(p.id, p);
    renderStatusItem();
    for (const el of document.querySelectorAll(`.tc-row[data-tc="${p.id}"]`)) {
      const bar = el.querySelector('.tc-bar i');
      const line = el.querySelector('.tc-line');
      if (bar && p.percent != null) bar.style.width = `${Math.max(4, p.percent)}%`;
      if (line && p.line) line.textContent = p.line;
    }
  });

  async function install(id) {
    await status();
    if (installing.has(id)) return false;
    installing.add(id);
    progress.delete(id);
    refresh(id);
    renderStatusItem();
    try {
      const res = await flux.installToolchain(id);
      Object.assign(info(id), res, { installed: true });
      toast(t('{name} is ready. Press Run!', { name: res.name }), 'ok');
      return true;
    } catch (err) {
      toast(String(err?.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, ''), 'error');
      return false;
    } finally {
      installing.delete(id);
      refresh(id);
    }
  }

  async function update(id) {
    if (installing.has(id)) return false;
    installing.add(id);
    progress.delete(id);
    refresh(id);
    try {
      const res = await flux.upgradeToolchain(id);
      Object.assign(info(id), res, { installed: true, update: null });
      toast(t('{name} was updated to {version}.', { name: res.name, version: res.version }), 'ok');
      return true;
    } catch (err) {
      toast(String(err?.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, ''), 'error');
      return false;
    } finally {
      installing.delete(id);
      refresh(id);
    }
  }

  // Nové verzie (winget upgrade) – ukážu sa pri jazyku ako „Update“.
  async function checkUpdates() {
    await status();
    let found = {};
    try {
      found = (await flux.toolchainUpdates()) || {};
    } catch {}
    for (const tc of cache.list) {
      tc.update = found[tc.id] || null;
      refresh(tc.id);
    }
    return found;
  }

  // Inštalácia viacerých jazykov na pozadí (po úvode) – dá sa pri tom normálne pracovať.
  async function installAll(ids) {
    await status();
    const add = ids.filter((id) => !queue.includes(id) && !installing.has(id) && !info(id)?.installed);
    const wasEmpty = !queue.length;
    queue.push(...add);
    add.forEach(refresh);
    renderStatusItem();
    if (!wasEmpty) return;
    while (queue.length) {
      const id = queue[0];
      await install(id);
      queue.shift();
      renderStatusItem();
    }
  }

  // Malá položka v stavovom riadku počas inštalácie.
  function renderStatusItem() {
    const el = document.getElementById('st-install');
    if (!el) return;
    const id = [...installing][0] || queue[0];
    el.hidden = !id;
    if (!id) return;
    const p = progress.get(id);
    const left = queue.length > 1 ? ` (+${queue.length - 1})` : '';
    el.innerHTML = `<span class="spin"></span>${t('Installing {name}…', { name: escapeHtml(info(id)?.name || id) })}${p?.percent != null ? ` ${p.percent}%` : ''}${left}`;
  }

  flux.onToolchainUpdated?.((res) => {
    if (cache) Object.assign(info(res.id) || {}, res, { installed: true, update: null });
    refresh(res.id);
    toast(t('{name} was updated to {version}.', { name: res.name, version: res.version }), 'ok');
  });

  // Kliky v ľubovoľnom kontajneri s riadkami.
  function bind(root) {
    root.addEventListener('click', (e) => {
      const b = e.target.closest('[data-tc-install],[data-tc-web],[data-tc-update]');
      if (!b) return;
      e.stopPropagation();
      if (b.dataset.tcUpdate) update(b.dataset.tcUpdate);
      else if (b.dataset.tcInstall) install(b.dataset.tcInstall);
      else flux.openExternal(info(b.dataset.tcWeb)?.url);
    });
  }

  // Okno „X nie je nainštalované“ pri spustení súboru. Vráti true, keď sa jazyk nainštaloval.
  function ask(tc, canInstall) {
    if (cache) Object.assign(info(tc.id) || {}, tc);
    else cache = { list: [tc], canInstall };
    const el = document.getElementById('tcask');
    el.innerHTML = `<div class="np-card tc-card" role="dialog">
      <header><h2>${t('{name} is not installed', { name: escapeHtml(tc.name) })}</h2><button class="icon-btn" data-close title="${t('Close (Esc)')}">${icon('x', 16)}</button></header>
      <p class="tc-why">${t('Flux keeps its installer small, so languages are downloaded only when you need them.')}</p>
      ${row(info(tc.id) || tc)}
      <footer><button class="ob-ghost" data-close>${t('Not now')}</button></footer>
    </div>`;
    el.hidden = false;
    bind(el);
    return new Promise((resolve) => {
      const done = (ok) => {
        off();
        el.hidden = true;
        resolve(ok);
      };
      const off = onChange((id, x) => {
        if (id === tc.id && x.installed) setTimeout(() => done(true), 600);
      });
      el.onclick = (e) => {
        if (e.target === el || e.target.closest('[data-close]')) done(false);
      };
      el.onkeydown = (e) => e.key === 'Escape' && done(false);
    });
  }

  return { status, info, row, bind, install, installAll, update, checkUpdates, ask, onChange, mb, canInstall: () => !!cache?.canInstall, busy: () => installing.size > 0 || queue.length > 0 };
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
}
