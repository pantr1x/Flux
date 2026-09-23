// Obchod s pluginmi (Nastavenia → Plugins): hľadanie, filtre, detail s obrázkami a popisom, hodnotenie, inštalácia.
import { t } from './i18n.js';
import { icon } from './icons.js';
import { markdown } from './aiPanel.js';

const flux = window.flux;
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const errText = (err) => String(err?.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, '');

// 👍 / 👎 → hviezdičky (1 až 5).
function stars(p) {
  const total = (p.likes || 0) + (p.dislikes || 0);
  if (!total) return `<span class="pl-new">${t('New')}</span>`;
  const score = 1 + (4 * p.likes) / total;
  return `<span class="pl-stars" title="${p.likes} 👍 · ${p.dislikes} 👎">★ ${score.toFixed(1)} <small>(${total})</small></span>`;
}

export function createPluginsUI({ host, toast, openProject }) {
  let list = [];
  let filter = 'all';
  let query = '';
  let box = null;

  const verified = (p) => (p.verified || p.publisher === 'Flux' ? `<span class="pl-verified" title="${t('Made by the Flux team')}">${icon('check', 10)}</span>` : '');
  const iconImg = (p, size = 40) =>
    p.iconUrl ? `<img class="pl-icon" src="${esc(p.iconUrl)}" width="${size}" height="${size}" alt="">` : `<span class="pl-icon ph" style="width:${size}px;height:${size}px">${icon('sparkle', size / 2)}</span>`;

  function actionBtn(p) {
    if (p.busy) return `<button class="s-btn" disabled><span class="spin"></span></button>`;
    if (!p.installed) return `<button class="s-btn primary" data-pl-install="${p.id}">${icon('download', 13)}${t('Install')}</button>`;
    if (p.update) return `<button class="s-btn primary" data-pl-install="${p.id}">${icon('download', 13)}${t('Update')}</button>`;
    return `<input type="checkbox" class="switch" data-pl-enable="${p.id}"${p.enabled ? ' checked' : ''} title="${t('On / off')}">`;
  }

  function card(p) {
    return `<div class="pl-card" data-pl-open="${p.id}">
      ${iconImg(p)}
      <div class="pl-txt"><b>${esc(p.name)}</b><small>${esc(p.publisher)}${verified(p)} · v${esc(p.version)}</small><p>${esc(p.description)}</p></div>
      <div class="pl-side">${stars(p)}${actionBtn(p)}</div>
    </div>`;
  }

  function shown() {
    const q = query.toLowerCase();
    return list.filter((p) => {
      if (filter === 'installed' && !p.installed) return false;
      if (filter === 'flux' && !(p.verified || p.publisher === 'Flux')) return false;
      if (filter === 'community' && (p.verified || p.publisher === 'Flux')) return false;
      return !q || `${p.name} ${p.description} ${p.publisher} ${(p.tags || []).join(' ')}`.toLowerCase().includes(q);
    });
  }

  function renderList() {
    const items = shown();
    box.querySelector('#pl-list').innerHTML = items.length ? items.map(card).join('') : `<div class="s-loading">${t('No plugins found.')}</div>`;
  }

  async function renderHome(force = false) {
    box.innerHTML = `
      <p class="s-lead">${t('Plugins add new features to Flux. Plugins from the Flux team are checked; community plugins are made by other people – install only what you trust.')}</p>
      <div class="pl-bar">
        <input class="s-search" id="pl-q" placeholder="${t('Search plugins…')}" value="${esc(query)}" spellcheck="false">
        <div class="pl-filters">${[['all', t('All')], ['installed', t('Installed')], ['flux', 'Flux'], ['community', t('Community')]]
          .map(([id, l]) => `<button class="np-chip${filter === id ? ' on' : ''}" data-pl-filter="${id}">${l}</button>`)
          .join('')}</div>
      </div>
      <div class="pl-list" id="pl-list"><div class="s-loading"><span class="spin"></span> ${t('Loading…')}</div></div>
      <h3>${t('For developers')}</h3>
      <div class="s-group">
        <div class="s-row"><span><b>${t('Make your own plugin')}</b><small>${t('Creates a ready-to-go plugin project with an example you can change.')}</small></span><button class="s-btn" data-pl-scaffold>${icon('plus', 13)}${t('Create a plugin')}</button></div>
        <div class="s-row"><span><b>${t('Test a plugin from a folder')}</b><small>${t('Loads plugin.json from a folder – press Reload after every change.')}</small></span><button class="s-btn" data-pl-folder>${icon('folderOpen', 13)}${t('Load from folder…')}</button></div>
        <div class="s-row"><span><b>${t('Publish it for everyone')}</b><small>${t('Fork Flux on GitHub, add your plugin folder and open a pull request.')}</small></span><button class="s-btn" data-pl-docs>${icon('external', 13)}${t('How to publish')}</button></div>
      </div>
      <div id="pl-local"></div>`;
    box.querySelector('#pl-q').oninput = (e) => {
      query = e.target.value;
      renderList();
    };
    try {
      list = await flux.pluginRegistry(force);
    } catch (err) {
      list = [];
      box.querySelector('#pl-list').innerHTML = `<div class="s-loading">${t('Could not load plugins: {msg}', { msg: esc(errText(err)) })}</div>`;
    }
    // Lokálne (z priečinka) pluginy, ktoré nie sú v katalógu.
    const inst = await flux.pluginsInstalled();
    // Lokálne pluginy a tie, ktoré už v obchode nie sú – dajú sa odstrániť.
    const local = inst.filter((m) => !list.some((p) => p.id === m.id));
    box.querySelector('#pl-local').innerHTML = local.length
      ? `<h3>${t('Other installed plugins')}</h3><div class="s-group">${local
          .map(
            (m) =>
              `<div class="s-row"><span><b>${esc(m.name)}</b><small>${esc(m.id)} · ${esc(m.source || t('no longer in the store'))}</small></span><span class="s-inline">${m.local ? `<button class="s-btn" data-pl-reload="${m.id}">${icon('refresh', 13)}${t('Reload')}</button>` : ''}<button class="icon-btn" data-pl-remove="${m.id}" title="${t('Remove')}">${icon('trash', 14)}</button></span></div>`,
          )
          .join('')}</div>`
      : '';
    if (list.length) renderList();
  }

  async function renderDetail(id) {
    const p = list.find((x) => x.id === id) || { id };
    box.innerHTML = `<button class="pl-back" data-pl-back>${icon('chevron', 13)}${t('All plugins')}</button><div class="s-loading"><span class="spin"></span></div>`;
    let d;
    try {
      d = await flux.pluginDetails(id);
    } catch (err) {
      box.querySelector('.s-loading').textContent = errText(err);
      return;
    }
    const full = { ...p, ...d, installed: p.installed, enabled: p.enabled, update: p.update };
    box.innerHTML = `
      <button class="pl-back" data-pl-back>${icon('chevron', 13)}${t('All plugins')}</button>
      <div class="pl-head">
        ${iconImg(full, 64)}
        <div class="pl-txt"><h2>${esc(full.name)}</h2><small>${esc(full.publisher)}${verified(full)} · v${esc(full.version)}${full.license ? ` · ${esc(full.license)}` : ''}</small><p>${esc(full.description)}</p>
          <div class="pl-tags">${(full.tags || []).map((x) => `<span>${esc(x)}</span>`).join('')}</div></div>
        <div class="pl-side">${actionBtn(full)}${full.installed ? `<button class="s-btn" data-pl-uninstall="${full.id}">${t('Uninstall')}</button>` : ''}</div>
      </div>
      <div class="pl-rate">${stars(full)}${
        full.issue
          ? `<button class="s-btn" data-pl-like="${full.issue}">👍 ${full.likes || 0}</button><button class="s-btn" data-pl-dislike="${full.issue}">👎 ${full.dislikes || 0}</button><button class="s-btn" data-pl-discuss="${full.issue}">${icon('external', 12)}${t('Reviews on GitHub')}</button>`
          : ''
      }</div>
      ${full.screenshotUrls?.length ? `<div class="pl-shots">${full.screenshotUrls.map((u) => `<img src="${esc(u)}" alt="" data-pl-zoom>`).join('')}</div>` : ''}
      <div class="pl-readme ai-body">${markdown(full.readme || full.description || '')}</div>`;
  }

  async function install(id) {
    const p = list.find((x) => x.id === id);
    if (p) p.busy = true;
    rerender(id);
    try {
      const m = await flux.pluginInstall(id);
      await host.refresh(id);
      toast(t('{name} is installed and running.', { name: m.name }), 'ok');
    } catch (err) {
      toast(errText(err), 'error', 7000);
    }
    list = await flux.pluginRegistry();
    rerender(id);
  }

  let detailId = null;
  function rerender(id) {
    if (detailId === id) renderDetail(id);
    else if (box.querySelector('#pl-list')) renderList();
  }

  async function onClick(e) {
    const b = e.target.closest('button, input, [data-pl-open], img[data-pl-zoom]');
    if (!b) return;
    if (b.matches('img[data-pl-zoom]')) {
      b.classList.toggle('zoom');
      return;
    }
    if (b.dataset.plFilter) {
      filter = b.dataset.plFilter;
      box.querySelectorAll('[data-pl-filter]').forEach((x) => x.classList.toggle('on', x === b));
      return renderList();
    }
    if (b.dataset.plInstall) {
      e.stopPropagation();
      return install(b.dataset.plInstall);
    }
    if (b.dataset.plEnable) {
      e.stopPropagation();
      await flux.pluginEnable(b.dataset.plEnable, b.checked);
      await host.refresh(b.dataset.plEnable);
      list = await flux.pluginRegistry();
      return;
    }
    if (b.dataset.plUninstall) {
      await host.unload(b.dataset.plUninstall);
      await flux.pluginUninstall(b.dataset.plUninstall);
      list = await flux.pluginRegistry();
      toast(t('Plugin removed.'), 'ok');
      return renderDetail(b.dataset.plUninstall);
    }
    if (b.dataset.plLike || b.dataset.plDislike) {
      try {
        await flux.pluginRate(b.dataset.plLike || b.dataset.plDislike, !!b.dataset.plLike);
        toast(t('Thanks for rating!'), 'ok');
        list = await flux.pluginRegistry(true);
        return renderDetail(detailId);
      } catch (err) {
        return toast(errText(err), 'error', 7000);
      }
    }
    if (b.dataset.plDiscuss) return flux.openExternal(`https://github.com/pantr1x/Flux/issues/${b.dataset.plDiscuss}`);
    if (b.dataset.plBack !== undefined) {
      detailId = null;
      return renderHome();
    }
    if (b.dataset.plScaffold !== undefined) {
      const dir = await flux.pluginScaffold(t('My plugin'));
      toast(t('Plugin project created. Load it with “Load from folder…” to try it.'), 'ok', 7000);
      return openProject(dir);
    }
    if (b.dataset.plFolder !== undefined) {
      try {
        const m = await flux.pluginLoadFolder();
        if (!m) return;
        await host.refresh(m.id);
        toast(t('{name} is loaded.', { name: m.name }), 'ok');
        return renderHome();
      } catch (err) {
        return toast(errText(err), 'error', 8000);
      }
    }
    if (b.dataset.plReload) {
      await flux.pluginReloadLocal(b.dataset.plReload);
      await host.refresh(b.dataset.plReload);
      return toast(t('Reloaded.'), 'ok', 1500);
    }
    if (b.dataset.plRemove) {
      await host.unload(b.dataset.plRemove);
      await flux.pluginUninstall(b.dataset.plRemove);
      return renderHome();
    }
    if (b.dataset.plDocs !== undefined) return flux.openExternal('https://github.com/pantr1x/Flux/blob/HEAD/docs/PLUGINS.md');
    const open = e.target.closest('[data-pl-open]');
    if (open && !e.target.closest('button, input')) {
      detailId = open.dataset.plOpen;
      return renderDetail(detailId);
    }
  }

  return {
    render(el) {
      box = el;
      detailId = null;
      box.onclick = onClick;
      renderHome();
    },
  };
}
