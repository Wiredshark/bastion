# Item 6 (persistent provisioning + #96/#97 protection): acceptance result

**Verdict: PASS**, on a paired before/after comparison rather than a refusal
counter -- the counter turned out to be structurally unobservable by the
architecture that makes it unnecessary (see "instrument gap" below).
Commit `cbfb8ae977` (`bastion/wip-batch-verify`).

## The acceptance run

`bastion-test-evidence/live-playthrough/script-12-item6-provisioning.txt`,
second attempt (first attempt was void -- see "what didn't work" below):
6-colonist colony, zero stockpile regions designated (DECISIONS #101's
discriminating case, in its strongest form -- no stockpile exists anywhere in
this run, so protection can only be resting on `BastionPile` membership, never
a region), one persistent pile provisioned via `/dropall true` at the driver's
own position, 25 minutes of observation across 5 checkpoints.

## Positive control -- the pile existed and the path was live

    bastion: ate — hunger restored   job=433,434,436,437,438,439,440,442,443

Nine EatFrom completions, `item=91` referenced at a **constant**
`item_pos=Vec3{15222,16024,420}` for the entire 25-minute run -- it never
respawned, which is only possible for a `persistent: true` drop (a loose one
has a 300s `DeleteAfter` and would have needed re-dropping at least four times
across a 25-minute run to still be there). This satisfies the positive control
registered before the data: a colonist pickup proves the pile existed and the
pickup path was live, so an absence of ambient pickups elsewhere in the same
run is a real result, not an artifact of nothing happening.

## The comparison that actually carries the row

    row89-green-v2 (pre-#96/#97, `/dropall` loose):
      13:14:06.288Z  dropall (40 mushroom)
      13:14:42.729Z  Voonoo (ambient world-traveler, picker_colonist=None,
                      picker_is_rtsim=true) takes the full 40-unit stack
                      -> SURVIVED 36.7 SECONDS

    script-12 (post-#96/#97, `/dropall true`, field-placed, zero stockpiles):
      persistent pile survives the full 25-minute run, colonists eating from
      it throughout, never taken by an ambient picker
      -> SURVIVED >= 1500 SECONDS (run ended, not exhausted)

Same world class, same drop mechanism, same kind of ambient population. Before
#96/#97: an ambient NPC took a full drop inside 37 seconds. After: an
equivalent, harder-to-protect (persistent, no stockpile) pile survived the
entire observed run while colonists ate from it repeatedly. (Note: the "~37s"
figure is measured directly against the driver's own `/dropall` timestamp,
converted to UTC and checked against the server log's own pickup line --
an earlier verbal citation of "~2 minutes" for this same specimen did not
survive that check and is not used here.)

## What didn't work, kept for the record

**First attempt was void, not a false pass, and would have been indistinguishable
from one:** `give_item`/`dropall` both returned `command-no-permission`. Root
cause: I granted the admin role via a separate `admin add` invocation *after*
the server had already booted; the running process loads `admins.ron` once at
startup and never re-reads it, so the on-disk grant never reached it -- exactly
the precondition every prior script's own header comment already stated
("before boot, against the same userdata"). A run with zero items ever placed
would have produced zero ambient pickups and read as a clean PASS on the
original bar. Caught before scoring, not after; fixed by granting admin before
the boot on the retry.

## Instrument gap, filed not built

**The server-side refusal counter (`"ambient-loot-disabled"` in
`inventory_manip.rs`) is structurally unobservable given the current
architecture, and this is not a bug in the counter.** The AI-side gate
(`action_nodes.rs`'s `is_valid_target`) returns `None` for every item when
`ambient_item_looting_enabled()` is false, which means an ambient rtsim NPC
never *targets* an item to pick up in the first place -- it never issues the
`InventoryManip::Pickup` intent the server-side layer counts. Layer 1 silences
layer 2 by construction: the belt is doing the job so the suspenders never
fire. Setting `BASTION_B55_TRACE_DELETES` and re-running would not have
produced a usable number -- it would have produced an expected zero
indistinguishable from "no ambient NPC came near," which is exactly the kind
of null result this acceptance's own positive-control discipline exists to
rule out.

**Follow-up, one line, not this row:** the AI-side gate should count its own
refusals when it fires (`ambient_item_looting_enabled() == false` and an item
was the candidate target) -- currently a silent `None`, so the era's most
sweeping ambient-behaviour change has no observability at all. That counter is
what makes a future thievery-gate-lift's day-one delta measurable, which was
the point of asking for counted refusals in the first place. Filed here for
whoever picks up the AI-side observability row; not built as part of item 6.

## Scoring against the packet's own criteria

| criterion | result |
|---|---|
| provision food persistent via `/dropall true` | done, `item=91`, never respawned |
| assert colonist pickups succeed | 9/9 EatFrom completions across the run |
| assert non-colonist pickups are refused | not directly observable (see instrument gap); demonstrated instead by outcome: 37s survival pre-fix vs full-run survival post-fix, same mechanism |
| #101 discriminating case (pile outside any stockpile region) | satisfied in its strongest form -- zero stockpiles exist in this run at all |
| regression check: loose `/dropall` unchanged | untouched by this row, verified at commit time (byte-for-byte default-false path) |

Two unrelated `ULTIMATE FAIL-SAFE` teleport rescues fired mid-`EatFrom`
(`uid=24`, `uid=25`) during this run -- flagged to item 4/#94's residual
fail-safe family, not investigated here.
