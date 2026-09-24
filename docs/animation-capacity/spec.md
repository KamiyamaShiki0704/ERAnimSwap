---
id: FEAT-ERAS-Animation-Capacity
type: spec
version: 1.0
updated: 2026-09-25
source: human-raw
---

# Animation Capacity Expansion

User confirmed the proposed first-stage workflow with an explicit narrowing:
"first do the animation file separately" on 2026-09-25. This baseline is only
for oversized animation archives causing entry/load crashes. TAE late-content
failure is deferred. Test durations are acceptance settings, not engine limits.

## Intent

Replace the DLL's weapon-triggered animation archive swapping with support for
valid animation archives exceeding the game's demonstrated loading capacity.
The initial target is the user's PC App 1.17.1 mod environment and player
animation content. A finite verified capacity, not unlimited RAM, is the goal.

## Behavioral Requirements

- SPEC-ERAS-CAP-001: When this replacement DLL is enabled, it shall not detect
  weapons to copy/rename archives or trigger the old character hot-reload flow.
- SPEC-ERAS-CAP-002: Given valid content which reproducibly fails only because
  of a confirmed capacity restriction, when its resource requirements are
  within the agreed expanded budget, the DLL shall enable loading and correct
  playback of that content without truncation or substitution. Actual expanded
  capacity is to be measured and reported, not guessed before investigation.
- SPEC-ERAS-CAP-003: When the game build or required target cannot be validated,
  the DLL shall report incompatibility and leave game memory unmodified by
  the expansion feature. It shall not pretend successful expansion.
- SPEC-ERAS-CAP-004: When content exceeds the supported expanded budget or
  required resources cannot be acquired, the DLL shall not bypass bounds
  protection and report success. The tested rejection behavior must be stated.
- SPEC-ERAS-CAP-005: With ordinary previously working animation content, the
  replacement shall preserve loading, playback, transitions and TAE events.
- SPEC-ERAS-CAP-006: Removing the replacement DLL and restarting shall restore
  original behavior without permanently modifying the game EXE or archives.

## Semantic Probe Cases

Prerequisites: backed-up mod assets/save, agreed loader and DLL set, offline
mod-enabled environment with anti-cheat disabled, a candidate baseline asset set
A and reproducible failing set B to be established. Do not deliberately run a known-crashing case
against an important unsaved session. Record exactly which files differ.

| Case | Input | Expected Output |
| --- | --- | --- |
| CAP-P01 baseline | A, new DLL disabled/enabled | Both load and play the same selected actions |
| CAP-P02 demonstrated expansion | B, new DLL disabled/enabled | Baseline reproduces documented capacity failure; expanded run loads and plays the same B |
| CAP-P03 entry boundary | Valid animation archives below/at/above original entry-count threshold, comparable per-entry content | Result identifies a count boundary or disproves the count hypothesis; confirmed boundary succeeds within expanded budget |
| CAP-P04 byte boundary | Valid below/at/above original byte-size threshold, same entry count where possible | Result identifies a byte boundary or disproves the byte hypothesis; confirmed boundary succeeds within expanded budget |
| CAP-P05 isolation | Switch among selected weapons/actions under expanded run | No old archive replacement or hot-reload behavior; expected animations and events play |
| CAP-P06 incompatible target | Missing, duplicated or incompatible target fixture | Explicit refusal and no expansion writes |
| CAP-P07 restart | Remove new DLL and restart with A | Original behavior and unchanged on-disk EXE/assets |
| CAP-P08 lifecycle | B, repeated loading/travel and selected action sequences | No crash, missing animation or persistent animation loss |

Proposed observations: allow 120 seconds to load a character; perform 20
action/weapon transitions; perform 5 load/travel cycles; then observe 10 minutes.
Record process memory after each cycle. Any crash or wrong/missing selected
animation is a failure. Memory trend is diagnostic until a numerical budget
and an acceptable retained-memory allowance are confirmed. A static or unit
test pass cannot substitute for playback acceptance.

## Inputs and Pending Measurements

- Current input directory supplied by user: `F:\GoldenAge\_GoldenAge\GA\chr`.
  User authorizes constructing a larger animation archive because no failing
  pair is currently retained. Construct fixtures outside the active MOD folder.
  Paths/hashes for measured A/B and the exact failure point are still required.
- TAE late-content failure is explicitly outside this baseline.
- Target asset scale (file bytes, number of files/animations/events and largest
  used IDs as relevant); original boundary must be measured, not assumed.
- Expanded capacity target and practical RAM allowance once the cause is known.
- Confirmation received for the animation-file-only phase. A reproducible
  boundary and capacity target remain measurement outputs before a patch can
  claim CAP-002; missing evidence must not be converted into an invented limit.

## Exclusions and Human Acceptance

No automatic archive merging, corrupt-resource repair, arbitrary ID remapping,
anti-cheat bypass, online multiplayer guarantee, or universal unlimited memory.
Initial acceptance is single-player loading/playback of agreed assets. Seamless
Co-op and per-player resource ownership need separate runtime evidence; success
in single-player does not prove multiplayer support.

The user judges correct visible animations, their existing associated events,
transitions, and the final PASS/FAIL. Capacity-patch implementation remains
conditional on evidence of the actual failure; fixture preparation is allowed.
