pub mod event;
pub mod rule;
pub mod tick;

use atomicwrites::{AtomicFile, OverwriteBehavior};
use common::{
    grid::Grid,
    mounting::VolumePos,
    rtsim::{Actor, NpcId, RtSimEntity, TerrainResource, WorldSettings},
    terrain::{CoordinateConversions, SpriteKind},
};
use common_ecs::{System, dispatch};
use common_state::BlockDiff;
use crossbeam_channel::{Receiver, Sender, unbounded};
use enum_map::EnumMap;
use rtsim::{
    RtState,
    data::{Data, ReadError, npc::SimulationMode},
    event::{OnDeath, OnHealthChange, OnHelped, OnMountVolume, OnSetup, OnTheft},
};
use specs::DispatcherBuilder;
use std::{
    fs::{self, File},
    io,
    path::PathBuf,
    thread::{self, JoinHandle},
    time::Instant,
};
use tracing::{debug, error, info, trace, warn};
use vek::*;
use world::{IndexRef, World};

/// Translate DETRNG's boot-time flag into the server/common-state execution
/// policy. Keeping this adapter here avoids a dependency from common-state
/// back into rtsim while making the live default explicit.
pub(crate) fn execution_mode() -> common_state::ExecutionMode {
    if ::rtsim::deterministic_rtsim_enabled() {
        common_state::ExecutionMode::DeterministicSerial
    } else {
        common_state::ExecutionMode::Parallel
    }
}

/// ★ A PIN THAT REACHES ONE OF FOUR DOORS PINS NOTHING (adversarial review,
/// ROW 51). `BASTION_PIN_TRAIT` was read at exactly ONE of the four sites that
/// give a colonist a personality — the founding loop in
/// [`RtSim::bastion_spawn_colony_seeded`]. The other three (the ADOPT-A-TOWN
/// *settle* branch, the ROW 38 settler drain and the ROW 50 birth drain) each
/// called `Personality::random` directly. That is fatal in the specific way an
/// instrument defect is fatal: the settle branch is the path EVERY play world
/// takes (`adopted_existing=0 settled=24` on world 108), so an experiment run
/// with `BASTION_PIN_TRAIT=Conscientious` on a play world got a fully random
/// colony AND a silent success — the loud-refusal panic below never fired,
/// because the only code that could raise it was never reached. A pinned run
/// and an unpinned one were indistinguishable in every log line.
///
/// So the pin lives HERE, once, and every colonist-minting site calls it.
///
/// SPLIT ON PURPOSE: [`bastion_pin_trait`] does the env read and the loud
/// refusal; this function is pure, so a test can drive both arms without
/// `set_var` (unsafe in edition 2024, and process-global besides).
///
/// FALLBACK IS IDENTITY, not a copy: with no pin this returns exactly
/// `Personality::random(rng)` — the same one draw off the same stream the four
/// call sites already made — so switching a site onto this helper cannot shift
/// a single existing world's colonists.
///
/// ★ WHY `random()` AND NOT `random_good()`, stated correctly this time. The
/// old comment claimed "vanilla gives town residents `random()` too", citing
/// `rtsim/src/rule/architect.rs::role_personality`. That citation does not say
/// what it was claimed to say: `role_personality` is profession-DEPENDENT, and
/// it hands `random_good` to Guard, Merchant and Captain. Two of the six
/// professions the settle branch mints are Guard and Merchant, so a third of a
/// settled village is NOT what the architect would have placed. The real and
/// sufficient reason to use `random()` everywhere is the measured one:
/// `random_good` clamps conscientiousness to [LOW_THRESHOLD, MAX], which makes
/// ~83% of the colony Conscientious and collapses the very trait spread the
/// guard row and the evening palette select from. That is a deliberate
/// divergence from vanilla for the colonist band, not parity with it.
pub(crate) fn bastion_colonist_personality(
    pin: Option<common::rtsim::PersonalityTrait>,
    rng: &mut impl rand::RngExt,
) -> common::rtsim::Personality {
    match pin {
        Some(t) => common::rtsim::Personality::pinned(t),
        None => common::rtsim::Personality::random(rng),
    }
}

/// The env half of [`bastion_colonist_personality`]: `BASTION_PIN_TRAIT`
/// (banked item 4 / #110, 2026-08-19) pins every colonist to ONE named trait
/// so an instrument aimed at a trait-carrying population has a subject.
///
/// ★ An UNRECOGNISED name REFUSES loudly rather than falling back to random. A
/// silent fallback would produce an unpinned run that is indistinguishable
/// from a pinned one in every log line — the exact shape of null that costs a
/// whole fan to diagnose.
pub(crate) fn bastion_pin_trait() -> Option<common::rtsim::PersonalityTrait> {
    use common::rtsim::PersonalityTrait as T;
    let name = std::env::var("BASTION_PIN_TRAIT").ok()?;
    let t = match name.as_str() {
        "Open" => T::Open,
        "Adventurous" => T::Adventurous,
        "Closed" => T::Closed,
        "Conscientious" => T::Conscientious,
        "Busybody" => T::Busybody,
        "Unconscientious" => T::Unconscientious,
        "Extroverted" => T::Extroverted,
        "Introverted" => T::Introverted,
        "Agreeable" => T::Agreeable,
        "Sociable" => T::Sociable,
        "Disagreeable" => T::Disagreeable,
        "Neurotic" => T::Neurotic,
        "Seeker" => T::Seeker,
        "Worried" => T::Worried,
        "SadLoner" => T::SadLoner,
        "Stable" => T::Stable,
        other => panic!(
            "BASTION_PIN_TRAIT={other:?} is not a PersonalityTrait. Valid:                              Open Adventurous Closed Conscientious Busybody Unconscientious                              Extroverted Introverted Agreeable Sociable Disagreeable Neurotic                              Seeker Worried SadLoner Stable. (NB: \"reckless\" is NOT one of                              them -- the only Reckless in the tree is BuffKind::Reckless, a                              different system.)"
        ),
    };
    info!(trait_ = %name, "bastion: colonist personality PINNED (BASTION_PIN_TRAIT)");
    Some(t)
}

/// ★ THE WITNESS ROW 51 SHIPPED WITHOUT. The row's whole claim is "24 of 24
/// colonists had `traits=[]`, and now they do not", and nothing in the tree
/// counted trait-carrying colonists — so the regression it repaired was
/// exactly as invisible after the fix as before it, in a field the inspector
/// faithfully reported as empty. This is the predicate behind that count.
///
/// It must DISCRIMINATE or it is blind: a default `Personality` has every axis
/// at MID, which is below every HIGH_THRESHOLD trait and above every
/// LOW_THRESHOLD one, so it carries nothing — that is precisely the bug state
/// ROW 51 found, and the pinned/random states must read differently from it.
/// Pinned in the test below against both degenerate answers.
pub(crate) fn bastion_carries_a_trait(personality: &common::rtsim::Personality) -> bool {
    use strum::IntoEnumIterator as _;
    common::rtsim::PersonalityTrait::iter().any(|t| personality.is(t))
}

/// How many rtsim ticks the demote witness's population census may be reused
/// before it is recomputed.
///
/// ★ THE GATE WAS ON THE WRONG PREDICATE (2026-08-29, second pass). Commit
/// fdbf72acea gated the all-npc census scan on `is_colonist`, on the
/// assumption that the expensive branch was rare. It is not rare — it is
/// UNIVERSAL in the population that reaches it. Every one of the ~38
/// demotions per second measured on the owner's world was a colonist, so the
/// gate was true every time and a linear scan of ~8,528 npc records (~7.5 MB)
/// ran ~38 times a second — roughly 324,000 record touches per second — to
/// decorate three fields of a log line. A GUARD MUST REFUSE BEFORE IT SPENDS,
/// and this one was refusing nothing.
///
/// The right gate is on the CLOCK, not on the caller: the census answers a
/// conservation question ("did the colony shrink?"), and a conservation
/// question does not need re-answering eight times inside one tick. 64 ticks
/// is ~2 s at the nominal 30 ticks/s, which is far finer than the interval at
/// which a colonist can plausibly die unnoticed, and it turns the cost from
/// O(npcs) per EVENT into O(npcs) per AUDIT — an ~490x reduction at the
/// measured event rate. The line carries `census_age_ticks` so the number
/// still names its own producer.
pub(crate) const BASTION_CENSUS_AUDIT_TICKS: u64 = 64;

/// Should the cached colonist census be recomputed at `now`?
///
/// Pure so the cost bound can be pinned without standing up an `RtSim` (see
/// `bastion_demote_witness_pins`). `None` — never computed — always recomputes;
/// that is the identity case, and it is why the first demotion after boot is
/// still exact.
pub(crate) fn bastion_census_needs_recompute(taken_at: Option<u64>, now: u64) -> bool {
    match taken_at {
        None => true,
        Some(t) => now.saturating_sub(t) >= BASTION_CENSUS_AUDIT_TICKS,
    }
}

/// A colonist demoted again within this many rtsim ticks of its own last
/// demotion is RINGING, not travelling: nobody walks out of the loaded island,
/// back in and out again inside four ticks.
pub(crate) const BASTION_RING_TICKS: u64 = 4;

/// Is this demotion a ring rather than a departure? O(1), and it is the field
/// that would have named the oscillation on its FIRST line instead of after
/// clustering two million of them.
pub(crate) fn bastion_demote_is_ringing(previous_demote_tick: Option<u64>, now: u64) -> bool {
    match previous_demote_tick {
        None => false,
        Some(t) => now.saturating_sub(t) <= BASTION_RING_TICKS,
    }
}

// ── ROW 54: THE HOUSING CAP MUST BIND ADOPTED RESIDENTS TOO ──────────────
//
// ★ THE DEFECT, MEASURED IN THE OWNER'S LIVE SESSION (2026-08-30). He
// adopted a real worldgen village and his log printed, in this order:
//
//   ADOPT-NPCS      site_population=46 eligible=46
//   ADOPT-A-TOWN    roll adopted_existing=46 settled=0 houses=10 wanted=8
//   ADOPT-IN-PLACE  house registered x10, adopted_beds=2 each  ->  beds=20
//   HOUSING GROWTH  fired=false deciding="drive_not_expand" ... drive=Grow
//   BIRTHS          fired=false deciding="drive_not_expand"
//   COURTSHIP       fired=false deciding="no_home" candidates=20
//   Build jobs created: 0
//
// 46 colonists into 20 bed slots. `Server::bastion_adopt_town_people` HAD
// computed the cap (`wanted_eff = min(8, 10) = 8`) and HAD logged
// "ADOPT-A-TOWN population CAPPED BY HOUSING — one colonist per house (Ben's
// ruling)" — but `wanted_eff` reached only `common::bastion::settle_plan`,
// which returns EMPTY the moment `adopted_existing >= wanted`. The conversion
// loop at the bottom of [`RtSim::bastion_adopt_town_npcs`] then wrote
// `npc.bastion_colonist = Some(colonist)` onto every eligible resident
// UNCONDITIONALLY. So the cap bound only the colonists the game MANUFACTURES
// and was ignored for the entire population of an adopted town — while
// printing a line saying it had been applied. A COMMENT CANNOT ENFORCE, and
// neither can a log line.
//
// ★ THE COST IS THREE SHIPPED SYSTEMS, EACH NAMING ITS OWN BLOCKER IN THAT
// LOG. `bastion_server::bastion_jobs::colony_drive_for` returns Grow while
// `beds < pop`, so 46 > 20 pins the drive at Grow forever; `immigration_verdict`
// and `birth_verdict` both refuse `drive_not_expand` off exactly that; and
// `courtship_verdict` refuses `no_home` because all 20 slots are owned, so
// `free_bed_in_house` can never resolve. Nothing can add a bed either:
// `JobBoard::adopt_beds_surface` mints ZERO build jobs by construction (the
// houses already exist), which is the `Build jobs created: 0` line. A
// PERMANENT SIGNAL BLOCKING ITS OWN CURE.

