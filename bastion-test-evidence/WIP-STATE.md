# WIP-STATE — 2026-08-04, Opus lane (first-line reviewer / fan owner)

**IN FLIGHT:** 48-seed paired A/B fan for Row B.

    ZONE=us-east1-b STAGGER=25 BRANCH=bastion/wip-batch-verify \
      bash vm-pool.sh 4 e2-standard-8 12 49 "--b5-rowb-paired" 25 90 \
      > bastion-test-evidence/corpus-waves/wave27-ROWB-PAIRED-fanlog-f7072cd346.txt

Remote tip verified `f7072cd346` before launch. **Diag deliberately OFF** — the
fan is the measurement, the diag is the investigation, and the measurement must
not pay the investigation's cost.

## When it lands

1. **Attest first**: `grep COMMIT= /tmp/bastion-pool/bastion-pool-*.log` — expect
   `f7072cd34` on all four, `DONE=12` each.
2. **Collect**: `python collect_wave.py corpus-waves/wave27_ROWBPAIRED_f7072cd346_FULL.json /tmp/bastion-pool/bastion-pool-*.log`
   (refuses on missing/mixed attestation or zero usable seeds — exit 2 means do
   not use the result).
3. **Read the DELTA DISTRIBUTION**, not any single seed. The question is whether
   the n=1 `mine_blocks_mined 25→26` is a distribution shift or a favourable
   draw. **Seed 90 alone proves nothing** — that was the whole reason for this fan.
4. **Report raw**, then hand 5b the EDGE seeds for targeted diag-ON re-runs:
   any seed where the variant did **worse**, and any where a job crossed the
   threshold but the outcome **didn't move**. Decided before the data so it
   cannot be fitted to it.

## Standing state

| item | state |
|---|---|
| **Row A** | SHIPPED, green (#58). Seeds 71 & 90 reported for the first time. |
| **Row B** | LANDED `f7072cd346`. Benches the named holdout (17989,9263,338): fire→3/2/1/0→graduate→clear. Ship gate = this fan. |
| **Observer effect** | MEASURED + bisected. Per-cell diag reads perturb scheduling; read budget (cells × reads × cadence) now in the acceptance framework. |
| **Playthrough** | HELD until the fan lands. Scorecard drafted (13 features, player language); read-budget check applied to its metrics plan. |
| **3 bisection worktrees** | HOLD — the observer effect's demonstrator rig. Do not clean. |

**Canonical noise-floor method:** parallel legs, separate `--data-dir`s. A zero
here is what licenses attributing any later drift to a change.

**Open, filed not fixed:** `mine_timeout_position_diag` is cumulative-ever-timed-out,
not currently-open — a name suggesting current state over historical content.
