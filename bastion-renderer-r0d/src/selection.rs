//! Canonical CPU render selection using checked integer policy only.

use crate::{
    DomainHashErrorV1,
    camera::{CameraFrustumV1, PlaneQ40V1},
    domain_hash_v1,
    extract::MAX_ABS_POSITION_MM_V1,
};

pub const MAX_LOD_TRANSITIONS_V1: usize = 32;
pub const MAX_ANIMATION_SAMPLES_V1: u64 = 1_048_576;
pub const MAX_ANIMATION_CLIP_TICKS_V1: u64 = 1_048_576;
pub const MAX_VISIBLE_RECORDS_V1: usize = 100_000;
pub const MAX_DRAW_RECORDS_V1: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionErrorV1 {
    InvalidAabb,
    AabbOutOfRange,
    ArithmeticOverflow,
    ZeroDistance,
    ZeroProjectionScale,
    TooManyLodTransitions { actual: usize, maximum: usize },
    InvalidLodThresholdOrder,
    InvalidPriorLod(u32),
    InvalidAnimationRange,
    InvalidVisibleAlias(u32),
    TooManyVisibleRecords { actual: usize, maximum: usize },
    TooManyDrawRecords { actual: usize, maximum: usize },
    DuplicateVisibleDigest([u8; 32]),
    DuplicateDrawKey,
    HashFailure(DomainHashErrorV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AabbMmV1 {
    min: [i64; 3],
    max: [i64; 3],
}

impl AabbMmV1 {
    pub fn new(min: [i64; 3], max: [i64; 3]) -> Result<Self, SelectionErrorV1> {
        for axis in 0..3 {
            if min[axis] > max[axis] {
                return Err(SelectionErrorV1::InvalidAabb);
            }
            if min[axis] < -MAX_ABS_POSITION_MM_V1 || max[axis] > MAX_ABS_POSITION_MM_V1 {
                return Err(SelectionErrorV1::AabbOutOfRange);
            }
        }
        Ok(Self { min, max })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrustumClassV1 {
    Outside,
    Intersect,
    Inside,
}

pub fn classify_frustum(
    frustum: &CameraFrustumV1,
    aabb: AabbMmV1,
) -> Result<FrustumClassV1, SelectionErrorV1> {
    let mut all_inside = true;
    for plane in frustum.planes() {
        match classify_plane(*plane, aabb)? {
            FrustumClassV1::Outside => return Ok(FrustumClassV1::Outside),
            FrustumClassV1::Intersect => all_inside = false,
            FrustumClassV1::Inside => {},
        }
    }
    Ok(if all_inside {
        FrustumClassV1::Inside
    } else {
        FrustumClassV1::Intersect
    })
}

fn classify_plane(plane: PlaneQ40V1, aabb: AabbMmV1) -> Result<FrustumClassV1, SelectionErrorV1> {
    let normal = plane.normal();
    let mut positive = [0_i64; 3];
    let mut negative = [0_i64; 3];
    for axis in 0..3 {
        if normal[axis] >= 0 {
            positive[axis] = aabb.max[axis];
            negative[axis] = aabb.min[axis];
        } else {
            positive[axis] = aabb.min[axis];
            negative[axis] = aabb.max[axis];
        }
    }
    if signed_distance(plane, positive)? < 0 {
        Ok(FrustumClassV1::Outside)
    } else if signed_distance(plane, negative)? >= 0 {
        Ok(FrustumClassV1::Inside)
    } else {
        Ok(FrustumClassV1::Intersect)
    }
}

fn signed_distance(plane: PlaneQ40V1, point: [i64; 3]) -> Result<i128, SelectionErrorV1> {
    plane
        .normal()
        .into_iter()
        .zip(point)
        .try_fold(i128::from(plane.distance()), |sum, (normal, point)| {
            i128::from(normal)
                .checked_mul(i128::from(point))
                .and_then(|term| sum.checked_add(term))
        })
        .ok_or(SelectionErrorV1::ArithmeticOverflow)
}

pub fn screen_space_error_q16(
    geometric_error_micrometers: u64,
    projection_scale_q16: u64,
    distance_micrometers: u64,
) -> Result<u64, SelectionErrorV1> {
    if distance_micrometers == 0 {
        return Err(SelectionErrorV1::ZeroDistance);
    }
    if projection_scale_q16 == 0 {
        return Err(SelectionErrorV1::ZeroProjectionScale);
    }
    let numerator = u128::from(geometric_error_micrometers)
        .checked_mul(u128::from(projection_scale_q16))
        .ok_or(SelectionErrorV1::ArithmeticOverflow)?;
    let denominator = u128::from(distance_micrometers);
    let rounded = numerator
        .checked_add(
            denominator
                .checked_sub(1)
                .ok_or(SelectionErrorV1::ArithmeticOverflow)?,
        )
        .ok_or(SelectionErrorV1::ArithmeticOverflow)?
        / denominator;
    u64::try_from(rounded).map_err(|_| SelectionErrorV1::ArithmeticOverflow)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LodTransitionV1 {
    pub promote_above_q16: u64,
    pub demote_below_q16: u64,
    pub minimum_residence_ticks: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LodStateV1 {
    pub tier: u32,
    pub residence_ticks: u64,
}

pub fn select_lod(
    sse_q16: u64,
    transitions: &[LodTransitionV1],
    prior: LodStateV1,
) -> Result<LodStateV1, SelectionErrorV1> {
    validate_lod_transitions(transitions)?;
    let tier =
        usize::try_from(prior.tier).map_err(|_| SelectionErrorV1::InvalidPriorLod(prior.tier))?;
    if tier > transitions.len() {
        return Err(SelectionErrorV1::InvalidPriorLod(prior.tier));
    }
    if tier < transitions.len() {
        let transition = transitions[tier];
        if sse_q16 > transition.promote_above_q16
            && prior.residence_ticks >= transition.minimum_residence_ticks
        {
            return Ok(LodStateV1 {
                tier: prior
                    .tier
                    .checked_add(1)
                    .ok_or(SelectionErrorV1::ArithmeticOverflow)?,
                residence_ticks: 0,
            });
        }
    }
    if tier > 0 {
        let transition = transitions[tier - 1];
        if sse_q16 < transition.demote_below_q16
            && prior.residence_ticks >= transition.minimum_residence_ticks
        {
            return Ok(LodStateV1 {
                tier: prior
                    .tier
                    .checked_sub(1)
                    .ok_or(SelectionErrorV1::ArithmeticOverflow)?,
                residence_ticks: 0,
            });
        }
    }
    Ok(LodStateV1 {
        tier: prior.tier,
        residence_ticks: prior
            .residence_ticks
            .checked_add(1)
            .ok_or(SelectionErrorV1::ArithmeticOverflow)?,
    })
}

fn validate_lod_transitions(transitions: &[LodTransitionV1]) -> Result<(), SelectionErrorV1> {
    if transitions.len() > MAX_LOD_TRANSITIONS_V1 {
        return Err(SelectionErrorV1::TooManyLodTransitions {
            actual: transitions.len(),
            maximum: MAX_LOD_TRANSITIONS_V1,
        });
    }
    for transition in transitions {
        if transition.demote_below_q16 >= transition.promote_above_q16 {
            return Err(SelectionErrorV1::InvalidLodThresholdOrder);
        }
    }
    for pair in transitions.windows(2) {
        if pair[1].promote_above_q16 <= pair[0].promote_above_q16
            || pair[1].demote_below_q16 <= pair[0].demote_below_q16
        {
            return Err(SelectionErrorV1::InvalidLodThresholdOrder);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationSamplingV1 {
    pub clip_length_ticks: u64,
    pub sample_count: u64,
    pub cadence_ticks: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationSampleV1 {
    pub sample_index: u64,
    pub update_due: bool,
}

pub fn animation_sample(
    entity_digest: [u8; 32],
    clip_digest: [u8; 32],
    simulation_tick: u64,
    sampling: AnimationSamplingV1,
) -> Result<AnimationSampleV1, SelectionErrorV1> {
    if sampling.clip_length_ticks == 0
        || sampling.clip_length_ticks > MAX_ANIMATION_CLIP_TICKS_V1
        || sampling.sample_count == 0
        || sampling.sample_count > MAX_ANIMATION_SAMPLES_V1
        || sampling.cadence_ticks == 0
        || sampling.cadence_ticks > sampling.clip_length_ticks
    {
        return Err(SelectionErrorV1::InvalidAnimationRange);
    }
    let mut payload = [0_u8; 64];
    payload[..32].copy_from_slice(&entity_digest);
    payload[32..].copy_from_slice(&clip_digest);
    let phase_digest = domain_hash_v1("bastion/r0d/animation-phase", 1, 0, &payload)
        .map_err(SelectionErrorV1::HashFailure)?;
    let phase = u64::from_le_bytes(
        phase_digest[..8]
            .try_into()
            .map_err(|_| SelectionErrorV1::ArithmeticOverflow)?,
    );
    let local_tick_wide = u128::from(simulation_tick)
        .checked_add(u128::from(phase))
        .ok_or(SelectionErrorV1::ArithmeticOverflow)?
        % u128::from(sampling.clip_length_ticks);
    let local_tick =
        u64::try_from(local_tick_wide).map_err(|_| SelectionErrorV1::ArithmeticOverflow)?;
    let index = u128::from(local_tick)
        .checked_mul(u128::from(sampling.sample_count))
        .ok_or(SelectionErrorV1::ArithmeticOverflow)?
        / u128::from(sampling.clip_length_ticks);
    let sample_index = u64::try_from(index).map_err(|_| SelectionErrorV1::ArithmeticOverflow)?;
    Ok(AnimationSampleV1 {
        sample_index,
        update_due: phase % sampling.cadence_ticks == simulation_tick % sampling.cadence_ticks,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VisibleRecordV1 {
    pub semantic_digest: [u8; 32],
    pub compact_alias: u32,
}

pub fn canonicalize_visible(
    mut records: Vec<VisibleRecordV1>,
) -> Result<Vec<VisibleRecordV1>, SelectionErrorV1> {
    if records.len() > MAX_VISIBLE_RECORDS_V1 {
        return Err(SelectionErrorV1::TooManyVisibleRecords {
            actual: records.len(),
            maximum: MAX_VISIBLE_RECORDS_V1,
        });
    }
    if let Some(alias) = records
        .iter()
        .map(|record| record.compact_alias)
        .find(|alias| *alias == 0)
    {
        return Err(SelectionErrorV1::InvalidVisibleAlias(alias));
    }
    records.sort_unstable_by(|left, right| {
        left.semantic_digest
            .cmp(&right.semantic_digest)
            .then(left.compact_alias.cmp(&right.compact_alias))
    });
    if let Some(duplicate) = records
        .windows(2)
        .find(|pair| pair[0].semantic_digest == pair[1].semantic_digest)
    {
        return Err(SelectionErrorV1::DuplicateVisibleDigest(
            duplicate[0].semantic_digest,
        ));
    }
    Ok(records)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DrawKeyV1 {
    pub pass_tag: u16,
    pub pipeline_tag: u16,
    pub package_digest: [u8; 32],
    pub material_tag: u16,
    pub lod: u32,
    pub semantic_digest: [u8; 32],
    pub submesh_tag: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrawRecordV1 {
    pub key: DrawKeyV1,
    pub arguments: DrawIndexedIndirectArgsV1,
}

pub fn canonicalize_draws(
    mut records: Vec<DrawRecordV1>,
) -> Result<Vec<DrawRecordV1>, SelectionErrorV1> {
    if records.len() > MAX_DRAW_RECORDS_V1 {
        return Err(SelectionErrorV1::TooManyDrawRecords {
            actual: records.len(),
            maximum: MAX_DRAW_RECORDS_V1,
        });
    }
    records.sort_unstable_by_key(|record| record.key);
    if records.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err(SelectionErrorV1::DuplicateDrawKey);
    }
    Ok(records)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrawIndirectArgsV1 {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

impl DrawIndirectArgsV1 {
    #[must_use]
    pub fn encode_le(self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[0..4].copy_from_slice(&self.vertex_count.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.instance_count.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.first_vertex.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.first_instance.to_le_bytes());
        bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrawIndexedIndirectArgsV1 {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl DrawIndexedIndirectArgsV1 {
    #[must_use]
    pub fn encode_le(self) -> [u8; 20] {
        let mut bytes = [0_u8; 20];
        bytes[0..4].copy_from_slice(&self.index_count.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.instance_count.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.first_index.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.base_vertex.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.first_instance.to_le_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{camera::CameraErrorV1, hex_bytes};

    fn frustum(half_mm: i64) -> CameraFrustumV1 {
        let q = 1_i64 << 40;
        CameraFrustumV1::new([
            PlaneQ40V1::new([q, 0, 0], q * half_mm).unwrap(),
            PlaneQ40V1::new([-q, 0, 0], q * half_mm).unwrap(),
            PlaneQ40V1::new([0, q, 0], q * half_mm).unwrap(),
            PlaneQ40V1::new([0, -q, 0], q * half_mm).unwrap(),
            PlaneQ40V1::new([0, 0, q], q * half_mm).unwrap(),
            PlaneQ40V1::new([0, 0, -q], q * half_mm).unwrap(),
        ])
        .unwrap()
    }

    #[test]
    fn conservative_frustum_vectors_are_frozen() {
        let value = frustum(1_000);
        assert_eq!(
            classify_frustum(&value, AabbMmV1::new([-100; 3], [100; 3]).unwrap()),
            Ok(FrustumClassV1::Inside)
        );
        assert_eq!(
            classify_frustum(
                &value,
                AabbMmV1::new([900, 0, 0], [1_100, 100, 100]).unwrap()
            ),
            Ok(FrustumClassV1::Intersect)
        );
        assert_eq!(
            classify_frustum(
                &value,
                AabbMmV1::new([5_000, 0, 0], [6_000, 100, 100]).unwrap()
            ),
            Ok(FrustumClassV1::Outside)
        );
        assert_eq!(
            AabbMmV1::new([1, 0, 0], [0, 0, 0]),
            Err(SelectionErrorV1::InvalidAabb)
        );
        assert!(matches!(
            PlaneQ40V1::new([0; 3], 0),
            Err(CameraErrorV1::InvalidPlane(0))
        ));
    }

    #[test]
    fn sse_is_checked_and_zero_distance_is_typed() {
        assert_eq!(screen_space_error_q16(500, 65_536, 1_000), Ok(32_768));
        assert_eq!(screen_space_error_q16(1, 10, 3), Ok(4));
        assert_eq!(
            screen_space_error_q16(1, 1, 0),
            Err(SelectionErrorV1::ZeroDistance)
        );
        assert_eq!(
            screen_space_error_q16(1, 0, 1),
            Err(SelectionErrorV1::ZeroProjectionScale)
        );
        assert_eq!(
            screen_space_error_q16(u64::MAX, u64::MAX, 1),
            Err(SelectionErrorV1::ArithmeticOverflow)
        );
    }

    #[test]
    fn lod_promote_hold_demote_and_residence_are_explicit() {
        let transitions = [
            LodTransitionV1 {
                promote_above_q16: 100,
                demote_below_q16: 80,
                minimum_residence_ticks: 2,
            },
            LodTransitionV1 {
                promote_above_q16: 200,
                demote_below_q16: 180,
                minimum_residence_ticks: 3,
            },
        ];
        assert_eq!(
            select_lod(150, &transitions, LodStateV1 {
                tier: 0,
                residence_ticks: 1
            }),
            Ok(LodStateV1 {
                tier: 0,
                residence_ticks: 2
            })
        );
        assert_eq!(
            select_lod(150, &transitions, LodStateV1 {
                tier: 0,
                residence_ticks: 2
            }),
            Ok(LodStateV1 {
                tier: 1,
                residence_ticks: 0
            })
        );
        assert_eq!(
            select_lod(90, &transitions, LodStateV1 {
                tier: 1,
                residence_ticks: 4
            }),
            Ok(LodStateV1 {
                tier: 1,
                residence_ticks: 5
            })
        );
        assert_eq!(
            select_lod(70, &transitions, LodStateV1 {
                tier: 1,
                residence_ticks: 2
            }),
            Ok(LodStateV1 {
                tier: 0,
                residence_ticks: 0
            })
        );
        assert_eq!(
            select_lod(
                1,
                &[LodTransitionV1 {
                    promote_above_q16: 10,
                    demote_below_q16: 10,
                    minimum_residence_ticks: 0
                }],
                LodStateV1 {
                    tier: 0,
                    residence_ticks: 0
                }
            ),
            Err(SelectionErrorV1::InvalidLodThresholdOrder)
        );
    }

    #[test]
    fn animation_is_tick_and_full_digest_pure() {
        let sampling = AnimationSamplingV1 {
            clip_length_ticks: 120,
            sample_count: 60,
            cadence_ticks: 4,
        };
        let a = animation_sample([0x11; 32], [0x22; 32], 120, sampling).unwrap();
        let b = animation_sample([0x11; 32], [0x22; 32], 120, sampling).unwrap();
        let changed = animation_sample([0x12; 32], [0x22; 32], 120, sampling).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, changed);
        assert_eq!(a, AnimationSampleV1 {
            sample_index: 50,
            update_due: true
        });
        assert_eq!(
            animation_sample([0; 32], [0; 32], 0, AnimationSamplingV1 {
                clip_length_ticks: 0,
                sample_count: 1,
                cadence_ticks: 1
            }),
            Err(SelectionErrorV1::InvalidAnimationRange)
        );
        for invalid in [
            AnimationSamplingV1 {
                clip_length_ticks: 1,
                sample_count: 0,
                cadence_ticks: 1,
            },
            AnimationSamplingV1 {
                clip_length_ticks: 1,
                sample_count: 1,
                cadence_ticks: 0,
            },
            AnimationSamplingV1 {
                clip_length_ticks: 4,
                sample_count: 1,
                cadence_ticks: 5,
            },
            AnimationSamplingV1 {
                clip_length_ticks: MAX_ANIMATION_CLIP_TICKS_V1 + 1,
                sample_count: 1,
                cadence_ticks: 1,
            },
        ] {
            assert_eq!(
                animation_sample([0; 32], [0; 32], 0, invalid),
                Err(SelectionErrorV1::InvalidAnimationRange)
            );
        }
    }

    #[test]
    fn visible_and_draw_order_and_duplicate_policy_are_explicit() {
        let visible = |digest, alias| VisibleRecordV1 {
            semantic_digest: [digest; 32],
            compact_alias: alias,
        };
        let ordered = canonicalize_visible(vec![visible(2, 2), visible(1, 1)]).unwrap();
        assert_eq!(ordered[0].semantic_digest, [1; 32]);
        assert_eq!(
            canonicalize_visible(vec![visible(1, 1), visible(1, 2)]),
            Err(SelectionErrorV1::DuplicateVisibleDigest([1; 32]))
        );
        let key = |pass, entity| DrawKeyV1 {
            pass_tag: pass,
            pipeline_tag: 1,
            package_digest: [2; 32],
            material_tag: 3,
            lod: 4,
            semantic_digest: [entity; 32],
            submesh_tag: 5,
        };
        let args = DrawIndexedIndirectArgsV1 {
            index_count: 1,
            instance_count: 1,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        };
        let ordered = canonicalize_draws(vec![
            DrawRecordV1 {
                key: key(2, 1),
                arguments: args,
            },
            DrawRecordV1 {
                key: key(1, 2),
                arguments: args,
            },
        ])
        .unwrap();
        assert_eq!(ordered[0].key.pass_tag, 1);
        assert_eq!(
            canonicalize_draws(vec![
                DrawRecordV1 {
                    key: key(1, 1),
                    arguments: args
                },
                DrawRecordV1 {
                    key: key(1, 1),
                    arguments: args
                },
            ]),
            Err(SelectionErrorV1::DuplicateDrawKey)
        );
    }

    #[test]
    fn indirect_argument_bytes_are_frozen_little_endian() {
        assert_eq!(
            hex_bytes(
                &DrawIndirectArgsV1 {
                    vertex_count: 6,
                    instance_count: 2,
                    first_vertex: 0,
                    first_instance: 1
                }
                .encode_le()
            ),
            "06000000020000000000000001000000"
        );
        assert_eq!(
            hex_bytes(
                &DrawIndexedIndirectArgsV1 {
                    index_count: 36,
                    instance_count: 1,
                    first_index: 0,
                    base_vertex: -4,
                    first_instance: 0
                }
                .encode_le()
            ),
            "240000000100000000000000fcffffff00000000"
        );
    }
}
