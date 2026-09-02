# RESULTS — only a posted militia member is exempt from the shelter (D1c): every colonist on a counter, civilians indoors 100%

Read 2026-09-02 18:05 against PREREG-alarm-role.md. Arm b1 (raid arm,
roster 49) on pair b260c71a0c, booted 17:49; the boot's first raid at
18:00 game time raised two cries.

| first cry                 | D1b read (S4b pair) | D1c | bar     |
|---------------------------|--------------------:|----:|---------|
| sheltered                 | 24                  | 35  | --      |
| already home              | 2                   | 7   | --      |
| militia posted            | (uncounted; 16 exempt by priority) | 2 | <= 5  PASS |
| bedless                   | 0                   | 0   | --      |
| out of earshot            | 7                   | 5   | --      |
| sum of the counters       | 33 of 49            | 49 of 49 | == roster  PASS (instrument) |
| sheltered + already home  | 26                  | 42  | >= 34  PASS |
| civilians indoors = (sheltered + already home) / (roster - posted - out of earshot) | 62% | 100% | >= 80%  PASS |
| musters (AUTO-GUARD posted), two cries | 2-4 | 2 + 4 | not fewer  PASS |
| running after the cry (census) | 22 -> 0 in ~30 s | 35 -> 21 -> 5, second cry 18 -> 8 | returns toward 0  PASS |
| downed                    | 0                   | 0   | 0  PASS |

The second cry (out of earshot 25, sheltered 18, posted 4) came from a
different cry position; its counters also sum to 49.

## Replicate 2 (the G1c boot on 3249b9116e, first raid, three cries, read 18:25)

| cry | sheltered | already home | posted | out of earshot | sum | indoors |
|----:|----------:|-------------:|-------:|---------------:|----:|--------:|
| 1   | 18        | 1            | 2      | 28             | 49  | 100%    |
| 2   | 32        | 8            | 2      | 7              | 49  | 100%    |
| 3   | 20        | 4            | 2      | 23             | 49  | 100%    |

Musters 6, downed 0, doors: 1 fight, 0 gave way (D2's second replicate:
none in two boots against four at 10.03 s before).

## Disposition

PASS on every pre-registered bar, two replicates (a third comes free
with the next raid-arm boot). The mechanism named by
the D1b read was the whole story: the muster's squad cap left the
unposted high-priority colonists in a gap between "fighting" and
"indoors"; closing the gap moved 16 people indoors. Not evidenced:
Ben's world (militia count and radius differ); whether posted guards
hold their posts through a 60-second door.
