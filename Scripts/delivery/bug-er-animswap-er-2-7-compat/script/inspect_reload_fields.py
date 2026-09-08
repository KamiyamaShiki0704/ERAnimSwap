"""Offline PE instruction evidence, not a runtime layout certification.

Topic: BUG-ERAnimSwap-ER-2-7-compat
Spec: spec.md@1.0 (compatibility cases extended to user-supplied 2.7.1.0)
Script-Version: 1.0
Requires pefile and capstone. No game process is opened or modified.
"""
import argparse
import bisect
import hashlib
import json
import struct
from pathlib import Path

import capstone
import pefile


def inspect(path):
    data = path.read_bytes()
    pe = pefile.PE(data=data)
    sections = [s for s in pe.sections if s.Characteristics & 0x20000000]
    functions = sorted((e.struct.BeginAddress, e.struct.EndAddress)
                       for e in pe.DIRECTORY_ENTRY_EXCEPTION)
    starts = [f[0] for f in functions]
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    decoder.detail = True
    result = {"executable": str(path), "sha256": hashlib.sha256(data).hexdigest(),
              "limitation": "Displacement references alone do not prove object ownership or runtime safety.",
              "fields": {}}
    for offset in (0x1E508, 0x1E668, 0x1E670, 0x1E678):
        needle = struct.pack("<I", offset)
        candidates = set()
        for section in sections:
            code = section.get_data()
            position = code.find(needle)
            while position >= 0:
                rva = section.VirtualAddress + position
                index = bisect.bisect_right(starts, rva) - 1
                if index >= 0 and rva < functions[index][1]:
                    candidates.add(functions[index])
                position = code.find(needle, position + 1)
        hits = []
        for start, end in sorted(candidates):
            for ins in decoder.disasm(pe.get_data(start, end - start), start):
                if any(op.type == capstone.CS_OP_MEM and op.mem.disp == offset
                       for op in ins.operands):
                    hits.append({"function_rva": hex(start), "instruction_rva": hex(ins.address),
                                 "instruction": f"{ins.mnemonic} {ins.op_str}"})
        result["fields"][hex(offset)] = hits
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    args = parser.parse_args()
    print(json.dumps(inspect(args.executable), indent=2))
