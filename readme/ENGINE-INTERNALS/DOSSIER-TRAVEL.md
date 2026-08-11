# DOSSIER — TRAVEL & THE STUCK-COLONIST CLASS

*Written at item 3's close, per the ritual. Collates what is now KNOWN about why
colonists fail to arrive, what was fixed, what survived the fix, and what the
residual turned out to be.*

---

## 1 · THE SHAPE OF THE PROBLEM

**A colonist accepts a job, sets out, and does not arrive.** For most of this
arc that single sentence covered at least three unrelated mechanisms, which is
why it resisted diagnosis for so long: **every fix worked, and the symptom
stayed.**

★ **The decisive move was not a better fix but a SEPARATION** — instrumenting
until each mechanism had its own witness, then letting the counts say which was
which.

---

## 2 · THE SIT TRAP — **CLASS ELIMINATED** (item 3, #94)

**Mechanism.** The idle branch pushed `Sit` at `hazard(rng, dt, IDLE_SIT_RATE)`.
The `Goto` arm called `dismount_uncontrollable` but never `Stand` — and the only
`Stand` in the behaviour tree sat under `NpcActivity::Talk`. **A commanded
colonist that had randomly sat down could not get up.**

**Fix.** The `Goto` arm stands a sitting NPC before moving.

**Why the acceptance was hard, and how it was solved.** *The precondition is
random,* so a live run showing zero failures cannot separate FIXED from
NOT-EXERCISED. **The fix emits when it fires**, so one run yields a pair:

    GOTO-STAND-RESCUE > 0    the precondition OCCURRED, N times
    ULTIMATE FAIL-SAFE = 0   none of them stranded

★★★ **That is the planted test's logical content without the harness** — and
`rescues == 0` was registered as **VOID**, applying the zero-cases discipline to
the witness itself.

**Scale, measured.** **526 rescues across 22 uids** at scoring; **373 rescues** in
the later farming runs. **Fail-safes 9 → 2.** ★★ **The class is eliminated, and
the residual was correctly attributed elsewhere rather than counted as failure.**

> ★★★★ **AND THE SECOND-ORDER RESULT, which is the more valuable one: item 3 did
> not merely remove its class — IT MADE THE RESIDUAL LEGIBLE.** *Before the fix,
> stuck colonists were overwhelmingly sitting ones; every other mechanism was
> buried in that noise. After it, a single fail-safe in ~12 live minutes is a
> clean specimen instead of one of nine sit-confounded ones.* **Item 4 could only
> close because item 3 cleared the field.**

---

## 3 · THE TRAVEL-TIMEOUT CLASS — **CHRONIC, NOT A REGRESSION**

**Colonists stop short of their jobs — 6 to 68 blocks — routinely.**

★ **This was mistaken for a regression and is not one.** *A specimen showing a
65.9-block median looked like the strongest lead in a mover hunt; the same seed
was **66.1 blocks** short one wave earlier.* **Corpus-wide the class is flat:
133 → 141 events, 31 → 32 seeds, across waves with a 16-commit delta.**

> ★★ **THE LESSON, banked: a large value on a known-noisy field is not a large
> SIGNAL — it is a large BASELINE.** *State any claim on these fields against
> their measured floor, or it is an assumption wearing data.*

**The three fields are NON-DETERMINISTIC** — measured at 20/48, 21/48 and 4/48
between provably scalar-identical waves:
`b5_travel_timeout_last_positions`, `b5_travel_timeout_min_distances`,
`b5_self_job_reachability_probe`. **Any gate comparing against them yields a
guaranteed red.** *The harness is **scalar**-deterministic; that scope is not a
detail.*

---

## 4 · THE QUEUE BUDGET — **LAWFUL WAITING THAT LOOKS LIKE STALLING**

`bastion_jobs.rs`, emergency-route queue: a colonist waiting its turn on a ladder
is granted

    QUEUE_WAIT_BASE_SECS (120) + QUEUE_WAIT_PER_TURN_SECS (90) × min(position, 8)

**pos 0 → 120 s · pos 3 → 390 s · pos 8 → 840 s.** *While within budget the
colonist is `WaitingForLadder` and its `stuck_watch` is wiped every pass; when the
budget lapses the suppression stops and the stuck machinery engages.*

★★★ **This class is NOT a defect — it is designed patience, and it was
misdiagnosed as stalling.** *Item 2's `ACCESS_STALL_SECS` was set to **120**,
exactly the position-0 budget, so the pruner fired precisely where the queue
mechanism was already releasing. A correct threshold must exceed **840 s**; runs
are 340–540 s; **a correctly-set pruner would essentially never fire.*** The
constant is now **derived in code** from the budget (`+ one PER_TURN unit` as
margin, so even the margin cannot drift).

