// Z CHANGELOG.md vyberie poznámky k aktuálnej verzii (package.json) → build/release-notes.md
// (electron-builder ich dá do GitHub Release). Spúšťa sa v .github/workflows/release.yml.
import { readFileSync, writeFileSync } from 'node:fs';

const { version } = JSON.parse(readFileSync('package.json', 'utf8'));
const log = readFileSync('CHANGELOG.md', 'utf8');
const part = log.split(/^## /m).find((s) => s.startsWith(`${version} `) || s.startsWith(`${version}\n`));
if (!part) throw new Error(`CHANGELOG.md has no section for ${version}`);
writeFileSync('build/release-notes.md', part.slice(part.indexOf('\n') + 1).trim() + '\n');
console.log(`Release notes for ${version} written.`);
