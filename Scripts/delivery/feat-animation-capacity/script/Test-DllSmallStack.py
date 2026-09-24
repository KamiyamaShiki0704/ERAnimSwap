"""Load the DLL and create small-stack native threads in an isolated child.

No game launch/attach. Child crashes are captured as exit codes. ctypes only.
"""
import argparse
import ctypes as ct
import json
from pathlib import Path
import subprocess
import sys
import shutil
import tempfile


def child(dll, stack_bytes, threads):
    kernel = ct.WinDLL("kernel32", use_last_error=True)
    kernel.SetErrorMode(3)
    kernel.CreateThread.argtypes = [ct.c_void_p, ct.c_size_t, ct.c_void_p,
                                   ct.c_void_p, ct.c_uint32, ct.POINTER(ct.c_uint32)]
    kernel.CreateThread.restype = ct.c_void_p
    kernel.WaitForSingleObject.argtypes = [ct.c_void_p, ct.c_uint32]
    kernel.WaitForSingleObject.restype = ct.c_uint32
    kernel.GetExitCodeThread.argtypes = [ct.c_void_p, ct.POINTER(ct.c_uint32)]
    kernel.CloseHandle.argtypes = [ct.c_void_p]
    loaded = ct.WinDLL(str(dll.resolve()))
    # ExitThread has a compatible one-argument Windows x64 entry signature.
    # No Python callback runs on the deliberately small stack.
    entry = ct.cast(kernel.ExitThread, ct.c_void_p)
    for _ in range(threads):
        thread_id = ct.c_uint32()
        thread = kernel.CreateThread(None, stack_bytes, entry, None,
                                     0x10000, ct.byref(thread_id))
        if not thread:
            raise ct.WinError(ct.get_last_error())
        try:
            if kernel.WaitForSingleObject(thread, 5000) != 0:
                raise RuntimeError("Native thread did not exit within five seconds")
            code = ct.c_uint32()
            if not kernel.GetExitCodeThread(thread, ct.byref(code)) or code.value != 0:
                raise RuntimeError("Native thread failed")
        finally:
            kernel.CloseHandle(thread)
    assert loaded


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dll", type=Path)
    parser.add_argument("--stack-bytes", type=int, default=65536)
    parser.add_argument("--threads", type=int, default=20)
    parser.add_argument("--expect-overflow", action="store_true")
    parser.add_argument("--child", action="store_true")
    args = parser.parse_args()
    if not args.dll.is_file() or not 65536 <= args.stack_bytes <= 2097152 or not 1 <= args.threads <= 100:
        parser.error("DLL must exist; stack65536..2097152, threads1..100")
    if args.child:
        child(args.dll, args.stack_bytes, args.threads)
    else:
        root = Path(__file__).resolve().parents[4] / "target" / "capacity-small-stack-tests"
        root.mkdir(parents=True, exist_ok=True)
        scratch = Path(tempfile.mkdtemp(prefix="run-", dir=root)).resolve()
        if not scratch.is_relative_to(root.resolve()):
            raise RuntimeError("Test directory escaped project target")
        copied = scratch / args.dll.name
        shutil.copy2(args.dll, copied)
        result = subprocess.run([sys.executable, __file__, str(copied),
                                 "--child", "--stack-bytes", str(args.stack_bytes),
                                 "--threads", str(args.threads)], timeout=30,
                                capture_output=True, text=True)
        code = result.returncode & 0xffffffff
        expected = 0xc00000fd if args.expect_overflow else 0
        print(json.dumps({"dll": str(args.dll.resolve()), "test_copy": str(copied), "stack_bytes": args.stack_bytes,
                          "threads": args.threads, "exit_code": hex(code),
                          "expected_exit": hex(expected), "passed": code == expected,
                          "stderr": result.stderr[-2000:]}, indent=2))
        sys.exit(0 if code == expected else 1)
