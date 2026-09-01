# PRE-REGISTRATION — housing is BEDS, not houses (Ben's ruling, 2026-09-01)
Base: bastion/item29-trade @ a770cc11c4. Written before the change exists.

## THE RULING (verbatim intent)
A house may hold several colonists — families, friends, farm hands. Everyone
should have their OWN bed, except spouses (share one) and young siblings
(share one). A colonist with NO bed is allowed ("it's only what they have")
but takes a MOOD penalty. It is a modifier, never a refusal.

## WHAT THE OLD RULING BUILT, site by site
  bastion_housing_cap(wanted, houses) = wanted.min(houses)     -> cap by house
  immigration_target_pop = houses with beds > 0                -> target = houses
  courtship no_home = !free_bed_in_house(host, mover)          -> needs a FREE bed
  crowded = shared - families  (shared = members > 1)          -> >1 is crowded
  two pins in server/src/rtsim/mod.rs assert "one per house"
Ben's flat-map village: 83 residents, 38 houses, 76 beds -> adopted 8 (env
wanted=8; the cap by house would have allowed 38). Under this ruling the
ceiling is ~76 plus sharers.

## THE CHANGE
C1. Cap at founding: wanted.min(house_plots * SLOTS_PER_HOUSE). Beds are not
    registered at founding (adopt_beds_surface needs loaded terrain), so the
    plot count times the worldgen bed count (2) is the honest estimate.
    FALLBACK IS IDENTITY where SLOTS_PER_HOUSE == 1.
C2. immigration_target_pop = sum of beds across households (not a house
    count). A house with 4 beds targets 4.
C3. courtship no_home -> "the host has a bed at all". Spouses SHARE, so a
    free second bed is not required. The mover keeps or drops their own bed
    as today.
C4. crowded = members > beds + couples_in_house (+ young sibling pairs when
    ages exist). A family of 3 in a 2-bed house with one couple = not
    crowded.
C5. A daily ChronicleKind::SleptWithoutABed thought for any colonist whose
    bed is None at the sleep block, feeding the existing thought_sum. This is
    the mood modifier. Magnitude is TASTE (Ben's number to set); shipped
    small and named.
C6. The two old-ruling pins are rewritten to assert the NEW ruling, with the
    old assertion quoted beside them as history.

## PASS / FAIL, pre-registered
B1. Ben's flat village adopts >= 38 (wanted permitting) instead of 8, and
    `left_as_npcs` falls accordingly.
B2. HOUSEHOLDS reads crowded=0 for a household of {couple + child} in a
    2-bed house, and crowded=1 for 3 unrelated adults in a 2-bed house.
B3. Courtship fires for a pair where the host's house has NO free bed (was
    refused no_home).
B4. A bedless colonist's mood is lower than a bedded one's by the named
    magnitude, and the thought appears in the inspector.
B5. Nothing starves or freezes: fed/rested/idle within the flat-arm bands;
    residual 0.

## WHAT FALSIFIES THIS
- F1. B5 fails at the higher roster -> the town cannot feed 38; that is the
  FARMLAND row arriving as a failure, not a reason to keep the old cap.
- F2. Sleep collapses because bed CLAIMS now exceed beds -> the bed
  assignment (B7-2) still assumes one owner per bed and needs a sharing
  model before this ships. Measure `B7-2 assigned` vs beds.
- F3. Crowded households never form because the courtship still requires a
  free bed somewhere -> C3 was applied at the wrong site.

## NOT EVIDENCED / JUDGEMENTS
- The mood magnitude (C5) is Ben's.
- "Young sibling" needs an age; children have born_day, adults from
  adoption do not. Sibling sharing ships as couples-only until ages are
  universal, and says so.
