// Live Server: statický HTTP server + automatický reload pri uložení súboru.
// Do každého HTML vloží malý skript, ktorý cez Server-Sent Events čaká na zmeny.
// CSS sa vymení bez reloadu, ostatné zmeny stránku obnovia.
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.htm': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.webp': 'image/webp',
  '.avif': 'image/avif',
  '.ico': 'image/x-icon',
  '.txt': 'text/plain; charset=utf-8',
  '.md': 'text/plain; charset=utf-8',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.mp4': 'video/mp4',
  '.webm': 'video/webm',
  '.mp3': 'audio/mpeg',
  '.wav': 'audio/wav',
  '.pdf': 'application/pdf',
  '.wasm': 'application/wasm',
};

const IGNORED = /(^|[\\/])(node_modules|\.git|__pycache__|\.venv|venv)([\\/]|$)/;

const CLIENT = `(() => {
  const es = new EventSource('/__flux/events');
  es.onmessage = (e) => {
    if (e.data === 'css') {
      for (const link of document.querySelectorAll('link[rel="stylesheet"]')) {
        const url = new URL(link.href);
        if (url.origin !== location.origin) continue;
        url.searchParams.set('_flux', Date.now());
        link.href = url.href;
      }
    } else if (e.data === 'reload') {
      location.reload();
    }
  };
  const send = (level, args) => {
    try {
      const text = args.map((a) => {
        if (a instanceof Error) return a.stack || a.message;
        if (typeof a === 'object') { try { return JSON.stringify(a); } catch { return String(a); } }
        return String(a);
      }).join(' ');
      navigator.sendBeacon('/__flux/log', JSON.stringify({ level, text }));
    } catch {}
  };
  for (const level of ['log', 'info', 'warn', 'error']) {
    const orig = console[level].bind(console);
    console[level] = (...args) => { send(level, args); orig(...args); };
  }
  addEventListener('error', (e) => {
    send('error', [e.message + ' (' + (e.filename || '').split('/').pop() + ':' + e.lineno + ')']);
    overlay(e.message, e.filename, e.lineno);
  });
  addEventListener('unhandledrejection', (e) => send('error', ['Unhandled promise: ', e.reason]));

  // Skok do kódu: súbor (cesta na serveri) a riadok sa otvoria vo Fluxe.
  const open = (file, line) => navigator.sendBeacon('/__flux/open', JSON.stringify({ file, line }));
  const sameOrigin = (u) => { try { return new URL(u, location.href).origin === location.origin; } catch { return false; } };

  // Chyba v JavaScripte: malá karta v rohu stránky s tlačidlom „Ukázať vo Fluxe“.
  let box = null;
  function overlay(message, file, line) {
    if (!document.body) return addEventListener('DOMContentLoaded', () => overlay(message, file, line), { once: true });
    if (!box) {
      const host = document.createElement('flux-overlay');
      host.style.cssText = 'position:fixed;right:14px;bottom:14px;z-index:2147483647;';
      box = host.attachShadow({ mode: 'open' });
      box.innerHTML = '<style>.c{font:13px/1.45 system-ui,sans-serif;background:#1c1c24;color:#f1f1f5;border-radius:12px;box-shadow:0 10px 30px rgba(0,0,0,.35),0 0 0 1px rgba(255,90,110,.5);padding:12px 14px;max-width:380px;margin-top:8px}.h{display:flex;gap:8px;align-items:center;font-weight:700;color:#ff6b7a}.m{margin:6px 0 10px;word-break:break-word;font-family:Consolas,monospace;font-size:12px}.r{display:flex;gap:8px}button{font:600 12px system-ui;border:0;border-radius:8px;padding:6px 10px;cursor:pointer}.o{background:#8b7bff;color:#fff}.x{background:rgba(255,255,255,.1);color:#ddd}</style><div id=l></div>';
      document.documentElement.append(host);
    }
    const list = box.getElementById('l');
    if (list.children.length >= 3) list.firstElementChild.remove();
    const c = document.createElement('div');
    c.className = 'c';
    const where = file && sameOrigin(file) ? new URL(file, location.href).pathname : '';
    c.innerHTML = '<div class=h>⚠ JavaScript error</div><div class=m></div><div class=r></div>';
    c.querySelector('.m').textContent = message + (where ? ' — ' + where.split('/').pop() + ':' + line : '');
    const r = c.querySelector('.r');
    if (where) {
      const b = document.createElement('button');
      b.className = 'o';
      b.textContent = 'Show in Flux';
      b.onclick = () => open(where, line);
      r.append(b);
    }
    const x = document.createElement('button');
    x.className = 'x';
    x.textContent = 'Dismiss';
    x.onclick = () => c.remove();
    r.append(x);
    list.append(c);
  }

  // Alt + klik na prvok stránky → Flux otvorí HTML na riadku, kde je prvok napísaný.
  let hl = null;
  let tip = null;
  const clear = () => { hl && hl.remove(); tip && tip.remove(); hl = tip = null; };
  const target = (el) => el && el.closest && el.closest('[data-flux-line]');
  addEventListener('mousemove', (e) => {
    if (!e.altKey) return clear();
    const el = target(e.target);
    if (!el) return clear();
    const r = el.getBoundingClientRect();
    if (!hl) {
      hl = document.createElement('div');
      hl.style.cssText = 'position:fixed;pointer-events:none;z-index:2147483646;border:2px solid #8b7bff;background:rgba(139,123,255,.12);border-radius:4px;transition:all .06s';
      tip = document.createElement('div');
      tip.style.cssText = 'position:fixed;pointer-events:none;z-index:2147483647;background:#8b7bff;color:#fff;font:600 11px system-ui;padding:2px 7px;border-radius:5px';
      document.documentElement.append(hl, tip);
    }
    Object.assign(hl.style, { left: r.left + 'px', top: r.top + 'px', width: r.width + 'px', height: r.height + 'px' });
    tip.textContent = '<' + el.tagName.toLowerCase() + (el.id ? '#' + el.id : '') + (el.classList.length ? '.' + [...el.classList].join('.') : '') + '>  line ' + el.dataset.fluxLine;
    Object.assign(tip.style, { left: Math.max(4, r.left) + 'px', top: Math.max(4, r.top - 22) + 'px' });
  }, true);
  addEventListener('keyup', (e) => { if (e.key === 'Alt') clear(); });
  addEventListener('blur', clear);
  addEventListener('click', (e) => {
    if (!e.altKey) return;
    const el = target(e.target);
    if (!el) return;
    e.preventDefault();
    e.stopPropagation();
    clear();
    let file = el.dataset.fluxFile || location.pathname;
    if (file.endsWith('/')) file += 'index.html';
    open(file, Number(el.dataset.fluxLine));
  }, true);
})();`;

