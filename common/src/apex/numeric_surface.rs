//! `APEX-T6.1` — numeric attack-surface inventory.
//!
//! T6's premise is that determinism stops being a property of our code
//! and becomes a property of the machine the moment a transcendental
//! result drives a branch. Before anything can be certified or replaced,
//! the surface has to be known and prevented from growing silently.
//!
//! **`T6.1a`** delivers the file-level tripwire, in the shape `T3.5.19`'s
//! bypass scanner proved: every file in the authoritative simulation
//! crates that performs a root, power or trigonometric operation is
//! classified, and an unclassified one fails the build. Granularity is
//! per FILE for the reason the disconnect inventory gives: line
//! positions drift with every unrelated edit and rot into noise, while a
//! file's ROLE is stable.
//!
//! **`T6.1b`** delivers the per-SITE half the row's acceptance criterion
//! actually names: owner, reach and protocol status for every site in
//! those files ([`NUMERIC_SITES`]). Sites are keyed by a distinctive
//! substring of their own expression, not by line number, and the
//! per-file line count is pinned so a NEW site fails the build.
//!
//! Two findings from `T6.1b` changed `T6.1a`'s own output and are
//! recorded here rather than folded in silently:
//!
//! 1. The `T6.1a` pattern list missed `acos`/`asin`/`atan`/`tan`/`log`
//!    entirely. Seven sites and two whole files
//!    (`comp/inventory/item/tool.rs`, `systems/melee.rs` — the latter a
//!    hit-test predicate, the strongest reach class there is) were
//!    outside the surface an inventory claimed to cover. The patterns are
//!    widened below and the authoritative file count moved 24 → 26.
//! 2. `sqrt` is not a cross-target hazard. IEEE 754 §5.4.1 requires it to
//!    be correctly rounded, and Rust lowers `f32::sqrt` to the hardware
//!    instruction with no fast-math, so it returns identical bits
//!    everywhere for identical input bits. `powf`/`sin`/`cos`/`ln` carry
//!    no such requirement and are the platform libm's. That distinction
//!    is what separates the two protocol statuses, and it is derived from
//!    the operation rather than asserted per site — see
//!    [`NumericOpV1::protocol_v1`].

use std::{fs, path::Path};

/// What a numeric-surface file is, for determinism purposes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum NumericRoleV1 {
    /// Authoritative simulation: its results reach state the server owns
    /// or that crosses a network, save or hash boundary.
    Authoritative,
    /// Presentation, tooling or diagnostics. Excluded WITH EVIDENCE —
    /// the `T5.4` finding (a "presentational" wind reaching glider
    /// steering) is why an assertion alone is not enough.
    ///
    /// `T6.1a` also carried a `TestSupport` class, for
    /// `apex/source_closure.rs`. `T6.1b`'s comment-stripping scan showed
    /// that file (and `clock.rs`) only ever MENTIONED the operations in
    /// prose, so both left the surface and the class had no members. It
    /// is not kept as an empty variant: a class nobody is in is a place
    /// for a future file to be filed without argument.
    PresentationOrTooling,
}

