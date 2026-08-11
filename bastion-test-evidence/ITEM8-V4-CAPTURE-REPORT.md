# ITEM 8 v4 — TEARDOWN + CAPTURE REPORT

**Pin:** `b96830d161` (routes 1-3 as originally shipped, F5/F6, sentinel S1,
b5 port). **Route 3's bug (churn) was found and fixed at `3cce15dd33`
DURING this run's flight — v4 flew on the ORIGINAL, buggy route-3 the
entire time, by design ("v4 flies to its end untouched").** This capture
is evidence of the pre-fix route-3 behavior at full scale, not of the
fix that supersedes it.

## TWO CLOCKS

    PROCESS START:       2026-08-11T18:52:50.414036Z (14:52:50 EDT)
    SCORED-WINDOW START: 2026-08-11T18:54:39.570589Z (14:54:39 EDT, driver disconnect)
    KILL SENT:            2026-08-11T21:27:55.052Z    (17:27:55 EDT)
    FINAL TICK LOGGED:   270600+ (server still creating/reaping jobs at the moment of kill)

## THE HEADLINE RESULT — routes 1+2 held; route 3's bug froze further
progress at the exact point it always would have

**Zero panics or debug_assert fires across the full run** (`grep -c
"panicked"` → 0) — the crash-fix's consequence half holds again, now on
a binary that also carries the famine fix. **Zero colonist demotions**
— `ROW-COLONY-PRESENCE` continues to hold under this arc's heaviest load
yet.

**Farm completions: 19 tilled + 20 sown + 20 harvested = 59 — IDENTICAL
to v3's count, to the job.** This is the clearest single number in this
capture: routes 1+2 (unconditional claim release) did their job for the
ORIGINAL founding-stock economy exactly as designed — the same 59
completions v3 achieved before its famine locked in, v4 also achieved.
**What v4 could NOT do that v3's fix was supposed to enable: make any
further progress**, because route 3's churn bug (see below) reaped every
regenerated Farm job before any colonist could claim it, for the entire
remaining ~2h33m of the run.

## THE CHURN, QUANTIFIED

    farm jobs created:            468,672   (v3: 87, over the same rough
                                              elapsed-flight duration)
    "unclaimed designation swept" 468,593   (confirmed via direct grep,
                                              not estimated)
    designated_sweep_reaps (final heartbeat value): 468,723
    last sweep event timestamp:   2026-08-11T21:27:55.540844Z (matches
                                   the kill timestamp -- the churn never
                                   stopped or slowed for the entire run)

**Root cause already diagnosed and fixed** (Fable's live diagnosis
during v4's flight, confirmed independently by Opus, committed at
`3cce15dd33`): the original route-3 sweep read `cycles_since_last_claim`
— keyed by POSITION, ticking every arbitration cycle regardless of any
particular job's existence, reset only on claim — as if it were the
JOB's own age. A freshly-created Farm job at a position that had gone
one gate-period (`access_stall_secs()`, ~930s) unclaimed was **born
already past the reap threshold**, swept on sight, the cell freed,
Farm's generator recreated an equivalent job there next pass (itself
born stale by the same position history), swept again — an unbounded
loop, not a threshold problem. **Not present in this capture's binary
fix** — this run is the direct, full-scale evidence of the bug the fix
addresses, captured for the record rather than discarded now that the
fix exists.

## THE WITNESS COUNTERS — final heartbeat values

    splits (b5_split_off_one_fired):    0   (the crash-fix's own split path
                                             never exercised this run --
                                             VOID on that specific claim,
                                             consistent with v3's own
                                             instrumentation gap finding;
                                             unrelated to the famine fix
                                             this run actually tests)
    claim_expiry_releases (F5):         2   (> 0 -- the targeted-release
                                             mechanism fired for real,
                                             confirmed live and working;
                                             satisfies F5's own VOID-on-
                                             zero rule)
    designated_sweep_reaps (route 3):   468,723   (the churn, see above)
    generic_claim_leak_releases (F6):   0   (clean -- the inverted bar's
                                             expected PASS: no leak route
                                             beyond route 2's own coverage
                                             was found; F6 never had
                                             occasion to fire, since
                                             route 2's claims never
                                             survived their claimant's
                                             preemption long enough to
                                             leak)

