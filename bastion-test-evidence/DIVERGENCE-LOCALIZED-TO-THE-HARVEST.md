# THE COLONY DIVERGENCE IS LOCALIZED TO TICK ~3,600 — THE FIRST HARVEST

Banked data, no spend. The claim-refusal census fires ~906 times per run, so the
split can be bracketed to a single sample interval.

## The two runs are IDENTICAL until tick 3,600

| tick | COLLAPSE `considered/eligible/assigned` | THRIVE |
|---|---|---|
| 300 | 70 / 29 / 2 | **70 / 29 / 2** |
| 600 | 33 / 0 / 0 | **33 / 0 / 0** |
| 900 | 217 / 0 / 0 | **217 / 0 / 0** |
| 1,200 – 3,000 | 224 / 0 / 0 | **224 / 0 / 0** |
| 3,300 | 180 / 0 / 0 | **180 / 0 / 0** |
| **3,600** | **224 / 0 / 0** | **160 / 0 / 0** ← **first difference** |

Ten consecutive census samples agree exactly, then split.

## What each board does afterwards

| | COLLAPSE | THRIVE |
|---|---|---|
| `considered` | climbs to **288 and FREEZES there** for 268,000 ticks | oscillates 160 → 116 → 64 → 39 |
| `assigned` | **0, forever, after tick 300** | periodic 1, 1, 2, 2, 5 … |
| refusal reason | **`materials`, 288 of 288** | `materials` transient, board drains |

★ **The discriminator is not `eligible=0`** — THRIVE shows `eligible=0` too,
whenever it has jobs pending. It is **whether the board ever drains**: THRIVE
keeps assigning work, COLLAPSE assigns nothing again for the rest of the run.

## ★ Tick 3,600 is the first harvest

The collapsed run's entire farm history spans ticks ~3,300–3,900 (8 sown, 8
matured, 8 harvested, 15 s of wall time). Its `food_stock` first becomes non-zero
at **tick 3,600**. So the boards split **at the moment the first crop is
harvested** — the two colonies run the whole sow→grow→harvest cycle identically,
and part company on what happens to the produce.

That is exactly where `emit_drop` fires, with a toss seeded on
`toss_scatter_rng(tick.0, …)` — a tick that this session measured as **not
reproducible** between twins.

★ **The instrument now running was aimed here before this was known.** The
`BASTION_DROP_TOSS_DIAG` fan was launched to answer *"are the seeds created at
all"*; this analysis independently says the split happens at the very event that
instrument logs. The convergence is a coincidence of ordering, not of design —
but it means the running fan lands on the right tick.

## ★★ THE LOCALIZATION DOES NOT GENERALIZE — corrected against the corpus

The tick-3,600 split above is real **for that pair**: ten consecutive census
samples agree exactly and then diverge. **It is not the general case.** Scoring
the whole corpus on *the last tick at which any job was assigned*:

| class | last tick with `assigned > 0` |
|---|---|
| **THRIVE** (7/7) | **265,200 … 271,200** — every run assigns work to the end |
| **COLLAPSE** (7/7) | **300 … 264,300** |

Collapse onset spans almost the entire run: 300, 4,800, 78,000, 82,800, 220,200,
264,300. **A colony can run for 264,000 ticks assigning work and still finish
with 10 maturations.** So "the split happens at the first harvest" describes one
pair, not the mechanism.

★ The discriminator that *does* hold 14/14 is weaker and I should state it as
such: **THRIVE assigns to the end; COLLAPSE stops before it.** The separation is
perfect but the margin is thin — 264,300 against 265,200, one run wide.

★ And it costs the tidy story. I had a mechanism localized to a single event,
generalized from n=1 pair, and the corpus says the onset is spread across two
orders of magnitude in time. **The first harvest is where THAT pair split, not
where colonies split.**

## What this does NOT yet say

It localizes the split to an event; it does not say what differs **at** that
event. The candidates remain: seeds not emitted, emitted but landing
unrecoverably (already refuted by arithmetic — a 0.5 horizontal toss lands within
half a block), or emitted and reachable but never claimed. **The drop-toss fan
separates the first from the other two**, which is the next thing to score.
