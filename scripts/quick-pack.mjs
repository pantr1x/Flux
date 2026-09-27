// Po electron-builderi: pripraví rýchlu aktualizáciu k vydaniu (release.yml) –
// release/Flux-<v>.asar.gz (len kód Fluxu) a release/quick.json (verzia, sha512, base z after-pack.cjs).
// Použitie: node scripts/quick-pack.mjs <version> [release/win-unpacked]
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';

const [version, dir = 'release/win-unpacked'] = process.argv.slice(2);
if (!version) throw new Error('usage: quick-pack.mjs <version> [dir]');
const asar = readFileSync(`${dir}/resources/app.asar`);
const base = readFileSync(`${dir}/resources/quick-base.txt`, 'utf8').trim();
const gz = gzipSync(asar, { level: 9 });
const file = `Flux-${version}.asar.gz`;
writeFileSync(`release/${file}`, gz);
const info = { version, file, base, sha512: createHash('sha512').update(asar).digest('base64'), size: asar.length, gzSize: gz.length };
writeFileSync('release/quick.json', JSON.stringify(info, null, 2) + '\n');
console.log(`Quick update ${version}: ${(gz.length / 1048576).toFixed(1)} MB (${file}), base ${base.slice(0, 12)}…`);
