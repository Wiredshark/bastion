# Farm P1 diagnosis (DECISIONS #62)

Assigned question: does the live paint path register the plot where the
job generator looks (Fable's registration-divergence hypothesis), or is
the defect downstream in the generator's own predicate?

## Answer: neither. Run 1/2's zero-jobs result was my own site-pick error.

**Registration path is identical, not divergent.** Both the paint handler
(`bastion_jobs.rs:4645`, `self.farms.push((id, region))`) and the trigger
pass (`bastion_jobs.rs:9027`, `for (_, plot) in board.farms.iter()`) read
and write the exact same `board.farms: Vec<(FarmZoneId, Region)>`. Run 1/2's
own server logs already show `"farm plot registered zone=1 region=..."`
firing on the live path both times — the plot WAS registered. Fable's
hypothesis is falsified by evidence already collected before this session
even started investigating it.

**The real cause: the trigger pass requires `ground.is_filled()` at
EXACTLY `plot.min.z`, with zero tolerance**, for every column
(`bastion_jobs.rs:9028-9041`) — unlike Mine, which scans a whole 3D box and
tolerates picking an approximate z. I painted Run 1/2's farm region with
`min.z=418` (guessed from the player's own spawn z minus one), never
checking the actual ground height there.

Verified with the driver's `survey` tool (`script-07-farmcheck.txt`)
against the exact Run 1/2 plot: the real surface across that footprint is
z=415-417, never 418. Every single column's `ground.is_filled()` check at
418 was checking air one-to-three blocks above the real ground —
"no field under a hole" fired for all 49 columns, silently, every tick,
for the whole 4.5-minute run. Zero jobs was the CORRECT behavior of the
code as written, given the input I gave it.

## Confirmation: farm works correctly live once the height is right

`script-08-farmfix.txt`, same live server, same colony, corrected to the
surveyed real ground height (z=415, a genuinely flat 7x2 strip found by
the survey):

- 13 TILL jobs generated immediately on paint (`farm job created ...
  sow=false`), all 13 completed by the colony.
- 13 SOW jobs generated as tilled cells appeared (`sow=true`), and all 13
  were resolved as `phantom job retired (task #57) — target cell no
  longer matches the designation` — which for Farm means success, not
  failure: sowing places a `WheatYellow` sprite, and the SOW predicate
  (`None | Some(SpriteKind::Empty)`) no longer matches a cell that now
  holds a growing crop, so it retires exactly as designed.
- Full HARVEST/`crop MATURE` was not observed within the session's
  remaining time budget (`FARM_GROWTH_MAX=15`, `FARM_STAGE_SECS=6.0` — a
  minimum ~84s of stage-advances after sow, and no growth-stage log lines
  appeared for several minutes after I disconnected my client, which may
  mean growth-clock advancement is gated on a connected player somehow —
  not confirmed, flagging as a separate open question rather than
  asserting it).

## Verdict

**Not a live/harness divergence.** Both the registration path and (as far
as till+sow) the generator's own predicate work correctly live. Task #60's
harness-observed "farm_tilled/farm_sown always false" was NOT reproduced
by this live test — the opposite happened: till and sow both succeeded
cleanly once the plot height was correct. That leaves two live
possibilities for #60, not resolved here: it's a harness-fixture-specific
artifact (the harness's own scenario setup gets the height wrong the same
way I did, or some other fixture-only condition), or it reproduces only
under conditions this quick live test didn't hit (a longer run, different
terrain, or the maturity/harvest stage specifically). Recommend re-checking
task #60's own harness scenario for the same z-height class of mistake
before assuming a deeper generator bug.

## Real, separate finding worth its own row: Farm has a silent, zero-tolerance failure mode

Independent of #60: painting a Farm designation one block off the true
ground height produces **total silent failure** — zero jobs, zero error
message, zero visible feedback of any kind, indistinguishable from "working
correctly, nothing to do yet." Mine tolerates height error via its whole-box
scan; other designations that support `z_extent: Some(_)` get server-side
surface resolution. Farm's literal-region path has neither. A player who
misjudges their own farm's ground level by one block gets nothing, with no
way to know why — that's the UX-facing shape of this P1, even though the
underlying registration/generator code is correct.
