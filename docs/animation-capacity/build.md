---
id: FEAT-ERAS-Animation-Capacity
type: build
version: 0.3
updated: 2026-09-25
based_on:
  - spec.md@1.0
  - design.md@1.2
---

# Experimental Capacity Build

## Follow-up Human Acceptance

After confirming scene load and character control, the user reported additional
testing with no apparent problems. Record provisional human acceptance for the
current input/environment and experimental.2. Keep the same binary and1MiB
budget; no further implementation change was made for this feedback.

Exact action/travel counts, observation duration, event coverage and memory
trend were not supplied. This is not a claim that every formal Probe case or
all larger archives passed. TAE capacity, other game builds and multiplayer
remain outside this acceptance. No Git publication or version promotion.

## Human Scene-load Result (Experimental.2)

User first reported normal game entry, then explicitly confirmed loading a
save into a scene and being able to control the character. This passes the
startup/character-load portion of CAP-P02, not its full playback acceptance.
Experimental.2 runtime log (05:30:28 local) records CAPACITY_PATCH_APPLIED and
131072->1048576 bytes. A private snapshot is retained at
target/animation-capacity-validation/20260925-exp2-load/runtime.log.

The currently installed DLC01 archive was rehashed and still matches the
original failing input B535BD3AB4D3DB1284ABBEF9CD5E77E51D0E1A963FB3156DF84EAFEAE604DCDD.
No game process existed when the agent inspected, so no post-patch live arena
measurement was performed. This combines installation log, configured input
fingerprint and human scene-load confirmation; it is not proof of every clip
having been opened or every animation/event being correct.

Keep1MiB budget unchanged. New/selected animation playback and associated
events,20 transitions,5 loading/travel cycles and10-minute observation remain
human acceptance items. No new code, MOD writes or Git publication this stage.
Earlier pending-game-load statements below describe the pre-acceptance stage.

## 0.3.0-experimental.2 Startup Regression Fix

Experimental.1 FAILED the user's startup test: a64KiB frame was reserved in
DllMain before the notification reason check. Two WER dumps and an independent
native small-stack-thread repro establish the fault in our DLL. See
[the startup report](startup-crash-20260925.md). Earlier offline passes below
did not cover this loader/thread interaction and are not runtime acceptance.

Experimental.2 removes the host-path buffer and prevents inlining process
initialization into the DLL notification gate. Budget1MiB/patch byte unchanged.
The exact previous DLL fails with0xC00000FD; the new delivered DLL passes100
native64KiB-stack thread creation/exit cycles. Release7/7 existing tests with
actual EXE configured, clippy and release build pass. Separate delivery and
offline profile prepared; human game loading/playback remains pending.

## 0.3.0-experimental.1 Evidence

Based on spec1.0/design1.1, project base827f262c48460d70c9b9c8e3e7365a24e78df19a
plus this working tree; KB9511e3e8137fd2cda9fca74bb07a4b688fe89087.
The older preparation notes below are chronological evidence, not the current
implementation verdict. TASK_STATUS.md remains the authoritative task state.

Implemented source: src/capacity/{lib,patch,tests}.rs. Cargo's lib entry selects
only this implementation; legacy swapping source remains uncompiled. A startup
single-byte change raises the proven general EzWork temporary budget128KiB to
1MiB. No allocator hook, file copying or hot reload. Runtime PE and code-chain
guards refuse unknown/conflicting targets before writes. Fixed budget, no old
TOML/delay. Restart required; no retrofit of existing workers.

Automatic results:

- Debug and release cargo test --locked --offline: **7 passed** with
  ER_GAME_EXE set to the audited2.7.1.0 EXE. No game checks skipped.
- Full EXE SHA256 and every guarded instruction match; altered PE/header/
  downstream instructions, duplicate signature and higher third-party budget
  fixtures refuse without changing their budget.
- Real constructor instruction in independent executable memory returns128KiB
  before and1MiB after, exactly one differing byte; RX protection restored.
- Read-only SEC_IMAGE mapping of the actual EXE in the test process passes the
  complete production apply path. Mapping is copy-on-write; game entry never
  runs. Exactly one window byte changes, repeat apply refuses, disk SHA256
  unchanged after unmapping. No running-game process is involved.
- Native allocator replay reproduces the captured failure exactly at128KiB;
  512KiB and1MiB pass5669 simulated entries with values preserved.
- cargo check, clippy --all-targets -- -D warnings, release build passed.
- cargo fmt --all -- --check and git diff --check passed; local task notes,
  binary delivery, dumps and fixture assets remain ignored by Git.
- LoadLibrary smoke in Python (not eldenring) refuses the host and logs no
  memory writes; worker logger exits normally.

Delivery: target/delivery-0.3.0-experimental.1/weapon_animation_hotreload.dll
(252416 bytes). SHA256:
`4BCA92674C047DF9C86360F4FBA7BAF7AB6FC847D2F2A194F018B0B9B213CFCD`.
The original0.2.1 delivery DLL remains intact. No MOD DLL/archive or existing
profile was replaced. No Git commit/push/release performed.

Private local test profile/launcher: same delivery directory,
Test_AnimationCapacity.me3 and Test_AnimationCapacity.bat. Original DLL order,
packages and ER0000.Shiki save selection preserved; new capacity DLL prepended
and offline forced. All paths checked. Launcher dry-run passed; it did not
launch a game. It preserves the existing offline launcher's no-mem-patch and
other flags, without adding a separate general-allocator modification.

