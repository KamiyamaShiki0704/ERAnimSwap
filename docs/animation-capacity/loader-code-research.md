# Bounded Anibnd Loader Code Research

Date: 2026-09-25. Research only; no Spec confirmation, implementation, memory
writes, fixture generation, or crash reproduction performed by this sidecar.
Scope is the oversized-archive load crash, not late-TAE playback failure.

## Result

**No fixed archive-byte ceiling or justified capacity patch was established.**
The strongest new lead is a two-allocator, dynamically sized binder-buffer split
at `0x23D490`, reached from the Anibnd load processor. Its allocation calls and
null-return branches provide specific observation points for the main agent.
Small immediate values encountered in the capsule/repository constructors are
object sizes, alignment values, versions or states, not demonstrated byte caps.

## Evidence and Method

- EXE: `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe`, version
  2.7.1.0. SHA256 independently recomputed as
  `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`.
- [Snapshot manifest](../../target/animation-capacity-memory/20260924-202056-677-36520/manifest.json):
  capture `2026-09-24T20:20:56.7005803Z`, PID 36520, module base
  `0x7FF687730000`. All addresses below are **RVAs**, not absolute process VAs.
- [Capsule snapshot](../../target/animation-capacity-memory/20260924-202056-677-36520/rva-001FF000.bin):
  range `[0x1FF000,0x208000)`, SHA256
  `08FA56B70FD486B1F4145BF4B20D78B0FF283D020E08903F1118DB3C5F7C8161`.
- [Repository snapshot](../../target/animation-capacity-memory/20260924-202056-677-36520/rva-00CD0000.bin):
  `[0xCD0000,0xCD5000)` matches the disk bytes exactly. The `0x7E000` snapshot
  also matches disk exactly. Relevant capsule/vector bytes
  `[0x200240,0x20131C)` match exactly, including padding.
- Do not generalize that equality to the entire first snapshot: it initially
  differs at 100 bytes; applying its 24 PE DIR64 relocations in a temporary
  Python bytearray leaves four differences at `0x204B6A..0x204B6D`, outside the
  traced capsule methods. Their cause was not investigated or assumed.
- Tools: `D:/Python/python.exe -B -`, pefile, Capstone x86-64. Read files only;
  scripts ran through stdin and created no analysis files. Function boundaries
  came from raw 12-byte PE exception-directory entries, with adjacent unwind
  fragments joined where control flow requires it. Naive linear decoding past
  returns produces junk/padding and was not treated as evidence. No EXE-wide
  instruction/string sweep was performed. Callees outside the snapshots below
  are **disk-only evidence**, not live-validated code.

## Constructor and Entry Flow

RTTI behind vtable `0x29D6A68` identifies `CS::AnibndFileCap`; its slot `+0x48`
is `0x200E60` and slot `+0x50` is `0x2007E0`. Constructor `0x200240` initializes
the repository handle at `+0x90`, an allocator-backed pointer vector at `+0x98`,
and optional related-resource state through `+0xCD`. `0x2003D0` releases these
resources: it is teardown, not a large-buffer constructor. Metadata size getter
`0x200740` returns `0xD0`, consistent with this object layout, not archive bytes.

`0x200640` and `0xCCFC60` construct type metadata (base call `0x1EC2820`, static
storage, name slots `+0x38/+0x40`). Their TLS/static-initialization comparisons
are not resource-count checks. `0x200FE0` and `0xCD0930` are the corresponding
metadata getters, not archive-buffer allocation routines.

The identified processing chain is:

```text
AnibndFileCap::vtable+48 -> 200E60
  allocate 0x80-byte object -> 236A50 (CSLoadProcessor_Anibnd ctor)
  register processor through 265C650
processor vtable+38 -> 236F70 -> 237000 (state dispatcher)
  acquire input pointer/length -> 23D490 (CSSplitBinderData preparation)
  allocate 0x128 object -> 228950 (CSLoadProcessData_Anibnd ctor)
  enqueue 0x28 EzWorker -> 236FC0 -> data vtable+10 -> 228FD0
AnibndFileCap::vtable+50 -> 2007E0 (consume CSTM result)
  CD0120 (repository lookup/create) -> CD1220 (attach processed data)
```

