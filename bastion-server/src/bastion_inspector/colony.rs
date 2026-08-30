//! bastion (INSPECTOR-M2): the **Colony** section provider — THE TOWN
//! AROUND THIS COLONIST.
//!
//! ★ THE FRAME WARNING FIRST, because it is the commonest defect in this
//! subsystem. Everything derived from `JobBoard` is RUNTIME-ONLY: the board
//! is rebuilt from `JobBoard::default()` at every server start, so the
//! professions, the colony drive and the household derivation all read
//! EMPTY after a restart until their own cadences run again. The roster and
//! the stock census are ECS. The two are not commensurable and every row
//! that renders them says which it is.
//!
//! ★ WHAT THIS SECTION ADDS TO THE SHIPPED `BastionColonyInspect`. That
//! payload is nine scalars. This one:
//!
//! * gives the colony drive its REASON and its MAGNITUDE, which
//!   `colony_drive_for` returns and the call site throws away, plus how
//!   long the drive has held;
//! * names the HOUSEHOLDS and their members, which is "one colonist per
//!   house" made visible rather than asserted;
//! * counts professions over `WorkType::ALL` PLUS an unnamed bucket, which
//!   is the owner's acceptance criterion ("name someone's job from an hour
//!   of watching") expressed as a number that can fall;
//! * and makes every stock figure a BREAKDOWN over three disjoint scopes,
//!   because a single number cannot tell a broken forge from broken
//!   hauling and this codebase has already shipped that exact ambiguity.

use common::{
    bastion::WorkType,
    comp::bastion_inspect::{
        ColonyDriveVerdictV1, ColonySectionV1, HouseholdMemberV1, HouseholdRowV1, SectionIdV1,
        SectionPayloadV1, StockRowV1, StockScopeV1,
    },
    uid::Uid,
};

use super::{InspectCtx, name_of, not_measured};
use crate::bastion_jobs::{
    JobBoard, colony_drive_food_per_cap, colony_drive_for, colony_sustain_bar, derive_households,
    household_capacity,
};

/// How many distinct item definitions the stock census transmits.
///
/// A colony holds thousands of item STACKS but few distinct definitions,
/// so this bites only in an unusual world. When it does,
/// `ColonySectionV1::stock_truncated` says so and the list is the HEAVIEST
/// items rather than an alphabetical prefix — a truncation that hid the
/// biggest pile would be worse than no list.
pub const STOCK_LABEL_CAP: usize = 48;

/// ★ THE ITEM CENSUS, SCOPED — never a scalar.
///
/// Three DISJOINT scopes, and their total:
///
/// * `InStockpileRegions` — a dropped item whose cell is inside a stockpile
///   (`JobBoard::stockpile_at`, the same predicate `colony_food_stock` and
///   the fetch leg use). The pantry.
/// * `OnGroundAnywhere` — a dropped item that is NOT in a stockpile.
/// * `CarriedByColonists` — an `Inventory` slot.
///
/// A dropped item is counted in exactly one of the first two, so the three
/// partition and `Total` is genuinely their sum. That is the whole point:
/// `0 in stockpiles · 64 carried · 3 on ground` diagnoses hauling at a
/// glance, and `0 · 0 · 0` diagnoses the forge.
///
/// DETERMINISM: accumulated into a `BTreeMap` keyed by the itemdef id, so
/// the census is independent of ECS join order; emitted heaviest-first with
/// the LABEL as the tiebreak, so two equal-weight items cannot swap places
/// between assemblies.
pub fn stock_census<'a>(
    dropped: impl Iterator<Item = (&'a common::comp::PickupItem, &'a common::comp::Pos)>,
    inventories: impl Iterator<Item = &'a common::comp::Inventory>,
    board: &JobBoard,
    cap: usize,
) -> (Vec<StockRowV1>, u32, bool) {
    // [stockpile, carried, ground]
    let mut by_label: std::collections::BTreeMap<String, [u32; 3]> =
        std::collections::BTreeMap::new();
    // A modular item has no `itemdef_id`. Bucketing it under one honest
    // label is better than dropping it: an uncounted item is exactly the
    // kind of absence this section exists to make visible.
    let label_of = |id: Option<&str>| id.unwrap_or("<modular item (no itemdef id)>").to_string();

    for (item, pos) in dropped {
        let label = label_of(item.item().item_definition_id().itemdef_id());
        let cell = pos.0.map(|e| e.floor() as i32);
        let slot = if board.stockpile_at(cell).is_some() { 0 } else { 2 };
        by_label.entry(label).or_default()[slot] += item.amount();
    }
    for inv in inventories {
        for item in inv.slots().flatten() {
            let label = label_of(item.item_definition_id().itemdef_id());
            by_label.entry(label).or_default()[1] += item.amount();
        }
    }

    let distinct = by_label.len() as u32;
    let mut ordered: Vec<(String, [u32; 3])> = by_label.into_iter().collect();
    // Heaviest first; the label breaks every tie, so the order is total.
    ordered.sort_by(|a, b| {
        let (ta, tb) = (a.1.iter().sum::<u32>(), b.1.iter().sum::<u32>());
        tb.cmp(&ta).then_with(|| a.0.cmp(&b.0))
    });
    let truncated = ordered.len() > cap;
    ordered.truncate(cap);

    let mut rows = Vec::with_capacity(ordered.len() * 4);
    for (label, counts) in ordered {
        for (i, scope) in StockScopeV1::DISJOINT.into_iter().enumerate() {
            rows.push(StockRowV1 {
                item_label: label.clone(),
                count: counts[i],
                scope,
            });
        }
        rows.push(StockRowV1 {
            item_label: label,
            // Summed HERE, from the same three numbers the rows above
            // carry, so a reader adding them up cannot get a different
            // answer than the row labelled `total`.
            count: counts.iter().sum(),
            scope: StockScopeV1::Total,
        });
    }
    (rows, distinct, truncated)
}

