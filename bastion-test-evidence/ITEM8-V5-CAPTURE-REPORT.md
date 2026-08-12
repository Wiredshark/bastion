# ITEM 8 v5 — TEARDOWN + CAPTURE REPORT

**Pin:** `4d918025` (mine-fix cluster: defect 2's egress-termination fix,
honest completion metric, all four watchdog safety-net gates). Double
gate-0 verified before launch (`ITEM8-LAUNCH-RECORD-V5.md`). Stopped at
the 5-cycle mark by pre-registered rule (Fable + Opus, "no extension
regardless of how well it's going" — optional-stopping law), NOT
continued to 7 cycles even though the run was healthy throughout.

## TWO CLOCKS

    PROCESS START:       2026-08-11T23:55:10.897657Z (19:55:10 EDT)
    SCORED-WINDOW START: 2026-08-11T23:56:43.678282Z (19:56:43 EDT, driver disconnect)
    KILL SENT:            2026-08-12T02:30:00Z         (22:30:00 EDT)
    FINAL LOG LINE:       2026-08-12T02:30:00.725373Z  (matches kill time, no truncation)
    SCORED-WINDOW DURATION: 2h 33m 17s

Real-time throughout — `BASTION_UNCAPPED_TPS` deliberately unset, per
the N=8 promotion-tick finding (`e1e193cedc`, zero-overlap 6x shift):
chunk-gen/promotion wall-coupling is a proven, not suspected, impossibility
for compressed mode this arc.

## THE HEADLINE RESULT — sustained production, never seen before in this arc

**Zero panics, zero debug_assert fires** (`grep -c "panicked"` → 0)
across the full 2h33m run. **Zero colonist demotions.** Log stable across
three reads post-kill (161,749,162 bytes, unchanged), final line
complete (not mid-write), stderr empty throughout.

**Farm: tilled=56, sown=1749, harvested=1721** — with `harvested` itself
returning a cell directly to "tilled" (its own log line says so), only
the FIRST till per plot logs; 1749 sown against 56 initial tills means
**each plot cycled roughly 31 times.** v4's entire lifetime total was
19 tilled / 20 sown / 20 harvested, once, before the trap captured the
labor force. v5 is not clearing F1's amended "generation-2 completions"
bar — it laps it by two orders of magnitude.

**Food stock: max seen 1957** (v3/v4 peaked at ~18-22, per prior
capture reports). 915 heartbeat samples, 292 read zero at some point
(normal production/consumption fluctuation), longest consecutive
zero-streak 47 heartbeats (~7.8 sim-minutes) — never a terminal streak;
the run ends with `food_stock=341`, actively fluctuating, healthy.

## THE WITNESS COUNTERS — final heartbeat (tick=274500)

    food_stock:                          341
    claim_expiry_releases (F5):          189   (nonzero across the whole
                                                 window -- the fix's own
                                                 precondition genuinely
                                                 exercised, not VOID)
    designated_sweep_reaps (route 3):     34   (flat since early in the
                                                 run -- no runaway churn,
                                                 the v4 defect stays fixed)
    generic_claim_leak_releases (F6):      0   (clean across the entire
                                                 run -- the inverted bar's
                                                 expected PASS, holds)
    emergency_access_completions:         33   (the honest measure, per
                                                 Opus's ruling NOT a bar --
                                                 reported for context, see
                                                 below)

## F7 / F8 — the two new bars from `ROW-INDESTRUCTIBLE-MINE-CELL.md`

**F8 (`"bastion: job completed"` fires ONLY with a world-effect):
PASSES by construction and by count.** The generic (non-emergency)
completion line fired **zero times** in this run — expected, not a
red flag: the founding script (`script-15-item8-endurance.txt`) only
designates Stockpile/Farm/Bed, never Mine/Chop/Build, so the only
Mine-completion channel that exists at all in this scenario is the
emergency-access one, and every one of those 33 completions correctly
used the OTHER, honestly-labeled line
(`"bastion: emergency access job completed (no world effects...)"`).
Zero drops, zero XP, zero cave-ins attributed to those 33 — correct,
by design, and confirmed in the log, not assumed.

**F7 (no single position accounts for >10% of completions): PASSES,
decisively.** 33 emergency-access completions across **28 distinct
positions**; the single most-repeated position saw exactly **2**
completions — **6.1% of total**, well under the 10% bound, and nothing
close to v4's 143/145 = 98.6% at one cell.

**v4's own trap cell, `(15212, 16043, 425)`, reappears here** — 2
completions this run (not 281), no loop. Consistent with the earlier
`completed_kind` read at this same position (`Some(Leaves)`, mid-run) —
defect 1's SUBJECT (foliage, not rock) is confirmed again; its
MECHANISM (why that write doesn't stick) remains unread, staged not
closed, exactly as the packet specified. **The `userdata` save state is
committed untouched specifically so this question remains answerable
after the fact.**

## THE FAIL-SAFE CHAIN — observed live, not code-read

**65 `ULTIMATE FAIL-SAFE` firings total, across 20 distinct colonist
uids** (max 6 firings on any one uid — `57`, `129`, `60`, `59` each hit
6; not a single-colonist trap, a distributed pattern). **Two distinct
`terminal_cause` values:**

    egress_plan_or_climb_free_failed:            51
    egress_no_route_then_climb_free_expired:      14

Both real, both distinct — not one cause dominating a suspiciously
uniform set. Every firing observed at exactly `secs=60.0`
(`STUCK_TELEPORT_SECS`), confirming the watchdog reset gates (all four,
landed in `e60e34ec5d`/`4d9180252f`) genuinely let the clock accumulate
to its designed threshold instead of being silently disarmed by phantom
completions the way v4's was for 2.5 hours straight.

## PROVENANCE — split log, lossless, same procedure as v4

Raw log 161,749,162 bytes (~161.7 MB), exceeding GitHub's 100 MB
single-file limit. Split via `split -b 90M -d -a3` into
`server-stdout-item8-endurance-v5-split/part-{000,001}` (94,371,840 /
67,377,322 bytes). **Verified lossless before committing**: `md5sum` of
the reassembled stream (`f04a4c72ba1a8c9775263cb183eb7705`) matches the
original file's `md5sum` exactly.

`userdata-item8-endurance-v5/` committed **untouched**: 790 MB, 477
files, no single file over 90 MB. Consistent in shape with v3/v4's own
userdata (779-784 MB, 474-477 files).

## WHAT THIS CAPTURE IS AND IS NOT EVIDENCE OF

**IS:** full-scale, quantified evidence that the mine-fix cluster
(defect 2's termination, the honest completion metric, and all four
watchdog gates) holds under 2.5 hours of sustained real load — zero
panics, F5/F6/F7/F8 all passing or informationally clean, farm
production continuing at ~31 cycles per plot with no sign of slowing,
the fail-safe chain firing correctly and distributedly rather than
being disarmed.

**IS NOT:** a closure of defect 1. The block-write mechanism at v4's
trap cell (and, per this run, at least one other reappearing position)
remains unread — its subject is now confirmed (foliage, not rock,
twice), its mechanism is not. The committed `userdata` is what keeps
that question answerable.

## KNOWN CAVEAT, PRE-REGISTERED, NOT WITHDRAWN

v5 ran with substantially heavier diagnostics than v3/v4 baseline
(`BASTION_EGRESS_DIAG=1`, ~114x v3's byte rate — registered, real,
estimated small in absolute terms, ~14 KB/s / ~34 lines/s, implausible
to materially compete with CPU-bound worldgen). Applies to any
wall-clock cross-run comparison (rescue rate, promotion timing);
COUNT-based measures in this report (farm cycles, F5/F6/F7/F8,
completions-by-position, fail-safe firings by uid/cause) are unaffected
by it.
