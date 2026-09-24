// Zabalí renderer (editor, terminál, UI) do priečinka dist/renderer a basedpyright do build/pyright.
import { build } from 'esbuild';
import { cp, mkdir, rm } from 'node:fs/promises';
import { readFileSync, writeFileSync, readdirSync, rmSync, mkdirSync } from 'node:fs';

const out = 'dist/renderer';
const watch = process.argv.includes('--watch');

await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });

const common = {
  bundle: true,
  minify: !watch,
  sourcemap: watch ? 'inline' : false,
  logLevel: 'warning',
  loader: { '.ttf': 'file' },
};

const monaco = 'node_modules/monaco-editor/esm/vs';
await Promise.all([
  build({
    ...common,
    entryPoints: { app: 'src/renderer/app.js' },
    outdir: out,
    format: 'esm',
  }),
  build({
    ...common,
    entryPoints: {
      'workers/editor': `${monaco}/editor/editor.worker.js`,
      'workers/json': `${monaco}/language/json/json.worker.js`,
      'workers/css': `${monaco}/language/css/css.worker.js`,
      'workers/html': `${monaco}/language/html/html.worker.js`,
      'workers/ts': `${monaco}/language/typescript/ts.worker.js`,
    },
    outdir: out,
    format: 'iife',
  }),
  cp('src/renderer/index.html', `${out}/index.html`),
  cp('src/renderer/styles.css', `${out}/styles.css`),
]);

console.log('Renderer zostavený → ' + out);

// basedpyright ako jeden .tar pre inštalátor (package.json → extraResources); Flux si ho rozbalí sám (src/main/lsp.js).
// Tisíce malých súborov robili aktualizáciu pomalou – inštalátor ich zakaždým mazal a rozbaľoval znova.
// Tar je deterministický (zoradené súbory, pevný čas) – rovnaký basedpyright = rovnaký súbor, inak by
// sa pri každom vydaní zmenil odtlačok a rýchla aktualizácia (quickUpdate.js) by nešla.
if (!watch) {
  const { version } = JSON.parse(readFileSync('node_modules/basedpyright/package.json', 'utf8'));
  rmSync('build/pyright', { recursive: true, force: true });
  mkdirSync('build/pyright', { recursive: true });
  const list = (rel) =>
    readdirSync(`node_modules/${rel}`, { withFileTypes: true })
      .sort((a, b) => (a.name < b.name ? -1 : 1))
      .flatMap((e) => (e.isDirectory() ? list(`${rel}/${e.name}`) : e.name.endsWith('.map') ? [] : [`${rel}/${e.name}`]));
  const parts = list('basedpyright').flatMap((f) => {
    const data = readFileSync(`node_modules/${f}`);
    return [tarHeader(f, data.length), data, Buffer.alloc((512 - (data.length % 512)) % 512)];
  });
  writeFileSync(`build/pyright/pyright-${version}.tar`, Buffer.concat([...parts, Buffer.alloc(1024)]));
  console.log(`basedpyright ${version} zabalený → build/pyright`);
}

// hlavička ustar (cesta do 255 znakov: prefix + name)
function tarHeader(file, size) {
  const h = Buffer.alloc(512);
  let name = file;
  let prefix = '';
  if (name.length > 100) {
    const cut = file.slice(0, 156).lastIndexOf('/');
    prefix = file.slice(0, cut);
    name = file.slice(cut + 1);
    if (name.length > 100 || prefix.length > 155 || cut < 0) throw new Error(`tar: path too long: ${file}`);
  }
  const put = (str, off, len) => h.write(str, off, len, 'utf8');
  const oct = (n, off, len) => put(n.toString(8).padStart(len - 1, '0') + '\0', off, len);
  put(name, 0, 100);
  oct(0o644, 100, 8);
  oct(0, 108, 8);
  oct(0, 116, 8);
  oct(size, 124, 12);
  oct(1577836800, 136, 12); // pevný čas (2020-01-01) – tar je vždy rovnaký
  h.fill(' ', 148, 156);
  put('0', 156, 1);
  put('ustar\0', 257, 6);
  put('00', 263, 2);
  put(prefix, 345, 155);
  let sum = 0;
  for (const b of h) sum += b;
  put(sum.toString(8).padStart(6, '0') + '\0 ', 148, 8);
  return h;
}
