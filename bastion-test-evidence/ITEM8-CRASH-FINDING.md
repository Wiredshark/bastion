# ITEM 8 ENDURANCE RUN — CRASHED at ~23.6 sim/wall-min (tick 45000): ROOT
# CAUSED to #89's OWN `split_off_one`, not a vanilla bug

**Status: root-caused per Fable's ruling. NO FIX landed — routed to Opus for
first-line review, per her explicit instruction. Not related to
ROW-COLONY-PRESENCE or #85's fields, both cleared below.**

★ **Correction to this doc's own first draft:** the section below originally
classified this as "not related to bastion" on the strength of a grep
showing `try_merge` is never *called* from `bastion_jobs.rs`. **That answers
the wrong question.** Fable's catch: the caller of the assert is not the
producer of the violating state — `read-the-producer` applies to the
CONSTRUCTION site, not the call site. Corrected below; the original
"not-bastion" framing is kept struck through in spirit only, replaced
entirely by the section that follows.

## What happened

The server process (`veloren-server-cli.exe`, PID 40488 / WINPID 197609,
launched 09:14:08 EDT / 13:14:08 UTC) is **confirmed dead**: absent from
`tasklist`, log stopped growing (verified via two `wc -c` reads 3s apart,
byte-identical), stderr contains a panic:

    thread 'main' (29800) panicked at common\src\comp\inventory\item\mod.rs:1752:13:
    All items before the last in `PickupItem` should have a full amount

**Last tick: 45000** (`bastion food stock sample tick=45000 food_stock=54`,
13:37:43 UTC). **Run duration: ~23 min 37 sec** (13:14:08 → 13:37:45 UTC) —
well short of even ONE cycle (~30 sim-min), so this died partway through
cycle 1, not at a scored boundary.

## What the panic means

`PickupItem::try_merge` (`common/src/comp/inventory/item/mod.rs:1734`)
asserts that every item in a `PickupItem`'s internal stack **except the
last** must already be at `max_amount()` — the invariant that lets a merge
safely `.append()` the two vectors without re-validating each element. This
`debug_assert!` fired, meaning **some earlier operation left a `PickupItem`
with a non-full, non-last item** — the corruption happened before this
merge attempt, not during it.

**This is a `debug_assert!`, and it fired under the `no_overflow` profile**
(debug assertions on, optimized) — exactly the profile this arc's own
standing rule requires for catching invariant violations a release build
would silently corrupt through instead of panic on.

## ROOT CAUSE — the producer, read and cited (Fable's ruling applied)

**`PickupItem::split_off_one`** (`common/src/comp/inventory/item/mod.rs:1823`)
— **this arc's own #89 (DECISIONS #89, Option B, per-unit reservation
capacity)**, added to enable the eat path to split one unit off a stack for
a single consumer. It is the producer.

**Mechanism, traced end to end:**

1. `PickupItem::max_amount()`-adjacent fact (`Item::max_amount`, line 908):
   `if self.is_stackable() { u32::MAX } else { 1 }`. **For a stackable item
   (food, farm produce), "full amount" is `u32::MAX` — a value nothing in
   this game ever reaches.** `item.amount() == item.max_amount()` is
   therefore **structurally false for every stackable entry that isn't
   exactly `u32::MAX` units**, i.e. every real one.
2. `split_off_one` finds the first entry with `amount() >= 2` at index
   `idx` (deliberately not always the last — its own doc explains why),
   `decrease_amount(1)`s it in place, then **pushes the new single-unit
   split as the new LAST entry** (`self.items.push(single)`).
3. **After this call, `self.items[idx]` — the original, now-decremented
   stack — is no longer the last entry.** For a stackable, its amount can
   never equal `u32::MAX`, so it now permanently violates
   `try_merge`'s invariant for as long as it sits at a non-last index.
4. **The violation is dormant until this exact `PickupItem` entity is
   merge-checked against a fresh drop of the same item type** — which is
   exactly what a sustained, multi-generational farm/haul economy produces
   repeatedly (harvest drops landing near existing hauled piles). 23
   minutes of `preempt_attempts` 0→12, active `EatFrom` completions, and
   continuous haul/farm cycling is precisely the exposure this needs.

