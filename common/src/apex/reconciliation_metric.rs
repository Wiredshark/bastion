//! `APEX-T7.3c` — the divergence metric. Ruled at
//! `numeric_probe::{QUANTIZATION_LAW_V1, QUANTIZATION_RULING_V1}` (one
//! ruling, two consumers: the determinism-audit semantic probe there,
//! and this reconciliation comparison here). This module implements the
//! ruling's three field classes against the concrete fields a client's
//! replayed rolling state and the server's authoritative CompSync
//! snapshot actually carry — it does not re-litigate the ruling, it
//! applies it.
//!
//! **The law, restated because it is what makes this module safe to
//! call from a reconciliation path:** quantization decides WHETHER
//! states agree, never WHAT gets written. [`check_agreement_v1`]
//! returns a decision, not a value — nothing here produces a Pos, Vel,
//! Ori, or any other component that could accidentally be written back
//! as if it were authoritative. A caller that wants to act on agreement
//! writes the ALREADY-VERBATIM authoritative values it already has, not
//! anything this function computed.

use crate::comp::{CharacterActivity, CharacterState, Density, Energy, Ori, Pos, Vel};
use vek::Vec3;

/// Position tolerance, class 2 of the ruling: below player perception
/// and below gameplay effect. 1e-3 world units (1mm at this engine's
/// scale).
pub const POS_TOLERANCE_V1: f32 = 1e-3;
/// Velocity tolerance, class 2. Same reviewed order of magnitude as
/// position — see `QUANTIZATION_RULING_V1`.
pub const VEL_TOLERANCE_V1: f32 = 1e-3;
/// Orientation tolerance, class 2, in RADIANS — compared as the angle
/// between look directions (`Ori::look_dir`), not raw quaternion
/// components, so double-cover (`q` and `-q` are the same rotation)
/// cannot manufacture a spurious divergence.
pub const ORI_TOLERANCE_V1: f32 = 1e-3;
/// Density tolerance. Class 2 by the same reasoning as position/
/// velocity/orientation, but NOT individually named in the ruling —
/// disclosed classification, not a value the ruling stated: density is
/// `TransitionInput`-classified rolling state
/// (`prediction_boundary::PREDICTION_FIELD_ROLES`), so it needs SOME
/// comparison, and nothing about it is branch-driving/discrete the way
/// class 1's fields are.
pub const DENSITY_TOLERANCE_V1: f32 = 1e-3;
/// Accumulator display precision, class 3, float case. Recorded for the
/// row's general law even though `Energy` (this module's only
/// accumulator field today) is integer-backed and compared exactly —
/// the constant exists for whatever float accumulator joins this
/// comparison next, not because today's fields need it.
pub const ACCUMULATOR_DISPLAY_PRECISION_V1: f32 = 0.01;

/// Why two rolling states were judged to disagree. First-reason
/// discipline (`WorldRevisionV1::replayable_against_v1`'s pattern,
/// `T7.1`'s Decision 2): the FIRST field the comparison found
/// differing, not an exhaustive diff — the first reason is the one to
/// act on, the rest are usually downstream of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DivergenceReasonV1 {
    /// Class 1: exact equality, no tolerance.
    CharacterStateDiffers,
    /// Class 1: exact equality, no tolerance.
    CharacterActivityDiffers,
    /// Class 2: quantized comparison exceeded its reviewed tolerance.
    PositionExceedsTolerance { distance: f32, tolerance: f32 },
    VelocityExceedsTolerance { distance: f32, tolerance: f32 },
    OrientationExceedsTolerance { radians: f32, tolerance: f32 },
    DensityExceedsTolerance { distance: f32, tolerance: f32 },
    /// Class 3: `Energy` is integer-backed, so exact equality.
    EnergyDiffers,
    /// A NaN/inf was found in a compared field, BEFORE any tolerance
    /// check ran on it. Its own reason, never quantized, never
    /// sentinel-mapped — the ruling's non-finite rule, and the same
    /// non-reflexivity trap `numeric_probe`'s semantic probe closes for
    /// the determinism-audit case.
    NonFinite { field: &'static str },
}

/// The rolling-state fields a divergence check compares, on either
/// side. Deliberately the same shape as
/// `character_behavior::RollingStateV1` — the comparison is field-for-
/// field against what that struct carries, not a projection of it, so
/// a caller builds one side from its `RollingStateV1` and the other
/// from CompSync-applied ECS reads without a translation layer.
#[derive(Clone, Debug, PartialEq)]
pub struct ComparableStateV1 {
    pub char_state: CharacterState,
    pub character_activity: CharacterActivity,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub density: Density,
    pub energy: Energy,
}

fn finite_vec3(field: &'static str, v: Vec3<f32>) -> Result<Vec3<f32>, DivergenceReasonV1> {
    if v.map(f32::is_finite).reduce_and() {
        Ok(v)
    } else {
        Err(DivergenceReasonV1::NonFinite { field })
    }
}

fn finite_scalar(field: &'static str, x: f32) -> Result<f32, DivergenceReasonV1> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(DivergenceReasonV1::NonFinite { field })
    }
}

