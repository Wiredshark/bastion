# BAR 2 ORDERING FIX — A/B, pre-registered

Written before any run. One axis: `BASTION_DETERMINISTIC_CHUNK_SEND=1`.

## What the fix does

`SerializedChunk` now carries its `key`, and `chunk_send` drains the tick's
chunks and emits them **sorted by `(x, y)`** instead of in `try_iter()` arrival
order. Arrival order is SlowJob completion order — a thread race — which is why
the client received terrain differently on every run.

## Bars

| bar | arm | PASS | FAIL |
|---|---|---|---|
| **P — precondition** | fix arm | `bastion: chunk send ORDERED by key` appears | absent ⇒ **arm VOID**, not "no effect" |
| **1 — the control still diverges** | control | tick-sequence DIFFERS, reproducing the 31/31 baseline | control identical ⇒ the phenomenon is gone at this tip and the whole A/B is **VOID** |
| **2 — the fix removes it** | fix | tick-sequence **IDENTICAL** between twins ⇒ **BAR 2 OF THE CERTIFICATION PASSES** | still differs ⇒ send order is **not the whole cause** |
| **3 — membership unharmed** | both | promoted key sets stay identical within each pair | sets diverge ⇒ the fix **broke** something the barrier had pinned |

★ Bar 3 is not a formality. Re-ordering the send path could change what the
client requests next, which feeds back into promotion. **A fix that pinned the
schedule by perturbing membership would be a regression wearing a green badge.**

## ★ The registered prediction, and it is deliberately modest

`BAR2-CAUSE-FOUND.md` states: *"ordering the send should REDUCE, not necessarily
eliminate, the tick-sequence divergence."*

That was written before this fix existed and it stands. **Three outcomes, all
informative:**

| outcome | reading |
|---|---|
| fix arm fully identical | the send race was the **whole** residual — bar 2 passes |
| fix arm diverges **later** or **less** | the send race is **one** contributor; the prediction holds and the remainder is a new named target |
| fix arm unchanged | the send race is **not** a contributor; `BAR2-CAUSE-FOUND.md` is **withdrawn** |

★ The third outcome costs me the finding, which is why it is written down before
the data. A read-derived mechanism that has never been measured is exactly the
kind of thing that looks right and is not — the haul deadlock survived that test
this morning; this one may not.

## ★★ THE FIX IS PARTIAL BY CONSTRUCTION — said before the data, not after

Sorting happens **within one tick's drain**. But SlowJobs complete
asynchronously, so **which tick a chunk lands in is still a thread race**: a
batch that finishes late is drained on a later tick and sorted into *that*
tick's sequence.

So the fix makes the order **within** a tick a pure function of the chunk set,
and leaves the **assignment of chunks to ticks** unpinned.

**That is precisely why the registered prediction says REDUCE, not ELIMINATE** —
and now the reason is structural rather than cautious. If the fix arm comes back
fully identical, that would mean tick-assignment happens to be stable under this
fixture, not that the race is gone.

★ **The complete fix would need a barrier on the serialize side**, holding a
tick's chunks until every batch for that tick has completed — the same shape as
`recv_new_chunks_deterministic` does for the generator. That is a bigger change
and it should not be attempted before this A/B says how much of the divergence
the cheap half removes.

★★ Recording this now matters because a partial result is exactly what invites
over-claiming. If the divergence shrinks, the honest headline is *"the within-
tick order was one contributor"* — **not** *"bar 2 is fixed"*.

## Preconditions above every verdict

1. Both twins boot and carry a terminator.
2. The ordering witness appears in the **fix** arm and is **absent** in the
   control — its presence in both would mean the env leaked across arms.
3. `provtravcap` (capped TPS) so the tick axis is comparable, matching every
   banked measurement of this row.
