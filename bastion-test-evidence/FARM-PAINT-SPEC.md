# FARM-PAINT SPEC — surface-relative resolution + refusal (DECISIONS #63)

**Blob for every cite: `90dc4d11f0`.** Line numbers move — re-locate by symbol.

## §1 — THE DEFECT, AND THE CODE NAMES IT ITSELF

Live-observed: a 49-cell farm plot produced **zero jobs for 4.5 minutes**, with
**no message and no feedback** — indistinguishable from "working, nothing to do
yet." Root cause was a paint one block above real ground.

**The paint path (`bastion_jobs.rs:4640-4647`) admits the gap in its own comment:**

> *"v1 farms are FLAT plots: `region.min.z` is the field's ground level
> (**per-column surface resolution is a slope extension**)."*

So the plot is registered with **one z for the whole region**, and the trigger
pass then requires `ground.is_filled()` at **exactly `plot.min.z`, per column,
zero tolerance.** Any column whose real ground differs is silently skipped.

> ## ★ THE DEFECT IS BROADER THAN THE BUG THAT FOUND IT
> This is not only "the player painted one block off." **Any non-flat ground
> silently drops columns** — a plot straddling a 1-block step loses every column
> on the far side, forever, with no signal. The operator error was the *cheapest
> possible reproduction* of a standing correctness limit, and v1's own comment
> already knew about it.

**Mine forgives this** (whole-box scan finds the block at any z in the box);
other kinds resolve surface-relative via **`z_extent`**
(`client/src/lib.rs:2808`: *"`z_extent: Some(_)` switches to the surface-relative
path"*, carried on the wire at `common/net/src/msg/client.rs:166-175`, handled at
`server/src/sys/msg/in_game.rs:337`). **Farm has neither.**

★ **The sibling-caller check runs BACKWARDS here:** normally we ask *"which
siblings also need this fix?"* Here the capability exists and **Farm is the
sibling that never got it.** Do not invent a resolver — **wire the existing one.**

## §2 — THE FIX

**1. Farm paint accepts `z_extent`** like the kinds that already do, taking the
same wire path. **No new resolver, no new message type.**

**2. Resolve per-column ONCE, at registration.** Store the resolved ground z per
column with the plot, rather than re-resolving in the trigger pass.

- **Determinism:** resolution happens at a single known point, from terrain state
  at that instant — not re-derived every tick from a possibly-changed world.
- **Observability budget** (the standing law): re-resolving per column per tick is
  a per-cell read on a hot path — *exactly the shape the observer-effect bisection
  indicted.* **Resolve once; store; read the stored value.** Cost is
  `columns × 1` at paint time and **zero per tick.**

**3. Keep the trigger's `is_filled` check** — but against the **stored per-column
z**, not `plot.min.z`. The check stays; only its reference changes.

## §3 — THE REFUSAL (the row's real player value)

> **If ZERO columns resolve to valid ground, the paint REFUSES — with a message.
> Never silent.**

**Wording, under the behavioral-claims standard** (a message may assert only what
its guard establishes — the law that corrected the blocked-designation text):

```
"No ground found under that field — nothing was planted."
```

- ✅ states what was **checked** (ground under the painted columns) and what
  **followed** (no plot registered).
- ❌ **not** *"invalid location"* — a judgement the guard cannot support.
- ❌ **not** *"the terrain is too steep"* — a **cause** it did not measure.

★ **Partial resolution is NOT a refusal.** If *some* columns resolve, register
the plot with those and **report the count** — *"field registered, 34 of 49
columns had ground."* A player who paints across a cliff edge should get the
usable part **and know they did.** Silence and refusal are both wrong there.

## §4 — WHY THIS IS THE LAW'S FIRST PLAYER-FACING COSTUME

The campaign has catalogued this defect in seven internal forms — `git status`
hiding ignored files, a type guard dropping a class, exit-0 on an empty log, a
sentinel inside the valid range, ANSI inside a value, a wholesale rewrite, a
control character in a regex.

> **AN EXCLUSION AND AN ABSENCE MUST NEVER RENDER IDENTICALLY.**

**Every prior instance was ours.** This one is **a player's**: a mis-painted farm
and a patient farm look exactly the same, and the player has no way to tell them
apart. That is why the row is worth more than the bug that found it.

## §5 — ACCEPTANCE

- **Planted-failure test (required):** paint a plot one block off the real ground
  **on purpose**, and **assert the refusal fires.** ★ *The assertion is on the
  refusal, never on the silence* — a test that passes when nothing happens cannot
  distinguish the fix from the bug.
- **Second planted case:** paint across a **1-block step**; assert **partial
  registration with the reported count**, not refusal and not silence.
- **Regression:** the existing flat-plot path still registers 49/49 and tills.
  5b's live repro (surveyed correct height → 13 TILL + 13 SOW) is the baseline.
- **Live gate:** per #62 this milestone gets a scored session; the scorecard's
  farm row upgrades to *"I painted a field on uneven ground and was told what
  happened."*
- **Budget:** `columns × 1` read at paint time, **zero per-tick.** State it.
