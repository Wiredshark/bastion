# THE RELIEF EMIT'S BLIND SPOT (F2) — **RESULTS & ROW DISPOSITION**

Scored against `RELIEF-EMIT-BLINDSPOT-PREREG.md` (`fa6400acaf`). Engine tip `bbef73f9e5`.

## THE SCORE — **3 PASS, 0 FAIL** *(E3 on a corrected denominator — see below)*

| bar | verdict | evidence |
|---|---|---|
| **E1** invisible case becomes visible | ✅ PASS | 6 lines with `datum_resolved=false`, `resolved=0`, `branch="absence"` |
| **E2** resolved case untouched | ✅ PASS | census **byte-identical**; `datum_resolved=true` the only addition |
| **E3** emits == attempts | ✅ PASS **(denominator corrected)** | **8 of 8** terrain-stage; the same run gave **2** before the fix |

| plant | required red | observed |
|---|---|---|
| `None`-arm emit removed | E3 back to the old count | **2 of 8**, only `datum_resolved=true` survives, the 6 absence lines vanish |

Restored: **8 of 8**, unit **123/123**.

## ⚠ E3's REGISTERED DENOMINATOR WAS WRONG

I registered **"24 emits from 24 attempts"**, justified by *"nothing founds under it, so
no attempt is short-circuited by `colony_exists`."*

That premise was measured under the **bound-0 plant binary** from the worldgen row — the
one where nothing could found. On the **restored** binary a colony founds partway through
the search, and `colony_exists` short-circuits the remaining **16** attempts *by design*:
§4 checks the one-colony boundary **first**, deliberately, because "your colony already
lives here" is true regardless of the ground.

So 24 was never the right denominator for this bar. The sound one is **attempts that
reach the terrain stage**: `24 − 16 = 8`, and the bar is **8 of 8**.

This is a denominator error of exactly the kind that makes a percentage lie — the numbers
never moved, the *unit* did. Recorded against myself, not restated as if I had meant it.

## THE BEFORE/AFTER, ON IDENTICAL CONDITIONS

| | relief emits | of terrain-stage | absence lines |
|---|---|---|---|
| before the fix | **2** | 8 | **0** — did not exist |
| after | **8** | 8 | **6** |

Six attempts that previously produced *nothing at all* now report `resolved=0`,
`branch="absence"`, and the hint they were measured against. That is the difference
between "unreachable" and "dry" being **observable** versus being inferred by counting
log lines against script lines by hand — which is how this gap was found, and is not a
method.

## WHAT I DECLINE TO CLAIM

- **Not** that the emit now fires on *every* attempt. It fires on every attempt that
  reaches the terrain stage. `colony_exists` short-circuits before it, **by design**, and
  that is the correct behaviour — but it means the phrase "every attempt" is still wrong,
  and I am not repeating my own earlier overclaim in a new form.
- **Not** that `datum=<hint>` on an unresolved line is a datum. It is the hint, labelled
  `datum_resolved=false` beside it.

## SESSION QUEUE STATE

1. ✅ Founding preset on real worldgen — PASS (`f51213cc4c`)
2. ✅ Arena trees / F8-C1 — CLOSED (`793df9401a`)
3. ✅ S1 sentinel scored-bar — PASS (`dcc0b950e9`)
4. ✅ The water gate (F1) — PASS (`95a597ec5a`)
5. ✅ **Relief-emit blind spot (F2) — PASS**, this document
6. → Next: **driver-binary freshness guard (F3)** — the stale driver that silently
   discarded targeted coordinates, caught only by its own echo; then the roadmap's open
   items.
