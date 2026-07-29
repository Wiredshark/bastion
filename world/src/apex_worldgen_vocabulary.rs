//! `APEX-T4-PV` — the frozen worldgen vocabulary, and the derivation of
//! [`WorldgenProtocolVersion`] from it.
//!
//! **This exists in `world` and not in `common` for the reason `T4.3`
//! stated when it banked the derivation: `world` depends on `common`,
//! never the reverse, and the vocabulary is made of `world`'s own types
//! (`FileOpts`, `GenOpts`) plus `common`'s `Calendar`.** `common` can
//! hold the carrier; only `world` can fill it.
//!
//! # What is in the vocabulary, and why it is this short
//!
//! The survey (`readme/apex/APEX-T4-PV-WORLDGEN-VOCABULARY-SURVEY-v1.md`)
//! established that almost every worldgen input is ALREADY pinned by
//! something else, so enumerating it here would restate an existing root
//! in a second hand-maintained list — the two-lists-that-drift failure
//! `E13`/`E14-3` spent five chunks making unrepresentable:
//!
//! - Everything compiled — `CONFIG`'s 17 fields, the erosion
//!   coefficients, the seed-diffusion multiplier, `Noise`'s octave count
//!   and `seed+0/+1/+2` derivation, all 48 site-plot generators — rides
//!   `T1.2`'s SOURCE CLOSURE. Editing any of them edits `world/src`,
//!   which moves that root already. The closure also pins `Cargo.lock`
//!   (so the `noise` crate's own version is covered) and
//!   `rust-toolchain` (so rustc codegen differences, which would change
//!   float-heavy terrain while every source file stayed byte-identical,
//!   are covered too).
//! - Everything under `assets/` — `world.style.colors`,
//!   `world.features`' toggles, the wildlife spawn manifests — rides the
//!   CONTENT root.
//!
//! So the vocabulary is exactly the inputs that can differ **between two
//! servers running the same binary over the same assets**. There are
//! few, and each is here because it was read, not because it looked
//! like a parameter.
//!
//! # The one that a constants-based derivation would have missed
//!
//! [`WorldgenVocabularyV1::map_source`]. Under `FileOpts`' load
//! variants the world is **not derived from the seed at all** — it is
//! the bytes of a map file. Two servers with identical code, identical
//! assets and the same seed generate different worlds if one of them
//! loaded a map. And per the orchestrator's ruling, the LOADED MAP'S
//! CONTENT DIGEST is in the vocabulary rather than merely the fact that
//! loading happened: "a map was loaded" is not an identity, the map's
//! bytes are. Otherwise the vocabulary would record that the
//! seed→world derivation was broken without recording what replaced it.

use common::{
    apex::{
        digest::{DigestDomainIdV1, DigestErrorV1, digest_canonical_bytes_v1},
        subsystem::descriptor::WorldgenProtocolVersion,
    },
    calendar::{Calendar, CalendarEvent},
};

use common::resources::MapKind;

use crate::sim::{FileOpts, GenOpts};

/// Where this world's terrain came from.
///
/// A separate type from `FileOpts` on purpose: `FileOpts` also carries
/// PATHS, and a path is not world identity — two servers loading
/// byte-identical maps from different directories generate the same
/// world. Encoding `FileOpts` directly would make the vocabulary
/// sensitive to where an operator keeps their files, which is the
/// too-wide direction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapSourceV1 {
    /// Generated from the seed and the options below.
    Generated,
    /// Loaded from a map file OUTSIDE the asset tree, identified by the
    /// digest of its BYTES.
    Loaded { map_digest: [u8; 32] },
    /// Loaded from a map ASSET, identified by its asset path.
    ///
    /// **A third variant, added at wiring time rather than designed in,
    /// and worth the extra surface.** The default server configuration
    /// uses `FileOpts::LoadAsset` -- so folding it into `Loaded` would
    /// have demanded a byte digest on the NORMAL path and reported
    /// every stock server as carrying an unidentified map.
    ///
    /// The asset path is sufficient identity HERE, and only here,
    /// because the asset's CONTENT already rides the content root: two
    /// servers agreeing on the content root and on this path have
    /// agreed on the map's bytes. Recording the path rather than
    /// re-digesting it avoids restating a root that already exists --
    /// the same discipline that kept `CONFIG` out of this vocabulary.
    LoadedAsset { asset_path: String },
}

