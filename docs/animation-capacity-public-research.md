# ER Animation Capacity: Public-Source Research

Date: 2026-09-25. Status: research only, before Spec/Probe confirmation.
Scope: public primary sources; no local EXE analysis, process access, DLL changes,
asset edits, or commits. Root AGENTS.md and project TASK_STATUS.md were read first;
no project-scoped AGENTS.md was found. This document is the only write.

## Decision Summary

**No verified, reproducible Elden Ring TAE/anibnd capacity patch was found in this
bounded investigation.** This is a search result, not proof that no such patch
exists. No supported maximum file size, animation count, TAE count, pool size,
patch address, original/replacement bytes, or 1.17.1 compatibility claim can be
offered from the inspected public evidence.

The useful evidence separates serialization, behavior selection, and runtime
capacity. It does not justify replacing the existing DLL's archive-switching
path with an assumed constant increase.

## Findings and Evidence Strength

### 1. Serialized counts are not runtime pool limits

**Strong for this tooling implementation; insufficient for engine limits.**
SoulsAssetPipeline reads the TAE-level ID and animation count with `ReadInt32`,
allocates a list using that count, and iterates over the entries. Its writer emits
`Animations.Count` as Int32 and a separate repeated count using its variable-width
integer API. It writes the TAE file-size header as Int32, casting the final
position. None of these operations establishes the game's accepted byte limit
or maximum simultaneously resident animation count. In particular, a 32-bit
serialized field is not evidence that the runtime accepts billions of entries.
[TAE.cs, immutable revision d11caf989c917c7a43e2c7559915b1c5af218153, lines 261-275,
332-341, 547-552, 698](https://github.com/Meowmaritus/SoulsAssetPipeline/blob/d11caf989c917c7a43e2c7559915b1c5af218153/SoulsAssetPipeline/Animation/TAE/TAE.cs#L261)

### 2. IDs and counts must be measured independently

**Strong for serialization; no runtime widening patch.** The animation object's
ID is `long`, read and written through `ReadVarint`/`WriteVarint`, separately from
its action and action-track counts. This does not establish the width of every
downstream game index or selector. Widening a tool-side ID would not by itself
widen engine lookup tables or behavior naming rules.
[TAE.Animation.cs, same immutable revision, lines 395, 426, 473-474, 549](https://github.com/Meowmaritus/SoulsAssetPipeline/blob/d11caf989c917c7a43e2c7559915b1c5af218153/SoulsAssetPipeline/Animation/TAE/TAE.Animation.cs#L395)

**Author-maintained mechanism documentation, not a capacity experiment.**
ERClipGeneratorTool describes `a[TaeId]_[AnimationId]` naming with three and six
digits respectively, and selection through `CustomManualSelectorGenerator`
using active TAE IDs. It also documents missing clip generators and HKS/table
coverage as reasons new animations fail to trigger or fall back. Thus failure
after adding animations can be a selection/configuration failure, not necessarily
exhaustion. The documented three-digit convention does not prove a universal
999-file or 999-animation capacity ceiling.
[ERClipGeneratorTool README, main as accessed 2026-09-25, behavior and
troubleshooting sections](https://github.com/The12thAvenger/ERClipGeneratorTool/blob/main/README.md#clip-generators-and-their-role-in-behavior-files)

### 3. Similarly named patches do not establish capacity expansion

**Strong exclusion for the documented feature.** er-patcher-2's
`--increase-animation-distance` addresses low-rate animation updates at screen
edges or at a distance. It is not documented as increasing loaded TAE/HKX entries,
archive sizes, or resource pools. Its version-support statement cannot be reused
as support evidence for a capacity patch.
[er-patcher-2 README, main as accessed 2026-09-25](https://github.com/helpme970/er-patcher-2#options)

## Patch Candidates

- **Ready-to-reproduce capacity patch: none.** No inspected source supplied a
  versioned patch site, a demonstrated original limit, replacement allocation or
  bounds logic, and an ER reproduction together.
- **Behavior-side corrective candidate, not expansion:** inspect/add matching
  clip generators and valid HKS selection when assets load but do not play.
  ERClipGeneratorTool provides the author-documented workflow above. This is only
  relevant if diagnosis identifies selection failure; it does not fix an actual
  loader crash or exhausted pool.
- **Runtime allocation/bounds changes: hypothesis only.** Do not publish a guessed
  address or remove a bounds check without identifying storage allocation and all
  consumers. No source-backed animation-specific hook was established here.

## What Still Needs Disambiguation

These are proposed diagnostic axes, not confirmed Spec requirements or executed
tests:

| Axis | Evidence to distinguish it |
| --- | --- |
| File bytes | Record compressed DCX bytes, decompressed binder bytes, and individual TAE/HKX bytes separately; vary payload size without changing IDs or entry counts. |
| Entry count | Record binder entries, TAE files, animation entries per TAE, and total HKX clips separately; vary count with valid small entries. |
| ID/lookup domain | Hold count fixed while changing the highest TAE/animation ID and sparsity; retain valid behavior/HKS mappings. |
| Allocation/pool | Compare resident resources and allocation failures across cold load, reload, and character instances; identify ownership and lifetime. |
| Archive discovery | Distinguish an archive never opened from one opened but rejected; a naming/slot restriction is not a size limit. |

Minimum missing inputs: a known-good/known-failing asset pair with hashes and
counts, the first failure phase and crash evidence, loader/mod configuration, and
a live process for the main agent's authorized read-only investigation. A precise
threshold and successful expansion remain unproven.

## Main-Agent Context, Not Public Evidence

The main agent reports current-EXE strings/RTTI: `AnibndFileCap` at RVA
`0x29d6828`, `AnibndRepositoryImp` at `0x2ba2880`, and `TaeDat` RTTI at
`0x3c66e58`; no game process is running. These values were supplied in the task
message and were **not independently verified by this sidecar**. They are lookup
anchors, not capacity constants, executable patch sites, or evidence that `Cap`
means capacity. Trace their references before assigning semantics.

## Search Boundary and Handoff

Web searches covered ER TAE/anibnd size, count, limit, patch, capacity and pool
terms. Primary material inspected included the pinned SoulsAssetPipeline files,
ERClipGeneratorTool documentation, and patcher documentation. A public me3 main
snapshot was also inspected in memory for targeted animation-capacity terms; it
did not yield a reproducible animation patch. That narrow negative scan is not a
complete audit of me3. GitHub API throttling and sparse search indexing limited
discovery; no exhaustive sweep or claim about private research is intended.

Next: main agent narrows the actual failing dimension using local evidence, then
asks the user to confirm semantic inputs, expected outputs, thresholds and manual
acceptance before Technical Design/Build. This note does not baseline a Spec,
authorize writes to the game, or supersede human final acceptance.
