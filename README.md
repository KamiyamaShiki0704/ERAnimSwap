# weapon-animation-hotreload

Rust DLL for Elden Ring/Nightreign-style animation archive swapping.

The DLL polls the local player's active weapon, reads a configurable weapon param field such as `sp_atkcategory`, maps that ID to an animation archive variant in `chr/ExtraAnimation` or a detected `chr` folder, copies it over the active archive, then requests a DSAnimStudio-style character hot reload.

## Files

- `weapon_animation_hotreload.dll`
- `weapon_animation_hotreload.toml`
- `chr/ExtraAnimation/c0000_dlc01.anibnd_00.dcx`
- `chr/ExtraAnimation/c0000_dlc01.anibnd_01.dcx`
- `mod/chr/c0000_dlc01.anibnd.dcx`

Use `weapon_animation_hotreload.example.toml` as the starting config and rename/copy it to `weapon_animation_hotreload.toml` next to the DLL.

## Build Note

This project currently depends on local path crates from `fromsoftware-rs-0.14.0`. By default, `Cargo.toml` expects that folder to exist at `../../fromsoftware-rs-0.14.0` relative to this project. Adjust the paths if your checkout layout is different.

With `auto_detect_paths = true`, the DLL searches near its own directory and near the game executable for any folder containing a `chr` directory with one of the mapped source files. The parent folder can be named `GA`, `mod`, or anything else.

The default hot reload offsets follow Elden-Ring-HKS-Hotloader's newer Elden Ring layout: `0x1E668`, `0x1E670`, `0x1E678`.

For supplemental player animation archives such as `c0000_dlc01.anibnd.dcx`, the DLL now requests both `c0000_dlc01` and the base player character name `c0000` by default. To control this manually, set `also_reload_base_character = false` or use `reload_names = ["c0000_dlc01", "c0000"]` in the TOML.

To avoid replaying the weapon-switch animation, mapped weapon changes are delayed until the detected ID has remained stable for a short time. Tune `reload_delay_frames`, `stable_frames_required`, and `copy_before_delay` in the TOML if the reload feels too early or too late.

You can keep multiple detection methods in one config by defining `[[detectors]]`. A mapping can set `detector = "name"` to choose which detector produces the ID for that mapping. Mappings without `detector` use `active_detector`, and the old top-level `detect_field`/`hand` still work when no detector profiles are defined.
