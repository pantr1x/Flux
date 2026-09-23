// AI asistent (Claude) v hlavnom procese: kľúč a MCP servery sú v nastaveniach zašifrované
// (Electron safeStorage), odpoveď sa streamuje do okna po kúskoch.
const { safeStorage } = require('electron');
const Anthropic = require('@anthropic-ai/sdk').default || require('@anthropic-ai/sdk');
const { t } = require('./i18n');

const MODELS = [
  { id: 'claude-opus-5', name: 'Claude Opus 5' },
  { id: 'claude-sonnet-5', name: 'Claude Sonnet 5' },
  { id: 'claude-haiku-4-5', name: 'Claude Haiku 4.5' },
];
const DEFAULT_MODEL = 'claude-opus-5';

const SYSTEM = `You are the coding assistant built into Flux, a small code editor for beginners.
Explain things simply and briefly, and prefer short, complete, runnable code.
Put code in fenced blocks with the language name (e.g. \`\`\`python) so the user can insert it into the file with one click.
When the user shares a file, refer to it by name and line numbers.
Answer in the language the user writes in.`;

const PROJECT_HINT = `
The user has a project open in Flux. You can see it with flux_project_info and read its files with flux_read_file.
When the user asks you to plan work, add tasks, tick tasks off or describe the project, update the project's to-do list and description with the flux_* tools – they show up on the project page right away.`;

const enc = (text) => (text && safeStorage.isEncryptionAvailable() ? `enc:${safeStorage.encryptString(text).toString('base64')}` : text || '');
const dec = (value) => {
  if (!value) return '';
  if (!String(value).startsWith('enc:')) return value;
  try {
    return safeStorage.decryptString(Buffer.from(value.slice(4), 'base64'));
  } catch {
    return '';
  }
};

