// Prvé spustenie (ako v Zen Browseri): animované logo → jazyk → čo programuješ → vzhľad → hotovo,
// potom voliteľná krátka prehliadka funkcií so zvýraznením častí okna.
import { flag } from './flags.js';
import { t, setLocale } from './i18n.js';
import { THEMES, themeSwatch } from './themes.js';

const flux = window.flux;

const LOGO = `<svg class="ob-logo" viewBox="0 0 120 120" aria-hidden="true">
  <rect class="ob-logo-bg" x="6" y="6" width="108" height="108" rx="28"/>
  <path class="ob-draw" d="M44 38 22 60l22 22M76 38l22 22-22 22"/>
  <path class="ob-draw ob-slash" d="M67 30 53 90"/>
</svg>`;

const CODE_LANGS = [
  { id: 'python', label: 'Python', icon: 'a.py', tool: 'python' },
  { id: 'web', label: 'HTML & CSS', icon: 'a.html' },
  { id: 'js', label: 'JavaScript', icon: 'a.js', tool: 'node' },
  { id: 'java', label: 'Java', icon: 'a.java', tool: 'java' },
  { id: 'cpp', label: 'C / C++', icon: 'a.cpp', tool: 'cpp' },
  { id: 'csharp', label: 'C#', icon: 'a.cs', tool: 'csharp' },
  { id: 'go', label: 'Go', icon: 'a.go', tool: 'go' },
  { id: 'rust', label: 'Rust', icon: 'a.rs', tool: 'rust', more: true },
  { id: 'ruby', label: 'Ruby', icon: 'a.rb', tool: 'ruby', more: true },
  { id: 'php', label: 'PHP', icon: 'a.php', tool: 'php', more: true },
  { id: 'lua', label: 'Lua', icon: 'a.lua', tool: 'lua', more: true },
  { id: 'explore', label: 'Just exploring', icon: null, more: true },
];

// Ukážka kódu pri výbere vzhľadu (farby sa menia podľa zvolenej témy).
const PREVIEW_CODE = `# Guess the number
import random

def play(name: str) -> int:
    secret = random.randint(1, 10)
    tries = 0
    while True:
        tries += 1
        if int(input("Guess: ")) == secret:
            print(f"Well done, {name}!")
            return tries`;

// Ktoré vybrané jazyky ešte treba stiahnuť.
function chosenTools(app) {
  const chosen = app.getSettings().codeLangs || [];
  return [...new Set(CODE_LANGS.filter((c) => chosen.includes(c.id) && c.tool).map((c) => c.tool))];
}
function missingTools(app, includeFinished = false) {
  const list = chosenTools(app).map((id) => app.tools.info(id)).filter(Boolean);
  // Počas inštalácie ostanú v zozname aj tie, ktoré sa medzitým dokončili (s fajkou).
  return includeFinished ? list.filter((tc) => !tc.installed || installedHere.has(tc.id)) : list.filter((tc) => !tc.installed);
}
const installedHere = new Set();

const LOOK_THEMES = ['vscode-dark', 'flux', 'tokyo-night', 'catppuccin', 'vscode-light', 'github-light'];

