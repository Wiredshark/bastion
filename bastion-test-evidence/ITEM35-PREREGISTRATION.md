# Item 35 (Injuries & medicine) — PRE-REGISTRATION

**Substrate, read not assumed:** injuries EXIST — cave-ins damage health +
drop Mood (`cavein_eject_and_injure`, FR11 Q6), hostiles damage colonists
(item 13's flee-health branch), and vanilla `Health` carries the fraction the
inspect payload already displays. Vanilla healing exists as items/sprites
(potions, food regen). **Nothing colony-side RESPONDS to a wounded colonist:
no rest-to-heal, no tending, no medic.**

## Build shape (v1 = the response loop, not a medical sim)

1. **Wounded self-care**: health below a threshold weighs into the arbiter's
   need scoring (the item-11/B7 pattern) → the colonist takes RestAt; bed
   rest regenerates health slowly (reuse `Health::change_by`; rate derived
   from a requirement — heal-to-work in N game-hours — not invented).
2. **A TEND job (the medicine half)**: a healthy colonist claims Tend at a
   wounded-in-bed colonist (the co-located-pair shape item 22's producer
   proved); tending multiplies the regen rate. Witness: tender, patient,
   rate-before/after.
3. **Witnesses**: wound (source, fraction), rest-heal tick sum, tend events
   — treatment beside outcome per patient.

## BARS

1. A planted wound (cave-in fixture or the flee-health plant) is followed
   by: the wounded colonist reaching a bed, health regenerating to the
   work threshold, and RETURN TO WORK — the whole loop witnessed.
2. Tended arm vs untended arm (A/B, same wound): tended heals measurably
   faster; both directions stated.
3. The null: an unwounded colony shows ZERO heal/tend events (couldn't-
   happen witness = the wound witness's absence plus health=1.0 census).
4. Twin determinism.

VOID branches: the wound plant doesn't reach the subject (item 14's lesson —
print treatment beside outcome); beds unbuilt (the item-23/27 materials wall
— cleared first or the leg is VOID by precondition); regen rate zero by
config (report the number).