/// ★ THE DENOMINATOR IS HOUSE PLOTS, with the env count surviving only as an
/// upper bound — `min(wanted, houses.max(1))`. Ben's ruling, already quoted
/// in the line this fix makes true, is *"one colonist per house"*.
///
/// REJECTED — registered bed SLOTS (20 on the owner's town), for three
/// independent reasons, any one of which is fatal:
///  1. **It does not exist yet.** Slots are registered by
///     `JobBoard::adopt_beds_surface` / `adopt_furniture_surface`, which take
///     a `&TerrainGrid` and run inside the job-board tick system — strictly
///     AFTER this call, which is why the owner's log prints ADOPT-IN-PLACE
///     below ADOPT-A-TOWN. At founding tick the town has no loaded chunks at
///     all; that is the whole reason the caller reads `get_alt_approx`
///     instead of terrain. Adoption cannot count a bed it cannot see.
///  2. **It is the wrong number even if handed to us.** The growth ceiling
///     `immigration_target_pop` counts HOUSES that hold beds, not slots — 10,
///     not 20. Wherever the slot denominator actually binds it puts `roster`
///     at twice that ceiling, so `birth_verdict` refuses `no_room_in_town`
///     and `immigration_verdict` refuses `roster_at_target`; every house is
///     full, so courtship's `free_bed_in_house` cannot resolve either. The
///     drive would leave Grow and the other two witnesses would STILL be
///     dead. MEASURE THE OUTCOME, NOT THE RESPONSE.
///
///     ★ HONESTLY: on the owner's OWN town this reason is inert, and that was
///     found by falsification rather than review — planting the slot
///     denominator left the reachability pin green. His env `wanted` is 8 and
///     his town has 10 houses, so `min(8, 10)` and `min(8, 20)` are both 8
///     and the two denominators are indistinguishable at his numbers. They
///     separate only when the housing binds below the env count. The pin now
///     tests them there; see `a_town_with_fewer_houses_than_the_env_count_
///     starts_at_its_ceiling` for what that boundary costs.
///  3. **The number is a documented artefact.** `PainterSpriteExt::bed`'s
///     `with_corner_sprite_side` sets BOTH corners of the head side, so one
///     physical bed registers as TWO slots — measured and deliberately left
///     alone in `is_adoptable_bed_sprite`'s own doc block. A denominator
///     built on a count its own producer records as doubled is a denominator
///     waiting to halve under someone else's row.
///
/// REJECTED — `immigration_target_pop` (10). Same availability problem (it
/// reads `derive_households` over board regions the post-adoption terrain
/// sweep populates), and it is the town's growth CEILING, not its starting
/// population. Capping the start AT the ceiling makes `roster >= target_pop`
/// true on tick one: births refuse `no_room_in_town`, housing growth refuses
/// `roster_at_target`. The gates would be reachable and would never fire. A
/// population loop needs somewhere to go.
///
/// CHOSEN — house plots. Available (it is the argument this path already
/// carries, resolved from worldgen plot data, terrain-independent by
/// construction); it is literally the ruling; and it is REACHABLE on the
/// owner's own numbers: 10 houses, env wanted 8 → cap 8; beds 20 ≥ pop 8 so
/// the drive reaches Expand; roster 8 < target_pop 10 so the two roster arms
/// clear; and 8 people in 10 houses leaves 2 houses empty with 4 unowned
/// slots, so courtship's `free_bed_in_house` resolves. All three witnesses
/// can fire — pinned in `the_three_witnesses_become_reachable`, against the
/// shipped `colony_drive_for`.
///
/// THE ONE PRODUCER of that cap. `Server::bastion_adopt_town_people` applies
/// it to the house positions it resolved; [`RtSim::bastion_adopt_town_npcs`]
/// re-applies it to the same slice it is handed. That is a RE-APPLICATION,
/// not a second producer: the function is idempotent —
/// `cap(cap(w, h), h) == cap(w, h)`, pinned — so the two can never disagree,
/// and the rtsim entry point (which is `pub`) defends itself against a future
/// caller that passes an uncapped `wanted`.
///
/// The `.max(1)` floor is carried over UNCHANGED from the shipped expression:
/// a colony of zero is not a colony, and retuning that floor is the DECISIONS
/// log's row, not this one.
pub(crate) fn bastion_housing_cap(wanted: usize, houses: usize) -> usize {
    // ★ BEN'S RULING, 2026-09-01, superseding "one colonist per house":
    // "we can have more than 1 colonist per house — families, friends, farm
    // hands." The unit is the BED. Beds are not registered at founding
    // (`adopt_beds_surface` needs loaded terrain), so the honest estimate is
    // the plot count times worldgen's bed count per house, which every
    // logged adoption reports as `adopted_beds=2`. FALLBACK IS IDENTITY at
    // BEDS_PER_HOUSE_AT_FOUNDING == 1.
    //
    // His flat-map village: 83 residents, 38 houses, 76 beds. The old cap
    // allowed 38 (and the harness asked for 8); this allows 76.
    //
    // The zero-house floor stays exactly ONE: "a colony of zero is not a
    // colony" was always a floor of one settler, and multiplying it by the
    // bed count would have made it two — the idempotence pin caught that.
    if houses == 0 {
        wanted.min(1)
    } else {
        wanted.min(houses * BEDS_PER_HOUSE_AT_FOUNDING)
    }
}

/// Worldgen beds per house plot, as every logged `ADOPT-IN-PLACE` reports
/// (`adopted_beds=2 each`). The founding-time stand-in for the registered
/// bed count, which does not exist until the town's chunks load.
pub(crate) const BEDS_PER_HOUSE_AT_FOUNDING: usize = 2;

/// How far a resident is from the nearest house of the town being adopted, as
/// an INTEGER squared block distance. `i64::MAX` when the town resolved no
/// houses at all, so a house-less town ranks everyone equally and the NpcId
/// tiebreak alone decides — a total order either way.
///
/// Integer by construction, using the same `.map(|e| e as i64)` idiom the two
/// site lookups in this file already use: a float comparator would put a
/// NaN-ordering hazard on the path that chooses who lives in the town.
pub(crate) fn bastion_adoption_rank(wpos: Vec3<f32>, house_plots: &[Vec3<f32>]) -> i64 {
    house_plots
        .iter()
        .map(|h| {
            h.xy()
                .map(|e| e as i64)
                .distance_squared(wpos.xy().map(|e| e as i64))
        })
        .min()
        .unwrap_or(i64::MAX)
}

/// ★ WHO GETS ADOPTED when a town has more residents than houses.
///
/// BED OWNERSHIP IS NOT AVAILABLE HERE, and that is worth stating plainly
/// because it is the rule one reaches for first: `BedSlot.owner` lives on the
/// job board's bed table, which does not exist until the post-adoption
/// terrain sweep builds it. At adoption tick NOBODY owns a bed, because there
/// are no beds yet — the same reason the bed-slot denominator was rejected
/// above.
///
/// The nearest signal that does exist is where the person is standing.
/// `Site.population` is site MEMBERSHIP, not proximity — it holds villagers
/// out on the roads — so ranking by distance to the closest house plot adopts
/// the people actually in the village and leaves the wanderers to the vanilla
/// rules. It also agrees with the bed assigner, which scores by distance: the
/// people adopted are the ones already standing next to the beds.
///
/// ★ THE KEPT SET IS RE-SORTED BY NpcId BEFORE IT IS RETURNED, and that is
/// not tidiness — it is FALLBACK IS IDENTITY. The caller draws one
/// `BastionColonist::generate` per entry from a seeded stream, so the ORDER
/// decides which record each person receives. Rank order SELECTS; NpcId order
/// CONVERTS. That makes the uncapped case (`cap >= ranked.len()`) byte-for-
/// byte the `residents.sort()` that shipped — an adopted town already at or
/// under its housing is bit-for-bit unchanged, pinned.
///
/// DETERMINISM BY CONSTRUCTION: integer rank, NpcId total-order tiebreak, one
/// explicit sort. No wall clock, no RNG, and nothing here can see a HashMap.
pub(crate) fn bastion_adoption_pick<T: Copy + Ord>(
    mut ranked: Vec<(i64, T)>,
    cap: usize,
) -> Vec<T> {
    ranked.sort_by_key(|(rank, id)| (*rank, *id));
    ranked.truncate(cap);
    let mut kept: Vec<T> = ranked.into_iter().map(|(_, id)| id).collect();
    kept.sort();
    kept
}

/// A MOVER'S REFUSAL MUST NOT REARM ITS RIVAL: the settle branch's want is the
/// CAP, never the raw `wanted`.
///
/// [`RtSim::bastion_adopt_town_npcs`] now caps the population BEFORE asking
/// `common::bastion::settle_plan` for the shortfall. If that second question
/// were still measured against an uncapped want, the settle branch would
/// manufacture back exactly the people the cap had just declined to adopt —
/// 36 strangers on the owner's town — and the fix would have moved the
/// overcrowding from one producer to the other while every count still looked
/// right.
///
/// ★ THIS FUNCTION EXISTS BECAUSE THE PIN THAT WAS MEANT TO GUARD THAT RULE
/// COULD NOT. Written first as `settle_plan(adopted_existing, cap, ..)` at the
/// call site, the rule lived in a chosen ARGUMENT, and an argument is not
/// something a pure pin can reach: the test could only call `settle_plan`
/// itself with hand-passed values, so it asserted a property of `settle_plan`
/// (which this row may not even edit) and would have stayed GREEN through any
/// mutation of the line it claimed to guard. Deriving the cap INSIDE the
/// function moves the rule from an argument into code, which is the only form
/// a pin can break. Pinned by `the_cap_does_not_rearm_the_settle_branch`.
///
/// Idempotent in the live path (`wanted` arrives already capped), so this is
/// the identity there — the re-application exists for the `pub` entry point's
/// self-defence, exactly as in [`bastion_housing_cap`].
pub(crate) fn bastion_settle_plan_capped(
    adopted: usize,
    wanted: usize,
    houses: usize,
) -> Vec<usize> {
    common::bastion::settle_plan(adopted, bastion_housing_cap(wanted, houses), houses)
}

/// ★★ THE CAP IS ENFORCED BY THE TYPE, BECAUSE THE PIN COULD NOT.
///
/// FOUND BY FALSIFICATION, and it is the more useful half of this row.
/// `the_cap_binds_the_towns_own_residents` stayed GREEN when the cap
/// application was deleted from [`RtSim::bastion_adopt_town_npcs`] and the
/// shipped uncapped `residents.sort()` put back in its place — a VALID cycle,
/// with `Compiling veloren-server` and a 22.59s rebuild in the log, so not the
/// stale-binary trap. It had to be green: the pin calls
/// [`bastion_adoption_pick`] itself, so it can assert what that helper DOES
/// and never that the caller CALLED it. No pure pin can witness a call site.
///
/// That blindness is the exact shape of the defect this row exists to fix:
/// `wanted_eff` was computed correctly, logged correctly, and simply not
/// applied. Re-asserting the rule in a stronger sentence would repeat the
/// mistake — A COMMENT CANNOT ENFORCE — so the TYPE carries it instead.
///
/// [`Eligible`]'s field is private and this module is a CHILD of `rtsim`, so
/// the parent cannot reach inside it: `eligible.0` and `eligible.into_iter()`
/// both fail to compile. The ONLY way the adoption path can turn its census
/// into a roster is [`Eligible::into_capped`], which applies the cap. The
/// mutation the pin missed is now a compile error rather than a silent
/// regression, and [`Roster`] hands the witness line the same numbers the
/// decision used — A NUMBER CARRIES ITS PRODUCER.
pub(crate) mod adoption {
    /// What the cap decided, from the SAME call that decided it.
    pub(crate) struct Roster {
        pub(crate) population_seen: usize,
        pub(crate) cap: usize,
        pub(crate) adopted: usize,
        pub(crate) left_as_npcs: usize,
    }

    /// The eligible residents of a town being adopted, each paired with its
    /// [`super::bastion_adoption_rank`]. PRIVATE FIELD BY DESIGN — see the
    /// module doc block above.
    pub(crate) struct Eligible<T>(Vec<(i64, T)>);

    impl<T: Copy + Ord> Eligible<T> {
        pub(crate) fn empty() -> Self { Self(Vec::new()) }

        pub(crate) fn push(&mut self, rank: i64, id: T) { self.0.push((rank, id)); }

        /// THE ONLY EXIT, and it caps.
        pub(crate) fn into_capped(self, wanted: usize, houses: usize) -> (Vec<T>, Roster) {
            let cap = super::bastion_housing_cap(wanted, houses);
            let population_seen = self.0.len();
            let kept = super::bastion_adoption_pick(self.0, cap);
            let adopted = kept.len();
            (kept, Roster {
                population_seen,
                cap,
                adopted,
                left_as_npcs: population_seen - adopted,
            })
        }
    }
}

/// The bounded state behind the demote witness in
/// [`RtSim::hook_rtsim_entity_unload`].
///
/// EVERY FIELD IS O(1) TO UPDATE. `demotes` is point-queried by npc id and
/// never iterated, so its order cannot decide anything; it holds at most one
/// entry per npc that has ever been demoted (~200 KB at this world's 8,528
/// npcs) and is deliberately not persisted — a restart's counts start at zero
/// because a restart's oscillation is a new observation.
#[derive(Default)]
pub(crate) struct BastionDemoteWitness {
    /// npc -> (how many times it has been demoted, the tick of the last one).
    demotes: std::collections::HashMap<NpcId, (u64, u64)>,
    /// The cached (loaded, simulated) colonist counts and the `Data.tick` they
    /// were taken at.
    census: (u32, u32),
    census_tick: Option<u64>,
}

pub struct RtSim {
    file_path: PathBuf,
    world_seed: u32,
    last_saved: Option<Instant>,
    state: RtState,
    save_thread: Option<(Sender<Data>, JoinHandle<()>)>,
    /// The demote witness's own bounded state -- see
    /// [`BastionDemoteWitness`] and `hook_rtsim_entity_unload`.
    bastion_demote_witness: BastionDemoteWitness,
    // `APEX-T4.6` chunk 3b: the staged multi-store epoch commit's own
    // state, separate from the pre-existing rtsim-file save machinery
    // above (which this row does not replace, only supplements).
    save_universe_layout: crate::save_universe::SaveUniverseLayoutV1,
    save_epoch_ledger: common::apex::save_universe::SaveEpochLedgerV1,
}

