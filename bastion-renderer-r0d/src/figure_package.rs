//! BUILD-007A10.3 — D2 reproducible VOX-to-FigurePackage compilation substrate
//! (design §8). Self-contained, golden-vector-checkable core:
//!
//! - §8.2 machine-path normalization: the closed rejection grammar
//!   (`.`/`..`, absolute, drive prefixes, backslashes, trailing dot/space,
//!   Windows reserved names, control chars) with typed failures.
//! - §8.1 `FigurePackageV1`: a content-addressed package — fixed header,
//!   canonical-CBOR manifest, `SectionTagV1`-sorted section table, raw section
//!   bytes, trailing package SHA-256. Uncompressed by construction.
//! - §8.4 deterministic greedy voxel meshing: frozen visit order producing
//!   quads whose bytes are invariant to worker count and partition-completion
//!   order (the gather-sort-commit property golden-tested here in Rust rather
//!   than imported from oneTBB).
//!
//! The full §8.8 reproducibility matrix (multi-OS clean builds, temp roots) is
//! the CI/integration surface; the worker-order independence it certifies is
//! proven at the unit level by [`tests::partition_order_does_not_change_bytes`].

use crate::cbor::{CborValue, int_map};
use sha2::{Digest, Sha256};

// ----------------------------------------------------------------------------
// §8.2 machine-path normalization
// ----------------------------------------------------------------------------

/// Typed path-normalization failures (§8.2). A malformed source path is never
/// silently repaired — it terminates asset admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathError {
    Empty,
    Backslash,
    DrivePrefix,
    AbsolutePath,
    DotSegment,
    TrailingDotOrSpace,
    ControlChar,
    NonAsciiOrUppercase,
    WindowsReservedName { name: String },
}

const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Normalize a repository-relative source path to a canonical machine path
/// (§8.2): slash-separated, lowercase ASCII, no `.`/`..`, no absolute/drive/
/// backslash forms, no trailing dot/space, no control chars, no Windows
/// reserved segment names. Returns the accepted path unchanged (it must already
/// be canonical) or a typed [`PathError`].
pub fn normalize_machine_path(raw: &str) -> Result<String, PathError> {
    if raw.is_empty() {
        return Err(PathError::Empty);
    }
    if raw.contains('\\') {
        return Err(PathError::Backslash);
    }
    // Drive prefix like `c:` anywhere is rejected; catch the classic leading form.
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(PathError::DrivePrefix);
    }
    if raw.starts_with('/') {
        return Err(PathError::AbsolutePath);
    }
    for &b in bytes {
        if b < 0x20 || b == 0x7f {
            return Err(PathError::ControlChar);
        }
        if b >= 0x80 || b.is_ascii_uppercase() {
            return Err(PathError::NonAsciiOrUppercase);
        }
    }
    for seg in raw.split('/') {
        if seg.is_empty() {
            // Leading/trailing/double slash yields an empty segment.
            return Err(PathError::DotSegment);
        }
        if seg == "." || seg == ".." {
            return Err(PathError::DotSegment);
        }
        if seg.ends_with('.') || seg.ends_with(' ') {
            return Err(PathError::TrailingDotOrSpace);
        }
        // Windows reserved name check applies to the segment stem (before first dot).
        let stem = seg.split('.').next().unwrap_or(seg);
        if WINDOWS_RESERVED.contains(&stem) {
            return Err(PathError::WindowsReservedName {
                name: stem.to_string(),
            });
        }
    }
    Ok(raw.to_string())
}

/// A normalized source file (§8.2): canonical path + content digest. The canonical
/// sort is by normalized path, then by source SHA-256.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFileV1 {
    pub path: String,
    pub sha256: [u8; 32],
}

/// Sort source files by the frozen §8.2 key: normalized path, then source digest.
pub fn sort_source_files(mut files: Vec<SourceFileV1>) -> Vec<SourceFileV1> {
    files.sort_by(|a, b| a.path.cmp(&b.path).then(a.sha256.cmp(&b.sha256)));
    files
}

// ----------------------------------------------------------------------------
// §8.1 FigurePackageV1 content-addressed package format
// ----------------------------------------------------------------------------

/// Package section tag (§8.1). The section table is sorted by this value, so a
/// package's bytes never depend on section insertion order.
pub type SectionTagV1 = u16;

