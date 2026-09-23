// Save Confetti – malá oslava pri každom uložení. 🎉 (ukážkový komunitný plugin)
export function activate(flux) {
  flux.ui.addStyle(`
    .sc-bit { position: fixed; z-index: 9999; pointer-events: none; font-size: 18px; animation: sc-fly 900ms cubic-bezier(.2,.7,.3,1) forwards; }
    @keyframes sc-fly { to { transform: translate(var(--dx), var(--dy)) rotate(var(--r)); opacity: 0; } }
  `);
  const EMOJI = ['🎉', '✨', '💾', '⭐', '🎊'];
  flux.onSave(() => {
    if (!flux.storage.get('on', true)) return;
    const rect = document.querySelector('.tab.active')?.getBoundingClientRect() || { left: innerWidth / 2, top: 40, width: 0 };
    for (let i = 0; i < 14; i++) {
      const el = document.createElement('span');
      el.className = 'sc-bit';
      el.textContent = EMOJI[i % EMOJI.length];
      el.style.left = `${rect.left + rect.width / 2}px`;
      el.style.top = `${rect.top + 10}px`;
      el.style.setProperty('--dx', `${(Math.random() - 0.5) * 260}px`);
      el.style.setProperty('--dy', `${40 + Math.random() * 160}px`);
      el.style.setProperty('--r', `${(Math.random() - 0.5) * 540}deg`);
      document.body.append(el);
      setTimeout(() => el.remove(), 1000);
    }
  });
  flux.commands.register('toggle', 'Turn confetti on / off', async () => {
    const on = !flux.storage.get('on', true);
    await flux.storage.set('on', on);
    flux.toast(on ? 'Confetti on 🎉' : 'Confetti off');
  });
}