impl RtSim {
    pub fn new(
        settings: &WorldSettings,
        world_seed: u32,
        index: IndexRef,
        world: &World,
        data_dir: PathBuf,
        // `APEX-T4.3` chunk 2: the caller (`server/src/lib.rs`) already
        // builds `WorldMapMsg` via `World::get_map_data` for the real
        // bootstrap send -- passed in rather than re-derived here, one
        // computation, two consumers. See
        // `common_net::msg::world_msg::world_map_geometry_root_v1`.
        map_geometry_root: common::apex::digest::ArtifactIdentityV1,
        // `APEX-T4-PV`: derived by the caller from the ACTUAL WorldOpts
        // this server generated with -- same one-computation-two-consumers
        // reason as `map_geometry_root`. `None` only when the derivation
        // itself failed, which is recorded as absent rather than faked.
        worldgen_protocol_root: Option<
            common::apex::subsystem::descriptor::WorldgenProtocolVersion,
        >,
        // `APEX-T4.1-CONTENT-LIVE`: derived by the caller from a REAL,
        // once-at-boot asset-tree walk (`common::content_manifest::
        // build_from_asset_tree_v1`) -- same one-computation-two-
        // consumers reason as `map_geometry_root`/`worldgen_protocol_root`
        // (the other consumer being `bootstrap_manifest_v1`'s own Content
        // descriptor). `None` only when the walk itself failed, recorded
        // as absent rather than faked.
        content_protocol_root: Option<
            common::apex::subsystem::descriptor::ContentProtocolVersion,
        >,
    ) -> Result<Self, ron::Error> {
        // `APEX-T4.6` chunk 3a: `get_file_path` consumes `data_dir` below
        // (it may push "rtsim" onto it), so the save-universe layout
        // root -- a SIBLING of `rtsim/`, not nested under it -- is
        // derived first, borrowing rather than needing its own clone.
        let save_universe_layout = crate::save_universe::SaveUniverseLayoutV1::new(data_dir.join("save_universe"));
        let file_path = Self::get_file_path(data_dir);

        info!("Looking for rtsim data at {}...", file_path.display());
        let mut data = 'load: {
            if std::env::var("RTSIM_NOLOAD").map_or(true, |v| v != "1") {
                match File::open(&file_path) {
                    Ok(file) => {
                        info!("Rtsim data found. Attempting to load...");

                        let ignore_version = std::env::var("RTSIM_IGNORE_VERSION").is_ok();
                        // `APEX-T4.5-FIXTURES`: the exact decision, extracted
                        // so the offline-recovery proof calls the real
                        // function rather than a duplicate of this guard.
                        let load_unmigrated = matches!(
                            crate::save_migration::rtsim_version_mismatch_disposition_v1(ignore_version),
                            crate::save_migration::RtsimVersionMismatchDispositionV1::LoadUnmigrated
                        );

                        match Data::from_reader(io::BufReader::new(file)) {
                            Err(ReadError::VersionMismatch(_)) if !load_unmigrated => {
                                warn!(
                                    "Rtsim data version mismatch (implying a breaking change), \
                                     rtsim data will be purged"
                                );
                            },
                            Ok(data) | Err(ReadError::VersionMismatch(data)) => {
                                info!("Rtsim data loaded.");
                                if data.should_purge {
                                    warn!(
                                        "The should_purge flag was set on the rtsim data, \
                                         generating afresh"
                                    );
                                } else {
                                    break 'load *data;
                                }
                            },
                            Err(ReadError::Load(err)) => {
                                error!("Rtsim data failed to load: {}", err);
                                info!("Old rtsim data will now be moved to a backup file");
                                let mut i = 0;
                                loop {
                                    let mut backup_path = file_path.clone();
                                    backup_path.set_extension(if i == 0 {
                                        "ron_backup".to_string()
                                    } else {
                                        format!("ron_backup_{}", i)
                                    });
                                    if !backup_path.exists() {
                                        fs::rename(&file_path, &backup_path)?;
                                        warn!(
                                            "Failed rtsim data was moved to {}",
                                            backup_path.display()
                                        );
                                        info!("A fresh rtsim data will now be generated.");
                                        break;
                                    } else {
                                        info!(
                                            "Backup file {} already exists, trying another name...",
                                            backup_path.display()
                                        );
                                    }
                                    i += 1;
                                }
                            },
                        }
                    },
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        info!("No rtsim data found. Generating from world...")
                    },
                    Err(e) => return Err(e.into()),
                }
            } else {
                warn!(
                    "'RTSIM_NOLOAD' is set, skipping loading of rtsim state (old state will be \
                     overwritten)."
                );
            }

