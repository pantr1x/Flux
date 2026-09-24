# Porovnanie Flux vs VS Code na tomto počítači (Windows PowerShell 5.1+).
# Obe appky otvoria ten istý testovací Python súbor; meria sa čas do zobrazenia okna (3×, medián),
# pamäť po 30 s (súčet všetkých procesov appky, „private working set“ ako v Správcovi úloh),
# veľkosť inštalátora a priečinka na disku. Na konci vypíše tabuľku (markdown) a skopíruje ju do schránky.
#
# Spustenie (zavri pred tým Flux aj VS Code):
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/pantr1x/Flux/claude/friendly-brahmagupta-i3lr2u/scripts/compare-vscode.ps1)))
param(
  [string]$Flux = '',
  [string]$Code = '',
  [int]$Runs = 3,
  [int]$Settle = 30
)
$ErrorActionPreference = 'Stop'

function Find-Exe($candidates) {
  foreach ($c in $candidates) { if ($c -and (Test-Path $c)) { return (Resolve-Path $c).Path } }
  return $null
}
if (-not $Flux) { $Flux = Find-Exe @("$env:LOCALAPPDATA\Programs\Flux\Flux.exe", "$env:ProgramFiles\Flux\Flux.exe") }
if (-not $Code) { $Code = Find-Exe @("$env:LOCALAPPDATA\Programs\Microsoft VS Code\Code.exe", "$env:ProgramFiles\Microsoft VS Code\Code.exe") }
if (-not $Flux) { throw 'Flux.exe not found – pass -Flux "C:\path\to\Flux.exe"' }
if (-not $Code) { throw 'VS Code (Code.exe) not found – pass -Code "C:\path\to\Code.exe"' }

$apps = @(
  @{ Name = 'Flux'; Exe = $Flux; Proc = [IO.Path]::GetFileNameWithoutExtension($Flux) },
  @{ Name = 'VS Code'; Exe = $Code; Proc = [IO.Path]::GetFileNameWithoutExtension($Code) }
)
foreach ($a in $apps) {
  if (Get-Process -Name $a.Proc -ErrorAction SilentlyContinue) { throw "$($a.Name) is running – close it first (nothing is measured while it is open)." }
}

# rovnaký malý projekt pre obe appky
$proj = Join-Path $env:TEMP 'flux-vs-code-test'
New-Item -ItemType Directory -Force -Path $proj | Out-Null
$py = Join-Path $proj 'main.py'
$lines = @('import os', 'import random', '', 'def roll(sides=6):', '    return random.randint(1, sides)', '')
for ($i = 1; $i -le 60; $i++) { $lines += "def task_$i(x):"; $lines += "    total = sum(roll() for _ in range(x))"; $lines += "    return total + $i"; $lines += '' }
$lines += 'if __name__ == "__main__":'
$lines += '    print(task_1(3), os.getcwd())'
Set-Content -Path $py -Value $lines -Encoding UTF8

function Get-PrivateMB($procName) {
  $ids = @(Get-Process -Name $procName -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
  if (-not $ids.Count) { return 0 }
  try {
    $perf = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -ErrorAction Stop | Where-Object { $ids -contains [int]$_.IDProcess }
    $sum = ($perf | Measure-Object -Property WorkingSetPrivate -Sum).Sum
    if ($sum) { return [math]::Round($sum / 1MB) }
  } catch {}
  return [math]::Round(((Get-Process -Id $ids | Measure-Object -Property WorkingSet64 -Sum).Sum) / 1MB)
}

function Stop-App($procName) {
  foreach ($p in @(Get-Process -Name $procName -ErrorAction SilentlyContinue)) { try { [void]$p.CloseMainWindow() } catch {} }
  for ($i = 0; $i -lt 50 -and (Get-Process -Name $procName -ErrorAction SilentlyContinue); $i++) { Start-Sleep -Milliseconds 200 }
  Get-Process -Name $procName -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  Start-Sleep -Seconds 2
}

function Measure-Start($a) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $p = Start-Process -FilePath $a.Exe -ArgumentList "`"$py`"" -PassThru
  $shown = $null
  while ($sw.Elapsed.TotalSeconds -lt 60) {
    $w = Get-Process -Name $a.Proc -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if ($w) { $shown = $sw.Elapsed.TotalSeconds; break }
    Start-Sleep -Milliseconds 50
  }
  return $shown
}

$result = @{}
foreach ($a in $apps) {
  Write-Host "`n== $($a.Name) ==" -ForegroundColor Cyan
  $times = @()
  $mem = 0
  for ($r = 1; $r -le $Runs; $r++) {
    $t = Measure-Start $a
    Write-Host ("  start {0}: {1:N2} s" -f $r, $t)
    if ($t) { $times += $t }
    if ($r -eq $Runs) {
      Write-Host "  waiting $Settle s, then measuring memory..."
      Start-Sleep -Seconds $Settle
      $mem = Get-PrivateMB $a.Proc
      Write-Host "  memory: $mem MB"
    }
    Stop-App $a.Proc
  }
  $sorted = @($times | Sort-Object)
  $median = if ($sorted.Count) { $sorted[[int][math]::Floor($sorted.Count / 2)] } else { $null }
  $dir = Split-Path $a.Exe
  $disk = [math]::Round(((Get-ChildItem -Path $dir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum) / 1MB)
  $result[$a.Name] = @{ Start = $median; Mem = $mem; Disk = $disk }
}

# veľkosť inštalátorov (najnovšie verzie)
$fluxSetup = '?'
$codeSetup = '?'
try {
  $rel = Invoke-RestMethod -UseBasicParsing -Uri 'https://api.github.com/repos/pantr1x/Flux/releases/latest'
  $exe = $rel.assets | Where-Object { $_.name -like '*.exe' } | Select-Object -First 1
  if ($exe) { $fluxSetup = [math]::Round($exe.size / 1MB) }
} catch {}
try {
  $h = Invoke-WebRequest -UseBasicParsing -Method Head -Uri 'https://update.code.visualstudio.com/latest/win32-x64-user/stable'
  $len = $h.Headers['Content-Length']
  if ($len) { $codeSetup = [math]::Round([double]("$len".Split(',')[0]) / 1MB) }
} catch {}

# rozšírenia vo VS Code
$ext = '?'
$codeCli = Join-Path (Split-Path $Code) 'bin\code.cmd'
if (Test-Path $codeCli) { try { $ext = @(& $codeCli --list-extensions 2>$null | Where-Object { $_ }).Count } catch {} }

$f = $result['Flux']
$c = $result['VS Code']
$fmt = { param($v) if ($null -eq $v) { '?' } else { '{0:N1} s' -f $v } }
$table = @"
| | Flux | VS Code |
|---|---|---|
| Installer download | $fluxSetup MB | $codeSetup MB |
| Size on disk | $($f.Disk) MB | $($c.Disk) MB |
| Start-up (window visible, median of $Runs) | $(& $fmt $f.Start) | $(& $fmt $c.Start) |
| RAM after opening a Python file ($Settle s) | $($f.Mem) MB | $($c.Mem) MB |
| Extensions installed in VS Code | – | $ext |
"@
$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name.Trim()
$ram = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)
$os = (Get-CimInstance Win32_OperatingSystem).Caption
$table += "`n`n_Measured on $os, $cpu, $ram GB RAM._"
Write-Host "`n$table`n" -ForegroundColor Green
try { Set-Clipboard -Value $table; Write-Host 'The table is in your clipboard.' } catch {}
Remove-Item -Recurse -Force $proj -ErrorAction SilentlyContinue
