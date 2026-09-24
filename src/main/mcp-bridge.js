// Most pre aplikácie, ktoré MCP servery spúšťajú ako program (napr. Claude Desktop):
// číta správy zo štandardného vstupu a posiela ich bežiacemu Fluxu na http://127.0.0.1:…/mcp.
// Spúšťa sa cez Flux.exe s ELECTRON_RUN_AS_NODE=1:  Flux.exe mcp-bridge.js <url> <kľúč>
const readline = require('node:readline');

const [url, key] = process.argv.slice(2).filter((a) => !a.endsWith('mcp-bridge.js'));
const out = (msg) => process.stdout.write(`${JSON.stringify(msg)}\n`);

readline.createInterface({ input: process.stdin }).on('line', async (line) => {
  if (!line.trim()) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    return out({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } });
  }
  try {
    const res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream', Authorization: `Bearer ${key}` },
      body: JSON.stringify(msg),
    });
    if (res.status === 202) return;
    const text = await res.text();
    if (text) process.stdout.write(`${text.trim()}\n`);
  } catch {
    if (msg.id !== undefined && msg.id !== null) out({ jsonrpc: '2.0', id: msg.id, error: { code: -32002, message: 'Flux is not running. Open Flux and try again.' } });
  }
});
