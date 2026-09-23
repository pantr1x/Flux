// Pomodoro Timer – 25 minút práce, 5 minút pauza. Časovač je v stavovom riadku.
export function activate(flux) {
  const focus = () => flux.storage.get('focus', 25);
  const pause = () => flux.storage.get('break', 5);
  let mode = 'focus';
  let left = focus() * 60;
  let running = false;
  let rounds = flux.storage.get('rounds', 0);

  const fmt = (s) => `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
  const item = flux.statusBar.add({ onClick: () => toggle() });
  const draw = () => {
    item.set(`${mode === 'focus' ? '🍅' : '☕'} ${fmt(left)}${running ? '' : ' ⏸'}`);
    item.setTitle(`Pomodoro – ${running ? 'click to pause' : 'click to start'} · ${rounds} done today`);
  };

  function toggle() {
    running = !running;
    draw();
  }

  function reset() {
    running = false;
    mode = 'focus';
    left = focus() * 60;
    draw();
  }

  flux.every(1000, () => {
    if (!running) return;
    left -= 1;
    if (left <= 0) {
      if (mode === 'focus') {
        rounds += 1;
        flux.storage.set('rounds', rounds);
      }
      mode = mode === 'focus' ? 'break' : 'focus';
      left = (mode === 'focus' ? focus() : pause()) * 60;
      const msg = mode === 'break' ? `Great work! Take a ${pause()} minute break ☕` : 'Break is over – back to coding 🍅';
      flux.toast(msg, 'ok', 10000);
      try {
        new Notification('Pomodoro', { body: msg });
      } catch {}
    }
    draw();
  });

  flux.commands.register('toggle', 'Start / pause timer', toggle, { key: 'Ctrl+Alt+T' });
  flux.commands.register('reset', 'Reset timer', reset);
  flux.commands.register('short', 'Use short sessions (15 / 3 min)', async () => {
    await flux.storage.set('focus', 15);
    await flux.storage.set('break', 3);
    reset();
    flux.toast('Pomodoro: 15 minutes focus, 3 minutes break.');
  });
  flux.commands.register('classic', 'Use classic sessions (25 / 5 min)', async () => {
    await flux.storage.set('focus', 25);
    await flux.storage.set('break', 5);
    reset();
    flux.toast('Pomodoro: 25 minutes focus, 5 minutes break.');
  });
  draw();
}
