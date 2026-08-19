# ENGINE-ONLY DETERMINISM — the measurement that settles bar 2's scope

`provheadless`: **`provtravcap`'s env exactly** — real terrain, capped TPS,
provisioning census — with **no client**. The autofound colony carries its own
`Presence` (`COLONY_PRESENCE_VIEW_DISTANCE = 1`), so the **server requests
terrain for itself** and there is no second process to race with.

## Why this is the right experiment now

Three server-side candidates are dead by measurement:

| candidate | result |
|---|---|
| #89's ten | all excluded |
| chunk-send ordering | ran **11,400×**, changed nothing |
| request-side modulus barrier | engaged **226×**, moved the divergence onto a boundary |

And the cause is unanimous: **38 of 38 pairs diverge first on request arrival,
0 on promotion.** Since arrival is set by two independent clocks, removing the
second process is the only remaining way to test **whether the engine itself is
deterministic**.

## Bars

| bar | PASS | FAIL |
|---|---|---|
| **P — the arm produces data** | ≥1 promoting tick in both twins | zero promotion ⇒ **VOID** — a colony with VD=1 may request nothing, and an empty census is not a clean result |
| **1 — engine determinism** | tick-sequence **IDENTICAL** between twins | differs ⇒ the engine is nondeterministic **independently of any client** |
| **2 — membership** | promoted key sets identical | differs ⇒ same |

## ★ The two readings, and both are decision-relevant

| outcome | what it means for bar 2 |
|---|---|
| **identical** | The engine **is** deterministic. Bar 2's failure is caused entirely by having a networked client in the loop, and the bar is measuring the client rather than the thing the row built. **That makes it a scoping question with an answer attached.** |
| **differs** | The engine is nondeterministic on its own, the client was never the whole story, and the 38/38 arrival finding is a **symptom of something upstream** rather than the cause. Every conclusion in `BAR2-LOCALIZED.md` would need re-reading. |

★ I expect **identical** — every measurement points that way. Which is exactly why
the second row is written down: **the outcome I expect is the one I am least able
to judge fairly**, and the last two mechanisms I was confident about were both
refuted by their own A/Bs.

## ★★ The precondition that most likely bites

`COLONY_PRESENCE_VIEW_DISTANCE = 1` is a **3×3 chunk area** against a client's
13×13. An earlier attempt at a driverless terrain arm was **VOID at 6 promoting
ticks of 3,382** — *"a stationary colony in a pre-generated arena requests no
chunks."*

This arm uses `PITARENA=""` (**real terrain**, not the pre-generated flat arena),
so there is genuinely something to generate. **If promotion is still near zero,
the arm is VOID and says nothing about determinism** — it must not be reported as
"identical".
