const { app, BrowserWindow, ipcMain, dialog, shell, Menu, protocol, net, nativeTheme } = require('electron');
const fs = require('node:fs');
const fsp = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { findPython, probe } = require('./python');
const { Runner, commandFor } = require('./runner');
const { LiveServer } = require('./liveServer');
const { LanguageServer } = require('./lsp');

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
let settings = { recent: [], theme: 'dark', accent: 'violet', pythonOverrides: {} };
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
    backgroundColor: mica ? '#00000000' : dark ? '#26262c' : '#ececf1',
    // Windows 11: vlastná horná lišta s natívnymi tlačidlami a efekt Mica (priesvitné pozadie).
    titleBarStyle: process.platform === 'linux' ? 'default' : 'hidden',
    titleBarOverlay: isWin ? { color: '#00000000', symbolColor: dark ? '#e8e8ef' : '#1d1d24', height: 44 } : false,
    backgroundMaterial: mica ? 'acrylic' : undefined,
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
      buttons: ['Uložiť všetko', 'Neukladať', 'Zrušiť'],
      defaultId: 0,
      cancelId: 2,
      title: 'Neuložené zmeny',
      message: `Máš ${dirtyCount} neuložen${dirtyCount === 1 ? 'ý súbor' : 'é súbory'}.`,
      detail: 'Chceš ich pred zatvorením uložiť?',
    });
    if (choice === 2) return;
    allowClose = true;
    if (choice === 0) send('app:save-all-and-close');
    else win.close();
  });
  win.on('closed', () => {
    win = null;
  });
}

// ---------- pracovný priečinok ----------
const IGNORED_DIRS = new Set(['node_modules', '.git', '__pycache__', '.mypy_cache', '.pytest_cache', '.ruff_cache', '.idea', '.vs']);

function setWorkspace(dir) {
  workspace = dir;
  settings.recent = [dir, ...settings.recent.filter((d) => d !== dir)].slice(0, 8);
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
  if (!insideWorkspace(path.resolve(p))) throw new Error('Cesta je mimo otvoreného priečinka.');
  return path.resolve(p);
}

// ---------- IPC ----------
function registerIpc() {
  ipcMain.handle('app:init', () => ({
    platform: process.platform,
    mica,
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
    return settings;
  });
  ipcMain.on('app:dirty', (_e, count) => {
    dirtyCount = count;
  });
  ipcMain.on('app:close', () => win && win.close());

  ipcMain.handle('workspace:open-dialog', async () => {
    const r = await dialog.showOpenDialog(win, { properties: ['openDirectory', 'createDirectory'], title: 'Otvoriť priečinok' });
    if (r.canceled || !r.filePaths[0]) return null;
    setWorkspace(r.filePaths[0]);
    return r.filePaths[0];
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
    if (stat.size > 5 * 1024 * 1024) throw new Error('Súbor je príliš veľký (viac ako 5 MB).');
    const buf = await fsp.readFile(p);
    if (buf.subarray(0, 8000).includes(0)) throw new Error('Toto je binárny súbor – editor ho nevie zobraziť.');
    return buf.toString('utf8');
  });
  ipcMain.handle('fs:read-any', async (_e, file) => {
    // Na „Prejsť na definíciu“ do knižníc mimo projektu (napr. typeshed od Pyrightu) – iba na čítanie.
    const buf = await fsp.readFile(file);
    return buf.toString('utf8');
  });
  ipcMain.handle('fs:write', async (_e, file, content) => {
    await fsp.writeFile(guard(file), content, 'utf8');
    return true;
  });
  ipcMain.handle('fs:create', async (_e, target, isDir) => {
    const p = guard(target);
    if (fs.existsSync(p)) throw new Error('Taký súbor alebo priečinok už existuje.');
    if (isDir) await fsp.mkdir(p, { recursive: true });
    else {
      await fsp.mkdir(path.dirname(p), { recursive: true });
      await fsp.writeFile(p, '');
    }
    return p;
  });
  ipcMain.handle('fs:rename', async (_e, from, to) => {
    const a = guard(from);
    const b = guard(to);
    if (fs.existsSync(b)) throw new Error('Taký názov už existuje.');
    await fsp.rename(a, b);
    return b;
  });
  ipcMain.handle('fs:trash', async (_e, target) => {
    const p = guard(target);
    const { response } = await dialog.showMessageBox(win, {
      type: 'question',
      buttons: ['Presunúť do koša', 'Zrušiť'],
      defaultId: 0,
      cancelId: 1,
      message: `Zmazať „${path.basename(p)}“?`,
      detail: 'Položka sa presunie do koša, odkiaľ ju môžeš obnoviť.',
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
        { label: 'Nový súbor…', click: pick('new-file') },
        { label: 'Nový priečinok…', click: pick('new-folder') },
        { type: 'separator' },
      ];
      if (item && !item.dir) {
        template.unshift({ label: 'Spustiť', click: pick('run') }, { type: 'separator' });
      }
      if (item && item.path !== workspace) {
        template.push({ label: 'Premenovať…', click: pick('rename') }, { label: 'Zmazať', click: pick('delete') }, { type: 'separator' });
      }
      template.push(
        { label: 'Kopírovať cestu', click: pick('copy-path') },
        { label: isWin ? 'Zobraziť v Prieskumníkovi' : 'Zobraziť v priečinku', click: pick('reveal') },
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
      title: 'Vybrať Python interpreter',
      properties: ['openFile'],
      filters: isWin ? [{ name: 'Python', extensions: ['exe'] }] : [],
    });
    if (r.canceled || !r.filePaths[0]) return null;
    const info = await probe(r.filePaths[0]);
    if (!info) throw new Error('Vybraný súbor nie je funkčný Python.');
    if (workspace) {
      settings.pythonOverrides[workspace] = r.filePaths[0];
      saveSettings();
    }
    return { ...info, source: 'vybraný ručne' };
  });
  ipcMain.handle('python:reset', () => {
    if (workspace) delete settings.pythonOverrides[workspace];
    saveSettings();
  });

  // Spúšťanie
  ipcMain.handle('run:file', (_e, file, python, lang) => {
    const target = guard(file);
    const command = commandFor(target, python, lang);
    if (!command) return { ok: false, error: 'Tento typ súboru zatiaľ neviem spustiť.' };
    const ok = runner.start({ ...command, cwd: path.dirname(target), label: path.basename(target) });
    return { ok };
  });
  ipcMain.handle('run:pip', (_e, python, pkg) => {
    if (!/^[A-Za-z0-9._\-\[\]]+$/.test(pkg)) return { ok: false, error: 'Neplatný názov balíka.' };
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
    if (!workspace) throw new Error('Najprv otvor priečinok.');
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
