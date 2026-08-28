//! Production adapter for deterministic figure-shadow importance budgets.

use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

use bastion_renderer_r0d::{
    individual_tier::{IndividualTierDecisionV1, RepresentationTierV1},
    presentation::PresentationFrameV1,
    shadow::{
        ShadowAvailabilityV1, ShadowBudgetV1, ShadowDecisionV1, ShadowErrorV1, ShadowFallbackV1,
        ShadowInputV1, ShadowPlanPublisherV1, ShadowPlanV1, ShadowPolicyV1, ShadowStateV1,
        ShadowTierV1,
    },
};

use crate::r1a_presentation::ProductionPresentationInputV1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ShadowProductionEvidenceV1 {
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub frame_digest: [u8; 32],
    pub decision_root: [u8; 32],
    pub detailed_count: u32,
    pub proxy_count: u32,
    pub reduced_frequency_count: u32,
    pub group_contact_count: u32,
    pub none_count: u32,
    pub fallback_count: u32,
    pub protected_detailed_count: u32,
    pub proxy_unavailable_fallbacks: u32,
    pub reduced_frequency_unavailable_fallbacks: u32,
    pub group_contact_unavailable_fallbacks: u32,
    pub detailed_budget_fallbacks: u32,
    pub max_detailed: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ShadowAdapterErrorV1 {
    MissingTierPlan,
    TierPlanMismatch,
    MissingEntityTier,
    DuplicateUid,
    Core(ShadowErrorV1),
    StatePoisoned,
}

impl From<ShadowErrorV1> for ShadowAdapterErrorV1 {
    fn from(value: ShadowErrorV1) -> Self { Self::Core(value) }
}

#[derive(Debug, Default)]
struct ShadowAdapterStateV1 {
    publisher: ShadowPlanPublisherV1,
    by_uid: BTreeMap<u64, ShadowDecisionV1>,
    evidence: Option<ShadowProductionEvidenceV1>,
}

static STATE: OnceLock<Mutex<ShadowAdapterStateV1>> = OnceLock::new();

fn state() -> &'static Mutex<ShadowAdapterStateV1> {
    STATE.get_or_init(|| Mutex::new(ShadowAdapterStateV1::default()))
}

pub(crate) fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = ShadowAdapterStateV1::default();
    }
}

#[must_use]
pub(crate) fn decision_for_uid(uid: u64) -> Option<ShadowDecisionV1> {
    state()
        .lock()
        .ok()
        .and_then(|state| state.by_uid.get(&uid).copied())
}

#[must_use]
pub(crate) fn latest_evidence() -> Option<ShadowProductionEvidenceV1> {
    state().lock().ok().and_then(|state| state.evidence)
}

#[must_use]
pub(crate) fn should_render_uid(uid: u64, tick: u64) -> Option<bool> {
    decision_for_uid(uid).map(|decision| decision.should_render(tick))
}

pub(crate) fn update(
    frame: &PresentationFrameV1,
    input: &ProductionPresentationInputV1,
) -> Result<ShadowProductionEvidenceV1, ShadowAdapterErrorV1> {
    let generation = frame.generation().client_applied_generation;
    let frame_digest = frame.frame_digest();
    let tiers = crate::r1d_tiers::latest_plan().ok_or(ShadowAdapterErrorV1::MissingTierPlan)?;
    if tiers.generation != generation || tiers.frame_digest != frame_digest {
        return Err(ShadowAdapterErrorV1::TierPlanMismatch);
    }
    let prior = state()
        .lock()
        .map_err(|_| ShadowAdapterErrorV1::StatePoisoned)?
        .publisher
        .current();
    let mut uid_by_semantic = BTreeMap::new();
    let mut inputs = Vec::with_capacity(input.entities.len());
    for entity in &input.entities {
        let semantic = crate::r1a_presentation::production_entity_semantic_id(entity.uid)
            .map_err(|_| ShadowAdapterErrorV1::MissingEntityTier)?;
        if uid_by_semantic.insert(semantic, entity.uid).is_some() {
            return Err(ShadowAdapterErrorV1::DuplicateUid);
        }
        let tier = tiers
            .decision(&semantic)
            .copied()
            .ok_or(ShadowAdapterErrorV1::MissingEntityTier)?;
        let protected = entity.uid == input.anchor_uid
            || crate::r1d_groups::member_group(entity.uid)
                .is_some_and(|member| member.slot.protected);
        inputs.push(input_from_tier(
            tier,
            protected,
            prior
                .as_ref()
                .and_then(|plan| plan.decision(&semantic))
                .copied(),
        ));
    }
    let plan = ShadowPlanV1::build(
        generation,
        frame_digest,
        tiers.tick,
        ShadowPolicyV1::PRODUCTION,
        ShadowBudgetV1::PRODUCTION,
        inputs,
    )?;
    let evidence = evidence(&plan)?;
    let mut by_uid = BTreeMap::new();
    for decision in &plan.decisions {
        let uid = uid_by_semantic
            .get(&decision.semantic_entity)
            .copied()
            .ok_or(ShadowAdapterErrorV1::MissingEntityTier)?;
        if by_uid.insert(uid, *decision).is_some() {
            return Err(ShadowAdapterErrorV1::DuplicateUid);
        }
    }
    let mut state = state()
        .lock()
        .map_err(|_| ShadowAdapterErrorV1::StatePoisoned)?;
    let published = state.publisher.publish(plan)?;
    state.by_uid = by_uid;
    state.evidence = Some(evidence);
    debug_assert_eq!(published.decision_root, evidence.decision_root);
    Ok(evidence)
}

