// Farebné témy kódu. Každá téma určuje farby syntaxe (aj „sémantické“ farby z Pyrightu:
// funkcie, triedy, parametre…) a či je tmavá alebo svetlá – podľa toho sa prepne aj vzhľad okna.

export const THEMES = {
  'vscode-dark': {
    name: 'VS Code Dark',
    type: 'dark',
    t: { fg: 'D4D4D4', comment: '6A9955', keyword: 'C586C0', storage: '569CD6', string: 'CE9178', number: 'B5CEA8', type: '4EC9B0', function: 'DCDCAA', variable: '9CDCFE', parameter: '9CDCFE', property: '9CDCFE', constant: '4FC1FF', tag: '569CD6', attr: '9CDCFE', delimiter: 'D4D4D4', regexp: 'D16969' },
  },
  flux: {
    name: 'Flux',
    type: 'dark',
    t: { fg: 'E2E2EA', comment: '7A7A90', keyword: 'C792EA', storage: 'C792EA', string: 'A8DB8A', number: 'FFA86B', type: '7DD3FC', function: '82AAFF', variable: 'E2E2EA', parameter: 'F7C98B', property: 'B4C2F0', constant: 'FF9E64', tag: 'F7768E', attr: 'E0AF68', delimiter: 'A9B1D6', regexp: 'B4F9F8', italicComments: true },
  },
  'one-dark': {
    name: 'One Dark Pro',
    type: 'dark',
    t: { fg: 'ABB2BF', comment: '7F848E', keyword: 'C678DD', storage: 'C678DD', string: '98C379', number: 'D19A66', type: 'E5C07B', function: '61AFEF', variable: 'E06C75', parameter: 'D19A66', property: 'E06C75', constant: 'D19A66', tag: 'E06C75', attr: 'D19A66', delimiter: 'ABB2BF', regexp: '56B6C2', italicComments: true },
  },
  dracula: {
    name: 'Dracula',
    type: 'dark',
    t: { fg: 'F8F8F2', comment: '6272A4', keyword: 'FF79C6', storage: 'FF79C6', string: 'F1FA8C', number: 'BD93F9', type: '8BE9FD', function: '50FA7B', variable: 'F8F8F2', parameter: 'FFB86C', property: 'F8F8F2', constant: 'BD93F9', tag: 'FF79C6', attr: '50FA7B', delimiter: 'F8F8F2', regexp: 'FF5555' },
  },
  'tokyo-night': {
    name: 'Tokyo Night',
    type: 'dark',
    t: { fg: 'C0CAF5', comment: '565F89', keyword: 'BB9AF7', storage: '9D7CD8', string: '9ECE6A', number: 'FF9E64', type: '2AC3DE', function: '7AA2F7', variable: 'C0CAF5', parameter: 'E0AF68', property: '73DACA', constant: 'FF9E64', tag: 'F7768E', attr: 'BB9AF7', delimiter: '89DDFF', regexp: 'B4F9F8', italicComments: true },
  },
  catppuccin: {
    name: 'Catppuccin Mocha',
    type: 'dark',
    t: { fg: 'CDD6F4', comment: '7F849C', keyword: 'CBA6F7', storage: 'CBA6F7', string: 'A6E3A1', number: 'FAB387', type: 'F9E2AF', function: '89B4FA', variable: 'CDD6F4', parameter: 'EBA0AC', property: 'B4BEFE', constant: 'FAB387', tag: '89B4FA', attr: 'F9E2AF', delimiter: '9399B2', regexp: 'F5C2E7', italicComments: true },
  },
  nord: {
    name: 'Nord',
    type: 'dark',
    t: { fg: 'D8DEE9', comment: '616E88', keyword: '81A1C1', storage: '81A1C1', string: 'A3BE8C', number: 'B48EAD', type: '8FBCBB', function: '88C0D0', variable: 'D8DEE9', parameter: 'D8DEE9', property: 'D8DEE9', constant: '81A1C1', tag: '81A1C1', attr: '8FBCBB', delimiter: 'ECEFF4', regexp: 'EBCB8B', italicComments: true },
  },
  'github-dark': {
    name: 'GitHub Dark',
    type: 'dark',
    t: { fg: 'E6EDF3', comment: '8B949E', keyword: 'FF7B72', storage: 'FF7B72', string: 'A5D6FF', number: '79C0FF', type: 'FFA657', function: 'D2A8FF', variable: 'E6EDF3', parameter: 'FFA657', property: '79C0FF', constant: '79C0FF', tag: '7EE787', attr: '79C0FF', delimiter: 'E6EDF3', regexp: '7EE787' },
  },
  monokai: {
    name: 'Monokai',
    type: 'dark',
    t: { fg: 'F8F8F2', comment: '88846F', keyword: 'F92672', storage: '66D9EF', string: 'E6DB74', number: 'AE81FF', type: 'A6E22E', function: 'A6E22E', variable: 'F8F8F2', parameter: 'FD971F', property: 'F8F8F2', constant: 'AE81FF', tag: 'F92672', attr: 'A6E22E', delimiter: 'F8F8F2', regexp: 'E6DB74' },
  },
  'vscode-light': {
    name: 'VS Code Light',
    type: 'light',
    t: { fg: '1F1F1F', comment: '008000', keyword: 'AF00DB', storage: '0000FF', string: 'A31515', number: '098658', type: '267F99', function: '795E26', variable: '001080', parameter: '001080', property: '001080', constant: '0070C1', tag: '800000', attr: 'E50000', delimiter: '1F1F1F', regexp: '811F3F' },
  },
  'github-light': {
    name: 'GitHub Light',
    type: 'light',
    t: { fg: '1F2328', comment: '6E7781', keyword: 'CF222E', storage: 'CF222E', string: '0A3069', number: '0550AE', type: '953800', function: '8250DF', variable: '1F2328', parameter: '953800', property: '0550AE', constant: '0550AE', tag: '116329', attr: '0550AE', delimiter: '1F2328', regexp: '116329' },
  },
  'catppuccin-latte': {
    name: 'Catppuccin Latte',
    type: 'light',
    t: { fg: '4C4F69', comment: '8C8FA1', keyword: '8839EF', storage: '8839EF', string: '40A02B', number: 'FE640B', type: 'DF8E1D', function: '1E66F5', variable: '4C4F69', parameter: 'E64553', property: '7287FD', constant: 'FE640B', tag: '1E66F5', attr: 'DF8E1D', delimiter: '7C7F93', regexp: 'EA76CB', italicComments: true },
  },
};

