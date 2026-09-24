// Aktualizácie Fluxu: nové verzie sa berú z GitHub Releases (pantr1x/Flux).
// Automaticky: stiahne sa na pozadí a nainštaluje pri zatvorení Fluxu (dá sa vypnúť v Nastaveniach).
const { app, net } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const { t } = require('./i18n');

const REPO = 'pantr1x/Flux';
let autoUpdater = null;
try {
  ({ autoUpdater } = require('electron-updater'));
} catch {}

function createUpdater({ getSettings, send }) {
  const state = { version: app.getVersion(), status: 'idle', latest: '', progress: 0, error: '', canUpdate: app.isPackaged && !!autoUpdater };
  let notesCache = { at: 0, list: null };

  const emit = (patch) => {
    Object.assign(state, patch);
    send('update:state', { ...state });
  };
  const auto = () => getSettings().autoUpdate !== false;

  if (state.canUpdate) {
    autoUpdater.autoDownload = auto();
    autoUpdater.autoInstallOnAppQuit = true;
    autoUpdater.allowPrerelease = false;
    autoUpdater.on('checking-for-update', () => emit({ status: 'checking', error: '' }));
    autoUpdater.on('update-available', (info) => emit({ status: autoUpdater.autoDownload ? 'downloading' : 'available', latest: info.version, progress: 0 }));
    // Pri inštalácii pri zatvorení sa vždy použije posledná stiahnutá (najnovšia) verzia.
    autoUpdater.on('update-not-available', (info) => emit({ status: 'latest', latest: info?.version || state.version }));
    autoUpdater.on('download-progress', (p) => emit({ status: 'downloading', progress: Math.round(p.percent || 0) }));
    autoUpdater.on('update-downloaded', (info) => emit({ status: 'ready', latest: info.version, progress: 100 }));
    autoUpdater.on('error', (err) => emit({ status: 'error', error: String(err?.message || err).split('\n')[0] }));
  }

  // Poznámky pribalené v aplikácii (CHANGELOG.md) – fungujú aj bez internetu.
  function localNotes() {
    try {
      const log = fs.readFileSync(path.join(__dirname, '../../CHANGELOG.md'), 'utf8');
      return log
        .split(/^## /m)
        .slice(1)
        .map((part) => {
          const [head, ...rest] = part.split('\n');
          const [version, date] = head.split(/\s+[–-]\s+/);
          return { version: version.trim(), name: `Flux ${version.trim()}`, date: date ? new Date(date.trim()).toISOString() : '', body: rest.join('\n').trim(), url: '', local: true };
        });
    } catch {
      return [];
    }
  }

  // Poznámky k vydaniam: z GitHub Releases (novšie verzie) + pribalené v aplikácii.
  async function notes(force = false) {
    if (!force && notesCache.list && Date.now() - notesCache.at < 10 * 60 * 1000) return notesCache.list;
    let remote = [];
    try {
      const res = await net.fetch(`https://api.github.com/repos/${REPO}/releases?per_page=15`, { headers: { Accept: 'application/vnd.github+json', 'User-Agent': 'Flux' } });
      if (res.ok)
        remote = (await res.json())
          .filter((r) => !r.draft)
          .map((r) => ({ version: String(r.tag_name || '').replace(/^v/, ''), name: r.name || r.tag_name, date: r.published_at, body: r.body || '', url: r.html_url, prerelease: r.prerelease }));
    } catch {}
    const list = [...remote];
    for (const n of localNotes()) if (!list.some((r) => r.version === n.version)) list.push(n);
    list.sort((a, b) => (newer(a.version, b.version) ? -1 : newer(b.version, a.version) ? 1 : 0));
    notesCache = { at: Date.now(), list };
    return list;
  }

  async function check() {
    if (state.canUpdate) {
      autoUpdater.autoDownload = auto();
      try {
        await autoUpdater.checkForUpdates();
      } catch (err) {
        emit({ status: 'error', error: String(err?.message || err).split('\n')[0] });
      }
      return { ...state };
    }
    // Vývojová verzia (nie nainštalovaná): len zistí, aká verzia je najnovšia.
    emit({ status: 'checking', error: '' });
    try {
      const latest = (await notes(true)).find((r) => !r.prerelease)?.version || '';
      emit({ status: latest && newer(latest, state.version) ? 'available' : 'latest', latest });
    } catch (err) {
      emit({ status: 'error', error: err.message });
    }
    return { ...state };
  }

  async function download() {
    if (!state.canUpdate) return false;
    emit({ status: 'downloading', progress: 0 });
    await autoUpdater.downloadUpdate();
    return true;
  }

  // „Reštartovať teraz“: tichá inštalácia a Flux sa sám znova otvorí.
  // Najprv sa ešte pozrie, či medzitým nevyšla novšia verzia – inak by si musel aktualizovať dvakrát.
  // Značka v TEMP je poistka – inštalátor podľa nej Flux spustí, aj keby --force-run nezabral.
  let installing = false;
  async function install() {
    if (!(state.canUpdate && state.status === 'ready') || installing) return;
    installing = true;
    try {
      const ready = state.latest;
      autoUpdater.autoDownload = true;
      const r = await autoUpdater.checkForUpdates().catch(() => null);
      const newest = r?.updateInfo?.version;
      if (newest && newer(newest, ready)) {
        emit({ status: 'downloading', latest: newest, progress: 0 });
        await r.downloadPromise;
      }
    } catch {}
    try {
      fs.writeFileSync(path.join(require('node:os').tmpdir(), 'flux-relaunch-after-update'), String(Date.now()));
    } catch {}
    emit({ status: 'installing', progress: 100 });
    showInstallWindow(state.latest);
    setTimeout(() => autoUpdater.quitAndInstall(true, true), 900);
  }

  // Kým je Flux zatvorený a inštalátor beží potichu, malé okno Windows ukazuje priebeh.
  // Samostatný proces PowerShellu (Flux sa počas inštalácie nesmie spustiť) – zavrie sa,
  // keď sa Flux znova otvorí, najneskôr po 3 minútach. Ak by sa nepodarilo, aktualizácia ide ďalej.
  function showInstallWindow(version) {
    if (process.platform !== 'win32') return;
    const q = (s) => `'${String(s).replace(/'/g, "''")}'`;
    const title = t('Updating Flux to {v}…', { v: version });
    const note = t('Flux closes, installs the update and opens again by itself.');
    const ps = `
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()
$f = New-Object System.Windows.Forms.Form
$f.FormBorderStyle = 'None'; $f.StartPosition = 'CenterScreen'; $f.Size = New-Object System.Drawing.Size(420, 132)
$f.BackColor = [System.Drawing.Color]::FromArgb(24, 24, 30); $f.TopMost = $true; $f.ShowInTaskbar = $true; $f.Text = 'Flux'
$a = New-Object System.Windows.Forms.Label; $a.Text = ${q(title)}; $a.ForeColor = 'White'
$a.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 12); $a.AutoSize = $true; $a.Location = New-Object System.Drawing.Point(24, 20)
$b = New-Object System.Windows.Forms.Label; $b.Text = ${q(note)}; $b.ForeColor = [System.Drawing.Color]::FromArgb(160, 160, 176)
$b.Font = New-Object System.Drawing.Font('Segoe UI', 9); $b.AutoSize = $true; $b.Location = New-Object System.Drawing.Point(25, 50)
$p = New-Object System.Windows.Forms.ProgressBar; $p.Style = 'Marquee'; $p.MarqueeAnimationSpeed = 25
$p.Location = New-Object System.Drawing.Point(24, 86); $p.Size = New-Object System.Drawing.Size(372, 8)
$f.Controls.AddRange(@($a, $b, $p))
# proces je spustený skrytý – okno treba ukázať výslovne
Add-Type -Name W -Namespace FluxUpd -MemberDefinition '[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);'
$f.Add_Shown({ [FluxUpd.W]::ShowWindow($f.Handle, 5) | Out-Null; $f.Activate() })
$start = Get-Date
$t = New-Object System.Windows.Forms.Timer; $t.Interval = 700
$t.Add_Tick({
  # nový Flux (spustený inštalátorom po aktualizácii) = hotovo
  $flux = Get-Process -Name 'Flux' -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -gt $start.AddSeconds(3) }
  if ($flux -or ((Get-Date) - $start).TotalSeconds -gt 180) { $f.Close() }
})
$t.Start(); [void]$f.ShowDialog()
`;
    try {
      const { spawn } = require('node:child_process');
      const child = spawn('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-WindowStyle', 'Hidden', '-EncodedCommand', Buffer.from(ps, 'utf16le').toString('base64')], {
        detached: true,
        stdio: 'ignore',
        windowsHide: true,
      });
      child.unref();
    } catch {}
  }

  // Pri štarte: skontrolovať (a pri automatických aktualizáciách aj stiahnuť).
  function start() {
    setTimeout(check, 4000);
    setInterval(check, 6 * 60 * 60 * 1000);
  }

  return { state: () => ({ ...state }), check, download, install, notes, start };
}

function newer(a, b) {
  const pa = String(a).split('.').map(Number);
  const pb = String(b).split('.').map(Number);
  for (let i = 0; i < 3; i++) if ((pa[i] || 0) !== (pb[i] || 0)) return (pa[i] || 0) > (pb[i] || 0);
  return false;
}

module.exports = { createUpdater };
