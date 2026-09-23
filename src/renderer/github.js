// GitHub v okne: nastavenia (token), výber repozitára, karta Git na stránke projektu, položka v stavovom riadku.
import { t } from './i18n.js';
import { icon, fileIcon } from './icons.js';

const flux = window.flux;
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const errText = (err) => String(err?.message || err).replace(/^Error invoking remote method '[^']+': (Error: )?/, '');

export function createGitHub({ toast, tools, openSettingsTab, openProject, getWorkspace, onStatus }) {
  let lastStatus = null;

  // ---------- nastavenia ----------
  async function renderSettings(box) {
    if (!box) return;
    const info = await flux.ghInfo();
    await tools.status();
    const gitTc = tools.info('git');
    box.innerHTML = `
      <p class="s-lead">${t('Connect your GitHub account to open your repositories as projects and to save (push) your work online.')}</p>
      <h3>${t('Account')}</h3>
      <div class="s-group">${
        info.connected && info.user
          ? `<div class="s-row"><span class="gh-user">${info.user.avatar ? `<img src="${esc(info.user.avatar)}" alt="">` : ''}<span><b>${esc(info.user.name)}</b><small>@${esc(info.user.login)}</small></span></span><button class="s-btn" data-gh-disconnect>${t('Disconnect')}</button></div>`
          : `<div class="s-row"><span><b>${t('Personal access token')}</b><small>${t('Create one on GitHub with the “repo” permission, then paste it here.')} <a href="#" data-gh-newtoken>${t('Create token')}</a></small></span><span class="s-inline"><input type="password" id="gh-token" placeholder="ghp_… / github_pat_…" autocomplete="off" spellcheck="false"><button class="s-btn" data-gh-connect>${t('Connect')}</button></span></div>`
      }</div>
      <h3>Git</h3>
      <div class="tc-list">${gitTc ? tools.row(gitTc) : ''}</div>`;
    tools.bind(box);
    box.onclick = async (e) => {
      if (e.target.closest('[data-gh-newtoken]')) {
        e.preventDefault();
        return flux.openExternal('https://github.com/settings/tokens/new?scopes=repo&description=Flux');
      }
      const c = e.target.closest('[data-gh-connect]');
      if (c) {
        const token = box.querySelector('#gh-token').value.trim();
        if (!token) return;
        c.disabled = true;
        try {
          const user = await flux.ghConnect(token);
          toast(t('Connected as {user}.', { user: user.login }), 'ok');
        } catch (err) {
          toast(errText(err), 'error', 7000);
        }
        return renderSettings(box);
      }
      if (e.target.closest('[data-gh-disconnect]')) {
        await flux.ghDisconnect();
        return renderSettings(box);
      }
    };
  }

  // ---------- výber repozitára (nový projekt z GitHubu) ----------
  async function pickRepo() {
    const info = await flux.ghInfo();
    if (!info.connected) {
      toast(t('Connect GitHub first.'), 'info', 5000);
      return openSettingsTab('github');
    }
    const el = document.getElementById('ghpick');
    el.innerHTML = `<div class="np-card gh-card" role="dialog">
      <header><h2>${icon('git', 18)}${t('Open a repository')}</h2><button class="icon-btn" data-close title="${t('Close (Esc)')}">${icon('x', 16)}</button></header>
      <input class="s-search" id="gh-q" placeholder="${t('Search your repositories…')}" spellcheck="false" autocomplete="off">
      <div class="gh-list" id="gh-list"><div class="s-loading"><span class="spin"></span> ${t('Loading…')}</div></div>
      <p class="gh-foot">${t('Flux downloads (clones) the repository into your projects folder and opens it.')}</p>
    </div>`;
    el.hidden = false;
    let repos = [];
    const list = el.querySelector('#gh-list');
    const draw = (q = '') => {
      const f = repos.filter((r) => r.full.toLowerCase().includes(q.toLowerCase()));
      list.innerHTML = f.length
        ? f
            .map(
              (r) =>
                `<button class="gh-repo" data-full="${esc(r.full)}"><span class="gh-ic">${icon('git', 16)}</span><span class="gh-txt"><b>${esc(r.full)}</b><small>${esc(r.description || r.language || '')}</small></span>${r.private ? `<span class="gh-badge">${t('private')}</span>` : ''}</button>`,
            )
            .join('')
        : `<div class="s-loading">${t('No repositories found.')}</div>`;
    };
    flux
      .ghRepos()
      .then((r) => {
        repos = r;
        draw();
      })
      .catch((err) => (list.innerHTML = `<div class="s-loading">${esc(errText(err))}</div>`));
    el.querySelector('#gh-q').oninput = (e) => draw(e.target.value);
    el.querySelector('#gh-q').focus();
    const close = () => (el.hidden = true);
    el.onkeydown = (e) => e.key === 'Escape' && close();
    el.onclick = async (e) => {
      if (e.target === el || e.target.closest('[data-close]')) return close();
      const b = e.target.closest('[data-full]');
      if (!b) return;
      await tools.status();
      if (!tools.info('git')?.installed) {
        close();
        if (!(await tools.ask(tools.info('git'), tools.canInstall()))) return;
      }
      list.innerHTML = `<div class="s-loading"><span class="spin"></span> ${t('Downloading {repo}…', { repo: esc(b.dataset.full) })}</div>`;
      try {
        const dir = await flux.ghClone(b.dataset.full, await flux.projectRoot());
        close();
        await openProject(dir);
        toast(t('{repo} is ready.', { repo: b.dataset.full }), 'ok');
      } catch (err) {
        toast(errText(err), 'error', 8000);
        draw();
      }
    };
  }

  // ---------- karta Git na stránke projektu ----------
  async function renderCard(box) {
    if (!box) return;
    let st;
    try {
      st = await flux.gitStatus();
    } catch (err) {
      st = { repo: false, error: errText(err) };
    }
    lastStatus = st;
    onStatus(st);
    const info = await flux.ghInfo();
    if (!st.repo) {
      box.innerHTML = `<div class="pj-h"><span>GitHub</span></div>
        <div class="gh-box"><span class="gh-ic big">${icon('git', 20)}</span><span class="gh-txt"><b>${t('Save this project on GitHub')}</b><small>${t('Keep a backup online and share it with a link.')}</small></span>
        ${info.connected ? `<label class="gh-priv"><input type="checkbox" id="gh-private" checked> ${t('private')}</label><button class="s-btn" data-gh-publish>${icon('plus', 13)}${t('Publish')}</button>` : `<button class="s-btn" data-gh-settings>${t('Connect GitHub')}</button>`}</div>`;
    } else {
      const sync = [st.ahead ? `↑${st.ahead}` : '', st.behind ? `↓${st.behind}` : ''].filter(Boolean).join(' ');
      box.innerHTML = `<div class="pj-h"><span>${st.github ? 'GitHub' : 'Git'}</span><small>${esc(st.branch)}${sync ? ` · ${sync}` : ''}</small></div>
        <div class="gh-box col">
          <div class="gh-row"><span class="gh-ic big">${icon('git', 20)}</span><span class="gh-txt"><b>${esc(st.github || st.remote || t('Local repository'))}</b><small>${
            st.changes ? t('{n} changed file(s) not saved to GitHub yet', { n: st.changes }) : t('Everything is saved.')
          }</small></span>${st.github ? `<button class="icon-btn" data-gh-open title="${t('Open on GitHub')}">${icon('external', 14)}</button>` : ''}</div>
          ${st.changes ? `<div class="gh-files">${st.files.map((f) => `<span class="gh-file"><i class="st-${esc(f.state[0] || 'M')}">${esc(f.state || 'M')}</i>${fileIcon(f.file.split('/').pop())}${esc(f.file)}</span>`).join('')}</div>` : ''}
          <form class="gh-commit" id="gh-commit">
            <input id="gh-msg" placeholder="${t('What did you change? (e.g. Added a menu)')}" autocomplete="off" spellcheck="true">
            <button class="s-btn primary" ${!st.changes && !st.ahead ? 'disabled' : ''}>${icon('download', 13)}${st.remote ? t('Commit & push') : t('Commit')}</button>
            ${st.remote ? `<button type="button" class="s-btn" data-gh-pull title="${t('Get the newest version from GitHub')}">${t('Pull')}</button>` : ''}
          </form>
        </div>`;
    }
    box.onclick = async (e) => {
      if (e.target.closest('[data-gh-settings]')) return openSettingsTab('github');
      if (e.target.closest('[data-gh-open]')) return flux.openExternal(`https://github.com/${st.github}`);
      const pub = e.target.closest('[data-gh-publish]');
      if (pub) {
        await tools.status();
        if (!tools.info('git')?.installed && !(await tools.ask(tools.info('git'), tools.canInstall()))) return;
        pub.disabled = true;
        pub.innerHTML = `<span class="spin"></span>${t('Publishing…')}`;
        try {
          const res = await flux.ghPublish({ private: box.querySelector('#gh-private').checked });
          toast(t('Published on GitHub: {repo}', { repo: res.full }), 'ok', 6000);
        } catch (err) {
          toast(errText(err), 'error', 8000);
        }
        return renderCard(box);
      }
      const pl = e.target.closest('[data-gh-pull]');
      if (pl) {
        pl.disabled = true;
        try {
          await flux.gitPull();
          toast(t('You have the newest version.'), 'ok');
        } catch (err) {
          toast(errText(err), 'error', 8000);
        }
        return renderCard(box);
      }
    };
    const form = box.querySelector('#gh-commit');
    if (form)
      form.onsubmit = async (e) => {
        e.preventDefault();
        const btn = form.querySelector('button.primary');
        btn.disabled = true;
        btn.innerHTML = `<span class="spin"></span>${t('Saving…')}`;
        try {
          await flux.gitCommitPush(form.querySelector('#gh-msg').value.trim() || t('Update from Flux'));
          toast(st.remote ? t('Saved to GitHub.') : t('Changes committed.'), 'ok');
        } catch (err) {
          toast(errText(err), 'error', 9000);
        }
        renderCard(box);
      };
  }

  async function refreshStatus() {
    if (!getWorkspace()) return onStatus({ repo: false });
    try {
      lastStatus = await flux.gitStatus();
    } catch {
      lastStatus = { repo: false };
    }
    onStatus(lastStatus);
  }

  return { renderSettings, pickRepo, renderCard, refreshStatus, status: () => lastStatus };
}
