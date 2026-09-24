# Upstream Worker Temporary Budget Research

Date: 2026-09-25. Scope: offline upstream call-site research only.
No process access, executable/dump modification, allocator patch, Rust change,
or runtime test was performed. This file is the only research output.

## Decision

**The failing thread's 128 KiB source is verified:** instruction RVA
`0xE83E18` in `CS::CSEzWorkThread` construction supplies `0x20000` as the
temporary-budget argument to `DLKR::DLThread` construction. This reaches
`DLThread+0x20`, then the known temporary-budget setter `0x1EBCBA0`.
The captured failing thread is **`LoadProcess_M5`**, not an unidentified
animation-only worker.

**A narrow animation-only data/constant patch point was not established.**
The verified immediate is narrower than changing the common allocator or
global default, but it is shared by general `CSEzWorkThread` instances.
Changing it would include the failing animation-loading worker AND unrelated
work pools. The captured dump already contains 26 matching general workers.
There is no separately forwarded load-pool budget at this construction call:
the worker constructor supplies the immediate itself.

For the main agent: `0xE83E18` is a verified **general-worker budget candidate**,
not an animation capacity constant or an approved patch. A strictly
`LoadProcess`-only solution needs conditional selection or another proven
mechanism; this bounded investigation did not find a single pool-specific
budget datum to change. No replacement size or patch mechanics are prescribed.

## Evidence Identity

- Project revision: `827f262c48460d70c9b9c8e3e7365a24e78df19a`.
- Knowledge-base revision: `9511e3e8137fd2cda9fca74bb07a4b688fe89087`.
- **D**: [captured dump](../../target/animation-capacity-crashes/20260924-203239-017/eldenring.exe_260925_043448.dmp),
  11,570,755,570 bytes. Module base `0x7FF687730000`; fault thread `0x673C`.
- **E**: `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe`,
  version `2.7.1.0`, 87,042,128 bytes. SHA256
  `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`.
  This is a SHA256 fingerprint, not SHA1.
- **P**: [previous crash analysis](crash-20260925.md) and its
  [read-only analyzer](../../Scripts/delivery/feat-animation-capacity/script/Analyze-AnimationCrash.py).
  P supplies the established vector-allocation failure and allocator-init
  interpretation; this sidecar does not reimplement that work.

All code addresses below are module-relative RVAs unless explicitly marked
VA. Disassembly uses D's mapped code, Capstone x86-64, and D's `.pdata`
function boundaries. Selected original bytes are independently compared to E.
Some other disk/mapped code regions differ, so disk disassembly alone was not
used to infer this chain. The full dump was not hashed.

## Captured Thread Binding

D contains the thread-entry return `base+0x1F3C5D2` at stack VA
`0x78A6BFFD68`. This is the return from the virtual callback at `0x1F3C5D0`.
`CSEzWorkThread` callback `0xE84090` saves caller RBX at entry `[rsp+0x10]`
(`0xE84090: 48 89 5C 24 10`). Therefore stack VA `0x78A6BFFD78`
holds the entry function's DLThread object, `0x18986B43080`.
This is a specific prologue check and object validation, not a general unwind.

| D location | Captured value / independent check |
| --- | --- |
| DLThread VA `0x18986B43080`, vtable | `base+0x30B5BC0`; RTTI `.?AVDLThread@DLKR@@` |
| DLThread `+0x10`, DWORD | `0x673C`, equal to exception ThreadId |
| DLThread `+0x18`, QWORD | callback object VA `0x18986B42FC0` |
| DLThread `+0x20`, QWORD | **`0x20000`** |
| DLThread `+0x78`, inline string | `LoadProcess_M5`; length `+0x88 = 14`, capacity `+0x90 = 15` |
| Callback object vtable | `base+0x2BFF238`; RTTI `.?AVCSEzWorkThread@CS@@` |
| Callback `+0x18` | back-reference `0x18986B43080`, agreeing with DLThread |
| Callback `+0x08` | pool VA `0x18986B52400`, RTTI `.?AVCSEzWork@CS@@` |
| Callback vtable slot 0 | `base+0xE84090`, matching the reviewed prologue |

RTTI anchors in D: DLThread COL RVA `0x33D7A18`, type descriptor RVA
`0x3D46CF0`; CSEzWorkThread COL `0x3372A88`, descriptor `0x3D0EF28`;
CSEzWork COL `0x33727D0`, descriptor `0x3D0EDE8`.

## Verified Upstream Chain

