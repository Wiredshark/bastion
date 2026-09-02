//! bastion (G1c): the colony **BUILDS** the worldgen plot it asked for.
//!
//! G1a laid a plot out; G1b put it into the live world index and handed back
//! `GrownPlot { blocks, beds, .. }`. This module is the third piece: turning
//! that block list into work the colony's existing Build pipeline can do, and
//! deciding what to do when worldgen says no.
//!
//! Everything here is **pure**. There is no world, no index, no board and no
//! ECS in this file — the wiring lives at its call sites in
//! `bastion_jobs.rs`, marked `// ★ G1c`. That split is deliberate: the four
//! decisions this row actually gets wrong are all decidable from values
//! (which cells are worth building, when a cell is finished, what seed to
//! retry with, and which of the two house paths the drain takes), and a
//! decision that needs a generated world to test is a decision that does not
//! get tested.
//!
//! # The three findings this module encodes
//!
//! 1. **BUILD ONLY WHAT SHOWS.** `GrownPlot::blocks` is the plot's whole fill
//!    tree — foundations included. A house's foundation courses are dirt and
//!    stone swapped for stone *below the surface*: a colonist would spend
//!    real days on cells no one can ever see. Worse, the plot is ALREADY in
//!    the index by the time we get here, so any cell whose target the terrain
//!    already holds is finished before it starts. [`visible_cells`] drops
//!    both classes and counts each one, because "the house was 900 blocks and
//!    we built 300" and "the house was 900 blocks and we built 900" are
//!    different worlds and only one of them is worth a colonist's week.
//!
//! 2. **A BED SPRITE IS NOT `is_filled()`.** The existing plan generator
//!    retires a cell when `terrain.get(pos).is_filled()`. Every bed, every
//!    window, every door in a worldgen house is an *air* block carrying a
//!    sprite, so under that rule those cells are never done, the plan never
//!    retires, and the house is never registered as a house. A plan cell with
//!    a known target is done when it MATCHES ITS TARGET — see
//!    [`plot_cell_is_done`], which keeps the old rule for the cells that have
//!    no target (today's bedroll plans) so the change is an extension, not a
//!    replacement.
//!
//! 3. **`NoRoom` IS PER-SEED, NOT SATURATION.** G1b measured it: 200 seeds on
//!    one fresh town grew 11 houses and refused 189 times, and *nine of the
//!    eleven successes came after the first refusal*. A caller that read one
//!    `NoRoom` as "the town is full" would have stopped at two houses. So a
//!    refusal retries with a fresh seed ([`next_house_seed`]) up to
//!    [`HOUSE_NO_ROOM_RETRIES_PER_DAY`] times, and only then gives the day up
//!    — loudly, with the count, so a town that has genuinely run out of
//!    roadside says so in its own log.

use common::{store::Id, terrain::Block};
use vek::*;
use world::site::{Site, bastion_layout::LayoutKind};

/// How many fresh seeds the colony will spend on one day's house before it
/// accepts that worldgen has no roadside room for it.
///
/// Sized from G1b's measurement rather than picked: on the fresh test town
/// the per-seed success rate was 11/200 in the *saturating* limit, but the
/// first two houses came inside the first handful of seeds. Eight retries is
/// generous for a town with room and cheap for one without — each refusal is
/// free by construction (`grow_plot` refuses before it spends), so the whole
/// day's budget costs eight placement searches spread over eight ticks, or
/// about a quarter of a second of one core.
pub const HOUSE_NO_ROOM_RETRIES_PER_DAY: u32 = 8;

