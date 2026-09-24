// Vlastné rozbaľovacie menu namiesto systémového <select> (ten má na Windows biely zoznam).
// Pôvodný <select> ostane skrytý v stránke – hodnota, udalosť 'change' aj nastavenia fungujú ako doteraz.
import { icon } from './icons.js';

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
let openMenu = null;

function closeMenu() {
  if (!openMenu) return;
  openMenu.menu.remove();
  openMenu.button.classList.remove('open');
  openMenu.button.setAttribute('aria-expanded', 'false');
  openMenu = null;
}

const close = () => {
  closeMenu();
  closeColor();
};

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

// ---------- výber farby namiesto systémového okna ----------
const PRESETS = ['#8b7bff', '#6366f1', '#3b82f6', '#06b6d4', '#14b8a6', '#22c55e', '#84cc16', '#eab308', '#f59e0b', '#f97316', '#ef4444', '#ec4899', '#d946ef', '#a855f7', '#e8e8ef', '#26262c'];
const hex2rgb = (h) => {
  const m = /^#?([0-9a-f]{6})$/i.exec(h || '');
  const n = m ? parseInt(m[1], 16) : 0x8b7bff;
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
};
const rgb2hex = (r, g, b) => '#' + [r, g, b].map((x) => Math.round(x).toString(16).padStart(2, '0')).join('');
function rgb2hsv(r, g, b) {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min;
  let h = 0;
  if (d) h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [((h * 60) + 360) % 360, max ? d / max : 0, max];
}
function hsv2rgb(h, s, v) {
  const f = (n) => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
  };
  return [f(5) * 255, f(3) * 255, f(1) * 255];
}
let openColor = null;
function closeColor() {
  if (!openColor) return;
  openColor.pop.remove();
  openColor = null;
}
function openColorFor(input, button) {
  if (openColor?.input === input) return closeColor();
  closeColor();
  closeMenu();
  let [h, sat, val] = rgb2hsv(...hex2rgb(input.value));
  const pop = document.createElement('div');
  pop.className = 'fcolor-pop';
  pop.innerHTML = `<div class="fc-sv"><i class="fc-knob"></i></div>
    <input type="range" class="fc-hue" min="0" max="360" step="1">
    <div class="fc-presets">${PRESETS.map((c) => `<button type="button" style="--c:${c}" data-c="${c}"></button>`).join('')}</div>
    <div class="fc-row"><span class="fc-now"></span><input class="fc-hex" maxlength="7" spellcheck="false"></div>`;
  document.body.append(pop);
  const sv = pop.querySelector('.fc-sv');
  const knob = pop.querySelector('.fc-knob');
  const hue = pop.querySelector('.fc-hue');
  const hexIn = pop.querySelector('.fc-hex');
  const now = pop.querySelector('.fc-now');
  const draw = (fromHex = false) => {
    const hex = rgb2hex(...hsv2rgb(h, sat, val));
    sv.style.background = `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(${h} 100% 50%))`;
    knob.style.left = `${sat * 100}%`;
    knob.style.top = `${(1 - val) * 100}%`;
    knob.style.background = hex;
    hue.value = h;
    now.style.background = hex;
    if (!fromHex) hexIn.value = hex;
    return hex;
  };
  const commit = (final) => {
    const hex = rgb2hex(...hsv2rgb(h, sat, val));
    if (input.value.toLowerCase() !== hex) {
      input.value = hex;
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
    if (final) input.dispatchEvent(new Event('change', { bubbles: true }));
    paintButton(input, button);
  };
  const pick = (e) => {
    const r = sv.getBoundingClientRect();
    sat = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
    val = Math.min(1, Math.max(0, 1 - (e.clientY - r.top) / r.height));
    draw();
    commit(false);
  };
  sv.onpointerdown = (e) => {
    sv.setPointerCapture(e.pointerId);
    pick(e);
    sv.onpointermove = pick;
    sv.onpointerup = () => {
      sv.onpointermove = null;
      commit(true);
    };
  };
  hue.oninput = () => {
    h = Number(hue.value);
    draw();
    commit(false);
  };
  hue.onchange = () => commit(true);
  hexIn.oninput = () => {
    if (!/^#?[0-9a-f]{6}$/i.test(hexIn.value.trim())) return;
    [h, sat, val] = rgb2hsv(...hex2rgb(hexIn.value.trim().replace(/^#?/, '#')));
    draw(true);
    commit(true);
  };
  pop.querySelector('.fc-presets').onclick = (e) => {
    const b = e.target.closest('[data-c]');
    if (!b) return;
    [h, sat, val] = rgb2hsv(...hex2rgb(b.dataset.c));
    draw();
    commit(true);
  };
  draw();
  const r = button.getBoundingClientRect();
  const w = 232;
  pop.style.left = `${Math.max(8, Math.min(r.right - w, window.innerWidth - w - 8))}px`;
  const hgt = pop.offsetHeight;
  pop.style.top = window.innerHeight - r.bottom > hgt + 12 ? `${r.bottom + 6}px` : `${Math.max(8, r.top - hgt - 6)}px`;
  openColor = { input, button, pop };
}
function paintButton(input, button) {
  button.querySelector('.fc-sw').style.background = input.value;
  button.querySelector('.fc-txt').textContent = input.value.toUpperCase();
}
function enhanceColor(input) {
  if (input.dataset.fsel || input.classList.contains('native')) return;
  input.dataset.fsel = '1';
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'fcolor';
  if (input.title) button.title = input.title;
  button.innerHTML = '<span class="fc-sw"></span><span class="fc-txt"></span>';
  input.after(button);
  input.classList.add('fsel-native');
  paintButton(input, button);
  input.addEventListener('input', () => paintButton(input, button));
  // hodnota zmenená z kódu (napr. HEX pole v štúdiu tém) → prekresliť štvorček
  const desc = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value');
  Object.defineProperty(input, 'value', {
    configurable: true,
    get: () => desc.get.call(input),
    set: (v) => {
      desc.set.call(input, v);
      paintButton(input, button);
    },
  });
  button.onclick = (e) => {
    e.preventDefault();
    e.stopPropagation();
    openColorFor(input, button);
  };
}

// Posuvník: dráha vyplnená po jazdca (CSS premenná --fill).
function fillRange(r) {
  const min = Number(r.min || 0);
  const max = Number(r.max || 100);
  r.style.setProperty('--fill', `${((Number(r.value) - min) / (max - min || 1)) * 100}%`);
}

// Všetky <select> v aplikácii – aj tie, ktoré pribudnú neskôr.
export function setupFancySelects(root = document.body) {
  root.querySelectorAll('select').forEach(enhance);
  root.querySelectorAll('input[type=color]').forEach(enhanceColor);
  root.querySelectorAll('input[type=range]').forEach(fillRange);
  document.addEventListener('input', (e) => {
    if (e.target.type === 'range') fillRange(e.target);
  }, true);
  new MutationObserver((list) => {
    for (const m of list)
      for (const n of m.addedNodes) {
        if (n.nodeType !== 1) continue;
        if (n.tagName === 'SELECT') enhance(n);
        else n.querySelectorAll?.('select').forEach(enhance);
        if (n.type === 'range') fillRange(n);
        else n.querySelectorAll?.('input[type=range]').forEach(fillRange);
        if (n.type === 'color') enhanceColor(n);
        else n.querySelectorAll?.('input[type=color]').forEach(enhanceColor);
      }
  }).observe(root, { childList: true, subtree: true });
  document.addEventListener('pointerdown', (e) => {
    if (openMenu && !openMenu.menu.contains(e.target) && !openMenu.button.contains(e.target)) closeMenu();
    if (openColor && !openColor.pop.contains(e.target) && !openColor.button.contains(e.target)) closeColor();
  }, true);
  // Esc zavrie len výber farby (nie celé nastavenia) – registrované skôr ako ostatné skratky.
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && openColor) {
      e.preventDefault();
      e.stopImmediatePropagation();
      closeColor();
    }
  }, true);
  window.addEventListener('resize', close);
  document.addEventListener('scroll', (e) => {
    if (openMenu && !openMenu.menu.contains(e.target)) closeMenu();
    if (openColor && !openColor.pop.contains(e.target)) closeColor();
  }, true);
}