/// The profession histogram over `WorkType::ALL`, plus the unnamed bucket.
///
/// ★ TOTAL BY CONSTRUCTION and TWO FRAMES KEPT APART. The array is sized by
/// `WorkType::COUNT`, so a new lane cannot fall out of the histogram the
/// way it fell out of three hand-written variant lists before. The
/// denominator is the LOADED ECS roster, and membership is tested by keyed
/// `get` on the board rather than by walking `JobBoard::professions`, so
/// board entries for colonists who have since unloaded cannot inflate the
/// count. Those entries are reported separately
/// (`professions_board_entries`) rather than subtracted, because
/// subtracting one frame from another is how this subsystem loses rows.
///
/// ★ A CORRECTION, FOUND BY RUNNING THE FALSIFIER. An earlier version of
/// this doc also claimed the keyed lookup was what kept the reply
/// independent of `HashMap` iteration order. It is not: the output is an
/// array indexed by `lane_index()`, and counting into an array is
/// order-free however the input is walked. Rewriting this body to iterate
/// `board.professions` left the determinism pin GREEN — correctly, because
/// that rewrite is not a determinism defect. The keyed lookup earns its
/// place on the FRAME argument alone, which is the one above.
pub fn profession_histogram(
    board: &JobBoard,
    colonist_uids: &[Uid],
) -> ([u32; WorkType::COUNT], u32) {
    let mut hist = [0u32; WorkType::COUNT];
    let mut unnamed = 0u32;
    for u in colonist_uids {
        match board.professions.get(u) {
            Some(w) => hist[w.lane_index()] += 1,
            None => unnamed += 1,
        }
    }
    (hist, unnamed)
}

