// Plynulé posúvanie so zotrvačnosťou pre celú aplikáciu (nastavenia, zoznamy, bočný panel, AI…).
// Editor má vlastné (inertiaScroll v app.js) – obe sa riadia tým istým nastavením.
// Koliesko pridá rýchlosť, obsah sa posúva a po pustení ešte chvíľu dokĺže.
const SKIP = '.monaco-editor, .xterm, iframe, select, input[type="range"], .no-smooth';

export function setupSmoothScroll(isOn) {
  const moving = new Map(); // prvok → { v, pos, frame, last }

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

  function step(el) {
    const m = moving.get(el);
    if (!m) return;
    const now = performance.now();
    const dt = Math.min(34, now - m.last) / 16.67;
    m.last = now;
    // niekto posunul inak (posuvník, klávesnica) – prevezmeme jeho polohu
    if (Math.abs(el.scrollTop - Math.round(m.pos)) > 2) m.pos = el.scrollTop;
    const max = el.scrollHeight - el.clientHeight;
    m.pos = Math.max(0, Math.min(max, m.pos + m.v * dt));
    el.scrollTop = m.pos;
    m.v *= Math.pow(0.88, dt);
    if (Math.abs(m.v) < 0.25 || m.pos <= 0 || m.pos >= max) {
      moving.delete(el);
      return;
    }
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
      const dy = e.deltaMode === 1 ? e.deltaY * 40 : e.deltaMode === 2 ? e.deltaY * el.clientHeight : e.deltaY;
      let m = moving.get(el);
      if (!m) {
        m = { v: 0, pos: el.scrollTop, frame: 0, last: performance.now() };
        moving.set(el, m);
        m.frame = requestAnimationFrame(() => step(el));
      }
      // zmena smeru zastaví pohyb hneď
      if (Math.sign(dy) !== Math.sign(m.v)) m.v = 0;
      m.v += dy * 0.14;
    },
    { passive: false },
  );
}
