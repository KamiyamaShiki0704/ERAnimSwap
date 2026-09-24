# ERAnimSwap: Animation Capacity Experiment

Version `0.3.0-experimental.2` replaces weapon-driven archive swapping with a
targeted temporary-memory budget expansion for an observed animation load crash.
**Experimental: automated checks pass; the user confirms successful save loading,
character control and follow-up testing without apparent problems on the current
animation package.** This is not exhaustive acceptance of all animation events,
larger archives, other game builds or multiplayer. See the
[validation record](docs/animation-capacity/build.md).

Do not use experimental.1: its DLL entry reserved a64KiB stack frame even for
thread notifications and crashed on the game's small worker stacks. Version2
removes that buffer and isolates process initialization behind a non-inlined
function. The capacity budget and patch location are unchanged. The old failure
and new regression results are recorded in
[startup crash analysis](docs/animation-capacity/startup-crash-20260925.md).

## What Changes

The captured failing loading worker had a 128 KiB temporary arena. Animation
entry-vector growth from 5,395 to 8,092 slots requested 64,736 bytes with only
1,128 bytes left. The DLL raises the general EzWork worker constructor's budget
to **1 MiB**, allowing the existing allocator to obtain and bound a larger arena.
It does not disable checks, suppress a crash, resize live arenas, edit files,
merge animation packs, or provide unlimited physical memory.

This is a general-worker constructor shared by animation loading and other
jobs, not an animation-only allocator. For the 26 matching workers in the
captured process, allocating all enlarged arenas would add about 22.75 MiB.
Actual worker counts and total game memory vary. Other constructor budgets
and the global 512 KiB default are unchanged.

The exact game allocator was replayed offline: the original 128 KiB budget
reproduces the captured failure; 512 KiB and 1 MiB pass the 5,669-entry input.
This does **not** prove complete archive loading or correct playback. Details:
[crash analysis](docs/animation-capacity/crash-20260925.md),
[worker budget trace](docs/animation-capacity/thread-budget-research.md).

## Installation And Configuration

1. Use an offline MOD environment with anti-cheat already disabled. Back up
   your save and retain a known-working animation package.
2. Load `weapon_animation_hotreload.dll` through your existing startup DLL
   loader, before the game's work threads are created. Fully exit and restart
   the game. Runtime injection/hot reload is not supported.
3. Check `weapon_animation_hotreload.log` beside the DLL for
   `CAPACITY_PATCH_APPLIED`. `CAPACITY_PATCH_REFUSED` means the budget was not
   successfully installed; report that line rather than assuming it worked.
4. Test the previously failing archive unchanged. Check actual animations and
   events, 20 transitions, 5 load/travel cycles, and 10 minutes of observation.
   Initial character loading beyond 120 seconds requires separate diagnosis.

**No configuration or MOD path is needed for this test version.** Budget is
fixed at 1 MiB. Old `weapon_animation_hotreload.toml`, detectors, mappings,
startup delay and reload settings are ignored. The new compiled entry point
is `src/capacity/lib.rs`; old sources remain historical, uncompiled reference.
Do not also load an old swapping DLL that can overwrite the test package.

`CAPACITY_PATCH_APPLIED` proves the constructor byte changed, not that existing
workers changed or the archive passed. Only workers created after installation
receive the new budget. There is no hot-unload support. Remove the DLL from
the loader and restart to restore original behavior. Neither EXE nor archives
are modified on disk. Content beyond the expanded budget may still hit the
engine's original allocation failure; malformed content is not repaired.

## Compatibility

Only the audited WW App 1.17.1 / executable `2.7.1.0` is targeted:

- SHA256: `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`
- PE timestamp: `0x6A96B418`; image size: `0x5E0DA00`.
- Budget instruction: RVA `0xE83E18`; one changed byte: `0xE83E1F`, `02 -> 10`.

Runtime validation checks PE identity/layout, a unique complete constructor
window, and downstream allocation-chain instructions before any expansion
write. Different builds or conflicting modifications refuse. These are
compatibility guards, not cryptographic authentication; full EXE SHA256 is
also checked in the configured offline test. No fallback to guessed offsets.

**TAE late-content limits and Seamless Co-op are not addressed or certified by
this build.** It also does not implement per-player partition binding.

## Build And Automated Checks

```powershell
$env:ER_GAME_EXE = 'C:\path\to\Game\eldenring.exe'
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --locked --release
```

Without `ER_GAME_EXE`, the test suite explicitly skips the real EXE hash/guard
check. The other tests use isolated fixtures/executable memory, never a running
game. The standalone native allocator replay requires Python, pefile and capstone:

```powershell
python Scripts/delivery/feat-animation-capacity/script/Replay-AnimationScratch.py $env:ER_GAME_EXE
```

Native DLL startup regression (isolated child, no game launched):

```powershell
python Scripts/delivery/feat-animation-capacity/script/Test-DllSmallStack.py target/x86_64-pc-windows-msvc/release/weapon_animation_hotreload.dll --threads 100
```

This tests actual DLL thread notifications on64KiB reserved stacks. It copies
the DLL into a new ignored test directory to avoid overwriting a runtime log.

The existing FSRS dependency declarations remain pinned to upstream commit
`902851865d05069eda7dcfd6385c8881eadf29bf`, not a local checkout. The new capacity
entry itself does not call FSRS game APIs. Legacy behavior and configuration are
documented separately in [0.2.1 documentation](docs/legacy-swap-0.2.1.md).

Full dumps, generated assets, local status/plans and build products are ignored
and must not be published. No proprietary game assets are included. MIT license.
