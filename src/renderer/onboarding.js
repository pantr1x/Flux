// Prvé spustenie (ako v Zen Browseri): animované logo → jazyk → čo programuješ → vzhľad → hotovo,
// potom voliteľná krátka prehliadka funkcií so zvýraznením častí okna.
import { t, setLocale } from './i18n.js';
import { THEMES, themeSwatch } from './themes.js';

const flux = window.flux;

const LOGO = `<svg class="ob-logo" viewBox="0 0 120 120" aria-hidden="true">
  <rect class="ob-logo-bg" x="6" y="6" width="108" height="108" rx="28"/>
  <path class="ob-draw" d="M44 38 22 60l22 22M76 38l22 22-22 22"/>
  <path class="ob-draw ob-slash" d="M67 30 53 90"/>
</svg>`;

const CODE_LANGS = [
  { id: 'python', label: 'Python', icon: 'a.py' },
  { id: 'web', label: 'HTML & CSS', icon: 'a.html' },
  { id: 'js', label: 'JavaScript', icon: 'a.js' },
  { id: 'explore', label: 'Just exploring', icon: null },
];

const LOOK_THEMES = ['vscode-dark', 'flux', 'tokyo-night', 'catppuccin', 'vscode-light', 'github-light'];

export function createOnboarding(app) {
  // app: { fileIcon, icon, setCodeTheme, setAccent, accents, accentHex, currentAccent, getSettings, saveSettings, toast }
  const el = document.getElementById('onboard');
  let step = 0;
  let languages = [{ code: 'en', name: 'English' }];
  let langChanged = false;

  function dots() {
    return `<div class="ob-dots">${[1, 2, 3, 4].map((i) => `<i class="${i === step ? 'on' : ''}"></i>`).join('')}</div>`;
  }

  function nav(nextLabel = t('Continue')) {
    return `<div class="ob-nav"><button class="ob-ghost" data-back>${t('Back')}</button>${dots()}<button class="ob-primary" data-next>${nextLabel}</button></div>`;
  }

  function render() {
    const s = app.getSettings();
    let body = '';
    if (step === 0) {
      body = `<div class="ob-splash">${LOGO}<h1 class="ob-title">flux</h1>
        <p class="ob-tag">${t('Code. Run. Create.')}</p>
        <button class="ob-primary ob-start" data-next>${t('Get started')}</button></div>`;
    } else if (step === 1) {
      body = `<div class="ob-step"><h2>${t('Choose your language')}</h2><p>${t('More languages are downloaded from GitHub, so Flux stays small.')}</p>
        <div class="ob-list">${languages
          .map((l) => `<button class="ob-option${(s.language || 'en') === l.code ? ' on' : ''}" data-lang="${l.code}"><b>${l.name}</b><small>${l.native || ''}</small></button>`)
          .join('')}</div></div>${nav()}`;
    } else if (step === 2) {
      const chosen = s.codeLangs || [];
      body = `<div class="ob-step"><h2>${t('What do you want to code?')}</h2><p>${t('Flux will show the right templates and buttons first. Pick as many as you like.')}</p>
        <div class="ob-grid">${CODE_LANGS.map(
          (c) =>
            `<button class="ob-card${chosen.includes(c.id) ? ' on' : ''}" data-code="${c.id}"><span class="ob-ic">${
              c.icon ? app.fileIcon(c.icon).replace(/width="16" height="16"/, 'width="34" height="34"') : app.icon('sparkle', 30)
            }</span><b>${t(c.label)}</b><i class="ob-check">${app.icon('check', 14)}</i></button>`,
        ).join('')}</div></div>${nav()}`;
    } else if (step === 3) {
      body = `<div class="ob-step"><h2>${t('Make it yours')}</h2><p>${t('You can change all of this later in Settings.')}</p>
        <div class="ob-themes">${LOOK_THEMES.map(
          (id) =>
            `<button class="ob-theme${(s.codeTheme || 'vscode-dark') === id ? ' on' : ''}" data-theme="${id}"><span class="swatch">${themeSwatch(id)
              .map((c) => `<i style="background:${c}"></i>`)
              .join('')}</span><b>${THEMES[id].name}</b></button>`,
        ).join('')}</div>
        <div class="ob-accents">${app.accents
          .map((a) => `<button data-accent="${a}" style="--c:${app.accentHex(a)}" class="${app.accentHex(a) === app.currentAccent() ? 'on' : ''}"></button>`)
          .join('')}</div></div>${nav()}`;
    } else {
      body = `<div class="ob-splash ob-done">${LOGO}<h1 class="ob-title">${t("You're all set!")}</h1>
        <p class="ob-tag">${t('Want a 30-second tour of the main features?')}</p>
        <div class="ob-row"><button class="ob-primary" data-tour>${t('Show me around')}</button><button class="ob-ghost" data-finish>${t('Start coding')}</button></div></div>`;
    }
    el.innerHTML = `<div class="ob-glow"></div><div class="ob-body" data-step="${step}">${body}</div>`;
  }

  async function finish(tour) {
    await app.saveSettings({ onboarded: true });
    el.classList.add('leaving');
    setTimeout(() => {
      el.hidden = true;
      el.classList.remove('leaving');
      if (langChanged) location.reload();
      else if (tour) startTour();
    }, 350);
  }

  el.onclick = async (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    if (b.dataset.next !== undefined) {
      step = Math.min(4, step + 1);
      if (step === 1) {
        flux.i18nList().then((list) => {
          if (Array.isArray(list) && list.length) languages = list;
          if (step === 1) render();
        });
      }
      return render();
    }
    if (b.dataset.back !== undefined) {
      step = Math.max(0, step - 1);
      return render();
    }
    if (b.dataset.lang) {
      try {
        b.classList.add('loading');
        const dict = await flux.i18nUse(b.dataset.lang);
        setLocale(dict);
        await app.saveSettings({ language: b.dataset.lang });
        langChanged = true;
      } catch {
        app.toast(t('Could not download the language. Check your internet connection.'), 'error');
      }
      return render();
    }
    if (b.dataset.code) {
      const set = new Set(app.getSettings().codeLangs || []);
      set.has(b.dataset.code) ? set.delete(b.dataset.code) : set.add(b.dataset.code);
      await app.saveSettings({ codeLangs: [...set] });
      return render();
    }
    if (b.dataset.theme) {
      await app.setCodeTheme(b.dataset.theme);
      return render();
    }
    if (b.dataset.accent) {
      await app.setAccent(b.dataset.accent);
      return render();
    }
    if (b.dataset.tour !== undefined) return finish(true);
    if (b.dataset.finish !== undefined) return finish(false);
  };

  function open() {
    step = 0;
    langChanged = false;
    render();
    el.hidden = false;
  }

  // ---------- prehliadka funkcií ----------
  const TOUR = [
    { sel: '#projects', title: 'Projects', text: 'All your projects in one place. Hover to pin one, right-click to rename it.' },
    { sel: '#essentials', title: 'Files', text: 'Create files from templates (Ctrl+N), folders and refresh the tree.' },
    { sel: '.brand', title: 'Start screen', text: 'Click the logo any time to pick what to build next.' },
    { sel: '#topbar .actions', title: 'Run & Live Server', text: 'F5 runs Python instantly. For websites you get a live preview that reloads on save.' },
    { sel: '#statusbar', title: 'Status bar', text: 'Python version, autocomplete and errors – click the error count to jump to a problem.' },
    { sel: '#btn-settings', title: 'Settings', text: 'Themes, colors, fonts, language and more. Tip: Ctrl+Shift+P finds any command.' },
  ];
  let tourStep = 0;
  const tour = document.getElementById('tour');

  function showTour() {
    const s = TOUR[tourStep];
    const target = document.querySelector(s.sel);
    if (!target || !target.getBoundingClientRect().width) {
      tourStep++;
      if (tourStep >= TOUR.length) return endTour();
      return showTour();
    }
    const r = target.getBoundingClientRect();
    const pad = 6;
    tour.innerHTML = `<div class="tour-spot" style="left:${r.left - pad}px;top:${r.top - pad}px;width:${r.width + pad * 2}px;height:${r.height + pad * 2}px"></div>
      <div class="tour-tip"><small>${tourStep + 1} / ${TOUR.length}</small><h3>${t(s.title)}</h3><p>${t(s.text)}</p>
      <div class="tour-nav"><button class="ob-ghost" data-skip>${t('Skip')}</button><button class="ob-primary" data-tnext>${tourStep === TOUR.length - 1 ? t('Finish') : t('Next')}</button></div></div>`;
    const tip = tour.querySelector('.tour-tip');
    const tw = 300;
    let left = r.right + 16;
    let top = r.top;
    if (left + tw > innerWidth - 12) left = Math.max(12, r.left - tw - 16);
    if (left < 12 || r.width > innerWidth / 2) {
      left = Math.min(innerWidth - tw - 12, Math.max(12, r.left));
      top = r.bottom + 14;
    }
    if (top + 170 > innerHeight) top = Math.max(12, r.top - 184);
    tip.style.left = `${left}px`;
    tip.style.top = `${top}px`;
  }

  function startTour() {
    tourStep = 0;
    tour.hidden = false;
    showTour();
  }

  function endTour() {
    tour.hidden = true;
    tour.innerHTML = '';
  }

  tour.onclick = (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    if (b.dataset.skip !== undefined) return endTour();
    if (b.dataset.tnext !== undefined) {
      tourStep++;
      if (tourStep >= TOUR.length) return endTour();
      showTour();
    }
  };

  return { open, startTour };
}
