// Vlajky jazykov ako malé SVG (Windows nevie zobraziť vlajkové emoji).
const W = 30;
const H = 20;
const stripesH = (colors) => colors.map((c, i) => `<rect y="${(H / colors.length) * i}" width="${W}" height="${H / colors.length}" fill="${c}"/>`).join('');
const stripesV = (colors) => colors.map((c, i) => `<rect x="${(W / colors.length) * i}" width="${W / colors.length}" height="${H}" fill="${c}"/>`).join('');

const FLAGS = {
  en:
    '<rect width="30" height="20" fill="#012169"/><path d="M0 0L30 20M30 0L0 20" stroke="#fff" stroke-width="4"/><path d="M0 0L30 20M30 0L0 20" stroke="#c8102e" stroke-width="1.6"/><path d="M15 0v20M0 10h30" stroke="#fff" stroke-width="6"/><path d="M15 0v20M0 10h30" stroke="#c8102e" stroke-width="3.4"/>',
  sk:
    stripesH(['#fff', '#0b4ea2', '#ee1c25']) +
    '<path d="M7 4.2h8.4v6.6c0 3-2.2 4.5-4.2 5.4C9.2 15.3 7 13.8 7 10.8z" fill="#ee1c25" stroke="#fff" stroke-width=".7"/><path d="M11.2 6v7.8M9.4 8h3.6M8.8 10h4.8" stroke="#fff" stroke-width="1"/><path d="M7.6 13c1-1 1.8-1 2.4-.2.6-.8 1.8-.8 2.4 0 .6-.8 1.6-.6 2.2.2-.8 1.5-2.2 2.4-3.4 3-1.3-.5-2.8-1.5-3.6-3z" fill="#0b4ea2"/>',
  de: stripesH(['#000', '#dd0000', '#ffce00']),
  es: '<rect width="30" height="20" fill="#aa151b"/><rect y="5" width="30" height="10" fill="#f1bf00"/>',
  fr: stripesV(['#0055a4', '#fff', '#ef4135']),
  it: stripesV(['#009246', '#fff', '#ce2b37']),
  pl: stripesH(['#fff', '#dc143c']),
};

// Neznámy jazyk: sivý obdĺžnik s kódom.
export function flag(code, size = 20) {
  const inner = FLAGS[code] || `<rect width="30" height="20" fill="#667"/><text x="15" y="14" font-size="10" text-anchor="middle" fill="#fff" font-family="sans-serif">${String(code).slice(0, 2).toUpperCase()}</text>`;
  return `<svg class="flag" viewBox="0 0 ${W} ${H}" width="${size}" height="${Math.round((size * H) / W)}" aria-hidden="true"><clipPath id="fc-${code}"><rect width="30" height="20" rx="3"/></clipPath><g clip-path="url(#fc-${code})">${inner}</g></svg>`;
}
