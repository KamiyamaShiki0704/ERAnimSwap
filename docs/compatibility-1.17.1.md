# ERAnimSwap 0.2.1: App 1.17.1 Compatibility Evidence

## Scope and Identity

Inspection date: 2026-09-08. Build base: ERAnimSwap
`d517bb82d42de000464bb35768fb06c28d586ac1` plus this working-tree update.
Existing compatibility contract: `SPEC-ERAS-270-001` and
`SPEC-ERAS-270-004`, extended to executable 2.7.1.0 without changing weapon
mapping semantics. This is automated/static evidence, not human acceptance.

- Game file: `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe`
- File version: `2.7.1.0` (user-reported App Ver. 1.17.1).
- Size: 87,042,128 bytes.
- SHA-256: `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`.
- FSRS remains Git-pinned to `902851865d05069eda7dcfd6385c8881eadf29bf`.
  No vendored dependency or local FSRS modification is required.

## Findings

| Item | Prior evidence | 2.7.1.0 result |
| --- | --- | --- |
| Task registration RVA | 2.7.0.0: `0xEB3DE0` | `0xEB3E50`, unique signature |
| Crash-patch signature | Existing configured signature | Exactly one match |
| Singleton discovery | FD4 signature and required names | 7,488 candidates; required name strings present |
| Main-player displacement | `WorldChrMan + 0x1E508` | Instructions with this displacement exist |
| Reload list/count/timer | `0x1E668` / `0x1E670` / `0x1E678` | Not conclusively verified; unchanged in configuration |

The DLL already resolves task registration dynamically. The changed task RVA
therefore does not need a hard-coded runtime replacement. Do not add `0x70`
to every address or structure offset. This project does not use the HUD raycast
or debug-render entry points discussed in the supplied screenshot.

The inspected task registration, singleton discovery and parameter-row lookup
paths do not invoke FSRS's version-bound RVA table. An unsupported entry in
that table is not by itself evidence that this DLL aborts on 2.7.1.0.

Singleton name/signature presence does not prove live singleton resolution.
Likewise, a matching displacement does not prove the object type or full
weapon/parameter pointer chain. The offline instruction script found no
decoded hits for the three reload fields within its scanned function ranges;
that result is inconclusive, not evidence that those fields moved or vanished.

## Changes and Automated Checks

- Stop the reload request if applying the enabled crash patch fails. Cache
  patch success only after success. An earlier archive copy is not rolled back.
- Assert the independently recorded expected task RVA in the executable test.
- Reject missing and duplicate task signatures in negative fixtures.
- Retain existing detection, delayed switching and global reload behavior.
- Add optional offline field-displacement inspection using Python `pefile`
  and `capstone`; it does not attach to the game or modify the executable.

Results: `cargo check --offline`, `cargo test --locked` (17 tests with the new
EXE configured), `cargo clippy --locked --all-targets -- -D warnings`, and
`cargo build --locked --release` passed. The PowerShell compatibility entry
point passed all three executable tests on both 2.7.1.0 and retained 2.6.2.0.
The latter resolved task registration to `0xEB1FE0`.

Reproduction entry points, relative to this project:

```powershell
.\Scripts\delivery\bug-er-animswap-er-2-7-compat\script\Test-ERAnimSwapCompatibility.ps1 -GameExe '<eldenring.exe>'
python .\Scripts\delivery\bug-er-animswap-er-2-7-compat\script\inspect_reload_fields.py '<eldenring.exe>'
```

## Required In-game Acceptance

Built artifact: `target/delivery-0.2.1/weapon_animation_hotreload.dll`.
SHA-256: `C4EA3996A13BF742BF4BE6BFECE292F56114534EFFCD073CD34F799476BBEF0F`.
Existing installed DLL/configuration was not overwritten; no GitHub release
was published as part of this local check.

No game process was running during inspection. Final PASS/FAIL remains with
the user. Back up the active archive and previous DLL before testing in the
existing mod-enabled environment, with anti-cheat disabled as required by the
loader. Keep the existing mapping configuration.

1. Start the game and load a character; verify successful runtime task setup
   with RVA `0xEB3E50` and no compatibility or pointer validation errors.
2. Equip two weapons with different configured mapping IDs; verify logged
   detector values and archive replacement match the intended variants.
3. Wait for the configured delay and verify the actual moveset changes, not
   merely the copied file or queued-reload log. Confirm no extra weapon switch.
4. Repeat changes and reload the character; verify no crash or stuck animation.
5. If a patch/pointer error or crash occurs, stop testing and retain the log;
   fixed object fields require runtime investigation before claiming support.

This update does not implement per-player animation partition binding for
Seamless Co-op. The existing resource replacement remains global to c0000.
