# PRE-REGISTRATION — do need BANDS actually break the hunger monopoly?

Written BEFORE the run. Arm: tip (bands + determination), commit 087e18fcc2.
Baseline: the measured 118 hunger : 26 rest (4.5:1), 3 replicates, pre-band.

## The mechanism, and the prediction it FORCES

Old comparator: `b.1.cmp(&a.1)` — kind first, unconditionally. Hunger is kind 1.
So whenever BOTH needs were candidates, hunger won 100% of the time; value
never got consulted because it only broke ties WITHIN a kind, and with two
kinds there are none. Not "usually" — always.

New comparator: band first, kind (life-threat) as the within-band tiebreak.

Contest only happens when BOTH are below their interrupt (0.2). So:

| rest band | hunger band | old winner | new winner |
|---|---|---|---|
| Pressing | Pressing | hunger | hunger (tie -> life-threat) |
| Urgent   | Pressing | hunger | **REST** |
| Dire     | Pressing | hunger | **REST** |
| Pressing | Dire     | hunger | hunger |
| Dire     | Dire     | hunger | hunger (tie) |

★ HONEST LIMIT OF THE MECHANISM, stated before measuring: hunger decays at
EXACTLY 2x rest, so hunger reaches a deeper band FIRST and ties are resolved to
hunger. Banding does NOT hand rest a majority and I do not predict one. It
converts the "always" into a "usually". The case it genuinely fixes is
rest-deeper-than-hunger, which the old rule could not express at all.

★ AND THE CASE IT DOES **NOT** FIX, registered now so I cannot claim it later:
a colonist that cannot REACH food sits at hunger Dire permanently. Once rest
also reaches Dire they tie, and hunger wins again. For that colonist the
band change alone changes nothing. What covers it is the Sleep-block schedule
rule (hunger refused at night with an empty pack), NOT the bands. If the run
shows sleepers improving, I must attribute it to the schedule unless I can
separate them.

## PASS / FAIL, declared now

- **PASS**: hunger:rest preempt ratio falls materially below 4.5:1, AND distinct
  sleepers >= 7 of 8, AND no colonist has 0 rest preempts while having >5 hunger
  preempts (that pairing is the monopoly's signature).
- **FAIL (monopoly intact)**: ratio still >= 4.0:1 and >=2 colonists show the
  0-rest / many-hunger pairing.
- **FAIL (overcorrected)**: rest preempts overtake hunger, or `ate` collapses.
  A gate that refuses everything also stops the bug reproducing.
- **VOID (looks like either)**: run does not reach the Sleep block (~tick
  31,800), or colonists die, or the colony never founds. Check the PRECONDITION
  line before reading any ratio.

## ★ AMENDMENT, written BEFORE my arm's numbers exist (see timestamps)

Ben's live `play-server.log` — server started 19:30, i.e. the binary BEFORE the
bands were built at 20:15 — already reads **rest=208, hunger=66**. Rest is
beating hunger 3.2:1 WITHOUT the bands.

So my baseline of 118:26 is STALE. The schedule work (8/8/8 + the Sleep-block
rule + collapse-from-exhaustion) already flipped the ratio, and my pre-registered
PASS threshold ("materially below 4.5:1") would be satisfied by a binary that
contains none of the change I am testing. Had I not looked, I would have run my
arm, seen rest ahead of hunger, and credited the bands for the schedule's work.

**Revised disposition rule:** this run can no longer establish that the bands
did anything. The ratio is now a REGRESSION check, not a proof:

- **PASS (as a non-regression)**: sleepers >= 7/8, ratio does not swing back
  toward hunger, `ate` does not collapse.
- **NOT ESTABLISHED, and I must say so**: any claim that the bands improved
  the ratio. Attribution needs a same-seed A/B with only the comparator
  reverted, which I am not running here.

Comparability caveats against Ben's log, stated above the difference:
his is a LIVE session (client attached, real-time pacing, player walking
around); mine is headless and uncapped. Different population dynamics,
different span, n=1 each. The two are NOT a matched pair and I will not
subtract them.

## What this run CANNOT test

- The client -> server start_site link (needs a real character screen).
- Whether the *town looks like a town* — this is a counter run, not a looking
  sweep. Numbers can be green while the place looks insane.
- Attribution between bands and the schedule rule (both are in this arm).
  A same-seed A/B with only the comparator reverted would be needed, and I am
  not running it here.
- Run-to-run variance is 2-3x on colony event counts. n=1 here, so only a LARGE
  move is readable; a small one is noise. Stuck/preempt counts are the stable
  family, which is why the verdict keys on them.

## Span

Target tick >= 45,000 (world starts hour 9; Sleep block ~31,800) so a full
night is inside the window.
