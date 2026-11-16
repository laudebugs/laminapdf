<#
.SYNOPSIS
Installs and uninstalls each Windows installer on the runner and checks what it leaves.

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

function Write-CheckResult([bool]$ok, [string]$what) {
    if ($ok) { Write-Host "ok      $what" } else { $script:failures.Add($what); Write-Host "FAILED  $what" }
}

# A registry value as a string, or $null when the key or the value is missing. `(default)` is the
# key's default value. Read through .NET, which tells a missing value from an empty one.
function Get-RegistryString([string]$key, [string]$name) {
    $hive, $path = $key -split ':\\', 2
    $root = switch ($hive) {
        'HKCU' { [Microsoft.Win32.Registry]::CurrentUser }
        'HKLM' { [Microsoft.Win32.Registry]::LocalMachine }
        default { throw "unknown hive in $key" }
    }
    $opened = $root.OpenSubKey($path)
    if (-not $opened) { return $null }
    try {
        $value = $opened.GetValue($(if ($name -eq '(default)') { '' } else { $name }))
        if ($null -eq $value) { return $null }
        return [string]$value
    } finally { $opened.Dispose() }
}

function Format-RegistryValue($value) { if ($null -eq $value) { 'none' } else { "'$value'" } }

# Whether .pdf's default class under `classes` is still `before`.
function Test-DefaultUnchanged([string]$classes, [string]$label, $before) {
    $default = Get-RegistryString "$classes\.pdf" '(default)'
    Write-CheckResult ($default -ceq $before) "$label leaves .pdf's default class as it was (before $(Format-RegistryValue $before), now $(Format-RegistryValue $default))"
}

# $defaultBefore is untyped: a [string] parameter would turn "no default" ($null) into ''.
function Test-Registered([string]$hive, [string]$label, $defaultBefore) {
    $classes = "${hive}:\Software\Classes"
    Write-CheckResult ($null -ne (Get-RegistryString "$classes\.pdf\OpenWithProgids" $progId)) "$label lists $progId in .pdf's OpenWithProgids"
    Test-DefaultUnchanged $classes $label $defaultBefore
    $command = Get-RegistryString "$classes\$progId\shell\open\command" '(default)'
    Write-CheckResult ($null -ne $command -and $command -match '^"(?<exe>[^"]+)" "%1"$') "$label's open command is `"<app>`" `"%1`" ($command)"
    if ($command -match '^"(?<exe>[^"]+)"') {
        $exe = $Matches.exe
        Write-CheckResult (Test-Path -LiteralPath $exe -PathType Leaf) "$label's open command runs an installed file ($exe)"
        if ($Signed) {
            Write-CheckResult ((Get-AuthenticodeSignature -LiteralPath $exe).Status -eq 'Valid') "$label's app has a Valid signature"
        }
        return $exe
    }
    return $null
}

function Test-Removed([string]$hive, [string]$label, $defaultBefore, [string]$exe) {
    $classes = "${hive}:\Software\Classes"
    Write-CheckResult ($null -eq (Get-RegistryString "$classes\.pdf\OpenWithProgids" $progId)) "$label removes $progId from .pdf's OpenWithProgids"
    Write-CheckResult (-not (Test-Path -LiteralPath "$classes\$progId")) "$label removes the $progId class"
    Test-DefaultUnchanged $classes $label $defaultBefore
    if ($exe) { Write-CheckResult (-not (Test-Path -LiteralPath $exe)) "$label removes the app ($exe)" }
}

function Invoke-Installer([string]$file, [string[]]$arguments) {
    $process = Start-Process -FilePath $file -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "$file $($arguments -join ' ') exited with $($process.ExitCode)" }
}

$nsis = @(Get-ChildItem -LiteralPath $Dir -Filter '*-setup.exe')
$msi = @(Get-ChildItem -LiteralPath $Dir -Filter '*.msi')
if ($nsis.Count -ne 1) { Write-Host "::error::expected one NSIS installer in $Dir, found $($nsis.Count)"; exit 1 }

# The MSI first, per machine: HKLM, and the install flavour that turns the updater off. Tauri's MSI
# installs where an earlier install of either kind went (HKCU\Software\<manufacturer>\<product>), so
# it goes before the NSIS installer, whose uninstall leaves that key.
foreach ($package in $msi) {
    $before = Get-RegistryString 'HKLM:\Software\Classes\.pdf' '(default)'
    $log = Join-Path $env:RUNNER_TEMP "$($package.BaseName).install.log"
    Invoke-Installer 'msiexec.exe' @('/i', "`"$($package.FullName)`"", '/qn', '/l*v', "`"$log`"")
    $exe = Test-Registered 'HKLM' 'the MSI install' $before
    Write-CheckResult ($null -ne $exe -and $exe.StartsWith("$env:ProgramFiles\", [StringComparison]::OrdinalIgnoreCase)) "the MSI installs under Program Files ($exe)"
    Write-CheckResult ((Get-RegistryString 'HKLM:\Software\Lamina' 'InstallFlavour') -eq 'msi') 'the MSI install writes InstallFlavour = msi'
    Invoke-Installer 'msiexec.exe' @('/x', "`"$($package.FullName)`"", '/qn')
    Test-Removed 'HKLM' 'the MSI uninstall' $before $exe
    Write-CheckResult ($null -eq (Get-RegistryString 'HKLM:\Software\Lamina' 'InstallFlavour')) 'the MSI uninstall removes InstallFlavour'
}

# The NSIS installer, per user (installMode currentUser): HKCU.
$before = Get-RegistryString 'HKCU:\Software\Classes\.pdf' '(default)'
Invoke-Installer $nsis[0].FullName @('/S')
$exe = Test-Registered 'HKCU' 'the NSIS install' $before
Write-CheckResult ($null -eq (Get-RegistryString 'HKLM:\Software\Lamina' 'InstallFlavour')) 'the NSIS install writes no InstallFlavour'
$uninstall = Get-RegistryString "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$product" 'UninstallString'
Write-CheckResult ($null -ne $uninstall) 'the NSIS install registers its uninstaller'
if ($uninstall) {
    $uninstaller = $uninstall.Trim('"')
    if ($Signed) {
        Write-CheckResult ((Get-AuthenticodeSignature -LiteralPath $uninstaller).Status -eq 'Valid') 'the NSIS uninstaller has a Valid signature'
    }
    Invoke-Installer $uninstaller @('/S')
    # The uninstaller copies itself to a temporary folder and returns at once.
    for ($i = 0; $i -lt 60 -and $exe -and (Test-Path -LiteralPath $exe); $i++) { Start-Sleep -Seconds 1 }
    Test-Removed 'HKCU' 'the NSIS uninstall' $before $exe
}

if ($failures.Count -gt 0) {
    Write-Host "::error::$($failures.Count) installer checks failed: $($failures -join '; ')"
    exit 1
}
Write-Host 'the installers register Lamina PDF without the default and uninstall cleanly'
