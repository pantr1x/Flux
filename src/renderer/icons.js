// Jednoduché čiarové ikony (štýl Lucide), 24×24.
const paths = {
  check: '<path d="m5 12 5 5 9-10"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.6-3.6"/>',
  pin: '<path d="M9 4h6l-1 6 3 3v2H7v-2l3-3Z"/><path d="M12 15v5"/>',
  edit: '<path d="M4 20h4L19 9l-4-4L4 16Z"/><path d="m13.5 6.5 4 4"/>',
  code: '<path d="m8 7-5 5 5 5M16 7l5 5-5 5M13.5 4.5l-3 15"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z"/>',
  template: '<rect x="3" y="3" width="18" height="18" rx="3"/><path d="M3 9h18M9 21V9"/>',
  palette: '<circle cx="13.5" cy="6.5" r="1.2"/><circle cx="17.5" cy="10.5" r="1.2"/><circle cx="8.5" cy="7.5" r="1.2"/><circle cx="6.5" cy="12.5" r="1.2"/><path d="M12 2a10 10 0 0 0 0 20 2 2 0 0 0 2-2c0-.5-.2-1-.5-1.3-.3-.4-.5-.8-.5-1.3a1.8 1.8 0 0 1 1.8-1.8H17a5 5 0 0 0 5-5C22 6 17.5 2 12 2Z"/>',
  play: '<path d="M7 4.5v15a1 1 0 0 0 1.5.86l12-7.5a1 1 0 0 0 0-1.72l-12-7.5A1 1 0 0 0 7 4.5Z" fill="currentColor" stroke="none"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="2.5" fill="currentColor" stroke="none"/>',
  globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>',
  folder: '<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v8a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5Z"/>',
  folderOpen: '<path d="M3 17V7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7a2 2 0 0 1 2 2v1"/><path d="M3 17l2.2-5.6A2 2 0 0 1 7.1 10H21l-2.4 7.2a2 2 0 0 1-1.9 1.3H4.5A1.5 1.5 0 0 1 3 17Z"/>',
  filePlus: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8Z"/><path d="M14 3v5h5M12 11v6M9 14h6"/>',
  folderPlus: '<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v8a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5Z"/><path d="M12 10.5v6M9 13.5h6"/>',
  refresh: '<path d="M20 11a8 8 0 0 0-14.3-4.9L4 8"/><path d="M4 4v4h4M4 13a8 8 0 0 0 14.3 4.9L20 16"/><path d="M20 20v-4h-4"/>',
  collapse: '<path d="m7 20 5-5 5 5M7 4l5 5 5-5"/>',
  chevron: '<path d="m9 6 6 6-6 6"/>',
  eye: '<path d="M2 12c1.5-3.5 5.5-7 10-7s8.5 3.5 10 7c-1.5 3.5-5.5 7-10 7s-8.5-3.5-10-7z"/><circle cx="12" cy="12" r="3"/>',
  eyeOff: '<path d="M3 3l18 18M10.6 5.1A10 10 0 0 1 12 5c5 0 9 4.5 10 7a13 13 0 0 1-3.2 4.3M6.6 6.6C4.3 8 2.7 10.2 2 12c1 2.5 5 7 10 7a9.6 9.6 0 0 0 5.4-1.6M9.9 9.9a3 3 0 0 0 4.2 4.2"/>',
  rotate: '<path d="M3 12a9 9 0 0 1 15.5-6.2L21 8M21 3v5h-5M21 12a9 9 0 0 1-15.5 6.2L3 16M3 21v-5h5"/>',
  camera: '<path d="M4 8h3l2-3h6l2 3h3a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z"/><circle cx="12" cy="13.5" r="3.5"/>',
  laptop: '<path d="M5 6h14v10H5zM2 19h20"/>',
  qr: '<path d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h2v2h-2zM18 14h2M14 18h2v2M18 18h2v2h-2"/>',
  arrowLeft: '<path d="M19 12H5m6-6-6 6 6 6"/>',
  arrowRight: '<path d="M5 12h14m-6-6 6 6-6 6"/>',
  x: '<path d="M6 6l12 12M18 6 6 18"/>',
  sidebar: '<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M9 4v16"/>',
  panel: '<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M3 14h18"/>',
  panelRight: '<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M14 4v16"/>',
  puzzle: '<path d="M9.5 4.5a2 2 0 1 1 4 0V6H17a1 1 0 0 1 1 1v3.5h1.5a2 2 0 1 1 0 4H18V18a1 1 0 0 1-1 1h-3.5v-1.5a2 2 0 1 0-4 0V19H6a1 1 0 0 1-1-1v-3.5h1.5a2 2 0 1 0 0-4H5V7a1 1 0 0 1 1-1h3.5Z"/>',
  rocket: '<path d="M5 15c-1.5 1-2 4-2 6 2 0 5-.5 6-2"/><path d="M9 12a13 13 0 0 1 11-9c0 4.5-3 9-9 11l-2-2Z"/><path d="M9 12H5.5l2-3.5H11M12 15v3.5l3.5-2V13"/><circle cx="15.5" cy="8.5" r="1.3"/>',
  upload: '<path d="M12 16V4m0 0 5 5m-5-5-5 5M5 20h14"/>',
  lock: '<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
  menu: '<path d="M4 7h16M4 12h16M4 17h16"/>',
  dot: '<circle cx="12" cy="12" r="4" fill="currentColor" stroke="none"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  moon: '<path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z"/>',
  command: '<path d="M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3Z"/>',
  external: '<path d="M14 4h6v6M20 4l-9 9"/><path d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"/>',
  monitor: '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>',
  tablet: '<rect x="5" y="3" width="14" height="18" rx="2"/><path d="M11 18h2"/>',
  phone: '<rect x="7" y="3" width="10" height="18" rx="2"/><path d="M11 18h2"/>',
  trash: '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3"/>',
  terminal: '<path d="m5 8 4 4-4 4M12 17h7"/>',
  download: '<path d="M12 4v11M7 10l5 5 5-5M5 20h14"/>',
  git: '<circle cx="6" cy="6" r="2.2"/><circle cx="6" cy="18" r="2.2"/><circle cx="18" cy="8" r="2.2"/><path d="M6 8.2v7.6M18 10.2c0 4-6 3-11.2 6.3"/>',
  clock: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
  file: '<path d="M6 3h8l4 4v14H6Z"/><path d="M14 3v4h4"/>',
  todo: '<path d="m4 7 2 2 3-3M4 16l2 2 3-3M12 8h8M12 17h8"/>',
  python: '<path d="M12 3c-4 0-4 1.8-4 3v2h4.2v1H6c-2 0-3 1.4-3 4s1 4 3 4h1.8v-2.5c0-1.6 1.3-2.9 3-2.9h4.3c1.3 0 2.4-1.1 2.4-2.4V6c0-1.6-1.8-3-5.5-3Z"/><path d="M12 21c4 0 4-1.8 4-3v-2h-4.2v-1H18c2 0 3-1.4 3-4s-1-4-3-4h-1.8"/><circle cx="10" cy="5.8" r=".6" fill="currentColor"/><circle cx="14" cy="18.2" r=".6" fill="currentColor"/>',
  sparkle: '<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8Z"/><path d="M19 17l.7 1.8 1.8.7-1.8.7L19 22l-.7-1.8-1.8-.7 1.8-.7Z"/>',
  save: '<path d="M5 3h11l3 3v13a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2Z"/><path d="M8 3v5h7M8 21v-7h8v7"/>',
};

