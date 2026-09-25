# ERCapacityExpansion

An Elden Ring DLL that increases temporary memory available to loading workers,
allowing larger animation archives to load without running out of that space.

## Features

- Increases the temporary memory budget from **128 KiB to 1 MiB per worker**.
- Expands internal animation clip indices to **0..65,533**, retaining the two reserved values.
- Works with animation archives in external MOD folders. No path configuration required.
- Applies in memory at startup without modifying the game executable or animation files.
- Replaces the previous weapon-based archive swapping and hot-reload functionality.

The increase applies to general EzWork workers, including animation-loading
workers. It is not an animation file-size limit and does not provide unlimited
animation capacity. The clip-index expansion covers up to **65,534 combined
animation slots**, not 65,534 entries per TAE file. Other TAE and event limits
are not removed.

## Installation

1. Use an offline MOD setup with EAC disabled.
2. Add `CapacityExpansion.dll` to your MOD loader's startup DLL list.
3. Fully restart the game.

The DLL must load before the game creates its work threads. Injecting it into
an already running game is not supported.

No configuration file is required. The budget is fixed at 1 MiB, and settings
in the old `weapon_animation_hotreload.toml` are no longer used. Do not load the
old archive-swapping DLL or a previous capacity DLL alongside this version.

When upgrading from `weapon_animation_hotreload.dll`, update the loader entry
to `CapacityExpansion.dll` and remove the old entry to avoid loading both.

To uninstall, remove the DLL from your loader configuration and restart.

## Compatibility

- Elden Ring **App Ver. 1.17.1**, executable version **2.7.1.0** (WW).
- Incompatible game builds or conflicting patches are rejected.
- Does not provide per-player animation partitioning for Seamless Co-op.

A log is written beside the DLL as `weapon_animation_hotreload.log`.
`CAPACITY_PATCH_APPLIED` indicates the patch was installed;
`CAPACITY_PATCH_REFUSED` includes the reason it could not be applied.

## Build

Requires Rust and the Windows MSVC toolchain.

```powershell
cargo build --locked --release
```

Output: `target/x86_64-pc-windows-msvc/release/CapacityExpansion.dll`.

Cargo downloads the pinned FSRS dependencies from GitHub; no local FSRS checkout
is required.

## License

[MIT](LICENSE).
