use crate::{
    comp::{Alignment, Body, Group, Player},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{
    ops::Add,
    time::{Duration, Instant},
};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct LootOwner {
    // TODO: Fix this if expiry is needed client-side, Instant is not serializable
    #[serde(skip, default = "Instant::now")]
    expiry: Instant,
    owner: LootOwnerKind,
    soft: bool,
}

/// Loot becomes free-for-all after the initial ownership period
pub const ONWERSHIP_TIMEOUT_SLOW: u64 = 45;
pub const ONWERSHIP_TIMEOUT_FAST: u64 = 10;

impl LootOwner {
    pub fn new(kind: LootOwnerKind, soft: bool, duration: u64) -> Self {
        Self {
            expiry: Instant::now().add(Duration::from_secs(duration)),
            owner: kind,
            soft,
        }
    }

    pub fn uid(&self) -> Option<Uid> {
        match &self.owner {
            LootOwnerKind::Player(uid) => Some(*uid),
            LootOwnerKind::Group(_) => None,
        }
    }

    pub fn owner(&self) -> LootOwnerKind { self.owner }

    pub fn time_until_expiration(&self) -> Duration { self.expiry - Instant::now() }

    pub fn expired(&self) -> bool { self.expiry <= Instant::now() }

    pub fn default_instant() -> Instant { Instant::now() }

    /// This field stands as a wish for NPC's to not pick the loot up, they will
    /// however be able to decide whether they want to follow your wishes or not
    /// (players will be able to pick the item up)
    pub fn is_soft(&self) -> bool { self.soft }

    pub fn can_pickup(
        &self,
        uid: Uid,
        group: Option<&Group>,
        alignment: Option<&Alignment>,
        body: Option<&Body>,
        player: Option<&Player>,
    ) -> bool {
        let is_owned = matches!(alignment, Some(Alignment::Owned(_)));
        let is_player = player.is_some();
        let is_pet = is_owned && !is_player;

        let owns_loot = match self.owner {
            LootOwnerKind::Player(loot_uid) => loot_uid.0 == uid.0,
            LootOwnerKind::Group(loot_group) => {
                matches!(group, Some(group) if loot_group == *group)
            },
        };
        let is_humanoid = matches!(body, Some(Body::Humanoid(_)));

        // Pet's can't pick up owned loot
        // Humanoids must own the loot
        // Non-humanoids ignore loot ownership
        !is_pet && (self.soft || owns_loot || !is_humanoid)
    }
}

impl Component for LootOwner {
    type Storage = DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LootOwnerKind {
    Player(Uid),
    Group(Group),
}

// bastion ENGINE-OPT-3 (ledger #160): `can_pickup` is THE loot authority —
// the AI's attempt decision AND the commit gate (`inventory_manip`'s Pickup
// arm) both call it. Its table is pinned here so neither caller can drift
// on a silent semantics change.
#[cfg(test)]
mod tests {
    use super::*;

    fn owner_of(uid: Uid, soft: bool) -> LootOwner {
        LootOwner::new(LootOwnerKind::Player(uid), soft, 60)
    }

    fn uid(n: u64) -> Uid { Uid(core::num::NonZeroU64::new(n).unwrap()) }

    #[test]
    fn item_160_can_pickup_truth_table() {
        let humanoid = Body::Humanoid(crate::comp::humanoid::Body::random());
        let wolf = Body::QuadrupedMedium(crate::comp::quadruped_medium::Body::random());
        let owner = owner_of(uid(1), false);
        // The owner picks up their own hard-owned loot.
        assert!(owner.can_pickup(uid(1), None, None, Some(&humanoid), None));
        // A foreign humanoid cannot take hard-owned loot.
        assert!(!owner.can_pickup(uid(2), None, None, Some(&humanoid), None));
        // Soft ownership authorizes anyone (the WISH is courtesy, not law).
        let soft = owner_of(uid(1), true);
        assert!(soft.can_pickup(uid(2), None, None, Some(&humanoid), None));
        // Non-humanoids ignore ownership BY DESIGN (documented in-function).
        assert!(owner.can_pickup(uid(2), None, None, Some(&wolf), None));
        // Pets never pick up owned loot (owned alignment, not a player).
        let pet_alignment = Alignment::Owned(uid(9));
        assert!(!owner.can_pickup(uid(2), None, Some(&pet_alignment), Some(&wolf), None));
    }
}
