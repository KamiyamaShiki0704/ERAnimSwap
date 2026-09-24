---
id: FEAT-ERAS-Animation-Capacity
type: draft
version: 0.1
updated: 2026-09-25
---

# Local Investigation

## Evidence Identity

- Project base: `827f262c48460d70c9b9c8e3e7365a24e78df19a`.
- Inspected EXE: `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe`.
- File version: 2.7.1.0; size: 87,042,128 bytes.
- SHA256: `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`.
- No `eldenring` process found on 2026-09-25; no live memory reads or writes
  performed. Existing DLL, source and active mod assets are unchanged.

## Static Leads, Not Patch Addresses

Read-only PE string inspection using pefile found:

| RVA | String / type |
| --- | --- |
| 0x29D6828 | AnibndFileCap |
| 0x29D6B90 | Source/File/FileCap/AnibndFileCap.cpp (original string uses backslashes) |
| 0x29E27A8 | CSLoadProcessor_Anibnd |
| 0x2BA2880 | AnibndRepositoryImp |
| 0x3C66E58 | TaeDat RTTI name |
| 0x2A2FD88 | chranibnd:/c0000_dlc01.anibnd (UTF-16) |
| 0x2BED180 | LogCheck.AnibndInfo (UTF-16) |

Decoded code at 0x7E5F0 references AnibndFileCap's name and registration
metadata; 0xAC6B0 similarly references AnibndRepositoryImp. These are leads
for resource investigation, not discovered capacity checks. Nearby raw LEA
matches for the source-file string at 0x20084E and 0x20086C were not validated
from established function boundaries and must not be used as patch locations.

## Reference Source

- `../../_Example/fromsoftware-rs/fromsoftware-rs-0.14.0/crates/eldenring/src/cs/chr_ins/module/time_act.rs`:
  CSChrTimeActModule has a 10-element playback circular queue with read/write
  indices. This is not evidence for a 10-animation asset loading limit.
- `../../_Example/libER-main/include/coresystem/file/resource_repositories.inl`:
  lists separate AnibndRepository, HkxRepository and other resource categories.
- `../../_Example/libER-main/include/coresystem/file/file.hpp` and
  `include/fd4/resource.hpp` in that reference: file capsules, resource requests
  and repository structures exist. They do not document a verified TAE byte
  cap or a ready-to-apply expansion patch.

Reference paths above are relative to the selected project root, not this file.
The reference directories were read-only. The pinned Cargo dependency was not
modified. See also [public-source research](../animation-capacity-public-research.md).

## Competing Explanations

File bytes before/after decompression, binder entry count, animation/event
count, ID range, resource allocation budget and malformed/duplicate content
remain distinct possibilities. No measured original limit or patchable
comparison/allocation chain has yet been established. Do not NOP bounds checks
or increase presumed capacities without tracing corresponding storage/users.

Next useful evidence is the same valid asset loading below a reproducible
boundary and failing above it, with the failure stage and a running process
or crash trace. The expansion cannot be honestly implemented from the type
names alone.

## User Clarification and Current Assets

User describes two separate failures: earlier TAE content becoming too large
causes later TAE content to stop working; an oversized animation archive crashes
while entering the game. No current known-failing pair is retained. The user
allows constructing a larger test archive. These reports guide hypotheses but
do not yet establish a threshold or causality.

Current MOD root: `F:\GoldenAge\_GoldenAge\GA\chr`. Read-only inspection used
the existing SoulsFormats assembly at
`../../_Example/ER_HKX2Navmesh-main/Resources/SoulsFormats.dll`; KRAK decoding
used the game's installed `oo2core_6_win64.dll` in the inspection process only.
No data was written to the MOD directory and no assets were copied into Git.

| Archive | Disk bytes | Binder entries | Parsed content |
| --- | --- | --- | --- |
| c0000.anibnd.dcx | 18,769,322 | 745 | 730 TAE, 1 HKX, 14 TXT |
| c0000_dlc01.anibnd.dcx | 74,065,696 | 3,313 | 3,310 HKX, 1 compendium, 2 TXT |

The first archive uses DFLT; the second uses KRAK. All 730 TAE files parsed
without an exception in this tooling. Totals: 112,087,520 TAE bytes, 23,872
animation entries, 971,349 event entries. Largest TAE is `a00.tae` with
7,693,168 bytes, 1,960 animations and 63,216 events. The second archive's
combined binder payload is 174,404,648 bytes. Payload sums exclude binder
headers, names and alignment and are not equivalent to total runtime memory.

- c0000.anibnd.dcx SHA256:
  `B0797EF5FE1CE73229BA15EE6845F272ABC0F78649655E41E5A53ABFDC954F25`.
- c0000_dlc01.anibnd.dcx SHA256:
  `8D143FE8DFC1E3A11C5BD0DE601D52757D577B383A6C73E753D415D7D43216CB`.

Parser success is not proof of valid runtime playback or correct behavior/TAE
selection. No current asset has been labeled a runtime PASS or FAIL. Candidate
fixtures must vary one axis at a time, maintain valid IDs/references, and keep
a sentinel late TAE/action unchanged while increasing earlier content. Blind
padding or duplicate IDs would not be a useful capacity reproduction.
