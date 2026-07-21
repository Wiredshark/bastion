use serde::{Deserialize, Serialize};
use specs::Component;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InviteKind {
    Group,
    Trade,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InviteResponse {
    Accept,
    Decline,
}

pub struct Invite {
    pub inviter: specs::Entity,
    pub kind: InviteKind,
}

impl Component for Invite {
    type Storage = specs::DenseVecStorage<Self>;
}

/// Pending invites that an entity currently has sent out
/// (invited entity, invite kind, SIM-TIME in seconds at which the invite
/// times out).
///
/// DET-ADD-002 (determinism audit): the timeout deadline was a wall-clock
/// `std::time::Instant`, so invite expiry depended on real time — it
/// diverged across a paused/laggy server, a replay, or a save/reload. Keyed
/// to the deterministic `Time` sim-clock (`f64` seconds) instead.
pub struct PendingInvites(pub Vec<(specs::Entity, InviteKind, f64)>);
impl Component for PendingInvites {
    type Storage = specs::DenseVecStorage<Self>;
}
