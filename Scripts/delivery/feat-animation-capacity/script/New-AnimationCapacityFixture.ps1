# Spec: docs/animation-capacity/spec.md@1.0
# Creates loader stress fixtures, not new playable movesets. Never deploys them.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$SourceArchive,
    [Parameter(Mandatory)][string]$SoulsFormatsDll,
    [Parameter(Mandatory)][string]$OodleDll,
    [ValidateRange(2, 8)][int]$Multiplier = 2,
    [ValidateRange(1, 2048)][int]$MaxPayloadMiB = 1024
)

$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path
$source = (Resolve-Path -LiteralPath $SourceArchive).Path
$assembly = (Resolve-Path -LiteralPath $SoulsFormatsDll).Path
$oodle = (Resolve-Path -LiteralPath $OodleDll).Path
$sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
$root = Join-Path $project 'target/animation-capacity-fixtures'
$outDir = Join-Path $root ('{0}-{1}x' -f $sourceHash.Substring(0, 12), $Multiplier)
if (Test-Path -LiteralPath $outDir) { throw "Fixture already exists: $outDir" }
[void][Reflection.Assembly]::LoadFrom($assembly)
[SoulsFormats.Oodle]::Oodle6Ptr = [Runtime.InteropServices.NativeLibrary]::Load($oodle)
$binder = [SoulsFormats.BND4]::Read($source)
$original = @($binder.Files)
$clips = @($original | Where-Object { [IO.Path]::GetExtension($_.Name) -ieq '.hkx' })
if ($clips.Count -eq 0) { throw 'No HKX clips in source.' }
$ids = [Collections.Generic.HashSet[int]]::new()
$names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$nextId = @{}
$clipSpecs = @($clips | ForEach-Object {
    if ([IO.Path]::GetFileName($_.Name) -notmatch '^a(\d{3})_(\d{6})\.hkx$') {
        throw "Unsupported clip name: $($_.Name)"
    }
    $prefix = [int]$Matches[1]
    $animation = [int]$Matches[2]
    if ($_.ID -ne 1000000000 + $prefix * 1000000 + $animation) {
        throw "Unsupported clip binder ID: $($_.ID)"
    }
    [pscustomobject]@{ File = $_; Prefix = $prefix }
})
foreach ($entry in $original) {
    if (-not $ids.Add($entry.ID) -or -not $names.Add($entry.Name)) {
        throw 'Source already contains duplicate binder IDs/names.'
    }
}
$originalPayload = [long](($original | ForEach-Object { $_.Bytes.LongLength } | Measure-Object -Sum).Sum)
$clipPayload = [long](($clips | ForEach-Object { $_.Bytes.LongLength } | Measure-Object -Sum).Sum)
$plannedPayload = $originalPayload + $clipPayload * ($Multiplier - 1)
if ($plannedPayload -gt [long]$MaxPayloadMiB * 1MB) {
    throw "Planned payload $plannedPayload exceeds configured safety budget."
}
$drive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($project))
if ($drive.AvailableFreeSpace -lt $plannedPayload * 3 + 256MB) { throw 'Insufficient staging space.' }

function Get-PayloadHash([byte[]]$Bytes) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($Bytes))
}
$expectations = [Collections.Generic.Dictionary[int,object]]::new()
foreach ($entry in $original) {
    $expectations.Add($entry.ID, [pscustomobject]@{
        Name = $entry.Name; Flags = $entry.Flags; Sha256 = Get-PayloadHash $entry.Bytes
        OriginalId = $entry.ID
    })
}
for ($round = 1; $round -lt $Multiplier; $round++) {
    foreach ($spec in $clipSpecs) {
        $file = $spec.File
        $prefix = $spec.Prefix
        if (-not $nextId.ContainsKey($prefix)) { $nextId[$prefix] = 999999 }
        do {
            $number = $nextId[$prefix]
            if ($number -lt 0) { throw "No free animation IDs under prefix $prefix." }
            $nextId[$prefix]--
            $id = 1000000000 + $prefix * 1000000 + $number
            $name = [IO.Path]::Combine([IO.Path]::GetDirectoryName($file.Name),
                ('a{0:D3}_{1:D6}.hkx' -f $prefix, $number))
        } while ($ids.Contains($id) -or $names.Contains($name))
        [void]$ids.Add($id)
        [void]$names.Add($name)
        $copy = [SoulsFormats.BinderFile]::new($file.Flags, $id, $name, $file.Bytes)
        $binder.Files.Add($copy)
        $expectations.Add($id, [pscustomobject]@{
            Name = $name; Flags = $copy.Flags; Sha256 = $expectations[$file.ID].Sha256
            OriginalId = $file.ID
        })
    }
}

[void](New-Item -ItemType Directory -Path $outDir)
$output = Join-Path $outDir ([IO.Path]::GetFileName($source))
if ([StringComparer]::OrdinalIgnoreCase.Equals($source, $output)) { throw 'Output collides with source.' }
Write-Host "Writing $Multiplier x clip fixture: $($binder.Files.Count) entries, $plannedPayload payload bytes."
$binder.Write($output)
$reopened = [SoulsFormats.BND4]::Read($output)
if ($reopened.Files.Count -ne $expectations.Count) { throw 'Round-trip entry count mismatch.' }
$seen = [Collections.Generic.HashSet[int]]::new()
foreach ($entry in $reopened.Files) {
    if (-not $seen.Add($entry.ID) -or -not $expectations.ContainsKey($entry.ID)) {
        throw "Round-trip unexpected/duplicate ID: $($entry.ID)"
    }
    $expected = $expectations[$entry.ID]
    if ($entry.Name -cne $expected.Name -or $entry.Flags -ne $expected.Flags -or
        (Get-PayloadHash $entry.Bytes) -ne $expected.Sha256) {
        throw "Round-trip entry mismatch: $($entry.ID)"
    }
}
if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne $sourceHash) {
    throw 'Source changed during generation; do not treat this fixture as verified.'
}
$manifest = [ordered]@{
    Spec = 'FEAT-ERAS-Animation-Capacity@1.0'; CreatedUtc = [DateTime]::UtcNow.ToString('o')
    Source = $source; SourceSha256 = $sourceHash
    SourceDiskBytes = (Get-Item -LiteralPath $source).Length
    SoulsFormatsSha256 = (Get-FileHash -LiteralPath $assembly -Algorithm SHA256).Hash
    OodleSha256 = (Get-FileHash -LiteralPath $oodle -Algorithm SHA256).Hash
    Multiplier = $Multiplier; OriginalEntries = $original.Count; OriginalClips = $clips.Count
    OutputEntries = $reopened.Files.Count; OutputClips = $clips.Count * $Multiplier
    OriginalPayloadBytes = $originalPayload; OutputPayloadBytes = $plannedPayload
    Output = $output; OutputDiskBytes = (Get-Item -LiteralPath $output).Length
    OutputSha256 = (Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash
    Compression = $binder.Compression.Type.ToString()
    RoundTripEveryPayloadVerified = $true; SourceUnchanged = $true
    RuntimeVerified = $false; Deployed = $false
    Limitation = 'Copied clips have unique IDs/names but no new TAE/behavior selectors. Loader stress only; lazy loading may not allocate them. Repeated clips do not model compression of unique clips.'
    AddedEntries = @($expectations.GetEnumerator() | Where-Object {$_.Key -ne $_.Value.OriginalId} |
        Sort-Object Key | ForEach-Object {
            [pscustomobject]@{Id=$_.Key; SourceId=$_.Value.OriginalId; Name=$_.Value.Name}
        })
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outDir 'manifest.json')
Write-Output $output
