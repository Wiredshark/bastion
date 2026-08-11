# ITEM 8 ENDURANCE RUN — v3 PREREGISTRATION

**Launch procedure: identical to v2.** Cite
`bastion-test-evidence/ITEM8-LAUNCH-RECORD-V2.md` for the full boot/founding/
verification sequence (fresh userdata dir, `BASTION_ENTITY_EVENT_LOG=1`,
`script-15-item8-endurance.txt`, founding verified via 8 promotions/0
demotions before ending the launch turn, driver disconnects into the scored
window). Nothing about the launch mechanics changes for v3 — only the pin.

**Fix pin this run launches against:** `5509dc95c3` (fill in the exact tip at
launch time if further commits land before then — check `git rev-parse HEAD`
fresh, never carried from memory, per standing rule).

## THE EXPECTED-DELTA TABLE (Fable's ask, point 1)

Every named consumer of pile amounts / stack structure, checked concretely
against the source before this run, not assumed:

| consumer | reads | sees a change? | why |
|---|---|---|---|
| `board.has_capacity`/`reserved_count` | reservation-count bookkeeping (`reservations_by_item`), never the `PickupItem`'s own Vec | **NO** | Reservation tracking has always been structurally independent of the physical stack — checked the implementation directly (`bastion_jobs.rs:5649`), not assumed. |
| `PickupItem::amount()` (every safe caller, incl. the candidate-search filter, item 8's own food-stock sampler) | sums ALL entries | **NO** | `amount()` was always total-invariant across a split in BOTH the old and the new design — the old design's split redistributed the SAME total across two Vec entries; the physical decrement only ever happened at the eventual consumption (async vanilla `pick_up()` in the old design, synchronous `split_off_one` in the new one), at essentially the same arrival-time moment either way. Re-derived from the code, not assumed benign. |
| `job_still_wanted`'s Farm arm | `common::bastion::JobKind::Designated(DesignationKind::Farm) => true` | **NO** | Trivial always-true predicate — never reads `PickupItem` state of any kind. |
| Mine-supply generator (`bastion_jobs.rs:9098`) | `.item().amount()` (the UNSAFE accessor, against its own doc warning) | **NO** | Targets `MINE_DROP_ITEM` (stone) exclusively — `split_off_one` is never called on stone piles, so this pre-existing (and unrelated) trap is untouched by this fix either way. |
| `b5_pile_pickup_by_member` (ROW-ITEM6-WITNESS-PACKET B2) | only ever incremented inside the vanilla `InventoryManip::Pickup` handler's `"accepted"` branch | **YES — was a real regression, NOW FIXED** (`5509dc95c3`) | The direct-consumption path bypasses that handler entirely for the majority of eats (any pile with >1 unit remaining). Restored at the new site, gated on the same `BastionPile` membership check, since this branch is always the "member" case. |
| entity event log `ItemEventKind::PickedUp` (item 6's pickup-attribution trail, `record_pickup_event`) | same vanilla handler, `"partial"`/`"accepted"` verdicts only | **YES — was a real regression, NOW FIXED** (`5509dc95c3`) | Same bypass. Restored: subject = the pile's own uid, actor = the eater. |

**Net effect after both fixes: no unaddressed delta.** The two seams that DID
lose a witness (Fable's point 2) now enter through equivalent instrumented
doors, named and cited above — not silently.

## THE BAR — unchanged from `ITEM8-PREFLIGHT-BAR-PREREGISTRATION.md`

N=5 scored cycles, designed to continue to 7 if healthy. All 6 original
measures stand as previously registered (despondency trend, eats/cycle,
sleeps/cycle, food stock non-decreasing from cycle 2, no permanent stall via
`NeedCrossed`, fail-safe rate not climbing) — nothing about this fix changes
any of them; it only removes the crash that prevented the run from reaching
a scored cycle at all.

## THE REGISTERED PREDICTION (Fable's ask)

**Before this run: state what the planted test's trigger population looks
like live, and predict the outcome.**

The planted test
(`split_off_one_never_grows_the_stack_even_under_repeated_splits_then_merge`,
`common/src/comp/inventory/item/mod.rs`) proved the fix in isolation —
repeated splits from a stack, then a merge-check against a fresh drop of the
same item, no panic. **Live, this exact trigger population occurs routinely**:
the farm loop produces repeated harvest drops of the same item def landing
near existing hauled stockpile piles, and the eat path repeatedly splits from
those same piles across the run's many cycles — v2's own log already showed
this pattern before it crashed (`preempt_attempts` 0→12, active haul/farm
cycling for the full ~23 minutes it ran).

> **Prediction: the trigger population (repeated splits, then a same-item
> merge-check against a pile that has been split from) occurs live during
> v3's scored window, and the `debug_assert` in `try_merge` stays silent
> throughout.** If it fires anyway, that is not a v4 script error to
> diagnose away — it is a finding that the fix's fix is incomplete, reported
> the same way this crash was: root-caused before any relaunch, not
> patched around.

## Status

Awaiting Opus's commit review on `5509dc95c3` (folds in the witness-
restoration follow-up to `e14795700e`). No relaunch until cleared.
