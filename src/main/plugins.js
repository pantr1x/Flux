// Pluginy: katalóg je na GitHube (priečinok plugins/ v repozitári Flux), nainštalované sú v userData/plugins.
// Každý plugin má plugin.json (manifest), README.md, obrázky a JS súbor s funkciou activate(flux).
// Recenzie (1–5 hviezdičiek + komentár) = komentáre pod GitHub issue pluginu so značkou <!-- flux-review stars=N -->.
// Používateľ issues nevidí – vo Fluxe sú to hodnotenia a komentáre; čítať môže každý, písať po prihlásení cez GitHub.
const fs = require('node:fs');
const fsp = require('node:fs/promises');
const path = require('node:path');
const { app, net } = require('electron');
const { t } = require('./i18n');

const REPO = 'pantr1x/Flux';
const BRANCHES = ['main', 'claude/optimistic-darwin-5i7m9t'];
const ID = /^[a-z0-9][a-z0-9.-]{1,60}$/;

const pluginsDir = () => path.join(app.getPath('userData'), 'plugins');

// Pribalené pluginy: žiadne. Všetky pluginy sa sťahujú z GitHubu až keď ich v obchode nainštaluješ
// (predtým sa Error Lens, Bookmarks… inštalovali a zapínali samé). Mechanizmus ostáva pre prípad potreby.
const BUILTIN_IDS = [];
const BUILTIN_DIR = path.join(__dirname, '..', '..', 'plugins');

