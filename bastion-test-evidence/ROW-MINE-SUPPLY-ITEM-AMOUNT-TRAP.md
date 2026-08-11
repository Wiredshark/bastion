# ROW: Mine-supply generator reads `.item().amount()`, against that
# accessor's own documented warning — currently harmless, named so it can
# page someone the moment it stops being harmless

**Filed per Opus's ask on the ITEM8-CRASH-FINDING.md fix review** (v3
prereg amendment): a scoped-out failure mode recorded in a document, not a
ledger, is exactly the shape that cost item 8's first run — "a doc comment
cannot page anyone" applies to prereg delta tables too, not just method
docs.

## THE ACCESSOR AND ITS WARNING

`PickupItem::item()` (`common/src/comp/inventory/item/mod.rs:1694`):

    /// Get a reference to the last item in this stack
    ///
    /// The amount of this item should *not* be used.
    pub fn item(&self) -> &Item { ... }

**Its own doc says not to read `.amount()` off the return value.** The
correct accessor for a stack's true total is `PickupItem::amount()` (sums
every entry), which is what item 8's own food-stock sampler and the
reservation site's candidate search both correctly use.

## THE VIOLATING SITE

`bastion-server/src/bastion_jobs.rs:9119`, the `GeneratorKind::Mine` supply
computation:

    for pickup in (&pickup_items).join() {
        if pickup.item().item_definition_id().itemdef_id() == Some(MINE_DROP_ITEM) {
            supply += pickup.item().amount() as u64;
        }
    }

`.item()` returns the LAST entry; `.amount()` on it is that ONE entry's
count, not the `PickupItem`'s true total. For a genuinely single-entry
stack (the only shape stone piles have ever had) this is harmless by
coincidence — the last entry IS the whole stack. It stops being harmless
the moment a stone pile is ever multi-entry.

## WHY IT IS CURRENTLY HARMLESS

`split_off_one` (the only method in this codebase that has ever created a
multi-entry stackable `PickupItem` — see ITEM8-CRASH-FINDING.md) is called
from exactly one site (`bastion_jobs.rs`'s `EatFrom` reservation/completion
path), filtered to `FOOD_DEFS` items only. **`MINE_DROP_ITEM` (stone) is
never passed to `split_off_one`, so a stone `PickupItem` has never had more
than one entry.** `.item().amount()` and `.amount()` agree by construction,
today.

## THE CONDITION THAT WOULD MAKE IT BITE

Any future per-unit split mechanism applied to `MINE_DROP_ITEM` (or any
other item class this supply computation, or a similar one, reads via
`.item().amount()`) would silently UNDERCOUNT supply the moment a pile
becomes multi-entry — the generator would see only the last entry's amount,
not the true remaining total, and could over-dig believing supply is lower
than it is. This is the exact undercounting shape `split_off_one`'s own
pre-fix doc comment warned about for `try_merge`, one accessor over: not a
crash, a silent wrong number feeding a planning decision.

**No fix, no investigation — filed so it can page someone if that
condition is ever introduced**, per Opus's instruction: same law that
justified `split_off_one`'s own `debug_assert!` post-condition, applied to
a site outside this fix's own blast radius rather than left as a footnote
in a delta table nobody re-reads.
