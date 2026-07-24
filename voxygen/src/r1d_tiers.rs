//! Production compatibility seam for renderer-owned R1D individual tiers.
//!
//! One immutable presentation frame and one fixed-point camera position produce
//! a complete canonical plan. The plan is published whole behind a mutex;
//! readers never observe partial allocation.

use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

use bastion_renderer_r0d::{
    individual_tier::{
        IndividualTierBudgetV1, IndividualTierDecisionV1, IndividualTierErrorV1,
        IndividualTierInputV1, IndividualTierPlanV1, IndividualTierPolicyV1, IndividualTierStateV1,
        RepresentationTierV1, TierContentAvailabilityV1, TierFallbackReasonV1,
    },
    presentation::PresentationFrameV1,
};

use crate::r1a_presentation::ProductionPresentationInputV1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProductionTierEvidenceV1 {
    pub generation: u64,
    pub frame_digest: [u8; 32],
    pub decision_root: [u8; 32],
    pub full_count: u32,
    pub reduced_count: u32,
    pub lod_count: u32,
    pub impostor_count: u32,
    pub culled_count: u32,
    pub full_shadow_count: u32,
    pub proxy_shadow_count: u32,
    pub fallback_count: u32,
    pub transition_count: u32,
    pub max_visible: u32,
    pub max_full: u32,
    pub max_reduced: u32,
    pub max_lod: u32,
    pub max_impostor: u32,
    pub full_budget_fallbacks: u32,
    pub animation_budget_fallbacks: u32,
    pub lod_budget_fallbacks: u32,
    pub impostor_budget_fallbacks: u32,
    pub visible_budget_fallbacks: u32,
    pub lod_unavailable_fallbacks: u32,
    pub impostor_unavailable_fallbacks: u32,
    pub shadow_proxy_unavailable_fallbacks: u32,
}

#[derive(Clone, Debug, Default)]
struct ProductionTierStateV1 {
    plan: Option<IndividualTierPlanV1>,
    by_uid: BTreeMap<u64, IndividualTierDecisionV1>,
    evidence: Option<ProductionTierEvidenceV1>,
}

static STATE: OnceLock<Mutex<ProductionTierStateV1>> = OnceLock::new();

fn state() -> &'static Mutex<ProductionTierStateV1> {
    STATE.get_or_init(|| Mutex::new(ProductionTierStateV1::default()))
}

pub fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = ProductionTierStateV1::default();
    }
}

pub fn update(
    frame: &PresentationFrameV1,
    input: &ProductionPresentationInputV1,
) -> Result<ProductionTierEvidenceV1, IndividualTierErrorV1> {
    let generation = frame.generation().client_applied_generation;
    let frame_digest = frame.frame_digest();
    let tick = input.simulation_tick;
    let prior = state().lock().ok().and_then(|state| state.plan.clone());
    let mut by_semantic = BTreeMap::new();
    if let Some(plan) = prior {
        for decision in plan.decisions {
            by_semantic.insert(decision.semantic_entity, IndividualTierStateV1 {
                generation: decision.generation,
                representation: decision.representation,
                accepted_tick: decision.accepted_tick,
            });
        }
    }
    let mut uid_by_semantic = BTreeMap::new();
    let mut candidates = Vec::with_capacity(input.entities.len());
    for entity in &input.entities {
        let semantic_entity = crate::r1a_presentation::production_entity_semantic_id(entity.uid)
            .map_err(|_| IndividualTierErrorV1::InvalidCount)?;
        if uid_by_semantic
            .insert(semantic_entity, entity.uid)
            .is_some()
        {
            return Err(IndividualTierErrorV1::DuplicateSemanticEntity);
        }
        let distance_mm = distance_mm(input.camera_position_mm, entity.position_mm)?;
        let screen_size_milli = 10_000_000_u32 / distance_mm.max(1);
        candidates.push(IndividualTierInputV1 {
            semantic_entity,
            importance: if entity.uid == input.anchor_uid {
                u16::MAX
            } else {
                1_000
            },
            screen_size_milli,
            distance_mm,
            availability: TierContentAvailabilityV1 {
                lod: true,
                // The accepted package has a deterministic fixture section,
                // but Voxygen has no billboard production draw seam yet.
                impostor: false,
                shadow_proxy: false,
            },
            prior: by_semantic.get(&semantic_entity).copied(),
        });
    }
    let plan = IndividualTierPlanV1::build(
        generation,
        frame_digest,
        tick,
        IndividualTierPolicyV1::PRODUCTION,
        IndividualTierBudgetV1::PRODUCTION,
        candidates,
    )?;
    let mut by_uid = BTreeMap::new();
    for decision in &plan.decisions {
        let Some(uid) = uid_by_semantic.get(&decision.semantic_entity) else {
            return Err(IndividualTierErrorV1::InvalidCount);
        };
        by_uid.insert(*uid, *decision);
    }
    let transition_count = plan
        .decisions
        .iter()
        .filter(|decision| {
            by_semantic
                .get(&decision.semantic_entity)
                .is_some_and(|prior| prior.representation != decision.representation)
        })
        .count()
        .try_into()
        .map_err(|_| IndividualTierErrorV1::LengthOverflow)?;
    let evidence = evidence(&plan, transition_count)?;
    let mut state = state()
        .lock()
        .map_err(|_| IndividualTierErrorV1::InvalidCount)?;
    state.plan = Some(plan);
    state.by_uid = by_uid;
    state.evidence = Some(evidence);
    Ok(evidence)
}

