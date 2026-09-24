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
    id: 'git',
    name: 'Git',
    detail: 'GitHub & version control',
    probe: [['git', ['--version']]],
    winget: 'Git.Git',
    download: 65,
    disk: 350,
    url: 'https://git-scm.com/downloads',
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
    id: 'perl',
    name: 'Perl',
    detail: 'Strawberry Perl',
    probe: [['perl', ['-v']]],
    winget: 'StrawberryPerl.StrawberryPerl',
    download: 170,
    disk: 600,
    url: 'https://strawberryperl.com/',
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
  {
    id: 'zig',
    name: 'Zig',
    probe: [['zig', ['version']]],
    winget: 'zig.zig',
    download: 50,
    disk: 300,
    url: 'https://ziglang.org/download/',
  },
  {
    id: 'r',
    name: 'R',
    detail: 'statistics & data',
    probe: [['Rscript', ['--version']]],
    winget: 'RProject.R',
    download: 85,
    disk: 300,
    url: 'https://cran.r-project.org/bin/windows/base/',
  },
  {
    id: 'julia',
    name: 'Julia',
    detail: 'Juliaup',
    probe: [['julia', ['--version']]],
    // Juliaup z Microsoft Store – oficiálny spôsob inštalácie Julie vo Windows
    winget: '9NJNWW8PVKMN',
    download: 180,
    disk: 600,
    url: 'https://julialang.org/downloads/',
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
  addRPath();
}

// Inštalátor R nepridáva R do PATH – pridáme priečinok bin najnovšej verzie (C:\Program Files\R\R-4.x.y\bin).
function addRPath() {
  if (!isWin) return;
  const fs = require('node:fs');
  const path = require('node:path');
  for (const root of [process.env.ProgramFiles, process.env.LOCALAPPDATA && path.join(process.env.LOCALAPPDATA, 'Programs')].filter(Boolean)) {
    let dirs = [];
    try {
      dirs = fs.readdirSync(path.join(root, 'R')).filter((d) => /^R-\d/.test(d));
    } catch {
      continue;
    }
    const newest = dirs.sort((a, b) => b.localeCompare(a, undefined, { numeric: true }))[0];
    const bin = newest && path.join(root, 'R', newest, 'bin');
    if (bin && fs.existsSync(path.join(bin, 'Rscript.exe')) && !(process.env.PATH || '').includes(bin)) process.env.PATH = `${process.env.PATH};${bin}`;
    return;
  }
}
addRPath();

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
function install(id, onProgress, verb = 'install') {
  const tc = TOOLCHAINS.find((x) => x.id === id);
  if (!tc) return Promise.reject(new Error('Unknown language'));
  if (!isWin) return Promise.reject(new Error(t('Automatic install works on Windows. Download it from {url}', { url: tc.url })));
  if (installing.has(id)) return installing.get(id);
  const job = new Promise((resolve, reject) => {
    const args = [verb, '--id', tc.winget, '-e', '--silent', '--accept-source-agreements', '--accept-package-agreements', '--disable-interactivity'];
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

// Ktoré nainštalované jazyky majú novšiu verziu (winget upgrade vypíše tabuľku).
async function checkUpdates() {
  if (!isWin) return {};
  const out = await run('winget', ['upgrade', '--accept-source-agreements', '--disable-interactivity'], 60000);
  if (!out) return {};
  const found = {};
  for (const line of out.split(/\r?\n/)) {
    const cols = line.trim().split(/\s{1,}/);
    for (const tc of TOOLCHAINS) {
      const i = cols.findIndex((c) => c.toLowerCase() === tc.winget.toLowerCase());
      if (i >= 0 && cols[i + 2]) found[tc.id] = { current: cols[i + 1], available: cols[i + 2] };
    }
  }
  return found;
}

const upgrade = (id, onProgress) => install(id, onProgress, 'upgrade');

module.exports = { TOOLCHAINS, status, install, upgrade, checkUpdates, refreshPath };
