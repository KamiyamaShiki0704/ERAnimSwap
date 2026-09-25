"""Offline, read-only PE audit of candidate word accesses at displacement C6.

Candidates are decoded from PE unwind starts, not arbitrary pattern offsets.
An offset match alone does not establish that the object is a clip generator.
"""
import argparse
import bisect
import hashlib
import json
import re
import struct
from pathlib import Path

import capstone
import pefile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("exe", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    data = args.exe.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != "1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891":
        raise ValueError("Unrecognized executable")
    pe = pefile.PE(data=data)
    image = pe.get_memory_mapped_image()
    directory = pe.OPTIONAL_HEADER.DATA_DIRECTORY[3]
    # pefile may cap the parsed entry count; iterate the bounded PE directory.
    pdata = image[directory.VirtualAddress:directory.VirtualAddress+directory.Size]
    entries = sorted((a, b) for a, b, _ in struct.iter_unpack("<III", pdata)
                     if 0 < a < b < len(image))
    starts = [a for a, _ in entries]
    section = next(s for s in pe.sections if s.Name.rstrip(b"\0") == b".text")
    text_start = section.VirtualAddress
    text_end = text_start + section.Misc_VirtualSize
    candidates = set()
    cursor = text_start
    while True:
        cursor = image.find(b"\xc6\0\0\0", cursor, text_end)
        if cursor < 0:
            break
        idx = bisect.bisect_right(starts, cursor) - 1
        if idx >= 0 and entries[idx][0] <= cursor < entries[idx][1]:
            candidates.add(entries[idx])
        cursor += 4
    dis = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    dis.detail = True
    findings = []
    for start, end in sorted(candidates):
        instructions = list(dis.disasm(image[start:end], start))
        for idx, ins in enumerate(instructions):
            if any(o.type == capstone.x86.X86_OP_MEM and o.mem.disp == 0xc6
                   and o.size == 2 for o in ins.operands):
                context = instructions[max(0, idx-3):idx+6]
                findings.append({"rva": hex(ins.address), "function": hex(start),
                                 "instruction": f"{ins.mnemonic} {ins.op_str}",
                                 "context": [f"{i.address:x} {i.mnemonic} {i.op_str}"
                                             for i in context]})
    windows = [(0x14608e7, 58), (0x15351c0, 27), (0x151524b, 5), (0x1515274, 10)]
    incoming = []
    text = image[text_start:text_end]
    for match in re.finditer(rb"[\xe8\xe9]|\x0f[\x80-\x8f]", text):
        source = text_start + match.start()
        size = len(match.group()) + 4
        if source + size > text_end:
            continue
        target = source + size + struct.unpack_from("<i", image, source+size-4)[0]
        if any(a < target < a+n and not a <= source < a+n for a, n in windows):
            incoming.append({"source": hex(source), "target": hex(target),
                             "kind": "unvalidated rel32 candidate"})
    for a, n in windows:
        for source in range(a-128, a+n+128):
            if a <= source < a+n:
                continue
            opcode = image[source]
            if opcode == 0xeb or 0x70 <= opcode <= 0x7f:
                target = source + 2 + struct.unpack_from("<b", image, source+1)[0]
                if a < target < a+n:
                    idx = bisect.bisect_right(starts, source)-1
                    if idx >= 0 and entries[idx][0] <= source < entries[idx][1]:
                        start, end = entries[idx]
                        if any(i.address == source and i.size == 2
                               for i in dis.disasm(image[start:end], start)):
                            incoming.append({"source": hex(source), "target": hex(target),
                                             "kind": "decoded rel8"})
    report = {"sha256": digest, "candidate_unwind_ranges": len(candidates),
              "word_c6_accesses": findings,
              "external_branches_to_patch_interiors": incoming,
              "limitation": "Candidates only; unwind-free leaf accessors are audited separately."}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