// Plné (nie obrysové) ikony – logo GitHubu.
const FILLED = {
  github: ['0 0 16 16', '<path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>'],
};

export function icon(name, size = 16) {
  if (FILLED[name]) return `<svg class="icon" width="${size}" height="${size}" viewBox="${FILLED[name][0]}" fill="currentColor" aria-hidden="true">${FILLED[name][1]}</svg>`;
  return `<svg class="icon" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] || ''}</svg>`;
}

// Farebné „odznaky“ typov súborov v strome a na taboch.
const badges = {
  py: ['py', '#4b8bbe'], pyi: ['py', '#4b8bbe'], pyw: ['py', '#4b8bbe'],
  js: ['js', '#e8c547'], mjs: ['js', '#e8c547'], cjs: ['js', '#e8c547'], jsx: ['jsx', '#61dafb'],
  ts: ['ts', '#3178c6'], tsx: ['tsx', '#3178c6'],
  html: ['<>', '#e96b3d'], htm: ['<>', '#e96b3d'],
  css: ['#', '#a86bf0'], scss: ['#', '#cd6799'], less: ['#', '#4a6fb1'],
  json: ['{}', '#9aa0a6'], md: ['M', '#70a4d6'], txt: ['T', '#9aa0a6'],
  png: ['img', '#3ecf8e'], jpg: ['img', '#3ecf8e'], jpeg: ['img', '#3ecf8e'], gif: ['img', '#3ecf8e'], svg: ['svg', '#f2a33a'], webp: ['img', '#3ecf8e'], ico: ['img', '#3ecf8e'],
  bat: ['>', '#c1c1c1'], cmd: ['>', '#c1c1c1'], ps1: ['>', '#5391fe'], sh: ['>', '#89e051'],
  csv: ['csv', '#3ecf8e'], toml: ['cfg', '#9c4121'], yml: ['yml', '#cb171e'], yaml: ['yml', '#cb171e'], ini: ['cfg', '#9aa0a6'],
  c: ['C', '#5c6bc0'], h: ['h', '#5c6bc0'], cpp: ['C++', '#00599c'], java: ['J', '#e76f00'], cs: ['C#', '#68217a'], go: ['go', '#00add8'], rs: ['rs', '#dea584'],
};

