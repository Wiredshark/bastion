# SELF-RESCUE HAS A 100% REFUSAL RATE IN THE CORPUS — 71 CALLS, ZERO PLANS

**Computed from `wave25_BASELINE_e86fe79893_FULL.json` and
`wave26_ROWA_d5b56d1c79_FULL.json`, 48 seeds each. READ FROM DISK, NO RUN.**
Counter semantics verified at `5f8cdf1392`. **Identical in both waves.**

## §1 — THE NUMBERS

| caller | calls | emissions | **refused** | rate | seeds w/ refusal |
|---|--:|--:|--:|--:|--:|
| **`self_rescue`** | **71** | **0** | **71** | **100%** | 15/48 |
| `emergency` | 579 | 78 | **501** | **86.5%** | 41/48 |
| `proactive_descent` | 0 | 0 | 0 | — | 0/48 |

**`self_rescue_starved` = 0 in every seed of both waves.**

## §2 — WHY THE NUMBERS MEAN WHAT THEY APPEAR TO MEAN

**The fields' own doc comments settle the semantics** — I did not infer them:

> `access_plan_calls`: *"Counts every invocation **regardless of outcome**."*
> `access_plan_emissions`: *"per-caller **SUCCESSFUL** `plan_access` emissions
> (the `Some((kind, steps))` arm) … `emissions <= calls` always; **the gap is
> refusals, not non-calls.**"*

And the **non-call** half is separately instrumented: `access_plan_self_rescue_
starved` counts requests killed by the colony-global `access_pending` bar before
the loop body ran. **It is zero in all 96 seed-runs.**

> ★ **So all 71 self-rescue requests genuinely reached `plan_access`, and
> `plan_access` refused every single one.** Not starved, not un-called, not
> rarely-successful. **Zero plans, ever.**

## §3 — ★★★★★ WHAT THIS IS

**The mechanism that exists to dig a stranded colonist out has never once
produced a plan in the entire corpus.** The need arises regularly — **71 times,
across 15 of 48 seeds** — and the answer is always nothing.

And it is **deterministic**: byte-identical totals across two waves at different
commits. **This is not a flake; it is a standing property of the system.**

★ **The emergency path is the same defect at 86.5%**, in **41/48 seeds** — the
broader and better-exercised population, refusing five of every six requests.

## §4 — WHAT THIS DOES **NOT** ESTABLISH — the WHY is still open

`plan_access` has **several** refusal gates, and this measurement does not
distinguish them:

1. ★ **the designation-mask gate** — `allowed = in_access_mask(mask, p)`, the
   condemned-cell hypothesis (`CONDEMNED-CELL-FINDING.md`)
2. `unavailable_cells` — plan cells overlapping other jobs / live emergency routes
3. **geometric failure** — no walkable stair base adjacent to the digger
4. the **M2 walkability validation** — vertical rises the digger cannot stand to work

> **The approved discriminator is still exactly the right next step, and it is
> now MUCH better motivated:** it no longer asks *"does this ever happen?"* — the
> corpus answers that — but *"which of four gates closes on 71 out of 71."*

★ **Do not read this as confirming the mask hypothesis.** It confirms the
**refusal**, which the mask hypothesis predicts and so do three others.

## §5 — ★★★ THE META-FINDING: THE INSTRUMENT WAS BUILT FOR THIS AND NEVER READ

These counters exist **because of DECISIONS #49** — the previous instance of
*"the corpus cannot see access-plan state."* The field doc says so outright:

> *"the question #61's non-call-vs-rejection falsification needed and the corpus
> couldn't answer (zero access-plan state visible anywhere)."*

**The instrument was correctly identified, correctly specified, correctly built,
and has been faithfully recording a 100% failure rate ever since — and nobody
subtracted one column from the other.**

> **AN UNREAD FIELD AND AN ABSENT FIELD PRODUCE THE SAME SILENCE.** Adding the
> instrument was never the finish line. **A measurement nobody computes is
> indistinguishable from one nobody took.**

★ This is [[enumerate-what-the-instrument-can-see]]'s companion failure, and the
cheaper one to fix: **the corpus is UNDER-READ**, and the fix is a standing
derived-quantity check — *calls − emissions*, per caller, every wave — not a new
field.

## §5b — ★★★★★★ THE WHY IS FOUND, AND THE ABSORBER IS MEASURED

