# Flux vs VS Code – záťažový test (Windows PowerShell 5.1+)
# Obe appky dostanú rovnaký projekt a rovnaký scenár: spustenie s malým súborom, 20 bežných súborov,
# súbor s 5 000 riadkami, veľký súbor s 50 000 riadkami, písanie do neho rýchlosťou človeka a dobeh.
# Celý čas sa meria RAM a CPU všetkých ich procesov (okno, GPU, rozšírenia, jazykový server pre Python).
# Na konci sa otvorí stránka s grafmi (flux-vs-code-report.html na Ploche).
#
# Beží s dočasnými profilmi – tvoje nastavenia, projekty ani otvorené okná sa nezmenia a Flux aj VS Code
# môžu ostať otvorené. Počas testu (asi 5 minút) nepoužívaj myš ani klávesnicu – test píše do okna.
param(
  [string]$Flux = '',
  [string]$Code = '',
  [int]$Cps = 8,              # rýchlosť písania: znaky za sekundu (8 ≈ 95 slov/min)
  [int]$TypeSeconds = 40,     # ako dlho sa píše
  [int]$Normal = 20,          # koľko bežných súborov (250 riadkov) sa otvorí
  [int]$Medium = 5000,        # riadky stredného súboru
  [int]$Big = 50000           # riadky veľkého súboru
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class FxWin {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte k, byte s, uint f, UIntPtr e);
}
'@

function Find-Exe($list) { foreach ($c in $list) { if ($c -and (Test-Path $c)) { return (Resolve-Path $c).Path } }; return $null }
if (-not $Flux) { $Flux = Find-Exe @("$env:LOCALAPPDATA\Programs\Flux\Flux.exe", "$env:ProgramFiles\Flux\Flux.exe") }
if (-not $Code) { $Code = Find-Exe @("$env:LOCALAPPDATA\Programs\Microsoft VS Code\Code.exe", "$env:ProgramFiles\Microsoft VS Code\Code.exe") }
if (-not $Flux) { throw 'Flux.exe not found – run with -Flux "C:\path\to\Flux.exe"' }
if (-not $Code) { throw 'VS Code (Code.exe) not found – run with -Code "C:\path\to\Code.exe"' }

