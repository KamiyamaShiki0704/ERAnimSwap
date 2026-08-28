use std::{collections::HashMap, ffi::c_void, mem::size_of};

use eldenring::cs::{PlayerIns, WorldChrMan};
use fromsoftware_shared::FromStatic;
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS, VirtualQuery,
};

use crate::{Config, DetectionKey, find_matched_mapping_for_player, log};

const CHR_INS_CHR_RES_OFFSET: usize = 0x28;
const CHR_CTRL_ANIMATION_CTRL_OFFSET: usize = 0x20;
const CHR_CTRL_HKXPWV_RES_CAP_OFFSET: usize = 0xC0;
const BEHAVIOR_MODULE_SLOT_OFFSET: usize = 0x10;
const BEHAVIOR_SLOT_HKB_CHARACTER_OFFSET: usize = 0x30;
const HKB_CHARACTER_GRAPH_OFFSET: usize = 0x98;
const BEHAVIOR_GRAPH_ROOT_OFFSET: usize = 0xC8;

#[derive(Default)]
pub(crate) struct ProbeState {
    sampled_once: bool,
    last_sample_frame: u32,
    players: HashMap<String, PlayerProbeSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PlayerProbeSnapshot {
    identity: String,
    field_handle: String,
    p2p_handle: String,
    chr_type: String,
    local: bool,
    mapping: Option<DetectionKey>,
    mapping_file: Option<String>,
    chr_ins: usize,
    chr_res: usize,
    chr_ctrl: usize,
    animation_ctrl: usize,
    hkxpwv_res_cap: usize,
    behavior_module: usize,
    behavior_slot: usize,
    hkb_character: usize,
    behavior_graph: usize,
    behavior_root: usize,
}

pub(crate) fn tick(config: &Config, frame: u32, state: &mut ProbeState) {
    if !config.per_player_probe_enabled {
        return;
    }

    let interval = config.per_player_probe_every_frames.max(1);
    if state.sampled_once && frame.wrapping_sub(state.last_sample_frame) < interval {
        return;
    }
    state.sampled_once = true;
    state.last_sample_frame = frame;

    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        if !state.players.is_empty() {
            log::line(format_args!(
                "per-player probe WorldChrMan unavailable; retaining previous snapshot"
            ));
        }
        return;
    };

    let local_chr_ins = world_chr_man
        .main_player
        .as_deref()
        .map(|player| player as *const PlayerIns as usize);
    let mut current = HashMap::new();

    for player in world_chr_man.player_chr_set.characters() {
        let snapshot = PlayerProbeSnapshot::collect(config, player, local_chr_ins);
        let identity = snapshot.identity.clone();
        if state.players.get(&identity) != Some(&snapshot) {
            let change = if state.players.contains_key(&identity) {
                "changed"
            } else {
                "added"
            };
            log_snapshot(change, &snapshot);
        }
        current.insert(identity, snapshot);
    }

    for (identity, previous) in &state.players {
        if !current.contains_key(identity) {
            log::line(format_args!(
                "per-player probe removed identity='{}' chr_ins=0x{:X} mapping={}",
                identity,
                previous.chr_ins,
                mapping_text(previous)
            ));
        }
    }

    if current != state.players {
        log_summary(&current);
    }
    state.players = current;
}

impl PlayerProbeSnapshot {
    fn collect(config: &Config, player: &PlayerIns, local_chr_ins: Option<usize>) -> Self {
        let chr_ins = player as *const PlayerIns as usize;
        let chr_ctrl = player.chr_ins.chr_ctrl.as_ref() as *const _ as usize;
        let behavior_module = player.chr_ins.modules.behavior.as_ref() as *const _ as usize;
        let field_handle = player.chr_ins.field_ins_handle.to_string();
        let p2p_handle = player.chr_ins.p2p_entity_handle.to_string();
        let identity = format!("{field_handle}|{p2p_handle}");
        let matched = find_matched_mapping_for_player(config, player);

        let behavior_slot = read_pointer(behavior_module, BEHAVIOR_MODULE_SLOT_OFFSET);
        let hkb_character = read_pointer(behavior_slot, BEHAVIOR_SLOT_HKB_CHARACTER_OFFSET);
        let behavior_graph = read_pointer(hkb_character, HKB_CHARACTER_GRAPH_OFFSET);

        Self {
            identity,
            field_handle,
            p2p_handle,
            chr_type: format!("{:?}", player.chr_ins.chr_type),
            local: local_chr_ins == Some(chr_ins),
            mapping: matched.as_ref().map(|mapping| mapping.key.clone()),
            mapping_file: matched.map(|mapping| mapping.file.display().to_string()),
            chr_ins,
            chr_res: read_pointer(chr_ins, CHR_INS_CHR_RES_OFFSET),
            chr_ctrl,
            animation_ctrl: read_pointer(chr_ctrl, CHR_CTRL_ANIMATION_CTRL_OFFSET),
            hkxpwv_res_cap: read_pointer(chr_ctrl, CHR_CTRL_HKXPWV_RES_CAP_OFFSET),
            behavior_module,
            behavior_slot,
            hkb_character,
            behavior_graph,
            behavior_root: read_pointer(behavior_graph, BEHAVIOR_GRAPH_ROOT_OFFSET),
        }
    }
}

