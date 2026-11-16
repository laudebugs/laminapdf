<#
.SYNOPSIS
Authenticode-signs Windows files as the owner (docs/guidelines/desktop-release.md §1.5; ADR 0014 §6).

.DESCRIPTION
The Tauri CLI calls it for each file it signs (bundle.windows.signCommand: the app, the engine
sidecar, the installers, the NSIS uninstaller), and the release build's beforeBundleCommand calls it
with -Presign for the PE files the bundle copies as they are (PDFium's DLL). The desktop-build
action writes that configuration and sets the environment below; nothing else signs.

LAMINA_PDF_WIN_SIGN_PATH chooses the identity. `artifact-signing` (the only one set up: the owner
qualifies for Azure Artifact Signing) signs with SignTool and Microsoft's Artifact Signing dlib, which
authenticates through the Azure CLI session that azure/login opened with GitHub's OIDC token, so no
client secret exists. The runbook's fallback, an OV certificate in a cloud HSM, is refused until it
is set up and this script learns it.

  SIGNTOOL                    an x64 signtool.exe (the dlib is x64 only; Arm64 runners emulate it)
  ARTIFACT_SIGNING_DLIB       Azure.CodeSigning.Dlib.dll from Microsoft.ArtifactSigning.Client
  ARTIFACT_SIGNING_METADATA   the account's metadata.json: endpoint, account, certificate profile

Every file gets a SHA-256 signature with an RFC 3161 timestamp from Microsoft's timestamp service.
A failure is retried twice (the service and the timestamp server answer slowly at times), then
fails the build. The script prints no credential: it never sees one.

.PARAMETER Files
The files to sign.

.PARAMETER Presign
Sign the files LAMINA_PDF_WIN_PRESIGN lists (separated by `;`), leaving out those that don't
exist: the beforeBundleCommand's places PDFium may sit. The Tauri CLI hands that hook to cmd.exe
with any quotes kept as part of the arguments, so the hook passes no path.
#>
[CmdletBinding(DefaultParameterSetName = 'Files')]
param(
    [Parameter(Mandatory, ParameterSetName = 'Presign')]
    [switch]$Presign,
    [Parameter(Mandatory, ParameterSetName = 'Files', ValueFromRemainingArguments)]
    [string[]]$Files
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Exit-WithError([string]$message) {
    Write-Host "::error::sign-windows: $message"
    exit 1
}

function Get-RequiredVariable([string]$name) {
    $value = [Environment]::GetEnvironmentVariable($name)
    if ([string]::IsNullOrEmpty($value)) { Exit-WithError "$name is not set" }
    return $value
}

$path = Get-RequiredVariable 'LAMINA_PDF_WIN_SIGN_PATH'
if ($path -ne 'artifact-signing') {
    Exit-WithError "LAMINA_PDF_WIN_SIGN_PATH is '$path'; only artifact-signing is set up (runbook §1.5)"
}
$signtool = Get-RequiredVariable 'SIGNTOOL'
$dlib = Get-RequiredVariable 'ARTIFACT_SIGNING_DLIB'
$metadata = Get-RequiredVariable 'ARTIFACT_SIGNING_METADATA'
foreach ($tool in @($signtool, $dlib, $metadata)) {
    if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) { Exit-WithError "$tool doesn't exist" }
}

if ($Presign) {
    $Files = @((Get-RequiredVariable 'LAMINA_PDF_WIN_PRESIGN') -split ';' | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) })
    if ($Files.Count -eq 0) { Exit-WithError "none of LAMINA_PDF_WIN_PRESIGN's files exists" }
}

$attempts = 3
foreach ($file in $Files) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { Exit-WithError "$file doesn't exist" }
    for ($attempt = 1; ; $attempt++) {
        & $signtool sign /v /fd SHA256 /tr 'http://timestamp.acs.microsoft.com' /td SHA256 `
            /dlib $dlib /dmdf $metadata $file
        if ($LASTEXITCODE -eq 0) { break }
        if ($attempt -ge $attempts) { Exit-WithError "signtool failed on $file after $attempts attempts" }
        $wait = 15 * $attempt
        Write-Host "::warning::sign-windows: signtool failed on $file (attempt $attempt); trying again in $wait s"
        Start-Sleep -Seconds $wait
    }
    Write-Host "signed $file"
}
