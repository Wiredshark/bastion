//! BUILD-007A10.7 (part 2) — D4/D5 structural visual oracle (design §17) and
//! deterministic shadow-atlas placement (§18.1).
//!
//! CPU selection is primary structural truth; GPU captures corroborate it. The
//! self-contained substrate here is: the opaque draw-ID side table (§17.2), the
//! WPT-style explicit fuzzy image comparator (§17.5), the environment-tuple
//! digest that scopes exact-pixel certification (§17.4), and the timing-free
//! shadow-atlas tile assignment (§18.1). The GPU capture itself (R32Uint
//! target, warm RGBA readback) is the integration surface.

/// Invalid / background opaque ID (§17.2): valid draw IDs are `1..=N`.
pub const OPAQUE_ID_INVALID: u32 = 0xFFFF_FFFF;

/// Side-table entry for an opaque draw ID (§17.2). The ID is a positional
/// authority, mapped here to the full semantic identity — no hash truncation is
/// ever ID authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpaqueIdEntryV1 {
    pub entity_digest: [u8; 32],
    pub pass_tag: u16,
    pub submesh_index: u32,
    pub material_tag: u16,
}

/// Assign opaque draw IDs by sorted structural draw order (§17.2). Input MUST
/// be pre-sorted by the canonical draw key; IDs are `1..=N` contiguous, and the
/// side table maps each back to full identity. Index `i` holds ID `i+1`.
#[must_use]
pub fn assign_opaque_ids(sorted_draws: &[OpaqueIdEntryV1]) -> Vec<OpaqueIdEntryV1> {
    sorted_draws.to_vec()
}

/// Look up the semantic identity for an opaque ID from the side table (§17.2).
/// The invalid/background ID and any out-of-range ID return `None`.
#[must_use]
pub fn opaque_id_lookup(table: &[OpaqueIdEntryV1], id: u32) -> Option<&OpaqueIdEntryV1> {
    if id == OPAQUE_ID_INVALID || id == 0 {
        return None;
    }
    table.get((id - 1) as usize)
}

/// Result of a fuzzy image comparison (§17.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FuzzyImageResultV1 {
    pub pass: bool,
    pub changed_pixels: u64,
    pub max_channel_diff: u16,
}

/// WPT-style explicit fuzzy image comparison (§17.5): a pixel is "changed" if
/// any RGBA channel differs by more than `max_channel_diff`; the test passes if
/// the changed-pixel count is within `max_changed_pixels`. There is no
/// universal threshold — the limits are per-test policy. Buffers are
/// tightly-packed RGBA8.
#[must_use]
pub fn fuzzy_image_compare(
    a: &[u8],
    b: &[u8],
    max_channel_diff: u8,
    max_changed_pixels: u64,
) -> FuzzyImageResultV1 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len() % 4, 0);
    let mut changed = 0u64;
    let mut worst = 0u16;
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let mut pixel_changed = false;
        for c in 0..4 {
            let d = pa[c].abs_diff(pb[c]);
            worst = worst.max(u16::from(d));
            if d > max_channel_diff {
                pixel_changed = true;
            }
        }
        if pixel_changed {
            changed += 1;
        }
    }
    FuzzyImageResultV1 {
        pass: changed <= max_changed_pixels,
        changed_pixels: changed,
        max_channel_diff: worst,
    }
}

/// The environment tuple that scopes exact-pixel certification (§17.4). Any
/// change to a member revokes certification. Stored here as pre-hashed member
/// digests; the tuple digest binds them in frozen order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentTupleV1 {
    pub os_build: [u8; 32],
    pub backend: [u8; 32],
    pub adapter: [u8; 32],
    pub driver: [u8; 32],
    pub shader_compiler: [u8; 32],
    pub package_digests: [u8; 32],
    pub features_limits: [u8; 32],
    pub resolution_format: [u8; 32],
}

impl EnvironmentTupleV1 {
    /// Domain-separated environment digest (§17.4).
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut p = Vec::with_capacity(32 * 8);
        for m in [
            &self.os_build,
            &self.backend,
            &self.adapter,
            &self.driver,
            &self.shader_compiler,
            &self.package_digests,
            &self.features_limits,
            &self.resolution_format,
        ] {
            p.extend_from_slice(m);
        }
        crate::domain_hash("bastion/r0d/environment-tuple", 1, 0, &p)
    }
}

/// Shadow-atlas overflow (§18.1): a caster could not be placed. Never silently
/// dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowAtlasOverflow {
    pub caster_rank: u32,
    pub capacity: u32,
}