export function fileBadge(name) {
  const ext = name.includes('.') ? name.split('.').pop().toLowerCase() : '';
  const [text, color] = badges[ext] || ['•', '#8a8a96'];
  return `<span class="badge" style="--c:${color}">${text}</span>`;
}

// ---------- ikony typov súborov (farebné, ako vo VS Code) ----------
const svg16 = (body) => `<svg class="ficon" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">${body}</svg>`;
const txt = (x, y, s, fill, size = 6.2) =>
  `<text x="${x}" y="${y}" fill="${fill}" font-family="Segoe UI, Arial, sans-serif" font-weight="800" font-size="${size}" text-anchor="middle">${s}</text>`;

const FILE_ICONS = {
  python:
    '<svg class="ficon" width="16" height="16" viewBox="0 0 32 32" aria-hidden="true">' +
    '<path d="M15.9 2.1c-7.1 0-6.7 3.1-6.7 3.1v3.2h6.8v1H6.5S2 8.8 2 16s4 6.9 4 6.9h2.3v-3.4s-.1-4 3.9-4H19s3.8.1 3.8-3.7V5.8s.6-3.7-6.9-3.7Zm-3.7 2.1a1.2 1.2 0 1 1 0 2.4 1.2 1.2 0 0 1 0-2.4Z" fill="#3B77A8"/>' +
    '<path d="M16.1 29.9c7.1 0 6.7-3.1 6.7-3.1v-3.2H16v-1h9.5S30 23.2 30 16s-4-6.9-4-6.9h-2.4v3.3s.1 4-3.9 4H13s-3.8-.1-3.8 3.7v6.1s-.6 3.7 6.9 3.7Zm3.7-2.1a1.2 1.2 0 1 1 0-2.4 1.2 1.2 0 0 1 0 2.4Z" fill="#FFD43B"/>' +
    '</svg>',
  html: svg16('<path d="M2 1h12l-1.1 12.4L8 15l-4.9-1.6Z" fill="#E44D26"/><path d="M8 2.1v11.8l3.9-1.3.9-10.5Z" fill="#F16529"/><path d="M5 5h6l-.1 1.4H6.5l.1 1.4h4.2l-.3 3.3L8 11.8l-2.5-.7-.2-1.8h1.3l.1.9 1.3.4 1.3-.4.2-1.5H5.3Z" fill="#fff"/>'),
  css: svg16('<path d="M2 1h12l-1.1 12.4L8 15l-4.9-1.6Z" fill="#1572B6"/><path d="M8 2.1v11.8l3.9-1.3.9-10.5Z" fill="#33A9DC"/><path d="M5 5h6l-.1 1.4H6.5l.1 1.4h4.2l-.3 3.3L8 11.8l-2.5-.7-.2-1.8h1.3l.1.9 1.3.4 1.3-.4.2-1.5H5.3Z" fill="#fff"/>'),
  scss: svg16('<circle cx="8" cy="8" r="6.5" fill="#CD6799"/>' + txt(8, 10.3, 'S', '#fff', 7.5)),
  javascript: svg16('<rect x="1.5" y="1.5" width="13" height="13" rx="2" fill="#F7DF1E"/>' + txt(9.6, 12.6, 'JS', '#222', 6.4)),
  jsx: svg16('<circle cx="8" cy="8" r="1.3" fill="#61DAFB"/><g fill="none" stroke="#61DAFB" stroke-width=".9"><ellipse cx="8" cy="8" rx="6.3" ry="2.4"/><ellipse cx="8" cy="8" rx="6.3" ry="2.4" transform="rotate(60 8 8)"/><ellipse cx="8" cy="8" rx="6.3" ry="2.4" transform="rotate(120 8 8)"/></g>'),
  typescript: svg16('<rect x="1.5" y="1.5" width="13" height="13" rx="2" fill="#3178C6"/>' + txt(9.6, 12.6, 'TS', '#fff', 6.4)),
  json: svg16('<path d="M5.5 2.5C4 2.5 4 3.5 4 4.5v1.3C4 6.8 3.3 7.6 2.5 8c.8.4 1.5 1.2 1.5 2.2v1.3c0 1 0 2 1.5 2M10.5 2.5c1.5 0 1.5 1 1.5 2v1.3c0 1 .7 1.8 1.5 2.2-.8.4-1.5 1.2-1.5 2.2v1.3c0 1 0 2-1.5 2" fill="none" stroke="#F5C542" stroke-width="1.4" stroke-linecap="round"/>'),
  markdown: svg16('<rect x="1" y="3.5" width="14" height="9" rx="1.8" fill="none" stroke="#519ABA" stroke-width="1.2"/><path d="M3.3 10.3V5.7l1.6 2 1.6-2v4.6M10.7 5.7v4.4M9 8.5l1.7 1.8 1.7-1.8" fill="none" stroke="#519ABA" stroke-width="1.2" stroke-linejoin="round" stroke-linecap="round"/>'),
  image: svg16('<rect x="1.5" y="2.5" width="13" height="11" rx="2" fill="#2E9E6B"/><circle cx="5.3" cy="6" r="1.4" fill="#C8F2DD"/><path d="M2.5 12.5l3.7-3.7 2.3 2.3 2.5-3 2.5 3v1.4Z" fill="#C8F2DD"/>'),
  svg: svg16('<rect x="1.5" y="1.5" width="13" height="13" rx="2" fill="#FFB13B"/><path d="M4 11.5c3-6 5-6 8-7" fill="none" stroke="#5a3a00" stroke-width="1.4" stroke-linecap="round"/><circle cx="4" cy="11.5" r="1.2" fill="#5a3a00"/><circle cx="12" cy="4.5" r="1.2" fill="#5a3a00"/>'),
  shell: svg16('<rect x="1.5" y="2.5" width="13" height="11" rx="2" fill="#3a3f4b"/><path d="M4 6l2.2 2L4 10M7.5 10.5H12" fill="none" stroke="#89E051" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>'),
  powershell: svg16('<path d="M3.5 2.5h11l-2 11h-11Z" fill="#2671BE"/><path d="M5.3 5.5l3 2.5-4 2.6M8.2 10.8h3" fill="none" stroke="#fff" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>'),
  text: svg16('<path d="M3.5 1.5h6l3 3v10h-9Z" fill="none" stroke="#9AA0A6" stroke-width="1.1" stroke-linejoin="round"/><path d="M5.5 7.5h5M5.5 9.7h5M5.5 11.9h3" stroke="#9AA0A6" stroke-width="1.1" stroke-linecap="round"/>'),
  config: svg16('<circle cx="8" cy="8" r="2.2" fill="none" stroke="#9AA0A6" stroke-width="1.3"/><path d="M8 1.8v2M8 12.2v2M1.8 8h2M12.2 8h2M3.6 3.6 5 5M11 11l1.4 1.4M3.6 12.4 5 11M11 5l1.4-1.4" stroke="#9AA0A6" stroke-width="1.3" stroke-linecap="round"/>'),
  c: svg16('<path d="M8 1 14 4.5v7L8 15l-6-3.5v-7Z" fill="#5C6BC0"/>' + txt(8, 10.4, 'C', '#fff', 7.5)),
  cpp: svg16('<path d="M8 1 14 4.5v7L8 15l-6-3.5v-7Z" fill="#00599C"/>' + txt(8, 10.1, 'C++', '#fff', 5.2)),
  csharp: svg16('<path d="M8 1 14 4.5v7L8 15l-6-3.5v-7Z" fill="#68217A"/>' + txt(8, 10.1, 'C#', '#fff', 6)),
  java: svg16('<path d="M5.5 12.5h5M6 14h4M8 2c1.5 1.5-1.5 2.5 0 4.5M10 3.5c1 1-1 1.8 0 3" fill="none" stroke="#E76F00" stroke-width="1.2" stroke-linecap="round"/><path d="M4 8h7.5v2A2.5 2.5 0 0 1 9 12.5H6.5A2.5 2.5 0 0 1 4 10Z" fill="#5382A1"/>'),
  go: svg16(txt(8, 11, 'GO', '#00ADD8', 7.5)),
  rust: svg16('<circle cx="8" cy="8" r="6.2" fill="none" stroke="#DEA584" stroke-width="1.6" stroke-dasharray="1.6 1"/>' + txt(8, 10.3, 'R', '#DEA584', 7)),
  git: svg16('<rect x="2.6" y="2.6" width="10.8" height="10.8" rx="2" transform="rotate(45 8 8)" fill="#F05033"/><path d="M6.2 5.2 8 7v3.6M8 7l2 2" stroke="#fff" stroke-width="1.1" fill="none" stroke-linecap="round"/><circle cx="8" cy="10.8" r=".9" fill="#fff"/><circle cx="10.1" cy="9.1" r=".9" fill="#fff"/>'),
  ruby: svg16('<path d="M4 2.5h8l2.5 3.5L8 14 1.5 6Z" fill="#CC342D"/><path d="M1.5 6h13M5.5 6 8 14l2.5-8M4 2.5 5.5 6 8 2.5l2.5 3.5L12 2.5" fill="none" stroke="#fff" stroke-opacity=".45" stroke-width=".7"/>'),
  php: svg16('<ellipse cx="8" cy="8" rx="7" ry="4.6" fill="#777BB4"/>' + txt(8, 10.2, 'php', '#fff', 5.6)),
  perl: svg16('<rect x="1.5" y="1.5" width="13" height="13" rx="3" fill="#39457E"/>' + txt(8, 11.4, 'pl', '#fff', 6.6)),
  lua: svg16('<circle cx="8" cy="8.5" r="5.5" fill="#000080"/><circle cx="10.2" cy="6.3" r="1.5" fill="#fff"/><circle cx="13.3" cy="2.8" r="1.5" fill="#000080"/>'),
  csv: svg16('<rect x="1.5" y="2" width="13" height="12" rx="1.8" fill="#1F9D55"/><path d="M1.5 6h13M1.5 10h13M6 2v12M10.5 2v12" stroke="#E6F7EE" stroke-width=".9"/>'),
  file: svg16('<path d="M3.5 1.5h6l3 3v10h-9Z" fill="none" stroke="#8A8A96" stroke-width="1.1" stroke-linejoin="round"/><path d="M9.5 1.5v3h3" fill="none" stroke="#8A8A96" stroke-width="1.1"/>'),
};

