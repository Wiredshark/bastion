# Corpus-wave baselines (canonical copy, rescued off C: temp 2026-08-03)
One JSON per fan wave: seed -> full b5 report. Anchor: wave18_FULL.json = 12/48
failures @ a057ed66 (current baseline; chopfell fix 15850c61cc is harness-only).
wave13_EMPTY_zone-exhausted-zero-seeds.json is NOT DATA: that fan lost all 6 VMs
to ZONE_RESOURCE_POOL_EXHAUSTED and delivered zero seeds. It is renamed out of
the wave*_FULL.json glob so no comparison silently ingests an empty dict �
"couldn't measure" must never share a shape with "measured nothing."

## ★ PROVENANCE: filenames are labels — attested COMMIT= wins (checked 2026-08-04)

All six attestable waves were re-checked against their filenames. **Five match;
one does not — the problem is a one-off, not systemic:**

| artifact | filename says | attested `COMMIT=` | verdict |
|---|---|---|---|
| wave19 | `ed532c600e` | `ed532c60` | match |
| wave20 | `d010339a55` | `d010339a` | match |
| **wave21** | **`ed532c600e`** | **`1bf3ab2e`** | **MISMATCH** |
| wave22 | `34db70bac2` | `34db70ba` | match |
| wave23 | `b89cbc799d` | `b89cbc79` | match |
| wave24 | `d3235e5329` | `d3235e53` | match |

**wave21 ran on `1bf3ab2e1c`, not `ed532c600e`.** Resolved, and the comparison
is still sound: `1bf3ab2e1c` is exactly ONE commit ahead of `ed532c600e`, and
that commit changes **one markdown file (+24 lines) and ZERO code files** —
verified with `git diff --name-only ed532c600e 1bf3ab2e | grep -E '\.(rs|toml|ron)$'`
→ empty. Binaries are behaviourally identical, so **wave19 vs wave21 remains a
valid same-code determinism comparison.**

**Cite it as `1bf3ab2e` (docs-only ahead of `ed532c600e`), not as
`ed532c600e`.** An audit that trusted the filename would have recorded the
wrong provenance for the run that anchors our determinism claim.

### ★ FILE RENAMED 2026-08-04 (architect ruling) — old → new mapping

```
wave21-fanlog-BASELINE-RERUN-ed532c600e.txt   (OLD — hash was FALSE)
        ↓
wave21-fanlog-BASELINE-RERUN-1bf3ab2e1c.txt   (CURRENT — matches attestation)
```

**The raw directory `wave21-raw-BASELINE-RERUN/` is unchanged** — it carries no
hash in its name, so it was never wrong; its logs attest `COMMIT=1bf3ab2e`.

**Why rename rather than leave a dangling reference:** this directory's entire
value is per-seed provenance, so **a plausible-looking wrong hash is a worse
trap than a missing file.** A missing file forces the reader here, where this
section explains everything; a wrong hash forces nothing and is believed. Same
reasoning as the `wave13_EMPTY` rename above — **the label must not contradict
the content it names.**

**Rule this instance re-earns:** verify the attested commit before *citing* a
wave, not only before reading one. The mismatch was invisible for a day because
nothing compares the label to the attestation, and the label is what gets
quoted. See [[read-the-content-not-the-label]] — filenames are labels, and this
directory already carries one such trap (`wave13`, above).

## ★★★ CAPTURE PROCEDURE (mandatory from wave25 — DECISIONS #56)

> **A FAN THAT PERSISTS ONLY ITS VERDICT IS NOT A BASELINE.**
> Every fan writes a per-seed `_FULL.json` **plus** its `COMMIT=` attestation to
> this directory before teardown, or the wave is a verdict and not evidence.

**Why this is a procedure and not just a rule.** wave24 kept only its fanlogs,
so when a later mandatory hold-check needed a structured referent there was
none — and the gate would have GREENed over a comparison mixing ten commits of
prior work with the change under test. **A gate that cannot fail.**

**And the reason wave24 lost its body was not carelessness: there was no
collector.** `vm-pool.sh` streams each seed's JSON back inside `@@@SEED n@@@`
markers (the VMs are deleted immediately afterwards) and nothing in the tree
turned that stream into a file. **A ratified law with no implementing mechanism
is a comment.**

### The two-command capture

```bash
# 1. fan  (logs land in /tmp/bastion-pool/bastion-pool-N.log)
ZONE=us-east1-b STAGGER=25 BRANCH=<branch> \
  bash vm-pool.sh 4 e2-standard-8 12 49 "--b5-scenario" 25 90 \
  > corpus-waves/wave<N>-fanlog-<commit>.txt 2>&1

# 2. collect  — REFUSES rather than guesses; exit 2 = do not use the result
python collect_wave.py corpus-waves/wave<N>_FULL.json /tmp/bastion-pool/bastion-pool-*.log
```

`collect_wave.py` refuses on: a missing `COMMIT=` line; logs attesting
**different** commits (the mid-fan push hazard); and **zero usable seeds** — so
wave13's `{}` is now unwritable. It excludes-and-names, rather than silently
dropping: empty blocks, unparseable blocks, and seeds short of the **modal key
set** (the 2026-08-03 short-JSON ghost — a seed missing fields is UNPROVEN, not
a data point).

### Comparing two waves

```bash
python holdcheck.py wave<BASE>_FULL.json wave<NEW>_FULL.json \
  --descend=b5_mine_cell_diag,farm_cell_diag \
  --ignore=b5_build_stamp,b5_soak_avg_tick_ms \
  '--expect-new=<every new field, patterns allowed>'
```

Exit **0** = hold, **1** = violation, **2** = the check could not be run.

**`--ignore` is not free and says so:** it prints how many seeds each ignored
field *would* have moved on. `b5_build_stamp` (carries a build timestamp) and
`b5_soak_avg_tick_ms` (wall-clock) move on all 48 seeds of any rebuild — without
handling them the gate reds on noise, and **a guard that cries wolf teaches its
own bypass.** `b5_build_stamp` should eventually be *asserted* against the
expected commit rather than ignored; filed, not done.

### Two rules that bit during setup, both mechanical

1. **Nothing is pushed while a fan is in flight — docs included.** Each VM runs
   `git reset --hard origin/$BRANCH` at *its own* start time, so a mid-run
   commit can split the pool across tips. The post-commit hook auto-pushes, so
   **hold all commits for the duration.**
2. **Never read an exit code through a pipe.** `cmd | tail` reports *tail's*
   status. This is recorded in memory and still caught someone during the
   collector's own control run; the habit that catches it is re-verifying
   unpiped, not remembering.
