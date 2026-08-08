# THE MODE-DISPATCH PREDICATE — READ, AND IT REQUIRES THE ONE THING JOB 20 ISN'T

**All line cites `5f8cdf1392`, `server/agent/src/action_nodes.rs`.**
★ **Claims marked READ carry a line; anything else is marked UNVERIFIED.**

## §1 — ★★★★★★★ THE PREDICATE (READ, 497-508)

```rust
fn traverse(&self, controller: &mut Controller, bearing: Vec3<f32>, speed: f32) {
    controller.inputs.move_dir = bearing.xy().try_normalized()... * speed;

    // Only jump if we are grounded and can't blockhop or if we can fly
    self.jump_if(
        (self.physics_state.on_ground.is_some() && bearing.z > 1.5)
            || self.traversal_config.can_fly,
        controller,
    );
    controller.inputs.move_z = bearing.z;
}
```

**Two conjuncts. `can_fly` is false for colonists.** So the jump fires only when
**grounded AND `bearing.z > 1.5`.**

> ## ★★★★★★★★ AND 5b MEASURED JOB 20 AT `on_ground = false` FOR **1781 OF 2031 TICKS**
>
> **The colonist is airborne 88% of the time, and the jump dispatch REQUIRES
> being grounded.** ★ **The jump can only be issued in the remaining 12% — and
> only if `bearing.z > 1.5` happens to coincide with those same ticks.**

★ **That is branch 2's mechanism, matching the physics evidence exactly: not a
capability gap, a DISPATCH gap.** *Walking-gait air time (0.005–0.35) is the
colonist stepping off ledges and falling — and every one of those ticks is a tick
the jump cannot be issued.*

## §2 — ★★★★★★ THE SAME-TICK CANCEL, AND IT MAY BE THE WHOLE STORY

**Call order at the chase site (READ, 546-547):**

```rust
self.unstuck_if(stuck, read_data.dt.0, controller);      // may PUSH Jump
self.traverse(controller, bearing, speed * speed_multiplier);  // may CANCEL it
```

**`unstuck_if` (READ, 517-531)** pushes `InputKind::Jump` on a hazard-gated random
draw. **`jump_if` (READ, 533-539):**

```rust
if condition { controller.push_basic_input(InputKind::Jump); }
else if controller.queued_inputs.contains_key(&InputKind::Jump) {
    controller.push_cancel_input(InputKind::Jump)
}
```

> ★★★ **`unstuck_if` PUSHES a jump; `traverse` CANCELS it in the SAME TICK
> whenever the colonist is not grounded.** **And job 20 is not grounded 88% of
> the time.**

★★ **This would also explain the job-33 contrast:** the bed specimen's **7.48
spikes may be UNSTUCK jumps that survived** — *i.e. fired on ticks where the
colonist happened to be grounded, so `jump_if` pushed rather than cancelled.*
★ **Two dispatchers writing one input, one cancelling the other, gated on a
condition that is false most of the time.**

## §3 — ★ WHAT IS **UNVERIFIED** AND MUST BE READ BEFORE ANY FIX

1. ★★★ **`push_cancel_input` semantics.** **I have NOT read it.** If a cancel in
   the same tick actually suppresses the queued jump, §2 is the mechanism. **If
   it only takes effect on a later tick, the jump may still fire and §2 is
   wrong.** ★ **This single read decides whether §2 stands** — and it is exactly
   the producer-read discipline that caught me three times today.
2. **That job 33's spikes came from `unstuck_if`.** **Inferred, not traced.**
   *Attributing them requires instrumenting which call site pushed.*
3. **Whether `bearing.z > 1.5` is ever satisfied for job 20.** ★ **If it never
   is, the `on_ground` conjunct is irrelevant and the story is different** —
   `bearing.z` is not in the current trace.

## §4 — ★★★★★ THE COMMENT DESCRIBES A GUARD THAT ISN'T THERE

> *"Only jump if we are grounded **and can't blockhop** or if we can fly"*

★ **There is NO blockhop term in the expression.** It may be implicit in
`bearing.z > 1.5` — *plausible, and UNVERIFIED* — **but the comment names a
condition the code does not separately express.**

★★ **The week's signature again**, and the third time today a comment's stated
justification hasn't matched the case it fires on *(the shove-reset branch's
"target switched"; ENDURE's "unreachable bed"; now this)*.

## §4b — ★★★★★★ REFUTED AND REPLACED: THE DISCRIMINATOR IS THE STAND-AT CELL

**5b measured the matched pair. My grounded-fraction framing is BACKWARDS:**

    job 23 (SUCCEEDS): grounded   2/603  =  0.3%
    job 20 (FAILS)   : grounded 250/2031 = 12.3%   <- 40x MORE grounded

★ **Raw grounded-fraction does not predict dispatch.** Job 23 needed **one** of
its two grounded ticks to coincide with a qualifying bearing; job 20 had **250
opportunities** and never got one.

### ★★★★★★★ AND THE CORPUS ALREADY HELD THE REASON

| target | `standable_target` | below_open | top | minDist | progress | offered/timeouts |
|---|---|--:|---|--:|--:|---|
| **job 2** `[…,9263,336]` | ★ `[…,9264,336]` — **same z, lateral** | 0 | F | **3.78** | **0.878** | 2 / 1 |
| **job 20** `[…,9263,338]` | ★★★ `[…,9263,339]` — **z+3, ABOVE** | **2** | **T** | **16.24** | **0.000** | **5 / 5** |