/// Every file in the authoritative crates touching a root, power or
/// trigonometric operation, with what it is and why.
pub(crate) const NUMERIC_SURFACE_ROLES: &[(&str, NumericRoleV1, &str)] = &[
    ("common/src/combat.rs", NumericRoleV1::Authoritative, "damage/knockback scaling reaches health and physics"),
    ("common/src/comp/ability.rs", NumericRoleV1::Authoritative, "ability scaling feeds combat"),
    ("common/src/comp/buff.rs", NumericRoleV1::Authoritative, "buff strength curve (powf) feeds combat and movement"),
    ("common/src/comp/fluid_dynamics.rs", NumericRoleV1::Authoritative, "drag/lift powf drives glider and projectile motion"),
    ("common/src/comp/inventory/item/tool.rs", NumericRoleV1::Authoritative, "weapon buff-strength curve reaches combat and is derived from persisted item state"),
    ("common/src/comp/ori.rs", NumericRoleV1::Authoritative, "orientation normalisation is synced state"),
    ("common/src/comp/projectile.rs", NumericRoleV1::Authoritative, "projectile kinematics"),
    ("common/src/comp/skillset/mod.rs", NumericRoleV1::Authoritative, "skill-point curve is persisted state"),
    ("common/src/path.rs", NumericRoleV1::Authoritative, "pathfinding heuristics decide NPC movement"),
    ("common/src/region.rs", NumericRoleV1::Authoritative, "region membership decides what is synced to whom"),
    ("common/src/resources.rs", NumericRoleV1::Authoritative, "time/scale resources feed every tick"),
    ("common/src/states/basic_aura.rs", NumericRoleV1::Authoritative, "aura radius decides who is affected"),
    ("common/src/states/basic_summon.rs", NumericRoleV1::Authoritative, "summon placement is authoritative spawn position"),
    ("common/src/states/dash_melee.rs", NumericRoleV1::Authoritative, "dash kinematics"),
    ("common/src/states/glide_wield.rs", NumericRoleV1::Authoritative, "glider orientation feeds flight"),
    ("common/src/states/rapid_ranged.rs", NumericRoleV1::Authoritative, "projectile launch parameters"),
    ("common/src/states/utils.rs", NumericRoleV1::Authoritative, "movement scaling powf reaches position"),
    ("common/src/terrain/map.rs", NumericRoleV1::PresentationOrTooling, "map image sampling for the client map view; worldgen owns the authoritative geometry"),
    ("common/src/time.rs", NumericRoleV1::Authoritative, "calendar/day-cycle arithmetic is synced"),
    ("common/src/util/color.rs", NumericRoleV1::PresentationOrTooling, "colour space conversion, rendering only"),
    ("common/src/util/dir.rs", NumericRoleV1::Authoritative, "Dir normalisation is used by orientation and aiming"),
    ("common/src/util/find_dist.rs", NumericRoleV1::Authoritative, "distance predicates gate interactions"),
    ("common/systems/src/melee.rs", NumericRoleV1::Authoritative, "melee hit-cone predicate: the atan IS the comparison that decides who is hit"),
    ("common/systems/src/phys/collision.rs", NumericRoleV1::Authoritative, "collision resolution"),
    ("common/systems/src/phys/mod.rs", NumericRoleV1::Authoritative, "the physics tick itself; T6.3's ordering row lives here"),
    ("common/systems/src/phys/weather.rs", NumericRoleV1::Authoritative, "wind forces reach flight; see T5.4 on the presentation/authority split"),
    ("common/systems/src/projectile.rs", NumericRoleV1::Authoritative, "projectile system"),
    ("common/systems/src/shockwave.rs", NumericRoleV1::Authoritative, "shockwave geometry decides who is hit"),
];

/// The operations that make a file part of the surface.
///
/// Widened by `T6.1b`: the inverse trigonometrics, `tan`, `exp` and the
/// arbitrary-base `log` were absent, so two files and seven sites sat
/// outside an inventory that claimed to be complete. A pattern list is a
/// coverage CLAIM, and this one was wrong.
pub(crate) const NUMERIC_SURFACE_PATTERNS: [&str; 21] = [
    "powf", "sqrt()", ".sin()", ".cos()", ".ln()", "hypot", ".acos()", ".asin()", ".atan()",
    ".atan2(", ".tan()", ".exp()", ".cbrt()", ".log(", ".log2()", ".log10()", ".exp_m1()",
    ".ln_1p()", ".sinh()", ".cosh()", ".tanh()",
];

/// Branch-driving `powf` call sites, seeded from the T6 tier spec's own
/// reads. This is the START of `T6.1b`'s owned inventory, not its
/// completion — see the module doc.
pub(crate) const BRANCH_DRIVING_SEED: &[(&str, &str)] = &[
    ("common/src/comp/fluid_dynamics.rs", "drag coefficient: ar.powf(0.68)"),
    ("common/src/comp/fluid_dynamics.rs", "scale.powf(2.0) in the force sum"),
    ("common/src/comp/fluid_dynamics.rs", "(PI/6 * dim).powf(2.0/3.0)"),
    ("common/src/states/utils.rs", "scale.powf(13.0).powf(0.25) movement scaling"),
    ("common/src/comp/buff.rs", "f32::powf(1.0 - nn_scaling(strength), 1.1)"),
];

/// Matching non-comment lines in `text`.
///
/// Comments are stripped because `path.rs` carries commented-out trig and
/// several files describe the operation they are about to perform. A
/// mention is not a call, and an inventory that cannot tell them apart
/// pins noise.
pub(crate) fn matching_lines_v1(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            let code = line.split("//").next().unwrap_or("");
            NUMERIC_SURFACE_PATTERNS.iter().any(|p| code.contains(p))
        })
        .count()
}

// ---------------------------------------------------------------------
// `T6.1b` — per-site owner, reach and protocol status.
// ---------------------------------------------------------------------

/// The operation performed, which is what decides the protocol status.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum NumericOpV1 {
    /// `sqrt` and NOTHING else. IEEE 754 §5.4.1 requires square root to
    /// be correctly rounded; `f32::sqrt` lowers to the hardware
    /// instruction and Rust does not enable fast-math, so the result is
    /// bit-identical across conforming targets for identical input bits.
    ///
    /// `cbrt` and `hypot` are libm functions with no such requirement and
    /// must NOT be classified here — they are [`NumericOpV1::Power`].
    SquareRoot,
    /// `powf`, `exp`, `cbrt`, `hypot` — the platform libm's.
    Power,
    /// `sin`/`cos`/`tan` and the inverses — the platform libm's.
    Trig,
    /// `ln`/`log`/`log2`/`log10` — the platform libm's.
    Log,
}

