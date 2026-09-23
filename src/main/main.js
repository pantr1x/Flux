const { app, BrowserWindow, ipcMain, dialog, shell, Menu, protocol, net, nativeTheme, screen } = require('electron');
const fs = require('node:fs');
const fsp = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { findPython, probe } = require('./python');
const { Runner, commandFor } = require('./runner');
const { LiveServer } = require('./liveServer');
const { LanguageServer } = require('./lsp');
const i18n = require('./i18n');
const { t } = i18n;

const isWin = process.platform === 'win32';
// Priesvitné pozadie (Acrylic – rozmazaná tapeta ako v Zen Browseri) je len vo Windows 11 22H2+.
const mica = isWin && Number(os.release().split('.')[2]) >= 22621;
const RENDERER_DIR = path.join(__dirname, '..', '..', 'dist', 'renderer');
const ICON = path.join(__dirname, '..', '..', 'build', 'icon.png');

// Vlastný protokol app:// – editor a jeho web workery sa načítajú spoľahlivejšie než cez file://.
protocol.registerSchemesAsPrivileged([
  { scheme: 'app', privileges: { standard: true, secure: true, supportFetchAPI: true } },
]);

// ---------- nastavenia ----------
const settingsFile = () => path.join(app.getPath('userData'), 'settings.json');
let settings = { recent: [], theme: 'dark', pythonOverrides: {} };
function loadSettings() {
  try {
    settings = { ...settings, ...JSON.parse(fs.readFileSync(settingsFile(), 'utf8')) };
  } catch {}
}
function saveSettings() {
  try {
    fs.mkdirSync(path.dirname(settingsFile()), { recursive: true });
    fs.writeFileSync(settingsFile(), JSON.stringify(settings, null, 2));
  } catch {}
}

// ---------- priesvitnosť ----------
// „wallpaper“: Flux si sám nakreslí rozmazanú tapetu – vyzerá priesvitne aj keď okno nie je aktívne
// (Acrylic/Mica od Windows vtedy okno vždy zosivia).
function wallpaperPath() {
  if (process.env.FLUX_WALLPAPER) return process.env.FLUX_WALLPAPER;
  if (!isWin || !process.env.APPDATA) return null;
  const p = path.join(process.env.APPDATA, 'Microsoft', 'Windows', 'Themes', 'TranscodedWallpaper');
  return fs.existsSync(p) ? p : null;
}

function materialMode() {
  let m = settings.material || 'wallpaper';
  if (settings.translucent === false) m = 'none';
  if (m === 'wallpaper' && !wallpaperPath()) m = mica ? 'acrylic' : 'none';
  if ((m === 'acrylic' || m === 'mica') && !mica) m = 'none';
  return m;
}

function applyMaterial() {
  if (!win) return;
  const m = materialMode();
  const dark = settings.theme !== 'light';
  if (mica) win.setBackgroundMaterial(m === 'acrylic' || m === 'mica' ? m : 'none');
  win.setBackgroundColor(m === 'acrylic' || m === 'mica' ? '#00000000' : dark ? '#26262c' : '#ececf1');
  send('app:material', m);
}

function sendBounds() {
  if (!win) return;
  const b = win.getContentBounds();
  const d = screen.getDisplayMatching(b).bounds;
  send('win:bounds', { x: b.x - d.x, y: b.y - d.y, dw: d.width, dh: d.height });
}

// ---------- okno ----------
let win = null;
let workspace = null;
let workspaceWatcher = null;
let dirtyCount = 0;
let allowClose = false;

const send = (channel, payload) => {
  if (win && !win.isDestroyed()) win.webContents.send(channel, payload);
};

const runner = new Runner(send);
const live = new LiveServer((entry) => send('live:log', entry));
const lsp = new LanguageServer(
  (msg) => send('lsp:message', msg),
  (code) => send('lsp:exit', code),
);