/// One package section: its tag, media type, and raw uncompressed bytes.
#[derive(Clone, Debug)]
pub struct SectionV1 {
    pub tag: SectionTagV1,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

/// OCI-style content descriptor (§8.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentDescriptorV1 {
    pub media_type: String,
    pub sha256: [u8; 32],
    pub size: u64,
    pub section_tag: SectionTagV1,
}

/// Fixed package header magic (§8.1). `FigurePackageV1`, version 1.
const PKG_MAGIC: &[u8; 8] = b"BSTRFP1\0";
const PKG_VERSION: u32 = 1;

/// A `FigurePackageV1` builder. `canonical_bytes` produces the uncompressed
/// content-addressed package; `package_sha256` is the trailing digest.
#[derive(Clone, Debug, Default)]
pub struct FigurePackageV1 {
    sections: Vec<SectionV1>,
}

impl FigurePackageV1 {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a section. Duplicate tags are the caller's error to avoid; the
    /// canonical sort is by tag and would make duplicates ambiguous, so we keep
    /// the last write per tag deterministic by tag-then-insertion stability.
    pub fn with_section(mut self, tag: SectionTagV1, media_type: &str, bytes: Vec<u8>) -> Self {
        self.sections.push(SectionV1 {
            tag,
            media_type: media_type.to_string(),
            bytes,
        });
        self
    }

    /// Descriptors sorted by section tag (§8.1).
    fn sorted_descriptors(&self) -> Vec<(ContentDescriptorV1, &[u8])> {
        let mut d: Vec<(ContentDescriptorV1, &[u8])> = self
            .sections
            .iter()
            .map(|s| {
                (
                    ContentDescriptorV1 {
                        media_type: s.media_type.clone(),
                        sha256: Sha256::digest(&s.bytes).into(),
                        size: s.bytes.len() as u64,
                        section_tag: s.tag,
                    },
                    s.bytes.as_slice(),
                )
            })
            .collect();
        d.sort_by_key(|(desc, _)| desc.section_tag);
        d
    }

    /// The canonical-CBOR `FigurePackageManifestV1`: schema + the sorted
    /// descriptor table (media type, digest, size, tag) as an integer-keyed map.
    fn manifest_cbor(&self, descriptors: &[(ContentDescriptorV1, &[u8])]) -> Vec<u8> {
        let descs: Vec<CborValue> = descriptors
            .iter()
            .map(|(d, _)| {
                int_map(vec![
                    (0, CborValue::Uint(u64::from(d.section_tag))),
                    (1, CborValue::Text(d.media_type.clone())),
                    (2, CborValue::Bytes(d.sha256.to_vec())),
                    (3, CborValue::Uint(d.size)),
                ])
            })
            .collect();
        int_map(vec![
            (0, CborValue::Uint(u64::from(PKG_VERSION))),
            (1, CborValue::Array(descs)),
        ])
        .to_bytes()
    }

    /// The full canonical package bytes, package SHA-256 appended (§8.1):
    /// `magic || version_le || manifest_len_le || manifest || section_count_le
    /// || raw section bytes (tag order) || package_sha256`.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let descriptors = self.sorted_descriptors();
        let manifest = self.manifest_cbor(&descriptors);
        let mut out = Vec::new();
        out.extend_from_slice(PKG_MAGIC);
        out.extend_from_slice(&PKG_VERSION.to_le_bytes());
        out.extend_from_slice(&(manifest.len() as u32).to_le_bytes());
        out.extend_from_slice(&manifest);
        out.extend_from_slice(&(descriptors.len() as u32).to_le_bytes());
        for (_, bytes) in &descriptors {
            out.extend_from_slice(bytes);
        }
        let digest = Sha256::digest(&out);
        out.extend_from_slice(&digest);
        out
    }

    /// The trailing package digest (§8.1) — the content address.
    #[must_use]
    pub fn package_sha256(&self) -> [u8; 32] {
        let full = self.canonical_bytes();
        let mut d = [0u8; 32];
        d.copy_from_slice(&full[full.len() - 32..]);
        d
    }
}

// ----------------------------------------------------------------------------
// §8.4 deterministic greedy voxel meshing
// ----------------------------------------------------------------------------

