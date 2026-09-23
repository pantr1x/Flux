// Spúšťanie programov (▶ Run) – jeden proces naraz, výstup ide do terminálu v okne.
//
// Program beží v pseudoterminále (na Windows ConPTY, rovnako ako vo VS Code):
//  - funguje input(), farby a Ctrl+C,
//  - neotvára sa čierne okno konzoly,
//  - okná GUI programov (tkinter, turtle, pygame) sa normálne zobrazia.
// Ak by sa pseudoterminál nepodarilo načítať, použije sa obyčajný podproces.
const { spawn, execFile } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const isWin = process.platform === 'win32';

let pty = null;
try {
  pty = require('@lydell/node-pty');
} catch {}

// Čím spustiť ktorý typ súboru (ako rozšírenie „Code Runner“ vo VS Code).
function commandFor(file, python, lang) {
  const ext = path.extname(file).toLowerCase();
  // Súbor bez prípony, ktorý editor rozpoznal ako Python.
  if (!ext && lang === 'python') return python ? { cmd: python, args: ['-u', file] } : null;
  switch (ext) {
    case '.py':
    case '.pyw':
      return python ? { cmd: python, args: ['-u', file] } : null;
    case '.js':
    case '.mjs':
    case '.cjs':
      return { cmd: 'node', args: [file] };
    case '.bat':
    case '.cmd':
      return isWin ? { cmd: 'cmd.exe', args: ['/d', '/c', file] } : null;
    case '.ps1':
      return { cmd: isWin ? 'powershell.exe' : 'pwsh', args: ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', file] };
    case '.sh':
      return isWin ? null : { cmd: 'bash', args: [file] };
    default:
      return null;
  }
}

// Nájde napr. „node“ → C:\Program Files\nodejs\node.exe (pseudoterminál na Windows potrebuje celú cestu).
function resolveCommand(cmd) {
  if (!isWin || /[\\/]/.test(cmd)) return cmd;
  const exts = path.extname(cmd) ? [''] : (process.env.PATHEXT || '.EXE;.CMD;.BAT').split(';');
  for (const dir of (process.env.PATH || '').split(';')) {
    if (!dir) continue;
    for (const ext of exts) {
      const full = path.join(dir, cmd + ext);
      try {
        if (fs.statSync(full).isFile()) return full;
      } catch {}
    }
  }
  return cmd;
}

class Runner {
  constructor(send) {
    this.send = send; // (channel, payload) => void
    this.proc = null;
    this.cols = 100;
    this.rows = 20;
  }

  get running() {
    return !!this.proc;
  }

  start({ cmd, args, cwd, label }) {
    cmd = resolveCommand(cmd);
    if (this.proc) this.kill(this.proc);
    this.proc = null;
    const env = {
      ...process.env,
      PYTHONUNBUFFERED: '1',
      PYTHONIOENCODING: 'utf-8',
      PYTHONUTF8: '1',
      FORCE_COLOR: '1',
    };
    delete env.ELECTRON_RUN_AS_NODE;
    const started = Date.now();
    const handle = { started, pty: null, child: null, pid: 0, done: false };

    const finish = (code, error) => {
      if (handle.done) return;
      handle.done = true;
      // Výsledok starého (zastaveného) behu neposielať, ak už beží nový.
      const current = this.proc === handle;
      if (current) this.proc = null;
      if (current || !this.proc) this.send('run:exit', { code, error, ms: Date.now() - started });
    };

    if (pty) {
      try {
        handle.pty = pty.spawn(cmd, args, { name: 'xterm-256color', cols: this.cols, rows: this.rows, cwd, env, useConpty: true });
      } catch (err) {
        const msg = /ENOENT|not found|cannot find|File not found/i.test(String(err.message))
          ? `Príkaz „${cmd}“ sa nenašiel. Je nainštalovaný?`
          : String(err.message || err);
        this.send('run:start', { label, cwd, pid: 0 });
        finish(-1, msg);
        return true;
      }
      handle.pid = handle.pty.pid;
      this.proc = handle;
      this.send('run:start', { label, cwd, pid: handle.pid, pty: true });
      handle.pty.onData((data) => this.send('run:data', data));
      handle.pty.onExit(({ exitCode }) => {
        // Chvíľu počkať, nech dorazí zvyšok výstupu.
        setTimeout(() => finish(exitCode), 60);
      });
      return true;
    }

    let child;
    try {
      child = spawn(cmd, args, { cwd, env, windowsHide: true });
    } catch (err) {
      this.send('run:start', { label, cwd, pid: 0 });
      finish(-1, String(err.message || err));
      return true;
    }
    handle.child = child;
    handle.pid = child.pid;
    this.proc = handle;
    this.send('run:start', { label, cwd, pid: child.pid, pty: false });
    const forward = (chunk) => this.send('run:data', chunk.toString('utf8'));
    child.stdout.on('data', forward);
    child.stderr.on('data', forward);
    child.on('error', (err) => finish(-1, err.code === 'ENOENT' ? `Príkaz „${cmd}“ sa nenašiel. Je nainštalovaný?` : err.message));
    child.on('close', (code) => finish(code));
    return true;
  }

  input(text) {
    const h = this.proc;
    if (!h) return;
    if (h.pty) h.pty.write(text);
    else if (h.child.stdin.writable) h.child.stdin.write(text.replace(/\r/g, '\n'));
  }

  resize(cols, rows) {
    this.cols = Math.max(20, cols | 0);
    this.rows = Math.max(4, rows | 0);
    try {
      this.proc?.pty?.resize(this.cols, this.rows);
    } catch {}
  }

  stop() {
    if (this.proc) this.kill(this.proc);
  }

  kill(h) {
    if (isWin) {
      // Zabije aj všetky podprocesy (napr. keď skript sám niečo spúšťa).
      if (h.pid) execFile('taskkill', ['/pid', String(h.pid), '/T', '/F'], { windowsHide: true }, () => {});
      setTimeout(() => {
        try {
          h.pty?.kill();
        } catch {}
      }, 300);
      return;
    }
    try {
      if (h.pty) h.pty.kill('SIGTERM');
      else h.child.kill('SIGTERM');
    } catch {}
    setTimeout(() => {
      if (h.done) return;
      try {
        if (h.pty) h.pty.kill('SIGKILL');
        else h.child.kill('SIGKILL');
      } catch {}
    }, 1500);
  }
}

module.exports = { Runner, commandFor };
