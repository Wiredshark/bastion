# A DOCUMENTED "CAN NEVER EXTINGUISH" INVARIANT EXTINGUISHED IN 7 OF 14 RUNS

The end of the chain that started as *"why is the promotion total quantized?"*

## The claim, in the code's own words

`bastion_jobs.rs`:

```rust
/// bastion (FARM): harvest yields — SEED_YIELD (2) > the 1 seed sowing
/// consumed = the conservation invariant holds STRICTLY (the crop can
/// never extinguish; mirrors B5's drop-conservation proof shape).
pub const FARM_WHEAT_YIELD: u32 = 2;
pub const FARM_SEED_YIELD: u32 = 2;
```

Deterministic constants, and the arithmetic is right: 2 out for 1 in is strictly
expanding.

## The measurement

**The crop extinguished in 7 of 14 runs** with identical world seed, identical
founding tick and identical colonists. Collapsed runs sow exactly 8 times in
271,000 ticks, harvest 8, and then never sow again — `materials` refusals
240/304, `blocked_materials` pinned at 28–30 for the rest of the run.

## Where the proof stops covering the behaviour

The harvest site drops the yield through a **scatter**:

```rust
let mut rng = toss_scatter_rng(tick.0, job.pos, 0xFA47_0001);
for _ in 0..FARM_SEED_YIELD {
    emit_drop(&mut item_drop_emitter, job.pos, Item::new_from_asset_expect(FARM_SEED_ITEM), …, &mut rng);
}
```

★ **The scatter RNG is seeded on `tick.0` — the tick the harvest fires on.** And
the harvest tick is precisely what the determinism measurement showed is *not*
reproducible: with the barrier ON, `preempt_attempts` differs on 816 of 905
tick-aligned samples between twins.

**So the conservation proof covers the COUNT and says nothing about the
RECOVERABILITY.** Two seeds dropped where nothing collects them conserve
nothing. *"The crop can never extinguish"* is true of the quantity emitted and
false of the quantity that returns to the claimable pool — which is the one the
sow job needs. [[a-comment-cannot-enforce]]

## The chain, end to end

| step | evidence |
|---|---|
| promotion totals quantized {196, 242, 304} | 62 runs |
| → a harness defect anchoring 55% of census runs at the world origin | fixed + plant-demonstrated |
| → determinism FAILS with the barrier on, to the final tick | 816/905 samples |
| → colony outcome **bimodal**, 8 vs 2,015 maturations | 14 runs, gap 46–936 empty |
| → selector is whether `blocked_materials` clears | **14/14** |
| → collapsed runs: 240/304 refusals are `materials` | reproduces #114's signature |
| → seeds are yielded but never reach the stockpile | 0–1 `wheat_seeds` deposits vs 1,091 |
| → the drop scatter is seeded on the **non-reproducible harvest tick** | code read |

## ★ Stated limits — the last link is NOT measured

That the scatter *position* is what makes the seeds unrecoverable is **inferred,
not tested**. `emit_drop` does not log where items land, so the logs cannot
separate:

- seeds scattered somewhere no haul job can reach, versus
- seeds reachable but never claimed, because claims were refused for another
  reason first

Both are consistent with everything above. **The next test needs an instrument,
not another run:** log the drop position in `emit_drop` and compare the scatter
between a thriving and a collapsed run.

What *is* established without that: **a documented strict-conservation invariant
does not hold in the observable that matters, half the time, and its proof never
covered that observable.**
