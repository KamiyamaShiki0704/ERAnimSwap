use std::{
    collections::HashMap,
    ffi::c_void,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use eldenring::{
    cs::{
        CSTaskGroupIndex, CSTaskImp, ChrAsm, ChrAsmArmStyle, EquipParamWeapon, SoloParamRepository,
        WorldChrMan,
    },
    fd4::FD4TaskData,
};
use fromsoftware_shared::{FromStatic, SharedTaskImpExt};
use serde::Deserialize;
use windows::Win32::{
    Foundation::HINSTANCE,
    System::{
        LibraryLoader::GetModuleHandleW,
        Memory::{PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect},
    },
};

mod log;

const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_PROCESS_DETACH: u32 = 0;
const DEFAULT_RELOAD_NAME_LEN: u8 = 0x1F;

static STARTED: AtomicBool = AtomicBool::new(false);
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct Config {
    enabled: bool,
    auto_detect_paths: bool,
    game_root: Option<PathBuf>,
    source_dir: PathBuf,
    target_archive: PathBuf,
    character_reload_name: String,
    reload_names: Vec<String>,
    also_reload_base_character: bool,
    hand: HandMode,
    detect_field: DetectField,
    active_detector: String,
    detectors: Vec<DetectorConfig>,
    startup_delay_seconds: f32,
    poll_every_frames: u32,
    reload_delay_frames: u32,
    stable_frames_required: u32,
    copy_before_delay: bool,
    copy_enabled: bool,
    hot_reload_enabled: bool,
    reload_list_offset: usize,
    reload_count_offset: usize,
    reload_timer_offset: usize,
    reload_timer_seconds: f32,
    reload_name_len: u8,
    crash_patch_enabled: bool,
    crash_patch_aob: String,
    crash_patch_dist_from_end: usize,
    crash_patch_write_bytes: Vec<u8>,
    mappings: Vec<Mapping>,
}

#[derive(Clone, Debug, Deserialize)]
struct Mapping {
    detector: Option<String>,
    id: i32,
    file: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DetectionKey {
    detector: String,
    id: i32,
}

struct MatchedMapping<'a> {
    key: DetectionKey,
    file: &'a Path,
}

#[derive(Clone, Debug)]
struct DetectorSpec {
    name: String,
    detect_field: DetectField,
    hand: HandMode,
}

#[derive(Clone, Debug, Deserialize)]
struct DetectorConfig {
    name: String,
    detect_field: DetectField,
    hand: Option<HandMode>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum HandMode {
    Auto,
    Right,
    Left,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DetectField {
    ActiveWepmotionId,
    WepmotionOneHandId,
    WepmotionBothHandId,
    WepmotionCategory,
    GuardmotionCategory,
    WeaponCategory,
    WeaponParamId,
    BaseWeaponParamId,
    WepType,
    SpAttribute,
    SpAtkcategory,
    ResidentSpEffectId,
    ResidentSpEffectId1,
    ResidentSpEffectId2,
    SwordArtsParamId,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_detect_paths: true,
            game_root: None,
            source_dir: PathBuf::from("chr/ExtraAnimation"),
            target_archive: PathBuf::from("mod/chr/c0000_dlc01.anibnd.dcx"),
            character_reload_name: "c0000_dlc01".to_string(),
            reload_names: Vec::new(),
            also_reload_base_character: true,
            hand: HandMode::Auto,
            detect_field: DetectField::ActiveWepmotionId,
            active_detector: String::new(),
            detectors: Vec::new(),
            startup_delay_seconds: 3.0,
            poll_every_frames: 6,
            reload_delay_frames: 45,
            stable_frames_required: 8,
            copy_before_delay: false,
            copy_enabled: true,
            hot_reload_enabled: true,
            reload_list_offset: 0x1E668,
            reload_count_offset: 0x1E670,
            reload_timer_offset: 0x1E678,
            reload_timer_seconds: 10.0,
            reload_name_len: DEFAULT_RELOAD_NAME_LEN,
            crash_patch_enabled: true,
            crash_patch_aob: "80 65 ?? FD 48 C7 45 ?? 07 00 00 00 ?? 8D 45 48 4C 89 60 ?? 48 83 78 ?? 08 72 03 48 8B 00 66 44 89 20 49 8B 8F ?? ?? ?? ?? 48 8B 01 48 ?? ??".to_string(),
            crash_patch_dist_from_end: 3,
            crash_patch_write_bytes: vec![0x48, 0x31, 0xD2],
            mappings: vec![
                Mapping {
                    detector: None,
                    id: 0,
                    file: PathBuf::from("c0000_dlc01.anibnd_00.dcx"),
                },
                Mapping {
                    detector: None,
                    id: 1,
                    file: PathBuf::from("c0000_dlc01.anibnd_01.dcx"),
                },
            ],
        }
    }
}

#[derive(Default)]
struct RuntimeState {
    frame: u32,
    last_detected_key: Option<DetectionKey>,
    last_applied_key: Option<DetectionKey>,
    last_unmapped_key: Option<DetectionKey>,
    pending_key: Option<DetectionKey>,
    pending_elapsed_frames: u32,
    pending_stable_frames: u32,
    pending_copied_key: Option<DetectionKey>,
    crash_patch_attempted: bool,
}

#[derive(Clone, Copy)]
enum ActiveHand {
    Left,
    Right,
}

/// # Safety
/// Exposed for Windows LoadLibrary. Do not call directly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DllMain(hmodule: HINSTANCE, reason: u32, _: *mut c_void) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            SHUTDOWN.store(false, Ordering::Release);
            if STARTED.swap(true, Ordering::AcqRel) {
                return 1;
            }

            let module = hmodule.0 as usize;
            std::thread::spawn(move || run_task_thread(module));
            1
        }
        DLL_PROCESS_DETACH => {
            SHUTDOWN.store(true, Ordering::Release);
            log::line(format_args!("dll detach requested"));
            1
        }
        _ => 1,
    }
}