/// The frozen worldgen vocabulary.
///
/// Every field is here because a difference in it produces a different
/// world between two servers on the same binary and assets. Adding a
/// field is a deliberate act: it changes every world baseline root, so
/// every existing save is refused as world-incompatible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldgenVocabularyV1 {
    pub world_seed: u32,
    pub map_source: MapSourceV1,
    /// `GenOpts.x_lg`/`y_lg` — map dimensions, log2.
    pub x_lg: u32,
    pub y_lg: u32,
    /// `GenOpts.scale`, by its exact bits. A float's VALUE is what
    /// matters, and `to_bits` is the only encoding of it that cannot
    /// vary by formatting.
    pub scale_bits: u64,
    /// `GenOpts.map_kind`, as its discriminant tag.
    pub map_kind_tag: u8,
    /// `GenOpts.erosion_quality` bits — erosion is the dominant
    /// terrain-shaping pass, and quality changes its RESULT, not merely
    /// its cost.
    pub erosion_quality_bits: u32,
    /// `WorldOpts.seed_elements`.
    pub seed_elements: bool,
    /// The calendar events in effect, SORTED and deduplicated.
    ///
    /// In the vocabulary because `Calendar` reaches block generation and
    /// branches on `Christmas`/`Halloween` to emit DIFFERENT BLOCKS
    /// (`block.rs`) — generated content, not seasonal presentation. Two
    /// servers generating the same seed on different DATES do not
    /// produce the same world. This was filed IRRELEVANT in the survey's
    /// first pass and corrected on a read; the hedge that made it
    /// re-checkable is why.
    ///
    /// Sorted because the vocabulary must identify the SET of active
    /// events, not the order a caller happened to list them in.
    pub calendar_events: Vec<u8>,
}

impl WorldgenVocabularyV1 {
    /// Builds the vocabulary from the live options.
    ///
    /// `map_digest` is the caller's job: `world` does not read the map
    /// file itself, and inventing a digest here would be exactly the
    /// fabrication this row exists to avoid. `None` under a load variant
    /// is representable and DELIBERATE — see
    /// [`Self::map_source_is_honest`].
    pub fn from_opts_v1(
        world_seed: u32,
        file_opts: &FileOpts,
        gen_opts: &GenOpts,
        seed_elements: bool,
        calendar: Option<&Calendar>,
        loaded_map_digest: Option<[u8; 32]>,
    ) -> Self {
        let map_source = match (file_opts, loaded_map_digest) {
            (FileOpts::Generate(_) | FileOpts::Save(_, _), _) => MapSourceV1::Generated,
            (FileOpts::LoadAsset(path), _) => MapSourceV1::LoadedAsset { asset_path: path.clone() },
            (_, Some(map_digest)) => MapSourceV1::Loaded { map_digest },
            // A load variant with no digest supplied. Represented as
            // Generated would be a LIE; the caller is expected to have
            // supplied one, and `map_source_is_honest` is how a caller
            // checks itself before trusting the root.
            (_, None) => MapSourceV1::Loaded { map_digest: [0u8; 32] },
        };

        let mut calendar_events: Vec<u8> = calendar
            .map(|c| c.events().map(|e| *e as u8).collect())
            .unwrap_or_default();
        calendar_events.sort_unstable();
        calendar_events.dedup();

        Self {
            world_seed,
            map_source,
            x_lg: gen_opts.x_lg,
            y_lg: gen_opts.y_lg,
            scale_bits: gen_opts.scale.to_bits(),
            map_kind_tag: match gen_opts.map_kind {
                MapKind::Square => 0,
                MapKind::Circle => 1,
            },
            erosion_quality_bits: gen_opts.erosion_quality.to_bits(),
            seed_elements,
            calendar_events,
        }
    }