/// A dense voxel volume. `mat[index(x,y,z)]` is the material id; `0` is empty.
/// Indexing is `x + y*sx + z*sx*sy` so iterating `x` fastest walks the frozen
/// `(z, y, x)` source-voxel order (§8.3 item 6).
#[derive(Clone, Debug)]
pub struct VoxelVolumeV1 {
    pub dims: [u32; 3],
    pub mat: Vec<u16>,
}

impl VoxelVolumeV1 {
    #[must_use]
    pub fn new(dims: [u32; 3]) -> Self {
        Self {
            mat: vec![0; (dims[0] * dims[1] * dims[2]) as usize],
            dims,
        }
    }

    fn idx(&self, c: [u32; 3]) -> usize {
        (c[0] + c[1] * self.dims[0] + c[2] * self.dims[0] * self.dims[1]) as usize
    }

    pub fn set(&mut self, c: [u32; 3], m: u16) {
        let i = self.idx(c);
        self.mat[i] = m;
    }

    /// Material at `c`, or `0` (empty) if out of bounds.
    fn at(&self, c: [i64; 3]) -> u16 {
        for a in 0..3 {
            if c[a] < 0 || c[a] >= i64::from(self.dims[a]) {
                return 0;
            }
        }
        self.mat[self.idx([c[0] as u32, c[1] as u32, c[2] as u32])]
    }
}

/// A merged greedy quad (§8.4). `dir_tag` is the frozen face-direction index;
/// `slice` is the coordinate along the face axis; `(u0,v0)` is the frozen
/// in-plane origin; `(du,dv)` the merged extent; `material` the surface id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuadV1 {
    pub dir_tag: u8,
    pub slice: u32,
    pub u0: u32,
    pub v0: u32,
    pub du: u32,
    pub dv: u32,
    pub material: u16,
}

/// Frozen face-direction order (§8.4 step 2): `-X,+X,-Y,+Y,-Z,+Z`.
const FACE_DIRS: [(usize, i64); 6] = [(0, -1), (0, 1), (1, -1), (1, 1), (2, -1), (2, 1)];

fn dir_tag(axis: usize, sign: i64) -> u8 {
    (axis as u8) * 2 + u8::from(sign > 0)
}

/// The in-plane `(u, v)` axes for a face whose normal is along `axis` (frozen).
fn uv_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// Greedy-mesh one face direction into quads (§8.4 steps 1-8), in the frozen
/// slice / row-major-mask / first-unconsumed / width-then-height order. This is
/// one independently-constructible partition; the caller assembles partitions by
/// the total order.
fn mesh_direction(vol: &VoxelVolumeV1, axis: usize, sign: i64) -> Vec<QuadV1> {
    let (ua, va) = uv_axes(axis);
    let nu = vol.dims[ua];
    let nv = vol.dims[va];
    let mut quads = Vec::new();
    for slice in 0..vol.dims[axis] {
        // Build the visible-face mask (row-major: v outer, u inner).
        let mut mask = vec![0u16; (nu * nv) as usize];
        for iv in 0..nv {
            for iu in 0..nu {
                let mut here = [0i64; 3];
                here[axis] = i64::from(slice);
                here[ua] = i64::from(iu);
                here[va] = i64::from(iv);
                let m = vol.at(here);
                if m == 0 {
                    continue;
                }
                let mut nb = here;
                nb[axis] += sign;
                if vol.at(nb) == 0 {
                    mask[(iv * nu + iu) as usize] = m;
                }
            }
        }
        // Greedy-merge: first unconsumed cell, extend width then height.
        let mut consumed = vec![false; (nu * nv) as usize];
        for iv in 0..nv {
            for iu in 0..nu {
                let base = (iv * nu + iu) as usize;
                if consumed[base] || mask[base] == 0 {
                    continue;
                }
                let m = mask[base];
                let mut w = 1;
                while iu + w < nu {
                    let j = (iv * nu + iu + w) as usize;
                    if consumed[j] || mask[j] != m {
                        break;
                    }
                    w += 1;
                }
                let mut h = 1;
                'height: while iv + h < nv {
                    for k in 0..w {
                        let j = ((iv + h) * nu + iu + k) as usize;
                        if consumed[j] || mask[j] != m {
                            break 'height;
                        }
                    }
                    h += 1;
                }
                for dv in 0..h {
                    for du in 0..w {
                        consumed[((iv + dv) * nu + iu + du) as usize] = true;
                    }
                }
                quads.push(QuadV1 {
                    dir_tag: dir_tag(axis, sign),
                    slice,
                    u0: iu,
                    v0: iv,
                    du: w,
                    dv: h,
                    material: m,
                });
            }
        }
    }
    quads
}

