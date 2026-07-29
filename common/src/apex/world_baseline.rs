//! `APEX-T4.3` chunk 1 — `WorldBaselineManifestV1`: the world-identity
//! anchor RTSim reconciliation must check before adopting a loaded save's
//! world state (`server/src/rtsim/mod.rs::RtSim::new`, right before
//! `break 'load *data` -- confirmed at premise-check time, not guessed).
//!
//! **The gap this closes.** RTSim data carries its own `version`
//! (`rtsim/src/data/mod.rs:39-45`) and reconciles against whatever world
//! the current binary generates. The world seed alone does not pin the
//! *result*: identical seed with altered worldgen, content, or economy
//! math produces a different world that RTSim will nonetheless reconcile
//! against, silently.
//!
//! **Chunk scope, self-sized per this program's own standing discipline**
//! (the `T4.1`/`T4.2` precedent): this chunk is the DATA MODEL and the
//! PURE composite-hash function, fully testable against fixtures alone.
//! Deliberately NOT built here, banked for chunk 2 (orchestrator-approved
//! split): the REAL derivation of [`WorldgenProtocolVersion`]/
//! [`ContentProtocolVersion`]/[`NumericProtocolVersion`] from an honest
//! frozen vocabulary (this row's own ruling: "frozen-vocabulary
//! content-root derivation per `net_envelope_profile_root_v1`'s pattern,
//! never arbitrary integers" -- each vocabulary's reason will be recorded
//! beside its golden once chunk 2 defines it, matching every frozen root
//! before it), the real map-geometry/economy hashers (`common` cannot
//! depend on `world`, so those stay caller-supplied opaque roots here),
//! the live wiring into `RtSim::new`, the `rtsim::data::Data` bridge
//! field (`T4.6-INTERIM`, orchestrator-ruled: a versioned
//! `#[serde(default)]` field on `Data`, since `T4.6`'s durable save
//! manifest -- the row that would naturally own this -- does not exist
//! yet), and recording the root into `T4.4`'s already-built save
//! inventory.
//!
//! **Why `common` cannot depend on `world`'s real types.** `world`
//! depends on `common`, not the reverse -- a dependency this module must
//! respect. [`SiteBaselineEntryV1::kind_tag`] is therefore a caller-
//! supplied numeric tag standing in for `world::site::SiteKind`, and
//! [`WorldBaselineInputV1::map_geometry_root`] /
//! [`WorldBaselineInputV1::economy_root`] are caller-supplied opaque
//! roots standing in for canonical terrain and `Economy` hashes -- chunk
//! 2 (living in `world`/`server`, where those real types are visible)
//! computes them and calls this pure function.

use crate::apex::digest::{DigestBytes32V1, DigestDomainIdV1, DigestErrorV1, ProtocolDigestV1, digest_canonical_bytes_v1};
use crate::apex::subsystem::descriptor::{ContentProtocolVersion, NumericProtocolVersion, WorldgenProtocolVersion};

/// One site's identity-relevant shape, for the "site identity, the site
/// origin/kind graph" component of the baseline. `neighbor_site_ids`
/// carries the GRAPH's edges (not just the node), same shape as
/// `Civs::neighbors` (`E11-3b`) -- sorted inside
/// [`compute_world_baseline_root_v1`], never by the caller, for the same
/// canonicalize-by-stable-key reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteBaselineEntryV1 {
    /// A stand-in for `Id<Site>` -- `common` cannot depend on `world`'s
    /// slotmap-style `Id<T>` either, so this is the caller's own stable
    /// numeric identity for the site (chunk 2's job to derive honestly;
    /// `Id<Site>` itself is already confirmed stable/never-recycled per
    /// `E11-3b`'s own premise-check, so a direct `.id()` mapping is
    /// plausible there, not guessed here).
    pub site_id: u64,
    pub origin_x: i32,
    pub origin_y: i32,
    pub kind_tag: u16,
    pub neighbor_site_ids: Vec<u64>,
}

