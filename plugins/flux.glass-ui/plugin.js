// Glass UI – priesvitnejšie „sklenené“ panely, jemná žiara akcentovej farby a mäkšie tiene.
// Príkaz „Glass UI: strength“ prepína slabé / stredné / silné sklo.
const LEVELS = { light: 0.62, medium: 0.48, strong: 0.34 };

export function activate(flux) {
  let level = flux.storage.get('level', 'medium');
  let style = null;
  const apply = () => {
    style?.remove();
    const a = LEVELS[level] ?? LEVELS.medium;
    style = flux.ui.addStyle(`
      body.theme-dark { --card: rgba(34, 34, 44, ${a}); --card-2: rgba(255, 255, 255, 0.055); --line: rgba(255, 255, 255, 0.09); --line-strong: rgba(255, 255, 255, 0.15); }
      body.theme-light { --card: rgba(255, 255, 255, ${a + 0.1}); }
      #card, #settings > *, .np-card, #palette, .pv-pop, .toast, .fsel-menu {
        backdrop-filter: blur(24px) saturate(1.5);
      }
      #card {
        box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.1), inset 0 1px 0 rgba(255, 255, 255, 0.07), 0 24px 60px rgba(0, 0, 0, 0.35) !important;
      }
      .run-btn:not(:disabled), .ob-primary, .s-btn.primary {
        box-shadow: 0 0 22px color-mix(in srgb, var(--accent) 45%, transparent);
      }
      .pr-row.active, .tab.active, .s-tab.on {
        box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 45%, transparent), 0 0 18px color-mix(in srgb, var(--accent) 18%, transparent);
      }
      .monaco-editor .cursor { box-shadow: 0 0 8px var(--accent); }
    `);
  };
  apply();
  flux.commands.register('strength', 'Glass UI: strength (light / medium / strong)', async () => {
    const order = Object.keys(LEVELS);
    level = order[(order.indexOf(level) + 1) % order.length];
    await flux.storage.set('level', level);
    apply();
    flux.toast(`Glass UI: ${level}`, 'ok', 1800);
  });
}
