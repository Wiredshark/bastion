# PRE-REGISTRATION — ITEM 14 bar 1b re-run (Arc 3)

Written **before** the leg. Bar 1b was banked, not failed, and its premise has
since changed — so this is a re-run against a different world, not a retry.

## Why it was banked, and why that reason is now void

Bar 1b was banked on **colonist target-acquisition**: the bar could not be
scored because guarded colonists never engaged. The improvement list records the
same fact from the player's side — *"339 `FLEE — drive preempts work`, most at
`health=1.0`; in 45 samples not one colonist was ever in a `Defend` drive; Guard
XP across the whole session: 0."*

Those are one fact, and `703039d927` names its mechanism. `JobKind::Guard` sat in
the `false` group of the moot predicate — **`false` means always moot** — so a
guard assignment was destroyed `6.0/(1+0.2·level)` seconds after the colonist
reached the post (6.00s at level 0, 3.00s at level 5; the sessions logged
exactly 6s and 3s). Guard assignments are minted **only** by the paint and
nothing regenerates them, so the destruction was permanent.

**Both ITEM 14 consumers gate on "the board still resolves my active job."** With
the assignment gone there was no flee suppression (axis 2) and no mode response
(axis 1). So bar 1b was not measuring a bravery threshold that failed to hold —
**it was measuring colonists who no longer had a guard job at all.**

> A banked bar dies only when its own reason tests absent. This one's reason is
> now a fixed defect with a derived-and-matched timing signature, so the bar is
> re-runnable — but it has **not** been re-measured, and nothing below is a
> claim about the outcome.

## The prediction

**PASS requires all three:**

1. **The assignment SURVIVES.** A guard reaches its post and the job is still
   resolvable ≥60s later. Witness: no `job moot` for a `Guard` kind, and the
   post still present in `board.designated` with a live job behind it.
2. **Guard XP > 0.** The flat zero was downstream of destruction; if XP is still
   zero with assignments surviving, the earning path is a *separate* defect.
3. **Axis 2 measures in both directions** — a brave colonist HOLDS and a timid
   colonist FLEES **at identical health, on the same threat**. This is the bar's
   actual content and the only one that tests parameterisation rather than
   plumbing.

**FAIL / VOID branches, named now:**

| Observation | Means |
|---|---|
| Assignments survive, **but nothing ever approaches the post** | VOID, not fail. The bar needs a threat that arrives; a colony nothing attacks cannot score axis 2. Check the raid tick fired before reading anything else. |
| Assignments survive, XP still 0 | Bar 1b's original blocker was **only partly** the moot bug. A second defect in the XP path — file it, do not re-bank 1b. |
| Brave and timid behave **identically** | The real bar-1b failure: bravery is not coupled to the hold decision. This is the outcome the ruling most needs to know about, and it is now measurable for the first time. |
| Both flee at `health=1.0` | Flee still preempts unconditionally — the suppression consumer never reads `guarding`. Different defect from the moot bug, same symptom. |
| No `Guard` job minted at all | Upstream of this bar entirely: the paint or the generator. Not an item-14 result. |

## Contaminants in the same binary

State them now so a moved number has a candidate list:

- `703039d927` — the moot fix itself (**the intended change**).
- `d0f41f5553` — cancel now prunes `cook_stations`/`beds`. Should not touch
  guards; if guard counts move, suspect it.
- `57aeb2707a` — the food-surplus deposit trip. Changes what idle colonists do,
  which **changes who is standing where when a threat arrives**. Plausible
  confound for axis 2 timing.
- `4100485461` — census `engaged`. Instrument only.

## What this leg does NOT test

- **Axis 1 (fight vs alarm escalation).** Bar 1b is the hold-vs-flee axis only.
- **Patrol**, as distinct from post — a patrol assignment oscillating at tick
  rate is a separate open finding.
- Whether a guard **wins**. Holding is the bar; the combat outcome is not.
