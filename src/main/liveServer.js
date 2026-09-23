// Live Server: statický HTTP server + automatický reload pri uložení súboru.
// Do každého HTML vloží malý skript, ktorý cez Server-Sent Events čaká na zmeny.
// CSS sa vymení bez reloadu, ostatné zmeny stránku obnovia.
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

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
  addEventListener('error', (e) => send('error', [e.message + ' (' + (e.filename || '').split('/').pop() + ':' + e.lineno + ')']));
  addEventListener('unhandledrejection', (e) => send('error', ['Unhandled promise: ', e.reason]));
})();`;

class LiveServer {
  constructor(onLog) {
    this.onLog = onLog; // ({level, text}) => void
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

  async start(root) {
    if (this.server && this.root === root) return this.info();
    await this.stop();
    this.root = root;
    this.server = http.createServer((req, res) => this.handle(req, res));
    this.port = await listenOnFreePort(this.server, 5500);
    try {
      this.watcher = fs.watch(root, { recursive: true }, (_event, file) => this.changed(file));
      this.watcher.on('error', () => {});
    } catch {}
    return this.info();
  }

  info() {
    return { url: `http://127.0.0.1:${this.port}`, port: this.port, root: this.root };
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
        res.end(injectClient(html));
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

function listenOnFreePort(server, port) {
  return new Promise((resolve, reject) => {
    const tryPort = (p) => {
      server.once('error', (err) => {
        if (err.code === 'EADDRINUSE' && p < port + 100) tryPort(p + 1);
        else reject(err);
      });
      server.listen(p, '127.0.0.1', () => resolve(p));
    };
    tryPort(port);
  });
}

module.exports = { LiveServer };
