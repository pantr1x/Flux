// Flux Together cez Wi-Fi: Flux-y v tej istej sieti sa nájdu a spoja priamo (bez internetu a bez GitHubu).
//  - hľadanie: UDP správa každé 2 s na broadcast adresu siete (port 41777),
//  - spojenie: TCP, správy ako riadky JSON,
//  - „miestnosť“ = kód, ktorý zadajú všetci – kto ho nemá, nič neuvidí.
const dgram = require('node:dgram');
const net = require('node:net');
const os = require('node:os');
const crypto = require('node:crypto');

const PORT = 41777;
const MAGIC = 'flux-together-1';

function broadcastAddrs() {
  const out = new Set(['255.255.255.255']);
  for (const list of Object.values(os.networkInterfaces())) {
    for (const n of list || []) {
      if (n.family !== 'IPv4' || n.internal || !n.netmask) continue;
      const ip = n.address.split('.').map(Number);
      const mask = n.netmask.split('.').map(Number);
      out.add(ip.map((b, i) => (b | (~mask[i] & 255)) >>> 0).join('.'));
    }
  }
  return [...out];
}

function createLan({ send }) {
  const id = crypto.randomBytes(6).toString('hex');
  let room = null; // hash kódu miestnosti
  let me = { name: '' };
  let udp = null;
  let server = null;
  let tcpPort = 0;
  let timer = null;
  const peers = new Map(); // id → { sock, info, buf }
  let lastState = null;

  const emit = () => send('lan:peers', [...peers.values()].filter((p) => p.info).map((p) => ({ id: p.id, ...p.info })));

  function write(sock, msg) {
    try {
      sock.write(`${JSON.stringify(msg)}\n`);
    } catch {}
  }

  function attach(sock, peerId) {
    const p = { id: peerId, sock, info: null, buf: '' };
    sock.setEncoding('utf8');
    sock.setNoDelay(true);
    sock.on('data', (chunk) => {
      p.buf += chunk;
      if (p.buf.length > 2e6) return sock.destroy();
      let i;
      while ((i = p.buf.indexOf('\n')) >= 0) {
        const line = p.buf.slice(0, i);
        p.buf = p.buf.slice(i + 1);
        let msg;
        try {
          msg = JSON.parse(line);
        } catch {
          continue;
        }
        if (msg.t === 'hello') {
          // Spojenie bez správnej miestnosti sa hneď ukončí.
          if (msg.room !== room || !msg.id || msg.id === id) return sock.destroy();
          if (peers.has(msg.id) && peers.get(msg.id) !== p) {
            sock.destroy();
            return;
          }
          p.id = msg.id;
          peers.set(msg.id, p);
          p.info = { name: String(msg.name || 'Flux').slice(0, 40), state: null, at: Date.now() };
          if (!p.helloSent) {
            p.helloSent = true;
            write(sock, { t: 'hello', id, room, name: me.name });
          }
          if (lastState) write(sock, { t: 'state', s: lastState });
          emit();
        } else if (msg.t === 'state' && p.info) {
          p.info.state = msg.s;
          p.info.at = Date.now();
          emit();
        } else if (msg.t === 'ping' && p.info) {
          p.info.at = Date.now();
        }
      }
    });
    const gone = () => {
      if (peers.get(p.id) === p) {
        peers.delete(p.id);
        emit();
      }
    };
    sock.on('close', gone);
    sock.on('error', gone);
    return p;
  }

  function connectTo(peerId, address, port) {
    if (peers.has(peerId)) return;
    const sock = net.connect({ host: address, port, timeout: 5000 });
    const p = attach(sock, peerId);
    peers.set(peerId, p);
    sock.on('connect', () => {
      p.helloSent = true;
      write(sock, { t: 'hello', id, room, name: me.name });
    });
    sock.on('timeout', () => sock.destroy());
  }

  function announce() {
    if (!udp) return;
    const msg = Buffer.from(JSON.stringify({ m: MAGIC, id, room, port: tcpPort }));
    for (const addr of broadcastAddrs()) udp.send(msg, PORT, addr, () => {});
    for (const p of peers.values()) if (p.info) write(p.sock, { t: 'ping' });
  }

  async function start({ code, name }) {
    await stop();
    room = crypto.createHash('sha256').update(`flux:${String(code || '').trim().toLowerCase()}`).digest('hex').slice(0, 24);
    me = { name: String(name || os.userInfo().username || 'Flux').slice(0, 40) };
    server = net.createServer((sock) => attach(sock, null));
    await new Promise((res, rej) => {
      server.once('error', rej);
      server.listen(0, '0.0.0.0', res);
    });
    tcpPort = server.address().port;
    udp = dgram.createSocket({ type: 'udp4', reuseAddr: true });
    udp.on('message', (buf, rinfo) => {
      let msg;
      try {
        msg = JSON.parse(buf.toString());
      } catch {
        return;
      }
      if (msg.m !== MAGIC || msg.room !== room || msg.id === id || !msg.port) return;
      // Spojenie nadväzuje ten s „menším“ id – aby nevznikli dve.
      if (id < msg.id) connectTo(msg.id, rinfo.address, msg.port);
    });
    await new Promise((res) => {
      udp.once('error', res);
      udp.bind(PORT, () => {
        udp.setBroadcast(true);
        res();
      });
    });
    announce();
    timer = setInterval(() => {
      announce();
      // kto sa 20 s neozval, odišiel
      for (const [pid, p] of peers) if (p.info && Date.now() - p.info.at > 20000) p.sock.destroy(), peers.delete(pid);
      emit();
    }, 2000);
    return { running: true };
  }

  function update(state) {
    lastState = state;
    for (const p of peers.values()) if (p.info) write(p.sock, { t: 'state', s: state });
  }

  async function stop() {
    clearInterval(timer);
    timer = null;
    for (const p of peers.values()) p.sock.destroy();
    peers.clear();
    try {
      udp?.close();
    } catch {}
    udp = null;
    if (server) await new Promise((r) => server.close(() => r()));
    server = null;
    room = null;
    lastState = null;
    emit();
    return true;
  }

  return { start, stop, update };
}

module.exports = { createLan };
