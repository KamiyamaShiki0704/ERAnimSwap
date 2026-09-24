# Experimental.1 Startup Crash and Experimental.2 Correction

## Failure Packet

- Classification: regression introduced in the capacity DLL entry, stack
  overflow during native thread notification, not animation arena exhaustion.
- Expected: startup DLL accepts worker creation and preserves small stacks.
- Actual: immediate game exit; both WER dumps have EXCEPTION_STACK_OVERFLOW
  (0xC00000FD) in weapon_animation_hotreload.dll, RVA0x1A877 (stack probe).
- Private evidence retained under target/animation-capacity-crashes/
  20260925-startup/: eldenring.exe.41000.dmp (05:23:02),
  eldenring.exe.44056.dmp (05:23:23), and old-dll-repro.json.

Initial game log was read before any reproduction and contained:

```text
animation-capacity 0.3.0-experimental.1 (legacy weapon swap/reload disabled; old TOML ignored)
CAPACITY_PATCH_APPLIED rva=0xE83E1F EzWork temporary budget=131072 -> 1048576 bytes; future workers only
```

This proves byte installation only, not successful loading. Evidence handling
note: the first reproduction loaded the original delivery path and its logger
overwrote that log after it was read. The above is a recorded excerpt, not a
claim that the remaining file is the original runtime log. The finalized
harness always copies the DLL into an isolated target/ directory before load.

## Root Cause

Experimental.1 exported DllMain begins atRVA0x2A70. The optimized prologue
executes `mov eax,0x100C8; call __chkstk` BEFORE `cmp edx,1` (reason gate).
The host-name buffer `[u16;32768]` in an inlined callee forces this approximately
64KiB frame on EVERY notification, even when source-level code immediately
returns for DLL_THREAD_ATTACH. Game worker stacks can be64KiB. The captured
fault has RAX0x100C8 and RDX2 (THREAD_ATTACH); stack probe return is DLL+0x2A82.

Previously passing memory-patch tests and a large-stack Python LoadLibrary smoke
did not cover this Windows loader/thread interaction. The missing regression
was actual small-stack thread creation after loading the DLL.

## Repro and Fix

```powershell
python Scripts/delivery/feat-animation-capacity/script/Test-DllSmallStack.py target/delivery-0.3.0-experimental.1/weapon_animation_hotreload.dll --expect-overflow
python Scripts/delivery/feat-animation-capacity/script/Test-DllSmallStack.py target/delivery-0.3.0-experimental.2/weapon_animation_hotreload.dll --threads 100
```

The script loads a test copy in an isolated child and creates native threads
with a64KiB reservation. Their entry calls ExitThread directly, so no Python
callback consumes the deliberately small thread stack. It neither launches
nor attaches to a game. Child crash exit codes are checked by the parent.

- Old DLL: exit0xC00000FD, matching the game failure. Reproduced twice.
- Corrected DLL: exit0,100 thread create/exit cycles PASS.
- Fixed DllMain has a0x30-byte local frame instead of0x100C8. The process-only
  routine is inline(never), and the large pathname buffer is eliminated.
- Host identity uses main-module equality with GetModuleHandleW(L"eldenring.exe").
  PE and downstream instruction checks remain unchanged.
- CRT thread notifications remain enabled; no unsupported static-CRT
  DisableThreadLibraryCalls workaround. Patch timing and1MiB budget unchanged.

Release7/7 tests (actual EXE configured), clippy with warnings denied and release
build pass. The existing allocator/mapped-image regression still passes.
Delivery: target/delivery-0.3.0-experimental.2/weapon_animation_hotreload.dll,
250880 bytes, SHA256:
`E406E76B96FD230760F7C1D52A350112253065CFFB3023987AA35707D7BFAD57`.

The new local test profile differs only by the capacity DLL path from the
previous offline test profile. Original user MOD/assets/profile are untouched.
The isolated regression establishes this stack-overflow fix, not full game
loading/playback acceptance; the user must rerun the new startup launcher.
