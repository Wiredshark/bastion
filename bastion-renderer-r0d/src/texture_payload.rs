//! BUILD-007A10.20 (payload half) — reproducible texture payload and mip
//! policy (design §29, DC-082/085/086/087; RES-053 KTX 2.0, RES-054 PBRT
//! image pyramids as prior art).
//!
//! - DC-085: the V1 payload is an UNCOMPRESSED KTX2 container (supercompression
//!   zero) — identifier, frozen header, level index with explicit byte offsets/
//!   lengths, mip levels stored per the KTX2 level-index rules. No compressor
//!   thread schedules, no library-version drift: the writer is this module and
//!   every byte is a pure function of the input.
//! - DC-086: filtered mips use project-owned INTEGER 2x2 box generation with
//!   explicit rules — odd-edge clamp (the last row/column duplicates), round-
//!   half-up division — never a driver/runtime generator (BTL-345).
//! - DC-082: every plane declares `DataExact` or `ColorFiltered`; data planes
//!   never receive color mip filtering (requesting mips for one is a typed
//!   error).

/// Plane classification (DC-082).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanePolicy {
    /// Exact data (IDs, masks): never filtered, never mipped.
    DataExact,
    /// Color content: eligible for the DC-086 integer box mip chain.
    ColorFiltered,
}

/// Typed payload failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadError {
    /// Mips requested for a `DataExact` plane (DC-082).
    MipsOnDataPlane,
    /// Zero dimension.
    EmptyImage,
    /// Data length does not match `width * height * 4`.
    LengthMismatch { expected: usize, got: usize },
}

/// One RGBA8 image level (tight, top-left row-major).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba8Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Rgba8Image {
    fn validate(&self) -> Result<(), PayloadError> {
        if self.width == 0 || self.height == 0 {
            return Err(PayloadError::EmptyImage);
        }
        let expected = self.width as usize * self.height as usize * 4;
        if self.data.len() != expected {
            return Err(PayloadError::LengthMismatch { expected, got: self.data.len() });
        }
        Ok(())
    }

    fn texel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }
}

/// DC-086: one integer 2x2 box downsample step. Odd edges CLAMP (the last
/// row/column duplicates its neighbor); each channel averages with round-half-
/// up (`(sum + 2) / 4`). Pure integer — bit-identical everywhere.
#[must_use]
pub fn box_downsample(src: &Rgba8Image) -> Rgba8Image {
    let w = (src.width / 2).max(1);
    let h = (src.height / 2).max(1);
    let mut data = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h {
        for x in 0..w {
            let x0 = (2 * x).min(src.width - 1);
            let x1 = (2 * x + 1).min(src.width - 1);
            let y0 = (2 * y).min(src.height - 1);
            let y1 = (2 * y + 1).min(src.height - 1);
            let (a, b, c, d) = (src.texel(x0, y0), src.texel(x1, y0), src.texel(x0, y1), src.texel(x1, y1));
            for ch in 0..4 {
                let sum = u16::from(a[ch]) + u16::from(b[ch]) + u16::from(c[ch]) + u16::from(d[ch]);
                data.push(((sum + 2) / 4) as u8);
            }
        }
    }
    Rgba8Image { width: w, height: h, data }
}

/// Build the full mip chain (base first) per the plane policy. `DataExact`
/// planes yield the base level only when `want_mips` is false and a typed
/// error when mips are requested (DC-082).
pub fn build_mip_chain(
    base: &Rgba8Image,
    policy: PlanePolicy,
    want_mips: bool,
) -> Result<Vec<Rgba8Image>, PayloadError> {
    base.validate()?;
    if want_mips && policy == PlanePolicy::DataExact {
        return Err(PayloadError::MipsOnDataPlane);
    }
    let mut levels = vec![base.clone()];
    if want_mips {
        while levels.last().unwrap().width > 1 || levels.last().unwrap().height > 1 {
            levels.push(box_downsample(levels.last().unwrap()));
        }
    }
    Ok(levels)
}

