//! BUILD-007A10.5 (part 1) — frame-index camera and deterministic camera
//! compiler substrate (design §11).
//!
//! Canonical camera selection performs NO trig and NO vector normalization at
//! runtime: an offline compiler stores the right/up/forward basis directly as
//! signed fixed-point integers (Q1.30) and the frustum as fixed-point planes
//! (Q24.40), then validates them once. The compiled script is content-addressed
//! like an asset. One accepted frame token consumes exactly one sample; surface
//! loss is a typed terminal, never a silent skip/reuse (§11.3).
//!
//! The live per-frame runtime adapter (surface acquire, screenshot callback) is
//! the integration surface; this module is the self-contained compiler +
//! validator + progression core with a golden vector.

/// Q1.30 fixed-point unit (`1.0`) and its square, used for basis norm bounds.
const Q1_30_ONE: i64 = 1 << 30;
const Q1_30_ONE_SQ: i128 = (Q1_30_ONE as i128) * (Q1_30_ONE as i128); // 2^60

/// Compiler-version norm tolerance: a basis vector's squared length must lie
/// within this band of unit (Q1.30). ~1.5% either way.
const NORM_SQ_TOLERANCE: i128 = 1 << 54;

/// Right/up/forward basis stored directly as signed Q1.30 integers (§11.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraBasisQ1_30V1 {
    pub right: [i32; 3],
    pub up: [i32; 3],
    pub forward: [i32; 3],
}

/// A frustum plane in Q24.40 fixed-point (§11.1): normal + signed distance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaneQ24_40V1 {
    pub normal: [i64; 3],
    pub distance: i64,
}

/// Camera projection (§11.1). Both variants carry integer near/far/viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// A capture request bound to a camera sample (§11.1). Opaque spec bytes here;
/// the live capture semantics are in the integration surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureSpecV1 {
    pub capture_tag: u16,
    pub spec_bytes: Vec<u8>,
}

/// One canonical camera sample (§11.1): names both simulation tick and render
/// frame, so multiple render frames per tick are permitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererCameraSampleV1 {
    pub frame: u64,
    pub simulation_tick: u64,
    pub position_mm: [i64; 3],
    pub basis: CameraBasisQ1_30V1,
    pub projection: CameraProjectionV1,
    pub frustum_planes_q24_40: [PlaneQ24_40V1; 6],
    pub capture_requests: Vec<CaptureSpecV1>,
}

