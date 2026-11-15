<#
.SYNOPSIS
Installs and uninstalls each Windows installer on the runner and checks what it leaves (todo D6d).

.DESCRIPTION
For the NSIS installer (per user, silent) and the MSI (per machine, quiet), checks that the
install:

- lists Lamina PDF in `.pdf`'s OpenWithProgids, with a class whose open command runs the installed
  app, and leaves `.pdf`'s default class as it was (ADR 0008 §10: registered, never the default);
- for the MSI, writes HKLM\Software\Lamina\InstallFlavour = msi, which turns the in-app updater off
  (ADR 0014); the NSIS install writes none;
- with -Signed, puts an app on disk whose signature is Valid;

and that the uninstall removes the app, the class and the OpenWithProgids entry, and leaves the
default class alone. The runner is a fresh VM, so nothing else registered Lamina PDF before.

.PARAMETER Dir
The folder holding the installers (the desktop-build action's output).

.PARAMETER Signed
Also check the installed app's Authenticode signature.
#>
param(
    [Parameter(Mandatory)][string]$Dir,
    [switch]$Signed
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$progId = 'LaminaPDF.Document'
$product = 'Lamina PDF'
$failures = [System.Collections.Generic.List[string]]::new()

function Check([bool]$ok, [string]$what) {
    if ($ok) { Write-Host "ok      $what" } else { $script:failures.Add($what); Write-Host "FAILED  $what" }
}

function Value([string]$key, [string]$name) {
    $item = Get-ItemProperty -LiteralPath $key -ErrorAction SilentlyContinue
    if (-not $item) { return $null }
    $property = $item.PSObject.Properties[$name]
    if ($property) { return [string]$property.Value }
    return $null
}

function Test-Registered([string]$hive, [string]$label, [string]$defaultBefore) {
    $classes = "${hive}:\Software\Classes"
    Check ($null -ne (Value "$classes\.pdf\OpenWithProgids" $progId)) "$label lists $progId in .pdf's OpenWithProgids"
    Check ((Value "$classes\.pdf" '(default)') -eq $defaultBefore) "$label leaves .pdf's default class as it was ('$defaultBefore')"
    $command = Value "$classes\$progId\shell\open\command" '(default)'
    Check ($null -ne $command -and $command -match '^"(?<exe>[^"]+)" "%1"$') "$label's open command is `"<app>`" `"%1`" ($command)"
    if ($command -match '^"(?<exe>[^"]+)"') {
        $exe = $Matches.exe
        Check (Test-Path -LiteralPath $exe -PathType Leaf) "$label's open command runs an installed file ($exe)"
        if ($Signed) {
            Check ((Get-AuthenticodeSignature -LiteralPath $exe).Status -eq 'Valid') "$label's app has a Valid signature"
        }
        return $exe
    }
    return $null
}

function Test-Removed([string]$hive, [string]$label, [string]$defaultBefore, [string]$exe) {
    $classes = "${hive}:\Software\Classes"
    Check ($null -eq (Value "$classes\.pdf\OpenWithProgids" $progId)) "$label removes $progId from .pdf's OpenWithProgids"
    Check (-not (Test-Path -LiteralPath "$classes\$progId")) "$label removes the $progId class"
    Check ((Value "$classes\.pdf" '(default)') -eq $defaultBefore) "$label leaves .pdf's default class as it was"
    if ($exe) { Check (-not (Test-Path -LiteralPath $exe)) "$label removes the app ($exe)" }
}

function Run([string]$file, [string[]]$arguments) {
    $process = Start-Process -FilePath $file -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "$file $($arguments -join ' ') exited with $($process.ExitCode)" }
}

$nsis = @(Get-ChildItem -LiteralPath $Dir -Filter '*-setup.exe')
$msi = @(Get-ChildItem -LiteralPath $Dir -Filter '*.msi')
if ($nsis.Count -ne 1) { Write-Host "::error::expected one NSIS installer in $Dir, found $($nsis.Count)"; exit 1 }

# The NSIS installer, per user (installMode currentUser): HKCU.
$before = Value 'HKCU:\Software\Classes\.pdf' '(default)'
Run $nsis[0].FullName @('/S')
$exe = Test-Registered 'HKCU' 'the NSIS install' $before
Check ($null -eq (Value 'HKLM:\Software\Lamina' 'InstallFlavour')) 'the NSIS install writes no InstallFlavour'
$uninstall = Value "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$product" 'UninstallString'
Check ($null -ne $uninstall) 'the NSIS install registers its uninstaller'
if ($uninstall) {
    if ($Signed) {
        $uninstaller = $uninstall.Trim('"')
        Check ((Get-AuthenticodeSignature -LiteralPath $uninstaller).Status -eq 'Valid') 'the NSIS uninstaller has a Valid signature'
    }
    Run ($uninstall.Trim('"')) @('/S')
    # The uninstaller copies itself to a temporary folder and returns at once.
    for ($i = 0; $i -lt 60 -and $exe -and (Test-Path -LiteralPath $exe); $i++) { Start-Sleep -Seconds 1 }
    Test-Removed 'HKCU' 'the NSIS uninstall' $before $exe
}

# The MSI, per machine: HKLM, and the install flavour that turns the updater off.
foreach ($package in $msi) {
    $before = Value 'HKLM:\Software\Classes\.pdf' '(default)'
    $log = Join-Path $env:RUNNER_TEMP "$($package.BaseName).install.log"
    Run 'msiexec.exe' @('/i', "`"$($package.FullName)`"", '/qn', '/l*v', "`"$log`"")
    $exe = Test-Registered 'HKLM' 'the MSI install' $before
    Check ((Value 'HKLM:\Software\Lamina' 'InstallFlavour') -eq 'msi') 'the MSI install writes InstallFlavour = msi'
    Run 'msiexec.exe' @('/x', "`"$($package.FullName)`"", '/qn')
    Test-Removed 'HKLM' 'the MSI uninstall' $before $exe
    Check ($null -eq (Value 'HKLM:\Software\Lamina' 'InstallFlavour')) 'the MSI uninstall removes InstallFlavour'
}

if ($failures.Count -gt 0) {
    Write-Host "::error::$($failures.Count) installer checks failed: $($failures -join '; ')"
    exit 1
}
Write-Host 'the installers register Lamina PDF without the default and uninstall cleanly'