/// One worldgen plot the colony has ordered, and everything needed to know
/// whether it is finished and what to register when it is.
///
/// ★ DEVIATION FROM THE BRIEF, DELIBERATE: `id` is a
/// [`common::bastion::ZoneId`] (`u64`), not a `u32`. This id is not a private
/// counter — it is *the same value* as the key of the matching entry in
/// `JobBoard::plans`, which the board mints from `next_zone: ZoneId`. A `u32`
/// here would mean a narrowing cast at every correlation site, i.e. a plan id
/// that cannot round-trip to the collection it indexes. Everything else in
/// the struct is exactly as specified.
#[derive(Clone, Debug)]
pub struct PlotPlan {
    /// The `JobBoard::plans` id this plot's cells were queued under. When
    /// that plan retires, this plot is built.
    pub id: common::bastion::ZoneId,
    /// The site the plot was grown on.
    pub site: Id<Site>,
    /// What worldgen was asked for.
    pub kind: LayoutKind,
    /// The plot's footprint. `min` inclusive, `max` EXCLUSIVE — the
    /// convention `world::site::bastion_layout::PlacedPlot::aabr_wpos` uses,
    /// carried through unchanged rather than silently re-based (a
    /// half-open/closed mix-up on a footprint is a whole row of wall).
    pub aabr_wpos: Aabr<i32>,
    /// The door colonists walk through, at the plot's altitude. `None` for
    /// kinds that have no door.
    pub door: Option<Vec3<i32>>,
    /// One entry per bed: the bed HEAD sprite's position. These are what get
    /// registered as sleepable slots when the plot is built.
    pub beds: Vec<Vec3<i32>>,
    /// Every block the plot is made of, BEFORE the visibility filter — the
    /// denominator of "we built 300 of 900". Progress is reported against
    /// the queued cells, not against this.
    pub total: usize,
    /// Whether the house has already been registered for the household
    /// census. **One-shot**: a bed is a resource nothing re-adds, so a second
    /// registration would push a second Bed region over the same footprint
    /// and the town would count one house twice.
    pub registered: bool,
    /// The game day the plot was queued, so `PLOT BUILT` can say how long it
    /// took.
    pub queued_day: i64,
}

/// What the colony should do about a house it asked worldgen for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseRequestOutcome {
    /// Worldgen placed the plot. The request is finished.
    Grown,
    /// The world index was shared this tick (a chunk job holds it), so
    /// nothing was attempted and nothing was spent. The request STAYS and is
    /// retried on the next tick. This is the healthy refusal, not an error.
    Deferred,
    /// Worldgen found no roadside room *for this seed*. The request stays for
    /// today and the next tick tries a fresh seed. `refusals` is the count so
    /// far today, including this one.
    ///
    /// (Named for the brief's API. It means "keep the request and retry" —
    /// the retry happens on the next TICK, not the next day; the day is only
    /// what the retry budget is scoped to.)
    RetryNextDay { refusals: u32 },
    /// [`HOUSE_NO_ROOM_RETRIES_PER_DAY`] fresh seeds in a row were refused.
    /// The request is dropped; the daily housing gate may ask again tomorrow.
    GaveUp { refusals: u32 },
}

/// The `NoRoom` decision, given the refusal count AFTER this refusal is
/// counted. Split out from the wiring so the retry budget is testable without
/// a generated world.
pub fn no_room_outcome(refusals: u32) -> HouseRequestOutcome {
    if refusals >= HOUSE_NO_ROOM_RETRIES_PER_DAY {
        HouseRequestOutcome::GaveUp { refusals }
    } else {
        HouseRequestOutcome::RetryNextDay { refusals }
    }
}

