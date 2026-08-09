# RE-SCORE ON WAVE 30 — **THE BIGGEST BIN IS NOT A DISAGREEMENT**

**Same unit as `ROUTER-VS-PROBE-DISAGREE.md`: a probe result WITH a real timeout.
★ Same n: 44.** ★★★ **Third column added, per DECISIONS: what the colony DID.**
★★ **It adjudicates PREDICTION QUALITY, not path truth.**

## ★★★★★★★★ THE TABLE

| router says route exists | probe from LAST TIMEOUT | probe from SPAWN | n |
|---|---|---|--:|
| ★★★ **TRUE** | ★★★ **TRUE** | ★★★ **TRUE** | ★★★★★ **18** |
| **FALSE** | TRUE | TRUE | **9** |
| **TRUE** | FALSE | FALSE | **8** |
| FALSE | FALSE | FALSE | 5 |
| ★★ **TRUE** | ★★ **TRUE** | **FALSE** | **3** |
| TRUE | FALSE | TRUE | 1 |

> ## ★★★★★★★ **THE LARGEST CELL — 18 OF 44 — IS UNANIMOUS AGREEMENT THAT A PATH
> EXISTS, FOLLOWED BY A LIVE TIMEOUT.**

★★★ **Add the `local=TRUE` row where only the spawn probe dissents and it is
21 of 44 — 48%.**

> ## ★★★★★ **IN NEARLY HALF OF ALL PROBED TIMEOUTS, THE ROUTER AND THE LOCAL
> PROBE BOTH CONFIRM A PATH AND THE COLONIST STILL FAILS.**

★★ **That is not a router-vs-probe question. It is a TRAVERSAL question, and the
original 41%-disagreement framing could not see it** — *it compared two
instruments to each other, and the biggest cell is where they AGREE and are both
unhelpful.*

## ★★★★★ THE THREE ORDERED BINS, AND THE ONE NOBODY NAMED

| bin | cells | n | reading |
|---|---|--:|---|
| **router-vindicated** *(probe over-promises)* | `router=F, probe=T` | **9** | ★ Class B's foundation erodes further |
| **probe-vindicated** *(router under-finds)* | `router=T, probe=F` | **9** | ★ the astar-reset family's territory |
| **both agree UNREACHABLE** | `F/F/F` | **5** | ★★ *both instruments right; the job genuinely can't be reached* |
| ★★★★★★★ **BOTH AGREE REACHABLE, LIVE FAILS** | `router=T, local=T` | ★★★ **21** | ★★★★★ **THE UNNAMED BIN, AND THE LARGEST** |

## ★★★★★★★★ AND THE COLUMN THAT MAKES IT CONCRETE: `min_distance_to_target`

**Seed 71, nine probed timeouts, ALL with router=TRUE and both probes TRUE:**

    3.7  3.3  3.4  4.3  17.4  3.7  4.3  3.4  18.4

> ## ★★★★★ **COLONISTS ARE FAILING **3.3 BLOCKS** FROM THE TARGET WITH EVERY
> INSTRUMENT SAYING THE PATH EXISTS.**

★★★ **`ARRIVE_DIST` is 2.5.** ★★★★★ **So they close to just outside arrival
tolerance and cannot cover the last block.** ★★ **This is not pathfinding. It is
ARRIVAL.**

★ **Seed 52 is the mirror case: router=FALSE on all ten, at 3.5-7.2 blocks — the
router never finds a route at all while the probe insists one exists.**

## ★★★★★★★ CORRECTION TO MY OWN CHOP MESSAGE

**I wrote: *"on seed 85 the live attempts side with the ROUTER against the
PROBE."*** ★★★★★ **WRONG. Seed 85 is `router=TRUE, local=TRUE, spawn=FALSE`.**

> ★★★ **The router said a route existed. The local probe said a path existed.
> BOTH agreed, and the colonist failed anyway.**

★★ **My error came from the spec quoting only ONE of the probe's two arms.** ★★★★★
**`from_last_timeout` (23 columns visited, from 8.5 blocks away) says TRUE;
`from_spawn` (65,325 columns visited) says FALSE.** ★ **The spec's *"TRUE, TRUE,
TRUE"* was the LOCAL arm — and I read a one-armed quote as the probe's whole
answer.**

> ★★★★★ **A field with two arms reported by one of them is the same failure as a
> label read for its value.** ★★★ **Seed 85 belongs in the LARGEST bin, not in a
> disagreement bin — which strengthens the conclusion rather than weakening it.**

## ★★★ WHAT THE TRAVEL ROW INHERITS

1. ★★★★★ **Its central target is ARRIVAL, not routing** — *48% of probed timeouts
   have unanimous path confirmation.*
2. ★★★ **`min_distance_to_target` clustering at 3.3-4.3 against `ARRIVE_DIST` 2.5
   is the specimen set** — *and it is already in the corpus, on every wave.*
3. ★★ **The two genuine disagreement bins are 9 and 9 — real, but each smaller
   than the agreement-and-fail bin.** ★ **Class B's erosion stands as a live
   question; it is no longer the biggest one.**
4. ★ **The 5 `F/F/F` cases are the honestly-unreachable set** — *the amnesty
   design's "392/425 cells honestly unreachable" population, and correctly
   latched.*

> ★★★★★★★ **THE ORIGINAL ROW ASKED WHICH INSTRUMENT TO TRUST. THE ANSWER IS THAT
> IN THE BIGGEST BIN, BOTH WERE RIGHT AND THE COLONIST STILL COULD NOT WALK
> THERE.**