fn run_task_thread(module: usize) {
    log::initialize(module);

    let config = Config::load(module);
    log::line(format_args!(
        "config enabled={} mappings={} detect_field={:?} hand={:?} active_detector={} startup_delay_seconds={} reload_delay_frames={} stable_frames_required={} copy_before_delay={}",
        config.enabled,
        config.mappings.len(),
        config.active_detect_field(),
        config.active_hand_mode(),
        config.active_detector_name(),
        config.startup_delay_seconds,
        config.reload_delay_frames,
        config.stable_frames_required,
        config.copy_before_delay
    ));

    if !config.enabled {
        log::line(format_args!("disabled by config"));
        STARTED.store(false, Ordering::Release);
        return;
    }

    wait_startup_delay(&config);
    if SHUTDOWN.load(Ordering::Acquire) {
        STARTED.store(false, Ordering::Release);
        return;
    }

    let Ok(cs_task) = CSTaskImp::wait_for_instance(Duration::MAX) else {
        log::line(format_args!("failed to find CSTaskImp"));
        STARTED.store(false, Ordering::Release);
        return;
    };

    let mapping_files = config
        .mappings
        .iter()
        .map(|entry| entry.file.clone())
        .collect::<Vec<_>>();
    let paths = config.resolve_runtime_paths(module, &mapping_files);
    let mut state = RuntimeState::default();

    log::line(format_args!("game_root={}", paths.game_root.display()));
    log::line(format_args!("source_dir={}", paths.source_dir.display()));
    log::line(format_args!(
        "target_archive={}",
        paths.target_archive.display()
    ));

    cs_task.run_recurring(
        move |_: &FD4TaskData| {
            if SHUTDOWN.load(Ordering::Acquire) {
                return;
            }

            state.frame = state.frame.wrapping_add(1);
            if config.poll_every_frames > 1 && !state.frame.is_multiple_of(config.poll_every_frames)
            {
                return;
            }

            tick(
                &config,
                &paths.source_dir,
                &paths.target_archive,
                &mut state,
            );
        },
        CSTaskGroupIndex::FrameBegin,
    );
}

fn tick(config: &Config, source_dir: &Path, target_archive: &Path, state: &mut RuntimeState) {
    let Some(matched) = find_matched_mapping(config) else {
        clear_pending(state);
        return;
    };

    if state.last_detected_key.as_ref() != Some(&matched.key) {
        log::line(format_args!(
            "detected mapping changed to detector='{}' id={}",
            matched.key.detector, matched.key.id
        ));
        state.last_detected_key = Some(matched.key.clone());
    }

    if state.last_applied_key.as_ref() == Some(&matched.key) {
        clear_pending(state);
        return;
    }
    state.last_unmapped_key = None;

    apply_when_stable(
        config,
        source_dir,
        target_archive,
        state,
        matched.key,
        matched.file,
    );
}

fn wait_startup_delay(config: &Config) {
    let delay = config.startup_delay_duration();
    if delay.is_zero() {
        return;
    }

    log::line(format_args!(
        "startup delay waiting {:.3} seconds",
        delay.as_secs_f32()
    ));

    let step = Duration::from_millis(100);
    let mut waited = Duration::ZERO;
    while waited < delay {
        if SHUTDOWN.load(Ordering::Acquire) {
            return;
        }

        let remaining = delay.saturating_sub(waited);
        let sleep_for = remaining.min(step);
        std::thread::sleep(sleep_for);
        waited += sleep_for;
    }
}

fn find_matched_mapping(config: &Config) -> Option<MatchedMapping<'_>> {
    let mut detection_cache = HashMap::<String, Option<i32>>::new();

    for mapping in &config.mappings {
        let Some(detector) = config.detector_for_mapping(mapping) else {
            continue;
        };

        let detected_id = if let Some(cached) = detection_cache.get(&detector.name) {
            *cached
        } else {
            let detected = detect_weapon_id_with_detector(&detector);
            detection_cache.insert(detector.name.clone(), detected);
            detected
        };

        if detected_id == Some(mapping.id) {
            return Some(MatchedMapping {
                key: DetectionKey {
                    detector: detector.name,
                    id: mapping.id,
                },
                file: mapping.file.as_path(),
            });
        }
    }

    None
}