**Continued reading closed both halves. §4's four-gate list was incomplete — the
decisive gate is a FIFTH one I had not listed.**

### The tier table — self-rescue is the only caller flying with one wing

`plan_access`'s ladder tier (**976-978**) lights on:

```rust
None if AUTO_LADDER_ACCESS                       // const = FALSE  (85)
    || emergency_owner.is_some()
    || (dig_provisioned && DIG_PROVISIONED_LADDER_ACCESS)   // const = true (98)
```

| caller | `dig_provisioned` | `emergency_owner` | **ladder tier** | measured |
|---|---|---|---|---|
| **`self_rescue`** (13142) | **false** | **None** | ★ **DARK** | **0/71** |
| `emergency` (13855) | false | **Some** | LIT | 78/579 |
| `proactive_descent` (16096) | **true** | None | LIT | 0 calls |

> ★★★ **Self-rescue is the ONLY caller of the three whose ladder tier is dark,
> and the ONLY caller with a 100% failure rate.** It gets **stairs or nothing** —
> and a colonist stranded in a pit or shaft is precisely the geometry where
> stairs cannot route. **That is what the emergency path's 78 emissions have that
> self-rescue does not.**

`AUTO_LADDER_ACCESS = false` is **deliberate** (B6 hotfix, Ben live-test): the
auto-pillar caused a queue-fight. **Not a bug — a ruled trade-off.**

### ★★★★★ AND THE ABSORBER, NAMED IN THE FLAG'S OWN COMMENT, MEASURED IN THE CORPUS

> *"the universal **teleport-to-ground fail-safe** (B6, entombment impossible by
> construction) backstops any colonist a stair can't reach."*

| measurement | wave25 | wave26 |
|---|--:|--:|
| self-rescue carve calls | 71 (15/48) | 71 (15/48) |
| self-rescue plans emitted | **0** | **0** |
| **`b5_rescue_fired`** (ultimate fail-safe) | ★ **44/48** | ★ **44/48** |

> **THE COLONY'S DIG-YOURSELF-OUT MECHANISM NEVER WORKS, AND THE TELEPORT
> FAIL-SAFE FIRES IN 92% OF SEEDS.**

★ **This is [[refusal-needs-refusal-aware-consumers]] in full:** *"if a wrong
value has worked for months, find what's absorbing it FIRST."* **Found it.** And
`b5_rescue_fired` is deliberately **report-only** — an Opus fan review previously
caught it being wrongly gated and reverted it. **So every layer is behaving as
designed, and the composition is still wrong. No wrong site, again.**

### ★★★★★★ THE CONSEQUENCE THAT MATTERS MOST — IT CHANGES THE PRE-REGISTERED FIX

DECISIONS #65 pre-registered *"rescue outranks paint"* — exempt self-rescue from
`board.designated`. **On this evidence that fix alone may change nothing.**

> **THE MASK AND THE DARK LADDER TIER ARE EACH SUFFICIENT TO PRODUCE ZERO
> EMISSIONS. Removing one sufficient blocker while another remains yields the
> same 0/71 — and ships looking like a fix.**

★ **This is the matched-control law pointed at a REPAIR instead of an
experiment:** *what would this fix have shown if my explanation were wrong but
the failure still occurred?* **Answer: exactly the same numbers.** So the
discriminator must now name **which gate closes**, and the fix must clear **every
sufficient blocker** or be explicitly scoped as partial.

### ★ HONEST SEVERITY REVISION — I OVERSTATED IT

**Colonists are not dying and not permanently condemned.** The fail-safe catches
them; entombment really is impossible by construction. **"Condemned cell" is the
wrong name for what I measured.** The true defect is narrower and still real:

- **Colonists TELEPORT instead of digging, in 44/48 seeds.** An immersion and
  legibility defect at the player surface — *the thing a player watches happen.*
- **The entire self-rescue carving subsystem is effectively dead code**, while
  its call site fires 71 times and its refusal is invisible behind the backstop.

## §6 — CONSEQUENCE FOR ROW PRIORITY

The condemned-cell row was banked **third**. **A rescue mechanism with a measured
100% failure rate, deterministic, in 15/48 seeds, is not a third-priority row** —
and the emergency path's 86.5% across 41/48 seeds is a *wider* population than
anything AUTON-2 touches. **Re-ranking is Fable's call; the number is the
argument.**
