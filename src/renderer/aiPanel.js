// AI panel (Claude) vpravo vedľa editora: chat, kontext z otvoreného súboru, vloženie kódu jedným klikom.
import { t } from './i18n.js';
import { icon } from './icons.js';

const flux = window.flux;

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);

// Ikona k nadpisu podľa témy (v poznámkach k vydaniu namiesto emoji).
const HEAD_ICONS = [
  [/ai|claude|mcp|assistant/i, 'sparkle'],
  [/fix|bug|repair/i, 'check'],
  [/updat|release|version/i, 'refresh'],
  [/look|theme|design|make it yours|personal/i, 'palette'],
  [/github|git\b/i, 'git'],
  [/plugin|extension/i, 'download'],
  [/file|markdown|editor/i, 'file'],
  [/run|language/i, 'play'],
];
const EMOJI = /^[\p{Extended_Pictographic}\uFE0F\u200D\s]+/u;

// Jednoduchý Markdown: nadpisy, zoznamy, tabuľky, citácie, bloky kódu, `kód`, **tučné**, *kurzíva*, ~~prečiarknuté~~, odkazy a obrázky.
// opts.icons – nadpisy dostanú ikonu namiesto emoji; opts.preview – náhľad súboru (bloky kódu bez „Insert“).
export function markdown(text, opts = {}) {
  const parts = String(text).replace(/\r\n?/g, '\n').split(/```/);
  let html = '';
  parts.forEach((part, i) => {
    if (i % 2 === 1) {
      const nl = part.indexOf('\n');
      const lang = nl > 0 ? part.slice(0, nl).trim() : '';
      const code = nl >= 0 ? part.slice(nl + 1).replace(/\n$/, '') : part;
      html += `<div class="ai-code"><div class="ai-code-bar"><span>${esc(lang || 'code')}</span><button data-copy>${icon('file', 12)}${t('Copy')}</button>${opts.preview ? '' : `<button data-insert>${icon('plus', 12)}${t('Insert')}</button>`}</div><pre><code>${esc(code)}</code></pre></div>`;
      return;
    }
    const inline = (s) =>
      esc(s)
        .replace(/`([^`]+)`/g, '<code>$1</code>')
        .replace(/!\[([^\]]*)\]\((https?:[^)\s]+)\)/g, '<img src="$2" alt="$1">')
        .replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
        .replace(/__([^_]+)__/g, '<b>$1</b>')
        .replace(/~~([^~]+)~~/g, '<s>$1</s>')
        .replace(/(^|[\s(])\*([^*\s][^*]*)\*(?=[\s).,!?:]|$)/g, '$1<i>$2</i>')
        .replace(/(^|[\s(])_([^_\s][^_]*)_(?=[\s).,!?:]|$)/g, '$1<i>$2</i>')
        .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a href="$2" target="_blank">$1</a>');
    const heading = (level, raw) => {
      let txt = raw;
      let ic = '';
      if (opts.icons) {
        txt = raw.replace(EMOJI, '');
        ic = `<span class="md-h-ic">${icon((HEAD_ICONS.find(([re]) => re.test(txt)) || [0, 'sparkle'])[1], 14)}</span>`;
      }
      const tag = opts.preview ? `h${Math.min(level + 1, 6)}` : 'h4';
      return `<${tag} class="md-h md-h${level}">${ic}${inline(txt)}</${tag}>`;
    };
    // Riadok po riadku (aj keď bloky nie sú oddelené prázdnym riadkom).
    const lines = part.split('\n');
    let para = [];
    const flush = () => {
      if (para.length) html += `<p>${para.map(inline).join('<br>')}</p>`;
      para = [];
    };
    for (let k = 0; k < lines.length; k++) {
      const line = lines[k];
      if (!line.trim()) {
        flush();
        continue;
      }
      const head = line.match(/^\s*(#{1,6})\s+(.*?)\s*#*\s*$/);
      if (head) {
        flush();
        html += heading(head[1].length, head[2]);
        continue;
      }
      if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) {
        flush();
        html += '<hr>';
        continue;
      }
      if (/^\s*>/.test(line)) {
        flush();
        const q = [];
        while (k < lines.length && /^\s*>/.test(lines[k])) q.push(lines[k++].replace(/^\s*>\s?/, ''));
        k--;
        html += `<blockquote>${markdown(q.join('\n'), opts)}</blockquote>`;
        continue;
      }
      if (/^\s*([-*+]|\d+\.)\s+/.test(line)) {
        flush();
        const ordered = /^\s*\d+\./.test(line);
        const items = [];
        const same = ordered ? /^\s*\d+\.\s+/ : /^\s*[-*+]\s+/;
        while (k < lines.length && same.test(lines[k])) items.push(lines[k++].replace(/^\s*([-*+]|\d+\.)\s+/, ''));
        k--;
        const li = (x) => {
          const box = x.match(/^\[([ xX])\]\s+(.*)$/);
          return box ? `<li class="md-task"><input type="checkbox" disabled${box[1] !== ' ' ? ' checked' : ''}> ${inline(box[2])}</li>` : `<li>${inline(x)}</li>`;
        };
        html += `<${ordered ? 'ol' : 'ul'}>${items.map(li).join('')}</${ordered ? 'ol' : 'ul'}>`;
        continue;
      }
      if (/^\s*\|.*\|\s*$/.test(line)) {
        flush();
        const rows = [];
        while (k < lines.length && /^\s*\|.*\|\s*$/.test(lines[k])) rows.push(lines[k++]);
        k--;
        const cells = (r) => r.trim().replace(/^\||\|$/g, '').split('|').map((c) => c.trim());
        const body = rows.filter((r) => !/^\s*\|[\s:|-]+\|\s*$/.test(r));
        const [first, ...rest] = body;
        html += `<table><thead><tr>${cells(first).map((c) => `<th>${inline(c)}</th>`).join('')}</tr></thead><tbody>${rest.map((r) => `<tr>${cells(r).map((c) => `<td>${inline(c)}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
        continue;
      }
      para.push(line);
    }
    flush();
  });
  return html;
}

export function createAIPanel({ getContext, insertCode, openSettingsAI, toast }) {
  const el = document.getElementById('ai');
  const resizer = document.getElementById('ai-resizer');
  let history = []; // správy pre API
  let view = []; // čo sa ukazuje: { role, text, tools:[], error }
  let busyId = null;
  let config = null;
  let withFile = true;

  function render() {
    const hasKey = config?.hasKey;
    const model = config?.models.find((m) => m.id === config.model)?.name || '';
    el.innerHTML = `
      <div class="ai-head"><span class="ai-title">${icon('sparkle', 15)}AI</span><small>${esc(model)}</small><div class="grow"></div>
        <button class="icon-btn" data-new title="${t('New chat')}">${icon('plus', 15)}</button>
        <button class="icon-btn" data-settings title="${t('AI settings')}">${icon('settings', 15)}</button>
        <button class="icon-btn" data-close title="${t('Close (Esc)')}">${icon('x', 15)}</button></div>
      <div class="ai-log" id="ai-log">${
        !hasKey
          ? `<div class="ai-empty">${icon('sparkle', 28)}<b>${t('Your coding assistant')}</b><p>${t('Flux can use Claude to explain code, fix errors and write new code. Add your own Anthropic API key to start.')}</p><button class="ob-primary" data-settings>${t('Add API key')}</button></div>`
          : view.length
            ? view.map(msgHtml).join('')
            : `<div class="ai-empty">${icon('sparkle', 28)}<b>${t('Ask anything about your code')}</b><div class="ai-ideas">${[t('Explain this file'), t('Why does my code not work?'), t('Add comments to this code')]
                .map((q) => `<button data-idea>${esc(q)}</button>`)
                .join('')}</div></div>`
      }</div>
      ${
        hasKey
          ? `<form class="ai-input" id="ai-form">
        <button type="button" class="ai-ctx${withFile ? ' on' : ''}" data-ctx title="${t('Send the open file with your question')}">${icon('file', 12)}<span id="ai-ctx-name">${esc(ctxLabel())}</span></button>
        <textarea id="ai-q" rows="1" placeholder="${t('Ask Claude…')}" title="${t('Enter to send, Shift+Enter for a new line')}"></textarea>
        <button class="ai-send" ${busyId ? 'data-stop' : ''} title="${busyId ? t('Stop') : t('Send')}">${icon(busyId ? 'stop' : 'play', 14)}</button>
      </form>`
          : ''
      }`;
    const log = el.querySelector('#ai-log');
    log.scrollTop = log.scrollHeight;
    const q = el.querySelector('#ai-q');
    if (q) {
      q.oninput = () => {
        q.style.height = 'auto';
        q.style.height = `${Math.min(160, q.scrollHeight)}px`;
      };
      q.onkeydown = (e) => {
        e.stopPropagation();
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          send(q.value);
        }
        if (e.key === 'Escape') hide();
      };
    }
  }

  function ctxLabel() {
    const c = getContext();
    if (!c.file) return t('No file');
    return c.selection ? t('{file} (selection)', { file: c.fileName }) : c.fileName;
  }

  function msgHtml(m) {
    if (m.role === 'user') return `<div class="ai-msg user"><div class="ai-bubble">${esc(m.text).replace(/\n/g, '<br>')}</div>${m.file ? `<small class="ai-att">${icon('file', 11)}${esc(m.file)}</small>` : ''}</div>`;
    const tools = (m.tools || []).map((x) => `<div class="ai-tool">${icon('command', 12)}${esc(x.server)} › ${esc(x.name)}</div>`).join('');
    const body = m.text ? markdown(m.text) : m.pending ? '<div class="ai-dots"><i></i><i></i><i></i></div>' : '';
    return `<div class="ai-msg bot">${tools}<div class="ai-body">${body}</div>${m.error ? `<div class="ai-err">${esc(m.error)}</div>` : ''}${m.note ? `<small class="ai-note">${esc(m.note)}</small>` : ''}</div>`;
  }

  let raf = 0;
  function renderLast() {
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(() => {
      const log = el.querySelector('#ai-log');
      const last = log?.lastElementChild;
      if (!last) return render();
      const near = log.scrollHeight - log.scrollTop - log.clientHeight < 80;
      last.outerHTML = msgHtml(view[view.length - 1]);
      if (near) log.scrollTop = log.scrollHeight;
    });
  }

  async function send(text) {
    text = String(text || '').trim();
    if (!text || busyId) return;
    const c = getContext();
    const attach = withFile && c.file;
    let content = text;
    if (attach) {
      const body = c.selection || c.text;
      content = `${text}\n\n<file name="${c.fileName}"${c.selection ? ' part="selection"' : ''} language="${c.language}">\n${body.slice(0, 120000)}\n</file>`;
    }
    history.push({ role: 'user', content });
    view.push({ role: 'user', text, file: attach ? ctxLabel() : '' });
    const bot = { role: 'bot', text: '', tools: [], pending: true };
    view.push(bot);
    busyId = `ai-${Date.now()}`;
    render();
    try {
      const res = await flux.aiChat(busyId, history);
      bot.pending = false;
      if (res.aborted) {
        bot.note = t('Stopped.');
        history.pop();
        view.splice(view.length - 2, 1);
        // (prerušená odpoveď sa do histórie nepridá)
      } else {
        // Celý priebeh (aj volania nástrojov a ich výsledky) ide späť do ďalšej otázky.
        history.push(...res.added);
        if (!bot.text) {
          const last = res.added.filter((m) => m.role === 'assistant').pop();
          bot.text = (last?.content || []).filter((b) => b.type === 'text').map((b) => b.text).join('');
        }
        if (res.refused) bot.note = t('Claude could not help with this request.');
      }
    } catch (err) {
      bot.pending = false;
      bot.error = String(err?.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, '');
      history.pop();
    }
    busyId = null;
    render();
  }

  flux.onAiEvent((ev) => {
    if (ev.id !== busyId) return;
    const bot = view[view.length - 1];
    if (ev.kind === 'text') {
      bot.pending = false;
      bot.text += ev.text;
    } else if (ev.kind === 'tool') bot.tools.push(ev);
    renderLast();
  });

  el.addEventListener('click', async (e) => {
    const b = e.target.closest('button, a');
    if (!b) return;
    if (b.tagName === 'A') {
      e.preventDefault();
      return flux.openExternal(b.href);
    }
    if (b.dataset.close !== undefined) return hide();
    if (b.dataset.settings !== undefined) return openSettingsAI();
    if (b.dataset.new !== undefined) {
      if (busyId) flux.aiStop(busyId);
      history = [];
      view = [];
      return render();
    }
    if (b.dataset.idea !== undefined) return send(b.textContent);
    if (b.dataset.ctx !== undefined) {
      withFile = !withFile;
      b.classList.toggle('on', withFile);
      return;
    }
    if (b.dataset.stop !== undefined) {
      e.preventDefault();
      return flux.aiStop(busyId);
    }
    const code = b.closest('.ai-code')?.querySelector('code')?.textContent;
    if (code != null && b.dataset.copy !== undefined) {
      navigator.clipboard.writeText(code);
      return toast(t('Copied.'), 'ok', 1600);
    }
    if (code != null && b.dataset.insert !== undefined) return insertCode(code);
  });
  el.addEventListener('submit', (e) => {
    e.preventDefault();
    send(el.querySelector('#ai-q')?.value);
  });

  async function show() {
    config = await flux.aiConfig();
    el.hidden = false;
    resizer.hidden = false;
    render();
    el.querySelector('#ai-q')?.focus();
  }

  function hide() {
    el.hidden = true;
    resizer.hidden = true;
  }

  return {
    toggle: () => (el.hidden ? show() : hide()),
    show,
    hide,
    refreshConfig: async () => {
      config = await flux.aiConfig();
      if (!el.hidden) render();
    },
    updateContext: () => {
      const n = el.querySelector('#ai-ctx-name');
      if (n) n.textContent = ctxLabel();
    },
    ask: async (text) => {
      await show();
      send(text);
    },
  };
}