## OTHER MEASURES

    eats ("ate — hunger restored"):     11
    sleeps ("slept — rest restored"):   0
    colonist demotions:                 0
    sentinel S1 ("COLONY TERMINAL") fires: 3 -- edge-triggered per
        qualifying window; food_stock touched nonzero between at least
        some of these windows (consistent with the 11 eats occurring
        against a stock that mostly read 0 on the 300-tick sample
        cadence -- food existed briefly between production and
        consumption more than once), each separate sustained-zero
        streak logging its own line as designed. Not investigated
        further here -- log-only, no action taken, matching its own
        design.
    client connect/disconnect:          1 pair (2 lines), both before
                                         the scored window

## LOG STABILITY (capture protocol, identical procedure to v2/v3)

    read 1 (immediately post-kill): 278,753,195 bytes
    read 2 (~5s later):             278,753,195 bytes
    read 3 (~9s later):             278,753,195 bytes -- stable

Server process (PID 22500 / msys pid 1443) confirmed gone via `ps` after
`kill`. stderr log: 0 bytes throughout.

**Log size note, disclosed rather than left to speak for itself:**
278.75 MB — roughly 250x v3's 1.12 MB. **Fully explained, not a
mystery**: 468,593 of the log's lines are the churn-bug's own sweep
events, confirmed via direct grep against the log itself, not inferred.
Excluding those, this run's remaining log volume (~59 completions, 11
eats, 3 sentinel fires, boot/founding, and the per-300-tick heartbeat
series) is consistent with v3's own rate.

## PROVENANCE — the raw-commit / untouched-userdata decision, unchanged
from v3's pre-ruling, one new accommodation forced by GitHub itself

Full raw log committed, no gzip, no filter, same reasoning as v3
(unpredictable which lines a future reader needs). `userdata-item8-
endurance-v4/` committed untouched: **784 MB, 477 files** — consistent
in shape with v3's 779 MB/474 files, no single file over GitHub's 100 MB
limit.

**★★★ NEW: the log itself, this time, DID hit that limit** —
`server-stdout-item8-endurance-v4.log` is 278,753,195 bytes (~265.8 MB),
first pushed as a single file and **rejected by GitHub's own pre-receive
hook** (`GH001: Large files detected... File ... is 265.84 MB; this
exceeds GitHub's file size limit of 100.00 MB`). Every prior run's log
(v2 287 KB, v3 1.12 MB, v4's own userdata files all individually small)
never came close to this limit, so the "commit raw" pre-ruling never had
occasion to name it. **Fixed by splitting, not compressing or
filtering** — `server-stdout-item8-endurance-v4-split/part-{000,001,002}`,
each under 100 MB, reassembles via `cat part-000 part-001 part-002 >
server-stdout-item8-endurance-v4.log` (documented in that directory's
own README). **Verified lossless before committing**: `md5sum` of the
reassembled stream matches the original file's `md5sum` exactly,
checked directly, not assumed from the byte counts alone. This
satisfies both halves of the pre-ruling (no compression, no content
filtering) — it is a purely mechanical accommodation for a hard
platform limit the ruling didn't anticipate, not a scope change to what
gets preserved. **Flagged here for the record and for v5/future runs**:
any run whose log volume approaches ~90 MB (this run's churn-bug volume
is the only precedent) needs this same split step before commit, not
discovered at push-rejection time.

## WHAT THIS CAPTURE IS AND IS NOT EVIDENCE OF

**IS:** full-scale, quantified evidence of the route-3 churn bug (already
fixed) at the volume a real 2.5-hour run produces it, useful for anyone
who wants the shape of the failure mode rather than the 15-minute
early-check snapshot. **IS** further confirmation the crash fix and
routes 1+2 hold under the heaviest sustained load this arc has produced
(468k+ job-lifecycle events, zero panics). **IS NOT** evidence the
famine fix works end-to-end — route 3's bug prevented the run from ever
reaching the state (sustained farm progress past the founding-stock
economy) that would demonstrate that. **v5, on the fixed route 3,
carries that question next.**
