// Programovacie jazyky, ktoré Flux nemá v balíčku – stiahnu sa až keď ich chceš (cez winget vo Windows).
// Každý má príkaz na overenie, balík vo wingete a približnú veľkosť.
const { execFile, spawn } = require('node:child_process');
const { t } = require('./i18n');

const isWin = process.platform === 'win32';

const TOOLCHAINS = [
  {
    id: 'python',
    name: 'Python',
    probe: isWin ? [['py', ['-3', '--version']], ['python', ['--version']]] : [['python3', ['--version']]],
    winget: 'Python.Python.3.13',
    download: 30,
    disk: 110,
    url: 'https://www.python.org/downloads/',
  },
  {
    id: 'node',
    name: 'Node.js',
    detail: 'JavaScript',
    probe: [['node', ['--version']]],
    winget: 'OpenJS.NodeJS.LTS',
    download: 30,
    disk: 100,
    url: 'https://nodejs.org/',
  },
  {
    id: 'java',
    name: 'Java',
    detail: 'OpenJDK 21',
    probe: [['java', ['-version']]],
    winget: 'Microsoft.OpenJDK.21',
    download: 180,
    disk: 320,
    url: 'https://learn.microsoft.com/java/openjdk/download',
  },
  {
    id: 'cpp',
    name: 'C / C++',
    detail: 'GCC (WinLibs)',
    probe: [['g++', ['--version']]],
    winget: 'BrechtSanders.WinLibs.POSIX.UCRT',
    download: 250,
    disk: 1000,
    url: 'https://winlibs.com/',
  },
  {
    id: 'go',
    name: 'Go',
    probe: [['go', ['version']]],
    winget: 'GoLang.Go',
    download: 75,
    disk: 250,
    url: 'https://go.dev/dl/',
  },
  {
    id: 'csharp',
    name: 'C#',
    detail: '.NET 10 SDK',
    probe: [['dotnet', ['--list-sdks']]],
    winget: 'Microsoft.DotNet.SDK.10',
    download: 220,
    disk: 750,
    url: 'https://dotnet.microsoft.com/download',
  },
  {
    id: 'rust',
    name: 'Rust',
    detail: 'GNU toolchain',
    probe: [['rustc', ['--version']]],
    winget: 'Rustlang.Rust.GNU',
    download: 300,
    disk: 1100,
    url: 'https://www.rust-lang.org/tools/install',
  },
  {
    id: 'ruby',
    name: 'Ruby',
    probe: [['ruby', ['--version']]],
    winget: 'RubyInstallerTeam.Ruby.3.3',
    download: 30,
    disk: 120,
    url: 'https://rubyinstaller.org/',
  },
  {
    id: 'php',
    name: 'PHP',
    probe: [['php', ['--version']]],
    winget: 'PHP.PHP.8.4',
    download: 32,
    disk: 110,
    url: 'https://windows.php.net/download/',
  },
  {
    id: 'lua',
    name: 'Lua',
    probe: [['lua', ['-v']]],
    winget: 'DEVCOM.Lua',
    download: 2,
    disk: 5,
    url: 'https://www.lua.org/download.html',
  },
];

function run(cmd, args, timeout = 8000) {
  return new Promise((resolve) => {
    execFile(cmd, args, { timeout, windowsHide: true }, (err, stdout, stderr) => {
      resolve(err ? null : `${stdout}\n${stderr}`.trim());
    });
  });
}

// Po inštalácii má Windows novú PATH len v registri – načítame ju, aby Flux nový jazyk hneď našiel.
async function refreshPath() {
  if (!isWin) return;
  const out = await run('powershell.exe', [
    '-NoProfile',
    '-Command',
    "[Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')",
  ]);
  if (out) process.env.PATH = [...new Set([...out.split(';'), ...(process.env.PATH || '').split(';')].filter(Boolean))].join(';');
}

async function probeOne(tc) {
  for (const [cmd, args] of tc.probe) {
    const out = await run(cmd, args);
    // Windows „Store“ alias pre python vypíše len návod – ten neberieme.
    if (out && !/Microsoft Store|was not found/i.test(out) && (tc.id !== 'csharp' || /\d+\.\d+/.test(out))) {
      // „version "21.0.2"“ má prednosť (java najprv vypíše aj iné riadky, napr. s IP adresou proxy).
      const version = (out.match(/version\s+"?v?(\d+(?:\.\d+)+)/i) || out.match(/(?<![\d.])v?(\d+\.\d+(?:\.\d+)?)(?![\d.])/) || [])[1] || '';
      return { installed: true, version };
    }
  }
  return { installed: false, version: '' };
}

async function status() {
  const list = await Promise.all(TOOLCHAINS.map(async (tc) => ({ ...publicInfo(tc), ...(await probeOne(tc)) })));
  return { list, canInstall: isWin && !!(await run('winget', ['--version'])) };
}

function publicInfo(tc) {
  return { id: tc.id, name: tc.name, detail: tc.detail || '', download: tc.download, disk: tc.disk, url: tc.url };
}

const installing = new Map();

// Inštalácia cez winget; priebeh (percentá a posledný riadok) ide do okna.
function install(id, onProgress) {
  const tc = TOOLCHAINS.find((x) => x.id === id);
  if (!tc) return Promise.reject(new Error('Unknown language'));
  if (!isWin) return Promise.reject(new Error(t('Automatic install works on Windows. Download it from {url}', { url: tc.url })));
  if (installing.has(id)) return installing.get(id);
  const job = new Promise((resolve, reject) => {
    const args = ['install', '--id', tc.winget, '-e', '--silent', '--accept-source-agreements', '--accept-package-agreements', '--disable-interactivity'];
    const child = spawn('winget', args, { windowsHide: true });
    let last = '';
    const onData = (buf) => {
      const text = buf.toString();
      const pct = [...text.matchAll(/(\d{1,3})\s?%/g)].pop();
      const line = text
        .split(/[\r\n]+/)
        .map((l) => l.replace(/[█▒░\-\\|/]{3,}.*$/, '').trim())
        .filter((l) => l.length > 3)
        .pop();
      if (line) last = line;
      onProgress({ id, percent: pct ? Number(pct[1]) : null, line: last });
    };
    child.stdout.on('data', onData);
    child.stderr.on('data', onData);
    child.on('error', (err) => reject(new Error(/ENOENT/.test(err.message) ? t('winget was not found. Download it from {url}', { url: tc.url }) : err.message)));
    child.on('close', async (code) => {
      await refreshPath();
      const st = await probeOne(tc);
      // winget vráti nenulový kód aj keď je už nainštalované – rozhoduje, či to teraz nájdeme.
      if (st.installed) resolve({ ...publicInfo(tc), ...st });
      else reject(new Error(last || t('Installation failed (code {code}).', { code })));
    });
  }).finally(() => installing.delete(id));
  installing.set(id, job);
  return job;
}

module.exports = { TOOLCHAINS, status, install, refreshPath };