const EXT_ICON = {
  py: 'python', pyw: 'python', pyi: 'python', ipynb: 'python',
  html: 'html', htm: 'html', css: 'css', scss: 'scss', sass: 'scss', less: 'css',
  js: 'javascript', mjs: 'javascript', cjs: 'javascript', jsx: 'jsx', tsx: 'jsx', ts: 'typescript', mts: 'typescript',
  json: 'json', jsonc: 'json', md: 'markdown', markdown: 'markdown',
  png: 'image', jpg: 'image', jpeg: 'image', gif: 'image', webp: 'image', ico: 'image', bmp: 'image', avif: 'image', svg: 'svg',
  sh: 'shell', bash: 'shell', bat: 'shell', cmd: 'shell', ps1: 'powershell',
  txt: 'text', log: 'text', csv: 'csv', tsv: 'csv',
  toml: 'config', ini: 'config', cfg: 'config', yml: 'config', yaml: 'config', env: 'config', gitignore: 'config', lock: 'config',
  c: 'c', h: 'c', cpp: 'cpp', hpp: 'cpp', cc: 'cpp', cs: 'csharp', java: 'java', go: 'go', rs: 'rust', rb: 'ruby', php: 'php', lua: 'lua', pl: 'perl', pm: 'perl', cts: 'typescript', git: 'git',
};

const LANG_ICON = { python: 'python', html: 'html', css: 'css', javascript: 'javascript', typescript: 'typescript', json: 'json', markdown: 'markdown', shell: 'shell' };

// Ikona podľa prípony; pri súbore bez prípony podľa rozpoznaného jazyka.
export function fileIcon(name, lang) {
  const ext = name.includes('.') ? name.split('.').pop().toLowerCase() : '';
  const key = EXT_ICON[ext] || (!ext && LANG_ICON[lang]) || 'file';
  return FILE_ICONS[key];
}