            let data = Data::generate(settings, world, index);
            info!("Rtsim data generated.");
            data
        };

        // `APEX-T4.3` chunk 2: verify the freshly-generated world's
        // baseline against whatever this rtsim data was last checked
        // against, BEFORE wiring it into live simulation (`RtState::new`
        // below is the reconciliation-commit point this row's own
        // architecture note names -- world generation is already
        // complete by this line, `world`/`index` are the finished
        // product). Trailing expression: `APEX-T4.6` chunk 3b's own
        // seeded (or fresh) epoch ledger, so `save_epoch_ledger_seed`
        // (computed inside this block, from the SAME `recover_v1` call
        // the baseline check already needs) doesn't need to escape it
        // via a second mutable local.
        let save_epoch_ledger = {
            let baseline_input = common::apex::world_baseline::WorldBaselineInputV1 {
                world_seed,
                // `T4-PV`: the worldgen slot is DERIVED, from the frozen
                // vocabulary this row's survey settled (see
                // `world::apex_worldgen_vocabulary`).
                // `APEX-T4.1-CONTENT-LIVE`: `content` is now DERIVED too,
                // from the caller's real, once-at-boot asset-tree walk.
                // `numeric` stays undescribed rather than fabricated --
                // its own premise-check (same row) found no compile-time
                // toolchain/codegen introspection exists yet to derive it
                // honestly (a `build.rs`-class addition, not this row's
                // "one incision"); recorded as absent, not faked.
                worldgen: worldgen_protocol_root,
                content: content_protocol_root,
                numeric: None,
                map_geometry_root: map_geometry_root.digest.bytes,
                sites: world.civs().baseline_site_graph_v1(),
                economy_root: index.world_economy_root_v1().digest.bytes,
            };
            let fresh_root = common::apex::world_baseline::compute_world_baseline_root_v1(&baseline_input)
                .expect("a locally-constructed baseline input always encodes under the domain's own limit");
            let fresh_root_bytes: [u8; 32] = *fresh_root.bytes.as_array();

            // `APEX-T4.6` chunk 3a: subsumption, read side. Once a
            // durable save-universe manifest has been published, IT is
            // the real reader per the orchestrator's own ruling ("never
            // remove the old path before the new one is the actual
            // reader") -- `data.world_baseline_root` stays the fallback
            // for the `EpochZero`/pre-adoption case (no manifest exists
            // yet) and keeps being written below either way, so an old
            // save is never worse off. A recovery ERROR (corrupt
            // manifest/pointer) is logged and treated the same as
            // `EpochZero` here: this comparison is advisory, not
            // authoritative for anything else in this chunk, so it must
            // not block startup over a manifest-layer read failure.
            //
            // chunk 3b also needs this SAME recovery result to seed the
            // in-process epoch ledger below -- one call, two consumers,
            // rather than recovering twice.
            let (recovered_world_baseline_root, save_epoch_ledger_seed): (
                Option<[u8; 32]>,
                Option<(common::apex::identity::SaveEpoch, common::apex::digest::ArtifactDigestV1, Option<common::apex::identity::UniverseBranchId>)>,
            ) = match crate::save_universe::recover_v1(&save_universe_layout) {
                Ok(crate::save_universe::SaveUniverseRecoveryV1::Recovered { manifest, manifest_identity }) => (
                    manifest.world_baseline_root.map(|d| *d.bytes.as_array()),
                    Some((manifest.lineage.epoch, manifest_identity.digest, manifest.lineage.branch)),
                ),
                Ok(crate::save_universe::SaveUniverseRecoveryV1::EpochZero) => (None, None),
                Err(e) => {
                    error!(
                        ?e,
                        "failed to recover save-universe manifest (falling back to data.world_baseline_root, starting a fresh epoch ledger)"
                    );
                    (None, None)
                },
            };
            let world_baseline_root_source = recovered_world_baseline_root.or(data.world_baseline_root);

            if let Some(stored_root_bytes) = world_baseline_root_source
                && stored_root_bytes != fresh_root_bytes
            {
                // `RESOLUTION_LAW_V1` ("loss is recorded"): write the
                // sidecar BEFORE any purge below, so the fact of the
                // mismatch survives even though `data.dat` itself is
                // about to be overwritten with fresh data. Best-effort:
                // a write failure here must not block startup, since the
                // mismatch disposition itself (purge/ignore) still has
                // to happen either way.
                #[derive(serde::Serialize)]
                struct WorldBaselineMismatchRecordV1 {
                    stored_root: Vec<u8>,
                    observed_root: Vec<u8>,
                    detected_at_unix_seconds: u64,
                }
                let record = WorldBaselineMismatchRecordV1 {
                    stored_root: stored_root_bytes.to_vec(),
                    observed_root: fresh_root_bytes.to_vec(),
                    detected_at_unix_seconds: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                };
                let sidecar_path = file_path.with_file_name("world_baseline_mismatch.json");
                match serde_json::to_vec_pretty(&record) {
                    Ok(bytes) => {
                        if let Err(e) = fs::write(&sidecar_path, bytes) {
                            error!(?e, "failed to write world-baseline-mismatch sidecar (proceeding anyway)");
                        }
                    },
                    Err(e) => error!(?e, "failed to encode world-baseline-mismatch sidecar (proceeding anyway)"),
                }

                let ignore_baseline = std::env::var("RTSIM_IGNORE_WORLD_BASELINE").is_ok();
                if ignore_baseline {
                    warn!(
                        "Rtsim data's recorded world baseline does not match this world \
                         (RTSIM_IGNORE_WORLD_BASELINE set, loading unmigrated -- the ExplicitRecoveryOnly path)"
                    );
                } else {
                    warn!(
                        "Rtsim data's recorded world baseline does not match this world \
                         (worldgen/content/economy changed since this save was written); \
                         rtsim data will be purged and regenerated"
                    );
                    data = Data::generate(settings, world, index);
                }
            }

            // Stamp the current baseline as the new floor -- covers both
            // the first-ever check (`None`) and every check that agreed.
            data.world_baseline_root = Some(fresh_root_bytes);

            match save_epoch_ledger_seed {
                Some((epoch, root, branch)) => common::apex::save_universe::SaveEpochLedgerV1::seeded_from_recovery_v1(epoch, root, branch),
                None => common::apex::save_universe::SaveEpochLedgerV1::new(),
            }
        };

        let mut this = Self {
            last_saved: None,
            world_seed,
            state: RtState::new(data).with_resource(ChunkStates(Grid::populate_from(
                world.sim().get_size().as_(),
                |_| None,
            ))),
            file_path,
            save_thread: None,
            save_universe_layout,
            save_epoch_ledger,
            bastion_demote_witness: BastionDemoteWitness::default(),
        };

        rule::start_rules(&mut this.state);

        this.state.emit(OnSetup, &mut (), world, index);

        Ok(this)
    }

    fn get_file_path(mut data_dir: PathBuf) -> PathBuf {
        let mut path = std::env::var("VELOREN_RTSIM")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                data_dir.push("rtsim");
                data_dir
            });
        path.push("data.dat");
        path
    }

    pub fn hook_character_mount_volume(
        &mut self,
        world: &World,
        index: IndexRef,
        pos: VolumePos<NpcId>,
        actor: Actor,
    ) {
        self.state
            .emit(OnMountVolume { actor, pos }, &mut (), world, index)
    }

    pub fn hook_pickup_owned_sprite(
        &mut self,
        world: &World,
        index: IndexRef,
        sprite: SpriteKind,
        wpos: Vec3<i32>,
        actor: Actor,
    ) {
        let site = world.sim().get(wpos.xy().wpos_to_cpos()).and_then(|chunk| {
            chunk
                .sites
                .iter()
                .find_map(|site| self.state.data().sites.world_site_map.get(site).copied())
        });

        self.state.emit(
            OnTheft {
                actor,
                wpos,
                sprite,
                site,
            },
            &mut (),
            world,
            index,
        )
    }

    /// T0.49 (master build order; T0-003): allocate the next persistent
    /// item-instance identity from the world-save allocator — called only
    /// at the authoritative creation commit (`create_item_drop`).
    pub fn allocate_item_instance_id(&mut self) -> common::comp::item::ItemInstanceId {
        self.state.get_data_mut().item_instance_allocator.allocate()
    }

    pub fn hook_load_chunk(
        &mut self,
        key: Vec2<i32>,
        max_res: EnumMap<TerrainResource, usize>,
        world: &World,
    ) {
        if let Some(chunk_state) = self.state.get_resource_mut::<ChunkStates>().0.get_mut(key) {
            *chunk_state = Some(LoadedChunkState { max_res });
        }

        if let Some(chunk) = world.sim().get(key) {
            let data = self.state.get_data_mut();
            for site in chunk.sites.iter() {
                let Some(site) = data.sites.world_site_map.get(site) else {
                    continue;
                };

                let site = *site;
                let Some(site) = data.sites.get_mut(site) else {
                    continue;
                };

                site.count_loaded_chunks += 1;
            }
        }
    }

    /// ★ THIS HOOK RUNS LATE, AND ITS SIBLING RUNS EARLY — a window, not a
    /// symmetry (2026-08-29). `hook_load_chunk` is called from INSIDE the
    /// dispatcher (`sys::terrain::Sys`, at the moment the chunk is inserted),
    /// while this one is called from `Server::tick` AFTER the dispatcher has
    /// finished, over `TerrainChanges::removed_chunks`. So between
    /// `sys::terrain::Sys` dropping a chunk and this line running, the
    /// `TerrainGrid` no longer has it and `ChunkStates` still says it is
    /// loaded — one whole dispatcher pass of disagreement, every time any
    /// chunk unloads.
    ///
    /// That window used to be worth one spurious promotion per chunk unload:
    /// `rtsim::tick::Sys` promoted npcs off `ChunkStates` alone, and the
    /// entity-cleanup sweep later in the same `Server::tick` deleted them off
    /// the `TerrainGrid`. The promote side is fixed (it now reads the
    /// `TerrainGrid` too, and is ordered after `sys::terrain::Sys` — see
    /// `tick::bastion_may_promote_npc`), so nothing acts on the stale entry
    /// any more. The window itself is still here, and any NEW reader of
    /// `ChunkStates` inside the dispatcher inherits it: `ChunkStates` is a
    /// per-tick-lagging CACHE of the terrain, not a second copy of it.
    pub fn hook_unload_chunk(&mut self, key: Vec2<i32>, world: &World) {
        if let Some(chunk_state) = self.state.get_resource_mut::<ChunkStates>().0.get_mut(key) {
            *chunk_state = None;
        }

        if let Some(chunk) = world.sim().get(key) {
            let data = self.state.get_data_mut();
            for site in chunk.sites.iter() {
                let Some(site) = data.sites.world_site_map.get(site) else {
                    continue;
                };

                let site = *site;
                let Some(site) = data.sites.get_mut(site) else {
                    continue;
                };

                site.count_loaded_chunks = site.count_loaded_chunks.saturating_sub(1);
            }
        }
    }

    // Note that this hook only needs to be invoked if the block change results in a
    // change to the rtsim resource produced by [`Block::get_rtsim_resource`].
    pub fn hook_block_update(&mut self, world: &World, index: IndexRef, changes: Vec<BlockDiff>) {
        self.state
            .emit(event::OnBlockChange { changes }, &mut (), world, index);
    }

    /// bastion (B3): spawn the player-colony starting band near `wpos` as
    /// ordinary rtsim NPCs carrying a colonist record — they promote/demote
    /// through the standard loaded↔simulated machinery. Returns the roster
    /// names.
    pub fn bastion_spawn_colony(&mut self, wpos: Vec3<f32>, count: u8) -> Vec<String> {
        // Live founding seeds from the current rtsim tick — each founding is a
        // unique roll (intended gameplay).
        let seed_tick = self.state.get_data_mut().tick;
        self.bastion_spawn_colony_seeded(wpos, count, seed_tick)
    }

    /// Deterministic-seed variant of [`Self::bastion_spawn_colony`]: seeds the
    /// colony-generation RNG from an EXPLICIT tick instead of the live
    /// `data.tick`. For reproducible founding in determinism captures
    /// (`BASTION_AUTOFOUND_COLONY`) — the live `data.tick` is NOT deterministic
    /// at boot in a real server (rtsim generation advances it a variable amount
    /// before the colony is founded), so a fixed `seed_tick` pins colonist
    /// identities and spawn positions across runs.
    /// ★ ADOPT A TOWN = ADOPT ITS PEOPLE (Ben direct, 2026-08-21: *"when you
    /// adopt a town you should adopt the existing npc in that town"*).
    ///
    /// Converts the residents of the site nearest `near` into colonists,
    /// **spawning nobody**. Returns their names.
    ///
    /// WHY THIS REPLACES A SPAWN RATHER THAN JOINING ONE: adoption used to
    /// walk to a village of ~22 residents and spawn 8 strangers beside them,
    /// who owned nothing, knew nothing, and stood about while the actual
    /// inhabitants went about their lives. Nobody would design that — it is
    /// what you get when "adopt a town" is implemented as "found a colony, but
    /// over there". A whole night of adoption defects had this shape: no
    /// tools, no homes, no skills, village-as-scenery. We were re-deriving,
    /// badly, everything the village already had, for people who should never
    /// have been spawned.
    ///
    /// WHAT IS INHERITED, rather than invented:
    /// - **their name** — `get_name()`, so a colonist is the villager the
    ///   player already saw, not a stranger wearing their coordinates;
    /// - **their home** — already set, so they stay where they live;
    /// - **their trade** — `Role::Civilised(profession)` seeds the matching
    ///   skill, so the village blacksmith arrives knowing how to build.
    ///
    /// DETERMINISM: keyed on `(world_seed, seed_tick, domain)` like every other
    /// rtsim draw, and the conversion order is sorted by NpcId — a slotmap's
    /// iteration order is not a promise, and two runs of one seed must adopt
    /// the same villagers with the same skills.
    pub fn bastion_adopt_town_npcs(
        &mut self,
        near: Vec2<i32>,
        seed_tick: u64,
        house_plots: &[Vec3<f32>],
        wanted: usize,
    ) -> Vec<String> {
        use common::rtsim::{Profession, Role};
        use common::bastion::{ADOPTED_TRADE_XP, WorkType};
        use rand::{RngExt as _, prelude::IndexedRandom};

        let data = self.state.get_data_mut();
        let mut rng = ::rtsim::tick_rng(self.world_seed, seed_tick, 0xBA57_C012);
        // ★ ROW 51: THE SETTLE BRANCH HAD NO PERSONALITIES — measured, 24 of
        // 24 colonists with `traits=[]` on every world I have soaked, plus
        // `consc=true` and `neur=true` at zero. Not a thin roll: each axis
        // is a sum of three uniforms centred at MID with the trait bands at
        // 102/153, so ~56% of axes clear a band and a five-axis colonist has
        // ~98% odds of carrying SOMETHING. 24/24 empty is structural.
        //
        // The cause is one missing builder call. `Npc::new` leaves
        // `personality: Default::default()` — every axis at MID, which is
        // below every HIGH_THRESHOLD trait and above every LOW_THRESHOLD one,
        // so a default colonist can NEVER carry a trait. The founding path
        // was fixed for exactly this in August; the ADOPT-A-TOWN *settle*
        // branch below was missed — and the settle branch is the path every
        // play world actually takes (`adopted_existing=0 settled=24` in the
        // adoption roll on world 108). Roadmap ARC 5 item 21 ("Personalities
        // visible") has therefore been dead at the source the whole time,
        // silently, in a field the inspector faithfully reported as empty.
        //
        // Its OWN salted stream, never `rng`: drawing inline would shift
        // every subsequent draw (bodies, species, ids) and silently replace
        // the colonists every existing baseline was measured on. Same
        // discipline, and same reason, as the founding path's 0xBA57_C011.
        let mut personality_rng =
            ::rtsim::tick_rng(self.world_seed, seed_tick, 0xBA57_C017);
        // Resolved ONCE, above the loop: the env read and its loud refusal are
        // properties of the RUN, not of colonist #7.
        let pin_trait = bastion_pin_trait();

        // The site the player chose (or the nearest, when they chose nothing)
        // — the same "nearest site to a position" rule the plot lookup uses,
        // so the people adopted and the plots adopted can never be different
        // villages.
        let Some((site_id, site_wpos)) = data
            .sites
            .iter()
            .min_by_key(|(_, site)| {
                site.wpos
                    .map(|e| e as i64)
                    .distance_squared(near.map(|e| e as i64))
            })
            .map(|(id, site)| (id, site.wpos))
        else {
            tracing::warn!("bastion: ADOPT-NPCS — no rtsim site near the target; adopted nobody");
            return Vec::new();
        };

        // ★ ASK THE SITE WHO LIVES THERE (2026-08-21). Two predicate
        // versions of this returned ZERO -- first `home == site_id`, then a
        // position radius -- and a peer session found why BOTH had to fail:
        // rtsim's worldgen creates NO villagers at boot. It records a WANTED
        // population and the Architect rule spawns it progressively. The same
        // boot logs "Registering 194 rtsim sites" and "Generated 2008 rtsim
        // NPCs to be spawned", while adoption runs ~11 seconds later with only
        // 252 alive, none of them this village's.
        //
        // So no predicate could ever have worked: I was selecting from a
        // population that did not exist yet. Swapping `home` for position was
        // a second wrong fix for the same reason as the first, and I was one
        // step from a third.
        //
        // `Site.population` is the authoritative membership AND the readiness
        // signal in one: empty means "not populated yet, come back later",
        // never "this village has nobody". The caller defers on empty rather
        // than falling back to spawning strangers.
        let population: Vec<::rtsim::data::npc::NpcId> = data
            .sites
            .get(site_id)
            .map(|site| site.population.iter().copied().collect())
            .unwrap_or_default();

        let mut total = 0u32;
        let mut already = 0u32;
        let mut not_civilised = 0u32;
        let mut not_humanoid = 0u32;
        let mut eligible = adoption::Eligible::<::rtsim::data::npc::NpcId>::empty();
        for id in population.iter().copied() {
            let Some(npc) = data.npcs.npcs.get(id) else { continue };
            total += 1;
            if npc.bastion_colonist.is_some() {
                already += 1;
                continue;
            }
            if !matches!(npc.role, Role::Civilised(_)) {
                not_civilised += 1;
                continue;
            }
            if !matches!(npc.body, common::comp::Body::Humanoid(_)) {
                not_humanoid += 1;
                continue;
            }
            eligible.push(bastion_adoption_rank(npc.wpos, house_plots), id);
        }

        // ★ ROW 54: AND NOW THE CAP ACTUALLY BINDS. This is the decision the
        // defect was missing — see the block above `bastion_housing_cap` for
        // the measurement, and that function's doc for why the denominator is
        // house plots rather than bed slots or the growth ceiling.
        //
        // Through `Eligible::into_capped` because it is the ONLY exit that
        // type offers: the census cannot become a roster without being capped.
        // See the `adoption` module's doc for the falsification result that
        // forced that shape. The cap is re-derived there rather than trusted
        // from the caller — idempotent, and this entry point is `pub`.
        //
        // Selection is by distance-to-a-house with an NpcId tiebreak, and the
        // kept set is restored to NpcId order, so the uncapped case is
        // byte-for-byte the `residents.sort()` that shipped. Determinism by
        // construction — integers and one sort, no clock, no RNG, no HashMap.
        let (mut residents, roll) = eligible.into_capped(wanted, house_plots.len());
        tracing::info!(
            site_population = total,
            eligible = roll.population_seen,
            rej_already_colonist = already,
            rej_not_civilised = not_civilised,
            rej_not_humanoid = not_humanoid,
            ?site_id,
            "bastion: ADOPT-NPCS census — the site's own roll, and who on it is eligible"
        );
        // EVERY MECHANISM WITNESSES ITSELF, and this one must be able to show
        // itself REFUSING: `capped` is the field that separates "the cap bound
        // and 38 people stayed villagers" from "the town was already within
        // its housing". Printed on BOTH arms — a cap that only logs when it
        // fires cannot be told from a cap that is not running at all, which is
        // exactly how the shipped line lied for three rows. Every field comes
        // off the `Roster` the decision itself returned, so the witness cannot
        // drift off the decision it reports.
        tracing::info!(
            population_seen = roll.population_seen,
            cap = roll.cap,
            denominator = "house_plots",
            houses = house_plots.len(),
            wanted,
            adopted = roll.adopted,
            left_as_npcs = roll.left_as_npcs,
            capped = roll.left_as_npcs > 0,
            ?site_id,
            "bastion: ADOPT-A-TOWN HOUSING CAP — two beds per house at founding (Ben's BEDS ruling, 2026-09-01; one-per-house is superseded), BINDING on the town's own residents; those not adopted stay ordinary              villagers, untouched"
        );

        // ★ THE ARCHITECT WILL NEVER FILL THIS VILLAGE, SO FILL IT OURSELVES
        // (2026-08-21). A peer traced the last hop of a bug three of us had
        // each fixed wrongly. Every architect spawn path filters
        // `!site.is_loaded()`, and founding creates the colony Presence AT the
        // town, which loads its chunks. So the very act of adopting a town
        // makes it PERMANENTLY ineligible for the only rule that would have
        // populated it. Waiting for `Site.population` to fill -- the fix I was
        // three lines into writing -- would have waited forever, on exactly
        // the site our own founding disqualified.
        //
        // And at founding tick the architect has not run AT ALL: it fires on
        // `tick % 32 == 0` and autofound founds at tick 30. The 252 NPCs alive
        // then are airship crew (2 per spawning location, 2 x 126), and their
        // `.with_home()` is commented out in rtsim's generator -- which is the
        // whole of the old, baffling `rej_wrong_home=252`. No predicate could
        // ever have selected a villager, because no villager existed.
        //
        // So we settle the village the way the architect would have: one
        // resident per house plot, `with_home(site_id)` -- which registers
        // them in `Site.population` through `spawn_npc` -- and the site's
        // faction inherited. They are villagers by every test vanilla applies,
        // and they live in the town's real houses instead of milling around a
        // spawn point.
        //
        // HONESTY, because this is the line most likely to be misread later:
        // these people are SETTLED, not adopted. They did not exist before we
        // made them. The census reports the two counts SEPARATELY and never
        // sums them, because on a world where the player walks to a town
        // before founding, `adopted_existing` is the only number that says the
        // feature did what Ben asked for.
        let adopted_existing = residents.len();
        // The plan is a PURE function with its own pins (`settle_plan`), so
        // "adopt rather than manufacture" is asserted by a test rather than
        // trusted to this loop: one house index per resident to create, empty
        // when the village already has its own people.
        // ★ AND THE SETTLE BRANCH MEASURES AGAINST THE CAP, NOT THE RAW WANT
        // — through `bastion_settle_plan_capped`, which derives the cap
        // ITSELF rather than taking it as an argument. See that function's doc
        // for why: as a chosen argument the rule was unpinnable, and the pin
        // that claimed to guard it was green against every mutation of this
        // line. A MOVER'S REFUSAL MUST NOT REARM ITS RIVAL.
        let plan = bastion_settle_plan_capped(adopted_existing, wanted, house_plots.len());
        let settled = plan.len();
        if !plan.is_empty() {
            let faction = data.sites.get(site_id).and_then(|s| s.faction);
            let professions = [
                Profession::Farmer,
                Profession::Chef,
                Profession::Blacksmith,
                Profession::Hunter,
                Profession::Guard,
                Profession::Merchant,
            ];
            for (i, house) in plan.iter().copied().enumerate() {
                // ★ SPREAD THEM AROUND THE DOOR, don't stack them on it.
                // `settle_plan` wraps when there are fewer houses than
                // colonists -- 8 residents over 4 houses is [0,1,2,3,0,1,2,3]
                // -- so without an offset each pair materialises at the SAME
                // Vec3. The first live leg founded 8 and the census read
                // total=8 at tick 300, 600, then 7 from tick 900 onward with
                // downed=0, no COLONIST DIED, and no despawn line: one
                // colonist left the ECS silently and never came back.
                //
                // NOT CLAIMED AS THE CAUSE -- one vanished, not four, so
                // co-location does not by itself explain it and the real
                // remover is still unidentified. This offset is here because
                // two people occupying one block is wrong regardless, and
                // because it removes co-location as a CONFOUND from the next
                // run: if a colonist still vanishes with everyone on their own
                // block, the cause is elsewhere and we will know it in one leg
                // instead of arguing about it.
                //
                // Deterministic by construction (index-derived, no RNG draw):
                // a ring of 8 around the house centre, so the same world always
                // places the same person in the same spot.
                let ring = [
                    (0i32, 0i32), (2, 0), (0, 2), (-2, 0),
                    (0, -2), (2, 2), (-2, 2), (2, -2),
                ];
                let (dx, dy) = ring[i % ring.len()];
                let home_wpos =
                    house_plots[house] + Vec3::new(dx as f32, dy as f32, 0.0);
                let species = *common::comp::humanoid::ALL_SPECIES
                    .choose(&mut rng)
                    .expect("humanoid species catalog must not be empty");
                let body = common::comp::Body::Humanoid(
                    common::comp::humanoid::Body::random_with(&mut rng, &species),
                );
                let mut npc = ::rtsim::data::npc::Npc::new(
                    rng.random(),
                    home_wpos,
                    body,
                    Role::Civilised(Some(professions[i % professions.len()])),
                )
                .with_home(site_id);
                if let Some(f) = faction {
                    npc = npc.with_faction(f);
                }
                // ★ ROW 51: the missing line (see the stream's own doc).
                // Through the ONE helper, so `BASTION_PIN_TRAIT` reaches the
                // path every play world actually takes — this branch used to
                // call `Personality::random` directly and the pin was inert
                // here, silently, with its own loud refusal unreachable.
                npc.personality = bastion_colonist_personality(pin_trait, &mut personality_rng);
                residents.push(data.spawn_npc(npc));
            }
        }
        tracing::info!(
            adopted_existing,
            settled,
            houses = house_plots.len(),
            wanted,
            ?site_id,
            "bastion: ADOPT-A-TOWN roll — adopted_existing are people who were              ALREADY there; settled are people we created into the village's own              houses because the architect had not populated it yet"
        );

        let mut names = Vec::new();
        // ★ ROW 51'S MISSING WITNESS. The row's finding was a COUNT — 24 of 24
        // colonists carrying `traits=[]` — and nothing counted it, so the fix
        // was as unverifiable as the defect. Counted on the loop the adoption
        // already runs, over people already in hand: free, and it costs
        // nothing at all on a world that never adopts.
        let mut trait_carriers = 0usize;
        for id in residents {
            let Some(npc) = data.npcs.npcs.get_mut(id) else { continue };
            if bastion_carries_a_trait(&npc.personality) {
                trait_carriers += 1;
            }
            let mut colonist = common::bastion::BastionColonist::generate(&mut rng);
            // Their OWN name, not a generated one. This is the whole point:
            // the player adopts people they can already see.
            if let Some(name) = npc.get_name() {
                colonist.name = name;
            }
            // Their trade becomes their skill. A village that already has a
            // blacksmith should not have to teach him to build.
            if let Role::Civilised(Some(profession)) = npc.role {
                // ROW 50: the ONE map (common::bastion). This arm was the
                // original; the settler drain's copy of it had already
                // drifted apart in the one way that mattered.
                let work = common::bastion::WorkPriorities::work_for_profession(profession);
                if let Some(w) = work {
                    colonist.skills.grant_xp(w, ADOPTED_TRADE_XP);
                    // ★ AND THE LANE (Ben RULED: "a farmer should farm, a
                    // cook should cook... for the most part they should
                    // stay in their lane"). XP made them GOOD at the trade;
                    // nothing made them PREFER it — a chef farmed as
                    // eagerly as cooking, which is how farming ate 99% of
                    // everyone's day. Priority 4 on the trade, 2 elsewhere
                    // (cross-domain possible, lane outbids), guard at the
                    // default because defence is everyone's business.
                    colonist.work_priorities =
                        common::bastion::WorkPriorities::in_lane(w);
                }
            }
            names.push(colonist.name.clone());
            npc.bastion_colonist = Some(colonist);
        }
        tracing::info!(
            colonists = names.len(),
            adopted_existing,
            settled,
            trait_carriers,
            pinned = pin_trait.is_some(),
            ?site_id,
            "bastion: ADOPT-A-TOWN — the village's residents are the colony now"
        );
        names
    }

    pub fn bastion_spawn_colony_seeded(
        &mut self,
        wpos: Vec3<f32>,
        count: u8,
        seed_tick: u64,
    ) -> Vec<String> {
        use common::rtsim::{Profession, Role};
        use rand::{RngExt as _, prelude::IndexedRandom};
        use rtsim::data::npc::Npc;

        let data = self.state.get_data_mut();
        // DETRNG/ARCH-003: colony generation is simulation input, not
        // cosmetic entropy. Reuse the one rtsim RNG authority.
        let mut rng = ::rtsim::tick_rng(self.world_seed, seed_tick, 0xBA57_C010);
        // Personality is rolled from its OWN salted stream, not `rng`: drawing
        // it inline would shift every subsequent draw (names, bodies, offsets),
        // silently replacing the colonists every existing baseline was measured
        // on. A separate stream keeps the pre-fix sequence byte-identical and
        // adds traits orthogonally.
        let mut personality_rng = ::rtsim::tick_rng(self.world_seed, seed_tick, 0xBA57_C011);
        // Resolved ONCE, above the loop: the env read and its loud refusal are
        // properties of the RUN, not of colonist #7.
        let pin_trait = bastion_pin_trait();
        // Home = nearest site, so simulated-mode AI keeps them local.
        let home = data
            .sites
            .iter()
            .min_by_key(|(_, site)| {
                site.wpos
                    .map(|e| e as i64)
                    .distance_squared(wpos.xy().map(|e| e as i64))
            })
            .map(|(id, _)| id);
        let professions = [
            Profession::Farmer,
            Profession::Hunter,
            Profession::Blacksmith,
            Profession::Chef,
        ];
        let mut names = Vec::new();
        // ★ ROW 51'S MISSING WITNESS, founding half — see
        // `bastion_carries_a_trait`. One line on a loop that already runs.
        let mut trait_carriers = 0usize;
        for i in 0..count {
            let colonist = common::bastion::BastionColonist::generate(&mut rng);
            names.push(colonist.name.clone());
            let offset = Vec3::new(
                rng.random_range(-5.0..5.0),
                rng.random_range(-5.0..5.0),
                0.0,
            );
            let species = *common::comp::humanoid::ALL_SPECIES
                .choose(&mut rng)
                .expect("humanoid species catalog must not be empty");
            let body = common::comp::Body::Humanoid(common::comp::humanoid::Body::random_with(
                &mut rng, &species,
            ));
            let mut npc = Npc::new(
                rng.random(),
                wpos + offset,
                body,
                Role::Civilised(Some(professions[i as usize % professions.len()])),
            )
            .with_bastion_colonist(colonist);
            npc.home = home;
            // Npc::new leaves `personality: Default::default()` = all five
            // axes at MID (127) -- below every >HIGH_THRESHOLD trait and above
            // every <LOW_THRESHOLD one, so a default colonist can NEVER carry
            // a personality trait. Measured before this fix: 0 trait-true
            // colonists across 348 driver logs, every fixture. Wild NPCs get
            // `.with_personality(random_good(..))` in rtsim::generate; this is
            // the founding site that rolls the COLONIST band. The pin, the
            // loud refusal and the random-vs-random_good reasoning now live
            // once, in `bastion_colonist_personality` — this used to be the
            // ONLY one of four minting sites that honoured BASTION_PIN_TRAIT.
            npc.personality = bastion_colonist_personality(pin_trait, &mut personality_rng);
            if bastion_carries_a_trait(&npc.personality) {
                trait_carriers += 1;
            }
            data.npcs.create_npc(npc);
        }
        info!(
            ?names,
            count,
            trait_carriers,
            pinned = pin_trait.is_some(),
            "bastion: spawned starting colony"
        );
        names
    }

    /// bastion (B4): set a work priority on a colonist's rtsim record by
    /// name. Returns whether any record matched.
    pub fn bastion_set_work_priority(
        &mut self,
        name: &str,
        work: common::bastion::WorkType,
        priority: u8,
    ) -> bool {
        let data = self.state.get_data_mut();
        let mut found = false;
        for (_, npc) in data.npcs.npcs.iter_mut() {
            if let Some(colonist) = &mut npc.bastion_colonist
                && colonist.name == name
            {
                colonist.work_priorities.set(work, priority);
                found = true;
            }
        }
        found
    }

    /// bastion (B5.5, harness): set a colonist's skill level for a work type
    /// on the rtsim record (the ECS mirror is handled by the Server hook).
    pub fn bastion_set_colonist_skill(
        &mut self,
        name: &str,
        work: common::bastion::WorkType,
        level: u16,
    ) -> bool {
        let data = self.state.get_data_mut();
        let mut found = false;
        for (_, npc) in data.npcs.npcs.iter_mut() {
            if let Some(colonist) = &mut npc.bastion_colonist
                && colonist.name == name
            {
                colonist.skills.set_level_for(work, level);
                found = true;
            }
        }
        found
    }

    /// bastion (LOD-0, harness): force-DEMOTE a loaded colonist by flipping
    /// its rtsim mode to Simulated — the sync loop's demote arm FLUSHES the
    /// live state into the persistent record and deletes the entity; the
    /// loaded-chunk spawn machinery then RE-PROMOTES it (the chunk stays
    /// loaded), exercising the REAL unload/re-promote cycle end-to-end.
    /// Returns whether a matching loaded colonist was found.
    pub fn bastion_force_demote(&mut self, name: &str) -> bool {
        let data = self.state.get_data_mut();
        let mut found = false;
        for (_, npc) in data.npcs.npcs.iter_mut() {
            if let Some(colonist) = &npc.bastion_colonist
                && colonist.name == name
                && matches!(npc.mode, ::rtsim::data::npc::SimulationMode::Loaded)
            {
                npc.mode = ::rtsim::data::npc::SimulationMode::Simulated;
                found = true;
            }
        }
        found
    }

    /// bastion (B3): the colony roster (headless harness dump + inspectors).
    pub fn bastion_colony_roster(&self) -> Vec<common::bastion::BastionColonist> {
        self.state
            .data()
            .npcs
            .npcs
            .values()
            .filter_map(|npc| npc.bastion_colonist.clone())
            .collect()
    }

    /// bastion (FOUNDING PRESET v1, packet §4 + review §8 B6): does a
    /// colony already live in this world?
    ///
    /// TEMPORAL SHAPE: **SNAPSHOT** (PACKET-CRAFT-CHECKLIST entry 1) — the
    /// answer is "right now", never "ever". An extinct colony leaves no
    /// records, so re-founding is permitted by construction; that is the
    /// ruled behaviour, not an accident of the read.
    ///
    /// WHY THE RTSIM RECORDS AND NOTHING ELSE: they are the only part of a
    /// colony that survives a server restart. The JobBoard and its
    /// designations do NOT persist (found live restarting the celebration
    /// world: colonists came back, the zones did not), and the colony
    /// presence entity is not persistence-backed either. A boundary check
    /// reading either of those would answer "no colony here" after any
    /// restart WHILE THE FIRST COLONY'S COLONISTS ARE STILL STANDING IN
    /// THE WORLD — and would then bless exactly the second founding whose
    /// cross-country leash-march the one-colony boundary exists to make
    /// impossible. The predicate has to outlive a restart because the
    /// failure it prevents does.
    pub fn bastion_colony_exists(&self) -> bool {
        self.state
            .data()
            .npcs
            .npcs
            .values()
            .any(|npc| npc.bastion_colonist.is_some())
    }

    /// bastion (B-AG2, harness): how many rtsim NPCs carry each CONVERTED
    /// archetype's profession (herbalist, hunter, guard) — evidence the
    /// table applies to a real generated population, not just test keys.
    pub fn bastion_profession_census(&self) -> (usize, usize, usize) {
        use common::rtsim::{Profession, Role};
        let data = self.state.data();
        let mut census = (0, 0, 0);
        for (_, npc) in data.npcs.npcs.iter() {
            if let Role::Civilised(Some(p)) = &npc.role {
                match p {
                    Profession::Herbalist => census.0 += 1,
                    Profession::Hunter => census.1 += 1,
                    Profession::Guard => census.2 += 1,
                    _ => {},
                }
            }
        }
        census
    }

    /// bastion (HIST-0, harness): soak-record `n` chronicle test events at
    /// an importance band (0 = Routine, 1 = Notable, other = Legendary)
    /// through THE ONE capture entry point. Returns the last stamped seq.
    pub fn bastion_chronicle_record_test(&mut self, band: u8, n: u32) -> u64 {
        use ::rtsim::data::{ChronicleKind, Importance, Scope};
        let data = self.state.get_data_mut();
        let now = data.time_of_day;
        let importance = match band {
            0 => Importance::Routine,
            1 => Importance::Notable,
            _ => Importance::Legendary,
        };
        let mut last = 0;
        for i in 0..n {
            last = data.chronicle.record(
                now,
                ChronicleKind::Founding,
                Vec::new(),
                None,
                Some(Vec3::new(i as i32, 0, 0)),
                importance,
                Scope::Colony,
                None,
            );
        }
        last
    }

    /// bastion (HIST-0, harness): (routine, notable, legendary) live
    /// counts — the bounded-growth probe.
    pub fn bastion_chronicle_counts(&self) -> (usize, usize, usize) {
        self.state.data().chronicle.counts()
    }

    /// bastion (HIST-0, harness): the B10 boundary round-trip + the
    /// immortality sweep, in vivo. (1) An end-of-time cleanup must not
    /// touch a single Legendary entry; (2) the LIVE `Data` encodes through
    /// the exact persistence encoder (`Data::write_to`) and decodes back
    /// (`Data::from_reader`, version-checked) with the chronicle surviving
    /// BYTE-FOR-BYTE (fingerprint equality) and counts intact.
    pub fn bastion_chronicle_roundtrip(&mut self) -> bool {
        let data = self.state.get_data_mut();
        let legendary_before = data.chronicle.counts().2;
        let end_of_time = common::resources::TimeOfDay(data.time_of_day.0 + 1.0e12);
        data.chronicle.cleanup(end_of_time);
        if data.chronicle.counts().2 != legendary_before {
            return false;
        }
        let mut bytes = Vec::new();
        if data.write_to(&mut bytes).is_err() {
            return false;
        }
        let decoded = match ::rtsim::data::Data::from_reader(bytes.as_slice()) {
            Ok(d) => d,
            Err(_) => return false,
        };
        match (
            data.chronicle.fingerprint(),
            decoded.chronicle.fingerprint(),
        ) {
            (Some(a), Some(b)) => a == b && data.chronicle.counts() == decoded.chronicle.counts(),
            _ => false,
        }
    }

    pub fn hook_rtsim_entity_unload(&mut self, entity: RtSimEntity) {
        // ★ THE COLONY DOES NOT SHRINK -- THE INSTRUMENT DOES (2026-08-21).
        // A verification run read total=8 at ticks 300 and 600, then total=7
        // from tick 900 onward, with downed=0, no COLONIST DIED and no despawn
        // line. It looked exactly like a colonist silently vanishing, and that
        // is how I first read it. Nobody died: ONE colonist walked out of the
        // loaded area and was demoted here, losing its ECS entity.
        //
        // A JOIN IS A FILTER. Every ECS-joined instrument drops that colonist
        // at once -- the EXPERIENCE census `total`, `fed`, `rested`, `engaged`,
        // and (the part with teeth) food_per_cap, beds_short and the colony
        // drive, which therefore decide against a denominator that shrinks
        // whenever somebody goes for a walk. So the boundary emit carries the
        // WHOLE population, counted from rtsim rather than the ECS: a reader
        // cannot mistake a demotion for a death, because the totals sit beside
        // it and they conserve.
        //
        // ★ COST FIX, SECOND PASS (2026-08-29). The first pass gated that
        // whole-world scan on `is_colonist` and called it done. It was not:
        // every one of the ~38 demotions per second on the owner's world IS a
        // colonist, so the gate was true every time and the scan ran ~38 times
        // a second over ~8,528 records -- ~324,000 npc-record touches per
        // second to decorate three fields. The predicate was right about the
        // shape of the waste and wrong about which axis was rare. See
        // `BASTION_CENSUS_AUDIT_TICKS`: the census is now recomputed on a
        // CLOCK, at most once per audit window, and the line carries
        // `census_age_ticks` so a stale number still names its producer.
        //
        // ★ AND THE WITNESS GOT SHARPER, NOT WEAKER. The census answered "did
        // anybody die?"; it could not say "is this colonist ringing?" -- that
        // took clustering two million lines after the fact. `demotes` and
        // `since_last_demote_ticks` are O(1) and say it on the FIRST line: on
        // the world that spun, every line would have read `ringing=true` with
        // `since_last_demote_ticks=2`.
        let now = self.state.data().tick;
        let (mut loaded_c, mut simulated_c) = self.bastion_demote_witness.census;
        let census_taken_at = self.bastion_demote_witness.census_tick;
        let want_recompute = bastion_census_needs_recompute(census_taken_at, now);
        let previous = self.bastion_demote_witness.demotes.get(&entity).copied();
        let demotes = previous.map_or(1, |(n, _)| n + 1);
        let ringing = bastion_demote_is_ringing(previous.map(|(_, t)| t), now);

        let mut recomputed = false;
        let mut demoted_a_colonist = false;
        {
            let data = self.state.get_data_mut();

            let is_colonist = data
                .npcs
                .get(entity)
                .is_some_and(|n| n.bastion_colonist.is_some());
            if is_colonist && want_recompute {
                let (mut l, mut s) = (0u32, 0u32);
                for (_, n) in data.npcs.npcs.iter() {
                    if n.bastion_colonist.is_some() {
                        match n.mode {
                            SimulationMode::Loaded => l += 1,
                            SimulationMode::Simulated => s += 1,
                        }
                    }
                }
                loaded_c = l;
                simulated_c = s;
                recomputed = true;
            }
            let census_age_ticks = if recomputed {
                0
            } else {
                census_taken_at.map_or(0, |t| now.saturating_sub(t))
            };

            if let Some(npc) = data.npcs.get_mut(entity) {
                if matches!(npc.mode, SimulationMode::Simulated) {
                    error!("Unloaded already unloaded entity");
                }
                // bastion (B3): the loaded<->simulated boundary, log-verified.
                if let Some(colonist) = &npc.bastion_colonist {
                    tracing::info!(
                        name = colonist.name.as_str(),
                        // Counted BEFORE this demotion lands, so the line reads
                        // as the transition it is. `colony_total` is the number
                        // that must NOT move for a demotion -- if it drops,
                        // somebody really did die and this is not the reason.
                        // It is as old as `census_age_ticks` says it is.
                        colony_total = loaded_c + simulated_c,
                        loaded_before = loaded_c,
                        simulated_before = simulated_c,
                        census_age_ticks,
                        // The free half, and the half that names an oscillation
                        // on sight.
                        demotes,
                        ringing,
                        "bastion: colonist demoted to SimulationMode::Simulated"
                    );
                    demoted_a_colonist = true;
                }
                npc.mode = SimulationMode::Simulated;
            }
        }

        if recomputed {
            self.bastion_demote_witness.census = (loaded_c, simulated_c);
            self.bastion_demote_witness.census_tick = Some(now);
        }
        if demoted_a_colonist {
            self.bastion_demote_witness
                .demotes
                .insert(entity, (demotes, now));
        }
    }

    pub fn hook_rtsim_actor_hp_change(
        &mut self,
        world: &World,
        index: IndexRef,
        actor: Actor,
        cause: Option<Actor>,
        new_hp_fraction: f32,
        change: f32,
        old_hp_fraction: f32,
    ) {
        self.state.emit(
            OnHealthChange {
                actor,
                cause,
                new_health_fraction: new_hp_fraction,
                change,
                old_health_fraction: old_hp_fraction,
            },
            &mut (),
            world,
            index,
        )
    }

    pub fn hook_rtsim_actor_death(
        &mut self,
        world: &World,
        index: IndexRef,
        actor: Actor,
        wpos: Option<Vec3<f32>>,
        killer: Option<Actor>,
    ) {
        self.state.emit(
            OnDeath {
                wpos,
                actor,
                killer,
            },
            &mut (),
            world,
            index,
        );
    }

    pub fn hook_rtsim_actor_helped(
        &mut self,
        world: &World,
        index: IndexRef,
        actor: Actor,
        saver: Option<Actor>,
    ) {
        self.state
            .emit(OnHelped { actor, saver }, &mut (), world, index);
    }

    // `APEX-T4.6` chunk 3b: `character_db_dir` is the character DB's own
    // directory (`DatabaseSettings::db_dir`) -- neither call site had a
    // reason to know it before this row; both are threaded now
    // (`rtsim/tick.rs`'s periodic save, `lib.rs`'s shutdown save).
    pub fn save(&mut self, wait_until_finished: bool, character_db_dir: &std::path::Path) {
        debug!("Saving rtsim data...");

        // Create the save thread if it doesn't already exist
        // We're not using the slow job pool here for two reasons:
        // 1) The thread is mostly blocked on IO, not compute
        // 2) We need to synchronise saves to ensure monotonicity, which slow jobs
        // aren't designed to allow
        let (tx, _) = self.save_thread.get_or_insert_with(|| {
            trace!("Starting rtsim data save thread...");
            let (tx, rx) = unbounded();
            let file_path = self.file_path.clone();
            (tx, thread::spawn(move || save_thread(file_path, rx)))
        });

        // Send rtsim data to the save thread
        if let Err(err) = tx.send(self.state.data().clone()) {
            error!("Failed to perform rtsim save: {}", err);
        }

        // If we need to wait until the save thread has done its work (due to, for
        // example, server shutdown) then do that.
        if wait_until_finished && let Some((tx, handle)) = self.save_thread.take() {
            drop(tx);
            info!("Waiting for rtsim save thread to finish...");
            handle.join().expect("Save thread failed to join");
            info!("Rtsim save thread finished.");
        }

        self.last_saved = Some(Instant::now());

        // `APEX-T4.6` chunk 3b: the staged multi-store epoch commit, run
        // SYNCHRONOUSLY and best-effort, strictly ADDITIVE to the
        // existing rtsim-file save above (never blocks or fails it -- a
        // staged-commit failure only ever prevents THIS epoch's commit,
        // logged loudly, not the primary save this function already
        // promised). A future perf pass can move this to its own
        // thread; correctness-first for this landing.
        self.commit_save_universe_epoch_v1(character_db_dir);
    }

    /// See [`Self::save`]'s own doc comment for why this runs where it
    /// does and why it is best-effort.
    fn commit_save_universe_epoch_v1(&mut self, character_db_dir: &std::path::Path) {
        let (frozen_tick, world_baseline_root_bytes) = {
            let data = self.state.data();
            (data.tick, data.world_baseline_root)
        };
        let mut rtsim_bytes = Vec::new();
        if let Err(e) = self.state.data().write_to(&mut rtsim_bytes) {
            error!(?e, "failed to encode rtsim payload for save-universe staging (skipping this epoch's commit)");
            return;
        }

        let world_baseline_root = world_baseline_root_bytes.map(|bytes| common::apex::digest::ArtifactDigestV1 {
            algorithm: common::apex::digest::DigestAlgorithmIdV1::Sha256,
            bytes: common::apex::digest::DigestBytes32V1::from_array(bytes),
        });

        let candidate_epoch = common::apex::identity::SaveEpoch::new(self.save_epoch_ledger.current_epoch().get() + 1);
        let lineage = common::apex::save_universe::SaveEpochLineageV1 {
            epoch: candidate_epoch,
            predecessor_root: self.save_epoch_ledger.current_root(),
            // Carry forward whatever branch the ledger already tracks
            // (`None` for a lineage never branched, unchanged behavior;
            // `Some(id)` once `APEX-T9.2` branching creates one) -- an
            // ordinary forward save must not silently drop a branch
            // identity, or the very next `admit_v1` would refuse it as a
            // `BranchMismatch`.
            branch: self.save_epoch_ledger.current_branch(),
        };

        let rtsim_payload = match crate::save_universe::stage_payload_v1(
            &self.save_universe_layout,
            candidate_epoch,
            common::apex::save_universe::SaveStoreIdV1::RtsimData,
            |f| std::io::Write::write_all(f, &rtsim_bytes),
        ) {
            Ok(p) => p,
            Err(e) => {
                error!(?e, "failed to stage rtsim payload for save-universe epoch (skipping this epoch's commit)");
                return;
            },
        };

        let character_db_payload = match crate::save_universe::stage_character_db_v1(&self.save_universe_layout, candidate_epoch, character_db_dir) {
            Ok(p) => p,
            Err(e) => {
                error!(?e, "failed to stage character-db payload for save-universe epoch (skipping this epoch's commit)");
                return;
            },
        };

        let manifest = common::apex::save_universe::SaveUniverseManifestV1 {
            lineage,
            frozen_tick,
            // Canonical store order (`SaveStoreIdV1`'s own discriminant
            // order: `CharacterDb` then `RtsimData`) -- the type's own
            // "caller supplies sorted order for reproducibility" doc note.
            stores: vec![character_db_payload, rtsim_payload],
            world_baseline_root,
            // `T4-PV` (parked, orchestrator-ruled): same undescribed-
            // rather-than-fabricated discipline as the world-baseline
            // check above -- no honest frozen-vocabulary derivation
            // exists yet for content/build/numeric/schedule identity.
            descriptors: Vec::new(),
            // `T4.5`'s confirmed-EMPTY rtsim migration graph -- nothing
            // to journal yet.
            migration_journal_digest: None,
        };

        match crate::save_universe::commit_epoch_v1(&self.save_universe_layout, &manifest) {
            Ok(pointer) => {
                if let Err(e) = self.save_epoch_ledger.admit_v1(manifest.lineage, pointer.manifest_identity.digest) {
                    error!(
                        ?e,
                        "save-universe epoch committed to disk but the in-process ledger refused to admit it -- internal inconsistency, investigate"
                    );
                }
            },
            Err(e) => error!(?e, "failed to commit save-universe epoch (rtsim/character-db payloads staged but not published)"),
        }
    }

    // TODO: Clean up this API a bit
    pub fn get_chunk_resources(&self, key: Vec2<i32>) -> EnumMap<TerrainResource, f32> {
        self.state
            .data()
            .nature
            .chunk_resources(key)
            .copied()
            .unwrap_or_default()
    }

    pub fn state(&self) -> &RtState { &self.state }

    pub fn set_should_purge(&mut self, should_purge: bool) {
        self.state.data_mut().should_purge = should_purge;
    }
}

