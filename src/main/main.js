const { app, BrowserWindow, ipcMain, dialog, shell, Menu, protocol, net, nativeTheme, screen, nativeImage } = require('electron');
const fs = require('node:fs');
const fsp = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { findPython, probe } = require('./python');
const { Runner, commandFor, toolchainFor } = require('./runner');
const toolchains = require('./toolchains');
const { createAI } = require('./ai');
const { createGitHub } = require('./github');
const { createPlugins, pluginsDir } = require('./plugins');
const { createUpdater } = require('./updater');
const { createMcpServer } = require('./mcpServer');
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
  // Vlastný obrázok pozadia (Nastavenia → Personalize) má prednosť.
  if (settings.bgImage && fs.existsSync(settings.bgImage)) return settings.bgImage;
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
const knownTools = new Set(); // jazyky, o ktorých už vieme, že sú nainštalované
// Čo AI smie vidieť a meniť v otvorenom projekte (len v rámci priečinka projektu).
async function projectTool(name, input = {}) {
  if (name === 'has') return !!workspace;
  if (!workspace) return { error: 'No project is open.' };
  const all = { ...(settings.projectMeta || {}) };
  const meta = { ...(all[workspace] || {}) };
  const save = () => {
    all[workspace] = meta;
    settings.projectMeta = all;
    saveSettings();
  };
  if (name === 'flux_project_info') {
    const files = [];
    const walk = async (d, depth) => {
      if (depth > 6 || files.length > 400) return;
      let entries = [];
      try {
        entries = await fsp.readdir(d, { withFileTypes: true });
      } catch {
        return;
      }
      for (const e of entries) {
        if (IGNORED_DIRS.has(e.name) || e.name.startsWith('.') || e.name === 'venv') continue;
        const p = path.join(d, e.name);
        if (e.isDirectory()) await walk(p, depth + 1);
        else files.push(path.relative(workspace, p).split(path.sep).join('/'));
      }
    };
    await walk(workspace, 0);
    return {
      result: {
        name: path.basename(workspace),
        folder: workspace,
        type: await projectKind(workspace),
        description: meta.description || '',
        todos: (meta.todos || []).map((x) => ({ id: x.id, text: x.text, done: !!x.done })),
        files,
      },
    };
  }
  if (name === 'flux_read_file') {
    const file = path.resolve(workspace, input.path);
    if (!insideWorkspace(file)) return { error: 'That file is outside the project.' };
    try {
      const st = await fsp.stat(file);
      if (st.size > 400 * 1024) return { error: 'The file is too large to read (over 400 KB).' };
      return { result: await fsp.readFile(file, 'utf8') };
    } catch {
      return { error: `File not found: ${input.path}` };
    }
  }
  if (name === 'flux_add_todos') {
    const now = Date.now();
    const added = input.tasks.map((text, i) => ({ id: now + i, text: String(text).slice(0, 200), done: false }));
    meta.todos = [...(meta.todos || []), ...added];
    save();
    return { result: { added: added.map((x) => ({ id: x.id, text: x.text })) }, changed: true };
  }
  if (name === 'flux_update_todo') {
    const todos = meta.todos || [];
    const item = todos.find((x) => x.id === input.id);
    if (!item) return { error: `No to-do with id ${input.id}.` };
    if (input.remove) meta.todos = todos.filter((x) => x !== item);
    else {
      if (typeof input.done === 'boolean') item.done = input.done;
      if (typeof input.text === 'string' && input.text.trim()) item.text = input.text.trim().slice(0, 200);
      meta.todos = todos;
    }
    save();
    return { result: 'ok', changed: true };
  }
  if (name === 'flux_set_description') {
    meta.description = String(input.description).trim().slice(0, 160);
    save();
    return { result: 'ok', changed: true };
  }
  return { error: `Unknown tool ${name}` };
}

