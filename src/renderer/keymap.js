// Mapa skratiek: každá vstavaná akcia má predvolenú skratku, ktorú si môžeš zmeniť
// (Nastavenia → Skratky → klikni na skratku a stlač novú). Zmeny sú v nastaveniach „keymap“.
import { t } from './i18n.js';
import { parseKey } from './userShortcuts.js';

// [id, kategória, popis, predvolená skratka, akcia editora (Monaco) – ak je to editorová skratka]
export const BINDINGS = [
  ['run', 'run', 'Run current file', 'F5'],
  ['stop', 'run', 'Stop program', 'Shift+F5'],
  ['live', 'run', 'Live Server: start / stop', 'Alt+L'],
  ['newFile', 'files', 'New file…', 'Ctrl+N'],
  ['newProject', 'files', 'New project…', 'Ctrl+Shift+N'],
  ['openFolder', 'files', 'Open folder…', 'Ctrl+O'],
  ['openFile', 'files', 'Open file…', 'Ctrl+Alt+O'],
  ['quickOpen', 'files', 'Quick open file…', 'Ctrl+P'],
  ['save', 'files', 'Save', 'Ctrl+S'],
  ['saveAll', 'files', 'Save all', 'Ctrl+Shift+S'],
  ['closeFile', 'files', 'Close file', 'Ctrl+W'],
  ['searchAll', 'view', 'Search everything', 'Ctrl+Shift+A'],
  ['palette', 'view', 'Commands', 'Ctrl+Shift+P'],
  ['settings', 'view', 'Settings', 'Ctrl+,'],
  ['home', 'view', 'Home screen', ''],
  ['projectPage', 'view', 'Project page', ''],
  ['sidebar', 'view', 'Toggle sidebar (compact mode)', 'Ctrl+B'],
  ['focus', 'view', 'Focus mode (only the code)', 'F11'],
  ['panel', 'view', 'Toggle output panel', 'Ctrl+J'],
  ['ai', 'view', 'AI assistant', 'Ctrl+I'],
  ['zoomIn', 'view', 'Increase font size', 'Ctrl+='],
  ['zoomOut', 'view', 'Decrease font size', 'Ctrl+-'],
  ['theme', 'view', 'Toggle light / dark theme', ''],
  ['mdPreview', 'view', 'Markdown: show preview', 'Ctrl+Shift+V'],
  ['find', 'editor', 'Find', 'Ctrl+F', 'actions.find'],
  ['replace', 'editor', 'Find and replace', 'Ctrl+H', 'editor.action.startFindReplaceAction'],
  ['commentLine', 'editor', 'Comment / uncomment line', 'Ctrl+/', 'editor.action.commentLine'],
  ['addComment', 'editor', 'Add a comment above the line', 'Ctrl+Alt+/', 'flux.addComment'],
  ['rename', 'editor', 'Rename symbol everywhere', 'F2', 'editor.action.rename'],
  ['selectNext', 'editor', 'Select next match', 'Ctrl+D', 'editor.action.addSelectionToNextFindMatch'],
  ['moveUp', 'editor', 'Move line up', 'Alt+Up', 'editor.action.moveLinesUpAction'],
  ['moveDown', 'editor', 'Move line down', 'Alt+Down', 'editor.action.moveLinesDownAction'],
  ['copyDown', 'editor', 'Copy line down', 'Shift+Alt+Down', 'editor.action.copyLinesDownAction'],
  ['deleteLine', 'editor', 'Delete line', 'Ctrl+Shift+K', 'editor.action.deleteLines'],
  ['gotoLine', 'editor', 'Go to line', 'Ctrl+G', 'editor.action.gotoLine'],
  ['suggest', 'editor', 'Show suggestions', 'Ctrl+Space', 'editor.action.triggerSuggest'],
  ['definition', 'editor', 'Go to definition', 'F12', 'editor.action.revealDefinition'],
  ['format', 'editor', 'Format document', 'Shift+Alt+F', 'editor.action.formatDocument'],
  ['fold', 'editor', 'Fold block', 'Ctrl+Shift+[', 'editor.fold'],
  ['unfold', 'editor', 'Unfold block', 'Ctrl+Shift+]', 'editor.unfold'],
].map(([id, cat, label, key, editorAction]) => ({ id, cat, label, key, editorAction }));

export const CATEGORIES = [
  ['run', 'Running'],
  ['files', 'Files'],
  ['view', 'Window & view'],
  ['editor', 'Editor'],
  ['custom', 'Your own shortcuts'],
  ['plugins', 'Plugins'],
  ['more', 'More editor commands'],
];

