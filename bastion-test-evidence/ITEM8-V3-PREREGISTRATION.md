# ITEM 8 ENDURANCE RUN — v3 PREREGISTRATION

**Launch procedure: identical to v2.** Cite
`bastion-test-evidence/ITEM8-LAUNCH-RECORD-V2.md` for the full boot/founding/
verification sequence (fresh userdata dir, `BASTION_ENTITY_EVENT_LOG=1`,
`script-15-item8-endurance.txt`, founding verified via 8 promotions/0
demotions before ending the launch turn, driver disconnects into the scored
window). Nothing about the launch mechanics changes for v3 — only the pin.

**Fix pin this run launches against:** `517cb50f6d` — read fresh via
`git rev-parse HEAD` at launch time (per standing rule, never carried from
memory), confirmed identical to the tip Opus cleared as GREEN (`468fe8f07c`
plus the two post-review closures: exclusivity placement verification and
this doc's own rate-condition/health-signal text).

## THE EXPECTED-DELTA TABLE (Fable's ask, point 1)

Every named consumer of pile amounts / stack structure, checked concretely
against the source before this run, not assumed:

| consumer | reads | sees a change? | why |
|---|---|---|---|
| `board.has_capacity`/`reserved_count` | reservation-count bookkeeping (`reservations_by_item`), never the `PickupItem`'s own Vec | **NO** | Reservation tracking has always been structurally independent of the physical stack — checked the implementation directly (`bastion_jobs.rs:5649`), not assumed. |
| `PickupItem::amount()` (every safe caller, incl. the candidate-search filter, item 8's own food-stock sampler) | sums ALL entries | **NO** | `amount()` was always total-invariant across a split in BOTH the old and the new design — the old design's split redistributed the SAME total across two Vec entries; the physical decrement only ever happened at the eventual consumption (async vanilla `pick_up()` in the old design, synchronous `split_off_one` in the new one), at essentially the same arrival-time moment either way. Re-derived from the code, not assumed benign. |
| `job_still_wanted`'s Farm arm | `common::bastion::JobKind::Designated(DesignationKind::Farm) => true` | **NO** | Trivial always-true predicate — never reads `PickupItem` state of any kind. |
| Mine-supply generator (`bastion_jobs.rs:9119`) | `.item().amount()` (the UNSAFE accessor, against its own doc warning) | **NO** | Targets `MINE_DROP_ITEM` (stone) exclusively — `split_off_one` is never called on stone piles, so this pre-existing (and unrelated) trap is untouched by this fix either way. **Filed as its own row rather than left as a footnote**: `bastion-test-evidence/ROW-MINE-SUPPLY-ITEM-AMOUNT-TRAP.md`. |
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

★★★ **Precondition witness required (Opus's amendment, the sit-trap lesson
applied here): a silent `debug_assert` alone cannot distinguish "the fix
held" from "the trigger never occurred."** `board.b5_split_off_one_fired`
(incremented every time `split_off_one` returns `Some`) is the precondition
half of the claim; the assert's silence is the consequence half. **Both
must be read together at scoring time:**

    b5_split_off_one_fired > 0  AND  debug_assert never fired  ->  PASS (fix exercised and held)
    b5_split_off_one_fired == 0                                ->  VOID on this claim (fix not exercised, whatever else passed)
    b5_split_off_one_fired > 0  AND  debug_assert fired         ->  the finding, reported not patched around

**The merge-check half of the trigger (a split-from pile later merge-
checked against a fresh drop) is NOT independently witnessed** — no cheap
cross-crate hook exists between `bastion_jobs.rs`'s reservation state and
`try_merge`'s own call sites without touching vanilla merge code this fix
deliberately leaves alone. **This is an honest gap, named rather than
silently assumed**: the merge half is inferred from the same farm-drop
traffic pattern v2's log already showed (repeated same-item harvest drops
landing near existing piles), not witnessed directly. If `b5_split_off_one_
fired > 0` and the assert stays silent but no farm-drop merge traffic is
independently confirmed in the log, treat the merge half as UNPROVEN for
that run, not assumed exercised.

★★★ **The rate condition — CORRECTED (Opus's own catch on his own gate
item): the first version of this paragraph asked for a comparison to
"v2's split rate," which cannot be obtained — `b5_split_off_one_fired`
did not exist until the commit that fixed this crash, so v2 has no split
rate to compare against, and never can.** Replaced with an internal
consistency check, available entirely from this run's own data, no
external baseline required:

> **`b5_split_off_one_fired` must be commensurate with the run's own
> `EatFrom` completions.** Every eat against a pile holding more than one
> unit takes the `Some` path, so splits should track (eats minus
> last-unit eats) closely. **Splits near zero while eats are healthy
> (measure 2's own `"bastion: ate — hunger restored"` counts, per-cycle)
> means something suppressed the split path** — that is VOID on the fix
> claim, not a pass, and it is checkable from measure 2's own counts
> without any external baseline. **A run whose `b5_split_off_one_fired`
> count is high early and then collapses, or stays near zero for extended
> stretches while eats keep occurring, is not a pass** — it is an
> unexercised or degraded run for that stretch, and the trigger population
> (why did splits stop while eats didn't) gets investigated before the
> result is read as "the fix held."

## OPTIONAL HEALTH SIGNAL (Opus's addendum, folded in — cheap, not a gate item)

v2's crash arrived at tick 45000 in ~23.6 min wall (1416s): **45000 / 1416 s
≈ 31.8 ticks/s, empirically measured on this exact run mode and scenario**
— not the ~9× headless divergence noted elsewhere for a different run mode.
If the server's target TPS is ~30, this confirms the stated 1:1 ratio this
run's ~2.5-hour estimate (5 cycles × ~30 min) depends on. **Free secondary
signal for v3**: if ticks-per-wall-second drops materially below ~31.8
during the run, the server is degrading under load — an endurance finding
in its own right, uncaught by anything else in the current bar.

## Status

**GREEN — Opus cleared `468fe8f07c` and the two post-review closures at
`517cb50f6d`.** All four original gate items plus both refinements (rate
condition corrected to an internal check, exclusivity placement verified
structurally) are closed. v3 launching on this pin.
