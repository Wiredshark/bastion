//! BUILD-007A10.7 (part 1) — counter-based cosmetic RNG (design §18.2).
//!
//! Cosmetic effects (particles, dithering, lightning) draw randomness from a
//! FROZEN stateless counter-based generator — Philox4x32-10 — keyed by HKDF
//! owner material, so there is no shared cursor and no consumption-order
//! dependency: any CPU or GPU worker reproduces the same sample by semantic
//! tuple. Authoritative RNG is never consumed here.
//!
//! Philox4x32-10 is implemented exactly to the Random123 definition and proven
//! against its published known-answer vectors, so the sample ABI is
//! deterministic by construction across platforms.

use crate::bootstrap::hkdf_expand;

// Philox4x32 constants (Random123).
const PHILOX_M0: u32 = 0xD251_1F53;
const PHILOX_M1: u32 = 0xCD9E_8D57;
const PHILOX_W0: u32 = 0x9E37_79B9; // golden ratio
const PHILOX_W1: u32 = 0xBB67_AE85; // sqrt(3)-1

/// Philox4x32-10: the frozen stateless sample ABI (§18.2). Ten rounds, key
/// bumped by the Weyl constants between rounds. Byte/word order is little-endian
/// by the two-u32 key / four-u32 counter convention.
#[must_use]
pub fn philox4x32_10(mut ctr: [u32; 4], mut key: [u32; 2]) -> [u32; 4] {
    for r in 0..10 {
        if r > 0 {
            key[0] = key[0].wrapping_add(PHILOX_W0);
            key[1] = key[1].wrapping_add(PHILOX_W1);
        }
        let p0 = u64::from(ctr[0]) * u64::from(PHILOX_M0);
        let p1 = u64::from(ctr[2]) * u64::from(PHILOX_M1);
        let (hi0, lo0) = ((p0 >> 32) as u32, p0 as u32);
        let (hi1, lo1) = ((p1 >> 32) as u32, p1 as u32);
        ctr = [hi1 ^ ctr[1] ^ key[0], lo1, hi0 ^ ctr[3] ^ key[1], lo0];
    }
    ctr
}

/// Frozen HKDF info label for cosmetic owner material (§18.2).
const COSMETIC_LABEL: &[u8] = b"bastion/r0d/cosmetic-philox/v1";

fn read_u32_le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Derive one cosmetic Philox sample by semantic tuple (§18.2). `root_cosmetic_prk`
/// is the HKDF PRK for the cosmetic root; the effect/emitter/instance identity
/// selects the owner material, and the tick/ordinal/purpose select the counter.
/// Direct random access — no shared cursor, no order dependency.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn cosmetic_sample(
    root_cosmetic_prk: &[u8; 32],
    effect_kind_tag: u16,
    emitter_digest: &[u8; 32],
    effect_instance_digest: &[u8; 32],
    spawn_tick: u64,
    spawn_ordinal: u32,
    purpose_lane: u32,
    sample_index: u32,
) -> [u32; 4] {
    // owner_material = HKDF-Expand(prk, frame(label)||effect_kind||emitter||instance, 24)
    let mut info = Vec::with_capacity(2 + COSMETIC_LABEL.len() + 2 + 64);
    info.extend_from_slice(&(COSMETIC_LABEL.len() as u16).to_le_bytes());
    info.extend_from_slice(COSMETIC_LABEL);
    info.extend_from_slice(&effect_kind_tag.to_le_bytes());
    info.extend_from_slice(emitter_digest);
    info.extend_from_slice(effect_instance_digest);
    let owner = hkdf_expand(root_cosmetic_prk, &info, 24);

    let philox_key = [read_u32_le(&owner, 0), read_u32_le(&owner, 4)];
    let base_counter = [
        read_u32_le(&owner, 8),
        read_u32_le(&owner, 12),
        read_u32_le(&owner, 16),
        read_u32_le(&owner, 20),
    ];
    let low = spawn_tick as u32;
    let high = (spawn_tick >> 32) as u32;
    let sample_counter = [
        base_counter[0] ^ low,
        base_counter[1] ^ high,
        base_counter[2] ^ spawn_ordinal,
        base_counter[3] ^ (purpose_lane ^ sample_index),
    ];
    philox4x32_10(sample_counter, philox_key)
}