/// What the numeric protocol can say about a site.
///
/// There are two variants and **there is deliberately no third**. A
/// "certified cross-target" status would need a certified kernel to point
/// at, and `T6.5` has not been built; a variant for it here would let a
/// site claim a guarantee that does not exist anywhere in the tree. When
/// `T6.5` lands, the variant lands with it.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProtocolStatusV1 {
    /// Reproducible within one build of one target and no further. This
    /// is the ceiling for every site in the tree today; `T6.4`'s build
    /// tuple is what will make even this claim checkable rather than
    /// assumed.
    SameBuildOnly,
    /// `SameBuildOnly`, and additionally a named `T6.5` substitution
    /// candidate because the operation is the platform libm's — the
    /// result is implementation-defined, so two conforming targets may
    /// disagree in the last place and the disagreement propagates.
    KernelCandidate,
}

impl ProtocolStatusV1 {
    pub(crate) const ALL: [Self; 2] = [Self::SameBuildOnly, Self::KernelCandidate];
}

impl NumericOpV1 {
    /// Derived, never stored. Pairing a site with the wrong protocol
    /// status is unrepresentable rather than merely tested for: the
    /// operation decides.
    pub(crate) const fn protocol_v1(self) -> ProtocolStatusV1 {
        match self {
            Self::SquareRoot => ProtocolStatusV1::SameBuildOnly,
            Self::Power | Self::Trig | Self::Log => ProtocolStatusV1::KernelCandidate,
        }
    }
}

/// The subsystem that owns the arithmetic — who is called when the site
/// has to change.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum NumericOwnerV1 {
    Combat,
    Movement,
    Orientation,
    FlightAndFluid,
    PhysicsTick,
    Projectiles,
    Pathfinding,
    Progression,
    AreaOfEffect,
    Spawning,
    WorldSync,
    TimeOfDay,
}

impl NumericOwnerV1 {
    /// Every owner. An owner with no sites is a naming exercise, and the
    /// test below says so.
    pub(crate) const ALL: [Self; 12] = [
        Self::Combat,
        Self::Movement,
        Self::Orientation,
        Self::FlightAndFluid,
        Self::PhysicsTick,
        Self::Projectiles,
        Self::Pathfinding,
        Self::Progression,
        Self::AreaOfEffect,
        Self::Spawning,
        Self::WorldSync,
        Self::TimeOfDay,
    ];
}

/// How far a difference at this site travels. Ordered by severity, and
/// judged by the site's IMMEDIATE consumer: almost everything reaches a
/// comparison eventually, so "branch-driving" would classify the whole
/// tree if taken transitively. The immediate consumer is also where a
/// `T6.2` probe can actually be sited.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub(crate) enum NumericReachV1 {
    /// The result is consumed by a comparison or predicate that selects a
    /// code path or a membership set. This is the only class where one
    /// ulp becomes an arbitrarily large difference in one step.
    BranchCondition,
    /// The result lands in velocity, position, orientation, health or
    /// other state carried into the next tick or a save. Error
    /// accumulates rather than amplifying.
    CarriedAcrossTicks,
    /// The site has no live consumer at this tip. Classified by the state
    /// it reads, not by a consumer, and it must be re-classified when one
    /// appears.
    NoLiveConsumer,
}

/// One numeric site.
pub(crate) struct NumericSiteV1 {
    /// Repo-relative file, forward-slashed. Must be `Authoritative`.
    pub(crate) file: &'static str,
    /// A distinctive substring of the site's own expression. Verified to
    /// still occur in the file, so an edited-away site fails here instead
    /// of rotting into a stale claim. Not a line number: those drift.
    pub(crate) key: &'static str,
    /// How many matching non-comment LINES this entry accounts for. Lines,
    /// not calls — one line may hold two calls (`utils.rs`'s paired
    /// `atan`), and the scanner counts lines.
    pub(crate) lines: usize,
    pub(crate) op: NumericOpV1,
    pub(crate) owner: NumericOwnerV1,
    pub(crate) reach: NumericReachV1,
    /// What the immediate consumer is. For `BranchCondition` this must
    /// name the comparison; "it's used in physics" is not a reach
    /// argument, it is a restatement of the file's role.
    pub(crate) why: &'static str,
}

const fn site(
    file: &'static str,
    key: &'static str,
    lines: usize,
    op: NumericOpV1,
    owner: NumericOwnerV1,
    reach: NumericReachV1,
    why: &'static str,
) -> NumericSiteV1 {
    NumericSiteV1 { file, key, lines, op, owner, reach, why }
}

