// Obchod s pluginmi (Nastavenia → Plugins): hľadanie, filtre, detail s obrázkami a popisom, hodnotenie, inštalácia.
import { t } from './i18n.js';
import { icon } from './icons.js';
import { markdown } from './aiPanel.js';

const flux = window.flux;
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const errText = (err) => String(err?.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, '');

// Priemer hviezdičiek z recenzií (plugins.js → summary).
function stars(p) {
  if (!p.ratingCount) return `<span class="pl-new">${t('New')}</span>`;
  return `<span class="pl-stars" title="${t('{n} ratings', { n: p.ratingCount })}">★ ${p.rating.toFixed(1)} <small>(${p.ratingCount})</small></span>`;
}
// súhrn hore v detaile: „4.5 ★★★★½ 12 ratings · 5 comments“
function rateSum(p) {
  return p.ratingCount
    ? `<b class="rv-big">${p.rating.toFixed(1)}</b>${starRow(p.rating, 16)}<small>${t('{n} ratings', { n: p.ratingCount })}${p.commentCount ? ` · ${t('{n} comments', { n: p.commentCount })}` : ''}</small>`
    : `<small>${t('No ratings yet')}</small>`;
}
// rad hviezdičiek, aj s polovičnými (4.5 → ★★★★½)
function starRow(value, size = 14) {
  let html = '';
  for (let i = 1; i <= 5; i++) {
    const fill = Math.max(0, Math.min(1, value - i + 1));
    html += `<span class="rv-star" style="--f:${Math.round(fill * 100)}%;font-size:${size}px">★</span>`;
  }
  return `<span class="rv-row" aria-label="${value.toFixed(1)} / 5">${html}</span>`;
}
function ago(date) {
  const s = (Date.now() - new Date(date)) / 1000;
  if (s < 60) return t('just now');
  if (s < 3600) return t('{n} min ago', { n: Math.floor(s / 60) });
  if (s < 86400) return t('{n} h ago', { n: Math.floor(s / 3600) });
  if (s < 86400 * 30) return t('{n} days ago', { n: Math.floor(s / 86400) });
  return new Date(date).toLocaleDateString();
}

// Screenshoty na celú obrazovku: šípky/koliesko prepínajú, klik priblíži (a ťahaním posúvaš), Esc zavrie.
function openShots(urls, start = 0) {
  let i = start;
  let zoom = false;
  const el = document.createElement('div');
  el.className = 'pl-lightbox';
  el.innerHTML = `<button class="lb-close" title="${t('Close (Esc)')}">${icon('x', 18)}</button>
    ${urls.length > 1 ? `<button class="lb-nav lb-prev" title="←">${icon('arrowLeft', 20)}</button><button class="lb-nav lb-next" title="→">${icon('arrowRight', 20)}</button>` : ''}
    <div class="lb-stage"><img alt=""></div>
    <div class="lb-bar"><span class="lb-count"></span><span>${t('Click to zoom')}</span></div>`;
  const img = el.querySelector('img');
  const stage = el.querySelector('.lb-stage');
  const show = () => {
    zoom = false;
    el.classList.remove('zoomed');
    img.src = urls[i];
    el.querySelector('.lb-count').textContent = urls.length > 1 ? `${i + 1} / ${urls.length}` : '';
  };
  const go = (d) => {
    i = (i + d + urls.length) % urls.length;
    show();
  };
  const close = () => {
    el.remove();
    window.removeEventListener('keydown', onKey, true);
  };
  const onKey = (e) => {
    if (e.key === 'Escape') close();
    else if (e.key === 'ArrowLeft') go(-1);
    else if (e.key === 'ArrowRight') go(1);
    else return;
    e.preventDefault();
    e.stopPropagation();
  };
  // priblíženie na miesto kliknutia
  img.onclick = (e) => {
    e.stopPropagation();
    zoom = !zoom;
    el.classList.toggle('zoomed', zoom);
    if (zoom) {
      const r = img.getBoundingClientRect();
      const fx = (e.clientX - r.left) / r.width;
      const fy = (e.clientY - r.top) / r.height;
      requestAnimationFrame(() => {
        stage.scrollLeft = fx * stage.scrollWidth - stage.clientWidth / 2;
        stage.scrollTop = fy * stage.scrollHeight - stage.clientHeight / 2;
      });
    }
  };
  let drag = null;
  stage.onpointerdown = (e) => {
    if (!zoom) return;
    drag = { x: e.clientX, y: e.clientY, l: stage.scrollLeft, t: stage.scrollTop, moved: false };
  };
  stage.onpointermove = (e) => {
    if (!drag) return;
    if (Math.abs(e.clientX - drag.x) + Math.abs(e.clientY - drag.y) > 4) drag.moved = true;
    stage.scrollLeft = drag.l - (e.clientX - drag.x);
    stage.scrollTop = drag.t - (e.clientY - drag.y);
  };
  stage.onpointerup = () => setTimeout(() => (drag = null), 0);
  img.addEventListener('click', (e) => drag?.moved && e.stopImmediatePropagation(), true);
  el.onclick = (e) => {
    if (e.target.closest('.lb-prev')) return go(-1);
    if (e.target.closest('.lb-next')) return go(1);
    if (e.target === el || e.target === stage || e.target.closest('.lb-close')) close();
  };
  el.onwheel = (e) => {
    if (zoom || urls.length < 2) return;
    e.preventDefault();
    go(e.deltaY > 0 ? 1 : -1);
  };
  window.addEventListener('keydown', onKey, true);
  document.body.append(el);
  show();
}

