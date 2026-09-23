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

function createAI({ getSettings, saveSettings, send }) {
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

  // messages: [{ role, content }] – obsah z predchádzajúcich odpovedí sa posiela späť celý (aj MCP bloky).
  async function chat(id, messages) {
    const ai = getSettings().ai || {};
    const apiKey = dec(ai.key);
    if (!apiKey) throw new Error(t('Add your Anthropic API key in Settings → AI first.'));
    const client = new Anthropic({ apiKey });
    const model = ai.model || DEFAULT_MODEL;
    const servers = (ai.mcp || []).filter((m) => m.enabled !== false && m.url && m.name);
    const betas = [];
    const params = {
      model,
      max_tokens: 64000,
      system: SYSTEM,
      messages,
      output_config: { effort: model === 'claude-haiku-4-5' ? undefined : 'medium' },
    };
    if (model === 'claude-haiku-4-5') delete params.output_config;
    else params.thinking = { type: 'adaptive' };
    // Opus 5: keď bezpečnostný filter odmietne, server skúsi odporúčaný záložný model.
    if (model === 'claude-opus-5') {
      betas.push('server-side-fallback-2026-07-01');
      params.fallbacks = 'default';
    }
    // MCP konektory: Anthropic sa na ne pripojí sám, Claude môže volať ich nástroje.
    if (servers.length) {
      betas.push('mcp-client-2025-11-20');
      params.mcp_servers = servers.map((m) => {
        const token = dec(m.token);
        return { type: 'url', url: m.url, name: m.name, ...(token ? { authorization_token: token } : {}) };
      });
      params.tools = servers.map((m) => ({ type: 'mcp_toolset', mcp_server_name: m.name }));
    }
    if (betas.length) params.betas = betas;

    const stream = client.beta.messages.stream(params);
    running.set(id, stream);
    try {
      for await (const event of stream) {
        if (event.type === 'content_block_start') {
          const b = event.content_block;
          if (b.type === 'mcp_tool_use') send('ai:event', { id, kind: 'tool', server: b.server_name, name: b.name });
        } else if (event.type === 'content_block_delta' && event.delta.type === 'text_delta') {
          send('ai:event', { id, kind: 'text', text: event.delta.text });
        }
      }
      const final = await stream.finalMessage();
      const refused = final.stop_reason === 'refusal';
      return {
        content: final.content,
        stopReason: final.stop_reason,
        refused,
        model: final.model,
        usage: { input: final.usage?.input_tokens, output: final.usage?.output_tokens },
      };
    } catch (err) {
      if (err instanceof Anthropic.AuthenticationError) throw new Error(t('The API key was rejected. Check it in Settings → AI.'));
      if (err instanceof Anthropic.RateLimitError) throw new Error(t('Too many requests – wait a moment and try again.'));
      if (err instanceof Anthropic.APIUserAbortError) return { aborted: true, content: [] };
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
