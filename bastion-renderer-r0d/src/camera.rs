//! Content-addressed fixed-point camera scripts.
//!
//! Runtime camera decisions consume only validated integer samples. One exact
//! frame token advances one sample; missing tokens and surface loss are typed
//! terminals and never reuse or skip a sample.

use crate::{
    DomainHashErrorV1, bootstrap::V1_TICK_CAP, domain_hash_v1, extract::MAX_ABS_POSITION_MM_V1,
};

pub const Q30_ONE_V1: i32 = 1 << 30;
pub const MAX_CAMERA_SAMPLES_V1: usize = 65_536;
pub const MAX_CAPTURE_REQUESTS_PER_SAMPLE_V1: usize = 64;
pub const MAX_CAPTURE_SPEC_BYTES_V1: usize = 4_096;
pub const MAX_VIEWPORT_DIMENSION_V1: u32 = 16_384;
pub const MAX_VERTICAL_FOV_MICRORADIANS_V1: u32 = 3_140_000;
pub const MAX_PROJECTION_DISTANCE_MM_V1: i64 = 9_000_000_000_000;
pub const MAX_PLANE_NORMAL_Q40_V1: i64 = 1_i64 << 48;

const Q30_NORM_SQUARED_V1: i128 = 1_i128 << 60;
const Q30_NORM_TOLERANCE_V1: i128 = 1_i128 << 50;
const Q30_ORTHOGONAL_TOLERANCE_V1: i128 = 1_i128 << 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CameraErrorV1 {
    ZeroBasisVector(u8),
    BasisNormOutOfRange(u8),
    BasisNotOrthogonal(u8, u8),
    NonPositiveHandedness,
    PositionOutOfRange,
    InvalidPerspectiveFov(u32),
    InvalidOrthographicHeight(i64),
    InvalidNearFar { near_mm: i64, far_mm: i64 },
    InvalidViewport { width: u32, height: u32 },
    InvalidPlane(u8),
    PlaneOutOfRange(u8),
    InvalidCaptureTag,
    CaptureSpecTooLarge { actual: usize, maximum: usize },
    TooManyCaptureRequests { actual: usize, maximum: usize },
    DuplicateCaptureTag(u16),
    EmptyScript,
    TooManySamples { actual: usize, maximum: usize },
    NonIncreasingFrame { previous: u64, offered: u64 },
    NonMonotonicSimulationTick { previous: u64, offered: u64 },
    TickOutOfRange(u64),
    SizeOverflow,
    AllocationFailure,
    HashFailure(DomainHashErrorV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CameraBasisQ30V1 {
    right: [i32; 3],
    up: [i32; 3],
    forward: [i32; 3],
}

impl CameraBasisQ30V1 {
    pub fn new(right: [i32; 3], up: [i32; 3], forward: [i32; 3]) -> Result<Self, CameraErrorV1> {
        let value = Self { right, up, forward };
        value.validate()?;
        Ok(value)
    }

    pub fn axis_aligned() -> Self {
        Self {
            right: [Q30_ONE_V1, 0, 0],
            up: [0, Q30_ONE_V1, 0],
            forward: [0, 0, Q30_ONE_V1],
        }
    }

    pub fn validate(&self) -> Result<(), CameraErrorV1> {
        let vectors = [self.right, self.up, self.forward];
        for (index, vector) in vectors.iter().copied().enumerate() {
            let index = u8::try_from(index).map_err(|_| CameraErrorV1::SizeOverflow)?;
            if vector == [0; 3] {
                return Err(CameraErrorV1::ZeroBasisVector(index));
            }
            let norm = dot_q30(vector, vector)?;
            let difference = norm
                .checked_sub(Q30_NORM_SQUARED_V1)
                .ok_or(CameraErrorV1::SizeOverflow)?
                .checked_abs()
                .ok_or(CameraErrorV1::SizeOverflow)?;
            if difference > Q30_NORM_TOLERANCE_V1 {
                return Err(CameraErrorV1::BasisNormOutOfRange(index));
            }
        }
        for (left, right) in [(0_usize, 1_usize), (0, 2), (1, 2)] {
            let absolute = dot_q30(vectors[left], vectors[right])?
                .checked_abs()
                .ok_or(CameraErrorV1::SizeOverflow)?;
            if absolute > Q30_ORTHOGONAL_TOLERANCE_V1 {
                return Err(CameraErrorV1::BasisNotOrthogonal(
                    u8::try_from(left).map_err(|_| CameraErrorV1::SizeOverflow)?,
                    u8::try_from(right).map_err(|_| CameraErrorV1::SizeOverflow)?,
                ));
            }
        }
        let cross = cross_q30(self.right, self.up)?;
        let handedness = cross
            .iter()
            .zip(self.forward)
            .try_fold(0_i128, |sum, (left, right)| {
                left.checked_mul(i128::from(right))
                    .and_then(|term| sum.checked_add(term))
            })
            .ok_or(CameraErrorV1::SizeOverflow)?;
        if handedness <= 0 {
            return Err(CameraErrorV1::NonPositiveHandedness);
        }
        Ok(())
    }

    #[must_use]
    pub const fn right(&self) -> [i32; 3] { self.right }

    #[must_use]
    pub const fn up(&self) -> [i32; 3] { self.up }

    #[must_use]
    pub const fn forward(&self) -> [i32; 3] { self.forward }
}

fn dot_q30(left: [i32; 3], right: [i32; 3]) -> Result<i128, CameraErrorV1> {
    left.into_iter()
        .zip(right)
        .try_fold(0_i128, |sum, (left, right)| {
            i128::from(left)
                .checked_mul(i128::from(right))
                .and_then(|term| sum.checked_add(term))
        })
        .ok_or(CameraErrorV1::SizeOverflow)
}

fn cross_q30(left: [i32; 3], right: [i32; 3]) -> Result<[i128; 3], CameraErrorV1> {
    let component = |a: usize, b: usize, c: usize, d: usize| {
        i128::from(left[a])
            .checked_mul(i128::from(right[b]))
            .and_then(|first| {
                i128::from(left[c])
                    .checked_mul(i128::from(right[d]))
                    .and_then(|second| first.checked_sub(second))
            })
            .ok_or(CameraErrorV1::SizeOverflow)
    };
    Ok([
        component(1, 2, 2, 1)?,
        component(2, 0, 0, 2)?,
        component(0, 1, 1, 0)?,
    ])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CameraProjectionV1 {
    Perspective {
        vertical_fov_microradians: u32,
        near_mm: i64,
        far_mm: i64,
        viewport_width: u32,
        viewport_height: u32,
    },
    Orthographic {
        half_height_micrometers: i64,
        near_mm: i64,
        far_mm: i64,
        viewport_width: u32,
        viewport_height: u32,
    },
}

impl CameraProjectionV1 {
    pub fn validate(self) -> Result<(), CameraErrorV1> {
        let (near, far, width, height) = match self {
            Self::Perspective {
                vertical_fov_microradians,
                near_mm,
                far_mm,
                viewport_width,
                viewport_height,
            } => {
                if vertical_fov_microradians == 0
                    || vertical_fov_microradians > MAX_VERTICAL_FOV_MICRORADIANS_V1
                {
                    return Err(CameraErrorV1::InvalidPerspectiveFov(
                        vertical_fov_microradians,
                    ));
                }
                (near_mm, far_mm, viewport_width, viewport_height)
            },
            Self::Orthographic {
                half_height_micrometers,
                near_mm,
                far_mm,
                viewport_width,
                viewport_height,
            } => {
                if half_height_micrometers <= 0
                    || half_height_micrometers > MAX_PROJECTION_DISTANCE_MM_V1
                {
                    return Err(CameraErrorV1::InvalidOrthographicHeight(
                        half_height_micrometers,
                    ));
                }
                (near_mm, far_mm, viewport_width, viewport_height)
            },
        };
        if near <= 0 || far <= near || far > MAX_PROJECTION_DISTANCE_MM_V1 {
            return Err(CameraErrorV1::InvalidNearFar {
                near_mm: near,
                far_mm: far,
            });
        }
        if width == 0
            || height == 0
            || width > MAX_VIEWPORT_DIMENSION_V1
            || height > MAX_VIEWPORT_DIMENSION_V1
        {
            return Err(CameraErrorV1::InvalidViewport { width, height });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlaneQ40V1 {
    normal: [i64; 3],
    distance: i64,
}

impl PlaneQ40V1 {
    pub fn new(normal: [i64; 3], distance: i64) -> Result<Self, CameraErrorV1> {
        let plane = Self { normal, distance };
        plane.validate(0)?;
        Ok(plane)
    }

    fn validate(self, index: u8) -> Result<(), CameraErrorV1> {
        if self.normal == [0; 3] {
            return Err(CameraErrorV1::InvalidPlane(index));
        }
        if self.normal.iter().copied().any(|component| {
            component < -MAX_PLANE_NORMAL_Q40_V1 || component > MAX_PLANE_NORMAL_Q40_V1
        }) {
            return Err(CameraErrorV1::PlaneOutOfRange(index));
        }
        Ok(())
    }

    #[must_use]
    pub const fn normal(self) -> [i64; 3] { self.normal }

    #[must_use]
    pub const fn distance(self) -> i64 { self.distance }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CameraFrustumV1 {
    planes: [PlaneQ40V1; 6],
}

impl CameraFrustumV1 {
    pub fn new(planes: [PlaneQ40V1; 6]) -> Result<Self, CameraErrorV1> {
        for (index, plane) in planes.iter().copied().enumerate() {
            plane.validate(u8::try_from(index).map_err(|_| CameraErrorV1::SizeOverflow)?)?;
        }
        Ok(Self { planes })
    }

    #[must_use]
    pub const fn planes(&self) -> &[PlaneQ40V1; 6] { &self.planes }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureSpecV1 {
    tag: u16,
    bytes: Vec<u8>,
}

impl CaptureSpecV1 {
    pub fn new(tag: u16, bytes: Vec<u8>) -> Result<Self, CameraErrorV1> {
        if tag == 0 {
            return Err(CameraErrorV1::InvalidCaptureTag);
        }
        if bytes.len() > MAX_CAPTURE_SPEC_BYTES_V1 {
            return Err(CameraErrorV1::CaptureSpecTooLarge {
                actual: bytes.len(),
                maximum: MAX_CAPTURE_SPEC_BYTES_V1,
            });
        }
        Ok(Self { tag, bytes })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererCameraSampleV1 {
    pub frame: u64,
    pub simulation_tick: u64,
    pub position_mm: [i64; 3],
    pub basis: CameraBasisQ30V1,
    pub projection: CameraProjectionV1,
    pub frustum: CameraFrustumV1,
    pub capture_requests: Vec<CaptureSpecV1>,
}

impl RendererCameraSampleV1 {
    fn validate_and_canonicalize(&mut self) -> Result<(), CameraErrorV1> {
        if self.simulation_tick >= V1_TICK_CAP {
            return Err(CameraErrorV1::TickOutOfRange(self.simulation_tick));
        }
        if self.position_mm.iter().copied().any(|component| {
            component < -MAX_ABS_POSITION_MM_V1 || component > MAX_ABS_POSITION_MM_V1
        }) {
            return Err(CameraErrorV1::PositionOutOfRange);
        }
        self.basis.validate()?;
        self.projection.validate()?;
        CameraFrustumV1::new(*self.frustum.planes())?;
        if self.capture_requests.len() > MAX_CAPTURE_REQUESTS_PER_SAMPLE_V1 {
            return Err(CameraErrorV1::TooManyCaptureRequests {
                actual: self.capture_requests.len(),
                maximum: MAX_CAPTURE_REQUESTS_PER_SAMPLE_V1,
            });
        }
        self.capture_requests
            .sort_unstable_by_key(|capture| capture.tag);
        if let Some(duplicate) = self
            .capture_requests
            .windows(2)
            .find(|pair| pair[0].tag == pair[1].tag)
        {
            return Err(CameraErrorV1::DuplicateCaptureTag(duplicate[0].tag));
        }
        Ok(())
    }

    fn encode(&self, output: &mut Vec<u8>) -> Result<(), CameraErrorV1> {
        output.extend_from_slice(&self.frame.to_le_bytes());
        output.extend_from_slice(&self.simulation_tick.to_le_bytes());
        for component in self.position_mm {
            output.extend_from_slice(&component.to_le_bytes());
        }
        for vector in [self.basis.right(), self.basis.up(), self.basis.forward()] {
            for component in vector {
                output.extend_from_slice(&component.to_le_bytes());
            }
        }
        match self.projection {
            CameraProjectionV1::Perspective {
                vertical_fov_microradians,
                near_mm,
                far_mm,
                viewport_width,
                viewport_height,
            } => {
                output.push(0);
                output.extend_from_slice(&vertical_fov_microradians.to_le_bytes());
                output.extend_from_slice(&near_mm.to_le_bytes());
                output.extend_from_slice(&far_mm.to_le_bytes());
                output.extend_from_slice(&viewport_width.to_le_bytes());
                output.extend_from_slice(&viewport_height.to_le_bytes());
            },
            CameraProjectionV1::Orthographic {
                half_height_micrometers,
                near_mm,
                far_mm,
                viewport_width,
                viewport_height,
            } => {
                output.push(1);
                output.extend_from_slice(&half_height_micrometers.to_le_bytes());
                output.extend_from_slice(&near_mm.to_le_bytes());
                output.extend_from_slice(&far_mm.to_le_bytes());
                output.extend_from_slice(&viewport_width.to_le_bytes());
                output.extend_from_slice(&viewport_height.to_le_bytes());
            },
        }
        for plane in self.frustum.planes() {
            for component in plane.normal() {
                output.extend_from_slice(&component.to_le_bytes());
            }
            output.extend_from_slice(&plane.distance().to_le_bytes());
        }
        output.extend_from_slice(
            &u16::try_from(self.capture_requests.len())
                .map_err(|_| CameraErrorV1::SizeOverflow)?
                .to_le_bytes(),
        );
        for capture in &self.capture_requests {
            output.extend_from_slice(&capture.tag.to_le_bytes());
            output.extend_from_slice(
                &u16::try_from(capture.bytes.len())
                    .map_err(|_| CameraErrorV1::SizeOverflow)?
                    .to_le_bytes(),
            );
            output.extend_from_slice(&capture.bytes);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCameraScriptV1 {
    samples: Vec<RendererCameraSampleV1>,
    digest: [u8; 32],
}

impl CompiledCameraScriptV1 {
    pub fn compile(mut samples: Vec<RendererCameraSampleV1>) -> Result<Self, CameraErrorV1> {
        if samples.is_empty() {
            return Err(CameraErrorV1::EmptyScript);
        }
        if samples.len() > MAX_CAMERA_SAMPLES_V1 {
            return Err(CameraErrorV1::TooManySamples {
                actual: samples.len(),
                maximum: MAX_CAMERA_SAMPLES_V1,
            });
        }
        for sample in &mut samples {
            sample.validate_and_canonicalize()?;
        }
        for pair in samples.windows(2) {
            if pair[1].frame <= pair[0].frame {
                return Err(CameraErrorV1::NonIncreasingFrame {
                    previous: pair[0].frame,
                    offered: pair[1].frame,
                });
            }
            if pair[1].simulation_tick < pair[0].simulation_tick {
                return Err(CameraErrorV1::NonMonotonicSimulationTick {
                    previous: pair[0].simulation_tick,
                    offered: pair[1].simulation_tick,
                });
            }
        }
        let fixed_bytes = samples
            .len()
            .checked_mul(400)
            .ok_or(CameraErrorV1::SizeOverflow)?;
        let capture_bytes = samples
            .iter()
            .flat_map(|sample| &sample.capture_requests)
            .try_fold(0_usize, |sum, capture| {
                sum.checked_add(capture.bytes.len().checked_add(4)?)
            })
            .ok_or(CameraErrorV1::SizeOverflow)?;
        let capacity = fixed_bytes
            .checked_add(capture_bytes)
            .and_then(|value| value.checked_add(8))
            .ok_or(CameraErrorV1::SizeOverflow)?;
        let mut payload = Vec::new();
        payload
            .try_reserve_exact(capacity)
            .map_err(|_| CameraErrorV1::AllocationFailure)?;
        payload.extend_from_slice(
            &u64::try_from(samples.len())
                .map_err(|_| CameraErrorV1::SizeOverflow)?
                .to_le_bytes(),
        );
        for sample in &samples {
            sample.encode(&mut payload)?;
        }
        let digest = domain_hash_v1("bastion/r0d/camera-script", 1, 0, &payload)
            .map_err(CameraErrorV1::HashFailure)?;
        Ok(Self { samples, digest })
    }

    #[must_use]
    pub fn len(&self) -> usize { self.samples.len() }

    #[must_use]
    pub const fn digest(&self) -> [u8; 32] { self.digest }

    pub fn cursor(&self, run_epoch: u64) -> Result<CameraScriptCursorV1<'_>, CameraTerminalV1> {
        if run_epoch == 0 {
            return Err(CameraTerminalV1::InvalidRunEpoch);
        }
        Ok(CameraScriptCursorV1 {
            script: self,
            run_epoch,
            position: 0,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CameraFrameTokenV1 {
    pub run_epoch: u64,
    pub frame: u64,
    pub simulation_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceStateV1 {
    Available,
    Lost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CameraTerminalV1 {
    InvalidRunEpoch,
    MissingFrameToken,
    SurfaceLost,
    ScriptExhausted,
    FrameTokenMismatch,
}

pub struct CameraScriptCursorV1<'a> {
    script: &'a CompiledCameraScriptV1,
    run_epoch: u64,
    position: usize,
}

impl<'a> CameraScriptCursorV1<'a> {
    pub fn consume(
        &mut self,
        token: Option<CameraFrameTokenV1>,
        surface: SurfaceStateV1,
    ) -> Result<&'a RendererCameraSampleV1, CameraTerminalV1> {
        let token = token.ok_or(CameraTerminalV1::MissingFrameToken)?;
        if surface == SurfaceStateV1::Lost {
            return Err(CameraTerminalV1::SurfaceLost);
        }
        let sample = self
            .script
            .samples
            .get(self.position)
            .ok_or(CameraTerminalV1::ScriptExhausted)?;
        if token.run_epoch != self.run_epoch
            || token.frame != sample.frame
            || token.simulation_tick != sample.simulation_tick
        {
            return Err(CameraTerminalV1::FrameTokenMismatch);
        }
        self.position = self
            .position
            .checked_add(1)
            .ok_or(CameraTerminalV1::ScriptExhausted)?;
        Ok(sample)
    }

    #[must_use]
    pub fn remaining(&self) -> usize { self.script.samples.len() - self.position }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    fn perspective() -> CameraProjectionV1 {
        CameraProjectionV1::Perspective {
            vertical_fov_microradians: 1_570_000,
            near_mm: 100,
            far_mm: 100_000,
            viewport_width: 1_920,
            viewport_height: 1_080,
        }
    }

    fn frustum() -> CameraFrustumV1 {
        let q = 1_i64 << 40;
        CameraFrustumV1::new([
            PlaneQ40V1::new([q, 0, 0], q * 1_000).unwrap(),
            PlaneQ40V1::new([-q, 0, 0], q * 1_000).unwrap(),
            PlaneQ40V1::new([0, q, 0], q * 1_000).unwrap(),
            PlaneQ40V1::new([0, -q, 0], q * 1_000).unwrap(),
            PlaneQ40V1::new([0, 0, q], q * 1_000).unwrap(),
            PlaneQ40V1::new([0, 0, -q], q * 1_000).unwrap(),
        ])
        .unwrap()
    }

    fn sample(frame: u64, tick: u64, captures: Vec<CaptureSpecV1>) -> RendererCameraSampleV1 {
        RendererCameraSampleV1 {
            frame,
            simulation_tick: tick,
            position_mm: [0, 1_000, -5_000],
            basis: CameraBasisQ30V1::axis_aligned(),
            projection: perspective(),
            frustum: frustum(),
            capture_requests: captures,
        }
    }

    #[test]
    fn basis_checks_norm_orthogonality_and_handedness() {
        assert_eq!(CameraBasisQ30V1::axis_aligned().validate(), Ok(()));
        assert!(matches!(
            CameraBasisQ30V1::new([0; 3], [0, Q30_ONE_V1, 0], [0, 0, Q30_ONE_V1]),
            Err(CameraErrorV1::ZeroBasisVector(0))
        ));
        assert!(matches!(
            CameraBasisQ30V1::new([Q30_ONE_V1 / 2, 0, 0], [0, Q30_ONE_V1, 0], [
                0, 0, Q30_ONE_V1
            ]),
            Err(CameraErrorV1::BasisNormOutOfRange(0))
        ));
        assert!(matches!(
            CameraBasisQ30V1::new([Q30_ONE_V1, 0, 0], [Q30_ONE_V1, 0, 0], [0, 0, Q30_ONE_V1]),
            Err(CameraErrorV1::BasisNotOrthogonal(0, 1))
        ));
        assert_eq!(
            CameraBasisQ30V1::new([0, Q30_ONE_V1, 0], [Q30_ONE_V1, 0, 0], [0, 0, Q30_ONE_V1]),
            Err(CameraErrorV1::NonPositiveHandedness)
        );
    }

    #[test]
    fn projection_and_capture_bounds_are_typed() {
        assert_eq!(perspective().validate(), Ok(()));
        assert_eq!(
            CameraProjectionV1::Perspective {
                vertical_fov_microradians: 0,
                near_mm: 1,
                far_mm: 2,
                viewport_width: 1,
                viewport_height: 1
            }
            .validate(),
            Err(CameraErrorV1::InvalidPerspectiveFov(0))
        );
        assert!(matches!(
            CameraProjectionV1::Orthographic {
                half_height_micrometers: 0,
                near_mm: 1,
                far_mm: 2,
                viewport_width: 1,
                viewport_height: 1
            }
            .validate(),
            Err(CameraErrorV1::InvalidOrthographicHeight(0))
        ));
        assert!(matches!(
            CameraProjectionV1::Perspective {
                vertical_fov_microradians: 1,
                near_mm: 10,
                far_mm: 10,
                viewport_width: 1,
                viewport_height: 1
            }
            .validate(),
            Err(CameraErrorV1::InvalidNearFar { .. })
        ));
        assert!(matches!(
            CameraProjectionV1::Perspective {
                vertical_fov_microradians: 1,
                near_mm: 1,
                far_mm: 2,
                viewport_width: 0,
                viewport_height: 1
            }
            .validate(),
            Err(CameraErrorV1::InvalidViewport { .. })
        ));
        assert_eq!(
            PlaneQ40V1::new([0; 3], 0),
            Err(CameraErrorV1::InvalidPlane(0))
        );
        assert_eq!(
            PlaneQ40V1::new([MAX_PLANE_NORMAL_Q40_V1 + 1, 0, 0], 0),
            Err(CameraErrorV1::PlaneOutOfRange(0))
        );
        assert_eq!(
            CaptureSpecV1::new(0, vec![]),
            Err(CameraErrorV1::InvalidCaptureTag)
        );
        assert!(matches!(
            CaptureSpecV1::new(1, vec![0; MAX_CAPTURE_SPEC_BYTES_V1 + 1]),
            Err(CameraErrorV1::CaptureSpecTooLarge { .. })
        ));
    }

    #[test]
    fn malformed_duplicate_and_out_of_order_scripts_reject() {
        assert_eq!(
            CompiledCameraScriptV1::compile(vec![]),
            Err(CameraErrorV1::EmptyScript)
        );
        assert!(matches!(
            CompiledCameraScriptV1::compile(vec![sample(0, 0, vec![]), sample(0, 1, vec![])]),
            Err(CameraErrorV1::NonIncreasingFrame { .. })
        ));
        assert!(matches!(
            CompiledCameraScriptV1::compile(vec![sample(0, 2, vec![]), sample(1, 1, vec![])]),
            Err(CameraErrorV1::NonMonotonicSimulationTick { .. })
        ));
        let duplicate = vec![
            CaptureSpecV1::new(1, vec![1]).unwrap(),
            CaptureSpecV1::new(1, vec![2]).unwrap(),
        ];
        assert_eq!(
            CompiledCameraScriptV1::compile(vec![sample(0, 0, duplicate)]),
            Err(CameraErrorV1::DuplicateCaptureTag(1))
        );
    }

    #[test]
    fn capture_producer_order_does_not_change_script_identity() {
        let left = CompiledCameraScriptV1::compile(vec![sample(0, 0, vec![
            CaptureSpecV1::new(2, vec![2]).unwrap(),
            CaptureSpecV1::new(1, vec![1]).unwrap(),
        ])])
        .unwrap();
        let right = CompiledCameraScriptV1::compile(vec![sample(0, 0, vec![
            CaptureSpecV1::new(1, vec![1]).unwrap(),
            CaptureSpecV1::new(2, vec![2]).unwrap(),
        ])])
        .unwrap();
        assert_eq!(left.digest(), right.digest());
        assert_eq!(
            hex_bytes(&left.digest()),
            "46ce3b0340a8767abfa73bfb658dd1321d8356f6bd821e2464793f1efe68ceba"
        );
    }

    #[test]
    fn cursor_advances_exactly_once_and_errors_do_not_advance() {
        let script =
            CompiledCameraScriptV1::compile(vec![sample(10, 5, vec![]), sample(11, 5, vec![])])
                .unwrap();
        let mut cursor = script.cursor(7).unwrap();
        assert_eq!(
            cursor.consume(None, SurfaceStateV1::Available),
            Err(CameraTerminalV1::MissingFrameToken)
        );
        assert_eq!(cursor.remaining(), 2);
        let first = CameraFrameTokenV1 {
            run_epoch: 7,
            frame: 10,
            simulation_tick: 5,
        };
        assert_eq!(
            cursor.consume(Some(first), SurfaceStateV1::Lost),
            Err(CameraTerminalV1::SurfaceLost)
        );
        assert_eq!(cursor.remaining(), 2);
        assert_eq!(
            cursor.consume(
                Some(CameraFrameTokenV1 { frame: 99, ..first }),
                SurfaceStateV1::Available
            ),
            Err(CameraTerminalV1::FrameTokenMismatch)
        );
        assert_eq!(cursor.remaining(), 2);
        assert_eq!(
            cursor
                .consume(Some(first), SurfaceStateV1::Available)
                .unwrap()
                .frame,
            10
        );
        assert_eq!(cursor.remaining(), 1);
        assert_eq!(
            cursor
                .consume(
                    Some(CameraFrameTokenV1 {
                        run_epoch: 7,
                        frame: 11,
                        simulation_tick: 5
                    }),
                    SurfaceStateV1::Available
                )
                .unwrap()
                .frame,
            11
        );
        assert_eq!(
            cursor.consume(Some(first), SurfaceStateV1::Available),
            Err(CameraTerminalV1::ScriptExhausted)
        );
    }
}
