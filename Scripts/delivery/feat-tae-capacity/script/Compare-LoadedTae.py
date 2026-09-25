"""Compare resident TAE action tables with an existing private archive inventory.

Audited build only. No game calls, process writes, suspension or heap scan.
This checks resident metadata, not playback or runtime lookup correctness.
"""
import argparse
import datetime
import importlib.util
import json
from pathlib import Path
import re
import struct


def compare(args):
    spec = importlib.util.spec_from_file_location(
        "reader", Path(__file__).with_name("Read-TaeRepository.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    reader = module.Reader(args.pid, args.base)
    try:
        if reader.rtti(args.capsule) != ".?AVAnibndResCap@CS@@":
            raise ValueError("Wrong capsule type")
        if reader.name(args.capsule, reader.read(args.capsule, 0x90)) != "c0000":
            raise ValueError("Not the base c0000 capsule")
        resource = reader.qword(args.capsule + 0x78)
        if reader.rtti(resource) != ".?AVCSLoadProcessData_Anibnd@CS@@":
            raise ValueError("Wrong processed resource type")
        havok = reader.qword(resource + 0x60)
        if reader.rtti(havok) != ".?AVHvkAnim@CS@@":
            raise ValueError("Wrong animation resource type")
        tae = reader.qword(havok + 0xa8)
        if reader.rtti(tae) != ".?AVTaeDat@CS@@":
            raise ValueError("Wrong TaeDat type")
        # Live code 1B6568/1B65D8/1B6677 establishes these indexed fields.
        table = reader.read(tae + 8, 999 * 8)
        wrappers = reader.read(tae + 0x1f40, 999 * 8)
        source = json.loads(args.inventory.read_text(encoding="utf-8-sig"))
        records = []
        expected_slots = set()
        for file in source["Files"]:
            match = re.search(r"(?:^|[\\/])a(\d+)\.tae$", file["Name"], re.I)
            if not match:
                raise ValueError("Unexpected TAE filename: " + file["Name"])
            slot = int(match[1])
            if slot in expected_slots:
                raise ValueError("Duplicate expected TAE slot")
            expected_slots.add(slot)
            row = {"name": file["Name"], "slot": slot,
                   "expected_count": file["ActionCount"]}
            records.append(row)
            if not 0 <= slot < 999:
                row["status"] = "outside_observed_loader_id_range"
                continue
            pointer = struct.unpack_from("<Q", table, slot * 8)[0]
            row["pointer"] = hex(pointer)
            row["wrapper"] = hex(struct.unpack_from("<Q", wrappers, slot * 8)[0])
            if not pointer:
                row["status"] = "missing_resident_tae"
                continue
            header = reader.read(pointer, 0x80)
            if header[:8] != b"TAE \0\0\0\xff":
                raise ValueError("Unrecognized TAE header")
            size = struct.unpack_from("<I", header, 0xc)[0]
            count, actions = struct.unpack_from("<IQ", header, 0x54)
            if count > 65536 or size > 256 * 1024 * 1024:
                raise ValueError("Unexpected TAE bounds")
            if count and not pointer <= actions <= actions + count * 16 <= pointer + size:
                raise ValueError("Action table outside resident file")
            raw = reader.read(actions, count * 16) if count else b""
            ids = [struct.unpack_from("<q", raw, i * 16)[0] for i in range(count)]
            expected = [a["Id"] for a in file["Actions"]]
            row.update(resident_count=count, resident_bytes=size,
                       ids_equal=ids == expected,
                       status="match" if ids == expected and size == file["Bytes"] else "mismatch")
            if ids != expected:
                row["resident_ids"] = ids
            if reader.read(pointer, 0x80) != header:
                raise ValueError("TAE header changed during read")
        if reader.read(tae + 8, 999 * 8) != table or reader.read(tae + 0x1f40, 999 * 8) != wrappers:
            raise ValueError("TAE registry changed during read")
        if reader.qword(args.capsule + 0x78) != resource or reader.qword(resource + 0x60) != havok or reader.qword(havok + 0xa8) != tae:
            raise ValueError("Ownership changed during read")
        result = {"pid": args.pid, "base": hex(args.base),
                  "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  "capsule": hex(args.capsule), "resource": hex(resource),
                  "havok": hex(havok), "tae_dat": hex(tae),
                  "budget_instruction": reader.read(args.base + 0xe83e18, 9).hex(),
                  "expected_files": len(records),
                  "matched_files": sum(x["status"] == "match" for x in records),
                  "resident_action_records": sum(x.get("resident_count", 0) for x in records),
                  "extra_resident_slots": [i for i in range(999) if i not in expected_slots and struct.unpack_from("<Q", table, i*8)[0]],
                  "read_bytes": reader.used, "writes": False,
                  "limitation": "Non-atomic resident file/header/ID comparison; not event payload, executable lookup, import resolution or playback validation.",
                  "records": records}
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with args.out.open("x", encoding="utf-8") as stream:
            json.dump(result, stream, indent=2)
        print(json.dumps({k: v for k, v in result.items() if k != "records"}, indent=2))
        print(json.dumps([x for x in records if x["status"] != "match"][:10], indent=2))
    finally:
        reader.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("base", type=lambda x: int(x, 0))
    parser.add_argument("capsule", type=lambda x: int(x, 0))
    parser.add_argument("inventory", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    target = Path(__file__).resolve().parents[4] / "target"
    if not args.out.resolve().is_relative_to(target):
        parser.error("Output must be within this project's ignored target directory")
    compare(args)