/// Everything [`compute_world_baseline_root_v1`] binds and hashes,
/// exactly the spec's own list: "world seed plus worldgen/content/
/// numeric protocol identity, and hash: canonical map geometry, site
/// identity, the site origin/kind graph, and the economic baseline."
#[derive(Clone, Debug, PartialEq)]
pub struct WorldBaselineInputV1 {
    pub world_seed: u32,
    /// `None` means "not yet honestly derivable" -- `T4-PV`'s own scope,
    /// undescribed rather than fabricated, same discipline as `T4.1`'s
    /// own bootstrap manifest leaving un-derived slots absent. `Some`
    /// only once `T4-PV` wires a real frozen-vocabulary derivation.
    pub worldgen: Option<WorldgenProtocolVersion>,
    pub content: Option<ContentProtocolVersion>,
    pub numeric: Option<NumericProtocolVersion>,
    /// Opaque caller-supplied root -- see the module doc's dependency note.
    pub map_geometry_root: DigestBytes32V1,
    /// Canonicalized by [`compute_world_baseline_root_v1`] (sorted by
    /// `site_id`), never by the caller -- the non-vacuity required test
    /// (permuted site ordering must not move the root) is exactly this
    /// property.
    pub sites: Vec<SiteBaselineEntryV1>,
    /// Opaque caller-supplied root -- see the module doc's dependency note.
    pub economy_root: DigestBytes32V1,
}

fn push_u32(buf: &mut Vec<u8>, v: u32) { buf.extend_from_slice(&v.to_be_bytes()); }
fn push_u64(buf: &mut Vec<u8>, v: u64) { buf.extend_from_slice(&v.to_be_bytes()); }
fn push_i32(buf: &mut Vec<u8>, v: i32) { buf.extend_from_slice(&v.to_be_bytes()); }
fn push_u16(buf: &mut Vec<u8>, v: u16) { buf.extend_from_slice(&v.to_be_bytes()); }
fn push_bytes32(buf: &mut Vec<u8>, v: &DigestBytes32V1) { buf.extend_from_slice(v.as_array()); }

/// `Option<u32>` has no bare-bytes representation that can't collide with
/// a real value (`0` is a valid `ProtocolVersion`) -- an explicit 0/1
/// presence marker, same discipline the manifest-value codec uses for
/// `Option<T>` elsewhere in this program.
fn push_option_u32(buf: &mut Vec<u8>, v: Option<u32>) {
    match v {
        Some(x) => {
            buf.push(1);
            push_u32(buf, x);
        },
        None => buf.push(0),
    }
}

/// The frozen preimage encoding -- every field length-prefixed or
/// fixed-width so no two distinct inputs can ever produce the same
/// bytes (the classic concatenation collision, same discipline as
/// `NumericProfileV1::id_v1`).
fn world_baseline_preimage_v1(input: &WorldBaselineInputV1) -> Vec<u8> {
    let mut buf = Vec::new();
    push_u32(&mut buf, input.world_seed);
    push_option_u32(&mut buf, input.worldgen.map(|w| w.get().get()));
    push_option_u32(&mut buf, input.content.map(|c| c.get().get()));
    push_option_u32(&mut buf, input.numeric.map(|n| n.get().get()));
    push_bytes32(&mut buf, &input.map_geometry_root);

    // Canonicalize by site_id -- an unrelated pre-sort permutation (the
    // order sites happened to be collected in) must never move the root.
    let mut sites: Vec<&SiteBaselineEntryV1> = input.sites.iter().collect();
    sites.sort_unstable_by_key(|s| s.site_id);
    push_u64(&mut buf, sites.len() as u64);
    for site in sites {
        push_u64(&mut buf, site.site_id);
        push_i32(&mut buf, site.origin_x);
        push_i32(&mut buf, site.origin_y);
        push_u16(&mut buf, site.kind_tag);
        let mut neighbors = site.neighbor_site_ids.clone();
        neighbors.sort_unstable();
        push_u64(&mut buf, neighbors.len() as u64);
        for n in neighbors {
            push_u64(&mut buf, n);
        }
    }

    push_bytes32(&mut buf, &input.economy_root);
    buf
}

