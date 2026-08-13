# S1 SENTINEL — **RESULTS & ROW DISPOSITION**

Scored against `S1-SENTINEL-PREREG.md` (`0b39f4fa2b`). Engine tip `a9b50b373d`.

## THE SCORE — **4 PASS, 0 FAIL. Every plant fired.**

| bar | verdict | evidence |
|---|---|---|
| **S1-A** famine fires once | ✅ PASS | 517 zeros ⇒ `fired == [9]` |
| **S1-B** sawtooth stays silent | ✅ PASS | 341 samples, every zero-run exactly 9 ⇒ `[]` |
| **S1-C** boundary, both sides | ✅ PASS | 9 ⇒ silent; 10 ⇒ fires once |
| **S1-D** live emit survives | ✅ PASS | `tick=3000 consecutive_zero_samples=10` |

| plant | required red | observed |
|---|---|---|
| `==` → `>=` | S1-A red | **508 firings** (indices 9–516) instead of 1 — the exact number the prereg named. S1-B, S1-C stayed green |
| reset arm deleted | S1-B red | *"the sentinel must stay silent"* — S1-A, S1-C stayed green |
| **delegation** (`+= 1` → `+= 2`) | live emit moves | **tick 3000 → tick 1500**, exactly half |

Restored: unit **121/121**, live emit back at `tick=3000`.

## THE ROW WAS NOT WHAT THE BRIEF SAID — and the prereg said so first

`colony_terminal_should_fire` was **already** a pure predicate with a
both-polarity test. Reporting "predicate extraction" as this row's work would have
been claiming credit for a commit that already landed.

The real gap: the predicate judges **one already-computed streak**. The increment, the
**reset on any nonzero sample**, and the edge trigger lived **inline inside a 20 000-line
function**. v4's famine and v5's sawtooth are claims about a **sequence**, and the
predicate cannot separate them at all — both collapse to *"is this number 10"*. The
sawtooth's entire content is the reset arm, and **no test touched it.**

## ★ THE PLANT MY OWN TEST COULD NOT CATCH — named, then closed at the live tier

Prereg plant 3 was *"make the site stop calling the extracted function ⇒ a delegation
test goes red."* **My delegation test cannot do that.** It drives `step` and `scan` and
asserts they agree — both extracted, so a site that reverted to inline arms would sail
past it.

Rather than quietly rescore the plant, I ran the test that *can* discriminate: mutate
`colony_terminal_step` and watch the **live** emit. `+= 1 → += 2` moved it from **tick
3000 to tick 1500**. The shipping site demonstrably routes through the extracted
function. **A unit-tier plant was insufficient and the live tier closed it** — which is
the same lesson as gate-must-test-live-path, arriving from the other direction.

## ⚠ S1-D WAS VOID BEFORE IT WAS GREEN

The first live run showed **zero** emits. That reads as the exact regression S1-D exists
to catch — a green harness over an inert feature. It was not: the run reached only
**tick 1200**, and the sentinel fires at **tick 3000**. Premise checked before the result
was reported, and the fix derived rather than guessed: the emit carries
`consecutive_zero_samples=10` at tick 3000 ⇒ **300 ticks per sample** ⇒ the run must pass
tick 3000. Held past it, and the emit reproduces exactly, matching the pre-refactor log.

**Third void-not-red of this session** (A4's restart, W4's water radius, now S1-D). The
pattern is worth naming: *a red I want earns more scepticism than a green.*

## WHAT I DECLINE TO CLAIM

- **Not** that the 517/341 shapes are the literal v4/v5 corpus traces. They are the
  shapes registered in the prereg; the corpus was not re-read to confirm them. If the
  real traces differ, the bars need re-running against them — registered as open.
- **Not** that S1 is wired to anything. **LIVE-ABORT STAYS OUT**: v3 would have
  terminated 79 minutes early had a naive version acted. It remains an observer.

## SESSION QUEUE STATE

1. ✅ Founding preset on real worldgen — **PASS**, disposition `f51213cc4c`
2. ✅ Arena trees / F8-C1 — **CLOSED**, disposition `793df9401a`
3. ✅ S1 sentinel scored-bar — **PASS**, this document
4. → Next: roadmap open items (tick-driven loading spec, save/load colony-state
   persistence, §8 N2's widget tier), plus the three successors this session opened:
   **the water gate** (preset founds on a flat lakebed), **the relief emit's
   `Some(datum_z)` blind spot**, and **a driver-binary freshness guard**.