/// Deterministic greedy mesh (§8.4): all six directions concatenated in the
/// frozen total order. Byte-identical regardless of how the six partitions are
/// scheduled (see [`assemble_partitions`]).
#[must_use]
pub fn greedy_mesh(vol: &VoxelVolumeV1) -> Vec<QuadV1> {
    let mut out = Vec::new();
    for (axis, sign) in FACE_DIRS {
        out.extend(mesh_direction(vol, axis, sign));
    }
    out
}

/// Gather-sort-commit assembly (§8.4 step 9 / §8.9): given per-direction
/// partitions produced in ANY order, sort by direction tag and concatenate.
/// Worker count and completion order cannot affect the result.
#[must_use]
pub fn assemble_partitions(mut partitions: Vec<Vec<QuadV1>>) -> Vec<QuadV1> {
    partitions.sort_by_key(|p| p.first().map_or(u8::MAX, |q| q.dir_tag));
    partitions.into_iter().flatten().collect()
}

/// Length-framed canonical serialization of a quad stream, for content-address
/// digesting and golden vectors.
#[must_use]
pub fn serialize_quads(quads: &[QuadV1]) -> Vec<u8> {
    let mut b = Vec::with_capacity(quads.len() * 23);
    b.extend_from_slice(&(quads.len() as u64).to_le_bytes());
    for q in quads {
        b.push(q.dir_tag);
        b.extend_from_slice(&q.slice.to_le_bytes());
        b.extend_from_slice(&q.u0.to_le_bytes());
        b.extend_from_slice(&q.v0.to_le_bytes());
        b.extend_from_slice(&q.du.to_le_bytes());
        b.extend_from_slice(&q.dv.to_le_bytes());
        b.extend_from_slice(&q.material.to_le_bytes());
    }
    b
}