/// Do a client's replayed rolling state and the server's authoritative
/// snapshot agree, per `QUANTIZATION_LAW_V1`/`QUANTIZATION_RULING_V1`?
///
/// `Ok(())` = agree. `Err(reason)` = the FIRST field found differing.
/// Field order below is the order the ruling lists its classes in:
/// discrete/semantic first (cheapest, and a mismatch there makes the
/// continuous comparison moot), then continuous physics, then the
/// accumulator.
pub fn check_agreement_v1(
    rolling: &ComparableStateV1,
    authoritative: &ComparableStateV1,
) -> Result<(), DivergenceReasonV1> {
    // Class 1: discrete/semantic -- exact equality, no tolerance.
    if rolling.char_state != authoritative.char_state {
        return Err(DivergenceReasonV1::CharacterStateDiffers);
    }
    if rolling.character_activity != authoritative.character_activity {
        return Err(DivergenceReasonV1::CharacterActivityDiffers);
    }

    // Class 2: continuous physics -- non-finite check first (its own
    // reason, never quantized), then the reviewed tolerance.
    let rolling_pos = finite_vec3("pos", rolling.pos.0)?;
    let authoritative_pos = finite_vec3("pos", authoritative.pos.0)?;
    let pos_distance = rolling_pos.distance(authoritative_pos);
    if pos_distance > POS_TOLERANCE_V1 {
        return Err(DivergenceReasonV1::PositionExceedsTolerance {
            distance: pos_distance,
            tolerance: POS_TOLERANCE_V1,
        });
    }

    let rolling_vel = finite_vec3("vel", rolling.vel.0)?;
    let authoritative_vel = finite_vec3("vel", authoritative.vel.0)?;
    let vel_distance = rolling_vel.distance(authoritative_vel);
    if vel_distance > VEL_TOLERANCE_V1 {
        return Err(DivergenceReasonV1::VelocityExceedsTolerance {
            distance: vel_distance,
            tolerance: VEL_TOLERANCE_V1,
        });
    }

    // Orientation: compared as the angle between look directions, not
    // raw quaternion components -- `q` and `-q` represent the SAME
    // rotation, and a raw-component comparison could report a
    // divergence between two orientations that are not actually
    // different.
    let rolling_look = *rolling.ori.look_dir();
    let authoritative_look = *authoritative.ori.look_dir();
    let rolling_look = finite_vec3("ori.look_dir", rolling_look)?;
    let authoritative_look = finite_vec3("ori.look_dir", authoritative_look)?;
    let ori_dot = finite_scalar("ori.dot", rolling_look.dot(authoritative_look))?;
    let ori_radians = ori_dot.clamp(-1.0, 1.0).acos();
    if ori_radians > ORI_TOLERANCE_V1 {
        return Err(DivergenceReasonV1::OrientationExceedsTolerance {
            radians: ori_radians,
            tolerance: ORI_TOLERANCE_V1,
        });
    }

    let rolling_density = finite_scalar("density", rolling.density.0)?;
    let authoritative_density = finite_scalar("density", authoritative.density.0)?;
    let density_distance = (rolling_density - authoritative_density).abs();
    if density_distance > DENSITY_TOLERANCE_V1 {
        return Err(DivergenceReasonV1::DensityExceedsTolerance {
            distance: density_distance,
            tolerance: DENSITY_TOLERANCE_V1,
        });
    }

    // Class 3: accumulator -- `Energy`'s current/base_max/maximum are
    // `u32`-backed (integer, per the struct's own doc: scaled fixed-
    // point specifically so it stays integer), so exact equality per
    // the ruling. `Energy` derives `PartialEq` over the whole struct,
    // which also compares `regen_rate` (an `f32`) exactly -- disclosed,
    // not silently included: `regen_rate` is recomputed fresh from
    // active buffs each tick rather than accumulated over time, so
    // comparing it exactly does not reintroduce the float-accumulation
    // drift the ruling's tolerance carve-out exists to absorb.
    if rolling.energy != authoritative.energy {
        return Err(DivergenceReasonV1::EnergyDiffers);
    }

    Ok(())
}

#[cfg(test)]
mod reconciliation_metric_v1 {
    use super::*;
    use crate::states::idle;

    fn baseline() -> ComparableStateV1 {
        ComparableStateV1 {
            char_state: CharacterState::Idle(idle::Data::default()),
            character_activity: CharacterActivity::default(),
            pos: Pos(Vec3::new(10.0, 20.0, 30.0)),
            vel: Vel(Vec3::new(1.0, 0.0, 0.0)),
            ori: Ori::default(),
            density: Density(1.0),
            energy: Energy::new(crate::comp::Body::Humanoid(
                crate::comp::humanoid::Body::random_with(
                    &mut rand::rng(),
                    &crate::comp::humanoid::Species::Human,
                ),
            )),
        }
    }

    /// Ruling acceptance: a sub-tolerance perturbation on a continuous
    /// field does NOT fire -- the damping proven, not assumed.
    #[test]
    fn sub_tolerance_perturbation_does_not_diverge() {
        let a = baseline();
        let mut b = baseline();
        b.pos.0.x += POS_TOLERANCE_V1 * 0.5;
        assert_eq!(check_agreement_v1(&a, &b), Ok(()));
    }

