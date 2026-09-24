// Most pre aplikácie, ktoré MCP servery spúšťajú ako program (napr. Claude Desktop):
// číta správy zo štandardného vstupu a posiela ich bežiacemu Fluxu na http://127.0.0.1:…/mcp.
// Spúšťa sa cez Flux.exe s ELECTRON_RUN_AS_NODE=1:  Flux.exe mcp-bridge.js <url> <kľúč>
const readline = require('node:readline');
const { spawn } = require('node:child_process');

const [url, key] = process.argv.slice(2).filter((a) => !a.endsWith('mcp-bridge.js'));
// Aplikácia zavrela spojenie → koniec (bez chyby).
process.stdout.on('error', () => process.exit(0));
const out = (msg) => process.stdout.write(`${JSON.stringify(msg)}\n`);

// Keď Flux nebeží, most ho sám spustí a chvíľu počká, kým server naštartuje.
let launched = false;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
function launchFlux() {
  if (launched) return;
  launched = true;
  try {
    const env = { ...process.env };
    delete env.ELECTRON_RUN_AS_NODE;
    // V rozšírení pre Claude Desktop beží most v Node od Claude – cestu k Fluxu dostane v FLUX_EXE.
    const exe = process.env.FLUX_EXE || process.execPath;
    spawn(exe, [], { detached: true, stdio: 'ignore', env, windowsHide: false }).unref();
  } catch {}
}
async function send(body) {
  const opts = {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream', Authorization: `Bearer ${key}` },
    body,
  };
  try {
    return await fetch(url, opts);
  } catch (err) {
    launchFlux();
    for (let i = 0; i < 40; i++) {
      await sleep(500);
      try {
        return await fetch(url, opts);
      } catch {}
    }
    throw err;
  }
}

readline.createInterface({ input: process.stdin }).on('line', async (line) => {
  if (!line.trim()) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    return out({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } });
  }
  try {
    const res = await send(JSON.stringify(msg));
    if (res.status === 202) return;
    const text = await res.text();
    if (text) process.stdout.write(`${text.trim()}\n`);
  } catch {
    if (msg.id !== undefined && msg.id !== null) out({ jsonrpc: '2.0', id: msg.id, error: { code: -32002, message: 'Flux is not running. Open Flux and try again.' } });
  }
});