    /// Whether a load-variant vocabulary actually carries a map digest.
    ///
    /// The all-zero digest is the representable-but-dishonest case: a
    /// caller that forgot to supply the bytes' identity. This is a
    /// checkable predicate rather than a panic because `world` is not
    /// the layer that decides what to do about it — but a caller that
    /// certifies a world baseline on a dishonest vocabulary is
    /// certifying "a map was loaded" and nothing more, which the
    /// orchestrator's own ruling rejects.
    pub fn map_source_is_honest(&self) -> bool {
        !matches!(self.map_source, MapSourceV1::Loaded { map_digest } if map_digest == [0u8; 32])
    }

    /// The frozen preimage. Every field fixed-width or length-prefixed,
    /// so no two distinct vocabularies can produce the same bytes — the
    /// same discipline `world_baseline_preimage_v1` states for itself,
    /// and for the same reason: a collision here is a save adopted
    /// against a world that no longer exists.
    fn preimage_v1(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.world_seed.to_be_bytes());
        match &self.map_source {
            MapSourceV1::Generated => buf.push(0),
            MapSourceV1::Loaded { map_digest } => {
                buf.push(1);
                buf.extend_from_slice(map_digest);
            },
            MapSourceV1::LoadedAsset { asset_path } => {
                buf.push(2);
                buf.extend_from_slice(&(asset_path.len() as u64).to_be_bytes());
                buf.extend_from_slice(asset_path.as_bytes());
            },
        }
        buf.extend_from_slice(&self.x_lg.to_be_bytes());
        buf.extend_from_slice(&self.y_lg.to_be_bytes());
        buf.extend_from_slice(&self.scale_bits.to_be_bytes());
        buf.push(self.map_kind_tag);
        buf.extend_from_slice(&self.erosion_quality_bits.to_be_bytes());
        buf.push(u8::from(self.seed_elements));
        // Length-prefixed: without it, one event list could be confused
        // with a longer one whose tail happened to match the next field.
        buf.extend_from_slice(&(self.calendar_events.len() as u64).to_be_bytes());
        buf.extend_from_slice(&self.calendar_events);
        buf
    }

    /// The derived worldgen protocol root.
    ///
    /// A frozen-vocabulary content-root derivation per
    /// `net_envelope_profile_root_v1`'s pattern, under this program's
    /// own domain-separated digest — never an arbitrary integer, which
    /// is `T4.3`'s standing rule for all three of these roots.
    pub fn protocol_root_v1(&self) -> Result<WorldgenProtocolVersion, DigestErrorV1> {
        let digest = digest_canonical_bytes_v1(
            DigestDomainIdV1::WorldgenProtocolRoot,
            &self.preimage_v1(),
            4096,
        )?;
        Ok(WorldgenProtocolVersion::new(digest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vocab() -> WorldgenVocabularyV1 {
        WorldgenVocabularyV1 {
            world_seed: 42,
            map_source: MapSourceV1::Generated,
            x_lg: 10,
            y_lg: 10,
            scale_bits: 2.0f64.to_bits(),
            map_kind_tag: 0,
            erosion_quality_bits: 1.0f32.to_bits(),
            seed_elements: true,
            calendar_events: Vec::new(),
        }
    }

    fn root(v: &WorldgenVocabularyV1) -> [u8; 32] {
        *v.protocol_root_v1().expect("root").get().bytes.as_array()
    }

    #[test]
    fn the_same_vocabulary_produces_the_same_root() {
        assert_eq!(root(&vocab()), root(&vocab()));
    }

    /// Every MUST-BE member must MOVE the root. A vocabulary member that
    /// does not reach the root is a member in name only -- and this is
    /// the test that would catch someone adding a field to the struct
    /// and forgetting the preimage.
    #[test]
    fn every_vocabulary_member_moves_the_root() {
        let base = root(&vocab());

        let mut v = vocab();
        v.world_seed = 43;
        assert_ne!(root(&v), base, "world_seed");

        let mut v = vocab();
        v.map_source = MapSourceV1::Loaded { map_digest: [7u8; 32] };
        assert_ne!(root(&v), base, "map_source");

        let mut v = vocab();
        v.x_lg = 11;
        assert_ne!(root(&v), base, "x_lg");

        let mut v = vocab();
        v.y_lg = 11;
        assert_ne!(root(&v), base, "y_lg");

        let mut v = vocab();
        v.scale_bits = 4.0f64.to_bits();
        assert_ne!(root(&v), base, "scale");

        let mut v = vocab();
        v.map_kind_tag = 1;
        assert_ne!(root(&v), base, "map_kind");

        let mut v = vocab();
        v.erosion_quality_bits = 2.0f32.to_bits();
        assert_ne!(root(&v), base, "erosion_quality");

        let mut v = vocab();
        v.seed_elements = false;
        assert_ne!(root(&v), base, "seed_elements");

        let mut v = vocab();
        v.calendar_events = vec![CalendarEvent::Christmas as u8];
        assert_ne!(root(&v), base, "calendar_events");
    }

    /// Two DIFFERENT loaded maps must produce different roots.
    ///
    /// This is the orchestrator's Q4 ruling as a test: if only the FACT
    /// of loading reached the root, these two would collide and two
    /// servers holding different maps would certify the same world.
    #[test]
    fn different_loaded_maps_do_not_share_a_root() {
        let mut a = vocab();
        a.map_source = MapSourceV1::Loaded { map_digest: [1u8; 32] };
        let mut b = vocab();
        b.map_source = MapSourceV1::Loaded { map_digest: [2u8; 32] };
        assert_ne!(root(&a), root(&b));
        // ...and neither may collide with the generated case.
        assert_ne!(root(&a), root(&vocab()));
    }

    /// Calendar identifies a SET, not a listing order.
    #[test]
    fn calendar_event_order_does_not_move_the_root() {
        let mut a = vocab();
        a.calendar_events = vec![CalendarEvent::Christmas as u8, CalendarEvent::Easter as u8];
        let mut b = vocab();
        b.calendar_events = vec![CalendarEvent::Easter as u8, CalendarEvent::Christmas as u8];
        b.calendar_events.sort_unstable();
        assert_eq!(root(&a), root(&b));
    }

    /// The length prefix earns its place: without it, a longer event
    /// list could alias a shorter one plus the bytes that follow.
    #[test]
    fn event_list_length_is_prefixed_not_implied() {
        let mut a = vocab();
        a.calendar_events = vec![0, 1];
        let mut b = vocab();
        b.calendar_events = vec![0, 1, 2];
        assert_ne!(root(&a), root(&b));
    }

    /// The three map sources are mutually distinct, including the two
    /// LOADED ones.
    ///
    /// The asset case is not a special-case of the file case: they
    /// identify the map by different things (a path whose content rides
    /// the content root, versus raw bytes), so a vocabulary must not
    /// collapse them.
    #[test]
    fn the_three_map_sources_do_not_collide() {
        let mut generated = vocab();
        generated.map_source = MapSourceV1::Generated;
        let mut loaded = vocab();
        loaded.map_source = MapSourceV1::Loaded { map_digest: [9u8; 32] };
        let mut asset = vocab();
        asset.map_source = MapSourceV1::LoadedAsset { asset_path: "world.map.veloren".to_owned() };

        let (g, l, a) = (root(&generated), root(&loaded), root(&asset));
        assert_ne!(g, l);
        assert_ne!(g, a);
        assert_ne!(l, a, "an asset load and a file load are different identity stories");
    }

    /// Different map ASSETS must not share a root either.
    #[test]
    fn different_map_assets_do_not_share_a_root() {
        let mut a = vocab();
        a.map_source = MapSourceV1::LoadedAsset { asset_path: "world.map.alpha".to_owned() };
        let mut b = vocab();
        b.map_source = MapSourceV1::LoadedAsset { asset_path: "world.map.beta".to_owned() };
        assert_ne!(root(&a), root(&b));
    }

    /// A load variant with no digest supplied is representable, and
    /// callers can detect it. Recorded as a checkable predicate rather
    /// than made unrepresentable, because `world` is not the layer that
    /// decides what to do about a caller's omission.
    #[test]
    fn a_load_without_a_digest_is_detectable() {
        let mut v = vocab();
        v.map_source = MapSourceV1::Loaded { map_digest: [0u8; 32] };
        assert!(!v.map_source_is_honest());

        v.map_source = MapSourceV1::Loaded { map_digest: [3u8; 32] };
        assert!(v.map_source_is_honest());

        assert!(vocab().map_source_is_honest(), "the generated case is always honest");
    }
}
