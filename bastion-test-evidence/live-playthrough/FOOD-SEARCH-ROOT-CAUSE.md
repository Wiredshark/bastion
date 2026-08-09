# Food-search finding from run-51 — root cause (2026-08-09)

Two cheap code reads (Fable's suggested order, producer before hypothesis)
fully explain run-51's `EatFrom` result (1 job created in 40 minutes, 0
completions, 21,176 `no_food_found`) without needing another live run.

## Read 1: the eat-search's actual scanned population

`bastion-server/src/bastion_jobs.rs` (~line 10204): joins
`(&pickup_items, &positions, &uids)`, filters to items whose
`item_definition_id()` is in `FOOD_DEFS` (`["common.items.food.mushroom",
FARM_WHEAT_ITEM]` — matches the script's `give_item
common.items.food.mushroom`) and not already reserved, then
`min_by_key`s by distance. **No radius cutoff at all** — any matching,
unreserved `PickupItem` anywhere on the map is a valid candidate. The
search itself is correct and unrestricted; it is not scanning the wrong
population or the wrong item definition.

## Read 2: what the script's provisioning actually creates

`cmd dropall` -> `server/src/cmd.rs::handle_drop_all` -> `server.state
.create_item_drop(pos, ori, vel, item, loot_owner: None, persistent:
false)` — **`persistent` is hardcoded `false`**. `server/src
/state_ext.rs` (~line 385-389): a non-persistent drop gets
`Object::DeleteAfter { timeout: Duration::from_secs(300) }` —
**every `/dropall` item despawns after exactly 5 minutes.** The
persistent/no-despawn path (`BastionPile`, "a colonist-produced player
resource") is a separate, bastion-specific mechanism `dropall` does not
use.

## The mismatch

script-10 drops food twice: once at t=0 (before spawn, before any
designations), once at checkpoint 3 (~15min mark). Each drop is only
discoverable for 5 minutes. But colonists' hunger crossings are staggered
across roughly a 15-30+ minute window as each individual colonist's own
decay timer independently crosses interrupt (not scoped by this run's own
`hunger below interrupt` timing since it fires per-successful-find, but
`no_food_found`'s 21,176 count over ~25 minutes of active search
confirms most colonists were searching well outside either drop's
5-minute window). **The one success (job 713, uid 54, ~18:37:12) lands
almost exactly at the checkpoint-3 resupply's timing** — the one
colonist whose search happened to land inside that drop's narrow window.

**This is a scoping fact about the script's provisioning method, not a
bug in the eat-search or the need-arbitration fix.** Matches Fable's
predicted shape exactly, though the mismatch is temporal (a 5-minute
availability window against a 15-30min staggered demand window) rather
than a population/unit mismatch. `#51`'s own bed-arrival finding is
unaffected — `RestAt`/beds never touch this despawn path at all.

## If this row gets picked up

Not a fix to bastion_jobs.rs's search logic (verified correct). Options,
not evaluated further here: script-side (drop food as a `BastionPile`
persistent resource instead of vanilla `dropall`, or resupply on a
<5min cadence matching the despawn timer), or a design question about
whether milestone/acceptance scripts should provision food via a
mechanism that doesn't silently expire. Filed as a scoping finding for
whoever owns the script's next revision, not an engine defect.