fn apply_when_stable(
    config: &Config,
    source_dir: &Path,
    target_archive: &Path,
    state: &mut RuntimeState,
    key: DetectionKey,
    source_file: &Path,
) {
    if state.pending_key.as_ref() != Some(&key) {
        state.pending_key = Some(key.clone());
        state.pending_elapsed_frames = 0;
        state.pending_stable_frames = 0;
        state.pending_copied_key = None;
        log::line(format_args!(
            "pending mapped detector='{}' id={}; waiting delay_frames={} stable_frames={}",
            key.detector, key.id, config.reload_delay_frames, config.stable_frames_required
        ));
    }

    let source_path = resolve_path(source_dir, source_file);
    if config.copy_before_delay && state.pending_copied_key.as_ref() != Some(&key) {
        if copy_archive_for_id(config, &key, &source_path, target_archive).is_err() {
            return;
        }
        state.pending_copied_key = Some(key.clone());
    }

    let frame_step = config.poll_stride_frames();
    state.pending_elapsed_frames = state.pending_elapsed_frames.saturating_add(frame_step);
    state.pending_stable_frames = state.pending_stable_frames.saturating_add(frame_step);

    if state.pending_elapsed_frames < config.reload_delay_frames
        || state.pending_stable_frames < config.stable_frames_required
    {
        return;
    }

    if !config.copy_before_delay {
        if copy_archive_for_id(config, &key, &source_path, target_archive).is_err() {
            return;
        }
    } else if !config.copy_enabled {
        state.pending_copied_key = Some(key.clone());
    }

    state.last_applied_key = Some(key);
    clear_pending(state);

    if config.hot_reload_enabled {
        request_hot_reload(config, state);
    }
}

fn copy_archive_for_id(
    config: &Config,
    key: &DetectionKey,
    source_path: &Path,
    target_archive: &Path,
) -> Result<(), ()> {
    if config.copy_enabled {
        match replace_archive(source_path, target_archive) {
            Ok(()) => {
                log::line(format_args!(
                    "copied {} -> {} target_size={}",
                    source_path.display(),
                    target_archive.display(),
                    file_size_text(target_archive)
                ));
            }
            Err(err) => {
                log::line(format_args!(
                    "copy failed detector='{}' id={} source={} target={} err={err}",
                    key.detector,
                    key.id,
                    source_path.display(),
                    target_archive.display()
                ));
                return Err(());
            }
        }
    }

    Ok(())
}

fn clear_pending(state: &mut RuntimeState) {
    state.pending_key = None;
    state.pending_elapsed_frames = 0;
    state.pending_stable_frames = 0;
    state.pending_copied_key = None;
}

fn detect_weapon_id_with_detector(detector: &DetectorSpec) -> Option<i32> {
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    let player = world_chr_man.main_player.as_deref()?;
    let chr_asm = player.chr_asm.as_ref();
    let field = detector.detect_field;
    let hand = active_hand(detector.hand, chr_asm);
    let slot_index = active_weapon_slot_index(chr_asm, hand)?;
    let param_id = chr_asm.equipment_param_ids.get(slot_index).copied()?;
    if param_id <= 0 {
        return None;
    }

    if matches!(field, DetectField::WeaponParamId) {
        return Some(param_id);
    }
    if matches!(field, DetectField::BaseWeaponParamId) {
        return Some((param_id / 100) * 100);
    }

    let repo = unsafe { SoloParamRepository::instance() }.ok()?;
    let weapon = repo.get::<EquipParamWeapon>(((param_id as u32) / 100) * 100)?;

    Some(read_weapon_field(field, weapon, chr_asm, hand))
}

fn active_hand(hand_mode: HandMode, chr_asm: &ChrAsm) -> ActiveHand {
    match hand_mode {
        HandMode::Left => ActiveHand::Left,
        HandMode::Right => ActiveHand::Right,
        HandMode::Auto => match chr_asm.equipment.arm_style {
            ChrAsmArmStyle::LeftBothHands => ActiveHand::Left,
            _ => ActiveHand::Right,
        },
    }
}

fn active_weapon_slot_index(chr_asm: &ChrAsm, hand: ActiveHand) -> Option<usize> {
    let slots = &chr_asm.equipment.selected_slots;
    let slot = match hand {
        ActiveHand::Left => slots.left_weapon_slot,
        ActiveHand::Right => slots.right_weapon_slot,
    };

    if slot > 2 {
        return None;
    }

    Some(match hand {
        ActiveHand::Left => (slot * 2) as usize,
        ActiveHand::Right => (slot * 2 + 1) as usize,
    })
}

