# a875_040000 Index Investigation

## User Reproduction

User identifies a875_040000 as a stable T-pose when using the skill of weapon
68510000, Red Bear's Claw. They confirm prior failure with experimental.2 and
restarted with it for this inspection. This is separate from archive-load crash.

PID42216/start2026-09-25T17:44:55+08:00, EXE base0x7FF75D8A0000.
Loaded DLL from F:/GoldenAge/_GoldenAge/dll/CapacityExpansion matches
E406E76B96FD230760F7C1D52A350112253065CFFB3023987AA35707D7BFAD57.
Live budget bytes atRVA0xE83E18 are48c744242800001000,1MiB.
Current c0000 archive matches failing1 SHA256
`E7E3F7D53A6EB396B9BD8894A2015DDCF5A740FD7017B54A8AE74C945CC870E2`.
Full resident comparison repeats779 files/
44212 action IDs matching; a999 remains outside the observed loading range.

## Verified Target Metadata

| Item | Failing session result |
| --- | --- |
| a875.tae action count | 6 |
| Target full ID | 875040000 |
| Resource animation mapping count | 48059 |
| Target zero-based mapping index | 35781 /0x8BC5 |
| Target interpreted as signed16 | -29755 |
| Character animation binding count | 48059 |
| Character binding at index35781 | Same binding as resource map |
| Animation object | hkaSplineCompressedAnimation |
| Duration /transform tracks | 2.2000000477s /42 |
| Target TAE events | 78 |

Relevant validated pointers, only for this session:

- Player0x23C3C54A880; CSChrTimeActModule0x23DA1515D40.
- HvkAnim0x23D68B50140; map0x23C367E0080, entry0x23C36A54FE8.
- TaeDat0x23D6BA9A080; a8750x2415BE2DE80; action body0x2415BE2E060.
- hkaAnimationBinding0x23D6B911040; animation0x23D6B913900.
- hkbCharacter0x23C15458000; binding set0x23D6BF9CBE0;
  hkbAnimationBindingWithTriggers0x23C3709FDC0.

Read-AnimationIndex.py reacquires the local-player chain, validates RTTI and
ownership, confirms the sorted mapping, exact TAE action pointer and character
binding equality. It reads approximately3.5MiB and writes only private JSON.
Success means metadata is present, not that playback succeeds.

Both supplied a875 files have6 actions and Standard miniheaders for40000:
ImportsHKX=false,AllowDelayLoad=false,ImportHKXSourceAnimID=-1, same
a875_040000.hkt name. Failing target has78 events, working target74, so these
are not byte-identical controls. Failing resident event types and start times
match all78;75 endpoints match,3 file100.0 values are runtimeFLT_MAX. Runtime
event time fields contain inline floats, not file offsets/pointers. Parameters
and event dispatch are not validated. Do not label endpoint conversion damage.

## Narrowing Evidence

Native live code and loaded templates establish a specific candidate mechanism:

1. HvkAnim map builder0x1B44D0 creates72-byte records and stores32-bit count
   at+0x98, pointer+0xA0. ID lookup0x1B3F90 searches with32-bit indices.
2. Behbnd registration0x22E3D0 uses that lookup, then narrows the result before
   virtual setter+0xE0 (call sites0x22E4C6 and0x22E5F1).
3. hkbClipGenerator vtableRVA0x2D4D2D0 was found via RTTI type0x3D17838 and
   COL0x3382FC8. Its setter0x1462320 writes DX to object+0xC6.
4. Activation0x1460830 reads that field.0x146090D and0x146091E use MOVSX,
   making values32768..65535 negative. This is concrete code behavior, not an
   inferred type solely from a reference SDK.
5. A bounded search of the actual behbnd payload, in two separately budgeted
   intervals totaling19766728 bytes, finds two target clip templates:
   0x23C3A199C10 and0x23C3A6B1330. Both names are a875_040000 and both have
   index0x8BC5. Their runtime-control fields are null because these are templates,
   not an observed executing clip. No heap-wide search was performed.

Disassembly of fragmented functions follows direct control flow rather than
assuming that one PE unwind interval is the whole function. Discovery-only
linear snippets may include padding; only the verified paths above are used.

This supports a signed16 internal-index limitation as the leading hypothesis.
It does not yet prove every downstream lookup/fallback, the actual skill node's
execution, a safe patch strategy, or the exact supported capacity after a fix.
Unsigned widening alone may conflict with -1/-2 sentinels or other16-bit users.
Do not simply replace every MOVSX, enlarge arbitrary allocations or promise
unlimited TAE capacity. Existing1MiB archive patch remains unchanged.

## User-triggered Playback-node Capture

