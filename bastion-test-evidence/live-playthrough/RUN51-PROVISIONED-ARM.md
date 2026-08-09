# Run-51 — #51 provisioned arm (2026-08-09)

`script-10-milestone-food.txt`, post-fix tip, food provisioned (40
mushrooms dropped at anchor + resupply at checkpoint 3), full 72,300-tick
budget, all diagnostics on. Scored against `ROW51-PROVISIONED-ARM-BAR.md`
(written before this run).

## Preconditions

1. `Certified asset root path=E:/.../assets` fired at boot (explicit
   assets, not ambient).
2. **Server-authoritative final tick: 72,628 of 72,300 — 100.5% of
   budget**, measured from the last `IS-LOADED-FILTER-DIAG` sample with
   `b_count=8` (tick 72628, 19:02:12), not from the driver's "script
   complete" claim. Checkpoint 8 (the driver's own last note) landed at
   the expected ~40min mark.
3. Zero `debug_assert!`/panic in either server log (grepped explicitly).
   Assertions armed, silent.
4. Admin role required for `give_item`/`dropall`: `server-cli --no-auth
   admin add bastion_llm_player admin` before boot (the subcommand's own
   UUID lookup fails against the real auth server without `--no-auth`
   placed before it — hit and fixed before starting).
5. Calibrator (`run15-extract.log`) canary confirmed readable at the byte
   level (`BREAKDOWN` scores nonzero in this run's own log directly —
   13 — proving the em-dash encoding path works; the calibrator itself
   proves the reading METHOD, not this run's specific counts).

## #51 primary measure — bed arrival under contention: CLEAN, ARRIVAL WORKS

- **9 distinct `RestAt` job ids created, 8 `slept — rest restored`
  completions.** Creations ≈ completions (the 1-job gap is consistent
  with ordinary churn, not creations≫completions).
- First `RestAt` preempt ~18:50:41 (≈29min post-boot), matching the
  script's own ~30min rest-crossing prediction; first `slept` ~34s later.
- **Reading against the bar: ARRIVAL WORKS.** The original "colonists
  keep failing to reach a bed" finding does not reproduce under this
  provisioned, contended arm.

## The unplanned finding: food-search is functionally broken despite provisioning

- **1 distinct `EatFrom` job id created (713, uid 54) in the entire
  40-minute run. 0 `ate — hunger restored` completions.**
- `need preempt — hunger below interrupt` (the SUCCESS path -- fires
  only when food is actually found) fired exactly once, at 18:37:12
  (≈15.5min post-boot, matching the script's own ~15min hunger-crossing
  prediction for the FIRST occurrence only).
- `no_food_found` (the failure path) fired **21,176 times** across the
  run -- hunger crossing and re-searching repeatedly for the other 7
  colonists (and for uid 54 after job 713's activity stopped), almost
  never finding the provisioned food.
- Job 713 itself never completed or showed an explicit release/reclaim
  message -- its last `ARB-PERSONAL-DIAG` appearance is at tick 30166
  (18:38:32, ~80s after creation), `on_self_job=true` the whole visible
  window. It does not reappear afterward in this colonist's own diag
  stream.

**This is NOT a bed-arrival finding.** Per the bar's own scope limit
("a FAIL here indicts travel/arrival, NOT the need-arbitration fix"),
by the same logic this indicts the FOOD-DISCOVERY mechanism specifically,
not #51's question. Filed as its own open item, not resolved here --
root cause (item despawn timing, drop-location reachability, a
search-radius/eligibility gap, or something else) is unread. The `EatFrom`
side of the bar's table reads FAILS by every cell (creations≫completions
in the sense that almost no creation happens at all), while the `RestAt`
side reads WORKS cleanly -- a split result the bar's own per-observable
table anticipated as possible.

## Despondency -- reads as a consequence of the food finding, not arrival

**13 `BREAKDOWN` events.** Per the bar's registered despond inversion:
zero despond would have meant both needs satisfied; despond firing means
at least one need stayed chronically unmet. Given hunger's search
functionally failed for the whole run (0/8 colonists ever ate), 13
despond events is the expected consequence of the food finding above,
not a separate arrival-failure signal -- rest was satisfied (8/8 slept),
so any colonist's mood-break traces to hunger specifically.

## Travel-row falsifier (#60) -- not extractable from this run's tracked cells

No dedicated "access-job claim/release" log line exists in the codebase
(checked directly, not assumed). This script's two fixed `inspect_cell`
checkpoints (a Mine cell and the Bed cell) never showed an `is_access:
true` job at any of the 8 sampled checkpoints, so the churn-vs-latch
question from `ROW51-PROVISIONED-ARM-BAR.md` isn't answerable from this
specific run's observable surface. Reporting the gap rather than forcing
a read from adjacent data -- would need either a script that tracks an
access-job's own cell directly, or a dedicated diagnostic at the claim/
release site itself (neither exists yet).

## Summary

| question | answer |
|---|---|
| #51 (bed arrival under contention) | **CLOSES** -- arrival works, 8/9→8 |
| Food-search under provisioning | **NEW OPEN FINDING** -- functionally broken, 1 attempt/40min, 0 completions |
| Despondency (13 events) | consequence of the food finding, not a separate arrival signal |
| #60 travel-row falsifier | not extractable from this run's tracked cells |
