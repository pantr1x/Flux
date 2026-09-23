// „Minihra“: XP a úrovne projektu, séria dní, odznaky a malá oslava pri úspechu.
import { t } from './i18n.js';

export const ACHIEVEMENTS = [
  { id: 'first_run', icon: '🚀', name: 'Liftoff', desc: 'Run your first program' },
  { id: 'hello', icon: '👋', name: 'Hello, world', desc: 'A program finished without errors' },
  { id: 'web', icon: '🌐', name: "It's live!", desc: 'Open a website in Live Server' },
  { id: 'fixer', icon: '🩹', name: 'Bug squasher', desc: 'Fix every error in a file and save it' },
  { id: 'runs10', icon: '🔁', name: 'Getting warm', desc: 'Run programs 10 times' },
  { id: 'runs100', icon: '⚡', name: 'Speedrunner', desc: 'Run programs 100 times' },
  { id: 'lines100', icon: '✍️', name: 'First hundred', desc: 'Have 100 lines of code in a project' },
  { id: 'lines1000', icon: '📚', name: 'Thousand lines', desc: 'Have 1 000 lines of code in a project' },
  { id: 'hour', icon: '⏱️', name: 'In the zone', desc: 'Spend an hour in one project' },
  { id: 'streak3', icon: '🔥', name: 'On fire', desc: 'Code 3 days in a row' },
  { id: 'streak7', icon: '🏆', name: 'Unstoppable', desc: 'Code 7 days in a row' },
  { id: 'night', icon: '🦉', name: 'Night owl', desc: 'Run code after 11 PM' },
  { id: 'projects3', icon: '🗂️', name: 'Collector', desc: 'Create 3 projects' },
];

// XP projektu z času, riadkov a spustení.
export function xpOf(st = {}, pg = {}) {
  return Math.floor((st.time || 0) / 60) * 2 + Math.floor((st.lines || 0) / 5) + (pg.runs || 0) * 5 + (pg.okRuns || 0) * 3 + (pg.saves || 0);
}

// Úroveň L potrebuje spolu 40·L·(L−1) XP: 0, 80, 240, 480, 800…
const needFor = (level) => 40 * level * (level - 1);
export function levelOf(xp) {
  let level = 1;
  while (needFor(level + 1) <= xp) level++;
  const from = needFor(level);
  const to = needFor(level + 1);
  return { level, xp, into: xp - from, span: to - from, pct: Math.min(100, ((xp - from) / (to - from)) * 100) };
}

const today = () => new Date().toISOString().slice(0, 10);

// Počet dní po sebe (dnes alebo včera ako posledný deň).
export function streakOf(days = []) {
  const set = new Set(days);
  const d = new Date();
  if (!set.has(today())) d.setDate(d.getDate() - 1);
  let n = 0;
  while (set.has(d.toISOString().slice(0, 10))) {
    n++;
    d.setDate(d.getDate() - 1);
  }
  return n;
}

