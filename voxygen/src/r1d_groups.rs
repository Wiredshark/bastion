//! Production compatibility seam for renderer-owned R1D groups.
//!
//! The current live client does not expose an authoritative renderer group
//! feed. Consequently this seam accepts groups only when they already exist in
//! the sealed presentation frame; the certification lane marks its source as
//! an explicit packet fixture.

use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

use bastion_renderer_r0d::{
    group_representation::{
        GroupBudgetV1, GroupDeclarationV1, GroupDigestV1, GroupMemberSlotV1, GroupPolicyV1,
        GroupPriorStateV1, GroupRepresentationErrorV1, GroupRepresentationPlanV1,
        GroupRepresentationTierV1, GroupSourceProvenanceV1, GroupTransitionV1,
    },
    presentation::PresentationFrameV1,
};

use crate::r1a_presentation::ProductionPresentationInputV1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProductionGroupEvidenceV1 {
    pub generation: u64,
    pub frame_digest: GroupDigestV1,
    pub individual_plan_root: GroupDigestV1,
    pub group_plan_root: GroupDigestV1,
    pub group_count: u32,
    pub member_count: u32,
    pub individual_group_count: u32,
    pub formation_group_count: u32,
    pub aggregate_group_count: u32,
    pub protected_member_count: u32,
    pub transition_count: u32,
    pub fixture_group_count: u32,
    pub authoritative_group_count: u32,
    pub line_count: u32,
    pub column_count: u32,
    pub wedge_count: u32,
    pub grid_count: u32,
    pub procession_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionMemberGroupV1 {
    pub group_id: GroupDigestV1,
    pub group_plan_root: GroupDigestV1,
    pub group_tier: GroupRepresentationTierV1,
    pub formation: bastion_renderer_r0d::group_representation::FormationKindV1,
    pub transition: GroupTransitionV1,
    pub slot: GroupMemberSlotV1,
}

#[derive(Clone, Debug, Default)]
struct ProductionGroupStateV1 {
    plan: Option<GroupRepresentationPlanV1>,
    by_uid: BTreeMap<u64, ProductionMemberGroupV1>,
    evidence: Option<ProductionGroupEvidenceV1>,
}

static STATE: OnceLock<Mutex<ProductionGroupStateV1>> = OnceLock::new();

fn state() -> &'static Mutex<ProductionGroupStateV1> {
    STATE.get_or_init(|| Mutex::new(ProductionGroupStateV1::default()))
}

pub fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = ProductionGroupStateV1::default();
    }
}

