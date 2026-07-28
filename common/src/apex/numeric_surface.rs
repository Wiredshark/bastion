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
//!
//! **`T6.1c` — re-derived and widened, and the completeness marker on
//! `T6.1b` is retired with it.** `T6.1b` said its widened pattern list
//! closed the gap. It did not, and the second reader found two things
//! that a third might not have:
//!
//! 1. **`powi` was missing** — 359 lines across 54 files. It is the
//!    operation this game reaches for whenever it compares a squared
//!    distance against a squared radius, which is to say most of the hit
//!    and range predicates in the tree. `exp2`, `mul_add` and the inverse
//!    hyperbolics went in with it.
//! 2. **The ROOT SET was as wrong as the pattern list had been.** The
//!    scanner walked `common/src` and `common/systems/src` only, so
//!    `server/agent/src/attack.rs` — 191 squared-distance predicates
//!    choosing NPC combat behaviour — was never even opened. No amount of
//!    pattern widening would have revealed it. `server/agent/src` is now
//!    scanned; [`UNSCANNED_AUTHORITATIVE_ROOTS`] names what still is not,
//!    so the next gap is a decision rather than an oversight.
//!
//! The surface moved 26 → 42 authoritative files and 10 → 29
//! branch-driving sites across nine owners. **What this row does NOT
//! claim is that the list is now complete.** It has been wrong twice;
//! the falsification below shows exactly where its edge still is.
//!
//! **Falsified, both directions.** A file using a LISTED operation and no
//! classification fails `every_numeric_surface_file_is_classified` (a
//! planted `x.powi(3)` was rejected by name, then removed). A file using
//! an operation family the list does NOT name — a planted
//! `x.to_radians().recip()` — passes silently. That second result is the
//! honest limit and is recorded rather than left for a fourth reader to
//! discover: **this scanner catches growth of a known surface, not
//! discovery of an unknown one.**
//!
//! **`T6.1d` — the root set again, this time by ruling rather than by
//! discovery.** The three roots `T6.1c` named as decisions are now
//! scanned: `rtsim/src`, `server/src`, `world/src`. 71 files, 461 lines,
//! taking the surface to **113 authoritative files and 52 branch-driving
//! sites**. The falsification was re-run in `world/src` with the same
//! both-directions result, so the standing limit above is unchanged and
//! is deliberately left worded exactly as it was.
//!
//! **Two files that would have been mis-filed by presumption, and were
//! not.** `world/src` looks like pure worldgen and mostly is — hence
//! [`NumericReachV1::WorldGeneration`], whose failure mode is a DIFFERENT
//! WORLD rather than a drifting one. But `world/src/sim/mod.rs` and
//! `world/src/civ/airship_travel.rs` are queried at RUNTIME by rtsim
//! (`rule/architect.rs` calls `get_alt_approx`; `data/airship.rs`
//! consumes the route types), so both carry a live reach. That is T5.4's
//! lesson holding for the third time: the name of a module is not
//! evidence about its consumers.
//!
//! Two pinned files sit on 5b's active `T0.87` surface
//! (`server/src/sys/msg/in_game.rs`, `server/src/weather/sim.rs`). Their
//! counts will need re-deriving when that lands — which is the tripwire
//! doing its job, not a defect in it.

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
    ("common/src/apex/numeric_profile.rs", NumericRoleV1::PresentationOrTooling, "T6.4 evidence tooling; its only root operation is the sqrt inside #[cfg(test)] golden vectors, which never runs in a simulation build and computes no simulation state"),
    ("rtsim/src/ai/mod.rs", NumericRoleV1::Authoritative, "NPC behaviour-tree weighting; feeds the action an npc takes next tick"),
    ("rtsim/src/data/sentiment.rs", NumericRoleV1::Authoritative, "sentiment decay and magnitude, persisted in rtsim data across sessions"),
    ("rtsim/src/rule/npc_ai/airship_ai.rs", NumericRoleV1::Authoritative, "squared-distance gates deciding airship approach, docking and departure phases"),
    ("rtsim/src/rule/npc_ai/mod.rs", NumericRoleV1::Authoritative, "squared-distance gates selecting which npc behaviour node runs"),
    ("rtsim/src/rule/npc_ai/movement.rs", NumericRoleV1::Authoritative, "squared-distance arrival and repath gates; a flipped comparison sends an npc somewhere else"),
    ("rtsim/src/rule/npc_ai/quest.rs", NumericRoleV1::Authoritative, "squared-distance gates deciding quest-step completion"),
    ("rtsim/src/rule/simulate_npcs.rs", NumericRoleV1::Authoritative, "advances PERSISTED npc positions and travel state; the same path states/utils.rs's constant-argument ln feeds"),
    ("server/src/cmd.rs", NumericRoleV1::Authoritative, "admin command range and angle checks deciding whether a command applies to a target"),
    ("server/src/events/entity_manipulation.rs", NumericRoleV1::Authoritative, "damage, knockback and explosion falloff reaching Health and velocity"),
    ("server/src/events/interaction.rs", NumericRoleV1::Authoritative, "squared interaction ranges deciding whether an interaction is permitted"),
    ("server/src/events/inventory_manip.rs", NumericRoleV1::Authoritative, "squared pickup range deciding whether an item may be taken"),
    ("server/src/events/invite.rs", NumericRoleV1::Authoritative, "squared invite range deciding whether an invite may be sent"),
    ("server/src/events/mounting.rs", NumericRoleV1::Authoritative, "squared mount range deciding whether a mount may be entered"),
    ("server/src/rtsim/tick.rs", NumericRoleV1::Authoritative, "npc simulation scaling on the server side of rtsim"),
    ("server/src/state_ext.rs", NumericRoleV1::Authoritative, "squared-distance checks in entity placement and lookup helpers"),
    ("server/src/sys/agent/behavior_tree/mod.rs", NumericRoleV1::Authoritative, "squared-distance gates selecting the agent behaviour node"),
    ("server/src/sys/entity_sync.rs", NumericRoleV1::Authoritative, "squared distances deciding WHAT IS SYNCED TO WHOM, the same decision region.rs makes at region granularity"),
    ("server/src/sys/item.rs", NumericRoleV1::Authoritative, "squared range deciding item entity interaction"),
    ("server/src/sys/msg/in_game.rs", NumericRoleV1::Authoritative, "squared range checks admitting client requests. NOTE: on 5b's T0.87 surface — this pin will need re-deriving at that merge, which is the tripwire work"),
    ("server/src/sys/msg/terrain.rs", NumericRoleV1::Authoritative, "squared distance deciding which terrain requests are served"),
    ("server/src/sys/object.rs", NumericRoleV1::Authoritative, "squared range in object collision and detonation"),
    ("server/src/sys/pets.rs", NumericRoleV1::Authoritative, "squared follow distance deciding when a pet teleports to its owner"),
    ("server/src/sys/subscription.rs", NumericRoleV1::Authoritative, "view-distance magnitudes feeding the region subscription set"),
    ("server/src/sys/teleporter.rs", NumericRoleV1::Authoritative, "squared teleporter radius deciding whether a player is inside it"),
    ("server/src/sys/waypoint.rs", NumericRoleV1::Authoritative, "squared waypoint radius deciding whether a waypoint is claimed"),
    ("server/src/weather/sim.rs", NumericRoleV1::Authoritative, "weather simulation producing the wind that reaches flight. NOTE: on 5b's T0.87 surface — pin will need re-deriving at that merge"),
    ("world/src/block.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/civ/airship_travel.rs", NumericRoleV1::Authoritative, "VERIFIED LIVE: rtsim/src/data/airship.rs imports this module's route types and consumes them during simulation, so route geometry reaches npc behaviou"),
    ("world/src/civ/mod.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/column.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/layer/cave.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/layer/mod.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/layer/scatter.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/layer/spot.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/layer/tree.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/layer/wildlife.rs", NumericRoleV1::Authoritative, "spawn density decides authoritative entity spawns as chunks generate during play"),
    ("world/src/lib.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/sim/erosion.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/sim/map.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/sim/mod.rs", NumericRoleV1::Authoritative, "VERIFIED LIVE, not generation-only: rtsim queries get_alt_approx/get_surface_alt_approx at runtime (rtsim/src/rule/architect.rs, rtsim/src/data/airshi"),
    ("world/src/sim/util.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/sim/way.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/economy/mod.rs", NumericRoleV1::Authoritative, "economy values are simulated by rtsim at runtime rather than fixed at generation; T8 owns this surface"),
    ("world/src/site/generation.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/mod.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/adlet.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/barn.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/bridge.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/cliff_tower.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/cliff_town_airship_dock.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/desert_city_arena.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/dwarven_mine.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/farm_field.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/gnarling.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/house.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/jungle_ruin.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/pirate_hideout.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/plaza.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/savannah_airship_dock.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/savannah_guard_hut.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/savannah_hut.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/savannah_workshop.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/sea_chapel.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/tavern.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/terracotta_house.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/terracotta_palace.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/terracotta_yard.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/site/plot/vampire_castle.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/util/fast_noise.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/util/math.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("world/src/util/mod.rs", NumericRoleV1::Authoritative, "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT W"),
    ("common/src/bin/csv_export.rs", NumericRoleV1::PresentationOrTooling, "a CSV export binary for offline analysis; it never runs inside a simulation build and computes no simulation state"),
    ("common/src/comp/body/object.rs", NumericRoleV1::Authoritative, "object body volume feeds density, which feeds the fluid-dynamics force sum"),
    ("common/src/comp/body/ship.rs", NumericRoleV1::Authoritative, "ship hull volume feeds density and buoyancy"),
    ("common/src/interaction.rs", NumericRoleV1::Authoritative, "MAX_INTERACT_RANGE/MAX_MOUNT_RANGE squared-distance predicates decide whether an interaction is allowed"),
    ("common/src/states/climb.rs", NumericRoleV1::Authoritative, "climb energy cost and movement speed reach velocity"),
    ("common/src/states/glide.rs", NumericRoleV1::Authoritative, "glider aspect ratio and the ground-speed gate; T5.4's wind reaches this file"),
    ("common/src/states/interact.rs", NumericRoleV1::Authoritative, "interaction range predicate"),
    ("common/src/weather.rs", NumericRoleV1::Authoritative, "wind-magnitude threshold classifies weather, which reaches physics"),
    ("common/systems/src/arcing.rs", NumericRoleV1::Authoritative, "arc hit predicate decides who is struck"),
    ("common/systems/src/aura.rs", NumericRoleV1::Authoritative, "aura radius predicate decides who is affected"),
    ("common/systems/src/beam.rs", NumericRoleV1::Authoritative, "beam hit predicate decides who is struck"),
    ("common/systems/src/buff.rs", NumericRoleV1::Authoritative, "aura-radius predicate decides who keeps a buff"),
    ("common/systems/src/pool.rs", NumericRoleV1::Authoritative, "damage-pool radius predicate decides who is hit"),
    ("server/agent/src/action_nodes.rs", NumericRoleV1::Authoritative, "NPC action selection: ranges and speeds that decide which behaviour runs"),
    ("server/agent/src/attack.rs", NumericRoleV1::Authoritative, "NPC combat decisions; 191 squared-distance predicates choosing attacks and positioning"),
    ("server/agent/src/data.rs", NumericRoleV1::Authoritative, "agent range/threat helpers consumed by the decision predicates"),
    ("server/agent/src/util.rs", NumericRoleV1::Authoritative, "agent distance helpers consumed by the decision predicates"),
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
pub(crate) const NUMERIC_SURFACE_PATTERNS: [&str; 27] = [
    "powf", "sqrt()", ".sin()", ".cos()", ".ln()", "hypot", ".acos()", ".asin()", ".atan()",
    ".atan2(", ".tan()", ".exp()", ".cbrt()", ".log(", ".log2()", ".log10()", ".exp_m1()",
    ".ln_1p()", ".sinh()", ".cosh()", ".tanh()",
    // T6.1c: the second widening. `powi` alone was 359 lines across 54
    // files, and it is the operation the game reaches for whenever it
    // compares a squared distance against a squared radius -- which is to
    // say, most of the hit and range predicates in the tree.
    ".powi(", ".exp2()", ".mul_add(", ".asinh()", ".acosh()", ".atanh()",
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

/// The directory roots the surface is scanned from.
///
/// `T6.1c` added `server/agent/src`. Until then the scanner's ROOT SET
/// was as wrong as its pattern list had been: `server/agent/src/attack.rs`
/// alone holds 191 lines of squared-distance combat predicates, and not
/// one of them was inside a surface that claimed to cover authoritative
/// simulation. Widening the patterns would never have revealed it,
/// because the file was never walked.
///
/// Still NOT scanned, named so the gap is a decision rather than an
/// oversight: `server/src` (outside the agent), `rtsim/src` and
/// `world/src`. Each is authoritative in its own way and each is a
/// larger classification job than one row; see
/// [`UNSCANNED_AUTHORITATIVE_ROOTS`].
pub(crate) const SCANNED_ROOTS: [&str; 6] = [
    "common/src",
    "common/systems/src",
    "server/agent/src",
    // T6.1d: the roots T6.1c named as decisions rather than oversights.
    "rtsim/src",
    "server/src",
    "world/src",
];

/// Authoritative code the scanner still does not walk, with why it is
/// out of scope for now. An inventory that simply stopped at its own
/// root set would read as complete.
pub(crate) const UNSCANNED_AUTHORITATIVE_ROOTS: [(&str, &str); 3] = [
    (
        "bastion-server/src",
        "the colony simulation; 4 lines across 3 files, small enough that a pass is cheap but it          is a distinct authority with its own reach argument and has not been traced",
    ),
    (
        "client/src",
        "10 lines across 2 files. The client is not authoritative BY DEFINITION, but T5.4 is the          standing counterexample — its WeatherLerp reached glider steering — so this is listed as          untraced rather than excluded",
    ),
    (
        "common/net/src",
        "8 lines across 2 files; wire-side arithmetic whose consumers have not been traced",
    ),
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
    /// `powf`, `exp`, `exp2`, `cbrt`, `hypot` — the platform libm's.
    Power,
    /// `powi` — NOT libm. The compiler expands it to a multiply chain, so
    /// there is no implementation-defined library behind it. It is not in
    /// [`Self::SquareRoot`]'s class either: the chain's ASSOCIATION is the
    /// compiler's choice, so a different compiler can produce different
    /// bits for a large exponent. `T6.4`'s build tuple is what pins that,
    /// which is exactly why the tuple records the rustc and LLVM version.
    IntegerPower,
    /// `mul_add` — IEEE 754 fused multiply-add, specified to round once.
    /// Correctly rounded, so it belongs with `sqrt` and not with libm.
    FusedMultiplyAdd,
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
            // Correctly rounded (sqrt, fma) or compiler-expanded with no
            // library behind it (powi): no kernel to substitute.
            Self::SquareRoot | Self::FusedMultiplyAdd | Self::IntegerPower => {
                ProtocolStatusV1::SameBuildOnly
            },
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
    /// Worldgen and the civ/site layer. Its own owner because its output
    /// is the WORLD rather than the simulation, so its remedy differs.
    Worldgen,
    /// NPC decision-making in the agent crate. Its own owner because its
    /// numerics decide BEHAVIOUR rather than state: a flipped comparison
    /// makes an NPC choose a different action, which is visible long
    /// before any position drift would be.
    AgentDecision,
}

impl NumericOwnerV1 {
    /// Every owner. An owner with no sites is a naming exercise, and the
    /// test below says so.
    pub(crate) const ALL: [Self; 14] = [
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
        Self::AgentDecision,
        Self::Worldgen,
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
    /// The result lands in generated terrain and structure layout rather
    /// than in per-tick simulation state.
    ///
    /// Its own class because the failure mode differs: a divergence here
    /// produces a DIFFERENT WORLD, not a drifting one, and the remedy is
    /// a regeneration rather than a correction. Ranked below
    /// `CarriedAcrossTicks` only because it cannot compound tick over
    /// tick — not because it matters less.
    ///
    /// `T6.1d` assigns this ONLY where the generation-only claim was
    /// verified. `world/src/sim/mod.rs` and `world/src/civ/airship_travel.rs`
    /// both LOOK generation-only and are not: rtsim queries them at
    /// runtime. Presuming would have mis-filed both.
    WorldGeneration,
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
    use NumericOpV1::{FusedMultiplyAdd, IntegerPower, Log, Power, SquareRoot, Trig};
    use NumericOwnerV1::*;
    use NumericReachV1::{BranchCondition, CarriedAcrossTicks, NoLiveConsumer, WorldGeneration};
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
        site("common/src/comp/ability.rs", "modifiers.speed.powi(level.into())", 13, IntegerPower, Combat, CarriedAcrossTicks,
             "ability modifiers raised to the skill level; the IMMEDIATE consumer multiplies a damage/speed/regen value, it does not compare one, so this is carried state rather than a branch"),
        site("common/src/comp/fluid_dynamics.rs", "x.powi(2)", 1, IntegerPower, FlightAndFluid, CarriedAcrossTicks,
             "aspect-ratio term inside the finite-wing lift slope"),
        site("common/src/comp/projectile.rs", "u_sqrd.powi(2)", 3, IntegerPower, Projectiles, CarriedAcrossTicks,
             "ballistic discriminant terms in aim_projectile"),
        site("common/src/path.rs", "linear_eccentricity.powi(2)", 10, IntegerPower, Pathfinding, CarriedAcrossTicks,
             "spheroid axis terms and squared magnitudes in the waypoint sampler"),
        site("common/src/region.rs", "vd_extended.powi(2)", 1, IntegerPower, WorldSync, BranchCondition,
             "the squared view distance the region membership test compares against; this is the other half of the sync decision the sqrt entry above computes"),
        site("common/src/states/utils.rs", "modifiers.speed.powi(level.into())", 3, IntegerPower, Movement, CarriedAcrossTicks,
             "level-scaled swim acceleration, the body-scaler jump term, and the submersion clamp — all applied to velocity"),
        site("common/src/states/utils.rs", "MAX_MOUNT_RANGE.powi(2)", 2, IntegerPower, AreaOfEffect, BranchCondition,
             "squared mount range and squared block-interaction range, each compared against squared distance to decide whether the action is permitted at all"),
        site("common/systems/src/melee.rs", "MAX_PICKUP_RANGE.powi(2)", 2, IntegerPower, Combat, BranchCondition,
             "the squared reach the melee predicate compares distance against; the atan entry above is the angular half of the same decision"),
        site("common/systems/src/phys/collision.rs", "new_longitudinal_squared", 4, IntegerPower, PhysicsTick, CarriedAcrossTicks,
             "squared-speed terms in the friction and restitution arithmetic"),
        site("common/systems/src/phys/mod.rs", "collision_boundary.powi(2)", 2, IntegerPower, PhysicsTick, BranchCondition,
             "the squared broad-phase boundary at :512 that decides which entity pairs are collision candidates at all"),
        site("common/systems/src/projectile.rs", "target_radius.powi(2)", 2, IntegerPower, Projectiles, BranchCondition,
             "squared target radius compared against squared distance to decide whether the projectile hits"),
        site("common/systems/src/shockwave.rs", "dist.powi(2)", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared distance term in the disk-intersection test that decides who the shockwave reaches"),
        site("common/src/comp/body/object.rs", "self.dimensions().x.powi(3)", 1, IntegerPower, FlightAndFluid, CarriedAcrossTicks,
             "object volume feeding density, which feeds the aerodynamic force sum"),
        site("common/src/comp/body/ship.rs", "equat_d.powi(2)", 2, IntegerPower, FlightAndFluid, CarriedAcrossTicks,
             "ship hull volume feeding density and buoyancy"),
        site("common/src/interaction.rs", "MAX_INTERACT_RANGE.powi(2)", 2, IntegerPower, AreaOfEffect, BranchCondition,
             "the squared interact and mount ranges compared against squared distance; these two lines decide whether a player may interact or mount at all"),
        site("common/src/states/climb.rs", "modifiers.energy_cost.powi(level.into())", 3, IntegerPower, Movement, CarriedAcrossTicks,
             "level-scaled climb cost and speed, plus the squared movement speed applied to velocity"),
        site("common/src/states/glide.rs", "span_length.powi(2)", 2, IntegerPower, FlightAndFluid, CarriedAcrossTicks,
             "glider aspect ratio and the squared airflow magnitude used to scale control authority"),
        site("common/src/states/glide.rs", "ground_vel).magnitude_squared() < 2_f32.powi(2)", 1, IntegerPower, FlightAndFluid, BranchCondition,
             "the squared ground-speed gate that decides whether a glide may start; T5.4's wind reaches this same file"),
        site("common/src/states/interact.rs", "MAX_INTERACT_RANGE.powi(2)", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared interact range compared against squared distance to decide whether the interaction continues"),
        site("common/src/weather.rs", "24.5f32.powi(2)", 1, IntegerPower, FlightAndFluid, BranchCondition,
             "squared wind-magnitude threshold that classifies the weather, and the classification reaches physics"),
        site("common/systems/src/arcing.rs", "arc_rad + rad_b).powi(2)", 2, IntegerPower, AreaOfEffect, BranchCondition,
             "squared arc reach compared against squared distance to decide who is struck"),
        site("common/systems/src/aura.rs", "aura.radius.powi(2)", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared aura radius compared against squared distance to decide who is inside the aura"),
        site("common/systems/src/beam.rs", "bezier_rad + rad_b).powi(2)", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared beam radius compared against squared distance to decide who the beam hits"),
        site("common/systems/src/buff.rs", "aura.radius.powi(2)", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared aura radius compared against squared distance to decide who keeps the buff"),
        site("common/systems/src/pool.rs", "pool.properties.radius + rad_b).powi(2)", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared damage-pool radius compared against squared distance to decide who is damaged"),
        site("server/agent/src/attack.rs", "attack_data.dist_sqrd <", 191, IntegerPower, AgentDecision, BranchCondition,
             "191 squared-distance and squared-radius predicates choosing which attack an NPC uses and where it stands; a flipped comparison here changes BEHAVIOUR, which is visible long before any position drift would be"),
        site("server/agent/src/attack.rs", "sqrt()", 3, SquareRoot, AgentDecision, CarriedAcrossTicks,
             "distance and speed magnitudes feeding those predicates"),
        site("server/agent/src/action_nodes.rs", "powf", 5, Power, AgentDecision, CarriedAcrossTicks,
             "libm power terms in NPC speed and timing curves"),
        site("server/agent/src/action_nodes.rs", ".powi(", 7, IntegerPower, AgentDecision, BranchCondition,
             "squared ranges compared against squared distance to select an action node"),
        site("server/agent/src/action_nodes.rs", "sqrt()", 2, SquareRoot, AgentDecision, CarriedAcrossTicks,
             "magnitudes feeding those range comparisons"),
        site("server/agent/src/data.rs", ".powi(", 6, IntegerPower, AgentDecision, BranchCondition,
             "squared range helpers the decision predicates compare against"),
        site("server/agent/src/data.rs", "sqrt()", 2, SquareRoot, AgentDecision, CarriedAcrossTicks,
             "distance magnitudes feeding those helpers"),
        site("server/agent/src/util.rs", ".powi(", 6, IntegerPower, AgentDecision, BranchCondition,
             "squared distance thresholds deciding whether an NPC engages, flees, or holds"),
        site("rtsim/src/ai/mod.rs", "powf", 1, Power, AgentDecision, CarriedAcrossTicks,
             "NPC behaviour-tree weighting; feeds the action an npc takes next tick"),
        site("rtsim/src/data/sentiment.rs", ".powi(", 2, IntegerPower, AgentDecision, CarriedAcrossTicks,
             "sentiment decay and magnitude, persisted in rtsim data across sessions"),
        site("rtsim/src/data/sentiment.rs", "sqrt()", 1, SquareRoot, AgentDecision, CarriedAcrossTicks,
             "sentiment decay and magnitude, persisted in rtsim data across sessions"),
        site("rtsim/src/rule/npc_ai/airship_ai.rs", ".powi(", 4, IntegerPower, AgentDecision, BranchCondition,
             "squared-distance gates deciding airship approach, docking and departure phases"),
        site("rtsim/src/rule/npc_ai/mod.rs", ".powi(", 5, IntegerPower, AgentDecision, BranchCondition,
             "squared-distance gates selecting which npc behaviour node runs"),
        site("rtsim/src/rule/npc_ai/movement.rs", ".powi(", 7, IntegerPower, AgentDecision, BranchCondition,
             "squared-distance arrival and repath gates; a flipped comparison sends an npc somewhere else"),
        site("rtsim/src/rule/npc_ai/movement.rs", "sqrt()", 1, SquareRoot, AgentDecision, BranchCondition,
             "squared-distance arrival and repath gates; a flipped comparison sends an npc somewhere else"),
        site("rtsim/src/rule/npc_ai/quest.rs", ".powi(", 2, IntegerPower, AgentDecision, BranchCondition,
             "squared-distance gates deciding quest-step completion"),
        site("rtsim/src/rule/simulate_npcs.rs", ".powi(", 3, IntegerPower, AgentDecision, CarriedAcrossTicks,
             "advances PERSISTED npc positions and travel state; the same path states/utils.rs's constant-argument ln feeds"),
        site("rtsim/src/rule/simulate_npcs.rs", "sqrt()", 3, SquareRoot, AgentDecision, CarriedAcrossTicks,
             "advances PERSISTED npc positions and travel state; the same path states/utils.rs's constant-argument ln feeds"),
        site("server/src/cmd.rs", ".powi(", 2, IntegerPower, WorldSync, BranchCondition,
             "admin command range and angle checks deciding whether a command applies to a target"),
        site("server/src/cmd.rs", ".sin()", 2, Trig, WorldSync, BranchCondition,
             "admin command range and angle checks deciding whether a command applies to a target"),
        site("server/src/events/entity_manipulation.rs", ".powi(", 13, IntegerPower, Combat, CarriedAcrossTicks,
             "damage, knockback and explosion falloff reaching Health and velocity"),
        site("server/src/events/entity_manipulation.rs", "sqrt()", 1, SquareRoot, Combat, CarriedAcrossTicks,
             "damage, knockback and explosion falloff reaching Health and velocity"),
        site("server/src/events/entity_manipulation.rs", ".atan()", 4, Trig, Combat, CarriedAcrossTicks,
             "damage, knockback and explosion falloff reaching Health and velocity"),
        site("server/src/events/interaction.rs", ".powi(", 5, IntegerPower, AreaOfEffect, BranchCondition,
             "squared interaction ranges deciding whether an interaction is permitted"),
        site("server/src/events/inventory_manip.rs", ".powi(", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared pickup range deciding whether an item may be taken"),
        site("server/src/events/invite.rs", ".powi(", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared invite range deciding whether an invite may be sent"),
        site("server/src/events/mounting.rs", ".powi(", 2, IntegerPower, AreaOfEffect, BranchCondition,
             "squared mount range deciding whether a mount may be entered"),
        site("server/src/rtsim/tick.rs", "powf", 1, Power, AgentDecision, CarriedAcrossTicks,
             "npc simulation scaling on the server side of rtsim"),
        site("server/src/state_ext.rs", ".powi(", 1, IntegerPower, WorldSync, BranchCondition,
             "squared-distance checks in entity placement and lookup helpers"),
        site("server/src/state_ext.rs", "sqrt()", 1, SquareRoot, WorldSync, BranchCondition,
             "squared-distance checks in entity placement and lookup helpers"),
        site("server/src/sys/agent/behavior_tree/mod.rs", ".powi(", 8, IntegerPower, AgentDecision, BranchCondition,
             "squared-distance gates selecting the agent behaviour node"),
        site("server/src/sys/entity_sync.rs", ".powi(", 7, IntegerPower, WorldSync, BranchCondition,
             "squared distances deciding WHAT IS SYNCED TO WHOM, the same decision region.rs makes at region granularity"),
        site("server/src/sys/item.rs", ".powi(", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared range deciding item entity interaction"),
        site("server/src/sys/msg/in_game.rs", ".powi(", 2, IntegerPower, WorldSync, BranchCondition,
             "squared range checks admitting client requests. NOTE: on 5b's T0.87 surface — this pin will need re-deriving at that merge, which is the tripwire working"),
        site("server/src/sys/msg/terrain.rs", ".powi(", 1, IntegerPower, WorldSync, BranchCondition,
             "squared distance deciding which terrain requests are served"),
        site("server/src/sys/msg/terrain.rs", "sqrt()", 1, SquareRoot, WorldSync, BranchCondition,
             "squared distance deciding which terrain requests are served"),
        site("server/src/sys/object.rs", ".powi(", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared range in object collision and detonation"),
        site("server/src/sys/pets.rs", ".powi(", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared follow distance deciding when a pet teleports to its owner"),
        site("server/src/sys/subscription.rs", "sqrt()", 3, SquareRoot, WorldSync, CarriedAcrossTicks,
             "view-distance magnitudes feeding the region subscription set"),
        site("server/src/sys/teleporter.rs", ".powi(", 2, IntegerPower, AreaOfEffect, BranchCondition,
             "squared teleporter radius deciding whether a player is inside it"),
        site("server/src/sys/waypoint.rs", ".powi(", 1, IntegerPower, AreaOfEffect, BranchCondition,
             "squared waypoint radius deciding whether a waypoint is claimed"),
        site("server/src/weather/sim.rs", ".powi(", 4, IntegerPower, FlightAndFluid, CarriedAcrossTicks,
             "weather simulation producing the wind that reaches flight. NOTE: on 5b's T0.87 surface — pin will need re-deriving at that merge"),
        site("server/src/weather/sim.rs", "powf", 2, Power, FlightAndFluid, CarriedAcrossTicks,
             "weather simulation producing the wind that reaches flight. NOTE: on 5b's T0.87 surface — pin will need re-deriving at that merge"),
        site("world/src/block.rs", "sqrt()", 1, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/block.rs", ".sin()", 1, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/civ/airship_travel.rs", ".powi(", 2, IntegerPower, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE: rtsim/src/data/airship.rs imports this module's route types and consumes them during simulation, so route geometry reaches npc behaviour"),
        site("world/src/civ/airship_travel.rs", "powf", 1, Power, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE: rtsim/src/data/airship.rs imports this module's route types and consumes them during simulation, so route geometry reaches npc behaviour"),
        site("world/src/civ/airship_travel.rs", "sqrt()", 1, SquareRoot, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE: rtsim/src/data/airship.rs imports this module's route types and consumes them during simulation, so route geometry reaches npc behaviour"),
        site("world/src/civ/airship_travel.rs", ".atan2(", 1, Trig, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE: rtsim/src/data/airship.rs imports this module's route types and consumes them during simulation, so route geometry reaches npc behaviour"),
        site("world/src/civ/mod.rs", ".powi(", 2, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/civ/mod.rs", ".log(", 5, Log, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/civ/mod.rs", "sqrt()", 3, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/column.rs", ".powi(", 4, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/column.rs", "powf", 18, Power, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/column.rs", "sqrt()", 5, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/column.rs", ".cos()", 3, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/cave.rs", ".powi(", 17, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/cave.rs", "powf", 12, Power, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/cave.rs", "sqrt()", 3, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/cave.rs", ".sin()", 3, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/mod.rs", ".powi(", 3, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/mod.rs", "powf", 6, Power, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/mod.rs", "sqrt()", 1, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/mod.rs", ".cos()", 4, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/scatter.rs", ".powi(", 2, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/scatter.rs", "powf", 4, Power, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/spot.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/spot.rs", ".sin()", 2, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/tree.rs", ".powi(", 17, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/tree.rs", ".log2()", 12, Log, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/tree.rs", ".sin()", 3, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/layer/wildlife.rs", "powf", 2, Power, Spawning, CarriedAcrossTicks,
             "spawn density decides authoritative entity spawns as chunks generate during play"),
        site("world/src/layer/wildlife.rs", ".sin()", 2, Trig, Spawning, CarriedAcrossTicks,
             "spawn density decides authoritative entity spawns as chunks generate during play"),
        site("world/src/lib.rs", ".powi(", 4, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/erosion.rs", ".powi(", 4, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/erosion.rs", ".ln()", 3, Log, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/erosion.rs", "powf", 20, Power, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/erosion.rs", "sqrt()", 5, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/erosion.rs", ".tan()", 4, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/map.rs", "sqrt()", 3, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/mod.rs", ".powi(", 5, IntegerPower, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE, not generation-only: rtsim queries get_alt_approx/get_surface_alt_approx at runtime (rtsim/src/rule/architect.rs, rtsim/src/data/airship.rs), so these results reach npc placement and movement"),
        site("world/src/sim/mod.rs", ".ln()", 2, Log, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE, not generation-only: rtsim queries get_alt_approx/get_surface_alt_approx at runtime (rtsim/src/rule/architect.rs, rtsim/src/data/airship.rs), so these results reach npc placement and movement"),
        site("world/src/sim/mod.rs", ".exp2()", 12, Power, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE, not generation-only: rtsim queries get_alt_approx/get_surface_alt_approx at runtime (rtsim/src/rule/architect.rs, rtsim/src/data/airship.rs), so these results reach npc placement and movement"),
        site("world/src/sim/mod.rs", "sqrt()", 5, SquareRoot, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE, not generation-only: rtsim queries get_alt_approx/get_surface_alt_approx at runtime (rtsim/src/rule/architect.rs, rtsim/src/data/airship.rs), so these results reach npc placement and movement"),
        site("world/src/sim/mod.rs", ".tanh()", 4, Trig, Worldgen, CarriedAcrossTicks,
             "VERIFIED LIVE, not generation-only: rtsim queries get_alt_approx/get_surface_alt_approx at runtime (rtsim/src/rule/architect.rs, rtsim/src/data/airship.rs), so these results reach npc placement and movement"),
        site("world/src/sim/util.rs", ".mul_add(", 1, FusedMultiplyAdd, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/util.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/sim/way.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/economy/mod.rs", "powf", 1, Power, Worldgen, CarriedAcrossTicks,
             "economy values are simulated by rtsim at runtime rather than fixed at generation; T8 owns this surface"),
        site("world/src/site/generation.rs", ".powi(", 5, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/generation.rs", "powf", 3, Power, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/generation.rs", ".cos()", 7, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/mod.rs", ".powi(", 6, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/mod.rs", "powf", 16, Power, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/mod.rs", "sqrt()", 1, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/adlet.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/adlet.rs", ".log2()", 1, Log, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/adlet.rs", "sqrt()", 7, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/adlet.rs", ".sin()", 16, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/barn.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/bridge.rs", "sqrt()", 4, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/cliff_tower.rs", ".cos()", 2, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/cliff_town_airship_dock.rs", ".cos()", 2, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/desert_city_arena.rs", ".cos()", 10, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/dwarven_mine.rs", "sqrt()", 1, SquareRoot, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/farm_field.rs", ".powi(", 2, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/farm_field.rs", ".sin()", 1, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/gnarling.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/gnarling.rs", ".sin()", 1, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/house.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/jungle_ruin.rs", ".cos()", 6, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/pirate_hideout.rs", ".cos()", 2, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/plaza.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/savannah_airship_dock.rs", ".cos()", 2, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/savannah_guard_hut.rs", ".cos()", 2, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/savannah_hut.rs", ".cos()", 2, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/savannah_workshop.rs", ".cos()", 4, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/sea_chapel.rs", ".cos()", 4, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/tavern.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/terracotta_house.rs", ".cos()", 2, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/terracotta_palace.rs", ".cos()", 12, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/terracotta_yard.rs", ".cos()", 4, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/site/plot/vampire_castle.rs", ".cos()", 12, Trig, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/util/fast_noise.rs", ".powi(", 2, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/util/math.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/util/mod.rs", ".powi(", 1, IntegerPower, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
        site("world/src/util/mod.rs", "powf", 1, Power, Worldgen, WorldGeneration,
             "worldgen geometry: the result lands in generated terrain and structure layout, not in per-tick simulation state, so a divergence here is a DIFFERENT WORLD rather than a drifting one"),
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
    for relative in SCANNED_ROOTS {
        walk(&root.join(relative), &mut files);
    }

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
        // 24 at T6.1a; 26 once T6.1b widened the pattern list; 42 once
        // T6.1c added powi/exp2/mul_add and the server/agent root.
        assert_eq!(authoritative, 113, "the authoritative surface changed — re-derive T6.1b's owners");
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
            } else if matches!(
                s.op,
                NumericOpV1::IntegerPower | NumericOpV1::FusedMultiplyAdd
            ) {
                // T6.1c: neither is libm. `powi` is a compiler-expanded
                // multiply chain and `mul_add` is IEEE-754 fma, so there
                // is no kernel to substitute for either — the build tuple
                // is what pins them.
                assert_eq!(
                    s.op.protocol_v1(),
                    ProtocolStatusV1::SameBuildOnly,
                    "{} / {:?} is not a libm call and must not be a kernel candidate",
                    s.file,
                    s.key
                );
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
            52,
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
    /// ones where an ulp becomes a code path — are spread across nine
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
            9,
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
