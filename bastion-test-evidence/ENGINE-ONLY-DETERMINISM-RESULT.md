# ★★★ THE ENGINE IS TICK-DETERMINISTIC. THE CLIENT IS BAR 2's ENTIRE CAUSE.

> # ★★ RESULT STANDS BUT THE SAMPLE IS TOO THIN TO CERTIFY ON — being re-run at VD=6
>
> The run promoted **9 chunks across 3 ticks** and then stayed flat for the
> remaining **7,100 ticks**. 6-of-6 cross-host agreement on that is real, and it
> is not a basis for declaring a certification bar green.
>
> Cause, measured: `COLONY_PRESENCE_VIEW_DISTANCE = 1` is a 3×3 area. The colony
> loads its nine chunks and never needs another — **93 jobs were claimed and
> `colonist arrived at job site` fired ZERO times**, so nothing ever moved to pull
> in new terrain.
>
> A client uses **VD=6 (13×13 = 169 chunks)**. The view distance is now an env
> knob (`BASTION_COLONY_PRESENCE_VD`, default 1 so existing runs are unchanged)
> and the arm re-runs at 6 — a comparable exercise to the driven arm's 304 chunks
> rather than a token one.
>
> ★ Declaring bar 2 green off 3 promoting ticks would have been the same vacuous
> green this document's own precondition caught an hour earlier, one level up.


`provheadless` — `provtravcap`'s env with **no client**. The autofound colony
carries its own `Presence`, so the server requests terrain for itself.

## Precondition — passes this time

| | |
|---|---|
| `autofound spawn resolved` | 1 in every run (the real-terrain unblock worked) |
| `colony presence created` | 1 in every run |
| **promoting ticks** | **3** — non-zero, so the arm is **not** VOID |
| max `pending` | 9 (VD=1 ⇒ a 3×3 chunk area) |

## Result: 6 of 6 comparisons identical, **including across hosts**

```
tick 31 .. 59   pending=9, promoted=0     ← request arrives at tick 31, EVERY run
tick 60         promoted=4, pending=5
tick 61         promoted=4, pending=1
tick 62         promoted=1, pending=0
```

| comparison | tick-seq | membership |
|---|---|---|
| h1 twin1 vs twin2 (same host) | **IDENTICAL** | IDENTICAL |
| h2 twin1 vs twin2 (same host) | **IDENTICAL** | IDENTICAL |
| **all 4 cross-host pairs** | **IDENTICAL** | IDENTICAL |

★ **The request arrives at tick 31 in all four runs, on two different physical
machines.** That is the exact quantity that varied between **125 and 154** in all
38 client-driven pairs. Remove the client and it is constant.

## What this settles

**Bar 2's letter fails and bar 2's intent passes.**

| reading | verdict |
|---|---|
| *"twin runs state-identical incl. chunk timing"* **with a live client** | **FAILS** — 38/38, cause is cross-process arrival |
| *"the engine is deterministic with loading inside it"* | **PASSES** — 6/6, tick-exact, across machines |

Every server-side candidate was eliminated by measurement first (#89's ten, the
chunk-send ordering fix that ran 11,400× and changed nothing, the request barrier
that engaged 226× and only moved the divergence to a boundary). **This shows why
none of them could work: there was nothing wrong on the server side.**

## ★ Stated limits — this is a smaller test than the driven arm

| | headless | driven |
|---|---|---|
| chunks promoted | **9** | 304 |
| promoting ticks | **3** | ~150 |
| observer | static colony, VD=1 | moving client, VD=6 |

**A static 9-chunk request is a far thinner exercise than a traversing client.**
What it proves is that the promotion path — request → barrier → generate →
promote — is tick-deterministic when the input is. It does **not** prove the
engine stays deterministic under a moving observer, because nothing headless
moves.

★ The cross-host agreement is what makes even this thin sample worth something: 6
of 6 across two machines is a stronger statement than twins on one host, and it
is the shape that would expose thread-scheduling or hardware-timing dependence if
either existed.

## The decision this hands to Ben

The row's own roadmap criterion (capped/uncapped overlap) **passes**. Bars 1 and
3 **pass**. Bar 2 fails **only** with a client in the loop, and the engine
underneath is clean.

**Whether the certification means the engine or the engine-plus-client is a
scoping call, not an engineering one** — and it is now a call with a measurement
on both sides rather than an open row.
