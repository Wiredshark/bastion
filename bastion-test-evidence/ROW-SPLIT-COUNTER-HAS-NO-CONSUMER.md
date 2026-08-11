# ROW: `b5_split_off_one_fired` INCREMENTS AND IS NEVER EMITTED

**Filed 2026-08-11 by the reviewer whose gate shipped it unported.** *Per
`readme/PACKET-CRAFT-CHECKLIST.md` entry 3 — a scoped-out or defective condition
gets a row, not a note.*

---

## THE DEFECT

**Two sites in the whole tree:**

    bastion-server/src/bastion_jobs.rs:4436    pub b5_split_off_one_fired: u32,
    bastion-server/src/bastion_jobs.rs:13757   board.b5_split_off_one_fired += 1;

**No reader. No emitter. No log line.** *The counter increments correctly, in the
correct branch (verified structurally: inside the `Some` arm, against the `else` at
`:13811`), and nothing can ever observe it in a live run.*

## THE CLASS — **this is not one field**

★★★ **The entire `b5_*` family is HARNESS-ONLY.** *They are `JobBoard` fields read
by accessors such as `Server::bastion_item6_witness_stats()`, which
**bastion-harness** calls. A live `server-cli` run never calls them.*

**Measured, not inferred** — v2's live server log:

    "b5_"              ->    0 occurrences
    "preempt_attempts" ->  150 occurrences

> **A `b5_*` counter is invisible in every live run, by construction.**

## WHAT IT COST

**Item 8 v3's fix-claim precondition witness.** *The counter was required at review
specifically so a silent `debug_assert` could be distinguished from an unexercised
trigger — the sit-trap law. It cannot do that in a live run.*

★ **The run's verdict survives** via a substitute precondition (v2's detonation at
23.6 min on the same scenario supplies the population; see
`ITEM8-V3-SCORING-PROCEDURE.md`'s dated amendment). **The cost was a witness, not a
result** — which is why this is a row and not a re-run.

★★ **A second cost, worth naming:** *an early-check step and a pre-registered
two-branch fork were both built on this channel's silence before anyone checked it
could speak.* **Two readings of a mute channel is the exclusion/absence law
consuming an hour of two lanes' attention.**

## THE FIX

**Port it to a live emit.** *Either:*

1. ★★★ **Fold it into the `"bastion food stock sample"` heartbeat — PREFERRED, AND
   VERIFIED FEASIBLE, not assumed.**

   *The emit site (`bastion_jobs.rs:6377`) already has `board` in scope — it calls
   `board.stockpile_at(cell)` eleven lines above. The counter is directly
   reachable.* **One field on an existing line:**

       info!(tick = tick.0, food_stock,
             splits = board.b5_split_off_one_fired,      // <- the whole change
             "bastion food stock sample");

   ★ *Checked because the row originally said "PREFERRED" about a site its author
   had not read — the same error this programme made twice today. It holds.*

2. Its own periodic `tracing::info!`, gated like the other live diags.

### ★★★★ THE PREFERRED FIX YIELDS A BETTER INSTRUMENT THAN THE ONE SPECIFIED

**The heartbeat is `tick % 300`, unconditional. So the counter arrives as a TIME
SERIES, not a final total.**

| what was specified | what the fix gives |
|---|---|
| one number, harness-only, at run end | ★ **a per-300-tick series, live** |
| "did splits happen at all" | **splits PER WINDOW — the rate condition, directly** |
| no per-cycle resolution | **per-cycle counts, so the splits:eats denominator works cycle by cycle** |

> ★★★ **The prereg wanted a rate and could only ask for a count, because a count was
> what the board could hold. Riding the heartbeat gives the rate for the same one
> line.** *The instrument defect, fixed the preferred way, produces the instrument
> the bar actually wanted.*

★★★ **Whichever: it must carry a LIVE-EMIT declaration** (checklist entry 1) —
*that entry exists for exactly this, and not running it is how this shipped.*

## ★★★★ THE GENERAL DEFECT, WHICH IS BIGGER THAN THIS ROW

> ## **THE `b5_*` NAMING GIVES NO HINT OF REACHABILITY. A reviewer sees a counter
> and cannot tell from its name whether it is live-visible or harness-only.**

**Every `b5_*` field is a potential repeat of this.** ★ *Not proposing an audit of
all of them here — that is a separate row if anyone wants it.* **But any future
`b5_*` added as a WITNESS for a LIVE run inherits this trap unless its LIVE-EMIT
declaration is written and checked.**

## PREVENTION — **spec-time, not review-time**

    "add a field to THIS LOG LINE"   -> live by construction
    "add a counter"                  -> harness-only by default

★★★★ **#85's six fields were specified as fields on a named emit site and came out
LIVE. This one was specified as a quantity and came out MUTE.** *Same builder, same
review, same day — the framing of the ask decided it.* **Ask for the line, not the
number.**
