# ITEM 4 — SCORED ON ONE LIVE SPECIMEN

> ## ★★★ **QUALIFIED THE NEXT DAY — READ THIS FIRST.**
> **The population grew to three fail-safes and one of them is a COUNTEREXAMPLE
> to the reasoning below.** *The verdict on uid=166 stands on its own evidence;
> the verdict on the CLASS does not.*
>
>     uid=81   on_ground=TRUE   access_jobs_pending=0
>              egress_verdicts=6   egress_plans_emitted=0   egress_no_route=0
>              organic_destination=Some(...)   head_clear=true
>
> **Six verdicts, zero plans, and ZERO of them `no_route`** — on a grounded
> colonist, with no mutex engaged and a destination computed. ★★ **None of the
> invalid-start-state excuse that closed this row applies to it.** *That is item
> 4's original suspicion in its purest form.*
>
> ★ **I scored this row at n=1 and the n=3 population contains a case the closing
> reasoning cannot cover — the specimen-is-not-the-population error, committed
> one day after banking its two-specimen-diff sibling.** **Reopened on uid=81 (or
> re-scoped to "the suspended case is explained; the ground case is not"),
> architect's ruling. The sharp open question: WHAT ARE THOSE SIX VERDICTS, if
> none is `no_route`? One grep of the verdict enum starts it.**
>
> *Also routed by the same read: uid=109 has `access_jobs_pending=0`, so it is
> NOT the known access-mutex chain — it is a genuinely new dispatch question
> (zero verdicts, egress never consulted, and the mapped reason absent).*



**Specimen:** farming confirm run (script-13b), `2026-08-11T03:20:10`, uid=166
"Peri of the Vale".
**Log:** `.engine-integration-wt/bastion-test-evidence/live-playthrough/server-stdout-farmseed.log`
**§1a identity check passed:** the log carries **131** `GOTO-STAND-RESCUE` lines,
so it is demonstrably post-#94 code. *(The rule that caught the wrong log two
hours ago, applied to the right one.)*

## THE VERDICT — **NEITHER REGISTERED READING. A THIRD ONE.**

    egress_verdicts=11   egress_plans_emitted=0   egress_no_route=10
    terminal_cause="egress_no_route_then_climb_free_expired"

**The two readings on the table were:** *(a)* the planner is **correct** about
hard geometry, or *(b)* the planner is **refusing work it should do** — the row's
original suspicion. **The specimen supports a third:**

> ## **THE PLANNER IS CORRECT, BUT NOT ABOUT GEOMETRY — IT IS BEING ASKED TO ROUTE FROM A POSITION THAT IS NOT A VALID STANDING LOCATION.**

    on_ground = false        on_wall = false
    character_state = Idle   velocity = (-0.79, 0.13, 0.00)   <- z EXACTLY zero
    head_clear = true        climb_free_active = true

★★★ **The colonist is SUSPENDED IN MID-AIR** — no ground contact, no wall
contact, idle, and **not falling**. *A pathfinder asked to route from there has
no walkable start node, so `no_route` ten times is the correct answer to a
malformed question.*

★★ **And the planner was not idle-refusing: `organic_destination = Some(Vec3 {
15220, 16016, 421 })` — a destination WAS computed**, nine blocks away **at the
same z**. *The planner did its half; the route from a non-standable origin is
what failed.*

## SO: ITEM 4'S ORIGINAL SUSPICION IS **NOT SUPPORTED** — AND THE DEFECT MOVES UPSTREAM

**"Verdicts without plans" is real, reproduced post-sit-fix with the sit confound
removed by construction — and it is a SYMPTOM.** The row asked whether the
planner refuses work it should do. **On this specimen it does not.** The
question that replaces it:

> **How does a colonist end up suspended — `on_ground=false`, `on_wall=false`,
> idle, zero vertical velocity — with `climb_free` expired?**

*`terminal_cause` names the sequence: egress found no route, then climb-free
expired. The state that makes both inevitable is the suspension itself.*

## LIMITS, STATED PLAINLY

1. ★★★ **n = 1.** *A specimen, not a distribution.* The harvest run (13c, longer)
   produced **zero**.
2. **The chain is INFERRED, not witnessed.** *The log carries no prior climb-free
   lines for uid=166 — only the terminal record. "Climb-free expiry left it
   suspended" is read from `terminal_cause` plus the state, not traced step by
   step.*
3. ★★ **I cannot prove a route existed.** *There is no reachability probe for that
   position. "The planner is correct" rests on the invalid-start-state reading —
   NOT on demonstrating that a valid-origin route was available and refused.*
4. **Unresolved oddity, recorded rather than smoothed:** `feet.z = 421` but
   `d.z = 426`. If `d` is the fail-safe's destination, it teleported the colonist
   **five blocks UP** while the log says "to ground." *Not resolvable here; worth
   a look if anyone touches that fail-safe.*

## THE ASYMMETRY IS ITEM 3'S RESULT, NOT ITEM 4'S

**1 fail-safe in ~230 s versus 0 in ~490 s**, against a pre-fix baseline where
this was routine. **The same runs carry 373 rescues.** *The fail-safe went from
the colony's daily bread to a single event across ~12 live minutes — that is
item 3's victory measured in item 4's data, and it is why item 4 finally has a
clean specimen at all.*
