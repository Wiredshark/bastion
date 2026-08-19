# BAR 2's CAUSE: the chunk SEND path is a thread race, and #89 never tested it

> # ★★★ WITHDRAWN 2026-08-19 BY ITS OWN REGISTERED A/B
>
> The fix was built, ran on ~11,400 sends per run, harmed nothing — **and changed
> nothing.**
>
> | arm | witness | membership | first tick-sequence difference |
> |---|---|---|---|
> | CONTROL | 0 | identical | 154, 129 |
> | **FIX** | **11,438 / 11,411** | identical | **135, 154** |
>
> Registered outcome: *"control DIFFERS + fix unchanged ⇒ BAR2-CAUSE-FOUND.md is
> WITHDRAWN."* Control mean 141.5, fix mean 144.5. **It fires.**
>
> **What is refuted:** that ordering the chunk send *within a tick* contributes
> to the tick-sequence divergence. It does not, measurably, at all.
>
> **What remains untested:** the fix orders within a tick and cannot pin *which
> tick* a chunk lands in — a limit recorded here before the data. So the
> cross-tick assignment race is neither confirmed nor refuted; it was simply not
> what this experiment could reach.
>
> **What survives as fact, independent of the mechanism:** `SerializedChunk` did
> carry no key, the consumer *could* not sort, and the send order *is*
> thread-dependent. All true, all now demonstrably **not the cause of bar 2**.
> A true observation about the code is not a mechanism.
>
> The document below is kept intact because the reasoning is where the error is,
> and a withdrawn chain is only useful with its argument attached.


Found by reading, with the mechanism class learned from the haul deadlock:
**a race between two asynchronous producers feeding an order-sensitive consumer.**

## The chain

| step | code | ordered? |
|---|---|---|
| chunks released by the deterministic barrier | `recv_new_chunks_deterministic` | **YES — membership pinned** |
| new chunks fanned out to nearby players | `new_chunks.par_iter().for_each_init(…)` — **rayon** | **no** |
| serialization | `slow_jobs.spawn("CHUNK_SERIALIZER", …)`, batched 10 per job, each `chunk_sender.send(…)` **on completion** | **no** |
| delivery to clients | `for sc in chunk_receiver.try_iter() { … client.send_prepared(&sc.msg) }` | **consumes in ARRIVAL order** |

**The client receives chunks in SlowJob completion order** — i.e. in whatever
order the thread pool finishes batches.

★ And the consumer **cannot** repair it: `SerializedChunk { lossy_compression,
msg, recipients }` carries **no chunk key**. There is nothing to sort by.

★ Someone already hit a determinism bug one level down — `meta.recipients
.sort_unstable()` is right there in the spawn body. The *recipients* were made
deterministic; the *chunks* were not.

## Why the barrier does not cover it — stated in the barrier's own doc

> *"membership is what this pins."*

The barrier pins **which** chunks are released, not **when each arrives at a
client**. That is exactly what was measured:

| clause | result |
|---|---|
| membership | **IDENTICAL, 30/30 matched pairs** |
| schedule | **DIFFERS, 31/31** |

**Bar 2 asks for something the implemented mechanism explicitly does not
provide.** The gap is not a missed investigation — it is a scope mismatch
recorded in the code and never reconciled against the bar.

## ★ #89 did not test this, and its own conclusion points here

#89's fan tested six candidates. The nearest, `f-netorder`, censused
**inbound** message order — *client → server*. **The outbound chunk send is a
different direction and was never censused.**

And #89 closed with: *"the barrier controls RELEASE, not REQUEST, and the
client's chunk demand is the uncontrolled half."*

**This is WHY the demand is uncontrolled.** The feedback loop closes:

```
send order (thread race) → what the client HAS
   → what the client REQUESTS → server demand → promotion schedule
```

#89 correctly located the uncontrolled half and did not find its cause. The
cause is upstream of the request, in the send path.

## What would satisfy bar 2

1. Carry `chunk_key` in `SerializedChunk`.
2. Buffer at `chunk_send` and emit in **canonical key order** per tick, exactly
   as `canonical_haul_pickup_order` does for pickups — the codebase already has
   this pattern and a test for it.

Bounded, and gated like every other determinism control here.

## ★★ THE DOWNSTREAM PREDICTION HOLDS — measured on banked data, before the fan lands

The chain claims: send order (thread race) → what the client HAS → what the
client REQUESTS. So the client's **demand** should diverge between twins. The
demand census is already in the corpus, so this costs nothing:

| | |
|---|---|
| twin pairs with a demand census | **34** |
| **demand DIVERGES** | **31** |
| genuinely identical | **0** |
| **VACUOUS** (zero shared ticks — no comparison possible) | 3 |

★ My first summary line counted those 3 as "identical". **They are not** — zero
shared ticks means the two censuses never overlapped, so nothing was compared.
Calling that agreement is the same error as reading an unloaded chunk as air.
**31 of 31 comparable pairs diverge.**

And the first divergence lands at ticks **128–218** in most pairs — the same
window as the promotion-schedule divergence (126–193). **Demand and promotion
come apart at the same moment**, which is what a shared upstream cause looks
like.

★ What this does NOT establish: that send order *causes* the demand divergence.
Both could descend from a common upstream. It establishes that the chain's
predicted signature **is present**, which is the most a banked read can do — the
A/B now running is what separates cause from correlate.

## ★ Stated limits

This is a **READ**, not a measurement. What is established: the send order is
thread-dependent **by construction**, the consumer cannot sort, and #89 censused
the other direction. What is **not** established: that this is the *whole* of
bar 2's residual. A fix would need its own A/B, and the honest prediction is
registered here — **ordering the send should reduce, not necessarily eliminate,
the tick-sequence divergence.**
