# THE CORPUS HAS BEEN MEASURING THIS DEFECT FOR 25 WAVES AND CANNOT ATTRIBUTE IT

**Data: `wave25_BASELINE_e86fe79893_FULL.json`, 48 seeds, 89 keys. Read from
disk, no run.** Cross-checked against waves 14–19 (73–75 keys) — same picture.

## §1 — THERE IS NO REST, BED, SLEEP, PREEMPT, OR NEED FIELD. AT ALL.

Scanning every key in the widest corpus schema for
`rest|bed|sleep|preempt|need|despond` returns **six fields, and not one of them
is about a colonist's needs:**

```
b5_any_needs_materials            <- job MATERIALS, not colonist needs
b5_hill_reaches_crest             <- name collision on "rest"
b5_max_same_target_timeouts
b5_mine_timeout_position_diag
b5_timeouts_on_never_completed_jobs
b5_travel_timeouts
```

> ★ **The 48-seed corpus CANNOT SEE the AUTON-2 defect.** Not "it reports zero" —
> **it has no instrument.** Any claim that the fan would have caught a colonist
> that never sleeps is **false by construction**, and so is any claim that a
> post-fix fan proves the fix worked. [[enumerate-what-the-instrument-can-see]]

**Two of the six are name collisions** (`needs_materials`, `reaches_crest`) —
★ *exactly the "match on VALUE EXPRESSIONS, not names" trap.* A keyword sweep
that stopped at the field list would have reported **"needs coverage exists."**

## §2 — ★★★★★ AND YET THE MECHANISM IS RIGHT THERE, 222 TIMES

AUTON-2 §4c established the rest failure **is a travel timeout**. The corpus
counts travel timeouts. So it has been recording the *exact mechanism* all along:

| field | nonzero seeds | total | max |
|---|---|---|---|
| `b5_travel_timeouts` | **31/48** | **222** | 31 |
| `b5_timeouts_on_never_completed_jobs` | 10/48 | **75** | 17 |
| `b5_max_same_target_timeouts` | 31/48 | 62 | 6 |

**222 travel timeouts. 75 of them on jobs that NEVER COMPLETED — which is
precisely the shape of a colonist released on the way to a bed.** And the corpus
**cannot tell you the job kind of a single one**, because the field is a bare
count.

> **THE COLLAPSE IS THE DEFECT.** `travel_timeouts: u32` answers *"did travel
> stall?"* and **no adjacent question** — not *which kind*, not *whose*, not
> *toward what*. A rest-travel timeout and a mine-travel timeout are
> **indistinguishable by construction**. [[aggregate-late-keep-the-structure]]

★ This is that law's **most expensive instance to date**: not a measurement we
failed to take, but **one we have been taking, at scale, across 25 waves, in a
form that destroys the attribution at the moment of recording.**

## §3 — THE CHEAPEST POSSIBLE UPGRADE, AND WHY IT IS THE RIGHT ONE

**Replace the counter with a map keyed by job kind.** `travel_timeouts_by_kind:
HashMap<&'static str, u32>` at the single increment site (`~11487`, next to the
existing `board.travel_timeouts += 1`).

- **One producer, one line.** The existing counter stays; the map is additive, so
  every historical baseline remains comparable and the holdcheck sees a **new
  field**, not a moved one — `--expect-new` covers it exactly.
- **It answers the open question with NO new run design.** The very next fan says
  whether `RestAt` appears among the 222. **If it does not, AUTON-2 §4c's
  structural read is refuted by the corpus** — which is why this is worth doing
  *before* the fix, not after.
- **Observability budget: per-EVENT, not per-tick.** A timeout is already a rare
  logged event; this adds a hash insert to a path that fires ~5×/seed.
  ★ *This is the one shape [[the-instrument-changes-what-it-sees]] does not
  indict* — the bisection there condemned **per-cell, per-tick** reads.

## §4 — WHAT THIS DOES NOT SAY

- ★ **It does not say the 222 timeouts are rest jobs.** They are almost certainly
  dominated by mine/haul travel; the corpus scenarios may not create beds at all.
  **The map is how we find out, not a prediction of what it will show.**
- **It does not say the corpus is bad.** It was built to gate mining rows and it
  gates them well. **A corpus is only ever an answer to the questions someone
  thought to ask** — and nobody had asked about sleep.
- **It does not authorize a fan.** No behaviour has changed; there is nothing to
  fan yet.

## §4b — ★★★★★ WITHDRAWN: MY OWN FALSIFICATION CRITERION WAS UNSOUND

§3 said *"if `RestAt` never appears among the 222, §4c's structural read is
refuted by the corpus."* **Withdrawn. I specified a refutation without checking
that its precondition holds.**

`bastion-harness/src/main.rs` carries **separate scenarios** — `b5_scenario`
(**1943**) and `bed_scenario` (**11951**). **The corpus fan runs `b5`** (every
field is `b5_`-prefixed), and beds exist only via `board.beds.insert`
(**12614**) when a `DesignationKind::Bed` job **completes**.

> **The b5 corpus almost certainly contains NO BEDS. A by-kind map would show
> zero `RestAt` for a reason unrelated to whether §4c is correct — and I would
> have read that zero as refuting myself.**

★ **A falsifier must assert its own precondition.** I broke that rule inside the
same document that praises refutation-capable tests. **A wrong falsifier is worse
than none:** it manufactures a confident *"refuted"* from an instrument that was
never looking — precisely the defect §1 above indicts, committed one section
later. **The map remains worth building as a permanent attribution upgrade. The
test attached to it is gone.**

## §4c — ★★★ THE REAL INSTRUMENT EXISTS, AND THE CASE HAS A DESIGNED ANSWER

`preempt_scenario`'s own doc (**336-341**):

> *"an unreachable bed degrades to **ENDURE** (works through the cooldown, meter
> keeps decaying, no livelock, zero embeds)."*

**The unreachable-bed case is not unknown — it is a NAMED, SPECIFIED
degradation.** So the 6-initiation trace is one of two things, with different
fixes:

- **(a) ENDURE works as designed** ⇒ the complaint is that *enduring forever* is
  a bad end state. **A design row, not a bug.**
- **(b) ENDURE never engages** ⇒ a documented, designed live path that does not
  fire. **The gate-must-test-live-path class.**

★ And `preempted_rested` — the expected-red we have circled for two days — **is
that scenario's own acceptance test.** Its needs are **force-set**, so it reaches
the band by construction and is independent of the decay tuning entirely.

**Order: read `preempt_scenario` before building any new instrument.** It may
already print the discriminator.

## §5 — THE STANDING LESSON

> **Before believing a corpus is silent on a defect, ENUMERATE THE FIELDS IT
> CARRIES.** Silence from an instrument with no channel for the signal is not
> evidence. And when the channel *does* exist, check whether the **aggregation**
> destroyed what you needed before it reached disk.

Sibling of [[temp-tree-evidence-files-vanish]]'s central law in a new position:
**here the exclusion happens at WRITE time, not read time** — by the time anyone
looks, the distinction is already gone, and no amount of careful reading recovers
it.
