---
id: FEAT-ERAS-Animation-Capacity
type: design
version: 1.2
updated: 2026-09-25
based_on:
  - spec.md@1.0
---

# Animation Archive Investigation

## Evidence-backed Expansion Design

Startup correction in experimental.2: DllMain must remain a thin reason gate;
process_attach is inline(never). Do not allocate a large host-path buffer in
the gate or its inlined callees. Compare the main module handle against
GetModuleHandleW(L"eldenring.exe") instead. Retain live PE/code guards.
This is required because native64KiB-stack workers receive DLL_THREAD_ATTACH
before their entry point. Statically linked CRT notifications are left enabled.
Add an actual DLL-loading/small-stack-thread child-process regression; pure
patch-memory tests cannot catch entry-prologue stack use.

Supersedes the diagnostic-only implementation boundary below. Spec1.0 is
unchanged. See crash-20260925.md and thread-budget-research.md for the captured
allocator failure and constructor-to-TLS chain. Native allocator replay fails
at5395 entries/8092 requested/1128 free with128KiB and passes5669 entries at
512KiB and1MiB. This is not a complete game load test.

The first experimental DLL changes the general EzWork constructor's fixed
temporary budget from128KiB to1MiB. Full instruction at RVA0xE83E18:
`48 C7 44 24 28 00 00 02 00`. Only byteRVA0xE83E1F changes02->10.
This supplies a larger DLThread+0x20 size BEFORE worker start; the existing
TLS initializer allocates and bounds the same enlarged region. Preserve its
allocation checks, destruction and consumers unchanged. Other constructor
paths, global512KiB fallback, stack sizes and larger configured budgets are
untouched. No instruction relocation or callback is necessary.

Apply synchronously during process-attach, using only bounded memory reads,
Windows memory APIs and an atomic byte operation, before handing control back
to the loader. No config file, logging, sleep or disk hashing in this path.
Startup loaders only: preexisting workers are not resized. Changing this
literal late does not prove existing arena expansion. Test with a full restart.
Fixed1MiB is a measured test choice, not an animation count/MB capacity promise.
26 captured workers yield22.75MiB extra arena storage if all allocate; thread
counts vary and non-animation EzWork jobs share this constructor.

Fail-closed validation: x64 PE identity (timestamp0x6A96B418, image0x5E0DA00),
section boundaries, unique exact40-byte constructor window, downstream
argument store, TLS call, budget selection and back allocator guards. Validate
live readable committed image regions, then original byte again with atomic
compare/exchange. Write only02->10; a modified target, including a bigger
third-party budget, is a conflict and is never lowered. Flush instruction cache
and restore protection; on post-write API failure attempt rollback and log
explicit failure, not success. PE fields are compatibility evidence, not
cryptographic authentication. Offline EXE tests additionally enforce SHA256.

New [lib] entry is src/capacity/lib.rs. Old source remains uncompiled historical
reference; weapon configuration, filesystem copying and reload are unreachable.
Keep output filename for loader compatibility. Logger runs on a short worker
after loader lock release, records apply/refusal/disabled host and warns that
an applied constructor patch does not prove runtime loading. No hot-unload
support; keep DLL loaded until process exit. Remove DLL and restart to restore.

Tests: PE identity/section/guard mutations, target conflict/no-write, real
executable-memory immediate update and instruction execution, source SHA256
and guard uniqueness, native allocator red/green replay. CAP-P02/P05/P08 final
load/playback/lifecycle remain human items. Capacity exhaustion beyond1MiB still
uses the original engine failure behavior; this patch does not repair it.

## Boundary

Implement diagnostics/fixtures for the confirmed animation-file-only scope.
Do not change the existing DLL into a nominal capacity patch until an actual
bound/allocation chain is established. Keep the original DLL available.

## Fixture Preparation

Use the existing SoulsFormats assembly and installed Oodle decoder/encoder,
provided as explicit inputs, rather than inventing a DCX/BND writer. Read the
user's DLC01 archive, retain its original compendium and original clip entries,
and add copies of valid HKX bytes under unused clip names and binder IDs.
Allocate IDs within each existing three-digit TAE prefix; do not modify HKX
internals, compendium bytes or existing entries. Preserve compression settings.

This creates syntactically valid binder-size/entry-count stress inputs, not new
playable movesets. The copied clips have no new behavior/TAE selectors. If the
engine loads clips lazily, these extra entries may not exercise the failing
allocation path; runtime evidence must distinguish that case. Repeated payloads
also compress differently from unique clips. No threshold conclusion follows
from a nominal 2x/4x multiplier alone.

Write only under project `target/animation-capacity-fixtures`. Refuse existing
outputs, nonmatching clip naming/ID conventions, duplicate IDs/names, source
collision and excessive planned payload. No automatic MOD replacement or game
launch. Validate by reopening output, checking IDs/names and every payload,
and hashing the unchanged source again. Record exact disk/payload byte counts,
source/fixture hashes and original/new entry counts in a JSON manifest.

automation_scripts:
- Scripts/delivery/feat-animation-capacity/script/New-AnimationCapacityFixture.ps1
- Scripts/delivery/feat-animation-capacity/script/Read-AnimationProcessEvidence.ps1
- Scripts/delivery/feat-animation-capacity/script/Start-AnimationCrashCapture.ps1

## Runtime Evidence and Patch Gate

Obtain a live process from the user's EAC-disabled MOD environment. Start with
the unchanged archive. Authorized reads may inspect module memory, loader
objects and allocations. Do not change process privileges, bypass anti-cheat,
or silently deploy a large archive. No dump of unrelated processes.

If the failing archive crashes during startup, do not require a surviving
post-load process. After the user exits the baseline game, arm signed Microsoft
ProcDump for the next launch: full dump, unhandled exception including debug
breakpoints, one dump. Do not use persistent debugger installation or kill
flags. Keep dumps under ignored target/, never publish them. A capture can
briefly pause the failing process. The user uses their existing offline launcher;
the script neither launches the game nor swaps files. Record the intended
archive hash and loader context; intended path alone does not prove actual use.

Tool source and option semantics:
https://learn.microsoft.com/en-us/sysinternals/downloads/procdump

CAP-P01/P02/P03/P04 require controlled baseline/fixture runs and a failing call
stack or allocator/bounds observation. CAP-P05/P08 need visible playback of
existing selected animations, not only successful parsing. CAP-P06 needs a
future patch fixture proving fail-closed behavior. Record unsupported/unproven
cases explicitly until they can run. UE FrameRecorder infrastructure is not
applicable to this native Elden Ring DLL.

Before any patch: establish uniquely located version-specific instructions,
original bytes, allocation capacity, index widths, lifetime/cleanup and consumers.
If any remain unknown, do not ship a bypass or claim unlimited animations.
