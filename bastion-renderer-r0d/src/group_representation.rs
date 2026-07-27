//! Deterministic renderer-owned group and formation representation.
//!
//! Membership is accepted only from a sealed [`PresentationFrameV1`] plus an
//! explicit declaration that binds leader, formation and source provenance to
//! the corresponding [`PresentationGroupV1`]. This module never infers groups
//! from proximity, appearance, iteration order or renderer observations.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::{
    individual_tier::{IndividualTierDecisionV1, IndividualTierPlanV1},
    presentation::PresentationFrameV1,
};

pub const GROUP_REPRESENTATION_VERSION_V1: u16 = 1;
pub const GROUP_REPRESENTATION_MAX_GROUPS_V1: usize = 1_024;
pub const GROUP_REPRESENTATION_MAX_MEMBERS_V1: usize = 4_096;
pub const GROUP_REPRESENTATION_MAX_PROTECTED_V1: usize = 256;
pub const GROUP_REPRESENTATION_MAX_BYTES_V1: usize = 512 * 1024;
const MAGIC: &[u8; 8] = b"BASGRP01";

pub type GroupDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum GroupKindV1 {
    Crew = 1,
    Party = 2,
    Patrol = 3,
    Formation = 4,
    Procession = 5,
    Crowd = 6,
}