# ---------- testovací projekt ----------
$root = Join-Path $env:TEMP ("flux-bench-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
$proj = Join-Path $root 'project'
New-Item -ItemType Directory -Force -Path $proj | Out-Null
function New-PyFile($path, $cls, $count) {
  $sb = New-Object System.Text.StringBuilder
  [void]$sb.AppendLine('import os'); [void]$sb.AppendLine('import math'); [void]$sb.AppendLine('import random'); [void]$sb.AppendLine('')
  [void]$sb.AppendLine("class ${cls}:"); [void]$sb.AppendLine('    def __init__(self, size):'); [void]$sb.AppendLine('        self.size = size')
  [void]$sb.AppendLine('        self.items = [random.random() for _ in range(size)]'); [void]$sb.AppendLine('')
  $lines = 9
  for ($i = 1; $lines -lt $count; $i++) {
    [void]$sb.AppendLine("    def step_$i(self, x):")
    [void]$sb.AppendLine('        total = sum(math.sqrt(v) for v in self.items[:x])')
    [void]$sb.AppendLine("        return round(total * $i / max(1, self.size), 3)")
    [void]$sb.AppendLine('')
    $lines += 4
  }
  [void]$sb.AppendLine('if __name__ == "__main__":')
  [void]$sb.AppendLine("    print($cls(10).step_1(5), os.getcwd())")
  [IO.File]::WriteAllText($path, $sb.ToString())
  return $path
}
$first = New-PyFile (Join-Path $proj 'main.py') 'Main' 60
$normalFiles = @(for ($f = 1; $f -le $Normal; $f++) { New-PyFile (Join-Path $proj ("module_{0:D2}.py" -f $f)) "Module$f" 250 })
$mediumFile = New-PyFile (Join-Path $proj 'medium_5k.py') 'Medium' $Medium
$bigFile = New-PyFile (Join-Path $proj 'big_50k.py') 'Big' $Big
$bigMb = [math]::Round((Get-Item $bigFile).Length / 1MB, 1)
$files = @($first)

# ---------- scenár (pevné časy, nech sa dajú appky porovnať v jednom grafe) ----------
$batches = @(@{ At = 15; Files = $normalFiles }, @{ At = 30; Files = @($mediumFile) }, @{ At = 45; Files = @($bigFile) })
$typeAt = 60
$typeEnd = $typeAt + $TypeSeconds
$endAt = $typeEnd + 15
$k = { param($n) if ($n -ge 1000) { '{0}k' -f [math]::Round($n / 1000) } else { "$n" } }
$phases = @(
  @{ name = 'Start (1 small file)'; short = 'Start'; start = 0; end = 15 },
  @{ name = "$Normal files open"; short = "$Normal files"; start = 15; end = 30 },
  @{ name = "+ file with $Medium lines"; short = "$(& $k $Medium) lines"; start = 30; end = 45 },
  @{ name = "+ file with $Big lines ($bigMb MB)"; short = "$(& $k $Big) lines"; start = 45; end = $typeAt },
  @{ name = "Typing in the big file ($Cps chars/s)"; short = 'Typing'; start = $typeAt; end = $typeEnd },
  @{ name = 'After typing'; short = 'After'; start = $typeEnd; end = $endAt }
)
$cores = [Environment]::ProcessorCount

# všetky procesy appky = potomkovia hlavného procesu (okno, GPU, rozšírenia, jazykový server…)
function Get-Tree($rootPid, $exeName) {
  $all = @(Get-CimInstance Win32_Process -Filter "Name='$exeName'" -Property ProcessId, ParentProcessId, PrivatePageCount, KernelModeTime, UserModeTime)
  $keep = @{}; $keep[[int]$rootPid] = $true
  $changed = $true
  while ($changed) {
    $changed = $false
    foreach ($p in $all) { if (-not $keep[[int]$p.ProcessId] -and $keep[[int]$p.ParentProcessId]) { $keep[[int]$p.ProcessId] = $true; $changed = $true } }
  }
  return @($all | Where-Object { $keep[[int]$_.ProcessId] })
}

function Run-App($name, $exe, $argsFor, $openArgs, $warm) {
  $exeName = [IO.Path]::GetFileName($exe)
  $profile = Join-Path $root ($name -replace '\W', '')
  New-Item -ItemType Directory -Force -Path $profile | Out-Null
  & $warm $profile
  Write-Host "`n== $name ==" -ForegroundColor Cyan

  $samples = New-Object System.Collections.ArrayList
  $prevCpu = @{}
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $main = Start-Process -FilePath $exe -ArgumentList (& $argsFor $profile @($files[0])) -PassThru
  $startup = $null
  $lastSample = -1.0
  $procs = @()

  $sample = {
    $t = $sw.Elapsed.TotalSeconds
    if ($t - $lastSample -lt 0.95) { return }
    $procs = Get-Tree $main.Id $exeName
    $mem = 0; $cpu = 0.0
    $now = @{}
    foreach ($p in $procs) {
      $mem += [double]$p.PrivatePageCount
      $c = ([double]$p.KernelModeTime + [double]$p.UserModeTime) / 1e7
      $now[[int]$p.ProcessId] = $c
      if ($prevCpu.ContainsKey([int]$p.ProcessId)) { $cpu += [math]::Max(0, $c - $prevCpu[[int]$p.ProcessId]) }
    }
    $dt = if ($lastSample -ge 0) { $t - $lastSample } else { 1 }
    # prvá vzorka nemá s čím porovnať CPU
    $cpuPct = if ($lastSample -ge 0) { [math]::Round([math]::Min(100, 100 * $cpu / $dt / $cores), 1) } else { 0 }
    [void]$samples.Add([pscustomobject]@{ t = [math]::Round($t, 2); mem = [math]::Round($mem / 1MB); cpu = $cpuPct; n = $procs.Count })
    # blok beží cez „.“ v rozsahu Run-App – priame priradenie mení jej premenné
    $lastSample = $t
    $prevCpu = $now
  }
  $waitUntil = {
    param($until)
    while ($sw.Elapsed.TotalSeconds -lt $until) {
      if (-not $startup) {
        $w = Get-Process -Id $main.Id -ErrorAction SilentlyContinue
        if ($w -and $w.MainWindowHandle -ne 0) { $startup = [math]::Round($sw.Elapsed.TotalSeconds, 2); Write-Host "  window after $startup s" }
      }
      . $sample
      Start-Sleep -Milliseconds 100
    }
  }

  . $waitUntil 15
  foreach ($b in $batches) {
    . $waitUntil $b.At
    Start-Process -FilePath $exe -ArgumentList (& $openArgs $profile @($b.Files)) | Out-Null
    Write-Host "  opened $(@($b.Files).Count) more file(s)"
  }
  . $waitUntil $typeAt

  # písanie: okno dopredu, na koniec posledného súboru a píše riadky kódu
  $h = (Get-Process -Id $main.Id).MainWindowHandle
  [void][FxWin]::ShowWindow($h, 9)
  [FxWin]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [FxWin]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)  # Alt – Windows potom dovolí presunúť okno dopredu
  [void][FxWin]::SetForegroundWindow($h)
  Start-Sleep -Milliseconds 600
  [System.Windows.Forms.SendKeys]::SendWait('^{END}{ENTER}{ENTER}')
  Write-Host "  typing for $TypeSeconds s..."
  $n = 0; $typed = 0
  while ($sw.Elapsed.TotalSeconds -lt $typeEnd) {
    $n++
    $line = "    value_$n = value_$($n - 1) * 2 + $n  # typed line $n"
    foreach ($ch in $line.ToCharArray()) {
      if ($sw.Elapsed.TotalSeconds -ge $typeEnd) { break }
      $k = [string]$ch
      if ('+^%~(){}[]'.Contains($k)) { $k = "{$k}" }
      [System.Windows.Forms.SendKeys]::SendWait($k)
      $typed++
      . $sample
      Start-Sleep -Milliseconds ([int](1000 / $Cps))
    }
    [System.Windows.Forms.SendKeys]::SendWait('{ESC}{ENTER}{HOME}+{END}{DEL}')  # nový riadok bez automatického odsadenia/návrhu
  }
  Write-Host "  typed $typed characters ($n lines)"
  . $waitUntil $endAt

  # zatvoriť (celý strom), dočasný profil ostane zmazaný na konci
  foreach ($p in (Get-Tree $main.Id $exeName)) { Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue }
  Start-Sleep -Seconds 2
  return [pscustomobject]@{ name = $name; startup = $startup; typed = $typed; samples = @($samples) }
}

