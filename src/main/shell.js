// Terminál v paneli dole: skutočný príkazový riadok (PowerShell na Windows) v priečinku projektu.
// Beží vedľa výstupu programov – ▶ Run ho nepreruší.
const os = require('node:os');
const fs = require('node:fs');

const isWin = process.platform === 'win32';
let pty = null;
try {
  pty = require('@lydell/node-pty');
} catch {}

function shellCommand() {
  if (isWin) return { cmd: 'powershell.exe', args: ['-NoLogo'] };
  const sh = process.env.SHELL && fs.existsSync(process.env.SHELL) ? process.env.SHELL : '/bin/bash';
  return { cmd: sh, args: ['-i'] };
}

class Shell {
  constructor(send) {
    this.send = send;
    this.p = null;
    this.cols = 100;
    this.rows = 20;
  }

  get available() {
    return !!pty;
  }

  start(cwd) {
    if (this.p) return true;
    if (!pty) return false;
    const { cmd, args } = shellCommand();
    const env = { ...process.env, TERM_PROGRAM: 'Flux' };
    delete env.ELECTRON_RUN_AS_NODE;
    const dir = cwd && fs.existsSync(cwd) ? cwd : os.homedir();
    try {
      this.p = pty.spawn(cmd, args, { name: 'xterm-256color', cols: this.cols, rows: this.rows, cwd: dir, env, useConpty: true });
    } catch (err) {
      this.send('shell:data', `\r\n${String(err.message || err)}\r\n`);
      return false;
    }
    const me = this.p;
    me.onData((d) => this.send('shell:data', d));
    me.onExit(() => {
      if (this.p === me) this.p = null;
      this.send('shell:exit', {});
    });
    return true;
  }

  write(text) {
    this.p?.write(String(text));
  }

  resize(cols, rows) {
    this.cols = Math.max(20, cols | 0);
    this.rows = Math.max(4, rows | 0);
    try {
      this.p?.resize(this.cols, this.rows);
    } catch {}
  }

  // Nový terminál v inom priečinku (napr. po otvorení iného projektu).
  restart(cwd) {
    this.kill();
    return this.start(cwd);
  }

  kill() {
    const p = this.p;
    this.p = null;
    try {
      p?.kill();
    } catch {}
  }
}

module.exports = { Shell };
