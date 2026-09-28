// Plynulé posúvanie so zotrvačnosťou pre celú aplikáciu (nastavenia, zoznamy, bočný panel, AI…).
// Editor má vlastné (inertiaScroll v app.js) – obe sa riadia tým istým nastavením.
// Koliesko posunie cieľ, obsah k nemu plynule dobehne.
const SKIP = '.monaco-editor, .xterm, iframe, select, input[type="range"], .no-smooth';

export function setupSmoothScroll(isOn) {
  const moving = new Map(); // prvok → { target, pos, frame, last }

  // najbližší rodič, ktorý sa dá posunúť týmto smerom
  function scrollerFor(el, dy) {
    for (let n = el; n && n !== document.documentElement; n = n.parentElement) {
      if (n.nodeType !== 1) continue;
      const oy = getComputedStyle(n).overflowY;
      if ((oy === 'auto' || oy === 'scroll') && n.scrollHeight > n.clientHeight + 1) {
        if ((dy > 0 && n.scrollTop + n.clientHeight < n.scrollHeight - 1) || (dy < 0 && n.scrollTop > 0)) return n;
      }
    }
    return null;
  }

  // Koliesko posúva cieľ, obsah sa k nemu plynule dobehne (1 - e^(-dt/90)) – bez skokov v rýchlosti.
  function step(el) {
    const m = moving.get(el);
    if (!m) return;
    const now = performance.now();
    const dt = Math.min(50, Math.max(0, now - m.last));
    m.last = now;
    // niekto posunul inak (posuvník, klávesnica) – nechať ho tak
    if (Math.abs(el.scrollTop - Math.round(m.pos)) > 2) return moving.delete(el);
    m.pos += (m.target - m.pos) * (1 - Math.exp(-dt / 90));
    if (Math.abs(m.target - m.pos) < 0.5) m.pos = m.target;
    el.scrollTop = m.pos;
    if (m.pos === m.target) return moving.delete(el);
    m.frame = requestAnimationFrame(() => step(el));
  }

  addEventListener(
    'wheel',
    (e) => {
      if (!isOn() || e.defaultPrevented || e.ctrlKey || e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
      if (e.target.closest?.(SKIP)) return;
      const el = scrollerFor(e.target, e.deltaY);
      if (!el) return;
      e.preventDefault();
      const dy = (e.deltaMode === 1 ? e.deltaY * 40 : e.deltaMode === 2 ? e.deltaY * el.clientHeight : e.deltaY) * 1.3;
      const max = el.scrollHeight - el.clientHeight;
      let m = moving.get(el);
      if (!m) {
        m = { target: el.scrollTop, pos: el.scrollTop, frame: 0, last: performance.now() };
        moving.set(el, m);
        m.frame = requestAnimationFrame(() => step(el));
      }
      // zmena smeru zastaví pohyb hneď
      if (Math.sign(dy) !== Math.sign(m.target - m.pos)) m.target = m.pos;
      m.target = Math.max(0, Math.min(max, m.target + dy));
    },
    { passive: false },
  );
}
