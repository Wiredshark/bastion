use crate::{ViewDistances, character::CharacterId};
use serde::{Deserialize, Serialize};
use specs::Component;
use std::time::{Duration, Instant};
use vek::*;

#[derive(Debug)]
pub struct Presence {
    pub terrain_view_distance: ViewDistance,
    pub entity_view_distance: ViewDistance,
    /// If mutating this (or the adding/replacing the Presence component as a
    /// whole), make sure the mapping of `CharacterId` in `IdMaps` is
    /// updated!
    pub kind: PresenceKind,
    pub lossy_terrain_compression: bool,
    /// bastion (B1.6): optional god-camera terrain anchor — a second center
    /// (besides the entity position) that terrain chunk requests are accepted
    /// around, so an embodied overseer can stream terrain around the camera.
    /// Set via `ClientGeneral::BastionCameraAnchor`; never moves the entity
    /// (unlike `SpectatePosition`).
    pub bastion_terrain_anchor: Option<Vec3<f32>>,
}

impl Presence {
    pub fn new(view_distances: ViewDistances, kind: PresenceKind) -> Self {
        let now = Instant::now();
        Self {
            terrain_view_distance: ViewDistance::new(view_distances.terrain, now),
            entity_view_distance: ViewDistance::new(view_distances.entity, now),
            kind,
            lossy_terrain_compression: false,
            bastion_terrain_anchor: None,
        }
    }
}

impl Component for Presence {
    type Storage = specs::DenseVecStorage<Self>;
}

#[derive(Debug, Clone, Copy)]
/// The entity the player is currently spectating.
pub struct SpectatingEntity(pub specs::Entity);

impl Component for SpectatingEntity {
    type Storage = specs::DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresenceKind {
    Spectator,
    // Note: we don't know if this character ID is valid and associated with the player until the
    // character has loaded successfully. The ID should only be trusted and included in the
    // mapping when the variant is changed to `Character`.
    LoadingCharacter(CharacterId),
    Character(CharacterId),
    Possessor,
    /// bastion (ROW-COLONY-PRESENCE, DECISIONS #106): a server-owned
    /// presence with no connected client and no `CharacterId` -- every
    /// other variant maps to a human session, this one is minted at colony
    /// founding and dropped at disband. Its sole job is to hold the
    /// colony's footprint in the SAME terrain-loading join every player
    /// presence goes through (`server/src/sys/terrain.rs`'s
    /// `(entities, presences, positions, clients.mask().maybe())` join
    /// treats a presence with no `Client` component as a client-less
    /// load-center, not a special case), which is what keeps founded
    /// colonists in `SimulationMode::Loaded` without anyone connected --
    /// the finding this row exists to fix: colonists demoted to
    /// `Simulated` ~20s after every founding to date, invisible only
    /// because a client always happened to be present.
    Colony,
}

impl PresenceKind {
    /// Check if the presence represents a control of a character, and thus
    /// certain in-game messages from the client such as control inputs
    /// should be handled.
    pub fn controlling_char(&self) -> bool { matches!(self, Self::Character(_) | Self::Possessor) }

    pub fn character_id(&self) -> Option<CharacterId> {
        if let Self::Character(character_id) = self {
            Some(*character_id)
        } else {
            None
        }
    }

    /// Controls whether this entity is synced to other clients.
    ///
    /// Note, if it ends up being useful this could be generalized to an
    /// independent component that is required for any entity to be synced
    /// (as an independent component it could use NullStorage).
    pub fn sync_me(&self) -> bool {
        match self {
            Self::Spectator | Self::LoadingCharacter(_) | Self::Colony => false,
            Self::Character(_) | Self::Possessor => true,
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum Direction {
    Up,
    Down,
}

/// Distance from the [Presence] from which the world is loaded and information
/// is synced to clients.
///
/// We limit the frequency that changes in the view distance change direction
/// (e.g. shifting from increasing the value to decreasing it). This is useful
/// since we want to avoid rapid cycles of shrinking and expanding of the view
/// distance.
#[derive(Debug, Clone, Copy)]
pub struct ViewDistance {
    direction: Direction,
    last_direction_change_time: Instant,
    target: Option<u32>,
    current: u32,
}

impl ViewDistance {
    /// Minimum time allowed between changes in direction of value adjustments.
    const TIME_PER_DIR_CHANGE: Duration = Duration::from_millis(300);

    pub fn new(start_value: u32, now: Instant) -> Self {
        Self {
            direction: Direction::Up,
            last_direction_change_time: now.checked_sub(Self::TIME_PER_DIR_CHANGE).unwrap_or(now),
            target: None,
            current: start_value,
        }
    }

    /// Returns the current value.
    pub fn current(&self) -> u32 { self.current }

    /// Applies deferred change based on the whether the time to apply it has
    /// been reached.
    pub fn update(&mut self, now: Instant) {
        if let Some(target_val) = self.target
            && now.saturating_duration_since(self.last_direction_change_time)
                > Self::TIME_PER_DIR_CHANGE
        {
            self.last_direction_change_time = now;
            self.current = target_val;
            self.target = None;
        }
    }

    /// Sets the target value.
    ///
    /// If this hasn't been changed recently or it is in the same direction as
    /// the previous change it will be applied immediately. Otherwise, it
    /// will be deferred to a later time (limiting the frequency of changes
    /// in the change direction).
    pub fn set_target(&mut self, new_target: u32, now: Instant) {
        use core::cmp::Ordering;
        let new_direction = match new_target.cmp(&self.current) {
            Ordering::Equal => return, // No change needed.
            Ordering::Less => Direction::Down,
            Ordering::Greater => Direction::Up,
        };

        // Change is in the same direction as before so we can just apply it.
        if new_direction == self.direction {
            self.current = new_target;
            self.target = None;
        // If it has already been a while since the last direction change we can
        // directly apply the request and switch the direction.
        } else if now.saturating_duration_since(self.last_direction_change_time)
            > Self::TIME_PER_DIR_CHANGE
        {
            self.direction = new_direction;
            self.last_direction_change_time = now;
            self.current = new_target;
            self.target = None;
        // Otherwise, we need to defer the request.
        } else {
            self.target = Some(new_target);
        }
    }
}
