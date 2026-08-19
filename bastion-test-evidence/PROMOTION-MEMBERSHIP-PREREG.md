# PROMOTION MEMBERSHIP — pre-registered, written before the key sets are read

## What forced this

Scoring certification BAR 1 (capped/uncapped promotion-distribution overlap) on
the banked corpus, the per-run promotion TOTAL took three values {196, 242, 304}
across 54 capped runs — and one twin pair disagreed with itself:

| run | ticks | promoted |
|---|---|---|
| `0200/i1` twin1 | 11,377 | **304** |
| `0200/i1` twin2 | 11,408 | **242** |

Same host, same seed, same commit. **The LONGER run promoted FEWER chunks.**
Truncation cannot produce that ordering, so "the runs were cut at different
points" is excluded before the measurement starts.

## The measurement

The census already prints `keys=[...]` — the actual chunk keys promoted each
tick. So membership is directly observable, not inferred from a count. For every
twin pair: build the promoted-key SET per run and compare.

## Both branches, registered now

| outcome | reading | consequence |
|---|---|---|
| **key sets IDENTICAL**, only order/tick differs | membership is deterministic; the residual is purely SCHEDULE | certification may proceed with timing out of scope — the thing being certified is intact |
| **key sets DIFFER** | membership is NOT deterministic | **BAR 1 scores FAIL.** "Certify membership, timing out of scope" is no longer available, because membership is the half that moved. The tick-loading row is NOT certifiable at this commit |

## Preconditions checked ABOVE the verdict

1. `keys=[]` must be non-empty on promoting ticks — an empty key list makes every
   set identical and the comparison VACUOUS ([[null-needs-a-couldnt-happen-witness]]).
2. Both twins must boot (`ready to accept` = 1) and carry a terminator.
3. Set sizes must equal the promoted totals already counted — if they disagree,
   the key list is truncated and the instrument, not the engine, is the subject.

## What would make me withdraw this

If the {196, 242, 304} spread tracks a CONFIG difference across fans (seed,
arena, view distance) and the i1 pair turns out to span two configs, then the
pair is not a twin pair and this whole document is measuring a mislabel. Config
provenance is checked for the i1 pair specifically, first.
