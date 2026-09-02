# PREREG — the assist is the last writer; a window is not a hurdle (W4)

Registered 2026-09-02 14:35, before the binary exists. Source: Ben's
session of 13:56 (RESULTS-bens-session-2026-09-02-1356.md) and arm b1's
W2c boot.

## Defect

A body at a fence or a window with the route head two blocks beyond never
crosses. MOVE ASSIST says "the vault/step completes"; the feet do not
move; the glide is refused into the hurdle every tick. Ben: uids 36, 37
(fence, 24 and 19 assists in 45 s) and 233 (window, 16). b1: 814 vault
assists in a day made of three (colonist, head) pairs x687, x127, x63.

## Mechanism (read, not guessed)

1. `pending_kinematic` receives the mover's hold/glide for the body BEFORE
   the assist decides; the assist drain writes the promised cell; the
   kinematic drain runs AFTER it and writes the stale position back
   (bastion_jobs ~32582/32654/32668 push; ~37004 assist drain; ~37132
   kinematic drain).
2. The router's vault edge accepts any waist-band solid sprite (path.rs
   ~1926); windows are in the band and the body refuses them
   (`blocks_colonist_body`) — generator and consumer disagree.

## Fix

- `assist_outranks_rival_writes(pending, entity, at)`: retain every other
  body's writes, drop this body's, append `(entity, at, 0, "assist-wins")`.
  Identity switch: `BASTION_NO_ASSIST_OUTRANK`.
- `common::path::is_hurdle(&Block)`: band AND NOT `blocks_colonist_body`;
  the router and the body both call it. WitchWindow added to the body's
  exclusion.

## Instrument validation (must pass before the bars are read)

- DID NOT STICK: on the S4b boot (today's order), reading the saved log
  with the same (colonist, head) repeat count must show the three-cell
  shape (a top pair > 50). The witness itself does not exist there; the
  repeat count is computed from the MOVE ASSIST lines, so the two
  instruments are compared on the W4 boot: the witness's exact count (<=16
  exact, then sampled) must agree with the line count within the sampling.
- OUTRANKS THE MOVER must fire on the W4 boot at least once with
  `dropped >= 1` (if it never fires, the rival write did not exist in the
  drain and the mechanism claim is wrong — FAIL regardless of the bars).

## Pre-registered outcomes (b1, day 0 by 18:00, W4 boot vs the S4b boot)

| measure                                   | control (S4b boot) | W4 bar   |
|-------------------------------------------|--------------------|----------|
| DID NOT STICK (from the MOVE ASSIST lines)| ~811 expected      | <= 10    |
| worst (colonist, head) repeat             | hundreds           | <= 3     |
| vault assists total                       | ~814               | <= 100   |
| MOVE ASSIST lines with front=Window       | > 0 if any window is met | 0  |
| GLIDE REFUSED INTO ROCK                   | ~29                | <= control |
| probes / shuns                            | S4b's              | within 2-3x |
| panics                                    | 0                  | 0        |

PASS = every row. FAIL branches:
- repeats persist with OUTRANKS firing -> a third writer after the
  kinematic drain (physics or the rtsim sync) — run with
  BASTION_POS_WRITE_DIAG and read the site names.
- repeats persist and OUTRANKS never fires -> the hold was not in
  pending_kinematic; the drain order is not the mechanism; re-read.
- vaults land but probes rise at field edges -> bodies vault INTO fenced
  fields whose exits the router does not price; the field-gate row.
- window fronts stay > 0 -> a window kind outside blocks_colonist_body;
  name it from the line.
- routes that used windows go Exhausted (probes with route_head at a
  house wall) -> the door row, not this one.

NOT evidenced live yet. Ben's next session is the acceptance test: no
colonist standing at a fence or a window for more than a hop.