| Requirement | Automatic Evidence | Remaining Human / Runtime Evidence |
| --- | --- | --- |
| CAP-001 | New compiled entry has no weapon/copy/reload flow | P05 visible isolation |
| CAP-002 | Captured allocation red/green;1MiB budget instruction executes | P02 actual same archive load and playback |
| CAP-003 | Identity/guard/duplicate/conflict tests; actual image mapping | Report any live loader-time refusal |
| CAP-004 | Existing engine checks untouched; no success on failed patch APIs | Beyond1MiB retains engine failure, no graceful recovery claim |
| CAP-005 | Engine logic/assets unmodified except budget byte | P01/P05 ordinary playback, transitions/events |
| CAP-006 | Actual mapped EXE disk hash unchanged; no archive writes | P07 removing DLL/restarting with known-working archive |

P03 count evidence is the captured5395->8092 vector step, not a universal clip
limit. P04 byte-size ceiling has NOT been established. P08 loading/travel and
memory trend are untested. TAE late-content failure and multiplayer are out of
scope. Full patch timing before worker creation needs the controlled startup
test. No complete CAP-002 or human PASS claim is made.

## Earlier Diagnostic Preparation

Project base: 827f262c48460d70c9b9c8e3e7365a24e78df19a plus this diagnostic
working tree. KB root rules read from D:/_Ugit/FSTool/_Mod/AGENTS.md.
Only diagnostics and isolated fixtures implemented. No source DLL behavior,
game memory or active MOD asset was changed. No new release is justified yet.

## Automated Evidence

The three scripts listed in design.md pass PowerShell syntax parsing. Negative
checks executed successfully: fixture output already exists; planned payload
exceeds its budget; memory range exceeds the main module; crash capture would
attach to a still-running baseline game. The last check aborts before invoking
ProcDump. ValidateOnly checks the archive fingerprint and tool signature without
attaching or arming anything.

Fixture invocation used the explicitly supplied current DLC01 archive, the
existing reference SoulsFormats.dll, installed oo2core_6_win64.dll, multiplier 2.
It produced:

- `target/animation-capacity-fixtures/8D143FE8DFC1-2x/c0000_dlc01.anibnd.dcx`
- 6,623 binder entries / 6,620 HKX clips.
- 148,125,728 disk bytes; 348,799,656 combined payload bytes.
- SHA256 `2F6F52543FA468A5B4DC70050EE0136AABA731934AEEF8D4814C09572C253ACD`.
- Adjacent manifest.json records source/tool fingerprints and copied entry IDs.
- Every payload, name, ID and flags were checked after reopening; original
  source hash remained unchanged. Not installed or tested in-game.

The user subsequently reported creating their own new animation package. Prefer
that input once its path is provided; no 4x fixture was generated.

## Live Read Evidence

Baseline process PID 36520, module base 0x7FF687730000, executable 2.7.1.0.
Read-only snapshots:

- `target/animation-capacity-memory/20260924-202056-677-36520/manifest.json`
- `target/animation-capacity-memory/20260924-202200-618-36520/manifest.json`
- `target/animation-capacity-memory/20260924-202250-156-36520/manifest.json`

Global RVA 0x3D7F618 had value 0x1A500831AC0. The sampled object contains ANIBND
as its name. At holder offset +0x78, bucket count is 2039 and bucket pointer is
non-null. A chained hash table's bucket count does not limit resource count.
These snapshots are not a synchronized view of concurrent loader state and do
not establish what archive was opened. See loader-code-research.md for carefully
qualified dynamic-allocation leads; no fixed size cap was proven.

## Crash Loop Preparation

Full-dump capture script is prepared and ValidateOnly passed, but no monitor is
armed yet. ProcDump was obtained from the Microsoft Sysinternals download link
and its Authenticode signature validated as Microsoft Corporation. SHA256:
`D1FC99AE304BD1D2BF28ABEB62531DA959E2431916194981B88C958FD713A8E6`.

Official semantics: `-e` catches unhandled exceptions, `-b` includes debug
breakpoints, `-ma` requests full memory, `-w` waits for launch and `-n 1` stops
after one dump. No `-i` installation or `-k` process-kill option is used.
[Microsoft ProcDump documentation](https://learn.microsoft.com/en-us/sysinternals/downloads/procdump).

Three existing recent Windows dumps have EXCEPTION_BREAKPOINT at EXE RVA
0xC58DB6. They are from earlier PIDs 15076, 44536 and 44908, with no established
relationship to the newly constructed archive. They are not evidence for a
capacity patch. A new capture must be paired with the exact tested archive.

## Spec Coverage and Remaining Work

### User-supplied Crash Input and Armed Capture

The user replaced MOD chr/c0000_dlc01.anibnd.dcx with a new archive:
107,701,856 disk bytes; 5,672 entries including 5,669 HKX; 277,382,680 combined
payload bytes. External BND4 parsing succeeds, with no duplicate IDs or names.
SHA256: `B535BD3AB4D3DB1284ABBEF9CD5E77E51D0E1A963FB3156DF84EAFEAE604DCDD`.
This does not certify HKX semantics or runtime acceptance.

After verifying no eldenring process remained, the one-shot capture script was
started in a hidden PowerShell host (28524), with ProcDump PID 44212. Its log
confirms waiting for eldenring.exe. Capture folder:
`target/animation-capacity-crashes/20260924-203239-017/`.
The user was notified to start through the existing offline MOD launcher and
allow the dump to finish after a crash. This supersedes the earlier unarmed
preparation note; no fresh crash evidence has yet been observed at this point.

CAP-001/002/003/004/005/006: replacement DLL not implemented; no PASS claim.
CAP-P01 through CAP-P08: full runtime acceptance not executed. Fixture round-trip
and bounded reads only prepare those cases; they are not the crash regression.
No deterministic red-capable reproduction command exists until the user's test
input, loader invocation and fresh failure are tied together. TAE expansion is
deferred by user instruction. Human final acceptance remains outstanding.
