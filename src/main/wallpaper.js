// Tapeta plochy na Linuxe – Flux ju nakreslí rozmazanú za oknom (ako na Windowse, material „wallpaper“).
// Každé prostredie ju ukladá inde: KDE Plasma, GNOME/Cinnamon/MATE, XFCE, Hyprland (hyprpaper, swww), feh, nitrogen.
// Výsledok sa pamätá 10 s (keď sa nič nenašlo, 60 s) – volá sa pri každom pohybe okna.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { fileURLToPath } = require('node:url');

const IMG = /\.(jpe?g|png|webp|bmp)$/i;
const home = os.homedir();
const read = (p) => {
  try {
    return fs.readFileSync(p, 'utf8');
  } catch {
    return '';
  }
};
const run = (cmd, args) => {
  try {
    return execFileSync(cmd, args, { encoding: 'utf8', timeout: 1500, stdio: ['ignore', 'pipe', 'ignore'] }).trim();
  } catch {
    return '';
  }
};
// file:///… alebo cesta (aj ~/…) → existujúci obrázok; priečinok = balík tapiet KDE (contents/images/WxH.jpg)
function toImage(value) {
  let p = String(value || '')
    .trim()
    .replace(/^['"]|['"]$/g, '');
  if (!p) return null;
  try {
    if (p.startsWith('file://')) p = fileURLToPath(p);
  } catch {
    return null;
  }
  p = p.replace(/^~(?=\/)/, home);
  try {
    const st = fs.statSync(p);
    if (st.isFile()) return IMG.test(p) ? p : null;
    if (st.isDirectory()) return fromPackage(p);
  } catch {}
  return null;
}
// balík tapiet: najväčší obrázok v contents/images (názvy ako 5120x2880.png)
function fromPackage(dir) {
  for (const sub of ['contents/images', 'contents/images_dark', '.']) {
    let files = [];
    try {
      files = fs.readdirSync(path.join(dir, sub)).filter((f) => IMG.test(f));
    } catch {
      continue;
    }
    if (!files.length) continue;
    const px = (f) => {
      const m = f.match(/(\d+)x(\d+)/);
      return m ? Number(m[1]) * Number(m[2]) : 0;
    };
    files.sort((a, b) => px(b) - px(a));
    return path.join(dir, sub, files[0]);
  }
  return null;
}

const sources = {
  kde() {
    const cfg = read(path.join(home, '.config/plasma-org.kde.plasma.desktop-appletsrc'));
    if (!cfg) return null;
    // prvá plocha s obrázkom: [Containments][N][Wallpaper][org.kde.image][General] → Image=
    let inImage = false;
    for (const line of cfg.split('\n')) {
      if (line.startsWith('[')) inImage = /\[Wallpaper\]\[org\.kde\.image\]\[General\]$/.test(line.trim());
      else if (inImage && line.startsWith('Image=')) {
        const img = toImage(line.slice(6));
        if (img) return img;
      }
    }
    return /KDE/i.test(process.env.XDG_CURRENT_DESKTOP || '') ? fromPackage('/usr/share/wallpapers/Next') : null;
  },
  gnome() {
    const dark = /dark/.test(run('gsettings', ['get', 'org.gnome.desktop.interface', 'color-scheme']));
    for (const [schema, key] of [
      ['org.gnome.desktop.background', dark ? 'picture-uri-dark' : 'picture-uri'],
      ['org.gnome.desktop.background', 'picture-uri'],
      ['org.cinnamon.desktop.background', 'picture-uri'],
      ['org.mate.background', 'picture-filename'],
    ]) {
      const img = toImage(run('gsettings', ['get', schema, key]));
      if (img) return img;
    }
    return null;
  },
  xfce() {
    const prop = run('xfconf-query', ['-c', 'xfce4-desktop', '-l'])
      .split('\n')
      .find((l) => /\/last-image$/.test(l));
    return prop ? toImage(run('xfconf-query', ['-c', 'xfce4-desktop', '-p', prop])) : null;
  },
  swww() {
    return toImage(run('swww', ['query']).match(/image: (.+)$/m)?.[1]);
  },
  hyprpaper() {
    const cfg = read(path.join(home, '.config/hypr/hyprpaper.conf'));
    for (const m of cfg.matchAll(/^\s*(?:wallpaper\s*=\s*[^,\n]*,|path\s*=)\s*(.+)$/gm)) {
      const img = toImage(m[1]);
      if (img) return img;
    }
    return null;
  },
  feh() {
    const m = read(path.join(home, '.fehbg')).match(/'([^']+)'\s*$/m);
    return m ? toImage(m[1]) : null;
  },
  nitrogen() {
    return toImage(read(path.join(home, '.config/nitrogen/bg-saved.cfg')).match(/^file=(.+)$/m)?.[1]);
  },
};

let cache = { at: 0, path: null };
function linuxWallpaper() {
  if (Date.now() - cache.at < (cache.path ? 10000 : 60000)) return cache.path;
  const desk = (process.env.XDG_CURRENT_DESKTOP || '').toLowerCase();
  // najprv zdroj pre aktuálne prostredie, potom ostatné
  const first = /kde/.test(desk) ? 'kde' : /gnome|cinnamon|mate|budgie|unity/.test(desk) ? 'gnome' : /xfce/.test(desk) ? 'xfce' : /hyprland/.test(desk) ? 'swww' : null;
  const order = [first, ...Object.keys(sources).filter((k) => k !== first)].filter(Boolean);
  let found = null;
  for (const k of order) {
    try {
      found = sources[k]();
    } catch {}
    if (found) break;
  }
  cache = { at: Date.now(), path: found };
  return found;
}

module.exports = { linuxWallpaper };
