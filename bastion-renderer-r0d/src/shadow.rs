//! Deterministic figure-shadow selection and importance budgets.
//!
//! The plan chooses only among production capabilities declared by the
//! caller. It never treats draw observation, residency timing, wall time, or
//! GPU duration as semantic authority.

use std::{collections::BTreeSet, sync::Arc};

use sha2::{Digest, Sha256};

pub const SHADOW_POLICY_VERSION_V1: u16 = 1;
pub const SHADOW_DECISION_BYTES_V1: usize = 128;
pub const MAX_SHADOW_INPUTS_V1: usize = 4_096;

pub type ShadowDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ShadowTierV1 {
    S0Detailed = 1,
    S1Proxy = 2,
    S2ReducedFrequency = 3,
    S3GroupContact = 4,
    S4None = 5,
}

impl ShadowTierV1 {
    fn from_u8(value: u8) -> Result<Self, ShadowErrorV1> {
        match value {
            1 => Ok(Self::S0Detailed),
            2 => Ok(Self::S1Proxy),
            3 => Ok(Self::S2ReducedFrequency),
            4 => Ok(Self::S3GroupContact),
            5 => Ok(Self::S4None),
            _ => Err(ShadowErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ShadowFallbackV1 {
    None = 0,
    ProxyUnavailableToDetailed = 1,
    ProxyUnavailableToNone = 2,
    ReducedFrequencyUnavailable = 3,
    GroupContactUnavailable = 4,
    DetailedBudgetExhausted = 5,
    DetailedPathUnavailable = 6,
}

impl ShadowFallbackV1 {
    fn from_u8(value: u8) -> Result<Self, ShadowErrorV1> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::ProxyUnavailableToDetailed),
            2 => Ok(Self::ProxyUnavailableToNone),
            3 => Ok(Self::ReducedFrequencyUnavailable),
            4 => Ok(Self::GroupContactUnavailable),
            5 => Ok(Self::DetailedBudgetExhausted),
            6 => Ok(Self::DetailedPathUnavailable),
            _ => Err(ShadowErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShadowAvailabilityV1 {
    pub detailed: bool,
    pub proxy: bool,
    pub reduced_frequency: bool,
    pub group_contact: bool,
}

impl ShadowAvailabilityV1 {
    pub const PRODUCTION_FIGURE_MAP: Self = Self {
        detailed: true,
        proxy: false,
        reduced_frequency: false,
        group_contact: false,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShadowStateV1 {
    pub generation: u64,
    pub requested: ShadowTierV1,
    pub active: ShadowTierV1,
    pub accepted_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShadowInputV1 {
    pub semantic_entity: ShadowDigestV1,
    pub importance: u16,
    pub screen_size_milli: u32,
    pub distance_mm: u32,
    pub protected: bool,
    pub availability: ShadowAvailabilityV1,
    pub prior: Option<ShadowStateV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShadowPolicyV1 {
    pub detailed_distance_mm: u32,
    pub proxy_distance_mm: u32,
    pub reduced_distance_mm: u32,
    pub group_distance_mm: u32,
    pub detailed_screen_milli: u32,
    pub proxy_screen_milli: u32,
    pub reduced_screen_milli: u32,
    pub group_screen_milli: u32,
    pub minimum_residence_ticks: u64,
}

impl ShadowPolicyV1 {
    pub const PRODUCTION: Self = Self {
        detailed_distance_mm: 20_000,
        proxy_distance_mm: 50_000,
        reduced_distance_mm: 80_000,
        group_distance_mm: 120_000,
        detailed_screen_milli: 500,
        proxy_screen_milli: 180,
        reduced_screen_milli: 60,
        group_screen_milli: 20,
        minimum_residence_ticks: 30,
    };

    pub fn validate(self) -> Result<Self, ShadowErrorV1> {
        if self.detailed_distance_mm == 0
            || self.detailed_distance_mm >= self.proxy_distance_mm
            || self.proxy_distance_mm >= self.reduced_distance_mm
            || self.reduced_distance_mm >= self.group_distance_mm
            || self.detailed_screen_milli <= self.proxy_screen_milli
            || self.proxy_screen_milli <= self.reduced_screen_milli
            || self.reduced_screen_milli <= self.group_screen_milli
            || self.group_screen_milli == 0
            || self.minimum_residence_ticks == 0
        {
            return Err(ShadowErrorV1::InvalidPolicy);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShadowBudgetV1 {
    pub max_detailed: u32,
    pub max_proxy: u32,
    pub max_reduced_frequency: u32,
    pub max_group_contact: u32,
}

impl ShadowBudgetV1 {
    pub const PRODUCTION: Self = Self {
        max_detailed: 64,
        max_proxy: 0,
        max_reduced_frequency: 0,
        max_group_contact: 0,
    };

    pub fn validate(self) -> Result<Self, ShadowErrorV1> {
        let total = self
            .max_detailed
            .checked_add(self.max_proxy)
            .and_then(|value| value.checked_add(self.max_reduced_frequency))
            .and_then(|value| value.checked_add(self.max_group_contact))
            .ok_or(ShadowErrorV1::LengthOverflow)?;
        if total == 0 || usize::try_from(total).map_or(true, |value| value > MAX_SHADOW_INPUTS_V1) {
            return Err(ShadowErrorV1::InvalidBudget);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShadowDecisionV1 {
    pub version: u16,
    pub generation: u64,
    pub frame_digest: ShadowDigestV1,
    pub semantic_entity: ShadowDigestV1,
    pub requested: ShadowTierV1,
    pub active: ShadowTierV1,
    pub fallback: ShadowFallbackV1,
    pub cadence_ticks: u8,
    pub protected: bool,
    pub importance: u16,
    pub screen_size_milli: u32,
    pub distance_mm: u32,
    pub accepted_tick: u64,
    pub priority_ordinal: u32,
}

impl ShadowDecisionV1 {
    pub fn to_canonical_bytes(self) -> [u8; SHADOW_DECISION_BYTES_V1] {
        let mut bytes = [0_u8; SHADOW_DECISION_BYTES_V1];
        bytes[0..4].copy_from_slice(b"R1FS");
        bytes[4..6].copy_from_slice(&self.version.to_le_bytes());
        bytes[6] = self.requested as u8;
        bytes[7] = self.active as u8;
        bytes[8] = self.fallback as u8;
        bytes[9] = self.cadence_ticks;
        bytes[10] = u8::from(self.protected);
        bytes[12..20].copy_from_slice(&self.generation.to_le_bytes());
        bytes[20..52].copy_from_slice(&self.frame_digest);
        bytes[52..84].copy_from_slice(&self.semantic_entity);
        bytes[84..86].copy_from_slice(&self.importance.to_le_bytes());
        bytes[88..92].copy_from_slice(&self.screen_size_milli.to_le_bytes());
        bytes[92..96].copy_from_slice(&self.distance_mm.to_le_bytes());
        bytes[96..104].copy_from_slice(&self.accepted_tick.to_le_bytes());
        bytes[104..108].copy_from_slice(&self.priority_ordinal.to_le_bytes());
        bytes
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ShadowErrorV1> {
        if bytes.len() != SHADOW_DECISION_BYTES_V1 {
            return Err(ShadowErrorV1::MalformedLength);
        }
        if &bytes[0..4] != b"R1FS" {
            return Err(ShadowErrorV1::InvalidMagic);
        }
        if bytes[11] != 0 || bytes[86..88] != [0, 0] || bytes[108..].iter().any(|v| *v != 0) {
            return Err(ShadowErrorV1::NonCanonicalReservedBytes);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != SHADOW_POLICY_VERSION_V1 {
            return Err(ShadowErrorV1::UnsupportedVersion);
        }
        if bytes[10] > 1 {
            return Err(ShadowErrorV1::UnknownTag);
        }
        let mut frame_digest = [0; 32];
        frame_digest.copy_from_slice(&bytes[20..52]);
        let mut semantic_entity = [0; 32];
        semantic_entity.copy_from_slice(&bytes[52..84]);
        let decision = Self {
            version,
            generation: u64::from_le_bytes(
                bytes[12..20]
                    .try_into()
                    .map_err(|_| ShadowErrorV1::MalformedLength)?,
            ),
            frame_digest,
            semantic_entity,
            requested: ShadowTierV1::from_u8(bytes[6])?,
            active: ShadowTierV1::from_u8(bytes[7])?,
            fallback: ShadowFallbackV1::from_u8(bytes[8])?,
            cadence_ticks: bytes[9],
            protected: bytes[10] == 1,
            importance: u16::from_le_bytes(
                bytes[84..86]
                    .try_into()
                    .map_err(|_| ShadowErrorV1::MalformedLength)?,
            ),
            screen_size_milli: u32::from_le_bytes(
                bytes[88..92]
                    .try_into()
                    .map_err(|_| ShadowErrorV1::MalformedLength)?,
            ),
            distance_mm: u32::from_le_bytes(
                bytes[92..96]
                    .try_into()
                    .map_err(|_| ShadowErrorV1::MalformedLength)?,
            ),
            accepted_tick: u64::from_le_bytes(
                bytes[96..104]
                    .try_into()
                    .map_err(|_| ShadowErrorV1::MalformedLength)?,
            ),
            priority_ordinal: u32::from_le_bytes(
                bytes[104..108]
                    .try_into()
                    .map_err(|_| ShadowErrorV1::MalformedLength)?,
            ),
        };
        if decision.generation == 0
            || decision.cadence_ticks
                != match decision.active {
                    ShadowTierV1::S0Detailed | ShadowTierV1::S1Proxy => 1,
                    ShadowTierV1::S2ReducedFrequency => 4,
                    ShadowTierV1::S3GroupContact => 1,
                    ShadowTierV1::S4None => 0,
                }
        {
            return Err(ShadowErrorV1::InvalidDecision);
        }
        Ok(decision)
    }

    pub fn digest(self) -> ShadowDigestV1 { Sha256::digest(self.to_canonical_bytes()).into() }

    pub const fn should_render(self, tick: u64) -> bool {
        self.cadence_ticks != 0 && tick.is_multiple_of(self.cadence_ticks as u64)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShadowPlanV1 {
    pub generation: u64,
    pub frame_digest: ShadowDigestV1,
    pub tick: u64,
    pub decisions: Vec<ShadowDecisionV1>,
    pub detailed_count: u32,
    pub proxy_count: u32,
    pub reduced_frequency_count: u32,
    pub group_contact_count: u32,
    pub none_count: u32,
    pub fallback_count: u32,
    pub decision_root: ShadowDigestV1,
}

impl ShadowPlanV1 {
    pub fn build(
        generation: u64,
        frame_digest: ShadowDigestV1,
        tick: u64,
        policy: ShadowPolicyV1,
        budget: ShadowBudgetV1,
        mut inputs: Vec<ShadowInputV1>,
    ) -> Result<Self, ShadowErrorV1> {
        policy.validate()?;
        budget.validate()?;
        if generation == 0 || inputs.is_empty() || inputs.len() > MAX_SHADOW_INPUTS_V1 {
            return Err(ShadowErrorV1::InvalidCount);
        }
        let mut observed = BTreeSet::new();
        for input in &inputs {
            if !observed.insert(input.semantic_entity) {
                return Err(ShadowErrorV1::DuplicateSemanticEntity);
            }
            if input.prior.is_some_and(|prior| {
                prior.generation > generation
                    || (prior.generation == generation && prior.accepted_tick > tick)
            }) {
                return Err(ShadowErrorV1::StaleGeneration);
            }
        }
        inputs.sort_by(|left, right| {
            right
                .protected
                .cmp(&left.protected)
                .then_with(|| right.importance.cmp(&left.importance))
                .then_with(|| right.screen_size_milli.cmp(&left.screen_size_milli))
                .then_with(|| left.distance_mm.cmp(&right.distance_mm))
                .then_with(|| left.semantic_entity.cmp(&right.semantic_entity))
        });
        let mut counts = [0_u32; 5];
        let mut decisions = Vec::with_capacity(inputs.len());
        for (ordinal, input) in inputs.into_iter().enumerate() {
            let requested = requested_tier(input, generation, tick, policy);
            let (active, fallback) = activate(requested, input.availability, budget, &mut counts);
            let prior = input.prior.filter(|prior| prior.generation == generation);
            let accepted_tick = if prior.is_some_and(|prior| prior.active == active) {
                prior.map_or(tick, |prior| prior.accepted_tick)
            } else {
                tick
            };
            let cadence_ticks = match active {
                ShadowTierV1::S0Detailed | ShadowTierV1::S1Proxy => 1,
                ShadowTierV1::S2ReducedFrequency => 4,
                ShadowTierV1::S3GroupContact => 1,
                ShadowTierV1::S4None => 0,
            };
            decisions.push(ShadowDecisionV1 {
                version: SHADOW_POLICY_VERSION_V1,
                generation,
                frame_digest,
                semantic_entity: input.semantic_entity,
                requested,
                active,
                fallback,
                cadence_ticks,
                protected: input.protected,
                importance: input.importance,
                screen_size_milli: input.screen_size_milli,
                distance_mm: input.distance_mm,
                accepted_tick,
                priority_ordinal: u32::try_from(ordinal)
                    .map_err(|_| ShadowErrorV1::LengthOverflow)?,
            });
        }
        decisions.sort_by_key(|decision| decision.semantic_entity);
        let mut hasher = Sha256::new();
        hasher.update(b"bastion/r1f/shadow-plan/v1");
        hasher.update(generation.to_le_bytes());
        hasher.update(frame_digest);
        hasher.update(tick.to_le_bytes());
        for decision in &decisions {
            hasher.update(decision.to_canonical_bytes());
        }
        Ok(Self {
            generation,
            frame_digest,
            tick,
            detailed_count: counts[0],
            proxy_count: counts[1],
            reduced_frequency_count: counts[2],
            group_contact_count: counts[3],
            none_count: counts[4],
            fallback_count: u32::try_from(
                decisions
                    .iter()
                    .filter(|decision| decision.fallback != ShadowFallbackV1::None)
                    .count(),
            )
            .map_err(|_| ShadowErrorV1::LengthOverflow)?,
            decision_root: hasher.finalize().into(),
            decisions,
        })
    }

    pub fn decision(&self, semantic_entity: &ShadowDigestV1) -> Option<&ShadowDecisionV1> {
        self.decisions
            .binary_search_by_key(semantic_entity, |decision| decision.semantic_entity)
            .ok()
            .map(|index| &self.decisions[index])
    }
}

#[derive(Clone, Debug, Default)]
pub struct ShadowPlanPublisherV1 {
    current: Option<Arc<ShadowPlanV1>>,
}

impl ShadowPlanPublisherV1 {
    pub fn current(&self) -> Option<Arc<ShadowPlanV1>> { self.current.as_ref().map(Arc::clone) }

    pub fn publish(&mut self, plan: ShadowPlanV1) -> Result<Arc<ShadowPlanV1>, ShadowErrorV1> {
        if let Some(current) = &self.current {
            if plan.generation < current.generation {
                return Err(ShadowErrorV1::StaleGeneration);
            }
            if plan.generation == current.generation {
                if current.as_ref() == &plan {
                    return Ok(Arc::clone(current));
                }
                return Err(ShadowErrorV1::GenerationConflict);
            }
        }
        let plan = Arc::new(plan);
        self.current = Some(Arc::clone(&plan));
        Ok(plan)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowErrorV1 {
    UnsupportedVersion,
    InvalidMagic,
    MalformedLength,
    NonCanonicalReservedBytes,
    UnknownTag,
    InvalidDecision,
    InvalidPolicy,
    InvalidBudget,
    InvalidCount,
    DuplicateSemanticEntity,
    StaleGeneration,
    GenerationConflict,
    LengthOverflow,
}

fn requested_tier(
    input: ShadowInputV1,
    generation: u64,
    tick: u64,
    policy: ShadowPolicyV1,
) -> ShadowTierV1 {
    if let Some(prior) = input.prior
        && prior.generation == generation
        && tick - prior.accepted_tick < policy.minimum_residence_ticks
    {
        return prior.requested;
    }
    if input.protected
        || input.distance_mm <= policy.detailed_distance_mm
        || input.screen_size_milli >= policy.detailed_screen_milli
    {
        ShadowTierV1::S0Detailed
    } else if input.distance_mm <= policy.proxy_distance_mm
        || input.screen_size_milli >= policy.proxy_screen_milli
    {
        ShadowTierV1::S1Proxy
    } else if input.distance_mm <= policy.reduced_distance_mm
        || input.screen_size_milli >= policy.reduced_screen_milli
    {
        ShadowTierV1::S2ReducedFrequency
    } else if input.distance_mm <= policy.group_distance_mm
        || input.screen_size_milli >= policy.group_screen_milli
    {
        ShadowTierV1::S3GroupContact
    } else {
        ShadowTierV1::S4None
    }
}

fn activate(
    requested: ShadowTierV1,
    availability: ShadowAvailabilityV1,
    budget: ShadowBudgetV1,
    counts: &mut [u32; 5],
) -> (ShadowTierV1, ShadowFallbackV1) {
    match requested {
        ShadowTierV1::S0Detailed if !availability.detailed => {
            counts[4] += 1;
            (
                ShadowTierV1::S4None,
                ShadowFallbackV1::DetailedPathUnavailable,
            )
        },
        ShadowTierV1::S0Detailed if counts[0] >= budget.max_detailed => {
            counts[4] += 1;
            (
                ShadowTierV1::S4None,
                ShadowFallbackV1::DetailedBudgetExhausted,
            )
        },
        ShadowTierV1::S0Detailed => {
            counts[0] += 1;
            (ShadowTierV1::S0Detailed, ShadowFallbackV1::None)
        },
        ShadowTierV1::S1Proxy if availability.proxy && counts[1] < budget.max_proxy => {
            counts[1] += 1;
            (ShadowTierV1::S1Proxy, ShadowFallbackV1::None)
        },
        ShadowTierV1::S1Proxy if availability.detailed && counts[0] < budget.max_detailed => {
            counts[0] += 1;
            (
                ShadowTierV1::S0Detailed,
                ShadowFallbackV1::ProxyUnavailableToDetailed,
            )
        },
        ShadowTierV1::S1Proxy => {
            counts[4] += 1;
            (
                ShadowTierV1::S4None,
                ShadowFallbackV1::ProxyUnavailableToNone,
            )
        },
        ShadowTierV1::S2ReducedFrequency
            if availability.reduced_frequency && counts[2] < budget.max_reduced_frequency =>
        {
            counts[2] += 1;
            (ShadowTierV1::S2ReducedFrequency, ShadowFallbackV1::None)
        },
        ShadowTierV1::S2ReducedFrequency => {
            counts[4] += 1;
            (
                ShadowTierV1::S4None,
                ShadowFallbackV1::ReducedFrequencyUnavailable,
            )
        },
        ShadowTierV1::S3GroupContact
            if availability.group_contact && counts[3] < budget.max_group_contact =>
        {
            counts[3] += 1;
            (ShadowTierV1::S3GroupContact, ShadowFallbackV1::None)
        },
        ShadowTierV1::S3GroupContact => {
            counts[4] += 1;
            (
                ShadowTierV1::S4None,
                ShadowFallbackV1::GroupContactUnavailable,
            )
        },
        ShadowTierV1::S4None => {
            counts[4] += 1;
            (ShadowTierV1::S4None, ShadowFallbackV1::None)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> ShadowDigestV1 { [value; 32] }

    fn input(id: u8, distance_mm: u32, protected: bool) -> ShadowInputV1 {
        ShadowInputV1 {
            semantic_entity: digest(id),
            importance: if protected {
                u16::MAX
            } else {
                u16::from(100 - id)
            },
            screen_size_milli: 10_000_000 / distance_mm.max(1),
            distance_mm,
            protected,
            availability: ShadowAvailabilityV1::PRODUCTION_FIGURE_MAP,
            prior: None,
        }
    }

    fn plan(inputs: Vec<ShadowInputV1>) -> ShadowPlanV1 {
        ShadowPlanV1::build(
            7,
            digest(99),
            300,
            ShadowPolicyV1::PRODUCTION,
            ShadowBudgetV1::PRODUCTION,
            inputs,
        )
        .unwrap()
    }

    #[test]
    fn production_tiers_and_honest_fallbacks_are_explicit() {
        let mut group = input(4, 100_000, false);
        group.screen_size_milli = 30;
        let plan = plan(vec![
            input(1, 10_000, false),
            input(2, 30_000, false),
            input(3, 60_000, false),
            group,
            input(5, 140_000, false),
        ]);
        assert_eq!(plan.detailed_count, 2);
        assert_eq!(plan.none_count, 3);
        assert_eq!(
            plan.decision(&digest(2)).unwrap().fallback,
            ShadowFallbackV1::ProxyUnavailableToDetailed
        );
        assert_eq!(
            plan.decision(&digest(3)).unwrap().fallback,
            ShadowFallbackV1::ReducedFrequencyUnavailable
        );
        assert_eq!(
            plan.decision(&digest(4)).unwrap().fallback,
            ShadowFallbackV1::GroupContactUnavailable
        );
    }

    #[test]
    fn input_permutation_and_stable_ties_have_one_root() {
        let a = plan(vec![input(1, 20_000, false), input(2, 20_000, false)]);
        let b = plan(vec![input(2, 20_000, false), input(1, 20_000, false)]);
        assert_eq!(a, b);
        assert_eq!(a.decisions[0].semantic_entity, digest(1));
    }

    #[test]
    fn protected_entities_win_a_bounded_detailed_budget() {
        let mut budget = ShadowBudgetV1::PRODUCTION;
        budget.max_detailed = 1;
        let plan = ShadowPlanV1::build(1, digest(9), 1, ShadowPolicyV1::PRODUCTION, budget, vec![
            input(1, 1_000, false),
            input(2, 100_000, true),
        ])
        .unwrap();
        assert_eq!(
            plan.decision(&digest(2)).unwrap().active,
            ShadowTierV1::S0Detailed
        );
        assert_eq!(
            plan.decision(&digest(1)).unwrap().fallback,
            ShadowFallbackV1::DetailedBudgetExhausted
        );
    }

    #[test]
    fn canonical_round_trip_and_exact_eof() {
        let decision = plan(vec![input(1, 10_000, true)]).decisions[0];
        let bytes = decision.to_canonical_bytes();
        assert_eq!(ShadowDecisionV1::from_canonical_bytes(&bytes), Ok(decision));
        let mut trailing = bytes.to_vec();
        trailing.push(0);
        assert_eq!(
            ShadowDecisionV1::from_canonical_bytes(&trailing),
            Err(ShadowErrorV1::MalformedLength)
        );
        let mut reserved = bytes;
        reserved[127] = 1;
        assert_eq!(
            ShadowDecisionV1::from_canonical_bytes(&reserved),
            Err(ShadowErrorV1::NonCanonicalReservedBytes)
        );
    }

    #[test]
    fn cadence_is_tick_derived() {
        let mut availability = ShadowAvailabilityV1::PRODUCTION_FIGURE_MAP;
        availability.reduced_frequency = true;
        let mut budget = ShadowBudgetV1::PRODUCTION;
        budget.max_reduced_frequency = 1;
        let mut value = input(1, 60_000, false);
        value.availability = availability;
        let decision =
            ShadowPlanV1::build(1, digest(2), 10, ShadowPolicyV1::PRODUCTION, budget, vec![
                value,
            ])
            .unwrap()
            .decisions[0];
        assert_eq!(decision.active, ShadowTierV1::S2ReducedFrequency);
        assert!(decision.should_render(12));
        assert!(!decision.should_render(13));
    }

    #[test]
    fn publisher_is_monotonic_idempotent_and_preserves_held_reader() {
        let first = plan(vec![input(1, 10_000, false)]);
        let mut publisher = ShadowPlanPublisherV1::default();
        let held = publisher.publish(first.clone()).unwrap();
        assert!(Arc::ptr_eq(&held, &publisher.publish(first).unwrap()));
        let next = ShadowPlanV1::build(
            8,
            digest(100),
            301,
            ShadowPolicyV1::PRODUCTION,
            ShadowBudgetV1::PRODUCTION,
            vec![input(1, 10_000, false)],
        )
        .unwrap();
        publisher.publish(next).unwrap();
        assert_eq!(held.generation, 7);
        let stale = plan(vec![input(2, 10_000, false)]);
        assert_eq!(
            publisher.publish(stale),
            Err(ShadowErrorV1::StaleGeneration)
        );
    }

    #[test]
    fn same_generation_fallback_rebuild_is_byte_identical() {
        let original_input = input(1, 30_000, false);
        let first = plan(vec![original_input]);
        let decision = first.decisions[0];
        assert_eq!(
            decision.fallback,
            ShadowFallbackV1::ProxyUnavailableToDetailed
        );
        let mut repeated_input = original_input;
        repeated_input.prior = Some(ShadowStateV1 {
            generation: decision.generation,
            requested: decision.requested,
            active: decision.active,
            accepted_tick: decision.accepted_tick,
        });
        let repeated = plan(vec![repeated_input]);
        assert_eq!(first, repeated);
    }

    #[test]
    fn duplicate_malformed_stale_and_invalid_bounds_reject() {
        assert_eq!(
            ShadowPlanV1::build(
                1,
                digest(1),
                1,
                ShadowPolicyV1::PRODUCTION,
                ShadowBudgetV1::PRODUCTION,
                vec![input(1, 1_000, false), input(1, 2_000, false)]
            ),
            Err(ShadowErrorV1::DuplicateSemanticEntity)
        );
        let mut stale = input(1, 1_000, false);
        stale.prior = Some(ShadowStateV1 {
            generation: 2,
            requested: ShadowTierV1::S0Detailed,
            active: ShadowTierV1::S0Detailed,
            accepted_tick: 1,
        });
        assert_eq!(
            ShadowPlanV1::build(
                1,
                digest(1),
                1,
                ShadowPolicyV1::PRODUCTION,
                ShadowBudgetV1::PRODUCTION,
                vec![stale]
            ),
            Err(ShadowErrorV1::StaleGeneration)
        );
        let mut policy = ShadowPolicyV1::PRODUCTION;
        policy.proxy_distance_mm = policy.detailed_distance_mm;
        assert_eq!(policy.validate(), Err(ShadowErrorV1::InvalidPolicy));
    }
}
