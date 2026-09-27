// Preklady pre hlavný proces (dialógy, menu, chybové hlášky).
// Jazyky sú pribalené vo Fluxe (locales/ v inštalátore) – prepnutie je okamžité aj bez internetu.
// Novšie preklady sa sťahujú z verejného GitHub repozitára na pozadí a ukladajú do userData/locales.
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
const BUNDLED_DIR = path.join(__dirname, '..', '..', 'locales');
const readJson = (file) => {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return null;
  }
};

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

// Zoznam jazykov: pribalený hneď, z GitHubu len keď pribudol nový jazyk.
async function listLanguages() {
  const bundled = readJson(path.join(BUNDLED_DIR, 'index.json')) || [{ code: 'en', name: 'English', native: 'English' }];
  try {
    // pomalé pripojenie nesmie zdržať výber jazyka – po 2,5 s ostane pribalený zoznam
    const remote = await Promise.race([fetchJson('index.json'), new Promise((_, no) => setTimeout(() => no(new Error('timeout')), 2500))]);
    return [...bundled, ...remote.filter((l) => !bundled.some((b) => b.code === l.code))];
  } catch {
    return bundled;
  }
}

async function downloadLanguage(code) {
  const data = await fetchJson(`${code}.json`);
  await fsp.mkdir(localeDir(), { recursive: true });
  await fsp.writeFile(path.join(localeDir(), `${code}.json`), JSON.stringify(data));
  return data;
}

// Pribalený preklad + novšie texty stiahnuté z GitHubu (kľúče sú anglické texty, takže sa dajú zlúčiť).
function loadCached(code) {
  if (!code || code === 'en') return {};
  const bundled = readJson(path.join(BUNDLED_DIR, `${code}.json`));
  const fresh = readJson(path.join(localeDir(), `${code}.json`));
  if (!bundled && !fresh) return null;
  return { ...(bundled || {}), ...(fresh || {}) };
}

function setLanguage(code) {
  dict = loadCached(code) || {};
  return dict;
}

module.exports = { t, listLanguages, downloadLanguage, loadCached, setLanguage };