export function createOnboarding(app) {
  // app: { fileIcon, icon, setCodeTheme, setAccent, setDarkLift, accents, accentHex, currentAccent, getSettings, saveSettings, toast }
  const el = document.getElementById('onboard');
  app.tools.bind(el);
  let step = 0;
  let languages = [{ code: 'en', name: 'English' }];
  let langChanged = false;
  let moreLangs = false;

  function dots() {
    return `<div class="ob-dots">${[1, 2, 3, 4].map((i) => `<i class="${i === step ? 'on' : ''}"></i>`).join('')}</div>`;
  }

  function nav(nextLabel = t('Continue')) {
    return `<div class="ob-nav"><button class="ob-ghost" data-back>${t('Back')}</button>${dots()}<button class="ob-primary" data-next>${nextLabel}</button></div>`;
  }

  function cardStatus(c) {
    const tc = c.tool && app.tools.info(c.tool);
    if (!tc) return c.tool ? '<small class="ob-st">&nbsp;</small>' : '<small class="ob-st">&nbsp;</small>';
    return tc.installed
      ? `<small class="ob-st ok">${t('Installed')}</small>`
      : `<small class="ob-st">${t('{size} download', { size: app.tools.mb(tc.download) })}</small>`;
  }

  function langList() {
    const cur = app.getSettings().language || 'en';
    return `<div class="ob-list">${languages
      .map((l) => `<button class="ob-option${cur === l.code ? ' on' : ''}" data-lang="${l.code}">${flag(l.code, 26)}<span class="ob-lang-txt"><b>${l.native || l.name}</b><small>${l.name}</small></span></button>`)
      .join('')}</div>`;
  }

  function body() {
    const s = app.getSettings();
    if (step === 0) {
      return `<div class="ob-splash">${LOGO}<h1 class="ob-title">flux</h1>
        <p class="ob-tag">${t('Code. Run. Create.')}</p>
        <button class="ob-primary ob-start" data-next>${t('Get started')}</button></div>`;
    }
    if (step === 1) {
      return `<div class="ob-step"><h2>${t('Choose your language')}</h2><p>${t('More languages are downloaded from GitHub, so Flux stays small.')}</p>
        ${langList()}</div>${nav()}`;
    }
    if (step === 2) {
      const chosen = s.codeLangs || [];
      // Ďalšie jazyky sa ukážu po kliknutí na „More languages“ (alebo keď už nejaký z nich vybral).
      const showMore = moreLangs || CODE_LANGS.some((c) => c.more && c.id !== 'explore' && chosen.includes(c.id));
      const list = CODE_LANGS.filter((c) => showMore || !c.more);
      return `<div class="ob-step"><h2>${t('What do you want to code?')}</h2><p>${t('Flux will show the right templates and buttons first. Pick as many as you like.')}</p>
        <div class="ob-grid${showMore ? ' more' : ''}">${list
          .map(
            (c) =>
              `<button class="ob-card${chosen.includes(c.id) ? ' on' : ''}" data-code="${c.id}"><span class="ob-ic">${
                c.icon ? app.fileIcon(c.icon).replace(/width="16" height="16"/, 'width="34" height="34"') : app.icon('sparkle', 30)
              }</span><b>${t(c.label)}</b>${cardStatus(c)}<i class="ob-check">${app.icon('check', 14)}</i></button>`,
          )
          .join('')}${showMore ? '' : `<button class="ob-card ob-more" data-more-langs><span class="ob-ic">${app.icon('plus', 28)}</span><b>${t('More languages')}</b><small class="ob-st">Rust, Ruby, PHP, Lua…</small></button>`}</div></div>${nav()}`;
    }
    if (step === 3) {
      return `<div class="ob-step ob-look"><h2>${t('Make it yours')}</h2><p>${t('You can change all of this later in Settings.')}</p>
        <div class="ob-look-cols"><div class="ob-look-pick">
        <div class="ob-themes">${LOOK_THEMES.map(
          (id) =>
            `<button class="ob-theme${(s.codeTheme || 'vscode-dark') === id ? ' on' : ''}" data-theme="${id}"><span class="swatch">${themeSwatch(id)
              .map((c) => `<i style="background:${c}"></i>`)
              .join('')}</span><b>${THEMES[id].name}</b></button>`,
        ).join('')}</div>
        <div class="ob-accents">${app.accents
          .map((a) => `<button data-accent="${a}" style="--c:${app.accentHex(a)}" class="${app.accentHex(a) === app.currentAccent() ? 'on' : ''}"></button>`)
          .join('')}</div>
        <label class="ob-lift"><span><b>${t('Brightness of dark areas')}</b><small>${t('Turn it up if your wallpaper is very dark.')}</small></span><input type="range" min="0" max="100" data-lift value="${Number(s.darkLift) || 0}"></label></div>
        <div class="ob-preview" aria-hidden="true">
          <div class="obp-bar"><i></i><i></i><i></i><span class="obp-tab">${app.fileIcon('main.py')}main.py</span><span class="obp-run">${app.icon('play', 11)}${t('Run')}</span></div>
          <pre class="obp-code"></pre>
        </div></div></div>${nav()}`;
    }
    if (step === 4) {
      // Inštalácia vybraných jazykov – beží na pozadí, dá sa preskočiť.
      const miss = missingTools(app, true);
      const total = miss.reduce((a, tc) => a + tc.download, 0);
      const can = app.tools.canInstall();
      return `<div class="ob-step"><h2>${can ? t('Installing your tools') : t('Get your tools')}</h2><p>${
        can
          ? t('Flux is downloading your languages from their official sources (about {size}). You can skip this – it keeps installing in the background and you can already code in everything else.', { size: app.tools.mb(total) })
          : t('These languages are not on your computer yet. Flux downloads them from their official sources – about {size} in total. You can also do this later.', { size: app.tools.mb(total) })
      }</p>
        <div class="tc-list ob-tools">${miss.map((tc) => app.tools.row(tc)).join('')}</div></div>
        <div class="ob-nav"><button class="ob-ghost" data-back>${t('Back')}</button>${dots()}<button class="ob-primary" data-next>${can ? t('Skip') : t('Continue')}</button></div>`;
    }
    return `<div class="ob-splash ob-done">${LOGO}<h1 class="ob-title">${t("You're all set!")}</h1>
        <p class="ob-tag">${t('Want a 30-second tour of the main features?')}</p>
        ${app.tools.busy() ? `<p class="ob-note"><span class="spin"></span>${t('Your languages keep installing in the background.')}</p>` : ''}
        <div class="ob-row"><button class="ob-primary" data-tour>${t('Show me around')}</button><button class="ob-ghost" data-finish>${t('Start coding')}</button></div></div>`;
  }

  // Pozadie ostáva, mení sa len obsah kroku – žiadne blikanie pri kliknutí.
  function shell() {
    el.innerHTML = `<div class="ob-drag"></div><div class="ob-aurora"><i></i><i></i><i></i></div><div class="ob-grain"></div><div class="ob-stage"></div>`;
  }

  let dir = 1;
  let busyUntil = 0;
  function render(animate = true) {
    const stage = el.querySelector('.ob-stage');
    // Všetky staré verzie kroku preč – aj tie, ktoré ešte len odchádzajú (inak by sa dve prekrývali).
    for (const old of stage.querySelectorAll('.ob-body')) {
      if (!animate || old.classList.contains('leaving')) {
        old.remove();
        continue;
      }
      old.classList.add('leaving', dir > 0 ? 'out-fwd' : 'out-back');
      setTimeout(() => old.remove(), 320);
    }
    const next = document.createElement('div');
    next.className = `ob-body${animate ? (dir > 0 ? ' in-fwd' : ' in-back') : ''}`;
    next.dataset.step = step;
    next.innerHTML = body();
    stage.append(next);
    if (animate) busyUntil = performance.now() + 420;
    // Ukážka editora pri výbere vzhľadu – farby sa menia hneď s témou a akcentom.
    const pre = next.querySelector('.obp-code');
    if (pre && app.colorize) app.colorize(PREVIEW_CODE, 'python').then((html) => (pre.innerHTML = html));
  }

  // Označenie výberu bez prekreslenia celej stránky.
  const mark = (sel, pick) => el.querySelectorAll(sel).forEach((b) => b.classList.toggle('on', pick(b)));

  async function finish(tour) {
    await app.saveSettings({ onboarded: true });
    el.classList.add('leaving');
    setTimeout(() => {
      el.hidden = true;
      el.classList.remove('leaving');
      document.body.classList.remove('onboarding');
      if (langChanged) flux.reload();
      else if (tour) startTour();
    }, 450);
  }

  el.onclick = async (e) => {
    const b = e.target.closest('button');
    if (!b || b.disabled || b.closest('.leaving')) return;
    const nav = b.dataset.next !== undefined || b.dataset.back !== undefined;
    if (nav && performance.now() < busyUntil) return; // dvojklik počas prechodu
    if (b.dataset.next !== undefined) {
      dir = 1;
      step = Math.min(5, step + 1);
      // Krok „inštalácia“ len ak niečo z vybraných jazykov chýba.
      if (step === 4) {
        await app.tools.status();
        const miss = missingTools(app);
        if (!miss.length && !app.tools.busy()) step = 5;
        else if (app.tools.canInstall()) {
          miss.forEach((tc) => installedHere.add(tc.id));
          app.tools.installAll(miss.map((tc) => tc.id));
          // Keď je všetko hotové, úvod sám pokračuje.
          const off = app.tools.onChange(() => {
            if (step === 4 && !app.tools.busy()) {
              off();
              setTimeout(() => {
                if (step !== 4) return;
                dir = 1;
                step = 5;
                render();
              }, 900);
            }
          });
        }
      }
      if (step === 1) {
        flux.i18nList().then((list) => {
          if (Array.isArray(list) && list.length > languages.length) {
            languages = list;
            // Obnoviť len zoznam – celý krok by sa prekreslil počas animácie.
            const list2 = el.querySelector('.ob-body:not(.leaving) .ob-list');
            if (step === 1 && list2) list2.outerHTML = langList();
          }
        });
      }
      return render();
    }
    if (b.dataset.back !== undefined) {
      dir = -1;
      step = Math.max(0, step - 1);
      if (step === 4 && !missingTools(app, true).length) step = 3;
      return render();
    }
    if (b.dataset.lang) {
      const code = b.dataset.lang;
      mark('[data-lang]', (x) => x === b);
      b.classList.add('loading');
      try {
        const dict = await flux.i18nUse(code);
        setLocale(dict);
        await app.saveSettings({ language: code });
        langChanged = true;
        render(false); // texty v novom jazyku
      } catch {
        b.classList.remove('loading');
        mark('[data-lang]', (x) => x.dataset.lang === (app.getSettings().language || 'en'));
        app.toast(t('Could not download the language. Check your internet connection.'), 'error');
      }
      return;
    }
    if (b.dataset.moreLangs !== undefined) {
      moreLangs = true;
      const grid = el.querySelector('.ob-body:not(.leaving) .ob-grid');
      if (grid) {
        const tmp = document.createElement('div');
        tmp.innerHTML = body();
        grid.replaceWith(tmp.querySelector('.ob-grid'));
      }
      return;
    }
    if (b.dataset.code) {
      const set = new Set(app.getSettings().codeLangs || []);
      set.has(b.dataset.code) ? set.delete(b.dataset.code) : set.add(b.dataset.code);
      b.classList.toggle('on', set.has(b.dataset.code));
      return app.saveSettings({ codeLangs: [...set] });
    }
    if (b.dataset.installAll !== undefined) {
      b.disabled = true;
      for (const tc of missingTools(app)) await app.tools.install(tc.id);
      return;
    }
    if (b.dataset.theme) {
      mark('[data-theme]', (x) => x === b);
      return app.setCodeTheme(b.dataset.theme);
    }
    if (b.dataset.accent) {
      mark('[data-accent]', (x) => x === b);
      return app.setAccent(b.dataset.accent);
    }
    if (b.dataset.tour !== undefined) return finish(true);
    if (b.dataset.finish !== undefined) return finish(false);
  };

  el.addEventListener('input', (e) => {
    if (e.target.matches?.('[data-lift]')) app.setDarkLift(Number(e.target.value));
  });

  function open() {
    step = 0;
    dir = 1;
    langChanged = false;
    app.tools.status().then(() => {
      // Stav (nainštalované / veľkosť) na kartách jazykov, ak je krok práve otvorený.
      for (const c of CODE_LANGS) {
        const card = el.querySelector(`.ob-body:not(.leaving) [data-code="${c.id}"] .ob-st`);
        if (card) card.outerHTML = cardStatus(c);
      }
    });
    shell();
    render();
    document.body.classList.add('onboarding');
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
