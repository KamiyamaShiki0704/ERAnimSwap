# Read-only Failing-pack Baseline

## Environment

User entered a scene with the failing pack and authorized read-only inspection.
PID51344, startup2026-09-25T17:29:04.440006+08:00, base0x7FF75D8A0000.
EXE2.7.1.0, timestamp0x6A96B418, image size0x5E0DA00; SHA256
1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891.
Active disk c0000 archive hash equals supplied failing1:
E7E3F7D53A6EB396B9BD8894A2015DDCF5A740FD7017B54A8AE74C945CC870E2.

Important: no weapon_animation_hotreload.dll appears in the module list.
RVA0xE83E18 reads `48 c7 44 24 28 00 00 02 00`, the original128KiB budget.
This is an **unpatched baseline**, not experimental.2-active verification.
The user's earlier report of failure with experimental.2 is not contradicted;
it is simply not the configuration observed in this session.

## Method and Boundaries

Only OpenProcess query/read access and ReadProcessMemory. No process writes,
game calls, injected threads, suspension, privilege changes or heap sweep.
No game resource replacement, launch or restart. Generated private evidence
is under ignored target/tae-memory/51344-baseline-1.

Read-TaeRepository.py validates known FD4 ownership and bounded hash chains:
130 resource capsules,2039 buckets. Bucket count is NOT a capacity limit.
The c0000 object chain is:

| Object | Address | Field to next object |
| --- | --- | --- |
| AnibndResCap | 0x27DD9BC0760 | +0x78 |
| CSLoadProcessData_Anibnd | 0x27DD74160C0 | +0x60 |
| HvkAnim | 0x27EE4E80080 | +0xA8 |
| TaeDat | 0x27DF4065080 | Indexed TAE pointers |

Every object type was checked by RTTI. Session addresses must not be reused
after a restart. Early broad object captures include adjacent allocation bytes;
do not interpret bytes beyond code-established object bounds as fields.

Bounded live disassembly establishes the interpretation:

- HvkAnim constructor/loader0x1B5850 allocates TaeDat0x4260 bytes and stores it
  at+0xA8. TaeDat constructor0x1B60D0 calls loader0x1B64A0.
-0x1B6568 compares file index against999 (`0x3E7`), with unsigned >= skipped.
  This is a file-index gate, not an action-count limit.
-0x1B65D8 stores payload pointer at TaeDat+8+index*8.
-0x1B6677 stores the corresponding wrapper at TaeDat+0x1F40+index*8.
- TAE header+0x54 count/+0x58 relocated action-table pointer were checked
  against supplied inventory. Table records have16-byte stride and64-bit IDs.
  Header lengths and table intervals were checked before reading.

Compare-LoadedTae.py validates ownership/RTTI, reads accepted slots, compares
each resident header length and ordered ID list to parsed disk metadata, then
rereads headers/registry/ownership to reject observed mutation. These checks
do not make the process snapshot atomic. Each comparison read943763 bytes.

## Result

Two captures at09:39:43Z and09:40:05Z have identical comparison records:

| Check | Result |
| --- | --- |
| Supplied TAE files | 780 |
| Accepted slots matched by byte length and ordered IDs | 779 |
| Matched resident action records | 44212 |
| Missing accepted slots | 0 |
| Extra resident slots | 0 |
| Null wrappers for matched slots | 0 |
| Outside accepted file-index range | a999.tae,3 records |

Both supplied packs include a999.tae with action IDs0,100,4000. Their payload
hashes differ, but the presence of this out-of-range file is not unique to the
failing pack. Do not change the bound or claim it explains late-content failure.
In particular, increasing a comparison without resizing both indexed tables
would be unsafe.

This rules out broad missing/truncated resident TAE action tables in this
snapshot. It does NOT prove runtime lookup reaches every record, imported
actions resolve, events dispatch, HKX binding works, or character playback is
correct. Event payloads and lookup execution were not verified. No reproduced
playback failure, root cause or justified new capacity patch exists yet.

## Evidence and Next Step

- target/tae-memory/51344-baseline-1/repository.json
- target/tae-memory/51344-baseline-1/tae-comparison.json
- target/tae-memory/51344-baseline-1/tae-comparison-repeat.json
- Same folder code-1b5850.txt,code-1b60d0.txt,code-1b64a0.txt and bounded
  parser/capsule snapshots. No proprietary evidence is to be published.

Reproduction entry for this historical process only:

```powershell
D:\Python\python.exe -B Scripts/delivery/feat-tae-capacity/script/Compare-LoadedTae.py 51344 0x7FF75D8A0000 0x27dd9bc0760 target/tae-capacity-inventory/20260925-pair/1.json target/tae-memory/51344-baseline-1/new-comparison.json
```

Next obtain same failing assets under verified experimental.2 startup, then a
matched working2 session. Reacquire every process/object identity each time.
Preserve the1MiB patch unchanged and continue downstream lookup/import diagnosis
only with evidence. TAE Spec0.1 remains unconfirmed for patch implementation;
final playback acceptance remains human.