// ---------- MCP server: Flux pre iné AI aplikácie (Claude Desktop, Claude Code, Cursor…) ----------
const MCP_TOOLS = [
  { name: 'flux_list_projects', description: 'List the projects in Flux (name, folder, main language, languages, whether it is a GitHub repository) and which one is open.', inputSchema: { type: 'object', properties: {} } },
  { name: 'flux_open_project', description: 'Open a project in Flux by its folder (from flux_list_projects). The other tools then work with this project.', inputSchema: { type: 'object', properties: { folder: { type: 'string' } }, required: ['folder'] } },
  { name: 'flux_project_info', description: 'Get the open Flux project: name, folder, type, short description, to-do list (with ids) and its files.', inputSchema: { type: 'object', properties: {} } },
  { name: 'flux_read_file', description: 'Read a text file of the open project. Path is relative to the project folder, e.g. "index.html" or "src/main.py".', inputSchema: { type: 'object', properties: { path: { type: 'string' } }, required: ['path'] } },
  { name: 'flux_write_file', description: 'Create or overwrite a text file in the open project with the full new content. Flux shows the change right away.', inputSchema: { type: 'object', properties: { path: { type: 'string' }, content: { type: 'string' } }, required: ['path', 'content'] } },
  { name: 'flux_open_file', description: 'Open a file of the project in the Flux editor so the user sees it.', inputSchema: { type: 'object', properties: { path: { type: 'string' } }, required: ['path'] } },
  { name: 'flux_add_todos', description: "Add tasks to the project's to-do list (shown on the project page in Flux).", inputSchema: { type: 'object', properties: { tasks: { type: 'array', items: { type: 'string' } } }, required: ['tasks'] } },
  { name: 'flux_update_todo', description: 'Mark a to-do as done or not done, rename it, or remove it (id from flux_project_info).', inputSchema: { type: 'object', properties: { id: { type: 'number' }, done: { type: 'boolean' }, text: { type: 'string' }, remove: { type: 'boolean' } }, required: ['id'] } },
  { name: 'flux_set_description', description: 'Set the short description of the project (one sentence).', inputSchema: { type: 'object', properties: { description: { type: 'string' } }, required: ['description'] } },
];

async function mcpCall(name, input) {
  if (name === 'flux_list_projects') {
    ensureProjects();
    const list = await Promise.all(
      settings.projects
        .filter((p) => fs.existsSync(p.dir))
        .map(async (p) => {
          const l = await projectLangs(p.dir);
          return { name: path.basename(p.dir), folder: p.dir, main: l.kind, languages: l.langs, github: l.github, open: p.dir === workspace, description: settings.projectMeta?.[p.dir]?.description || '' };
        }),
    );
    return { result: list };
  }
  if (name === 'flux_open_project') {
    const dir = String(input.folder || '');
    if (!dir || !fs.existsSync(dir) || !fs.statSync(dir).isDirectory()) return { error: `Folder not found: ${dir}` };
    send('mcp:open-project', dir);
    for (let i = 0; i < 40 && workspace !== dir; i++) await new Promise((r) => setTimeout(r, 100));
    return workspace === dir ? { result: `Opened ${path.basename(dir)}.` } : { error: 'Flux could not open the project.' };
  }
  if (name === 'flux_write_file') {
    if (!workspace) return { error: 'No project is open.' };
    if (typeof input.path !== 'string' || typeof input.content !== 'string') return { error: 'path and content must be text.' };
    const file = path.resolve(workspace, input.path);
    if (!insideWorkspace(file)) return { error: 'That file is outside the project.' };
    await fsp.mkdir(path.dirname(file), { recursive: true });
    await fsp.writeFile(file, input.content);
    send('mcp:file-written', file);
    return { result: `Saved ${input.path} (${input.content.length} characters).` };
  }
  if (name === 'flux_open_file') {
    if (!workspace) return { error: 'No project is open.' };
    const file = path.resolve(workspace, String(input.path || ''));
    if (!insideWorkspace(file) || !fs.existsSync(file)) return { error: `File not found: ${input.path}` };
    send('mcp:open-file', file);
    return { result: `Opened ${input.path} in Flux.` };
  }
  if (!MCP_TOOLS.some((t2) => t2.name === name)) return { error: `Unknown tool ${name}` };
  const need = { flux_read_file: 'path', flux_add_todos: 'tasks', flux_update_todo: 'id', flux_set_description: 'description' }[name];
  if (need && input[need] === undefined) return { error: `Missing "${need}".` };
  if (name === 'flux_add_todos' && !Array.isArray(input.tasks)) return { error: 'tasks must be a list of text.' };
  const out = await projectTool(name, input);
  if (out?.changed) send('project:meta-changed');
  return out;
}

