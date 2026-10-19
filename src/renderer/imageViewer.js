// Prehliadač obrázkov (Glance): ← → a pás náhľadov listujú obrázky v priečinku.
// Úpravy: otočenie, prevrátenie, orezanie, zmena veľkosti; uloženie aj do iného formátu (PNG, JPG, WebP, ICO).
// Po prevode do nového súboru app.js ponúkne opraviť cesty v kóde (afterConvert), aby stránka fungovala ďalej.
import { t } from './i18n.js';
import { icon } from './icons.js';

export const IMAGES = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'ico', 'bmp', 'avif']);
// do týchto formátov vie Flux zapisovať (canvas → súbor)
const WRITABLE = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', webp: 'image/webp' };
const ICO_SIZES = [16, 32, 48, 64, 128, 256];
const ASPECTS = [
  ['free', t('Free')],
  ['1:1', '1:1'],
  ['16:9', '16:9'],
  ['4:3', '4:3'],
  ['3:2', '3:2'],
];

const extOf = (p) => (p.match(/\.([^.\\/]+)$/)?.[1] || '').toLowerCase();
const nameOf = (p) => p.split(/[\\/]/).pop();
const dirOf = (p) => p.replace(/[\\/][^\\/]*$/, '');
const sep = (p) => (p.includes('\\') ? '\\' : '/');
const kb = (n) => (n < 1024 * 1024 ? `${Math.max(1, Math.round(n / 1024))} kB` : `${(n / 1024 / 1024).toFixed(1)} MB`);

const blobOf = (canvas, type, quality) => new Promise((ok, fail) => canvas.toBlob((b) => (b ? ok(b) : fail(new Error(t('This format cannot be saved here.')))), type, quality));
const b64 = async (blob) => {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
};

// ICO = hlavička + PNG obrázky vo viacerých veľkostiach (štvorec, obrázok vycentrovaný)
async function icoBlob(src) {
  const max = Math.max(src.width, src.height);
  const sizes = ICO_SIZES.filter((s, i) => i < 2 || s <= max);
  const pngs = [];
  for (const s of sizes) {
    const c = document.createElement('canvas');
    c.width = c.height = s;
    const g = c.getContext('2d');
    g.imageSmoothingQuality = 'high';
    const k = s / max;
    const w = src.width * k;
    const h = src.height * k;
    g.drawImage(src, (s - w) / 2, (s - h) / 2, w, h);
    pngs.push(new Uint8Array(await (await blobOf(c, 'image/png')).arrayBuffer()));
  }
  const head = new DataView(new ArrayBuffer(6 + 16 * sizes.length));
  head.setUint16(2, 1, true);
  head.setUint16(4, sizes.length, true);
  let offset = head.byteLength;
  sizes.forEach((s, i) => {
    const o = 6 + 16 * i;
    head.setUint8(o, s >= 256 ? 0 : s);
    head.setUint8(o + 1, s >= 256 ? 0 : s);
    head.setUint16(o + 4, 1, true);
    head.setUint16(o + 6, 32, true);
    head.setUint32(o + 8, pngs[i].length, true);
    head.setUint32(o + 12, offset, true);
    offset += pngs[i].length;
  });
  return new Blob([head.buffer, ...pngs], { type: 'image/x-icon' });
}

// nové plátno z existujúceho (otočenie/prevrátenie/výrez/zmena veľkosti)
function transform(src, { rotate = 0, flip = false, crop = null, width = null }) {
  const r = crop || { x: 0, y: 0, w: src.width, h: src.height };
  let w = r.w;
  let h = r.h;
  if (width) {
    h = Math.max(1, Math.round((h * width) / w));
    w = width;
  }
  const turn = rotate % 180 !== 0;
  const c = document.createElement('canvas');
  c.width = turn ? h : w;
  c.height = turn ? w : h;
  const g = c.getContext('2d');
  g.imageSmoothingQuality = 'high';
  g.translate(c.width / 2, c.height / 2);
  g.rotate((rotate * Math.PI) / 180);
  if (flip) g.scale(-1, 1);
  g.drawImage(src, r.x, r.y, r.w, r.h, -w / 2, -h / 2, w, h);
  return c;
}

