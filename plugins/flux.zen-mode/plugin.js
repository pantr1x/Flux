// Zen Mode – len ty a tvoj kód: skryje bočný panel, výstup aj lišty.
export function activate(flux) {
  flux.ui.addStyle(`
    body.zen-mode #sidebar, body.zen-mode #side-resizer, body.zen-mode #panel, body.zen-mode #panel-resizer,
    body.zen-mode #statusbar, body.zen-mode #tabs, body.zen-mode #peek-zone { display: none !important; }
    body.zen-mode #main { padding: 0 !important; }
    body.zen-mode #card { border-radius: 0 !important; box-shadow: none !important; }
    body.zen-mode #editor-wrap { padding: 4vh max(4vw, calc((100% - 980px) / 2)) 0; }
    body.zen-mode #topbar { opacity: 0; transition: opacity .2s; }
    body.zen-mode #topbar:hover { opacity: 1; }
    body.zen-mode .monaco-editor .line-numbers { opacity: .35; }
  `);
  let on = false;
  const toggle = () => {
    on = !on;
    document.body.classList.toggle('zen-mode', on);
    window.dispatchEvent(new Event('resize'));
    if (on) flux.toast('Zen mode – press Ctrl+Alt+Z or Esc to leave.', 'info', 3000);
  };
  flux.commands.register('toggle', 'Toggle Zen mode', toggle, { key: 'Ctrl+Alt+Z' });
  // Esc zachytíme skôr ako editor (capture), inak by ho editor „zjedol“.
  const onKey = (e) => {
    if (on && e.key === 'Escape') toggle();
  };
  window.addEventListener('keydown', onKey, true);
  flux.ui.toggleClass('zen-mode', false);
}

export function deactivate() {
  document.body.classList.remove('zen-mode');
}