/// Typed camera-compiler failures (§11.2). Every one aborts compilation; no
/// best-effort camera is ever produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CameraError {
    ZeroBasisVector { which: &'static str },
    BasisNormOutOfBounds { which: &'static str, norm_sq: i128 },
    NonPositiveHandedness { triple_product: i128 },
    NearNotPositive { near_mm: i64 },
    FarNotBeyondNear { near_mm: i64, far_mm: i64 },
    ZeroViewport,
    DegeneratePlane { index: usize },
    NonIncreasingFrame { prev: u64, next: u64 },
    EmptyScript,
}

fn dot_i128(a: [i32; 3], b: [i32; 3]) -> i128 {
    (0..3).map(|i| i128::from(a[i]) * i128::from(b[i])).sum()
}

fn cross_i128(a: [i32; 3], b: [i32; 3]) -> [i128; 3] {
    [
        i128::from(a[1]) * i128::from(b[2]) - i128::from(a[2]) * i128::from(b[1]),
        i128::from(a[2]) * i128::from(b[0]) - i128::from(a[0]) * i128::from(b[2]),
        i128::from(a[0]) * i128::from(b[1]) - i128::from(a[1]) * i128::from(b[0]),
    ]
}

impl CameraBasisQ1_30V1 {
    /// Validate the basis with exact wide integer arithmetic (§11.2): each
    /// vector nonzero, squared norm within the compiler tolerance band of unit,
    /// and positive handedness (`(right × up) · forward > 0`). No trig, no
    /// normalization, no float.
    pub fn validate(&self) -> Result<(), CameraError> {
        for (v, name) in [(self.right, "right"), (self.up, "up"), (self.forward, "forward")] {
            if v == [0, 0, 0] {
                return Err(CameraError::ZeroBasisVector { which: name });
            }
            let n = dot_i128(v, v);
            if (n - Q1_30_ONE_SQ).abs() > NORM_SQ_TOLERANCE {
                return Err(CameraError::BasisNormOutOfBounds { which: name, norm_sq: n });
            }
        }
        // Handedness: (right × up) · forward, all in Q1.30 => product scale 2^90.
        let rxu = cross_i128(self.right, self.up);
        let tp: i128 = (0..3).map(|i| rxu[i] * i128::from(self.forward[i])).sum();
        if tp <= 0 {
            return Err(CameraError::NonPositiveHandedness { triple_product: tp });
        }
        Ok(())
    }
}

impl CameraProjectionV1 {
    fn near_far_viewport(&self) -> (i64, i64, u32, u32) {
        match *self {
            CameraProjectionV1::Perspective { near_mm, far_mm, viewport_width, viewport_height, .. }
            | CameraProjectionV1::Orthographic { near_mm, far_mm, viewport_width, viewport_height, .. } => {
                (near_mm, far_mm, viewport_width, viewport_height)
            }
        }
    }

    /// Validate near/far/viewport (§11.2): `near > 0`, `far > near`, viewport
    /// nonzero.
    pub fn validate(&self) -> Result<(), CameraError> {
        let (near, far, w, h) = self.near_far_viewport();
        if near <= 0 {
            return Err(CameraError::NearNotPositive { near_mm: near });
        }
        if far <= near {
            return Err(CameraError::FarNotBeyondNear { near_mm: near, far_mm: far });
        }
        if w == 0 || h == 0 {
            return Err(CameraError::ZeroViewport);
        }
        Ok(())
    }
}

impl RendererCameraSampleV1 {
    /// Validate one sample (§11.2): basis, projection, and six non-degenerate
    /// planes present.
    pub fn validate(&self) -> Result<(), CameraError> {
        self.basis.validate()?;
        self.projection.validate()?;
        for (i, p) in self.frustum_planes_q24_40.iter().enumerate() {
            if p.normal == [0, 0, 0] {
                return Err(CameraError::DegeneratePlane { index: i });
            }
        }
        Ok(())
    }

    /// Canonical length-framed serialization for content addressing.
    fn encode(&self, b: &mut Vec<u8>) {
        b.extend_from_slice(&self.frame.to_le_bytes());
        b.extend_from_slice(&self.simulation_tick.to_le_bytes());
        for c in self.position_mm {
            b.extend_from_slice(&c.to_le_bytes());
        }
        for v in [self.basis.right, self.basis.up, self.basis.forward] {
            for c in v {
                b.extend_from_slice(&c.to_le_bytes());
            }
        }
        match self.projection {
            CameraProjectionV1::Perspective { vertical_fov_microradians, near_mm, far_mm, viewport_width, viewport_height } => {
                b.push(0);
                b.extend_from_slice(&vertical_fov_microradians.to_le_bytes());
                b.extend_from_slice(&near_mm.to_le_bytes());
                b.extend_from_slice(&far_mm.to_le_bytes());
                b.extend_from_slice(&viewport_width.to_le_bytes());
                b.extend_from_slice(&viewport_height.to_le_bytes());
            }
            CameraProjectionV1::Orthographic { half_height_micrometers, near_mm, far_mm, viewport_width, viewport_height } => {
                b.push(1);
                b.extend_from_slice(&half_height_micrometers.to_le_bytes());
                b.extend_from_slice(&near_mm.to_le_bytes());
                b.extend_from_slice(&far_mm.to_le_bytes());
                b.extend_from_slice(&viewport_width.to_le_bytes());
                b.extend_from_slice(&viewport_height.to_le_bytes());
            }
        }
        for p in &self.frustum_planes_q24_40 {
            for c in p.normal {
                b.extend_from_slice(&c.to_le_bytes());
            }
            b.extend_from_slice(&p.distance.to_le_bytes());
        }
        b.extend_from_slice(&(self.capture_requests.len() as u64).to_le_bytes());
        for c in &self.capture_requests {
            b.extend_from_slice(&c.capture_tag.to_le_bytes());
            b.extend_from_slice(&(c.spec_bytes.len() as u64).to_le_bytes());
            b.extend_from_slice(&c.spec_bytes);
        }
    }
}

/// A compiled, validated, content-addressed camera script (§11.1).
#[derive(Clone, Debug)]
pub struct CompiledCameraScriptV1 {
    samples: Vec<RendererCameraSampleV1>,
}

impl CompiledCameraScriptV1 {
    /// Compile and validate a camera script (§11.2): every sample valid, and
    /// strictly increasing render-frame indices (V1 uses unique frames, no
    /// subframe ordinal). Returns the content-addressable script or a typed
    /// error.
    pub fn compile(samples: Vec<RendererCameraSampleV1>) -> Result<Self, CameraError> {
        if samples.is_empty() {
            return Err(CameraError::EmptyScript);
        }
        for s in &samples {
            s.validate()?;
        }
        for w in samples.windows(2) {
            if w[1].frame <= w[0].frame {
                return Err(CameraError::NonIncreasingFrame { prev: w[0].frame, next: w[1].frame });
            }
        }
        Ok(Self { samples })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The script content address (§4.4 domain-separated).
    #[must_use]
    pub fn script_digest(&self) -> [u8; 32] {
        let mut payload = Vec::new();
        payload.extend_from_slice(&(self.samples.len() as u64).to_le_bytes());
        for s in &self.samples {
            s.encode(&mut payload);
        }
        crate::domain_hash("bastion/r0d/camera-script", 1, 0, &payload)
    }

    #[must_use]
    pub fn cursor(&self) -> CameraScriptCursorV1 {
        CameraScriptCursorV1 { script: self, position: 0 }
    }
}

/// Frame-token progression over a compiled script (§11.3). Each accepted frame
/// token consumes exactly one sample. GPU/surface events cannot advance it.
pub struct CameraScriptCursorV1<'a> {
    script: &'a CompiledCameraScriptV1,
    position: usize,
}

impl<'a> CameraScriptCursorV1<'a> {
    /// Consume one frame token => exactly one sample advance (§11.3). `None`
    /// once the script is exhausted.
    pub fn consume_frame_token(&mut self) -> Option<&'a RendererCameraSampleV1> {
        let s = self.script.samples.get(self.position);
        if s.is_some() {
            self.position += 1;
        }
        s
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.script.samples.len() - self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    fn axis_basis() -> CameraBasisQ1_30V1 {
        CameraBasisQ1_30V1 {
            right: [1 << 30, 0, 0],
            up: [0, 1 << 30, 0],
            forward: [0, 0, 1 << 30],
        }
    }

    fn perspective() -> CameraProjectionV1 {
        CameraProjectionV1::Perspective {
            vertical_fov_microradians: 1_570_000,
            near_mm: 100,
            far_mm: 100_000,
            viewport_width: 1920,
            viewport_height: 1080,
        }
    }

    fn planes() -> [PlaneQ24_40V1; 6] {
        let mut p = [PlaneQ24_40V1 { normal: [1 << 40, 0, 0], distance: 0 }; 6];
        // Give each plane a distinct nonzero normal axis so none is degenerate.
        p[0].normal = [1 << 40, 0, 0];
        p[1].normal = [-(1 << 40), 0, 0];
        p[2].normal = [0, 1 << 40, 0];
        p[3].normal = [0, -(1 << 40), 0];
        p[4].normal = [0, 0, 1 << 40];
        p[5].normal = [0, 0, -(1 << 40)];
        p
    }

    fn sample(frame: u64) -> RendererCameraSampleV1 {
        RendererCameraSampleV1 {
            frame,
            simulation_tick: frame / 2,
            position_mm: [0, 1000, -5000],
            basis: axis_basis(),
            projection: perspective(),
            frustum_planes_q24_40: planes(),
            capture_requests: vec![],
        }
    }

    #[test]
    fn axis_aligned_basis_validates() {
        assert!(axis_basis().validate().is_ok());
    }

    #[test]
    fn zero_basis_vector_rejected() {
        let mut b = axis_basis();
        b.up = [0, 0, 0];
        assert_eq!(b.validate(), Err(CameraError::ZeroBasisVector { which: "up" }));
    }

    #[test]
    fn left_handed_basis_rejected() {
        // Swap right/up so (right × up) · forward flips sign.
        let b = CameraBasisQ1_30V1 {
            right: [0, 1 << 30, 0],
            up: [1 << 30, 0, 0],
            forward: [0, 0, 1 << 30],
        };
        assert!(matches!(b.validate(), Err(CameraError::NonPositiveHandedness { .. })));
    }

    #[test]
    fn non_unit_basis_rejected() {
        let b = CameraBasisQ1_30V1 {
            right: [1 << 29, 0, 0], // 0.5, norm_sq = 2^58, far below unit band
            up: [0, 1 << 30, 0],
            forward: [0, 0, 1 << 30],
        };
        assert!(matches!(b.validate(), Err(CameraError::BasisNormOutOfBounds { which: "right", .. })));
    }

    #[test]
    fn projection_range_checks() {
        let bad_near = CameraProjectionV1::Perspective { vertical_fov_microradians: 1, near_mm: 0, far_mm: 10, viewport_width: 1, viewport_height: 1 };
        assert_eq!(bad_near.validate(), Err(CameraError::NearNotPositive { near_mm: 0 }));
        let bad_far = CameraProjectionV1::Perspective { vertical_fov_microradians: 1, near_mm: 10, far_mm: 10, viewport_width: 1, viewport_height: 1 };
        assert_eq!(bad_far.validate(), Err(CameraError::FarNotBeyondNear { near_mm: 10, far_mm: 10 }));
        let bad_vp = CameraProjectionV1::Orthographic { half_height_micrometers: 1, near_mm: 1, far_mm: 2, viewport_width: 0, viewport_height: 1 };
        assert_eq!(bad_vp.validate(), Err(CameraError::ZeroViewport));
    }

    #[test]
    fn script_requires_strictly_increasing_frames() {
        assert!(CompiledCameraScriptV1::compile(vec![sample(0), sample(1), sample(2)]).is_ok());
        assert_eq!(
            CompiledCameraScriptV1::compile(vec![sample(0), sample(0)]).unwrap_err(),
            CameraError::NonIncreasingFrame { prev: 0, next: 0 }
        );
        assert!(matches!(
            CompiledCameraScriptV1::compile(vec![sample(5), sample(3)]),
            Err(CameraError::NonIncreasingFrame { .. })
        ));
        assert_eq!(CompiledCameraScriptV1::compile(vec![]).unwrap_err(), CameraError::EmptyScript);
    }

    #[test]
    fn cursor_consumes_one_sample_per_token() {
        let script = CompiledCameraScriptV1::compile(vec![sample(0), sample(1), sample(2)]).unwrap();
        let mut cur = script.cursor();
        assert_eq!(cur.remaining(), 3);
        assert_eq!(cur.consume_frame_token().unwrap().frame, 0);
        assert_eq!(cur.consume_frame_token().unwrap().frame, 1);
        assert_eq!(cur.remaining(), 1);
        assert_eq!(cur.consume_frame_token().unwrap().frame, 2);
        assert!(cur.consume_frame_token().is_none()); // exhausted, no reuse
    }

    #[test]
    fn frozen_script_digest() {
        let script = CompiledCameraScriptV1::compile(vec![sample(0), sample(1)]).unwrap();
        assert_eq!(
            hex_bytes(&script.script_digest()),
            "012041a413bf0e5302fb402d2983a74280a181af2de94de2a40ea447f20398a4",
            "frozen camera script digest drift",
        );
    }
}
