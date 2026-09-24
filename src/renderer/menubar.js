// Ponuka aplikácie (File, Edit, View, Run, Help).
// Predvolene je schovaná v logu „flux“ (šetrí miesto): klik otvorí zoznam ponúk, prejdením myšou sa otvorí podmenu.
// Voliteľne ako klasický riadok File / Edit / View… v hornej lište.
// Položky dodáva app.js: [popis, akcia, skratka, { checked, radio, disabled }], '-' = oddeľovač, 'text' = nadpis.
export function createMenubar({ bar, triggers, icon, esc, getMenus }) {
  let state = null; // { anchor, menuId | null (koreň), sub }

  const itemsHtml = (items) =>
    items
      .filter(Boolean)
      .map((it, i) => {
        if (it === '-') return '<div class="mb-sep"></div>';
        if (typeof it === 'string') return `<div class="mb-cap">${esc(it)}</div>`;
        const [label, , key, o = {}] = it;
        const mark = o.checked === undefined ? '' : o.checked ? icon(o.radio ? 'dot' : 'check', 13) : '';
        return `<button class="mb-item${o.disabled ? ' off' : ''}" data-mi="${i}" type="button"${o.disabled ? ' disabled' : ''}><span class="mb-mark">${mark}</span><span class="mb-label">${esc(label)}</span>${key ? `<kbd>${esc(key)}</kbd>` : ''}</button>`;
      })
      .join('');

  function place(drop, x, y) {
    document.body.append(drop);
    drop.style.left = `${Math.max(6, Math.min(x, innerWidth - drop.offsetWidth - 6))}px`;
    drop.style.top = `${Math.max(6, Math.min(y, innerHeight - drop.offsetHeight - 6))}px`;
  }

  function dropFor(menu, cls = '') {
    const d = document.createElement('div');
    d.className = `mb-drop ${cls}`;
    d.setAttribute('role', 'menu');
    d.innerHTML = itemsHtml(menu.items);
    d.onclick = (e) => {
      const b = e.target.closest('[data-mi]');
      if (!b) return;
      const item = menu.items.filter(Boolean)[Number(b.dataset.mi)];
      close();
      setTimeout(() => item[1]?.(), 0);
    };
    return d;
  }

  function render() {
    if (bar) {
      bar.innerHTML = getMenus()
        .filter((m) => !m.run)
        .map((m) => `<button class="mb-top${state?.menuId === m.id && state.anchor === 'bar' ? ' on' : ''}" data-mb="${m.id}" type="button">${esc(m.label)}</button>`)
        .join('');
    }
    document.querySelectorAll('.mb-drop').forEach((d) => d.remove());
    for (const t of triggers) t.classList.toggle('on', state?.anchor === t);
    if (!state) return;
    const menus = getMenus();
    if (state.anchor === 'bar') {
      const menu = menus.find((m) => m.id === state.menuId);
      const btn = bar.querySelector(`[data-mb="${state.menuId}"]`);
      if (!menu || !btn) return;
      const r = btn.getBoundingClientRect();
      place(dropFor(menu), r.left, r.bottom + 4);
      return;
    }
    // Koreň (☰ vedľa loga): File ▸, Edit ▸ … a vedľa otvorené podmenu.
    // Koreň sa kreslí raz; pri prechode myšou sa mení len podmenu (bez blikania a animácie znova).
    const root = document.createElement('div');
    root.className = 'mb-drop mb-root';
    root.innerHTML = menus
      .map((m) =>
        m.run
          ? `<button class="mb-item" data-run="${m.id}" type="button"><span class="mb-mark">${m.icon ? icon(m.icon, 13) : ''}</span><span class="mb-label">${esc(m.label)}</span>${m.key ? `<kbd>${esc(m.key)}</kbd>` : ''}</button><div class="mb-sep"></div>`
          : `<button class="mb-item mb-parent" data-sub="${m.id}" type="button"><span class="mb-mark"></span><span class="mb-label">${esc(m.label)}</span>${icon('chevron', 12)}</button>`,
      )
      .join('');
    const r = state.anchor.getBoundingClientRect();
    place(root, r.left, r.bottom + 4);
    let timer = 0;
    const showSub = (id) => {
      clearTimeout(timer);
      if (!state) return;
      state.sub = id;
      root.querySelectorAll('[data-sub]').forEach((b) => b.classList.toggle('on', b.dataset.sub === id));
      document.querySelector('.mb-sub')?.remove();
      const menu = menus.find((m) => m.id === id);
      if (!menu) return;
      const row = root.querySelector(`[data-sub="${id}"]`).getBoundingClientRect();
      const sub = dropFor(menu, 'mb-sub');
      document.body.append(sub);
      const x = row.right + 4 + sub.offsetWidth > innerWidth ? row.left - sub.offsetWidth - 4 : row.right + 4;
      place(sub, x, row.top - 5);
      sub.addEventListener('mouseenter', () => clearTimeout(timer));
    };
    // Malé oneskorenie: keď ideš myšou šikmo do podmenu cez iný riadok, podmenu nepreskočí.
    root.addEventListener('mouseover', (e) => {
      const b = e.target.closest('[data-sub]');
      if (!b || b.dataset.sub === state?.sub) return clearTimeout(timer);
      clearTimeout(timer);
      timer = setTimeout(() => showSub(b.dataset.sub), 140);
    });
    root.addEventListener('click', (e) => {
      const run = e.target.closest('[data-run]');
      if (run) {
        const m = menus.find((x) => x.id === run.dataset.run);
        close();
        return setTimeout(() => m?.run(), 0);
      }
      const b = e.target.closest('[data-sub]');
      if (b) showSub(b.dataset.sub);
    });
    if (state.sub) showSub(state.sub);
  }

  function close() {
    if (!state) return;
    state = null;
    render();
  }

  bar?.addEventListener('click', (e) => {
    const b = e.target.closest('[data-mb]');
    if (!b) return;
    state = state?.anchor === 'bar' && state.menuId === b.dataset.mb ? null : { anchor: 'bar', menuId: b.dataset.mb };
    render();
  });
  // Keď je jedna ponuka otvorená, prejdením myšou sa otvorí susedná.
  bar?.addEventListener('mouseover', (e) => {
    const b = e.target.closest('[data-mb]');
    if (b && state?.anchor === 'bar' && state.menuId !== b.dataset.mb) {
      state.menuId = b.dataset.mb;
      render();
    }
  });
  for (const t of triggers) {
    t.addEventListener('click', (e) => {
      e.stopPropagation();
      state = state?.anchor === t ? null : { anchor: t, sub: null };
      render();
    });
  }
  document.addEventListener('pointerdown', (e) => {
    if (!state || e.target.closest('.mb-drop, [data-menu-trigger]') || bar?.contains(e.target) || triggers.some((t) => t.contains(e.target))) return;
    close();
  });
  window.addEventListener(
    'keydown',
    (e) => {
      if (state && e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        close();
      }
    },
    true,
  );
  window.addEventListener('blur', close);
  window.addEventListener('resize', close);

  // Ponuka z ďalšieho tlačidla (napr. ☰ na domovskej obrazovke, ktorá sa kreslí znova a znova).
  function openAt(el) {
    state = state?.anchor === el ? null : { anchor: el, sub: null };
    render();
  }

  return { render, close, openAt, isOpen: () => !!state };
}
