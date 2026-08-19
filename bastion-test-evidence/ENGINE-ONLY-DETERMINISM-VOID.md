# ENGINE-ONLY DETERMINISM — **VOID**, with the cause named

The arm ran, booted, and emitted **7,180–7,240 census lines per run**. It
promoted **nothing**:

| run | census emits | **promoting ticks** | max `pending` |
|---|---|---|---|
| h1 twin1 | 7,240 | **0** | 0 |
| h1 twin2 | 7,188 | **0** | 0 |
| h2 twin1 | 7,182 | **0** | 0 |
| h2 twin2 | 7,219 | **0** | 0 |

## ★ The precondition caught a false green

Registered before the run: *"if promotion is still near zero the arm is VOID and
must not be reported as identical."*

**Four runs that promote nothing have trivially identical tick-sequences.** A
plain diff would have returned IDENTICAL and I would have reported *"the engine
is deterministic"* — the strongest possible conclusion from the weakest possible
evidence, on the most decision-relevant question of the day.

## The cause, exactly

`autofound colony founded` appears **0 times**. The colony was never created:

```rust
std::env::var("BASTION_AUTOFOUND_COLONY")
    .filter(|&n| n > 0 && bastion_flat_arena::enabled())
```

**Autofound only fires when the FLAT ARENA is enabled.** The arm sets
`PITARENA=""` (real terrain) so that there is something to generate — which
switches autofound off. No colony ⇒ no `Presence` ⇒ no requests ⇒ no promotion.

## ★★ THE BIND — and it is in the existing code, not in my arm

| configuration | requester exists? | anything to generate? |
|---|---|---|
| **flat arena** | ✅ autofound fires | ❌ terrain is pre-generated |
| **real terrain** | ❌ autofound filtered out | ✅ yes |

**There is no configuration in which a headless server both has a terrain
requester and has terrain to request.** That is why the earlier driverless
attempt was VOID at 6 promoting ticks of 3,382 — the note recorded it as *"a
stationary colony in a pre-generated arena requests no chunks"*, which named the
symptom; this names the mechanism.

## What it means for bar 2's scoping question

The engine-only test is **not currently possible**, and the blocker is a code
constraint rather than a fixture gap. To run it, autofound would need a
deterministic spawn on **real** terrain — today it depends on
`bastion_flat_arena::world_center_wpos`, which only exists for the arena.

★ So the honest state of bar 2: **every server-side candidate is eliminated by
measurement, and the experiment that would isolate the engine from the client
cannot be built without one more code change.** That change is small and
well-defined (a deterministic real-terrain spawn point), and it is the single
remaining path — but it is a build, not a run, and it should be Ben's call
whether bar 2 is worth it given that the row's own roadmap criterion already
passes.
