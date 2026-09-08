# weapon-animation-hotreload

Rust DLL for Elden Ring animation archive swapping.

## Game Compatibility

Version `0.2.1` passes offline signature checks on official WW Elden Ring
executables `2.6.2.0` (App Ver. 1.16.2) and `2.7.1.0` (App Ver. 1.17.1),
retaining the previous `2.7.0.0` (App Ver. 1.17) resolution strategy.
New-version object layouts and visible hot reload still require in-game
acceptance; see [the 1.17.1 evidence report](docs/compatibility-1.17.1.md).
Task registration is resolved
from the running executable instead of using the old version-bound
`fromsoftware-rs` address. If the required runtime entry is missing or
ambiguous, the DLL logs the compatibility failure and stops before installing
its recurring task.

Version `0.2.1` also blocks a hot reload request when the enabled crash patch
fails, instead of continuing without that protection. This does not undo an
archive copy already completed before the reload request.

Run the offline executable probe with:

```powershell
.\Scripts\delivery\bug-er-animswap-er-2-7-compat\script\Test-ERAnimSwapCompatibility.ps1 `
  -GameExe "F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe"
```

The DLL polls the local player's active weapon, reads a configurable weapon param field such as `sp_atkcategory`, maps that ID to an animation archive variant in `chr/ExtraAnimation` or a detected `chr` folder, copies it over the active archive, then requests a DSAnimStudio-style character hot reload.

## Files

- `weapon_animation_hotreload.dll`
- `weapon_animation_hotreload.toml`

Use `weapon_animation_hotreload.example.toml` as the starting config and rename/copy it to `weapon_animation_hotreload.toml` next to the DLL.

## Build Note

The project does not require a sibling FSRS checkout. Cargo fetches
`eldenring` and `fromsoftware-shared` from the official
[`vswarte/fromsoftware-rs`](https://github.com/vswarte/fromsoftware-rs)
repository, with both packages pinned to commit
`902851865d05069eda7dcfd6385c8881eadf29bf` from the `v0.14.0` release.

```powershell
cargo build --release
```

For supplemental player animation archives such as `c0000_dlc01.anibnd.dcx`, the DLL now requests both `c0000_dlc01` and the base player character name `c0000` by default. To control this manually, set `also_reload_base_character = false` or use `reload_names = ["c0000_dlc01", "c0000"]` in the TOML.

To avoid replaying the weapon-switch animation, mapped weapon changes are delayed until the detected ID has remained stable for a short time. Tune `reload_delay_frames`, `stable_frames_required`, and `copy_before_delay` in the TOML if the reload feels too early or too late.

You can keep multiple detection methods in one config by defining `[[detectors]]`. A mapping can set `detector = "name"` to choose which detector produces the ID for that mapping. Mappings without `detector` use `active_detector`, and the old top-level `detect_field`/`hand` still work when no detector profiles are defined.

Use `startup_delay_seconds` to wait a few seconds after DLL load before initializing game pointers and recurring tasks.

## Seamless Co-op Limitation

The current file-replacement hot reload is global to the `c0000` character
resource. It cannot give two player instances different replacement archives at
the same time. The animation variants must remain separate load partitions;
merging every configured variant can exceed the game's stable animation count
and crash. True per-player support therefore requires a new instance-local
animation-resource binding that keeps only partitions selected by players who
are currently loaded.

### Read-only ownership probe

Version `0.2.0` includes an opt-in diagnostic probe for locating that binding:

```toml
per_player_probe_enabled = true
per_player_probe_every_frames = 300
```

The probe enumerates every loaded `PlayerIns`, evaluates each player's mapping
from that player's own equipment, and writes pointer topology to
`weapon_animation_hotreload.log` when a player is added, changed, respawned, or
removed. Relevant lines begin with `per-player probe`.

The probe is read-only: it does not bind resources, write animation pointers,
or change the existing local-player copy/reload behavior. Keep the normal
`copy_enabled` and `hot_reload_enabled` settings unchanged only when you also
want to exercise the existing local swap during the diagnostic session. Set
both to `false` for a topology-only session.
