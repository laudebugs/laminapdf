<#
.SYNOPSIS
Checks that every Windows file the release ships is signed by the owner, with a timestamp.

.DESCRIPTION
For each installer in -Dir (the NSIS `*-setup.exe` and the `.msi`), checks the installer, then
unpacks it (the MSI by an administrative install, `msiexec /a`; NSIS with 7-Zip) and checks every
PE file inside: `signtool verify /pa`, and Get-AuthenticodeSignature's status Valid, a signer whose
name is -Publisher and a timestamp. A PE file's signature, read from its certificate table, must also
use SHA-256 and carry an RFC 3161 timestamp rather than the older Authenticode countersignature. An
MSI keeps its signature in a stream of its own, so it gets the checks before that one only; the same
SignTool call signs it as every PE file. Any file that fails fails the step, with the list.

NSIS's plugin DLLs in $PLUGINSDIR count like any other file: the Tauri CLI signs its copies of them
before it builds the installer. The administrative install also writes a copy of the MSI database
into the folder, which isn't a file the MSI ships, so it is left out.

.PARAMETER Dir
The folder holding the installers (the desktop-build action's output).

.PARAMETER Publisher
The signer's name as the certificate gives it (CN): the owner's legal name.
#>
param(
    [Parameter(Mandatory)][string]$Dir,
    [Parameter(Mandatory)][string]$Publisher
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$signtool = $env:SIGNTOOL
if ([string]::IsNullOrEmpty($signtool)) { Write-Host '::error::SIGNTOOL is not set'; exit 1 }
$sevenZip = Join-Path $env:ProgramFiles '7-Zip\7z.exe'

$failures = [System.Collections.Generic.List[string]]::new()
$checked = 0

function Test-Pe([string]$file) {
    if ($file -match '\.(exe|dll|sys|ocx|msi)$') { return $true }
    $stream = [System.IO.File]::OpenRead($file)
    try {
        $header = [byte[]]::new(2)
        return ($stream.Read($header, 0, 2) -eq 2 -and $header[0] -eq 0x4D -and $header[1] -eq 0x5A)
    } finally { $stream.Dispose() }
}

# The PKCS #7 signature in a PE file's certificate table (the data directory's entry 4, whose
# address is a file offset), or $null when the file has none or isn't a PE file.
function Get-PeSignature([string]$file) {
    $bytes = [System.IO.File]::ReadAllBytes($file)
    if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) { return $null }
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($pe -lt 0 -or $pe + 24 -ge $bytes.Length) { return $null }
    $optional = $pe + 24
    $directories = switch ([BitConverter]::ToUInt16($bytes, $optional)) {
        0x10b { $optional + 96 }
        0x20b { $optional + 112 }
        default { return $null }
    }
    $offset = [BitConverter]::ToInt32($bytes, $directories + 32)
    $size = [BitConverter]::ToInt32($bytes, $directories + 36)
    if ($offset -le 0 -or $size -le 8 -or $offset + $size -gt $bytes.Length) { return $null }
    # WIN_CERTIFICATE: dwLength, wRevision, wCertificateType, then the signature.
    $length = [BitConverter]::ToInt32($bytes, $offset)
    $cms = [System.Security.Cryptography.Pkcs.SignedCms]::new()
    $cms.Decode([byte[]]$bytes[($offset + 8)..($offset + $length - 1)])
    return $cms
}

$sha256 = '2.16.840.1.101.3.4.2.1'
$rfc3161 = '1.3.6.1.4.1.311.3.3.1'

function Test-Signed([string]$file, [string]$label) {
    $script:checked++
    $problems = @()
    & $signtool verify /pa /q $file *> $null
    if ($LASTEXITCODE -ne 0) { $problems += 'signtool verify /pa failed' }
    $signature = Get-AuthenticodeSignature -LiteralPath $file
    if ($signature.Status -ne 'Valid') { $problems += "status $($signature.Status)" }
    $signer = if ($signature.SignerCertificate) {
        $signature.SignerCertificate.GetNameInfo([System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName, $false)
    } else { '' }
    if ($signer -ne $Publisher) { $problems += "signer '$signer'" }
    if (-not $signature.TimeStamperCertificate) { $problems += 'no timestamp' }
    if ($file -notmatch '\.msi$') {
        $cms = Get-PeSignature $file
        if (-not $cms) {
            $problems += 'no signature in the certificate table'
        } else {
            $info = $cms.SignerInfos[0]
            if ($info.DigestAlgorithm.Value -ne $sha256) { $problems += "digest $($info.DigestAlgorithm.FriendlyName)" }
            $stamped = $info.UnsignedAttributes | Where-Object { $_.Oid.Value -eq $rfc3161 }
            if (-not $stamped) { $problems += 'no RFC 3161 timestamp' }
        }
    }
    if ($problems.Count -eq 0) {
        Write-Host "ok      $label"
    } else {
        $script:failures.Add("$label ($($problems -join ', '))")
        Write-Host "FAILED  $label ($($problems -join ', '))"
    }
}

function Test-Unpacked([string]$root, [string]$installer) {
    foreach ($item in Get-ChildItem -LiteralPath $root -Recurse -File) {
        if (-not (Test-Pe $item.FullName)) { continue }
        $label = "$installer › $($item.FullName.Substring($root.Length).TrimStart('\'))"
        # msiexec /a's own copy of the database, unsigned: what ships is the .msi checked above.
        if ($item.DirectoryName -eq $root -and $item.Extension -eq '.msi') { continue }
        Test-Signed $item.FullName $label
    }
}

$installers = @(Get-ChildItem -LiteralPath $Dir -File | Where-Object { $_.Name -like '*-setup.exe' -or $_.Name -like '*.msi' })
if ($installers.Count -eq 0) { Write-Host "::error::no installer in $Dir"; exit 1 }

$work = Join-Path $env:RUNNER_TEMP 'signature-check'
foreach ($installer in $installers) {
    Test-Signed $installer.FullName $installer.Name
    $into = Join-Path $work $installer.BaseName
    New-Item -ItemType Directory -Force -Path $into | Out-Null
    if ($installer.Extension -eq '.msi') {
        $process = Start-Process msiexec.exe -Wait -PassThru -ArgumentList @('/a', "`"$($installer.FullName)`"", '/qn', "TARGETDIR=`"$into`"")
        if ($process.ExitCode -ne 0) { Write-Host "::error::msiexec /a failed on $($installer.Name): $($process.ExitCode)"; exit 1 }
    } else {
        & $sevenZip x -y "-o$into" $installer.FullName | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Host "::error::7-Zip couldn't unpack $($installer.Name)"; exit 1 }
    }
    Test-Unpacked $into $installer.Name
}

$summary = @("### Windows signatures", "", "$checked files checked; signer must be $Publisher.", "")
if ($failures.Count -gt 0) {
    $summary += 'Failed:'
    $summary += ($failures | ForEach-Object { "- $_" })
}
if ($env:GITHUB_STEP_SUMMARY) { $summary -join "`n" | Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY }
if ($failures.Count -gt 0) {
    Write-Host "::error::$($failures.Count) Windows files aren't signed by $Publisher"
    exit 1
}
Write-Host "$checked files signed by $Publisher"