// Crate-split seam: the bastion job system (now in the `bastion-server` leaf)
// reads rtsim state through this one-method trait instead of naming `RtSim`
// directly; `sys/mod.rs` registers `bastion_jobs::Sys<RtSim>`.
impl bastion_server::bastion_jobs::RtSimAccess for RtSim {
    fn rt_state(&self) -> &RtState { self.state() }
}

fn save_thread(file_path: PathBuf, rx: Receiver<Data>) {
    if let Some(dir) = file_path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    let atomic_file = AtomicFile::new(file_path, OverwriteBehavior::AllowOverwrite);
    while let Ok(data) = rx.recv() {
        debug!("Writing rtsim data to file...");
        match atomic_file.write(move |file| data.write_to(io::BufWriter::new(file))) {
            Ok(_) => debug!("Rtsim data saved."),
            Err(e) => error!("Saving rtsim data failed: {}", e),
        }
    }
}

pub struct ChunkStates(pub Grid<Option<LoadedChunkState>>);

pub struct LoadedChunkState {
    // The maximum possible number of each resource in this chunk
    pub max_res: EnumMap<TerrainResource, usize>,
}

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    // T0.16 (master build order; ledger #67): the jobs -> RTSim THOUGHT
    // OUTBOX edge, DECLARED — bastion_jobs pushes `pending_thoughts` that
    // this system drains; without the explicit dependency their order was
    // implicit shred staging (deterministic per build, but an undeclared
    // contract a registration shuffle could silently flip, moving thought
    // delivery by a tick).
    // ★ AN UNDECLARED EDGE THIS SYSTEM NOW DEPENDS ON (2026-08-29, left
    // undeclared ON PURPOSE — read before "fixing" it).
    //
    // `tick::Sys` gates promotion on `TerrainGrid::get_key_real`, the same grid
    // the entity-cleanup sweep in `Server::tick` deletes on (see
    // `tick::bastion_may_promote_npc`). That agreement is exact only while this
    // system runs AFTER `sys::terrain::Sys`, the one code path that inserts and
    // removes terrain chunks. Today it does, but by STAGING, not by
    // declaration: `sys::terrain::Sys` shares this system's `Phase::Create` and
    // conflicts with it (both hold `WriteExpect<RtSim>`), and it is registered
    // first — `sys::add_server_systems` at server/src/lib.rs:635 runs before
    // `rtsim::add_server_systems` at :638.
    //
    // Adding `&crate::sys::terrain::Sys::sys_name()` to the list below makes it
    // explicit, exactly as T0.16 and T0.20 did at the two seams above, and it
    // introduces no cycle (`terrain::Sys` depends only on `msg::terrain::Sys`,
    // and nothing in the tree depends on `rtsim::tick::Sys`). It is NOT done
    // here because the T0.12 phase manifest mirrors this registration from
    // another module's test, and that manifest's own assert says to update BOTH
    // deliberately. If the edge is declared, the manifest entry
    // `("rtsim", "tick", ["phys", "bastion_jobs"])` becomes
    // `("rtsim", "tick", ["phys", "bastion_jobs", "sys::terrain"])`.
    //
    // What is at stake if the staging ever flips: this system would read the
    // PREVIOUS tick's terrain, so one spurious promotion could survive per
    // chunk unload — the same defect that produced the 38-transitions-per-second
    // spin, at a far lower rate.
    dispatch::<tick::Sys>(dispatch_builder, &[
        &common_systems::phys::Sys::sys_name(),
        &crate::bastion_jobs::Sys::<crate::rtsim::RtSim>::sys_name(),
    ]);
}

