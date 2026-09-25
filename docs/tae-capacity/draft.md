# TAE Investigation Input Notes

## Latest Runtime Evidence

The historical no-process notes below are superseded by the
[read-only failing-pack baseline](live-baseline-20260925.md).779 accepted TAE
files and44212 ordered action IDs match their resident tables on two reads.
The remaining a999/3 records fall outside this loader's observed ID range.
Current session lacks experimental.2; matched patched/working sessions and
runtime playback/lookup evidence are still pending. No root cause or patch
is established by resident metadata equality.

## User Report

More event content and more animation/action IDs can make later TAE content
ineffective. Action-entry count reportedly dominates: deleting earlier events
does not restore later content once the entry count exceeds an unknown value.
User further clarifies that the animation itself stops playing (T-pose), and
events also stop. This is not merely a missing visible event on a playing clip.
This does not yet identify a hard count limit, ID domain, ordering rule, lookup
failure or a runtime allocation limit. No candidate address or patch selected.

## Read-only Inventory (2026-09-25)

Current inspected folder: F:/GoldenAge/_GoldenAge/GA/chr.
F:/GoldenAge/GA/chr did not exist at inspection. The newly added workspace root
alone is not proof that it contains or supplies the active MOD.

| File | Disk bytes | SHA256 |
| --- | --- | --- |
| c0000.anibnd.dcx | 18769322 | B0797EF5FE1CE73229BA15EE6845F272ABC0F78649655E41E5A53ABFDC954F25 |
| c0000_dlc01.anibnd.dcx | 74065696 | 8D143FE8DFC1E3A11C5BD0DE601D52757D577B383A6C73E753D415D7D43216CB |

The first fingerprint matches the previous externally parsed inventory in
[animation-capacity draft](../animation-capacity/draft.md):730 TAE files,
23872 action entries,971349 events,112087520 combined TAE bytes. Largest file
a00.tae:1960 actions,63216 events,7693168 bytes. These are reused same-hash
parser measurements, not new game-memory counts or a demonstrated ceiling.

DLC01 now matches the old74065696-byte input, not the107701856-byte expanded
package used for the accepted animation-loading fix. Do not attribute this
change to the DLL or invalidate the retained prior acceptance evidence.
The agent made no MOD writes. The user subsequently supplied the pair below.

## User-supplied Pair: Read-only Parse

- Failing: F:/GoldenAge/_GoldenAge/GA/chr/1/c0000.anibnd.dcx.
  SHA256 E7E3F7D53A6EB396B9BD8894A2015DDCF5A740FD7017B54A8AE74C945CC870E2.
- Working: F:/GoldenAge/_GoldenAge/GA/chr/2/c0000.anibnd.dcx.
  SHA256 B0797EF5FE1CE73229BA15EE6845F272ABC0F78649655E41E5A53ABFDC954F25.
- SoulsFormats parsing used the same local reference assembly as the previous
  inventory. Source hashes checked before and after inspection and unchanged.
- Detailed generated inventories: target/tae-capacity-inventory/20260925-pair/
  1.json and2.json (ignored). Includes per-file hashes, action IDs/event counts
  and miniheader kinds, but no copied proprietary resource bytes.

| Quantity | Working2 | Failing1 |
| --- | ---: | ---: |
| Disk bytes | 18769322 | 24640291 |
| Binder entries | 745 | 795 |
| TAE files | 730 | 780 |
| Sum of TAE bytes | 112087520 | 147261648 |
| Action records | 23872 | 44215 |
| Event records | 971349 | 1278275 |
| Actions without events | 3827 | 18564 |
| ImportOtherAnim miniheaders | 4721 | 17734 |
| Standard miniheaders | 19151 | 26481 |
| Extra within-file duplicate action IDs | 0 | 6 |
| Descending action IDs | 0 | 0 |
| Parser failures | 0 | 0 |

Action counts include reference/import entries and are NOT a count of distinct
HKX files or simultaneously playing animations. Both archives have no duplicate
binder IDs/names. Successful external parsing does not establish runtime validity.

This is not a one-variable comparison. There are677 common TAE filenames,103
only in failing and53 only in working. None of the677 common payload hashes
match; differences may include metadata, not necessarily changed behavior.
Skeleton.hkx differs too:472956 failing bytes vs91868 working bytes. These
are confounders to control, not proven causes of the reported failure.

Duplicate action IDs (each appears twice in its own TAE):
a02/99210, a279/599100, a888/13250, a951/7021, a953/7021, a955/7021.
Do not delete or rewrite these without establishing expected references and
whether they overlap the failing action. They do not alone explain a general
late-content boundary.

Largest action count remains a00.tae:1960 working vs2131 failing. Global and
per-file counts both changed. No numerical engine limit follows from two such
different packs. An exact observable action/control is still required.

## Missing Reproduction Inputs

Update: user confirms the failure persists with experimental.2 loaded, identifies
it as separate from the archive-size crash, and cannot currently give a specific
action. Requested direction is read-only process-memory diagnosis. Exact action
is deferred for playback acceptance; do not repeatedly demand it before resource
inventory. No game process existed at inspection. The active root archive still
hashes to working2, not failing1. User was asked to load failing1 offline with
experimental.2, enter a scene and report readiness without switching resources.

Runtime comparison must first identify the actual loaded resources and account
for lazy loading, imported/shared actions, lifetime and completion. An apparent
missing resource or a smaller raw object count alone does not prove truncation.
No in-game memory bytes have yet been read for this TAE phase.

- Pair paths and experimental.2 use are supplied; retain identical other assets
  and loader settings when collecting the normal/failing runtime comparison.
- Whether failure starts within a TAE or at later TAE files.
- One specific failing file, full action ID (or weapon/input sequence), expected
  output and one earlier control action. T-pose plus event failure is confirmed
  as a user report, not yet reproduced by this task.
- Approximate threshold and whether the report still holds with experimental.2
  loaded; initial comparisons must keep its1MiB animation worker budget fixed.

The normal/failing runtime pair and semantic probe confirmation are pending.
Until then no red-capable TAE runtime regression exists. Inventory alone is not
enough to justify capacity writes or low-level reverse-engineering conclusions.
