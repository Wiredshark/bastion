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
still never opened. That's evidence against the `is_loaded`-flapping
hypothesis specifically on this run, not proof it can't occur — driver-12's
own machine/session state at the time isn't reproducible.

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

## What this means for driver-12

**driver-12's null does not reproduce on a matched re-run at current tip.**
Where driver-12 logged zero preempts and zero despondency across the full
40 minutes, run-15 — same script, same duration, same footprint — produced
normal, healthy activity throughout: 8/8 colonists slept, thousands of
ordinary skip-reason events, zero `is_loaded` drops. Combined with
script-14's clean 10x pass, this is now two independent live runs post-fix
showing no trace of driver-12's specific failure mode. driver-12 remains
unexplained but increasingly looks like a one-off condition specific to
that session (machine load, concurrent work, or something else not
captured by any instrument that existed at the time) rather than a
recurring mechanism reachable from the code alone. The `#63` row stays
open only in the sense that "we still don't know what happened to
driver-12 specifically" — it is not open in the sense of "the fix might
not work," which this run answers directly: it does, at shipped rates,
unaccelerated, full duration.

## Despondency / calibrator

14 `BREAKDOWN` events fired during this run — see
`bastion-test-evidence/calibrators/run15-calibrator.md` for the byte-level
counts and the replacement-calibrator record (Run B's calibrator was lost
in the driver-9..14 deletion incident).

## Tail-end population collapse (not an `is_loaded` finding)

`b_count` and `c_count` both drop to 0 in the final ~1,000 ticks of the
log (after the script's own `=== script complete, disconnecting ===`).
Since `b_count` (the pre-filter join population) collapses too, this is a
component-level despawn tied to the driver disconnecting, not a filter
drop — the same distinction Opus's read established for the earlier
0→4→0 sanity-check oddity. `A` collapsing and `B − C` opening remain
separable in this run's own data, not just in principle.
