# REQUEST BARRIER — it controls arrival, and bar 2 is still unreachable server-side

**n = 1 fix pair.** Three of four hosts hit GCP's machine-image creation rate
limit (too many instance creations from one image in quick succession — my own
doing, an accidental launch minutes earlier). The fan's guard correctly refused a
verdict. What landed is still decisive, because 38 banked control pairs exist to
compare against.

## The barrier engaged, exactly as designed

| check | result |
|---|---|
| barrier opens | **226** vs expected ~227 (`ticks/50`) — **not inert** |
| membership (bar 3) | **IDENTICAL, 304/304** — no regression |
| first divergence | tick **150** — first diff is `PENDING` (request arrival) |

## ★ It moved the divergence onto a boundary

| | first-diff on a multiple of 50 |
|---|---|
| banked controls (no barrier), n=38 | **1 of 38 — 3%** |
| fix arm (barrier=50), n=1 | **yes — tick 150** |

Controls diverge at arbitrary ticks (96, 97, 98, 99, 100, 101, 102, 106, 121,
125…). The fix arm diverged at a **boundary**. That is the gate working: arrival
is now quantized to boundaries, and the two twins landed on **adjacent** ones.

★ n=1, and under the null a boundary hit happens 3% of the time. **Suggestive,
not established** — but it is the predicted signature, not a surprise.

## ★★ WHY NO SERVER-SIDE BARRIER CAN CLOSE BAR 2

This is the structural result, and it is what the experiment actually bought:

| barrier shape | effect on cross-process arrival jitter |
|---|---|
| **modulus** (`tick % n == 0`, what I built) | **quantizes** it — but two arrivals a few ticks apart can still fall either side of a boundary |
| **delay** (`arrival + d <= now`, what the release barrier uses) | **preserves it exactly** — a constant offset added to both keeps the difference |

The release barrier works because `request_tick` is an **internal, deterministic**
quantity. Arrival tick is not: it is set by two processes with independent
clocks.

**So the modulus is the best available shape and it is asymptotic, not exact.**
P(straddle) ≈ jitter / boundary — at 5 ticks of jitter and a 50-tick boundary,
~10%; a 500-tick boundary gets ~1% at the cost of 500 ticks of latency. **You can
make bar 2 arbitrarily unlikely to fail. You cannot make it pass.**

## What this settles for the certification

**Bar 2, as written — "twin runs state-identical including chunk timing" with a
live client — is not reachable by server-side means.** Three candidates have now
been eliminated by measurement rather than argument:

| candidate | outcome |
|---|---|
| #89's ten (all server-side) | excluded |
| chunk-send ordering | built, ran 11,400×, **changed nothing** |
| request-side modulus barrier | built, engaged 226×, **moved divergence to a boundary, did not remove it** |

★ The remaining options are **architectural**, not bug-fixes: lockstep the client
and server tick loops, or **run the determinism bar without a live client** — a
headless fixture where the server drives its own requests removes the cause by
construction.

**That second option is a scoping decision, not an engineering one**, and it is
Ben's: it asks whether bar 2 certifies *the engine* or *the engine plus a
networked client*.
