# PREREG — only a posted militia member is exempt from the shelter (D1c)

Registered 2026-09-02 15:20, before the binary exists.

## Defect (D1b read, arm b1 on 9b42974b7e, first cry at 18:00)

| sheltered | already home | bedless | out of earshot | posted (muster lines) | unaccounted |
|----------:|-------------:|--------:|---------------:|----------------------:|------------:|
| 24        | 2            | 0       | 7              | 2                     | 16          |

Civilians indoors 26 of 42 in earshot = 62% (bar 80%). Read in the
producer: the muster's candidate set is "Guard priority >= 4" (16 on
this arm) and its squad cap is threat-scaled (2); the shelter loop
exempted by the same priority predicate, so 14 were neither posted nor
sheltered.

## Mechanism

`alarm_exempts_from_shelter(&JobKind)`: exempt only while on a Guard job
(posted this cry or a standing post). `militia_posted` on the ALARM
RAISED line. Pin: a posted guard job is exempt; a shelter job or a rest
is not; planted defect `true` for every kind.

## Instrument validation

The line's counters must sum to the roster on the first cry:
sheltered + already_home + militia_posted + skipped_bedless +
out_of_earshot == roster (49). If they do not, a branch is still
uncounted and the bars below are not read.

## Pre-registered outcomes (arm b1's first cry on the D1c pair)

| measure                                                        | bar    |
|----------------------------------------------------------------|--------|
| militia_posted                                                 | <= 5   |
| sheltered + already_home                                       | >= 34  |
| civilians indoors = (sheltered + already_home) / (roster - militia_posted - out_of_earshot) | >= 80% |
| running returns to 0 within 60 s (census)                      | yes    |
| downed                                                         | 0      |

FAIL branches: sheltered does not rise -> the 14 have another skip
(re-read the loop); running stays > 0 -> the extra shelter jobs wedge
(the movement rows, not this one); musters fall -> the exemption
removed a fighter (it must not: the muster runs first and posts before
the shelter loop reads the board).

NOT evidenced live yet; the raid on Ben's world is his acceptance test.