function createAI({ getSettings, saveSettings, send, project }) {
  const running = new Map(); // id → stream (kvôli zastaveniu)

  function publicConfig() {
    const s = getSettings();
    const ai = s.ai || {};
    return {
      hasKey: !!dec(ai.key),
      keyHint: dec(ai.key) ? `…${dec(ai.key).slice(-4)}` : '',
      model: ai.model || DEFAULT_MODEL,
      models: MODELS,
      mcp: (ai.mcp || []).map((m) => ({ name: m.name, url: m.url, enabled: m.enabled !== false, hasToken: !!m.token })),
    };
  }

  function update(patch) {
    const s = getSettings();
    const ai = { ...(s.ai || {}) };
    if ('key' in patch) ai.key = enc(String(patch.key || '').trim());
    if (patch.model) ai.model = patch.model;
    if (patch.mcp) {
      const old = ai.mcp || [];
      ai.mcp = patch.mcp.map((m) => {
        const prev = old.find((o) => o.name === m.name);
        // Prázdny token = ponechať pôvodný (v okne sa token nikdy neukazuje).
        const token = m.token ? enc(m.token) : m.clearToken ? '' : prev?.token || '';
        return { name: m.name, url: m.url, enabled: m.enabled !== false, token };
      });
    }
    saveSettings({ ai });
    return publicConfig();
  }

  // Nástroje, cez ktoré Claude vidí a upravuje otvorený projekt (popis, úlohy, súbory).
  const TOOLS = [
    {
      name: 'flux_project_info',
      description: 'Get the open Flux project: name, folder, type, short description and to-do list (with ids and done state), plus the files in the project. Call this before answering questions about the project.',
      input_schema: { type: 'object', properties: {}, additionalProperties: false },
    },
    {
      name: 'flux_read_file',
      description: 'Read a text file from the open project. Path is relative to the project folder, e.g. "index.html" or "src/main.py".',
      input_schema: { type: 'object', properties: { path: { type: 'string' } }, required: ['path'], additionalProperties: false },
    },
    {
      name: 'flux_add_todos',
      description: "Add one or more tasks to the project's to-do list shown on the project page.",
      input_schema: { type: 'object', properties: { tasks: { type: 'array', items: { type: 'string' } } }, required: ['tasks'], additionalProperties: false },
    },
    {
      name: 'flux_update_todo',
      description: 'Mark a to-do as done or not done, rename it, or remove it. Use the id from flux_project_info.',
      input_schema: {
        type: 'object',
        properties: { id: { type: 'number' }, done: { type: 'boolean' }, text: { type: 'string' }, remove: { type: 'boolean' } },
        required: ['id'],
        additionalProperties: false,
      },
    },
    {
      name: 'flux_set_description',
      description: 'Set the short description of the project (one sentence, shown under the project name).',
      input_schema: { type: 'object', properties: { description: { type: 'string' } }, required: ['description'], additionalProperties: false },
    },
  ].map((tool) => ({ ...tool, eager_input_streaming: true }));

  // Vstup z nástroja: pri streamovaní ho API neoverí, overíme ho sami.
  function checkInput(name, input) {
    const need = { flux_read_file: { path: 'string' }, flux_add_todos: { tasks: 'object' }, flux_update_todo: { id: 'number' }, flux_set_description: { description: 'string' } }[name] || {};
    if (!input || typeof input !== 'object') return 'Invalid input: expected an object.';
    for (const [k, type] of Object.entries(need)) if (typeof input[k] !== type) return `Invalid input: "${k}" must be a ${type === 'object' ? 'list' : type}.`;
    if (name === 'flux_add_todos' && !input.tasks.every((x) => typeof x === 'string')) return 'Invalid input: tasks must be strings.';
    return null;
  }

  async function runTool(name, input) {
    const bad = checkInput(name, input);
    if (bad) return { error: bad };
    return project(name, input);
  }

  // messages: [{ role, content }] – celá história (aj bloky nástrojov) sa posiela späť.
  async function chat(id, messages) {
    const ai = getSettings().ai || {};
    const apiKey = dec(ai.key);
    if (!apiKey) throw new Error(t('Add your Anthropic API key in Settings → AI first.'));
    const client = new Anthropic({ apiKey });
    const model = ai.model || DEFAULT_MODEL;
    const servers = (ai.mcp || []).filter((m) => m.enabled !== false && m.url && m.name);
    const betas = [];
    const hasProject = !!project('has');
    const params = { model, max_tokens: 64000, system: SYSTEM + (hasProject ? PROJECT_HINT : '') };
    if (model !== 'claude-haiku-4-5') {
      params.thinking = { type: 'adaptive' };
      params.output_config = { effort: 'medium' };
    }
    // Opus 5: keď bezpečnostný filter odmietne, server skúsi odporúčaný záložný model.
    if (model === 'claude-opus-5') {
      betas.push('server-side-fallback-2026-07-01');
      params.fallbacks = 'default';
    }
    params.tools = hasProject ? [...TOOLS] : [];
    // MCP konektory: Anthropic sa na ne pripojí sám, Claude môže volať ich nástroje.
    if (servers.length) {
      betas.push('mcp-client-2025-11-20');
      params.mcp_servers = servers.map((m) => {
        const token = dec(m.token);
        return { type: 'url', url: m.url, name: m.name, ...(token ? { authorization_token: token } : {}) };
      });
      params.tools.push(...servers.map((m) => ({ type: 'mcp_toolset', mcp_server_name: m.name })));
    }
    if (!params.tools.length) delete params.tools;
    if (betas.length) params.betas = betas;

    const convo = [...messages];
    const start = convo.length;
    let changed = false;
    let final = null;
    try {
      // Slučka: Claude môže volať nástroje Fluxu, výsledky mu vrátime, kým neodpovie.
      for (let turn = 0; turn < 12; turn++) {
        const stream = client.beta.messages.stream({ ...params, messages: convo });
        running.set(id, stream);
        for await (const event of stream) {
          if (event.type === 'content_block_start') {
            const b = event.content_block;
            if (b.type === 'mcp_tool_use') send('ai:event', { id, kind: 'tool', server: b.server_name, name: b.name });
            if (b.type === 'tool_use') send('ai:event', { id, kind: 'tool', server: 'Flux', name: b.name.replace(/^flux_/, '') });
          } else if (event.type === 'content_block_delta' && event.delta.type === 'text_delta') {
            send('ai:event', { id, kind: 'text', text: event.delta.text });
          }
        }
        final = await stream.finalMessage();
        convo.push({ role: 'assistant', content: final.content });
        if (final.stop_reason === 'refusal') break;
        if (final.stop_reason === 'pause_turn') continue; // dlhšia práca MCP nástrojov – pokračovať
        if (final.stop_reason !== 'tool_use') break;
        const results = [];
        for (const block of final.content) {
          if (block.type !== 'tool_use') continue;
          const out = await runTool(block.name, block.input);
          if (out?.changed) changed = true;
          results.push({ type: 'tool_result', tool_use_id: block.id, content: JSON.stringify(out?.result ?? out), is_error: !!out?.error });
        }
        // Všetky výsledky v jednej správe.
        convo.push({ role: 'user', content: results });
      }
      if (changed) send('project:meta-changed');
      return {
        added: convo.slice(start),
        stopReason: final?.stop_reason,
        refused: final?.stop_reason === 'refusal',
        model: final?.model,
      };
    } catch (err) {
      if (changed) send('project:meta-changed');
      if (err instanceof Anthropic.AuthenticationError) throw new Error(t('The API key was rejected. Check it in Settings → AI.'));
      if (err instanceof Anthropic.RateLimitError) throw new Error(t('Too many requests – wait a moment and try again.'));
      if (err instanceof Anthropic.APIUserAbortError) return { aborted: true, added: [] };
      if (err instanceof Anthropic.APIError) throw new Error(`${t('AI error')} ${err.status ?? ''}: ${err.message}`);
      if (err instanceof Anthropic.APIConnectionError) throw new Error(t('Could not reach the AI service. Check your internet connection.'));
      throw err;
    } finally {
      running.delete(id);
    }
  }

  function stop(id) {
    running.get(id)?.abort();
  }

  return { publicConfig, update, chat, stop };
}

module.exports = { createAI, MODELS };
