# ROW: ITEM-6 CORPUS WITNESS + ITEM-2 THRESHOLD UNBLOCK

**Status:** APPROVED (Fable, `1795ca2738`) — *"witness row APPROVED (refusals-by-reason,
pile-pickup attribution, reserved-units), lands with item-2's threshold changes,
one fan scores both."*

**Batching is part of the ruling: ONE commit range, ONE fan, BOTH parts.** A fan
is ~25 min and the two rows are independent in code and joint in cost.

**Engine reads in this packet are at `07ba0cc17b`** (the attested wave33 binary).
Note the checkout trap: `bastion/block-B6HAUL` is the evidence branch and does
**not** contain `cbfb8ae977`. Read engine code with `git show <tip>:path`, never
from `HEAD`.

---

## WHY THIS ROW EXISTS — THE FINDING, READ AT SOURCE

wave33 could neither implicate nor clear item 6 for the 5 corpus movers, because
**no field matching `pile|protect|ambient|loot|provision` exists in any of the 48
seeds.** The cause is not a missing counter. It is a **mute channel**:

`server/src/events/inventory_manip.rs:86` @ `07ba0cc17b`

```rust
fn record_pickup_verdict(tick: u64, picker: u64, item: u64, verdict: &str, extra: String) {
    if bastion_server::bastion_flight_recorder::enabled() {
        ... note: format!("item={item}; verdict={verdict}; {extra}"), ...
    }
}
```

Item 6's refusals **are** recorded, and the commit message is accurate that both
layers are "counted via `record_pickup_verdict` under their own reasons." But:

1. **It is an EVENT STREAM, not a tally** — nothing aggregates.
2. **It is gated on `flight_recorder::enabled()`**, which a corpus fan does not
   set. The fan reads stdout JSON; this writes elsewhere, when switched on.
3. **The reason is buried in a free-text `note`** — `verdict={verdict}` inside a
   formatted string, so even with the recorder on, reading it means parsing prose.

> **Three independent reasons the corpus is blind, any one of which suffices.**
> Clearing only the `enabled()` gate would still leave an unparsed stream with no
> aggregate. Name all three or the fix looks done and isn't.

The three verdict reasons that exist at `07ba0cc17b`: **`bastion-pile-protected`**,
**`ambient-loot-disabled`**, **`loot-owned`**.

This is the same family as task #78 (the AI-side gate refuses silently, layer 1
silencing layer 2's counter) — **and #78 is subsumed by part B below**, because
counting by reason at the server layer makes layer-1 silencing visible as a
*shortfall between layers* rather than as an absence.

---

## PART A — ITEM 2: UNBLOCK THE THRESHOLD CALIBRATION

**The problem, restated so this packet stands alone:** `b5_f3_stalled_peak` was
added to set `ACCESS_STALL_SECS` from measured data. `access_stalled_secs`
**resets to 0 at the threshold** (`bastion_jobs.rs:15199`), so the field is
**right-censored by the constant it exists to calibrate** — 8 of 48 wave33 seeds
report exactly 120.0 with true dwell unmeasured. **More seeds cannot fix this.**

### A1. Make both thresholds env-tunable

`bastion_jobs.rs:15032` and `:15040` @ `07ba0cc17b` are hard-coded `const`s:

```rust
const ACCESS_STALE_SECS: f32 = 20.0;
const ACCESS_STALL_SECS: f32 = 120.0;   // PROVISIONAL
```

Read from env with these defaults, following the existing `BASTION_*` pattern.
Suggested names: `BASTION_ACCESS_STALE_SECS`, `BASTION_ACCESS_STALL_SECS`.

- **Log the EFFECTIVE value at startup, always** — not only when overridden. A
  defaulted config and a set one must be distinguishable in the log, or a
  mis-set arm is indistinguishable from a correct one and the whole A/B is void.
- **A malformed value must REFUSE (fail loud), never silently fall back.** A
  silent fallback to 120.0 in the arm that was supposed to raise it produces a
  confidently wrong calibration.