/// The seed the next house attempt is driven by.
///
/// **Deterministic by construction**: a pure function of the game day, how
/// many plots the colony has already planned, and how many times worldgen has
/// refused today. No wall clock, no OS entropy, no `HashMap` order — two runs
/// of the same colony ask worldgen for the same houses in the same order.
///
/// The mixing is SplitMix64's finalizer over an odd-multiplied packing of the
/// three inputs. Each input is multiplied by a distinct odd constant before
/// the XOR, so varying any ONE input alone is a bijection on the pre-mix
/// value — which is exactly the property the retry loop depends on: eight
/// consecutive refusals must produce eight DIFFERENT seeds, or the retry is
/// theatre (it would re-run the identical placement search and get the
/// identical refusal, eight times).
pub fn next_house_seed(day: i64, plan_count: usize, refusals_today: u32) -> u64 {
    let mut x = (day as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (plan_count as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (refusals_today as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    x
}

/// **BUILD ONLY WHAT SHOWS.** Reduce a grown plot's whole block list to the
/// cells a colonist should actually be sent to, against the terrain as it
/// stands right now.
///
/// Returns `(cells_to_build, skipped_underground, skipped_unchanged)`, with
/// the kept cells in the input's own (z, y, x) order so a builder still lays
/// courses bottom-up.
///
/// Three rules, in this order:
///
/// 1. **Already there** — the current block equals the target. Counted as
///    `skipped_unchanged`. This is not a rare case: the plot was inserted
///    into the world index before this list was produced, so any chunk that
///    (re)generates afterwards paints the plot itself, and the colony must
///    not send anyone to build a wall that is standing.
/// 2. **Invisible** — the current block and the target are BOTH `is_filled()`.
///    A foundation course swapping dirt for stone underground changes nothing
///    a player or a colonist can see. Counted as `skipped_underground`.
/// 3. Everything else is kept, **including cells the terrain cannot read**
///    (`None` — an unloaded chunk). Keeping them is the deliberate choice:
///    the existing generator already skips unloaded cells at job-creation
///    time, and dropping them HERE would freeze the plot's own denominator at
///    whatever happened to be streamed on the tick the plot was grown — a
///    house would be permanently "complete" because the half of it nobody had
///    loaded was never in the plan.
///
/// Rule 1 is checked before rule 2 on purpose: a cell that is both (an
/// already-correct filled block) is `unchanged`, which is the more specific
/// and more useful fact — it says the world already agrees with worldgen,
/// where `underground` says the work would be pointless.
pub fn visible_cells(
    blocks: &[(Vec3<i32>, Block)],
    terrain_at: impl Fn(Vec3<i32>) -> Option<Block>,
) -> (Vec<(Vec3<i32>, Block)>, usize, usize) {
    let mut kept = Vec::with_capacity(blocks.len());
    let mut skipped_underground = 0usize;
    let mut skipped_unchanged = 0usize;
    for (pos, target) in blocks.iter().copied() {
        match terrain_at(pos) {
            Some(current) if current == target => skipped_unchanged += 1,
            Some(current) if current.is_filled() && target.is_filled() => {
                skipped_underground += 1
            },
            _ => kept.push((pos, target)),
        }
    }
    (kept, skipped_underground, skipped_unchanged)
}

/// Is a plan cell finished?
///
/// `current` is the terrain block at the cell (`None` = unreadable, i.e. an
/// unloaded chunk). `target` is the block the plot wants there (`None` = this
/// cell belongs to a plan with no per-cell target — today's bedroll plans,
/// which the Build completion fills with a generic grey rock).
///
/// - **With a target**: done iff the terrain already *is* that block. This is
///   the rule the whole row turns on. A worldgen house is mostly sprites on
///   air — beds, doors, windows, lanterns — none of which are `is_filled()`,
///   so the old rule could never retire a plot plan and the house would never
///   be registered. It is also *stricter* in the right direction: a Build
///   completion that placed the wrong block leaves the cell unfinished
///   instead of silently retiring it.
/// - **Without a target**: today's rule, unchanged (`is_filled()`), so every
///   existing plan behaves exactly as it did.
/// - **Unreadable terrain** is never done, under either rule. That is the
///   existing fail-open-on-retirement policy: a plan must not be declared
///   complete on cells nobody could read.
pub fn plot_cell_is_done(current: Option<Block>, target: Option<Block>) -> bool {
    match (current, target) {
        (Some(current), Some(target)) => current == target,
        (Some(current), None) => current.is_filled(),
        (None, _) => false,
    }
}

/// Which of the two house paths the `pending_house` drain takes.
///
/// `true` = ask worldgen for a real plot (G1c); `false` = today's single
/// bedroll designation, byte-for-byte the pre-G1c behaviour.
///
/// **THE IDENTITY FALLBACK.** With `no_worldgen_plots` set (the
/// `BASTION_NO_WORLDGEN_PLOTS` env var) this is `false` for every input, so
/// the whole row switches off and the drain does exactly what it did before.
/// Factored out as a pure predicate precisely so that claim is a test and not
/// a comment — a test that read the env var would be testing the harness.
///
/// `have_site` is the fourth term and is NOT in the brief; it is here because
/// a guard must not starve the thing it protects. A colony founded on open
/// ground sits on no worldgen site at all, so there is nothing for
/// `grow_plot` to grow on. Without this term such a colony would set a plot
/// request that could never be fulfilled and would never place a bed either —
/// it would stop building houses forever, which is strictly worse than the
/// behaviour this row replaces. With it, a siteless colony keeps the bedroll
/// path and only a colony that actually stands in a town gets worldgen plots.
pub fn drain_takes_the_plot_path(
    no_worldgen_plots: bool,
    have_site: bool,
    plot_plan_in_flight: bool,
    plot_request_pending: bool,
) -> bool {
    !no_worldgen_plots && have_site && !plot_plan_in_flight && !plot_request_pending
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::terrain::{BlockKind, SpriteKind};

    fn rock() -> Block { Block::new(BlockKind::Rock, Rgb::new(80, 80, 80)) }
    fn wood() -> Block { Block::new(BlockKind::Wood, Rgb::new(90, 60, 30)) }
    fn earth() -> Block { Block::new(BlockKind::Earth, Rgb::new(70, 50, 30)) }
    fn air() -> Block { Block::air(SpriteKind::Empty) }
    fn bed() -> Block { Block::air(SpriteKind::BedWoodWoodlandHead) }

    /// THE row's first finding, as a pin: the colony is sent to the cells a
    /// player can see change, and to no others.
    ///
    /// PLANTED DEFECT (the one this test exists to catch): keep the
    /// filled -> filled cell. That is the "build the foundation" version of
    /// this function — it compiles, it is not obviously wrong, and it costs
    /// the colony days of work on blocks that are underground. The
    /// `skipped_underground` assert below turns red for it, and so does the
    /// kept-order assert, because the extra cell lands in the middle of the
    /// sequence.
    #[test]
    fn a_plot_builds_only_what_shows() {
        // Deliberately laid out in (z, y, x) order, like a real plot list.
        let plan = [
            // z=0: an underground foundation course. Earth -> Rock: both
            // filled, nobody will ever see it.
            (Vec3::new(0, 0, 0), rock()),
            // z=1: a wall that the world index ALREADY painted (a chunk
            // regenerated after the plot went in).
            (Vec3::new(0, 0, 1), wood()),
            // z=1: a wall that is genuinely missing. THE work.
            (Vec3::new(1, 0, 1), wood()),
            // z=2: the bed. An AIR block with a sprite — not `is_filled()`,
            // which is why the old retire rule could never finish this house.
            (Vec3::new(1, 0, 2), bed()),
        ];
        let terrain_at = |p: Vec3<i32>| match (p.x, p.y, p.z) {
            (0, 0, 0) => Some(earth()), // filled, and the target is filled
            (0, 0, 1) => Some(wood()),  // identical to the target
            (1, 0, 1) => Some(air()),   // open air: build it
            (1, 0, 2) => Some(air()),   // open air: place the bed
            _ => None,
        };

        let (kept, underground, unchanged) = visible_cells(&plan, terrain_at);
        println!(
            "visible_cells: plan={} kept={} skipped_underground={} skipped_unchanged={} \
             kept_cells={:?}",
            plan.len(),
            kept.len(),
            underground,
            unchanged,
            kept.iter().map(|(p, _)| *p).collect::<Vec<_>>(),
        );

        assert_eq!(
            underground, 1,
            "the Earth -> Rock foundation course is invisible and must not be built"
        );
        assert_eq!(
            unchanged, 1,
            "a cell whose terrain already IS the target is finished before it starts"
        );
        assert_eq!(
            kept,
            vec![(Vec3::new(1, 0, 1), wood()), (Vec3::new(1, 0, 2), bed())],
            "only the missing wall and the bed are work — and in the plot's own \
             (z, y, x) order, so the builder still lays courses bottom-up"
        );

        // An UNREADABLE cell (unloaded chunk) is kept, not dropped: dropping
        // it would freeze the house's denominator at whatever was streamed on
        // the tick it was grown, and the missing half would never be built.
        let (kept_unloaded, u2, c2) =
            visible_cells(&[(Vec3::new(9, 9, 9), wood())], |_| None);
        println!("unloaded cell: kept={} underground={u2} unchanged={c2}", kept_unloaded.len());
        assert_eq!(kept_unloaded.len(), 1, "an unloaded cell must stay in the plan");
        assert_eq!((u2, c2), (0, 0), "an unloaded cell is neither skip reason");
    }

    /// THE row's second finding: a plan cell with a target is done when it
    /// MATCHES that target, not when it is solid.
    ///
    /// PLANTED DEFECT: use `is_filled()` for everything (today's rule). The
    /// bed assertions below turn red — and that defect is not hypothetical,
    /// it is the pre-existing rule this row had to change: under it a house's
    /// beds, doors and windows are never done, the plan never retires, and
    /// `PLOT BUILT` never fires.
    #[test]
    fn a_plot_cell_is_done_only_when_it_matches_its_target() {
        // A bed target over open air: NOT done. Under `is_filled()` this cell
        // is also not done, so the discriminating case is the one below it.
        assert!(
            !plot_cell_is_done(Some(air()), Some(bed())),
            "an empty cell that wants a bed is not done"
        );
        // THE discriminating case: the bed is placed. `is_filled()` is FALSE
        // for a bed sprite (it is air), so today's rule would still call this
        // unfinished forever.
        assert!(
            !bed().is_filled(),
            "precondition of this whole row: a bed sprite is not a filled block. \
             If this ever becomes true the retire rule can go back to is_filled()"
        );
        assert!(
            plot_cell_is_done(Some(bed()), Some(bed())),
            "a placed bed matches its target and IS done — the case the old rule missed"
        );
        // A wall built as the WRONG block stays unfinished rather than
        // silently retiring.
        assert!(
            !plot_cell_is_done(Some(rock()), Some(wood())),
            "a filled block that is not the target is not done"
        );
        // No target: today's rule, exactly.
        assert!(
            plot_cell_is_done(Some(rock()), None),
            "a targetless plan cell keeps the old is_filled() rule"
        );
        assert!(
            !plot_cell_is_done(Some(air()), None),
            "a targetless plan cell over air is still unfinished"
        );
        // Unreadable terrain is never done, under either rule.
        assert!(!plot_cell_is_done(None, Some(bed())));
        assert!(!plot_cell_is_done(None, None));
        println!(
            "plot_cell_is_done: bed_over_air={} bed_placed={} rock_no_target={} unloaded={}",
            plot_cell_is_done(Some(air()), Some(bed())),
            plot_cell_is_done(Some(bed()), Some(bed())),
            plot_cell_is_done(Some(rock()), None),
            plot_cell_is_done(None, Some(bed())),
        );
    }

    /// G1b measured that `NoRoom` is a per-seed placement failure, not
    /// saturation (200 seeds on one town: 11 houses, 189 refusals, 9 of the
    /// 11 AFTER the first refusal). So the retry must actually change the
    /// input, and it must still stop.
    ///
    /// PLANTED DEFECT: reuse the same seed on every retry. The distinctness
    /// assert turns red — and that defect is invisible in a live log, because
    /// eight identical refusals look exactly like eight honest ones.
    #[test]
    fn no_room_retries_with_a_fresh_seed_then_gives_up_for_the_day() {
        let (day, plans) = (17i64, 3usize);
        let mut seeds = Vec::new();
        let mut outcomes = Vec::new();
        // The request starts with 0 refusals; each NoRoom bumps the count.
        for refusals_before in 0..HOUSE_NO_ROOM_RETRIES_PER_DAY {
            seeds.push(next_house_seed(day, plans, refusals_before));
            outcomes.push(no_room_outcome(refusals_before + 1));
        }
        println!("seeds={seeds:?}");
        println!("outcomes={outcomes:?}");

        let mut sorted = seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            seeds.len(),
            "every retry must draw a FRESH seed, or the retry re-runs the identical \
             placement search and earns the identical refusal: {seeds:?}"
        );

        // The budget: the first seven refusals keep the request, the eighth
        // ends the day.
        for (i, o) in outcomes.iter().enumerate() {
            let n = i as u32 + 1;
            if n < HOUSE_NO_ROOM_RETRIES_PER_DAY {
                assert_eq!(
                    *o,
                    HouseRequestOutcome::RetryNextDay { refusals: n },
                    "refusal {n} of {HOUSE_NO_ROOM_RETRIES_PER_DAY} must keep the request"
                );
            } else {
                assert_eq!(
                    *o,
                    HouseRequestOutcome::GaveUp { refusals: n },
                    "at the cap the day must be given up, not retried forever"
                );
            }
        }
        assert_eq!(
            outcomes.last().copied(),
            Some(HouseRequestOutcome::GaveUp {
                refusals: HOUSE_NO_ROOM_RETRIES_PER_DAY
            }),
            "the loop must terminate at the cap"
        );

        // Determinism, stated as an assert rather than trusted: the same
        // colony on the same day asks for the same houses.
        assert_eq!(
            next_house_seed(day, plans, 4),
            next_house_seed(day, plans, 4),
            "the seed must be a pure function of (day, plans, refusals)"
        );
        // ...and a different day, or a different plot count, is a different
        // house — otherwise every day would retry the same refused search.
        assert_ne!(next_house_seed(day, plans, 0), next_house_seed(day + 1, plans, 0));
        assert_ne!(next_house_seed(day, plans, 0), next_house_seed(day, plans + 1, 0));
    }

    /// The identity fallback, as a test rather than a promise: with
    /// `BASTION_NO_WORLDGEN_PLOTS` set the drain takes the bedroll path for
    /// EVERY input, so the row is switchable off to exactly today's
    /// behaviour.
    ///
    /// The env var itself is read at the call site; what is asserted here is
    /// the predicate it feeds, so the test is about the decision and not
    /// about the harness's environment.
    #[test]
    fn the_fallback_is_identity() {
        // With the fallback on, nothing else can turn the plot path back on.
        for have_site in [false, true] {
            for in_flight in [false, true] {
                for pending in [false, true] {
                    assert!(
                        !drain_takes_the_plot_path(true, have_site, in_flight, pending),
                        "BASTION_NO_WORLDGEN_PLOTS must be absolute: \
                         site={have_site} in_flight={in_flight} pending={pending}"
                    );
                }
            }
        }
        // With it off, the plot path needs a site and a clear queue.
        assert!(
            drain_takes_the_plot_path(false, true, false, false),
            "a colony on a worldgen site with nothing in flight asks for a plot"
        );
        assert!(
            !drain_takes_the_plot_path(false, false, false, false),
            "a colony that stands on no worldgen site keeps the bedroll path — a guard \
             must not starve the thing it protects"
        );
        assert!(
            !drain_takes_the_plot_path(false, true, true, false),
            "one house at a time: a plot already being built blocks the next request"
        );
        assert!(
            !drain_takes_the_plot_path(false, true, false, true),
            "a request already in flight is not re-issued"
        );
        println!(
            "drain_takes_the_plot_path: fallback_on={} normal={} no_site={} in_flight={}",
            drain_takes_the_plot_path(true, true, false, false),
            drain_takes_the_plot_path(false, true, false, false),
            drain_takes_the_plot_path(false, false, false, false),
            drain_takes_the_plot_path(false, true, true, false),
        );
    }
}
