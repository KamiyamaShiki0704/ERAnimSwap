# PLANS

## Implementation Plan

### Findings
- Existing projects in this repository use Rust `cdylib` DLLs loaded into the game process.
- The repository's `eldenring` crate exposes `WorldChrMan`, `CSTaskImp`, `PlayerGameData`, equipment slots, and `SoloParamRepository`.
- DSAnimStudio's live refresh for Elden Ring/Nightreign queues a character reload in `WorldChrMan` by inserting a small data node into the reload list at configurable offsets. Current defaults are:
  - reload list offset: `0x1E660`
  - reload count flag offset: `0x1E668`
  - reload timer offset: `0x1E670`
- DSAnimStudio also applies a small crash patch found by AOB scan before requesting reload.

### Build Plan
- Create a new Rust `cdylib` project in this folder.
- Add a TOML config next to the DLL, with defaults:
  - target archive: `mod/chr/c0000_dlc01.anibnd.dcx`
  - source directory: `chr/ExtraAnimation`
  - character reload name: `c0000_dlc01`
  - detection field: active hand `wepmotion_one_hand_id` or `wepmotion_both_hand_id`
  - mappings from detected ID to source archive filename.
- Poll the main player through `CSTaskImp::run_recurring`.
- Resolve the active weapon from `PlayerGameData.equipment.chr_asm`:
  - use right hand by default,
  - switch to both-hand ID if the player is two-handing that side,
  - allow config to choose left/right/auto and alternate fields such as weapon param ID or motion category.
- When the detected ID changes to a configured value:
  - copy the configured source file from `chr/ExtraAnimation` to a temporary file in the target directory,
  - replace the target archive with `rename` fallback to `copy`,
  - request in-game character reload through the DSAnimStudio-style `WorldChrMan` reload queue.
- Add logging and a README/example config so the DLL can be adjusted without recompiling.

### Risk Controls
- Only copy from paths relative to the configured game/mod root unless an absolute path is explicitly configured.
- Only trigger work on ID change, not every frame.
- Keep hot reload offsets configurable in case game versions differ.
- Build and run unit tests locally; runtime hot reload still needs in-game validation.

## Auto Path Detection Plan

### Reason
- The first implementation resolves relative paths from the game executable directory.
- This is inconvenient for Mod Engine / external mod layouts where the DLL lives outside the game directory and the mod content lives in another sibling folder such as `F:\GoldenAge\GA\chr`.

### Build Plan
- Add automatic `chr` directory detection enabled by default.
- Keep explicit absolute paths working exactly as before.
- Prefer paths that actually contain the target archive or one of the mapped source archive files.
- Search likely roots in this order:
  - explicit configured paths
  - DLL directory
  - DLL parent directories
  - sibling directories named `GA`, `mod`, `mods`, or `Mod`
  - game executable directory
- Resolve source files from the detected `chr` directory, preferring `ExtraAnimation` when it exists.
- Resolve target archive from the detected `chr` directory when the configured target is only a filename or starts with `chr/`.
- Preserve configurable overrides for unusual layouts.

## Supplemental Character Reload Name Plan

### Reason
- User's runtime log shows detection and file replacement now work, but the game does not visibly hot reload the changed archive.
- The current DLL only queues reload for `c0000_dlc01`.
- The referenced HkbEditor/HKS Hotloader reload path defaults to `c0000`.
- DSAnimStudio's player-character refresh logic treats `c0000` as the broad player character reload name and includes supplemental archives such as `c0000_dlc01.anibnd.dcx`.

### Build Plan
- Keep `character_reload_name` for backwards-compatible single-name configs.
- Add optional `reload_names = [...]` for explicit multi-name reload requests.
- Add `also_reload_base_character = true` by default so a configured name like `c0000_dlc01` also queues `c0000`.
- Queue hot reload once for each effective reload name and log each requested name.
- Update the example TOML and README so the template explains when to use `c0000_dlc01`, `c0000`, or both.

## Delayed Stable Reload Plan

### Reason
- Hot reload now works, but doing it immediately after the detected weapon category changes can interrupt or reset the player's weapon-switch animation.
- In-game behavior reported by the user: the switch animation replays after hot reload and can advance to the next weapon.

### Build Plan
- Add configurable timing controls:
  - `reload_delay_frames`: minimum number of frames to wait after a new mapped ID is first detected.
  - `stable_frames_required`: number of consecutive frames the same mapped ID must remain detected before applying it.
  - `copy_before_delay`: optional mode for copying immediately but delaying only hot reload; default false.
- Track a pending mapped ID in runtime state.
- Reset the pending timer whenever the detected ID changes or becomes unmapped/unavailable.
- Apply copy and hot reload only after the pending ID satisfies both delay and stability checks.
- Keep the existing "only apply a mapped ID once" behavior so the DLL does not repeatedly reload the same animation variant.

## Named Detector Profiles Plan

### Reason
- The current config has only one global `detect_field` and `hand`.
- User wants to define multiple possible detection methods in one TOML and choose which one is active without rewriting the rest of the config.

### Build Plan
- Keep existing `detect_field` and `hand` as the backward-compatible legacy/default detector.
- Add `active_detector = "name"` plus repeated `[[detectors]]` entries.
- Each detector entry has:
  - `name`
  - `detect_field`
  - optional `hand`, falling back to global `hand` when omitted.
- If detectors are defined and `active_detector` matches one, use that detector.
- If detectors are defined but `active_detector` is empty or missing, use the first detector.
- If `active_detector` does not match any detector, log the issue and fall back to the first detector.
- Keep mappings global for now; mapping IDs are interpreted according to the selected detector's output.

## Per-Mapping Detector Plan

### Reason
- User clarified they want each `[[mappings]]` entry to declare which detector should be used for that mapping.
- This allows one config to mix mappings driven by `sp_atkcategory`, weapon param ID, weapon type, etc.

### Build Plan
- Add optional `detector = "name"` to each mapping.
- A mapping without `detector` keeps backward-compatible behavior and uses the active/default detector.
- A mapping with `detector` uses that named detector profile regardless of `active_detector`.
- Evaluate mappings in TOML order and apply the first mapping whose selected detector output equals that mapping's `id`.
- Track last/pending/applied state by `(detector, id)` instead of only `id`.
- Log invalid mapping detector names at config load and skip those mappings at runtime.
- Update the example TOML, README, and tests.

## GitHub Upload Plan

### Reason
- User created `KamiyamaShiki0704/ERAnimSwap` and asked to upload the project.

### Build Plan
- Initialize this project folder as its own git repository.
- Add `.gitignore` to exclude build output, local runtime configs, logs, generated DLL/PDB/EXP/LIB artifacts, and editor files.
- Keep source code, `Cargo.toml`, `Cargo.lock`, README, example config, and project status/plan docs under version control.
- Add a README note that the Rust dependencies currently use local relative paths to `fromsoftware-rs-0.14.0`.
- Commit the initial project state and push `main` to `git@github.com:KamiyamaShiki0704/ERAnimSwap.git` using the configured GitHub SSH key.
