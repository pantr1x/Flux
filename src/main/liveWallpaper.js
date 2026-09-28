// Živé tapety: ak beží Lively Wallpaper alebo Wallpaper Engine, tapeta Windows (TranscodedWallpaper)
// je stará – Flux preto zistí, čo majú tieto programy práve nastavené, a použije to isté (video, GIF,
// obrázok; pri scénach a webových tapetách ich náhľad). Len čítanie súborov s nastaveniami, nič sa nespúšťa.
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const VIDEO = new Set(['.mp4', '.webm', '.mov', '.m4v', '.mkv', '.avi', '.wmv']);
const IMAGE = new Set(['.jpg', '.jpeg', '.png', '.bmp', '.webp']);

const readJson = (p) => {
  try {
    return JSON.parse(fs.readFileSync(p, 'utf8').replace(/^\uFEFF/, ''));
  } catch {
    return null;
  }
};
const exists = (p) => {
  try {
    return !!p && fs.statSync(p).isFile();
  } catch {
    return false;
  }
};
const kindOf = (file) => {
  const ext = path.extname(file).toLowerCase();
  if (VIDEO.has(ext)) return 'video';
  if (ext === '.gif') return 'gif';
  if (IMAGE.has(ext)) return 'image';
  return null;
};
// Všetky reťazce v JSON-e (štruktúra nastavení sa medzi verziami mení – hľadáme podľa obsahu).
function strings(v, out = []) {
  if (typeof v === 'string') out.push(v);
  else if (v && typeof v === 'object') for (const x of Object.values(v)) strings(x, out);
  return out;
}

// ---------- Lively Wallpaper ----------
function livelyDirs() {
  const local = process.env.LOCALAPPDATA;
  if (!local) return [];
  const dirs = [path.join(local, 'Lively Wallpaper')];
  // verzia z Microsoft Store
  try {
    for (const d of fs.readdirSync(path.join(local, 'Packages'))) {
      if (/LivelyWallpaper/i.test(d)) dirs.push(path.join(local, 'Packages', d, 'LocalCache', 'Local', 'Lively Wallpaper'));
    }
  } catch {}
  return dirs.filter((d) => fs.existsSync(d));
}

function lively() {
  for (const dir of livelyDirs()) {
    const layout = readJson(path.join(dir, 'WallpaperLayout.json'));
    if (!layout) continue;
    // Priečinky tapiet, ktoré sú práve na obrazovkách (prvá = hlavný monitor).
    const folders = strings(layout).filter((s) => /[\\/]/.test(s) && fs.existsSync(path.join(s, 'LivelyInfo.json')));
    for (const folder of folders) {
      const info = readJson(path.join(folder, 'LivelyInfo.json'));
      if (!info) continue;
      const pick = (f) => (f ? (path.isAbsolute(f) ? f : path.join(folder, f)) : null);
      const main = pick(info.FileName);
      if (exists(main) && kindOf(main)) return { source: 'Lively Wallpaper', title: info.Title || '', file: main, kind: kindOf(main) };
      // web, aplikácia, stream… → náhľad
      for (const f of [info.Preview, info.Thumbnail].map(pick)) if (exists(f) && kindOf(f)) return { source: 'Lively Wallpaper', title: info.Title || '', file: f, kind: kindOf(f), preview: true };
    }
  }
  return null;
}

// ---------- Wallpaper Engine (Steam) ----------
function steamRoots() {
  const roots = [];
  if (process.env.FLUX_STEAM_PATH) roots.push(process.env.FLUX_STEAM_PATH);
  if (process.platform === 'win32') {
    try {
      const out = execFileSync('reg', ['query', 'HKCU\\Software\\Valve\\Steam', '/v', 'SteamPath'], { encoding: 'utf8', timeout: 3000, windowsHide: true });
      const m = out.match(/SteamPath\s+REG_SZ\s+(.+)/);
      if (m) roots.push(m[1].trim());
    } catch {}
    roots.push('C:\\Program Files (x86)\\Steam', 'C:\\Program Files\\Steam');
  }
  // ďalšie knižnice Steamu (iné disky)
  for (const r of [...roots]) {
    try {
      const vdf = fs.readFileSync(path.join(r, 'steamapps', 'libraryfolders.vdf'), 'utf8');
      for (const m of vdf.matchAll(/"path"\s+"([^"]+)"/g)) roots.push(m[1].replace(/\\\\/g, '\\'));
    } catch {}
  }
  return [...new Set(roots.map((r) => path.normalize(r)))];
}

function wallpaperEngine() {
  for (const root of steamRoots()) {
    const we = path.join(root, 'steamapps', 'common', 'wallpaper_engine');
    const cfg = readJson(path.join(we, 'config.json'));
    if (!cfg) continue;
    // Vybrané tapety sú cesty k project.json (v „selectedwallpapers“ pre každý monitor).
    const projects = strings(cfg).filter((s) => /project\.json$/i.test(s) && exists(s));
    for (const pj of projects) {
      const info = readJson(pj);
      if (!info) continue;
      const folder = path.dirname(pj);
      const main = info.file ? path.join(folder, info.file) : null;
      if (String(info.type).toLowerCase() === 'video' && exists(main) && kindOf(main)) return { source: 'Wallpaper Engine', title: info.title || '', file: main, kind: kindOf(main) };
      // scéna / web / aplikácia → náhľad (často animovaný GIF)
      const prev = info.preview ? path.join(folder, info.preview) : null;
      if (exists(prev) && kindOf(prev)) return { source: 'Wallpaper Engine', title: info.title || '', file: prev, kind: kindOf(prev), preview: true };
    }
  }
  return null;
}

// Beží program? Keď je zavretý, na ploche je zas obyčajná tapeta Windows.
function running() {
  if (process.platform !== 'win32') return { lively: true, we: true }; // test mimo Windows (FLUX_STEAM_PATH, LOCALAPPDATA)
  try {
    const out = execFileSync('tasklist', ['/FO', 'CSV', '/NH'], { encoding: 'utf8', timeout: 4000, windowsHide: true }).toLowerCase();
    return { lively: /"lively(\.ui\.winui)?\.exe"/.test(out), we: /"wallpaper(32|64)\.exe"/.test(out) };
  } catch {
    return { lively: false, we: false };
  }
}

// Výsledok sa krátko pamätá – volá sa pri každom návrate do okna.
let cache = { at: 0, value: null };
function detect() {
  if (Date.now() - cache.at < 10000) return cache.value;
  let value = null;
  try {
    const run = running();
    value = (run.lively && lively()) || (run.we && wallpaperEngine()) || null;
  } catch {}
  cache = { at: Date.now(), value };
  return value;
}

module.exports = { detect };