    /// Ruling acceptance: a supra-tolerance perturbation fires, and
    /// names the field and the measured distance.
    #[test]
    fn supra_tolerance_perturbation_diverges() {
        let a = baseline();
        let mut b = baseline();
        b.pos.0.x += POS_TOLERANCE_V1 * 2.0;
        let Err(DivergenceReasonV1::PositionExceedsTolerance { distance, tolerance }) =
            check_agreement_v1(&a, &b)
        else {
            panic!("expected a PositionExceedsTolerance divergence");
        };
        assert_eq!(tolerance, POS_TOLERANCE_V1);
        assert!(distance > tolerance);
    }

    /// Just under the tolerance boundary is NOT a divergence -- the
    /// comparison is `>`, not `>=`, matching "below player perception
    /// AND below gameplay effect". Deliberately not testing the EXACT
    /// f32 boundary: `1.0 + VEL_TOLERANCE_V1` is not exactly
    /// representable, so reconstructing it via addition lands a hair on
    /// either side depending on rounding -- `sub_tolerance_perturbation_
    /// does_not_diverge` already covers "clearly under agrees" and
    /// `supra_tolerance_perturbation_diverges` covers "clearly over
    /// fires"; this covers the boundary's NEAR side without depending on
    /// float addition hitting an exact value.
    #[test]
    fn just_under_tolerance_does_not_diverge() {
        let a = baseline();
        let mut b = baseline();
        b.vel.0.x += VEL_TOLERANCE_V1 * 0.99;
        assert_eq!(check_agreement_v1(&a, &b), Ok(()));
    }

    /// Ruling acceptance: NaN diverges with its own reason -- never
    /// quantized, never sentinel-mapped, and reflexive comparisons
    /// (same-object-to-itself) do not silently pass either, since the
    /// non-finite check runs before the tolerance check on both sides.
    #[test]
    fn nan_diverges_with_its_own_reason_not_a_tolerance_reason() {
        let a = baseline();
        let mut b = baseline();
        b.pos.0.x = f32::NAN;
        assert_eq!(
            check_agreement_v1(&a, &b),
            Err(DivergenceReasonV1::NonFinite { field: "pos" })
        );

        // Reflexive: comparing the NaN-carrying state to ITSELF still
        // diverges with the NonFinite reason, not `Ok(())` -- a NaN is
        // never treated as agreeing with anything, including itself.
        assert_eq!(
            check_agreement_v1(&b, &b),
            Err(DivergenceReasonV1::NonFinite { field: "pos" })
        );
    }

    /// Infinity is non-finite too, and reported distinctly from NaN by
    /// field name where they differ.
    #[test]
    fn infinity_diverges_as_non_finite() {
        let a = baseline();
        let mut b = baseline();
        b.vel.0.y = f32::INFINITY;
        assert_eq!(
            check_agreement_v1(&a, &b),
            Err(DivergenceReasonV1::NonFinite { field: "vel" })
        );
    }

    /// Class 1: any CharacterState mismatch is a divergence regardless
    /// of how "close" the two variants might seem -- no tolerance
    /// exists for discrete state, and this is checked FIRST (before any
    /// continuous field), matching the ruling's stated field order.
    #[test]
    fn discrete_character_state_mismatch_diverges_before_continuous_fields_are_even_checked() {
        let a = baseline();
        let mut b = baseline();
        b.char_state = CharacterState::Idle(idle::Data {
            is_sneaking: true,
            ..idle::Data::default()
        });
        // Also perturb pos supra-tolerance -- if the discrete check
        // did not run first, this would report PositionExceedsTolerance
        // instead.
        b.pos.0.x += POS_TOLERANCE_V1 * 2.0;
        assert_eq!(check_agreement_v1(&a, &b), Err(DivergenceReasonV1::CharacterStateDiffers));
    }

    /// Class 3: `Energy` is integer-backed, so ANY difference in its
    /// current/max counters is an exact-equality divergence -- there is
    /// no "sub-tolerance energy drift" the way there is for position.
    /// `change_by(-1.0)` rather than `+1.0`: `Energy::new` starts at
    /// full (`current == maximum`), so a POSITIVE change is clamped
    /// right back to `maximum` and produces no observable change at
    /// all -- reducing is the only direction guaranteed to move it from
    /// a freshly-constructed baseline.
    #[test]
    fn accumulator_energy_difference_diverges_exactly_no_tolerance() {
        let a = baseline();
        let mut b = baseline();
        b.energy.change_by(-1.0);
        assert_eq!(check_agreement_v1(&a, &b), Err(DivergenceReasonV1::EnergyDiffers));
    }

    /// Identical states agree -- the reflexive positive case, so the
    /// NaN test's reflexive negative case (above) has something to
    /// contrast against.
    #[test]
    fn identical_states_agree() {
        let a = baseline();
        assert_eq!(check_agreement_v1(&a, &a), Ok(()));
    }
}
