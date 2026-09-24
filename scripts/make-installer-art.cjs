// Vygeneruje obrázky pre inštalátor (NSIS potrebuje 24-bit BMP): build/installerSidebar.bmp (164×314)
// a build/installerHeader.bmp (150×57). Spustenie: npx electron scripts/make-installer-art.cjs
const { app, BrowserWindow } = require('electron');
const fs = require('node:fs');
const path = require('node:path');

const ICON = `data:image/png;base64,${fs.readFileSync(path.join(__dirname, '../build/icon.png')).toString('base64')}`;
const BASE = `*{margin:0;box-sizing:border-box}body{width:100vw;height:100vh;overflow:hidden;background:#101016;font-family:'Segoe UI',system-ui,sans-serif;color:#fff;position:relative}
.a{position:absolute;border-radius:50%;filter:blur(26px)}`;

const SIDEBAR = `<style>${BASE}
.logo{position:absolute;left:24px;top:34px;width:52px;height:52px;border-radius:14px;box-shadow:0 8px 24px rgba(0,0,0,.5)}
h1{position:absolute;left:24px;top:98px;font-size:34px;font-weight:800;letter-spacing:-.5px}
p{position:absolute;left:24px;top:142px;font-size:11px;color:rgba(255,255,255,.72);line-height:1.5}
.dots{position:absolute;inset:0;background-image:radial-gradient(rgba(255,255,255,.08) 1px,transparent 1px);background-size:10px 10px}
.v{position:absolute;left:24px;bottom:18px;font-size:10px;color:rgba(255,255,255,.45);letter-spacing:.08em}</style>
<div class="a" style="width:190px;height:190px;left:-60px;bottom:-40px;background:#5b3fd6;opacity:.75"></div>
<div class="a" style="width:150px;height:150px;right:-50px;bottom:70px;background:#1f7ae0;opacity:.6"></div>
<div class="a" style="width:120px;height:120px;left:40px;bottom:-60px;background:#18b89a;opacity:.45"></div>
<div class="dots"></div>
<img class="logo" src="${ICON}"><h1>Flux</h1><p>Code. Run. Create.<br>A small, modern editor<br>for Windows 11.</p><div class="v">FLUX · SETUP</div>`;

const HEADER = `<style>${BASE}
img{position:absolute;right:12px;top:10px;width:37px;height:37px;border-radius:10px}
b{position:absolute;right:58px;top:15px;font-size:20px;font-weight:800}</style>
<div class="a" style="width:90px;height:90px;right:-20px;top:-30px;background:#5b3fd6;opacity:.7"></div>
<div class="a" style="width:70px;height:70px;left:10px;bottom:-40px;background:#1f7ae0;opacity:.5"></div>
<img src="${ICON}"><b>Flux</b>`;

// BGRA bitmapa z Electronu → 24-bit BMP (riadky odspodu, zarovnané na 4 bajty).
function bmp(image) {
  const { width: w, height: h } = image.getSize();
  const src = image.toBitmap();
  const row = Math.ceil((w * 3) / 4) * 4;
  const out = Buffer.alloc(54 + row * h);
  out.write('BM', 0);
  out.writeUInt32LE(out.length, 2);
  out.writeUInt32LE(54, 10);
  out.writeUInt32LE(40, 14);
  out.writeInt32LE(w, 18);
  out.writeInt32LE(h, 22);
  out.writeUInt16LE(1, 26);
  out.writeUInt16LE(24, 28);
  out.writeUInt32LE(row * h, 34);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const s = (y * w + x) * 4;
      const d = 54 + (h - 1 - y) * row + x * 3;
      out[d] = src[s];
      out[d + 1] = src[s + 1];
      out[d + 2] = src[s + 2];
    }
  }
  return out;
}

async function render(html, width, height, file) {
  const win = new BrowserWindow({ width, height, show: false, useContentSize: true, frame: false, webPreferences: { offscreen: true } });
  const tmp = path.join(app.getPath('temp'), `flux-art-${width}x${height}.html`);
  fs.writeFileSync(tmp, `<!doctype html><meta charset="utf-8">${html}`);
  await win.loadFile(tmp);
  await new Promise((r) => setTimeout(r, 400));
  let img = await win.webContents.capturePage();
  if (img.getSize().width !== width) img = img.resize({ width, height, quality: 'best' });
  fs.writeFileSync(path.join(__dirname, '../build', file), bmp(img));
  fs.writeFileSync(path.join(__dirname, '../build', file.replace('.bmp', '.preview.png')), img.toPNG());
  win.destroy();
}

app.disableHardwareAcceleration();
app.on('window-all-closed', () => {}); // medzi obrázkami nekončiť
app.whenReady().then(async () => {
  await render(SIDEBAR, 164, 314, 'installerSidebar.bmp');
  await render(HEADER, 150, 57, 'installerHeader.bmp');
  console.log('ok');
  app.quit();
});
