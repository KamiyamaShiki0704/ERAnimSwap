"""Recognize the observed 2.7.1.0 animation scratch-allocation failure.

Requires minidump. Read-only; does not attach, unwind arbitrary stacks, or patch.
Offsets below are rejected unless this specific captured call sequence matches.
"""
import argparse
import json
import struct
from pathlib import Path

from minidump.minidumpfile import MinidumpFile
from minidump.streams.ContextStream import CONTEXT


def analyze(path):
    dump = MinidumpFile.parse(str(path))
    if not dump.exception or len(dump.exception.exception_records) != 1:
        raise ValueError("Expected exactly one exception record")
    exception = dump.exception.exception_records[0]
    module = next(m for m in dump.modules.modules
                  if Path(m.name).name.lower() == "eldenring.exe")
    base = module.baseaddress
    reader = dump.get_reader()
    with path.open("rb") as stream:
        stream.seek(exception.ThreadContext.Rva)
        context = CONTEXT.parse(stream)

    def qword(address):
        return struct.unpack("<Q", reader.read(address, 8))[0]

    def rtti(address):
        vtable = qword(address)
        col = qword(vtable - 8)
        type_rva = struct.unpack_from("<I", reader.read(col, 24), 12)[0]
        return reader.read(base + type_rva + 16, 128).split(b"\0")[0].decode("ascii")

    if context.Rip - base != 0x1EBB809:
        raise ValueError("Not the reviewed panic site; needs independent diagnosis")
    if reader.read(context.Rip, 11) != bytes.fromhex("c7042500000000baadde00"):
        raise ValueError("Unexpected panic instruction")
    expected_returns = [(0x78, 0x1EBB628), (0xA8, 0x22A1B0),
                        (0xD8, 0x22A046), (0x138, 0x229AC5)]
    for offset, expected in expected_returns:
        if qword(context.Rsp + offset) != base + expected:
            raise ValueError("Captured stack does not match the reviewed prologues")
    vector = qword(context.Rsp + 0x140)
    if vector != context.Rsp + 0x188:
        raise ValueError("Unexpected local vector address")
    allocator, begin, end, capacity_end = struct.unpack("<4Q", reader.read(vector, 32))
    if not begin <= end <= capacity_end or (end - begin) % 8:
        raise ValueError("Invalid captured vector bounds")
    allocator_name = rtti(allocator)
    if allocator_name != ".?AVDLBackAllocator@DLKR@@":
        raise ValueError("Unexpected vector allocator")
    parent = qword(allocator + 8)
    parent_name = rtti(parent)
    if not parent_name.startswith(".?AVTemporaryAllocator@"):
        raise ValueError("Unexpected parent allocator")
    arena = parent - 0x20
    arena_begin, arena_end, front, back = struct.unpack("<4Q", reader.read(arena, 32))
    if not arena_begin <= front <= back <= arena_end:
        raise ValueError("Invalid captured temporary arena bounds")
    count = (end - begin) // 8
    requested_elements = max(count + 1, count + count // 2)
    if context.R14 != requested_elements or end != capacity_end:
        raise ValueError("Not the reviewed full-vector growth request")
    requested_bytes = requested_elements * 8
    allocation_start = (back - requested_bytes) & ~7
    header_start = allocation_start - 16
    return {
        "dump": str(path), "dump_bytes": path.stat().st_size,
        "exception": str(exception.ExceptionRecord.ExceptionCode),
        "exception_information": exception.ExceptionRecord.ExceptionInformation,
        "fault_rva": hex(context.Rip - base),
        "reviewed_return_rvas": [hex(rva) for _, rva in expected_returns],
        "stack_method": "Specific prologue-derived frame offsets, not a general stack unwinder",
        "vector_entries": count, "requested_entries": requested_elements,
        "requested_bytes": requested_bytes,
        "allocator_type": allocator_name, "parent_type": parent_name,
        "arena_payload_bytes": arena_end - arena_begin,
        "arena_total_including_header": arena_end - arena,
        "arena_remaining_bytes": back - front,
        "allocation_header_bytes": 16,
        "allocation_fits": header_start >= front,
        "default_budget_bytes": struct.unpack("<I", reader.read(base + 0x3B445F8, 4))[0],
        "limitation": "Proves this captured scratch request cannot fit. Does not establish an archive byte ceiling or validate a patch. Default budget is not necessarily the crashing thread's selected budget.",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    args = parser.parse_args()
    print(json.dumps(analyze(args.dump), indent=2))
