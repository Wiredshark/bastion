# DROP-TOSS WITNESS + IMAGE-LEVER VERIFICATION — pre-registered

One fan, two registered questions.

## Q1 — does the image lever actually pay? (A2's before/after)

`vm-fan.sh` now resolves the newest dated image at launch, and
`bastion-golden-0819-0855` was baked minutes ago at tip `72ede1a0d2`.

| | |
|---|---|
| **before** (2026-08-19 fans, 96 host-builds) | median **164 s**, total 257.3 min |
| best floor previously observed | **49–50 s** |
| **registered prediction** | median **< 90 s** on this fan |

| outcome | verdict |
|---|---|
| median < 90 s | **lever CONFIRMED** — and the earlier bake's failure is retro-explained by the source-image bug |
| median ≥ 90 s | **lever REFUTED at this tip** — the image is fresh, so freshness is not what costs the 164 s, and §1's 177 min/day estimate must be withdrawn |

★ This is a real falsifier for a number I have quoted twice. A fresh image that
still builds slowly would kill the largest lever on the board.

## Q2 — are the seeds created at all?

`BASTION_DROP_TOSS_DIAG=1` makes `emit_drop` log every item drop. Until now
`emit_drop` logged nothing, so *"16 seed items created and never collected"* and
*"no seed items created"* were indistinguishable — and the whole seed-conservation
chain rests on assuming the first.

| outcome | reading |
|---|---|
| collapsed run shows **2 seed drops per harvest** (`FARM_SEED_YIELD`) | the yield fires; the loss is **downstream** (haul/claim), and the conservation invariant fails at *recovery* |
| collapsed run shows **fewer/no seed drops** | the yield itself does not fire, and the invariant fails at *emission* — a different defect entirely |

**Precondition:** the `bastion: drop toss` emit must appear at all. Absent ⇒ the
env did not reach the server and the arm is **VOID**, not a zero.

★ Registered before the data because both outcomes are interesting and I have
already been wrong once on this chain — my "seeds scatter out of reach" reading
died to arithmetic (horizontal toss velocity 0.5 ⇒ lands within half a block).