const CODE_KEYS = { Slash: '/', Backslash: '\\', Comma: ',', Period: '.', Semicolon: ';', Quote: "'", BracketLeft: '[', BracketRight: ']', Minus: '-', Equal: '=', Backquote: '`', Space: 'Space' };
const ARROWS = { ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right' };

// Stlačené klávesy → „Ctrl+Shift+K“ (podľa fyzickej klávesy, funguje aj so slovenskou klávesnicou).
export function comboFromEvent(e) {
  if (['Control', 'Shift', 'Alt', 'Meta', 'AltGraph'].includes(e.key)) return null;
  let key;
  if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
  else if (/^Digit\d$/.test(e.code)) key = e.code.slice(5);
  else if (/^Numpad\d$/.test(e.code)) key = e.code.slice(6);
  else if (CODE_KEYS[e.code]) key = CODE_KEYS[e.code];
  else if (ARROWS[e.key]) key = ARROWS[e.key];
  else if (/^F\d{1,2}$/.test(e.key)) key = e.key;
  else key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const mods = [e.ctrlKey && 'Ctrl', e.shiftKey && 'Shift', e.altKey && 'Alt', e.metaKey && 'Win'].filter(Boolean);
  // Bez modifikátora len F-klávesy (inak by písmeno prestalo písať).
  if (!mods.length && !/^F\d{1,2}$/.test(key)) return null;
  return [...mods, key].join('+');
}

function same(e, combo) {
  const k = parseKey(combo.replace(/\bSpace\b/, 'space'));
  if (!k) return false;
  return comboFromEvent(e)?.toLowerCase() === combo.toLowerCase() || (e.ctrlKey === k.ctrl && e.shiftKey === k.shift && e.altKey === k.alt && e.metaKey === k.meta && e.key.toLowerCase() === k.key);
}

// getBindings: vstavané + všetky príkazy editora + pluginy + vlastné (shortcuts.json).
// Riadok s b.save(combo) si skratku ukladá sám (vlastné skratky → shortcuts.json).
export function createKeymap({ getOverrides, saveOverrides, runAction, toast, getBindings = () => BINDINGS }) {
  const current = (b) => (b.save ? b.key : getOverrides()[b.id] ?? b.key);

  // Zmenené skratky: nová kombinácia spustí akciu, stará (predvolená) už nič nerobí.
  window.addEventListener(
    'keydown',
    (e) => {
      if (window.fluxRecordingKeys) return; // práve nahrávaš novú skratku
      const over = getOverrides();
      const ids = Object.keys(over);
      if (!ids.length || e.repeat) return;
      const all = getBindings().filter((b) => !b.save);
      for (const b of all) {
        if (!(b.id in over)) continue;
        if (over[b.id] && same(e, over[b.id])) {
          e.preventDefault();
          e.stopImmediatePropagation();
          return runAction(b);
        }
      }
      for (const b of all) {
        if (!(b.id in over) || !b.key || !same(e, b.key)) continue;
        // Predvolenú kombináciu nepoužíva nič iné → pohltiť, aby nezbehla stará akcia.
        const usedElsewhere = all.some((o) => o !== b && current(o) && same(e, current(o)));
        if (!usedElsewhere) {
          e.preventDefault();
          e.stopImmediatePropagation();
        }
        return;
      }
    },
    true,
  );

  // Nahrávanie novej skratky na riadku v nastaveniach.
  function record(button, b, onDone) {
    window.fluxRecordingKeys = true;
    button.classList.add('recording');
    button.innerHTML = `<span class="rec">${t('Press the new shortcut…')}</span>`;
    const onKey = async (e) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.key === 'Escape') return stop();
      if (e.key === 'Backspace' || e.key === 'Delete') {
        if (b.save) {
          await b.save('');
          return stop(true);
        }
        const over = { ...getOverrides() };
        delete over[b.id];
        await saveOverrides(over);
        return stop(true);
      }
      const combo = comboFromEvent(e);
      if (!combo) return;
      const clash = getBindings().find((o) => o.id !== b.id && current(o) && current(o).toLowerCase() === combo.toLowerCase());
      if (clash) toast(t('{keys} was used for “{action}” – that one now has no shortcut.', { keys: combo, action: t(clash.label) }), 'info', 6000);
      if (clash?.save) await clash.save('');
      if (b.save) await b.save(combo);
      const over = { ...getOverrides() };
      if (!b.save) over[b.id] = combo;
      if (clash && !clash.save) over[clash.id] = '';
      await saveOverrides(over);
      stop(true);
    };
    const stop = (changed) => {
      window.fluxRecordingKeys = false;
      window.removeEventListener('keydown', onKey, true);
      button.classList.remove('recording');
      onDone(changed);
    };
    window.addEventListener('keydown', onKey, true);
  }

  return { current, record, isChanged: (b) => !b.save && b.id in getOverrides(), overridden: (id) => id in getOverrides() };
}

export function kbdHtml(combo, esc) {
  if (!combo) return `<span class="kbd-none">${t('none')}</span>`;
  return combo
    .split('+')
    .filter(Boolean)
    .map((x) => `<kbd>${esc(x)}</kbd>`)
    .join('<i>+</i>');
}