/// Uniform `[0, 1)` conversion (§18.2): `(sample_u32 >> 8) * 2^-24`. Presentation
/// only unless the exact bits are certified.
#[must_use]
pub fn u01_f32(sample_u32: u32) -> f32 {
    ((sample_u32 >> 8) as f32) * (1.0 / (1u32 << 24) as f32)
}

/// BUILD-007A10.13 — the closed purpose-lane registry for the cosmetic tuple
/// ABI. Lanes are frozen append-only ordinals; an effect using an unregistered
/// lane is a contract violation caught at declaration, not a silent new stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurposeLane {
    SpawnPosition = 0,
    SpawnVelocity = 1,
    Lifetime = 2,
    ColorJitter = 3,
    SizeJitter = 4,
    PhaseOffset = 5,
    DitherPhase = 6,
    LightningFork = 7,
}

impl PurposeLane {
    /// Resolve a numeric lane against the closed registry.
    pub fn from_lane(lane: u32) -> Option<PurposeLane> {
        use PurposeLane::*;
        Some(match lane {
            0 => SpawnPosition,
            1 => SpawnVelocity,
            2 => Lifetime,
            3 => ColorJitter,
            4 => SizeJitter,
            5 => PhaseOffset,
            6 => DitherPhase,
            7 => LightningFork,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // External validation: the two published Random123 kat_vectors.txt entries
    // for philox4x32-10 (all-ones and the pi vector). Matching all four output
    // words on these non-trivial inputs is conclusive proof of correctness.
    // The all-zero case below is a frozen regression vector (Random123's KAT
    // file carries no all-zero philox entry), computed by this proven impl.
    #[test]
    fn philox_frozen_all_zero() {
        assert_eq!(
            philox4x32_10([0, 0, 0, 0], [0, 0]),
            [1713891541, 3781805453, 3159862348, 2600524760]
        );
    }

    #[test]
    fn philox_kat_all_ones() {
        assert_eq!(
            philox4x32_10([0xffff_ffff; 4], [0xffff_ffff; 2]),
            [0x408f_276d, 0x41c8_3b0e, 0xa20b_c7c6, 0x6d54_51fd]
        );
    }

    #[test]
    fn philox_kat_pi_vector() {
        assert_eq!(
            philox4x32_10(
                [0x243f_6a88, 0x85a3_08d3, 0x1319_8a2e, 0x0370_7344],
                [0xa409_3822, 0x299f_31d0]
            ),
            [0xd16c_fe09, 0x94fd_cceb, 0x5001_e420, 0x2412_6ea1]
        );
    }

    #[test]
    fn cosmetic_sample_is_random_access_and_stable() {
        let prk = [0x33u8; 32];
        let em = [0x44u8; 32];
        let inst = [0x55u8; 32];
        let a = cosmetic_sample(&prk, 7, &em, &inst, 100, 0, 0, 0);
        let b = cosmetic_sample(&prk, 7, &em, &inst, 100, 0, 0, 0);
        assert_eq!(a, b, "same tuple => same sample");
        // Neighboring tick is an independent canary.
        let c = cosmetic_sample(&prk, 7, &em, &inst, 101, 0, 0, 0);
        assert_ne!(a, c);
        // Neighboring sample_index too.
        let d = cosmetic_sample(&prk, 7, &em, &inst, 100, 0, 0, 1);
        assert_ne!(a, d);
    }

    #[test]
    fn u01_is_in_unit_interval() {
        assert!((u01_f32(0) - 0.0).abs() < f32::EPSILON);
        assert!(u01_f32(u32::MAX) < 1.0);
        assert!(u01_f32(u32::MAX) > 0.99);
    }

    // ---- BUILD-007A10.13 additions ----

    #[test]
    fn purpose_lane_registry_is_closed() {
        assert_eq!(PurposeLane::from_lane(0), Some(PurposeLane::SpawnPosition));
        assert_eq!(PurposeLane::from_lane(7), Some(PurposeLane::LightningFork));
        assert_eq!(PurposeLane::from_lane(8), None);
    }

    #[test]
    fn frozen_tuple_derivation_vector_for_wgsl_parity() {
        // THE CPU golden vector a WGSL implementation must reproduce exactly
        // before it may be used in deterministic capture (packet A10.13).
        let s = cosmetic_sample(
            &[0x33; 32],
            7,
            &[0x44; 32],
            &[0x55; 32],
            0x1_0000_0064, // >32-bit tick exercises the lo/hi split
            9,
            PurposeLane::ColorJitter as u32,
            2,
        );
        assert_eq!(
            s,
            [0x407400fe, 0x74e841ee, 0x3419c605, 0xe414c054],
            "frozen cosmetic tuple vector drift"
        );
    }

    #[test]
    fn authority_isolation_root_and_labels_are_separate() {
        // The cosmetic PRK is an INDEPENDENT input: the same bytes used as an
        // authority bootstrap root produce unrelated streams because the HKDF
        // info labels differ (bootstrap salt label vs cosmetic-philox label).
        // Cosmetic sampling can never consume an authority seed by construction
        // — there is no code path from SeedRegistryV1 into cosmetic_sample.
        let shared_root = [0x77u8; 32];
        let bootstrap_seed = {
            let reg = crate::bootstrap::SeedRegistryV1::new(
                shared_root,
                vec![crate::bootstrap::SeedDomainDeclarationV1 {
                    domain: "bastion/r0d/worldgen".to_string(),
                    schema_major: 1,
                    schema_minor: 0,
                    owner_digest: [0x22; 32],
                }],
            )
            .unwrap();
            reg.seed("bastion/r0d/worldgen").unwrap()
        };
        let cosmetic = cosmetic_sample(&shared_root, 0, &[0; 32], &[0; 32], 0, 0, 0, 0);
        let cosmetic_bytes: Vec<u8> = cosmetic.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_ne!(bootstrap_seed[..16], cosmetic_bytes[..16], "streams unrelated");
    }

    #[test]
    fn neighboring_input_canaries_all_axes() {
        let base = cosmetic_sample(&[1; 32], 5, &[2; 32], &[3; 32], 100, 4, 1, 0);
        // Every tuple axis independently changes the sample.
        assert_ne!(base, cosmetic_sample(&[9; 32], 5, &[2; 32], &[3; 32], 100, 4, 1, 0), "prk");
        assert_ne!(base, cosmetic_sample(&[1; 32], 6, &[2; 32], &[3; 32], 100, 4, 1, 0), "kind");
        assert_ne!(base, cosmetic_sample(&[1; 32], 5, &[9; 32], &[3; 32], 100, 4, 1, 0), "emitter");
        assert_ne!(base, cosmetic_sample(&[1; 32], 5, &[2; 32], &[9; 32], 100, 4, 1, 0), "instance");
        assert_ne!(base, cosmetic_sample(&[1; 32], 5, &[2; 32], &[3; 32], 101, 4, 1, 0), "tick");
        assert_ne!(base, cosmetic_sample(&[1; 32], 5, &[2; 32], &[3; 32], 100, 5, 1, 0), "ordinal");
        assert_ne!(base, cosmetic_sample(&[1; 32], 5, &[2; 32], &[3; 32], 100, 4, 2, 0), "purpose");
        assert_ne!(base, cosmetic_sample(&[1; 32], 5, &[2; 32], &[3; 32], 100, 4, 1, 1), "index");
    }
}