# zahriatie: raz spustiť a zavrieť (disk cache, u Fluxu aj prvé rozbalenie Pythonu)
$fluxWarm = {
  param($profile)
  $ver = (Get-Item $Flux).VersionInfo.ProductVersion
  Set-Content (Join-Path $profile 'settings.json') ('{"onboarded":true,"lastSeenVersion":"' + $ver + '","autoUpdate":false}') -Encoding UTF8
  $p = Start-Process -FilePath $Flux -ArgumentList "--user-data-dir=`"$profile`"", "`"$($files[0])`"" -PassThru
  Start-Sleep -Seconds 12
  foreach ($x in (Get-Tree $p.Id 'Flux.exe')) { Stop-Process -Id $x.ProcessId -Force -ErrorAction SilentlyContinue }
  Start-Sleep -Seconds 2
}
$codeWarm = {
  param($profile)
  $p = Start-Process -FilePath $Code -ArgumentList '--user-data-dir', "`"$profile`"", '--disable-workspace-trust', "`"$($files[0])`"" -PassThru
  Start-Sleep -Seconds 12
  foreach ($x in (Get-Tree $p.Id ([IO.Path]::GetFileName($Code)))) { Stop-Process -Id $x.ProcessId -Force -ErrorAction SilentlyContinue }
  Start-Sleep -Seconds 2
}
$flux = Run-App 'Flux' $Flux `
  { param($profile, $f) @("--user-data-dir=`"$profile`"") + ($f | ForEach-Object { "`"$_`"" }) } `
  { param($profile, $f) @("--user-data-dir=`"$profile`"") + ($f | ForEach-Object { "`"$_`"" }) } $fluxWarm
