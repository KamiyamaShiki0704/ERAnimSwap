---
id: FEAT-ERAS-TAE-Capacity
type: design
version: 1.0
updated: 2026-09-25
based_on: spec.md@1.0
---

# Bounded Clip Index Expansion

## Evidence and Scope

See a875-index-investigation.md for paired live evidence. Offline audit of the
SHA256-pinned2.7.1.0 EXE finds27 word+C6 accesses among351 candidate unwind
ranges; most belong to other structures. Decode all entries from the exception
directory directly: pefile's default parsed list truncates before Havok code.
Private audit: target/tae-index-audit/c6-accesses.json.

Clip-specific reads: copy constructor145FE8E copies raw16 bits; activation
14608E7 resolves binding; teardown1461836 compares raw -2/-1 and restores -1;
leaf getter1461A90 zero-extends; setter1462320 writes16 bits. Constructor and
reset1462308 use -1. Graph type4 name registration151524B/1515274 also narrows
name-table indices; selective graph update15351C0 resolves the same clip field
and remap table. These are included, not just the observed activation path.
Other+C6 users (e.g.146B25B strided word array) are not blanket-patched.

Supported candidate domain:0..65533;65534/65535 reserved for -2/-1. This is a
bounded experimental range, not unlimited, a per-file limit, or a guarantee
for malformed resources. Content exceeding65534 combined slots is unsupported;
this patch does not widen serialized layouts or implement a global admission
controller. Existing32-bit resource tables are not resized or reordered.

## In-place Instruction Changes

1. Activation RVA14608E7,58 bytes: keep raw-word read, RBP stack save and
   existing -1 branch destination1460A1B. Sign-extend then distinguish -2;
   pass -2 unchanged without remap. Zero-extend all remaining indices, use
   original context remap table when present, join at1460921. No stack layout,
   calls, unwind prologue, binding ownership or fallback changes.
2. Graph selection RVA15351C0,27 bytes: signed read, unsigned comparison
   against0xFFFFFFFE to preserve both sentinels; otherwise zero-extend and
   perform existing optional remap. Same join15351DB. RAX high32 not live:
   following MOVSXD consumes EAX and subsequent call clobbers volatile RAX.
3. Name registration RVA151524B,5 bytes: zero-extend the nonnegative table
   count default into R8D instead of sign-extending its low16 bits into R8.
   Within supported domain default0..65533 is preserved by native hash lookup.
4. Registration equality RVA1515274,10 bytes: zero-extend the stored word,
   compare against full32-bit nonnegative name count. This path resets clip
   state before writing either lookup result or -1; it does not produce -2.
   Under supported counts0..65533 neither reserved word compares equal.

Preserve all branch destinations, subsequent native binding lookup, reference
counts, fallback animation creation, cleanup and track/event handling. The
existing128KiB->1MiB constructor literal remains the fifth change.

## Installation Contract

Startup loader only, synchronous process attach before game initialization;
no hot injection or hot reload. Reject if WorldChrMan singleton already exists.
That check detects known late loading, not every possible loader race; startup
loading is still a precondition. Never write to another process in this work.

Validate PE identity, exact original windows for every change and relevant
surrounding sentinel/consumer guards before any write. Existing or mixed
patches are refused, including experimental.2 already loaded. Use a bounded
five-edit transaction: make all target windows writable, recheck originals,
apply edits, flush cache, restore protections; failures attempt rollback and
report refusal, not success. No allocation/file I/O in apply. Pin DLL until exit.
New wider instruction writes are safe only under the startup precondition;
do not represent them as atomic or safe concurrent with executing game code.

## Verification

Native executable-page harness runs exact original/replacement windows,
without invoking any game entry point. Cover all65536 raw words through both
activation/remap paths and graph resolution, explicit sentinel control flow,
registered-name defaults/equality, and native teardown behavior. Demonstrate
original35781->-29755 then corrected35781, retaining remapped32-bit results.
These are instruction semantics, not whole-game acceptance.

Guard mutations/conflicts must reject before writes. Map the actual EXE as a
private SEC_IMAGE copy in the test process, apply full transaction, verify
exact changed ranges and unchanged disk hash. Retain small-stack DLL loader
regression, fmt, tests, clippy and release build. Store isolated candidate DLL,
hashes and test results; do not overwrite the active mod DLL.

Human Probe: offline1.17.1, both supplied packages,120s load timeout, target
skill/ordinary attack/movement three repetitions each. Reacquire all process
pointers, verify candidate bytes, capture fresh target binding; user judges
motion and attack/effect correctness. Prior captures cannot validate a new DLL.
No automated visible-event oracle exists; mark this manual, not automatic PASS.
No UE runtime is involved; UE FrameRecorder infrastructure is inapplicable.
