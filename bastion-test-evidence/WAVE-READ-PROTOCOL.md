# WAVE-READ PROTOCOL — **THE ORDER IS THE POINT**

**Ruled by Fable, 2026-08-10, after wave33.** Binds every corpus-fan read, every
lane. The collector already prints the strongest signal; this makes it
impossible to meet it second.

> ## **THE CLAUSE DELTA IS READ FIRST — BEFORE THE MOVER LIST, BEFORE ANY
> HEADLINE, BEFORE ANY DISTRIBUTION.**

## WHY — THE FOUNDING SPECIMEN (wave33)

`collect_wave.py` prints, unprompted:

> *"A GAINED clause is a REGRESSION the fail COUNT cannot show — the seed was
> already failing."*

**It printed that on wave33 and I read the mover list first anyway.** The fail
count moved 11→12 and I reported 5 movers. **Seed 66 — a PERSISTENT failer, so
absent from every mover list — had SWAPPED its clause set**, losing `tl_ok` and
gaining `build_placed` + `stone_sum_lower`: the same build/material family as two
of the five. The build signature was **3 seeds, not 2**, and I found the third
only by chasing an unrelated loose end.

★ **The tool was not silent. The reading order made it inaudible.** Vigilance was
not the fix; **section order** is.

---

## THE RITUAL — IN THIS ORDER, NO EXCEPTIONS

### 1. ATTESTATION — before anything is read as data

Every VM's `COMMIT=` and `DONE=` count; total seed blocks; absence of `STALE` /
`BUILD_FAIL`. **A wave without attestation is not evidence** — no ATTEST line, no
read. Record the fan log's identity alongside.

### 2. SCHEMA VALIDATION — every seed, before any value is interpreted

Run the field checker over **all** seeds. **A seed missing keys is UNPROVEN,
never averaged in. Absent is not zero.** A refusal outranks a value.

**If the checker fails, suspect the checker first when the failures are
UNIFORM.** Real defects scatter; a constant across an entire failure set is a
threshold or an unmodelled producer. *wave33 failed 8/48 on a clause of mine that
encoded a single-producer assumption; all 8 sat at exactly the same value, and
that uniformity was the tell before any code was read.*

### 3. ★ **CLAUSE DELTA — EVERY SEED, FAILING OR NOT**

**Not the mover list. The per-seed GAINED/LOST clause sets, across the whole
corpus**, including seeds that failed in both waves and seeds that passed in
both.

- **GAINED on an already-failing seed is a regression the count cannot show.**
- **LOST on a still-failing seed is a fix the count cannot show.**
- **A swap is both at once, and is invisible to every summary statistic.**

Report the clause **families** that moved, not the seed count. *"The build family
gained 3 seeds"* is the finding; *"11→12"* is bookkeeping.

### 4. THE BASELINE IS REGISTERED, NEVER AUTO-SELECTED

Always `--baseline <explicit wave>`. **Auto-select is EXPLORATORY and its numbers
are not citable.** *On wave33 auto-select reported 2 movers where the registered
read reported 5 — the divergence is real and this is what #67 exists to prevent.*

**And a baseline is only a baseline for the producers it contained.** Check the
commit range before attributing anything: **a bundle diff is not an A/B**, and
"controlling the commit" is not "controlling the mechanism."

### 5. CENSORING AND CONSTANCY CHECKS — before any distribution is quoted

- **A spike exactly at a named constant is the constant, not data.** Any field
  bounded by a threshold it is being used to calibrate is
  [[a-field-cannot-calibrate-its-own-bound]] — say so and stop; more seeds cannot
  fix censoring.
- **STOPPED/STARTED VARYING is two readings** — instrument died, or the thing
  stopped happening. **The frozen VALUE is the clue** (stuck at passing suggests
  the latter), but **it is UNREAD until the producer is read.** Record it as
  UNREAD; never score it as a pass.

### 6. ONLY NOW — THE MOVER LIST, THE DISTRIBUTIONS, THE HEADLINE

By this point the movers are a *subset* of §3's finding, and the distributions
are known to be uncensored or known not to be.

### 7. SCOPE THE VERDICT

Say what the wave does **not** establish. **Zero cases = VOID, not PASS.** An
attribution the range cannot support is refused explicitly, with the reason.

---

## RESULTS-DOC TEMPLATE — SECTIONS IN THIS ORDER

```
# WAVE NN — <subject>
   binary / seed count / wave JSON / fan log

## ATTESTATION                       (§1)
## FIELD VALIDATION                  (§2 — n/N, refusals named)
## CLAUSE DELTA  ← FIRST FINDING     (§3 — families, incl. already-failing seeds)
## BASELINE + RANGE                  (§4 — registered baseline; commit range enumerated)
## CENSORING / CONSTANCY             (§5 — UNREAD items named as UNREAD)
## THE REGISTERED QUESTIONS          (§6 — one heading each, answered or refused)
## MOVERS                            (§6 — a subset of the clause delta, never the headline)
## WHAT THIS WAVE DOES NOT ESTABLISH (§7)
## INSTRUMENT GAPS FOUND
## STANDING  (what closes, what stays provisional, what is parked with its unlock)
```

**`WAVE33-RESULTS.md` predates this template** and is left as written — it is the
specimen the protocol was derived from, and re-ordering it retroactively would
erase the evidence of the failure it exists to prevent.
