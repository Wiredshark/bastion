# APEX T4–T9 — cross-tier dependency map (v1)

One page. Row-level edges only. Authored alongside the T4–T9 tier specs;
several edges below were discovered while writing them and are **not**
implied by tier numbering. **Resume order should come off this map, not
off the numbers.**

Legend: `A ⇒ B` = A is a hard prerequisite of B. `A ⇢ B` = A's output is
carried/consumed by B. `⟂` = deliberately independent.

---

## Hard prerequisites (within tier)

```
T4.4 ⇒ T4.5 ⇒ T4.6        save side: inventory → corpus → staged epochs
T4.1 ⇒ T4.2               manifest → its freshness binding
T5.2 ⇒ T5.3               a receipt needs a complete frame to receipt
T6.1 ⇒ T6.3               inventory the surface before freezing its order
T6.2 + T6.4 ⇒ T6.5        probes + profile before any kernel substitution
T7.1 ⇒ T7.2 ⇒ T7.3 ⇒ T7.4 ⇒ T7.5     strict chain, gated at T7.1
T8.1 ⇒ T8.2 ⟂ T8.3 ⟂ T8.4 ⇒ T8.5     one gate, three parallel lanes, one decision
```

**T4.5 ⇒ T4.6 is a correctness claim, not sequencing taste.** Staged-epoch
adoption must read an old pointer-less save directory as *epoch zero*
rather than as corruption. Without the corpus that case is untested and
the first boot after adoption is a coin flip.

**T7.1 is a gate, not a preamble.** Its scope decision *is* T7.2's kernel
interface (which `JoinData` fields are transition inputs vs ambient
access). Deciding it during implementation means nobody reviewed it.

---

## Cross-tier carries

```
T3.6 ⇢ T5.2, T5.3, T7.3, T9.1     physics generation: eligibility + history invalidation
T3.5 ⇢ T7.4, T9.1                 command journal: terminal replay on reconnect
T3.5 ⇢ T4.2                       monotone-sequence-with-floor, reused not re-derived
T4.3 ⇢ T4.4, T4.6                 world baseline root recorded into inventory + save manifest
T5.4 ⇢ T5.2                       weather snapshot id IS the frame's environment reference
T6.4 ⇢ T8.2                       Lane A's cells ARE numeric profiles
T6.4 ⇢ T4.1                       numeric protocol is equality-critical in the bootstrap manifest
T7.2 ⇢ T4.1                       kernel version is equality-critical in the bootstrap manifest
T8.5 ⇢ T4.3, T4.6                 remedy rungs (3)/(5) move the baseline root; (6) is a save-manifest change
T4.6 ⇢ T9.2                       branching restores a committed epoch
T0.4 ⇢ T9.2                       UniverseBranchId already exists; T9.2 uses it, does not define it
everything ⇢ T9.3                 the certificate is generated from tiers' attestations
```

**T5.4 ⇢ T5.2 inverts the numbering.** The weather snapshot must exist
before the input frame can reference it, so T5.4 lands *before or with*
T5.2, never after.

**T8.5 ⇢ T4.3/T4.6 was not in the rows.** The economy remedy decision
cannot land before the T4 save sequence is ready to carry it — three of
the six ladder rungs change artifacts T4 owns.

---

## Shared types (build once, three consumers each)

```
raw/semantic probe pair     ⇠ T5.3, T6.2, T8.1
```
Same non-certification rule in all three: a semantic match must never be
able to certify raw equality, and that should be **unrepresentable**
(distinct types, no `From`, no cross-comparison) rather than documented.
Three instances is the threshold — build it once, in T6.2, and name its
three consumers in the type's own doc.

```
identity nesting            ⇠ T3.5 (commands), T5.2 (inputs), T7.3 (history)
```
`(boot, session, connection epoch, generation, sequence)`. A sequence is
only meaningful inside the identity it is scoped to; every one of these
rows needs the same nest.

---

## The three-mechanism composition (state once, cite everywhere)

```
T3.6 generation   decides which frames are ELIGIBLE
T5.2 input seq    ORDERS eligible frames
T3.5 LatestState  picks the WINNER among them
```
Three mechanisms, three jobs. Per-row re-derivation is how
three-mechanism systems drift into two; collapsing any two presents as
"prediction is occasionally wrong under load" — the kind of bug that gets
misattributed to netcode for months.

---

## Startable now (no unmet prerequisite)

| Row | Classification | Note |
|---|---|---|
| **T4.4** | READY after T0.5 | read-only: enumerate + digest, no writes to any save |
| **T5.1** | READY after T3.1–T3.3 | cohort must be disjoint from the moderation force list |
| **T6.1** | READY after T0.5 | scanner in the T3.5 bypass-scanner shape |

Everything else is `PREREQUISITE-MISSING`, `NEEDS-DESIGN` (T7.1, T8.5),
`CONDITIONAL` (T6.5), or `DEFERRED` (T9.3).

---

## Traps that the numbering hides

1. **Do not pre-empt T8's lanes.** The three order seams at
   `world/src/site/economy/context.rs:200/209/219` are an afternoon each
   to canonicalise. Landing that before Lane B measures it destroys the
   evidence *and* changes every generated world without knowing the
   change was necessary or sufficient.
2. **T6.3 before any kernel work.** A pure ordering fix with no gameplay
   change. Until the contribution tape is reproducible there is no way to
   tell whether substituting a transcendental helped or moved the noise.
3. **T7.2 is a one-way door.** Once client prediction and server
   authority share a kernel, every later character-behaviour change
   changes both. That is the row's purpose and the reason T7.1 gates it.
4. **Presentation-only exclusions need evidence.** T5.4's `local_wind`
   looked presentational and reached glider steering. T6.1 must not
   accept an exclusion on assertion.
5. **T9 should not invent.** If a builder reaches for a new mechanism
   there, an earlier tier under-delivered — raise it rather than closing
   the gap where it will be invisible.

---

## Suggested resume order (off this map)

1. **State::client feature-invariance** — written, unverified, blocks all
   combined-workspace floors.
2. **T3.6 step 2** — live-path wiring; types and rules already landed.
3. **CKPT-174 / ECS preflight** — remaining named-OPEN, catalog-backed.
4. Then the startable frontier: **T6.1** (cheapest, unblocks T6.3, and
   T6.3 is the highest-value ordering fix in the program), **T4.4**,
   **T5.1** — in that order, because T6.1 → T6.3 has the longest
   downstream reach.
