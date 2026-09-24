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
          ? `<div class="s-row"><span class="gh-user">${info.user.avatar ? `<img src="${esc(info.user.avatar)}" alt="">` : ''}<span><b>${esc(info.user.name)}</b><small>@${esc(info.user.login)}</small></span></span><button class="s-btn" data-gh-disconnect>${t('Disconnect')}</button></div>
             <div class="s-row"><span><b>${t('Open a repository')}</b><small>${t('Pick one of your repositories – Flux opens it as a project.')}</small></span><button class="s-btn primary" data-gh-pick>${icon('github', 13)}${t('Choose…')}</button></div>`
          : `<div class="s-row"><span><b>${t('Sign in with GitHub')}</b><small>${t('Opens your browser – log in or create an account (also with Google) and press Authorize.')}</small></span><button class="s-btn primary" data-gh-signin>${icon('github', 13)}${t('Sign in')}</button></div>
             <details class="gh-more s-row-details"><summary>${t('Advanced: use a token')}</summary><div class="s-row"><span><b>${t('Personal access token')}</b><small>${t('Create one on GitHub with the “repo” permission, then paste it here.')} <a href="#" data-gh-newtoken>${t('Create token')}</a></small></span><span class="s-inline"><input type="password" id="gh-token" class="s-text" placeholder="ghp_… / github_pat_…" autocomplete="off" spellcheck="false"><button class="s-btn" data-gh-connect>${t('Connect')}</button></span></div></details>`
      }</div>
      <h3>Git</h3>
      <div class="tc-list">${gitTc ? tools.row(gitTc) : ''}</div>`;
    tools.bind(box);
    box.onclick = async (e) => {
      if (e.target.closest('[data-gh-signin]')) return signIn(box.querySelector('.s-group'), () => renderSettings(box));
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
      if (e.target.closest('[data-gh-pick]')) return pickRepo();
      if (e.target.closest('[data-gh-disconnect]')) {
        await flux.ghDisconnect();
        return renderSettings(box);
      }
    };
  }

  // ---------- prihlásenie cez prehliadač ----------
  // Zobrazí kód, skopíruje ho, otvorí GitHub a čaká na potvrdenie. onDone(user) po úspechu.
  async function signIn(box, onDone) {
    const back = box.innerHTML;
    let started;
    try {
      started = await flux.ghSignInStart();
    } catch (err) {
      return toast(errText(err), 'error', 8000);
    }
    try {
      await navigator.clipboard.writeText(started.userCode);
    } catch {}
    box.innerHTML = `<div class="gh-signin">
      <p>${t('A GitHub window opened and Flux types this code into it for you:')}</p>
      <div class="gh-code" title="${t('Click to copy')}">${esc(started.userCode)}</div>
      <ol class="gh-howto">
        <li>${t('Log in to GitHub if it asks (also with Google).')}</li>
        <li>${t('Press Continue, then Authorize.')}</li>
      </ol>
      <small>${t('If the code is not filled in, click the first box and press Ctrl+V – it is already copied.')}</small>
      <div class="gh-wait"><span class="spin"></span>${t('Waiting for GitHub…')}</div>
      <div class="s-inline"><button class="s-btn" data-gh-reopen>${icon('external', 13)}${t('Use my browser instead')}</button><button class="s-btn" data-gh-cancel>${t('Cancel')}</button></div>
    </div>`;
    const onClick = (e) => {
      if (e.target.closest('.gh-code')) navigator.clipboard.writeText(started.userCode).then(() => toast(t('Copied.'), 'ok', 1500));
      if (e.target.closest('[data-gh-reopen]')) {
        flux.ghSignInClose();
        flux.openExternal(started.url);
      }
      if (e.target.closest('[data-gh-cancel]')) flux.ghSignInCancel();
    };
    box.addEventListener('click', onClick);
    try {
      const user = await flux.ghSignInWait();
      flux.ghSignInClose();
      box.removeEventListener('click', onClick);
      if (!user) {
        box.innerHTML = back;
        return null;
      }
      toast(t('Connected as {user}.', { user: user.login }), 'ok');
      return onDone(user);
    } catch (err) {
      flux.ghSignInClose();
      box.removeEventListener('click', onClick);
      box.innerHTML = back;
      toast(errText(err), 'error', 8000);
      return null;
    }
  }

  // ---------- výber repozitára (nový projekt z GitHubu) ----------
  // Bez pripojenia sa účet pripojí priamo v tomto okne, potom sa hneď ukážu repozitáre.
  async function pickRepo() {
    const el = document.getElementById('ghpick');
    const close = () => (el.hidden = true);
    el.hidden = false;
    el.onkeydown = (e) => e.key === 'Escape' && close();

    const shell = (body) => {
      el.innerHTML = `<div class="np-card gh-card" role="dialog">
        <header><h2>${icon('github', 18)}${t('Open a repository')}</h2><button class="icon-btn" data-close title="${t('Close (Esc)')}">${icon('x', 16)}</button></header>${body}</div>`;
    };

    const info = await flux.ghInfo();
    if (!info.connected) {
      const tokenSteps = `<ol class="gh-steps">
            <li><span><b>${t('Create a token on GitHub')}</b><small>${t('The page opens with everything filled in – just press “Generate token” at the bottom and copy it.')}</small></span><button class="s-btn" data-gh-newtoken>${icon('external', 13)}${t('Open GitHub')}</button></li>
            <li><span><b>${t('Paste it here')}</b></span><span class="s-inline"><input type="password" id="gh-token" placeholder="ghp_… / github_pat_…" autocomplete="off" spellcheck="false"><button class="s-btn primary" data-gh-connect>${t('Connect')}</button></span></li>
          </ol>`;
      shell(`<div class="gh-connect">
          <p class="s-lead">${t('Connect your GitHub account once – then pick any of your repositories and Flux opens it as a project.')}</p>
          <button class="gh-big" data-gh-signin>${icon('github', 20)}<span><b>${t('Sign in with GitHub')}</b><small>${t('Opens your browser – log in or create an account (also with Google) and press Authorize.')}</small></span></button>
          <details class="gh-more"><summary>${t('Advanced: use a token')}</summary>${tokenSteps}</details>
          <p class="gh-foot">${t('Your login is stored encrypted on this computer. You can disconnect any time in Settings → GitHub.')}</p>
          <div class="gh-or"><span>${t('or open a public repository by link')}</span></div>
          <form class="gh-link" id="gh-link"><input id="gh-url" placeholder="https://github.com/owner/repo" spellcheck="false" autocomplete="off"><button class="s-btn">${t('Open')}</button></form>
        </div>`);
      el.onclick = async (e) => {
        if (e.target === el || e.target.closest('[data-close]')) return close();
        if (e.target.closest('[data-gh-newtoken]')) return flux.openExternal('https://github.com/settings/tokens/new?scopes=repo&description=Flux');
        if (e.target.closest('[data-gh-signin]')) return signIn(el.querySelector('.gh-connect'), () => pickRepo());
        const c = e.target.closest('[data-gh-connect]');
        if (c) {
          const token = el.querySelector('#gh-token').value.trim();
          if (!token) return el.querySelector('#gh-token').focus();
          c.disabled = true;
          try {
            const user = await flux.ghConnect(token);
            toast(t('Connected as {user}.', { user: user.login }), 'ok');
            return pickRepo();
          } catch (err) {
            c.disabled = false;
            return toast(errText(err), 'error', 7000);
          }
        }
      };
      el.querySelector('#gh-link').onsubmit = (e) => {
        e.preventDefault();
        const url = el.querySelector('#gh-url').value.trim();
        if (url) open(url);
      };
      (el.querySelector('[data-gh-signin]') || el.querySelector('#gh-token')).focus();
      return;
    }

    shell(`<div class="gh-who">${info.user?.avatar ? `<img src="${esc(info.user.avatar)}" alt="">` : ''}<span>${t('Signed in as {user}', { user: `<b>${esc(info.user?.login || '')}</b>` })}</span></div>
      <input class="s-search" id="gh-q" placeholder="${t('Search your repositories or paste a link…')}" spellcheck="false" autocomplete="off">
      <div class="gh-list" id="gh-list"><div class="s-loading"><span class="spin"></span> ${t('Loading…')}</div></div>
      <p class="gh-foot">${t('Flux downloads (clones) the repository into your projects folder and opens it. Next time it opens the downloaded copy and gets the newest changes.')}</p>`);
    let repos = [];
    const list = el.querySelector('#gh-list');
    const draw = (q = '') => {
      const link = /github\.com\/[\w.-]+\/[\w.-]+/i.test(q);
      const f = repos.filter((r) => r.full.toLowerCase().includes(q.toLowerCase()));
      list.innerHTML =
        (link ? `<button class="gh-repo" data-full="${esc(q)}"><span class="gh-ic">${icon('external', 16)}</span><span class="gh-txt"><b>${t('Open this link')}</b><small>${esc(q)}</small></span></button>` : '') +
        (f.length
          ? f
              .map(
                (r) =>
                  `<button class="gh-repo" data-full="${esc(r.full)}"><span class="gh-ic">${icon('github', 16)}</span><span class="gh-txt"><b>${esc(r.full)}</b><small>${esc(r.description || r.language || '')}</small></span>${r.private ? `<span class="gh-badge">${t('private')}</span>` : ''}</button>`,
              )
              .join('')
          : link
            ? ''
            : `<div class="s-loading">${t('No repositories found.')}</div>`);
    };
    flux
      .ghRepos()
      .then((r) => {
        repos = r;
        draw(el.querySelector('#gh-q')?.value || '');
      })
      .catch((err) => (list.innerHTML = `<div class="s-loading">${esc(errText(err))}</div>`));
    el.querySelector('#gh-q').oninput = (e) => draw(e.target.value.trim());
    el.querySelector('#gh-q').focus();
    el.onclick = async (e) => {
      if (e.target === el || e.target.closest('[data-close]')) return close();
      const b = e.target.closest('[data-full]');
      if (b) open(b.dataset.full, () => draw(el.querySelector('#gh-q')?.value || ''));
    };

  }

  async function open(full, onFail) {
    const el = document.getElementById('ghpick');
    await tools.status();
    if (!tools.info('git')?.installed) {
      el.hidden = true;
      if (!(await tools.ask(tools.info('git'), tools.canInstall()))) return;
      el.hidden = false;
    }
    const list = el.querySelector('#gh-list') || el.querySelector('.gh-connect');
    const name = full.replace(/^https?:\/\/github\.com\//i, '').replace(/\.git$/i, '');
    if (list) list.innerHTML = `<div class="s-loading"><span class="spin"></span> ${t('Downloading {repo}…', { repo: esc(name) })}</div>`;
    try {
      const dir = await flux.ghClone(full, await flux.projectRoot());
      el.hidden = true;
      await openProject(dir);
      toast(t('{repo} is ready.', { repo: name }), 'ok');
    } catch (err) {
      toast(errText(err), 'error', 8000);
      if (onFail) onFail();
      else pickRepo();
    }
  }

  // Pri otvorení projektu z GitHubu: potichu stiahne novinky (len keď nemáš neuložené zmeny).
  async function syncOnOpen() {
    try {
      const st = await flux.gitStatus();
      if (!st.repo || !st.github) return;
      const res = await flux.gitSync();
      if (res.pulled) {
        toast(t('Got {n} new change(s) from GitHub.', { n: res.pulled }), 'ok');
        return true;
      }
    } catch {}
    return false;
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
        <div class="gh-box"><span class="gh-ic big">${icon('github', 20)}</span><span class="gh-txt"><b>${t('Save this project on GitHub')}</b><small>${t('Keep a backup online and share it with a link.')}</small></span>
        ${info.connected ? `<label class="gh-priv"><input type="checkbox" id="gh-private" checked> ${t('private')}</label><button class="s-btn" data-gh-publish>${icon('plus', 13)}${t('Publish')}</button>` : `<button class="s-btn" data-gh-settings>${t('Connect GitHub')}</button>`}</div>`;
    } else {
      const sync = [st.ahead ? `↑${st.ahead}` : '', st.behind ? `↓${st.behind}` : ''].filter(Boolean).join(' ');
      box.innerHTML = `<div class="pj-h"><span>${st.github ? 'GitHub' : 'Git'}</span><small>${esc(st.branch)}${sync ? ` · ${sync}` : ''}</small></div>
        <div class="gh-box col">
          <div class="gh-row"><span class="gh-ic big">${icon('github', 20)}</span><span class="gh-txt"><b>${esc(st.github || st.remote || t('Local repository'))}</b><small>${
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

  return { renderSettings, pickRepo, renderCard, refreshStatus, syncOnOpen, status: () => lastStatus };
}
