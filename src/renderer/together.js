// Flux Together (Wi-Fi): kto z tvojej miestnosti má otvorený aký súbor, kde je kurzor a čo práve píše.
// Spojenie je priame v tvojej sieti (src/main/lan.js) – nič neide na internet.
const flux = window.flux;
const COLORS = ['#ff7eb6', '#4ade80', '#60a5fa', '#fbbf24', '#c084fc', '#2dd4bf', '#fb923c', '#f87171'];
const colorIdx = (key) => [...String(key)].reduce((a, c) => a + c.charCodeAt(0), 0) % COLORS.length;

export function createTogether({ monaco, editor, t, icon, esc, toast, getWorkspace, getName, isOn, openFile, relative, join, basename, promptPalette }) {
  let running = false;
  let code = '';
  let peers = [];
  let recent = [];
  let lastEdit = 0;
  let sendTimer = 0;
  const decorations = editor.createDecorationsCollection([]);
  const box = () => document.getElementById('together');
  const project = () => (getWorkspace() ? basename(getWorkspace()) : '');
  const relOf = (abs) => (abs && getWorkspace() && abs.toLowerCase().startsWith(getWorkspace().toLowerCase()) ? relative(abs).replace(/\\/g, '/') : '');
  const samePlace = (p) => p.state?.project && p.state.project === project();

  function note(a, f) {
    if (!f) return;
    recent = [{ a, f, at: Date.now() }, ...recent.filter((x) => !(x.a === a && x.f === f))].slice(0, 8);
  }

  function state() {
    const model = editor.getModel();
    const pos = editor.getPosition();
    const file = relOf(model?.uri?.fsPath);
    const line = file && pos ? pos.lineNumber : 0;
    return {
      project: project(),
      file,
      line,
      col: pos?.column || 0,
      text: file && line ? model.getLineContent(line).slice(0, 160) : '',
      typing: Date.now() - lastEdit < 3000,
      recent,
      at: Date.now(),
    };
  }
  // Posiela sa hneď (najviac ~12× za sekundu) – druhý vidí písanie takmer naživo.
  function publish() {
    if (!running || sendTimer) return;
    sendTimer = setTimeout(() => {
      sendTimer = 0;
      flux.lanUpdate(state());
    }, 80);
  }

  function render() {
    const el = box();
    if (!el) return;
    el.hidden = !isOn();
    if (!isOn()) return;
    const head = `<div class="pr-head"><span>${t('Together')}${running ? ` <i class="tg-code" title="${t('Room code')}">${esc(code)}</i>` : ''}</span>${running ? `<button class="icon-btn" data-tg-stop title="${t('Leave')}">${icon('x', 14)}</button>` : ''}</div>`;
    if (!running) {
      el.innerHTML = `${head}<button class="tg-start" data-tg-start>${icon('globe', 14)}<span><b>${t('Work together on Wi-Fi')}</b><small>${t('Everyone in the same Wi-Fi who enters the same code sees each other.')}</small></span></button>`;
      return;
    }
    const rows = peers.length
      ? peers
          .map((p) => {
            const c = COLORS[colorIdx(p.id)];
            const s = p.state || {};
            const here = samePlace(p);
            const where = !s.project ? t('no project open') : !here ? `${t('in')} ${esc(s.project)}` : s.file ? `${esc(s.file.split('/').pop())}${s.line ? `:${s.line}` : ''}` : t('no file open');
            return `<div class="tg-peer${here && s.file ? ' can' : ''}" data-file="${here ? esc(s.file || '') : ''}" data-line="${s.line || 1}" style="--c:${c}" title="${esc(s.file || '')}">
              <span class="tg-av">${esc((p.name || '?')[0].toUpperCase())}${s.typing ? '<i class="tg-typing"></i>' : ''}</span>
              <span class="tg-txt"><b>${esc(p.name)}${s.typing ? ` <small class="tg-tw">${t('typing…')}</small>` : ''}</b><small>${where}</small>${here && s.text?.trim() ? `<code>${esc(s.text.trim())}</code>` : ''}</span></div>`;
          })
          .join('')
      : `<div class="tg-empty">${t('Waiting for others… Tell them the code {code}.', { code: `<b>${esc(code)}</b>` })}</div>`;
    const feed = peers
      .filter(samePlace)
      .flatMap((p) => (p.state?.recent || []).map((r) => ({ ...r, id: p.id, name: p.name })))
      .sort((a, b) => b.at - a.at)
      .slice(0, 5)
      .map((r) => `<div class="tg-ev" data-file="${esc(r.f)}" style="--c:${COLORS[colorIdx(r.id)]}"><i></i><span><b>${esc(r.name)}</b> ${r.a === 'edit' ? t('edited') : r.a === 'save' ? t('saved') : t('opened')} <u>${esc(r.f.split('/').pop())}</u></span></div>`)
      .join('');
    el.innerHTML = head + rows + (feed ? `<div class="tg-feed">${feed}</div>` : '');
  }

  // Farebný pruh a meno pri riadku, kde je kolega v tom istom súbore.
  function decorate() {
    const model = editor.getModel();
    const rel = relOf(model?.uri?.fsPath);
    if (!running || !rel) return decorations.clear();
    const list = peers.filter((p) => samePlace(p) && p.state.file === rel && p.state.line > 0 && p.state.line <= model.getLineCount());
    decorations.set(
      list.map((p) => {
        const i = colorIdx(p.id);
        const col = Math.min(p.state.col || 1, model.getLineMaxColumn(p.state.line));
        return [
          { range: new monaco.Range(p.state.line, 1, p.state.line, 1), options: { isWholeLine: true, className: `tg-line tg-c${i}`, overviewRuler: { color: COLORS[i], position: monaco.editor.OverviewRulerLane.Left } } },
          { range: new monaco.Range(p.state.line, col, p.state.line, col), options: { showIfCollapsed: true, beforeContentClassName: `tg-caret tg-c${i}`, after: { content: ` ${p.name}${p.state.typing ? ' ✎' : ''}`, inlineClassName: `tg-name tg-c${i}` } } },
        ];
      }).flat(),
    );
  }

  // Bodka v strome pri súboroch, ktoré majú ostatní otvorené.
  function markTree() {
    document.querySelectorAll('#tree .tg-dot').forEach((d) => d.remove());
    if (!running || !getWorkspace()) return;
    for (const p of peers.filter(samePlace)) {
      if (!p.state.file) continue;
      const abs = join(getWorkspace(), p.state.file).toLowerCase();
      const row = [...document.querySelectorAll('#tree [data-path]')].find((r) => r.dataset.path.toLowerCase() === abs);
      if (row) row.insertAdjacentHTML('beforeend', `<i class="tg-dot" style="--c:${COLORS[colorIdx(p.id)]}" title="${esc(p.name)}"></i>`);
    }
  }

  function refresh() {
    render();
    decorate();
    markTree();
  }

  async function start() {
    const c = await promptPalette({ placeholder: t('Room code, e.g. 4821'), note: t('Everyone who wants to work together enters the same code. Only people on the same Wi-Fi can join.'), value: code || String(Math.floor(1000 + Math.random() * 9000)) });
    if (!c || !c.trim()) return;
    try {
      await flux.lanStart({ code: c.trim(), name: getName() });
      code = c.trim();
      running = true;
      flux.lanUpdate(state());
      toast(t('You are in room {code}. If Windows asks, allow Flux on private networks.', { code }), 'ok', 7000);
    } catch (err) {
      toast(String(err.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, ''), 'error', 8000);
    }
    refresh();
  }

  async function stop() {
    running = false;
    peers = [];
    await flux.lanStop().catch(() => {});
    refresh();
  }

  flux.onLanPeers((list) => {
    peers = list || [];
    refresh();
  });
  editor.onDidChangeCursorPosition(publish);
  editor.onDidChangeModel(() => {
    note('open', relOf(editor.getModel()?.uri?.fsPath));
    publish();
    decorate();
  });
  editor.onDidChangeModelContent(() => {
    lastEdit = Date.now();
    note('edit', relOf(editor.getModel()?.uri?.fsPath));
    publish();
  });
  // „prestal písať“ sa ukáže aj bez ďalšieho pohybu
  setInterval(() => {
    if (running && lastEdit && Date.now() - lastEdit > 3000 && Date.now() - lastEdit < 6000) publish();
  }, 1500);

  document.addEventListener('click', (e) => {
    const el = box();
    if (!el || !el.contains(e.target)) return;
    if (e.target.closest('[data-tg-start]')) return start();
    if (e.target.closest('[data-tg-stop]')) return stop();
    const row = e.target.closest('[data-file]');
    if (row?.dataset.file && getWorkspace()) openFile(join(getWorkspace(), row.dataset.file), { line: Number(row.dataset.line) || 1 });
  });

  return {
    refresh,
    stop,
    onSave: (abs) => {
      note('save', relOf(abs));
      publish();
    },
    onProject: () => {
      recent = [];
      publish();
      refresh();
    },
    markTree,
    running: () => running,
  };
}