export const DEFAULT_THEME = 'vscode-dark';

export function themeOf(id) {
  return THEMES[id] || THEMES[DEFAULT_THEME];
}

export function defineMonacoTheme(monaco, id, accent) {
  const theme = themeOf(id);
  const t = theme.t;
  const dark = theme.type === 'dark';
  const a = accent.replace('#', '');
  const rules = [
    { token: '', foreground: t.fg },
    { token: 'comment', foreground: t.comment, fontStyle: t.italicComments ? 'italic' : '' },
    { token: 'keyword', foreground: t.keyword },
    { token: 'keyword.flow', foreground: t.keyword },
    { token: 'string', foreground: t.string },
    { token: 'string.escape', foreground: t.string },
    { token: 'number', foreground: t.number },
    { token: 'regexp', foreground: t.regexp },
    { token: 'type', foreground: t.type },
    { token: 'type.identifier', foreground: t.type },
    { token: 'identifier', foreground: t.variable },
    { token: 'delimiter', foreground: t.delimiter },
    { token: 'tag', foreground: t.tag },
    { token: 'metatag', foreground: t.tag },
    { token: 'attribute.name', foreground: t.attr },
    { token: 'attribute.value', foreground: t.string },
    { token: 'attribute.value.number', foreground: t.number },
    { token: 'attribute.value.unit', foreground: t.number },
    { token: 'attribute.value.hex', foreground: t.number },
    // Sémantické farby (Pyright): rovnaké mená ako vo VS Code.
    { token: 'namespace', foreground: t.type },
    { token: 'class', foreground: t.type },
    { token: 'enum', foreground: t.type },
    { token: 'typeParameter', foreground: t.type },
    { token: 'function', foreground: t.function },
    { token: 'method', foreground: t.function },
    { token: 'decorator', foreground: t.function },
    { token: 'parameter', foreground: t.parameter },
    { token: 'selfParameter', foreground: t.storage, fontStyle: 'italic' },
    { token: 'clsParameter', foreground: t.storage, fontStyle: 'italic' },
    { token: 'variable', foreground: t.variable },
    { token: 'variable.readonly', foreground: t.constant },
    { token: 'property', foreground: t.property },
    { token: 'enumMember', foreground: t.constant },
    { token: 'builtinConstant', foreground: t.storage },
  ];
  // Monaco berie len písmená, čísla a pomlčky (témy z pluginov majú v id bodky).
  const monacoName = `flux-${String(id).replace(/[^a-z0-9-]/gi, '-')}`;
  monaco.editor.defineTheme(monacoName, {
    base: dark ? 'vs-dark' : 'vs',
    inherit: true,
    rules,
    colors: dark
      ? {
          'editor.background': '#00000000',
          'editor.foreground': `#${t.fg}`,
          'editorGutter.background': '#00000000',
          'editor.lineHighlightBackground': '#ffffff08',
          'editor.lineHighlightBorder': '#00000000',
          'editorLineNumber.foreground': '#6e6e7a',
          'editorLineNumber.activeForeground': '#c8c8d2',
          'editorCursor.foreground': accent,
          'editor.selectionBackground': `#${a}48`,
          'editor.inactiveSelectionBackground': `#${a}24`,
          'editor.wordHighlightBackground': '#ffffff12',
          'editorIndentGuide.background1': '#ffffff10',
          'editorIndentGuide.activeBackground1': '#ffffff2a',
          'editorWidget.background': '#303038',
          'editorWidget.border': '#ffffff18',
          'editorSuggestWidget.background': '#303038',
          'editorSuggestWidget.border': '#ffffff18',
          'editorSuggestWidget.selectedBackground': `#${a}40`,
          'editorSuggestWidget.highlightForeground': accent,
          'editorSuggestWidget.focusHighlightForeground': '#ffffff',
          'editorHoverWidget.background': '#303038',
          'editorHoverWidget.border': '#ffffff18',
          'scrollbarSlider.background': '#ffffff14',
          'scrollbarSlider.hoverBackground': '#ffffff22',
          'scrollbarSlider.activeBackground': '#ffffff2c',
          'editorOverviewRuler.border': '#00000000',
          focusBorder: '#00000000',
          'editorStickyScroll.background': '#2f2f37',
          'editorStickyScrollHover.background': '#383840',
          'editorGhostText.foreground': '#ffffff55',
          'minimap.background': '#2c2c34',
          'minimapSlider.background': '#ffffff14',
          'minimapSlider.hoverBackground': '#ffffff22',
        }
      : {
          'editor.background': '#00000000',
          'editor.foreground': `#${t.fg}`,
          'editorGutter.background': '#00000000',
          'editor.lineHighlightBackground': '#00000007',
          'editor.lineHighlightBorder': '#00000000',
          'editorLineNumber.foreground': '#a0a0ae',
          'editorLineNumber.activeForeground': '#3c3c48',
          'editorCursor.foreground': accent,
          'editor.selectionBackground': `#${a}38`,
          'editor.inactiveSelectionBackground': `#${a}1c`,
          'editorIndentGuide.background1': '#0000000e',
          'editorWidget.background': '#ffffff',
          'editorSuggestWidget.background': '#ffffff',
          'editorSuggestWidget.selectedBackground': `#${a}2a`,
          'editorSuggestWidget.highlightForeground': accent,
          'scrollbarSlider.background': '#00000014',
          'scrollbarSlider.hoverBackground': '#00000022',
          'editorOverviewRuler.border': '#00000000',
          focusBorder: '#00000000',
          'editorStickyScroll.background': '#f8f8fb',
          'minimap.background': '#f4f4f7',
        },
  });
  return monacoName;
}

// Malý náhľad témy do nastavení (farebné pásiky).
export function themeSwatch(id) {
  const t = themeOf(id).t;
  return [t.keyword, t.function, t.string, t.type, t.number].map((c) => `#${c}`);
}
