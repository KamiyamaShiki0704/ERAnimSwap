# weapon-animation-hotreload

Rust DLL for Elden Ring animation archive swapping.

The DLL polls the local player's active weapon, reads a configurable weapon param field such as `sp_atkcategory`, maps that ID to an animation archive variant in `chr/ExtraAnimation` or a detected `chr` folder, copies it over the active archive, then requests a DSAnimStudio-style character hot reload.

## Files

- `weapon_animation_hotreload.dll`
- `weapon_animation_hotreload.toml`

Use `weapon_animation_hotreload.example.toml` as the starting config and rename/copy it to `weapon_animation_hotreload.toml` next to the DLL.

## Build Note

For supplemental player animation archives such as `c0000_dlc01.anibnd.dcx`, the DLL now requests both `c0000_dlc01` and the base player character name `c0000` by default. To control this manually, set `also_reload_base_character = false` or use `reload_names = ["c0000_dlc01", "c0000"]` in the TOML.

To avoid replaying the weapon-switch animation, mapped weapon changes are delayed until the detected ID has remained stable for a short time. Tune `reload_delay_frames`, `stable_frames_required`, and `copy_before_delay` in the TOML if the reload feels too early or too late.

You can keep multiple detection methods in one config by defining `[[detectors]]`. A mapping can set `detector = "name"` to choose which detector produces the ID for that mapping. Mappings without `detector` use `active_detector`, and the old top-level `detect_field`/`hand` still work when no detector profiles are defined.
