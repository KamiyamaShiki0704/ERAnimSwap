---
id: FEAT-ERAS-TAE-Capacity
type: spec
version: 1.0
updated: 2026-09-25
source: human-confirmed
---

# Bounded Animation Capacity Expansion

Human confirmed the first-stage scope and Probe on2026-09-25 ("可以"). This is
separate from the already accepted large animation-archive loading correction.

## Inputs

Offline Elden Ring1.17.1 with save backup and the user's existing MOD setup.
Failing archive: F:/GoldenAge/_GoldenAge/GA/chr/1/c0000.anibnd.dcx.
Working archive: F:/GoldenAge/_GoldenAge/GA/chr/2/c0000.anibnd.dcx.
Fingerprints are recorded in a875-index-investigation.md. Both were observed
under experimental.2; the target fails with1 and works with2.

Target: a875_040000, triggered by Red Bear's Claw weapon68510000's skill.
The reported failure is T-pose with absent skill events, not just missing effects.

## Behavior

- SPEC-ERAS-TAE-001: With the supplied valid failing pack and corrected DLL,
  the target skill shall play completely and produce its expected skill
  effects/attack results, without reducing or splitting the supplied content.
- SPEC-ERAS-TAE-002: The existing1MiB loading correction, normal attacks,
  movement and the working control pack shall retain their behavior. Legacy
  weapon-triggered archive copying and hot reload shall remain disabled.
- SPEC-ERAS-TAE-003: Unsupported builds, conflicting patches or detected late
  loading shall be reported without applying the new correction. Removing
  the candidate and restoring experimental.2, then restarting, shall restore
  baseline behavior without persistent changes to the EXE or resource files.
- SPEC-ERAS-TAE-004: The first delivery shall address the supplied48059-slot
  case within a technically justified bounded range. At most65534 combined
  animation slots is the agreed first-stage candidate, not a per-TAE-file
  allowance or unlimited capacity. Report infeasibility before changing scope.

## Acceptance Probes

Fresh restart per package, identical remaining MOD assets. Allow120 seconds
to enter the scene; record timeout separately rather than declaring T-pose.

| Case | Input | Expected output |
| --- | --- | --- |
| TAE-P01 failing baseline | Failing1, experimental.2, target skill | Stable reported T-pose; already captured, not a required new test |
| TAE-P02 corrected target | Failing1, candidate, three skill releases | Complete motion, no T-pose, expected effects and attack results |
| TAE-P03 ordinary behavior | Failing1, candidate, three normal attacks and movement repetitions | Ordinary motion and effects remain correct |
| TAE-P04 working control | Working2, candidate, target and ordinary controls three times each | Previously working behavior remains correct |
| TAE-P05 refusal | Offline unsupported/conflicting/late-load fixtures | Explicit refusal and no successful correction claim |
| TAE-P06 rollback | Restore previous DLL and restart | Baseline behavior restored, resources unchanged |

The user judges motion/effect correctness against known working gameplay.
There is no invented numeric event-timing tolerance. Missing motion or skill
effects fails. Independent binding diagnostics support but do not replace human
acceptance. Boundary/guard/transaction tests are automatic implementation checks,
not proof that all gameplay or event content works.

## Exclusions

No automatic deployment, restarting, resource rewriting, EAC bypass, multiplayer
acceptance, unlimited animation guarantee, all-event-limit removal, or repairs
for malformed content/missing references. Manual startup loading only; no live
injection. Counts beyond the justified first-stage range remain unsupported.
Earlier event-reduction/ID-domain/resource-distribution experiments are optional
research, not permission to modify the user's assets. Final PASS/FAIL is human.
