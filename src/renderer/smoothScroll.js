// Plynulé posúvanie so zotrvačnosťou pre celú aplikáciu (nastavenia, zoznamy, bočný panel, AI…).
// Editor má vlastné (inertiaScroll v app.js) – obe sa riadia tým istým nastavením.
// Koliesko posunie cieľ, obsah k nemu ide ako tlmená pružina (springStep) – rýchlosť bez skokov.
// rovnaká pružina ako v editore (app.js by bol kruhový import)
function springStep(pos, vel, target, dt, w = 0.016) {
  const e = pos - target;
  const k = Math.exp(-w * dt);
  const tmp = (vel + w * e) * dt;
  return [target + (e + tmp) * k, (vel - w * tmp) * k];
}

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

  function step(el, now) {
    const m = moving.get(el);
    if (!m) return;
    // čas z requestAnimationFrame (zarovnaný s obrazovkou); nikdy záporný krok
    const dt = Math.min(50, Math.max(0, now - m.last));
    m.last = Math.max(m.last, now);
    // niekto posunul inak (posuvník, klávesnica) – nechať ho tak
    if (Math.abs(el.scrollTop - Math.round(m.pos)) > 2) return moving.delete(el);
    [m.pos, m.vel] = springStep(m.pos, m.vel, m.target, dt);
    m.pos = Math.max(0, Math.min(el.scrollHeight - el.clientHeight, m.pos));
    if (Math.abs(m.target - m.pos) < 0.5 && Math.abs(m.vel) < 0.02) m.pos = m.target;
    el.scrollTop = m.pos;
    if (m.pos === m.target) return moving.delete(el);
    m.frame = requestAnimationFrame((t) => step(el, t));
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
        m = { target: el.scrollTop, pos: el.scrollTop, vel: 0, frame: 0, last: performance.now() };
        moving.set(el, m);
        m.frame = requestAnimationFrame((t) => step(el, t));
      }
      // zmena smeru zastaví pohyb hneď
      if (Math.sign(dy) !== Math.sign(m.target - m.pos)) {
        m.target = m.pos;
        m.vel = 0;
      }
      m.target = Math.max(0, Math.min(max, m.target + dy));
    },
    { passive: false },
  );
}