fn log_snapshot(change: &str, snapshot: &PlayerProbeSnapshot) {
    log::line(format_args!(
        "per-player probe {change} identity='{}' local={} chr_type={} field='{}' p2p='{}' mapping={} chr_ins=0x{:X} chr_res=0x{:X} chr_ctrl=0x{:X} animation_ctrl=0x{:X} hkxpwv_res_cap=0x{:X} behavior_module=0x{:X} behavior_slot=0x{:X} hkb_character=0x{:X} behavior_graph=0x{:X} behavior_root=0x{:X}",
        snapshot.identity,
        snapshot.local,
        snapshot.chr_type,
        snapshot.field_handle,
        snapshot.p2p_handle,
        mapping_text(snapshot),
        snapshot.chr_ins,
        snapshot.chr_res,
        snapshot.chr_ctrl,
        snapshot.animation_ctrl,
        snapshot.hkxpwv_res_cap,
        snapshot.behavior_module,
        snapshot.behavior_slot,
        snapshot.hkb_character,
        snapshot.behavior_graph,
        snapshot.behavior_root,
    ));
}

fn mapping_text(snapshot: &PlayerProbeSnapshot) -> String {
    match (&snapshot.mapping, &snapshot.mapping_file) {
        (Some(key), Some(file)) => format!("{}:{}:{}", key.detector, key.id, file),
        _ => "none".to_string(),
    }
}

fn log_summary(players: &HashMap<String, PlayerProbeSnapshot>) {
    log::line(format_args!(
        "per-player probe summary players={} distinct_chr_res={} distinct_animation_ctrl={} distinct_hkxpwv_res_cap={} distinct_behavior_slot={} distinct_hkb_character={} distinct_behavior_graph={}",
        players.len(),
        distinct_nonzero(players, |player| player.chr_res),
        distinct_nonzero(players, |player| player.animation_ctrl),
        distinct_nonzero(players, |player| player.hkxpwv_res_cap),
        distinct_nonzero(players, |player| player.behavior_slot),
        distinct_nonzero(players, |player| player.hkb_character),
        distinct_nonzero(players, |player| player.behavior_graph),
    ));
}

fn distinct_nonzero(
    players: &HashMap<String, PlayerProbeSnapshot>,
    select: impl Fn(&PlayerProbeSnapshot) -> usize,
) -> usize {
    let mut values = players
        .values()
        .map(select)
        .filter(|address| *address != 0)
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values.len()
}

fn read_pointer(base: usize, offset: usize) -> usize {
    let Some(address) = base.checked_add(offset) else {
        return 0;
    };
    if !is_readable_range(address, size_of::<usize>()) {
        return 0;
    }

    unsafe { (address as *const usize).read_unaligned() }
}

fn is_readable_range(address: usize, length: usize) -> bool {
    if address < 0x10000 || length == 0 {
        return false;
    }

    let mut memory = MEMORY_BASIC_INFORMATION::default();
    let queried = unsafe {
        VirtualQuery(
            Some(address as *const c_void),
            &mut memory,
            size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if queried != size_of::<MEMORY_BASIC_INFORMATION>()
        || memory.State != MEM_COMMIT
        || memory.Protect.0 & (PAGE_NOACCESS.0 | PAGE_GUARD.0) != 0
    {
        return false;
    }

    let region_start = memory.BaseAddress as usize;
    let Some(region_end) = region_start.checked_add(memory.RegionSize) else {
        return false;
    };
    address
        .checked_add(length)
        .is_some_and(|end| address >= region_start && end <= region_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_pointer_chain_returns_zero() {
        assert_eq!(read_pointer(0, 0x28), 0);
        assert_eq!(read_pointer(usize::MAX, 1), 0);
    }

    #[test]
    fn distinct_pointer_count_ignores_zero_and_duplicates() {
        let template = PlayerProbeSnapshot {
            identity: String::new(),
            field_handle: String::new(),
            p2p_handle: String::new(),
            chr_type: String::new(),
            local: false,
            mapping: None,
            mapping_file: None,
            chr_ins: 0,
            chr_res: 0,
            chr_ctrl: 0,
            animation_ctrl: 0,
            hkxpwv_res_cap: 0,
            behavior_module: 0,
            behavior_slot: 0,
            hkb_character: 0,
            behavior_graph: 0,
            behavior_root: 0,
        };
        let mut players = HashMap::new();
        players.insert(
            "a".to_string(),
            PlayerProbeSnapshot {
                chr_res: 0x1000,
                ..template.clone()
            },
        );
        players.insert(
            "b".to_string(),
            PlayerProbeSnapshot {
                chr_res: 0x1000,
                ..template.clone()
            },
        );
        players.insert("c".to_string(), template);

        assert_eq!(distinct_nonzero(&players, |player| player.chr_res), 1);
    }
}