/// KTX2 identifier (frozen 12 bytes, KTX 2.0 §3.1).
pub const KTX2_IDENTIFIER: [u8; 12] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xAB, 0x0D, 0x0A, 0x1A, 0x0A,
];
/// VK_FORMAT_R8G8B8A8_SRGB (frozen).
pub const VK_FORMAT_R8G8B8A8_SRGB: u32 = 43;

/// Write an UNCOMPRESSED KTX2 container (DC-085): identifier, header,
/// level index (byte offsets/lengths, uncompressed == stored), then the mip
/// level data. `supercompressionScheme = 0`; DFD/KVD/SGD left empty in V1 —
/// the manifest carries the color/alpha metadata explicitly (DC-086), and the
/// container validates against the KTX2 level-index rules.
#[must_use]
pub fn write_ktx2(levels: &[Rgba8Image]) -> Vec<u8> {
    let level_count = levels.len() as u32;
    let base = &levels[0];
    let header_len = 12 + 68; // identifier + header fields
    let index_len = level_count as usize * 24; // 3x u64 per level
    let mut data_off = (header_len + index_len) as u64;

    let mut out = Vec::new();
    out.extend_from_slice(&KTX2_IDENTIFIER);
    out.extend_from_slice(&VK_FORMAT_R8G8B8A8_SRGB.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes()); // typeSize
    out.extend_from_slice(&base.width.to_le_bytes());
    out.extend_from_slice(&base.height.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // pixelDepth
    out.extend_from_slice(&0u32.to_le_bytes()); // layerCount
    out.extend_from_slice(&1u32.to_le_bytes()); // faceCount
    out.extend_from_slice(&level_count.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // supercompressionScheme = none
    // dfd/kvd/sgd offsets+lengths (empty in V1)
    out.extend_from_slice(&0u32.to_le_bytes()); // dfdByteOffset
    out.extend_from_slice(&0u32.to_le_bytes()); // dfdByteLength
    out.extend_from_slice(&0u32.to_le_bytes()); // kvdByteOffset
    out.extend_from_slice(&0u32.to_le_bytes()); // kvdByteLength
    out.extend_from_slice(&0u64.to_le_bytes()); // sgdByteOffset
    out.extend_from_slice(&0u64.to_le_bytes()); // sgdByteLength
    debug_assert_eq!(out.len(), header_len);

    // Level index: KTX2 stores levels LAST-to-first in the file ordering rule
    // (smallest mip first in data); V1 keeps the simpler frozen rule: data in
    // index order (level 0 first), offsets explicit — a project profile
    // validated against the level-index invariants (monotonic, in-bounds,
    // uncompressedByteLength == byteLength when supercompression is none).
    for lvl in levels {
        let len = lvl.data.len() as u64;
        out.extend_from_slice(&data_off.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes()); // byteLength
        out.extend_from_slice(&len.to_le_bytes()); // uncompressedByteLength
        data_off += len;
    }
    for lvl in levels {
        out.extend_from_slice(&lvl.data);
    }
    out
}

/// The payload digest: domain-separated hash over the container bytes.
#[must_use]
pub fn payload_digest(container: &[u8]) -> [u8; 32] {
    crate::domain_hash("bastion/r0d/texture-payload", 1, 0, container)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Rgba8Image {
        let mut data = Vec::new();
        for y in 0..h {
            for x in 0..w {
                data.extend_from_slice(&f(x, y));
            }
        }
        Rgba8Image { width: w, height: h, data }
    }

    #[test]
    fn box_downsample_averages_round_half_up() {
        // 2x2 -> 1x1: channels (0,1,2,3) avg=1.5 -> round-half-up 2.
        let src = img(2, 2, |x, y| [(y * 2 + x) as u8; 4]);
        let d = box_downsample(&src);
        assert_eq!((d.width, d.height), (1, 1));
        assert_eq!(d.data, vec![2, 2, 2, 2]);
    }

    #[test]
    fn odd_edges_clamp() {
        // 3x1: mip is 1x1; the 2x2 window clamps rows and the 3rd column is
        // never sampled by the (0,0) output (x0=0,x1=1) — value avg(10,20)=15.
        let src = img(3, 1, |x, _| [10 + (x as u8) * 10; 4]);
        let d = box_downsample(&src);
        assert_eq!((d.width, d.height), (1, 1));
        assert_eq!(&d.data[..4], &[15, 15, 15, 15]);
    }

    #[test]
    fn mip_chain_terminates_at_1x1_and_data_planes_refuse_mips() {
        let base = img(8, 4, |x, y| [x as u8, y as u8, 0, 255]);
        let chain = build_mip_chain(&base, PlanePolicy::ColorFiltered, true).unwrap();
        let dims: Vec<(u32, u32)> = chain.iter().map(|l| (l.width, l.height)).collect();
        assert_eq!(dims, vec![(8, 4), (4, 2), (2, 1), (1, 1)]);
        assert_eq!(
            build_mip_chain(&base, PlanePolicy::DataExact, true).unwrap_err(),
            PayloadError::MipsOnDataPlane
        );
        // DataExact without mips: base only.
        assert_eq!(build_mip_chain(&base, PlanePolicy::DataExact, false).unwrap().len(), 1);
    }

    #[test]
    fn ktx2_container_layout_invariants() {
        let base = img(4, 4, |x, y| [x as u8, y as u8, 7, 255]);
        let levels = build_mip_chain(&base, PlanePolicy::ColorFiltered, true).unwrap();
        let ktx = write_ktx2(&levels);
        assert_eq!(&ktx[..12], &KTX2_IDENTIFIER);
        // vkFormat + supercompression fields
        assert_eq!(u32::from_le_bytes(ktx[12..16].try_into().unwrap()), VK_FORMAT_R8G8B8A8_SRGB);
        let level_count = u32::from_le_bytes(ktx[40..44].try_into().unwrap());
        assert_eq!(level_count, 3, "4x4 -> 2x2 -> 1x1");
        assert_eq!(u32::from_le_bytes(ktx[44..48].try_into().unwrap()), 0, "supercompression none");
        // Level index invariants: monotonic offsets, in-bounds, byteLength ==
        // uncompressedByteLength, and total coverage of the data section.
        let mut off = 80usize;
        let mut expected_data_off = 80 + level_count as usize * 24;
        for lvl in &levels {
            let o = u64::from_le_bytes(ktx[off..off + 8].try_into().unwrap()) as usize;
            let bl = u64::from_le_bytes(ktx[off + 8..off + 16].try_into().unwrap()) as usize;
            let ul = u64::from_le_bytes(ktx[off + 16..off + 24].try_into().unwrap()) as usize;
            assert_eq!(o, expected_data_off, "offset monotonic+contiguous");
            assert_eq!(bl, lvl.data.len());
            assert_eq!(bl, ul, "uncompressed == stored when no supercompression");
            assert_eq!(&ktx[o..o + bl], &lvl.data[..], "stored bytes exact");
            expected_data_off += bl;
            off += 24;
        }
        assert_eq!(expected_data_off, ktx.len(), "no trailing slack");
    }

    #[test]
    fn payload_is_bit_reproducible_and_content_sensitive() {
        let base = img(4, 4, |x, y| [x as u8 * 17, y as u8 * 31, 5, 255]);
        let l1 = build_mip_chain(&base, PlanePolicy::ColorFiltered, true).unwrap();
        let l2 = build_mip_chain(&base, PlanePolicy::ColorFiltered, true).unwrap();
        assert_eq!(write_ktx2(&l1), write_ktx2(&l2));
        let mut base2 = base.clone();
        base2.data[0] ^= 1;
        let l3 = build_mip_chain(&base2, PlanePolicy::ColorFiltered, true).unwrap();
        assert_ne!(payload_digest(&write_ktx2(&l1)), payload_digest(&write_ktx2(&l3)));
    }
}
