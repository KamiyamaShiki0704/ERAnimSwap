# Spec: docs/animation-capacity/spec.md@1.0
# One-shot crash capture. Does not deploy assets, launch the game or install a debugger.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$TestArchive,
    [switch]$OfflineModEnvironment,
    [switch]$ValidateOnly
)

$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path
$archive = (Resolve-Path -LiteralPath $TestArchive).Path
$tool = Join-Path $project 'target/tools/procdump/procdump64.exe'
if (-not (Test-Path -LiteralPath $tool)) {
    throw 'ProcDump is missing. Obtain it from the official Sysinternals site first.'
}
$signature = Get-AuthenticodeSignature -LiteralPath $tool
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
    throw 'ProcDump Microsoft signature validation failed.'
}
$drive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($project))
if ($drive.AvailableFreeSpace -lt 20GB) { throw 'A full dump needs sufficient free space (20 GiB minimum for this run).' }
$identity = [ordered]@{
    TestArchive = $archive
    ArchiveBytes = (Get-Item -LiteralPath $archive).Length
    ArchiveSha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
    ProcDumpSha256 = (Get-FileHash -LiteralPath $tool -Algorithm SHA256).Hash
    Mode = 'Unhandled exception, including breakpoints; full dump; one dump only'
    Limitation = 'Archive identity records the intended input, not proof that the loader opened it. Preserve loader configuration and verify the actual path separately.'
}
if ($ValidateOnly) {
    $identity['State'] = 'Validated only; no debugger attached or armed'
    $identity | ConvertTo-Json
    return
}
if (-not $OfflineModEnvironment) { throw 'Confirm the EAC-disabled offline MOD environment before capture.' }
if (@(Get-Process eldenring -ErrorAction SilentlyContinue).Count -ne 0) {
    throw 'Exit the current game first so this capture targets the next launch, not the old loaded archive.'
}
$directory = Join-Path $project ('target/animation-capacity-crashes/{0}' -f
    [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff'))
[void](New-Item -ItemType Directory -Path $directory)
$identity['PreparedUtc'] = [DateTime]::UtcNow.ToString('o')
$identity | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $directory 'input.json')
Write-Host "Capture ready for the next eldenring.exe launch. Output: $directory"
Write-Host 'Start through your existing EAC-disabled MOD launcher. Full dumps may take time and contain private process data; do not publish them.'
& $tool -accepteula -ma -e -b -n 1 -w eldenring.exe $directory 2>&1 |
    ForEach-Object { $_.ToString() -replace "`0", '' } |
    Tee-Object -FilePath (Join-Path $directory 'procdump.log')
$captureExit = $LASTEXITCODE
$dumps = @(Get-ChildItem -LiteralPath $directory -Filter '*.dmp' -File)
$logText = Get-Content -LiteralPath (Join-Path $directory 'procdump.log') -Raw
$completionLogged = $logText -match 'Dump\s+\d+\s+complete:'
$headersValid = $dumps.Count -gt 0
foreach ($dump in $dumps) {
    $stream = [IO.File]::OpenRead($dump.FullName)
    try {
        $header = [byte[]]::new(32)
        $read = $stream.Read($header, 0, $header.Length)
        if ($read -ne 32 -or [Text.Encoding]::ASCII.GetString($header, 0, 4) -ne 'MDMP') {
            $headersValid = $false
            continue
        }
        $count = [BitConverter]::ToUInt32($header, 8)
        $directoryRva = [BitConverter]::ToUInt32($header, 12)
        if ($count -eq 0 -or ([long]$directoryRva + [long]$count * 12) -gt $stream.Length) {
            $headersValid = $false
        }
    } finally { $stream.Dispose() }
}
[ordered]@{
    FinishedUtc = [DateTime]::UtcNow.ToString('o'); ExitCode = $captureExit
    CompletionLogged = $completionLogged; DumpHeadersValid = $headersValid
    DumpFiles = @($dumps | Select-Object Name,Length)
    ArchiveUnchanged = ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -eq $identity.ArchiveSha256)
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $directory 'result.json')
if (-not $completionLogged -or -not $headersValid) { throw 'No confirmed crash capture. Inspect procdump.log; do not label this a passing capacity test.' }
if ($captureExit -ne 0) { Write-Warning "ProcDump exited $captureExit, but logged dump completion and produced valid dump headers. Inspect the dump separately." }
Write-Output $directory