class LiveServer {
  constructor(onLog, onOpen = () => {}) {
    this.onLog = onLog; // ({level, text}) => void
    this.onOpen = onOpen; // ({file, line}) => void – skok do kódu z náhľadu
    this.lan = false;
    this.server = null;
    this.watcher = null;
    this.clients = new Set();
    this.root = null;
    this.port = null;
    this.timer = null;
    this.pending = null;
  }

  get running() {
    return !!this.server;
  }

  // lan: server počúva aj na Wi-Fi, aby sa stránka dala otvoriť v mobile.
  async start(root, { lan = false } = {}) {
    if (this.server && this.root === root && this.lan === !!lan) return this.info();
    await this.stop();
    this.root = root;
    this.lan = !!lan;
    this.server = http.createServer((req, res) => this.handle(req, res));
    this.port = await listenOnFreePort(this.server, 5500, this.lan ? '0.0.0.0' : '127.0.0.1');
    try {
      this.watcher = fs.watch(root, { recursive: true }, (_event, file) => this.changed(file));
      this.watcher.on('error', () => {});
    } catch {}
    return this.info();
  }

  info() {
    const ip = this.lan ? lanAddress() : null;
    return { url: `http://127.0.0.1:${this.port}`, port: this.port, root: this.root, lan: this.lan, lanUrl: ip ? `http://${ip}:${this.port}` : null };
  }

  async stop() {
    clearTimeout(this.timer);
    if (this.watcher) this.watcher.close();
    this.watcher = null;
    for (const res of this.clients) res.end();
    this.clients.clear();
    if (this.server) await new Promise((r) => this.server.close(() => r()));
    this.server = null;
  }