/// Every site in every `Authoritative` file. The `lines` column sums, per
/// file, to what the scanner finds — so a new site fails the build.
pub(crate) const NUMERIC_SITES: &[NumericSiteV1] = {
    use NumericOpV1::{Log, Power, SquareRoot, Trig};
    use NumericOwnerV1::*;
    use NumericReachV1::{BranchCondition, CarriedAcrossTicks, NoLiveConsumer};
    &[
        site("common/src/combat.rs", "Self::Sqrt => (val / norm).sqrt()", 1, SquareRoot, Combat, CarriedAcrossTicks,
             "damage-scaling curve; the result reaches Health, which is persisted"),
        site("common/src/comp/ability.rs", "data.body.dimensions().z.sqrt()", 1, SquareRoot, Combat, CarriedAcrossTicks,
             "body-scale factor applied to ability effects"),
        site("common/src/comp/ability.rs", "max_angle.to_radians().tan()", 1, Trig, Combat, BranchCondition,
             "end_radius of a shockwave/melee cone; the radius is compared against target distance to decide who is hit"),
        site("common/src/comp/buff.rs", "f32::powf(1.0 - nn_scaling(data.strength), 1.1)", 1, Power, Combat, CarriedAcrossTicks,
             "MovementSpeed buff strength multiplies velocity every tick"),
        site("common/src/comp/fluid_dynamics.rs", "rel_flow.0 / v_sq.sqrt()", 1, SquareRoot, FlightAndFluid, CarriedAcrossTicks,
             "relative-flow direction feeds the aerodynamic force sum"),
        site("common/src/comp/fluid_dynamics.rs", "ar.powf(0.68)", 1, Power, FlightAndFluid, CarriedAcrossTicks,
             "Oswald efficiency from aspect ratio; feeds induced drag, hence glider velocity"),
        site("common/src/comp/fluid_dynamics.rs", "scale.powf(2.0)", 1, Power, FlightAndFluid, CarriedAcrossTicks,
             "planform-area scaling in the lift/drag sum"),
        site("common/src/comp/fluid_dynamics.rs", "(PI / 6.0 * dim.x * dim.y * dim.z).powf(2.0 / 3.0)", 3, Power, FlightAndFluid, CarriedAcrossTicks,
             "body reference area for drag; three bodies compute it identically"),
        site("common/src/comp/fluid_dynamics.rs", "a0 * sweep.cos()", 1, Trig, FlightAndFluid, CarriedAcrossTicks,
             "swept-wing lift-slope correction"),
        site("common/src/comp/fluid_dynamics.rs", "(1.0 + x.powi(2)).sqrt() + x", 2, SquareRoot, FlightAndFluid, CarriedAcrossTicks,
             "finite-wing lift slope, two aspect-ratio branches"),
        site("common/src/comp/inventory/item/tool.rs", "(self.buff_strength - base + 1.0).log(5.0)", 1, Log, Combat, CarriedAcrossTicks,
             "diminishing-returns curve on weapon buff strength, derived from persisted item state"),
        site("common/src/comp/ori.rs", "((1.0 + x) / 2.0).sqrt()", 1, SquareRoot, Orientation, CarriedAcrossTicks,
             "half-angle quaternion scalar; Ori is synced state"),
        site("common/src/comp/ori.rs", "((1.0 - x) / 2.0).sqrt()", 1, SquareRoot, Orientation, CarriedAcrossTicks,
             "half-angle quaternion vector; Ori is synced state"),
        site("common/src/comp/ori.rs", "between.w.clamp(-1.0, 1.0).acos()", 1, Trig, Orientation, CarriedAcrossTicks,
             "angle between orientations, used to drive turning"),
        site("common/src/comp/projectile.rs", ".sqrt()", 2, SquareRoot, Projectiles, CarriedAcrossTicks,
             "aim_projectile's ballistic discriminant, one site per arc; the result is a launch direction"),
        site("common/src/comp/skillset/mod.rs", "E.powf(0.025 * level)", 1, Power, Progression, BranchCondition,
             "experience required for the next level; compared against accumulated XP to decide whether a level-up happens"),
        site("common/src/comp/skillset/mod.rs", "E.powf(-SCALING_FACTOR * level as f32)", 1, Power, Progression, BranchCondition,
             "skill-point cost curve, compared against available points"),
        site("common/src/path.rs", "nd.sqrt()", 1, SquareRoot, Pathfinding, BranchCondition,
             "A* flee-heuristic term; heuristics are compared to ORDER the open set, so a tie broken differently is a different path"),
        site("common/src/path.rs", "linear_eccentricity.powi(2)).powf(0.5)", 1, Power, Pathfinding, CarriedAcrossTicks,
             "prolate-spheroid semi-axis. NOTE: this is sqrt written as a libm pow call — the cheapest KernelCandidate in the tree to retire, by writing sqrt"),
        site("common/src/path.rs", "rtheta.cos()", 3, Trig, Pathfinding, CarriedAcrossTicks,
             "point sampled on the spheroid surface; becomes an NPC waypoint"),
        site("common/src/path.rs", "theta.cos()", 9, Trig, Pathfinding, CarriedAcrossTicks,
             "axis-angle rotation matrix rotating that waypoint into world space"),
        site("common/src/path.rs", "(dz / radius).acos()", 1, Trig, Pathfinding, CarriedAcrossTicks,
             "polar angle of the sampled waypoint"),
        site("common/src/region.rs", "TETHER_LENGTH as f32 * 2.0f32.sqrt()", 2, SquareRoot, WorldSync, BranchCondition,
             "extended view distance, compared against squared region distance to decide WHAT IS SYNCED TO WHOM"),
        site("common/src/resources.rs", "-angle_rad.sin()", 1, Trig, TimeOfDay, CarriedAcrossTicks,
             "get_sun_dir. Reads as presentation and voxygen does use it — but phys/weather.rs:69 takes it for thermal lift, so it reaches velocity. The T5.4 pattern exactly, found by tracing the consumer instead of the name"),
        site("common/src/states/basic_aura.rs", "(self.static_data.combo_at_cast.max(1) as f32).sqrt()", 1, SquareRoot, AreaOfEffect, CarriedAcrossTicks,
             "combo scaling on aura strength, which becomes a Buff"),
        site("common/src/states/basic_summon.rs", "(summon_frac * 2.0 * PI).sin()", 4, Trig, Spawning, CarriedAcrossTicks,
             "ring placement of summoned entities; a spawn position is authoritative state"),
        site("common/src/states/basic_summon.rs", "phi + xy_angle", 2, Trig, Spawning, CarriedAcrossTicks,
             "beam-pillar target positions around the summoner"),
        site("common/src/states/dash_melee.rs", "charge_frac.sqrt()", 1, SquareRoot, Movement, CarriedAcrossTicks,
             "dash forward speed, applied to velocity"),
        site("common/src/states/glide_wield.rs", "scale.sqrt()", 1, SquareRoot, FlightAndFluid, CarriedAcrossTicks,
             "glider chord length, which sets the aerodynamic model's inputs"),
        site("common/src/states/rapid_ranged.rs", "rng.random::<f32>().sqrt()", 1, SquareRoot, Spawning, CarriedAcrossTicks,
             "uniform-disk radius for projectile spawn offset"),
        site("common/src/states/rapid_ranged.rs", "r * theta.sin()", 1, Trig, Spawning, CarriedAcrossTicks,
             "the same offset's xy components"),
        site("common/src/states/utils.rs", "(1.0 - FRIC_GROUND).ln()", 1, Log, Movement, CarriedAcrossTicks,
             "max_speed_approx; rtsim/src/rule/simulate_npcs.rs advances PERSISTED npc positions with it. The argument is a constant, so this hazard is removable outright rather than certifiable — write the value"),
        site("common/src/states/utils.rs", "data.scale.map_or(1.0, |s| s.0.sqrt())", 6, SquareRoot, Movement, CarriedAcrossTicks,
             "body-scale factor on acceleration and turn rate, six movement modes"),
        site("common/src/states/utils.rs", "(1.0 - data.body.ori_damping())).sqrt()", 1, SquareRoot, Orientation, CarriedAcrossTicks,
             "slerp angle factor; the result is the turn fraction applied to Ori"),
        site("common/src/states/utils.rs", "submersion.clamp(0.0, 1.0).sqrt()", 1, SquareRoot, Movement, CarriedAcrossTicks,
             "swim-depth scaling on movement force"),
        site("common/src/states/utils.rs", "s.0.powf(13.0).powf(0.25)", 2, Power, Movement, CarriedAcrossTicks,
             "jump-impulse scale factor, two jump paths; the impulse is applied to velocity"),
        site("common/src/states/utils.rs", "x_diff.atan()", 1, Trig, Orientation, CarriedAcrossTicks,
             "wall-normal orientation while climbing; two atan calls on one line"),
        site("common/src/time.rs", "* std::f64::consts::TAU).cos()", 1, Trig, TimeOfDay, NoLiveConsumer,
             "season_bias has no consumer at this tip; it reads synced TimeOfDay, so it becomes CarriedAcrossTicks the moment one appears"),
        site("common/src/util/dir.rs", "Vec3::new(a.cos(), a.sin(), 0.0)", 1, Trig, Orientation, CarriedAcrossTicks,
             "Dir from a z-angle, used for aiming and facing"),
        site("common/src/util/find_dist.rs", "(z_dist.powi(2) + xy_dist.powi(2)).sqrt()", 3, SquareRoot, AreaOfEffect, BranchCondition,
             "min_distance for three shape pairs; every caller compares it against a range to gate an interaction"),
        site("common/systems/src/melee.rs", "(rad_b / pos2.distance(pos_b2)).atan()", 1, Trig, Combat, BranchCondition,
             "the atan IS inside the `angle_between(..) < max_angle + ..` hit predicate — a libm trig function on the boundary of who takes damage"),
        site("common/systems/src/phys/collision.rs", "(1.0 - longitudinal_friction).powf(", 1, Power, PhysicsTick, CarriedAcrossTicks,
             "longitudinal friction factor, exponent is dt — a libm pow on the physics hot path"),
        site("common/systems/src/phys/collision.rs", "(1.0 - lateral_friction).powf(", 1, Power, PhysicsTick, CarriedAcrossTicks,
             "lateral friction factor, exponent is dt"),
        site("common/systems/src/phys/collision.rs", "new_longitudinal_squared.abs().sqrt()", 1, SquareRoot, PhysicsTick, CarriedAcrossTicks,
             "post-friction longitudinal speed"),
        site("common/systems/src/phys/collision.rs", "(1.0 - fric.min(1.0) * fric_mod).powf(", 1, Power, PhysicsTick, CarriedAcrossTicks,
             "ground friction applied directly to velocity"),
        site("common/systems/src/phys/mod.rs", ".powf(0.75)", 1, Power, PhysicsTick, CarriedAcrossTicks,
             "liquid drag coefficient"),
        site("common/systems/src/phys/mod.rs", "(1.0 / (1.0 + fric)).powf(dt.0 * 10.0)", 1, Power, PhysicsTick, CarriedAcrossTicks,
             "liquid drag applied to velocity"),
        site("common/systems/src/phys/mod.rs", "(flat_radius.powi(2) + half_height.powi(2)).sqrt()", 2, SquareRoot, PhysicsTick, BranchCondition,
             "collision_boundary; :512 compares the summed boundary against squared distance to decide which pairs are BROAD-PHASE CANDIDATES — the membership decision DET-PHY-005 canonicalises the order of"),
        site("common/systems/src/phys/mod.rs", "(1.0 - fric).powf(read.dt.0 * 60.0)", 1, Power, PhysicsTick, CarriedAcrossTicks,
             "per-tick friction on velocity"),
        site("common/systems/src/phys/weather.rs", "1.3f32.powf(", 1, Power, FlightAndFluid, CarriedAcrossTicks,
             "ridge-lift altitude falloff; reaches glider velocity (T5.4's finding)"),
        site("common/systems/src/phys/weather.rs", "0.96f32.powf(", 1, Power, FlightAndFluid, CarriedAcrossTicks,
             "wind altitude factor; reaches glider velocity"),
        site("common/systems/src/projectile.rs", "theta.cos()", 3, Trig, Projectiles, CarriedAcrossTicks,
             "firework burst directions; each becomes a spawned projectile's velocity"),
        site("common/systems/src/projectile.rs", "theta.sin() * phi.sin()", 3, Trig, Projectiles, CarriedAcrossTicks,
             "projectile-split spread directions"),
        site("common/systems/src/shockwave.rs", "(disk1.radius.powi(2) - x.powi(2)).sqrt()", 1, SquareRoot, AreaOfEffect, BranchCondition,
             "disk-intersection points defining the shockwave arc, which decides who is inside it"),
        site("common/systems/src/shockwave.rs", "(d.radius / dist).asin()", 1, Trig, AreaOfEffect, BranchCondition,
             "angular half-width of a target disk, compared against the shockwave's angular extent"),
    ]
};