/// `T4.3`'s "one complete root", domain-separated under the already-
/// reserved `DigestDomainIdV1::WorldBaselineManifest` (=4, frozen since
/// `T0.3`'s registry, never used until now).
pub fn compute_world_baseline_root_v1(input: &WorldBaselineInputV1) -> Result<ProtocolDigestV1, DigestErrorV1> {
    let preimage = world_baseline_preimage_v1(input);
    digest_canonical_bytes_v1(DigestDomainIdV1::WorldBaselineManifest, &preimage, 1 << 24)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::hash_artifact_bytes_v1;
    use crate::apex::scalar::ProtocolVersion;

    fn digest_root(tag: u8) -> DigestBytes32V1 { hash_artifact_bytes_v1(&[tag]).digest.bytes }

    fn site(id: u64, x: i32, y: i32, kind: u16, neighbors: &[u64]) -> SiteBaselineEntryV1 {
        SiteBaselineEntryV1 { site_id: id, origin_x: x, origin_y: y, kind_tag: kind, neighbor_site_ids: neighbors.to_vec() }
    }

    fn baseline() -> WorldBaselineInputV1 {
        WorldBaselineInputV1 {
            world_seed: 12345,
            worldgen: Some(WorldgenProtocolVersion::new(ProtocolVersion::new(1))),
            content: Some(ContentProtocolVersion::new(ProtocolVersion::new(1))),
            numeric: Some(NumericProtocolVersion::new(ProtocolVersion::new(1))),
            map_geometry_root: digest_root(1),
            sites: vec![site(1, 0, 0, 10, &[2]), site(2, 5, 5, 11, &[1])],
            economy_root: digest_root(2),
        }
    }

    #[test]
    fn the_same_input_produces_the_same_root() {
        assert_eq!(compute_world_baseline_root_v1(&baseline()).unwrap(), compute_world_baseline_root_v1(&baseline()).unwrap());
    }

    /// Required test: same seed, altered worldgen protocol identity ->
    /// the root must move.
    #[test]
    fn altered_worldgen_protocol_identity_moves_the_root() {
        let base = baseline();
        let mut altered = base.clone();
        altered.worldgen = Some(WorldgenProtocolVersion::new(ProtocolVersion::new(2)));
        assert_ne!(compute_world_baseline_root_v1(&base).unwrap(), compute_world_baseline_root_v1(&altered).unwrap());
    }

    /// Required test: same seed, altered content protocol identity ->
    /// the root must move.
    #[test]
    fn altered_content_protocol_identity_moves_the_root() {
        let base = baseline();
        let mut altered = base.clone();
        altered.content = Some(ContentProtocolVersion::new(ProtocolVersion::new(2)));
        assert_ne!(compute_world_baseline_root_v1(&base).unwrap(), compute_world_baseline_root_v1(&altered).unwrap());
    }

    /// Required test: same seed, altered economy math (the caller-
    /// supplied economy root stands in for "the economic baseline
    /// changed") -> the root must move.
    #[test]
    fn altered_economy_baseline_moves_the_root() {
        let base = baseline();
        let mut altered = base.clone();
        altered.economy_root = digest_root(99);
        assert_ne!(compute_world_baseline_root_v1(&base).unwrap(), compute_world_baseline_root_v1(&altered).unwrap());
    }

    /// Altered numeric protocol identity also moves the root -- named
    /// separately from the three explicitly required cases since the
    /// spec lists it as a fourth equal-status input, not incidental.
    #[test]
    fn altered_numeric_protocol_identity_moves_the_root() {
        let base = baseline();
        let mut altered = base.clone();
        altered.numeric = Some(NumericProtocolVersion::new(ProtocolVersion::new(2)));
        assert_ne!(compute_world_baseline_root_v1(&base).unwrap(), compute_world_baseline_root_v1(&altered).unwrap());
    }

    /// Required non-vacuity test: permuted site ordering must NOT change
    /// the root, or the hash is over an iteration order rather than over
    /// the world.
    #[test]
    fn permuted_site_ordering_does_not_move_the_root() {
        let forward = baseline();
        let mut reversed = forward.clone();
        reversed.sites.reverse();
        assert_eq!(compute_world_baseline_root_v1(&forward).unwrap(), compute_world_baseline_root_v1(&reversed).unwrap());
    }

    /// Companion to the ordering test: a genuinely DIFFERENT site set
    /// (not just reordered) DOES move the root -- proves the ordering
    /// test isn't passing because sites are ignored entirely.
    #[test]
    fn a_genuinely_different_site_set_moves_the_root() {
        let base = baseline();
        let mut different = base.clone();
        different.sites.push(site(3, 9, 9, 12, &[]));
        assert_ne!(compute_world_baseline_root_v1(&base).unwrap(), compute_world_baseline_root_v1(&different).unwrap());
    }

    /// A site's own neighbor list is canonicalized the same way -- a
    /// permuted neighbor list must not move the root either.
    #[test]
    fn permuted_neighbor_ordering_does_not_move_the_root() {
        let mut forward = baseline();
        forward.sites = vec![site(1, 0, 0, 10, &[2, 3, 4]), site(2, 5, 5, 11, &[])];
        let mut reordered = forward.clone();
        reordered.sites[0].neighbor_site_ids = vec![4, 2, 3];
        assert_eq!(compute_world_baseline_root_v1(&forward).unwrap(), compute_world_baseline_root_v1(&reordered).unwrap());
    }

    /// Altered map geometry (the caller-supplied geometry root stands in
    /// for "canonical map geometry changed") moves the root -- the
    /// fourth of the spec's named hash inputs.
    #[test]
    fn altered_map_geometry_root_moves_the_root() {
        let base = baseline();
        let mut altered = base.clone();
        altered.map_geometry_root = digest_root(77);
        assert_ne!(compute_world_baseline_root_v1(&base).unwrap(), compute_world_baseline_root_v1(&altered).unwrap());
    }

    /// Same seed, everything else unaltered: the root does NOT move --
    /// positive control proving the previous "altered X moves the root"
    /// assertions are meaningful (not vacuously true because every input
    /// moves the root regardless of content).
    #[test]
    fn an_unrelated_field_change_does_not_falsely_couple_to_the_root() {
        let a = baseline();
        let mut b = baseline();
        b.world_seed = a.world_seed; // explicit: proves this specific field, held equal, keeps the root equal
        assert_eq!(compute_world_baseline_root_v1(&a).unwrap(), compute_world_baseline_root_v1(&b).unwrap());
    }

    /// `T4-PV`'s own scope: an unpopulated (`None`) protocol version must
    /// hash distinctly from EVERY populated value, including the
    /// "reserved" value `0` -- otherwise "not yet derived" would be
    /// silently indistinguishable from a real, meaningful protocol
    /// version once one happens to be `0`.
    #[test]
    fn unpopulated_protocol_version_is_distinct_from_every_populated_value() {
        let unpopulated = WorldBaselineInputV1 { worldgen: None, ..baseline() };
        let populated_zero = WorldBaselineInputV1 {
            worldgen: Some(WorldgenProtocolVersion::new(ProtocolVersion::new(0))),
            ..baseline()
        };
        let populated_one = baseline(); // worldgen = Some(ProtocolVersion::new(1))

        let unpopulated_root = compute_world_baseline_root_v1(&unpopulated).unwrap();
        let zero_root = compute_world_baseline_root_v1(&populated_zero).unwrap();
        let one_root = compute_world_baseline_root_v1(&populated_one).unwrap();

        assert_ne!(unpopulated_root, zero_root, "None must not collide with Some(0)");
        assert_ne!(unpopulated_root, one_root);
        assert_ne!(zero_root, one_root);
    }

    /// The domain separation is real, not decorative: the same preimage
    /// bytes under a DIFFERENT domain produce a different root.
    #[test]
    fn domain_separation_is_real() {
        let preimage = world_baseline_preimage_v1(&baseline());
        let this_domain = digest_canonical_bytes_v1(DigestDomainIdV1::WorldBaselineManifest, &preimage, 1 << 24).unwrap();
        let other_domain = digest_canonical_bytes_v1(DigestDomainIdV1::SaveUniverseManifest, &preimage, 1 << 24).unwrap();
        assert_ne!(this_domain.bytes, other_domain.bytes);
    }
}