  changed(file) {
    if (!file || IGNORED.test(file)) return;
    const kind = path.extname(file).toLowerCase() === '.css' ? 'css' : 'reload';
    // Viac zmien naraz (napr. „Uložiť všetko“) → jeden reload.
    this.pending = this.pending === 'reload' ? 'reload' : kind;
    clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      const msg = this.pending;
      this.pending = null;
      for (const res of this.clients) res.write(`data: ${msg}\n\n`);
    }, 120);
  }

  handle(req, res) {
    const url = new URL(req.url, 'http://localhost');
    if (url.pathname === '/__flux/events') {
      res.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
      });
      res.write('retry: 1000\n\n');
      this.clients.add(res);
      req.on('close', () => this.clients.delete(res));
      return;
    }
    if (url.pathname === '/__flux/client.js') {
      res.writeHead(200, { 'Content-Type': MIME['.js'], 'Cache-Control': 'no-store' });
      return res.end(CLIENT);
    }
    if (url.pathname === '/__flux/open' && req.method === 'POST') {
      let body = '';
      req.on('data', (c) => (body += c).length > 1e4 && req.destroy());
      req.on('end', () => {
        try {
          const { file, line } = JSON.parse(body);
          const abs = path.join(this.root, decodeURIComponent(String(file)));
          if (abs.startsWith(this.root + path.sep) && fs.existsSync(abs)) this.onOpen({ file: abs, line: Math.max(1, Number(line) || 1) });
        } catch {}
        res.writeHead(204).end();
      });
      return;
    }
    if (url.pathname === '/__flux/log' && req.method === 'POST') {
      let body = '';
      req.on('data', (c) => (body += c).length > 1e5 && req.destroy());
      req.on('end', () => {
        try {
          const { level, text } = JSON.parse(body);
          this.onLog({ level: String(level), text: String(text) });
        } catch {}
        res.writeHead(204).end();
      });
      return;
    }

    let rel;
    try {
      rel = decodeURIComponent(url.pathname);
    } catch {
      return res.writeHead(400).end('Bad request');
    }
    const file = path.join(this.root, rel);
    // Nepustiť von z priečinka projektu (../../).
    if (file !== this.root && !file.startsWith(this.root + path.sep)) return res.writeHead(403).end('Forbidden');

    fs.stat(file, (err, stat) => {
      if (!err && stat.isDirectory()) {
        if (!url.pathname.endsWith('/')) {
          res.writeHead(301, { Location: url.pathname + '/' + url.search });
          return res.end();
        }
        const index = path.join(file, 'index.html');
        if (fs.existsSync(index)) return this.sendFile(index, res);
        return this.sendListing(file, url.pathname, res);
      }
      if (err) return this.notFound(res, url.pathname);
      this.sendFile(file, res);
    });
  }

  sendFile(file, res) {
    const ext = path.extname(file).toLowerCase();
    const type = MIME[ext] || 'application/octet-stream';
    const headers = { 'Content-Type': type, 'Cache-Control': 'no-store' };
    if (ext === '.html' || ext === '.htm') {
      fs.readFile(file, 'utf8', (err, html) => {
        if (err) return res.writeHead(500).end(String(err));
        res.writeHead(200, headers);
        res.end(injectClient(annotate(html)));
      });
      return;
    }
    res.writeHead(200, headers);
    fs.createReadStream(file).pipe(res);
  }

  sendListing(dir, urlPath, res) {
    let entries = [];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true }).filter((e) => !e.name.startsWith('.'));
    } catch {}
    const items = entries
      .sort((a, b) => b.isDirectory() - a.isDirectory() || a.name.localeCompare(b.name))
      .map((e) => {
        const name = e.name + (e.isDirectory() ? '/' : '');
        return `<li><a href="${encodeURI(urlPath + name)}">${escapeHtml(name)}</a></li>`;
      })
      .join('');
    res.writeHead(200, { 'Content-Type': MIME['.html'] });
    res.end(
      injectClient(
        `<!doctype html><meta charset="utf-8"><title>${escapeHtml(urlPath)}</title>` +
          `<style>body{font:15px system-ui;margin:40px;color:#333}a{color:#6b5cff;text-decoration:none}li{margin:6px 0}</style>` +
          `<h2>${escapeHtml(urlPath)}</h2><ul>${items || '<li>(empty)</li>'}</ul>`,
      ),
    );
  }

  notFound(res, urlPath) {
    res.writeHead(404, { 'Content-Type': MIME['.html'] });
    res.end(
      injectClient(
        `<!doctype html><meta charset="utf-8"><style>body{font:15px system-ui;margin:40px;color:#333}</style>` +
          `<h2>404</h2><p>File <code>${escapeHtml(urlPath)}</code> does not exist.</p>`,
      ),
    );
  }
}

