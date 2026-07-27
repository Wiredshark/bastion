//! Bounded deterministic certification policy for the R1D scale closure.
//!
//! This module does not create simulation authority. It supplies an explicit
//! renderer-only camera script and policy clock for the declared flat-arena
//! fixture while the authoritative server remains frozen.

use bastion_renderer_r0d::domain_hash_v1;

pub const SCALE_VISIBLE_COUNT_V1: u32 = 512;
pub const SCALE_GROUP_SIZE_V1: usize = 32;
pub const SCALE_GROUP_COUNT_V1: u32 = 16;
pub const SCALE_CAPTURE_COUNT_V1: u64 = 5;
pub const SCALE_POLICY_STEP_TICKS_V1: u64 = 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScaleCameraSampleV1 {
    pub ordinal: u64,
    pub distance_mm: u32,
}

pub const SCALE_CAMERA_SCRIPT_V1: [ScaleCameraSampleV1; 5] = [
    ScaleCameraSampleV1 {
        ordinal: 0,
        distance_mm: 6_000,
    },
    ScaleCameraSampleV1 {
        ordinal: 1,
        distance_mm: 36_000,
    },
    ScaleCameraSampleV1 {
        ordinal: 2,
        distance_mm: 120_000,
    },
    ScaleCameraSampleV1 {
        ordinal: 3,
        distance_mm: 36_000,
    },
    ScaleCameraSampleV1 {
        ordinal: 4,
        distance_mm: 6_000,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScaleErrorV1 {
    InvalidOrdinal,
    InvalidCount,
    DuplicateIdentity,
    MissingProtectedMember,
    TickOverflow,
    Hash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScaleSampleRecordV1 {
    pub ordinal: u64,
    pub policy_tick: u64,
    pub camera_distance_mm: u32,
    pub population_count: u32,
    pub member_set_digest: [u8; 32],
    pub tier_root: [u8; 32],
    pub full_count: u32,
    pub reduced_count: u32,
    pub lod_count: u32,
    pub impostor_count: u32,
    pub culled_count: u32,
    pub group_root: [u8; 32],
    pub group_count: u32,
    pub group_member_count: u32,
    pub protected_member_count: u32,
    pub tier_transition_count: u32,
    pub group_transition_count: u32,
}

#[must_use]
pub fn enabled() -> bool { std::env::var_os("BASTION_R1D_SCALE_SMOKE").is_some() }

/// The request counter advances synchronously when a screenshot command is
/// accepted. Callback completion order never advances this script. Once all
/// declared samples have been requested, the final camera pose is held.
#[must_use]
pub fn camera_sample_for_request_ordinal(requested: u64) -> ScaleCameraSampleV1 {
    let last = u64::try_from(SCALE_CAMERA_SCRIPT_V1.len() - 1).unwrap_or(0);
    SCALE_CAMERA_SCRIPT_V1[usize::try_from(requested.min(last)).unwrap_or(0)]
}

pub fn policy_tick_for_request_ordinal(
    base_tick: u64,
    requested: u64,
) -> Result<u64, ScaleErrorV1> {
    let sample = camera_sample_for_request_ordinal(requested);
    base_tick
        .checked_add(
            sample
                .ordinal
                .checked_mul(SCALE_POLICY_STEP_TICKS_V1)
                .ok_or(ScaleErrorV1::TickOverflow)?,
        )
        .ok_or(ScaleErrorV1::TickOverflow)
}

pub fn current_policy_tick(base_tick: u64) -> Result<u64, ScaleErrorV1> {
    if !enabled() {
        return Ok(base_tick);
    }
    policy_tick_for_request_ordinal(
        base_tick,
        crate::render::bastion_r0d::capture_requested_ordinal_v1(),
    )
}

#[must_use]
pub fn current_camera_sample() -> Option<ScaleCameraSampleV1> {
    enabled().then(|| {
        camera_sample_for_request_ordinal(
            crate::render::bastion_r0d::capture_requested_ordinal_v1(),
        )
    })
}

pub fn canonical_member_set_digest_v1(
    identities: impl IntoIterator<Item = [u8; 32]>,
) -> Result<[u8; 32], ScaleErrorV1> {
    let mut identities = identities.into_iter().collect::<Vec<_>>();
    if identities.len() != usize::try_from(SCALE_VISIBLE_COUNT_V1).unwrap_or(usize::MAX) {
        return Err(ScaleErrorV1::InvalidCount);
    }
    identities.sort_unstable();
    if identities.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ScaleErrorV1::DuplicateIdentity);
    }
    let mut bytes = Vec::with_capacity(4 + identities.len() * 32);
    bytes.extend_from_slice(&SCALE_VISIBLE_COUNT_V1.to_le_bytes());
    for identity in identities {
        bytes.extend_from_slice(&identity);
    }
    domain_hash_v1("bastion/r1d/scale-member-set", 1, 0, &bytes).map_err(|_| ScaleErrorV1::Hash)
}

impl ScaleSampleRecordV1 {
    pub fn validate(self) -> Result<Self, ScaleErrorV1> {
        let sample = SCALE_CAMERA_SCRIPT_V1
            .get(usize::try_from(self.ordinal).map_err(|_| ScaleErrorV1::InvalidOrdinal)?)
            .ok_or(ScaleErrorV1::InvalidOrdinal)?;
        let tier_total = self
            .full_count
            .checked_add(self.reduced_count)
            .and_then(|value| value.checked_add(self.lod_count))
            .and_then(|value| value.checked_add(self.impostor_count))
            .and_then(|value| value.checked_add(self.culled_count))
            .ok_or(ScaleErrorV1::InvalidCount)?;
        if self.camera_distance_mm != sample.distance_mm
            || self.population_count != SCALE_VISIBLE_COUNT_V1
            || tier_total != self.population_count
            || self.group_count != SCALE_GROUP_COUNT_V1
            || self.group_member_count != self.population_count
            || self.protected_member_count == 0
            || self.member_set_digest == [0; 32]
            || self.tier_root == [0; 32]
            || self.group_root == [0; 32]
        {
            return Err(if self.protected_member_count == 0 {
                ScaleErrorV1::MissingProtectedMember
            } else {
                ScaleErrorV1::InvalidCount
            });
        }
        Ok(self)
    }

    pub fn digest(self) -> Result<[u8; 32], ScaleErrorV1> {
        let value = self.validate()?;
        let mut bytes = Vec::with_capacity(216);
        bytes.extend_from_slice(&value.ordinal.to_le_bytes());
        bytes.extend_from_slice(&value.policy_tick.to_le_bytes());
        bytes.extend_from_slice(&value.camera_distance_mm.to_le_bytes());
        bytes.extend_from_slice(&value.population_count.to_le_bytes());
        bytes.extend_from_slice(&value.member_set_digest);
        bytes.extend_from_slice(&value.tier_root);
        for count in [
            value.full_count,
            value.reduced_count,
            value.lod_count,
            value.impostor_count,
            value.culled_count,
        ] {
            bytes.extend_from_slice(&count.to_le_bytes());
        }
        bytes.extend_from_slice(&value.group_root);
        for count in [
            value.group_count,
            value.group_member_count,
            value.protected_member_count,
            value.tier_transition_count,
            value.group_transition_count,
        ] {
            bytes.extend_from_slice(&count.to_le_bytes());
        }
        domain_hash_v1("bastion/r1d/scale-sample", 1, 0, &bytes).map_err(|_| ScaleErrorV1::Hash)
    }
}

#[cfg(test)]
pub fn trajectory_root_v1(
    records: impl IntoIterator<Item = ScaleSampleRecordV1>,
) -> Result<[u8; 32], ScaleErrorV1> {
    let mut records = records.into_iter().collect::<Vec<_>>();
    if records.len() != SCALE_CAMERA_SCRIPT_V1.len() {
        return Err(ScaleErrorV1::InvalidCount);
    }
    records.sort_unstable_by_key(|record| record.ordinal);
    let mut bytes = Vec::with_capacity(4 + records.len() * 32);
    bytes.extend_from_slice(
        &u32::try_from(records.len())
            .map_err(|_| ScaleErrorV1::InvalidCount)?
            .to_le_bytes(),
    );
    for (ordinal, record) in records.into_iter().enumerate() {
        if record.ordinal != u64::try_from(ordinal).map_err(|_| ScaleErrorV1::InvalidOrdinal)? {
            return Err(ScaleErrorV1::InvalidOrdinal);
        }
        bytes.extend_from_slice(&record.digest()?);
    }
    domain_hash_v1("bastion/r1d/scale-trajectory", 1, 0, &bytes).map_err(|_| ScaleErrorV1::Hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::{
        group_representation::{
            FormationKindV1, GroupBudgetV1, GroupDeclarationV1, GroupKindV1, GroupPolicyV1,
            GroupPriorStateV1, GroupRepresentationPlanV1, GroupRepresentationTierV1,
            GroupSourceProvenanceV1,
        },
        individual_tier::{
            IndividualTierBudgetV1, IndividualTierInputV1, IndividualTierPlanV1,
            IndividualTierPolicyV1, IndividualTierStateV1, TierContentAvailabilityV1,
        },
        presentation::{
            PresentationEntityV1, PresentationEnvironmentV1, PresentationFrameDraftV1,
            PresentationFrameV1, PresentationGenerationV1, PresentationGroupV1,
            PresentationVisualPolicyV1,
        },
    };

    fn identities() -> Vec<[u8; 32]> {
        (0..SCALE_VISIBLE_COUNT_V1)
            .map(|ordinal| {
                domain_hash_v1(
                    "bastion/r1d/scale-test-entity",
                    1,
                    0,
                    &ordinal.to_le_bytes(),
                )
                .unwrap()
            })
            .collect()
    }

    fn record(ordinal: u64, member_set_digest: [u8; 32]) -> ScaleSampleRecordV1 {
        let sample = camera_sample_for_request_ordinal(ordinal);
        ScaleSampleRecordV1 {
            ordinal,
            policy_tick: policy_tick_for_request_ordinal(300, ordinal).unwrap(),
            camera_distance_mm: sample.distance_mm,
            population_count: SCALE_VISIBLE_COUNT_V1,
            member_set_digest,
            tier_root: [11; 32],
            full_count: 32,
            reduced_count: 96,
            lod_count: 192,
            impostor_count: 0,
            culled_count: 192,
            group_root: [12; 32],
            group_count: SCALE_GROUP_COUNT_V1,
            group_member_count: SCALE_VISIBLE_COUNT_V1,
            protected_member_count: 1,
            tier_transition_count: u32::try_from(ordinal).unwrap(),
            group_transition_count: u32::try_from(ordinal).unwrap(),
        }
    }

    fn scale_fixture() -> (PresentationFrameV1, Vec<GroupDeclarationV1>) {
        let identities = identities();
        let mut entities = Vec::with_capacity(identities.len());
        let mut groups = Vec::with_capacity(usize::try_from(SCALE_GROUP_COUNT_V1).unwrap());
        let mut declarations = Vec::with_capacity(usize::try_from(SCALE_GROUP_COUNT_V1).unwrap());
        for group_ordinal in 0..SCALE_GROUP_COUNT_V1 {
            let first = usize::try_from(group_ordinal).unwrap() * SCALE_GROUP_SIZE_V1;
            let member_ids = identities[first..first + SCALE_GROUP_SIZE_V1].to_vec();
            let group_id = domain_hash_v1(
                "bastion/r1d/scale-test-group",
                1,
                0,
                &group_ordinal.to_le_bytes(),
            )
            .unwrap();
            for (member_ordinal, semantic_id) in member_ids.iter().copied().enumerate() {
                let global = first + member_ordinal;
                let row = i64::try_from(group_ordinal).unwrap();
                let column = i64::try_from(member_ordinal).unwrap() - 16;
                entities.push(PresentationEntityV1 {
                    semantic_id,
                    figure_resource: [70; 32],
                    group_id: Some(group_id),
                    position_mm: [row * 6_000, column * 2_000, 0],
                    orientation_q30: [0, 0, 0, 1 << 30],
                    scale_milli: 1_000,
                    state_tag: 1,
                    state_digest: domain_hash_v1(
                        "bastion/r1d/scale-test-state",
                        1,
                        0,
                        &u32::try_from(global).unwrap().to_le_bytes(),
                    )
                    .unwrap(),
                });
            }
            groups.push(PresentationGroupV1 {
                semantic_id: group_id,
                kind_tag: GroupKindV1::Formation as u16,
                member_ids: member_ids.clone(),
                state_digest: domain_hash_v1(
                    "bastion/r1d/scale-test-group-state",
                    1,
                    0,
                    &group_ordinal.to_le_bytes(),
                )
                .unwrap(),
            });
            declarations.push(GroupDeclarationV1 {
                group_id,
                leader_id: member_ids[0],
                protected_member_ids: (group_ordinal == 0)
                    .then_some(vec![member_ids[0]])
                    .unwrap_or_default(),
                formation: match group_ordinal % 4 {
                    0 => FormationKindV1::Wedge,
                    1 => FormationKindV1::Grid,
                    2 => FormationKindV1::Line,
                    _ => FormationKindV1::Column,
                },
                source_provenance: GroupSourceProvenanceV1::DeclaredPacketFixture,
                source_capability_digest: [71; 32],
            });
        }
        entities.sort_unstable_by_key(|entity| entity.semantic_id);
        groups.sort_unstable_by_key(|group| group.semantic_id);
        declarations.sort_unstable_by_key(|group| group.group_id);
        let frame = PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: 1,
                simulation_tick: 300,
                coherent_snapshot_root: [72; 32],
            },
            entities,
            groups,
            events: Vec::new(),
            environment: PresentationEnvironmentV1 {
                terrain_root: [73; 32],
                environment_digest: [74; 32],
                cloud_milli: 0,
                rain_milli: 0,
                wind_mm_s: [0, 0],
                daylight_milli: 500,
            },
            visual_policy: PresentationVisualPolicyV1 {
                policy_digest: [75; 32],
                terrain_view_distance: 16,
                entity_view_distance: 16,
                figure_lod_distance: 350,
                sprite_distance: 250,
                particles_enabled: true,
                weapon_trails_enabled: true,
                flashing_lights_enabled: false,
            },
            renderer_required_resources: vec![[70; 32]],
            complete: true,
        }
        .seal()
        .unwrap();
        (frame, declarations)
    }

    fn exercise_scale_plans(reverse_inputs: bool) -> (Vec<ScaleSampleRecordV1>, bool, bool) {
        let (frame, declarations) = scale_fixture();
        let member_set_digest = canonical_member_set_digest_v1(
            frame.entities().iter().map(|entity| entity.semantic_id),
        )
        .unwrap();
        let mut individual_prior = Vec::<IndividualTierStateV1>::new();
        let mut group_prior = Vec::<GroupPriorStateV1>::new();
        let mut records = Vec::new();
        let mut saw_individual_transition = false;
        let mut saw_aggregate_group = false;
        for sample in SCALE_CAMERA_SCRIPT_V1 {
            let policy_tick = policy_tick_for_request_ordinal(300, sample.ordinal).unwrap();
            let camera_x = -i64::from(sample.distance_mm);
            let mut inputs = frame
                .entities()
                .iter()
                .map(|entity| {
                    let distance_mm = u32::try_from(
                        i128::from(entity.position_mm[0])
                            .checked_sub(i128::from(camera_x))
                            .unwrap()
                            .unsigned_abs()
                            .max(1),
                    )
                    .unwrap();
                    let prior = individual_prior.iter().zip(frame.entities()).find_map(
                        |(state, prior_entity)| {
                            (prior_entity.semantic_id == entity.semantic_id).then_some(*state)
                        },
                    );
                    IndividualTierInputV1 {
                        semantic_entity: entity.semantic_id,
                        importance: if entity.semantic_id == frame.entities()[0].semantic_id {
                            u16::MAX
                        } else {
                            1_000
                        },
                        screen_size_milli: 10_000_000_u32 / distance_mm,
                        distance_mm,
                        availability: TierContentAvailabilityV1 {
                            lod: true,
                            impostor: false,
                            shadow_proxy: false,
                        },
                        prior,
                    }
                })
                .collect::<Vec<_>>();
            if reverse_inputs {
                inputs.reverse();
            }
            let individual = IndividualTierPlanV1::build(
                1,
                frame.frame_digest(),
                policy_tick,
                IndividualTierPolicyV1::PRODUCTION,
                IndividualTierBudgetV1::PRODUCTION,
                inputs,
            )
            .unwrap();
            saw_individual_transition |= individual.decisions.iter().any(|decision| {
                individual_prior
                    .iter()
                    .zip(frame.entities())
                    .any(|(prior, entity)| {
                        entity.semantic_id == decision.semantic_entity
                            && prior.representation != decision.representation
                    })
            });
            let mut declaration_order = declarations.clone();
            if reverse_inputs {
                declaration_order.reverse();
            }
            let groups = GroupRepresentationPlanV1::build_with_policy_tick(
                &frame,
                &individual,
                policy_tick,
                [camera_x, 0, 2_000],
                GroupPolicyV1::PRODUCTION,
                GroupBudgetV1::PRODUCTION,
                declaration_order,
                &group_prior,
            )
            .unwrap();
            saw_aggregate_group |= groups
                .groups
                .iter()
                .any(|group| group.tier == GroupRepresentationTierV1::AggregateFar);
            records.push(ScaleSampleRecordV1 {
                ordinal: sample.ordinal,
                policy_tick,
                camera_distance_mm: sample.distance_mm,
                population_count: u32::try_from(individual.decisions.len()).unwrap(),
                member_set_digest,
                tier_root: individual.decision_root,
                full_count: individual.full_count,
                reduced_count: individual.reduced_count,
                lod_count: individual.lod_count,
                impostor_count: individual.impostor_count,
                culled_count: individual.culled_count,
                group_root: groups.plan_root,
                group_count: u32::try_from(groups.groups.len()).unwrap(),
                group_member_count: groups.member_count,
                protected_member_count: groups.protected_member_count,
                tier_transition_count: individual
                    .decisions
                    .iter()
                    .filter(|decision| {
                        individual_prior
                            .iter()
                            .zip(frame.entities())
                            .any(|(prior, entity)| {
                                entity.semantic_id == decision.semantic_entity
                                    && prior.representation != decision.representation
                            })
                    })
                    .count()
                    .try_into()
                    .unwrap(),
                group_transition_count: groups.transition_count,
            });
            individual_prior = individual
                .decisions
                .iter()
                .map(|decision| IndividualTierStateV1 {
                    generation: decision.generation,
                    representation: decision.representation,
                    accepted_tick: decision.accepted_tick,
                })
                .collect();
            group_prior = groups
                .groups
                .iter()
                .map(|group| GroupPriorStateV1 {
                    group_id: group.group_id,
                    leader_id: group.leader_id,
                    member_ids: group.member_ids.clone(),
                    formation: group.formation,
                    tier: group.tier,
                    accepted_tick: group.accepted_tick,
                })
                .collect();
        }
        (records, saw_individual_transition, saw_aggregate_group)
    }

    #[test]
    fn camera_script_is_fixed_near_far_near_and_policy_ticks_are_monotonic() {
        assert_eq!(SCALE_CAMERA_SCRIPT_V1.map(|sample| sample.distance_mm), [
            6_000, 36_000, 120_000, 36_000, 6_000
        ]);
        assert_eq!(policy_tick_for_request_ordinal(300, 0), Ok(300));
        assert_eq!(policy_tick_for_request_ordinal(300, 4), Ok(420));
        assert_eq!(
            policy_tick_for_request_ordinal(u64::MAX, 1),
            Err(ScaleErrorV1::TickOverflow)
        );
        assert_eq!(
            camera_sample_for_request_ordinal(99),
            SCALE_CAMERA_SCRIPT_V1[4]
        );
    }

    #[test]
    fn full_512_identity_set_is_order_invariant_and_rejects_loss_or_duplicates() {
        let source = identities();
        let expected = canonical_member_set_digest_v1(source.clone()).unwrap();
        let mut reversed = source.clone();
        reversed.reverse();
        assert_eq!(canonical_member_set_digest_v1(reversed), Ok(expected));
        assert_eq!(
            canonical_member_set_digest_v1(source[..511].iter().copied()),
            Err(ScaleErrorV1::InvalidCount)
        );
        let mut duplicate = source;
        duplicate[511] = duplicate[510];
        assert_eq!(
            canonical_member_set_digest_v1(duplicate),
            Err(ScaleErrorV1::DuplicateIdentity)
        );
    }

    #[test]
    fn trajectory_is_replay_equal_under_sample_enumeration_permutation() {
        let members = canonical_member_set_digest_v1(identities()).unwrap();
        let records = (0..SCALE_CAPTURE_COUNT_V1)
            .map(|ordinal| record(ordinal, members))
            .collect::<Vec<_>>();
        let expected = trajectory_root_v1(records.clone()).unwrap();
        let mut reversed = records;
        reversed.reverse();
        assert_eq!(trajectory_root_v1(reversed), Ok(expected));
    }

    #[test]
    fn scale_record_requires_complete_budget_partition_and_protected_continuity() {
        let members = canonical_member_set_digest_v1(identities()).unwrap();
        let mut value = record(0, members);
        value.culled_count -= 1;
        assert_eq!(value.validate(), Err(ScaleErrorV1::InvalidCount));
        let mut value = record(0, members);
        value.protected_member_count = 0;
        assert_eq!(value.validate(), Err(ScaleErrorV1::MissingProtectedMember));
    }

    #[test]
    fn real_512_plans_are_order_invariant_replay_equal_and_transition_without_loss() {
        let (forward, saw_individual_transition, saw_aggregate_group) = exercise_scale_plans(false);
        let (reversed, reversed_transition, reversed_aggregate) = exercise_scale_plans(true);
        assert_eq!(forward, reversed);
        assert_eq!(
            trajectory_root_v1(forward.clone()),
            trajectory_root_v1(reversed)
        );
        assert!(saw_individual_transition && reversed_transition);
        assert!(saw_aggregate_group && reversed_aggregate);
        assert!(forward.iter().all(|record| record.validate().is_ok()));
    }
}
