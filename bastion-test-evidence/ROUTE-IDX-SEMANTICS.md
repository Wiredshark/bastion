# `route_next_idx_pinned` — PRODUCER READ, AND THE FIELD WAS BUILT FOR MY 41% QUESTION

**Read at `e5a288d9cc` (harness `3560-3576`) and `5f8cdf1392`
(`bastion_jobs.rs:4080-4094`). Semantics BANKED before the validation lands, so
interpretation is instant when it does.**

## §1 — WHAT THE FIELD ACTUALLY MEANS

```rust
let idxs: Option<Vec<usize>> = route_states.iter().map(|(_,_,idx)| *idx).collect();
idxs.filter(|v| v.len() >= 2)
    .map(|v| v.windows(2).all(|w| w[0] == w[1]))
```

| value | meaning |
|---|---|
| `true` | the waypoint index was **IDENTICAL at every recorded timeout** |
| `false` | it **changed at least once** between timeouts |
| ★ `null` | **fewer than 2 samples** ★ **OR any sample had no route at all** |

> ★★★ **`null` CARRIES TWO DIFFERENT FACTS.** `.collect()` into `Option<Vec<_>>`
> returns `None` if **any** element is `None`, and the `filter` returns `None`
> for **<2 samples**. **"Too few samples to compare" and "at least once there
> was no route at all" render IDENTICALLY** — and the second is a **substantive
> finding** while the first is an absence of data.

★ **The campaign's central law, inside the field built to diagnose the campaign.**
*(Not a defect in the answer — the raw `timeout_route_states` list still
distinguishes them. It is a defect in the SUMMARY, and the summary is what a
reader reaches for.)* **Read the list, not the flag.**

## §2 — ★★★ AND MY OWN READING NEEDED CORRECTING

I wrote that `route_next_idx = 0` means *"a route obtained and never advanced past
index zero."* **The producer says the index is sampled AT EACH TIMEOUT.** So
`idx = 0, 0` means **the colonist was at waypoint 0 at both timeout moments** —
not that it never moved at any instant between them.

**The engine's own doc states the intended reading exactly:**

> *"`route_next_idx` PINNED across successive timeouts means **stuck at one
> waypoint**; ADVANCING means **real progress along a route that still times
> out** — a different failure than getting stuck."*

★ My conclusion survives; **my justification was looser than the field's.** The
distinction matters the moment someone tries to build on it.

## §3 — ★★★★★★★★ THE FIELD EXISTS TO ANSWER MY 41% QUESTION. IT WAS BUILT JULY 30.

`timeout_route_states`' doc comment, verbatim:

> *"the last question the corrected reachability probe **can't answer alone** —
> **does the live A\* fail to FIND a route the probe proves exists, or does it
> find one the mover then fails to EXECUTE?**"*

> ★★★ **That is precisely the router-vs-probe contradiction I reported today as a
> new finding.** It was **asked, named, and instrumented a week ago** — reading
> the Chaser's existing `diagnostic_snapshot()`, *"already built for exactly
> this"* — **and never derived.**

★ **FIFTH instance of an instrument built and never read.** And the sharpest: the
previous four were fields whose *use* had to be invented. **This one came with its
question written on it.**

## §4 — ★★★★★ AND MY 9-CASE "NO ROUTE" POPULATION HAS A PRE-REGISTERED HYPOTHESIS

Same doc, final sentence:

> *"**No route at all** points at the search itself never producing one,
> **consistent with TGT-DRIFT's astar-reset repeatedly discarding whatever was
> found.**"*

**My 9 cases of *probe says a path exists, router found none* are exactly that
population** — and they arrive with a **named candidate mechanism already on
record: the astar-reset discarding found routes.**

★ **This does NOT confirm it.** It means the hypothesis is **pre-registered**, so
confirming it is a check rather than a story. ★ **And 5b independently observed
`TGT-DRIFT` firing once at the start of seed 7's job and never during either
regression** — which is evidence *against* astar-reset being active in that
specific case, and therefore a real discriminator to run against the 9.

## §5 — WHAT THIS CHANGES FOR THE VALIDATION RUN

**Nothing is retracted and nothing is pre-empted.** ★ **The live traversal still
decides whether to trust the probe**, and every interpretation below waits on it.

**What is now ready the moment it lands:**

| observation | reading |
|---|---|
| `route_exists=false` cases | **A\* never produced a route** → astar-reset hypothesis, pre-registered |
| `pinned = true` | **stuck at ONE waypoint** — a follower/execution failure |
| `pinned = false` | **advancing yet still timing out** — a *different* failure, not stuckness |
| `pinned = null` | ★ **AMBIGUOUS — go to the raw list.** Two facts share this value |

★ **That table is the fill's whole product:** the arbiter's output becomes
**readable on arrival** instead of starting another read.
