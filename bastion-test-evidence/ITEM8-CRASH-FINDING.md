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

★ **Precision note (Opus's read, folded in so no later reader hunts a diff
that doesn't exist): `PickupItem::pick_up` is UNMODIFIED.** #89 *added*
`split_off_one`; it never touched `pick_up`. `split_off_one` pushed the
split single as the new LAST entry specifically so the existing, untouched
`pick_up()` would pop exactly it. The producer of the violating state is
`split_off_one` alone.

★★ **And the invariant itself has been VACUOUS for the entire life of this
codebase until #89 (Opus's read, independently confirmed against the same
two facts this doc already cites):** work `try_merge`'s two cases —
non-stackable, `max_amount() == 1` and `amount()` is always 1, so the
invariant is trivially true; stackable, `max_amount() == u32::MAX`, so
`try_merge` only ever pushes a second entry on overflow past `u32::MAX`,
which never happens in practice, so the merge never produces a second
entry either. **Vanilla `PickupItem`s are effectively always single-entry.
The "items before the last" iterator has always been empty; the assert has
been passing trivially, never exercised, for as long as the type has
existed. `split_off_one` is what gave it its first population with any
content to check at all** — not a violation of an established, working
check, but the check's first real test.

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

★ **This is the sharpest instance this arc owns of a named-case
sufficiency claim failing anyway** (the standing law: "a confident comment
marks the spot nobody re-examined; name the excluded case"). The
`split_off_one` comment did everything that law asks — it named the
excluded case explicitly, in the method that causes it, at the moment it
was introduced. **And it detonated anyway**, because a doc comment cannot
page anyone when its own stated precondition becomes true; it can only be
found by someone who happens to read that exact method before the crash,
or — as here — by someone reading it after. **The law grows a clause from
this: a scoped-out failure mode is a scheduled crash unless it's tracked.**
A prediction written into a doc comment must ALSO become a ledger row or
rider with its trigger condition named as a live, checked thing — not just
prose sitting in the file it will eventually break, where nothing re-
examines it until the crash itself forces the read. This finding is that
re-examination, ~arriving on schedule.

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

## THE FIX — DIRECTION 1 (Fable's ruling: the eat path stops creating
## multi-entry stackable `PickupItem`s; the vanilla invariant is NOT
## redefined), awaiting Opus's commit review

**`split_off_one` no longer mutates `self.items` at all.** Signature
changed `-> bool` to `-> Option<Item>`: it still finds the first entry with
`amount() >= 2` and decrements it in place, but returns the duplicated
single as a VALUE instead of pushing it into the Vec. **Post-condition,
proven by test, not asserted by comment: `self.items.len()` is unchanged by
every call, `Some` or `None`.** The struct's documented invariant now holds
unconditionally, for every stackable `PickupItem`, always — not "usually,"
not "unless merge-checked."

**The one call site moved from reservation time to arrival/completion
time.** Previously: split at reservation, mutate the ground entity, let the
async vanilla `emit_pickup` path retry-consume the pushed single from the
bag. Now: reservation only tracks CAPACITY (`board.reserve`, already
independent of the Vec's structure — unchanged); at arrival, the colonist's
completion pass calls `split_off_one` directly on the ground entity, and
consumes the returned value immediately (hunger restored in the same pass,
no bag round-trip, no vanilla pickup event for this path). The "down to the
last unit" case is UNCHANGED — `split_off_one` returns `None` there and the
existing, always-correct `emit_pickup`/`pick_up()` vanilla path still
handles it, since a single-entry `PickupItem` was never part of the
violating shape.

★ **A note on the ruled "natural shape" (born-single-entry `PickupItem`,
Fable's suggestion) vs. what was actually built:** this fix does NOT spawn
a new entity/`PickupItem` for the split single at all. Opus's review
flagged the identity hazard that shape would introduce (a new Uid, and the
reservation/`EatFrom` job system is keyed on item Uid — a spawn-then-
reserve or reserve-then-spawn sequencing question, since entity creation is
event-driven and the new Uid wouldn't exist in the same tick). **This
fix sidesteps that hazard entirely rather than solving it**: since the
split now happens at arrival and is consumed immediately as a value (never
becoming an entity, never needing a Uid, never touched by the reservation
or job system at all), there is no new identity for anything to be keyed
on. The reservation and the `EatFrom` job continue to name the ORIGINAL
pile's Uid throughout, unchanged. This is the "alternative inside direction
1, named with reasons" Fable's ruling invited if the suggested shape didn't
fit the call sites — the call-site shape (async vanilla-pickup ceremony
built around a still-existing ground entity) made the immediate-consumption
alternative both simpler and hazard-free, so it was chosen over spawning a
second entity.

**The two eaters interleave correctly without ever growing `items`:**
sequential calls to `split_off_one` on the same entity within one tick each
decrement the same single entry in turn — no index games needed, since
nothing is ever pushed.

**Planted test, red pre-fix / green post-fix, by name:**
`split_off_one_never_grows_the_stack_even_under_repeated_splits_then_merge`
(`common/src/comp/inventory/item/mod.rs`) constructs the exact scenario —
repeated splits from a 40-stack down to exhaustion, then a merge-check
against a fresh drop of the same item, the precise operation that panicked
live at tick 45000. **Against the pre-fix implementation this would panic
on the merge-check** (the debug_assert this doc opened with); against the
fix, it passes — 3/3 tests green (`cargo test -p veloren-common --lib
split_off_one`), including the two pre-existing tests rewritten to assert
the corrected post-condition instead of the shape that used to crash.

**`split_off_one`'s ship-time comment is rewritten** to cite this test by
name and state the post-condition as the guarantee, replacing the old
"known residual … out of this row's scope" language that predicted and
then deferred the crash.

★★★ **The delta/seam analysis Fable required (checked concretely, not
assumed) lives in the fix's own commit messages** (`5509dc95c3` primarily,
with `e14795700e` for the rest) **and in `bastion-test-evidence/ITEM8-V3-
PREREGISTRATION.md`'s expected-delta table.** Short version: the "pile
amount changes during the walk" premise doesn't hold in EITHER design
(`PickupItem::amount()` was always total-invariant across a split); two
real regressions WERE found by checking rather than assuming
(`b5_pile_pickup_by_member`, the ROW-ITEM6-WITNESS-PACKET B2 falsifiability
counter, and the entity event log's `ItemEventKind::PickedUp` — both only
ever fired from the vanilla `InventoryManip::Pickup` handler this fix
bypasses for the common case) and both are now restored at the new
consumption site, entering through equivalent instrumented doors rather
than going dark silently.

**Status: pins compile clean (`cargo check` isolated, `cargo test` green
for the changed crate), awaiting Opus's commit review before landing.** No
relaunch until the reviewed fix is pinned.