export function createImageViewer({ el, flux, toast, errorText, confirmPalette, promptPalette, afterConvert }) {
  let list = [];
  let idx = 0;
  let path = null;
  let size = 0;
  let img = null; // načítaný obrázok
  let canvas = null; // upravená verzia (po prvej úprave)
  let undo = [];
  let crop = null; // { x, y, w, h, aspect } v pixeloch zobrazenia
  let token = 0;
  let thumbs = new Map();

  const dirty = () => !!canvas;
  const src = () => canvas || img;
  const q = (s) => el.querySelector(s);

  function shell() {
    el.innerHTML = `<div class="gl-card">
      <div class="gl-bar">
        <span class="gl-name"></span><span class="gl-meta"></span><div class="grow"></div>
        <div class="gl-tools">
          <button class="icon-btn" data-act="rotl" title="${t('Rotate left')}">${icon('rotateL', 15)}</button>
          <button class="icon-btn" data-act="rotr" title="${t('Rotate right (R)')}">${icon('rotateR', 15)}</button>
          <button class="icon-btn" data-act="flip" title="${t('Flip horizontally')}">${icon('flip', 15)}</button>
          <button class="icon-btn" data-act="crop" title="${t('Crop (C)')}">${icon('crop', 15)}</button>
          <button class="icon-btn" data-act="resize" title="${t('Resize…')}">${icon('resize', 15)}</button>
          <span class="gl-sep"></span>
          <button class="icon-btn" data-act="undo" title="${t('Undo (Ctrl+Z)')}">${icon('undo', 15)}</button>
          <button class="s-btn" data-act="save" title="Ctrl+S">${t('Save')}</button>
          <button class="s-btn primary" data-act="saveas" title="${t('Convert / save as…')} (Ctrl+Shift+S)">${icon('image', 13)}${t('Convert / save as…')}</button>
        </div>
        <div class="gl-croptools" hidden>
          <div class="seg gl-aspect">${ASPECTS.map(([v, l]) => `<button data-aspect="${v}">${l}</button>`).join('')}</div>
          <span class="gl-cropsize"></span>
          <button class="s-btn" data-act="cropcancel">${t('Cancel')}</button>
          <button class="s-btn primary" data-act="cropapply">${t('Crop')}</button>
        </div>
        <button class="icon-btn" data-act="close" title="${t('Close (Esc)')}">${icon('x', 15)}</button>
      </div>
      <div class="gl-stage">
        <button class="gl-nav prev" data-act="prev" title="←">${icon('arrowLeft', 18)}</button>
        <div class="gl-view"></div>
        <button class="gl-nav next" data-act="next" title="→">${icon('arrowRight', 18)}</button>
      </div>
      <div class="gl-strip no-smooth"></div>
    </div>`;
    el.onclick = (e) => {
      if (e.target === el) return close();
      const b = e.target.closest('[data-act], [data-aspect], [data-i]');
      if (!b) return;
      if (b.dataset.aspect) return setAspect(b.dataset.aspect);
      if (b.dataset.i) return go(Number(b.dataset.i));
      act(b.dataset.act);
    };
  }

  function act(a) {
    if (a === 'close') return close();
    if (a === 'prev') return go(idx - 1);
    if (a === 'next') return go(idx + 1);
    if (a === 'rotl') return edit({ rotate: 270 });
    if (a === 'rotr') return edit({ rotate: 90 });
    if (a === 'flip') return edit({ flip: true });
    if (a === 'crop') return startCrop();
    if (a === 'cropcancel') return stopCrop();
    if (a === 'cropapply') return applyCrop();
    if (a === 'resize') return resize();
    if (a === 'undo') return doUndo();
    if (a === 'save') return save();
    if (a === 'saveas') return saveAs();
  }

  async function listFolder(p) {
    try {
      const items = await flux.list(dirOf(p));
      const imgs = items.filter((i) => !i.dir && IMAGES.has(extOf(i.name))).map((i) => i.path);
      return imgs.includes(p) ? imgs : [p];
    } catch {
      return [p];
    }
  }

  async function open(p) {
    if (el.hidden || !el.firstChild) shell();
    el.hidden = false;
    document.activeElement?.blur?.();
    list = await listFolder(p);
    thumbs = new Map();
    renderStrip();
    await show(list.indexOf(p));
  }

  async function show(i) {
    idx = (i + list.length) % list.length;
    path = list[idx];
    canvas = null;
    undo = [];
    stopCrop();
    const my = ++token;
    q('.gl-name').textContent = nameOf(path);
    q('.gl-meta').textContent = '';
    let data;
    try {
      data = await flux.readImage(path);
    } catch (err) {
      return toast(errorText(err), 'error');
    }
    if (my !== token) return;
    size = data.size;
    const image = new Image();
    image.onload = () => {
      if (my !== token) return;
      // SVG bez rozmerov – aspoň 512 px, nech sa dá upravovať
      if (!image.naturalWidth) image.width = image.height = 512;
      img = image;
      draw();
    };
    image.onerror = () => my === token && (q('.gl-meta').textContent = t('This image cannot be shown.'));
    image.src = data.url;
    q('.gl-nav.prev').hidden = q('.gl-nav.next').hidden = list.length < 2;
    for (const b of el.querySelectorAll('.gl-thumb')) b.classList.toggle('on', Number(b.dataset.i) === idx);
    el.querySelector(`.gl-thumb[data-i="${idx}"]`)?.scrollIntoView({ block: 'nearest', inline: 'center' });
  }

  function draw() {
    const view = q('.gl-view');
    view.textContent = '';
    const node = canvas || img;
    node.classList.add('gl-img');
    view.append(node);
    const s = src();
    const w = s.naturalWidth || s.width;
    const h = s.naturalHeight || s.height;
    const pos = list.length > 1 ? ` · ${idx + 1} / ${list.length}` : '';
    q('.gl-meta').textContent = `${w} × ${h} px · ${dirty() ? t('edited') : kb(size)}${pos}`;
    q('[data-act="undo"]').disabled = !undo.length;
    q('[data-act="save"]').disabled = !dirty();
  }

  function base() {
    if (canvas) return canvas;
    const c = document.createElement('canvas');
    c.width = img.naturalWidth || img.width;
    c.height = img.naturalHeight || img.height;
    c.getContext('2d').drawImage(img, 0, 0, c.width, c.height);
    return c;
  }
  function edit(op) {
    if (!img) return;
    undo.push(canvas);
    if (undo.length > 15) undo.shift();
    canvas = transform(base(), op);
    draw();
  }
  function doUndo() {
    if (!undo.length) return;
    canvas = undo.pop();
    draw();
  }

  async function resize() {
    if (!img) return;
    const w = base().width;
    const v = await promptPalette({ value: String(w), note: t('New width in pixels or %, e.g. 800 or 50% (height follows)') });
    if (!v) return;
    const n = /%$/.test(v.trim()) ? Math.round((w * parseFloat(v)) / 100) : parseInt(v, 10);
    if (!(n > 0 && n <= 16384)) return toast(t('Enter a width between 1 and 16384.'), 'error');
    if (n !== w) edit({ width: n });
  }

  // ---------- orezanie ----------
  function startCrop() {
    if (!img || crop) return;
    const node = q('.gl-img');
    const w = node.clientWidth;
    const h = node.clientHeight;
    crop = { x: w * 0.1, y: h * 0.1, w: w * 0.8, h: h * 0.8, aspect: 'free' };
    const layer = document.createElement('div');
    layer.className = 'gl-croplayer';
    layer.innerHTML = '<div class="gl-croprect">' + ['nw', 'ne', 'sw', 'se'].map((c) => `<i data-h="${c}"></i>`).join('') + '</div>';
    q('.gl-view').append(layer);
    place(layer, node);
    cropEvents(layer);
    q('.gl-tools').hidden = true;
    q('.gl-croptools').hidden = false;
    setAspect('free');
  }
  function place(layer, node) {
    Object.assign(layer.style, { left: `${node.offsetLeft}px`, top: `${node.offsetTop}px`, width: `${node.clientWidth}px`, height: `${node.clientHeight}px` });
  }
  function stopCrop() {
    crop = null;
    el.querySelector('.gl-croplayer')?.remove();
    if (!q('.gl-tools')) return;
    q('.gl-tools').hidden = false;
    q('.gl-croptools').hidden = true;
  }
  function ratio() {
    if (!crop || crop.aspect === 'free') return 0;
    const [a, b] = crop.aspect.split(':').map(Number);
    return a / b;
  }
  function setAspect(a) {
    if (!crop) return;
    crop.aspect = a;
    for (const b of el.querySelectorAll('[data-aspect]')) b.classList.toggle('active', b.dataset.aspect === a);
    const r = ratio();
    if (r) {
      const layer = q('.gl-croplayer');
      const W = layer.clientWidth;
      const H = layer.clientHeight;
      let w = crop.w;
      let h = w / r;
      if (h > H) (h = H), (w = h * r);
      if (w > W) (w = W), (h = w / r);
      crop.x = Math.min(Math.max(0, crop.x + (crop.w - w) / 2), W - w);
      crop.y = Math.min(Math.max(0, crop.y + (crop.h - h) / 2), H - h);
      crop.w = w;
      crop.h = h;
    }
    paintCrop();
  }
  function paintCrop() {
    const rect = el.querySelector('.gl-croprect');
    if (!rect || !crop) return;
    Object.assign(rect.style, { left: `${crop.x}px`, top: `${crop.y}px`, width: `${crop.w}px`, height: `${crop.h}px` });
    const k = base().width / q('.gl-croplayer').clientWidth;
    q('.gl-cropsize').textContent = `${Math.round(crop.w * k)} × ${Math.round(crop.h * k)} px`;
  }
  function cropEvents(layer) {
    layer.onpointerdown = (e) => {
      e.preventDefault();
      layer.setPointerCapture(e.pointerId);
      const box = layer.getBoundingClientRect();
      const px = (ev) => Math.min(Math.max(0, ev.clientX - box.left), box.width);
      const py = (ev) => Math.min(Math.max(0, ev.clientY - box.top), box.height);
      const sx = px(e);
      const sy = py(e);
      const start = { ...crop };
      const handle = e.target.dataset.h;
      const inside = !handle && sx > crop.x && sx < crop.x + crop.w && sy > crop.y && sy < crop.y + crop.h;
      // pevný roh oproti ťahanému (pri novom výreze bod kliknutia)
      const ax = handle ? (handle.includes('w') ? start.x + start.w : start.x) : sx;
      const ay = handle ? (handle.includes('n') ? start.y + start.h : start.y) : sy;
      layer.onpointermove = (ev) => {
        const x = px(ev);
        const y = py(ev);
        if (inside) {
          crop.x = Math.min(Math.max(0, start.x + x - sx), box.width - crop.w);
          crop.y = Math.min(Math.max(0, start.y + y - sy), box.height - crop.h);
        } else {
          let w = Math.abs(x - ax);
          let h = Math.abs(y - ay);
          const r = ratio();
          if (r) {
            if (w / h > r) w = h * r;
            else h = w / r;
          }
          w = Math.max(8, w);
          h = Math.max(8, h);
          crop.x = Math.max(0, x < ax ? ax - w : ax);
          crop.y = Math.max(0, y < ay ? ay - h : ay);
          crop.w = Math.min(w, box.width - crop.x);
          crop.h = Math.min(h, box.height - crop.y);
        }
        paintCrop();
      };
      layer.onpointerup = () => (layer.onpointermove = layer.onpointerup = null);
    };
  }
  function applyCrop() {
    if (!crop) return;
    const layer = q('.gl-croplayer');
    const b = base();
    const k = b.width / layer.clientWidth;
    const r = { x: Math.round(crop.x * k), y: Math.round(crop.y * k), w: Math.max(1, Math.round(crop.w * k)), h: Math.max(1, Math.round(crop.h * k)) };
    stopCrop();
    edit({ crop: r });
  }

  // ---------- uloženie ----------
  async function encode(ext, quality = 0.92) {
    const s = base();
    if (ext === 'ico') return icoBlob(s);
    let c = s;
    if (ext === 'jpg' || ext === 'jpeg') {
      // JPG nemá priehľadnosť – biele pozadie namiesto čierneho
      c = document.createElement('canvas');
      c.width = s.width;
      c.height = s.height;
      const g = c.getContext('2d');
      g.fillStyle = '#fff';
      g.fillRect(0, 0, c.width, c.height);
      g.drawImage(s, 0, 0);
    }
    return blobOf(c, WRITABLE[ext], quality);
  }
  async function write(target, ext) {
    const blob = await encode(ext);
    await flux.writeImage(target, await b64(blob));
    return blob.size;
  }

  async function save() {
    if (!dirty()) return;
    const ext = extOf(path);
    if (!WRITABLE[ext]) return saveAs(); // SVG, GIF, ICO… sa nedajú prepísať – uložiť ako nový súbor
    try {
      size = await write(path, ext);
      canvas = null;
      undo = [];
      await show(idx);
      toast(t('Saved {name}.', { name: nameOf(path) }), 'ok');
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  async function saveAs() {
    if (!img) return;
    const ext = await confirmPalette(t('Convert “{name}” to…', { name: nameOf(path) }), [
      { label: t('PNG – sharp, keeps transparency'), value: 'png' },
      { label: t('JPG – small, good for photos'), value: 'jpg' },
      { label: t('WebP – smallest for websites'), value: 'webp' },
      { label: t('ICO – favicon for websites (16–256 px)'), value: 'ico' },
    ]);
    if (!ext) return;
    const stem = nameOf(path).replace(/\.[^.]+$/, '');
    const name = await promptPalette({ value: `${stem}.${ext}`, select: [0, stem.length], note: t('File name') });
    if (!name) return;
    const clean = /\.[a-z0-9]+$/i.test(name) ? name.trim() : `${name.trim()}.${ext}`;
    const target = dirOf(path) + sep(path) + clean;
    if (target !== path && list.includes(target)) {
      const ok = await confirmPalette(t('“{file}” already exists.', { file: clean }), [
        { label: t('Replace it'), value: true },
        { label: t('Cancel'), value: false },
      ]);
      if (!ok) return;
    }
    const from = path;
    try {
      const bytes = await write(target, extOf(clean) in WRITABLE || extOf(clean) === 'ico' ? extOf(clean) : ext);
      toast(t('Saved {name} ({size}).', { name: clean, size: kb(bytes) }), 'ok');
    } catch (err) {
      return toast(errorText(err), 'error');
    }
    list = await listFolder(target);
    thumbs = new Map();
    renderStrip();
    await show(list.indexOf(target));
    if (target !== from) afterConvert?.(from, target);
  }

  // ---------- pás náhľadov ----------
  function renderStrip() {
    const strip = q('.gl-strip');
    strip.hidden = list.length < 2;
    strip.innerHTML = list.map((p, i) => `<button class="gl-thumb" data-i="${i}" title=""><span></span></button>`).join('');
    strip.querySelectorAll('.gl-thumb').forEach((b, i) => (b.title = nameOf(list[i])));
    const io = new IntersectionObserver(
      (entries) => {
        for (const en of entries) {
          if (!en.isIntersecting) continue;
          io.unobserve(en.target);
          const p = list[Number(en.target.dataset.i)];
          if (thumbs.has(p)) continue;
          thumbs.set(p, true);
          flux
            .imageThumb(p)
            .then((url) => {
              if (url) en.target.firstChild.style.backgroundImage = `url("${url}")`;
              else en.target.firstChild.textContent = extOf(p).toUpperCase();
            })
            .catch(() => {});
        }
      },
      { root: strip, rootMargin: '200px' },
    );
    strip.querySelectorAll('.gl-thumb').forEach((b) => io.observe(b));
    strip.onwheel = (e) => {
      if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) {
        strip.scrollLeft += e.deltaY;
        e.preventDefault();
      }
    };
  }

  async function leave() {
    if (!dirty()) return true;
    const v = await confirmPalette(t('“{file}” has unsaved changes.', { file: nameOf(path) }), [
      { label: t('Discard changes'), value: 'discard' },
      { label: t('Keep editing'), value: 'keep' },
    ]);
    return v === 'discard';
  }
  async function go(i) {
    if (list.length < 2 || i === idx) return;
    if (!(await leave())) return;
    show(i);
  }
  async function close() {
    if (!(await leave())) return;
    el.hidden = true;
    canvas = img = null;
    undo = [];
    token++;
  }

  // klávesy, kým je prehliadač otvorený (volá globálny handler v app.js)
  function key(e) {
    const k = e.key;
    const ctrl = e.ctrlKey || e.metaKey;
    const done = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    if (k === 'Escape') return done(), crop ? stopCrop() : close();
    if (crop) {
      if (k === 'Enter') done(), applyCrop();
      return;
    }
    if (ctrl && k.toLowerCase() === 's') return done(), e.shiftKey ? saveAs() : save();
    if (ctrl && k.toLowerCase() === 'z') return done(), doUndo();
    if (ctrl || e.altKey) return;
    if (k === 'ArrowLeft') return done(), go(idx - 1);
    if (k === 'ArrowRight') return done(), go(idx + 1);
    if (k === 'Home') return done(), go(0);
    if (k === 'End') return done(), go(list.length - 1);
    if (k === 'r' || k === 'R') return done(), edit({ rotate: e.shiftKey ? 270 : 90 });
    if (k === 'c' || k === 'C') return done(), startCrop();
  }

  window.addEventListener('resize', () => {
    const layer = el.querySelector('.gl-croplayer');
    if (layer) stopCrop();
  });

  return { open, key, close, isOpen: () => !el.hidden };
}
