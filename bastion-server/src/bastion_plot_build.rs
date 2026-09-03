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
    /// ★ G1d: the seed this plot was grown from — the KEY back into the
    /// colony's growth log. `PLOT BUILT` flips that line's `registered` by
    /// matching this value, so a restart knows which houses are already in the
    /// household census and which are still half-built walls. Unique per entry
    /// by construction: [`next_house_seed`] mixes (day, plan count, refusals)
    /// and any two orders differ in at least one of the three.
    pub seed: u64,
}

// ─── ★ G1d: THE GROWTH LOG'S TWO SEAMS ──────────────────────────────────────

/// The wire mirror of a layout kind, from the runtime kind.
///
/// `common::bastion::BastionLayoutKindV1` exists because `common` cannot see
/// `world` and worldgen's own `LayoutKind` must not carry serde (a runtime
/// enum that is also a save format turns every worldgen change into a save
/// migration). These two functions are the whole of the mapping, in one place:
/// a kind added to either enum fails to compile HERE — non-exhaustive match —
/// rather than decoding into the wrong building somewhere else.
///
/// ★ DEVIATION FROM THE BRIEF, FORCED: the brief asks for `From` impls. They
/// are not writable here. Both types are foreign to this crate (one is
/// `common`'s, the other `world`'s), so `impl From<LayoutKind> for
/// BastionLayoutKindV1` is an orphan-rule error (E0117, confirmed by
/// compiling it), and the only two crates that could host the impls are
/// `world` — which the brief puts off limits — and `common`, which must not
/// learn about `world` at all. Free functions in the crate that already owns
/// this seam are what is left; they say the same thing and are called at the
/// same two places.
pub fn wire_kind(kind: LayoutKind) -> common::bastion::BastionLayoutKindV1 {
    use common::bastion::BastionLayoutKindV1 as K;
    match kind {
        LayoutKind::House => K::House,
        LayoutKind::FarmField => K::FarmField,
        LayoutKind::Workshop => K::Workshop,
    }
}

/// The runtime layout kind, from the wire mirror. The other half of the seam
/// above — a growth-log line is replayed by handing this back to `grow_plot`.
pub fn runtime_kind(kind: common::bastion::BastionLayoutKindV1) -> LayoutKind {
    use common::bastion::BastionLayoutKindV1 as K;
    match kind {
        K::House => LayoutKind::House,
        K::FarmField => LayoutKind::FarmField,
        K::Workshop => LayoutKind::Workshop,
    }
}

/// What the boot-time replay does with one line of the colony's growth log.
///
/// BOTH variants re-grow the plot, and that is the half of this row that is
/// easiest to get wrong. It is tempting to read "the house is finished, the
/// terrain kept it" as "there is nothing to do" — but the thing that did NOT
/// survive the restart is the plot's claim on the site's tiles. A finished
/// house whose plot is not re-grown reads as free roadside, and the next house
/// the colony orders is laid straight through its wall. The variants differ
/// only in what happens AFTER the grow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayDecision {
    /// `registered = true`: the house was finished and already in the
    /// household census before the restart. Re-grow the plot, then SKIP the
    /// plan queue and SKIP the registration. Both skips, for different
    /// reasons:
    ///
    /// - **Queue nothing** is not a rule this variant has to enforce so much
    ///   as one it makes cheap and honest: every block of a finished house is
    ///   standing in the saved terrain, so [`visible_cells`] against that
    ///   terrain returns an empty list anyway. Skipping the whole path saves
    ///   the render and says plainly that a finished house is not work.
    /// - **Register nothing** is load-bearing. The finished house comes back
    ///   on its own through the terrain scan (`adopt_beds_surface` re-adopts
    ///   it in place — beds, hearth, containers and all). Registering it a
    ///   second time would push a second Bed region over the same footprint,
    ///   and `derive_households` counts Bed REGIONS: the town would count one
    ///   house as two and the immigration gate would open on beds nobody has.
    ReGrowOnly { kind: LayoutKind, seed: u64 },
    /// `registered = false`: the house was still being built when the server
    /// stopped (or was never finished at all). Re-grow the plot AND queue what
    /// the terrain does not already hold — which, for a half-built house, is
    /// exactly the half that is missing, because `visible_cells` measures
    /// against the terrain as it stands NOW.
    ReGrowAndQueue { kind: LayoutKind, seed: u64 },
}

