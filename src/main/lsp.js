// Spustí basedpyright (Pyright s farbami ako vo VS Code – jazykový server pre Python) a posiela JSON-RPC správy medzi ním a oknom.
// Pyright beží cez Node, ktorý je súčasťou Electronu – netreba mať nainštalovaný Node.js.
const { spawn } = require('node:child_process');

function pyrightPath() {
  // V zabalenej aplikácii je basedpyright rozbalený mimo app.asar (kvôli spúšťaniu).
  return require.resolve('basedpyright/langserver.index.js').replace(/app\.asar([\\/])/, 'app.asar.unpacked$1');
}

class LanguageServer {
  constructor(onMessage, onExit) {
    this.onMessage = onMessage;
    this.onExit = onExit;
    this.proc = null;
    this.buffer = Buffer.alloc(0);
  }

  start(cwd) {
    this.stop();
    this.buffer = Buffer.alloc(0);
    const proc = spawn(process.execPath, [pyrightPath(), '--stdio'], {
      cwd,
      windowsHide: true,
      env: { ...process.env, ELECTRON_RUN_AS_NODE: '1' },
    });
    this.proc = proc;
    proc.stdout.on('data', (chunk) => this.receive(chunk));
    proc.stderr.on('data', () => {});
    proc.on('error', () => {});
    proc.on('exit', (code) => {
      if (this.proc !== proc) return;
      this.proc = null;
      this.onExit(code);
    });
  }

  send(message) {
    if (!this.proc) return;
    const body = Buffer.from(JSON.stringify(message), 'utf8');
    this.proc.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
    this.proc.stdin.write(body);
  }

  receive(chunk) {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    for (;;) {
      const headerEnd = this.buffer.indexOf('\r\n\r\n');
      if (headerEnd === -1) return;
      const header = this.buffer.subarray(0, headerEnd).toString('ascii');
      const match = /Content-Length: *(\d+)/i.exec(header);
      if (!match) {
        this.buffer = this.buffer.subarray(headerEnd + 4);
        continue;
      }
      const length = Number(match[1]);
      const start = headerEnd + 4;
      if (this.buffer.length < start + length) return;
      const body = this.buffer.subarray(start, start + length).toString('utf8');
      this.buffer = this.buffer.subarray(start + length);
      try {
        this.onMessage(JSON.parse(body));
      } catch {}
    }
  }

  stop() {
    if (!this.proc) return;
    const proc = this.proc;
    this.proc = null;
    proc.kill();
  }
}

module.exports = { LanguageServer };