pub fn provide(ctx: &InspectCtx<'_>) -> SectionPayloadV1 {
    let Some(c) = ctx.colony else {
        // Not requested, so nothing was measured. Answering with zeroes
        // would say "the colony holds nothing", which is the opposite
        // conclusion from "nobody counted".
        return not_measured(SectionIdV1::Colony);
    };
    let board = ctx.board;
    let (held, since) = board.colony_drive;

    // Households, in the board's own push order (stable per world), with
    // members uid-sorted by the derivation itself.
    let (houses, _bed_house) = derive_households(&board.designated, &board.beds);
    let beds_in_houses: u32 = houses.iter().map(|h| h.beds).sum();
    let households: Vec<HouseholdRowV1> = houses
        .iter()
        .map(|h| HouseholdRowV1 {
            min: h.min,
            max: h.max,
            beds: h.beds,
            capacity: household_capacity(h.beds),
            members: h
                .members
                .iter()
                .map(|u| HouseholdMemberV1 {
                    uid: *u,
                    // `None` is a real state: a bed whose owner has
                    // unloaded. Reported, not hidden behind the uid.
                    name: name_of(ctx.names, *u).map(str::to_owned),
                })
                .collect(),
        })
        .collect();

    let (professions, profession_unnamed) = profession_histogram(board, &c.colonist_uids);

    // ★ THE LADDER, RE-RUN. See `ColonyDriveVerdictV1`'s own doc for why
    // this is a second SAMPLE and not a second producer: the board stores
    // no reason to read, so the only honest way to name one is to run the
    // sim's own pure function again and carry every input beside the
    // answer.
    let pop = c.colonist_uids.len().max(1) as u32;
    let beds = board.beds.len() as u32;
    let food_per_cap = colony_drive_food_per_cap(c.food_pantry, c.food_total, pop);
    let (want, deciding, value) = colony_drive_for(food_per_cap, c.threats, beds, pop, held);

    SectionPayloadV1::Colony(ColonySectionV1 {
        drive: held,
        drive_since_tick: since,
        // Saturating: `server_tick` is boot-relative and a restart resets
        // it to 0, so a stale `since` would otherwise wrap into an
        // enormous age.
        drive_held_ticks: ctx.frames.server_tick.saturating_sub(since),
        verdict: Some(ColonyDriveVerdictV1 {
            want,
            deciding: deciding.to_string(),
            value,
            // The Sustain arm's threshold is a BAND, so a value without
            // its bar is unreadable. Read from `colony_sustain_bar`, the
            // ONE producer the gate itself uses — a witness that
            // recomputed the gate's threshold beside the gate is how a
            // wrong diagnosis got published here once already.
            bar: colony_sustain_bar(held),
            food_per_cap,
            food_pantry: c.food_pantry,
            food_total: c.food_total,
            threats: c.threats,
            beds,
            pop,
        }),
        households,
        beds_total: beds,
        beds_outside_households: beds.saturating_sub(beds_in_houses),
        professions,
        profession_unnamed,
        roster_loaded: c.colonist_uids.len() as u32,
        professions_board_entries: board.professions.len() as u32,
        stock: c.stock.clone(),
        stock_distinct: c.stock_distinct,
        stock_truncated: c.stock_truncated,
        jobs: c.jobs,
        designations: board.designated_regions().count() as u32,
        tick: ctx.frames.server_tick,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(n: u64) -> Uid { Uid(std::num::NonZeroU64::new(n).expect("nonzero")) }

    /// ★ THE HISTOGRAM IS TOTAL OVER `WorkType::ALL` AND ACCOUNTS FOR
    /// EVERY LOADED COLONIST.
    ///
    /// FALSIFIER: drop the `None => unnamed += 1` arm and the conservation
    /// assertion goes RED; index the array with a hand-written lane number
    /// instead of `lane_index()` and the `Craft` case goes RED.
    #[test]
    fn the_profession_histogram_is_total_and_conserves_the_roster() {
        let mut board = JobBoard::default();
        // Six loaded colonists; four named, two not.
        let roster: Vec<Uid> = (1..=6).map(uid).collect();
        board.professions.insert(uid(1), WorkType::Craft);
        board.professions.insert(uid(2), WorkType::Craft);
        board.professions.insert(uid(3), WorkType::Farm);
        board.professions.insert(uid(4), WorkType::Guard);
        // ★ AN ENTRY FOR A COLONIST WHO IS NOT LOADED. It must not be
        // counted -- the board outlives the ECS roster, and mixing the two
        // is the two-frames defect.
        board.professions.insert(uid(99), WorkType::Cook);

        let (hist, unnamed) = profession_histogram(&board, &roster);
        assert_eq!(hist[WorkType::Craft.lane_index()], 2);
        assert_eq!(hist[WorkType::Farm.lane_index()], 1);
        assert_eq!(hist[WorkType::Guard.lane_index()], 1);
        assert_eq!(
            hist[WorkType::Cook.lane_index()],
            0,
            "an unloaded colonist's board entry must not enter the loaded histogram"
        );
        assert_eq!(unnamed, 2);
        // CONSERVATION: every loaded colonist lands in exactly one bucket.
        assert_eq!(
            hist.iter().sum::<u32>() + unnamed,
            roster.len() as u32,
            "a colonist fell out of the histogram"
        );
        // And every lane is addressable, including the one three
        // hand-written lists dropped.
        for w in WorkType::ALL {
            let _ = hist[w.lane_index()];
        }
        assert_eq!(hist.len(), WorkType::COUNT);
    }

    /// The histogram's denominator is the ECS roster, so an EMPTY roster
    /// is all-zero and not "everyone is unnamed".
    #[test]
    fn an_empty_roster_names_nobody_and_invents_nobody() {
        let board = JobBoard::default();
        let (hist, unnamed) = profession_histogram(&board, &[]);
        assert_eq!(hist.iter().sum::<u32>(), 0);
        assert_eq!(unnamed, 0);
    }
}
