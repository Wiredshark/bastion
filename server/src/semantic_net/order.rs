//! `APEX-T3.3.12`: freeze the producer/phase/payload-rank registries
//! `ServerSemanticOrderKeyV1` (`T3.3.11`) needs, and add the bounded
//! `CanonicalSubjectKeyV1` constructors the packet names. Spec:
//! `PROJECT-BASTION-APEX-MICROSTEP-APEX-T3.3-SEMANTIC-NET-ENVELOPE.md`
//! section 12.
//!
//! Design choice worth stating up front: every registry here is a
//! closed Rust enum (or, for phase, the already-existing
//! `common_ecs::Phase`) with EXPLICIT numeric assignment, not an open,
//! string- or number-keyed lookup table someone could query with an
//! unregistered value. That eliminates this step's own "Unknown
//! producer/rank" failure mode entirely rather than adding code to
//! reject it at runtime -- a closed enum has no "unknown" variant to
//! construct in the first place; the compiler enforces exhaustiveness
//! at every match site instead.
//!
//! Coverage is deliberately MINIMAL, not speculative: the phase set
//! reuses `common_ecs::Phase` unchanged (zero new concept), the one
//! producer this whole packet section concretely names so far
//! (`EntitySync`, `T3.3.13`), and the four payload kinds section 6's own
//! prose names for it ("create/delete/entity-sync/comp-sync order").
//! `T3.3.14`'s full `server/src` send-site survey is what will actually
//! enumerate every other producer and payload kind that ends up needing
//! a rank -- adding speculative entries here ahead of that survey would
//! be guessing at shapes T3.3.14 might reshape anyway. New
//! producers/payload kinds get a new variant when they're actually
//! migrated, never a silent default onto an existing one.

use common::apex::manifest::{ManifestCodecErrorV1, ManifestDecodeLimitsV1, ManifestEncodeV1, ManifestValueV1, encode_manifest_v1};
use common_ecs::Phase;
use vek::{Vec2, Vec3};

use super::outbox::CanonicalSubjectKeyV1;

/// `phase_rank`: the three existing ECS dispatch phases, in their own
/// existing execution order (`Create` runs before `Review` runs before
/// `Apply`) -- not a new concept, just an explicit numeric mapping onto
/// one that already exists and already governs when a producer's system
/// actually runs.
pub const fn phase_rank(phase: Phase) -> u16 {
    match phase {
        Phase::Create => 0,
        Phase::Review => 1,
        Phase::Apply => 2,
    }
}

/// `producer_rank`.
#[repr(u16)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SemanticProducerV1 {
    EntitySync = 0,
}

impl SemanticProducerV1 {
    pub const fn producer_rank(self) -> u16 { self as u16 }
}

/// `payload_rank`. Section 6: "Separate payload ranks preserve create/
/// delete/entity-sync/comp-sync order within one stream" -- variant
/// order below IS that listed order.
#[repr(u16)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SemanticPayloadRankV1 {
    Create = 0,
    Delete = 1,
    EntitySync = 2,
    CompSync = 3,
}

impl SemanticPayloadRankV1 {
    pub const fn payload_rank(self) -> u16 { self as u16 }
}

/// Every [`CanonicalSubjectKeyV1`] constructor below prefixes its
/// payload with one of these as an explicit `Unsigned` tag before
/// encoding -- what makes different subject KINDS structurally unable
/// to collide even when their underlying numeric values happen to
/// coincide (a `for_uid(Uid(5))` and a `for_group(Group(5))` encode to
/// different bytes because they carry different tags, not because
/// `5 != 5`).
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum CanonicalSubjectKindV1 {
    Uid = 1,
    Position = 2,
    Chunk = 3,
    Region = 4,
    Character = 5,
    Group = 6,
    Trade = 7,
    Singleton = 8,
}

fn subject_limits() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 256, // matches CANONICAL_SUBJECT_KEY_MAX_BYTES
        max_depth: 4,
        max_nodes: 16,
        max_array_items: 8,
        max_map_entries: 0,
        max_machine_text_bytes: 0,
        max_byte_string_bytes: 64,
    }
}

