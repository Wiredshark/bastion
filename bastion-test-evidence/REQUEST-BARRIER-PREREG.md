# REQUEST-SIDE BARRIER — A/B, pre-registered

One axis: `BASTION_REQUEST_BARRIER_TICKS=50`. Written before any run.

## Why this and not another guess

The cause is **measured, not theorised**: 38 of 38 twin pairs diverge first on
`pending` (the client's request arriving on a different tick), 0 of 38 on
`promoted`. Observed jitter is small — e.g. `tick 125` vs `tick 130`.

The barrier holds message reads until `tick % 50 == 0`, so both arrivals are
consumed on the same tick and everything downstream re-aligns. **A boundary of 50
against ~5 ticks of jitter is a 10× margin.**

## Bars

| bar | PASS | FAIL |
|---|---|---|
| **P — precondition** | `request barrier OPEN` appears in the fix arm and is **absent** in control | absent in fix ⇒ **VOID**; present in control ⇒ env leaked ⇒ **VOID** |
| **1 — control reproduces** | control still diverges first on `pending` | control identical ⇒ phenomenon gone at this tip ⇒ whole A/B **VOID** |
| **2 — the barrier works** | fix arm tick-sequence **IDENTICAL** ⇒ **BAR 2 OF THE CERTIFICATION PASSES** | still differs ⇒ see the readings below |
| **3 — membership unharmed** | promoted key sets identical within each pair | diverge ⇒ the barrier **broke** what the release barrier had pinned |

## ★ The predicted outcome, and the two ways it can be wrong

**Prediction: the fix arm goes identical.** If arrival jitter is the sole cause
and jitter (~5 ticks) is far below the boundary (50), collapsing it should remove
the divergence entirely.

| outcome | reading |
|---|---|
| fix identical | arrival jitter **was** the whole cause — bar 2 passes |
| fix diverges **later** | jitter is one part; something else also differs, and the new first-difference tick names where |
| fix diverges **the same** | arrival timing is **not** the cause after all, and `BAR2-LOCALIZED.md`'s conclusion is wrong despite being 38/38 — the classification would then be describing a symptom that co-occurs with the cause |

★ The third outcome is the dangerous one: **a 38/38 unanimous measurement can
still be a symptom rather than a cause**, and no amount of unanimity fixes that.
Writing it down first is the only protection.

## ★★ The risk this bar exists to catch

Holding requests changes **when** terrain loads. A barrier could make runs look
identical simply by making them **uniformly slower** — determinism bought with
latency, not with correctness. Bar 3 (membership) is one guard; the other is that
the promoted **set** must stay 304, matching every banked capped run. **A fix
that pins the schedule by changing what loads is a regression, however green the
fingerprint looks.**

## Preconditions above every verdict

1. Both twins boot and carry a terminator.
2. `provtravcap` (capped TPS), matching every banked measurement of this row.
3. The barrier witness must fire on boundary ticks only — its count should be
   roughly `total_ticks / 50`, not every tick. A count equal to the tick count
   would mean the gate is inert.