// Každej značke v HTML pridá data-flux-line (riadok v súbore) – na Alt + klik v náhľade.
// Obsah <script>, <style>, <textarea> a komentáre ostanú nedotknuté.
function annotate(html) {
  if (html.length > 2e6) return html;
  let out = '';
  let line = 1;
  let i = 0;
  const count = (a, b) => {
    for (let k = a; k < b; k++) if (html.charCodeAt(k) === 10) line++;
  };
  const re = /<!--[\s\S]*?-->|<(script|style|textarea|pre)\b[^>]*>[\s\S]*?<\/\1\s*>|<([a-zA-Z][\w-]*)(?=[\s/>])/g;
  let m;
  while ((m = re.exec(html))) {
    count(i, m.index);
    if (m[2] && !/^(html|head|meta|link|title|base)$/i.test(m[2])) {
      out += html.slice(i, m.index) + m[0] + ` data-flux-line="${line}"`;
      i = m.index + m[0].length;
    } else if (m[1]) {
      // <script>/<style> samotná značka dostane riadok, obsah nie
      const tagEnd = m[0].indexOf('>');
      const open = m[0].slice(0, tagEnd);
      const name = open.match(/^<(\w+)/)[0];
      out += html.slice(i, m.index) + name + ` data-flux-line="${line}"` + m[0].slice(name.length);
      count(m.index, m.index + m[0].length);
      i = m.index + m[0].length;
    } else {
      out += html.slice(i, m.index + m[0].length);
      count(m.index, m.index + m[0].length);
      i = m.index + m[0].length;
    }
  }
  return out + html.slice(i);
}

// Adresa počítača v domácej sieti (Wi-Fi / kábel) pre mobil.
function lanAddress() {
  const nets = os.networkInterfaces();
  const all = [];
  for (const [name, list] of Object.entries(nets)) for (const n of list || []) if (n.family === 'IPv4' && !n.internal) all.push({ name, address: n.address });
  // Súkromné siete majú prednosť (192.168…, 10…, 172.16–31…), virtuálne adaptéry nie.
  const good = all.filter((n) => /^(192\.168|10\.|172\.(1[6-9]|2\d|3[01]))/.test(n.address) && !/vEthernet|VirtualBox|VMware|WSL|Hyper-V|docker/i.test(n.name));
  return (good[0] || all[0])?.address || null;
}

function injectClient(html) {
  // Čo najskôr (hneď za <head>), aby sa zachytil aj console.log z ostatných skriptov stránky.
  const tag = '<script src="/__flux/client.js"></script>';
  const head = /<head[^>]*>/i.exec(html);
  if (head) return html.slice(0, head.index + head[0].length) + tag + html.slice(head.index + head[0].length);
  const doctype = /<!doctype[^>]*>/i.exec(html);
  if (doctype) return html.slice(0, doctype.index + doctype[0].length) + tag + html.slice(doctype.index + doctype[0].length);
  return tag + html;
}

function escapeHtml(s) {
  return s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
}

function listenOnFreePort(server, port, host = '127.0.0.1') {
  return new Promise((resolve, reject) => {
    const tryPort = (p) => {
      server.once('error', (err) => {
        if (err.code === 'EADDRINUSE' && p < port + 100) tryPort(p + 1);
        else reject(err);
      });
      server.listen(p, host, () => resolve(p));
    };
    tryPort(port);
  });
}

module.exports = { LiveServer, annotate };