pub fn update(
    frame: &PresentationFrameV1,
    input: &ProductionPresentationInputV1,
) -> Result<Option<ProductionGroupEvidenceV1>, GroupRepresentationErrorV1> {
    if frame.groups().is_empty() {
        reset();
        return Ok(None);
    }
    let individual =
        crate::r1d_tiers::latest_plan().ok_or(GroupRepresentationErrorV1::StaleGeneration)?;
    let prior = state()
        .lock()
        .ok()
        .and_then(|state| state.plan.clone())
        .map(|plan| {
            plan.groups
                .into_iter()
                .map(|group| GroupPriorStateV1 {
                    group_id: group.group_id,
                    leader_id: group.leader_id,
                    member_ids: group.member_ids,
                    formation: group.formation,
                    tier: group.tier,
                    accepted_tick: group.accepted_tick,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut uid_by_semantic = BTreeMap::new();
    for entity in &input.entities {
        let semantic = crate::r1a_presentation::production_entity_semantic_id(entity.uid)
            .map_err(|_| GroupRepresentationErrorV1::MissingMember([0; 32]))?;
        if uid_by_semantic.insert(semantic, entity.uid).is_some() {
            return Err(GroupRepresentationErrorV1::DuplicateMembership(semantic));
        }
    }
    let mut declarations = Vec::with_capacity(input.groups.len());
    for group in &input.groups {
        let leader_id = crate::r1a_presentation::production_entity_semantic_id(group.leader_uid)
            .map_err(|_| GroupRepresentationErrorV1::InvalidLeader([0; 32]))?;
        let mut protected_member_ids = group
            .protected_member_uids
            .iter()
            .map(|uid| {
                crate::r1a_presentation::production_entity_semantic_id(*uid)
                    .map_err(|_| GroupRepresentationErrorV1::InvalidProtectedMember([0; 32]))
            })
            .collect::<Result<Vec<_>, _>>()?;
        protected_member_ids.sort_unstable();
        declarations.push(GroupDeclarationV1 {
            group_id: group.semantic_id,
            leader_id,
            protected_member_ids,
            formation: group.formation,
            source_provenance: group.source_provenance,
            source_capability_digest: group.source_capability_digest,
        });
    }
    let policy_tick = crate::r1d_scale::current_policy_tick(input.simulation_tick)
        .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?;
    let plan = GroupRepresentationPlanV1::build_with_policy_tick(
        frame,
        &individual,
        policy_tick,
        input.camera_position_mm,
        GroupPolicyV1::PRODUCTION,
        GroupBudgetV1::PRODUCTION,
        declarations,
        &prior,
    )?;
    let mut by_uid = BTreeMap::new();
    for group in &plan.groups {
        for slot in &group.member_slots {
            let uid = uid_by_semantic
                .get(&slot.semantic_id)
                .copied()
                .ok_or(GroupRepresentationErrorV1::MissingMember(slot.semantic_id))?;
            if by_uid
                .insert(uid, ProductionMemberGroupV1 {
                    group_id: group.group_id,
                    group_plan_root: plan.plan_root,
                    group_tier: group.tier,
                    formation: group.formation,
                    transition: group.transition,
                    slot: *slot,
                })
                .is_some()
            {
                return Err(GroupRepresentationErrorV1::DuplicateMembership(
                    slot.semantic_id,
                ));
            }
        }
    }
    let evidence = evidence(&plan)?;
    let mut state = state()
        .lock()
        .map_err(|_| GroupRepresentationErrorV1::InvalidCount)?;
    state.plan = Some(plan);
    state.by_uid = by_uid;
    state.evidence = Some(evidence);
    Ok(Some(evidence))
}

#[must_use]
pub fn latest_evidence() -> Option<ProductionGroupEvidenceV1> {
    state().lock().ok().and_then(|state| state.evidence)
}

#[must_use]
pub fn member_group(uid: u64) -> Option<ProductionMemberGroupV1> {
    state()
        .lock()
        .ok()
        .and_then(|state| state.by_uid.get(&uid).copied())
}

/// Canonical inspection/picking evidence for protected members. UIDs are
/// diagnostics at this compatibility seam; full renderer semantic digests
/// remain the authority inside the plan.
#[must_use]
pub fn protected_uid_csv() -> String {
    state()
        .lock()
        .ok()
        .map(|state| {
            state
                .by_uid
                .iter()
                .filter_map(|(uid, member)| member.slot.protected.then_some(uid.to_string()))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

fn evidence(
    plan: &GroupRepresentationPlanV1,
) -> Result<ProductionGroupEvidenceV1, GroupRepresentationErrorV1> {
    let count = |predicate: fn(
        &bastion_renderer_r0d::group_representation::GroupRepresentationV1,
    ) -> bool| {
        u32::try_from(plan.groups.iter().filter(|group| predicate(group)).count())
            .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)
    };
    Ok(ProductionGroupEvidenceV1 {
        generation: plan.generation,
        frame_digest: plan.frame_digest,
        individual_plan_root: plan.individual_plan_root,
        group_plan_root: plan.plan_root,
        group_count: u32::try_from(plan.groups.len())
            .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?,
        member_count: plan.member_count,
        individual_group_count: plan.individual_group_count,
        formation_group_count: plan.formation_group_count,
        aggregate_group_count: plan.aggregate_group_count,
        protected_member_count: plan.protected_member_count,
        transition_count: plan.transition_count,
        fixture_group_count: count(|group| {
            group.source_provenance == GroupSourceProvenanceV1::DeclaredPacketFixture
        })?,
        authoritative_group_count: count(|group| {
            group.source_provenance == GroupSourceProvenanceV1::AuthoritativePresentation
        })?,
        line_count: count(|group| {
            group.formation == bastion_renderer_r0d::group_representation::FormationKindV1::Line
        })?,
        column_count: count(|group| {
            group.formation == bastion_renderer_r0d::group_representation::FormationKindV1::Column
        })?,
        wedge_count: count(|group| {
            group.formation == bastion_renderer_r0d::group_representation::FormationKindV1::Wedge
        })?,
        grid_count: count(|group| {
            group.formation == bastion_renderer_r0d::group_representation::FormationKindV1::Grid
        })?,
        procession_count: count(|group| {
            group.formation
                == bastion_renderer_r0d::group_representation::FormationKindV1::Procession
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::{
        group_representation::{FormationKindV1, GroupKindV1},
        presentation::PresentationVisualPolicyV1,
    };

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    #[test]
    fn absent_production_source_is_explicit_and_clears_prior_state() {
        reset();
        assert_eq!(latest_evidence(), None);
        assert_eq!(member_group(7), None);
        assert_eq!(protected_uid_csv(), "");
    }

    #[test]
    fn explicit_fixture_runs_through_presentation_tiers_groups_and_picking() {
        let _guard = crate::r1a_presentation::TEST_LOCK_V1.lock().unwrap();
        reset();
        crate::r1d_tiers::reset();
        crate::r1a_presentation::reset();
        let entities = (1..=6)
            .map(
                |uid| crate::r1a_presentation::ProductionPresentationEntityInputV1 {
                    uid,
                    body: "Humanoid(Dwarf)".to_owned(),
                    position_mm: [i64::try_from(uid).unwrap() * 10_000, 0, 0],
                },
            )
            .collect::<Vec<_>>();
        let source_capability_digest = digest(50);
        let input = ProductionPresentationInputV1 {
            simulation_tick: 300,
            camera_position_mm: [0, 0, 2_000],
            anchor_uid: 1,
            anchor_body: "Humanoid(Dwarf)".to_owned(),
            anchor_position_mm: [10_000, 0, 0],
            entities,
            groups: vec![
                crate::r1a_presentation::ProductionPresentationGroupInputV1 {
                    semantic_id: digest(40),
                    kind_tag: GroupKindV1::Formation as u16,
                    member_uids: vec![1, 2, 3],
                    leader_uid: 1,
                    protected_member_uids: vec![1],
                    formation: FormationKindV1::Wedge,
                    source_provenance: GroupSourceProvenanceV1::DeclaredPacketFixture,
                    source_capability_digest,
                },
                crate::r1a_presentation::ProductionPresentationGroupInputV1 {
                    semantic_id: digest(41),
                    kind_tag: GroupKindV1::Formation as u16,
                    member_uids: vec![4, 5, 6],
                    leader_uid: 4,
                    protected_member_uids: Vec::new(),
                    formation: FormationKindV1::Grid,
                    source_provenance: GroupSourceProvenanceV1::DeclaredPacketFixture,
                    source_capability_digest,
                },
            ],
            render_islands: Vec::new(),
            terrain_resource: digest(2),
            environment_digest: digest(3),
            cloud_milli: 0,
            rain_milli: 0,
            wind_mm_s: [0, 0],
            daylight_milli: 500,
            environment_sample: crate::r1f_environment::sample_from_production(
                300,
                100_000.0,
                160.0,
                common::weather::WeatherKind::Clear,
                0.0,
                0.0,
                [0.0, 0.0],
                0.0,
                16,
                16,
            )
            .unwrap(),
            policy: PresentationVisualPolicyV1 {
                policy_digest: digest(4),
                terrain_view_distance: 16,
                entity_view_distance: 16,
                figure_lod_distance: 350,
                sprite_distance: 250,
                particles_enabled: true,
                weapon_trails_enabled: true,
                flashing_lights_enabled: false,
            },
        };
        let frame = crate::r1a_presentation::prepare_frame(&input, digest(9)).unwrap();
        crate::r1d_tiers::update(&frame, &input).unwrap();
        let evidence = update(&frame, &input).unwrap().unwrap();
        assert_eq!(evidence.group_count, 2);
        assert_eq!(evidence.member_count, 6);
        assert_eq!(evidence.fixture_group_count, 2);
        assert_eq!(evidence.protected_member_count, 1);
        assert_eq!(protected_uid_csv(), "1");
        let picked = member_group(1).unwrap();
        assert!(picked.slot.protected);
        assert_eq!(picked.group_id, digest(40));
    }
}