> ★★★★★ **Job 2's stand-at cell is LATERAL. Job 20's is THREE BLOCKS UP** — and
> job 20 is `top=True` with `below_open=2`: **the two cells beneath it are
> already mined out.** **The colonist destroyed its own footing, and the only
> standable position left is a perch on top.**

★ **A jump gains ~1 block.** So this is **not** a near-miss on a threshold — the
route is trying to deliver a colonist to a perch **3 blocks above open air**,
which is why `bearing.z` never qualifies and why there are **zero `vel_z` spikes
in 2031 ticks.** ★ **"Never even attempts" is geometrically correct behaviour
from the dispatch's point of view.**

★★ **Job 23's ABSENCE from `mine_cell_diag` is itself confirming** — the diag
lists only cells still holding an open job, so *the one that completed left no
trace.* **(The same definition-not-gate property I retracted a wrong claim about
earlier today, now paying as evidence.)**

★★★ **Candidate class:** this looks like the known *"digs clean, zero descent
access, crew strands at the rim"* case named in `AUTO_LADDER_ACCESS`'s own
comment — **the same defect in a mine-cell costume.** *UNVERIFIED.*

## §4c — ★★★★★★★★ §4b RETRACTED TOO. I INVERTED THE FIELD, AND THE READ IS DONE.

**5b's `BASTION_BEARING_TRACE_UID` at the dispatch predicate itself:**

    jump_condition = TRUE, 5 times · bearing_z = 2.0 · on_ground = true

> ★★★ **The predicate IS satisfied and the jump input IS pushed.** **§4b's
> "bearing.z never qualifies, geometrically out of reach" is DEAD**, and so is
> §1's "never grounded when it matters."

### ★★★★★★ AND THE FIELD I BUILT §4b ON MEANS THE OPPOSITE OF WHAT I SAID

`open_cells` is built (harness **3326-3335**) from
`bastion_inspect_cell(pos) == Job(_)` — **cells that still have an OPEN JOB.**

> **`cells_below_open` counts cells beneath with UNFINISHED WORK — i.e. STILL
> SOLID ROCK. Not open air.**

★★★ **I read `below_open=2` as "mined out" and wrote "the colonist destroyed its
own footing."** **Exactly backwards.** ★ **5b's live terrain dump — z=336-338
fully solid — CONFIRMS the corpus. There was never a discrepancy; my
interpretation was the discrepancy**, and they flagged it rather than resolving
it toward my read.

★ **Also wrong, same paragraph:** *"standable is 3 blocks up"* — job 20's
standable is **z+1 from its OWN target**, the ordinary stand-on-top position. **I
compared it to job 2's TARGET z, a different quantity.** ★ **Only LATERAL vs
ABOVE survives**, which the enclosure finding independently supports.

★★ **Fourth time today reasoning from a field's name rather than its producer —
and this one I had ALREADY READ the producer of, hours earlier.**

### ★★★★★ THE `push_cancel_input` READ — DONE, AND IT DOESN'T DECIDE THIS CASE

```rust
push_basic_input(i)  -> push_action(ControlAction::basic_input(i))
push_cancel_input(i) -> push_action(ControlAction::CancelInput { input: i })
```

**Both land in the same `actions` queue in order**, so a same-tick push-then-cancel
**would** process as start-then-cancel. ★ **The two-writer race is structurally
real — FILED.**

> ★★★ **But it is NOT this specimen's mechanism:** `jump_condition = TRUE` on the
> 5 ticks that matter, so `jump_if` **pushed**, never cancelled. **§2 is dead for
> job 20.**

### ★★★★★ THE HONEST RESIDUAL — AND WHY THE CHASER PIECE CLOSES

> **The predicate is satisfied, the jump IS pushed 5 times, and NO progress
> results — in a cell enclosed by solid rock on all 8 lateral sides, open only
> straight up.**

★ **5b's `1.75 / 0.0 / -0.75 / 2.5` cycle runs continuously and UNCORRELATED with
the 5 dispatch ticks** — *wall-contact jitter, not jump response*, against job
33's isolated **7.48** spikes. ★★ **And the two systems sampling `on_ground`
differently at the same wall-clock moment is a PIPELINE-POSITION artifact, not a
physics contradiction** — named rather than resolved toward the convenient
sample.

★★★ **The remaining question — why a pushed jump produces no displacement in an
enclosed column — is a PHYSICS / CHARACTER-STATE question, not a Chaser one.**
**Banked for whoever takes the enclosed-geometry case with a purpose-built
fixture.**

## §5 — (SUPERSEDED BY §4b, WHICH IS ITSELF RETRACTED — SEE §4c)

**Their trace says WHAT differs between cells 20 and 23. This says what the
predicate DOES with it.** ★ **Two specific things to pull from the matched pair:**

| measure | why |
|---|---|
| ★ **`on_ground` fraction for job 23** | if 23 is grounded far more than 20's 12%, **the predicate explains the pair outright** |
| ★ **`bearing.z` distribution, both** | if 20 never exceeds **1.5**, the ground conjunct is a red herring and the story moves to the *bearing* |

> ★★★ **Those two numbers, on a pair one block apart, close the bounded
> question.** Everything else is already held constant by construction.