fn input_from_tier(
    tier: IndividualTierDecisionV1,
    protected: bool,
    prior: Option<ShadowDecisionV1>,
) -> ShadowInputV1 {
    ShadowInputV1 {
        semantic_entity: tier.semantic_entity,
        importance: tier.importance,
        screen_size_milli: tier.screen_size_milli,
        distance_mm: tier.distance_mm,
        protected,
        availability: ShadowAvailabilityV1 {
            detailed: tier.representation != RepresentationTierV1::Culled,
            // Current production packaging declares a deterministic proxy
            // fixture, but Voxygen has no distinct proxy-shadow draw seam.
            proxy: false,
            // The directed map is rebuilt each frame, so per-figure cadence
            // cannot truthfully reuse an older map entry.
            reduced_frequency: false,
            // Cheap point shadows exist globally, but no group-contact
            // receipt is bound to an explicit presentation group.
            group_contact: false,
        },
        prior: prior.map(|prior| ShadowStateV1 {
            generation: prior.generation,
            requested: prior.requested,
            active: prior.active,
            accepted_tick: prior.accepted_tick,
        }),
    }
}

fn evidence(plan: &ShadowPlanV1) -> Result<ShadowProductionEvidenceV1, ShadowAdapterErrorV1> {
    let count_fallback = |fallback| {
        u32::try_from(
            plan.decisions
                .iter()
                .filter(|decision| decision.fallback == fallback)
                .count(),
        )
        .map_err(|_| ShadowAdapterErrorV1::Core(ShadowErrorV1::LengthOverflow))
    };
    Ok(ShadowProductionEvidenceV1 {
        presentation_generation: plan.generation,
        simulation_tick: plan.tick,
        frame_digest: plan.frame_digest,
        decision_root: plan.decision_root,
        detailed_count: plan.detailed_count,
        proxy_count: plan.proxy_count,
        reduced_frequency_count: plan.reduced_frequency_count,
        group_contact_count: plan.group_contact_count,
        none_count: plan.none_count,
        fallback_count: plan.fallback_count,
        protected_detailed_count: u32::try_from(
            plan.decisions
                .iter()
                .filter(|decision| {
                    decision.protected && decision.active == ShadowTierV1::S0Detailed
                })
                .count(),
        )
        .map_err(|_| ShadowAdapterErrorV1::Core(ShadowErrorV1::LengthOverflow))?,
        proxy_unavailable_fallbacks: count_fallback(ShadowFallbackV1::ProxyUnavailableToDetailed)?
            + count_fallback(ShadowFallbackV1::ProxyUnavailableToNone)?,
        reduced_frequency_unavailable_fallbacks: count_fallback(
            ShadowFallbackV1::ReducedFrequencyUnavailable,
        )?,
        group_contact_unavailable_fallbacks: count_fallback(
            ShadowFallbackV1::GroupContactUnavailable,
        )?,
        detailed_budget_fallbacks: count_fallback(ShadowFallbackV1::DetailedBudgetExhausted)?,
        max_detailed: ShadowBudgetV1::PRODUCTION.max_detailed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::individual_tier::{
        AnimationTierV1, INDIVIDUAL_TIER_VERSION_V1, IndividualShadowTierV1, TierFallbackReasonV1,
    };

    fn tier(id: u8, distance_mm: u32) -> IndividualTierDecisionV1 {
        IndividualTierDecisionV1 {
            version: INDIVIDUAL_TIER_VERSION_V1,
            generation: 1,
            frame_digest: [9; 32],
            semantic_entity: [id; 32],
            representation: RepresentationTierV1::Full,
            animation: AnimationTierV1::EveryTick,
            shadow: IndividualShadowTierV1::Full,
            fallback: TierFallbackReasonV1::None,
            importance: u16::from(100 - id),
            screen_size_milli: 10_000_000 / distance_mm,
            distance_mm,
            accepted_tick: 1,
            sample_tick: 1,
            fade_phase: 0,
            priority_ordinal: u32::from(id),
        }
    }

    #[test]
    fn production_capability_mapping_is_honest() {
        let value = input_from_tier(tier(1, 30_000), false, None);
        assert!(value.availability.detailed);
        assert!(!value.availability.proxy);
        assert!(!value.availability.reduced_frequency);
        assert!(!value.availability.group_contact);
    }

    #[test]
    fn protected_and_budget_evidence_is_exact() {
        let first = input_from_tier(tier(1, 10_000), true, None);
        let second = input_from_tier(tier(2, 30_000), false, None);
        let plan = ShadowPlanV1::build(
            1,
            [9; 32],
            300,
            ShadowPolicyV1::PRODUCTION,
            ShadowBudgetV1::PRODUCTION,
            vec![second, first],
        )
        .unwrap();
        let evidence = evidence(&plan).unwrap();
        assert_eq!(evidence.protected_detailed_count, 1);
        assert_eq!(evidence.detailed_count, 2);
        assert_eq!(evidence.proxy_unavailable_fallbacks, 1);
        assert_eq!(evidence.max_detailed, 64);
    }

    #[test]
    fn stale_plan_does_not_replace_published_state() {
        let mut publisher = ShadowPlanPublisherV1::default();
        let make = |generation| {
            ShadowPlanV1::build(
                generation,
                [generation as u8; 32],
                300,
                ShadowPolicyV1::PRODUCTION,
                ShadowBudgetV1::PRODUCTION,
                vec![input_from_tier(tier(1, 10_000), true, None)],
            )
            .unwrap()
        };
        publisher.publish(make(2)).unwrap();
        assert_eq!(
            publisher.publish(make(1)),
            Err(ShadowErrorV1::StaleGeneration)
        );
        assert_eq!(publisher.current().unwrap().generation, 2);
    }
}