fn read_weapon_field(
    field: DetectField,
    weapon: &eldenring::param::EQUIP_PARAM_WEAPON_ST,
    chr_asm: &ChrAsm,
    hand: ActiveHand,
) -> i32 {
    match field {
        DetectField::ActiveWepmotionId => {
            let both_handing = matches!(
                (hand, chr_asm.equipment.arm_style),
                (ActiveHand::Left, ChrAsmArmStyle::LeftBothHands)
                    | (ActiveHand::Right, ChrAsmArmStyle::RightBothHands)
            );
            if both_handing {
                weapon.wepmotion_both_hand_id() as i32
            } else {
                weapon.wepmotion_one_hand_id() as i32
            }
        }
        DetectField::WepmotionOneHandId => weapon.wepmotion_one_hand_id() as i32,
        DetectField::WepmotionBothHandId => weapon.wepmotion_both_hand_id() as i32,
        DetectField::WepmotionCategory => weapon.wepmotion_category() as i32,
        DetectField::GuardmotionCategory => weapon.guardmotion_category() as i32,
        DetectField::WeaponCategory => weapon.weapon_category() as i32,
        DetectField::WepType => weapon.wep_type() as i32,
        DetectField::SpAttribute => weapon.sp_attribute() as i32,
        DetectField::SpAtkcategory => weapon.sp_atkcategory() as i32,
        DetectField::ResidentSpEffectId => weapon.resident_sp_effect_id(),
        DetectField::ResidentSpEffectId1 => weapon.resident_sp_effect_id1(),
        DetectField::ResidentSpEffectId2 => weapon.resident_sp_effect_id2(),
        DetectField::SwordArtsParamId => weapon.sword_arts_param_id(),
        DetectField::WeaponParamId | DetectField::BaseWeaponParamId => unreachable!(),
    }
}

fn file_size_text(path: &Path) -> String {
    fs::metadata(path)
        .map(|metadata| metadata.len().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn replace_archive(source_path: &Path, target_archive: &Path) -> std::io::Result<()> {
    if let Some(parent) = target_archive.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = target_archive.with_extension("weapon_animation_hotreload.tmp");
    let _ = fs::remove_file(&tmp_path);
    fs::copy(source_path, &tmp_path)?;

    match fs::rename(&tmp_path, target_archive) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            let _ = fs::remove_file(target_archive);
            match fs::rename(&tmp_path, target_archive) {
                Ok(()) => Ok(()),
                Err(_) => {
                    let _ = fs::remove_file(&tmp_path);
                    fs::copy(source_path, target_archive)
                        .map(|_| ())
                        .map_err(|_| rename_err)
                }
            }
        }
    }
}

fn request_hot_reload(config: &Config, state: &mut RuntimeState) {
    if config.crash_patch_enabled && !state.crash_patch_attempted {
        state.crash_patch_attempted = true;
        match apply_crash_patch(config) {
            Ok(()) => log::line(format_args!("crash patch applied or already compatible")),
            Err(err) => log::line(format_args!("crash patch failed: {err}")),
        }
    }

    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        log::line(format_args!("hot reload failed: WorldChrMan unavailable"));
        return;
    };

    let world_addr = world_chr_man as *mut WorldChrMan as usize;
    for name in config.effective_reload_names() {
        unsafe {
            match queue_chr_reload(
                world_addr,
                config.reload_list_offset,
                config.reload_count_offset,
                config.reload_timer_offset,
                config.reload_timer_seconds,
                config.reload_name_len,
                &name,
            ) {
                Ok(()) => {}
                Err(err) => {
                    log::line(format_args!("hot reload skipped for '{name}': {err}"));
                    return;
                }
            }
        }

        log::line(format_args!("requested character reload '{name}'"));
    }
}

unsafe fn queue_chr_reload(
    world_addr: usize,
    list_offset: usize,
    count_offset: usize,
    timer_offset: usize,
    timer_seconds: f32,
    reload_name_len: u8,
    name: &str,
) -> Result<(), String> {
    if !is_plausible_ptr(world_addr) {
        return Err(format!(
            "WorldChrMan pointer is implausible: 0x{world_addr:x}"
        ));
    }

    let head = unsafe { ((world_addr + list_offset) as *const usize).read_unaligned() };
    if head == 0 {
        return Err(format!(
            "reload list head is null at WorldChrMan+0x{list_offset:x}"
        ));
    }

    if !is_plausible_ptr(head) {
        return Err(format!(
            "reload list head is implausible at WorldChrMan+0x{list_offset:x}: 0x{head:x}"
        ));
    }

    let data_pointer = unsafe { (head as *const usize).read_unaligned() };
    if !is_plausible_ptr(data_pointer) {
        return Err(format!(
            "reload list data pointer is implausible: 0x{data_pointer:x}"
        ));
    }

    let mut node = vec![0u8; 0x140].into_boxed_slice();
    let node_ptr = node.as_mut_ptr() as usize;
    let string_ptr = node_ptr + 0x100;

    unsafe {
        (node_ptr as *mut usize).write_unaligned(head);
        ((node_ptr + 0x08) as *mut usize).write_unaligned(data_pointer);
        ((node_ptr + 0x58) as *mut usize).write_unaligned(string_ptr);
        ((node_ptr + 0x70) as *mut u8).write_unaligned(reload_name_len);
    }

    let utf16 = name.encode_utf16().chain(std::iter::once(0));
    for (index, unit) in utf16.take(0x20).enumerate() {
        unsafe {
            ((string_ptr + index * 2) as *mut u16).write_unaligned(unit);
        }
    }

    // Keep the node alive for the game reload queue. This mirrors DSAnimStudio's
    // effective behavior, where its remote allocation is not reclaimed.
    Box::leak(node);

    unsafe {
        (head as *mut usize).write_unaligned(node_ptr);
        (node_ptr as *mut usize).write_unaligned(head);
        ((head + 0x08) as *mut usize).write_unaligned(node_ptr);
        ((node_ptr + 0x08) as *mut usize).write_unaligned(head);
        ((world_addr + count_offset) as *mut u32).write_unaligned(1);
        ((world_addr + timer_offset) as *mut f32).write_unaligned(timer_seconds);
    }

    Ok(())
}

