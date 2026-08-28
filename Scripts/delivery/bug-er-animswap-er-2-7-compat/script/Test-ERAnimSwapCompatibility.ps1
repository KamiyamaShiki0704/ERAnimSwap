# Topic: BUG-ERAnimSwap-ER-2-7-compat
# Topic-Path: Topics/Gameplay/bug-er-animswap-er-2-7-compat
# Spec: spec.md@1.0
# Script-Version: 1.0
# Verifies: SPEC-ERAS-270-001, SPEC-ERAS-270-004
# Project-Relative-Path: Scripts/delivery/bug-er-animswap-er-2-7-compat/script/Test-ERAnimSwapCompatibility.ps1

param(
    [Parameter(Mandatory = $false)]
    [string]$GameExe = "F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe"
)

$ErrorActionPreference = "Stop"
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..\..")).Path
$gamePath = (Resolve-Path -LiteralPath $GameExe).Path
$sourceFiles = Get-ChildItem -LiteralPath (Join-Path $projectRoot "src") -Filter "*.rs" -File
$source = ($sourceFiles | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw }) -join "`n"

$forbiddenSourcePatterns = @(
    "CSTaskImp::wait_for_instance",
    ".run_recurring("
)

foreach ($pattern in $forbiddenSourcePatterns) {
    if ($source.Contains($pattern)) {
        throw "Compatibility regression: source still contains '$pattern'."
    }
}

$version = (Get-Item -LiteralPath $gamePath).VersionInfo.FileVersion
$hash = (Get-FileHash -LiteralPath $gamePath -Algorithm SHA256).Hash
Write-Host "Checking Elden Ring executable: $gamePath"
Write-Host "File version: $version"
Write-Host "SHA-256: $hash"

$previousGameExe = $env:ER_GAME_EXE
try {
    $env:ER_GAME_EXE = $gamePath
    Push-Location $projectRoot
    try {
        & cargo test runtime::tests::compatibility_ -- --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "Runtime symbol compatibility test failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }
}
finally {
    $env:ER_GAME_EXE = $previousGameExe
}

Write-Host "ERAnimSwap static compatibility probe passed."