The user triggered the skill during the90-second sampling window beginning
2026-09-25T10:07:36.614324Z. clip-trigger-live.json records894 samples with no
read errors, and30 changes containing the exact a875_040000 runtime clip.
The map's template key0x23C3A199C10 resolves to independently RTTI/name-checked
hkbClipGenerator instances. This addresses the earlier concern that only an
unused template had been inspected.

All30 target records contain signed index-29755 (raw0x8BC5). Their actual
binding pointers differ from the resource/character-table binding
0x23D6B911040. All captured bindings are hkaAnimationBinding with zero entries
in the transform-track-to-bone mapping array at+0x30, versus42 entries in the
resource binding. The runtime creates different such bindings across activations.
Combined with the human T-pose report and code narrowing path, this is stronger
evidence for a playback binding-resolution failure than mere table counts.

The first15-second sample was empty. A later45-second supplemental window also
captured no target; it does not invalidate the intervening successful90-second
capture. The latter saved binding headers but not the pointed-to animation's
RTTI, so a specific fallback animation class is NOT established. Do not follow
old transient pointers after the clip is gone and call that current evidence.

No complete executing call stack is available. The normal2 control below now
supersedes the earlier missing-control checkpoint.
Other16-bit consumers, sentinel handling(-1/-2), fallback behavior and resource
lifetime still need review before choosing a patch. Captured data supports a
specific leading mechanism, not an already-verified capacity fix.

## Evidence and Remaining Checks

Private evidence under target/tae-memory/42216-exp2-a875:

- repository.json,tae-comparison.json,target-probe.json.
- animation-map-summary.json,clip-template-prefix.json,
  clip-template-remainder.json,hkbClipGenerator-vtables.json.
- code-map-entry.txt,caller-22e3d0.txt,cfg-1460830.txt.
- a875-40000-source-{1,2}.json,a875-resident.bin.

Watch-ClipBinding.py samples a candidate template-to-instance map rooted at
the player's graph+0x120; each target is checked by RTTI and exact name.
An empty sample does not show that the animation was never requested: the
trigger may be outside the sample window or use another graph/layer.

## Normal Control and Replay Check

User switched to working2, restarted and reports normal playback. PID47276,
start2026-09-25T18:15:30+08:00, same EXE base and experimental.2 DLL hash;
live1MiB instruction verified. Active archive SHA256 matches working2:
`B0797EF5FE1CE73229BA15EE6845F272ABC0F78649655E41E5A53ABFDC954F25`.

The90-second sampler completed896 samples with zero errors. Two nonempty
target observations were captured during user skill releases. Both use the
resource binding0x1F7FA5EA040, with hkaSplineCompressedAnimation at
0x1F7FA5ECCC0,2.2000000477s,42 transform tracks and42 mapping entries.
Private evidence: target/tae-memory/47276-normal-a875/{target-probe,
identity-templates,clip-trigger-live}.json.

| Measurement | Failing1 | Working2 |
| --- | --- | --- |
| Unified mapping entries | 48059 | 24528 |
| Target zero-based index | 35781 | 23341 |
| Actual clip signed index | -29755 | 23341 |
| Captured target observations | 30 | 2 |
| Bindings matching resource | 0 | 2 |
| Actual binding track mapping entries | 0 | 42 |
| User-visible result | T-pose | Normal |

Repeatable offline predicate (paths relative to project root):

```powershell
python -B Scripts/delivery/feat-tae-capacity/script/Check-ClipBindingEvidence.py target/tae-memory/42216-exp2-a875/target-probe.json target/tae-memory/42216-exp2-a875/clip-trigger-live.json
# FAIL, 0/30 matching, exit1
python -B Scripts/delivery/feat-tae-capacity/script/Check-ClipBindingEvidence.py target/tae-memory/47276-normal-a875/target-probe.json target/tae-memory/47276-normal-a875/clip-trigger-live.json
# PASS, 2/2 matching, exit0
```

Both commands were executed. This predicate checks captured binding identity,
not event dispatch or visual acceptance; user observations supply the latter's
motion component. The pair differs in content, and this is not yet a minimal
single-variable boundary experiment. A fresh patched failing-input capture is
required to validate any eventual correction. Saved evidence cannot turn green
merely because source code changes.

The signed16 boundary is now strongly supported by code and both live controls.
It is an internal combined-resource index, NOT the numeric animation ID or a
TAE byte-size limit. Remaining unknowns: complete consumer/fallback audit,
sentinel handling, exact corrected range, and independent event correctness.
No game writes, game calls, asset changes or DLL patching were performed.
The next step is confirmation of the staged Spec/Probe, then Technical Design.