fn signed_to_value(v: i64) -> ManifestValueV1 {
    if v < 0 { ManifestValueV1::negative(v).expect("just checked v < 0") } else { ManifestValueV1::Unsigned(v as u64) }
}

/// `Group`/`TradeId` keep their inner integer private -- their own
/// existing `Serialize` impl (already relied on elsewhere: both cross
/// the network today inside `ServerGeneral::GroupUpdate`/`FinishedTrade`
/// via this exact bincode-legacy encoding) is the sanctioned way to get
/// a deterministic byte representation without reaching for a private
/// field this module has no business touching directly.
fn bincode_legacy_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    bincode::serde::encode_to_vec(value, bincode::config::legacy())
        .expect("bincode legacy serde encoding of an owned value is infallible")
}

/// Only `common::apex::manifest` may construct raw CBOR bytes directly
/// (its own module doc: "nothing outside `common::apex::manifest`
/// constructs or inspects raw bytes") -- this thin DTO is how a subject
/// constructor reaches the public `encode_manifest_v1::<T>` entry point
/// instead of a private value-level encoder.
struct TaggedSubjectV1 {
    kind: CanonicalSubjectKindV1,
    fields: Vec<ManifestValueV1>,
}

impl ManifestEncodeV1 for TaggedSubjectV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        let mut items = Vec::with_capacity(self.fields.len() + 1);
        items.push(ManifestValueV1::Unsigned(self.kind as u64));
        items.extend(self.fields.iter().cloned());
        Ok(ManifestValueV1::Array(items))
    }
}

fn encode_tagged(kind: CanonicalSubjectKindV1, fields: Vec<ManifestValueV1>) -> CanonicalSubjectKeyV1 {
    let bytes = encode_manifest_v1(&TaggedSubjectV1 { kind, fields }, &subject_limits())
        .expect("subject constructors below are always within subject_limits' bounds");
    CanonicalSubjectKeyV1::try_new(bytes)
        .expect("encoded tagged subject bytes are always well within CANONICAL_SUBJECT_KEY_MAX_BYTES")
}

impl CanonicalSubjectKeyV1 {
    pub fn for_uid(uid: common::uid::Uid) -> Self {
        encode_tagged(CanonicalSubjectKindV1::Uid, vec![ManifestValueV1::Unsigned(uid.0.get())])
    }

    /// A world position, quantized to whole blocks (`Vec3<i32>`) --
    /// never a raw `Vec3<f32>`. Floats carry signed-zero/NaN/rounding
    /// ambiguity that would let two logically-identical positions
    /// encode to different subject bytes; callers quantize before
    /// calling this (this constructor deliberately does not do it
    /// silently on their behalf).
    pub fn for_position(pos: Vec3<i32>) -> Self {
        encode_tagged(
            CanonicalSubjectKindV1::Position,
            vec![signed_to_value(pos.x.into()), signed_to_value(pos.y.into()), signed_to_value(pos.z.into())],
        )
    }

    pub fn for_chunk(key: Vec2<i32>) -> Self {
        encode_tagged(CanonicalSubjectKindV1::Chunk, vec![signed_to_value(key.x.into()), signed_to_value(key.y.into())])
    }

    pub fn for_region(key: Vec2<i32>) -> Self {
        encode_tagged(CanonicalSubjectKindV1::Region, vec![signed_to_value(key.x.into()), signed_to_value(key.y.into())])
    }

    pub fn for_character(id: common::character::CharacterId) -> Self {
        encode_tagged(CanonicalSubjectKindV1::Character, vec![signed_to_value(id.0)])
    }

    pub fn for_group(group: common::comp::group::Group) -> Self {
        encode_tagged(CanonicalSubjectKindV1::Group, vec![ManifestValueV1::Bytes(bincode_legacy_bytes(&group))])
    }

    pub fn for_trade(id: common::trade::TradeId) -> Self {
        encode_tagged(CanonicalSubjectKindV1::Trade, vec![ManifestValueV1::Bytes(bincode_legacy_bytes(&id))])
    }

