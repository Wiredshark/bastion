# RESULTS — the colonists build the worldgen house (G1c): the request fires, the site resolves, and the index is never unshared

Read 2026-09-02 18:08 on arm b1, pair 3249b9116e (G1c), booted 18:08
with the fixture lever BASTION_FORCE_PLOT_REQUEST=1.

| witness                          | first minute |
|----------------------------------|-------------:|
| COLONY SITE RESOLVED             | 1 (site 1, anchor (7722, 6320, 180), footprint 147,168) |
| HOUSE REQUESTED FROM WORLDGEN    | 1 (day 0)    |
| PLOT LAYOUT REFUSED              | 272, every one `IndexShared { strong_count: 2 }` |
| PLOT LAID OUT / PLAN QUEUED / BUILT | 0 / 0 / 0 |
| panics                           | 0            |

The G1b pre-registration named this branch: "if IndexShared fires every
tick on a live server, the design's premise (quiet ticks exist) is
false." It is false, and for a reason the unit tests could not see:
the Server struct keeps its own `IndexOwned` (server/src/lib.rs:283)
and inserts a CLONE into the ECS (line 1034), so the index Arc has two
permanent owners and `Arc::get_mut` can never succeed. The chunk
generator's transient clones were the assumed sharer; the struct field
is the real one.

## The OWN boot (pair 21ab563470, the ECS resource as sole owner; b1, booted 19:04, lever on)

