// Prenesie ikony (src/renderer/icons.js) a farebné témy kódu (src/renderer/themes.js) do Flux Native
// (native/src/gen.rs), aby vyzeral rovnako ako Electron Flux.
// Text v SVG (napr. „JS“) sa vyberie zvlášť – Flux Native ho dokreslí vlastným písmom (resvg bez písiem).
// Spustenie: node scripts/native-gen.mjs
import fs from 'node:fs';

const root = new URL('..', import.meta.url);
let src = fs.readFileSync(new URL('src/renderer/icons.js', root), 'utf8').replace(/^export /gm, '');
src += '\nreturn { paths, FILLED, FILE_ICONS, EXT_ICON, icon };';
const { paths, FILLED, FILE_ICONS, EXT_ICON, icon } = new Function(src)();

const rs = (s) => JSON.stringify(s);
let out = '// Vygenerované: node scripts/native-gen.mjs (zdroj src/renderer/icons.js a themes.js). Neupravovať ručne.\n\n';
out += '// (x, y, text, farba 0xRRGGBB, veľkosť) v súradniciach viewBoxu, y = účaria\n';
out += "pub type Label = (f32, f32, &'static str, u32, f32);\n\n";

// farebné ikony súborov
out += 'pub fn file(key: &str) -> Option<(&\'static str, f32, &\'static [Label])> {\n    Some(match key {\n';
for (const [key, svg0] of Object.entries(FILE_ICONS)) {
  const labels = [];
  let svg = svg0.replace(/<text x="([\d.]+)" y="([\d.]+)" fill="(#[0-9a-fA-F]+)"[^>]*font-size="([\d.]+)"[^>]*>([^<]*)<\/text>/g, (_, x, y, fill, size, t) => {
    let hex = fill.slice(1);
    if (hex.length === 3) hex = [...hex].map((c) => c + c).join('');
    labels.push(`(${+x}f32, ${+y}f32, ${rs(t)}, 0x${hex}, ${+size}f32)`);
    return '';
  });
  svg = svg.replace(/ class="ficon"| aria-hidden="true"/g, '').replace(/<svg /, '<svg xmlns="http://www.w3.org/2000/svg" ');
  const vb = +svg.match(/viewBox="0 0 (\d+)/)[1];
  out += `        ${rs(key)} => (${rs(svg)}, ${vb}f32, &[${labels.join(', ')}]),\n`;
}
out += '        _ => return None,\n    })\n}\n\n';

// prípona → ikona
out += 'pub fn ext_key(ext: &str) -> &\'static str {\n    match ext {\n';
const byKey = {};
for (const [ext, key] of Object.entries(EXT_ICON)) (byKey[key] ||= []).push(ext);
for (const [key, exts] of Object.entries(byKey)) out += `        ${exts.map(rs).join(' | ')} => ${rs(key)},\n`;
out += '        _ => "file",\n    }\n}\n\n';

// čiarové ikony (biele, farbu dá tint)
out += 'pub fn line(name: &str) -> Option<&\'static str> {\n    Some(match name {\n';
for (const name of [...Object.keys(paths), ...Object.keys(FILLED)]) {
  const svg = icon(name, 24).replace(/ class="icon"| aria-hidden="true"/g, '').replace(/currentColor/g, '#fff').replace(/<svg /, '<svg xmlns="http://www.w3.org/2000/svg" ');
  out += `        ${rs(name)} => ${rs(svg)},\n`;
}
out += '        _ => return None,\n    })\n}\n';

// témy kódu (farby 0xRRGGBB v poradí poľa THEME_KEYS)
const tsrc = fs.readFileSync(new URL('src/renderer/themes.js', root), 'utf8').replace(/^export /gm, '') + '\nreturn { THEMES, DEFAULT_THEME };';
const { THEMES, DEFAULT_THEME } = new Function(tsrc)();
const KEYS = ['fg', 'comment', 'keyword', 'storage', 'string', 'number', 'type', 'function', 'variable', 'parameter', 'property', 'constant', 'tag', 'attr', 'delimiter', 'regexp'];
out += `\npub const DEFAULT_THEME: &str = ${rs(DEFAULT_THEME)};\n`;
out += `pub const THEME_KEYS: [&str; ${KEYS.length}] = [${KEYS.map(rs).join(', ')}];\n\n`;
out += '// (tmavá?, farby, kurzíva komentárov)\npub fn code_theme(id: &str) -> Option<(bool, [u32; ' + KEYS.length + '], bool)> {\n    Some(match id {\n';
for (const [id, th] of Object.entries(THEMES)) {
  out += `        ${rs(id)} => (${th.type === 'dark'}, [${KEYS.map((k) => '0x' + th.t[k]).join(', ')}], ${!!th.t.italicComments}),\n`;
}
out += '        _ => return None,\n    })\n}\n';

// vlajky jazykov (src/renderer/flags.js)
const fsrc = fs.readFileSync(new URL('src/renderer/flags.js', root), 'utf8').replace(/^export /gm, '').replace(/^import .*$/gm, '') + '\nreturn { FLAGS, flag };';
const { FLAGS, flag } = new Function(fsrc)();
out += '\npub fn flag(code: &str) -> Option<&\'static str> {\n    Some(match code {\n';
for (const code of Object.keys(FLAGS)) {
  const svg = flag(code, 30).replace(/ class="flag"| aria-hidden="true"/g, '').replace(/<svg /, '<svg xmlns="http://www.w3.org/2000/svg" ');
  out += `        ${rs(code)} => ${rs(svg)},\n`;
}
out += '        _ => return None,\n    })\n}\n';

fs.writeFileSync(new URL('native/src/gen.rs', root), out);
console.log('native/src/gen.rs:', Object.keys(FILE_ICONS).length, 'file icons,', Object.keys(paths).length, 'line icons,', Object.keys(THEMES).length, 'code themes,', Object.keys(FLAGS).length, 'flags');
