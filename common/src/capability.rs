//! T1.11 (master build order; T1-001 packet, step 9): server-issued
//! capability sets and scoped admission.
//!
//! A command is admitted only if the subject holds a matching
//! [`CapabilityGrant`] with the right scope, a CURRENT generation, an
//! unexpired validity, and the required progression state. Capabilities are
//! SERVER-ISSUED — camera / HUD / client mode is presentation only and can
//! never manufacture authority (there is no client path that mints a
//! grant).
//!
//! Determinism story (Ben's law): validity is a SIM-tick bound (never
//! wall-clock), grant lookup is over a keyed collection, the admission
//! check is pure; no RNG, no iteration-order dependence.

use crate::feature_protocol::CapabilityKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A server-issued grant id (never client-minted).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GrantId(pub u64);

/// The subject a grant is issued to (player uid bits).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Subject(pub u64);

/// The scope a capability applies within — global, a region, or a specific
/// target. A request's scope must be COVERED by the grant's scope.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityScope {
    Global,
    Region { min: (i32, i32), max: (i32, i32) },
    Target(u64),
}

impl CapabilityScope {
    /// Whether `self` (the grant scope) covers `requested`.
    pub fn covers(&self, requested: &CapabilityScope) -> bool {
        match (self, requested) {
            (CapabilityScope::Global, _) => true,
            (
                CapabilityScope::Region { min, max },
                CapabilityScope::Region {
                    min: rmin,
                    max: rmax,
                },
            ) => rmin.0 >= min.0 && rmin.1 >= min.1 && rmax.0 <= max.0 && rmax.1 <= max.1,
            (
                CapabilityScope::Region { min, max },
                CapabilityScope::Target(_),
            ) => {
                // A target request against a region grant is not coverable
                // without the target's position; conservatively deny.
                let _ = (min, max);
                false
            },
            (CapabilityScope::Target(a), CapabilityScope::Target(b)) => a == b,
            _ => false,
        }
    }
}

/// T1.11: a server-issued capability grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    pub grant_id: GrantId,
    pub subject: Subject,
    pub capability: CapabilityKind,
    pub scope: CapabilityScope,
    /// The progression/authority generation the grant was issued under; a
    /// request must present the current one.
    pub generation: u64,
    /// Sim tick after which the grant is expired.
    pub valid_until_tick: u64,
}

/// Why capability admission was denied — the minimal reason.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CapabilityDenial {
    NoGrant,
    OutOfScope,
    StaleGeneration,
    Expired,
    ProgressionNotMet,
}

/// A subject's held grants — server-issued only. `admits` is the sole
/// authority path; there is no client-facing mint.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    grants: BTreeMap<GrantId, CapabilityGrant>,
}

impl CapabilitySet {
    /// Server-issue a grant (server-side only — no client caller exists).
    pub fn grant(&mut self, grant: CapabilityGrant) { self.grants.insert(grant.grant_id, grant); }

    pub fn revoke(&mut self, grant_id: GrantId) { self.grants.remove(&grant_id); }

    /// THE admission check: is there a grant for `subject` + `capability`
    /// that covers `scope`, matches `current_generation`, is unexpired at
    /// `now_tick`, and whose progression precondition (`progression_ok`) is
    /// met? Grants iterate in id order (deterministic).
    #[expect(clippy::too_many_arguments, reason = "the full admission predicate")]
    pub fn admits(
        &self,
        subject: Subject,
        capability: CapabilityKind,
        scope: &CapabilityScope,
        current_generation: u64,
        now_tick: u64,
        progression_ok: bool,
    ) -> Result<GrantId, CapabilityDenial> {
        let mut best_denial = CapabilityDenial::NoGrant;
        for grant in self.grants.values() {
            if grant.subject != subject || grant.capability != capability {
                continue;
            }
            if !grant.scope.covers(scope) {
                best_denial = CapabilityDenial::OutOfScope;
                continue;
            }
            if grant.generation != current_generation {
                best_denial = CapabilityDenial::StaleGeneration;
                continue;
            }
            if now_tick > grant.valid_until_tick {
                best_denial = CapabilityDenial::Expired;
                continue;
            }
            if !progression_ok {
                best_denial = CapabilityDenial::ProgressionNotMet;
                continue;
            }
            return Ok(grant.grant_id);
        }
        Err(best_denial)
    }
}

#[cfg(test)]
mod t1_11_tests {
    use super::*;

    fn grant(scope: CapabilityScope, generation: u64, until: u64) -> CapabilityGrant {
        CapabilityGrant {
            grant_id: GrantId(1),
            subject: Subject(7),
            capability: CapabilityKind::SculptTerrain,
            scope,
            generation,
            valid_until_tick: until,
        }
    }

    #[test]
    fn t1_11_admits_matching_grant() {
        let mut set = CapabilitySet::default();
        set.grant(grant(CapabilityScope::Global, 3, 1000));
        assert_eq!(
            set.admits(
                Subject(7),
                CapabilityKind::SculptTerrain,
                &CapabilityScope::Target(42),
                3,
                500,
                true,
            ),
            Ok(GrantId(1))
        );
    }

    #[test]
    fn t1_11_each_denial_distinct() {
        let mut set = CapabilitySet::default();
        // No grant.
        assert_eq!(
            set.admits(Subject(7), CapabilityKind::CastPower, &CapabilityScope::Global, 3, 1, true),
            Err(CapabilityDenial::NoGrant)
        );
        set.grant(grant(
            CapabilityScope::Region {
                min: (0, 0),
                max: (10, 10),
            },
            3,
            1000,
        ));
        // Out of scope (region grant, target request → conservative deny).
        assert_eq!(
            set.admits(
                Subject(7),
                CapabilityKind::SculptTerrain,
                &CapabilityScope::Target(1),
                3,
                1,
                true,
            ),
            Err(CapabilityDenial::OutOfScope)
        );
        // Stale generation.
        assert_eq!(
            set.admits(
                Subject(7),
                CapabilityKind::SculptTerrain,
                &CapabilityScope::Region { min: (1, 1), max: (2, 2) },
                4,
                1,
                true,
            ),
            Err(CapabilityDenial::StaleGeneration)
        );
        // Expired (sim tick past valid_until).
        assert_eq!(
            set.admits(
                Subject(7),
                CapabilityKind::SculptTerrain,
                &CapabilityScope::Region { min: (1, 1), max: (2, 2) },
                3,
                2000,
                true,
            ),
            Err(CapabilityDenial::Expired)
        );
        // Progression not met.
        assert_eq!(
            set.admits(
                Subject(7),
                CapabilityKind::SculptTerrain,
                &CapabilityScope::Region { min: (1, 1), max: (2, 2) },
                3,
                1,
                false,
            ),
            Err(CapabilityDenial::ProgressionNotMet)
        );
    }
}