#[must_use]
pub fn decision_for_uid(uid: u64) -> Option<IndividualTierDecisionV1> {
    state()
        .lock()
        .ok()
        .and_then(|state| state.by_uid.get(&uid).copied())
}

#[must_use]
pub fn latest_evidence() -> Option<ProductionTierEvidenceV1> {
    state().lock().ok().and_then(|state| state.evidence)
}

#[must_use]
pub fn forced_lod(uid: u64) -> Option<usize> {
    match decision_for_uid(uid)?.representation {
        RepresentationTierV1::Full | RepresentationTierV1::ReducedAnimation => Some(0),
        RepresentationTierV1::Lod => Some(1),
        // A supported core impostor degrades to the lowest real mesh LOD at
        // this explicit compatibility boundary until a billboard seam exists.
        RepresentationTierV1::Impostor => Some(2),
        RepresentationTierV1::Culled => None,
    }
}

fn evidence(
    plan: &IndividualTierPlanV1,
    transition_count: u32,
) -> Result<ProductionTierEvidenceV1, IndividualTierErrorV1> {
    let fallback_count = |reason| {
        u32::try_from(
            plan.decisions
                .iter()
                .filter(|decision| decision.fallback == reason)
                .count(),
        )
        .map_err(|_| IndividualTierErrorV1::LengthOverflow)
    };
    let budget = IndividualTierBudgetV1::PRODUCTION;
    Ok(ProductionTierEvidenceV1 {
        generation: plan.generation,
        frame_digest: plan.frame_digest,
        decision_root: plan.decision_root,
        full_count: plan.full_count,
        reduced_count: plan.reduced_count,
        lod_count: plan.lod_count,
        impostor_count: plan.impostor_count,
        culled_count: plan.culled_count,
        full_shadow_count: plan.full_shadow_count,
        proxy_shadow_count: plan.proxy_shadow_count,
        fallback_count: plan.fallback_count,
        transition_count,
        max_visible: budget.max_visible,
        max_full: budget.max_full,
        max_reduced: budget.max_reduced,
        max_lod: budget.max_lod,
        max_impostor: budget.max_impostor,
        full_budget_fallbacks: fallback_count(TierFallbackReasonV1::FullBudget)?,
        animation_budget_fallbacks: fallback_count(TierFallbackReasonV1::AnimationBudget)?,
        lod_budget_fallbacks: fallback_count(TierFallbackReasonV1::LodBudget)?,
        impostor_budget_fallbacks: fallback_count(TierFallbackReasonV1::ImpostorBudget)?,
        visible_budget_fallbacks: fallback_count(TierFallbackReasonV1::VisibleBudget)?,
        lod_unavailable_fallbacks: fallback_count(TierFallbackReasonV1::LodUnavailable)?,
        impostor_unavailable_fallbacks: fallback_count(TierFallbackReasonV1::ImpostorUnavailable)?,
        shadow_proxy_unavailable_fallbacks: fallback_count(
            TierFallbackReasonV1::ShadowProxyUnavailable,
        )?,
    })
}

fn distance_mm(left: [i64; 3], right: [i64; 3]) -> Result<u32, IndividualTierErrorV1> {
    let mut square = 0_u128;
    for axis in 0..3 {
        let delta = i128::from(left[axis])
            .checked_sub(i128::from(right[axis]))
            .ok_or(IndividualTierErrorV1::LengthOverflow)?;
        let magnitude = delta.unsigned_abs();
        square = square
            .checked_add(
                magnitude
                    .checked_mul(magnitude)
                    .ok_or(IndividualTierErrorV1::LengthOverflow)?,
            )
            .ok_or(IndividualTierErrorV1::LengthOverflow)?;
    }
    u32::try_from(integer_sqrt(square)).map_err(|_| IndividualTierErrorV1::LengthOverflow)
}

fn integer_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut lower = 1_u128;
    let mut upper = value.min(u128::from(u64::MAX));
    while lower < upper {
        let middle = lower + (upper - lower).div_ceil(2);
        if middle <= value / middle {
            lower = middle;
        } else {
            upper = middle - 1;
        }
    }
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_distance_is_exact_and_checked() {
        assert_eq!(distance_mm([0, 0, 0], [3_000, 4_000, 0]), Ok(5_000));
        assert_eq!(integer_sqrt(15), 3);
        assert_eq!(integer_sqrt(16), 4);
    }
}
