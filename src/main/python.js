// Hľadanie Python interpretera – podobne ako VS Code / PyCharm.
// Poradie: ručne vybraný → .venv / venv v projekte → $VIRTUAL_ENV → py launcher → python v PATH.
const { execFile } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const isWin = process.platform === 'win32';

function probe(command, args = []) {
  return new Promise((resolve) => {
    execFile(
      command,
      [...args, '-c', 'import sys; print(sys.executable); print(sys.version.split()[0])'],
      { timeout: 8000, windowsHide: true },
      (err, stdout) => {
        if (err) return resolve(null);
        const [executable, version] = stdout.trim().split(/\r?\n/);
        // Windows „Store“ alias (WindowsApps\python.exe) nič nespustí – odfiltrujeme ho.
        if (!executable || !version || /WindowsApps/i.test(executable)) return resolve(null);
        resolve({ path: executable.trim(), version: version.trim() });
      },
    );
  });
}

function venvPython(dir) {
  return isWin ? path.join(dir, 'Scripts', 'python.exe') : path.join(dir, 'bin', 'python');
}

function commonWindowsInstalls() {
  const roots = [
    process.env.LOCALAPPDATA && path.join(process.env.LOCALAPPDATA, 'Programs', 'Python'),
    process.env.ProgramFiles,
  ].filter(Boolean);
  const found = [];
  for (const root of roots) {
    try {
      for (const name of fs.readdirSync(root)) {
        if (/^Python3\d+/i.test(name)) found.push(path.join(root, name, 'python.exe'));
      }
    } catch {}
  }
  // Najnovšia verzia ako prvá (Python313 pred Python312).
  return found.sort().reverse();
}

async function findPython(workspace, override) {
  const candidates = [];
  if (override) candidates.push({ cmd: override, source: 'vybraný ručne' });
  if (workspace) {
    for (const name of ['.venv', 'venv', 'env']) {
      const exe = venvPython(path.join(workspace, name));
      if (fs.existsSync(exe)) candidates.push({ cmd: exe, source: name });
    }
  }
  if (process.env.VIRTUAL_ENV) {
    candidates.push({ cmd: venvPython(process.env.VIRTUAL_ENV), source: 'VIRTUAL_ENV' });
  }
  if (isWin) {
    candidates.push({ cmd: 'py', args: ['-3'], source: 'py launcher' });
    candidates.push({ cmd: 'python', source: 'PATH' });
    for (const exe of commonWindowsInstalls()) candidates.push({ cmd: exe, source: 'inštalácia' });
  } else {
    candidates.push({ cmd: 'python3', source: 'PATH' }, { cmd: 'python', source: 'PATH' });
  }

  for (const c of candidates) {
    const info = await probe(c.cmd, c.args);
    if (info) return { ...info, source: c.source };
  }
  return null;
}

module.exports = { findPython, probe };
