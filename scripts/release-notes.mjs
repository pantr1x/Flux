// Z CHANGELOG.md vyberie poznámky k aktuálnej verzii (package.json) → build/release-notes.md
// (electron-builder ich dá do GitHub Release). Spúšťa sa v .github/workflows/release.yml.
// Vývojárska verzia (devBuild 1.4.4.1 = 1.4.5-beta.1) nemusí mať sekciu – vezmú sa správy posledných commitov.
import { readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';

const { version, devBuild } = JSON.parse(readFileSync('package.json', 'utf8'));
// vývojárska verzia môže mať sekciu pod zobrazovaným číslom (## 1.4.4.1)
const names = version.includes('-') && devBuild ? [version, devBuild] : [version];
const log = readFileSync('CHANGELOG.md', 'utf8');
const part = log.split(/^## /m).find((s) => names.some((n) => s.startsWith(`${n} `) || s.startsWith(`${n}\n`)));
let body = part?.slice(part.indexOf('\n') + 1).trim();
if (!body && version.includes('-')) {
  let commits = '';
  try {
    commits = execSync('git log -15 --no-merges --format="- %s"', { encoding: 'utf8' }).trim();
  } catch {}
  body = `Developer build – not a public release.\n\n${commits}`.trim();
}
if (!body) throw new Error(`CHANGELOG.md has no section for ${version}`);
writeFileSync('build/release-notes.md', body + '\n');
console.log(`Release notes for ${version} written.`);
