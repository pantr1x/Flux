// Spustí basedpyright (Pyright s farbami ako vo VS Code – jazykový server pre Python) a posiela JSON-RPC správy medzi ním a oknom.
// Pyright beží cez Node, ktorý je súčasťou Electronu – netreba mať nainštalovaný Node.js.
const { spawn, execFile } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

// V nainštalovanom Fluxe je basedpyright JEDEN súbor resources/pyright-<verzia>.tar (build.mjs), nie ~5 400 súborov –
// inštalátor ich pri každej aktualizácii mazal a rozbaľoval znova (a Defender kontroloval každý), preto to trvalo dlho.
// Rozbalí sa až pri prvom použití do userData/pyright/<verzia> a potom len keď príde nová verzia basedpyright.
let unpacking = null;
function pyrightPath() {
  const dev = () => require.resolve('basedpyright/langserver.index.js');
  const { app } = require('electron');
  if (!app.isPackaged) return Promise.resolve(dev());
  const tar = fs.readdirSync(process.resourcesPath).find((f) => /^pyright-.+\.tar$/.test(f));
  if (!tar) return Promise.resolve(dev().replace(/app\.asar([\\/])/, 'app.asar.unpacked$1'));
  const root = path.join(app.getPath('userData'), 'pyright');
  const dir = path.join(root, tar.slice(0, -4));
  const entry = path.join(dir, 'basedpyright', 'langserver.index.js');
  if (fs.existsSync(path.join(dir, '.ok'))) return Promise.resolve(entry);
  unpacking ||= (async () => {
    // staré verzie preč
    for (const old of fs.existsSync(root) ? fs.readdirSync(root) : []) if (old !== path.basename(dir)) fs.rmSync(path.join(root, old), { recursive: true, force: true });
    fs.rmSync(dir, { recursive: true, force: true });
    fs.mkdirSync(dir, { recursive: true });
    // tar.exe je vo Windows 10/11 (aj v Linuxe a macOS); vo Windows radšej ten zo System32 – GNU tar z Gitu nevie „C:\\…“
    const sysTar = path.join(process.env.SystemRoot || 'C:\\Windows', 'System32', 'tar.exe');
    const tarExe = process.platform === 'win32' && fs.existsSync(sysTar) ? sysTar : 'tar';
    await new Promise((ok, fail) => execFile(tarExe, ['-xf', path.join(process.resourcesPath, tar), '-C', dir], { windowsHide: true, timeout: 5 * 60 * 1000 }, (err) => (err ? fail(err) : ok())));
    fs.writeFileSync(path.join(dir, '.ok'), '');
    return entry;
  })().finally(() => (unpacking = null));
  return unpacking;
}

class LanguageServer {
  constructor(onMessage, onExit, memoryLimit = () => 2048) {
    this.onMessage = onMessage;
    this.onExit = onExit;
    this.memoryLimit = memoryLimit;
    this.proc = null;
    this.buffer = Buffer.alloc(0);
    this.pending = null; // správy odoslané, kým sa server ešte rozbaľuje/štartuje
    this.run = 0;
  }

  async start(cwd) {
    this.stop();
    this.buffer = Buffer.alloc(0);
    const run = ++this.run;
    this.pending = [];
    let file;
    try {
      file = await pyrightPath();
    } catch {
      if (run === this.run) (this.pending = null), this.onExit(1);
      return false;
    }
    if (run !== this.run) return false; // medzitým stop() alebo nový start()
    const proc = spawn(process.execPath, [file, '--stdio'], {
      cwd,
      windowsHide: true,
      // Strop pamäte – Pyright inak na veľkých projektoch zaberie aj niekoľko GB.
      env: { ...process.env, ELECTRON_RUN_AS_NODE: '1', NODE_OPTIONS: `--max-old-space-size=${this.memoryLimit()}` },
    });
    this.proc = proc;
    proc.stdout.on('data', (chunk) => this.receive(chunk));
    // Ak server skončí, zápis do neho nesmie zhodiť celú aplikáciu (EPIPE).
    proc.stdin.on('error', () => {});
    proc.stderr.on('data', () => {});
    proc.on('error', () => {});
    proc.on('exit', (code) => {
      if (this.proc !== proc) return;
      this.proc = null;
      this.onExit(code);
    });
    const queued = this.pending || [];
    this.pending = null;
    for (const m of queued) this.send(m);
    return true;
  }

  send(message) {
    if (this.pending) return void this.pending.push(message);
    if (!this.proc || !this.proc.stdin.writable) return;
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
    this.run++; // zruší aj štart, ktorý ešte čaká na rozbalenie
    this.pending = null;
    if (!this.proc) return;
    const proc = this.proc;
    this.proc = null;
    proc.kill();
  }
}

module.exports = { LanguageServer };