fn is_plausible_ptr(address: usize) -> bool {
    address >= 0x10000 && address < 0x0000_8000_0000_0000 && address.is_multiple_of(8)
}

fn apply_crash_patch(config: &Config) -> Result<(), String> {
    let pattern = parse_aob(&config.crash_patch_aob)?;
    let (base, size) = main_module_range().ok_or("failed to locate main module")?;
    let addr = scan_aob(base, size, &pattern).ok_or("crash patch AOB not found")?;
    if config.crash_patch_dist_from_end > pattern.len() {
        return Err("crash_patch_dist_from_end is larger than AOB length".to_string());
    }
    let patch_addr = addr + pattern.len() - config.crash_patch_dist_from_end;
    write_process_bytes(patch_addr, &config.crash_patch_write_bytes)
}

fn parse_aob(text: &str) -> Result<Vec<Option<u8>>, String> {
    text.split_whitespace()
        .map(|item| {
            if item == "?" || item == "??" {
                Ok(None)
            } else {
                u8::from_str_radix(item, 16)
                    .map(Some)
                    .map_err(|_| format!("invalid AOB byte '{item}'"))
            }
        })
        .collect()
}

fn main_module_range() -> Option<(usize, usize)> {
    let module = unsafe { GetModuleHandleW(None).ok()? };
    let base = module.0 as usize;
    if base == 0 {
        return None;
    }

    let e_lfanew = unsafe { ((base + 0x3C) as *const i32).read_unaligned() } as usize;
    let nt = base.checked_add(e_lfanew)?;
    let optional_header = nt.checked_add(0x18)?;
    let size_of_image = unsafe { ((optional_header + 0x38) as *const u32).read_unaligned() };
    Some((base, size_of_image as usize))
}

fn scan_aob(base: usize, size: usize, pattern: &[Option<u8>]) -> Option<usize> {
    if pattern.is_empty() || size < pattern.len() {
        return None;
    }

    let bytes = unsafe { std::slice::from_raw_parts(base as *const u8, size) };
    bytes
        .windows(pattern.len())
        .position(|window| {
            window
                .iter()
                .zip(pattern)
                .all(|(byte, expected)| expected.is_none_or(|expected| *byte == expected))
        })
        .map(|offset| base + offset)
}

fn write_process_bytes(address: usize, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Ok(());
    }

    unsafe {
        let mut old = PAGE_PROTECTION_FLAGS(0);
        VirtualProtect(
            address as *const c_void,
            bytes.len(),
            PAGE_EXECUTE_READWRITE,
            &mut old,
        )
        .map_err(|err| format!("VirtualProtect rw failed: {err}"))?;

        std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());

        let mut ignored = PAGE_PROTECTION_FLAGS(0);
        let _ = VirtualProtect(address as *const c_void, bytes.len(), old, &mut ignored);
    }

    Ok(())
}

fn resolve_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[derive(Clone, Debug)]
struct RuntimePaths {
    game_root: PathBuf,
    source_dir: PathBuf,
    target_archive: PathBuf,
}

impl Config {
    fn load(module: usize) -> Self {
        let mut config = Self::default();
        let mut loaded_path = None;

        for path in config_paths(module) {
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };

            match toml::from_str::<Self>(&text) {
                Ok(parsed) => {
                    loaded_path = Some(path);
                    config = parsed;
                    break;
                }
                Err(err) => {
                    log::line(format_args!(
                        "failed to parse config {}: {err}",
                        path.display()
                    ));
                }
            }
        }

        if let Some(path) = loaded_path {
            log::line(format_args!("loaded config {}", path.display()));
        } else {
            log::line(format_args!("no config found; using defaults"));
        }