export function createGame({ getSettings, saveSettings, projectStats, celebrate }) {
  const game = () => {
    const g = getSettings().game || {};
    return { projects: g.projects || {}, days: g.days || [], achievements: g.achievements || {}, levels: g.levels || {}, created: g.created || 0 };
  };
  let busy = Promise.resolve();

  async function update(mutator, dir) {
    busy = busy.then(async () => {
      const g = game();
      if (!g.days.includes(today())) g.days = [...g.days.slice(-60), today()];
      mutator(g);
      const unlocked = [];
      const unlock = (id) => {
        if (!g.achievements[id]) {
          g.achievements[id] = today();
          unlocked.push(ACHIEVEMENTS.find((a) => a.id === id));
        }
      };
      const totalRuns = Object.values(g.projects).reduce((a, p) => a + (p.runs || 0), 0);
      if (totalRuns >= 1) unlock('first_run');
      if (totalRuns >= 10) unlock('runs10');
      if (totalRuns >= 100) unlock('runs100');
      if (Object.values(g.projects).some((p) => p.okRuns)) unlock('hello');
      if (Object.values(g.projects).some((p) => p.live)) unlock('web');
      if (Object.values(g.projects).some((p) => p.fixed)) unlock('fixer');
      if (g.created >= 3) unlock('projects3');
      const streak = streakOf(g.days);
      if (streak >= 3) unlock('streak3');
      if (streak >= 7) unlock('streak7');
      if (g.night) unlock('night');
      let levelUp = null;
      if (dir) {
        const st = await projectStats(dir);
        if (st.lines >= 100) unlock('lines100');
        if (st.lines >= 1000) unlock('lines1000');
        if (st.time >= 3600) unlock('hour');
        const lv = levelOf(xpOf(st, g.projects[dir])).level;
        if (lv > (g.levels[dir] || 1)) levelUp = lv;
        g.levels[dir] = Math.max(lv, g.levels[dir] || 1);
      }
      await saveSettings({ game: g });
      for (const a of unlocked) celebrate({ kind: 'achievement', title: t(a.name), text: t(a.desc), icon: a.icon });
      if (levelUp) celebrate({ kind: 'level', title: t('Level {n}!', { n: levelUp }), text: t('Your project leveled up. Keep going!'), icon: '⭐' });
    });
    return busy;
  }

  const bump = (dir, key, n = 1) =>
    update((g) => {
      if (!dir) return;
      const p = (g.projects[dir] = g.projects[dir] || {});
      p[key] = (p[key] || 0) + n;
      if (key === 'runs' && new Date().getHours() >= 23) g.night = true;
    }, dir);

  return {
    game,
    run: (dir) => bump(dir, 'runs'),
    okRun: (dir) => bump(dir, 'okRuns'),
    save: (dir) => bump(dir, 'saves'),
    fixed: (dir) => bump(dir, 'fixed'),
    live: (dir) => bump(dir, 'live'),
    projectCreated: () => update((g) => (g.created = (g.created || 0) + 1)),
    touch: () => update(() => {}),
  };
}

// Konfety – krátka oslava pri odznaku alebo novej úrovni.
export function confetti(originX = innerWidth / 2, originY = innerHeight / 3) {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const canvas = document.createElement('canvas');
  canvas.className = 'confetti';
  canvas.width = innerWidth * devicePixelRatio;
  canvas.height = innerHeight * devicePixelRatio;
  document.body.append(canvas);
  const ctx = canvas.getContext('2d');
  ctx.scale(devicePixelRatio, devicePixelRatio);
  const colors = ['#ffffff', '#8b7bff', '#3ecf8e', '#f5c542', '#ff6fb1', '#4f9dff'];
  const parts = Array.from({ length: 110 }, () => ({
    x: originX,
    y: originY,
    vx: (Math.random() - 0.5) * 16,
    vy: Math.random() * -14 - 4,
    size: 4 + Math.random() * 5,
    rot: Math.random() * Math.PI,
    vr: (Math.random() - 0.5) * 0.4,
    color: colors[(Math.random() * colors.length) | 0],
  }));
  const start = performance.now();
  const frame = (now) => {
    const k = (now - start) / 1400;
    ctx.clearRect(0, 0, innerWidth, innerHeight);
    for (const p of parts) {
      p.vy += 0.45;
      p.vx *= 0.985;
      p.x += p.vx;
      p.y += p.vy;
      p.rot += p.vr;
      ctx.save();
      ctx.globalAlpha = Math.max(0, 1 - k);
      ctx.translate(p.x, p.y);
      ctx.rotate(p.rot);
      ctx.fillStyle = p.color;
      ctx.fillRect(-p.size / 2, -p.size / 4, p.size, p.size / 2);
      ctx.restore();
    }
    if (k < 1) requestAnimationFrame(frame);
    else canvas.remove();
  };
  requestAnimationFrame(frame);
}
