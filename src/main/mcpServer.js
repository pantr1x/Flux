// MCP server Fluxu: iné AI aplikácie (Claude Desktop, Claude Code, Cursor…) môžu pracovať s projektmi vo Fluxe.
// Beží len na tomto počítači (127.0.0.1) a pýta si tajný kľúč. Zapína sa v Nastavenia → AI.
const http = require('node:http');
const crypto = require('node:crypto');

const DEFAULT_PORT = 39217;
const PROTOCOL = '2025-06-18';

function createMcpServer({ getSettings, saveSettings, version, tools, callTool, onState }) {
  let server = null;
  let port = 0;

  const cfg = () => getSettings().mcpServer || {};
  function token() {
    let tk = cfg().token;
    if (!tk) {
      tk = crypto.randomBytes(24).toString('base64url');
      saveSettings({ mcpServer: { ...cfg(), token: tk } });
    }
    return tk;
  }

  const rpcResult = (id, result) => ({ jsonrpc: '2.0', id, result });
  const rpcError = (id, code, message) => ({ jsonrpc: '2.0', id, error: { code, message } });

  async function handle(msg) {
    if (!msg || msg.jsonrpc !== '2.0' || typeof msg.method !== 'string') return rpcError(msg?.id ?? null, -32600, 'Invalid request');
    const { id, method, params = {} } = msg;
    if (id === undefined || id === null) return null; // notifikácia – bez odpovede
    switch (method) {
      case 'initialize':
        return rpcResult(id, {
          protocolVersion: params.protocolVersion || PROTOCOL,
          capabilities: { tools: { listChanged: false } },
          serverInfo: { name: 'flux', title: 'Flux', version },
          instructions: 'Flux is the code editor on this computer. Use these tools to see and change the project that is open in Flux: its files, description and to-do list. Changes show up in Flux right away.',
        });
      case 'ping':
        return rpcResult(id, {});
      case 'tools/list':
        return rpcResult(id, { tools: tools() });
      case 'tools/call': {
        const out = await callTool(String(params.name || ''), params.arguments || {}).catch((err) => ({ error: String(err?.message || err) }));
        const text = out?.error ? out.error : typeof out?.result === 'string' ? out.result : JSON.stringify(out?.result ?? out, null, 2);
        return rpcResult(id, { content: [{ type: 'text', text }], isError: !!out?.error });
      }
      default:
        return rpcError(id, -32601, `Method not found: ${method}`);
    }
  }

  function authorized(req, url) {
    const tk = token();
    const header = String(req.headers.authorization || '');
    const given = header.startsWith('Bearer ') ? header.slice(7) : url.searchParams.get('token') || '';
    if (given.length !== tk.length) return false;
    return crypto.timingSafeEqual(Buffer.from(given), Buffer.from(tk));
  }

  function onRequest(req, res) {
    const url = new URL(req.url, 'http://127.0.0.1');
    // ochrana pred webovými stránkami (DNS rebinding): prehliadač vždy posiela Origin
    if (req.headers.origin && !/^https?:\/\/(127\.0\.0\.1|localhost)(:\d+)?$/i.test(req.headers.origin)) {
      res.writeHead(403).end();
      return;
    }
    if (url.pathname !== '/mcp') {
      res.writeHead(404, { 'Content-Type': 'text/plain' }).end('Flux MCP server – use /mcp');
      return;
    }
    if (!authorized(req, url)) {
      res.writeHead(401, { 'Content-Type': 'application/json' }).end(JSON.stringify(rpcError(null, -32001, 'Wrong or missing Flux MCP key')));
      return;
    }
    if (req.method !== 'POST') {
      res.writeHead(405, { Allow: 'POST' }).end();
      return;
    }
    let body = '';
    req.on('data', (c) => {
      body += c;
      if (body.length > 20 * 1024 * 1024) req.destroy();
    });
    req.on('end', async () => {
      let msg;
      try {
        msg = JSON.parse(body);
      } catch {
        res.writeHead(400, { 'Content-Type': 'application/json' }).end(JSON.stringify(rpcError(null, -32700, 'Parse error')));
        return;
      }
      const list = Array.isArray(msg) ? msg : [msg];
      const answers = (await Promise.all(list.map(handle))).filter(Boolean);
      if (!answers.length) {
        res.writeHead(202).end();
        return;
      }
      res.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify(Array.isArray(msg) ? answers : answers[0]));
    });
  }

  function listen(p) {
    return new Promise((resolve, reject) => {
      const s = http.createServer(onRequest);
      s.once('error', reject);
      s.listen(p, '127.0.0.1', () => resolve(s));
    });
  }

  async function start() {
    if (server) return info();
    const wanted = Number(cfg().port) || DEFAULT_PORT;
    for (let p = wanted; p < wanted + 20; p++) {
      try {
        server = await listen(p);
        port = p;
        break;
      } catch {}
    }
    if (!server) throw new Error('No free port for the MCP server.');
    if (cfg().port !== port) saveSettings({ mcpServer: { ...cfg(), port } });
    onState?.(info());
    return info();
  }

  function stop() {
    server?.close();
    server = null;
    port = 0;
    onState?.(info());
    return info();
  }

  function newKey() {
    saveSettings({ mcpServer: { ...cfg(), token: crypto.randomBytes(24).toString('base64url') } });
    return info();
  }

  function info() {
    return { running: !!server, port, url: server ? `http://127.0.0.1:${port}/mcp` : '', token: token(), enabled: !!cfg().enabled };
  }

  return { start, stop, info, newKey };
}

module.exports = { createMcpServer };
