"""Bounded read-only sampling of user-triggered hkbClipGenerator instances.

Graph +0x120 is a candidate template-to-instance map in the audited build.
Every returned clip is independently RTTI/name checked; snapshots are non-atomic.
"""
import argparse
import datetime
import importlib.util
import json
from pathlib import Path
import struct
import time


def watch(args):
    spec = importlib.util.spec_from_file_location(
        "reader", Path(__file__).with_name("Read-TaeRepository.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    reader = module.Reader(args.pid, args.base)
    output = {"pid": args.pid, "base": hex(args.base), "writes": False,
              "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "samples": 0, "changes": [], "errors": [],
              "limitation": "User-driven sampled metadata, not a frame-perfect execution trace."}
    previous = None
    start = time.monotonic()
    try:
        while time.monotonic() - start < args.seconds:
            world = reader.qword(args.base+0x3d69ff8)
            player = reader.qword(world+0x1e508)
            mods = reader.qword(player+0x190)
            behavior = reader.qword(mods+0x28)
            slot = reader.qword(behavior+0x10)
            character = reader.qword(slot+0x30)
            graph = reader.qword(character+0x98)
            if reader.rtti(graph) != ".?AVhkbBehaviorGraph@@":
                raise ValueError("Unexpected behavior graph")
            map_object = reader.qword(graph+0x120)
            header = reader.read(map_object,16)
            table, count, mask = struct.unpack("<Qii",header)
            if not 0 <= count <= mask+1 <= 4096 or (mask+1) & mask:
                raise ValueError("Candidate clone map outside bounds")
            data = reader.read(table,(mask+1)*16)
            clips = []
            for i in range(mask+1):
                key, value = struct.unpack_from("<QQ",data,i*16)
                if key not in args.templates:
                    continue
                if reader.rtti(value) != ".?AVhkbClipGenerator@@":
                    raise ValueError("Target map entry not a clip generator")
                clip = reader.read(value,0x160)
                name_pointer = struct.unpack_from("<Q",clip,0x98)[0] & ~1
                name = reader.read(name_pointer,64).split(b"\0")[0].decode("ascii")
                if name != args.name:
                    raise ValueError("Target clip name changed")
                index = struct.unpack_from("<h",clip,0xc6)[0]
                control = struct.unpack_from("<Q",clip,0xd8)[0]
                binding = struct.unpack_from("<Q",clip,0xf0)[0]
                record = {"template":hex(key), "clip":hex(value), "name":name,
                          "signed_index":index, "unsigned_index":index & 65535,
                          "control":hex(control), "binding":hex(binding)}
                if binding:
                    record["binding_type"] = reader.rtti(binding)
                    binding_header = reader.read(binding,0x38)
                    record["binding_header"] = binding_header.hex()
                    animation = struct.unpack_from("<Q",binding_header,0x20)[0]
                    if animation:
                        record["animation"] = hex(animation)
                        record["animation_type"] = reader.rtti(animation)
                        animation_header = reader.read(animation,0x28)
                        record["duration"] = struct.unpack_from("<f",animation_header,0x1c)[0]
                        record["transform_tracks"] = struct.unpack_from("<i",animation_header,0x20)[0]
                    if reader.read(value,0x160)[0xc6:0xf8] != clip[0xc6:0xf8]:
                        output["errors"].append("Clip binding changed during sample")
                        continue
                clips.append(record)
            if reader.read(map_object,16) != header:
                output["errors"].append("Map changed during sample")
            else:
                if clips != previous:
                    event = {"elapsed":round(time.monotonic()-start,3),"clips":clips}
                    output["changes"].append(event)
                    print(json.dumps(event),flush=True)
                    previous = clips
                output["samples"] += 1
            time.sleep(0.1)
    except (OSError, ValueError) as error:
        output["errors"].append(str(error))
    finally:
        output["read_bytes"] = reader.used
        output["elapsed"] = round(time.monotonic()-start,3)
        reader.close()
        args.out.parent.mkdir(parents=True,exist_ok=True)
        with args.out.open("x",encoding="utf-8") as stream:
            json.dump(output,stream,indent=2)
        print(json.dumps({k:v for k,v in output.items() if k != "changes"}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid",type=int)
    parser.add_argument("base",type=lambda x:int(x,0))
    parser.add_argument("name")
    parser.add_argument("out",type=Path)
    parser.add_argument("templates",nargs="+",type=lambda x:int(x,0))
    parser.add_argument("--seconds",type=int,default=90)
    args = parser.parse_args()
    if not 1 <= args.seconds <= 90:
        parser.error("Duration must be 1..90 seconds")
    target = Path(__file__).resolve().parents[4]/"target"
    if not args.out.resolve().is_relative_to(target):
        parser.error("Output must be in this project's target directory")
    watch(args)
