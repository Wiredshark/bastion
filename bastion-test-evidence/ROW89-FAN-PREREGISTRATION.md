# ROW #89 — PARALLEL HYPOTHESIS FAN, PRE-REGISTERED

**One document, every candidate, scored together.** Written before any arm runs.
Ben's speed mandate: #89 is the last blocker on tick-loading certification, and
certification is the multiplier for everything behind it.

## What is already excluded, and by what

| # | candidate | verdict | evidence |
|---|---|---|---|
| 1 | client request **timing** | **EXCLUDED** | join held +10.9 s, first-diff unchanged 5/5 arms (`279193df58`) |
| 2 | join **tick** | **EXCLUDED** | 22 of 24 same-join-tick pairs still diverge (`58c98f50e5`) |
| 3 | worker-pool spawn order | **EXCLUDED BY READING** | `slowjob.rs` sorts on a *total* key (rate desc, then unique name) — ENGOPT-4 (`c4a9a8a282`) |
| 4 | chunk **arrival** order | **EXCLUDED** | deterministic drain active on 11,439/11,439 and 20,840/20,871 ticks; divergence magnitude unchanged vs free-running arms (`dec8271514`) |

## Excluded in this pass, by reading only — the cheapest instrument

| # | candidate | verdict | what was read |
|---|---|---|---|
| 5 | wall-clock in the request chain | **EXCLUDED** | only two sites in `chunk_generator.rs`: the A3 plant (env-gated, `Err(_)` → clean budget, inert unset) and the 180 s barrier deadline — **measured never to fire** in banked logs |
| 6 | unsorted iteration in `chunk_generator` | **EXCLUDED** | both `HashMap` iterations are sorted before use (`due.sort_unstable_by_key`, `out.sort_unstable_by_key`) |
| 7 | **OS-entropy class** | **EXCLUDED** | `thread_rng`/`from_entropy` count is **ZERO** across `server/`, `common/systems/`, `bastion-server/`, `world/`, `client/`. Only 2 `tick_rng` call sites, both seeded. |

★ Candidate 7 is the one Ben named explicitly. It is dead by census, not by
sampling: there is no unseeded RNG anywhere in the request chain to be the
source.

## The fan — one instrumented arm per surviving candidate

All arms are **twin pairs on one host**, same seed, same commit, scored with the
first-diff instrument already built. Six arms = the 96 vCPU ceiling.

| arm | candidate | instrument | REFUTED if | CONFIRMED if |
|---|---|---|---|---|
| **d-unload** | rayon `par_pending_chunks` unload order | census removed-chunk **set** + per-tick order | sets identical both twins | sets differ → unload is the source |
| **e-demand** | client *demand* path (which chunks, not when) | census the request **key set** per tick | key sets identical | key sets differ → demand is the source |
| **f-netorder** | network message **arrival order** | census inbound msg kind order per tick | order identical | order differs → transport is the source |
| **g-ecsjoin** | ECS join/system iteration order | census entity order in the terrain system | order identical | order differs → join order is the source |
| **h-pool** | slowjob **completion** order (not spawn) | census chunk-completion order vs spawn order | completion order matches spawn | differs → completion is the source |
| **i-control** | **no instrument** — plain twin pair | — | *must* reproduce ~1% divergence, else the whole fan is VOID | — |

★★ **Arm `i-control` is not filler.** Every other arm adds an emit, and
[[the-instrument-changes-what-it-sees]] is a standing law here: if instrumenting
suppresses the divergence, all five readings are artifacts. The control's job is
to prove the phenomenon still exists at this commit with nothing added.

## Scoring — one rule for all six

1. **PRECONDITION:** every arm must boot, collect both twins, and the control
   must diverge. Any failure ⇒ that arm VOID, named, no verdict quoted.
2. **DIVERGENCE:** first-diff family per twin pair, the instrument used
   throughout this row.
3. **CANDIDATE VERDICT:** its censused quantity identical across twins ⇒
   **EXCLUDED**; differs ⇒ **CANDIDATE SURVIVES** and its first differing tick
   is the site.

## If the fan kills everything

Bisect the tick pipeline with the first-diff instrument — census progressively
earlier stages until the divergence site names itself. That is a *bisection*,
not a new guess, and it is the registered fallback rather than a fifth
hypothesis round.

## What would make me withdraw the whole approach

If the control arm does **not** diverge, the ~1 % residual is not a property of
this commit and every exclusion above is measured against a phenomenon that no
longer exists. That is checked **first**, before any candidate is read.
