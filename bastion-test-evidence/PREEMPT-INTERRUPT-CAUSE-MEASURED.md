# The interrupt cause, measured: a RestAt travel does genuinely churn (seed 7), and one plausible mechanism is refuted

Follow-on to `PREEMPT-SCENARIO-COOLDOWN-DEFECT.md`, per Opus's sequencing
("the interrupt cause is chased before Step 2"). That doc *named* the churn
class as the likely mechanism by inference ("travel fails — the same
claim-release churn class documented elsewhere in this campaign") without a
cited instance. This is that instance, plus one hypothesis raised and
refuted along the way.

## The measured instance

7-seed sweep of `preempt_scenario` (`BASTION_NEED_SKIP_DIAG=1`), counting
`need preempt` / `job unreachable — claim released` / `slept — rest
restored` lines:

| seed | preempt attempts | churned | slept |
|---|---|---|---|
| 7  | 2 | 2* | 1 |
| 12 | 2 | 0 | 1 |
| 19 | 3 | 0 | 1 |
| 42 | 1 | 0 | 1 |

*Seed 7's 2 churns include 1 REST-job churn and 1 (later, unrelated)
mine-job churn — see below; not both rest.

Seed 7, re-run with `BASTION_LEGC_DIAG=1` (the existing decisive-line
diagnostic at timeout firing) isolates the rest-job churn precisely:

```
need preempt — rest below interrupt  bed=(21872, 16025, 250)
...
LEGC-DIAG: travel timeout firing  job=33  job_pos=(21872,16025,250)
  actual_pos=(21868.451, 16003.426, 250.0)  steer=target=(21872.5,16025.5,251.0)
  sdist=22.46  stuck_time=10.000019  drive=Some(Work)  auton_travel_ok=true
job unreachable — claim released  job=33  pos=(21872,16025,250)  colonist=(21868,16003,250)
```

`steer`/`target` are correct (the bed, offset for stand-in-front-of).
`actual_pos` after the full 10s `STUCK_TIMEOUT` window is 22.46 units away
— *farther* than the ~13.6-unit gap present when the attempt started, and
displaced almost entirely in y, in the WRONG direction (target needs
y:+10, the colonist ends up at y:-12 from its start). z climbed correctly
toward the target's altitude (243→250, target 251) — only the lateral
approach failed. This is a real, non-trivial travel failure, not a
labeling artifact: the churn line's own reported colonist position matches
the LEGC-DIAG line's `actual_pos` exactly, and the bed's own coordinates
match the immediately-preceding `need preempt` line's `bed=` field.

## One hypothesis raised and refuted

The `drive=Some(Work)` field in the timeout log looked, at first read, like
the smoking gun: the colonist's arbiter never got told it switched to a
need-job, so maybe something downstream was still steering it toward Work
behavior instead of the bed. **Checked directly against the code
(`bastion_jobs.rs` ~11242-11268) and refuted:** `auton_travel_ok` is
computed as `matches!(job.kind, RestAt | EatFrom | Despond) || (drive ==
Work)` — self-jobs are UNCONDITIONALLY exempt from the Work-drive gate ("self-
job travel fires UNGATED," the code's own comment, B7's authority). The
`Goto(steer, speed)` movement command *was* correctly issued every tick
this ran. `drive` in the log is read for diagnostic display only here; it
does not gate or redirect this colonist's movement. Wrong mechanism,
checked and dropped rather than asserted.

## What's still open

Why the colonist's actual path diverged from a straight line to the bed —
gained altitude correctly, moved laterally the wrong way — is not yet
isolated to a cause. Two live candidates, neither confirmed:

1. A genuine terrain obstruction/detour specific to this bed's site (same
   shape as the seed-1337/seed-92 corner-cell investigations — would need
   the same site-survey treatment: a block-kind dump of the columns
   between the colonist's start position and the bed).
2. A chaser retry/oscillation pattern — this same function's own earlier
   comments (the "MIRAGE anchor"/`staged_at_anchor` grace-window logic,
   ~11372-11444) already document that stalls-while-pathing-toward-a-real-
   target are a known, handled shape for OTHER job kinds; whether the same
   applies here (and whether RestAt gets the same grace) is unchecked.

Not pursued further in this pass — this note exists to convert "travel
fails" from an inferred class-membership claim into a cited, measured
instance with a job id, position, and a refuted alternate mechanism, per
Opus's ask. The site-survey (candidate 1) is the natural next step if this
is worth chasing further before AUTON-2 Step 2.

## Instrumentation used (all pre-existing, env-gated, zero cost when unset)

- `BASTION_NEED_SKIP_DIAG` — need-check skip reasons.
- `BASTION_LEGC_DIAG` — the decisive line at travel-timeout firing
  (steer/target/actual_pos/stuck_time/drive/auton_travel_ok). Nothing new
  was added for this note; both flags already existed from this window's
  earlier hypothesis-closing work.
