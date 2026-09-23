// Náhľad kódu vpravo (minimapa) so skutočným, zafarbeným textom namiesto „kockovaných“ znakov.
// Klik alebo ťahanie posúva editor; zvýraznený pás ukazuje, čo je práve vidieť.
const LINE = 4.2; // výška riadku v náhľade (px)

export function createCodeMap(monaco, editor, host) {
  const content = document.createElement('div');
  content.className = 'cm-content';
  const slider = document.createElement('div');
  slider.className = 'cm-slider';
  host.append(content, slider);

  let offset = 0; // o koľko je obsah náhľadu posunutý hore (pri dlhých súboroch)
  let colorTimer = null;
  let colorRun = 0;
  let listeners = [];

  const editorLine = () => editor.getOption(monaco.editor.EditorOption.lineHeight);

  function layout() {
    const model = editor.getModel();
    if (!model || host.hidden) return;
    const lines = model.getLineCount();
    const mapH = host.clientHeight;
    const total = lines * LINE;
    const scrollTop = editor.getScrollTop();
    const maxScroll = Math.max(1, editor.getScrollHeight() - editor.getLayoutInfo().height);
    const visibleLines = editor.getLayoutInfo().height / editorLine();
    const sliderH = Math.max(12, visibleLines * LINE);
    // Ak sa celý súbor nezmestí, náhľad sa posúva spolu s editorom (ako vo VS Code).
    offset = total > mapH ? (scrollTop / maxScroll) * (total - mapH) : 0;
    content.style.transform = `translateY(${-offset}px)`;
    slider.style.height = `${sliderH}px`;
    slider.style.top = `${(scrollTop / editorLine()) * LINE - offset}px`;
  }

  async function colorize() {
    const model = editor.getModel();
    if (!model) {
      content.innerHTML = '';
      return;
    }
    const run = ++colorRun;
    const text = model.getLineCount() > 5000 ? model.getLinesContent().slice(0, 5000).join('\n') : model.getValue();
    const html = await monaco.editor.colorize(text, model.getLanguageId(), { tabSize: model.getOptions().tabSize });
    if (run !== colorRun) return;
    content.innerHTML = html;
    layout();
  }

  function scheduleColorize() {
    clearTimeout(colorTimer);
    colorTimer = setTimeout(colorize, 250);
  }

  function attachModel() {
    listeners.forEach((d) => d.dispose());
    listeners = [];
    const model = editor.getModel();
    if (model) listeners.push(model.onDidChangeContent(scheduleColorize));
    colorize();
  }

  // Posun editora podľa pozície myši v náhľade.
  function scrollTo(clientY, grabOffset) {
    const rect = host.getBoundingClientRect();
    const y = clientY - rect.top + offset - grabOffset;
    editor.setScrollTop((y / LINE) * editorLine());
  }

  host.addEventListener('pointerdown', (e) => {
    if (!editor.getModel()) return;
    e.preventDefault();
    host.setPointerCapture(e.pointerId);
    const s = slider.getBoundingClientRect();
    const onSlider = e.clientY >= s.top && e.clientY <= s.bottom;
    const grab = onSlider ? e.clientY - s.top : s.height / 2;
    host.classList.add('dragging');
    scrollTo(e.clientY, grab);
    const move = (ev) => scrollTo(ev.clientY, grab);
    const up = () => {
      host.classList.remove('dragging');
      host.removeEventListener('pointermove', move);
      host.removeEventListener('pointerup', up);
    };
    host.addEventListener('pointermove', move);
    host.addEventListener('pointerup', up);
  });
  host.addEventListener('wheel', (e) => editor.setScrollTop(editor.getScrollTop() + e.deltaY), { passive: true });

  editor.onDidChangeModel(attachModel);
  editor.onDidScrollChange(layout);
  editor.onDidLayoutChange(layout);
  editor.onDidChangeModelLanguage?.(scheduleColorize);
  new ResizeObserver(layout).observe(host);

  return {
    refresh: colorize,
    setVisible(show) {
      host.hidden = !show;
      document.body.classList.toggle('codemap-on', show);
      if (show) colorize();
    },
  };
}