const mcp = createMcpServer({
  getSettings: () => settings,
  saveSettings: (patch) => {
    Object.assign(settings, patch);
    saveSettings();
  },
  version: app.getVersion(),
  tools: () => MCP_TOOLS,
  callTool: mcpCall,
});

const ai = createAI({
  project: projectTool,
  getSettings: () => settings,
  saveSettings: (patch) => {
    Object.assign(settings, patch);
    saveSettings();
  },
  send: (ch, p) => send(ch, p),
});
const github = createGitHub({
  getSettings: () => settings,
  saveSettings: (patch) => {
    Object.assign(settings, patch);
    saveSettings();
  },
});
const plugins = createPlugins({
  getSettings: () => settings,
  saveSettings: (patch) => {
    Object.assign(settings, patch);
    saveSettings();
  },
  githubApi: (p, opts) => github.api(p, opts),
});
const live = new LiveServer((entry) => send('live:log', entry));
const lsp = new LanguageServer(
  (msg) => send('lsp:message', msg),
  (code) => send('lsp:exit', code),
);

// Automatické aktualizácie jazykov (raz za deň, na pozadí, len ak sú zapnuté).
const updater = createUpdater({ getSettings: () => settings, send });

async function autoUpdateToolchains() {
  if (!isWin || settings.autoUpdateLangs === false) return;
  if (Date.now() - (settings.lastLangUpdate || 0) < 24 * 3600 * 1000) return;
  settings.lastLangUpdate = Date.now();
  saveSettings();
  const updates = await toolchains.checkUpdates();
  for (const id of Object.keys(updates)) {
    try {
      const res = await toolchains.upgrade(id, (p) => send('toolchains:progress', p));
      send('toolchains:updated', { ...res, from: updates[id].current });
    } catch {}
  }
}

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

// Priečinok s tvojimi nastaveniami, ktoré sa upravujú ako súbory (skratky, vlastné témy).
const configDir = () => path.join(app.getPath('userData'), 'config');
function insideConfig(p) {
  const rel = path.relative(configDir(), p);
  return rel !== '' && !rel.startsWith('..') && !path.isAbsolute(rel);
}

function guard(p) {
  const abs = path.resolve(p);
  if (insideConfig(abs)) return abs;
  if (!insideWorkspace(abs)) throw new Error(t('The path is outside the open folder.'));
  return abs;
}

const SHORTCUTS_TEMPLATE = `{
  "//": [
    "Your own shortcuts. Save this file (Ctrl+S) and they work right away.",
    "key:      e.g. Ctrl+Alt+R, Ctrl+Shift+1, Alt+F5, F6",
    "What it does (pick one):",
    "  run:      any command or script, runs in the Output panel (python, node, git, .bat, .ps1…)",
    "  insert:   text to type into the editor, $0 = where the cursor ends up",
    "  command:  a Flux command (run, stop, save, saveAll, live, newFile, newProject, openFolder,",
    "            quickOpen, palette, settings, home, format, sidebar, panel, theme)",
    "            or any editor action, e.g. editor.action.commentLine",
    "  url:      open a web page",
    "  sequence: a list of the steps above, done one after another",
    "Variables: \${file} \${fileName} \${dir} \${workspace} \${selection} \${word} \${line}"
  ],
  "shortcuts": [
    { "key": "Ctrl+Alt+P", "insert": "print(f\\"{$0=}\\")", "label": "Debug print" },
    { "key": "Ctrl+Alt+G", "url": "https://www.google.com/search?q=\${selection}", "label": "Google the selection" },
    { "key": "Ctrl+Alt+S", "sequence": [{ "command": "saveAll" }, { "command": "run" }], "label": "Save all and run" }
  ]
}
`;

