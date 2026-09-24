// Vlastné rozbaľovacie menu namiesto systémového <select> (ten má na Windows biely zoznam).
// Pôvodný <select> ostane skrytý v stránke – hodnota, udalosť 'change' aj nastavenia fungujú ako doteraz.
import { icon } from './icons.js';

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
let openMenu = null;

function close() {
  if (!openMenu) return;
  openMenu.menu.remove();
  openMenu.button.classList.remove('open');
  openMenu.button.setAttribute('aria-expanded', 'false');
  openMenu = null;
}

function label(select) {
  const o = select.options[select.selectedIndex];
  return o ? o.textContent : '';
}

function openFor(select, button) {
  if (openMenu?.select === select) return close();
  close();
  const menu = document.createElement('div');
  menu.className = 'fsel-menu';
  menu.setAttribute('role', 'listbox');
  menu.innerHTML = [...select.options]
    .map((o, i) => `<button type="button" role="option" class="fsel-opt${i === select.selectedIndex ? ' on' : ''}" data-i="${i}"${o.disabled ? ' disabled' : ''}><span>${esc(o.textContent)}</span>${i === select.selectedIndex ? icon('check', 13) : ''}</button>`)
    .join('');
  document.body.append(menu);
  const r = button.getBoundingClientRect();
  const h = Math.min(menu.scrollHeight, 320);
  const below = window.innerHeight - r.bottom - 8;
  menu.style.minWidth = `${r.width}px`;
  menu.style.left = `${Math.min(r.left, window.innerWidth - menu.offsetWidth - 8)}px`;
  menu.style.top = below >= h || below > r.top ? `${r.bottom + 4}px` : `${Math.max(8, r.top - h - 4)}px`;
  menu.style.maxHeight = `${Math.max(120, below >= h || below > r.top ? below : r.top - 12)}px`;
  button.classList.add('open');
  button.setAttribute('aria-expanded', 'true');
  openMenu = { select, button, menu };
  menu.querySelector('.fsel-opt.on')?.scrollIntoView({ block: 'nearest' });
  (menu.querySelector('.fsel-opt.on') || menu.querySelector('.fsel-opt'))?.focus();
  menu.onclick = (e) => {
    const b = e.target.closest('.fsel-opt');
    if (!b) return;
    pick(select, Number(b.dataset.i));
    close();
    button.focus();
  };
  menu.onkeydown = (e) => {
    const opts = [...menu.querySelectorAll('.fsel-opt:not([disabled])')];
    const i = opts.indexOf(document.activeElement);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      opts[(i + (e.key === 'ArrowDown' ? 1 : opts.length - 1)) % opts.length]?.focus();
    } else if (e.key === 'Escape' || e.key === 'Tab') {
      e.preventDefault();
      e.stopPropagation();
      close();
      button.focus();
    }
  };
}

function pick(select, index) {
  if (select.selectedIndex === index) return;
  select.selectedIndex = index;
  select.dispatchEvent(new Event('input', { bubbles: true }));
  select.dispatchEvent(new Event('change', { bubbles: true }));
}

function enhance(select) {
  if (select.dataset.fsel || select.multiple || select.size > 1 || select.classList.contains('native')) return;
  select.dataset.fsel = '1';
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'fsel';
  button.setAttribute('aria-haspopup', 'listbox');
  if (select.title) button.title = select.title;
  const draw = () => {
    button.innerHTML = `<span>${esc(label(select))}</span>${icon('chevron', 12)}`;
    button.disabled = select.disabled;
  };
  draw();
  select.after(button);
  select.classList.add('fsel-native');
  select.addEventListener('change', draw);
  // obsah doplnený neskôr (napr. zoznam modelov) alebo zmena hodnoty z kódu
  new MutationObserver(draw).observe(select, { childList: true, subtree: true, attributes: true });
  const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value');
  Object.defineProperty(select, 'value', {
    configurable: true,
    get: () => setter.get.call(select),
    set: (v) => {
      setter.set.call(select, v);
      draw();
    },
  });
  button.onclick = (e) => {
    e.preventDefault();
    e.stopPropagation();
    openFor(select, button);
  };
  button.onkeydown = (e) => {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) {
      e.preventDefault();
      openFor(select, button);
    }
  };
}

// Všetky <select> v aplikácii – aj tie, ktoré pribudnú neskôr.
export function setupFancySelects(root = document.body) {
  root.querySelectorAll('select').forEach(enhance);
  new MutationObserver((list) => {
    for (const m of list)
      for (const n of m.addedNodes) {
        if (n.nodeType !== 1) continue;
        if (n.tagName === 'SELECT') enhance(n);
        else n.querySelectorAll?.('select').forEach(enhance);
      }
  }).observe(root, { childList: true, subtree: true });
  document.addEventListener('pointerdown', (e) => {
    if (openMenu && !openMenu.menu.contains(e.target) && !openMenu.button.contains(e.target)) close();
  }, true);
  window.addEventListener('resize', close);
  document.addEventListener('scroll', (e) => {
    if (openMenu && !openMenu.menu.contains(e.target)) close();
  }, true);
}