function createPlugins({ getSettings, saveSettings, githubApi }) {
  let branch = null; // vetva, na ktorej katalóg existuje
  let cache = { at: 0, data: null };

  // Na testovanie: FLUX_PLUGIN_REGISTRY=cesta k lokálnemu priečinku plugins/
  async function fetchText(rel) {
    if (process.env.FLUX_PLUGIN_REGISTRY) return fsp.readFile(path.join(process.env.FLUX_PLUGIN_REGISTRY, rel), 'utf8');
    const order = branch ? [branch] : BRANCHES;
    let last;
    for (const b of order) {
      try {
        const res = await net.fetch(`https://raw.githubusercontent.com/${REPO}/${b}/plugins/${rel}?t=${Date.now()}`);
        if (res.ok) {
          branch = b;
          return await res.text();
        }
        last = new Error(`HTTP ${res.status}`);
      } catch (err) {
        last = err;
      }
    }
    throw last || new Error('Download failed');
  }

  const assetUrl = (id, file) =>
    process.env.FLUX_PLUGIN_REGISTRY
      ? `app://flux/registry/${id}/${file}`
      : `https://raw.githubusercontent.com/${REPO}/${branch || BRANCHES[0]}/plugins/${id}/${file}`;

  // ---------- recenzie a komentáre ----------
  const MARK = /<!--\s*flux-review stars=(\d)\s*-->/;
  function parseComment(c) {
    const m = MARK.exec(c.body || '');
    return {
      id: c.id,
      user: c.user?.login || '',
      avatar: c.user?.avatar_url || '',
      date: c.created_at,
      edited: !!c.updated_at && c.updated_at !== c.created_at,
      stars: m ? Math.min(5, Math.max(1, Number(m[1]))) : 0,
      body: (c.body || '').replace(MARK, '').replace(/^\s*[★☆]{5}\s*/, '').trim(),
      url: c.html_url,
    };
  }
  // S prihlásením cez GitHub (5000 dopytov/h), inak bez neho (60/h – katalóg sa preto drží 10 min).
  async function getJson(pathname) {
    try {
      return await githubApi(pathname);
    } catch {}
    const res = await net.fetch(`https://api.github.com${pathname}`, { headers: { Accept: 'application/vnd.github+json', 'User-Agent': 'Flux' } });
    if (!res.ok) throw new Error(`GitHub ${res.status}`);
    return res.json();
  }
  async function comments(issue) {
    return (await getJson(`/repos/${REPO}/issues/${issue}/comments?per_page=100`)).map(parseComment);
  }
  // Každý má jedno hodnotenie – platí jeho posledná recenzia.
  function summary(items) {
    const byUser = new Map();
    for (const c of items) if (c.stars) byUser.set(c.user, c.stars);
    const all = [...byUser.values()];
    return { rating: all.length ? all.reduce((a, b) => a + b, 0) / all.length : 0, ratingCount: all.length, commentCount: items.filter((c) => c.body).length };
  }
  async function ratings(list) {
    await Promise.all(
      list.map(async (p) => {
        if (!p.issue) return;
        try {
          Object.assign(p, summary(await comments(p.issue)));
        } catch {}
      }),
    );
  }
  async function issueOf(id) {
    if (!cache.data) await registry().catch(() => {});
    return cache.data?.find((p) => p.id === id)?.issue;
  }
  const me = () => getSettings().github?.user?.login || '';

  // Detail pluginu: všetky recenzie a komentáre (najnovšie hore).
  async function reviews(id) {
    const issue = await issueOf(id);
    if (!issue) return { items: [], me: me(), ...summary([]) };
    const items = await comments(issue);
    return { items: items.reverse(), me: me(), ...summary(items) };
  }

  // Hodnotenie (stars 1–5, jedno na človeka – upraví sa) alebo obyčajný komentár (stars 0).
  async function review(id, stars, text) {
    const issue = await issueOf(id);
    if (!issue) throw new Error(t('This plugin has no reviews yet.'));
    const n = Math.max(0, Math.min(5, Math.round(Number(stars) || 0)));
    text = String(text || '').trim().slice(0, 4000);
    if (!n && !text) throw new Error(t('Pick stars or write a comment.'));
    const body = n ? `<!-- flux-review stars=${n} -->\n${'★'.repeat(n)}${'☆'.repeat(5 - n)}${text ? `\n\n${text}` : ''}` : text;
    const mine = n ? (await comments(issue)).filter((c) => c.stars && c.user === me()).pop() : null;
    if (mine) await githubApi(`/repos/${REPO}/issues/comments/${mine.id}`, { method: 'PATCH', body: { body } });
    else await githubApi(`/repos/${REPO}/issues/${issue}/comments`, { method: 'POST', body: { body } });
    cache.at = 0;
    return true;
  }

  async function deleteComment(commentId) {
    await githubApi(`/repos/${REPO}/issues/comments/${Number(commentId)}`, { method: 'DELETE' });
    cache.at = 0;
    return true;
  }

  async function registry(force = false) {
    if (!force && cache.data && Date.now() - cache.at < 10 * 60 * 1000) return withState(cache.data);
    const data = JSON.parse(await fetchText('index.json'));
    const list = (data.plugins || []).filter((p) => ID.test(p.id));
    for (const p of list) {
      p.iconUrl = p.icon ? assetUrl(p.id, p.icon) : '';
      p.screenshotUrls = (p.screenshots || []).map((s) => assetUrl(p.id, s));
    }
    await ratings(list);
    cache = { at: Date.now(), data: list };
    return withState(list);
  }

  function installedList() {
    const out = [];
    let names = [];
    try {
      names = fs.readdirSync(pluginsDir());
    } catch {}
    for (const n of names) {
      try {
        const m = JSON.parse(fs.readFileSync(path.join(pluginsDir(), n, 'plugin.json'), 'utf8'));
        out.push({ ...m, dir: path.join(pluginsDir(), n), local: !!m.local });
      } catch {}
    }
    return out;
  }

  const enabledMap = () => getSettings().plugins?.enabled || {};

  function builtinList() {
    const out = [];
    for (const id of BUILTIN_IDS) {
      try {
        const m = JSON.parse(fs.readFileSync(path.join(BUILTIN_DIR, id, 'plugin.json'), 'utf8'));
        out.push({ ...m, builtin: true, dir: path.join(BUILTIN_DIR, id) });
      } catch {}
    }
    return out;
  }
  function builtins() {
    const en = enabledMap();
    return builtinList().map((m) => ({ ...m, iconUrl: `app://flux/builtin/${m.id}/${m.icon || 'icon.svg'}`, enabled: en[m.id] !== false }));
  }

  function withState(list) {
    const inst = installedList();
    const en = enabledMap();
    return list.map((p) => {
      const i = inst.find((x) => x.id === p.id);
      if (BUILTIN_IDS.includes(p.id) && !i) return { ...p, installed: true, builtin: true, installedVersion: p.version, enabled: en[p.id] !== false, update: false };
      return { ...p, installed: !!i, installedVersion: i?.version || '', enabled: i ? en[p.id] !== false : false, update: !!i && i.version !== p.version };
    });
  }

  async function details(id) {
    if (!ID.test(id)) throw new Error('Invalid plugin id');
    const manifest = JSON.parse(await fetchText(`${id}/plugin.json`));
    let readme = '';
    try {
      readme = await fetchText(`${id}/${manifest.readme || 'README.md'}`);
    } catch {}
    return { ...manifest, readme, iconUrl: manifest.icon ? assetUrl(id, manifest.icon) : '', screenshotUrls: (manifest.screenshots || []).map((s) => assetUrl(id, s)) };
  }

  async function install(id) {
    if (!ID.test(id)) throw new Error('Invalid plugin id');
    const manifest = JSON.parse(await fetchText(`${id}/plugin.json`));
    const files = [...new Set([manifest.main || 'plugin.js', ...(manifest.files || [])])];
    const dir = path.join(pluginsDir(), id);
    const tmp = `${dir}.tmp`;
    await fsp.rm(tmp, { recursive: true, force: true });
    await fsp.mkdir(tmp, { recursive: true });
    for (const f of files) {
      if (f.includes('..') || path.isAbsolute(f)) continue;
      const text = await fetchText(`${id}/${f}`);
      await fsp.mkdir(path.dirname(path.join(tmp, f)), { recursive: true });
      await fsp.writeFile(path.join(tmp, f), text);
    }
    await fsp.writeFile(path.join(tmp, 'plugin.json'), JSON.stringify(manifest, null, 2));
    await fsp.rm(dir, { recursive: true, force: true });
    await fsp.rename(tmp, dir);
    setEnabled(id, true);
    return manifest;
  }

  // Vývoj: plugin z vlastného priečinka (skopíruje sa, tlačidlo „Reload“ ho načíta znova).
  async function installFromFolder(src) {
    const manifest = JSON.parse(await fsp.readFile(path.join(src, 'plugin.json'), 'utf8'));
    if (!ID.test(manifest.id || '')) throw new Error(t('plugin.json needs an "id" like "yourname.my-plugin".'));
    const dir = path.join(pluginsDir(), manifest.id);
    await fsp.rm(dir, { recursive: true, force: true });
    await fsp.cp(src, dir, { recursive: true, filter: (p) => !/[\\/](node_modules|\.git)([\\/]|$)/.test(p) });
    await fsp.writeFile(path.join(dir, 'plugin.json'), JSON.stringify({ ...manifest, local: true, source: src }, null, 2));
    setEnabled(manifest.id, true);
    return manifest;
  }

  async function uninstall(id) {
    if (!ID.test(id)) throw new Error('Invalid plugin id');
    await fsp.rm(path.join(pluginsDir(), id), { recursive: true, force: true });
    const s = getSettings();
    const en = { ...(s.plugins?.enabled || {}) };
    delete en[id];
    saveSettings({ plugins: { ...(s.plugins || {}), enabled: en } });
    return true;
  }

  function setEnabled(id, on) {
    const s = getSettings();
    saveSettings({ plugins: { ...(s.plugins || {}), enabled: { ...(s.plugins?.enabled || {}), [id]: !!on } } });
    return true;
  }

  // Čo sa má pri štarte načítať.
  function active() {
    const en = enabledMap();
    const inst = installedList();
    const own = inst.map((m) => ({ id: m.id, name: m.name, version: m.version, main: m.main || 'plugin.js', url: `app://flux/plugins/${m.id}/${m.main || 'plugin.js'}?v=${encodeURIComponent(m.version || '0')}-${Date.now()}` }));
    // Vstavané – ak nie je nainštalovaná vlastná (novšia) kópia z obchodu.
    const built = builtinList()
      .filter((m) => !inst.some((x) => x.id === m.id))
      .map((m) => ({ id: m.id, name: m.name, version: m.version, main: m.main || 'plugin.js', url: `app://flux/builtin/${m.id}/${m.main || 'plugin.js'}?v=${encodeURIComponent(m.version || '0')}` }));
    return [...own, ...built].filter((m) => en[m.id] !== false);
  }

  // Nová šablóna pluginu – rovno ako projekt vo Fluxe.
  async function scaffold(root, name) {
    const slug = (name || 'my-plugin').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'my-plugin';
    let dir = path.join(root, `flux-plugin-${slug}`);
    for (let i = 2; fs.existsSync(dir); i++) dir = path.join(root, `flux-plugin-${slug}-${i}`);
    await fsp.mkdir(dir, { recursive: true });
    const manifest = {
      id: `me.${slug}`,
      name: name || 'My plugin',
      publisher: 'me',
      version: '1.0.0',
      description: 'What does your plugin do? One sentence.',
      main: 'plugin.js',
      icon: 'icon.svg',
      screenshots: [],
      readme: 'README.md',
      tags: ['example'],
    };
    await fsp.writeFile(path.join(dir, 'plugin.json'), JSON.stringify(manifest, null, 2) + '\n');
    await fsp.writeFile(
      path.join(dir, 'plugin.js'),
      `// ${manifest.name} – a Flux plugin.
// activate() runs when Flux starts (or when you press Reload in Settings → Plugins).
// Everything you can use is on the \`flux\` object – see README.md.

export function activate(flux) {
  // A command in the command palette (Ctrl+Shift+P) with its own shortcut
  flux.commands.register('hello', 'Say hello', () => {
    flux.toast(\`Hello from ${manifest.name}! You are editing \${flux.activeFile()?.name || 'nothing'}.\`);
  }, { key: 'Ctrl+Alt+H' });

  // A small item in the status bar
  const item = flux.statusBar.add({ text: '👋 Hi', title: 'Click me', onClick: () => flux.commands.run('hello') });

  // React to saving a file
  flux.onSave((file) => item.set(\`💾 \${file.name}\`));
}

// Optional: clean up when the plugin is turned off
export function deactivate() {}
`,
    );
    await fsp.writeFile(
      path.join(dir, 'icon.svg'),
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="16" fill="#6b5cff"/><path d="M22 32h20M32 22v20" stroke="#fff" stroke-width="6" stroke-linecap="round"/></svg>\n`,
    );
    await fsp.writeFile(
      path.join(dir, 'README.md'),
      `# ${manifest.name}

${manifest.description}

## Try it
1. In Flux open **Settings → Plugins → Load from folder…** and pick this folder.
2. Change \`plugin.js\`, then press **Reload** next to the plugin.

## Publish it
Fork https://github.com/${REPO}, copy this folder to \`plugins/${manifest.id}/\`, add an entry to \`plugins/index.json\` and open a pull request.
The full guide is in docs/PLUGINS.md.
`,
    );
    return dir;
  }

  return { builtins, builtinDir: BUILTIN_DIR, registry, details, install, installFromFolder, uninstall, setEnabled, active, reviews, review, deleteComment, scaffold, installedList, dir: pluginsDir };
}

module.exports = { createPlugins, pluginsDir };
