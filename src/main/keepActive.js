// Acrylic/Mica, aj keď okno nie je aktívne. Windows inak okno po kliknutí do iného programu zosivie –
// DWM kreslí pozadie podľa toho, či je rám okna „aktívny“ (WM_NCACTIVATE). Po strate fokusu si preto
// Flux sám pošle WM_NCACTIVATE(TRUE): pozadie ostane priesvitné, fokus aj klávesnica ostanú v druhom programe.
// Správu posiela malý pomocník (C#), ktorý sa raz skompiluje cez csc.exe z .NET Frameworku vo Windows –
// žiadny natívny modul, nič nebeží na pozadí (spustí sa, pošle správu a skončí).
const fs = require('node:fs');
const path = require('node:path');
const { execFile } = require('node:child_process');

const VERSION = '1';
const SOURCE = `using System;
using System.Runtime.InteropServices;
static class FluxKeepActive {
  [DllImport("user32.dll")] static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] static extern bool IsWindow(IntPtr h);
  static void Main(string[] args) {
    if (args.Length < 1) return;
    IntPtr h = new IntPtr(long.Parse(args[0]));
    if (IsWindow(h)) SendMessage(h, 0x0086, new IntPtr(1), IntPtr.Zero); // WM_NCACTIVATE, TRUE
  }
}
`;

let dir = null;
let exe = null;
let building = null;

const cscPath = () => {
  const win = process.env.WINDIR || 'C:\\Windows';
  for (const fw of ['Framework64', 'Framework']) {
    const p = path.join(win, 'Microsoft.NET', fw, 'v4.0.30319', 'csc.exe');
    if (fs.existsSync(p)) return p;
  }
  return null;
};

// Pripraví pomocníka (raz; potom len overí, že je). Vráti cestu alebo null.
function ensure(userData) {
  if (process.platform !== 'win32') return Promise.resolve(null);
  if (exe) return Promise.resolve(exe);
  if (building) return building;
  dir = path.join(userData, 'bin');
  const out = path.join(dir, 'flux-keepactive.exe');
  const stamp = path.join(dir, 'flux-keepactive.ver');
  building = new Promise((resolve) => {
    try {
      if (fs.existsSync(out) && fs.readFileSync(stamp, 'utf8') === VERSION) return resolve((exe = out));
    } catch {}
    const csc = cscPath();
    if (!csc) return resolve(null);
    try {
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, 'flux-keepactive.cs'), SOURCE);
    } catch {
      return resolve(null);
    }
    execFile(csc, ['/nologo', '/target:winexe', '/optimize+', `/out:${out}`, path.join(dir, 'flux-keepactive.cs')], { windowsHide: true, timeout: 30000 }, (err) => {
      if (err || !fs.existsSync(out)) return resolve(null);
      try {
        fs.writeFileSync(stamp, VERSION);
      } catch {}
      resolve((exe = out));
    });
  }).finally(() => (building = null));
  return building;
}

// Pošle oknu „si aktívne“ (len pre vzhľad – fokus sa nemení).
function poke(win) {
  if (!exe || !win || win.isDestroyed()) return;
  try {
    const buf = win.getNativeWindowHandle();
    const hwnd = buf.length >= 8 ? buf.readBigUInt64LE(0) : BigInt(buf.readUInt32LE(0));
    execFile(exe, [hwnd.toString()], { windowsHide: true, timeout: 2000 }, () => {});
  } catch {}
}

module.exports = { ensure, poke };