1. Load-pool construction function `0x23C4F0` creates two `CSEzWork` pools.
   Its call at `0x23C57C` uses name `LoadProcess_S` (UTF-16 RVA `0x29E3F50`).
   Its call at `0x23C600` uses `LoadProcess_M` (UTF-16 RVA `0x29E3F70`,
   loaded at `0x23C59E`). Both call pool constructor `0xE81A30`.
2. Pool constructor calls worker constructor `0xE83C70` at `0xE81F31`.
   Immediately preceding setup passes the generated worker name via
   caller `[rsp+0x20]`, a byte flag via `[rsp+0x28]`, and another DWORD
   via `[rsp+0x30]`. These are not the downstream DLThread budget argument.
3. Worker constructor installs vtable `base+0x2BFF238` at `0xE83CD5` /
   `0xE83CDC`, allocates a `0xB8`-byte DLThread object, and supplies:

   ```text
   E83E13  44 89 6C 24 30              mov [rsp+30h],r13d
   E83E18  48 C7 44 24 28 00 00 02 00  mov qword ptr [rsp+28h],20000h
   E83E21  48 89 44 24 20              mov [rsp+20h],rax       ; name
   E83E26  44 8B 4D 9B                 mov r9d,[rbp-65h]
   E83E2A  41 B8 00 00 01 00           mov r8d,10000h          ; stack size
   E83E30  48 8B D6                    mov rdx,rsi             ; callback
   E83E33  49 8B CC                    mov rcx,r12             ; DLThread
   E83E36  E8 25 49 05 01              call 1ED8760h
   E83E40                              mov [rsi+18h],rax       ; retain thread
   ```

4. DLThread constructor `0x1ED8760` pushes five registers and reserves
   `0x50` bytes. Thus `[rsp+0xA8]` below is entry `[rsp+0x30]`, the sixth
   Windows x64 argument, supplied by caller `[rsp+0x28]` above:

   ```text
   1ED87A7  48 89 5E 18                 mov [rsi+18h],rbx
   1ED87AB  48 8B 84 24 A8 00 00 00     mov rax,[rsp+A8h]
   1ED87B3  48 89 46 20                 mov [rsi+20h],rax
   ```

5. The constructor passes the same object as launch context to `0x1F3C7E0`
   at `0x1ED886B`. That function installs entry `0x1F3C550` at `0x1F3C7E9`.
   The entry sets up thread state before this already established setter:

   ```text
   1F3C57F  48 8B 4B 20     mov rcx,[rbx+20h]
   1F3C583  E8 18 06 F8 FF  call 1EBCBA0h
   ```

The thread budget is therefore supplied BEFORE callback execution and before
its animation-load work. P establishes how that setter feeds initialization;
the object budget and failed arena both equal 128 KiB in this capture.

## Candidate Coordinates And Breadth

| Field | Verified value |
| --- | --- |
| Instruction RVA / size | `0xE83E18` / 9 bytes |
| Instruction bytes | `48 C7 44 24 28 00 00 02 00` |
| Immediate RVA / width | `0xE83E1D` / 4 bytes, sign-extended by instruction |
| E raw file offset: instruction / immediate | `0xE83418` / `0xE8341D` |
| Comparison window | RVA `0xE83E13`, 40 bytes, identical in E and D |
| Window SHA256 | `5C12AA4E925123261FBC1F6AA39EA5F39E7FA2CBC3A26B82E6BBF5D5149B8965` |
| Window occurrences in D's first `.text` | 1, in RVA range `[0x1000,0x29A5800)` |

These coordinates identify evidence, not an instruction to patch disk or
an endorsement of injection timing. Raising this literal does not rewrite
the global default, the generic DLThread constructor, or unrelated constructor
arguments. Accordingly, independently larger budgets outside this worker
constructor are not numerically clamped by this candidate. This is a static
scope property, not a whole-program safety guarantee.

The direct-reference scan found one caller of worker constructor `0xE83C70`,
at `0xE81F31`. Its parent pool constructor has callers beyond animation load:
`0x619FEC` uses `WorldShift`, `0xBFD222` uses `HkAiThread`, `0x69E223` uses
`CSGeomModelInsDelayCreator`, and `0xE83680` through `0xE83967` create general
EzWork pools. Thus the candidate cannot honestly be labeled animation-only.

Bounded cross-check: examine each captured thread stack for the exact entry
return, recover the saved object, and require matching DLThread vtable,
ThreadId, and CSEzWorkThread callback vtable. This matched 26 objects, all
`+0x20 = 0x20000`: 19 named EzWork pools, `CSPlacementDebugMan1`,
`LoadProcess_S1`, and `LoadProcess_M1` through `LoadProcess_M5`.
Examples: S1 object `0x18986D72D20`, M1 `0x18986D72E00`, M4
`0x18986D72620`, M5 `0x18986B43080`. This is not a census of every worker
or proof of every thread's currently allocated arena size.

