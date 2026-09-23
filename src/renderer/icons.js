// Jednoduché čiarové ikony (štýl Lucide), 24×24.
const paths = {
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
  x: '<path d="M6 6l12 12M18 6 6 18"/>',
  sidebar: '<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M9 4v16"/>',
  panel: '<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M3 14h18"/>',
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
  python: '<path d="M12 3c-4 0-4 1.8-4 3v2h4.2v1H6c-2 0-3 1.4-3 4s1 4 3 4h1.8v-2.5c0-1.6 1.3-2.9 3-2.9h4.3c1.3 0 2.4-1.1 2.4-2.4V6c0-1.6-1.8-3-5.5-3Z"/><path d="M12 21c4 0 4-1.8 4-3v-2h-4.2v-1H18c2 0 3-1.4 3-4s-1-4-3-4h-1.8"/><circle cx="10" cy="5.8" r=".6" fill="currentColor"/><circle cx="14" cy="18.2" r=".6" fill="currentColor"/>',
  sparkle: '<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8Z"/><path d="M19 17l.7 1.8 1.8.7-1.8.7L19 22l-.7-1.8-1.8-.7 1.8-.7Z"/>',
  save: '<path d="M5 3h11l3 3v13a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2Z"/><path d="M8 3v5h7M8 21v-7h8v7"/>',
};

export function icon(name, size = 16) {
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