        config.normalize_legacy_reload_offsets();
        config.log_detector_selection();
        config.log_mapping_detector_selection();
        config
    }

    fn normalize_legacy_reload_offsets(&mut self) {
        if self.reload_list_offset == 0x1E660
            && self.reload_count_offset == 0x1E668
            && self.reload_timer_offset == 0x1E670
        {
            self.reload_list_offset = 0x1E668;
            self.reload_count_offset = 0x1E670;
            self.reload_timer_offset = 0x1E678;
            log::line(format_args!(
                "migrated legacy hot reload offsets 0x1E660/0x1E668/0x1E670 to 0x1E668/0x1E670/0x1E678"
            ));
        }
    }

    fn poll_stride_frames(&self) -> u32 {
        self.poll_every_frames.max(1)
    }

    fn startup_delay_duration(&self) -> Duration {
        if self.startup_delay_seconds.is_finite() && self.startup_delay_seconds > 0.0 {
            Duration::from_secs_f32(self.startup_delay_seconds)
        } else {
            Duration::ZERO
        }
    }

    fn legacy_detector_spec(&self) -> DetectorSpec {
        DetectorSpec {
            name: "legacy".to_string(),
            detect_field: self.detect_field,
            hand: self.hand,
        }
    }

    fn active_detector_spec(&self) -> DetectorSpec {
        self.selected_detector()
            .map(|detector| DetectorSpec {
                name: detector.name.clone(),
                detect_field: detector.detect_field,
                hand: detector.hand.unwrap_or(self.hand),
            })
            .unwrap_or_else(|| self.legacy_detector_spec())
    }

    fn detector_spec_by_name(&self, name: &str) -> Option<DetectorSpec> {
        let trimmed = name.trim();
        if trimmed.eq_ignore_ascii_case("legacy") {
            return Some(self.legacy_detector_spec());
        }

        self.detectors
            .iter()
            .find(|detector| detector.name == trimmed)
            .map(|detector| DetectorSpec {
                name: detector.name.clone(),
                detect_field: detector.detect_field,
                hand: detector.hand.unwrap_or(self.hand),
            })
    }

    fn detector_for_mapping(&self, mapping: &Mapping) -> Option<DetectorSpec> {
        mapping
            .detector
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| self.detector_spec_by_name(name))
            .unwrap_or_else(|| Some(self.active_detector_spec()))
    }

    fn selected_detector(&self) -> Option<&DetectorConfig> {
        if self.detectors.is_empty() {
            return None;
        }

        let active = self.active_detector.trim();
        if !active.is_empty()
            && let Some(detector) = self.detectors.iter().find(|entry| entry.name == active)
        {
            return Some(detector);
        }

        self.detectors.first()
    }

    fn active_detector_name(&self) -> &str {
        self.selected_detector()
            .map(|detector| detector.name.as_str())
            .unwrap_or("legacy")
    }

    fn active_detect_field(&self) -> DetectField {
        self.active_detector_spec().detect_field
    }

    fn active_hand_mode(&self) -> HandMode {
        self.active_detector_spec().hand
    }

    fn log_detector_selection(&self) {
        if self.detectors.is_empty() {
            log::line(format_args!(
                "using legacy detector detect_field={:?} hand={:?}",
                self.detect_field, self.hand
            ));
            return;
        }

        if !self.active_detector.trim().is_empty()
            && self
                .detectors
                .iter()
                .all(|entry| entry.name != self.active_detector.trim())
        {
            log::line(format_args!(
                "active_detector '{}' not found; falling back to '{}'",
                self.active_detector,
                self.active_detector_name()
            ));
        }

        log::line(format_args!(
            "using detector '{}' detect_field={:?} hand={:?}",
            self.active_detector_name(),
            self.active_detect_field(),
            self.active_hand_mode()
        ));
    }

    fn log_mapping_detector_selection(&self) {
        for mapping in &self.mappings {
            let Some(name) = mapping.detector.as_deref().map(str::trim) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }

            if self.detector_spec_by_name(name).is_none() {
                log::line(format_args!(
                    "mapping id={} file={} references unknown detector '{}'; mapping will be ignored",
                    mapping.id,
                    mapping.file.display(),
                    name
                ));
            }
        }
    }

    fn effective_reload_names(&self) -> Vec<String> {
        let configured = if self.reload_names.is_empty() {
            std::slice::from_ref(&self.character_reload_name)
        } else {
            self.reload_names.as_slice()
        };

        let mut names = Vec::new();
        for name in configured {
            push_unique_reload_name(&mut names, name);

            if self.also_reload_base_character
                && let Some(base_name) = base_character_name(name)
            {
                push_unique_reload_name(&mut names, &base_name);
            }
        }

        names
    }

    fn game_root(&self) -> PathBuf {
        if let Some(path) = &self.game_root {
            if path.is_absolute() {
                return path.clone();
            }

            if let Ok(exe) = std::env::current_exe()
                && let Some(parent) = exe.parent()
            {
                return parent.join(path);
            }

            return path.clone();
        }

        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn resolve_runtime_paths(&self, module: usize, mapping_files: &[PathBuf]) -> RuntimePaths {
        let game_root = self.game_root();
        let configured_source = resolve_path(&game_root, &self.source_dir);
        let configured_target = resolve_path(&game_root, &self.target_archive);

        if !self.auto_detect_paths {
            return RuntimePaths {
                game_root,
                source_dir: configured_source,
                target_archive: configured_target,
            };
        }

        if source_dir_has_mapping(&configured_source, mapping_files)
            && target_path_is_plausible(&configured_target)
        {
            return RuntimePaths {
                game_root,
                source_dir: configured_source,
                target_archive: configured_target,
            };
        }

        for chr_dir in chr_dir_candidates(module, &game_root) {
            for source_dir in source_dir_candidates_from_chr_dir(&chr_dir, &self.source_dir) {
                if !source_dir_has_mapping(&source_dir, mapping_files) {
                    continue;
                }

                let target_archive = target_archive_from_chr_dir(&chr_dir, &self.target_archive);
                log::line(format_args!(
                    "auto path detection selected chr_dir={} source_dir={}",
                    chr_dir.display(),
                    source_dir.display()
                ));
                return RuntimePaths {
                    game_root,
                    source_dir,
                    target_archive,
                };
            }
        }

        log::line(format_args!(
            "auto path detection found no mapping source; using configured paths"
        ));
        RuntimePaths {
            game_root,
            source_dir: configured_source,
            target_archive: configured_target,
        }
    }
}