#[cfg(test)]
mod bastion_demote_witness_pins {
    use super::*;

    /// The owner's world, as measured: ~8,528 rtsim npc records, ~38 colonist
    /// demotions per second at ~5 server ticks/s.
    const NPCS: u64 = 8_528;
    const DEMOTES_PER_TICK: u64 = 8;
    const TICKS: u64 = 1_000;

    /// ★ THE COST OF THE WITNESS IS BOUNDED BY THE CLOCK, NOT BY THE EVENT
    /// RATE. This is the pin the cost fix exists for: the shipped code scanned
    /// every npc record on every demotion, so a defect that RAISED the demotion
    /// rate also raised the cost of observing it — an instrument that gets more
    /// expensive exactly when it is most needed.
    ///
    /// PLANT-AND-PROVE: make `bastion_census_needs_recompute` return `true`
    /// unconditionally (which is the shipped behaviour, since its `is_colonist`
    /// gate was true for every call that reached it) and both assertions fail:
    /// scans becomes 8,000 instead of 16, and touches equals the shipped total
    /// exactly.
    #[test]
    fn the_witness_cost_is_bounded_by_ticks_not_by_events() {
        let mut taken_at: Option<u64> = None;
        let mut scans = 0u64;
        for now in 0..TICKS {
            for _ in 0..DEMOTES_PER_TICK {
                if bastion_census_needs_recompute(taken_at, now) {
                    scans += 1;
                    taken_at = Some(now);
                }
            }
        }
        let ceiling = TICKS / BASTION_CENSUS_AUDIT_TICKS + 1;
        assert!(
            scans <= ceiling,
            "the census ran {scans} times in {TICKS} ticks; the audit clock allows at most \
             {ceiling}. The cost is following the event rate again."
        );
        let touches = scans * NPCS;
        let shipped = TICKS * DEMOTES_PER_TICK * NPCS;
        assert!(
            touches * 100 < shipped,
            "the witness touched {touches} npc records where the shipped scan touched \
             {shipped} — the fix must be at least two orders of magnitude cheaper, not a \
             rounding-down of the same loop"
        );
    }