## Size Classification

These classifications follow argument dataflow into `DLThread+0x20`, not
the numerical appearance of a constant. The no-affinity constructor
`0x1ED88F0` has the same sixth-argument mapping: `0x1ED893B` loads
`[rsp+0x98]`, and `0x1ED8943` writes `[rsi+0x20]` after its smaller prologue.

| Call / assignment RVA | Temporary-budget source | Separate stack-size source / qualification |
| --- | --- | --- |
| `0xE83E36`, assignment `0xE83E18` | `0x20000`, proven failing-worker source | R8D = `0x10000` |
| `0xC146B2`, assignment `0xC14694` | `0x8000` | R8D = `0x10000` |
| `0xDDE044`, assignment `0xDDE01F` | `0x400` | R8D = `0x4000` |
| `0x1EBBD3`, assignment `0x1EBBAC` | `0x400` | R8D = `0x4000`; calls `0x1ED88F0` |
| `0x1B44652`, `0x240F051`, `0x263D35C`, `0x26F6322` | explicit zero, default-selection input per P | stack sizes respectively dynamic, `0x4000`, `0x10000`, `0x8000` |
| `0x1F58464`, `0x1F584B5`, `0x1F9F7C1` | register-sourced | `0x2000` in R8D is NOT the budget |
| `0x1FCB115`, `0x1FD4314`, `0x1F592A3` | sign-extended result of `0x1EE5B40` | configuration-derived path; value not classified as a fixed cap |
| `0x26EE970` | sign-extended DWORD from source object `+0x40` | separate stack-size DWORD at `+0x3C` |
| `0x26D8E3B` | RDI supplied at `[rsp+0x28]` | R8D = `0x20000` is stack size, NOT another 128 KiB budget finding |
| `0x1F7F4C0` | RDI, zero initialized at `0x1F7F3C7` on reviewed path | `0x1F7F3FC/403/40A/411` choose `0x100000/0x40000/0x10000/0x2000` into EBP, forwarded to R8: stack sizes, NOT temporary capacities |
| `0x20B6FAD` | register-sourced, not resolved here | R8D = `0x40000` is stack size |

Stack-size classification is independently anchored: DLThread constructor
saves R8 to R13 (`0x1ED878B`), passes it as R9 at `0x1ED885E`, then launcher
`0x1F3C7E0` forwards its low DWORD as EDX at `0x1F3C80A` to `0x2541578`.
That routine preserves EDX at `0x2541587` and passes it as RDX at
`0x25415C0` to the import call at `0x25415DD`. IAT RVA `0x29B2500`
resolves, using D's kernel32 export table, to **CreateThread**. This is the
stack-size parameter, separate from the budget stored in the object.

The other known setter wrapper, `0x1ED9140`, has 11 scanned direct callers:
`0x2275527`, `0x22755CF`, `0x227567F`, `0x2275717`, `0x22757A7`,
`0x2275877`, `0x227C74D`, `0x227CB21`, `0x227CEA5`, `0x227CFA5`,
`0x227D09D`. Each passes ECX = `0x2000`, guarded by the preceding check.
These are explicit 8 KiB setter inputs, not the established source of this
128 KiB thread object. Their subsystem ownership was not pursued.

## Limits And Actionable Handoff

- No verified animation-only budget constant was found. LoadProcess is a
  shared loading pool; animation exclusivity must not be inferred from the
  work it happened to execute at the crash.
- The general-worker immediate is the strongest upstream candidate if its
  wider scope is acceptable. A load-only approach would need discrimination
  of the relevant thread/pool and separate implementation analysis.
- Known larger/default budgets outside this constructor must remain intact.
  Register/configuration-sourced budgets were not flattened or guessed.
- Scans were bounded to direct E8/E9 references and selected RIP-relative
  references in the first mapped `.text`, followed by disassembly and object
  checks. Indirect calls, other executable regions, code redirection, and
  later budget changes are not exhaustively excluded. One redirected caller
  near `0x20189E1` was not decoded through its transformed control flow.
- Evidence covers this executable and one pre-patch capture only. It does
  not establish a safe replacement budget, animation-count ceiling, memory
  cost under all configurations, or a successful enlarged-archive load.
- Main-agent work remains allocator/backing-storage correctness, patch timing
  and mechanics, regression checks, and human runtime acceptance. No PASS or
  production-ready patch is claimed by this research.
