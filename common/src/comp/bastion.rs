//! bastion (Project Bastion): ECS marker components for the overseer
//! interaction surface (B2a).

use serde::{Deserialize, Serialize};
use specs::{Component, NullStorage};

/// Marks the entity currently selected by the overseer (client-side; at most
/// a handful at once). Drives the inspection HUD and feeds the B1.6 cutaway
/// targets, replacing that block's focus+debug-marker stubs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionSelected;

impl Component for BastionSelected {
    type Storage = NullStorage<Self>;
}

/// A colony member (B3): the ECS mirror of the rtsim-side
/// [`crate::bastion::BastionColonist`], attached when the NPC promotes to a
/// loaded entity. Synced to clients (overhead markers, box-select, roster).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Colonist(pub crate::bastion::BastionColonist);

impl Component for Colonist {
    // Synced to clients → needs change-tracked storage.
    type Storage = specs::DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

/// Ownership tag: this entity belongs to THE player colony. Server-side only;
/// B2b's God-mode target restriction reads it.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerColony;

impl Component for PlayerColony {
    type Storage = NullStorage<Self>;
}

/// Need clocks, 1.0 = fully satisfied, 0.0 = starved/exhausted/miserable.
/// Attached in B3; decay + satisfaction behavior land in B7.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,
    pub rest: f32,
    pub recreation: f32,
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 1.0,
            rest: 1.0,
            recreation: 1.0,
        }
    }
}

impl Component for Needs {
    type Storage = specs::DenseVecStorage<Self>;
}

/// Mood aggregate, 0.0 (breakdown) ..= 1.0 (content). B7 feeds it.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mood(pub f32);

impl Default for Mood {
    fn default() -> Self { Self(0.6) }
}

impl Component for Mood {
    type Storage = specs::DenseVecStorage<Self>;
}

/// The colonist's current job assignment (B4). Server-side only; the job
/// system owns the colonist's rtsim-controller activity while this exists.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveJob {
    pub job: crate::bastion::JobId,
    pub state: ActiveJobState,
    /// Travel watchdog: best distance-to-target achieved so far + time since
    /// it last improved. Displacement alone is useless — an agent pacing
    /// around an unreachable target moves plenty without progressing.
    pub best_dist: f32,
    pub stuck_time: f32,
    /// bastion (B-LIVE3, reviewer R3 fix-1 — stuck-time HYSTERESIS): the
    /// distance at the last stuck_time ZERO. The accumulator only resets
    /// on ≥1 block of NET progress since then, so sub-block jitter (magnet
    /// nudges, hover bobbing, physics wobble — all ≥ the 0.5 EPSILON)
    /// can't starve the watchdog forever; real walking (2+ blocks/s)
    /// resets comfortably. Without this, a hovering colonist generated
    /// ZERO timeouts → zero churn → no net ever fired.
    #[serde(default)]
    pub reset_dist: f32,
    /// bastion (B6 SOFT-0): this stall already got its soft-collision
    /// GRACE WINDOW (SOFT-COLLISION-design §0 trigger a). The watchdog
    /// grants soft-pass ONCE per assignment before degrading to the
    /// carve/unreachable pipeline — most chokepoint deadlocks clear in
    /// the grace; a still-stuck soft colonist is genuinely blocked.
    #[serde(default)]
    pub soft_granted: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveJobState {
    /// Walking to the job site.
    Traveling,
    /// At the site, ready to work (B5 hooks here).
    Arrived,
}

impl Component for ActiveJob {
    type Storage = specs::DenseVecStorage<Self>;
}

/// The god-mode anchor marker (§4 standing directive): while the overseer is
/// active, the player's avatar entity carries this — the world must ignore it
/// (no targeting/aggro/greeting/pushback) and it must be invulnerable (the
/// server also applies a permanent `Invulnerability` buff). Removed on F9 /
/// anchor clear; mortality applies only under Embody (B12).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionGodAnchor;

impl Component for BastionGodAnchor {
    type Storage = NullStorage<Self>;
}

/// bastion (B-ASSET1): a direct movement order for test fixtures — the
/// colonist walks to `target` through the vanilla agent (the same
/// `NpcActivity::Goto` mechanism job travel uses) with the same 3D-arrival +
/// progress-watchdog semantics. Server-side only; inert unless inserted
/// (harness `--asset-test` and `--asset-arena` fixtures). Mutually exclusive
/// with [`ActiveJob`] by convention (the hook that inserts it refuses
/// job-holding colonists).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionTestGoto {
    pub target: vek::Vec3<f32>,
    /// Travel watchdog (same scheme as [`ActiveJob`]): best distance achieved
    /// so far + time since it last improved.
    pub best_dist: f32,
    pub stuck_time: f32,
    /// Sim seconds spent on this order (arrival-budget accounting).
    pub elapsed: f32,
    pub arrived: bool,
    /// The watchdog gave up: no progress within the stuck timeout.
    pub stuck: bool,
}

impl BastionTestGoto {
    pub fn new(target: vek::Vec3<f32>) -> Self {
        Self {
            target,
            best_dist: f32::INFINITY,
            stuck_time: 0.0,
            elapsed: 0.0,
            arrived: false,
            stuck: false,
        }
    }
}

impl Component for BastionTestGoto {
    type Storage = specs::DenseVecStorage<Self>;
}

/// A persistent colonist-produced item pile (B5.5). Entities carrying this:
/// never get a despawn timer (colonist output is a player resource — item
/// loss is an invariant violation), aggregate freely with each other via the
/// vanilla merge machinery, and NEVER merge across class with timed vanilla
/// drops (a pile merging into a timed drop would inherit its despawn — a
/// silent-loss path). Server-side only.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionPile;

impl Component for BastionPile {
    type Storage = NullStorage<Self>;
}
