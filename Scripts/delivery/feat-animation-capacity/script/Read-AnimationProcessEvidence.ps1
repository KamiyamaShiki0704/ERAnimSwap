# Spec: docs/animation-capacity/spec.md@1.0
# Reads explicitly selected main-module ranges. Never writes game memory.
[CmdletBinding()]
param(
    [string[]]$Ranges = @('0x1FF000:0x9000', '0xCD0000:0x5000', '0x7E000:0x1000'),
    [string[]]$PointerRanges = @(),
    [int]$GameProcessId = 0
)

$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path
$games = @(Get-Process eldenring -ErrorAction Stop | Where-Object {
    $GameProcessId -eq 0 -or $_.Id -eq $GameProcessId
})
if ($games.Count -ne 1) { throw 'Select exactly one eldenring process.' }
$game = $games[0]
$module = $game.MainModule
$base = $module.BaseAddress.ToInt64()
$size = $module.ModuleMemorySize
$specs = @($Ranges | ForEach-Object { [pscustomobject]@{Text=$_; Follow=$false} }) +
    @($PointerRanges | ForEach-Object { [pscustomobject]@{Text=$_; Follow=$true} })
$requests = @($specs | ForEach-Object {
    if ($_.Text -notmatch '^0x([0-9a-fA-F]+):0x([0-9a-fA-F]+)$') {
        throw "Invalid RVA:length range: $($_.Text)"
    }
    $rva = [Convert]::ToInt64($Matches[1], 16)
    $length = [Convert]::ToInt32($Matches[2], 16)
    $moduleReadLength = if ($_.Follow) { 8 } else { $length }
    if ($length -le 0 -or $rva + $moduleReadLength -gt $size) { throw 'Range outside main module.' }
    [pscustomobject]@{ Rva = $rva; Length = $length; Follow = $_.Follow }
})
if (($requests | Measure-Object Length -Sum).Sum -gt 16MB) {
    throw 'Read budget exceeds 16 MiB. Select narrower loader ranges.'
}

if (-not ('AnimationCapacity.ReadOnlyProcess' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace AnimationCapacity {
    public static class ReadOnlyProcess {
        [DllImport("kernel32.dll", SetLastError=true)]
        public static extern IntPtr OpenProcess(uint access, bool inherit, int processId);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool ReadProcessMemory(IntPtr process, IntPtr address,
            [Out] byte[] buffer, UIntPtr size, out UIntPtr read);
        [DllImport("kernel32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool CloseHandle(IntPtr handle);
    }
}
'@
}
$handle = [AnimationCapacity.ReadOnlyProcess]::OpenProcess(0x1010, $false, $game.Id)
if ($handle -eq [IntPtr]::Zero) {
    throw "OpenProcess(read-only) failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
}
$directory = Join-Path $project ('target/animation-capacity-memory/{0}-{1}' -f
    [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff'), $game.Id)
try {
    [void](New-Item -ItemType Directory -Path $directory)
    $records = @($requests | ForEach-Object {
        $address = $base + $_.Rva
        if ($_.Follow) {
            $pointer = [byte[]]::new(8)
            $pointerRead = [UIntPtr]::Zero
            $ok = [AnimationCapacity.ReadOnlyProcess]::ReadProcessMemory(
                $handle, [IntPtr]$address, $pointer, [UIntPtr]8, [ref]$pointerRead)
            if (-not $ok -or $pointerRead.ToUInt64() -ne 8) { throw 'Cannot read module pointer.' }
            $address = [BitConverter]::ToInt64($pointer)
            if ($address -lt 0x10000 -or $address -gt (0x7FFFFFFFFFFF - $_.Length)) {
                throw 'Null or noncanonical object pointer; no heap read attempted.'
            }
        }
        $buffer = [byte[]]::new($_.Length)
        $read = [UIntPtr]::Zero
        $ok = [AnimationCapacity.ReadOnlyProcess]::ReadProcessMemory(
            $handle, [IntPtr]$address, $buffer, [UIntPtr]$buffer.Length, [ref]$read)
        if (-not $ok -or $read.ToUInt64() -ne $buffer.Length) {
            throw "Read failed at RVA $('0x{0:X}' -f $_.Rva); Win32=$([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
        }
        $prefix = if ($_.Follow) { 'pointer-rva' } else { 'rva' }
        $path = Join-Path $directory ('{0}-{1:X8}.bin' -f $prefix, $_.Rva)
        [IO.File]::WriteAllBytes($path, $buffer)
        [pscustomobject]@{
            Rva = ('0x{0:X}' -f $_.Rva); Length = $_.Length
            FollowedPointer = $_.Follow; ReadAddress = ('0x{0:X}' -f $address)
            File = [IO.Path]::GetFileName($path)
            Sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
        }
    })
    $manifest = [ordered]@{
        CapturedUtc = [DateTime]::UtcNow.ToString('o'); ProcessId = $game.Id
        Module = $module.FileName; Version = $module.FileVersionInfo.FileVersion
        ExeSha256 = (Get-FileHash -LiteralPath $module.FileName -Algorithm SHA256).Hash
        BaseAddress = ('0x{0:X}' -f $base); ModuleSize = $size
        Access = 'PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ'
        GameMemoryWrites = $false; Ranges = $records
    }
    $manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $directory 'manifest.json')
    Write-Output $directory
} finally {
    [void][AnimationCapacity.ReadOnlyProcess]::CloseHandle($handle)
}