fn push_unique_reload_name(names: &mut Vec<String>, name: &str) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return;
    }

    if !names.iter().any(|existing| existing == trimmed) {
        names.push(trimmed.to_string());
    }
}

fn base_character_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    let (base, suffix) = trimmed.split_once('_')?;

    if base.len() == 5
        && base.starts_with('c')
        && base[1..].chars().all(|ch| ch.is_ascii_digit())
        && !suffix.is_empty()
    {
        Some(base.to_string())
    } else {
        None
    }
}

fn config_paths(module: usize) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(module_path) = log::module_path(module)
        && let Some(dir) = module_path.parent()
    {
        paths.push(dir.join("weapon_animation_hotreload.toml"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        paths.push(dir.join("weapon_animation_hotreload.toml"));
    }
    paths
}

fn source_dir_has_mapping(source_dir: &Path, mapping_files: &[PathBuf]) -> bool {
    mapping_files
        .iter()
        .any(|file| resolve_path(source_dir, file).is_file())
}

fn target_path_is_plausible(path: &Path) -> bool {
    path.is_file() || path.parent().is_some_and(Path::is_dir)
}

fn chr_dir_candidates(module: usize, game_root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(module_dir) =
        log::module_path(module).and_then(|path| path.parent().map(Path::to_path_buf))
    {
        push_chr_candidates_from_root(&mut candidates, &module_dir);
        for ancestor in module_dir.ancestors() {
            push_chr_candidates_from_root(&mut candidates, ancestor);
        }
    }

    push_chr_candidates_from_root(&mut candidates, game_root);
    push_unique(&mut candidates, game_root.join("mod").join("chr"));
    push_unique(&mut candidates, game_root.join("chr"));

    candidates
}

fn push_chr_candidates_from_root(candidates: &mut Vec<PathBuf>, root: &Path) {
    push_unique(candidates, root.join("chr"));

    for child in immediate_child_dirs(root).into_iter().take(256) {
        push_unique(candidates, child.join("chr"));
        push_unique(candidates, child.join("mod").join("chr"));
    }

    for name in ["mod", "mods", "Mod", "Mods"] {
        push_unique(candidates, root.join(name).join("chr"));
    }
}

fn immediate_child_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let Ok(file_type) = entry.file_type() else {
                return None;
            };
            file_type.is_dir().then(|| entry.path())
        })
        .collect()
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| same_path_text(existing, &path)) {
        paths.push(path);
    }
}

