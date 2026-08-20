# Arc 9 — THE COLONY MIND (#107) — PRE-REGISTRATION (v1 scope cut)

**Charter (Ben direct):** a colony-level drive arbiter (Sustain / Grow /
Defend / Expand) weighted by board-computed colony needs.

**Substrate already producing every input v1 needs:**
- food-days: `colony_food_stock` ÷ (population × eat rate) — both live.
- housing ratio: `board.beds` vs colonist count — both live.
- threat: the hostile-proximity census (#93) — live.
- wealth: the stockpile census (item 34's signal) — live.
- The per-colonist Arbiter (AUTON-0) already arbitrates individual drives;
  the colony mind is the SAME shape one level up.

## Build shape (v1 = one consumer, not a brain)

1. **`ColonyDrive` (Sustain|Grow|Defend|Expand)** computed at a slow
   cadence from the four inputs by fixed thresholds (data constants, not
   learned): food-days < 3 ⇒ Sustain; threat ⇒ Defend; else housing
   short ⇒ Grow; else Expand. Deterministic, witnessed per transition
   (drive, the input that decided, its value).
2. **ONE consumer to prove the wire** (the charter's own lesson: a signal
   nobody consumes is decorative): the colony WORK-PRIORITY vector tilts
   by drive — Sustain raises Farm/Cook/Haul; Defend raises Guard; Grow
   raises Build. Rides `bastion_set_work_priority` (the LIVE route item
   16 proved), so the tilt is visible in the priority witnesses that
   already exist.
3. Display: the colony inspect payload carries (drive, inputs) —
   same-source fill.

## BARS

1. A planted food shortage (seed 0, eat down) flips the drive to Sustain
   and the priority vector tilts measurably (the item-16 priority
   machinery scores it); restoring food flips it back. Both directions.
2. A hostile presence flips to Defend over Grow (precedence witnessed by
   the transition line naming the deciding input).
3. Twin determinism.
4. The null: a satisfied colony sits in Expand/Grow with ZERO transition
   churn (flip-flop guard: transitions require the deciding input to
   cross threshold + hysteresis band, witnessed).

VOID branches: an input producer returns a degenerate constant (report
the value); the priority tilt is delivered but no behavior moves (the
item-16 A/B already proved priorities bite, so this reads as a wiring
break, not a design gap).

**Strategic/tactical planning (Ben's hive-mind line) is EXPLICITLY the
later layer** — v1 is the reactive arbiter only; planning builds on its
proven inputs.
