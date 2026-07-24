//! Deterministic individual representation, animation and shadow tiers.
//!
//! This policy is renderer-owned. Inputs are immutable presentation facts and
//! explicit integer budgets. Full semantic digests are authority; allocation
//! never observes ECS order, worker completion, residency timing or GPU state.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

pub const INDIVIDUAL_TIER_VERSION_V1: u16 = 1;
pub const INDIVIDUAL_TIER_RECORD_BYTES_V1: usize = 160;
pub const MAX_INDIVIDUAL_TIER_INPUTS_V1: usize = 4_096;

pub type IndividualDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum RepresentationTierV1 {
    Full = 1,
    ReducedAnimation = 2,
    Lod = 3,
    Impostor = 4,
    Culled = 5,
}

impl RepresentationTierV1 {
    fn from_u8(value: u8) -> Result<Self, IndividualTierErrorV1> {
        match value {
            1 => Ok(Self::Full),
            2 => Ok(Self::ReducedAnimation),
            3 => Ok(Self::Lod),
            4 => Ok(Self::Impostor),
            5 => Ok(Self::Culled),
            _ => Err(IndividualTierErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum AnimationTierV1 {
    EveryTick = 1,
    EverySecondTick = 2,
    EveryFourthTick = 3,
    EveryEighthTick = 4,
    Frozen = 5,
}

impl AnimationTierV1 {
    pub const fn cadence(self) -> u64 {
        match self {
            Self::EveryTick => 1,
            Self::EverySecondTick => 2,
            Self::EveryFourthTick => 4,
            Self::EveryEighthTick => 8,
            Self::Frozen => 0,
        }
    }

    fn from_u8(value: u8) -> Result<Self, IndividualTierErrorV1> {
        match value {
            1 => Ok(Self::EveryTick),
            2 => Ok(Self::EverySecondTick),
            3 => Ok(Self::EveryFourthTick),
            4 => Ok(Self::EveryEighthTick),
            5 => Ok(Self::Frozen),
            _ => Err(IndividualTierErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum IndividualShadowTierV1 {
    Full = 1,
    Proxy = 2,
    None = 3,
}

impl IndividualShadowTierV1 {
    fn from_u8(value: u8) -> Result<Self, IndividualTierErrorV1> {
        match value {
            1 => Ok(Self::Full),
            2 => Ok(Self::Proxy),
            3 => Ok(Self::None),
            _ => Err(IndividualTierErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum TierFallbackReasonV1 {
    None = 0,
    LodUnavailable = 1,
    ImpostorUnavailable = 2,
    ShadowProxyUnavailable = 3,
    FullBudget = 4,
    AnimationBudget = 5,
    LodBudget = 6,
    ImpostorBudget = 7,
    VisibleBudget = 8,
}

impl TierFallbackReasonV1 {
    fn from_u8(value: u8) -> Result<Self, IndividualTierErrorV1> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::LodUnavailable),
            2 => Ok(Self::ImpostorUnavailable),
            3 => Ok(Self::ShadowProxyUnavailable),
            4 => Ok(Self::FullBudget),
            5 => Ok(Self::AnimationBudget),
            6 => Ok(Self::LodBudget),
            7 => Ok(Self::ImpostorBudget),
            8 => Ok(Self::VisibleBudget),
            _ => Err(IndividualTierErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TierContentAvailabilityV1 {
    pub lod: bool,
    pub impostor: bool,
    pub shadow_proxy: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndividualTierStateV1 {
    pub generation: u64,
    pub representation: RepresentationTierV1,
    pub accepted_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndividualTierInputV1 {
    pub semantic_entity: IndividualDigestV1,
    pub importance: u16,
    pub screen_size_milli: u32,
    pub distance_mm: u32,
    pub availability: TierContentAvailabilityV1,
    pub prior: Option<IndividualTierStateV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndividualTierPolicyV1 {
    pub full_enter_mm: u32,
    pub full_exit_mm: u32,
    pub reduced_enter_mm: u32,
    pub reduced_exit_mm: u32,
    pub lod_enter_mm: u32,
    pub lod_exit_mm: u32,
    pub impostor_enter_mm: u32,
    pub impostor_exit_mm: u32,
    pub full_enter_screen_milli: u32,
    pub full_exit_screen_milli: u32,
    pub reduced_enter_screen_milli: u32,
    pub reduced_exit_screen_milli: u32,
    pub lod_enter_screen_milli: u32,
    pub lod_exit_screen_milli: u32,
    pub impostor_enter_screen_milli: u32,
    pub impostor_exit_screen_milli: u32,
    pub minimum_residence_ticks: u64,
    pub fade_ticks: u16,
}

impl IndividualTierPolicyV1 {
    pub const PRODUCTION: Self = Self {
        full_enter_mm: 12_000,
        full_exit_mm: 15_000,
        reduced_enter_mm: 28_000,
        reduced_exit_mm: 34_000,
        lod_enter_mm: 52_000,
        lod_exit_mm: 62_000,
        impostor_enter_mm: 84_000,
        impostor_exit_mm: 96_000,
        full_enter_screen_milli: 800,
        full_exit_screen_milli: 650,
        reduced_enter_screen_milli: 360,
        reduced_exit_screen_milli: 280,
        lod_enter_screen_milli: 140,
        lod_exit_screen_milli: 100,
        impostor_enter_screen_milli: 30,
        impostor_exit_screen_milli: 20,
        minimum_residence_ticks: 30,
        fade_ticks: 12,
    };

    pub fn validate(self) -> Result<Self, IndividualTierErrorV1> {
        if self.full_enter_mm > self.full_exit_mm
            || self.full_exit_mm >= self.reduced_enter_mm
            || self.reduced_enter_mm > self.reduced_exit_mm
            || self.reduced_exit_mm >= self.lod_enter_mm
            || self.lod_enter_mm > self.lod_exit_mm
            || self.lod_exit_mm >= self.impostor_enter_mm
            || self.impostor_enter_mm > self.impostor_exit_mm
            || self.full_enter_screen_milli < self.full_exit_screen_milli
            || self.reduced_enter_screen_milli < self.reduced_exit_screen_milli
            || self.lod_enter_screen_milli < self.lod_exit_screen_milli
            || self.impostor_enter_screen_milli < self.impostor_exit_screen_milli
            || self.minimum_residence_ticks == 0
            || self.fade_ticks == 0
        {
            return Err(IndividualTierErrorV1::InvalidPolicy);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndividualTierBudgetV1 {
    pub max_visible: u32,
    pub max_full: u32,
    pub max_reduced: u32,
    pub max_lod: u32,
    pub max_impostor: u32,
    pub max_full_shadows: u32,
    pub max_proxy_shadows: u32,
}

impl IndividualTierBudgetV1 {
    pub const PRODUCTION: Self = Self {
        max_visible: 512,
        max_full: 32,
        max_reduced: 96,
        max_lod: 192,
        max_impostor: 256,
        max_full_shadows: 48,
        max_proxy_shadows: 192,
    };

    pub fn validate(self) -> Result<Self, IndividualTierErrorV1> {
        if self.max_visible == 0
            || self.max_full > self.max_visible
            || self.max_reduced > self.max_visible
            || self.max_lod > self.max_visible
            || self.max_impostor > self.max_visible
            || self.max_full_shadows > self.max_visible
            || self.max_proxy_shadows > self.max_visible
        {
            return Err(IndividualTierErrorV1::InvalidBudget);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndividualTierDecisionV1 {
    pub version: u16,
    pub generation: u64,
    pub frame_digest: IndividualDigestV1,
    pub semantic_entity: IndividualDigestV1,
    pub representation: RepresentationTierV1,
    pub animation: AnimationTierV1,
    pub shadow: IndividualShadowTierV1,
    pub fallback: TierFallbackReasonV1,
    pub importance: u16,
    pub screen_size_milli: u32,
    pub distance_mm: u32,
    pub accepted_tick: u64,
    pub sample_tick: u64,
    pub fade_phase: u16,
    pub priority_ordinal: u32,
}

impl IndividualTierDecisionV1 {
    pub fn to_canonical_bytes(self) -> [u8; INDIVIDUAL_TIER_RECORD_BYTES_V1] {
        let mut bytes = [0_u8; INDIVIDUAL_TIER_RECORD_BYTES_V1];
        bytes[0..4].copy_from_slice(b"R1DT");
        bytes[4..6].copy_from_slice(&self.version.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.generation.to_le_bytes());
        bytes[16..48].copy_from_slice(&self.frame_digest);
        bytes[48..80].copy_from_slice(&self.semantic_entity);
        bytes[80] = self.representation as u8;
        bytes[81] = self.animation as u8;
        bytes[82] = self.shadow as u8;
        bytes[83] = self.fallback as u8;
        bytes[84..86].copy_from_slice(&self.importance.to_le_bytes());
        bytes[88..92].copy_from_slice(&self.screen_size_milli.to_le_bytes());
        bytes[92..96].copy_from_slice(&self.distance_mm.to_le_bytes());
        bytes[96..104].copy_from_slice(&self.accepted_tick.to_le_bytes());
        bytes[104..112].copy_from_slice(&self.sample_tick.to_le_bytes());
        bytes[112..114].copy_from_slice(&self.fade_phase.to_le_bytes());
        bytes[116..120].copy_from_slice(&self.priority_ordinal.to_le_bytes());
        bytes
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, IndividualTierErrorV1> {
        if bytes.len() != INDIVIDUAL_TIER_RECORD_BYTES_V1 {
            return Err(IndividualTierErrorV1::MalformedLength);
        }
        if &bytes[0..4] != b"R1DT" {
            return Err(IndividualTierErrorV1::InvalidMagic);
        }
        if bytes[6..8] != [0, 0]
            || bytes[86..88] != [0, 0]
            || bytes[114..116] != [0, 0]
            || bytes[120..].iter().any(|byte| *byte != 0)
        {
            return Err(IndividualTierErrorV1::NonCanonicalReservedBytes);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != INDIVIDUAL_TIER_VERSION_V1 {
            return Err(IndividualTierErrorV1::UnsupportedVersion);
        }
        let mut frame_digest = [0_u8; 32];
        frame_digest.copy_from_slice(&bytes[16..48]);
        let mut semantic_entity = [0_u8; 32];
        semantic_entity.copy_from_slice(&bytes[48..80]);
        Ok(Self {
            version,
            generation: u64::from_le_bytes(bytes[8..16].try_into().expect("fixed slice")),
            frame_digest,
            semantic_entity,
            representation: RepresentationTierV1::from_u8(bytes[80])?,
            animation: AnimationTierV1::from_u8(bytes[81])?,
            shadow: IndividualShadowTierV1::from_u8(bytes[82])?,
            fallback: TierFallbackReasonV1::from_u8(bytes[83])?,
            importance: u16::from_le_bytes(bytes[84..86].try_into().expect("fixed slice")),
            screen_size_milli: u32::from_le_bytes(bytes[88..92].try_into().expect("fixed slice")),
            distance_mm: u32::from_le_bytes(bytes[92..96].try_into().expect("fixed slice")),
            accepted_tick: u64::from_le_bytes(bytes[96..104].try_into().expect("fixed slice")),
            sample_tick: u64::from_le_bytes(bytes[104..112].try_into().expect("fixed slice")),
            fade_phase: u16::from_le_bytes(bytes[112..114].try_into().expect("fixed slice")),
            priority_ordinal: u32::from_le_bytes(bytes[116..120].try_into().expect("fixed slice")),
        })
    }

    pub fn digest(self) -> IndividualDigestV1 { Sha256::digest(self.to_canonical_bytes()).into() }

    pub const fn should_sample_animation(self, tick: u64) -> bool {
        let cadence = self.animation.cadence();
        cadence != 0 && tick.is_multiple_of(cadence)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndividualTierPlanV1 {
    pub generation: u64,
    pub frame_digest: IndividualDigestV1,
    pub tick: u64,
    pub decisions: Vec<IndividualTierDecisionV1>,
    pub full_count: u32,
    pub reduced_count: u32,
    pub lod_count: u32,
    pub impostor_count: u32,
    pub culled_count: u32,
    pub full_shadow_count: u32,
    pub proxy_shadow_count: u32,
    pub fallback_count: u32,
    pub decision_root: IndividualDigestV1,
}

impl IndividualTierPlanV1 {
    pub fn build(
        generation: u64,
        frame_digest: IndividualDigestV1,
        tick: u64,
        policy: IndividualTierPolicyV1,
        budget: IndividualTierBudgetV1,
        mut inputs: Vec<IndividualTierInputV1>,
    ) -> Result<Self, IndividualTierErrorV1> {
        policy.validate()?;
        budget.validate()?;
        if generation == 0 || inputs.is_empty() || inputs.len() > MAX_INDIVIDUAL_TIER_INPUTS_V1 {
            return Err(IndividualTierErrorV1::InvalidCount);
        }
        let mut observed = BTreeSet::new();
        if inputs
            .iter()
            .any(|input| !observed.insert(input.semantic_entity))
        {
            return Err(IndividualTierErrorV1::DuplicateSemanticEntity);
        }
        inputs.sort_by(|left, right| {
            right
                .importance
                .cmp(&left.importance)
                .then_with(|| right.screen_size_milli.cmp(&left.screen_size_milli))
                .then_with(|| left.distance_mm.cmp(&right.distance_mm))
                .then_with(|| left.semantic_entity.cmp(&right.semantic_entity))
        });

        let mut counters = Counters::default();
        let mut decisions = Vec::with_capacity(inputs.len());
        for (ordinal, input) in inputs.into_iter().enumerate() {
            let requested = requested_tier(input, generation, tick, policy);
            let (representation, fallback) =
                allocate_representation(requested, input.availability, budget, &mut counters);
            let animation = animation_for(representation);
            let (shadow, shadow_fallback) =
                allocate_shadow(representation, input.availability, budget, &mut counters);
            let fallback = if fallback != TierFallbackReasonV1::None {
                fallback
            } else {
                shadow_fallback
            };
            let prior = input
                .prior
                .filter(|state| state.generation == generation && state.accepted_tick <= tick);
            let accepted_tick = if prior.is_some_and(|state| state.representation == representation)
            {
                prior.expect("checked").accepted_tick
            } else {
                tick
            };
            let cadence = animation.cadence();
            let sample_tick = if cadence == 0 {
                accepted_tick
            } else {
                tick - tick % cadence
            };
            let elapsed = tick.saturating_sub(accepted_tick);
            let fade_phase = u16::try_from(elapsed.min(u64::from(policy.fade_ticks)))
                .map_err(|_| IndividualTierErrorV1::LengthOverflow)?;
            decisions.push(IndividualTierDecisionV1 {
                version: INDIVIDUAL_TIER_VERSION_V1,
                generation,
                frame_digest,
                semantic_entity: input.semantic_entity,
                representation,
                animation,
                shadow,
                fallback,
                importance: input.importance,
                screen_size_milli: input.screen_size_milli,
                distance_mm: input.distance_mm,
                accepted_tick,
                sample_tick,
                fade_phase,
                priority_ordinal: u32::try_from(ordinal)
                    .map_err(|_| IndividualTierErrorV1::LengthOverflow)?,
            });
        }
        decisions.sort_by_key(|decision| decision.semantic_entity);
        let mut hasher = Sha256::new();
        hasher.update(b"bastion/r1d/individual-tier-plan/v1");
        hasher.update(&generation.to_le_bytes());
        hasher.update(&frame_digest);
        hasher.update(&tick.to_le_bytes());
        for decision in &decisions {
            hasher.update(&decision.to_canonical_bytes());
        }
        Ok(Self {
            generation,
            frame_digest,
            tick,
            full_count: counters.full,
            reduced_count: counters.reduced,
            lod_count: counters.lod,
            impostor_count: counters.impostor,
            culled_count: counters.culled,
            full_shadow_count: counters.full_shadow,
            proxy_shadow_count: counters.proxy_shadow,
            fallback_count: decisions
                .iter()
                .filter(|decision| decision.fallback != TierFallbackReasonV1::None)
                .count()
                .try_into()
                .map_err(|_| IndividualTierErrorV1::LengthOverflow)?,
            decision_root: hasher.finalize().into(),
            decisions,
        })
    }

    pub fn decision(
        &self,
        semantic_entity: &IndividualDigestV1,
    ) -> Option<&IndividualTierDecisionV1> {
        self.decisions
            .binary_search_by_key(semantic_entity, |decision| decision.semantic_entity)
            .ok()
            .map(|index| &self.decisions[index])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndividualTierErrorV1 {
    UnsupportedVersion,
    InvalidMagic,
    MalformedLength,
    NonCanonicalReservedBytes,
    UnknownTag,
    InvalidPolicy,
    InvalidBudget,
    InvalidCount,
    DuplicateSemanticEntity,
    LengthOverflow,
}

#[derive(Default)]
struct Counters {
    visible: u32,
    full: u32,
    reduced: u32,
    lod: u32,
    impostor: u32,
    culled: u32,
    full_shadow: u32,
    proxy_shadow: u32,
}

fn qualifies(distance: u32, screen: u32, max_distance: u32, min_screen: u32) -> bool {
    distance <= max_distance || screen >= min_screen
}

fn requested_tier(
    input: IndividualTierInputV1,
    generation: u64,
    tick: u64,
    policy: IndividualTierPolicyV1,
) -> RepresentationTierV1 {
    let prior = input
        .prior
        .filter(|state| state.generation == generation && state.accepted_tick <= tick);
    if let Some(prior) = prior
        && tick - prior.accepted_tick < policy.minimum_residence_ticks
    {
        return prior.representation;
    }
    if qualifies(
        input.distance_mm,
        input.screen_size_milli,
        if prior.is_some_and(|state| state.representation == RepresentationTierV1::Full) {
            policy.full_exit_mm
        } else {
            policy.full_enter_mm
        },
        if prior.is_some_and(|state| state.representation == RepresentationTierV1::Full) {
            policy.full_exit_screen_milli
        } else {
            policy.full_enter_screen_milli
        },
    ) {
        RepresentationTierV1::Full
    } else if qualifies(
        input.distance_mm,
        input.screen_size_milli,
        if prior.is_some_and(|state| state.representation == RepresentationTierV1::ReducedAnimation)
        {
            policy.reduced_exit_mm
        } else {
            policy.reduced_enter_mm
        },
        if prior.is_some_and(|state| state.representation == RepresentationTierV1::ReducedAnimation)
        {
            policy.reduced_exit_screen_milli
        } else {
            policy.reduced_enter_screen_milli
        },
    ) {
        RepresentationTierV1::ReducedAnimation
    } else if qualifies(
        input.distance_mm,
        input.screen_size_milli,
        if prior.is_some_and(|state| state.representation == RepresentationTierV1::Lod) {
            policy.lod_exit_mm
        } else {
            policy.lod_enter_mm
        },
        if prior.is_some_and(|state| state.representation == RepresentationTierV1::Lod) {
            policy.lod_exit_screen_milli
        } else {
            policy.lod_enter_screen_milli
        },
    ) {
        RepresentationTierV1::Lod
    } else if qualifies(
        input.distance_mm,
        input.screen_size_milli,
        if prior.is_some_and(|state| state.representation == RepresentationTierV1::Impostor) {
            policy.impostor_exit_mm
        } else {
            policy.impostor_enter_mm
        },
        if prior.is_some_and(|state| state.representation == RepresentationTierV1::Impostor) {
            policy.impostor_exit_screen_milli
        } else {
            policy.impostor_enter_screen_milli
        },
    ) {
        RepresentationTierV1::Impostor
    } else {
        RepresentationTierV1::Culled
    }
}

fn allocate_representation(
    requested: RepresentationTierV1,
    availability: TierContentAvailabilityV1,
    budget: IndividualTierBudgetV1,
    counters: &mut Counters,
) -> (RepresentationTierV1, TierFallbackReasonV1) {
    if counters.visible >= budget.max_visible {
        counters.culled += 1;
        return (
            RepresentationTierV1::Culled,
            TierFallbackReasonV1::VisibleBudget,
        );
    }
    let mut tier = requested;
    let mut fallback = TierFallbackReasonV1::None;
    loop {
        match tier {
            RepresentationTierV1::Full if counters.full < budget.max_full => {
                counters.full += 1;
                counters.visible += 1;
                return (tier, fallback);
            },
            RepresentationTierV1::Full => {
                tier = RepresentationTierV1::ReducedAnimation;
                fallback = TierFallbackReasonV1::FullBudget;
            },
            RepresentationTierV1::ReducedAnimation if counters.reduced < budget.max_reduced => {
                counters.reduced += 1;
                counters.visible += 1;
                return (tier, fallback);
            },
            RepresentationTierV1::ReducedAnimation => {
                tier = RepresentationTierV1::Lod;
                fallback = TierFallbackReasonV1::AnimationBudget;
            },
            RepresentationTierV1::Lod if !availability.lod => {
                if counters.reduced < budget.max_reduced {
                    counters.reduced += 1;
                    counters.visible += 1;
                    return (
                        RepresentationTierV1::ReducedAnimation,
                        TierFallbackReasonV1::LodUnavailable,
                    );
                }
                tier = RepresentationTierV1::Impostor;
                fallback = TierFallbackReasonV1::LodUnavailable;
            },
            RepresentationTierV1::Lod if counters.lod < budget.max_lod => {
                counters.lod += 1;
                counters.visible += 1;
                return (tier, fallback);
            },
            RepresentationTierV1::Lod => {
                tier = RepresentationTierV1::Impostor;
                fallback = TierFallbackReasonV1::LodBudget;
            },
            RepresentationTierV1::Impostor if !availability.impostor => {
                if availability.lod && counters.lod < budget.max_lod {
                    counters.lod += 1;
                    counters.visible += 1;
                    return (
                        RepresentationTierV1::Lod,
                        TierFallbackReasonV1::ImpostorUnavailable,
                    );
                }
                if counters.reduced < budget.max_reduced {
                    counters.reduced += 1;
                    counters.visible += 1;
                    return (
                        RepresentationTierV1::ReducedAnimation,
                        TierFallbackReasonV1::ImpostorUnavailable,
                    );
                }
                counters.culled += 1;
                return (
                    RepresentationTierV1::Culled,
                    TierFallbackReasonV1::ImpostorUnavailable,
                );
            },
            RepresentationTierV1::Impostor if counters.impostor < budget.max_impostor => {
                counters.impostor += 1;
                counters.visible += 1;
                return (tier, fallback);
            },
            RepresentationTierV1::Impostor => {
                counters.culled += 1;
                return (
                    RepresentationTierV1::Culled,
                    TierFallbackReasonV1::ImpostorBudget,
                );
            },
            RepresentationTierV1::Culled => {
                counters.culled += 1;
                return (tier, fallback);
            },
        }
    }
}

fn animation_for(representation: RepresentationTierV1) -> AnimationTierV1 {
    match representation {
        RepresentationTierV1::Full => AnimationTierV1::EveryTick,
        RepresentationTierV1::ReducedAnimation => AnimationTierV1::EverySecondTick,
        RepresentationTierV1::Lod => AnimationTierV1::EveryFourthTick,
        RepresentationTierV1::Impostor => AnimationTierV1::EveryEighthTick,
        RepresentationTierV1::Culled => AnimationTierV1::Frozen,
    }
}

fn allocate_shadow(
    representation: RepresentationTierV1,
    availability: TierContentAvailabilityV1,
    budget: IndividualTierBudgetV1,
    counters: &mut Counters,
) -> (IndividualShadowTierV1, TierFallbackReasonV1) {
    match representation {
        RepresentationTierV1::Full | RepresentationTierV1::ReducedAnimation
            if counters.full_shadow < budget.max_full_shadows =>
        {
            counters.full_shadow += 1;
            (IndividualShadowTierV1::Full, TierFallbackReasonV1::None)
        },
        RepresentationTierV1::Full
        | RepresentationTierV1::ReducedAnimation
        | RepresentationTierV1::Lod
            if availability.shadow_proxy && counters.proxy_shadow < budget.max_proxy_shadows =>
        {
            counters.proxy_shadow += 1;
            (IndividualShadowTierV1::Proxy, TierFallbackReasonV1::None)
        },
        RepresentationTierV1::Full
        | RepresentationTierV1::ReducedAnimation
        | RepresentationTierV1::Lod => (
            IndividualShadowTierV1::None,
            TierFallbackReasonV1::ShadowProxyUnavailable,
        ),
        RepresentationTierV1::Impostor | RepresentationTierV1::Culled => {
            (IndividualShadowTierV1::None, TierFallbackReasonV1::None)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> IndividualDigestV1 { [value; 32] }

    fn input(id: u8, distance_mm: u32, screen: u32) -> IndividualTierInputV1 {
        IndividualTierInputV1 {
            semantic_entity: digest(id),
            importance: if id == 1 { 255 } else { 100 },
            screen_size_milli: screen,
            distance_mm,
            availability: TierContentAvailabilityV1 {
                lod: true,
                impostor: true,
                shadow_proxy: true,
            },
            prior: None,
        }
    }

    fn plan(inputs: Vec<IndividualTierInputV1>) -> IndividualTierPlanV1 {
        IndividualTierPlanV1::build(
            7,
            digest(99),
            120,
            IndividualTierPolicyV1::PRODUCTION,
            IndividualTierBudgetV1::PRODUCTION,
            inputs,
        )
        .unwrap()
    }

    #[test]
    fn all_representation_animation_and_shadow_tiers_are_reachable() {
        let result = plan(vec![
            input(1, 5_000, 1_000),
            input(2, 20_000, 400),
            input(3, 45_000, 150),
            input(4, 75_000, 40),
            input(5, 120_000, 1),
        ]);
        let tiers = result
            .decisions
            .iter()
            .map(|value| (value.representation, value.animation, value.shadow))
            .collect::<BTreeSet<_>>();
        assert!(tiers.contains(&(
            RepresentationTierV1::Full,
            AnimationTierV1::EveryTick,
            IndividualShadowTierV1::Full
        )));
        assert!(tiers.contains(&(
            RepresentationTierV1::ReducedAnimation,
            AnimationTierV1::EverySecondTick,
            IndividualShadowTierV1::Full
        )));
        assert!(tiers.contains(&(
            RepresentationTierV1::Lod,
            AnimationTierV1::EveryFourthTick,
            IndividualShadowTierV1::Proxy
        )));
        assert!(tiers.contains(&(
            RepresentationTierV1::Impostor,
            AnimationTierV1::EveryEighthTick,
            IndividualShadowTierV1::None
        )));
        assert!(
            tiers
                .iter()
                .any(|value| value.0 == RepresentationTierV1::Culled)
        );
    }

    #[test]
    fn input_permutation_and_worker_partitions_do_not_change_plan() {
        let source = vec![
            input(1, 5_000, 1_000),
            input(2, 20_000, 400),
            input(3, 45_000, 150),
            input(4, 75_000, 40),
        ];
        let expected = plan(source.clone());
        let mut reversed = source.clone();
        reversed.reverse();
        assert_eq!(expected, plan(reversed));
        let partitioned = [
            source[1..3].to_vec(),
            source[..1].to_vec(),
            source[3..].to_vec(),
        ]
        .concat();
        assert_eq!(expected, plan(partitioned));
    }

    #[test]
    fn complete_priority_key_breaks_every_tie_by_full_digest() {
        let mut inputs = vec![input(3, 10_000, 900), input(1, 10_000, 900)];
        for value in &mut inputs {
            value.importance = 100;
        }
        let result = IndividualTierPlanV1::build(
            1,
            digest(9),
            1,
            IndividualTierPolicyV1::PRODUCTION,
            IndividualTierBudgetV1 {
                max_full: 1,
                max_reduced: 4,
                max_lod: 4,
                max_impostor: 4,
                max_visible: 4,
                max_full_shadows: 4,
                max_proxy_shadows: 4,
            },
            inputs,
        )
        .unwrap();
        assert_eq!(
            result.decision(&digest(1)).unwrap().representation,
            RepresentationTierV1::Full
        );
        assert_eq!(
            result.decision(&digest(3)).unwrap().fallback,
            TierFallbackReasonV1::FullBudget
        );
    }

    #[test]
    fn explicit_budget_ceiling_degrades_without_omission() {
        let mut inputs = (1..=8)
            .map(|id| input(id, 5_000, 1_000))
            .collect::<Vec<_>>();
        for value in &mut inputs {
            value.importance = 100;
        }
        let result = IndividualTierPlanV1::build(
            1,
            digest(9),
            1,
            IndividualTierPolicyV1::PRODUCTION,
            IndividualTierBudgetV1 {
                max_visible: 6,
                max_full: 1,
                max_reduced: 2,
                max_lod: 2,
                max_impostor: 1,
                max_full_shadows: 1,
                max_proxy_shadows: 2,
            },
            inputs,
        )
        .unwrap();
        assert_eq!(result.decisions.len(), 8);
        assert_eq!(result.full_count, 1);
        assert_eq!(result.reduced_count, 2);
        assert_eq!(result.lod_count, 2);
        assert_eq!(result.impostor_count, 1);
        assert_eq!(result.culled_count, 2);
        assert_eq!(
            result.full_count
                + result.reduced_count
                + result.lod_count
                + result.impostor_count
                + result.culled_count,
            8
        );
    }

    #[test]
    fn minimum_residence_holds_then_allows_demotion_without_oscillation() {
        let mut value = input(1, 70_000, 10);
        value.prior = Some(IndividualTierStateV1 {
            generation: 7,
            representation: RepresentationTierV1::Full,
            accepted_tick: 100,
        });
        let held = IndividualTierPlanV1::build(
            7,
            digest(9),
            129,
            IndividualTierPolicyV1::PRODUCTION,
            IndividualTierBudgetV1::PRODUCTION,
            vec![value],
        )
        .unwrap();
        assert_eq!(held.decisions[0].representation, RepresentationTierV1::Full);
        let released = IndividualTierPlanV1::build(
            7,
            digest(9),
            130,
            IndividualTierPolicyV1::PRODUCTION,
            IndividualTierBudgetV1::PRODUCTION,
            vec![value],
        )
        .unwrap();
        assert_eq!(
            released.decisions[0].representation,
            RepresentationTierV1::Impostor
        );
    }

    #[test]
    fn epoch_change_resets_prior_residence() {
        let mut value = input(1, 70_000, 10);
        value.prior = Some(IndividualTierStateV1 {
            generation: 6,
            representation: RepresentationTierV1::Full,
            accepted_tick: 119,
        });
        assert_eq!(
            plan(vec![value]).decisions[0].representation,
            RepresentationTierV1::Impostor
        );
    }

    #[test]
    fn missing_forms_use_typed_fallbacks_and_never_silent_drop() {
        let mut lod = input(1, 45_000, 150);
        lod.availability.lod = false;
        let mut impostor = input(2, 75_000, 40);
        impostor.availability.impostor = false;
        let result = plan(vec![lod, impostor]);
        assert_eq!(
            result.decision(&digest(1)).unwrap().fallback,
            TierFallbackReasonV1::LodUnavailable
        );
        assert_eq!(
            result.decision(&digest(2)).unwrap().fallback,
            TierFallbackReasonV1::ImpostorUnavailable
        );
        assert!(
            result
                .decisions
                .iter()
                .all(|decision| decision.representation != RepresentationTierV1::Culled)
        );
    }

    #[test]
    fn duplicate_semantic_identity_fails_closed() {
        let mut values = vec![input(1, 1, 1), input(2, 2, 2)];
        values[1].semantic_entity = values[0].semantic_entity;
        assert_eq!(
            IndividualTierPlanV1::build(
                1,
                digest(9),
                1,
                IndividualTierPolicyV1::PRODUCTION,
                IndividualTierBudgetV1::PRODUCTION,
                values
            ),
            Err(IndividualTierErrorV1::DuplicateSemanticEntity)
        );
    }

    #[test]
    fn malformed_policy_budget_and_count_fail_closed() {
        let mut policy = IndividualTierPolicyV1::PRODUCTION;
        policy.full_exit_mm = policy.full_enter_mm - 1;
        assert_eq!(policy.validate(), Err(IndividualTierErrorV1::InvalidPolicy));
        let mut budget = IndividualTierBudgetV1::PRODUCTION;
        budget.max_full = budget.max_visible + 1;
        assert_eq!(budget.validate(), Err(IndividualTierErrorV1::InvalidBudget));
        assert_eq!(
            IndividualTierPlanV1::build(
                0,
                digest(9),
                1,
                IndividualTierPolicyV1::PRODUCTION,
                IndividualTierBudgetV1::PRODUCTION,
                vec![input(1, 1, 1)]
            ),
            Err(IndividualTierErrorV1::InvalidCount)
        );
    }

    #[test]
    fn canonical_record_is_frozen_exact_and_rejects_trailing_or_reserved_bytes() {
        let decision = plan(vec![input(1, 5_000, 1_000)]).decisions[0];
        let bytes = decision.to_canonical_bytes();
        assert_eq!(&bytes[..6], b"R1DT\x01\0");
        assert_eq!(
            IndividualTierDecisionV1::from_canonical_bytes(&bytes),
            Ok(decision)
        );
        let mut trailing = bytes.to_vec();
        trailing.push(0);
        assert_eq!(
            IndividualTierDecisionV1::from_canonical_bytes(&trailing),
            Err(IndividualTierErrorV1::MalformedLength)
        );
        let mut reserved = bytes;
        reserved[159] = 1;
        assert_eq!(
            IndividualTierDecisionV1::from_canonical_bytes(&reserved),
            Err(IndividualTierErrorV1::NonCanonicalReservedBytes)
        );
    }

    #[test]
    fn tick_derived_animation_and_fade_are_replay_equal() {
        let left = plan(vec![input(1, 5_000, 1_000), input(3, 45_000, 150)]);
        let right = plan(vec![input(3, 45_000, 150), input(1, 5_000, 1_000)]);
        assert_eq!(left.decision_root, right.decision_root);
        assert_eq!(left.decisions, right.decisions);
        let lod = left.decision(&digest(3)).unwrap();
        assert_eq!(lod.sample_tick, 120);
        assert!(lod.should_sample_animation(120));
        assert!(!lod.should_sample_animation(121));
    }

    #[test]
    fn each_priority_key_field_changes_winner() {
        let tiny = IndividualTierBudgetV1 {
            max_visible: 2,
            max_full: 1,
            max_reduced: 2,
            max_lod: 2,
            max_impostor: 2,
            max_full_shadows: 2,
            max_proxy_shadows: 2,
        };
        let run = |a: IndividualTierInputV1, b: IndividualTierInputV1| {
            IndividualTierPlanV1::build(
                1,
                digest(9),
                1,
                IndividualTierPolicyV1::PRODUCTION,
                tiny,
                vec![a, b],
            )
            .unwrap()
        };
        let mut a = input(1, 5_000, 1_000);
        let mut b = input(2, 5_000, 1_000);
        a.importance = 200;
        b.importance = 100;
        assert_eq!(
            run(a, b).decision(&digest(1)).unwrap().representation,
            RepresentationTierV1::Full
        );
        a.importance = 100;
        b.importance = 100;
        a.screen_size_milli = 1_100;
        assert_eq!(
            run(a, b).decision(&digest(1)).unwrap().representation,
            RepresentationTierV1::Full
        );
        a.screen_size_milli = 1_000;
        a.distance_mm = 4_000;
        assert_eq!(
            run(a, b).decision(&digest(1)).unwrap().representation,
            RepresentationTierV1::Full
        );
    }
}
