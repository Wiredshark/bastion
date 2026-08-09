# Run-15 — `#63` is_loaded-filter follow-on, matched to driver-12 (2026-08-09)

Live re-run of `script-09-milestone.txt` verbatim (8 colonists, no food
anywhere, same footprint, full 72,300-tick / 2,410-sim-sec budget matching
driver-12 exactly) at current tip (`ad395ad449`, includes the starvation-
fallthrough fix and the ports-shipment diagnostics), with
`BASTION_DECAY_JOIN_DIAG`, `BASTION_NEED_LOAD_FILTER_DIAG`,
`BASTION_ARB_PERSONAL_DIAG`, `BASTION_NEED_SKIP_DIAG` all on, and
`VELOREN_ASSETS` + `BASTION_REQUIRE_EXPLICIT_ASSETS=1` set per the standing
launch protocol.

Pre-registered outcome bar written before reading any result (kept in this
session's scratchpad, reproduced here for the record).

## Result: dropped_by_is_loaded

**Zero across all 4,897 samples, full run, tick 13 through 74,383.**
Cross-checked against the independent `decay_join_count` meter at a
mid-run sample (tick 30000: `decay_join_count=8`; nearest
`IS-LOADED-FILTER-DIAG` sample at tick 29998/30013: `b_count=8, c_count=8`)
— all three instruments agree on population during the active window.

This is the row's own cheapest refutation (per the charter: a nonzero gap
in an early, loaded, pre-overrun window would mean the mechanism isn't
load-linked and the row is wrong) — it survives, but the claim was never
that it *must* be zero early; it's that it might open *later*, under
accumulated overrun. This run reached driver-12's full tick budget and
still never opened.

## Non-vacuity check (required before scoring, per Opus's charter §3)

The charter requires the loaded arm to independently show the suspected
overrun before a zero on `dropped_by_is_loaded` means anything:

| | driver-12 | run-15 |
|---|---|---|
| `slow system execution` warnings | 22, up to 625ms | **0** |
| wall-vs-nominal overage | ~324s / ~11.8% (2736s wall) | **~9.8s / ~0.4%** (2426.8s wall vs 2417.0s nominal, active window tick 13→72523) |

Measured (not recalled): `data.count(b'slow system execution')` on
`server-stdout-15.log` → 0. Wall span from the first `IS-LOADED-FILTER-
DIAG` timestamp (tick 13, 11:52:27.074881Z) to the last sample where
`b_count` was still 8 — i.e. the script's own active window, excluding the
post-disconnect idle tail (tick 72523, 12:32:53.899068Z) — is 2426.82s
real against 2417.0s nominal for that many ticks (72,510 / 30).

**The load precondition was not met.** Per the charter's own pre-registered
disjunction: this voids the arm for the `is_loaded` hypothesis specifically
— it is a clean non-result, not a refutation, and not evidence either way
on whether sustained overrun would cause a drop. It remains a full,
unqualified pass for the fix itself (below), which does not depend on load.

## driver-12's disposition — superseded, see RUN16-CONTENTION-ARM.md

**Update after the deliberate-contention arm (run-16):** the disposition
below was written when this row's only candidate for driver-12 was the
`is_loaded` filter, and run-15's non-vacuity void left it untested. Run-16
tested it directly (non-vacuous this time, precondition met two
independent ways) and **refuted** it. Run-16 also surfaced a different,
previously unknown mechanism — the driver's own "script complete" claim
can diverge substantially from the server's authoritative tick progress
under contention — which is now the live candidate for driver-12's null,
independent of `is_loaded` entirely. driver-12 is no longer best described
as an unreproduced one-off with no named mechanism; it has a specific,
demonstrated candidate cause, untested against driver-12 itself only
because that run's raw log is gone. Full account: `RUN16-CONTENTION-ARM.md`.

Original text, left for the record: driver-12's own raw log no longer
exists (the deletion incident); with run-15's load precondition unmet
there was no surviving evidence to distinguish "driver-12 was the
`is_loaded` mechanism under real load" from "driver-12 was a one-off." The
`A`/`B`/`C` counters (`BASTION_NEED_LOAD_FILTER_DIAG`) ship regardless and
stay on the live-run protocol — they are what catches an `is_loaded` drop
in seconds if it ever recurs, and remain a durable result of this arc
independent of which candidate explains driver-12 itself.

## Result: the fix, live, at shipped (non-accelerated) rates, full duration

This is the first live confirmation of the starvation-fallthrough fix at
1x shipped rates over the full milestone-length window (script-14 verified
reachability only at 10x acceleration).

- First `RestAt` job ~tick 52036 (≈1735 sim-sec ≈ 28.9 min) — matches the
  script's own predicted ~30min rest crossing.
- First `slept — rest restored` immediately after (travel + sleep time).
- **8 distinct sleep completions — one per colonist, 8/8** — all landing
  between tick 52036 and tick 61051 (≈29–34 min), well inside the 40.2min
  budget with buffer to spare, exactly as script-09's header predicted.
- `no_food_found`: 21,511 (hunger dead-ending permanently, as designed —
  no food exists in this scenario).
- `preempt_cooldown_active`: 2,640 — cooldown gating fired normally.
- Zero food successes (structurally guaranteed, no food anywhere).

## Despondency / calibrator

14 `BREAKDOWN` events fired during this run — see
`bastion-test-evidence/calibrators/run15-calibrator.md` and the committed
`run15-extract.log` for the replacement calibrator (Run B's was lost in
the driver-9..14 deletion incident).

## Tail-end population collapse (not an `is_loaded` finding)

`b_count` and `c_count` both drop to 0 in the final ~1,000 ticks of the
log (after the script's own `=== script complete, disconnecting ===`).
Since `b_count` (the pre-filter join population) collapses too, this is a
component-level despawn tied to the driver disconnecting, not a filter
drop — the same distinction Opus's read established for the earlier
0→4→0 sanity-check oddity. `A` collapsing and `B − C` opening remain
separable in this run's own data, not just in principle.