pub(crate) fn scan_numeric_surface_v1(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(&root.join("common/src"), &mut files);
    walk(&root.join("common/systems/src"), &mut files);

    let mut hits: Vec<String> = files
        .into_iter()
        .filter(|path| fs::read_to_string(path).is_ok_and(|text| matching_lines_v1(&text) > 0))
        .filter_map(|path| {
            let rel = path.strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/");
            // This inventory NAMES the operations; it does not perform
            // them. Same quoter-not-doer rule the disconnect scanner uses.
            (!rel.ends_with("numeric_surface.rs")).then_some(rel)
        })
        .collect();
    hits.sort();
    hits
}

#[cfg(test)]
mod numeric_surface_v1 {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        // CARGO_MANIFEST_DIR is <root>/common.
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("common has a parent").to_path_buf()
    }

    /// `T6.1`: the surface is fully classified, and a new numeric site
    /// fails the build rather than appearing quietly.
    #[test]
    fn every_numeric_surface_file_is_classified() {
        let scanned = scan_numeric_surface_v1(&repo_root());
        assert!(!scanned.is_empty(), "the scan found nothing — it is broken, not the tree");

        let claimed: std::collections::BTreeSet<&str> =
            NUMERIC_SURFACE_ROLES.iter().map(|(f, _, _)| *f).collect();
        let found: std::collections::BTreeSet<&str> = scanned.iter().map(String::as_str).collect();

        let unclaimed: Vec<&&str> = found.difference(&claimed).collect();
        assert!(
            unclaimed.is_empty(),
            "unclassified numeric-surface files (say what they are, with evidence for any \
             presentation-only exclusion):\n{unclaimed:#?}"
        );
        let vanished: Vec<&&str> = claimed.difference(&found).collect();
        assert!(vanished.is_empty(), "these files no longer touch the surface; drop them:\n{vanished:#?}");
    }

    /// Every exclusion carries evidence. `T5.4` is why: a value that
    /// looked presentational reached glider steering, so "it's only for
    /// display" is a claim that has to be argued, not asserted.
    #[test]
    fn presentation_exclusions_carry_evidence() {
        for (file, role, why) in NUMERIC_SURFACE_ROLES {
            assert!(!why.trim().is_empty(), "{file} has no stated reason");
            if *role == NumericRoleV1::PresentationOrTooling {
                assert!(
                    why.len() > 20,
                    "{file} is excluded from authority on a one-word claim: {why:?}"
                );
            }
        }
    }

    /// The authoritative set is the majority of the surface, and the
    /// physics tick is in it — T6.3's ordering row depends on that being
    /// true.
    #[test]
    fn the_authoritative_set_is_pinned() {
        let authoritative = NUMERIC_SURFACE_ROLES
            .iter()
            .filter(|(_, role, _)| *role == NumericRoleV1::Authoritative)
            .count();
        // 24 at T6.1a; 26 once T6.1b widened the pattern list and the
        // inverse trigonometrics pulled in tool.rs and melee.rs.
        assert_eq!(authoritative, 26, "the authoritative surface changed — re-derive T6.1b's owners");
        assert!(
            NUMERIC_SURFACE_ROLES.iter().any(|(f, role, _)| *f == "common/systems/src/phys/mod.rs"
                && *role == NumericRoleV1::Authoritative),
            "the physics tick must be authoritative or T6.3 is aimed at nothing"
        );
    }

    /// `T6.1b`'s seed is real: every branch-driving file named is one the
    /// scan actually classifies Authoritative.
    #[test]
    fn the_branch_driving_seed_sits_inside_the_authoritative_set() {
        for (file, what) in BRANCH_DRIVING_SEED {
            assert!(!what.trim().is_empty(), "{file} seed entry says nothing");
            let role = NUMERIC_SURFACE_ROLES
                .iter()
                .find(|(f, _, _)| f == file)
                .map(|(_, role, _)| *role)
                .unwrap_or_else(|| panic!("{file} is seeded but not classified"));
            assert_eq!(role, NumericRoleV1::Authoritative, "{file} is seeded but not authoritative");
        }
    }

    // ---------------- T6.1b ----------------

    /// The row's acceptance criterion: every site in every authoritative
    /// file has an owner and a protocol status, and the per-file line
    /// counts sum to what the scanner finds. A NEW numeric site in an
    /// already-classified file fails HERE — that is the tripwire T6.1a
    /// could not provide at file granularity.
    #[test]
    fn every_authoritative_file_is_fully_accounted_for_site_by_site() {
        let root = repo_root();
        let mut missing = Vec::new();
        let mut mismatched = Vec::new();

        for (file, role, _) in NUMERIC_SURFACE_ROLES {
            if *role != NumericRoleV1::Authoritative {
                continue;
            }
            let claimed: usize =
                NUMERIC_SITES.iter().filter(|s| s.file == *file).map(|s| s.lines).sum();
            if claimed == 0 {
                missing.push(*file);
                continue;
            }
            let text = fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
            let found = matching_lines_v1(&text);
            if claimed != found {
                mismatched.push(format!("{file}: inventory claims {claimed} lines, scan finds {found}"));
            }
        }

        assert!(missing.is_empty(), "authoritative files with no site inventory:\n{missing:#?}");
        assert!(
            mismatched.is_empty(),
            "site inventory is out of date — a numeric site was added or removed. Classify it \
             (owner, reach, why) rather than adjusting the count:\n{mismatched:#?}"
        );
    }

    /// Every site is in a file the classification calls authoritative, and
    /// its key still occurs there. A key that stops matching means the
    /// expression moved or changed; the entry has to be re-derived, not
    /// re-anchored.
    #[test]
    fn every_site_key_still_occurs_in_its_file() {
        let root = repo_root();
        for s in NUMERIC_SITES {
            let role = NUMERIC_SURFACE_ROLES
                .iter()
                .find(|(f, _, _)| *f == s.file)
                .map(|(_, role, _)| *role)
                .unwrap_or_else(|| panic!("{} is inventoried but not classified", s.file));
            assert_eq!(role, NumericRoleV1::Authoritative, "{} is inventoried but not authoritative", s.file);
            assert!(s.lines > 0, "{} / {:?} accounts for no lines", s.file, s.key);

            let text = fs::read_to_string(root.join(s.file)).unwrap_or_else(|e| panic!("{}: {e}", s.file));
            assert!(
                text.contains(s.key),
                "{} no longer contains {:?} — re-derive the entry from the code",
                s.file,
                s.key
            );
        }
    }

    /// Every site says what consumes it, and a `BranchCondition` says
    /// which comparison. "It's used in physics" restates the file's role;
    /// the reach claim is about the immediate consumer.
    #[test]
    fn every_site_names_its_consumer() {
        for s in NUMERIC_SITES {
            assert!(!s.why.trim().is_empty(), "{} / {:?} says nothing", s.file, s.key);
            if s.reach == NumericReachV1::BranchCondition {
                assert!(
                    s.why.len() > 40,
                    "{} / {:?} claims branch-driving reach without naming the comparison: {:?}",
                    s.file,
                    s.key,
                    s.why
                );
            }
        }
    }

    /// `sqrt` is correctly rounded and is NOT a kernel candidate; the
    /// libm functions are. The pairing is derived from the operation, so
    /// this test guards the classification of the OPERATION, which is the
    /// only thing left to get wrong.
    #[test]
    fn correctly_rounded_and_libm_operations_are_not_confused() {
        for s in NUMERIC_SITES {
            if s.op == NumericOpV1::SquareRoot {
                assert!(
                    s.key.contains("sqrt") || s.key.contains(".sqrt()"),
                    "{} / {:?} is classified SquareRoot but does not call sqrt",
                    s.file,
                    s.key
                );
                assert!(
                    !s.key.contains("hypot") && !s.key.contains("cbrt"),
                    "{} / {:?}: hypot and cbrt are libm, not correctly-rounded roots",
                    s.file,
                    s.key
                );
                assert_eq!(s.op.protocol_v1(), ProtocolStatusV1::SameBuildOnly);
            } else {
                assert_eq!(
                    s.op.protocol_v1(),
                    ProtocolStatusV1::KernelCandidate,
                    "{} / {:?} performs a libm operation and must be a kernel candidate",
                    s.file,
                    s.key
                );
            }
        }
    }

    /// No site anywhere claims a cross-target guarantee, because none
    /// exists: `T6.5` has not been built. This asserts the SHAPE of the
    /// status type, so adding a certified variant fails here and forces
    /// the certification to be pointed at something real.
    #[test]
    fn no_certified_cross_target_status_exists_yet() {
        assert_eq!(
            ProtocolStatusV1::ALL.len(),
            2,
            "a third protocol status appeared. If it certifies cross-target equality, T6.5's \
             kernel must exist and T6.2's probe must have measured this site — otherwise the \
             type now lets a site overstate what the tree can guarantee"
        );
    }

    /// The branch-driving set is pinned. Shrinking it silently would be a
    /// coverage loss disguised as a cleanup; growing it is a finding.
    #[test]
    fn the_branch_driving_set_is_pinned() {
        let branch: Vec<&str> = NUMERIC_SITES
            .iter()
            .filter(|s| s.reach == NumericReachV1::BranchCondition)
            .map(|s| s.file)
            .collect();
        assert_eq!(
            branch.len(),
            10,
            "the branch-driving set changed; these are the sites where one ulp becomes a \
             different code path:\n{branch:#?}"
        );
        for file in [
            "common/systems/src/melee.rs",
            "common/systems/src/phys/mod.rs",
            "common/src/region.rs",
            "common/src/util/find_dist.rs",
        ] {
            assert!(branch.contains(&file), "{file} must stay in the branch-driving set");
        }
    }

    /// Every owner owns something, and the branch-driving sites — the
    /// ones where an ulp becomes a code path — are spread across six
    /// subsystems rather than concentrated in physics. That is the fact
    /// T6.5 has to plan around: there is no single owner to hand the
    /// tier to.
    #[test]
    fn every_owner_owns_at_least_one_site() {
        for owner in NumericOwnerV1::ALL {
            assert!(
                NUMERIC_SITES.iter().any(|s| s.owner == owner),
                "{owner:?} owns nothing — drop the variant or inventory its sites"
            );
        }
        let mut branch_owners: Vec<NumericOwnerV1> = NUMERIC_SITES
            .iter()
            .filter(|s| s.reach == NumericReachV1::BranchCondition)
            .map(|s| s.owner)
            .collect();
        branch_owners.dedup_by(|a, b| a == b);
        branch_owners.sort_by_key(|o| format!("{o:?}"));
        branch_owners.dedup();
        assert_eq!(
            branch_owners.len(),
            6,
            "the branch-driving sites' owner spread changed: {branch_owners:?}"
        );
    }

    /// `T6.1a`'s seed is subsumed: every file it named is now inventoried
    /// site by site, so the seed cannot outlive the thing it seeded.
    #[test]
    fn the_seed_is_subsumed_by_the_site_table() {
        for (file, _) in BRANCH_DRIVING_SEED {
            assert!(
                NUMERIC_SITES.iter().any(|s| s.file == *file),
                "{file} was seeded by T6.1a but has no T6.1b site entry"
            );
        }
    }
}