> **MANDATORY, SAME COMMIT (Fable's ruling names this explicitly): register both
> vars in the host-input manifest** (`common/src/host_input_manifest.rs`) **and
> keep the config-route sync tests green** (the CI tests from #62). A new env var
> that skips the manifest is a silent hole in the determinism env surface — that
> route was closed deliberately and must not reopen.

### A2. Emit the FINAL stall value beside the peak

Add `b5_f3_stalled_final` (or equivalent) carrying `access_stalled_secs` **as of
run end**, alongside the existing `b5_f3_stalled_peak`.

**Why it is required and not a nicety:** the peak alone cannot distinguish
*"stalled 119 s then recovered"* from *"still stalling when the run ended."*
Identical value, opposite meanings — one is health, one is the bug. **wave33 seed
59 is exactly this case** (`stalled_peak = 119.0`, `prunes_fired = 0`, one second
under the wire) and it is **the single most informative seed in the wave and
currently unreadable.**

Same capture discipline and doc style as `b5_f3_stalled_peak`; `#[88]` LIVE-EMIT
declaration required on the accessor (`ported (FLAG)` or `harness-only -- reason`),
plus the SNAPSHOT-vs-ACCUMULATOR note — **`final` is a SNAPSHOT, `peak` is an
ACCUMULATOR**, and that distinction is the entire point of adding it.

---

## PART B — ITEM 6: THE CORPUS WITNESS

Three counters, named by Fable's ruling. All are **integer accumulators on the
board**, emitted in the harness JSON — the proven, corpus-carryable pattern from
#70's six F3 accumulators. **Do not route these through the flight recorder.**

### B1. Refusals by reason (the core of the row)

One counter per verdict reason, **kept separate — never summed into a total**:

| counter | reason string @ `07ba0cc17b` |
|---|---|
| `b5_pickup_refused_pile_protected` | `bastion-pile-protected` |
| `b5_pickup_refused_ambient_disabled` | `ambient-loot-disabled` |
| `b5_pickup_refused_loot_owned` | `loot-owned` |

> ## AMENDMENT (Fable's registered prediction, `1795ca2738` follow-up) — **BREAK EACH REASON DOWN BY PICKER CLASS**
>
> **A flat per-reason counter cannot test the prediction that has been registered
> against this fan, so the flat design is insufficient as written.**
>
> **THE REGISTERED PREDICTION, recorded here PRE-DATA, candidate-not-claim:**
> *if `ambient-loot-disabled` refusals fire on **colonist** pickers, the five
> movers are explained and the gate has a membership/timing bug.* Mechanism
> sketch, for the record only: the belt-and-suspenders predicate
> (`rtsim_entities.contains(entity) && bastion_colonists.get(..).is_none()`)
> depends on the `Colonist` component being present on the loaded entity **at
> pickup time** — and the `is_loaded` saga established that entity↔npc state
> timing is exactly where such predicates wobble.
>
> ### ★ CORRECTED — **THE FIRST VERSION OF THIS AMENDMENT WAS UNFALSIFIABLE** (5b's catch, verified at source)
>
> **The split as first written could not work for two of the three reasons.**
> Verified at `inventory_manip.rs` @ `07ba0cc17b`:
>
> - **`:315-316`** — `bastion_piles.contains(item_entity) && bastion_colonists.get(entity).is_none()`
> - **`:342-343`** — `rtsim_entities.contains(entity) && bastion_colonists.get(entity).is_none()`
>
> **Both branches carry `is_none()` as an ENTRY CONDITION.** A `_colonist` counter
> inside either reads the same component the gate just tested, at the same
> instant, and is **0 BY CONSTRUCTION**. The claim that `== 0` "kills the
> prediction clean" was **false** — it is 0 whether the race is real or not.
> **Not a weak test: a non-test.**
>
> ### THE CORRECTED DESIGN
>
> | reason | split? |
> |---|---|
> | `bastion-pile-protected` | **NO** — flat counter. Predicate fixes the value. |
> | `ambient-loot-disabled` | **NO** — flat counter. Same reason. |
> | `loot-owned` | **YES** — `..._loot_owned_colonist` / `..._loot_owned_ambient`. Its predicate (`loot_owner.can_pickup`, via groups/alignments/stats/players) **never touches `bastion_colonists`**, so a colonist genuinely can be refused here. Real signal. |
>
> **The timing prediction is tested by a DEFERRED READ, correlated by uid:**
>
> - at refusal under `ambient-loot-disabled`, record the **picker's uid** (and tick if free)
> - **at run end**, check which of those uids are colonists *then*
> - emit `b5_pickup_refused_ambient_uids` and **`b5_pickup_refused_ambient_later_colonist`**
>
> ★★★ **Why this works where the split did not: THE SECOND READ HAPPENS AT A
> DIFFERENT TIME FROM THE BRANCH PREDICATE.** *The tautology came from reading the
> same component in the same instant; a deferred read is not constrained by the
> branch that recorded the uid.* **`later_colonist > 0` means an entity refused as
> ambient turned out to be a colonist — the late-component race. `== 0` genuinely
> kills it.**
>
> **CONFOUND TO CLOSE, and it decides how decisive this is:** if colonists can be
> **recruited mid-run**, a uid may legitimately be ambient at refusal and a
> colonist at run end with no race. **If they are seeded at startup and membership
> never changes, the confound vanishes and `later_colonist > 0` is decisive
> alone.** Establish which and state it in the commit message; if mid-run
> recruitment is possible, record the refusal tick and read the delta — a small
> gap is a race, a large one is recruitment.
>
> ### ★ THE LAW THIS BROKE, STRENGTHENED
>
> > **Writing both branches as field expressions is NECESSARY AND NOT SUFFICIENT.
> > VERIFY THE FIELD CAN ACTUALLY TAKE BOTH VALUES WHERE IT IS PLACED.**
>
> **A field expression that is constant by construction has ONE branch, however
> many you write.** *A counter inside a branch cannot vary on a predicate that
> branch has already fixed.* — see [[a-registered-prediction-is-a-requirement-on-the-instrument]].

> **Separate counters, not a total, and not a bool.** A summed
> `refusals_total` answers "did anything refuse" and no adjacent question — and
> the adjacent question (*which layer refused*) is the entire diagnostic value.
> Aggregate late; the corpus can always sum, and can never un-sum.

**Increment at the same sites that call `record_pickup_verdict` today** — the
verdict is already computed there; this adds a counter beside an existing call,
not a new decision point.

### B2. Pile-pickup attribution

Counters distinguishing **who** successfully took from a `BastionPile`:

- `b5_pile_pickup_by_member` — a colony member took from its own pile (expected)
- `b5_pile_pickup_by_nonmember` — anyone else succeeded (**should be 0** under
  membership-only protection; a nonzero value is the protection leaking)

**This is the pair that makes the row falsifiable.** A refusal count alone cannot
distinguish *protection working* from *nobody ever tried*.

### B3. Reserved units

Fable's third named field. **Read the existing one before adding a new one:**
`b5_stack_reserved_units_max` already exists and is **0 across all 48 wave33
seeds** — a dead field in this scenario. **Determine whether it is dead because
nothing reserves, or because the accessor never wired**, and say which in the
commit message. If it measures correctly and this scenario simply never
reserves, **do not add a second producer to the same store** — that is
[[new-producer-must-fit-the-stores-unit]], and wave33 already cost a false alarm
on exactly that mistake.

---

## ACCEPTANCE FRAMEWORK

### Observable success measures

1. **Every one of the new fields is present in all seeds of the next fan** —
   validated by a schema checker BEFORE any value is read. Absent ≠ zero.
2. **`b5_pickup_refused_*` is nonzero in at least one seed**, proving the gate is
   exercised. A corpus where nothing ever refuses cannot score item 6 either way.
3. **`b5_pile_pickup_by_nonmember == 0` in every seed** — the protection invariant.
4. **Part A:** with the stall threshold raised, `b5_f3_stalled_peak` **no longer
   spikes at the threshold value**. That is the direct, visible proof the
   censoring is gone.

### Named failure modes, each with a planted test that must go red BY NAME

- **Counter counts sites, not firings** — increments where the verdict is
  computed but the refusal doesn't actually happen. *Plant:* force a path that
  computes a verdict and proceeds; the counter must not move.
- **A reason is silently dropped** because layer 1 refuses before layer 2 counts
  (#78's exact shape). *Plant:* disable layer 1; the layer-2 counter must rise by
  the amount layer 1 was absorbing.
- **Malformed env value silently defaults.** *Plant:* set
  `BASTION_ACCESS_STALL_SECS=banana`; the run must REFUSE, not run at 120.
- **Manifest drift:** a new env var absent from the manifest. *Plant:* the
  existing #62 sync test must fail if the manifest row is omitted.

### Non-vacuity

The corpus scenario must **demonstrably exercise** the pickup path: at least one
successful pile pickup **and** at least one refusal in the same wave. If the wave
produces refusals but zero successful pickups, the scenario is not a valid
control and the result is **VOID, not PASS** — the zero-cases discipline.

### Corpus, not a single run

Scored on **one 48-seed fan** covering both parts, against the wave33 registered
baseline via `--baseline` (never auto-select — see #67; the auto-select read of
wave33 reported 2 movers where the registered read reported 5).

### What a PASS does NOT establish

A pass proves the **witness works and the invariant holds in this scenario**. It
does **not** retroactively explain the 5 wave33 movers — those remain PARKED and
need this instrument plus a real A/B, since the wave32→wave33 range is a
16-commit bundle. **Say so in the results, so a green fan is not over-read into
an attribution it cannot support.**

---

## COST / DENSITY NOTE

These are integer increments on an existing board struct at sites that already
compute the verdict — **not** flight-recorder events. Diag density is budgeted
(two diag reads once broke bit-reproducibility), and this row deliberately adds
**no new event stream**, only aggregates. **Determinism: counters must not feed
any decision** — diagnostics only, never clause terms.
