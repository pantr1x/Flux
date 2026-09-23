// Zabalí renderer (editor, terminál, UI) do priečinka dist/renderer.
import { build } from 'esbuild';
import { cp, mkdir, rm } from 'node:fs/promises';

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
