# PRE-REGISTRATION — the colony cannot build a house
Base: bastion/item29-trade @ 856132601b. Written before any code exists.

## DEFECT (located, not guessed)
`JobBoard::queue_build_plan` is complete: it freezes the region's empty cells,
joins the claim mask, registers the region as `DesignationKind::Build`, and
the generator pass mints the jobs. Colonists then claim, haul and build them.

Its ONLY callers are `bastion-harness/src/main.rs` and the thin server
wrapper `Server::bastion_queue_build_plan`. NOTHING in the colony's own logic
ever calls it. The whole pipeline exists and is exercised by a test harness.

Consequence, measured in the owner's real session: `Build jobs created: 0` in
86 game days, and `adopt_beds_surface` says so in its own log line --
"ADOPT-IN-PLACE house registered (no build jobs minted)". Population is
one-per-house, houses only ever come from the adopted village, and the two
towns founded today offered 10 and 2. So the colony is permanently capped at
the village it inherited, and the GROW horizon cannot be reached at all.
rtsim/mod.rs already names this: "A PERMANENT SIGNAL BLOCKING ITS OWN CURE."

## PRIOR ART (the mechanism, not the story)
Banished, Dwarf Fortress, RimWorld and Manor Lords all use the same five-part
shape: a BLUEPRINT that (1) reserves a site, (2) bills materials, (3) is
claimed as ordinary work, (4) is built by haulers/builders, (5) on completion
REGISTERS a usable structure. The colony already has 1-4. Only 5 and the
decision to start are missing. Borrowed specifically from Banished: housing is
the limiting resource and population grows to fill it -- which is already
Ben's ruling here, just with no way to add housing.

## THE CHANGE
1. A colony-level decision, in the drive/growth path beside `HOUSING GROWTH`,
   that queues ONE build plan when housing is short and materials allow.
2. The plan places a bed, so `adopt_beds_surface` over the finished region
   registers it and `houses` grows -- reusing the existing registration
   rather than inventing a second one.
3. It witnesses itself: a `HOUSING BUILD` line naming fired/refused and the
   deciding term, in the same shape as HOUSING GROWTH / BIRTHS / COURTSHIP.

## PASS / FAIL, pre-registered
H1. `Build jobs created` > 0 in a live run -- the pipeline is reached at all.
H2. At least one plan COMPLETES and a bed is registered: `houses` strictly
    increases from its founding value.
H3. Population then exceeds the founding house count. This is the whole
    point: today it is arithmetically impossible.
H4. The witness names a refusal when it declines (no silent lane).
H5. Nothing regresses: census `stuck` <= 1, `residual` 0, EMBED WATCH does
    not rise above the post-856132601b baseline, hauls within 30%.

## WHAT FALSIFIES THIS
- F1. Build jobs mint but never complete -> the same "minted against but never
  fetched" trap the trade lane's `not_stockpiled` guard exists to prevent. If
  materials are not stockpiled the plan must REFUSE BEFORE IT SPENDS, not
  queue a job nobody can finish.
- F2. H2 fails while H1 passes -> the colony can build walls but not housing;
  the registration half is wrong and the feature is cosmetic.
- F3. Haul share explodes -> building starves every other lane. Bounded by
  ONE plan at a time.
- F4. The town builds where a player would not (in the plaza, on a farm, on a
  road). Site choice is the part most likely to look wrong on a flyover, and
  a bad site is worse than no house.

## NOT EVIDENCED / STATED ASSUMPTIONS
- That one plan at a time is the right rate. It is a guess chosen to bound
  F3, not a measurement, and it is exactly the kind of number Ben should
  overrule if it looks wrong on a flyover.
- Whether the colony HAS the materials. Ben's town held 0 stockpiled wood
  (`log.wood = not_stockpiled(0/180)`), so on his world this row may refuse
  every day for a legitimate reason -- and that refusal, named, is still a
  better answer than silence.
