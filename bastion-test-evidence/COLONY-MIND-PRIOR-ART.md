# COLONY MIND — PRIOR ART (#107's rider)

**Standing rule: survey prior art before designing, and a READ beats a
description.** *A description that travelled unverified into a design doc cost a
rework once already, so this leads with what is verifiable in-repo and marks
external recall as recall.*

---

## ★★★ THE STRONGEST PRIOR ART IS IN THIS REPO: `rtsim::data::Architect`

**Veloren already has a director that does structurally what #107 charters — at
WORLD scale.** `rtsim/src/data/architect.rs`, its own doc:

> *"The architect has the responsibility of making sure the game keeps working.
> Which means keeping the simulation in check, and making sure interesting stuff
> keeps happening."*

```rust
pub struct Architect {
    pub deaths: VecDeque<Death>,
    /// This is calculated on startup. And includes both dead and alive.
    #[serde(skip)]
    pub population: Population,      // indexed by TrackedPopulation
}
```

**It measures a world-level aggregate (population by category), and applies
corrective pressure (respawn something similar) — without commanding any
individual.** ★ *That is the charter's "weights on existing generators, never
orders to individuals," already shipping one scale up.*

> ## **THE COLONY MIND IS THE COLONY-SCALE ARCHITECT. That framing is not an
> analogy — it is the same pattern at a different radius.**

### ★★ THREE DESIGN DECISIONS WORTH COPYING, READ NOT ASSUMED

1. ★★★ **THE AGGREGATE IS RECOMPUTED, NOT PERSISTED.** *`population` is
   `#[serde(skip)]` and "calculated on startup."* **The events persist; the
   derived state is rebuilt.** *So a save can never carry a stale aggregate that
   disagrees with the facts underneath it — and nobody has to write migration for
   a number that is a function of other numbers.* **The colony metrics should do
   exactly this: persist nothing derived.**
2. **IT KEEPS THE QUEUE, NOT JUST THE COUNT** — `deaths: VecDeque<Death>`, not
   `death_count: u32`. ★ *Aggregate-late, already practised here: the structure
   survives so later questions ("deaths of what kind, when") remain answerable.*
3. **ITS ONLY CURRENT DRIVE IS SUSTAIN** — *"keep the world from dying out."*
   ★★ **The engine's own director shipped with ONE drive.** *That is independent
   support for the two-drive pilot: the precedent did not start with four either.*

---

## EXTERNAL PRIOR ART — **MARKED AS RECALL, NOT VERIFIED**

*I cannot read these codebases from here. Recorded as design intuition to be
checked before anything load-bearing rests on it — per the rule that a
description is not a read.*

- **Dwarf Fortress** — colony-level state surfaces as *reports* and thresholds
  (food/drink stocks, "nobody is hauling") rather than a central planner; the
  player is the executive. ★ *Relevant negative: DF deliberately has NO colony
  brain, and its emergent interest comes from the player supplying it. #107 is
  choosing differently, and should say why in the design doc.*
- **RimWorld** — a *storyteller* modulating pressure on the colony from outside,
  rather than a colony deciding for itself. **A different axis from #107's
  arbiter**: the storyteller is adversarial pacing, the Colony Mind is
  self-regulation. *Both could coexist; conflating them would be a design error.*
- **Ant-sim literature** — stigmergy plus **response thresholds**: individuals
  switch tasks when a stimulus crosses a personal threshold, and colony-level
  allocation emerges without any central decision. ★★ **This is the closest
  formal match to what already exists here** (job board = stimulus field, demand
  formulas = thresholds), *which means #107's arbiter is a deliberate departure
  from pure stigmergy toward a weak central signal — worth stating plainly rather
  than presenting as a natural extension.*

## ★ WHAT THE SURVEY CHANGES ABOUT THE DESIGN

1. **Frame the Colony Mind as the colony-scale `Architect`**, and reuse its
   persistence discipline (recompute derived state) rather than inventing one.
2. **The one-drive precedent supports the two-drive pilot** — the world director
   still has exactly one drive today.
3. ★★ **State the departure honestly: this adds a weak central signal to a system
   that is currently pure stigmergy.** *DF proves a colony sim can be excellent
   with no colony brain at all; the reason to add one here is the god-interface
   and RL substrate, not that the ants need it.*
