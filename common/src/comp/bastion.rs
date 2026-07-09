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
    /// Travel watchdog: last sampled position + time spent not progressing.
    pub last_pos: vek::Vec3<f32>,
    pub stuck_time: f32,
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
