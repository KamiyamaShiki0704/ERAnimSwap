# Topic: BUG-ERAnimSwap-ER-2-7-compat
# Topic-Path: Topics/Gameplay/bug-er-animswap-er-2-7-compat
# Spec: spec.md@1.0
# Script-Version: 1.1
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
$previousExpectedTask = $env:ER_EXPECT_REGISTER_TASK_RVA
try {
    $env:ER_GAME_EXE = $gamePath
    $env:ER_EXPECT_REGISTER_TASK_RVA = switch ($version) {
        '2.6.2.0' { '0xEB1FE0' }
        '2.7.0.0' { '0xEB3DE0' }
        '2.7.1.0' { '0xEB3E50' }
        default { throw "Executable $version has no reviewed task target. Audit it before claiming compatibility." }
    }
    Push-Location $projectRoot
    try {
        & cargo test --locked runtime::tests::compatibility_ -- --nocapture
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
    $env:ER_EXPECT_REGISTER_TASK_RVA = $previousExpectedTask
}

Write-Host "ERAnimSwap static signature checks passed. Object layout and visible reload still require in-game validation."