/// The mesh content address: SHA-256 over the canonical quad serialization.
#[must_use]
pub fn mesh_digest(quads: &[QuadV1]) -> [u8; 32] {
    Sha256::digest(serialize_quads(quads)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    // -------- §8.2 path normalization --------

    #[test]
    fn accepts_clean_machine_path() {
        assert_eq!(
            normalize_machine_path("voxel/humanoid/head.vox").unwrap(),
            "voxel/humanoid/head.vox"
        );
    }

    #[test]
    fn rejects_each_malformed_form() {
        assert_eq!(normalize_machine_path(""), Err(PathError::Empty));
        assert_eq!(normalize_machine_path("a\\b"), Err(PathError::Backslash));
        assert_eq!(normalize_machine_path("c:/a"), Err(PathError::DrivePrefix));
        assert_eq!(normalize_machine_path("/etc/x"), Err(PathError::AbsolutePath));
        assert_eq!(normalize_machine_path("a/../b"), Err(PathError::DotSegment));
        assert_eq!(normalize_machine_path("a/./b"), Err(PathError::DotSegment));
        assert_eq!(normalize_machine_path("a//b"), Err(PathError::DotSegment));
        assert_eq!(normalize_machine_path("a/b."), Err(PathError::TrailingDotOrSpace));
        assert_eq!(normalize_machine_path("a/b "), Err(PathError::TrailingDotOrSpace));
        assert_eq!(normalize_machine_path("a/\x01b"), Err(PathError::ControlChar));
        assert_eq!(normalize_machine_path("A/b"), Err(PathError::NonAsciiOrUppercase));
        assert_eq!(
            normalize_machine_path("a/con/b"),
            Err(PathError::WindowsReservedName { name: "con".to_string() })
        );
        assert_eq!(
            normalize_machine_path("a/nul.vox"),
            Err(PathError::WindowsReservedName { name: "nul".to_string() })
        );
    }

    #[test]
    fn source_files_sort_by_path_then_digest() {
        let f = |p: &str, d: u8| SourceFileV1 { path: p.to_string(), sha256: [d; 32] };
        let out = sort_source_files(vec![f("b", 1), f("a", 9), f("a", 1)]);
        let got: Vec<_> = out.iter().map(|x| (x.path.as_str(), x.sha256[0])).collect();
        assert_eq!(got, vec![("a", 1), ("a", 9), ("b", 1)]);
    }

    // -------- §8.1 package format --------

    #[test]
    fn package_section_table_is_tag_sorted_and_address_stable() {
        // Insert sections OUT of tag order; the package bytes must be identical
        // to the tag-sorted build => insertion order cannot leak into the address.
        let a = FigurePackageV1::new()
            .with_section(7, "application/vnd.bastion.mesh", b"mesh-bytes".to_vec())
            .with_section(2, "application/vnd.bastion.skel", b"skel".to_vec());
        let b = FigurePackageV1::new()
            .with_section(2, "application/vnd.bastion.skel", b"skel".to_vec())
            .with_section(7, "application/vnd.bastion.mesh", b"mesh-bytes".to_vec());
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(a.package_sha256(), b.package_sha256());
    }

    #[test]
    fn frozen_package_address() {
        let pkg = FigurePackageV1::new()
            .with_section(1, "application/vnd.bastion.mesh", b"AAAA".to_vec());
        // Golden content address of the minimal single-section package. A second
        // independent implementation of §8.1 must reproduce this exact digest.
        assert_eq!(
            hex_bytes(&pkg.package_sha256()),
            "856e131c1f789ca014032137214c0fb5966632cf046ae673b6fcca9fa2b1d9e6",
            "frozen package address drift"
        );
    }

    // -------- §8.4 greedy meshing --------

    fn solid(dims: [u32; 3], m: u16) -> VoxelVolumeV1 {
        let mut v = VoxelVolumeV1::new(dims);
        for z in 0..dims[2] {
            for y in 0..dims[1] {
                for x in 0..dims[0] {
                    v.set([x, y, z], m);
                }
            }
        }
        v
    }

    #[test]
    fn solid_cube_meshes_to_six_merged_quads() {
        // A solid NxNxN block has exactly one merged quad per face direction.
        let q = greedy_mesh(&solid([2, 2, 2], 1));
        assert_eq!(q.len(), 6);
        for quad in &q {
            assert_eq!((quad.du, quad.dv), (2, 2), "each face merges to one 2x2 quad");
        }
        // Directions appear in the frozen order -X,+X,-Y,+Y,-Z,+Z.
        let tags: Vec<u8> = q.iter().map(|x| x.dir_tag).collect();
        assert_eq!(tags, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn single_voxel_has_six_unit_faces() {
        let q = greedy_mesh(&solid([1, 1, 1], 5));
        assert_eq!(q.len(), 6);
        assert!(q.iter().all(|x| x.du == 1 && x.dv == 1 && x.material == 5));
    }

    #[test]
    fn frozen_mesh_digest_domino() {
        // A 2x1x1 domino: the ±Y and ±Z faces span 2 along X and merge; the ±X
        // faces are unit. Frozen digest pins the exact quad stream.
        let q = greedy_mesh(&solid([2, 1, 1], 3));
        assert_eq!(
            hex_bytes(&mesh_digest(&q)),
            "106d4ec284b90d66024151dc8f0991805c0cf898faaf0612a8646f842034c2d0",
            "frozen domino mesh digest drift",
        );
    }

    #[test]
    fn partition_order_does_not_change_bytes() {
        // The gather-sort-commit property: meshing the six directions as
        // independent partitions and assembling them in REVERSED completion
        // order yields byte-identical output to the in-order mesh. Worker count
        // / completion order cannot affect the mesh address.
        let vol = solid([3, 2, 2], 1);
        let in_order = greedy_mesh(&vol);
        let mut partitions: Vec<Vec<QuadV1>> = FACE_DIRS
            .iter()
            .map(|&(axis, sign)| mesh_direction(&vol, axis, sign))
            .collect();
        partitions.reverse(); // simulate out-of-order worker completion
        let assembled = assemble_partitions(partitions);
        assert_eq!(mesh_digest(&in_order), mesh_digest(&assembled));
    }
}
