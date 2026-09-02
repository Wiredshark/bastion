# RESULTS — a promised scramble is taken (W6): the instrument read, and the control that W3 had already changed

Read 2026-09-02 18:38 against PREREG-terrace-scramble.md.

## The instrument read (W6a, on the D1c pair b260c71a0c)

Arm b2 (8-day year), booted 17:46, cut at day-1 15:00 (tick 67,500):

| measure                    | F1 pair (no W3) | D1c pair (W3 in) |
|----------------------------|----------------:|-----------------:|
| probes                     | 228             | 72               |
| probes at the terrace cell (7748, 6328) | 209 | 22            |
| starving at the cut        | 20 of 50        | 2 of 50          |
| shuns                      | 230             | 93               |

`assist_why` over the 72 probes: head_far 35, eligible_climb 21,
committed_walker 11, no_head 4, head_is_feet 1. At the terrace cell:
eligible_climb 18, committed_walker 4 — the two-up head is standable
(the z+3 map shows air above it), within reach, and the assist WOULD
have taken it at the 10-second clock; the stall was re-targeted or
expired first (climb assists fired once all day, step 21, vault 21).
Both W6 arms address exactly this: the promised climb fires at the hop
clock (1.5 s), and a trunk walker is allowed it.

Arm b1 (raid arm, normal year, D1c and G1c boots): committed_walker 20
of 27 probes, no terrace probes on those boots (the terrace population
is a b2 phenomenon on the compressed year: crops mature in a day and
the eaters go to the plateau store).

## What W3 had already done

The F1-pair boot on b2 (no W3) is the baseline the pre-registration
was written against. The D1c-pair boot (W3 in, no W6 yet) already
reads 22 terrace probes and 2 starving at the same cut: a far target
starting at Medium finds a way the Small search did not, for most
eaters. W6's pre-registered bars are therefore read against BOTH
controls, and the honest baseline for W6 alone is the D1c-pair boot
(22 / 72 / 2).

## Also named by the read

- The step residual: 13 DID NOT STICK repeats of class step, every one
  with `last_push_site = bridge-refused-rock`, marker on, on ground —
  the search-gap glide pulling the body back after a re-target (W7).
- head_far 35: the largest refusal class is a route head more than two
  cells away — a trunk walker between distant waypoints, or a chaser
  that advanced past its nearest node; not an assist problem (the next
  probe class to read: W8).

## The W6 read (arm b2 on 552cac9f76, booted 18:39, cut at day-1 15:00; read 19:30)

| by day-1 15:00                 | control (D1c boot) | W6 boot | bar            |
|--------------------------------|-------------------:|--------:|----------------|
| probes                         | 72                 | 132     | <= 60  FAIL    |
| probes at the terrace cell     | 22                 | 36      | <= 20  FAIL    |
| EatFrom probes                 | 37                 | 66      | <= 15  FAIL    |
| starving at the cut / max      | 2 / 7              | 3 / 11  | <= 3  PASS (at the cut) |
| climb assists                  | 1                  | 4       | --             |
| step assists                   | 21                 | 185     | --  (9x)       |
| vault assists                  | 21                 | 18      | --             |
| DID NOT STICK (step)           | 13                 | 16      | --             |
| climb bans                     | 31                 | 48      | --             |
| embed / net events             | 1,055              | 2,036   | not above control  FAIL |
| assist-apply position writes   | 56                 | 221     | --             |
| shuns                          | 93                 | 144     | --             |
| store deposits                 | 120                | 127     | --             |
| panics                         | 0                  | 0       | 0  PASS        |

`assist_why` at the terrace is unchanged in shape (eligible_climb 37,
committed_walker 17): the head is eligible and the climb assist still
does not fire (4 all day). The mechanism named by this read: the
assist's clock is `active.stuck_time`, which only accrues while the
mover pushes nothing; at the foot of the step the chaser's override
glide bumps the body a fraction of a block every tick, `displaced`
resets the clock, and 1.5 s never elapses — the hop clock W6 lowered is
a clock the terrace never winds. The probe's own watcher (no NET
displacement over its window) is the clock that does fire there.

The step surge (21 -> 185) and the embed count (1,055 -> 2,036) are
either W6's doing or replicate variance (counts vary 2-3x). The same
pair is re-run on b2 with BASTION_NO_SCRAMBLE_ASSIST=1 (the identity
switch) as the matched control; if the surge persists with the switch
on, it is not W6's.

## Disposition (one replicate)

W6 FAILED its probe, terrace and embed bars and left the climb assist
unfired; the design's clock was wrong, not its arms. Next: W6-B — take
a promised, standable, adjacent two-up head from the STALL WATCHER's
clock (first_stall), where the probe already proves the body is not
advancing, rather than from the assist's stuck_time. The switch-on
control read decides whether the step/embed surge is W6's.
