---
id: FEAT-ERAS-TAE-Capacity
type: build
version: 1.0
updated: 2026-09-25
based_on: [spec.md@1.0, design.md@1.0]
---

# Experimental.3 Build

Project base revision3eecf103b3a17840f886cd05e65d530a2833edcd plus current
uncommitted changes; KB revision9511e3e8137fd2cda9fca74bb07a4b688fe89087.
No commit/push or game deployment was requested or performed in this phase.

## Implementation

src/capacity/clip_index.rs defines four exact in-place instruction replacements.
patch.rs validates all originals and consumer/sentinel guards, refuses known
late-world initialization and installs them with the existing1MiB budget in a
five-edit/four-page startup transaction. Failed operations attempt rollback.
Shared-page protection restoration is deduplicated. No trampolines, allocations,
game-object changes or file I/O in installation; logging remains on its worker.

Version0.3.0-experimental.3. FSRS remains pinned to902851865d05069eda7dcfd6385c8881eadf29bf.
README retains the user's concise format; only current function/range notes
were added to the preexisting simplification. Task notes remain ignored.

## Automatic Results

- RED: original native activation32768->-32768; original name default becomes
  a negative64-bit value. Original35781->-29755 matches live failing evidence.
- GREEN: all65536 raw words through activation and graph selection, each with
  and without remap (262144 checks); both reserved states preserved.
- GREEN: all65534 supported name defaults/equality; reserved values do not
  compare equal to supported nonnegative insertion indices.
- GREEN: exact original teardown instructions, with only external calls replaced
  by counters, preserve -1/-2 cleanup and high-index normal cleanup behavior.
- GREEN: nine transaction API failure positions recover original bytes and RX
  permissions; repeated/mixed/conflicting installation refused.
- GREEN: actual SHA256-pinned EXE guards and private SEC_IMAGE installation;
  entire text comparison permits only intended byte changes, disk hash unchanged.
- GREEN:13 Rust tests with ER_GAME_EXE set; no EXE checks skipped in final run.
- GREEN: clippy all-targets with warnings denied; optimized release build.
- GREEN: formatting/diff checks; six Python diagnostic scripts parse; static
  scan found no external direct rel32/decoded rel8 branch into patch interiors.
  This scan does not prove absence of all indirect control flow.
- GREEN: isolated DLL loading plus100 native threads with64KiB reserved stacks,
  exit0. Evidence target/capacity-small-stack-tests/run-a3iilaft.

Commands: cargo test --locked -- --nocapture with ER_GAME_EXE pointing to the
audited installed EXE; cargo clippy --locked --all-targets -- -D warnings;
cargo build --locked --release; Test-DllSmallStack.py with --threads 100.
Read-only investigation scripts and local evidence are described in the target
investigation report. UE FrameRecorder is inapplicable to this native DLL.

## Delivery and Manual Items

### DLL Output Rename

User requested the output filename CapacityExpansion.dll and source commit/push.
Cargo now explicitly names its library CapacityExpansion; package identity and
version0.3.0-experimental.3 remain unchanged. The crate naming lint is allowed
to retain this exact Windows loader-facing filename. Patch behavior is unchanged.
Current README installation/build paths use the new name; historical filenames
and hashes below remain evidence of the earlier human-tested artifact.

Renamed build SHA256:
`991CDE348B8F86A700804EAAE7101F1ECA47C565DA420EA4E2B41CCC93A2C708`.
Delivery: target/delivery-0.3.0-experimental.3-CapacityExpansion/CapacityExpansion.dll.
Re-ran13tests with actual EXE checks enabled, clippy with warnings denied,
release build and100x64KiB-stack threads (run-0ruttyio): all pass. No automatic
replacement of the active MOD DLL. Update the startup loader entry on upgrade;
do not load the old and new filenames together. Log filename remains unchanged.

### Human Feedback Update

User reports "看起来可以正常使用了" after installing the candidate. Read-only
verification finds the active CapacityExpansion DLL matches the candidate hash;
its log identifies0.3.0-experimental.3 and both patch-applied markers. Active
disk c0000.anibnd.dcx matches original failing1 hashE7E3F7D53A6EB396B9BD8894A2015DDCF5A740FD7017B54A8AE74C945CC870E2.
This supports a positive human smoke-test result for the supplied failing pack.
No fresh process-memory capture was taken, and on-disk identity is not itself
proof of resident resource identity. Exact repetitions, individual effects,
working2 regression and rollback were not separately reported. Do not broaden
this feedback into full-range, long-session or multiplayer acceptance.
The pending notes below remain applicable to those unreported cases only.

Candidate DLL SHA256:
`E1F1F50BB5FB504ED39B7A9AE62A0CD50B570C55DF65241228FCBDB9024112BC`.
Isolated delivery: target/delivery-0.3.0-experimental.3/weapon_animation_hotreload.dll.
Size254464 bytes; copied delivery hash matches release output. SHA256SUMS.txt
is included. Both delivery and local task/plan files remain Git-ignored.
Active MOD DLL remains experimental.2 hashE406E76B96FD230760F7C1D52A350112253065CFFB3023987AA35707D7BFAD57.

SPEC001/002 automatic instruction tests pass; TAE-P02/P03/P04 motion/effects
and load acceptance remain human pending. SPEC003 automatic refusal/rollback
checks pass; manual startup/uninstall behavior TAE-P06 pending. SPEC004 tested
instruction domain is bounded; full65534-slot game-content acceptance is not
claimed. No global admission controller beyond that range was added.

No fresh candidate game sample exists yet. User must close the game, replace
only the prior capacity DLL or change their startup loader path, select failing1
and restart. Never load both DLLs. First verify version and applied/refused log,
then reacquire process-specific addresses before the next read-only Probe.
