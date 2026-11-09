# Windows: build the spike twice (without and with the COOP/COEP headers), run the isolation
# check with each build, then the benchmark with the plain one. CSVs land in results\.
#
#   $env:LAMINA_PDF_PDFIUM_DIR = "…"; scripts\run-all.ps1 <label> [full|quick]
param([Parameter(Mandatory)][string]$Label, [string]$Mode = "full")
$ErrorActionPreference = "Stop"
if (-not $env:LAMINA_PDF_PDFIUM_DIR) { throw "set LAMINA_PDF_PDFIUM_DIR to the folder printed by cargo xtask fetch-pdfium" }
Set-Location (Join-Path $PSScriptRoot "..")

function Invoke-Checked([scriptblock]$Block) { & $Block; if ($LASTEXITCODE) { throw "exit $LASTEXITCODE" } }
Invoke-Checked { pnpm install --frozen-lockfile }
New-Item -ItemType Directory -Force bin, results | Out-Null
Invoke-Checked { pnpm tauri build --no-bundle }
Copy-Item src-tauri\target\release\desktop-ipc.exe bin\desktop-ipc-plain.exe
Invoke-Checked { pnpm tauri build --no-bundle --config src-tauri/tauri.coi.conf.json }
Copy-Item src-tauri\target\release\desktop-ipc.exe bin\desktop-ipc-coi.exe

$webview2 = (Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue).pv
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$gpu = (Get-CimInstance Win32_VideoController | ForEach-Object { "$($_.Name) $($_.CurrentHorizontalResolution)x$($_.CurrentVerticalResolution)" }) -join "; "
@(
  "date: $((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mmZ'))",
  "os: $($os.Caption) $($os.Version) (build $($os.BuildNumber))",
  "rustc: $(rustc --version)",
  "cpu: $($cpu.Name), $($cpu.NumberOfLogicalProcessors) logical cores",
  "memory: $([math]::Round($os.TotalVisibleMemorySize / 1MB)) GiB",
  "gpu/display: $gpu",
  "webview2 runtime: $webview2"
) | Set-Content "results\$Label-machine.txt"

function Invoke-Run([string]$Exe, [string]$RunLabel, [string]$RunMode, [int]$Seconds) {
  $env:LAMINA_SPIKE_LABEL = $RunLabel
  $env:LAMINA_SPIKE_MODE = $RunMode
  $p = Start-Process -FilePath $Exe -PassThru
  if (-not $p.WaitForExit($Seconds * 1000)) { $p.Kill(); throw "$RunLabel timed out" }
  if ($p.ExitCode) { throw "$RunLabel exited with $($p.ExitCode)" }
}
Invoke-Run bin\desktop-ipc-plain.exe "$Label-headers-off" coi 120
Invoke-Run bin\desktop-ipc-coi.exe "$Label-headers-on" coi 120
Invoke-Run bin\desktop-ipc-plain.exe $Label $Mode 5400
Write-Output "results in $(Resolve-Path results)"
