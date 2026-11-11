# Windows: build the spike, print every fixture through every path into PDFs (no dialog) with
# Microsoft Print to PDF and WebView2, score them, then screenshot each path's print dialog.
#
#   $env:LAMINA_PDF_PDFIUM_DIR = "…"; scripts\run-all.ps1 <label>
param([Parameter(Mandatory)][string]$Label)
$ErrorActionPreference = "Stop"
if (-not $env:LAMINA_PDF_PDFIUM_DIR) { throw "set LAMINA_PDF_PDFIUM_DIR to the folder printed by cargo xtask fetch-pdfium" }
Set-Location (Join-Path $PSScriptRoot "..")

function Invoke-Checked([scriptblock]$Block) { & $Block; if ($LASTEXITCODE) { throw "exit $LASTEXITCODE" } }
Invoke-Checked { pnpm install --frozen-lockfile }
Invoke-Checked { pnpm tauri build --no-bundle }
Push-Location src-tauri; Invoke-Checked { cargo build --release --bin compare }; Pop-Location
New-Item -ItemType Directory -Force results\out, results\dialogs | Out-Null
$app = "src-tauri\target\release\desktop-print.exe"

$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$webview2 = (Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue).pv
$printers = (Get-Printer | ForEach-Object { $_.Name }) -join "; "
@(
  "date: $((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mmZ'))",
  "os: $($os.Caption) $($os.Version) (build $($os.BuildNumber))",
  "rustc: $(rustc --version)",
  "cpu: $($cpu.Name), $($cpu.NumberOfLogicalProcessors) logical cores",
  "memory: $([math]::Round($os.TotalVisibleMemorySize / 1MB)) GiB",
  "webview2 runtime: $webview2",
  "printers: $printers"
) | Set-Content "results\$Label-machine.txt"

Remove-Item -ErrorAction SilentlyContinue "results\out\$Label-*.pdf", "results\$Label-jobs.csv", "results\$Label-fidelity.csv"
$env:LAMINA_SPIKE_LABEL = $Label
$env:LAMINA_SPIKE_MODE = "auto"
$p = Start-Process -FilePath $app -PassThru
if (-not $p.WaitForExit(1800 * 1000)) { $p.Kill(); Write-Warning "auto run timed out" }
Invoke-Checked { & src-tauri\target\release\compare.exe fixtures results\out "results\$Label-fidelity.csv" }

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
# .NET resolves relative paths against the process's directory, not PowerShell's location.
$dialogs = (Resolve-Path results\dialogs).Path
foreach ($path in "raster", "vector", "webview", "windowprint") {
  $env:LAMINA_SPIKE_MODE = "dialog-$path"
  $p = Start-Process -FilePath $app -PassThru
  Start-Sleep -Seconds 12
  $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
  $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
  [System.Drawing.Graphics]::FromImage($bmp).CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
  $bmp.Save((Join-Path $dialogs "$Label-$path.png"), [System.Drawing.Imaging.ImageFormat]::Png)
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
}
Write-Output "results in $(Resolve-Path results)"