| witness                       | value |
|-------------------------------|-------|
| WORLD INDEX OWNERSHIP         | strong_count=1 at boot |
| COLONY SITE RESOLVED          | site 1, anchor (7722, 6320, 180) |
| HOUSE REQUESTED FROM WORLDGEN | day 0 |
| PLOT LAYOUT REFUSED           | 0     |
| PLOT LAID OUT                 | plot 96, aabr (7618..7642, 6504..6516), door (7633, 6507, 180), 2 beds |
| PLOT PLAN QUEUED              | plan 81: blocks 10,549, cells 1,909, skipped underground 8,017, skipped unchanged 623, no stone bill |
| day 1: HAUL LANE CEILING      | neediest = Build; one demoted hauler returned to the Build trade |
| day 1: Build claims / cells placed | 12 / 12 (remaining 1,897) |
| day 1: stone_owed             | 1,905 (the plan's cells counted as stone demand: G1c-b) |
| panics                        | 0     |

The first worldgen house laid out on a live server, and the colonists
began placing it. Two gaps named by the same read: the mine generator
counted the house's cells as stone owed (G1c-b, queued), and the town
staffs its build with whoever already held the trade — one colonist,
12 cells a day, 158 days for the house (G1c-c: the town drafts
builders from its biggest trades while a plot plan is open; in an
isolated build). `BUILD PROGRESS placed=8652` counts the skipped
foundation as placed; corrected in G1c-c to queued minus remaining.

## The G1c-c boot (pair c51f78b672, the builder draft; b1, booted 20:41, lever on; read at +10 min and day 1)

Pins falsified at the commit before the read: the plan gate deleted
turned `builders_wanted` red; haulers allowed turned the trade-ranking
pin red; the tree restored clean.

| witness                 | +10 min                        | day 1 (00:00)                       |
|-------------------------|--------------------------------|-------------------------------------|
| COLONY SITE RESOLVED    | site 1, anchor (7722, 6320)    | --                                  |
| PLOT LAID OUT / QUEUED  | plan 81, the same footprint from the same seed (determinism) | -- |
| BUILDERS DRAFTED        | -- (drafts at the day line)    | 0                                   |
| BUILD PROGRESS          | --                             | queued 1,909 placed 31 remaining 1,878 builders 0 |
| phantom job retired (Build) | --                         | 28,140 over 10 distinct cells, all z 181 (2,814 each) |
| HOUSEHOLDS              | 58 / 48 occupied               | 58 / 49 occupied                    |
| panics                  | 0                              | 0                                   |

Two findings, one of them the cause of the slow build all along:

1. **The generator and the phantom check disagree on a plot cell.** The
   plan drain mints a cell while it does not match its worldgen target
   (`plot_cell_is_done`); the task-#57 phantom check retires a Build job
   whose cell is FILLED (`job_wanted(Build) = !is_filled`). The plan's
   first ten cells are solid ground under a floor course, so they were
   minted and retired 2,814 times each, ate the whole build budget every
   pass, and the other 1,868 cells never reached the board. Thirty-one
   cells a day was never a staffing number. G1c-d (queued): one pure
   predicate, a plot cell is wanted until it matches its target, at both
   retirement sites; witness PLOT CELL KEPT.
2. **No draft fired**, because `builders_now` counts colonists NAMED
   Build and the hauler cap had just returned four to that trade. With
   the churn gone those named builders have 1,868 open cells; whether a
   name is a worker at the plot is the next read, not a fix yet.

## The G1c-d boot (pair fce20cf9b9, the plot-target retirement rule; b1, booted 21:50 on 09-02, lever on; read at +10 min, day 1, day 2, and again at day 68 after 42 unattended hours)

The pin was falsified at the commit (the old kind rule planted: red);
the first falsification run had printed a verdict without planting and
was discarded.

| witness                     | day 1     | day 2     | day 68 |
|-----------------------------|----------:|----------:|-------:|
| phantom job retired (Build) | 0         | 0         | 0 in the last five days |
| PLOT CELL KEPT (count)      | 16,384    | 65,536    | --     |
| BUILD PROGRESS placed / remaining | 69 / 1,840 | 91 / 1,818 | 291 / 1,618 |
| builders (drafted)          | 0         | 6         | 8      |
| HOUSEHOLDS occupied         | 49        | 50        | 31 (the famine, below) |
| panics                      | 0         | 0         | 0      |

The churn is gone: 28,140 retirements a day became none, and the
plan's cells stay on the board. The build is still a crawl: 91 cells
by day 2 and 291 by day 68, with six to eight drafted builders whose
lane reads 30-80 works a day and 150-230 blocks of travel per claim.
The rate is not the staffing and not the retirement; the next read is
WHICH cells get placed (their z) and what the builders do at the ones
that do not -- the working hypothesis is reachability: a house's walls
and roof are above a walker's reach until the course below them
stands, and the plan mints its cells in the order the layout emitted
them, not from the ground up (G1c-e).

The placement curve over the 68 days (cells placed at each day line):
69, 91, 126, 130, 151, 158, 160, 169 by day 8, then 169 flat to day
15, 195 at day 16, 278 by day 29, 284 from day 40 to 57, 291 at day 68.
The fed phase (days 1-5, six to eight builders) placed about 25 cells
a day; the starving phase placed almost none.

The release census of that run's last days is the confounder: of the
work-class releases, 2,652 were "a personal entry releases the held
work job" (hunger and sleep taking the worker off the job), 1,450 the
job removed under the worker, and only 561 completions -- a starving
town drops its work to look for food that is not there. The house's
rate cannot be read from a starving town; the reachability read is
re-run on the F2 pair's first two days.

The same 42-hour run found the town starving from day 5 on both arms
(the year census: a 160-day year, a half-year crop cycle, a founding
stock of one day, fields whose yield covers a twentieth of the
town's 3.2 units per head per day). That is its own row (F2) and it
outranks the house.

## Disposition

The G1b premise is now TRUE on a live server (owner count 1) and G1c's
wiring carried a request through to a queued plan and the first placed
cells. Not yet evidenced: PLOT BUILT and the house registered (needs
the builders); the render (Ben's world).

The earlier read on the G1c pair stands as the record of the premise
failing: G1c's wiring PASSED its own instruments up to the seam (site
resolved, request raised, refusal named with its count) and could not
proceed past it. Not a colony defect: an ownership fact of the vanilla server. Fix
in flight (an isolated builder): the ECS resource becomes the sole
owner and the server's dozen uses fetch it, with a boot witness that
prints the strong count (must read 1). Until then no worldgen house
can be laid out on a live server; the G1c boot on b1 is kept for its
other reads. The premise is re-tested by the same boot the moment the
fix lands.

## The house day on b3 (G1c-e-i instrument pair d1788fa5d0, 17:24-17:54)

The instrument boot: house lever forced at day 0, `BUILD CELL PLACED`
per colonist and z, `build/*` release classes.

| read | placed | remaining | builders | drafted | swept (Build) | BUILDERS DRAFTED lines |
|---|---:|---:|---:|---:|---:|---:|
| +10 min (day 0, hour 13) | 0 | 1,909 | -- | -- | 0 | 0 |
| day 1 (hour 0) | 0 | 1,909 | 0 | 0 | 98 | 0 |
| day 2 (hour 1) | 57 (62 witnessed) | 1,852 | 2 | 0 | 187 | 0 |

Day 1 by builder: colonist 29 placed 44 cells, colonist 84 placed 18;
z histogram 180:27, 181:21, 182:14 (all Rock: the foundation courses).
The busiest builder's day-1 releases: 44 Completed, 3 + 2 + 2 Other
(a `moved` re-aim, a break over, a re-target), 1 recreate. A cell costs
a builder roughly 1,200 ticks (40 s) end to end -- claim, walk, place,
release, claim the next -- so the house is travel-priced per cell and
the crew size is the lever. (Instrument note: the G1c-e-i release
class reads "gone" for completions because the job is removed before
the class is computed; `reason=Completed` is the truth and the class
should be derived from the reason first.)

On day 0 no colonist considered a plot cell (no claim of a
`Designated(Build)` job; nobody was named Build yet). The draft's own
inputs, read off the same log and the code:

1. **Two builders by accident.** At the day-1 morning the professions
   tally named colonists 29 and 84 `profession=Build weight=0`: the
   argmax names a colonist with zero hours in EVERY lane by the first
   lane in sort order, which is Build. Colonist 29's day 0: Recreate x3,
   DepositRun x2, RestAt, Guard, EatFrom. Colonist 84's: 26 hauls.
   Named, they got `in_lane(Build)` priorities and placed 62 cells on
   day 1 (44 + 18, all Rock at z 180-181: about 31 cells per
   builder-day, so a month per house). The draft counted the two names
   (`builders_now = 2`) and added nobody.
2. **The backlog it sized on was the open jobs, not the house.** The
   Build job generator mints at most two jobs per colonist per pass
   (`BUILD_GEN_JOBS_PER_COLONIST`), and the unclaimed sweep reaps them
   at 930 s, so `open_build_cells` never exceeded ~100; `builders_wanted`
   saw 100 / 150 -> 1 where its own constant's doc assumes the 1,909
   cells -> 12 -> cap 6. `wanted = 1 <= builders_now = 2`: no draft, on
   every morning of every boot since G1c-c.

Two rows follow: G1c-e-a (a plot cell is renewable; the sweep leaves it)
and G1c-e (`counts_as_builder(named, build_hours, drafted)`; the backlog
is `plot_blocks.len()`; a daily `BUILD DRAFT SIZED` witness). The day-2
read of this boot is kept as the control for the next b3 boot on the
G1c-e pair.

### G1c-e-a: a plot cell is renewable (701bd155c6)

Defect: the unclaimed-designation sweep (ITEM8-V4 route 3 backstop)
reaped `Designated(Build)` plot cells after 930 s without a claimant
(b1 on the F2 pair: 98 in a day; b3 on the instrument pair: 98 by day 1,
187 by day 2), and the plan drain re-minted them next pass: the same
churn the sweep already exempts farm jobs from, on the third retirement
path for the same cells after the phantom check and the moot check
(G1c-d).

Mechanism: `designation_is_renewable(kind, is_plot_cell)` = a Farm job
or a cell in `plot_blocks`; the sweep's `is_renewable` argument reads
it; a witness `PLOT CELL NOT SWEPT` counts what it spares. A player's
Build or Mine mark still reaps on the old threshold.

Falsified at the commit: the plot arm dropped turned
`a_plot_cell_is_renewable_like_a_farm_cell` red at
`bastion_jobs.rs:52203`.

Live evidence pending: `unclaimed designation swept ... Designated(Build)`
at 0 with `PLOT CELL NOT SWEPT` > 0 on the next b3 boot.

### G1c-e: the draft counts builders and sizes on the plan (d054ec6c40)

Mechanism: `counts_as_builder(named_build, build_hours, drafted)` =
drafted, or named Build with hours held; the backlog passed to
`builders_wanted` is `plot_blocks.len()` (the plan's remaining cells)
plus the stray Build marks outside any plot; a daily `BUILD DRAFT SIZED`
witness prints wanted, builders_now, named_build, plan_cells.

Falsified at the commit: counting the name alone (`drafted ||
named_build`) turned `a_build_name_with_no_build_hours_is_not_a_builder`
red at `bastion_plot_build.rs:866`.

Known edge, accepted: on the morning the argmax names a zero-hour
colonist Build, that name carries priorities but no hours, so the draft
may staff wanted plus the accidental names until the plan closes.

Live evidence: the b3 house-day boot on this pair (below).

#### The house day on the G1c-e pair (b3, d054ec6c40, 19:53-)

Day 0: the plan queued at hour 11; 5 cells placed by colonist 142 before
the first morning; `unclaimed designation swept ... Designated(Build)` 0
(the G1c-e-a sweep fix holds).

Day-1 morning: `BUILD DRAFT SIZED wanted=6 builders_now=1 named_build=5
plan_cells=1904 open_build_cells=1909 roster_now=49` and `BUILDERS
DRAFTED ... drafted=["103:Cook", "120:Cook", "134:Cook", "139:Cook",
"906:Cook"]`. The draft fired for the first time since G1c-c: wanted 6
on the plan's cells (was 1 on the open jobs), builders_now counted the
one colonist with build hours and not the four zero-hour names.

Two things to read at day 2: the placement rate with a crew (the
control is 62 cells by two accidental builders), and how many distinct
builders place cells -- the four zero-hour names carry `in_lane(Build)`
priorities from the argmax and may build beside the drafted five (the
known edge: up to 10 on a cap of 6). The draft took five cooks; the
kitchen cap admits two at a time, so the four cooks left are enough
(read `cooked_today` at day 2 against day 1's 90 on b1 to be sure).

| read | placed (day) | placed (total) | remaining | builders | z | releases (day 1) |
|---|---:|---:|---:|---:|---|---|
| day 2 (hour 0) | 96 | 118 | 1,813 | 10 | 180:27 181:32 182:49 183:10 | 118 Completed, 18 TimedOut, 60 Other |

Day 1 placed 113 cells (the control day placed 62): the crew works, and
the house is now a fifteen-day house at this rate. Per builder the
rate FELL, 31 to about 12 cells a builder-day, with ten builders on a
25 x 13 plot (the drafted five, the four zero-hour names, and 142) and
18 timed-out releases -- crowding on the scaffold is the likely price
of the over-count, which makes the zero-hour naming the next row
rather than a footnote: the argmax's switch rule (`c * 2 >= inc_c *
3`) also renames any colonist with an idle day to Build (0 >= 0), so
the over-count recurs every morning. Sweeps: 3 Build cells at the
plot's south edge, no longer in the plan's block map; negligible.

#### P-zero-hours, registered before its binary

`argmax_verdict(top_count, incumbent)`: a zero top keeps the incumbent;
a zero top with no incumbent takes `scarcest_lane(lane_pop)`; a
positive top switches as before. Bars for the next b3 house-day boot on
its pair: `BUILD DRAFT SIZED named_build == builders_now + drafted`
(no zero-hour names), the crew reads 6 (7 at most), day-1 placement
near 180 with the per-builder rate back toward 30. Falsified if
named_build exceeds builders_now by three or more, or the crew reads
above seven. Planted defect for the pin: `(0, Some(_)) => Switch`.

P-zero-hours landed as 2e0b063b53 (01:57, staged 02:12). Falsified at
the commit: the idle-day switch planted turned
`an_idle_day_names_nobody_and_a_newcomer_takes_the_scarcest_lane` red
at `bastion_jobs.rs:53003` (02:16); the tree restored clean. The
house-day read on b3 (`wait-pz-b3-b.sh`) boots after the restored-boot
day-1 read and before the eighth restart test; its section follows.

### The house day on b3 (pz pair 2e0b063b53, boot 02:25, day 1 at 02:57)

| bar | registered | read | verdict |
|---|---|---|---|
| the morning draft's names | named_build within the crew (no zero-hour names beyond it) | `BUILD DRAFT SIZED wanted=6 builders_now=2 named_build=5` at the day-1 boundary | PASS (G1c-e grew four accidental builders a morning; here the names stay under the crew) |
| the crew | 6 (7 at most) | wanted 6, named 5 | PASS |
| day-1 placement | near 180 | 26 (`build/Completed` 26; BUILD PROGRESS placed=21 remaining=1888 at the day line; z 180-182) | FAIL, confounded |
| panics | 0 | 0 | -- |

The placement miss is this row's after all, read right at 04:20: since
P-zero-hours a fresh town has NO builder until its first midnight
draft. b1 on the E1-f pair: 6 cells on day 0; `BUILD DRAFT SIZED` at
the midnight boundary 07:35 named a crew of five; 96 cells went up
between 07:49 and 07:58 -- game hours 11 to 18, the work day -- and
none until the next midnight draft at 08:18, with open Build jobs a
constant 98 all day. The eighth restart test's first boot placed 0 in
the 40 minutes after its plan (the seventh's, before P-zero-hours: 25
in six -- the zero-hour argmax had named accidental builders at boot,
the very thing P-zero-hours removed). The sixth test's restored boot
placed 0 all day for the same reason. The day-1 bar of ~180 was set
before HUMAN HOURS; a crew of six between hours 8 and 16 places ~100.

P-zero-hours-b (`fix-pzb.py`, queued behind R3-b): when a plan is
queued and no colonist counts as a builder, the day's naming block
runs on the next tick (`board.profession_day` reset; `draft_at_plan`,
pinned; witness `BUILD DRAFT CALLED AT THE PLAN`). Bars on b1 and the
ninth restart test: the draft called within a second of the plan,
`BUILD DRAFT SIZED wanted=6` the same second, Build arrivals above 10
and placed cells above 15 within ten minutes of the plan, on a fresh
boot and on a restored one; day-1 placement re-registered at ~100.
Falsified if the draft is called and nothing is placed in ten minutes,
or the second pass renames settled colonists.

P-zero-hours' own claim -- an idle day names nobody, a newcomer takes
the scarcest lane -- holds on this boot; its cost was the day-0 crew.

### P-zero-hours-b landed (5382e809b0, 06:21)

Check clean, pin `a_plan_with_no_builder_calls_the_draft` green (1
passed), staged 06:21. The chain first refused at 05:58 on a stale
pin anchor (it had been written to sit after T1's pin, and T1 now
follows it in the queue); re-anchored on the R3-b pin, dry-run on
HEAD, relaunched 06:00, tree untouched by the refusal. Falsified at
the commit: the draft never called turned the pin red, tree restored
clean at 06:25. The live read is the
ninth restart test on b3 (both boots: BUILD DRAFT CALLED AT THE PLAN
within a second of PLOT PLAN QUEUED, cells placed within ten minutes)
and b1 fresh when the E2 pair reaches it.

### The ninth test's first boot, and P-zero-hours-c (registered 06:32)

Boot 1 on b3 (06:22): PLOT PLAN QUEUED and BUILD DRAFT CALLED AT THE
PLAN in the same second (plan=81 cells=1909 builders_now=0
drafts_at_plan=1) -- the call half of P-zero-hours-b PASSED. Then no
BUILD DRAFT SIZED line at all, no HAUL LANE CEILING line; the first
cell at +2 min, and by 06:30 (+8 min) 24 cells placed with 28 Build
arrivals -- the registered outcome bar (arrivals above 10, cells above
15 within ten minutes) PASSED, but by the argmax naming zero-hour
colonists to the scarcest lane (Build, with a plan open), not by the
sized draft. Larder at the stop: STORE SNAPSHOT SAVED cells=358
units=12,755 (7 saves); TERRAIN PERSISTED 8,050 blocks; graceful stop
at 06:31, boot 2 kept at 06:31. The daily block did run (it runs
every 300 ticks when today != profession_day); it sizes the ceiling,
the reserve and the draft from `board.professions.len()`, near empty
at a fresh boot's first pass, so `builders_wanted`'s
`roster / COLONISTS_PER_BUILDER` reads 0 and the draft wants nobody --
and the SIZED witness is gated on wanted or builders_now being
positive, so the pass that wanted nobody left no line. The size half
FAILED for a reason the instrument could not show.

P-zero-hours-c: `draft_roster(named, alive)` = max(names given,
colonists alive) is the roster for all three; the draft pass prints
BUILD DRAFT PASS unconditionally (roster, named, plot_plan_open,
open_build_cells, wanted, builders_now, drafts_at_plan). Pin
`the_draft_counts_heads_not_names`; planted defect: names only, red.
Keyed after W10-a-b, before E2; read on b1 fresh on the E2 pair
(`wait-pzc-b1.sh`): BUILD DRAFT PASS within ten seconds of the call
with roster >= 40 and wanted = 6, SIZED naming a crew, Build arrivals
above 10 and cells placed above 15 within ten minutes of the plan.
Falsified if roster >= 40 and wanted 0 (plot_plan_open names it), or
wanted 6 and nothing placed (a route row).

Landed 2752edc4d6 at 07:08: check clean, pin green (1 passed), staged
07:08, shipped to lab-bin 07:08. Falsified at the commit: names only
turned the pin red (0 passed, 1 failed), tree restored clean at
07:11.

Live on b1 (E2 pair 3ce9276f42, boot 07:32, plan at 07:33): BUILD
DRAFT PASS in the same second as the call with roster=48 named=0
plot_plan_open=true open_build_cells=1909 wanted=6 builders_now=0 --
the size half PASSED; then BUILD DRAFT SIZED wanted=6 builders_now=0
named_build=0, and in the eleven minutes that followed one Build
arrival against Farm 200, Mine 139, Chop 18, one cell placed at
+10 min. **The outcome bar FAILED**: the draft wanted six and named
nobody, because its candidate pool is the names map (empty at the
first pass), so heads fixed the size and not the crew. The same arm's
earlier fresh boots read 0-6 cells on day 0; the ninth test's 24 on
b3 came from a different arm (forced plot request) and are not the
control for this.

### P-zero-hours-d, registered 07:52 (keyed on the W10-a-c stage, before the binary; T1 re-keyed behind it)

The day line's naming loop iterates `tops`, built from lane_counts
entries, so a founder at the first pass (twenty seconds after boot,
before any claim) is not in the loop at all and the newcomer arm
cannot reach it. Now every live head enters `tops` at zero hours when
it has no entry and takes the scarcest lane by the existing verdict;
`newcomer_lanes(lane_pop, n)` recounts the scarcest lane after each
naming so the founders spread by scarcity instead of all taking one
lane; the loop runs in uid order. Pin
`newcomers_spread_across_the_scarcest_lanes` (three newcomers over
three empty lanes spread one each, a fourth returns to the most open
work; every naming counted); the P-zero-hours pin
(`an_idle_day_names_nobody_and_a_newcomer_takes_the_scarcest_lane`)
must stay green in the same chain. Planted defect: the recount
removed, all take one lane, red.

Prediction (b3 fresh, plot request forced, plan at +1 min): NAMED BY
NEED ~48 at the first pass with no lane above 12 practitioners;
BUILD DRAFT SIZED named_build >= 5 in the same pass; Build arrivals
above 10 and cells placed above 15 within ten minutes of the plan;
HAUL LANE CEILING demotes nobody at the first pass. Falsified if one
lane takes more than half the roster (the recount is not reaching the
verdict) or if named_build >= 5 and nothing is placed in ten minutes
(a route row).
