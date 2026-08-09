# FINDING — ONE PRODUCER'S REPORT-ONLY ENTRY **SILENCES** THE OTHER'S PLAYER MESSAGE

**Task #47, read 2026-08-08 at `0fb7ca07b7`. ★ Every line cited was read.**
★★ **Filed, not fixed — and it is about to get worse, see the site-6 section.**

## THE MECHANISM

**`board.blocked_regions` has TWO live producers:**

| producer | site | `notified` | player message |
|---|---|---|---|
| ★ **`plan_access`** *(carve-planner gave up)* | **`13725`** | **`true`** | ★★ **emitted INLINE, in the same block** |
| ★ **`route_exhausted`** *(`stuck_strikes >= PERSIST_ESCALATE_STRIKES`)* | **`12260`** | **`false`** | **deferred drain fires it (`16269`)** |

★★★ **Both guard on the same dedup:**

```rust
let already_recorded = board.blocked_regions.iter().any(|b| b.region == region);
if !already_recorded { board.blocked_regions.push(...); /* + plan_access's chat emit */ }
```

> ## ★★★★★★★ **DEDUP IS BY `region` ALONE — AND `plan_access`'s CHAT EMIT SITS *INSIDE* THE DEDUP GUARD.**

### ★★★★★ THE CONSEQUENCE

1. `route_exhausted` records region **R** → the drain says
   *"Colonists have repeatedly failed to reach a designation at (x,y,z)."*
2. Later the **carve planner gives up on the same region R** →
   `already_recorded == true` → ★★★ **the push is skipped AND the inline chat emit
   is skipped with it.**

> ★★★★★ **The player is told colonists kept failing. They are NEVER told the
> designation is structurally unreachable.** ★ **Two different causes with two
> different remedies** — *"try again / clear the path"* vs *"this cannot be
> reached at all"* — **and the weaker one wins by arriving first.**

★★ **Order-dependent and permanent: whichever producer touches a region FIRST
owns the player's explanation for the rest of the run.** *(The reverse order is
benign — the stronger message already went out.)*

### ★★★★★★ AND IT DEFEATS THE FIELD BUILT TO PREVENT EXACTLY THIS

**`source` exists because *"two mechanisms both landing in `blocked_regions`
would be INDISTINGUISHABLE"* (`3806`).** ★★★ **The attribution field is present
and correct — and the DEDUP KEY IGNORES IT.**

> ★ **The instrument that distinguishes producers was added; the logic that acts
> on producers still cannot tell them apart.**

## ★★ THE DOC IS STALE — ON THE FIELD WHOSE JOB IS ATTRIBUTION

**`source`'s own doc (`3802-3813`) says:** *"currently always `plan_access` (the
carve-planner failure site, **the only producer**)"*, and that the second
candidate *"was measured and then **parked** (n=0 demonstrated cases)."*

★★★ **That describes a task #61 chop probe. ★★★★★ `route_exhausted` (Row B′,
2026-08-04) is a DIFFERENT, LATER, LIVE producer — and the doc was never
updated.**

> ★★ **True when written, false within days** — *the same pattern as my own
> superseded invariant.* ★★★★★ **A doc-comment asserting "the only producer" is
> exactly the sentence a reader trusts instead of grepping**, and it is the
> reason this defect reads as impossible until you list the push sites.

## ★★★★★★★ SITE 6 MAKES THIS WORSE — AND THAT IS WHY IT IS FILED **NOW**

**`route_exhausted`'s gate is `stuck_strikes >= 3`. ★★★ For self-jobs
`stuck_strikes` never accumulates today** *(measured: 0 across all 660 ticks of
job 33)*. **Site 6's re-claim makes it accumulate — that is the point of the
change.**

> ★★★★★ **So `route_exhausted` entries become substantially MORE common after
> site 6, and each new one is a region whose future `plan_access` message is
> pre-suppressed.**

★ **Registered as D5/C3 in `SITE6-DELTA-REGISTRATION.md`** *(the count rise is
expected; this finding is about the MESSAGE, not the count)*.

## ★★★ THE FIX, WHEN IT IS SCHEDULED — AND THE TRAP IN IT

**Obvious fix: dedup on `(region, source)`.** ★★★★★ **DO NOT SHIP THAT ALONE.**

> ★★ **The dedup exists to stop a region being *"re-recorded (and re-notified)
> every tick it stays blocked"* (`3912`).** ★★★★★ **Widening the key to include
> `source` re-opens that door for any THIRD producer, and doubles the notification
> rate for a region both current producers hit.**

★★★ **The real shape: separate the ENTRY from the NOTIFICATION.** *Dedup the
entry by `region` as today; track notification per `(region, source)` so each
distinct CAUSE announces once.* ★ **The emit must come out of the dedup guard
either way — that is the actual bug, and it is one line's worth of structure.**

★★ **ACCEPTANCE, when built:** *record `route_exhausted` on a region, then force a
`plan_access` failure on the SAME region, and assert BOTH messages fire.*
★★★★★ **A one-producer test cannot see this** — *same shape as Fixture 2's
one-colonist trap, found the same day.*
