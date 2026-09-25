"""Bounded read-only Anibnd capsule inventory for the audited ER build.

No process writes, injected calls, suspension, privilege changes or heap scan.
Reference layout: libER FD4 resource.hpp; confirm RTTI/owner before interpreting.
"""
import argparse
import ctypes as ct
import datetime
import json
from pathlib import Path
import struct


class Reader:
    def __init__(self, pid, base):
        self.kernel = ct.WinDLL("kernel32", use_last_error=True)
        self.kernel.OpenProcess.argtypes = [ct.c_uint32, ct.c_int, ct.c_uint32]
        self.kernel.OpenProcess.restype = ct.c_void_p
        self.kernel.ReadProcessMemory.argtypes = [ct.c_void_p, ct.c_void_p,
            ct.c_void_p, ct.c_size_t, ct.POINTER(ct.c_size_t)]
        self.kernel.ReadProcessMemory.restype = ct.c_int
        self.kernel.QueryFullProcessImageNameW.argtypes = [ct.c_void_p, ct.c_uint32,
            ct.c_wchar_p, ct.POINTER(ct.c_uint32)]
        self.kernel.CloseHandle.argtypes = [ct.c_void_p]
        self.handle = self.kernel.OpenProcess(0x1010, False, pid)
        if not self.handle:
            raise ct.WinError(ct.get_last_error())
        self.used = 0
        self.base = base
        try:
            name = ct.create_unicode_buffer(32768)
            length = ct.c_uint32(len(name))
            if not self.kernel.QueryFullProcessImageNameW(self.handle, 0, name, ct.byref(length)):
                raise ct.WinError(ct.get_last_error())
            if Path(name.value).name.lower() != "eldenring.exe":
                raise ValueError("Not the selected game process")
            header = self.read(base, 4096)
            nt = struct.unpack_from("<I", header, 0x3c)[0]
            if header[:2] != b"MZ" or nt > 1024 or header[nt:nt+4] != b"PE\0\0":
                raise ValueError("Invalid supplied image base")
            if struct.unpack_from("<I", header, nt+8)[0] != 0x6A96B418:
                raise ValueError("Unreviewed executable timestamp")
            if struct.unpack_from("<I", header, nt+24+56)[0] != 0x5E0DA00:
                raise ValueError("Unreviewed image size")
        except Exception:
            self.close()
            raise

    def close(self):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None

    def read(self, address, size):
        if not 0x10000 <= address < address + size <= 0x7fffffffffff:
            raise ValueError("Invalid memory interval")
        if not 0 < size <= 1024 * 1024 or self.used + size > 16 * 1024 * 1024:
            raise ValueError("Read budget exceeded")
        self.used += size
        data = ct.create_string_buffer(size)
        got = ct.c_size_t()
        if not self.kernel.ReadProcessMemory(self.handle, address, data, size, ct.byref(got)) or got.value != size:
            raise OSError(f"Unreadable interval {address:#x}+{size:#x}")
        return data.raw

    def qword(self, address):
        return struct.unpack("<Q", self.read(address, 8))[0]

    def rtti(self, obj):
        vtable = self.qword(obj)
        if not self.base <= vtable < self.base + 0x5E0DA00:
            raise ValueError("Object vtable not in audited game image")
        locator = self.qword(vtable-8)
        if not self.base <= locator < self.base + 0x5E0DA00:
            raise ValueError("RTTI locator not in game image")
        col = self.read(locator, 24)
        type_rva = struct.unpack_from("<I", col, 12)[0]
        if struct.unpack_from("<I", col)[0] != 1 or type_rva >= 0x5E0DA00-144:
            raise ValueError("Invalid RTTI descriptor")
        return self.read(self.base+type_rva+16, 128).split(b"\0")[0].decode("ascii")

    def name(self, address, data):
        length, capacity = struct.unpack_from("<QQ", data, 0x28)
        if length > 256 or capacity < length or capacity > 65536:
            raise ValueError("Invalid resource name bounds")
        if not length:
            return ""
        pointer = address+0x18 if capacity < 8 else struct.unpack_from("<Q", data, 0x18)[0]
        return self.read(pointer, length*2).decode("utf-16-le")


def inventory(pid, base, out):
    reader = Reader(pid, base)
    try:
        repository = reader.qword(base+0x3D7F618)
        root = reader.read(repository, 0xa0)
        root_name = reader.name(repository, root)
        root_type = reader.rtti(repository)
        if root_name != "ANIBND":
            raise ValueError("Wrong repository name")
        if struct.unpack_from("<Q", root, 0x88)[0] != repository:
            raise ValueError("Holder owner does not match repository")
        count = struct.unpack_from("<I", root, 0x94)[0]
        table = struct.unpack_from("<Q", root, 0x98)[0]
        if not 0 < count <= 4096:
            raise ValueError("Unexpected bucket count")
        buckets = reader.read(table, count*8)
        seen = set()
        records = []
        capsules = {}
        for index in range(count):
            address = struct.unpack_from("<Q", buckets, index*8)[0]
            while address:
                if address in seen or len(seen) >= 4096:
                    raise ValueError("Cycle/duplicate or node limit reached")
                seen.add(address)
                data = reader.read(address, 0x90)
                owner, next_address = struct.unpack_from("<QQ", data, 0x48)
                # In this build the item points to its holder, whose +0x10
                # points back to the repository. Validate both directions.
                if owner != repository + 0x78:
                    raise ValueError("Resource owner does not match holder")
                name = reader.name(address, data)
                kind = reader.rtti(address)
                record = {"address": hex(address), "name": name, "rtti": kind,
                          "state_byte_0x88": data[0x88], "bucket": index}
                records.append(record)
                if "c0000" in name:
                    capsules[f"capsule-{address:x}.bin"] = reader.read(address, 0x300)
                if reader.read(address, 0x90) != data:
                    raise ValueError("Resource mutated during snapshot; retry later")
                address = next_address
        if reader.read(table, count*8) != buckets or reader.qword(base+0x3D7F618) != repository:
            raise ValueError("Repository changed during traversal; retry later")
        patch = reader.read(base+0xE83E18, 9)
        result = {"pid": pid, "base": hex(base), "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  "repository": hex(repository), "repository_type": root_type,
                  "bucket_count_not_limit": count, "capsule_count": len(records),
                  "budget_instruction": patch.hex(), "budget_immediate": struct.unpack_from("<I",patch,5)[0],
                  "read_bytes": reader.used, "writes": False, "records": records,
                  "limitation": "Read-only non-atomic resource inventory; not a proof of all actions/events being loaded or playable."}
        out.mkdir(parents=True, exist_ok=False)
        for filename, data in capsules.items():
            (out/filename).write_bytes(data)
        (out/"repository.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        print(json.dumps({k:v for k,v in result.items() if k != "records"}, indent=2))
        print(json.dumps([r for r in records if "c0000" in r["name"]], indent=2))
    finally:
        reader.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("base", type=lambda value: int(value, 0))
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    target = Path(__file__).resolve().parents[4]/"target"
    if not args.out.resolve().is_relative_to(target):
        parser.error("Output must be inside this project's ignored target directory")
    inventory(args.pid, args.base, args.out.resolve())
