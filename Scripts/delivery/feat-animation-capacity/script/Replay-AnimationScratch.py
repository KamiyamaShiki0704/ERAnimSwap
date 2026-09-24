"""Replay vector growth with the audited game's back allocator, in this process only.

Requires pefile and capstone. No game attach, game writes or archive modifications.
This is an allocator regression, not complete engine/playback acceptance.
"""
import argparse
import ctypes as ct
import hashlib
import json
from pathlib import Path

import capstone
import pefile

EXE_HASH = "1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891"
ALLOC_RVA = 0x1F35530
ALLOC_LENGTH = 0x50


def replay(allocate, budget, entries):
    storage = ct.create_string_buffer(budget)
    start = ct.addressof(storage)
    arena = (ct.c_uint64 * 4)(start + 56, start + budget, start + 56, start + budget)
    capacity = 0
    current = 0
    old_pointer = 0
    while current < entries:
        if current == capacity:
            requested = max(capacity + 1, capacity + capacity // 2)
            remaining = arena[3] - arena[2]
            pointer = allocate(ct.addressof(arena), requested * 8, 4)
            if not pointer:
                return {"budget_bytes": budget, "loaded_entries": current,
                        "requested_entries": requested, "remaining_bytes": remaining,
                        "request_bytes": requested * 8, "passed": False}
            if not arena[0] <= pointer < pointer + requested * 8 <= arena[1]:
                raise RuntimeError("Allocator produced an out-of-bounds pointer")
            if old_pointer:
                ct.memmove(pointer, old_pointer, current * 8)
                # With a newer allocation below it, the engine's free path marks
                # the old block unused; it cannot move the current back cursor.
                ct.c_uint64.from_address(old_pointer - 8).value = 0
            old_pointer = pointer
            capacity = requested
        ct.c_uint64.from_address(old_pointer + current * 8).value = current
        current += 1
    values = (ct.c_uint64 * entries).from_address(old_pointer)
    if any(value != index for index, value in enumerate(values)):
        raise RuntimeError("Vector contents did not survive growth")
    return {"budget_bytes": budget, "loaded_entries": current, "capacity": capacity,
            "remaining_bytes": arena[3] - arena[2], "passed": True}


def run(exe, entries):
    data = exe.read_bytes()
    if hashlib.sha256(data).hexdigest() != EXE_HASH:
        raise ValueError("Unreviewed executable; refusing to execute extracted code")
    pe = pefile.PE(data=data)
    code = pe.get_data(ALLOC_RVA, ALLOC_LENGTH)
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    decoder.detail = True
    instructions = list(decoder.disasm(code, ALLOC_RVA))
    if sum(i.size for i in instructions) != len(code) or instructions[-1].mnemonic != "ret":
        raise ValueError("Invalid function boundaries")
    for ins in instructions:
        if ins.group(capstone.CS_GRP_CALL):
            raise ValueError("Extracted function must not call outside code")
        for operand in ins.operands:
            if operand.type == capstone.CS_OP_MEM and operand.mem.base == capstone.x86.X86_REG_RIP:
                raise ValueError("Extracted function must not reference external RIP-relative data")
        if ins.group(capstone.CS_GRP_JUMP):
            if len(ins.operands) != 1 or ins.operands[0].type != capstone.CS_OP_IMM:
                raise ValueError("Indirect branch is not permitted")
            if not ALLOC_RVA <= ins.operands[0].imm < ALLOC_RVA + len(code):
                raise ValueError("Branch leaves audited function")
    kernel = ct.WinDLL("kernel32", use_last_error=True)
    kernel.VirtualAlloc.argtypes = [ct.c_void_p, ct.c_size_t, ct.c_uint32, ct.c_uint32]
    kernel.VirtualAlloc.restype = ct.c_void_p
    kernel.VirtualProtect.argtypes = [ct.c_void_p, ct.c_size_t, ct.c_uint32, ct.POINTER(ct.c_uint32)]
    kernel.VirtualProtect.restype = ct.c_int
    kernel.VirtualFree.argtypes = [ct.c_void_p, ct.c_size_t, ct.c_uint32]
    kernel.VirtualFree.restype = ct.c_int
    kernel.GetCurrentProcess.restype = ct.c_void_p
    kernel.FlushInstructionCache.argtypes = [ct.c_void_p, ct.c_void_p, ct.c_size_t]
    kernel.FlushInstructionCache.restype = ct.c_int
    memory = kernel.VirtualAlloc(None, len(code), 0x3000, 0x04)
    if not memory:
        raise ct.WinError(ct.get_last_error())
    try:
        ct.memmove(memory, code, len(code))
        previous = ct.c_uint32()
        if not kernel.VirtualProtect(memory, len(code), 0x20, ct.byref(previous)):
            raise ct.WinError(ct.get_last_error())
        if not kernel.FlushInstructionCache(kernel.GetCurrentProcess(), memory, len(code)):
            raise ct.WinError(ct.get_last_error())
        allocate = ct.WINFUNCTYPE(ct.c_void_p, ct.c_void_p, ct.c_uint64, ct.c_uint64)(memory)
        results = [replay(allocate, budget, entries) for budget in (128 * 1024, 512 * 1024, 1024 * 1024)]
        original = results[0]
        if (original["loaded_entries"], original["requested_entries"], original["remaining_bytes"]) != (5395, 8092, 1128):
            raise AssertionError("Baseline does not reproduce the captured failure")
        if not results[-1]["passed"]:
            raise AssertionError("Expanded budget does not accommodate this input")
        return {"exe_sha256": EXE_HASH, "allocator_rva": hex(ALLOC_RVA),
                "input_entries": entries, "results": results,
                "scope": "Actual allocator code with reconstructed vector growth/free marking. No engine threads, archive parsing or playback executed."}
    finally:
        kernel.VirtualFree(memory, 0, 0x8000)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("exe", type=Path)
    parser.add_argument("--entries", type=int, default=5669)
    args = parser.parse_args()
    if not 5396 <= args.entries <= 20000:
        parser.error("entries must be between 5396 and 20000")
    print(json.dumps(run(args.exe, args.entries), indent=2))
