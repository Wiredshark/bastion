# PRE-REGISTRATION — the flat gate excludes every real village
Base: bastion/item29-trade @ d0b289d29c. Written before the change exists.

## DEFECT (measured)
    bastion_adopt_sort_key, AUTOFOUND arm:
        (flat_bucket, -fields, -houses, d2)
    flat_bucket = (alt_range <= 8.0) ? 0 : 1     -- BINARY, and it sorts FIRST
Live, a 16,384-block search:
    considered=42  chosen_houses=2 chosen_fields=1 chosen_flat=TRUE
    alt_range=5.86  dist=10,998   runner_up: 3 houses, 0 fields
Only ONE flat site in 42 has a field at all. Ben's own town — 10 houses,
2 fields — has alt_range 80.8 and is therefore bucket 1: the scorer will
NEVER autofound on a village like the one he plays. A player who lets the
game choose gets a hamlet; only a player who picks gets a town.

## WHY THE GATE EXISTED, AND WHY THAT REASON IS NOW SPENT
Flatness protected pathing on relief. This session measured relief pathing
end to end and fixed it: embeds PRE 96.2/134.8 per 10k vs POST 0.0/0.4/0.7/
2.5/8.7/10.0 (n=2 vs n=6, non-overlapping). I explicitly DECLINED this row
earlier "not on this evidence alone" — that evidence now exists.

## THE CHANGE
Demote flatness below capacity in the AUTOFOUND arm only:
    (-fields, -houses, flat_bucket, d2)
The PLAYER-PICK arm is untouched — "the town you picked is the town you get"
stays exactly as it is.

## PASS / FAIL, pre-registered
F1. `chosen_houses` rises: the autofound site has >= 4 houses, up from 2.
F2. `chosen_fields` does not fall (fields already sort first among ties).
F3. Embeds on the chosen site stay inside the measured post-fix band
    (<= ~10 per 10k). Founding on relief is only safe because of that band.
F4. Nothing starves: fed >= 50% sustained, residual 0, stuck <= 1.
F5. The colony still founds at all — `ADOPT-A-TOWN VOID` does not appear.

## WHAT FALSIFIES THIS
- F1 fails -> capacity was never the tiebreak that mattered and the site pool
  is simply poor; revert, and the row becomes "worldgen villages near spawn
  have no fields", which is a different problem.
- F3 fails -> relief pathing is NOT fixed well enough to found on, the -95%
  was site-specific, and this reverts immediately. This is the real risk and
  the reason the row waited.
- F4 fails on a bigger town -> feeding does not scale with population, which
  is the untested case I named (10 colonists on 2 fields) arriving as a
  failure rather than a measurement.

## NOT EVIDENCED
- That a bigger adopted town behaves at all. Every arm this session has been
  2-5 colonists. F4 is deliberately the loosest bar because this is the first
  time the colony will be asked to run at ~10.