impl GroupKindV1 {
    fn from_tag(tag: u16) -> Result<Self, GroupRepresentationErrorV1> {
        match tag {
            1 => Ok(Self::Crew),
            2 => Ok(Self::Party),
            3 => Ok(Self::Patrol),
            4 => Ok(Self::Formation),
            5 => Ok(Self::Procession),
            6 => Ok(Self::Crowd),
            _ => Err(GroupRepresentationErrorV1::UnsupportedKind(tag)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FormationKindV1 {
    Line = 1,
    Column = 2,
    Wedge = 3,
    Grid = 4,
    Procession = 5,
}

impl FormationKindV1 {
    fn from_tag(tag: u8) -> Result<Self, GroupRepresentationErrorV1> {
        match tag {
            1 => Ok(Self::Line),
            2 => Ok(Self::Column),
            3 => Ok(Self::Wedge),
            4 => Ok(Self::Grid),
            5 => Ok(Self::Procession),
            _ => Err(GroupRepresentationErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum GroupRepresentationTierV1 {
    IndividualNear = 1,
    FormationMiddle = 2,
    AggregateFar = 3,
}

impl GroupRepresentationTierV1 {
    fn from_tag(tag: u8) -> Result<Self, GroupRepresentationErrorV1> {
        match tag {
            1 => Ok(Self::IndividualNear),
            2 => Ok(Self::FormationMiddle),
            3 => Ok(Self::AggregateFar),
            _ => Err(GroupRepresentationErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum GroupTransitionV1 {
    Stable = 1,
    Join = 2,
    Leave = 3,
    Split = 4,
    Merge = 5,
    LeaderLoss = 6,
    SelectedMemberPromotion = 7,
    FormationChange = 8,
}

impl GroupTransitionV1 {
    fn from_tag(tag: u8) -> Result<Self, GroupRepresentationErrorV1> {
        match tag {
            1 => Ok(Self::Stable),
            2 => Ok(Self::Join),
            3 => Ok(Self::Leave),
            4 => Ok(Self::Split),
            5 => Ok(Self::Merge),
            6 => Ok(Self::LeaderLoss),
            7 => Ok(Self::SelectedMemberPromotion),
            8 => Ok(Self::FormationChange),
            _ => Err(GroupRepresentationErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum GroupSourceProvenanceV1 {
    AuthoritativePresentation = 1,
    DeclaredPacketFixture = 2,
}

impl GroupSourceProvenanceV1 {
    fn from_tag(tag: u8) -> Result<Self, GroupRepresentationErrorV1> {
        match tag {
            1 => Ok(Self::AuthoritativePresentation),
            2 => Ok(Self::DeclaredPacketFixture),
            _ => Err(GroupRepresentationErrorV1::UnknownTag),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroupPolicyV1 {
    pub aggregate_enter_mm: u32,
    pub aggregate_exit_mm: u32,
    pub formation_enter_mm: u32,
    pub formation_exit_mm: u32,
    pub slot_spacing_mm: u32,
    pub minimum_residence_ticks: u64,
    pub transition_ticks: u16,
}

impl GroupPolicyV1 {
    pub const PRODUCTION: Self = Self {
        aggregate_enter_mm: 72_000,
        aggregate_exit_mm: 64_000,
        formation_enter_mm: 24_000,
        formation_exit_mm: 18_000,
        slot_spacing_mm: 2_000,
        minimum_residence_ticks: 30,
        transition_ticks: 12,
    };

    fn validate(self) -> Result<Self, GroupRepresentationErrorV1> {
        if self.formation_enter_mm >= self.aggregate_enter_mm
            || self.formation_exit_mm >= self.formation_enter_mm
            || self.aggregate_exit_mm >= self.aggregate_enter_mm
            || self.slot_spacing_mm == 0
            || self.slot_spacing_mm > 100_000
            || self.minimum_residence_ticks == 0
            || self.transition_ticks == 0
        {
            return Err(GroupRepresentationErrorV1::InvalidPolicy);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroupBudgetV1 {
    pub max_groups: u32,
    pub max_members: u32,
    pub max_individual_groups: u32,
    pub max_formation_groups: u32,
    pub max_aggregate_groups: u32,
}

impl GroupBudgetV1 {
    pub const PRODUCTION: Self = Self {
        max_groups: 1_024,
        max_members: 4_096,
        max_individual_groups: 128,
        max_formation_groups: 512,
        max_aggregate_groups: 1_024,
    };

    fn validate(self) -> Result<Self, GroupRepresentationErrorV1> {
        if self.max_groups == 0
            || usize::try_from(self.max_groups).ok() > Some(GROUP_REPRESENTATION_MAX_GROUPS_V1)
            || self.max_members == 0
            || usize::try_from(self.max_members).ok() > Some(GROUP_REPRESENTATION_MAX_MEMBERS_V1)
            || self.max_individual_groups > self.max_groups
            || self.max_formation_groups > self.max_groups
            || self.max_aggregate_groups > self.max_groups
        {
            return Err(GroupRepresentationErrorV1::InvalidBudget);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupDeclarationV1 {
    pub group_id: GroupDigestV1,
    pub leader_id: GroupDigestV1,
    pub protected_member_ids: Vec<GroupDigestV1>,
    pub formation: FormationKindV1,
    pub source_provenance: GroupSourceProvenanceV1,
    pub source_capability_digest: GroupDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupPriorStateV1 {
    pub group_id: GroupDigestV1,
    pub leader_id: GroupDigestV1,
    pub member_ids: Vec<GroupDigestV1>,
    pub formation: FormationKindV1,
    pub tier: GroupRepresentationTierV1,
    pub accepted_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GroupMemberSlotV1 {
    pub semantic_id: GroupDigestV1,
    pub slot_ordinal: u32,
    pub offset_mm: [i32; 3],
    pub protected: bool,
    pub individual_decision_digest: GroupDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupRepresentationV1 {
    pub version: u16,
    pub generation: u64,
    pub simulation_tick: u64,
    pub frame_digest: GroupDigestV1,
    pub individual_plan_root: GroupDigestV1,
    pub group_id: GroupDigestV1,
    pub kind: GroupKindV1,
    pub source_state_digest: GroupDigestV1,
    pub source_provenance: GroupSourceProvenanceV1,
    pub source_capability_digest: GroupDigestV1,
    pub member_ids: Vec<GroupDigestV1>,
    pub leader_id: GroupDigestV1,
    pub representative_id: GroupDigestV1,
    pub protected_member_ids: Vec<GroupDigestV1>,
    pub formation: FormationKindV1,
    pub member_slots: Vec<GroupMemberSlotV1>,
    pub bounds_min_mm: [i64; 3],
    pub bounds_max_mm: [i64; 3],
    pub centroid_mm: [i64; 3],
    pub distance_mm: u32,
    pub tier: GroupRepresentationTierV1,
    pub transition: GroupTransitionV1,
    pub accepted_tick: u64,
    pub transition_phase: u16,
    pub aggregate_digest: GroupDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupRepresentationPlanV1 {
    pub generation: u64,
    pub simulation_tick: u64,
    pub frame_digest: GroupDigestV1,
    pub individual_plan_root: GroupDigestV1,
    pub groups: Vec<GroupRepresentationV1>,
    pub individual_group_count: u32,
    pub formation_group_count: u32,
    pub aggregate_group_count: u32,
    pub protected_member_count: u32,
    pub transition_count: u32,
    pub member_count: u32,
    pub plan_root: GroupDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupRepresentationErrorV1 {
    InvalidPolicy,
    InvalidBudget,
    InvalidCount,
    MissingDeclaration(GroupDigestV1),
    ExtraDeclaration(GroupDigestV1),
    MissingMember(GroupDigestV1),
    DuplicateMembership(GroupDigestV1),
    DuplicateGroup(GroupDigestV1),
    InvalidLeader(GroupDigestV1),
    InvalidProtectedMember(GroupDigestV1),
    UnsupportedKind(u16),
    StaleGeneration,
    MissingIndividualDecision(GroupDigestV1),
    BudgetExhausted,
    LengthOverflow,
    CoordinateOverflow,
    InvalidMagic,
    UnsupportedVersion,
    UnknownTag,
    Malformed,
    TrailingBytes,
}

impl GroupRepresentationPlanV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        frame: &PresentationFrameV1,
        individual_plan: &IndividualTierPlanV1,
        camera_position_mm: [i64; 3],
        policy: GroupPolicyV1,
        budget: GroupBudgetV1,
        declarations: Vec<GroupDeclarationV1>,
        prior: &[GroupPriorStateV1],
    ) -> Result<Self, GroupRepresentationErrorV1> {
        Self::build_with_policy_tick(
            frame,
            individual_plan,
            frame.generation().simulation_tick,
            camera_position_mm,
            policy,
            budget,
            declarations,
            prior,
        )
    }

    /// Builds a renderer representation plan using an explicit deterministic
    /// policy tick. Production normally uses the presentation simulation tick;
    /// bounded certification scripts may advance this renderer-only policy
    /// clock while the authoritative fixture is frozen.
    #[allow(clippy::too_many_arguments)]
    pub fn build_with_policy_tick(
        frame: &PresentationFrameV1,
        individual_plan: &IndividualTierPlanV1,
        policy_tick: u64,
        camera_position_mm: [i64; 3],
        policy: GroupPolicyV1,
        budget: GroupBudgetV1,
        declarations: Vec<GroupDeclarationV1>,
        prior: &[GroupPriorStateV1],
    ) -> Result<Self, GroupRepresentationErrorV1> {
        let policy = policy.validate()?;
        let budget = budget.validate()?;
        let generation = frame.generation().client_applied_generation;
        let tick = policy_tick;
        if generation == 0
            || individual_plan.generation != generation
            || individual_plan.frame_digest != frame.frame_digest()
            || tick < frame.generation().simulation_tick
        {
            return Err(GroupRepresentationErrorV1::StaleGeneration);
        }
        if frame.groups().is_empty()
            || frame.groups().len() > GROUP_REPRESENTATION_MAX_GROUPS_V1
            || frame.groups().len()
                > usize::try_from(budget.max_groups)
                    .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?
        {
            return Err(GroupRepresentationErrorV1::InvalidCount);
        }

        let entities = frame
            .entities()
            .iter()
            .map(|entity| (entity.semantic_id, entity))
            .collect::<BTreeMap<_, _>>();
        let decisions = individual_plan
            .decisions
            .iter()
            .map(|decision| (decision.semantic_entity, decision))
            .collect::<BTreeMap<_, _>>();
        let mut declaration_map = BTreeMap::new();
        for mut declaration in declarations {
            declaration.protected_member_ids.sort_unstable();
            if declaration.source_capability_digest == [0; 32]
                || declaration.protected_member_ids.len() > GROUP_REPRESENTATION_MAX_PROTECTED_V1
                || declaration
                    .protected_member_ids
                    .windows(2)
                    .any(|pair| pair[0] == pair[1])
                || declaration_map
                    .insert(declaration.group_id, declaration)
                    .is_some()
            {
                return Err(GroupRepresentationErrorV1::InvalidCount);
            }
        }
        let frame_ids = frame
            .groups()
            .iter()
            .map(|group| group.semantic_id)
            .collect::<BTreeSet<_>>();
        if let Some(extra) = declaration_map
            .keys()
            .find(|group| !frame_ids.contains(*group))
        {
            return Err(GroupRepresentationErrorV1::ExtraDeclaration(*extra));
        }

        let prior_map = prior
            .iter()
            .map(|state| (state.group_id, state))
            .collect::<BTreeMap<_, _>>();
        if prior_map.len() != prior.len() {
            return Err(GroupRepresentationErrorV1::DuplicateGroup([0; 32]));
        }
        let prior_owner = membership_owner(
            prior
                .iter()
                .map(|state| (state.group_id, state.member_ids.as_slice())),
        )?;
        let current_owner = membership_owner(
            frame
                .groups()
                .iter()
                .map(|group| (group.semantic_id, group.member_ids.as_slice())),
        )?;

        let mut groups = Vec::with_capacity(frame.groups().len());
        let mut member_total = 0_usize;
        for group in frame.groups() {
            let declaration = declaration_map.remove(&group.semantic_id).ok_or(
                GroupRepresentationErrorV1::MissingDeclaration(group.semantic_id),
            )?;
            let kind = GroupKindV1::from_tag(group.kind_tag)?;
            if group.member_ids.is_empty()
                || group.member_ids.len() > GROUP_REPRESENTATION_MAX_MEMBERS_V1
            {
                return Err(GroupRepresentationErrorV1::InvalidCount);
            }
            member_total = member_total
                .checked_add(group.member_ids.len())
                .ok_or(GroupRepresentationErrorV1::LengthOverflow)?;
            if member_total
                > usize::try_from(budget.max_members)
                    .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?
            {
                return Err(GroupRepresentationErrorV1::BudgetExhausted);
            }
            if group
                .member_ids
                .binary_search(&declaration.leader_id)
                .is_err()
            {
                return Err(GroupRepresentationErrorV1::InvalidLeader(
                    declaration.leader_id,
                ));
            }
            if let Some(missing) = declaration
                .protected_member_ids
                .iter()
                .find(|member| group.member_ids.binary_search(member).is_err())
            {
                return Err(GroupRepresentationErrorV1::InvalidProtectedMember(*missing));
            }
            let mut bounds_min = [i64::MAX; 3];
            let mut bounds_max = [i64::MIN; 3];
            let mut sum = [0_i128; 3];
            let mut member_decisions = Vec::with_capacity(group.member_ids.len());
            for member in &group.member_ids {
                let entity = entities
                    .get(member)
                    .ok_or(GroupRepresentationErrorV1::MissingMember(*member))?;
                let decision = decisions.get(member).ok_or(
                    GroupRepresentationErrorV1::MissingIndividualDecision(*member),
                )?;
                for axis in 0..3 {
                    bounds_min[axis] = bounds_min[axis].min(entity.position_mm[axis]);
                    bounds_max[axis] = bounds_max[axis].max(entity.position_mm[axis]);
                    sum[axis] = sum[axis]
                        .checked_add(i128::from(entity.position_mm[axis]))
                        .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?;
                }
                member_decisions.push(**decision);
            }
            let divisor = i128::try_from(group.member_ids.len())
                .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?;
            let mut centroid = [0_i64; 3];
            for axis in 0..3 {
                centroid[axis] = i64::try_from(sum[axis] / divisor)
                    .map_err(|_| GroupRepresentationErrorV1::CoordinateOverflow)?;
            }
            let distance_mm = distance_mm(camera_position_mm, centroid)?;
            let prior_state = prior_map.get(&group.semantic_id).copied();
            let protected = !declaration.protected_member_ids.is_empty();
            let desired = choose_tier(
                distance_mm,
                protected,
                &member_decisions,
                prior_state,
                tick,
                policy,
            );
            let transition = classify_transition(
                group.semantic_id,
                &group.member_ids,
                declaration.leader_id,
                declaration.formation,
                &prior_map,
                &prior_owner,
                &current_owner,
            );
            let accepted_tick = if prior_state.is_some_and(|prior| prior.tier == desired)
                && transition == GroupTransitionV1::Stable
            {
                prior_state.map_or(tick, |prior| prior.accepted_tick)
            } else {
                tick
            };
            let transition_phase = transition_phase(tick, accepted_tick, policy.transition_ticks)?;
            let member_slots = formation_slots(
                &group.member_ids,
                &declaration.protected_member_ids,
                declaration.formation,
                policy.slot_spacing_mm,
                &decisions,
            )?;
            let representative_id = declaration
                .protected_member_ids
                .first()
                .copied()
                .unwrap_or(declaration.leader_id);
            let aggregate_digest = aggregate_digest(
                group.semantic_id,
                group.state_digest,
                &group.member_ids,
                &member_slots,
                bounds_min,
                bounds_max,
                centroid,
                desired,
            );
            groups.push(GroupRepresentationV1 {
                version: GROUP_REPRESENTATION_VERSION_V1,
                generation,
                simulation_tick: tick,
                frame_digest: frame.frame_digest(),
                individual_plan_root: individual_plan.decision_root,
                group_id: group.semantic_id,
                kind,
                source_state_digest: group.state_digest,
                source_provenance: declaration.source_provenance,
                source_capability_digest: declaration.source_capability_digest,
                member_ids: group.member_ids.clone(),
                leader_id: declaration.leader_id,
                representative_id,
                protected_member_ids: declaration.protected_member_ids,
                formation: declaration.formation,
                member_slots,
                bounds_min_mm: bounds_min,
                bounds_max_mm: bounds_max,
                centroid_mm: centroid,
                distance_mm,
                tier: desired,
                transition,
                accepted_tick,
                transition_phase,
                aggregate_digest,
            });
        }
        groups.sort_unstable_by_key(|group| group.group_id);
        let individual_group_count =
            count_tier(&groups, GroupRepresentationTierV1::IndividualNear)?;
        let formation_group_count =
            count_tier(&groups, GroupRepresentationTierV1::FormationMiddle)?;
        let aggregate_group_count = count_tier(&groups, GroupRepresentationTierV1::AggregateFar)?;
        if individual_group_count > budget.max_individual_groups
            || formation_group_count > budget.max_formation_groups
            || aggregate_group_count > budget.max_aggregate_groups
        {
            return Err(GroupRepresentationErrorV1::BudgetExhausted);
        }
        let protected_member_count = groups.iter().try_fold(0_u32, |total, group| {
            total
                .checked_add(
                    u32::try_from(group.protected_member_ids.len())
                        .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?,
                )
                .ok_or(GroupRepresentationErrorV1::LengthOverflow)
        })?;
        let transition_count = u32::try_from(
            groups
                .iter()
                .filter(|group| group.transition != GroupTransitionV1::Stable)
                .count(),
        )
        .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?;
        let member_count =
            u32::try_from(member_total).map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?;
        let plan_root = plan_root(
            generation,
            tick,
            frame.frame_digest(),
            individual_plan.decision_root,
            &groups,
        )?;
        Ok(Self {
            generation,
            simulation_tick: tick,
            frame_digest: frame.frame_digest(),
            individual_plan_root: individual_plan.decision_root,
            groups,
            individual_group_count,
            formation_group_count,
            aggregate_group_count,
            protected_member_count,
            transition_count,
            member_count,
            plan_root,
        })
    }

    pub fn group_for_member(&self, member: GroupDigestV1) -> Option<&GroupRepresentationV1> {
        self.groups
            .iter()
            .find(|group| group.member_ids.binary_search(&member).is_ok())
    }
}

impl GroupRepresentationV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, GroupRepresentationErrorV1> {
        if self.version != GROUP_REPRESENTATION_VERSION_V1
            || self.generation == 0
            || self.frame_digest == [0; 32]
            || self.individual_plan_root == [0; 32]
            || self.group_id == [0; 32]
            || self.source_state_digest == [0; 32]
            || self.source_capability_digest == [0; 32]
            || self.aggregate_digest == [0; 32]
            || self.member_ids.is_empty()
            || self.member_ids.len() != self.member_slots.len()
            || self.member_ids.windows(2).any(|pair| pair[0] >= pair[1])
            || self
                .protected_member_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.member_ids.binary_search(&self.leader_id).is_err()
            || self
                .member_ids
                .binary_search(&self.representative_id)
                .is_err()
            || self
                .protected_member_ids
                .iter()
                .any(|member| self.member_ids.binary_search(member).is_err())
            || (0..3).any(|axis| {
                self.bounds_min_mm[axis] > self.bounds_max_mm[axis]
                    || self.centroid_mm[axis] < self.bounds_min_mm[axis]
                    || self.centroid_mm[axis] > self.bounds_max_mm[axis]
            })
            || self
                .member_ids
                .iter()
                .zip(&self.member_slots)
                .enumerate()
                .any(|(ordinal, (member, slot))| {
                    slot.semantic_id != *member
                        || usize::try_from(slot.slot_ordinal).ok() != Some(ordinal)
                        || slot.protected != self.protected_member_ids.binary_search(member).is_ok()
                        || slot.individual_decision_digest == [0; 32]
                })
        {
            return Err(GroupRepresentationErrorV1::Malformed);
        }
        let mut output = Vec::with_capacity(512 + self.member_ids.len() * 124);
        output.extend_from_slice(MAGIC);
        put_u16(&mut output, self.version);
        put_u64(&mut output, self.generation);
        put_u64(&mut output, self.simulation_tick);
        for digest in [
            self.frame_digest,
            self.individual_plan_root,
            self.group_id,
            self.source_state_digest,
            self.source_capability_digest,
        ] {
            output.extend_from_slice(&digest);
        }
        put_u16(&mut output, self.kind as u16);
        output.push(self.source_provenance as u8);
        output.push(self.formation as u8);
        output.push(self.tier as u8);
        output.push(self.transition as u8);
        output.extend_from_slice(&self.leader_id);
        output.extend_from_slice(&self.representative_id);
        put_u64(&mut output, self.accepted_tick);
        put_u16(&mut output, self.transition_phase);
        put_u32(&mut output, self.distance_mm);
        for values in [self.bounds_min_mm, self.bounds_max_mm, self.centroid_mm] {
            for value in values {
                put_i64(&mut output, value);
            }
        }
        output.extend_from_slice(&self.aggregate_digest);
        put_count(&mut output, self.member_ids.len())?;
        put_count(&mut output, self.protected_member_ids.len())?;
        for member in &self.member_ids {
            output.extend_from_slice(member);
        }
        for member in &self.protected_member_ids {
            output.extend_from_slice(member);
        }
        for slot in &self.member_slots {
            output.extend_from_slice(&slot.semantic_id);
            put_u32(&mut output, slot.slot_ordinal);
            for offset in slot.offset_mm {
                output.extend_from_slice(&offset.to_le_bytes());
            }
            output.push(u8::from(slot.protected));
            output.extend_from_slice(&[0; 3]);
            output.extend_from_slice(&slot.individual_decision_digest);
        }
        if output.len() > GROUP_REPRESENTATION_MAX_BYTES_V1 {
            return Err(GroupRepresentationErrorV1::LengthOverflow);
        }
        Ok(output)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, GroupRepresentationErrorV1> {
        if bytes.len() > GROUP_REPRESENTATION_MAX_BYTES_V1 {
            return Err(GroupRepresentationErrorV1::LengthOverflow);
        }
        let mut reader = Reader::new(bytes);
        if reader.take(8)? != MAGIC {
            return Err(GroupRepresentationErrorV1::InvalidMagic);
        }
        let version = reader.u16()?;
        if version != GROUP_REPRESENTATION_VERSION_V1 {
            return Err(GroupRepresentationErrorV1::UnsupportedVersion);
        }
        let generation = reader.u64()?;
        let simulation_tick = reader.u64()?;
        let frame_digest = reader.digest()?;
        let individual_plan_root = reader.digest()?;
        let group_id = reader.digest()?;
        let source_state_digest = reader.digest()?;
        let source_capability_digest = reader.digest()?;
        let kind = GroupKindV1::from_tag(reader.u16()?)?;
        let source_provenance = GroupSourceProvenanceV1::from_tag(reader.u8()?)?;
        let formation = FormationKindV1::from_tag(reader.u8()?)?;
        let tier = GroupRepresentationTierV1::from_tag(reader.u8()?)?;
        let transition = GroupTransitionV1::from_tag(reader.u8()?)?;
        let leader_id = reader.digest()?;
        let representative_id = reader.digest()?;
        let accepted_tick = reader.u64()?;
        let transition_phase = reader.u16()?;
        let distance_mm = reader.u32()?;
        let bounds_min_mm = reader.i64x3()?;
        let bounds_max_mm = reader.i64x3()?;
        let centroid_mm = reader.i64x3()?;
        let aggregate_digest = reader.digest()?;
        let member_count = reader.count(GROUP_REPRESENTATION_MAX_MEMBERS_V1)?;
        let protected_count = reader.count(GROUP_REPRESENTATION_MAX_PROTECTED_V1)?;
        let mut member_ids = Vec::with_capacity(member_count);
        for _ in 0..member_count {
            member_ids.push(reader.digest()?);
        }
        let mut protected_member_ids = Vec::with_capacity(protected_count);
        for _ in 0..protected_count {
            protected_member_ids.push(reader.digest()?);
        }
        let mut member_slots = Vec::with_capacity(member_count);
        for _ in 0..member_count {
            let semantic_id = reader.digest()?;
            let slot_ordinal = reader.u32()?;
            let offset_mm = reader.i32x3()?;
            let protected = match reader.u8()? {
                0 => false,
                1 => true,
                _ => return Err(GroupRepresentationErrorV1::Malformed),
            };
            if reader.take(3)? != [0, 0, 0] {
                return Err(GroupRepresentationErrorV1::Malformed);
            }
            let individual_decision_digest = reader.digest()?;
            member_slots.push(GroupMemberSlotV1 {
                semantic_id,
                slot_ordinal,
                offset_mm,
                protected,
                individual_decision_digest,
            });
        }
        if !reader.is_empty() {
            return Err(GroupRepresentationErrorV1::TrailingBytes);
        }
        let value = Self {
            version,
            generation,
            simulation_tick,
            frame_digest,
            individual_plan_root,
            group_id,
            kind,
            source_state_digest,
            source_provenance,
            source_capability_digest,
            member_ids,
            leader_id,
            representative_id,
            protected_member_ids,
            formation,
            member_slots,
            bounds_min_mm,
            bounds_max_mm,
            centroid_mm,
            distance_mm,
            tier,
            transition,
            accepted_tick,
            transition_phase,
            aggregate_digest,
        };
        if value.canonical_bytes()? != bytes {
            return Err(GroupRepresentationErrorV1::Malformed);
        }
        Ok(value)
    }

    pub fn digest(&self) -> Result<GroupDigestV1, GroupRepresentationErrorV1> {
        Ok(Sha256::digest(self.canonical_bytes()?).into())
    }
}

fn membership_owner<'a>(
    groups: impl Iterator<Item = (GroupDigestV1, &'a [GroupDigestV1])>,
) -> Result<BTreeMap<GroupDigestV1, GroupDigestV1>, GroupRepresentationErrorV1> {
    let mut owner = BTreeMap::new();
    for (group, members) in groups {
        for member in members {
            if owner.insert(*member, group).is_some() {
                return Err(GroupRepresentationErrorV1::DuplicateMembership(*member));
            }
        }
    }
    Ok(owner)
}

fn choose_tier(
    distance_mm: u32,
    protected: bool,
    _members: &[IndividualTierDecisionV1],
    prior: Option<&GroupPriorStateV1>,
    tick: u64,
    policy: GroupPolicyV1,
) -> GroupRepresentationTierV1 {
    if protected {
        return GroupRepresentationTierV1::IndividualNear;
    }
    if let Some(prior) = prior
        && tick.saturating_sub(prior.accepted_tick) < policy.minimum_residence_ticks
    {
        return prior.tier;
    }
    match prior.map(|state| state.tier) {
        Some(GroupRepresentationTierV1::AggregateFar)
            if distance_mm >= policy.aggregate_exit_mm =>
        {
            GroupRepresentationTierV1::AggregateFar
        },
        Some(GroupRepresentationTierV1::FormationMiddle)
            if distance_mm >= policy.formation_exit_mm
                && distance_mm < policy.aggregate_enter_mm =>
        {
            GroupRepresentationTierV1::FormationMiddle
        },
        _ if distance_mm >= policy.aggregate_enter_mm => GroupRepresentationTierV1::AggregateFar,
        _ if distance_mm >= policy.formation_enter_mm => GroupRepresentationTierV1::FormationMiddle,
        _ => GroupRepresentationTierV1::IndividualNear,
    }
}

fn classify_transition(
    group_id: GroupDigestV1,
    members: &[GroupDigestV1],
    leader: GroupDigestV1,
    formation: FormationKindV1,
    prior: &BTreeMap<GroupDigestV1, &GroupPriorStateV1>,
    prior_owner: &BTreeMap<GroupDigestV1, GroupDigestV1>,
    current_owner: &BTreeMap<GroupDigestV1, GroupDigestV1>,
) -> GroupTransitionV1 {
    if let Some(previous) = prior.get(&group_id) {
        if !members.contains(&previous.leader_id) && leader != previous.leader_id {
            return GroupTransitionV1::LeaderLoss;
        }
        if formation != previous.formation {
            return GroupTransitionV1::FormationChange;
        }
        let prior_set = previous.member_ids.iter().copied().collect::<BTreeSet<_>>();
        let current_set = members.iter().copied().collect::<BTreeSet<_>>();
        if current_set.len() > prior_set.len() && prior_set.is_subset(&current_set) {
            return GroupTransitionV1::Join;
        }
        if current_set.len() < prior_set.len() && current_set.is_subset(&prior_set) {
            return GroupTransitionV1::Leave;
        }
        if leader != previous.leader_id {
            return GroupTransitionV1::SelectedMemberPromotion;
        }
        return GroupTransitionV1::Stable;
    }
    let ancestors = members
        .iter()
        .filter_map(|member| prior_owner.get(member))
        .copied()
        .collect::<BTreeSet<_>>();
    if ancestors.len() > 1 {
        return GroupTransitionV1::Merge;
    }
    if let Some(ancestor) = ancestors.first() {
        let descendants = prior
            .get(ancestor)
            .into_iter()
            .flat_map(|state| state.member_ids.iter())
            .filter_map(|member| current_owner.get(member))
            .copied()
            .collect::<BTreeSet<_>>();
        if descendants.len() > 1 {
            return GroupTransitionV1::Split;
        }
    }
    GroupTransitionV1::Stable
}

fn formation_slots(
    members: &[GroupDigestV1],
    protected: &[GroupDigestV1],
    formation: FormationKindV1,
    spacing_mm: u32,
    decisions: &BTreeMap<GroupDigestV1, &IndividualTierDecisionV1>,
) -> Result<Vec<GroupMemberSlotV1>, GroupRepresentationErrorV1> {
    let spacing =
        i32::try_from(spacing_mm).map_err(|_| GroupRepresentationErrorV1::CoordinateOverflow)?;
    let count =
        i32::try_from(members.len()).map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?;
    members
        .iter()
        .enumerate()
        .map(|(ordinal, member)| {
            let index =
                i32::try_from(ordinal).map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?;
            let offset = match formation {
                FormationKindV1::Line => {
                    let centered = index
                        .checked_mul(2)
                        .and_then(|value| value.checked_sub(count - 1))
                        .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?;
                    [
                        centered
                            .checked_mul(spacing)
                            .and_then(|value| value.checked_div(2))
                            .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?,
                        0,
                        0,
                    ]
                },
                FormationKindV1::Column | FormationKindV1::Procession => [
                    0,
                    index
                        .checked_mul(spacing)
                        .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?,
                    0,
                ],
                FormationKindV1::Wedge => {
                    if index == 0 {
                        [0, 0, 0]
                    } else {
                        let row = (index + 1) / 2;
                        let side = if index % 2 == 1 { -1 } else { 1 };
                        [
                            side * row
                                .checked_mul(spacing)
                                .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?,
                            row.checked_mul(spacing)
                                .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?,
                            0,
                        ]
                    }
                },
                FormationKindV1::Grid => {
                    let width = integer_sqrt_ceil(count.max(1));
                    [
                        (index % width)
                            .checked_mul(spacing)
                            .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?,
                        (index / width)
                            .checked_mul(spacing)
                            .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?,
                        0,
                    ]
                },
            };
            let decision = decisions.get(member).ok_or(
                GroupRepresentationErrorV1::MissingIndividualDecision(*member),
            )?;
            Ok(GroupMemberSlotV1 {
                semantic_id: *member,
                slot_ordinal: u32::try_from(ordinal)
                    .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?,
                offset_mm: offset,
                protected: protected.binary_search(member).is_ok(),
                individual_decision_digest: decision.digest(),
            })
        })
        .collect()
}

fn aggregate_digest(
    group_id: GroupDigestV1,
    state_digest: GroupDigestV1,
    members: &[GroupDigestV1],
    slots: &[GroupMemberSlotV1],
    bounds_min: [i64; 3],
    bounds_max: [i64; 3],
    centroid: [i64; 3],
    tier: GroupRepresentationTierV1,
) -> GroupDigestV1 {
    let mut hasher = Sha256::new();
    hasher.update(b"bastion/r1d/group-aggregate-v1");
    hasher.update(group_id);
    hasher.update(state_digest);
    hasher.update([tier as u8]);
    for values in [bounds_min, bounds_max, centroid] {
        for value in values {
            hasher.update(value.to_le_bytes());
        }
    }
    for (member, slot) in members.iter().zip(slots) {
        hasher.update(member);
        hasher.update(slot.slot_ordinal.to_le_bytes());
        for offset in slot.offset_mm {
            hasher.update(offset.to_le_bytes());
        }
        hasher.update([u8::from(slot.protected)]);
        hasher.update(slot.individual_decision_digest);
    }
    hasher.finalize().into()
}

fn plan_root(
    generation: u64,
    tick: u64,
    frame_digest: GroupDigestV1,
    individual_plan_root: GroupDigestV1,
    groups: &[GroupRepresentationV1],
) -> Result<GroupDigestV1, GroupRepresentationErrorV1> {
    let mut hasher = Sha256::new();
    hasher.update(b"bastion/r1d/group-plan-v1");
    hasher.update(generation.to_le_bytes());
    hasher.update(tick.to_le_bytes());
    hasher.update(frame_digest);
    hasher.update(individual_plan_root);
    for group in groups {
        let bytes = group.canonical_bytes()?;
        hasher.update(
            u64::try_from(bytes.len())
                .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?
                .to_le_bytes(),
        );
        hasher.update(bytes);
    }
    Ok(hasher.finalize().into())
}

fn count_tier(
    groups: &[GroupRepresentationV1],
    tier: GroupRepresentationTierV1,
) -> Result<u32, GroupRepresentationErrorV1> {
    u32::try_from(groups.iter().filter(|group| group.tier == tier).count())
        .map_err(|_| GroupRepresentationErrorV1::LengthOverflow)
}

fn distance_mm(left: [i64; 3], right: [i64; 3]) -> Result<u32, GroupRepresentationErrorV1> {
    let mut square = 0_u128;
    for axis in 0..3 {
        let delta = i128::from(left[axis])
            .checked_sub(i128::from(right[axis]))
            .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?;
        let magnitude = delta.unsigned_abs();
        square = square
            .checked_add(
                magnitude
                    .checked_mul(magnitude)
                    .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?,
            )
            .ok_or(GroupRepresentationErrorV1::CoordinateOverflow)?;
    }
    u32::try_from(integer_sqrt(square)).map_err(|_| GroupRepresentationErrorV1::CoordinateOverflow)
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

fn integer_sqrt_ceil(value: i32) -> i32 {
    let mut root = 1_i32;
    while root.saturating_mul(root) < value {
        root += 1;
    }
    root
}

fn transition_phase(
    tick: u64,
    accepted_tick: u64,
    transition_ticks: u16,
) -> Result<u16, GroupRepresentationErrorV1> {
    let elapsed = tick.saturating_sub(accepted_tick);
    let phase = elapsed
        .saturating_mul(u64::from(u16::MAX))
        .checked_div(u64::from(transition_ticks))
        .ok_or(GroupRepresentationErrorV1::LengthOverflow)?
        .min(u64::from(u16::MAX));
    u16::try_from(phase).map_err(|_| GroupRepresentationErrorV1::LengthOverflow)
}

fn put_count(output: &mut Vec<u8>, count: usize) -> Result<(), GroupRepresentationErrorV1> {
    put_u32(
        output,
        u32::try_from(count).map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?,
    );
    Ok(())
}

fn put_u16(output: &mut Vec<u8>, value: u16) { output.extend_from_slice(&value.to_le_bytes()); }
fn put_u32(output: &mut Vec<u8>, value: u32) { output.extend_from_slice(&value.to_le_bytes()); }
fn put_u64(output: &mut Vec<u8>, value: u64) { output.extend_from_slice(&value.to_le_bytes()); }
fn put_i64(output: &mut Vec<u8>, value: i64) { output.extend_from_slice(&value.to_le_bytes()); }

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self { Self { bytes, cursor: 0 } }

    fn take(&mut self, length: usize) -> Result<&'a [u8], GroupRepresentationErrorV1> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(GroupRepresentationErrorV1::Malformed)?;
        let result = self
            .bytes
            .get(self.cursor..end)
            .ok_or(GroupRepresentationErrorV1::Malformed)?;
        self.cursor = end;
        Ok(result)
    }

    fn u8(&mut self) -> Result<u8, GroupRepresentationErrorV1> {
        self.take(1)?
            .first()
            .copied()
            .ok_or(GroupRepresentationErrorV1::Malformed)
    }

    fn u16(&mut self) -> Result<u16, GroupRepresentationErrorV1> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| GroupRepresentationErrorV1::Malformed)?,
        ))
    }

    fn u32(&mut self) -> Result<u32, GroupRepresentationErrorV1> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| GroupRepresentationErrorV1::Malformed)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, GroupRepresentationErrorV1> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| GroupRepresentationErrorV1::Malformed)?,
        ))
    }

    fn i64(&mut self) -> Result<i64, GroupRepresentationErrorV1> {
        Ok(i64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| GroupRepresentationErrorV1::Malformed)?,
        ))
    }

    fn digest(&mut self) -> Result<GroupDigestV1, GroupRepresentationErrorV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| GroupRepresentationErrorV1::Malformed)
    }

    fn count(&mut self, maximum: usize) -> Result<usize, GroupRepresentationErrorV1> {
        let count =
            usize::try_from(self.u32()?).map_err(|_| GroupRepresentationErrorV1::LengthOverflow)?;
        if count > maximum {
            return Err(GroupRepresentationErrorV1::InvalidCount);
        }
        Ok(count)
    }

    fn i64x3(&mut self) -> Result<[i64; 3], GroupRepresentationErrorV1> {
        Ok([self.i64()?, self.i64()?, self.i64()?])
    }

    fn i32x3(&mut self) -> Result<[i32; 3], GroupRepresentationErrorV1> {
        let bytes = self.take(12)?;
        Ok([
            i32::from_le_bytes(
                bytes[0..4]
                    .try_into()
                    .map_err(|_| GroupRepresentationErrorV1::Malformed)?,
            ),
            i32::from_le_bytes(
                bytes[4..8]
                    .try_into()
                    .map_err(|_| GroupRepresentationErrorV1::Malformed)?,
            ),
            i32::from_le_bytes(
                bytes[8..12]
                    .try_into()
                    .map_err(|_| GroupRepresentationErrorV1::Malformed)?,
            ),
        ])
    }

    fn is_empty(&self) -> bool { self.cursor == self.bytes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        individual_tier::{
            IndividualTierBudgetV1, IndividualTierInputV1, IndividualTierPolicyV1,
            TierContentAvailabilityV1,
        },
        presentation::{
            PresentationEntityV1, PresentationEnvironmentV1, PresentationFrameDraftV1,
            PresentationGenerationV1, PresentationGroupV1, PresentationVisualPolicyV1,
        },
    };

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn fixture(
        groups: Vec<(u8, Vec<u8>)>,
        tick: u64,
    ) -> (
        PresentationFrameV1,
        IndividualTierPlanV1,
        Vec<GroupDeclarationV1>,
    ) {
        let mut entities = Vec::new();
        let mut presentation_groups = Vec::new();
        let mut declarations = Vec::new();
        for (group_id, members) in groups {
            for member in &members {
                entities.push(PresentationEntityV1 {
                    semantic_id: digest(*member),
                    figure_resource: digest(200),
                    group_id: Some(digest(group_id)),
                    position_mm: [
                        i64::from(group_id) * 20_000 + i64::from(*member) * 2_000,
                        i64::from(*member) * 1_000,
                        0,
                    ],
                    orientation_q30: [0, 0, 0, 1 << 30],
                    scale_milli: 1_000,
                    state_tag: 1,
                    state_digest: digest(member.saturating_add(80)),
                });
            }
            let mut member_ids = members
                .iter()
                .map(|member| digest(*member))
                .collect::<Vec<_>>();
            member_ids.sort_unstable();
            presentation_groups.push(PresentationGroupV1 {
                semantic_id: digest(group_id),
                kind_tag: GroupKindV1::Formation as u16,
                member_ids: member_ids.clone(),
                state_digest: digest(group_id.saturating_add(100)),
            });
            declarations.push(GroupDeclarationV1 {
                group_id: digest(group_id),
                leader_id: member_ids[0],
                protected_member_ids: if group_id == 10 {
                    vec![member_ids[0]]
                } else {
                    Vec::new()
                },
                formation: if group_id == 10 {
                    FormationKindV1::Wedge
                } else {
                    FormationKindV1::Grid
                },
                source_provenance: GroupSourceProvenanceV1::DeclaredPacketFixture,
                source_capability_digest: digest(250),
            });
        }
        entities.sort_unstable_by_key(|entity| entity.semantic_id);
        presentation_groups.sort_unstable_by_key(|group| group.semantic_id);
        declarations.sort_unstable_by_key(|group| group.group_id);
        let frame = PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: 1,
                simulation_tick: tick,
                coherent_snapshot_root: digest(251),
            },
            entities,
            groups: presentation_groups,
            events: Vec::new(),
            environment: PresentationEnvironmentV1 {
                terrain_root: digest(252),
                environment_digest: digest(253),
                cloud_milli: 0,
                rain_milli: 0,
                wind_mm_s: [0, 0],
                daylight_milli: 500,
            },
            visual_policy: PresentationVisualPolicyV1 {
                policy_digest: digest(254),
                terrain_view_distance: 16,
                entity_view_distance: 16,
                figure_lod_distance: 350,
                sprite_distance: 250,
                particles_enabled: true,
                weapon_trails_enabled: true,
                flashing_lights_enabled: false,
            },
            renderer_required_resources: vec![digest(200)],
            complete: true,
        }
        .seal()
        .unwrap();
        let inputs = frame
            .entities()
            .iter()
            .map(|entity| IndividualTierInputV1 {
                semantic_entity: entity.semantic_id,
                importance: if entity.semantic_id == digest(1) {
                    u16::MAX
                } else {
                    1_000
                },
                screen_size_milli: 100,
                distance_mm: u32::try_from(entity.position_mm[0]).unwrap_or(u32::MAX),
                availability: TierContentAvailabilityV1 {
                    lod: true,
                    impostor: false,
                    shadow_proxy: false,
                },
                prior: None,
            })
            .collect();
        let individual = IndividualTierPlanV1::build(
            1,
            frame.frame_digest(),
            tick,
            IndividualTierPolicyV1::PRODUCTION,
            IndividualTierBudgetV1::PRODUCTION,
            inputs,
        )
        .unwrap();
        (frame, individual, declarations)
    }

    fn build(
        groups: Vec<(u8, Vec<u8>)>,
    ) -> Result<GroupRepresentationPlanV1, GroupRepresentationErrorV1> {
        let (frame, individual, declarations) = fixture(groups, 300);
        GroupRepresentationPlanV1::build(
            &frame,
            &individual,
            [0, 0, 0],
            GroupPolicyV1::PRODUCTION,
            GroupBudgetV1::PRODUCTION,
            declarations,
            &[],
        )
    }

    #[test]
    fn input_and_membership_permutation_are_canonical() {
        let left = build(vec![(10, vec![3, 1, 2]), (20, vec![6, 4, 5])]).unwrap();
        let right = build(vec![(20, vec![5, 6, 4]), (10, vec![2, 3, 1])]).unwrap();
        assert_eq!(left.plan_root, right.plan_root);
        assert_eq!(left.groups, right.groups);
    }

    #[test]
    fn protected_member_stays_individual_and_inspectable() {
        let plan = build(vec![(10, vec![1, 2, 3]), (20, vec![4, 5, 6])]).unwrap();
        let group = plan.group_for_member(digest(1)).unwrap();
        assert_eq!(group.tier, GroupRepresentationTierV1::IndividualNear);
        assert_eq!(group.representative_id, digest(1));
        assert!(group.member_slots[0].protected);
    }

    #[test]
    fn distance_bands_select_middle_and_far_without_erasing_members() {
        let plan = build(vec![
            (10, vec![1, 2, 3]),
            (2, vec![4, 5, 6]),
            (5, vec![7, 8, 9]),
        ])
        .unwrap();
        assert_eq!(
            plan.groups
                .iter()
                .find(|group| group.group_id == digest(10))
                .unwrap()
                .tier,
            GroupRepresentationTierV1::IndividualNear
        );
        assert_eq!(
            plan.groups
                .iter()
                .find(|group| group.group_id == digest(2))
                .unwrap()
                .tier,
            GroupRepresentationTierV1::FormationMiddle
        );
        assert_eq!(
            plan.groups
                .iter()
                .find(|group| group.group_id == digest(5))
                .unwrap()
                .tier,
            GroupRepresentationTierV1::AggregateFar
        );
        assert_eq!(plan.member_count, 9);
    }

    #[test]
    fn formation_slots_are_unique_and_every_member_is_preserved() {
        let plan = build(vec![(10, vec![1, 2, 3, 4]), (20, vec![5, 6, 7, 8])]).unwrap();
        let all = plan
            .groups
            .iter()
            .flat_map(|group| group.member_slots.iter().map(|slot| slot.semantic_id))
            .collect::<BTreeSet<_>>();
        assert_eq!(all.len(), 8);
        for group in &plan.groups {
            assert_eq!(
                group
                    .member_slots
                    .iter()
                    .map(|slot| slot.offset_mm)
                    .collect::<BTreeSet<_>>()
                    .len(),
                group.member_slots.len()
            );
        }
    }

    #[test]
    fn canonical_round_trip_and_trailing_bytes_fail_closed() {
        let plan = build(vec![(10, vec![1, 2, 3]), (20, vec![4, 5, 6])]).unwrap();
        let group = &plan.groups[0];
        let bytes = group.canonical_bytes().unwrap();
        assert_eq!(
            GroupRepresentationV1::decode_exact(&bytes),
            Ok(group.clone())
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            GroupRepresentationV1::decode_exact(&trailing),
            Err(GroupRepresentationErrorV1::TrailingBytes)
        );
    }

    #[test]
    fn every_key_field_changes_the_group_digest() {
        let plan = build(vec![(10, vec![1, 2, 3]), (20, vec![4, 5, 6])]).unwrap();
        let base = plan.groups[0].digest().unwrap();
        let mut cases = Vec::new();
        let mut value = plan.groups[0].clone();
        value.source_state_digest = digest(77);
        cases.push(value);
        let mut value = plan.groups[0].clone();
        value.formation = FormationKindV1::Column;
        cases.push(value);
        let mut value = plan.groups[0].clone();
        value.source_capability_digest = digest(78);
        cases.push(value);
        let mut value = plan.groups[0].clone();
        value.transition_phase = 9;
        cases.push(value);
        assert!(cases.iter().all(|value| value.digest().unwrap() != base));
    }

    #[test]
    fn duplicate_membership_missing_member_and_invalid_leader_reject() {
        assert_eq!(
            membership_owner(
                [
                    (digest(10), [digest(1), digest(2)].as_slice()),
                    (digest(20), [digest(2), digest(3)].as_slice()),
                ]
                .into_iter(),
            ),
            Err(GroupRepresentationErrorV1::DuplicateMembership(digest(2)))
        );
        let (frame, individual, mut declarations) = fixture(vec![(10, vec![1, 2])], 300);
        declarations[0].leader_id = digest(9);
        assert_eq!(
            GroupRepresentationPlanV1::build(
                &frame,
                &individual,
                [0, 0, 0],
                GroupPolicyV1::PRODUCTION,
                GroupBudgetV1::PRODUCTION,
                declarations,
                &[],
            ),
            Err(GroupRepresentationErrorV1::InvalidLeader(digest(9)))
        );
    }

    #[test]
    fn budget_exhaustion_is_typed_and_rolls_back_to_caller() {
        let (frame, individual, declarations) =
            fixture(vec![(10, vec![1, 2]), (20, vec![3, 4])], 300);
        assert_eq!(
            GroupRepresentationPlanV1::build(
                &frame,
                &individual,
                [0, 0, 0],
                GroupPolicyV1::PRODUCTION,
                GroupBudgetV1 {
                    max_groups: 2,
                    max_members: 3,
                    max_individual_groups: 2,
                    max_formation_groups: 2,
                    max_aggregate_groups: 2,
                },
                declarations,
                &[],
            ),
            Err(GroupRepresentationErrorV1::BudgetExhausted)
        );
    }

    #[test]
    fn join_leave_leader_and_formation_changes_are_stable() {
        let (frame, individual, declarations) = fixture(vec![(10, vec![1, 2, 3])], 330);
        let base = GroupPriorStateV1 {
            group_id: digest(10),
            leader_id: digest(1),
            member_ids: vec![digest(1), digest(2)],
            formation: FormationKindV1::Wedge,
            tier: GroupRepresentationTierV1::IndividualNear,
            accepted_tick: 300,
        };
        let joined = GroupRepresentationPlanV1::build(
            &frame,
            &individual,
            [0, 0, 0],
            GroupPolicyV1::PRODUCTION,
            GroupBudgetV1::PRODUCTION,
            declarations.clone(),
            std::slice::from_ref(&base),
        )
        .unwrap();
        assert_eq!(joined.groups[0].transition, GroupTransitionV1::Join);

        let mut changed = declarations;
        changed[0].formation = FormationKindV1::Grid;
        let changed = GroupRepresentationPlanV1::build(
            &frame,
            &individual,
            [0, 0, 0],
            GroupPolicyV1::PRODUCTION,
            GroupBudgetV1::PRODUCTION,
            changed,
            std::slice::from_ref(&base),
        )
        .unwrap();
        assert_eq!(
            changed.groups[0].transition,
            GroupTransitionV1::FormationChange
        );
    }

    #[test]
    fn leader_loss_and_selected_promotion_are_explicit() {
        let prior = GroupPriorStateV1 {
            group_id: digest(10),
            leader_id: digest(1),
            member_ids: vec![digest(1), digest(2), digest(3)],
            formation: FormationKindV1::Wedge,
            tier: GroupRepresentationTierV1::IndividualNear,
            accepted_tick: 300,
        };
        let (frame, individual, mut declarations) = fixture(vec![(10, vec![1, 2, 3])], 330);
        declarations[0].leader_id = digest(2);
        let promoted = GroupRepresentationPlanV1::build(
            &frame,
            &individual,
            [0, 0, 0],
            GroupPolicyV1::PRODUCTION,
            GroupBudgetV1::PRODUCTION,
            declarations,
            std::slice::from_ref(&prior),
        )
        .unwrap();
        assert_eq!(
            promoted.groups[0].transition,
            GroupTransitionV1::SelectedMemberPromotion
        );

        let (frame, individual, mut declarations) = fixture(vec![(10, vec![2, 3])], 330);
        declarations[0].leader_id = digest(2);
        declarations[0].protected_member_ids = vec![digest(2)];
        let replaced = GroupRepresentationPlanV1::build(
            &frame,
            &individual,
            [0, 0, 0],
            GroupPolicyV1::PRODUCTION,
            GroupBudgetV1::PRODUCTION,
            declarations,
            std::slice::from_ref(&prior),
        )
        .unwrap();
        assert_eq!(replaced.groups[0].transition, GroupTransitionV1::LeaderLoss);
        assert_eq!(replaced.groups[0].leader_id, digest(2));
    }

    #[test]
    fn split_and_merge_continuity_are_derived_from_explicit_membership() {
        let (split_frame, split_individual, split_declarations) =
            fixture(vec![(10, vec![1, 2]), (20, vec![3, 4])], 330);
        let prior = GroupPriorStateV1 {
            group_id: digest(30),
            leader_id: digest(1),
            member_ids: vec![digest(1), digest(2), digest(3), digest(4)],
            formation: FormationKindV1::Line,
            tier: GroupRepresentationTierV1::FormationMiddle,
            accepted_tick: 300,
        };
        let split = GroupRepresentationPlanV1::build(
            &split_frame,
            &split_individual,
            [0, 0, 0],
            GroupPolicyV1::PRODUCTION,
            GroupBudgetV1::PRODUCTION,
            split_declarations,
            std::slice::from_ref(&prior),
        )
        .unwrap();
        assert!(
            split
                .groups
                .iter()
                .all(|group| group.transition == GroupTransitionV1::Split)
        );

        let (merge_frame, merge_individual, merge_declarations) =
            fixture(vec![(30, vec![1, 2, 3, 4])], 330);
        let prior = [
            GroupPriorStateV1 {
                group_id: digest(10),
                leader_id: digest(1),
                member_ids: vec![digest(1), digest(2)],
                formation: FormationKindV1::Line,
                tier: GroupRepresentationTierV1::FormationMiddle,
                accepted_tick: 300,
            },
            GroupPriorStateV1 {
                group_id: digest(20),
                leader_id: digest(3),
                member_ids: vec![digest(3), digest(4)],
                formation: FormationKindV1::Grid,
                tier: GroupRepresentationTierV1::FormationMiddle,
                accepted_tick: 300,
            },
        ];
        let merge = GroupRepresentationPlanV1::build(
            &merge_frame,
            &merge_individual,
            [0, 0, 0],
            GroupPolicyV1::PRODUCTION,
            GroupBudgetV1::PRODUCTION,
            merge_declarations,
            &prior,
        )
        .unwrap();
        assert_eq!(merge.groups[0].transition, GroupTransitionV1::Merge);
    }

    #[test]
    fn stale_individual_plan_rejects_and_prior_plan_remains_immutable() {
        let (frame, mut individual, declarations) = fixture(vec![(10, vec![1, 2])], 300);
        let prior = build(vec![(10, vec![1, 2])]).unwrap();
        let prior_root = prior.plan_root;
        individual.generation = 2;
        assert_eq!(
            GroupRepresentationPlanV1::build(
                &frame,
                &individual,
                [0, 0, 0],
                GroupPolicyV1::PRODUCTION,
                GroupBudgetV1::PRODUCTION,
                declarations,
                &[],
            ),
            Err(GroupRepresentationErrorV1::StaleGeneration)
        );
        assert_eq!(prior.plan_root, prior_root);
    }
}