    /// Several demotions inside ONE tick must cost ONE census, not one each.
    /// That is the exact shape of the live defect: ~8 colonist demotions landed
    /// in every single tick and every one of them scanned the world.
    #[test]
    fn many_demotions_in_one_tick_cost_one_census() {
        let mut taken_at: Option<u64> = None;
        let mut scans = 0u64;
        for _ in 0..500 {
            if bastion_census_needs_recompute(taken_at, 7) {
                scans += 1;
                taken_at = Some(7);
            }
        }
        assert_eq!(scans, 1, "500 demotions in one tick cost {scans} full-world scans");
    }

    /// VALIDATE THE INSTRUMENT FIRST: the audit must still fire. A cost fix
    /// that simply stopped recomputing would pass the bound above and report a
    /// frozen census forever, which is the "blind instrument agrees with you"
    /// failure.
    #[test]
    fn the_census_is_still_recomputed() {
        assert!(
            bastion_census_needs_recompute(None, 0),
            "the very first demotion must produce a real count, not a default"
        );
        assert!(bastion_census_needs_recompute(
            Some(0),
            BASTION_CENSUS_AUDIT_TICKS
        ));
        assert!(!bastion_census_needs_recompute(
            Some(0),
            BASTION_CENSUS_AUDIT_TICKS - 1
        ));
    }

    /// ★ THE WITNESS MUST STILL SHOW THE MECHANISM FAILING. The census told a
    /// reader "nobody died"; it never told them "this colonist is ringing" —
    /// that took clustering two million lines. `ringing` says it per line, for
    /// free, and it must DISCRIMINATE: a colonist that walks out of the loaded
    /// island once an hour is not ringing, and one demoted every other tick is.
    #[test]
    fn the_ring_witness_discriminates() {
        // The measured spin: demoted again two ticks after its last demotion.
        assert!(
            bastion_demote_is_ringing(Some(100), 102),
            "a colonist demoted twice within two ticks is the oscillation and must read as one"
        );
        // A colonist that genuinely went for a walk.
        assert!(!bastion_demote_is_ringing(Some(100), 100 + 30 * 60));
        // First demotion ever — nothing to compare against, so not a ring.
        assert!(!bastion_demote_is_ringing(None, 0));
        // The boundary itself, both sides, so the constant cannot drift
        // unnoticed.
        assert!(bastion_demote_is_ringing(Some(0), BASTION_RING_TICKS));
        assert!(!bastion_demote_is_ringing(Some(0), BASTION_RING_TICKS + 1));
    }
}

#[cfg(test)]
mod bastion_personality_pins {
    use super::*;
    use common::rtsim::{Personality, PersonalityTrait};

    fn stream() -> impl rand::RngExt { ::rtsim::tick_rng(7, 11, 0xBA57_C011) }

    /// FALLBACK MUST BE IDENTITY, NOT A COPY. Moving three call sites onto the
    /// shared helper is only safe if the unpinned arm is byte-for-byte the draw
    /// those sites already made — otherwise the fix silently replaces the
    /// colonists of every world measured before it. Compared through `Debug`
    /// because `Personality` derives no `PartialEq`.
    ///
    /// PLANT-AND-PROVE: change the `None` arm of `bastion_colonist_personality`
    /// to `Personality::random_good(rng)` (the plausible wrong fix — it is what
    /// vanilla hands a Guard) and this fails.
    #[test]
    fn the_unpinned_arm_is_exactly_personality_random() {
        let mut a = stream();
        let mut b = stream();
        assert_eq!(
            format!("{:?}", bastion_colonist_personality(None, &mut a)),
            format!("{:?}", Personality::random(&mut b)),
        );
    }

    /// The pinned arm actually pins, and — the half that matters — it does NOT
    /// touch the personality stream. A pin that consumed a draw would shift
    /// every later colonist, so a pinned run and an unpinned run of the same
    /// seed would not be comparable, which is the only thing a pinned run is
    /// for.
    #[test]
    fn the_pinned_arm_pins_and_spends_nothing() {
        let mut rng = stream();
        let pinned = bastion_colonist_personality(Some(PersonalityTrait::Conscientious), &mut rng);
        assert!(pinned.is(PersonalityTrait::Conscientious));
        // The stream is untouched, so the very next draw off it equals the
        // first draw off a fresh copy of the same stream.
        let mut fresh = stream();
        assert_eq!(
            format!("{:?}", Personality::random(&mut rng)),
            format!("{:?}", Personality::random(&mut fresh)),
        );
    }

    /// VALIDATE THE INSTRUMENT FIRST: ROW 51's witness is a COUNT of
    /// trait-carrying colonists, and a predicate that always answers the same
    /// thing would agree with any hypothesis. Pinned against BOTH degenerate
    /// answers — the default personality (every axis at MID, which is the exact
    /// bug state ROW 51 found: `traits=[]` on 24 of 24) must read false, and a
    /// pinned one must read true.
    #[test]
    fn the_trait_witness_discriminates() {
        assert!(
            !bastion_carries_a_trait(&Personality::default()),
            "a default personality sits at MID on every axis and carries nothing — a witness \
             that calls it trait-carrying cannot see ROW 51's defect"
        );
        for t in <PersonalityTrait as strum::IntoEnumIterator>::iter() {
            assert!(
                bastion_carries_a_trait(&Personality::pinned(t)),
                "a personality pinned to a named trait must read as carrying one"
            );
        }
    }
}

/// ROW 54's pins. Every constant below is a number from the owner's live
/// session of 2026-08-30, not a number invented to make a test pass.
#[cfg(test)]
mod bastion_adoption_cap_pins {
    use super::*;

    /// `ADOPT-NPCS site_population=46 eligible=46`.
    const SEEN: usize = 46;
    /// `ADOPT-A-TOWN roll ... houses=10`.
    const HOUSES: usize = 10;
    /// `ADOPT-A-TOWN roll ... wanted=8` — the env count, already the `wanted`
    /// this path receives.
    const ENV_WANTED: usize = 8;
    /// `ADOPT-IN-PLACE house registered x10, adopted_beds=2 each -> beds=20`.
    /// Two SLOTS per physical bed; see `is_adoptable_bed_sprite`'s doc.
    const SLOTS_PER_HOUSE: u32 = 2;
    /// `bastion_server::bastion_jobs::immigration_target_pop` counts HOUSES
    /// that hold beds, not slots — the owner's log reads `target_pop=10`
    /// against `houses=10`, which is the reading this transcribes. It is
    /// `pub(crate)` in a crate this row may not edit, so the value is carried
    /// here by symbol rather than called; the drive gate below IS called.
    const TARGET_POP: u32 = HOUSES as u32;
    /// The one drive input this row is not about: the same session logged
    /// `food_per_cap=3.5`, clear of both Sustain bars, and no threats. Held
    /// where it is not the blocker so the housing term is isolated.
    const FOOD_PER_CAP: f32 = 3.5;

    /// PRODUCTION LINE GUARDED: the `cap` derivation inside
    /// [`adoption::Eligible::into_capped`], which is the one exit the census
    /// has. Replace it with `usize::MAX` and this reads 46.
    ///
    /// ★ THIS PIN WAS GREEN AND BLIND ON ITS FIRST WRITING, and the finding
    /// outlived the fix. It was written against `bastion_adoption_pick`
    /// directly, and stayed green through a plant that DELETED the cap from
    /// `bastion_adopt_town_npcs` and restored the shipped uncapped
    /// `residents.sort()` — a valid cycle, 22.59s rebuild, `Compiling
    /// veloren-server` in the log. A pure pin cannot witness a call site. The
    /// production code was reshaped so that it does not have to: see the
    /// `adoption` module, where the uncapped construction no longer compiles.
    /// The pin now drives that one exit end to end.
    #[test]
    fn the_cap_binds_the_towns_own_residents() {
        let mut eligible = adoption::Eligible::<u32>::empty();
        for i in 0..SEEN as u32 {
            eligible.push(i as i64, i);
        }
        let (adopted, roll) = eligible.into_capped(ENV_WANTED, HOUSES);
        assert_eq!(roll.cap, 8, "min(env wanted 8, houses 10)");
        assert_eq!(
            adopted.len(),
            roll.cap,
            "the owner's town had {SEEN} eligible residents and room for {}; adoption kept \
             {}. The cap is being computed and ignored again.",
            roll.cap,
            adopted.len()
        );
        // The witness must be able to show the cap REFUSING, and its numbers
        // must come off the decision itself.
        assert_eq!(roll.population_seen, SEEN);
        assert_eq!(roll.adopted, 8);
        assert_eq!(roll.left_as_npcs, SEEN - 8, "38 residents stay ordinary villagers");
        assert!(roll.left_as_npcs > 0, "the `capped=true` arm must be reachable");
    }

    /// FALLBACK IS IDENTITY on the same exit: a town within its housing is
    /// adopted whole and the witness reports a cap that did NOT refuse, so
    /// `capped=false` is reachable too. A cap that could only ever log itself
    /// firing could not be told from one that is not running at all — which is
    /// precisely how the shipped line lied for three rows.
    #[test]
    fn the_witness_can_show_the_cap_not_refusing() {
        let mut eligible = adoption::Eligible::<u32>::empty();
        for i in 0..6u32 {
            eligible.push(i as i64, i);
        }
        let (adopted, roll) = eligible.into_capped(ENV_WANTED, HOUSES);
        assert_eq!(adopted.len(), 6, "six residents, room for eight — adopt all six");
        assert_eq!(roll.left_as_npcs, 0);
        assert!(!(roll.left_as_npcs > 0), "the `capped=false` arm must be reachable");
    }