export function createPluginsUI({ host, toast, openProject, builtins = [] }) {
  let list = [];
  let filter = 'all';
  let query = '';
  let box = null;

  const verified = (p) => (p.verified || p.publisher === 'Flux' ? `<span class="pl-verified" title="${t('Made by the Flux team')}">${icon('check', 10)}</span>` : '');
  const iconImg = (p, size = 40) =>
    p.iconUrl ? `<img class="pl-icon" src="${esc(p.iconUrl)}" width="${size}" height="${size}" alt="">` : `<span class="pl-icon ph" style="width:${size}px;height:${size}px">${icon('sparkle', size / 2)}</span>`;

  function actionBtn(p) {
    if (p.builtin) return `<span class="pl-new on">${icon('check', 11)}${t('Built in')}</span><input type="checkbox" class="switch" data-pl-enable="${p.id}"${p.enabled ? ' checked' : ''} title="${t('On / off')}">`;
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

  // Vstavané pluginy (súčasť Fluxu, zapínajú sa tu – napr. GitHub, ktorý potrebuje Git).
  let builtinBusy = null;
  function builtinCard(b) {
    const on = b.enabled();
    const btn =
      builtinBusy === b.id
        ? `<button class="s-btn" disabled><span class="spin"></span></button>`
        : on
          ? `<button class="s-btn" data-bi-off="${b.id}">${t('Uninstall')}</button>`
          : `<button class="s-btn primary" data-bi-on="${b.id}">${icon('download', 13)}${t('Install')}</button>`;
    return `<div class="pl-card builtin" data-bi-open="${b.id}">
      <span class="pl-icon ph bi-ic" style="width:40px;height:40px">${icon(b.icon, 20)}</span>
      <div class="pl-txt"><b>${esc(b.name)}</b><small>Flux${verified({ verified: true })}${b.needs ? ` · ${esc(b.needs)}` : ''}</small><p>${esc(b.description)}</p></div>
      <div class="pl-side">${on ? `<span class="pl-new on">${icon('check', 11)}${t('Installed')}</span>` : ''}${btn}</div>
    </div>`;
  }
  // Detail vstavaného pluginu (GitHub, Flux Together): popis a čo všetko robí.
  function builtinBtn(b) {
    if (builtinBusy === b.id) return `<button class="s-btn" disabled><span class="spin"></span></button>`;
    return b.enabled() ? `<button class="s-btn" data-bi-off="${b.id}">${t('Uninstall')}</button>` : `<button class="s-btn primary" data-bi-on="${b.id}">${icon('download', 13)}${t('Install')}</button>`;
  }
  function renderBuiltinDetail(id) {
    const b = builtins.find((x) => x.id === id);
    if (!b) return renderHome();
    detailId = `bi:${id}`;
    box.innerHTML = `
      <button class="pl-back" data-pl-back>${icon('arrowLeft', 15)}${t('All plugins')}</button>
      <div class="pl-head">
        <span class="pl-icon ph bi-ic" style="width:64px;height:64px">${icon(b.icon, 30)}</span>
        <div class="pl-txt"><h2>${esc(b.name)}</h2><small>Flux${verified({ verified: true })} · ${t('Built in')}${b.needs ? ` · ${esc(b.needs)}` : ''}</small><p>${esc(b.description)}</p></div>
        <div class="pl-side">${b.enabled() ? `<span class="pl-new on">${icon('check', 11)}${t('Installed')}</span>` : ''}${builtinBtn(b)}</div>
      </div>
      ${b.details?.length ? `<h3>${t('What it does')}</h3><ul class="bi-feats">${b.details.map((d) => `<li>${icon('check', 13)}<span>${esc(d)}</span></li>`).join('')}</ul>` : ''}`;
  }
  let bundled = [];
  const bundledCard = (p) => `<div class="pl-card builtin" data-pl-open="${p.id}">
      ${iconImg(p)}
      <div class="pl-txt"><b>${esc(p.name)}</b><small>Flux${verified(p)} · v${esc(p.version)}</small><p>${esc(p.description)}</p></div>
      <div class="pl-side"><input type="checkbox" class="switch" data-pl-enable="${p.id}"${p.enabled ? ' checked' : ''} title="${t('On / off')}"></div>
    </div>`;
  function renderBuiltins() {
    const el = box?.querySelector('#pl-builtin');
    if (el) el.innerHTML = builtins.map(builtinCard).join('') + bundled.map(bundledCard).join('');
  }
  async function loadBundled() {
    try {
      bundled = await flux.pluginsBuiltin();
    } catch {
      bundled = [];
    }
    renderBuiltins();
  }

  async function renderHome(force = false) {
    box.innerHTML = `
      <p class="s-lead">${t('Plugins add new features to Flux. Plugins from the Flux team are checked; community plugins are made by other people – install only what you trust.')}</p>
      ${builtins.length || true ? `<h3>${t('Built into Flux')}</h3><div class="pl-list" id="pl-builtin"></div><h3>${t('Store')}</h3>` : ''}
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
    renderBuiltins();
    loadBundled();
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
    box.innerHTML = `<button class="pl-back" data-pl-back>${icon('arrowLeft', 15)}${t('All plugins')}</button><div class="s-loading"><span class="spin"></span></div>`;
    let d;
    try {
      d = await flux.pluginDetails(id);
    } catch (err) {
      box.querySelector('.s-loading').textContent = errText(err);
      return;
    }
    const full = { ...p, ...d, installed: p.installed || bundled.some((x) => x.id === id), builtin: p.builtin || bundled.some((x) => x.id === id), enabled: p.enabled ?? bundled.find((x) => x.id === id)?.enabled, update: p.update };
    box.innerHTML = `
      <button class="pl-back" data-pl-back>${icon('arrowLeft', 15)}${t('All plugins')}</button>
      <div class="pl-head">
        ${iconImg(full, 64)}
        <div class="pl-txt"><h2>${esc(full.name)}</h2><small>${esc(full.publisher)}${verified(full)} · v${esc(full.version)}${full.license ? ` · ${esc(full.license)}` : ''}</small><p>${esc(full.description)}</p>
          <div class="pl-tags">${(full.tags || []).map((x) => `<span>${esc(x)}</span>`).join('')}</div></div>
        <div class="pl-side">${actionBtn(full)}${full.installed && !full.builtin ? `<button class="s-btn" data-pl-uninstall="${full.id}">${t('Uninstall')}</button>` : ''}</div>
      </div>
      <div class="pl-rate"><span class="pl-rate-sum">${rateSum(full)}</span>${full.issue ? `<button class="s-btn" data-rv-jump>${icon('star', 13)}${t('Rate')}</button>` : ''}</div>
      ${full.screenshotUrls?.length ? `<div class="pl-shots">${full.screenshotUrls.map((u, i) => `<button class="pl-shot" data-pl-zoom="${i}" title="${t('Click to enlarge')}"><img src="${esc(u)}" alt=""><span>${icon('search', 14)}</span></button>`).join('')}</div>` : ''}
      <div class="pl-readme ai-body">${markdown(full.readme || full.description || '')}</div>
      ${full.issue ? `<h3 id="rv-title">${t('Ratings and comments')}</h3><div id="pl-reviews"><div class="s-loading"><span class="spin"></span></div></div>` : ''}`;
    detailShots = full.screenshotUrls || [];
    if (full.issue) renderReviews(id);
  }

  // ---------- hodnotenia a komentáre ----------
  let detailShots = [];
  let rv = { id: null, data: null, stars: 0, busy: false };
  async function renderReviews(id, reload = true) {
    const el = box?.querySelector('#pl-reviews');
    if (!el) return;
    if (reload || rv.id !== id) {
      try {
        const data = await flux.pluginReviews(id);
        const mine = data.items.find((c) => c.stars && c.user === data.me);
        rv = { id, data, stars: mine?.stars || 0, text: mine?.body || '', mine, busy: false };
      } catch (err) {
        el.innerHTML = `<div class="s-loading">${t('Could not load comments: {msg}', { msg: esc(errText(err)) })}</div>`;
        return;
      }
    }
    if (detailId !== id || !box.contains(el)) return;
    const { data, mine } = rv;
    const sum = box.querySelector('.pl-rate-sum');
    if (sum) sum.innerHTML = rateSum(data);
    const pick = [1, 2, 3, 4, 5].map((n) => `<button class="rv-pick${n <= rv.stars ? ' on' : ''}" data-rv-star="${n}" title="${n} / 5">★</button>`).join('');
    const form = data.me
      ? `<div class="rv-form">
          <div class="rv-form-top"><span class="rv-picker">${pick}</span><small>${rv.stars ? [t('Bad'), t('Not great'), t('OK'), t('Good'), t('Excellent')][rv.stars - 1] : t('Tap a star to rate')}</small></div>
          <textarea id="rv-text" rows="3" maxlength="4000" placeholder="${t('Write a comment – what do you like, what is missing? (optional)')}">${esc(rv.text || '')}</textarea>
          <div class="rv-form-bar"><small>${t('Posted as {user} on GitHub', { user: `<b>${esc(data.me)}</b>` })}</small><span class="grow"></span>${mine ? `<button class="s-btn" data-rv-del="${mine.id}">${icon('trash', 13)}${t('Delete my review')}</button>` : ''}<button class="s-btn primary" data-rv-send${rv.busy ? ' disabled' : ''}>${rv.busy ? '<span class="spin"></span>' : icon('check', 13)}${mine ? t('Update review') : t('Post')}</button></div>
        </div>`
      : `<div class="rv-signin">${icon('github', 18)}<span><b>${t('Sign in with GitHub to rate and comment')}</b><small>${t('Everyone can read the comments. Your GitHub name is shown next to yours.')}</small></span><button class="s-btn primary" data-rv-signin>${t('Sign in')}</button></div>`;
    const items = data.items.filter((c) => c.stars || c.body);
    const list = items.length
      ? items
          .map(
            (c) => `<div class="rv-item${c.user === data.me ? ' mine' : ''}">
              ${c.avatar ? `<img class="rv-avatar" src="${esc(c.avatar)}&s=64" alt="">` : `<span class="rv-avatar ph">${esc(c.user.slice(0, 1).toUpperCase())}</span>`}
              <div class="rv-main">
                <div class="rv-meta"><b>${esc(c.user)}</b>${c.stars ? starRow(c.stars, 12) : ''}<small>${ago(c.date)}${c.edited ? ` · ${t('edited')}` : ''}</small>${c.user === data.me && !c.stars ? `<button class="icon-btn rv-x" data-rv-del="${c.id}" title="${t('Delete')}">${icon('trash', 12)}</button>` : ''}</div>
                ${c.body ? `<div class="rv-body ai-body">${markdown(c.body, { preview: true })}</div>` : ''}
              </div>
            </div>`,
          )
          .join('')
      : `<div class="rv-empty">${icon('sparkle', 16)}${t('No reviews yet – be the first!')}</div>`;
    el.innerHTML = form + `<div class="rv-list">${list}</div>`;
    const ta = el.querySelector('#rv-text');
    if (ta) ta.oninput = () => (rv.text = ta.value);
  }

  async function sendReview() {
    const text = box.querySelector('#rv-text')?.value || '';
    if (!rv.stars && !text.trim()) return toast(t('Pick stars or write a comment.'), 'info');
    rv.busy = true;
    renderReviews(rv.id, false);
    try {
      await flux.pluginReview(rv.id, rv.stars, text);
      toast(rv.stars ? t('Thanks for rating!') : t('Comment posted.'), 'ok');
      list = await flux.pluginRegistry(true).catch(() => list);
      await renderDetail(rv.id);
    } catch (err) {
      rv.busy = false;
      renderReviews(rv.id, false);
      toast(errText(err), 'error', 7000);
    }
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
    const b = e.target.closest('button, input, [data-pl-open], [data-bi-open]');
    if (!b) return;
    if (b.dataset.plZoom !== undefined) return openShots(detailShots, Number(b.dataset.plZoom) || 0);
    if (b.dataset.rvStar) {
      const n = Number(b.dataset.rvStar);
      rv.stars = rv.stars === n ? 0 : n;
      return renderReviews(rv.id, false);
    }
    if (b.dataset.rvSend !== undefined) return sendReview();
    if (b.dataset.rvJump !== undefined) {
      box.querySelector('#rv-title')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
      setTimeout(() => box.querySelector('#rv-text')?.focus(), 400);
      return;
    }
    if (b.dataset.rvDel) {
      try {
        await flux.pluginDeleteComment(b.dataset.rvDel);
        toast(t('Deleted.'), 'ok');
        list = await flux.pluginRegistry(true).catch(() => list);
        return renderDetail(rv.id);
      } catch (err) {
        return toast(errText(err), 'error', 7000);
      }
    }
    if (b.dataset.rvSignin !== undefined) {
      const tab = document.querySelector('#settings [data-tab="github"]');
      if (tab) return tab.click();
      return toast(t('Turn on GitHub in Settings → Plugins → Built into Flux first.'), 'info', 6000);
    }
    if (b.dataset.biOn || b.dataset.biOff) {
      const bi = builtins.find((x) => x.id === (b.dataset.biOn || b.dataset.biOff));
      builtinBusy = bi.id;
      renderBuiltins();
      if (detailId === `bi:${bi.id}`) renderBuiltinDetail(bi.id);
      try {
        await bi.set(!!b.dataset.biOn);
      } finally {
        builtinBusy = null;
        if (detailId === `bi:${bi.id}`) renderBuiltinDetail(bi.id);
        else renderBuiltins();
      }
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
      bundled = await flux.pluginsBuiltin().catch(() => bundled);
      return;
    }
    if (b.dataset.plUninstall) {
      await host.unload(b.dataset.plUninstall);
      await flux.pluginUninstall(b.dataset.plUninstall);
      list = await flux.pluginRegistry();
      toast(t('Plugin removed.'), 'ok');
      return renderDetail(b.dataset.plUninstall);
    }
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
    const biOpen = e.target.closest('[data-bi-open]');
    if (biOpen && !e.target.closest('button, input')) return renderBuiltinDetail(biOpen.dataset.biOpen);
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
