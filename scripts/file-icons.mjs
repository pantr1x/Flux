// Ikony súborov pre Prieskumník (build/file-icons/*.ico): list papiera s farebným pásikom a príponou.
// Súbory, ktoré otvára Flux, tak nemajú ikonu aplikácie, ale vyzerajú ako dokument daného typu.
// Spustenie (len keď treba ikony zmeniť, výsledok je v gite): node scripts/file-icons.mjs
// Potrebuje Playwright (Chromium vykreslí SVG do PNG); ICO obsahuje PNG obrázky vo viacerých veľkostiach.
import { mkdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { execSync } from 'node:child_process';

const require = createRequire(import.meta.url);
let pw;
try {
  pw = require('playwright');
} catch {
  pw = require(`${execSync('npm root -g').toString().trim()}/playwright`);
}

// prípona → [text na pásiku, farba]
const C = {
  py: '#3776ab', js: '#e3b72b', ts: '#3178c6', web: '#e44d26', css: '#2965f1', data: '#6b7280', md: '#4b5563',
  c: '#5c6bc0', cpp: '#00599c', cs: '#68217a', java: '#e76f00', go: '#00add8', rs: '#b7410e', rb: '#cc342d',
  php: '#777bb4', lua: '#2c2d72', sh: '#2e7d32', sql: '#c2185b', txt: '#8a8a96', pl: '#39457e',
};
const TYPES = {
  txt: ['TXT', C.txt], md: ['MD', C.md], markdown: ['MD', C.md], log: ['LOG', C.txt], csv: ['CSV', '#1d7a46'],
  json: ['JSON', C.data], xml: ['XML', C.data], yml: ['YML', C.data], yaml: ['YAML', C.data], toml: ['TOML', C.data],
  ini: ['INI', C.data], cfg: ['CFG', C.data], py: ['PY', C.py], pyw: ['PYW', C.py], js: ['JS', C.js], mjs: ['MJS', C.js],
  cjs: ['CJS', C.js], ts: ['TS', C.ts], jsx: ['JSX', '#149eca'], tsx: ['TSX', C.ts], html: ['HTML', C.web],
  htm: ['HTM', C.web], css: ['CSS', C.css], scss: ['SCSS', '#c6538c'], java: ['JAVA', C.java], c: ['C', C.c],
  h: ['H', C.c], cpp: ['C++', C.cpp], hpp: ['HPP', C.cpp], cs: ['C#', C.cs], go: ['GO', C.go], rs: ['RS', C.rs],
  rb: ['RB', C.rb], php: ['PHP', C.php], lua: ['LUA', C.lua], sh: ['SH', C.sh], bat: ['BAT', C.sh], ps1: ['PS1', '#012456'],
  sql: ['SQL', C.sql], pl: ['PL', C.pl],
  file: ['', '#8b7bff'], // neznáma prípona (Flux.File)
};

const svg = (label, color, size) => {
  const small = size <= 24;
  const font = label.length >= 4 ? 44 : label.length === 3 ? 54 : 64;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 256 256">
  <defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#e9e9ef"/></linearGradient></defs>
  <path d="M52 12h108l64 64v156a12 12 0 0 1-12 12H52a12 12 0 0 1-12-12V24a12 12 0 0 1 12-12z" fill="url(#g)" stroke="#b9b9c6" stroke-width="${small ? 10 : 4}"/>
  <path d="M160 12v52a12 12 0 0 0 12 12h52z" fill="#d6d6e0"/>
  ${
    label
      ? `<rect x="${small ? 20 : 26}" y="${small ? 132 : 150}" width="${small ? 216 : 170}" height="${small ? 104 : 70}" rx="${small ? 18 : 12}" fill="${color}"/>
  <text x="${small ? 128 : 111}" y="${small ? 206 : 200}" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-weight="800" font-size="${small ? Math.min(92, font * 1.6) : font}" fill="#fff" ${label.length >= 4 && small ? 'textLength="200" lengthAdjust="spacingAndGlyphs"' : ''}>${label}</text>`
      : ''
  }
  <g fill="none" stroke="${label ? '#5a5a68' : color}" stroke-width="${small ? 20 : 12}" stroke-linecap="round" stroke-linejoin="round" transform="translate(0 ${label ? (small ? -18 : -8) : 30})">
    <path d="M100 76 78 98l22 22"/><path d="M150 76l22 22-22 22"/>${small ? '' : '<path d="M136 66l-22 64"/>'}
  </g>
</svg>`;
};

// ICO s PNG obrázkami (Windows Vista+).
function ico(pngs) {
  const head = Buffer.alloc(6 + 16 * pngs.length);
  head.writeUInt16LE(0, 0);
  head.writeUInt16LE(1, 2);
  head.writeUInt16LE(pngs.length, 4);
  let offset = head.length;
  pngs.forEach(({ size, data }, i) => {
    const o = 6 + i * 16;
    head.writeUInt8(size >= 256 ? 0 : size, o);
    head.writeUInt8(size >= 256 ? 0 : size, o + 1);
    head.writeUInt16LE(1, o + 4);
    head.writeUInt16LE(32, o + 6);
    head.writeUInt32LE(data.length, o + 8);
    head.writeUInt32LE(offset, o + 12);
    offset += data.length;
  });
  return Buffer.concat([head, ...pngs.map((p) => p.data)]);
}

const SIZES = [16, 24, 32, 48, 64, 256];
const browser = await pw.chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
const page = await browser.newPage();
mkdirSync('build/file-icons', { recursive: true });
for (const [ext, [label, color]] of Object.entries(TYPES)) {
  const pngs = [];
  for (const size of SIZES) {
    await page.setViewportSize({ width: size, height: size });
    await page.setContent(`<html><body style="margin:0;background:transparent">${svg(label, color, size)}</body></html>`);
    pngs.push({ size, data: await page.locator('svg').screenshot({ omitBackground: true }) });
  }
  writeFileSync(`build/file-icons/${ext}.ico`, ico(pngs));
}
await browser.close();
console.log(`${Object.keys(TYPES).length} ikon → build/file-icons`);