---

## 5 · THE SUSPENSION STATE — **THE SURVIVING RESIDUAL** (successor row)

**One clean post-fix specimen** (uid=166, farming confirm 13b):

    on_ground=false   on_wall=false   character_state=Idle   velocity.z=0.0
    climb_free_active=true            terminal_cause="egress_no_route_then_climb_free_expired"
    egress_verdicts=11  egress_plans_emitted=0  egress_no_route=10

★★★ **A colonist suspended in mid-air — not standing, not climbing, not
falling.** *The egress planner's ten `no_route` verdicts were **correct**: there
is no walkable start node. And it was not idle-refusing —
`organic_destination` **was** computed, nine blocks away at the same z.*

> ★★★★ **BOTH REGISTERED READINGS ("planner correct about geometry" / "planner
> refusing work") ASSUMED THE START STATE WAS VALID and split on the planner's
> competence. The specimen rejected the shared premise.** *Enumerate what your
> alternatives both assume — the answer may live there, and neither option can
> reach it.*

### ★★ THE RESIDUAL IS THREE CLASSES, NOT ONE — measured across three specimens

    uid=166   on_ground=FALSE   11 verdicts, 0 plans, 10 no_route   "egress_no_route_then_climb_free_expired"
    uid=109   on_ground=true     0 verdicts                          "below_grade_watch_without_egress_verdict"
    uid=81    on_ground=true     6 verdicts, 0 plans,  0 no_route    "egress_plan_or_climb_free_failed"

★★★ **Only uid=166 is suspended.** *uid=109 got NO verdict at all — the opposite
failure from 166's eleven; one planner said "no route" repeatedly, the other was
never consulted (`access_jobs_pending=0`, so the known access-mutex chain is NOT
the cause). uid=81 has six verdicts, none of them route failures, on the ground,
with a destination computed — **item 4's original suspicion in its purest form**,
which re-opened that row as **4b**.*

> **Post-sit-fix the residual is STILL multi-mechanism. "Not one thing" is the
> dossier's organizing lesson arriving a second time, one level down.**

### THE `climb_free` MECHANISM — **READ, AND IT SPLIT THE QUESTION IN TWO**

    :6651   climb_free_until = max(climb_free_until, time.0 + 45.0)
    :6835   climb_free_now   = climb_free_until > time.0
    :8062   if climb_free { vel.0.x = drive.x; vel.0.y = drive.y; }    // Z NEVER WRITTEN

**A colonist under `climb_free` is DRIVEN horizontally by direct velocity writes
every tick, with z untouched. The grant lapses on a 45-second CLOCK regardless of
where the colonist is.** ★ *uid=166's `velocity = (-0.79, 0.13, 0.0)` — horizontal
residue, z exactly zero — is the fingerprint of a velocity-driven entity whose
driver was removed off-surface.*

**Two questions, and only one is answered:**

1. **Why is it not being driven?** ★ **ANSWERED** — the grant expired.
2. **Why is it not FALLING?** ★★★ **OPEN — and this is the actual suspension.**
   *Leading candidate: a colonist flipped to `SimulationMode::Simulated` may not
   have physics run at all, which would make this the simulated-mode freeze seen
   from the physics side. Discriminator: uid=166's mode at the fail-safe.*

> ## ★★★★ **DESIGN OBSERVATION (Fable), INDEPENDENT OF HOW #85 RESOLVES:**
> **A CLOCK-EXPIRING, POSITION-BLIND MOVEMENT GRANT IS A SMELL ON ITS OWN.**
> *A driver that lapses mid-air by timer rather than ending on SURFACE-ARRIVAL
> will strand its passenger under any physics regime.* **Whatever the suspension
> turns out to be, the grant's end condition probably belongs to arrival, not the
> clock.**

**Also unresolved:** the fail-safe logged *"teleporting to ground"* while `d.z`
was **five blocks above** `feet.z`.

---

## 6 · WHAT THE ARC ESTABLISHED ABOUT METHOD

1. ★ **A symptom covering N mechanisms needs N witnesses before it needs a fix.**
   *Every fix "worked"; only separation showed which one mattered.*
2. **A fix whose precondition is random must emit when it fires** — otherwise
   zero failures cannot distinguish fixed from unexercised.
3. **Eliminating a dominant class is also an INSTRUMENTATION result**: it raises
   the signal-to-noise for every remaining class.
4. **A threshold must be derived from the mechanism that bounds it**, never
   calibrated from a distribution that mechanism generates.