function createWindow() {
  const dark = settings.theme !== 'light';
  win = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 760,
    minHeight: 480,
    show: false,
    title: 'Flux',
    backgroundColor: ['acrylic', 'mica'].includes(materialMode()) ? '#00000000' : dark ? '#26262c' : '#ececf1',
    // Windows 11: vlastná horná lišta s natívnymi tlačidlami a efekt Mica (priesvitné pozadie).
    titleBarStyle: process.platform === 'linux' ? 'default' : 'hidden',
    titleBarOverlay: isWin ? { color: '#00000000', symbolColor: dark ? '#e8e8ef' : '#1d1d24', height: 44 } : false,
    backgroundMaterial: ['acrylic', 'mica'].includes(materialMode()) ? materialMode() : undefined,
    icon: fs.existsSync(ICON) ? ICON : undefined,
    webPreferences: {
      preload: path.join(__dirname, '..', 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      spellcheck: false,
    },
  });
  win.removeMenu();
  win.once('ready-to-show', () => win.show());
  win.loadURL('app://flux/index.html');

  // Odkazy z editora/preview otvárať v systémovom prehliadači.
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (/^https?:/.test(url)) shell.openExternal(url);
    return { action: 'deny' };
  });
  win.webContents.on('will-navigate', (e) => e.preventDefault());

  win.on('close', (e) => {
    if (allowClose || dirtyCount === 0) return;
    e.preventDefault();
    const choice = dialog.showMessageBoxSync(win, {
      type: 'warning',
      buttons: [t('Save all'), t("Don't save"), t('Cancel')],
      defaultId: 0,
      cancelId: 2,
      title: t('Unsaved changes'),
      message: dirtyCount === 1 ? t('You have 1 unsaved file.') : t('You have {n} unsaved files.', { n: dirtyCount }),
      detail: t('Do you want to save before closing?'),
    });
    if (choice === 2) return;
    allowClose = true;
    if (choice === 0) send('app:save-all-and-close');
    else win.close();
  });
  win.on('closed', () => {
    win = null;
  });
  for (const ev of ['move', 'resize', 'maximize', 'unmaximize', 'restore']) win.on(ev, sendBounds);
  win.webContents.on('did-finish-load', sendBounds);
}

// ---------- pracovný priečinok ----------
const IGNORED_DIRS = new Set(['node_modules', '.git', '__pycache__', '.mypy_cache', '.pytest_cache', '.ruff_cache', '.idea', '.vs']);

function setWorkspace(dir) {
  workspace = dir;
  settings.recent = [dir, ...settings.recent.filter((d) => d !== dir)].slice(0, 8);
  // Zoznam projektov má stále poradie – nový sa pridá na koniec, otvorenie ho nepresúva.
  ensureProjects();
  if (!settings.projects.some((p) => p.dir === dir)) settings.projects.push({ dir, pinned: false });
  settings.lastFolder = dir;
  saveSettings();
  if (workspaceWatcher) workspaceWatcher.close();
  workspaceWatcher = null;
  let timer = null;
  try {
    workspaceWatcher = fs.watch(dir, { recursive: true }, (_e, file) => {
      if (file && file.split(/[\\/]/).some((p) => IGNORED_DIRS.has(p))) return;
      clearTimeout(timer);
      timer = setTimeout(() => send('fs:changed'), 150);
    });
    workspaceWatcher.on('error', () => {});
  } catch {}
  live.stop();
  send('live:stopped');
}

function insideWorkspace(p) {
  if (!workspace) return false;
  const rel = path.relative(workspace, p);
  return rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel));
}

function guard(p) {
  if (!insideWorkspace(path.resolve(p))) throw new Error(t('The path is outside the open folder.'));
  return path.resolve(p);
}