The worker's full parser at `0x228FD0` was identified via RTTI/vtable and its prologue only;
its body is deliberately left as the next bounded analysis target. This trace
does not claim full framework scheduling order from a captured runtime call stack.

## Exact Instructions That Matter

### Dynamic binder-buffer allocation: strongest lead

At `0x2370CE` / `0x2370D9`, `0x2663E80` and `0x2663F30` supply the input pointer
and length. The latter returns a QWORD length at object `+0x28` when its access
count is nonzero. `0x23711E` calls `0x23D490` with:

```asm
237103  mov [rsp+20h], rbp          ; input length, fifth argument
237108  mov r9, r14                ; input pointer
23710B  mov r8, [rip+03B5424Eh]     ; global pointer slot RVA 03D8B360
237112  mov rdx, [rip+03B5426Fh]    ; global pointer slot RVA 03D8B388
237119  lea rcx, [rsp+38h]          ; split-data temporary
23711E  call 0023D490
```

`0x23D490` references `Source/File/Util/CSSplitBinderData.cpp` at string RVA
`0x29E42C8`. It starts a binder reader through `0x1ECB3D0`, obtains an entry count
from `0x1ECB940`, and visits entries through `0x1ECBB40`. A selected entry query
`0x1ECEE10` supplies a split offset; otherwise the split defaults to input length.
Thus `R14 = first-segment bytes`, and `[temporary+0x28] = total - R14`. The exact
entry predicate and whether the current input is already DCX-decoded were not
resolved here. The observed arithmetic is 64-bit; this does not prove all inner
parsers or allocators preserve that width.

```asm
23D597  sub r13, r14
23D59A  mov [rdi+28h], r13          ; remaining segment length
23D5BE  mov r8d, 10h               ; alignment
23D5C4  mov rdx, r14               ; dynamic first-segment length
23D5C7  mov rcx, r12               ; allocator from RVA 03D8B388
23D5CA  call qword ptr [rax+50h]
23D5DF  mov rcx, [rdi+10h]
23D5E3  test rcx, rcx
23D5E6  je 0023D686                ; cleanup / false result
23D5EC  mov rdx, [rdi+28h]          ; dynamic second-segment length
23D5F8  mov r8d, 10h               ; alignment
23D5FE  mov rcx, r9                ; allocator from RVA 03D8B360
23D601  call qword ptr [rax+50h]
23D60B  test rax, rax
23D60E  je 0023D686
```

The first allocator is optional in the general helper: when absent, the original
input pointer is reused. A zero remaining length skips the second allocation.
The `test al,0xF` at `0x23D57B` is a split-offset alignment check accompanied by
an alignment assertion, not a 15-byte capacity. The allocator virtual functions,
their concrete types, current budgets and failure behavior are **unknown**.
An allocator may itself assert rather than return null; these branches alone do
not prove oversized allocation failure is safely handled throughout the engine.

### Capsule result consumption: no upper byte bound here

At `0x20080D/0x200818`, the capsule acquires a pointer and size through
`0x265BA40/0x265BAA0`. Relevant checks:

```asm
200826  cmp rax, 4                 ; followed by jbe 20094E: minimum gate
200830  cmp dword ptr [rbx], 4D545343h ; "CSTM"
20083C  cmp dword ptr [rbx+4], 1    ; structure version assertion
20085A  cmp dword ptr [rbx+8], 2    ; anibnd processor-class version assertion
200878  mov rbx, [rbx+10h]         ; processed-data pointer
2008ED  mov r8, rbx
2008F5  call 00CD0120
2008FA  mov [rdi+90h], rax
```

Assertions reference AnibndFileCap.cpp and version-specific diagnostic strings.
These are not direct BND/DCX magic checks; patching the comparisons would not
expand the input allocation. `0x265BAD0` at the end releases the acquired access.

