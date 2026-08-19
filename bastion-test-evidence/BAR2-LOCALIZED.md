# BAR 2 LOCALIZED: 38 of 38 pairs diverge FIRST at REQUEST ARRIVAL

Measured, not inferred. Every `provtrav*` twin pair in the corpus, classified by
**what the first difference actually is**:

| first difference | pairs |
|---|---|
| **`pending` — the client's request arrives on a different tick** | **38** |
| `promoted` — server-side promotion differs | **0** |
| identical | 0 |

**Unanimous.** The shape is always the same: `pend 0 → 49` on one twin while the
other is still at `0`, a few ticks apart.

```
tick 124   twin1 p=0 pend=0     twin2 p=0 pend=0
tick 125   twin1 p=0 pend=49    twin2 p=0 pend=0     <<< first difference
```

## What this settles

**The server-side promotion machinery is not the source.** The deterministic
barrier is doing exactly what its doc claims. Everything bar 2 measures —
promotion schedule, demand census, downstream state — follows from **one input
arriving on a different tick**.

★ This is #89's own conclusion, *"the barrier controls RELEASE, not REQUEST, and
the client's chunk demand is the uncontrolled half"* — now **measured at 38/38**
rather than reasoned. #89 was right, and both of my subsequent theories were
detours from an answer it had already stated.

## ★★ Why bar 2 may be unsatisfiable as written

The client and the server are **separate processes with independent tick loops**.
The client's first request goes out when it has both a granted view distance and
a server-delivered position; when that lands relative to the *server's* tick
counter depends on the alignment of two clocks nobody controls.

**Bar 2 asks for two independent processes to interact on identical ticks.** No
server-side change can deliver that, which is why:

- the deterministic release barrier does not (measured: membership pinned, schedule not)
- ordering the chunk send does not (measured this session: fix ran 11,400×, changed nothing)
- #89's ten candidates did not (all server-side)

## The one shape that could work — a REQUEST-SIDE BARRIER

Symmetric to `recv_new_chunks_deterministic`: **hold received chunk requests and
dispatch them only on a fixed tick boundary.** A request arriving at tick 125 and
one arriving at 130 both dispatch at 150, so arrival jitter collapses to the same
tick and everything downstream re-aligns.

That is the only candidate that attacks the measured cause rather than a
downstream symptom. **It is a live-path change to `sys/msg/terrain.rs` and it is
not built** — recorded as the named next step, with the evidence for it on the
table.

★ Alternative worth pricing before building it: **run the determinism bar without
a live client.** If the divergence is entirely cross-process clock alignment, a
headless fixture where the server drives its own requests removes the cause by
construction — and would tell us whether bar 2's *intent* (the engine is
deterministic) is already satisfied while its *letter* (with a live client) is
not.