$code = Run-App 'VS Code' $Code `
  { param($profile, $f) @('--user-data-dir', "`"$profile`"", '--disable-workspace-trust') + ($f | ForEach-Object { "`"$_`"" }) } `
  { param($profile, $f) @('--user-data-dir', "`"$profile`"", '-r') + ($f | ForEach-Object { "`"$_`"" }) } $codeWarm

# ---------- report ----------
$ext = @()
$cli = Join-Path (Split-Path $Code) 'bin\code.cmd'
if (Test-Path $cli) { try { $ext = @(& $cli --list-extensions 2>$null | Where-Object { $_ }) } catch {} }
$cpuName = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name.Trim()
$ramGb = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)
$osName = (Get-CimInstance Win32_OperatingSystem).Caption
$data = [pscustomobject]@{
  machine = "$osName · $cpuName · $ramGb GB RAM · $cores threads"
  when = (Get-Date).ToString('yyyy-MM-dd HH:mm')
  settings = "$Normal files × 250 lines, then $Medium lines, then $Big lines ($bigMb MB) · typing $Cps chars/s for $TypeSeconds s"
  versions = [pscustomobject]@{ flux = (Get-Item $Flux).VersionInfo.ProductVersion; code = (Get-Item $Code).VersionInfo.ProductVersion }
  extensions = $ext
  phases = $phases
  apps = @($flux, $code)
}
$json = $data | ConvertTo-Json -Depth 6 -Compress
$html = @'
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Flux vs VS Code</title>
<style>
  :root {
    color-scheme: light;
    --surface-0: #f3f3f1;
    --surface-1: #fcfcfb;
    --text-primary: #0b0b0b;
    --text-secondary: #52514e;
    --text-muted: #7c7b76;
    --line: #e4e3df;
    --band: rgba(0, 0, 0, 0.035);
    --flux: #2a78d6;
    --code: #eb6834;
    --good: #0b8a3a;
  }
  @media (prefers-color-scheme: dark) {
    :root:where(:not([data-theme="light"])) {
      color-scheme: dark;
      --surface-0: #121211;
      --surface-1: #1a1a19;
      --text-primary: #ffffff;
      --text-secondary: #c3c2b7;
      --text-muted: #8f8e86;
      --line: #2e2e2c;
      --band: rgba(255, 255, 255, 0.04);
      --flux: #3987e5;
      --code: #d95926;
      --good: #3fbf6b;
    }
  }
  * { box-sizing: border-box; }
  body { margin: 0; background: var(--surface-0); color: var(--text-primary); font: 14px/1.5 "Segoe UI", system-ui, sans-serif; }
  main { max-width: 1100px; margin: 0 auto; padding: 28px 16px 60px; }
  h1 { font-size: 26px; margin: 0 0 4px; letter-spacing: -0.5px; }
  h2 { font-size: 16px; margin: 0 0 2px; }
  .sub { color: var(--text-secondary); margin: 0; }
  .meta { color: var(--text-muted); font-size: 12.5px; margin: 6px 0 22px; }
  .legend { display: flex; gap: 18px; margin: 0 0 18px; color: var(--text-secondary); font-size: 13px; }
  .legend i { display: inline-block; width: 18px; height: 3px; border-radius: 2px; vertical-align: middle; margin-right: 6px; }
  .tiles { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; margin-bottom: 18px; }
  .tile { background: var(--surface-1); border: 1px solid var(--line); border-radius: 12px; padding: 12px 14px; }
  .tile small { color: var(--text-muted); font-size: 12px; }
  .tile .row { display: flex; align-items: baseline; gap: 8px; margin-top: 4px; }
  .tile .row span { color: var(--text-secondary); font-size: 12px; width: 58px; }
  .tile .row b { font-size: 19px; font-variant-numeric: tabular-nums; }
  .tile .row i { width: 8px; height: 8px; border-radius: 50%; display: inline-block; }
  .tile .win { color: var(--good); font-size: 12px; margin-top: 4px; }
  .card { background: var(--surface-1); border: 1px solid var(--line); border-radius: 14px; padding: 16px 16px 8px; margin-bottom: 16px; position: relative; }
  .card .hint { color: var(--text-muted); font-size: 12px; margin: 0 0 6px; }
  svg { display: block; width: 100%; height: auto; overflow: visible; }
  /* na úzkej obrazovke graf nezmenšovať do nečitateľna – posúva sa do strany */
  .plot { overflow-x: auto; }
  @media (max-width: 700px) { .plot svg { min-width: 640px; } }
  .axis text, .band-label { fill: var(--text-muted); font-size: 11px; }
  .grid line { stroke: var(--line); }
  .tip { position: absolute; pointer-events: none; background: var(--surface-1); border: 1px solid var(--line); border-radius: 10px; padding: 8px 10px; font-size: 12.5px; box-shadow: 0 8px 24px rgba(0,0,0,.15); display: none; white-space: nowrap; }
  .tip b { font-variant-numeric: tabular-nums; }
  .tip .ph { color: var(--text-muted); font-size: 11.5px; }
  table { width: 100%; border-collapse: collapse; font-size: 13px; font-variant-numeric: tabular-nums; }
  th, td { text-align: right; padding: 7px 8px; border-bottom: 1px solid var(--line); }
  th:first-child, td:first-child { text-align: left; }
  th { color: var(--text-secondary); font-weight: 600; }
  td.better { color: var(--good); font-weight: 600; }
  .note { color: var(--text-muted); font-size: 12.5px; }
  @media (max-width: 760px) { .tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @media (max-width: 480px) { h1 { font-size: 21px; } .tiles { grid-template-columns: 1fr; } }
</style>
</head>
<body>
<main>
  <h1>Flux vs VS Code under load</h1>
  <p class="sub" id="settings"></p>
  <p class="meta" id="meta"></p>
  <div class="legend"><span><i style="background:var(--flux)"></i><b id="lg-flux">Flux</b></span><span><i style="background:var(--code)"></i><b id="lg-code">VS Code</b></span></div>
  <div class="tiles" id="tiles"></div>
  <div class="card"><h2>Memory (RAM)</h2><p class="hint">All processes of each app added up – window, GPU, extensions and the Python language server. Lower is better.</p><div id="mem" class="plot"></div></div>
  <div class="card"><h2>CPU</h2><p class="hint">Share of the whole processor. Lower is better.</p><div id="cpu" class="plot"></div></div>
  <div class="card"><h2>Per phase</h2><p class="hint">Averages for each part of the test. Green = the lower (better) value.</p><div id="table"></div></div>
  <p class="note" id="notes"></p>
</main>
<script>
const DATA = __DATA__;
const $ = (s) => document.querySelector(s);
const css = (v) => getComputedStyle(document.documentElement).getPropertyValue(v).trim();
const [F, C] = DATA.apps;
const COLORS = ['--flux', '--code'];
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
$('#settings').textContent = DATA.settings;
$('#meta').textContent = `${DATA.machine} · ${DATA.when} · Flux ${DATA.versions.flux} · VS Code ${DATA.versions.code}` + (DATA.extensions?.length ? ` · VS Code extensions: ${DATA.extensions.join(', ')}` : '');

const avg = (a, k, s, e) => { const v = a.samples.filter((x) => x.t >= s && x.t < e).map((x) => x[k]); return v.length ? v.reduce((p, q) => p + q, 0) / v.length : NaN; };
const peak = (a, k, s, e) => Math.max(...a.samples.filter((x) => x.t >= s && x.t < e).map((x) => x[k]));
const P = DATA.phases;
const ph = (i) => P[i];
const fmtMb = (v) => (isNaN(v) ? '–' : `${Math.round(v)} MB`);
const fmtPct = (v) => (isNaN(v) ? '–' : `${v.toFixed(1)} %`);
const fmtS = (v) => (v == null ? '–' : `${Number(v).toFixed(1)} s`);

// dlaždice: kľúčové čísla vedľa seba
const tiles = [
  ['Start-up (window visible)', F.startup, C.startup, fmtS],
  ['RAM at start', avg(F, 'mem', 5, ph(0).end), avg(C, 'mem', 5, ph(0).end), fmtMb],
  [`RAM with the ${ph(3).name.replace('+ file with ', '')} file`, avg(F, 'mem', ph(3).start + 5, ph(3).end), avg(C, 'mem', ph(3).start + 5, ph(3).end), fmtMb],
  ['RAM while typing', avg(F, 'mem', ph(4).start, ph(4).end), avg(C, 'mem', ph(4).start, ph(4).end), fmtMb],
  ['CPU while typing', avg(F, 'cpu', ph(4).start, ph(4).end), avg(C, 'cpu', ph(4).start, ph(4).end), fmtPct],
  ['Peak RAM', peak(F, 'mem', 0, 1e9), peak(C, 'mem', 0, 1e9), fmtMb],
];
$('#tiles').innerHTML = tiles.map(([label, f, c, fmt]) => {
  const win = !isNaN(f) && !isNaN(c) && f !== c ? (f < c ? `Flux ${Math.round((1 - f / c) * 100)} % lower` : `VS Code ${Math.round((1 - c / f) * 100)} % lower`) : '';
  return `<div class="tile"><small>${esc(label)}</small>
    <div class="row"><i style="background:var(--flux)"></i><span>Flux</span><b>${fmt(f)}</b></div>
    <div class="row"><i style="background:var(--code)"></i><span>VS Code</span><b>${fmt(c)}</b></div>
    ${win ? `<div class="win">${win}</div>` : ''}</div>`;
}).join('');

// tabuľka po fázach
$('#table').innerHTML = `<table><thead><tr><th>Phase</th><th>Flux RAM</th><th>VS Code RAM</th><th>Flux CPU</th><th>VS Code CPU</th></tr></thead><tbody>${P.map((p) => {
  const fm = avg(F, 'mem', p.start, p.end), cm = avg(C, 'mem', p.start, p.end), fc = avg(F, 'cpu', p.start, p.end), cc = avg(C, 'cpu', p.start, p.end);
  const b = (a, o) => (a < o ? ' class="better"' : '');
  return `<tr><td>${esc(p.name)} <span class="note">${p.start}–${p.end} s</span></td><td${b(fm, cm)}>${fmtMb(fm)}</td><td${b(cm, fm)}>${fmtMb(cm)}</td><td${b(fc, cc)}>${fmtPct(fc)}</td><td${b(cc, fc)}>${fmtPct(cc)}</td></tr>`;
}).join('')}</tbody></table>`;
$('#notes').textContent = `Typed: Flux ${F.typed} characters, VS Code ${C.typed}. Both apps ran with a fresh temporary profile (VS Code with your installed extensions), after one warm-up start. Your own settings and windows were not touched.`;

// čiarový graf so zónami fáz, mriežkou, popiskami na konci čiar a krížom s popisom pri myši
function chart(el, key, unit, fmt) {
  const W = 1000, H = 300, m = { l: 48, r: 72, t: 26, b: 28 };
  const tMax = Math.max(...DATA.apps.flatMap((a) => a.samples.map((s) => s.t)), P[P.length - 1].end);
  const vMax = Math.max(1, ...DATA.apps.flatMap((a) => a.samples.map((s) => s[key])));
  const nice = (v) => { const p = 10 ** Math.floor(Math.log10(v)); return Math.ceil(v / p / (v / p > 5 ? 2 : 1)) * p * (v / p > 5 ? 2 : 1); };
  const top = key === 'cpu' ? Math.min(100, nice(vMax * 1.1)) : nice(vMax * 1.1);
  const x = (t) => m.l + (t / tMax) * (W - m.l - m.r);
  const y = (v) => H - m.b - (v / top) * (H - m.t - m.b);
  let s = `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(unit)} over time">`;
  P.forEach((p, i) => {
    if (i % 2 === 0) s += `<rect x="${x(p.start)}" y="${m.t}" width="${x(p.end) - x(p.start)}" height="${H - m.t - m.b}" fill="var(--band)"/>`;
    s += `<text class="band-label" x="${(x(p.start) + x(p.end)) / 2}" y="${m.t - 8}" text-anchor="middle">${esc(p.short || p.name)}</text>`;
  });
  s += '<g class="grid axis">';
  for (let i = 0; i <= 4; i++) { const v = (top / 4) * i; s += `<line x1="${m.l}" x2="${W - m.r}" y1="${y(v)}" y2="${y(v)}"/><text x="${m.l - 8}" y="${y(v) + 4}" text-anchor="end">${fmt(v)}</text>`; }
  for (let t = 0; t <= tMax; t += 15) s += `<text x="${x(t)}" y="${H - 8}" text-anchor="middle">${t}s</text>`;
  s += '</g>';
  // popisky na konci čiar – odsunuté od seba, keď sú čiary blízko
  const ends = DATA.apps.map((a, i) => { const l = a.samples[a.samples.length - 1]; return l ? { i, x: x(l.t) + 8, y: y(l[key]) + 4 } : null; }).filter(Boolean).sort((p, q) => p.y - q.y);
  for (let k = 1; k < ends.length; k++) if (ends[k].y - ends[k - 1].y < 15) ends[k].y = ends[k - 1].y + 15;
  DATA.apps.forEach((a, i) => {
    const pts = a.samples.map((p) => `${x(p.t).toFixed(1)},${y(p[key]).toFixed(1)}`).join(' ');
    s += `<polyline points="${pts}" fill="none" stroke="var(${COLORS[i]})" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`;
  });
  ends.forEach((e) => (s += `<text x="${e.x}" y="${e.y}" style="fill:var(--text-secondary);font-size:12px;font-weight:600">${esc(DATA.apps[e.i].name)}</text>`));
  s += `<line class="cross" x1="0" x2="0" y1="${m.t}" y2="${H - m.b}" stroke="var(--text-muted)" stroke-dasharray="3 3" visibility="hidden"/>`;
  DATA.apps.forEach((a, i) => (s += `<circle class="dot${i}" r="4.5" fill="var(${COLORS[i]})" stroke="var(--surface-1)" stroke-width="2" visibility="hidden"/>`));
  s += `<rect class="hit" x="${m.l}" y="${m.t}" width="${W - m.l - m.r}" height="${H - m.t - m.b}" fill="transparent"/></svg><div class="tip"></div>`;
  el.innerHTML = s;
  const svg = el.querySelector('svg'), tip = el.querySelector('.tip'), cross = el.querySelector('.cross');
  const near = (a, t) => a.samples.reduce((b, p) => (Math.abs(p.t - t) < Math.abs(b.t - t) ? p : b), a.samples[0]);
  el.querySelector('.hit').addEventListener('pointermove', (e) => {
    const r = svg.getBoundingClientRect();
    const t = ((e.clientX - r.left) / r.width * W - m.l) / (W - m.l - m.r) * tMax;
    const pts = DATA.apps.map((a) => near(a, t));
    cross.setAttribute('x1', x(t)); cross.setAttribute('x2', x(t)); cross.setAttribute('visibility', 'visible');
    pts.forEach((p, i) => { const d = el.querySelector(`.dot${i}`); d.setAttribute('cx', x(p.t)); d.setAttribute('cy', y(p[key])); d.setAttribute('visibility', 'visible'); });
    const phase = P.find((p) => t >= p.start && t < p.end);
    tip.innerHTML = `<div class="ph">${Math.round(t)} s${phase ? ` · ${esc(phase.name)}` : ''}</div>` + DATA.apps.map((a, i) => `<div><span style="color:var(${COLORS[i]})">●</span> ${esc(a.name)} <b>${fmt(pts[i][key])}</b></div>`).join('');
    tip.style.display = 'block';
    const cr = el.getBoundingClientRect();
    const left = e.clientX - cr.left + 14;
    tip.style.left = `${Math.min(left, cr.width - tip.offsetWidth - 8)}px`;
    tip.style.top = `${e.clientY - cr.top - 10}px`;
  });
  el.querySelector('.hit').addEventListener('pointerleave', () => { tip.style.display = 'none'; cross.setAttribute('visibility', 'hidden'); el.querySelectorAll('circle').forEach((c) => c.setAttribute('visibility', 'hidden')); });
}
chart($('#mem'), 'mem', 'RAM', (v) => `${Math.round(v)} MB`);
chart($('#cpu'), 'cpu', 'CPU', (v) => `${Math.round(v * 10) / 10} %`);
</script>
</body>
</html>

'@
$html = $html.Replace('__DATA__', $json)
$out = Join-Path ([Environment]::GetFolderPath('Desktop')) 'flux-vs-code-report.html'
Set-Content -Path $out -Value $html -Encoding UTF8
Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
Write-Host "`nReport: $out" -ForegroundColor Green
if ($env:CI) { Write-Host "REPORT_JSON:$json" } else { Start-Process $out }