fn same_path_text(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn source_dir_candidates_from_chr_dir(chr_dir: &Path, configured_source: &Path) -> Vec<PathBuf> {
    if configured_source.is_absolute() {
        return vec![configured_source.to_path_buf()];
    }

    let mut candidates = Vec::new();

    if let Ok(rest) = configured_source.strip_prefix("chr") {
        push_unique(&mut candidates, chr_dir.join(rest));
    } else if let Ok(rest) = configured_source.strip_prefix(Path::new("mod").join("chr")) {
        push_unique(&mut candidates, chr_dir.join(rest));
    } else if configured_source != Path::new("auto") {
        push_unique(&mut candidates, chr_dir.join(configured_source));
    }

    push_unique(&mut candidates, chr_dir.join("ExtraAnimation"));
    push_unique(&mut candidates, chr_dir.to_path_buf());

    candidates
}

fn target_archive_from_chr_dir(chr_dir: &Path, configured_target: &Path) -> PathBuf {
    if configured_target.is_absolute() {
        return configured_target.to_path_buf();
    }

    if let Ok(rest) = configured_target.strip_prefix("chr") {
        return chr_dir.join(rest);
    }

    if let Ok(rest) = configured_target.strip_prefix(Path::new("mod").join("chr")) {
        return chr_dir.join(rest);
    }

    if configured_target
        .parent()
        .is_none_or(|parent| parent.as_os_str().is_empty())
    {
        return chr_dir.join(configured_target);
    }

    chr_dir.join(
        configured_target
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| configured_target.to_path_buf()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aob_wildcards() {
        let parsed = parse_aob("48 8B ?? 00 ? FF").unwrap();
        assert_eq!(
            parsed,
            vec![Some(0x48), Some(0x8B), None, Some(0x00), None, Some(0xFF)]
        );
    }

    #[test]
    fn resolves_relative_paths() {
        assert_eq!(
            resolve_path(Path::new("C:/Game"), Path::new("mod/chr/test.dcx")),
            PathBuf::from("C:/Game/mod/chr/test.dcx")
        );
    }

    #[test]
    fn parses_sp_atkcategory_detect_field() {
        let config = toml::from_str::<Config>(
            r#"
            detect_field = "sp_atkcategory"
            [[mappings]]
            id = 7
            file = "c0000_dlc01.anibnd_07.dcx"
            "#,
        )
        .unwrap();

        assert_eq!(config.detect_field, DetectField::SpAtkcategory);
    }

    #[test]
    fn parses_startup_delay_seconds() {
        let config = toml::from_str::<Config>(
            r#"
            startup_delay_seconds = 2.5
            [[mappings]]
            id = 7
            file = "c0000_dlc01.anibnd_07.dcx"
            "#,
        )
        .unwrap();

        assert_eq!(config.startup_delay_duration(), Duration::from_millis(2500));
    }

    #[test]
    fn expands_supplemental_reload_name_to_base_character() {
        let config = Config {
            character_reload_name: "c0000_dlc01".to_string(),
            ..Config::default()
        };

        assert_eq!(
            config.effective_reload_names(),
            vec!["c0000_dlc01".to_string(), "c0000".to_string()]
        );
    }

    #[test]
    fn explicit_reload_names_are_used_and_deduplicated() {
        let config = Config {
            reload_names: vec![
                "c0000_dlc01".to_string(),
                "c0000".to_string(),
                "c0000_dlc01".to_string(),
            ],
            ..Config::default()
        };

        assert_eq!(
            config.effective_reload_names(),
            vec!["c0000_dlc01".to_string(), "c0000".to_string()]
        );
    }

    #[test]
    fn legacy_detector_is_used_without_profiles() {
        let config = Config {
            detect_field: DetectField::SpAtkcategory,
            hand: HandMode::Left,
            ..Config::default()
        };

        assert_eq!(config.active_detector_name(), "legacy");
        assert_eq!(config.active_detect_field(), DetectField::SpAtkcategory);
        assert_eq!(config.active_hand_mode(), HandMode::Left);
    }

    #[test]
    fn named_detector_can_be_selected() {
        let config = toml::from_str::<Config>(
            r#"
            active_detector = "weapon_param"
            hand = "left"

            [[detectors]]
            name = "special_motion"
            detect_field = "sp_atkcategory"
            hand = "auto"

            [[detectors]]
            name = "weapon_param"
            detect_field = "base_weapon_param_id"
            "#,
        )
        .unwrap();

        assert_eq!(config.active_detector_name(), "weapon_param");
        assert_eq!(config.active_detect_field(), DetectField::BaseWeaponParamId);
        assert_eq!(config.active_hand_mode(), HandMode::Left);
    }

    #[test]
    fn missing_named_detector_falls_back_to_first_profile() {
        let config = toml::from_str::<Config>(
            r#"
            active_detector = "missing"

            [[detectors]]
            name = "special_motion"
            detect_field = "sp_atkcategory"
            hand = "auto"

            [[detectors]]
            name = "weapon_param"
            detect_field = "base_weapon_param_id"
            hand = "right"
            "#,
        )
        .unwrap();

        assert_eq!(config.active_detector_name(), "special_motion");
        assert_eq!(config.active_detect_field(), DetectField::SpAtkcategory);
        assert_eq!(config.active_hand_mode(), HandMode::Auto);
    }

    #[test]
    fn mapping_can_select_detector() {
        let config = toml::from_str::<Config>(
            r#"
            active_detector = "special_motion"

            [[detectors]]
            name = "special_motion"
            detect_field = "sp_atkcategory"
            hand = "auto"

            [[detectors]]
            name = "weapon_param"
            detect_field = "base_weapon_param_id"
            hand = "right"

            [[mappings]]
            detector = "weapon_param"
            id = 1000000
            file = "c0000_dlc01.anibnd_weapon.dcx"
            "#,
        )
        .unwrap();

        let detector = config.detector_for_mapping(&config.mappings[0]).unwrap();
        assert_eq!(detector.name, "weapon_param");
        assert_eq!(detector.detect_field, DetectField::BaseWeaponParamId);
        assert_eq!(detector.hand, HandMode::Right);
    }

    #[test]
    fn mapping_without_detector_uses_active_detector() {
        let config = toml::from_str::<Config>(
            r#"
            active_detector = "special_motion"

            [[detectors]]
            name = "special_motion"
            detect_field = "sp_atkcategory"
            hand = "auto"

            [[mappings]]
            id = 140
            file = "c0000_dlc01.anibnd_00.dcx"
            "#,
        )
        .unwrap();

        let detector = config.detector_for_mapping(&config.mappings[0]).unwrap();
        assert_eq!(detector.name, "special_motion");
        assert_eq!(detector.detect_field, DetectField::SpAtkcategory);
    }
}