**This was already known and documented at the time #89 shipped** —
`split_off_one`'s own doc comment (lines 1814–1822) states verbatim: *"the
struct-level invariant that non-last entries stay at `max_amount()` is
unenforceable for stackables … and is deliberately not maintained here;
`try_merge`'s own debug_assert on that invariant could in principle fire if
this entity is merge-checked against a fresh drop of the same item while
already split … out of this row's scope."* **The comment predicted this
exact crash and deferred it, correctly scoped at the time, now due.**

**The `idx == last` / single-entry case, checked explicitly:** `decrease_
amount(1)` mutates `self.items[idx]` in place, wherever `idx` sits (0,
`len()-1`, or between). `self.items.push(single)` then unconditionally
appends one entry AFTER whatever the vec's current length is — so
`items[idx]` is never the new last index for ANY value of `idx`, including
a single-entry stack (`idx == 0 == len()-1` before the call). **The
violation is unconditional whenever `split_off_one` returns `true` — no
edge case escapes it.**

**Vanilla's own `try_merge`, for contrast, maintains the invariant by
construction:** it always pops-and-re-pushes the potentially-partial item
LAST (`self.items.push(self_last)`, then the remainder if any). A
multi-entry stack built by ordinary merges has full non-last entries by
construction. `split_off_one` is the only mutator in this codebase that
creates a partial NON-last entry — its own doc comment's defence ("pre-
existing in shape, not introduced by this method") does not survive that
comparison; it is introduced by this method in effect, even if the
*general* risk shape (any multi-entry stack) predates it.

**Why "not related to bastion" (this doc's original classification) was
wrong:** it was reasoned from `try_merge`'s CALL site (never called from
`bastion_jobs.rs` — true, but irrelevant) instead of the CONSTRUCTION site
that put the violating state there (`split_off_one`, which is bastion's own
#89 code, living in the common crate). The status-quo law applies exactly as
Fable stated it: split-pickup under 23 minutes of compounding farm churn is
code no prior run in this arc ever exercised at this depth — the
never-before-exercised path IS the change, not a pre-existing vanilla
defect independent of this arc's work.

## Cleared — genuinely not implicated

- **ROW-COLONY-PRESENCE** (`ea2cfa5192`): its acceptance leg never
  designated a farm, so `food_stock=0` throughout that 15-minute run — no
  split/merge cycles were exercised there at all. The mechanism this crash
  needs (repeated `split_off_one` + a later merge against a fresh same-item
  drop) has no surface in that leg.
- **#85's fields** (`5d905a247d`): diagnostic-only reads at the ULTIMATE
  FAIL-SAFE emit site, never mutate `PickupItem` state, and were built via
  an isolated `cargo check -p bastion-server` that never touched the
  running endurance server (confirmed by unchanged PID/binary mtime both
  times).

## Evidence

    bastion-test-evidence/live-playthrough/server-stdout-item8-endurance-v2.log  (287KB, stable)
    bastion-test-evidence/live-playthrough/server-stderr-item8-endurance-v2.log  (228 bytes, the panic)
    bastion-test-evidence/ITEM8-LAUNCH-RECORD-V2.md                              (the launch this crashed from)

## Status — awaiting Opus's first-line review

- **Root cause found and cited above** (`split_off_one`, the entry's own
  predicting comment, the mechanism traced step by step).
- **NO FIX has been written or landed.** Per Fable's explicit ruling: route
  to Opus, he owns first-line review, no fix until reviewed.
- **No relaunch attempted.** Per the arc's own law ("if it dies at cycle 3,
  that is a RESULT, not a failed run"), this crash IS the result for this
  launch. Item 8's endurance run restarts only on a pinned fix, once one
  lands and is reviewed.
- **A candidate fix direction exists but is deliberately not written here**
  (this doc reports the finding, not the remedy) — plausible shapes include
  changing the invariant check to be stackable-aware (compare against the
  entry's *effective* full state rather than literal `u32::MAX`), or
  restructuring `split_off_one` to preserve last-entry-only-partial by
  construction. Opus's call, not pre-empted here.