    /// For messages with no natural per-entity/per-subject key (a
    /// server-wide broadcast, for instance). `label` should be a fixed,
    /// stable name for the MESSAGE KIND, not a per-instance value --
    /// multiple enqueues of the same kind in one tick are distinguished
    /// by `ServerSemanticOrderKeyV1::local_ordinal`, not by varying this.
    pub fn for_singleton(label: &'static str) -> Self {
        encode_tagged(CanonicalSubjectKindV1::Singleton, vec![ManifestValueV1::Bytes(label.as_bytes().to_vec())])
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;
    use crate::semantic_net::outbox::{CANONICAL_SUBJECT_KEY_MAX_BYTES, SemanticSendIntentV1, ServerSemanticOrderKeyV1};

    #[test]
    fn phase_rank_matches_existing_dispatch_order() {
        assert!(phase_rank(Phase::Create) < phase_rank(Phase::Review));
        assert!(phase_rank(Phase::Review) < phase_rank(Phase::Apply));
    }

    #[test]
    fn entity_sync_payload_ranks_match_the_packet_named_order() {
        assert!(SemanticPayloadRankV1::Create.payload_rank() < SemanticPayloadRankV1::Delete.payload_rank());
        assert!(SemanticPayloadRankV1::Delete.payload_rank() < SemanticPayloadRankV1::EntitySync.payload_rank());
        assert!(SemanticPayloadRankV1::EntitySync.payload_rank() < SemanticPayloadRankV1::CompSync.payload_rank());
    }

    #[test]
    fn subject_constructors_are_deterministic() {
        assert_eq!(
            CanonicalSubjectKeyV1::for_uid(common::uid::Uid(std::num::NonZeroU64::new(7).unwrap())),
            CanonicalSubjectKeyV1::for_uid(common::uid::Uid(std::num::NonZeroU64::new(7).unwrap()))
        );
        assert_eq!(CanonicalSubjectKeyV1::for_chunk(Vec2::new(3, -4)), CanonicalSubjectKeyV1::for_chunk(Vec2::new(3, -4)));
    }

    #[test]
    fn different_subject_kinds_never_collide_on_the_same_raw_value() {
        let uid = CanonicalSubjectKeyV1::for_uid(common::uid::Uid(std::num::NonZeroU64::new(5).unwrap()));
        let chunk = CanonicalSubjectKeyV1::for_chunk(Vec2::new(5, 0));
        let region = CanonicalSubjectKeyV1::for_region(Vec2::new(5, 0));
        let character = CanonicalSubjectKeyV1::for_character(common::character::CharacterId(5));
        let singleton = CanonicalSubjectKeyV1::for_singleton("five");
        let all = [uid, chunk, region, character, singleton];
        for (a, b) in all.iter().tuple_combinations() {
            assert_ne!(a, b, "distinct subject kinds must never encode to equal bytes");
        }
    }

    #[test]
    fn subject_bytes_never_exceed_the_outbox_maximum() {
        let keys = [
            CanonicalSubjectKeyV1::for_uid(common::uid::Uid(std::num::NonZeroU64::new(u64::MAX).unwrap())),
            CanonicalSubjectKeyV1::for_position(Vec3::new(i32::MIN, i32::MAX, i32::MIN)),
            CanonicalSubjectKeyV1::for_chunk(Vec2::new(i32::MIN, i32::MAX)),
            CanonicalSubjectKeyV1::for_region(Vec2::new(i32::MIN, i32::MAX)),
            CanonicalSubjectKeyV1::for_character(common::character::CharacterId(i64::MIN)),
            CanonicalSubjectKeyV1::for_group(common::comp::group::ENEMY),
            CanonicalSubjectKeyV1::for_singleton("a-fairly-long-singleton-message-kind-label"),
        ];
        for key in keys {
            assert!(key.as_bytes().len() <= CANONICAL_SUBJECT_KEY_MAX_BYTES);
        }
    }

    fn recipient() -> common_net::msg::envelope::ActiveSessionBindingV1 {
        common_net::msg::envelope::ActiveSessionBindingV1 {
            server_boot_id: common::apex::identity::ServerBootId::generate(
                &mut common::apex::identity::FixedRandomBytesSourceV1([1; 16]),
            )
            .unwrap(),
            session_id: common::apex::identity::SessionId::generate(&mut common::apex::identity::FixedRandomBytesSourceV1(
                [2; 16],
            ))
            .unwrap(),
            epoch: common::apex::identity::ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn intent_with(
        source_tick: u64,
        phase: Phase,
        producer: SemanticProducerV1,
        payload: SemanticPayloadRankV1,
        subject: CanonicalSubjectKeyV1,
        local_ordinal: u32,
    ) -> SemanticSendIntentV1 {
        SemanticSendIntentV1 {
            recipient: recipient(),
            semantic_stream: common_net::msg::envelope::SemanticStreamIdV1::General,
            causality: common_net::msg::envelope::SemanticCausalityV1 { producer_tick: None, snapshot: None },
            order_key: ServerSemanticOrderKeyV1 {
                source_tick,
                phase_rank: phase_rank(phase),
                producer_rank: producer.producer_rank(),
                payload_rank: payload.payload_rank(),
                subject,
                local_ordinal,
            },
            payload: std::sync::Arc::new(common_net::msg::ServerGeneral::UpdateRecipes),
        }
    }

    /// Golden order vectors: a fixed, hand-verifiable table proving
    /// each field of the total sort tuple dominates the comparison over
    /// every field to its right, holding everything to its left equal
    /// -- section 7.7's tuple, one field at a time.
    #[test]
    fn golden_order_vectors_each_field_dominates_the_ones_after_it() {
        let base_subject = CanonicalSubjectKeyV1::for_singleton("a");
        let later_subject = CanonicalSubjectKeyV1::for_singleton("b");

        // source_tick dominates phase/producer/payload/subject/ordinal.
        let earlier_tick =
            intent_with(1, Phase::Apply, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::CompSync, later_subject.clone(), 9);
        let later_tick =
            intent_with(2, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Create, base_subject.clone(), 0);
        assert!(earlier_tick.total_sort_key() < later_tick.total_sort_key());

        // phase_rank dominates producer/payload/subject/ordinal (tick equal).
        let create_phase =
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::CompSync, later_subject.clone(), 9);
        let apply_phase =
            intent_with(1, Phase::Apply, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Create, base_subject.clone(), 0);
        assert!(create_phase.total_sort_key() < apply_phase.total_sort_key());

        // payload_rank dominates subject/ordinal (tick, phase, producer equal).
        let create_payload =
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Create, later_subject.clone(), 9);
        let comp_sync_payload =
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::CompSync, base_subject.clone(), 0);
        assert!(create_payload.total_sort_key() < comp_sync_payload.total_sort_key());

        // subject bytes dominate ordinal (everything else equal).
        let subject_a =
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Create, base_subject.clone(), 9);
        let subject_b =
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Create, later_subject.clone(), 0);
        assert!(subject_a.total_sort_key() < subject_b.total_sort_key());

        // local_ordinal is the final tiebreak (everything else equal).
        let ordinal_low =
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Create, base_subject.clone(), 0);
        let ordinal_high =
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Create, base_subject, 1);
        assert!(ordinal_low.total_sort_key() < ordinal_high.total_sort_key());
    }

    #[test]
    fn permutation_sort_is_order_independent_across_the_full_rank_hierarchy() {
        let intents = vec![
            intent_with(2, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Create, CanonicalSubjectKeyV1::for_singleton("x"), 0),
            intent_with(1, Phase::Apply, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::CompSync, CanonicalSubjectKeyV1::for_singleton("y"), 0),
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Delete, CanonicalSubjectKeyV1::for_singleton("z"), 5),
            intent_with(1, Phase::Create, SemanticProducerV1::EntitySync, SemanticPayloadRankV1::Delete, CanonicalSubjectKeyV1::for_singleton("z"), 1),
        ];

        let mut expected: Vec<_> = intents.clone();
        expected.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
        let expected_ordinals: Vec<u32> = expected.iter().map(|i| i.order_key.local_ordinal).collect();

        for permutation in intents.into_iter().permutations(4) {
            let mut got = permutation;
            got.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
            let got_ordinals: Vec<u32> = got.iter().map(|i| i.order_key.local_ordinal).collect();
            assert_eq!(got_ordinals, expected_ordinals);
        }
    }
}