    /// FALLBACK IS IDENTITY. PRODUCTION LINES GUARDED: `ranked.truncate(cap)`
    /// and `kept.sort()` in [`bastion_adoption_pick`], together.
    ///
    /// The shipped behaviour was "adopt every eligible resident, in NpcId
    /// order" (`residents.sort()`), and an adopted town already within its
    /// housing must still get exactly that — SAME SET and SAME ORDER, because
    /// the caller draws one `BastionColonist::generate` per entry from a
    /// seeded stream and the order decides who receives which record.
    ///
    /// The ranks here are deliberately ANTI-correlated with the ids, so a pick
    /// that selected correctly but forgot to restore NpcId order returns a
    /// different permutation and fails.
    #[test]
    fn a_town_within_its_housing_is_bit_for_bit_unchanged() {
        let ids: Vec<u32> = vec![9, 2, 7, 1, 4, 3];
        let eligible: Vec<(i64, u32)> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (1_000 - i as i64, *id))
            .collect();
        let shipped = {
            let mut s = ids.clone();
            s.sort();
            s
        };
        for cap in [ids.len(), ids.len() + 1, ENV_WANTED, usize::MAX] {
            assert_eq!(
                bastion_adoption_pick(eligible.clone(), cap),
                shipped,
                "cap={cap}: a town at or under its housing must be the shipped adoption, \
                 set AND order"
            );
        }
    }

    /// ★ THE ROW'S OWN PIN: A GATE MUST BE REACHABLE, and the success
    /// condition is that the three witnesses can FIRE — not that a cap was
    /// applied.
    ///
    /// PRODUCTION LINES GUARDED: `bastion_housing_cap`'s denominator together
    /// with the `bastion_adoption_pick` call. Widen the denominator to bed
    /// SLOTS (`HOUSES * SLOTS_PER_HOUSE`) and the drive assertions still pass
    /// while `pop < TARGET_POP` fails — which is precisely why bed slots were
    /// rejected, and why this pin asserts the roster arms as well as the drive.
    ///
    /// The drive arm calls the SHIPPED `colony_drive_for`. The other two arms
    /// are transcribed (their verdict functions are `pub(crate)` in
    /// `bastion-server`, which this row may not edit) and each names the
    /// symbol and refusal string it stands for.
    #[test]
    fn the_three_witnesses_become_reachable() {
        use bastion_server::bastion_jobs::colony_drive_for;
        use common::bastion::ColonyDrive;

        let beds = HOUSES as u32 * SLOTS_PER_HOUSE;
        assert_eq!(beds, 20, "the owner's logged bed count");

        // ── CONTROL: the shipped behaviour, "adopt everyone". A FALSIFIER
        // NEEDS ITS OWN CONTROL — if these three refusals did not reproduce,
        // the "after" arm below would be measuring nothing.
        let pop_before = SEEN as u32;
        let (drive_before, term_before, _) =
            colony_drive_for(FOOD_PER_CAP, 0, beds, pop_before, ColonyDrive::Grow);
        assert_eq!(
            (drive_before, term_before),
            (ColonyDrive::Grow, "beds_short"),
            "46 people into 20 slots must pin the drive at Grow — that is the logged defect"
        );
        assert!(
            pop_before >= TARGET_POP,
            "control: `roster >= target_pop`, so birth_verdict refuses no_room_in_town and \
             immigration_verdict refuses roster_at_target"
        );
        assert!(
            pop_before.div_ceil(HOUSES as u32) >= SLOTS_PER_HOUSE,
            "control: every slot in every house is owned, so courtship's free_bed_in_house \
             cannot resolve and courtship_verdict refuses no_home"
        );

        // ── AFTER: the cap binds.
        let pop = bastion_housing_cap(ENV_WANTED, HOUSES) as u32;
        let (drive, term, _) =
            colony_drive_for(FOOD_PER_CAP, 0, beds, pop, ColonyDrive::Grow);
        assert_eq!(
            (drive, term),
            (ColonyDrive::Expand, "satisfied"),
            "beds {beds} >= pop {pop} must let the colony mind leave Grow — without this, \
             immigration_verdict and birth_verdict both refuse drive_not_expand forever"
        );
        assert!(
            pop < TARGET_POP,
            "roster {pop} must sit BELOW target_pop {TARGET_POP}, or the drive clears and \
             births still refuse no_room_in_town while housing growth refuses \
             roster_at_target. This is the assertion the bed-slot denominator fails."
        );
        assert!(
            (HOUSES as u32) > pop,
            "at least one house must stand empty, or immigration_verdict refuses \
             no_vacant_house"
        );
        assert!(
            pop.div_ceil(HOUSES as u32) < SLOTS_PER_HOUSE,
            "every occupied house must keep a free slot, or courtship's free_bed_in_house \
             fails for the winning pair's OWN home and no_home returns"
        );

        // ★ AND THE DENOMINATOR ITSELF, WHERE IT ACTUALLY BITES — a correction
        // found by falsification, not by review. Planting the bed-SLOT
        // denominator left everything above GREEN, because on the owner's town
        // the env count binds first: `min(8, 10)` and `min(8, 20)` are both 8,
        // so houses-vs-slots is INERT at his numbers. Everything above would
        // have passed under the denominator this row rejects.
        //
        // Take the env cap off and the two separate. One colonist per HOUSE is
        // the ruling, and it is a bound on the cap for every env count:
        // ★ RULING CHANGED 2026-09-01 (Ben): the unit is the BED, not the
        // house. The old assertion here read "with no env bound the cap must
        // be one colonist per house, not per bed slot" — now the WRONG
        // answer, quoted so the reversal is visible.
        let beds_denom = bastion_housing_cap(usize::MAX, HOUSES) as u32;
        assert_eq!(
            beds_denom,
            HOUSES as u32 * BEDS_PER_HOUSE_AT_FOUNDING as u32,
            "with no env bound the cap is one colonist per BED"
        );
        assert!(beds_denom > HOUSES as u32, "a house holds more than one");
        // That is what it would cost, in the witnesses' own terms: with the
        // slot denominator every house is full, so courtship's
        // free_bed_in_house cannot resolve for ANY pair, and the roster sits
        // at twice the growth ceiling.
        let slots_denom = HOUSES as u32 * SLOTS_PER_HOUSE;
        assert!(slots_denom.div_ceil(HOUSES as u32) >= SLOTS_PER_HOUSE);
        assert!(slots_denom > TARGET_POP);
        // ★ RULING CHANGED 2026-09-01: the old line asserted a FREE slot per
        // house (`div_ceil < SLOTS_PER_HOUSE`) so courtship's
        // free_bed_in_house could resolve. Spouses now SHARE a bed, so every
        // bed may be seated and courtship needs no free slot.
        assert_eq!(beds_denom.div_ceil(HOUSES as u32), SLOTS_PER_HOUSE, "every bed seated");
    }

    /// ★ THE HEADROOM IS THE ENV COUNT'S, NOT THE DENOMINATOR'S — stated
    /// because the pin above only found it under mutation, and a future reader
    /// should not have to.
    ///
    /// `beds >= pop` (the drive gate, and the blocker all three witnesses
    /// name) is made reachable by the denominator ROBUSTLY: the cap never
    /// exceeds the house count, and every registered house carries at least
    /// one bed, so `pop <= houses <= beds` holds for every town.
    ///
    /// The two ROSTER arms — `birth_verdict`'s `no_room_in_town` and
    /// `immigration_verdict`'s `roster_at_target` — need strictly more:
    /// `roster < target_pop`, where `immigration_target_pop` IS the house
    /// count. So they clear only when the env `wanted` sits BELOW the house
    /// count, which is true on the owner's town (8 < 10) and is a property of
    /// the env default, not of the denominator. A town with FEWER houses than
    /// the env count adopts exactly one colonist per house and therefore
    /// starts at its own growth ceiling: the drive reaches Expand, courtship
    /// can fire, and births and immigration correctly refuse until the colony
    /// BUILDS another house.
    ///
    /// That is a defensible equilibrium and not a regression — it is the
    /// ruling working — but it is a real boundary, so it is pinned rather than
    /// left to be rediscovered. It is the number to revisit if a live adopted
    /// town is seen sitting at `roster_at_target` forever.
    #[test]
    fn a_town_with_fewer_houses_than_the_env_count_starts_at_its_ceiling() {
        use bastion_server::bastion_jobs::colony_drive_for;
        use common::bastion::ColonyDrive;

        let houses = 3usize;
        let beds = houses as u32 * SLOTS_PER_HOUSE;
        // ★ RULING CHANGED 2026-09-01: three houses at two beds seat SIX; the
        // env count of 8 binds above that, so the cap is 6.
        let pop = bastion_housing_cap(ENV_WANTED, houses) as u32;
        assert_eq!(pop, 6, "housing binds by BEDS below the env count here");

        // The gate this row is about is still reached, robustly.
        let (drive, term, _) = colony_drive_for(FOOD_PER_CAP, 0, beds, pop, ColonyDrive::Grow);
        assert_eq!((drive, term), (ColonyDrive::Expand, "satisfied"));
        // ★ RULING CHANGED 2026-09-01: the old line asserted a free slot per
        // house so courtship could resolve. Spouses share; every bed seated.
        assert_eq!(pop.div_ceil(houses as u32), SLOTS_PER_HOUSE, "every bed seated");
        // The roster arms sit exactly AT the ceiling — refusing, correctly.
        assert_eq!(pop, beds, "one colonist per BED is the ceiling here (Ben, 2026-09-01)");
    }

    /// PRODUCTION LINE GUARDED: `wanted.min(houses.max(1))` in
    /// [`bastion_housing_cap`]. Idempotence is what makes the caller's
    /// application and the adoption path's re-application ONE producer rather
    /// than two; drop the `.max(1)` and the floor assertion goes red.
    #[test]
    fn the_cap_is_idempotent_and_never_reaches_zero_on_a_housed_town() {
        for w in 0..24usize {
            for h in 0..24usize {
                let c = bastion_housing_cap(w, h);
                assert_eq!(
                    bastion_housing_cap(c, h),
                    c,
                    "w={w} h={h}: re-applying the cap must be a no-op, or the caller and \
                     the adoption path are two producers of one number"
                );
                assert!(c <= w, "w={w} h={h}: the env count stays an upper bound");
                // ★ RULING CHANGED 2026-09-01: the denominator is BEDS —
                // houses times worldgen's beds per house — with the
                // zero-house floor of ONE kept exactly as shipped.
                let ceiling = if h == 0 { 1 } else { h * BEDS_PER_HOUSE_AT_FOUNDING };
                assert!(c <= ceiling, "w={w} h={h}: beds are the denominator");
            }
        }
        assert_eq!(bastion_housing_cap(ENV_WANTED, HOUSES), 8, "the owner's town");
        // ★ RULING CHANGED 2026-09-01: three houses at two beds each seat six.
        assert_eq!(bastion_housing_cap(8, 3), 6, "three houses, six beds, six colonists");
        assert_eq!(
            bastion_housing_cap(8, 0),
            1,
            "the shipped floor, carried over unchanged: a colony of zero is not a colony"
        );
    }

    /// PRODUCTION LINE GUARDED: `ranked.sort_by_key(|(rank, id)| (*rank,
    /// *id))` in [`bastion_adoption_pick`]. The far-away villagers carry the
    /// LOWEST ids here, so a pick that ignored rank — or that leaned on input
    /// order — keeps them and fails.
    #[test]
    fn selection_is_deterministic_and_prefers_residents_near_the_houses() {
        let mut input: Vec<(i64, u32)> = (0..5u32).map(|i| (10_000 + i as i64, i)).collect();
        input.extend((100..105u32).map(|i| (i as i64 - 100, i)));
        let kept = bastion_adoption_pick(input.clone(), 5);
        assert_eq!(
            kept,
            vec![100, 101, 102, 103, 104],
            "the five standing in the village must be adopted over the five out on the roads"
        );
        // ORDER-INDEPENDENT OVER THE INPUT SET — the property a slotmap-backed
        // source needs, since `Site.population`'s iteration order is not a
        // promise and neither is a specs join's.
        for shift in 0..input.len() {
            let mut rot = input.clone();
            rot.rotate_left(shift);
            assert_eq!(
                bastion_adoption_pick(rot, 5),
                kept,
                "shift={shift}: who lives in the town must not depend on iteration order"
            );
        }
        // Ties on rank fall through to NpcId, so the order is TOTAL.
        assert_eq!(bastion_adoption_pick(vec![(7, 30), (7, 10), (7, 20)], 2), vec![10, 20]);
    }

    /// VALIDATE THE INSTRUMENT FIRST. PRODUCTION LINE GUARDED: the `.min()`
    /// over `house_plots` in [`bastion_adoption_rank`]. A rank that returned a
    /// constant would let the pin above pass on the NpcId tiebreak alone and
    /// say nothing about proximity.
    #[test]
    fn the_rank_measures_distance_to_the_nearest_house() {
        let houses = vec![Vec3::new(0.0, 0.0, 40.0), Vec3::new(100.0, 0.0, 40.0)];
        // NEAREST, not first: this villager stands beside house two.
        assert_eq!(bastion_adoption_rank(Vec3::new(97.0, 4.0, 41.0), &houses), 9 + 16);
        assert_eq!(bastion_adoption_rank(Vec3::new(3.0, 4.0, 41.0), &houses), 9 + 16);
        // Height is not distance — a villager on the roof is home.
        assert_eq!(bastion_adoption_rank(Vec3::new(0.0, 0.0, 999.0), &houses), 0);
        // A town that resolved no house plots ranks everyone alike, so the
        // NpcId tiebreak alone decides and the order is still total.
        assert_eq!(bastion_adoption_rank(Vec3::new(3.0, 4.0, 41.0), &[]), i64::MAX);
    }

    /// A MOVER'S REFUSAL MUST NOT REARM ITS RIVAL. PRODUCTION LINE GUARDED:
    /// `bastion_housing_cap(wanted, houses)` INSIDE
    /// [`bastion_settle_plan_capped`] — swap it for the bare `wanted` and the
    /// first assertion goes red.
    ///
    /// ★ THIS PIN WAS GREEN AND FAKE ON ITS FIRST WRITING, and that is the
    /// more useful half of it. The rule then lived in an ARGUMENT at the call
    /// site (`settle_plan(adopted_existing, cap, ..)`), so the pin could only
    /// call `settle_plan` with hand-passed values — it asserted a property of
    /// `settle_plan`, a function this row may not even edit, and would have
    /// stayed green through any mutation of the line it named. The production
    /// code was changed to derive the cap inside a function so that a pin
    /// could reach it; see [`bastion_settle_plan_capped`].
    #[test]
    fn the_cap_does_not_rearm_the_settle_branch() {
        let uncapped_want = SEEN;
        let adopted = bastion_adoption_pick(
            (0..SEEN as u32).map(|i| (i as i64, i)).collect(),
            bastion_housing_cap(uncapped_want, HOUSES),
        )
        .len();
        // ★ RULING CHANGED 2026-09-01: the cap seats one per BED, so ten
        // houses adopt twenty. The old line read `assert_eq!(adopted, HOUSES)`.
        assert_eq!(adopted, HOUSES * BEDS_PER_HOUSE_AT_FOUNDING);
        assert_eq!(
            bastion_settle_plan_capped(adopted, uncapped_want, HOUSES),
            Vec::<usize>::new(),
            "a capped town settles NOBODY — the houses are all spoken for. Measured against \
             an UNCAPPED want, this branch manufactures {} strangers to replace the {} \
             residents the cap just left as villagers.",
            SEEN - HOUSES,
            SEEN - HOUSES
        );
        // FALLBACK IS IDENTITY, against `settle_plan`'s own shipped pin
        // `settle_plan_fills_only_the_shortfall_and_shares_houses`: whenever
        // the want is already within the housing — which is ALWAYS true on the
        // live path, since the caller applies the same producer first — the
        // wrapper is `settle_plan` unchanged. The cap is a ceiling, not a
        // freeze: an under-populated town still settles its shortfall.
        assert_eq!(bastion_settle_plan_capped(0, 3, 5), vec![0, 1, 2]);
        assert_eq!(bastion_settle_plan_capped(2, 5, 8), vec![0, 1, 2]);
    }
}