impl ReplayDecision {
    /// What to ask worldgen for. The same for both variants — every entry is
    /// re-grown.
    pub fn kind(self) -> LayoutKind {
        match self {
            Self::ReGrowOnly { kind, .. } | Self::ReGrowAndQueue { kind, .. } => kind,
        }
    }

    /// Which plot. The same for both variants, for the same reason.
    pub fn seed(self) -> u64 {
        match self {
            Self::ReGrowOnly { seed, .. } | Self::ReGrowAndQueue { seed, .. } => seed,
        }
    }

    /// Does this entry queue cells and eventually register a house? `false`
    /// for an entry that was already registered — the one-shot rule.
    pub fn queues(self) -> bool { matches!(self, Self::ReGrowAndQueue { .. }) }
}

/// The replay decision for one growth-log line.
///
/// Split out from the wiring for the usual reason: this is a decision the row
/// can actually get wrong, and it is decidable from a value, so it is pinned
/// without a generated world. The call site in `bastion_jobs.rs` is marked
/// `// ★ G1d`.
pub fn replay_decision(entry: &common::bastion::BastionGrownPlotV1) -> ReplayDecision {
    let (kind, seed) = (runtime_kind(entry.kind), entry.seed);
    if entry.registered {
        ReplayDecision::ReGrowOnly { kind, seed }
    } else {
        ReplayDecision::ReGrowAndQueue { kind, seed }
    }
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

/// ★ G1c-d (2026-09-02): is a designated job at this cell still wanted?
///
/// The generator mints a plot cell while it does not match its worldgen
/// target ([`plot_cell_is_done`]); the phantom check and the mid-travel
/// moot check retired a Build job while its cell was FILLED (the kind rule:
/// Build goes into open space). Measured on arm b1 (pair c51f78b672, day 1):
/// the plan's first ten cells -- solid ground under a floor course -- were
/// minted and retired 2,814 times each, ate the whole build budget every
/// pass, and the other 1,868 cells never reached the board (31 placed all
/// day). The generator and the consumer must agree:
///
/// - **With a target**: wanted until the cell MATCHES it.
/// - **Without one**: the kind rule, byte-for-byte (every non-plot job).
pub fn plot_job_still_wanted(target: Option<Block>, current: Block, wanted_by_kind: bool) -> bool {
    match target {
        Some(t) => !plot_cell_is_done(Some(current), Some(t)),
        None => wanted_by_kind,
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

// ─── ★ G1c-c: THE TOWN STAFFS ITS BUILD ─────────────────────────────────────
//
// # The defect these two functions close
//
// Measured on arm b1 (pair `21ab563470`, 2026-09-02 19:33). The first worldgen
// house was laid out live — `PLOT PLAN QUEUED plan=81`, 1,909 visible cells —
// and then the day-1 line read:
//
//     HAUL LANE CEILING ... neediest=Some(Build)
//         demoted=["21:Farm","22:Farm","30:Farm","40:Farm","42:Build","55:Mine",..]
//     BUILD PROGRESS plan=81 queued=1909 placed=8652 remaining=1897 builders=0
//
// The allocator NAMED THE RIGHT LANE: `neediest=Some(Build)`, 1,909 open Build
// jobs against 0 named builders. But naming is all it could do. The only
// mechanism in this town that MOVES a colonist between trades is the hauler
// cap, and that mechanism returns each surplus hauler to the trade they
// already had — one of them (42) merely happened to be a builder already.
// Twelve cells were placed in a whole game day. At twelve a day the house
// takes 158 days.
//
// So the gap is not knowledge, it is STAFFING: a lane nobody is named by gets
// whoever wanders past it, and a 1,909-cell house is not built by passers-by.
// `builders_wanted` says how many the town should have and `pick_builders`
// says who — both pure, so the two decisions this row can actually get wrong
// (how many, and at whose expense) are pinned without a generated world. Same
// split the rest of this module uses; the wiring is `// ★ G1c-c` in
// `bastion_jobs.rs`.

/// ★ TASTE NUMBER, PENDING BEN'S CALL. One builder per this many open Build
/// cells.
///
/// Not measured. Picked so the b1 house finishes in a plausible week rather
/// than a plausible season, and named here — rather than inlined — precisely
/// so Ben can overrule one number instead of re-deriving a policy. On the b1
/// plot (1,909 open cells) it asks for 12 and [`BUILD_LANE_CAP`] trims that to
/// 6; at the measured day-1 rate that is still a multi-day house, which is
/// what the GROW CYCLES ARE REAL ruling wants — a house is not cheaper than a
/// crop.
pub const BUILD_CELLS_PER_BUILDER: usize = 150;

/// ★ TASTE NUMBER, PENDING BEN'S CALL. The hard ceiling on the Build lane, in
/// the same spirit as `bastion_jobs::haul_lane_cap`: a town of builders is not
/// a town either. Six is enough to move a house in days and small enough that
/// the farms, the kitchen and the mine keep running.
pub const BUILD_LANE_CAP: usize = 6;

/// ★ TASTE NUMBER, PENDING BEN'S CALL. The town spares at most one colonist in
/// six for the Build lane, mirroring `bastion_jobs::COLONISTS_PER_HAULER` (the
/// same 6, for the same reason: it is the share that still leaves a small
/// colony a workforce). This is the term that makes the answer depend on the
/// TOWN and not only on the house — a roster of 9 gives 1 builder for the same
/// 1,909-cell plot a roster of 49 staffs with 6.
pub const COLONISTS_PER_BUILDER: usize = 6;

/// The town never drops below this many cooks to staff a build. Two, not one,
/// so a single cook asleep or off-shift does not close the kitchen.
pub const COOK_FLOOR: usize = 2;

/// How many builders the town should have right now.
///
/// - **No plot plan open ⇒ 0.** This is the load-bearing clause and it is
///   first for a reason: without it the town would keep a standing build crew
///   for a house that does not exist, and every stray Build cell in the colony
///   (a ladder, a bed, a cook station) would pull farmers off the fields
///   forever. The draft exists to finish a HOUSE; with no house in flight
///   there is nothing to staff.
/// - Otherwise one builder per [`BUILD_CELLS_PER_BUILDER`] open cells, clamped
///   to at least 1 (a plan with a handful of cells left still deserves
///   somebody) and at most [`BUILD_LANE_CAP`].
/// - And never more than `roster / COLONISTS_PER_BUILDER`, so the ceiling is a
///   property of the town as well as of the house.
///
/// Deliberately total and saturating: a roster under [`COLONISTS_PER_BUILDER`]
/// yields 0, which is correct — a colony of five that spares a builder has
/// spared a fifth of itself.
pub fn builders_wanted(open_build_cells: usize, roster: usize, plot_plan_open: bool) -> usize {
    if !plot_plan_open {
        return 0;
    }
    let by_house = (open_build_cells / BUILD_CELLS_PER_BUILDER).clamp(1, BUILD_LANE_CAP);
    by_house.min(roster / COLONISTS_PER_BUILDER)
}

/// Who gets retrained into the Build lane, in order.
///
/// `candidates` is `(uid, the trade the town names them by, that colonist's
/// TOTAL lane tally)` — the third number is how much of themselves they have
/// put into their trades, the same `lane_total` the `PROFESSION` witness
/// prints. `by_lane_named` is how many colonists the town names by each trade.
///
/// The order, and why each term is the way round it is:
///
/// 1. **The most-staffed trades give first.** Taking the town's only miner to
///    build a wall is how a colony ends up with a house and no stone. Ten
///    farmers can spare four; two farmers cannot spare two.
/// 2. **Within a trade, the LEAST invested retrains.** `lane_total` is
///    time-held (ROW 48's unit), so this takes the newest, least-practised
///    colonist and leaves the veteran farmer farming. It costs the town the
///    least skill, and it is the opposite of what an argmax-driven scheme
///    would pick.
/// 3. **uid ascending** last, so two identical colonists resolve the same way
///    on every run of the same save.
///
/// Three lanes are never drafted, and one is floored:
///
/// - **Haul** — the hauler cap and the dedicated-hauler floor both own that
///   lane; drafting out of it would fight two mechanisms that run in the same
///   block, on the same day, in both directions.
/// - **Guard** — a guard is the town's answer to a threat, and a threat does
///   not wait for the house.
/// - **Build** — already a builder; drafting them is a no-op that would eat a
///   slot and make the draft look bigger than it is.
/// - **Cook**, only down to [`COOK_FLOOR`]. A town that builds itself a
///   beautiful house and stops cooking has gained nothing.
///
/// Returns at most `wanted` uids, and fewer when nothing else qualifies. Ties
/// inside a staffing level break on the trade's debug name — `WorkType` has no
/// `Ord`, and the string is the stable key this crate already uses for exactly
/// this purpose (`cap_haul_lane`, `neediest_lane`).
///
/// The staffing snapshot is read ONCE, not recomputed as the list drains:
/// "the biggest trades give" is a statement about the town as it stands at the
/// day boundary, and re-ranking mid-draft would make the answer depend on the
/// draft's own size.
pub fn pick_builders(
    candidates: &[(u64, common::bastion::WorkType, u32)],
    by_lane_named: &hashbrown::HashMap<common::bastion::WorkType, u32>,
    wanted: usize,
) -> Vec<u64> {
    use common::bastion::WorkType as W;
    if wanted == 0 {
        return Vec::new();
    }
    let mut ranked: Vec<&(u64, W, u32)> = candidates
        .iter()
        .filter(|(_, w, _)| !matches!(w, W::Haul | W::Guard | W::Build))
        .collect();
    ranked.sort_by(|a, b| {
        let (sa, sb) = (
            by_lane_named.get(&a.1).copied().unwrap_or(0),
            by_lane_named.get(&b.1).copied().unwrap_or(0),
        );
        sb.cmp(&sa)
            .then_with(|| format!("{:?}", a.1).cmp(&format!("{:?}", b.1)))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.0.cmp(&b.0))
    });
    let cooks = by_lane_named.get(&W::Cook).copied().unwrap_or(0) as usize;
    let mut cooks_taken = 0usize;
    let mut out = Vec::with_capacity(wanted);
    for (uid, w, _) in ranked {
        if out.len() >= wanted {
            break;
        }
        if *w == W::Cook {
            if cooks.saturating_sub(cooks_taken + 1) < COOK_FLOOR {
                continue;
            }
            cooks_taken += 1;
        }
        out.push(*uid);
    }
    out
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

    /// ★ G1c-d pinned: a plot cell is wanted until it matches its target,
    /// whatever the kind rule says of the block that is there now; a cell
    /// with no target keeps the kind rule. PLANTED DEFECT: `Some(_) =>
    /// wanted_by_kind` (the old rule for plot cells) -- the first assert
    /// goes red, and that defect is exactly the b1 churn.
    #[test]
    fn a_plot_cell_that_does_not_match_its_target_is_still_wanted() {
        // solid ground under a wood floor course: the kind rule says "filled,
        // not wanted"; the target says "not yet a floor" -- wanted.
        assert!(plot_job_still_wanted(Some(wood()), earth(), false), "earth under a floor target");
        assert!(plot_job_still_wanted(Some(rock()), air(), true), "air under a wall target");
        // the cell already IS its target: done, whatever the kind rule says
        assert!(!plot_job_still_wanted(Some(wood()), wood(), false), "matches: done");
        assert!(!plot_job_still_wanted(Some(bed()), bed(), true), "a placed bed is done even though it is air");
        // no target: the kind rule, unchanged
        assert!(plot_job_still_wanted(None, air(), true), "no target, kind says wanted");
        assert!(!plot_job_still_wanted(None, earth(), false), "no target, kind says not wanted");
    }

    /// ★ G1c-c, the SIZE of the draft — the number the b1 arm never
    /// computed, against the b1 arm's own figures.
    ///
    /// PLANTED DEFECT (the one this test exists to catch): drop the plan
    /// gate, i.e. delete the `if !plot_plan_open { return 0 }`. The FIRST
    /// assert goes red. And that defect is not hypothetical — it is the
    /// obvious simplification ("just scale with the open cells"), and it
    /// would keep a standing build crew forever off the colony's stray Build
    /// cells (ladders, beds, cook stations), which is a permanent farm tax
    /// for no house.
    #[test]
    fn builders_wanted_scales_with_the_open_cells_and_stops_without_a_plan() {
        // 1. NO PLAN, NO CREW — at every size of demand and of town.
        for cells in [0usize, 1, 100, 1_909, 100_000] {
            for roster in [0usize, 9, 49, 500] {
                assert_eq!(
                    builders_wanted(cells, roster, false),
                    0,
                    "no plot plan is open: {cells} open cells and a roster of {roster} \
                     must still staff nobody — the draft finishes a HOUSE, and the \
                     colony's stray Build cells are not one"
                );
            }
        }

        // 2. A SMALL HOUSE STILL GETS SOMEBODY. 100 / 150 = 0, and the
        //    clamp floor turns that into 1: a plan with a handful of cells
        //    left is still a plan.
        assert_eq!(
            builders_wanted(100, 49, true),
            1,
            "under one builder's worth of cells still deserves one builder"
        );

        // 3. THE b1 HOUSE, THE b1 TOWN. 1,909 / 150 = 12, trimmed to the
        //    lane cap; the roster term (49/6 = 8) is not the binding one.
        assert_eq!(
            builders_wanted(1_909, 49, true),
            BUILD_LANE_CAP,
            "the measured b1 plot on a roster of 49: the lane cap binds, not the town"
        );
        assert_eq!(BUILD_LANE_CAP, 6, "the cap this row was sized against");

        // 4. THE SAME HOUSE, A SMALL TOWN. 9/6 = 1: the TOWN binds now.
        //    This is the term that stops a hamlet emptying its fields into
        //    one building site.
        assert_eq!(
            builders_wanted(1_909, 9, true),
            1,
            "a town of nine spares one builder for the same house a town of 49 \
             staffs with six — the roster share binds, not the cell count"
        );

        // 5. NEVER ABOVE THE CAP, at any demand, on any town big enough for
        //    the roster term to stop binding.
        for cells in [0usize, 1, 149, 150, 1_909, 10_000, usize::MAX] {
            for roster in [0usize, 1, 5, 6, 9, 36, 49, 500] {
                let n = builders_wanted(cells, roster, true);
                assert!(
                    n <= BUILD_LANE_CAP,
                    "cells={cells} roster={roster} wanted {n} > cap {BUILD_LANE_CAP}"
                );
                assert!(
                    n <= roster / COLONISTS_PER_BUILDER,
                    "cells={cells} roster={roster} wanted {n} — more than the town's share"
                );
            }
        }
        // A colony too small to spare anyone spares no one.
        assert_eq!(builders_wanted(1_909, 5, true), 0, "five colonists spare nobody");

        println!(
            "builders_wanted: no_plan={} small_house={} b1_house_town49={} \
             b1_house_town9={} town5={} cap={BUILD_LANE_CAP} per_builder={BUILD_CELLS_PER_BUILDER}",
            builders_wanted(1_909, 49, false),
            builders_wanted(100, 49, true),
            builders_wanted(1_909, 49, true),
            builders_wanted(1_909, 9, true),
            builders_wanted(1_909, 5, true),
        );
    }

    /// ★ G1c-c, WHOSE trade pays for the house. The staffing question the
    /// hauler cap answers for its own lane and nothing answers for Build.
    ///
    /// PLANTED DEFECT: allow Haul (or take a cook below [`COOK_FLOOR`]).
    /// Both go red below. Neither is a strawman — "just take whoever has the
    /// lowest lane total" is the one-line version of this function, and on
    /// the roster here it drafts three haulers, which puts the draft into a
    /// tug-of-war with `reserve_haulers`/`cap_haul_lane` running in the same
    /// daily block: the floor re-promotes them the next day and the town
    /// spends its days renaming people instead of building.
    #[test]
    fn the_builders_come_from_the_biggest_trades_least_invested_first() {
        use common::bastion::WorkType as W;

        // A hand-built town: Farm 10, Cook 6, Mine 4, Haul 12, Guard 3.
        // Lane totals are chosen so the ANSWER IS NOT THE UID ORDER — the
        // four lowest-invested farmers are uids 4, 5, 1, 2 and the pair at
        // 40 exercises the uid tie-break.
        let by_lane_named: hashbrown::HashMap<W, u32> = [
            (W::Farm, 10u32),
            (W::Cook, 6),
            (W::Mine, 4),
            (W::Haul, 12),
            (W::Guard, 3),
        ]
        .into_iter()
        .collect();
        let mut candidates: Vec<(u64, W, u32)> = Vec::new();
        // Farmers: uids 1..=10. uid 4 = 10 (newest), uid 5 = 20,
        // uids 1 and 2 tie at 40, everyone else 100+.
        for (uid, total) in [
            (1u64, 40u32),
            (2, 40),
            (3, 100),
            (4, 10),
            (5, 20),
            (6, 110),
            (7, 120),
            (8, 130),
            (9, 140),
            (10, 150),
        ] {
            candidates.push((uid, W::Farm, total));
        }
        // Cooks 11..=16, ALL less invested than any farmer, so only the
        // "most-staffed trade first" term keeps them out of the answer.
        for uid in 11u64..=16 {
            candidates.push((uid, W::Cook, 1));
        }
        // Miners 17..=20, haulers 21..=32, guards 33..=35 — the haulers and
        // guards are the least invested in the whole town, which is exactly
        // the trap a pure lane_total sort falls into.
        for uid in 17u64..=20 {
            candidates.push((uid, W::Mine, 5));
        }
        for uid in 21u64..=32 {
            candidates.push((uid, W::Haul, 0));
        }
        for uid in 33u64..=35 {
            candidates.push((uid, W::Guard, 0));
        }
        // Two colonists already in the lane: never drafted twice.
        candidates.push((36, W::Build, 0));
        candidates.push((37, W::Build, 0));

        let picked = pick_builders(&candidates, &by_lane_named, 4);
        println!("pick_builders(wanted=4) -> {picked:?}");
        assert_eq!(
            picked,
            vec![4u64, 5, 1, 2],
            "the four builders come out of Farm (10 named, the biggest draftable \
             trade), least-invested first, uid ascending on the 40/40 tie — NOT out \
             of Haul or Guard, whose colonists have the lowest lane totals in town"
        );

        // The three forbidden lanes, stated separately so a regression names
        // itself rather than just shifting the vector above.
        let forbidden: Vec<u64> = (21..=35).chain(36..=37).collect();
        for uid in &forbidden {
            assert!(
                !picked.contains(uid),
                "uid {uid} is Haul, Guard or already Build and must never be drafted"
            );
        }

        // Asking for more than one trade can spare walks DOWN the staffing
        // order: Farm (10) exhausts, then Cook (6) — but only to the floor —
        // then Mine (4).
        let big = pick_builders(&candidates, &by_lane_named, 30);
        println!("pick_builders(wanted=30) -> {big:?}");
        assert_eq!(
            big.len(),
            10 + (6 - COOK_FLOOR) + 4,
            "every farmer, every cook above the floor, every miner — and nobody else"
        );
        assert_eq!(
            big.iter().filter(|u| (11..=16).contains(*u)).count(),
            6 - COOK_FLOOR,
            "the kitchen keeps {COOK_FLOOR} cooks no matter how big the house is"
        );
        assert!(
            big.iter().all(|u| !forbidden.contains(u)),
            "the forbidden lanes stay forbidden however large the draft: {big:?}"
        );

        // THE SECOND CALL the brief names: a thin town, Farm 2 / Cook 2. The
        // cooks are AT the floor, so no cook is draftable at all, and a
        // request for four comes back with two farmers.
        let thin_named: hashbrown::HashMap<W, u32> =
            [(W::Farm, 2u32), (W::Cook, 2)].into_iter().collect();
        let thin: Vec<(u64, W, u32)> = vec![
            (1, W::Farm, 90),
            (2, W::Farm, 80),
            (3, W::Cook, 1),
            (4, W::Cook, 2),
        ];
        let thin_picked = pick_builders(&thin, &thin_named, 4);
        println!("pick_builders(thin town, wanted=4) -> {thin_picked:?}");
        assert_eq!(
            thin_picked,
            vec![2u64, 1],
            "Farm 2 / Cook 2: both farmers (least invested first) and NO cook — a \
             town that builds a beautiful house and stops cooking has gained nothing"
        );
        assert!(
            !thin_picked.contains(&3) && !thin_picked.contains(&4),
            "no cook may be taken below {COOK_FLOOR}"
        );

        // Zero wanted is zero taken, and the function is a pure function of
        // its inputs (the same call twice is the same answer — the sort has
        // no HashMap-order term in it).
        assert!(pick_builders(&candidates, &by_lane_named, 0).is_empty());
        assert_eq!(
            pick_builders(&candidates, &by_lane_named, 7),
            pick_builders(&candidates, &by_lane_named, 7),
            "deterministic: the same roster drafts the same people every run"
        );
    }

    // ─── ★ G1d: THE GROWTH LOG ─────────────────────────────────────────────

    /// THE row's claim, as a pin: a replayed entry queues ONLY the cells the
    /// world does not already have — so a half-built house resumes with its
    /// remaining half and a finished one resumes with nothing.
    ///
    /// This is [`visible_cells`] read at the replay's angle rather than the
    /// fresh request's, and the reason it works is that `visible_cells` was
    /// always measured against the terrain as it stands NOW: nothing in it
    /// remembers when the plot was grown. The restart is just a very long gap
    /// between the grow and the measurement.
    ///
    /// PLANTED DEFECT (the one this test exists to catch): queue every cell
    /// regardless of terrain — i.e. the `kept.push` arm taken unconditionally,
    /// which is what a replay written as "re-queue the plan" instead of
    /// "re-measure the plot" does. It costs the colony a second full build of
    /// a house that is already standing: the town would spend a week laying
    /// blocks on top of identical blocks, and `skipped_unchanged` — the number
    /// that says so — would read 0.
    #[test]
    fn a_replayed_entry_queues_only_what_is_still_missing() {
        // A ten-block wall, as worldgen laid it out.
        let plot: Vec<(Vec3<i32>, Block)> =
            (0..10).map(|x| (Vec3::new(x, 0, 1), wood())).collect();

        // The server stopped after the colonists had placed the first four.
        // Those four are in the saved terrain; the rest is open air.
        for placed in [0usize, 4, 10] {
            let terrain_at = |p: Vec3<i32>| {
                Some(if (p.x as usize) < placed { wood() } else { air() })
            };
            let (kept, underground, unchanged) = visible_cells(&plot, terrain_at);
            println!(
                "replay with {placed} of {} already standing -> queued={} \
                 skipped_unchanged={unchanged} skipped_underground={underground}",
                plot.len(),
                kept.len(),
            );
            assert_eq!(
                kept.len(),
                plot.len() - placed,
                "the replay queues total - already-placed, and nothing else"
            );
            assert_eq!(
                unchanged, placed,
                "every block the world already holds is counted as finished, not rebuilt"
            );
            assert_eq!(underground, 0, "nothing here is underground");
            // And the cells it queued are exactly the missing ones, in the
            // plot's own order — a resumed house lays its courses in the same
            // direction it started them.
            assert_eq!(
                kept.iter().map(|(p, _)| p.x).collect::<Vec<_>>(),
                (placed as i32..10).collect::<Vec<_>>(),
                "the queued cells are the MISSING ones, in the plot's own order"
            );
        }
    }

    /// A finished house is re-grown but never re-registered.
    ///
    /// The plot must go back into the site (or the next house is laid through
    /// its wall — the world index is regenerated from the seed and does not
    /// remember), and the household census must NOT be told about it twice
    /// (the terrain scan re-adopts it in place; a bed is a one-shot resource
    /// nothing re-adds, so a double registration is a town that counts one
    /// house as two and opens its immigration gate on beds nobody has).
    ///
    /// PLANTED DEFECT: `replay_decision` returning `ReGrowAndQueue` for every
    /// entry — the version that treats the log as a list of houses to build
    /// rather than a list of orders already given. The `queues()` assert below
    /// turns red for it.
    #[test]
    fn a_registered_entry_is_not_re_registered() {
        use common::bastion::{BastionGrownPlotV1 as Entry, BastionLayoutKindV1 as K};

        let finished = Entry { kind: K::House, seed: 0xABCD_1234, day: 3, registered: true };
        let half_built = Entry { registered: false, ..finished.clone() };

        let d_finished = replay_decision(&finished);
        let d_half = replay_decision(&half_built);
        println!("registered=true -> {d_finished:?}\nregistered=false -> {d_half:?}");

        // The SKIP half: a registered entry queues nothing and registers
        // nothing.
        assert!(
            !d_finished.queues(),
            "a house already in the census must not be queued or registered again"
        );
        assert_eq!(d_finished, ReplayDecision::ReGrowOnly {
            kind: LayoutKind::House,
            seed: 0xABCD_1234,
        });

        // The half nobody remembers to keep: it is still RE-GROWN. Same kind,
        // same seed, so the plot lands on the same tiles it held before the
        // restart and the site knows they are taken.
        assert_eq!(
            (d_finished.kind(), d_finished.seed()),
            (LayoutKind::House, 0xABCD_1234),
            "a finished house is still re-grown — the site must know its tiles are taken"
        );

        // And the unfinished one does queue.
        assert!(d_half.queues(), "a half-built house must be resumed");
        assert_eq!(d_half, ReplayDecision::ReGrowAndQueue {
            kind: LayoutKind::House,
            seed: 0xABCD_1234,
        });

        // The kind round-trips through the wire mirror unchanged, in both
        // directions and for every variant — the seam where a save could
        // decode into the wrong building.
        for (wire, runtime) in [
            (K::House, LayoutKind::House),
            (K::FarmField, LayoutKind::FarmField),
            (K::Workshop, LayoutKind::Workshop),
        ] {
            assert_eq!(runtime_kind(wire), runtime);
            assert_eq!(wire_kind(runtime), wire);
        }
    }

    /// A save written before the growth log existed still loads, with an empty
    /// log — the identity fallback: an empty log replays nothing and changes
    /// nothing.
    ///
    /// ★ DEVIATION, STATED: this pins the property on a two-field stand-in
    /// rather than on rtsim's `Data` itself, because `Data` has no `Default`
    /// and cannot be constructed in a unit test without generating a world
    /// (`nature: Nature::generate(world)`). What is faithful is everything
    /// that decides the outcome:
    ///
    /// - the same CODEC and the same call: `rmp_serde::encode::write_named` is
    ///   what `Data::write_to` uses, and `rmp_serde::decode::from_read` is
    ///   what `Data::from_reader` uses. `write_named` matters — the compact
    ///   rmp encoding writes structs as ARRAYS, under which a `#[serde(
    ///   default)]` field added anywhere but the end would silently shift
    ///   every field after it. The named encoding is why the sibling pattern
    ///   is safe at all, so the pin uses it.
    /// - the same attribute (`#[serde(default)]`) on the same shape of field
    ///   (a `Vec` of the real `BastionGrownPlotV1`).
    ///
    /// PLANTED DEFECT: drop the `#[serde(default)]`. The decode then fails
    /// with a missing-field error, which live is a save this build cannot read
    /// at all — every colony on an older save, gone.
    #[test]
    fn the_log_survives_an_old_save() {
        use common::bastion::{BastionGrownPlotV1 as Entry, BastionLayoutKindV1 as K};
        use serde::{Deserialize, Serialize};

        /// The save as an older build wrote it: no growth log at all.
        #[derive(Serialize)]
        struct OldSave {
            version: u32,
            tick: u64,
        }

        /// The save as this build reads it. Same sibling pattern as the field
        /// on rtsim's `Data`.
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct NewSave {
            version: u32,
            #[serde(default)]
            bastion_growth_log: Vec<Entry>,
            tick: u64,
        }

        let mut old = Vec::new();
        rmp_serde::encode::write_named(&mut old, &OldSave { version: 10, tick: 7 })
            .expect("the old save encodes");
        let loaded: NewSave =
            rmp_serde::decode::from_read(&old[..]).expect("an old save must still load");
        println!("old save ({} bytes) loaded as {loaded:?}", old.len());
        assert_eq!(loaded.version, 10, "the fields that were there are unchanged");
        assert_eq!(loaded.tick, 7);
        assert!(
            loaded.bastion_growth_log.is_empty(),
            "a save with no growth log loads with an EMPTY one — the identity fallback"
        );

        // And a log that IS there round-trips through the same codec, so the
        // fallback is a fallback and not the only thing that works.
        let written = NewSave {
            version: 10,
            bastion_growth_log: vec![
                Entry { kind: K::House, seed: 11, day: 2, registered: true },
                Entry { kind: K::Workshop, seed: 12, day: 5, registered: false },
            ],
            tick: 7,
        };
        let mut bytes = Vec::new();
        rmp_serde::encode::write_named(&mut bytes, &written).expect("the new save encodes");
        let read: NewSave =
            rmp_serde::decode::from_read(&bytes[..]).expect("the new save decodes");
        println!("round-trip: {read:?}");
        assert_eq!(read, written, "the log round-trips in order, registered flags and all");
    }
}
