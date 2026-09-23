// Preklady pre hlavný proces (dialógy, menu, chybové hlášky).
// Jazykové súbory sa sťahujú z verejného GitHub repozitára a ukladajú do userData/locales.
const { app, net } = require('electron');
const fs = require('node:fs');
const fsp = require('node:fs/promises');
const path = require('node:path');

const REPO = 'pantr1x/Flux';
const BRANCHES = ['main', 'claude/optimistic-darwin-5i7m9t'];

let dict = {};

function t(text, vars) {
  let out = dict[text] ?? text;
  if (vars) out = out.replace(/\{(\w+)\}/g, (_, k) => (vars[k] ?? `{${k}}`));
  return out;
}

const localeDir = () => path.join(app.getPath('userData'), 'locales');

async function fetchJson(file) {
  let lastErr;
  for (const branch of BRANCHES) {
    try {
      const res = await net.fetch(`https://raw.githubusercontent.com/${REPO}/${branch}/locales/${file}?t=${Date.now()}`);
      if (res.ok) return await res.json();
      lastErr = new Error(`HTTP ${res.status}`);
    } catch (err) {
      lastErr = err;
    }
  }
  throw lastErr || new Error('Download failed');
}

// Zoznam dostupných jazykov (locales/index.json).
async function listLanguages() {
  try {
    return await fetchJson('index.json');
  } catch {
    return [{ code: 'en', name: 'English' }];
  }
}

async function downloadLanguage(code) {
  const data = await fetchJson(`${code}.json`);
  await fsp.mkdir(localeDir(), { recursive: true });
  await fsp.writeFile(path.join(localeDir(), `${code}.json`), JSON.stringify(data));
  return data;
}

function loadCached(code) {
  if (!code || code === 'en') return {};
  try {
    return JSON.parse(fs.readFileSync(path.join(localeDir(), `${code}.json`), 'utf8'));
  } catch {
    return null;
  }
}

function setLanguage(code) {
  dict = loadCached(code) || {};
  return dict;
}

module.exports = { t, listLanguages, downloadLanguage, loadCached, setLanguage };
