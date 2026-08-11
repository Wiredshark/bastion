# ITEM 8 ENDURANCE RUN — CRASHED at ~23.6 sim/wall-min (tick 45000), a real
# engine panic, unrelated to the colony-presence fix or the #85 work

**Status: reporting immediately, not waiting for the ~11:44 EDT heartbeat —
the run already ended well before that check-in was due.**

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

## Why this arc's endurance run specifically found it

`try_merge` is not called anywhere in `bastion_jobs.rs` — it's generic
vanilla item-pickup/merge machinery, triggered whenever a dropped item lands
near (or gets hauled onto) an existing stack. **No prior run in this arc ran
long enough, with a healthy sustained farm+haul+eat economy, to accumulate
enough merge/split cycles to hit this.** In the ~23 minutes before the
crash: `preempt_attempts` climbed 0→12, multiple `"hunger restored"`
completions fired, `food_stock` reached and held at 54, and the haul/farm
loop was actively cycling (`"haul delivered"`, `"job claimed"`,
`"colonist arrived at job site"` lines throughout). **This is the arc's
first-ever sustained, multi-generational item-stacking workload — exactly
what a duration test is for.**

## Not related to this session's other work

- **ROW-COLONY-PRESENCE** (`ea2cfa5192`) and its acceptance leg never
  exercised the farm/haul economy long enough to hit repeated merges — its
  15-minute run had `food_stock=0` throughout (no farm designated in that
  leg, food came from a single `dropall`).
- **#85's fields** (`5d905a247d`) are diagnostic-only additions at the
  ULTIMATE FAIL-SAFE emit site — they read state, never mutate it, and were
  never touched by anything running against this endurance server (isolated
  `cargo check -p bastion-server` only, confirmed via unchanged PID/binary
  mtime after each build).
- **This is a genuinely new, previously-undiscovered finding** in
  vanilla-inherited item-stacking code, surfaced purely by sustained
  duration.

## Evidence

    bastion-test-evidence/live-playthrough/server-stdout-item8-endurance-v2.log  (287KB, stable)
    bastion-test-evidence/live-playthrough/server-stderr-item8-endurance-v2.log  (228 bytes, the panic)
    bastion-test-evidence/ITEM8-LAUNCH-RECORD-V2.md                              (the launch this crashed from)

## Not yet done, awaiting a ruling

- Root-causing the actual corruption site (which caller of `try_merge` or
  its sibling split/append paths left a non-last item partial) — not yet
  investigated, this doc is the prompt report, not the fix.
- No relaunch attempted. Per the arc's own law ("if it dies at cycle 3, that
  is a RESULT, not a failed run"), this crash IS the result for this launch
  — reporting it rather than quietly relaunching a v3 without a ruling.