### Object sizes and vector growth: exclude false positives

| Site | Observed meaning | Not established |
| --- | --- | --- |
| `0x200E98..0x200EA0` | `EDX=8`, `ECX=0x80`, call `0x1EBBD40`, then processor ctor `0x236A50` | 128-byte archive cap |
| `0xCD0164..0xCD017E` | `EDX=8`, `ECX=0xA0`, allocation then object ctor `0xCD0C30` | 160-entry pool limit |
| `0x23713B..0x23715F` | `EDX=8`, `ECX=0x128`, allocation then processed-data ctor `0x228950` | 296-byte file cap |
| `0x237243..0x237274` | `EDX=8`, `ECX=0x28`, allocation then EzWorker ctor | Work queue capacity |
| `0x201284` | `movabs r8,0x1FFFFFFFFFFFFFFF`; vector max-size arithmetic; failure string `vector<T> too long` | Practical MiB limit |
| `0x200D47`, `0x236F87..0x236F94` | Compare completion/state bytes or state enum to 2/3/4 | Animation count ceiling |

`0x1EBBD40` forwards `(size, alignment, allocator)` to allocator vtable `+0x50`.
In `0x200AB0`, a related-resource list uses 0x30-byte records. Its derived count
feeds pointer-vector resize `0x201220`, which grows dynamically through `0x201010`
and `0x201100`. The latter computes `bytes = count * 8` at `0x20110D`, alignment
8 at `0x201115`, and calls allocator `+0x50` at `0x20111E`. This is a related
archive-reference vector, not a proven array of TAE animations or HKX clips.

## Next Bounded Targets and Unknowns

1. **Allocator identity and failure:** main agent can correlate the pointer slots
   `0x3D8B388` and `0x3D8B360` with its live read-only object work. The latter is
   also used by capsule/vector and repository-object allocation. Do not infer a
   pool budget or allocator class from the global address alone.
2. **Worker parser `0x228FD0`:** proven target of
   `CSLoadProcessData_Anibnd` vtable `0x29E0B58 + 0x10`, invoked by `0x236FC0`.
   This is a better next parser target than metadata constructors. Its limits,
   per-entry allocations, HKX handling and failure propagation remain unexamined.
3. **Binder reader helpers:** `0x1ECB3D0`, `0x1ECB940`, `0x1ECBB40`, and
   `0x1ECEE10` determine split inputs. Their internal validation and integer
   widths need checking only if the fixture evidence points to this phase.
4. **Repository attachment:** `0xCD0120` looks up/reuses an entry before allocating
   a new object and calls `0xCD1220`. The latter attaches/refcounts data and calls
   `0xCD1110`; its disk entry jumps outside the local range, so no claims were
   made from linear decoding of its following bytes. No allocator budget was
   recovered from repository metadata or attachment code.

Still missing: a reproduced failing archive threshold, failure call stack/state,
allocator request sizes and returned values for that failure, and confirmation
of the actual decode stage. The main agent owns fixture generation and live
reads; this static sidecar intentionally does not replace the red-capable
reproduction loop. **No instruction above is a recommended patch site.**

## Closing Context From Main Agent

At handoff, the user has made a large archive (path still pending) and reports
a crash on restart. The main agent is preparing archive verification and ProcDump
capture. Prioritize that crash evidence over further speculative patch research.

The main agent also reports global RVA `0x3D7F618` points to an ANIBND-named
repository. Its 0x200-byte read is stored at
[repository pointer snapshot](../../target/animation-capacity-memory/20260924-202250-156-36520/pointer-rva-03D7F618.bin).
The reported `+0x78` holder has `bucket_count=2039` at `+0x94` and linked-list
buckets: a hash-table bucket count is **not** a demonstrated resource capacity
bound. This later object interpretation is supplied by the main agent, not
independently re-read or verified by this static sidecar. No code patch is justified.
