//! BUILD-007A10.19 (pure core) — canonical offscreen capture and readback ABI
//! (design §29, DC-078..081, BTL-338..342).
//!
//! - `RendererCaptureDescriptorV1` with FROZEN enums for target kind, format,
//!   channel order, row origin, alpha mode, and transfer function. Capture
//!   identity = the immutable frame token + this descriptor's digest (DC-080);
//!   surface format/size and callback timing are downstream diagnostics.
//! - Tight-row extraction (BTL-339): GPU readback rows are padded to
//!   `COPY_BYTES_PER_ROW_ALIGNMENT` (256); canonical color bytes are the TIGHT
//!   top-left row-major bytes with padding stripped and the length validated —
//!   padded slack can never enter a hash.
//!
//! The live wgpu texture/buffer wiring in `screenshot.rs` is the integration
//! seam; this module is the byte-layout authority, testable with synthetic
//! buffers.

/// wgpu's row-alignment constant, frozen into the V1 ABI (RES-052).
pub const ROW_ALIGN: usize = 256;

/// Frozen capture-target kind (DC-078: explicit offscreen final composite).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureTargetKind {
    OffscreenFinalComposite = 0,
    StructuralIdBuffer = 1,
    DepthBuffer = 2,
}

/// Frozen capture pixel format (DC-078: Rgba8UnormSrgb for color).
/// `Rgb8Srgb` records the CURRENT live screenshot path honestly (BGRA/RGBA →
/// RGB, alpha dropped) until the full DC-079 RGBA readback lands; the V1
/// descriptor never claims a format the bytes don't have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureFormat {
    Rgba8UnormSrgb = 0,
    R32Uint = 1,
    Depth32Float = 2,
    Rgb8Srgb = 3,
}

impl CaptureFormat {
    /// Bytes per pixel (frozen).
    #[must_use]
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            CaptureFormat::Rgba8UnormSrgb | CaptureFormat::R32Uint | CaptureFormat::Depth32Float => 4,
            CaptureFormat::Rgb8Srgb => 3,
        }
    }
}

/// Frozen channel order (DC-079: RGBA, alpha preserved).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelOrder {
    Rgba = 0,
}

/// Frozen row origin (DC-079: top-left row-major).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowOrigin {
    TopLeft = 0,
}

/// Frozen alpha mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaMode {
    PreservedStraight = 0,
    PreservedPremultiplied = 1,
}

/// Frozen transfer function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferFunction {
    Srgb = 0,
    Linear = 1,
}

/// The canonical capture descriptor (DC-078/080). Fixed dimensions, sample
/// count 1 — the surface/present path is downstream only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RendererCaptureDescriptorV1 {
    pub target: CaptureTargetKind,
    pub format: CaptureFormat,
    pub channel_order: ChannelOrder,
    pub row_origin: RowOrigin,
    pub alpha: AlphaMode,
    pub transfer: TransferFunction,
    pub width: u32,
    pub height: u32,
}

impl RendererCaptureDescriptorV1 {
    /// Domain-separated descriptor digest — half of the capture identity
    /// (the other half is the immutable frame token).
    #[must_use]
    pub fn descriptor_digest(&self) -> [u8; 32] {
        let mut p = Vec::with_capacity(16);
        p.push(self.target as u8);
        p.push(self.format as u8);
        p.push(self.channel_order as u8);
        p.push(self.row_origin as u8);
        p.push(self.alpha as u8);
        p.push(self.transfer as u8);
        p.extend_from_slice(&self.width.to_le_bytes());
        p.extend_from_slice(&self.height.to_le_bytes());
        crate::domain_hash("bastion/r0d/capture-descriptor", 1, 0, &p)
    }

    /// The capture identity (DC-080): frame token + descriptor. Callback time
    /// appears nowhere.
    #[must_use]
    pub fn capture_identity(&self, frame_token: u64) -> [u8; 32] {
        let mut p = Vec::with_capacity(40);
        p.extend_from_slice(&frame_token.to_le_bytes());
        p.extend_from_slice(&self.descriptor_digest());
        crate::domain_hash("bastion/r0d/capture-identity", 1, 0, &p)
    }

    /// The padded bytes-per-row a wgpu readback of this descriptor uses.
    #[must_use]
    pub fn padded_bytes_per_row(&self) -> usize {
        let tight = self.width as usize * self.format.bytes_per_pixel();
        tight.div_ceil(ROW_ALIGN) * ROW_ALIGN
    }

