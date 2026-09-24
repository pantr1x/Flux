// Theme Pack – 8 obľúbených farebných tém kódu. Objavia sa v Nastaveniach → Vzhľad → Téma kódu.
const THEMES = {
  gruvbox: {
    name: 'Gruvbox Dark',
    type: 'dark',
    colors: { fg: 'EBDBB2', comment: '928374', keyword: 'FB4934', storage: 'FE8019', string: 'B8BB26', number: 'D3869B', type: '8EC07C', function: 'FABD2F', variable: 'EBDBB2', parameter: '83A598', property: '83A598', constant: 'D3869B', tag: 'FB4934', attr: 'FABD2F', delimiter: 'A89984', regexp: '8EC07C' },
    italicComments: true,
  },
  'solarized-dark': {
    name: 'Solarized Dark',
    type: 'dark',
    colors: { fg: '93A1A1', comment: '657B83', keyword: '859900', storage: '268BD2', string: '2AA198', number: 'D33682', type: 'B58900', function: '268BD2', variable: '93A1A1', parameter: 'CB4B16', property: '6C71C4', constant: 'CB4B16', tag: '268BD2', attr: 'B58900', delimiter: '839496', regexp: 'DC322F' },
  },
  'solarized-light': {
    name: 'Solarized Light',
    type: 'light',
    colors: { fg: '586E75', comment: '93A1A1', keyword: '859900', storage: '268BD2', string: '2AA198', number: 'D33682', type: 'B58900', function: '268BD2', variable: '586E75', parameter: 'CB4B16', property: '6C71C4', constant: 'CB4B16', tag: '268BD2', attr: 'B58900', delimiter: '657B83', regexp: 'DC322F' },
  },
  'rose-pine': {
    name: 'Rosé Pine',
    type: 'dark',
    colors: { fg: 'E0DEF4', comment: '6E6A86', keyword: '3E8FB0', storage: '3E8FB0', string: 'F6C177', number: 'EBBCBA', type: '9CCFD8', function: 'EBBCBA', variable: 'E0DEF4', parameter: 'C4A7E7', property: 'C4A7E7', constant: 'EB6F92', tag: '9CCFD8', attr: 'C4A7E7', delimiter: '908CAA', regexp: 'F6C177' },
    italicComments: true,
  },
  'rose-pine-dawn': {
    name: 'Rosé Pine Dawn',
    type: 'light',
    colors: { fg: '575279', comment: '9893A5', keyword: '286983', storage: '286983', string: 'EA9D34', number: 'D7827E', type: '56949F', function: 'D7827E', variable: '575279', parameter: '907AA9', property: '907AA9', constant: 'B4637A', tag: '56949F', attr: '907AA9', delimiter: '797593', regexp: 'EA9D34' },
    italicComments: true,
  },
  'ayu-mirage': {
    name: 'Ayu Mirage',
    type: 'dark',
    colors: { fg: 'CCCAC2', comment: '6E7C8F', keyword: 'FFAD66', storage: 'FFAD66', string: 'D5FF80', number: 'DFBFFF', type: '73D0FF', function: 'FFD173', variable: 'CCCAC2', parameter: 'DFBFFF', property: '5CCFE6', constant: 'DFBFFF', tag: '5CCFE6', attr: 'FFD173', delimiter: 'CCCAC2', regexp: '95E6CB' },
    italicComments: true,
  },
  'night-owl': {
    name: 'Night Owl',
    type: 'dark',
    colors: { fg: 'D6DEEB', comment: '637777', keyword: 'C792EA', storage: 'C792EA', string: 'ECC48D', number: 'F78C6C', type: 'FFCB8B', function: '82AAFF', variable: 'D6DEEB', parameter: 'D7DBE0', property: '7FDBCA', constant: '82AAFF', tag: 'CAECE6', attr: 'C5E478', delimiter: '7FDBCA', regexp: '5CA7E4' },
    italicComments: true,
  },
  everforest: {
    name: 'Everforest',
    type: 'dark',
    colors: { fg: 'D3C6AA', comment: '859289', keyword: 'E67E80', storage: 'E69875', string: 'A7C080', number: 'D699B6', type: 'DBBC7F', function: '83C092', variable: 'D3C6AA', parameter: 'D3C6AA', property: '7FBBB3', constant: 'D699B6', tag: 'E67E80', attr: 'DBBC7F', delimiter: '9DA9A0', regexp: '83C092' },
    italicComments: true,
  },
};

export function activate(flux) {
  for (const [key, def] of Object.entries(THEMES)) flux.themes.add(key, def);
}
