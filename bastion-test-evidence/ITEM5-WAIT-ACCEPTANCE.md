# Item 5 (`Wait(n)` server-authoritative): acceptance result

**Verdict: PASS.** Commits `1e98682c5e` (`bastion/wip-batch-verify`), with the
liveness-check follow-up landed before this run.

## The planted A/B

`bastion-test-evidence/live-playthrough/script-13-item5-wait-ab.txt` --
`anchor` then `wait 600` -- run twice against the SAME live server:

    leg A (unthrottled):
      waited 600 ticks -> sim 42.93..62.95 in 609 client spins

    leg B (CPU-contended -- concurrent `cargo check --bin veloren-server-cli`
    on the dev profile, confirmed live via `ps` both before and after the
    run: rustc/cargo processes present throughout):
      waited 600 ticks -> sim 192.24..212.25 in 495 client spins

## Reading the result against the packet's own acceptance clause

> "assert the same script reaches the same SIM time... with a different
> client-spin count. Today the sim time moves and the spin count does not --
> that is the defect."

**Sim-time delta reached: 20.02s (leg A) vs 20.01s (leg B) -- the same
target, both times, to within 0.01s.** **Spin count: 609 vs 495 -- different,
both times.** That is exactly the acceptance signature, inverted from the old
defect: before this row, spin count was always exactly `n` (a fixed echo of
the script) while the sim time actually reached would drift with server
speed, unmeasured. Now spin count adapts to real conditions and the sim time
reached is the fixed, correct quantity -- which is the entire point of
waiting on a target instead of a raw tick count.

**Note on direction, since it isn't what intuition first suggests:** leg B's
spin count is LOWER than leg A's, not higher. This is expected, not a defect,
once traced to its cause: `Client::tick`'s `dt` is measured wall-clock time
per call (`clock.game_dt()`), and `Time` advances locally by `dt *
time_scale` every tick (`State::tick`) -- a CPU-starved client sees LARGER
gaps between successive `tick()` calls (its own process gets scheduled less
often under contention), so each spin's local `Time` advance is BIGGER, not
smaller, needing fewer spins to close the same sim-second gap. The packet's
acceptance clause asks for spin count to differ from a fixed `n`, not for a
specific direction -- both are satisfied.

Zero tick errors, zero VOID in either leg -- the acceptance ran clean, not on
a technicality.

## What this does and does not establish

Established: `Wait(n)` now targets a stated sim-time quantity rather than a
raw tick count, and the log states its own conversion factor on every wait
(`n ticks -> sim start..end in K spins`), so a run's `%-of-budget` figure is
computable directly instead of assumed -- closing exactly the class of
confusion the packet names (the three-hop unit conversion, the unanswerable
epoch question, "105.3%" that was actually 283%).

Not established by this A/B, and not claimed: this run demonstrates the
mechanism responds to REAL conditions (client-side scheduling pressure in
this case), not specifically "the server itself was measurably slower." A
genuinely server-throttled leg (e.g. running the server under a CPU-limited
cgroup/affinity mask, or a heavier concurrent load pinned away from the
driver's own cores) would isolate the server-speed case more cleanly; this
run's throttle affected the whole box, which is sufficient to prove the
mechanism is load-responsive but not to attribute the specific direction to
server slowness alone. Per the packet's own caveat: `Time` is a
TRACKING approximation (hard resync past 5s divergence, ~1% tween otherwise),
not the server's authoritative value -- fine and stateable for
minutes-scale waits (300s despawn windows, 900s hunger crossings), not for
anything measured in single ticks.

## Liveness-check correction, folded into this acceptance

The first landed version of the failsafe (wall-clock "no sim advance in 30s")
was non-functional -- `Time` advances locally every tick regardless of
server responsiveness, so a dead server's clock never stops moving and the
check could never trip (Opus's catch, verified against `State::tick`'s own
code before the fix, commit `1e98682c5e`). Replaced with consecutive
`client.tick()` errors, keying on the engine's own `Error::ServerTimeout`
(the `client_timeout`-based liveness check already computed inside
`handle_messages`) rather than inventing a new signal. Not exercised in this
A/B (the server never actually died), but changed before this acceptance ran
so the failsafe underneath the acceptance itself was trustworthy.