// ---------- IPC ----------
function registerIpc() {
  ipcMain.handle('i18n:list', () => i18n.listLanguages());
  ipcMain.handle('i18n:use', async (_e, code) => {
    // Použije jazyk; ak ešte nie je stiahnutý, stiahne ho z GitHubu.
    let data = code === 'en' ? {} : i18n.loadCached(code);
    if (!data) data = await i18n.downloadLanguage(code);
    settings.language = code;
    saveSettings();
    i18n.setLanguage(code);
    return data;
  });
  ipcMain.handle('i18n:current', () => i18n.loadCached(settings.language || 'en') || {});
  ipcMain.handle('app:init', () => ({
    platform: process.platform,
    mica,
    material: materialMode(),
    hasWallpaper: !!wallpaperPath(),
    settings,
    version: app.getVersion(),
    lastFolder: settings.lastFolder && fs.existsSync(settings.lastFolder) ? settings.lastFolder : null,
  }));
  ipcMain.handle('app:set-settings', (_e, patch) => {
    settings = { ...settings, ...patch };
    saveSettings();
    if (win && isWin && patch.theme) {
      win.setTitleBarOverlay({ color: '#00000000', symbolColor: patch.theme === 'light' ? '#1d1d24' : '#e8e8ef', height: 44 });
    }
    if (patch.theme) nativeTheme.themeSource = patch.theme;
    if ('material' in patch || 'translucent' in patch || 'theme' in patch) applyMaterial();
    return settings;
  });
  ipcMain.on('app:dirty', (_e, count) => {
    dirtyCount = count;
  });
  ipcMain.on('app:close', () => win && win.close());

  ipcMain.handle('workspace:open-dialog', async () => {
    const r = await dialog.showOpenDialog(win, { properties: ['openDirectory', 'createDirectory'], title: t('Open folder') });
    if (r.canceled || !r.filePaths[0]) return null;
    setWorkspace(r.filePaths[0]);
    return r.filePaths[0];
  });
  ipcMain.handle('workspace:projects', async () => {
    ensureProjects();
    const list = settings.projects.filter((p) => fs.existsSync(p.dir));
    const ordered = [...list.filter((p) => p.pinned), ...list.filter((p) => !p.pinned)];
    return Promise.all(ordered.map(async (p) => ({ dir: p.dir, pinned: !!p.pinned, name: path.basename(p.dir), kind: await projectKind(p.dir) })));
  });
  ipcMain.handle('workspace:forget', (_e, dir) => {
    ensureProjects();
    settings.recent = settings.recent.filter((d) => d !== dir);
    settings.projects = settings.projects.filter((p) => p.dir !== dir);
    saveSettings();
    return true;
  });
  ipcMain.handle('project:pin', (_e, dir, pinned) => {
    ensureProjects();
    const p = settings.projects.find((x) => x.dir === dir);
    if (p) p.pinned = !!pinned;
    saveSettings();
    return true;
  });
  // Premenovanie projektu = premenovanie priečinka na disku.
  ipcMain.handle('project:rename', async (_e, dir, newName) => {
    if (!/^[^\\/:*?"<>|]+$/.test(newName) || newName.trim() !== newName) throw new Error(t('Invalid folder name.'));
    const target = path.join(path.dirname(dir), newName);
    if (fs.existsSync(target)) throw new Error(t('A folder with this name already exists.'));
    const wasOpen = workspace === dir;
    if (wasOpen) {
      if (workspaceWatcher) workspaceWatcher.close();
      workspaceWatcher = null;
      lsp.stop();
      await live.stop();
      runner.stop();
    }
    await fsp.rename(dir, target);
    const swap = (d) => (d === dir ? target : d);
    ensureProjects();
    settings.projects = settings.projects.map((p) => ({ ...p, dir: swap(p.dir) }));
    settings.recent = settings.recent.map(swap);
    if (settings.lastFolder === dir) settings.lastFolder = target;
    for (const key of ['accents', 'pythonOverrides']) {
      if (settings[key]?.[dir] !== undefined) {
        settings[key][target] = settings[key][dir];
        delete settings[key][dir];
      }
    }
    saveSettings();
    return { dir: target, wasOpen };
  });
  ipcMain.handle('wallpaper:get', async () => {
    const p = wallpaperPath();
    if (!p) return null;
    try {
      const buf = await fsp.readFile(p);
      const mime = buf[0] === 0x89 ? 'image/png' : 'image/jpeg';
      return `data:${mime};base64,${buf.toString('base64')}`;
    } catch {
      return null;
    }
  });
  ipcMain.on('win:bounds?', sendBounds);

  // Čas strávený v projekte (počíta sa len keď je okno aktívne a niečo robíš).
  ipcMain.on('project:add-time', (_e, dir, secs) => {
    if (!dir || !(secs > 0)) return;
    settings.projectTime = settings.projectTime || {};
    settings.projectTime[dir] = (settings.projectTime[dir] || 0) + Math.min(secs, 120);
    saveSettings();
  });
  ipcMain.handle('project:stats', (_e, dir) => projectStats(dir));
  // Staršie verzie používali „Flux projekty“ – ak priečinok existuje, ostaneme pri ňom.
  const defaultRoot = () => {
    const legacy = path.join(app.getPath('documents'), 'Flux projekty');
    return fs.existsSync(legacy) ? legacy : path.join(app.getPath('documents'), 'Flux Projects');
  };
  ipcMain.handle('project:root', () => defaultRoot());
  ipcMain.handle('project:create', async (_e, name, root) => {
    if (!/^[^\\/:*?"<>|]+$/.test(name) || name.trim() !== name) throw new Error(t('Invalid project name.'));
    const base = root || defaultRoot();
    const dir = path.join(base, name);
    if (fs.existsSync(dir)) throw new Error(t('This project already exists.'));
    await fsp.mkdir(dir, { recursive: true });
    return dir;
  });
  ipcMain.handle('project:choose-root', async () => {
    const r = await dialog.showOpenDialog(win, { properties: ['openDirectory', 'createDirectory'], title: t('Where to save the new project') });
    return r.canceled ? null : r.filePaths[0];
  });
  // Obrázok ako data: URL pre náhľad (Glance).
  ipcMain.handle('fs:read-image', async (_e, file) => {
    const p = guard(file);
    const stat = await fsp.stat(p);
    if (stat.size > 25 * 1024 * 1024) throw new Error(t('The image is too large.'));
    const ext = path.extname(p).slice(1).toLowerCase();
    const mime = { svg: 'image/svg+xml', jpg: 'image/jpeg', jpeg: 'image/jpeg', ico: 'image/x-icon' }[ext] || `image/${ext}`;
    return { url: `data:${mime};base64,${(await fsp.readFile(p)).toString('base64')}`, size: stat.size };
  });
  ipcMain.handle('workspace:open', (_e, dir) => {
    if (!fs.existsSync(dir)) {
      settings.recent = settings.recent.filter((d) => d !== dir);
      saveSettings();
      return null;
    }
    setWorkspace(dir);
    return dir;
  });

  ipcMain.handle('fs:list', async (_e, dir) => {
    const entries = await fsp.readdir(guard(dir), { withFileTypes: true });
    return entries
      .filter((e) => !IGNORED_DIRS.has(e.name))
      .map((e) => ({ name: e.name, path: path.join(dir, e.name), dir: e.isDirectory() }))
      .sort((a, b) => b.dir - a.dir || a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' }));
  });
  ipcMain.handle('fs:list-all', async () => {
    // Zoznam všetkých súborov pre rýchle otvorenie (Ctrl+P).
    const out = [];
    const walk = async (dir, depth) => {
      if (depth > 12 || out.length > 5000) return;
      let entries = [];
      try {
        entries = await fsp.readdir(dir, { withFileTypes: true });
      } catch {
        return;
      }
      for (const e of entries) {
        if (IGNORED_DIRS.has(e.name) || e.name === '.venv' || e.name === 'venv') continue;
        const p = path.join(dir, e.name);
        if (e.isDirectory()) await walk(p, depth + 1);
        else out.push(p);
      }
    };
    if (workspace) await walk(workspace, 0);
    return out;
  });
  ipcMain.handle('fs:read', async (_e, file) => {
    const p = guard(file);
    const stat = await fsp.stat(p);
    if (stat.size > 5 * 1024 * 1024) throw new Error(t('The file is too large (over 5 MB).'));
    const buf = await fsp.readFile(p);
    if (buf.subarray(0, 8000).includes(0)) throw new Error(t('This is a binary file – the editor cannot show it.'));
    return buf.toString('utf8');
  });
  ipcMain.handle('fs:read-any', async (_e, file) => {
    // Na „Prejsť na definíciu“ do knižníc mimo projektu (napr. typeshed od Pyrightu) – iba na čítanie.
    const buf = await fsp.readFile(file);
    return buf.toString('utf8');
  });
  ipcMain.handle('fs:write', async (_e, file, content) => {
    staleStats(file);
    await fsp.writeFile(guard(file), content, 'utf8');
    return true;
  });
  ipcMain.handle('fs:create', async (_e, target, isDir) => {
    staleStats(target);
    const p = guard(target);
    if (fs.existsSync(p)) throw new Error(t('A file or folder with this name already exists.'));
    if (isDir) await fsp.mkdir(p, { recursive: true });
    else {
      await fsp.mkdir(path.dirname(p), { recursive: true });
      await fsp.writeFile(p, '');
    }
    return p;
  });
  ipcMain.handle('fs:rename', async (_e, from, to) => {
    staleStats(from);
    const a = guard(from);
    const b = guard(to);
    if (fs.existsSync(b)) throw new Error(t('This name already exists.'));
    await fsp.rename(a, b);
    return b;
  });
  ipcMain.handle('fs:trash', async (_e, target) => {
    staleStats(target);
    const p = guard(target);
    const { response } = await dialog.showMessageBox(win, {
      type: 'question',
      buttons: [t('Move to Recycle Bin'), t('Cancel')],
      defaultId: 0,
      cancelId: 1,
      message: t('Delete “{name}”?', { name: path.basename(p) }),
      detail: t('It will be moved to the Recycle Bin, where you can restore it.'),
    });
    if (response !== 0) return false;
    await shell.trashItem(p);
    return true;
  });
  ipcMain.handle('fs:reveal', (_e, target) => shell.showItemInFolder(guard(target)));

  ipcMain.handle('menu:tree', (_e, item) => {
    return new Promise((resolve) => {
      let picked = null;
      const pick = (id) => () => (picked = id);
      const template = [
        { label: t('New file…'), click: pick('new-file') },
        { label: t('New folder…'), click: pick('new-folder') },
        { type: 'separator' },
      ];
      if (item && !item.dir) {
        template.unshift({ label: t('Run'), click: pick('run') }, { type: 'separator' });
      }
      if (item && item.path !== workspace) {
        template.push({ label: t('Rename…'), click: pick('rename') }, { label: t('Delete'), click: pick('delete') }, { type: 'separator' });
      }
      template.push(
        { label: t('Copy path'), click: pick('copy-path') },
        { label: isWin ? t('Show in File Explorer') : t('Show in folder'), click: pick('reveal') },
      );
      Menu.buildFromTemplate(template).popup({ window: win, callback: () => resolve(picked) });
    });
  });

  // Python
  ipcMain.handle('python:find', async () => {
    const override = workspace ? settings.pythonOverrides[workspace] : null;
    return findPython(workspace, override);
  });
  ipcMain.handle('python:choose', async () => {
    const r = await dialog.showOpenDialog(win, {
      title: t('Select Python interpreter'),
      properties: ['openFile'],
      filters: isWin ? [{ name: 'Python', extensions: ['exe'] }] : [],
    });
    if (r.canceled || !r.filePaths[0]) return null;
    const info = await probe(r.filePaths[0]);
    if (!info) throw new Error(t('The selected file is not a working Python.'));
    if (workspace) {
      settings.pythonOverrides[workspace] = r.filePaths[0];
      saveSettings();
    }
    return { ...info, source: t('chosen manually') };
  });
  ipcMain.handle('python:reset', () => {
    if (workspace) delete settings.pythonOverrides[workspace];
    saveSettings();
  });

  // Spúšťanie
  ipcMain.handle('run:file', (_e, file, python, lang) => {
    const target = guard(file);
    const command = commandFor(target, python, lang);
    if (!command) return { ok: false, error: t("Can't run this type of file yet.") };
    const ok = runner.start({ ...command, cwd: path.dirname(target), label: path.basename(target) });
    return { ok };
  });
  ipcMain.handle('run:pip', (_e, python, pkg) => {
    if (!/^[A-Za-z0-9._\-\[\]]+$/.test(pkg)) return { ok: false, error: t('Invalid package name.') };
    const ok = runner.start({ cmd: python, args: ['-m', 'pip', 'install', pkg], cwd: workspace || undefined, label: `pip install ${pkg}` });
    return { ok };
  });
  ipcMain.handle('run:create-venv', (_e, python) => {
    if (!workspace) return { ok: false };
    const ok = runner.start({ cmd: python, args: ['-m', 'venv', '.venv'], cwd: workspace, label: 'python -m venv .venv' });
    return { ok };
  });
  ipcMain.on('run:input', (_e, text) => runner.input(text));
  ipcMain.on('run:stop', () => runner.stop());
  ipcMain.on('run:resize', (_e, cols, rows) => runner.resize(cols, rows));

  // Live Server
  ipcMain.handle('live:start', async () => {
    if (!workspace) throw new Error(t('Open a folder first.'));
    return live.start(workspace);
  });
  ipcMain.handle('live:stop', async () => {
    await live.stop();
    return true;
  });
  ipcMain.handle('shell:open-external', (_e, url) => {
    if (/^https?:\/\//.test(url)) shell.openExternal(url);
  });

  // Python autocomplete (Pyright)
  ipcMain.handle('lsp:start', () => {
    lsp.start(workspace || app.getPath('home'));
    return true;
  });
  ipcMain.on('lsp:send', (_e, msg) => lsp.send(msg));
}

// Neočakávaná chyba v pozadí (napr. ukončený podproces) nesmie ukázať chybové okno – len sa zapíše.
process.on('uncaughtException', (err) => console.error('[flux]', err));

// ---------- projekty (nedávne priečinky) ----------
function ensureProjects() {
  if (!Array.isArray(settings.projects)) {
    settings.projects = [...settings.recent].reverse().map((dir) => ({ dir, pinned: false }));
  }
}

// Druh projektu podľa súborov v priečinku – na ikonu v zozname projektov.
async function projectKind(dir) {
  let py = 0;
  let web = 0;
  const scan = async (d, depth) => {
    let entries = [];
    try {
      entries = await fsp.readdir(d, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      if (IGNORED_DIRS.has(e.name) || e.name.startsWith('.') || e.name === 'venv') continue;
      if (e.isDirectory()) {
        if (depth < 1) await scan(path.join(d, e.name), depth + 1);
      } else if (/\.pyw?$/i.test(e.name)) py++;
      else if (/\.(html?|css)$/i.test(e.name)) web++;
    }
  };
  await scan(dir, 0);
  if (!py && !web) return 'folder';
  return py >= web ? 'python' : 'web';
}

// Štatistiky projektu: súbory, riadky, znaky, čas.
const statsCache = new Map();
// Po zmene súboru zahodíme štatistiky projektov, ktoré ho obsahujú.
function staleStats(file) {
  for (const dir of statsCache.keys()) if (String(file).startsWith(dir)) statsCache.delete(dir);
}
const TEXT_EXT = /\.(py|pyw|pyi|html?|css|scss|less|js|mjs|cjs|jsx|ts|tsx|json|md|txt|csv|xml|svg|yml|yaml|toml|ini|cfg|bat|cmd|ps1|sh|c|h|cpp|hpp|cs|java|go|rs|php|rb|lua|sql)$/i;
async function projectStats(dir) {
  const cached = statsCache.get(dir);
  if (cached && Date.now() - cached.at < 20000) return { ...cached.data, time: settings.projectTime?.[dir] || 0 };
  let files = 0;
  let lines = 0;
  let chars = 0;
  let lastModified = 0;
  const kinds = {};
  const walk = async (d, depth) => {
    if (depth > 8 || files > 5000) return;
    let entries = [];
    try {
      entries = await fsp.readdir(d, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      if (IGNORED_DIRS.has(e.name) || e.name.startsWith('.') || e.name === 'venv') continue;
      const p = path.join(d, e.name);
      if (e.isDirectory()) {
        await walk(p, depth + 1);
        continue;
      }
      files++;
      const ext = path.extname(e.name).slice(1).toLowerCase() || '—';
      kinds[ext] = (kinds[ext] || 0) + 1;
      try {
        const st = await fsp.stat(p);
        lastModified = Math.max(lastModified, st.mtimeMs);
        if (TEXT_EXT.test(e.name) && st.size < 2 * 1024 * 1024) {
          const text = await fsp.readFile(p, 'utf8');
          chars += text.length;
          lines += text ? text.split('\n').length : 0;
        }
      } catch {}
    }
  };
  await walk(dir, 0);
  const data = { files, lines, chars, lastModified, kinds };
  statsCache.set(dir, { at: Date.now(), data });
  return { ...data, time: settings.projectTime?.[dir] || 0 };
}

// ---------- štart ----------
// Iba jedno okno aplikácie – druhé spustenie len prenesie existujúce okno dopredu.
if (!app.requestSingleInstanceLock()) app.quit();
app.on('second-instance', () => {
  if (!win) return;
  if (win.isMinimized()) win.restore();
  win.focus();
});

app.whenReady().then(() => {
  loadSettings();
  i18n.setLanguage(settings.language || 'en');
  nativeTheme.themeSource = settings.theme === 'light' ? 'light' : 'dark';
  protocol.handle('app', (req) => {
    const { pathname } = new URL(req.url);
    const file = path.normalize(path.join(RENDERER_DIR, decodeURIComponent(pathname)));
    if (!file.startsWith(RENDERER_DIR)) return new Response('Forbidden', { status: 403 });
    return net.fetch(pathToFileURL(file).toString());
  });
  registerIpc();
  createWindow();
});

app.on('window-all-closed', () => {
  runner.stop();
  lsp.stop();
  live.stop();
  app.quit();
});
