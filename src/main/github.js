// GitHub a Git: pripojenie tokenom, zoznam repozitárov, klonovanie, commit/push/pull, nové repo z projektu.
// Token je v nastaveniach zašifrovaný; do gitu ide len ako hlavička pre github.com (nikdy do .git/config).
const { execFile } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const { net, safeStorage } = require('electron');
const { t } = require('./i18n');

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

function createGitHub({ getSettings, saveSettings }) {
  const token = () => dec(getSettings().github?.token);

  async function api(pathname, { method = 'GET', body } = {}) {
    const tk = token();
    if (!tk) throw new Error(t('Connect GitHub in Settings → GitHub first.'));
    const res = await net.fetch(`https://api.github.com${pathname}`, {
      method,
      headers: {
        Authorization: `Bearer ${tk}`,
        Accept: 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
        'User-Agent': 'Flux',
        ...(body ? { 'Content-Type': 'application/json' } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
    });
    const data = await res.json().catch(() => ({}));
    if (res.status === 401) throw new Error(t('GitHub rejected the token. Create a new one and connect again.'));
    if (!res.ok) throw new Error(data.message || `GitHub ${res.status}`);
    return data;
  }

  // git s tokenom len pre github.com (hlavička sa neuloží do repozitára)
  function git(args, cwd, { auth = false, timeout = 120000 } = {}) {
    const pre = [];
    const tk = token();
    if (auth && tk) pre.push('-c', `http.https://github.com/.extraheader=AUTHORIZATION: basic ${Buffer.from(`x-access-token:${tk}`).toString('base64')}`);
    const user = getSettings().github?.user;
    if (user) pre.push('-c', `user.name=${user.name || user.login}`, '-c', `user.email=${user.id}+${user.login}@users.noreply.github.com`);
    return new Promise((resolve, reject) => {
      execFile('git', [...pre, ...args], { cwd, timeout, windowsHide: true, maxBuffer: 8 * 1024 * 1024, env: { ...process.env, GIT_TERMINAL_PROMPT: '0' } }, (err, stdout, stderr) => {
        if (err) {
          if (err.code === 'ENOENT') return reject(new Error(t('Git is not installed. Install it in Settings → Languages.')));
          return reject(new Error((stderr || err.message).trim().split('\n').slice(-3).join('\n')));
        }
        resolve(stdout);
      });
    });
  }

  async function connect(newToken) {
    const s = getSettings();
    saveSettings({ github: { ...(s.github || {}), token: enc(newToken.trim()) } });
    try {
      const u = await api('/user');
      const user = { login: u.login, name: u.name || u.login, id: u.id, avatar: u.avatar_url };
      saveSettings({ github: { ...(getSettings().github || {}), user } });
      return user;
    } catch (err) {
      saveSettings({ github: {} });
      throw err;
    }
  }

  function disconnect() {
    saveSettings({ github: {} });
    return true;
  }

  const info = () => ({ connected: !!token(), user: getSettings().github?.user || null });

  async function repos() {
    const list = await api('/user/repos?per_page=100&sort=updated&affiliation=owner,collaborator,organization_member');
    return list.map((r) => ({ name: r.name, full: r.full_name, private: r.private, description: r.description || '', url: r.clone_url, updated: r.updated_at, language: r.language || '' }));
  }

  async function clone(full, root) {
    const name = full.split('/')[1];
    let dir = path.join(root, name);
    for (let i = 2; fs.existsSync(dir); i++) dir = path.join(root, `${name}-${i}`);
    fs.mkdirSync(root, { recursive: true });
    await git(['clone', `https://github.com/${full}.git`, dir], root, { auth: true, timeout: 600000 });
    return dir;
  }

  // Stav projektu: je to git? vetva, zmeny, remote na GitHube, koľko commitov čaká na push/pull
  async function status(dir) {
    if (!dir || !fs.existsSync(path.join(dir, '.git'))) return { repo: false };
    const out = await git(['status', '--porcelain=v1', '-b'], dir);
    const lines = out.split('\n').filter(Boolean);
    const head = lines.shift() || '';
    const branch = (head.match(/^## (?:No commits yet on )?([^.\s]+)/) || [])[1] || '';
    const ahead = Number((head.match(/ahead (\d+)/) || [])[1] || 0);
    const behind = Number((head.match(/behind (\d+)/) || [])[1] || 0);
    let remote = '';
    try {
      remote = (await git(['remote', 'get-url', 'origin'], dir)).trim();
    } catch {}
    const gh = remote.match(/github\.com[/:]([^/]+\/[^/.]+?)(?:\.git)?$/);
    return { repo: true, branch, changes: lines.length, files: lines.slice(0, 50).map((l) => ({ state: l.slice(0, 2).trim(), file: l.slice(3) })), ahead, behind, remote, github: gh ? gh[1] : '' };
  }

  async function commitPush(dir, message) {
    await git(['add', '-A'], dir);
    const st = await status(dir);
    if (st.changes) await git(['commit', '-m', message || 'Update'], dir);
    if (st.remote) await git(['push', '-u', 'origin', st.branch || 'HEAD'], dir, { auth: true, timeout: 300000 });
    return status(dir);
  }

  async function pull(dir) {
    await git(['pull', '--rebase', '--autostash'], dir, { auth: true, timeout: 300000 });
    return status(dir);
  }

  // Nový repozitár na GitHube z projektu (aj keď ešte nie je git).
  async function publish(dir, { name, isPrivate = true, description = '' }) {
    const repo = await api('/user/repos', { method: 'POST', body: { name, private: isPrivate, description } });
    if (!fs.existsSync(path.join(dir, '.git'))) await git(['init', '-b', 'main'], dir);
    const gitignore = path.join(dir, '.gitignore');
    if (!fs.existsSync(gitignore)) fs.writeFileSync(gitignore, '.venv/\nvenv/\n__pycache__/\nnode_modules/\n*.exe\n.DS_Store\n');
    try {
      await git(['remote', 'add', 'origin', repo.clone_url], dir);
    } catch {
      await git(['remote', 'set-url', 'origin', repo.clone_url], dir);
    }
    await git(['add', '-A'], dir);
    try {
      await git(['commit', '-m', 'First commit from Flux'], dir);
    } catch {}
    const st = await status(dir);
    await git(['push', '-u', 'origin', st.branch || 'main'], dir, { auth: true, timeout: 300000 });
    return { url: repo.html_url, full: repo.full_name };
  }

  return { info, connect, disconnect, repos, clone, status, commitPush, pull, publish, api };
}

module.exports = { createGitHub };