const THEME_TEMPLATE = (name, type, colors) => `{
  "//": "Your own color theme. Change the colors (hex), save with Ctrl+S and pick it in Settings → Appearance.",
  "name": ${JSON.stringify(name)},
  "type": ${JSON.stringify(type)},
  "colors": ${JSON.stringify(colors, null, 4).replace(/\n/g, '\n  ')}
}
`;

// Súbor s nastaveniami Claude Desktop (tam sa pridávajú MCP servery).
function claudeDesktopConfigPath() {
  if (process.platform === 'win32') return path.join(process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming'), 'Claude', 'claude_desktop_config.json');
  if (process.platform === 'darwin') return path.join(os.homedir(), 'Library', 'Application Support', 'Claude', 'claude_desktop_config.json');
  return path.join(os.homedir(), '.config', 'Claude', 'claude_desktop_config.json');
}

// ---------- prihlásenie na GitHub v okne Fluxu ----------
// Otvorí github.com/login/device a kód z Fluxu doň vpíše sám (prihlásiť sa dá aj cez Google).
let ghLoginWin = null;
function closeGitHubLogin() {
  if (ghLoginWin && !ghLoginWin.isDestroyed()) ghLoginWin.close();
  ghLoginWin = null;
  return true;
}
function openGitHubLogin({ url, userCode }) {
  closeGitHubLogin();
  const w = new BrowserWindow({
    width: 540,
    height: 780,
    parent: win || undefined,
    title: 'Sign in with GitHub',
    autoHideMenuBar: true,
    backgroundColor: '#0d1117',
    webPreferences: { partition: 'persist:github-login', contextIsolation: true, nodeIntegration: false, sandbox: true },
  });
  ghLoginWin = w;
  // bežný prehliadač (niektoré prihlásenia, napr. Google, inak odmietnu vložené okno)
  w.webContents.setUserAgent(w.webContents.getUserAgent().replace(/\s(Electron|flux)\/\S+/gi, ''));
  w.webContents.setWindowOpenHandler(({ url: u }) => {
    if (/^https:\/\/([\w-]+\.)*(github\.com|google\.com|googleusercontent\.com|apple\.com|microsoft\.com|live\.com)\//i.test(u)) return { action: 'allow' };
    shell.openExternal(u);
    return { action: 'deny' };
  });
  const code = String(userCode || '').replace(/[^A-Z0-9]/gi, '');
  let filled = false;
  const tryFill = async () => {
    if (filled || w.isDestroyed() || !/github\.com\/login\/device/i.test(w.webContents.getURL())) return;
    const n = await w.webContents
      .executeJavaScript(
        `(() => { const ins = [...document.querySelectorAll('input')].filter((i) => i.offsetParent && i.type !== 'hidden' && (i.maxLength === 1 || /user[-_]?code/i.test(i.name + i.id))); if (!ins.length || ins.some((i) => i.value)) return 0; ins[0].focus(); return ins.length; })()`,
      )
      .catch(() => 0);
    if (!n) return;
    filled = true;
    const chars = n === 1 ? String(userCode) : code;
    for (const ch of chars) {
      if (w.isDestroyed()) return;
      w.webContents.sendInputEvent({ type: 'char', keyCode: ch });
      await new Promise((r) => setTimeout(r, 45));
    }
  };
  w.webContents.on('did-finish-load', () => setTimeout(tryFill, 400));
  w.webContents.on('did-navigate-in-page', () => setTimeout(tryFill, 400));
  w.on('closed', () => {
    if (ghLoginWin === w) ghLoginWin = null;
  });
  w.loadURL(url || 'https://github.com/login/device');
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
  ipcMain.handle('app:reload', () => win?.webContents.reload());
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
    return Promise.all(
      ordered.map(async (p) => {
        const l = await projectLangs(p.dir);
        return { dir: p.dir, pinned: !!p.pinned, name: path.basename(p.dir), kind: await projectKind(p.dir), langs: l.langs, github: l.github, recent: settings.recent.indexOf(p.dir) };
      }),
    );
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
  // Tapeta je aj tak rozmazaná – stačí malá kópia (rýchlejšie a menej pamäte). Mení sa len keď sa zmení súbor.
  let wallCache = { key: '', url: null };
  ipcMain.handle('wallpaper:get', async () => {
    const p = wallpaperPath();
    if (!p) return null;
    try {
      const st = await fsp.stat(p);
      const key = `${p}:${st.mtimeMs}:${st.size}`;
      if (wallCache.key === key) return wallCache.url;
      const img = nativeImage.createFromPath(p);
      let url;
      if (img.isEmpty()) {
        const buf = await fsp.readFile(p);
        url = `data:${buf[0] === 0x89 ? 'image/png' : 'image/jpeg'};base64,${buf.toString('base64')}`;
      } else {
        const small = img.getSize().width > 640 ? img.resize({ width: 640, quality: 'good' }) : img;
        url = `data:image/jpeg;base64,${small.toJPEG(82).toString('base64')}`;
      }
      wallCache = { key, url };
      return url;
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

  // Jazyky na stiahnutie (Java, C++, Go…)
  // GitHub / Git – operácie len nad otvoreným projektom
  // MCP server pre iné AI aplikácie
  ipcMain.handle('mcp:info', () => ({ ...mcp.info(), exe: process.execPath, bridge: path.join(__dirname, 'mcp-bridge.js'), claudeConfig: claudeDesktopConfigPath() }));
  ipcMain.handle('mcp:enable', async (_e, on) => {
    settings.mcpServer = { ...(settings.mcpServer || {}), enabled: !!on };
    saveSettings();
    if (on) await mcp.start();
    else mcp.stop();
    return mcp.info();
  });
  ipcMain.handle('mcp:new-key', () => mcp.newKey());
  ipcMain.handle('mcp:add-to-claude', async () => {
    const file = claudeDesktopConfigPath();
    let data = {};
    try {
      data = JSON.parse(await fsp.readFile(file, 'utf8'));
    } catch (err) {
      if (err.code !== 'ENOENT') throw new Error(t('Claude Desktop settings could not be read: {msg}', { msg: err.message }));
    }
    const i = mcp.info();
    data.mcpServers = { ...(data.mcpServers || {}), flux: { command: process.execPath, args: [path.join(__dirname, 'mcp-bridge.js'), `http://127.0.0.1:${i.port || settings.mcpServer?.port || 39217}/mcp`, i.token], env: { ELECTRON_RUN_AS_NODE: '1' } } };
    await fsp.mkdir(path.dirname(file), { recursive: true });
    await fsp.writeFile(file, JSON.stringify(data, null, 2));
    return file;
  });
  // Verzia a aktualizácie Fluxu
  ipcMain.handle('update:state', () => updater.state());
  ipcMain.handle('update:check', () => updater.check());
  ipcMain.handle('update:download', () => updater.download());
  ipcMain.handle('update:install', () => updater.install());
  ipcMain.handle('update:notes', (_e, force) => updater.notes(!!force));
  ipcMain.handle('gh:info', () => github.info());
  ipcMain.handle('gh:connect', (_e, token) => github.connect(String(token || '')));
  ipcMain.handle('gh:disconnect', () => github.disconnect());
  ipcMain.handle('gh:signin-start', async (_e, inApp = true) => {
    const r = await github.signInStart();
    if (inApp) openGitHubLogin(r);
    return r;
  });
  ipcMain.handle('gh:signin-wait', () => github.signInWait());
  ipcMain.handle('gh:signin-cancel', () => {
    closeGitHubLogin();
    return github.signInCancel();
  });
  ipcMain.handle('gh:signin-close', () => closeGitHubLogin());
  ipcMain.handle('gh:repos', () => github.repos());
  ipcMain.handle('gh:clone', (_e, full, root) => {
    // aj celý odkaz: https://github.com/owner/repo(.git)
    full = String(full || '').trim().replace(/^https?:\/\/github\.com\//i, '').replace(/\.git$/i, '').replace(/\/+$/, '');
    if (!/^[\w.-]+\/[\w.-]+$/.test(full)) throw new Error(t('That is not a GitHub repository link.'));
    return github.clone(full, root || defaultRoot());
  });
  ipcMain.handle('git:status', () => github.status(workspace));
  ipcMain.handle('git:commit-push', (_e, message) => github.commitPush(workspace, String(message || '')));
  ipcMain.handle('git:pull', () => github.pull(workspace));
  ipcMain.handle('git:sync', () => (workspace ? github.sync(workspace) : { pulled: 0 }));
  ipcMain.handle('gh:publish', (_e, opts) => {
    if (!workspace) throw new Error(t('Open a folder first.'));
    return github.publish(workspace, { name: opts?.name || path.basename(workspace), isPrivate: opts?.private !== false, description: opts?.description || '' });
  });

  // Pluginy
  ipcMain.handle('plugins:registry', (_e, force) => plugins.registry(!!force));
  ipcMain.handle('plugins:details', (_e, id) => plugins.details(id));
  ipcMain.handle('plugins:install', (_e, id) => plugins.install(id));
  ipcMain.handle('plugins:uninstall', (_e, id) => plugins.uninstall(id));
  ipcMain.handle('plugins:enable', (_e, id, on) => plugins.setEnabled(id, on));
  ipcMain.handle('plugins:active', () => plugins.active());
  ipcMain.handle('plugins:installed', () => plugins.installedList());
  ipcMain.handle('plugins:rate', (_e, issue, like) => plugins.rate(Number(issue), !!like));
  ipcMain.handle('plugins:load-folder', async () => {
    const r = await dialog.showOpenDialog(win, { title: t('Plugin folder (with plugin.json)'), properties: ['openDirectory'] });
    if (r.canceled || !r.filePaths[0]) return null;
    return plugins.installFromFolder(r.filePaths[0]);
  });
  ipcMain.handle('plugins:reload-local', async (_e, id) => {
    const m = plugins.installedList().find((x) => x.id === id);
    if (m?.source && fs.existsSync(m.source)) await plugins.installFromFolder(m.source);
    return true;
  });
  ipcMain.handle('plugins:scaffold', (_e, name) => plugins.scaffold(defaultRoot(), name));

  // AI asistent
  ipcMain.handle('ai:config', () => ai.publicConfig());
  ipcMain.handle('ai:update', (_e, patch) => ai.update(patch || {}));
  ipcMain.handle('ai:chat', (_e, id, messages) => ai.chat(id, messages));
  ipcMain.on('ai:stop', (_e, id) => ai.stop(id));

  // Nastavenia ako súbory (vlastné skratky, témy)
  ipcMain.handle('config:shortcuts', async () => {
    const file = path.join(configDir(), 'shortcuts.json');
    await fsp.mkdir(configDir(), { recursive: true });
    if (!fs.existsSync(file)) await fsp.writeFile(file, SHORTCUTS_TEMPLATE);
    return { path: file, text: await fsp.readFile(file, 'utf8') };
  });
  ipcMain.handle('config:themes', async () => {
    const dir = path.join(configDir(), 'themes');
    let names = [];
    try {
      names = (await fsp.readdir(dir)).filter((n) => n.endsWith('.json'));
    } catch {}
    return Promise.all(names.map(async (n) => ({ id: n.replace(/\.json$/, ''), path: path.join(dir, n), text: await fsp.readFile(path.join(dir, n), 'utf8') })));
  });
  ipcMain.handle('config:new-theme', async (_e, name, type, colors) => {
    const dir = path.join(configDir(), 'themes');
    await fsp.mkdir(dir, { recursive: true });
    const base = (name || 'my-theme').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'my-theme';
    let file = path.join(dir, `${base}.json`);
    for (let i = 2; fs.existsSync(file); i++) file = path.join(dir, `${base}-${i}.json`);
    await fsp.writeFile(file, THEME_TEMPLATE(name, type, colors));
    return file;
  });
  ipcMain.handle('config:dir', () => configDir());
  ipcMain.on('app:zoom', (_e, f) => {
    const z = Math.min(1.6, Math.max(0.7, Number(f) || 1));
    if (win && Math.abs(win.webContents.getZoomFactor() - z) > 0.001) win.webContents.setZoomFactor(z);
  });
  ipcMain.handle('app:choose-background', async () => {
    const r = await dialog.showOpenDialog(win, { title: t('Background image'), properties: ['openFile'], filters: [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'webp', 'bmp'] }] });
    if (r.canceled || !r.filePaths[0]) return false;
    const dest = path.join(app.getPath('userData'), `background${path.extname(r.filePaths[0]).toLowerCase()}`);
    await fsp.copyFile(r.filePaths[0], dest);
    settings.bgImage = dest;
    settings.material = 'wallpaper';
    settings.translucent = true;
    saveSettings();
    applyMaterial();
    return true;
  });
  // Vlastný kurzor myši: obrázok sa zmenší na max. 64 px a uloží ako data URL do nastavení.
  ipcMain.handle('app:choose-cursor', async () => {
    const r = await dialog.showOpenDialog(win, { title: t('Choose a cursor image'), properties: ['openFile'], filters: [{ name: 'Images', extensions: ['png', 'svg', 'cur', 'ico', 'gif', 'webp'] }] });
    if (r.canceled || !r.filePaths[0]) return null;
    const file = r.filePaths[0];
    const ext = path.extname(file).slice(1).toLowerCase();
    const buf = await fsp.readFile(file);
    if (buf.length > 2 * 1024 * 1024) throw new Error(t('The image is too large.'));
    if (ext === 'svg' || ext === 'cur' || ext === 'gif') {
      if (buf.length > 256 * 1024) throw new Error(t('The image is too large.'));
      const mime = { svg: 'image/svg+xml', cur: 'image/x-icon', gif: 'image/gif' }[ext];
      return { url: `data:${mime};base64,${buf.toString('base64')}`, width: 32, height: 32 };
    }
    let img = nativeImage.createFromBuffer(buf);
    if (img.isEmpty()) throw new Error(t('This file is not an image Flux can use.'));
    const { width, height } = img.getSize();
    if (Math.max(width, height) > 64) img = img.resize(width >= height ? { width: 64 } : { height: 64 });
    return { url: img.toDataURL(), ...img.getSize() };
  });
  ipcMain.handle('app:reset-background', () => {
    delete settings.bgImage;
    saveSettings();
    applyMaterial();
    return true;
  });
  // Vlastný príkaz zo skratky – beží vo výstupe ako program.
  ipcMain.handle('run:shell', (_e, command, cwd) => {
    const dir = cwd && fs.existsSync(cwd) ? cwd : workspace || os.homedir();
    const ok = isWin
      ? runner.start({ cmd: 'cmd.exe', args: `/d /s /c "${command}"`, cwd: dir, label: command })
      : runner.start({ cmd: 'bash', args: ['-lc', command], cwd: dir, label: command });
    return { ok };
  });
  ipcMain.handle('toolchains:status', () => toolchains.status());
  ipcMain.handle('toolchains:updates', () => toolchains.checkUpdates());
  ipcMain.handle('toolchains:upgrade', (_e, id) => toolchains.upgrade(id, (p) => send('toolchains:progress', p)));
  ipcMain.handle('toolchains:install', async (_e, id) => {
    const res = await toolchains.install(id, (p) => send('toolchains:progress', p));
    knownTools.add(id);
    return res;
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
  ipcMain.handle('run:file', async (_e, file, python, lang) => {
    const target = guard(file);
    const command = commandFor(target, python, lang);
    // Chýba jazyk (napr. Java)? Okno ponúkne stiahnutie aj s veľkosťou.
    const need = toolchainFor(target);
    if (need && need !== 'python' && !knownTools.has(need)) {
      const st = await toolchains.status();
      const info = st.list.find((x) => x.id === need);
      if (info && !info.installed) return { ok: false, missing: info, canInstall: st.canInstall };
      knownTools.add(need);
    }
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
// Typ projektu podľa najčastejších súborov (alebo podľa toho, čo si vybral pri vytvorení).
const KIND_EXT = {
  python: /\.pyw?$/i,
  web: /\.(html?|css)$/i,
  node: /\.(m?js|cjs)$/i,
  java: /\.java$/i,
  cpp: /\.(c|cpp|cc|cxx|h|hpp)$/i,
  go: /\.go$/i,
  csharp: /\.cs$/i,
  rust: /\.rs$/i,
  ruby: /\.rb$/i,
  php: /\.php$/i,
  lua: /\.lua$/i,
};

// Jazyky projektu podľa množstva kódu (ako na GitHube): hlavný jazyk, ostatné a či je to repozitár z GitHubu.
const langCache = new Map();
async function projectLangs(dir) {
  const hit = langCache.get(dir);
  if (hit && Date.now() - hit.at < 60000) return hit.data;
  const bytes = {};
  let files = 0;
  const scan = async (d, depth) => {
    let entries = [];
    try {
      entries = await fsp.readdir(d, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      if (files > 3000) return;
      if (IGNORED_DIRS.has(e.name) || e.name.startsWith('.') || e.name === 'venv' || e.name === 'dist' || e.name === 'build' || e.name === 'release') continue;
      const p = path.join(d, e.name);
      if (e.isDirectory()) {
        if (depth < 4) await scan(p, depth + 1);
        continue;
      }
      const kind = Object.keys(KIND_EXT).find((k) => KIND_EXT[k].test(e.name));
      if (!kind || /\.min\.(js|css)$/i.test(e.name)) continue;
      files++;
      let size = 1;
      try {
        size = Math.max(1, (await fsp.stat(p)).size);
      } catch {}
      bytes[kind] = (bytes[kind] || 0) + size;
    }
  };
  await scan(dir, 0);
  const langs = Object.entries(bytes)
    .sort((a, b) => b[1] - a[1])
    .map(([k]) => k);
  // HTML stránka s JavaScriptom je web.
  if (langs[0] === 'node' && bytes.web && bytes.web * 3 > bytes.node && !fs.existsSync(path.join(dir, 'package.json'))) langs.splice(langs.indexOf('web'), 1), langs.unshift('web');
  let github = false;
  try {
    github = /github\.com[/:]/i.test(fs.readFileSync(path.join(dir, '.git', 'config'), 'utf8'));
  } catch {}
  const data = { kind: langs[0] || 'folder', langs, github };
  langCache.set(dir, { at: Date.now(), data });
  return data;
}

async function projectKind(dir) {
  const chosen = settings.projectMeta?.[dir]?.kind;
  if (chosen && chosen !== 'empty') return chosen === 'c' ? 'cpp' : chosen;
  return (await projectLangs(dir)).kind;
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
    // Nainštalované pluginy: app://flux/plugins/<id>/<súbor>
    if (pathname.startsWith('/plugins/')) {
      const base = pluginsDir();
      const file = path.normalize(path.join(base, decodeURIComponent(pathname.slice('/plugins/'.length))));
      if (!file.startsWith(base + path.sep)) return new Response('Forbidden', { status: 403 });
      return net.fetch(pathToFileURL(file).toString());
    }
    if (pathname.startsWith('/registry/') && process.env.FLUX_PLUGIN_REGISTRY) {
      const base = path.resolve(process.env.FLUX_PLUGIN_REGISTRY);
      const file = path.normalize(path.join(base, decodeURIComponent(pathname.slice('/registry/'.length))));
      if (!file.startsWith(base + path.sep)) return new Response('Forbidden', { status: 403 });
      return net.fetch(pathToFileURL(file).toString());
    }
    const file = path.normalize(path.join(RENDERER_DIR, decodeURIComponent(pathname)));
    if (!file.startsWith(RENDERER_DIR)) return new Response('Forbidden', { status: 403 });
    return net.fetch(pathToFileURL(file).toString());
  });
  registerIpc();
  createWindow();
  setTimeout(() => autoUpdateToolchains().catch(() => {}), 20000);
  updater.start();
  if (settings.mcpServer?.enabled) mcp.start().catch(() => {});
});

app.on('window-all-closed', () => {
  runner.stop();
  lsp.stop();
  live.stop();
  app.quit();
});
