"""Read one local player's animation mapping and Havok binding, without writes.

Audited ER 2.7.1.0 layout. This is metadata evidence, not a playback verdict.
"""
import argparse
import datetime
import importlib.util
import json
from pathlib import Path
import struct


def signed_short(index):
    return struct.unpack("<h", struct.pack("<H", index & 0xffff))[0]


def inspect(args):
    spec = importlib.util.spec_from_file_location(
        "reader", Path(__file__).with_name("Read-TaeRepository.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    r = module.Reader(args.pid, args.base)
    try:
        def typed(address, name):
            if r.rtti(address) != name:
                raise ValueError(f"Unexpected object at {address:#x}; expected {name}")
            return address

        world = typed(r.qword(args.base + 0x3d69ff8), ".?AVWorldChrManImp@CS@@")
        player = typed(r.qword(world + 0x1e508), ".?AVPlayerIns@CS@@")
        modules = r.qword(player + 0x190)
        time_act = typed(r.qword(modules + 0x18), ".?AVCSChrTimeActModule@CS@@")
        if r.qword(time_act + 8) != player:
            raise ValueError("TimeAct owner mismatch")
        havok = typed(r.qword(time_act + 0x10), ".?AVHvkAnim@CS@@")
        layout = r.read(havok + 0x98, 24)
        count = struct.unpack_from("<I", layout)[0]
        table, tae_dat = struct.unpack_from("<QQ", layout, 8)
        typed(tae_dat, ".?AVTaeDat@CS@@")
        if not 0 < count <= 100000:
            raise ValueError("Animation mapping count outside read budget")
        total = count * 72
        data = b"".join(r.read(table + i, min(0x80000, total-i))
                        for i in range(0, total, 0x80000))
        ids = [struct.unpack_from("<I", data, i*72)[0] for i in range(count)]
        if ids != sorted(ids):
            raise ValueError("Animation map is not ordered")
        matches = [i for i, value in enumerate(ids) if value == args.animation]
        if len(matches) != 1:
            raise ValueError(f"Expected one mapping, found {len(matches)}")
        index = matches[0]
        entry = data[index*72:(index+1)*72]
        binding, body = struct.unpack_from("<QQ", entry, 8)
        typed(binding, ".?AVhkaAnimationBinding@@")
        animation = r.qword(binding + 0x20)
        animation_type = r.rtti(animation)
        anim_header = r.read(animation, 0x28)
        duration = struct.unpack_from("<f", anim_header, 0x1c)[0]
        tracks = struct.unpack_from("<i", anim_header, 0x20)[0]

        tae_id, action_id = divmod(args.animation, 1000000)
        if not 0 <= tae_id < 999:
            raise ValueError("TAE file ID outside audited loader range")
        tae = r.qword(tae_dat + 8 + tae_id*8)
        header = r.read(tae, 128)
        size = struct.unpack_from("<I", header, 12)[0]
        if header[:8] != b"TAE \0\0\0\xff" or not 128 <= size <= 0x100000:
            raise ValueError("Target TAE outside header/size constraints")
        payload = r.read(tae, size)
        action_count, actions = struct.unpack_from("<IQ", header, 0x54)
        if not tae <= actions <= actions + action_count*16 <= tae+size:
            raise ValueError("Action table outside target TAE")
        entries = [struct.unpack_from("<qQ", payload, actions-tae+i*16)
                   for i in range(action_count)]
        if [pointer for aid, pointer in entries if aid == action_id] != [body]:
            raise ValueError("Mapping does not point to the expected TAE action")
        if not tae <= body <= body+48 <= tae+size:
            raise ValueError("Action body outside target TAE")
        body_offset = body-tae
        event_count = struct.unpack_from("<H", payload, body_offset+0x20)[0]
        events_ptr = struct.unpack_from("<Q", payload, body_offset)[0]
        if event_count and not tae <= events_ptr <= events_ptr+event_count*24 <= tae+size:
            raise ValueError("Event table outside target TAE")
        events = []
        for i in range(event_count):
            offset = events_ptr-tae+i*24
            event_data = struct.unpack_from("<Q", payload, offset+16)[0]
            if not tae <= event_data <= event_data+4 <= tae+size:
                raise ValueError("Event data outside target TAE")
            # The runtime has replaced time offsets with inline float values.
            events.append({"start": struct.unpack_from("<f", payload, offset)[0],
                           "end": struct.unpack_from("<f", payload, offset+8)[0],
                           "type": struct.unpack_from("<i", payload, event_data-tae)[0]})

        behavior = typed(r.qword(modules+0x28), ".?AVCSChrBehaviorModule@CS@@")
        slot = r.qword(behavior+0x10)
        character = typed(r.qword(slot+0x30), ".?AVhkbCharacter@@")
        setup = typed(r.qword(character+0x90), ".?AVhkbCharacterSetup@@")
        bindings = typed(r.qword(setup+0x40), ".?AVhkbAnimationBindingSet@@")
        binding_header = r.read(bindings+0x18, 16)
        binding_table, binding_count, _ = struct.unpack("<Qii", binding_header)
        if not 0 <= index < binding_count <= 100000:
            raise ValueError("Target outside character binding table")
        with_triggers = typed(r.qword(binding_table+index*8),
                              ".?AVhkbAnimationBindingWithTriggers@@")
        bound_animation = r.qword(with_triggers+0x18) & ~1
        if bound_animation != binding:
            raise ValueError("Character and resource animation bindings differ")
        if (r.read(havok+0x98, 24) != layout or r.read(table+index*72,72) != entry
                or r.qword(time_act+0x10) != havok or r.qword(world+0x1e508) != player
                or r.read(bindings+0x18,16) != binding_header
                or r.qword(binding_table+index*8) != with_triggers
                or r.read(tae,128) != header):
            raise ValueError("Ownership or target metadata changed during read")
        result = {"pid": args.pid, "base": hex(args.base),
                  "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  "animation_id": args.animation, "player": hex(player),
                  "havok": hex(havok), "map_count": count,
                  "map_index_zero_based": index, "index_as_signed_16": signed_short(index),
                  "map_entry": hex(table+index*72), "tae": hex(tae),
                  "tae_action_count": action_count, "body": hex(body),
                  "event_count": event_count, "events": events,
                  "binding": hex(binding), "animation": hex(animation),
                  "animation_type": animation_type, "duration": duration,
                  "transform_tracks": tracks, "character": hex(character),
                  "binding_set": hex(bindings), "binding_count": binding_count,
                  "binding_with_triggers": hex(with_triggers),
                  "character_binding_matches_resource": True,
                  "budget_instruction": r.read(args.base+0xe83e18,9).hex(),
                  "read_bytes": r.used, "writes": False,
                  "limitation": "No live clip activation, trigger dispatch or playback verdict; signed conversion is a candidate mechanism, not a demonstrated complete cause."}
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with args.out.open("x", encoding="utf-8") as stream:
            json.dump(result, stream, indent=2)
        print(json.dumps({k:v for k,v in result.items() if k != "events"}, indent=2))
    finally:
        r.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("base", type=lambda x: int(x, 0))
    parser.add_argument("animation", type=int)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    target = Path(__file__).resolve().parents[4] / "target"
    if not args.out.resolve().is_relative_to(target):
        parser.error("Output must remain inside this project's target directory")
    inspect(args)