/// Fixed-size shadow atlas (§18.1): `grid × grid` tiles per page. Sorted caster
/// rank maps directly to `(page, tile)` — no timing-dependent allocator or
/// free-list order can change placement.
#[derive(Clone, Copy, Debug)]
pub struct ShadowAtlasV1 {
    pub grid: u32,
    pub max_pages: u32,
}

impl ShadowAtlasV1 {
    /// Place a caster by its sorted rank (§18.1). Deterministic function of
    /// rank alone; overflow past capacity is a typed error.
    pub fn place(&self, caster_rank: u32) -> Result<(u32, u32), ShadowAtlasOverflow> {
        let tiles_per_page = self.grid * self.grid;
        let capacity = tiles_per_page * self.max_pages;
        if caster_rank >= capacity {
            return Err(ShadowAtlasOverflow {
                caster_rank,
                capacity,
            });
        }
        Ok((caster_rank / tiles_per_page, caster_rank % tiles_per_page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(d: u8, pass: u16) -> OpaqueIdEntryV1 {
        OpaqueIdEntryV1 {
            entity_digest: [d; 32],
            pass_tag: pass,
            submesh_index: 0,
            material_tag: 0,
        }
    }

    #[test]
    fn opaque_ids_are_positional_1_to_n() {
        let table = assign_opaque_ids(&[entry(1, 0), entry(2, 0), entry(3, 0)]);
        assert_eq!(opaque_id_lookup(&table, 1).unwrap().entity_digest, [1; 32]);
        assert_eq!(opaque_id_lookup(&table, 3).unwrap().entity_digest, [3; 32]);
        assert!(opaque_id_lookup(&table, OPAQUE_ID_INVALID).is_none());
        assert!(opaque_id_lookup(&table, 0).is_none());
        assert!(opaque_id_lookup(&table, 4).is_none());
    }

    #[test]
    fn identical_images_pass_with_zero_changes() {
        let img = vec![10u8, 20, 30, 255, 40, 50, 60, 255];
        let r = fuzzy_image_compare(&img, &img, 0, 0);
        assert!(r.pass);
        assert_eq!(r.changed_pixels, 0);
        assert_eq!(r.max_channel_diff, 0);
    }

    #[test]
    fn within_fuzz_tolerance_passes_but_counts_worst() {
        // One channel differs by 2; tolerance 3 => not "changed", but worst=2.
        let a = vec![10u8, 20, 30, 255];
        let b = vec![12u8, 20, 30, 255];
        let r = fuzzy_image_compare(&a, &b, 3, 0);
        assert!(r.pass);
        assert_eq!(r.changed_pixels, 0);
        assert_eq!(r.max_channel_diff, 2);
    }

    #[test]
    fn over_tolerance_and_over_budget_fails() {
        let a = vec![10u8, 20, 30, 255, 10, 20, 30, 255];
        let b = vec![100u8, 20, 30, 255, 200, 20, 30, 255];
        let r = fuzzy_image_compare(&a, &b, 5, 1); // 2 changed pixels > budget 1
        assert!(!r.pass);
        assert_eq!(r.changed_pixels, 2);
        assert_eq!(r.max_channel_diff, 190);
    }

    #[test]
    fn environment_digest_is_member_sensitive() {
        let mut e = EnvironmentTupleV1 {
            os_build: [1; 32],
            backend: [2; 32],
            adapter: [3; 32],
            driver: [4; 32],
            shader_compiler: [5; 32],
            package_digests: [6; 32],
            features_limits: [7; 32],
            resolution_format: [8; 32],
        };
        let d0 = e.digest();
        e.driver = [0xff; 32];
        assert_ne!(d0, e.digest());
    }

    #[test]
    fn shadow_atlas_places_by_rank_and_overflows_typed() {
        let atlas = ShadowAtlasV1 {
            grid: 4,
            max_pages: 2,
        }; // 16 tiles/page, 32 capacity
        assert_eq!(atlas.place(0), Ok((0, 0)));
        assert_eq!(atlas.place(15), Ok((0, 15)));
        assert_eq!(atlas.place(16), Ok((1, 0))); // spills to page 1
        assert_eq!(atlas.place(31), Ok((1, 15)));
        assert_eq!(
            atlas.place(32),
            Err(ShadowAtlasOverflow {
                caster_rank: 32,
                capacity: 32
            })
        );
    }
}
