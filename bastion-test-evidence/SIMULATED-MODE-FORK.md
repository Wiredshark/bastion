# THE SIMULATED-MODE FORK — PRICED

**The finding (5b, endurance run 02:37→03:42):** founding colonists freeze into
`SimulationMode::Simulated` ~20 s post-founding, and **need-preemption has been
inert for every founding to date.** *Every prior live leg had a client keeping the
world loaded; the unattended gate is the first run to meet the architecture's
real default state.*

> ★★★ **The colony cannot currently live unattended BY CONSTRUCTION — which is
> exactly what item 8 exists to certify, and it was found because the gate was
> ruled unattended for an unrelated reason (an observer is a variable).**

---

## ★ CORRECTION TO THE FORK'S PREMISE — **(a) IS NOT MOSTLY BUILT**

**The hope was that the overseer terrain anchor already carried most of (a). It
does not.**

    bastion_terrain_anchor consumers:  client/src/lib.rs  x7      (:2758, :2768, :3508, :3514, :3595 …)
                                       common/src/comp/presence.rs — the field itself
                                       SERVER-SIDE: none

★★ **The anchor is a CLIENT-side view hint — it tells a connected client where to
keep terrain loaded. It does not make the SERVER hold chunks with no client
present.** *The field's name made it look like the mechanism; its consumers say
otherwise.*

**And `PresenceKind` has no playerless variant:**

    Spectator · LoadingCharacter(CharacterId) · Character(CharacterId) · Possessor

**Every one is a client/player concept.** *(a) therefore requires a NEW
server-owned presence kind, not the reuse of an existing one.*

## AND (b) IS DEEPER THAN "WIRE UP THE TICK"

**The `rtsim` crate has NO needs concept at all** — grepping `hunger|rest|need`
there returns only the English word "need" in doc comments. **Needs live entirely
in `bastion-server` / `server`, i.e. ECS-side.**

★ *So (b) is not "let the existing simulated tick carry needs." It is
**introducing needs to rtsim**, which today does not model them.*

---

## THE FORK AS IT ACTUALLY STANDS

| | (a) SERVER-SIDE ANCHOR-LOAD | (b) SIMULATED-MODE NEEDS |
|---|---|---|
| **what changes** | ★ **scope** — WHAT stays loaded | ★ **model** — needs become rtsim-native |
| **needs/jobs/planner code** | **untouched, one implementation** | relocated or **duplicated** |
| **new concept required** | a playerless `PresenceKind` + its view distance | needs in `rtsim`, and a satisfaction path without an ECS body |
| **cost now** | **bounded and known** | **substantial** |
| **cost at scale** | ★★ **grows with colony COUNT — every colony permanently loads chunks** | ★★ **flat — a colony far from any player costs nothing** |

> ## ★★★ **THE FORK IS A SCALING DECISION, NOT A CORRECTNESS ONE. BOTH MAKE THE GATE PASS.**

★★ **The consideration I'd weight heaviest, because this programme has paid for
it twice this week: (b) risks TWO IMPLEMENTATIONS OF NEEDS** — ECS-side for
loaded colonists, rtsim-side for simulated ones — **unless needs are moved
wholesale.** *Two producers of one fact is the defect we removed from
`record_pickup_verdict` and audited out of the counters. A hybrid (c) inherits it
by construction, plus a new consistency question: what happens to a need that
crosses while simulated and then loads?*

## RECOMMENDATION — **(a), scoped honestly, with (b) named as the eventual answer**

1. **(a) unblocks item 8 now and preserves one implementation of everything.**
2. **Its scaling limit is real and should be written down at the same time**:
   *server-held chunks per colony, growing with colony count.* **That is a known
   debt, not a hidden one.**
3. **(b) is the right end state for a world with many colonies** — and it is a
   *needs-in-rtsim* row, sized accordingly, not a wiring task.

★ **What I would NOT do: hybrid (c) first.** *It buys the least and inherits the
duplication of (b) plus a cross-mode consistency question nobody has scoped.*

## WHAT THIS MEANS FOR ITEM 8 REGARDLESS OF THE RULING

**The endurance gate cannot certify "the colony sustains itself unattended" until
one of these lands** — *every measure in its bar assumes colonists are ticking.*
★★ **But the run already produced the arc's most valuable result**: **the promise
"a world that lives without being watched" is currently false, and it was
invisible for the entire arc because every prior leg had a watcher.**

*That is the is_loaded saga's final form, and it took an unattended run to see it.*