    /// The canonical tight byte length.
    #[must_use]
    pub fn tight_len(&self) -> usize {
        self.width as usize * self.height as usize * self.format.bytes_per_pixel()
    }
}

/// Typed extraction failures (BTL-339/341: never silent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtractError {
    /// The padded buffer's length does not match the descriptor's geometry.
    LengthMismatch { expected: usize, got: usize },
}

/// Strip row padding (BTL-339): validate the padded length exactly, then copy
/// the tight top-left row-major bytes. Slack bytes can never reach a hash.
pub fn extract_tight_rows(
    desc: &RendererCaptureDescriptorV1,
    padded: &[u8],
) -> Result<Vec<u8>, ExtractError> {
    let ppr = desc.padded_bytes_per_row();
    let tight_row = desc.width as usize * desc.format.bytes_per_pixel();
    let expected = ppr * desc.height as usize;
    if padded.len() != expected {
        return Err(ExtractError::LengthMismatch { expected, got: padded.len() });
    }
    let mut out = Vec::with_capacity(desc.tight_len());
    for row in 0..desc.height as usize {
        let start = row * ppr;
        out.extend_from_slice(&padded[start..start + tight_row]);
    }
    Ok(out)
}

/// The canonical capture-bytes digest: domain-separated hash over the TIGHT
/// bytes only, bound to the capture identity.
#[must_use]
pub fn capture_bytes_digest(identity: &[u8; 32], tight: &[u8]) -> [u8; 32] {
    let mut p = Vec::with_capacity(32 + tight.len());
    p.extend_from_slice(identity);
    p.extend_from_slice(tight);
    crate::domain_hash("bastion/r0d/capture-bytes", 1, 0, &p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc(w: u32, h: u32) -> RendererCaptureDescriptorV1 {
        RendererCaptureDescriptorV1 {
            target: CaptureTargetKind::OffscreenFinalComposite,
            format: CaptureFormat::Rgba8UnormSrgb,
            channel_order: ChannelOrder::Rgba,
            row_origin: RowOrigin::TopLeft,
            alpha: AlphaMode::PreservedStraight,
            transfer: TransferFunction::Srgb,
            width: w,
            height: h,
        }
    }

    #[test]
    fn padded_row_geometry_matches_wgpu_alignment() {
        // 100px * 4B = 400 tight -> padded to 512.
        assert_eq!(desc(100, 1).padded_bytes_per_row(), 512);
        // Exactly aligned stays exact: 64px * 4B = 256.
        assert_eq!(desc(64, 1).padded_bytes_per_row(), 256);
    }

    #[test]
    fn padding_bytes_cannot_enter_the_digest() {
        let d = desc(100, 2);
        let ppr = d.padded_bytes_per_row();
        // Two buffers identical in tight bytes, WILDLY different in padding.
        let mut a = vec![0u8; ppr * 2];
        let mut b = vec![0xffu8; ppr * 2];
        for row in 0..2 {
            for i in 0..400 {
                a[row * ppr + i] = (row * 31 + i) as u8;
                b[row * ppr + i] = (row * 31 + i) as u8;
            }
        }
        let ta = extract_tight_rows(&d, &a).unwrap();
        let tb = extract_tight_rows(&d, &b).unwrap();
        assert_eq!(ta, tb);
        assert_eq!(ta.len(), d.tight_len());
        let id = d.capture_identity(7);
        assert_eq!(capture_bytes_digest(&id, &ta), capture_bytes_digest(&id, &tb));
    }

    #[test]
    fn wrong_length_is_typed_failure() {
        let d = desc(100, 2);
        let bad = vec![0u8; d.padded_bytes_per_row() * 2 - 1];
        assert_eq!(
            extract_tight_rows(&d, &bad).unwrap_err(),
            ExtractError::LengthMismatch { expected: d.padded_bytes_per_row() * 2, got: d.padded_bytes_per_row() * 2 - 1 }
        );
    }

    #[test]
    fn capture_identity_binds_token_and_descriptor() {
        let d = desc(100, 2);
        assert_ne!(d.capture_identity(1), d.capture_identity(2), "frame token");
        let mut d2 = d;
        d2.transfer = TransferFunction::Linear;
        assert_ne!(d.capture_identity(1), d2.capture_identity(1), "descriptor field");
    }
}
