# THE DRIVER ANCHORED 60% OF RUNS AT THE WORLD ORIGIN

**Found while scoring certification BAR 1 on banked data. No new VM spend.**

## The chain, in the order it actually ran

1. **BAR 1 needed the capped/uncapped promotion distribution.** Censusing
   `promoted=` per tick over 62 runs, the per-run TOTAL took only three values:
   **{196, 242, 304}**.
2. **One twin pair disagreed with itself** — `0200/i1` twin1 = 11,377 ticks /
   **304** promoted, twin2 = 11,408 ticks / **242**. The LONGER run promoted
   FEWER chunks, which truncation cannot order that way. Registered both
   branches before reading further (`PROMOTION-MEMBERSHIP-PREREG.md`).
3. **62 runs collapse to exactly 3 distinct promoted key SETS**, nested on a
   shared **196-chunk core**: `196 ⊂ 242`, `196 ⊂ 304`, and `242 ∩ 304 = 196`
   exactly — so the two extras are disjoint.
4. **Geometry named the mechanism.** The +46 extra sits at chunk
   **x[0..6] y[0..6]** — the WORLD ORIGIN, 470 chunks from the play area, and
   exactly a view-distance-6 square clamped at the world edge. The +108 extra is
   a full 13×13 VD-6 square centred on **(475, 500)**.
5. **The demand census carries the origin keys**, so the client genuinely
   *requests* them; the server serves them at exactly 4/tick, cap-saturated,
   draining 46 pending in 12 ticks. The server is behaving correctly.
6. **The driver's own log says it outright:**

```
class 242:  player pos at script start: Vec3 { x: 0.0,     y: 0.0,     z: 0.0 }
class 304:  player pos at script start: Vec3 { x: 15216.5, y: 16016.5, z: 419.5 }
```

`15216.5 / 32 = 475.5` and `16016.5 / 32 = 500.5` — **exactly** the (475, 500)
centre of the +108 block. `0.0 / 32` → the clamped 7×7 at the origin.

## Root cause

`client/src/bin/bastion_playtest.rs` — a **fixed** warm-up spin followed by ONE
read of the position, with a silent fallback:

```rust
for _ in 0..(TPS * 2) { client.tick(..); }          // a hope, not a wait
let mut current_pos = own_pos(&client).unwrap_or_else(|| {
    warn!("no Pos component readable yet; defaulting to origin");
    Vec3::zero()                                    // <- a DIFFERENT EXPERIMENT
});
```

The driver then anchors the god-camera there, and the client streams terrain
around chunk (0, 0). **There is no neutral position to fall back to**: the origin
is not a degraded answer, it is a different condition. The `warn!` fired into a
log nothing scored, so both outcomes rendered identically.

**Exposure: 41 of 68 runs (60%) anchored at the world origin.**

## What the measurement did to my own alarm

My first reading was that this confounded the whole corpus and refuted #89's
"platform-level, not program-controlled" closure. **It does not.** Joining anchor
class to membership per pair:

| | pairs |
|---|---|
| both twins REAL | 11 |
| both twins ORIGIN | 20 |
| **SPLIT** | **1** (`0200/i1`) |

**30 of 30 matched-input pairs have IDENTICAL promoted membership.** The single
differing pair is the single pair whose *inputs* differed. So:

- #89's twin-pair measurements are **not** broadly confounded — 31 of 32 pairs
  had matched anchors, and its closure stands.
- The i1 membership split, which I was one step from filing as engine
  nondeterminism, is a **harness input difference**.
- BAR 1 gets **stronger**, not weaker: membership is identical on 30/30 pairs
  with matched inputs, and the capped, uncapped, and cap=8-plant runs land on
  the **identical** 242-key set — the cap changes the SCHEDULE and nothing else.

Recording the correction because the alarm was mine and the data killed it.

## Fix

Wait on the condition, not a clock; keep the settle unchanged; and **refuse**
rather than guess — exit non-zero so a run that could not be anchored can never
be scored as one that was. Sampling still happens after the settle, so the fix
changes only whether `Pos` exists, not when it is read.

## REGISTERED OPEN QUESTION — not resolved here

`0229/r1a/b/c` are **O/O yet land in class 196 with no origin block at all**,
unlike every other O/O pair. Same script, same arena config, same anchor line.
Their demand census contains **zero** origin keys, so the client never asked.
Candidate mechanism, **untested**: the `BastionCameraAnchor` message racing the
terrain request through server-side request validation
(`server/src/sys/msg/terrain.rs`), so requests sent before the anchor is
recorded are rejected and never reach the census. **Named, not concluded** — the
fix above removes the whole class regardless of which way this falls.
