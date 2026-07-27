//! `APEX-T3.3.11`: the immutable server semantic outbox. Producers
//! enqueue owned send intents from any thread (including Rayon
//! workers); no sequence allocation or network send happens here --
//! that is `T3.3.15`'s `SemanticEgressSysV1`, not built yet. Spec:
//! `PROJECT-BASTION-APEX-MICROSTEP-APEX-T3.3-SEMANTIC-NET-ENVELOPE.md`
//! sections 6.4, 7.7, 7.8.
//!
//! Determinism story: producer insertion order is intentionally
//! ignored (section 7.8's own words) -- nothing in this module reads
//! insertion order for anything. The one property this step's
//! acceptance gate cares about ("identical intent multisets produce
//! identical sorted candidates regardless of thread order") is proven
//! by [`SemanticSendIntentV1::total_sort_key`]: every field it draws
//! from either already carries a manually-specified, wire-independent
//! `Ord` ([`SessionId`]'s unsigned byte-lexicographic order, `T0.4`;
//! [`SemanticStreamIdV1`]'s explicit discriminant order) or is a plain
//! numeric/byte comparison -- nothing here depends on pointer values,
//! hash iteration, or arrival time.
//!
//! Scope note on this step's own failure list ("No active attachment,
//! oversized key; no silent key invention"): "oversized key" and "no
//! silent key invention" map directly to
//! [`CanonicalSubjectKeyV1::try_new`] below, this module's only
//! constructor. "No active attachment" does not describe anything
//! `enqueue` itself can check -- by the time a caller has a fully
//! constructed [`SemanticSendIntentV1`], `enqueue` has no way to
//! independently ask "is this recipient still attached?" without a
//! live `T3.2` session registry, which is exactly what section 7.8
//! step 2 assigns to `SemanticEgressSysV1` ("validates each intent
//! against current active T3.2 binding") -- `T3.3.15`'s job, matching
//! this step's own explicit non-goal ("No send or sequence
//! allocation"). Deferred there, not invented here.

use std::sync::{Arc, Mutex};

use common_net::msg::{
    ServerGeneral,
    envelope::{ActiveSessionBindingV1, SemanticCausalityV1, SemanticStreamIdV1},
};

/// Section 7.7's payload type is named `ServerSemanticPayloadV1` but
/// never given its own field list anywhere in the packet -- `ServerGeneral`
/// is the only payload type ever routed through any of the four fenced
/// semantic streams (`T3.3.01-10`), so this is a plain alias, not a new
/// wrapper struct with no spec-given shape to justify one.
pub type ServerSemanticPayloadV1 = ServerGeneral;

/// Max byte length of a [`CanonicalSubjectKeyV1`] (section 7.7's own
/// comment: "deterministic T0.2 bytes; max 256").
pub const CANONICAL_SUBJECT_KEY_MAX_BYTES: usize = 256;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CanonicalSubjectKeyErrorV1 {
    OversizedKey { len: usize },
}

/// Section 7.7. The caller is responsible for producing `bytes` via the
/// T0.2-canonical encoding conventions already established elsewhere in
/// this program ([`common::apex::manifest`]) when a subject needs to
/// carry structured domain data (a UID, a chunk key, ...) --
/// `T3.3.12`'s own job is the actual per-domain constructors ("bounded
/// T0.2 `CanonicalSubjectKeyV1` constructors for UIDs, positions/
/// chunks/regions, characters, groups/trades, and singleton
/// messages"). This type only owns the bound: [`Self::try_new`] is the
/// ONLY constructor, and it is fallible -- there is deliberately no
/// `Default` or empty-key fallback a caller could reach for instead of
/// supplying real bytes ("no silent key invention").
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CanonicalSubjectKeyV1(Vec<u8>);

impl CanonicalSubjectKeyV1 {
    pub fn try_new(bytes: Vec<u8>) -> Result<Self, CanonicalSubjectKeyErrorV1> {
        if bytes.len() > CANONICAL_SUBJECT_KEY_MAX_BYTES {
            return Err(CanonicalSubjectKeyErrorV1::OversizedKey { len: bytes.len() });
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] { &self.0 }
}

/// Section 7.7. Field declaration order matches the total sort tuple's
/// own positions 4-9 exactly (`source_tick, phase_rank, producer_rank,
/// payload_rank, subject, local_ordinal`), so a `#[derive(Ord)]` here
/// is correct without hand-writing a comparator -- `Vec<u8>`'s own
/// derived `Ord` (inherited by `CanonicalSubjectKeyV1`) is already
/// lexicographic byte comparison, matching "subject bytes
/// lexicographically" precisely.
///
/// `phase_rank`/`producer_rank`/`payload_rank` are populated by
/// producers as plain `u16` values in this step; `T3.3.12` ("Freeze
/// producer and total-order registries") is what defines the actual
/// registries mapping real systems/payload kinds to specific rank
/// numbers -- this step only defines the shape they're carried in.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ServerSemanticOrderKeyV1 {
    pub source_tick: u64,
    pub phase_rank: u16,
    pub producer_rank: u16,
    pub payload_rank: u16,
    pub subject: CanonicalSubjectKeyV1,
    pub local_ordinal: u32,
}

/// Section 7.7. `payload` is `Arc` specifically so parallel producers
/// (packet 6.4: "many producers... parallel entity_sync region
/// workers") can share one encoded/encodable value cheaply across
/// threads without cloning it per-recipient -- the outbox never hands
/// back a `&mut` to a stored intent's payload, so once enqueued it is
/// immutable for the rest of its life in this structure ("immutable
/// payload").
#[derive(Clone, Debug)]
pub struct SemanticSendIntentV1 {
    pub recipient: ActiveSessionBindingV1,
    pub semantic_stream: SemanticStreamIdV1,
    pub causality: SemanticCausalityV1,
    pub order_key: ServerSemanticOrderKeyV1,
    pub payload: Arc<ServerSemanticPayloadV1>,
}

impl SemanticSendIntentV1 {
    /// Section 7.7's total sort tuple: `(recipient.session_id bytes,
    /// recipient.connection_epoch, semantic_stream tag, source_tick,
    /// phase_rank, producer_rank, payload_rank, subject bytes
    /// lexicographically, local_ordinal)`. `payload` and `causality`
    /// deliberately take no part in ordering -- the packet's tuple
    /// never mentions them.
    pub fn total_sort_key(
        &self,
    ) -> (
        common::apex::identity::SessionId,
        common::apex::identity::ConnectionEpoch,
        SemanticStreamIdV1,
        u64,
        u16,
        u16,
        u16,
        &CanonicalSubjectKeyV1,
        u32,
    ) {
        (
            self.recipient.session_id,
            self.recipient.epoch,
            self.semantic_stream,
            self.order_key.source_tick,
            self.order_key.phase_rank,
            self.order_key.producer_rank,
            self.order_key.payload_rank,
            &self.order_key.subject,
            self.order_key.local_ordinal,
        )
    }
}

/// Section 7.8. `Mutex<Vec<...>>`, exactly as specified -- "producer
/// insertion order is intentionally ignored" is not a performance
/// afterthought, it's the whole point: nothing downstream of this
/// struct may depend on the order producers happened to call
/// [`Self::enqueue`] in.
#[derive(Default)]
pub struct ServerSemanticOutboxV1 {
    pending: Mutex<Vec<SemanticSendIntentV1>>,
}

impl ServerSemanticOutboxV1 {
    pub fn new() -> Self { Self::default() }

    /// Append-only, infallible, thread-safe. No binding-freshness,
    /// duplicate-key, or send/sequence work happens here -- see this
    /// module's own top-level doc comment for exactly why "No active
    /// attachment" isn't a rejection this function can perform.
    pub fn enqueue(&self, intent: SemanticSendIntentV1) {
        self.pending.lock().expect("semantic outbox mutex poisoned").push(intent);
    }

    /// Section 7.8 step 1, "takes the complete pending vector" --
    /// `T3.3.15`'s `SemanticEgressSysV1` is the step that actually
    /// calls this as part of a real per-tick flush; this step has no
    /// egress system yet, so it's exercised only by this module's own
    /// tests below.
    pub fn take_pending(&self) -> Vec<SemanticSendIntentV1> {
        std::mem::take(&mut *self.pending.lock().expect("semantic outbox mutex poisoned"))
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use common_net::msg::envelope::ActiveSessionBindingV1;
    use itertools::Itertools;

    use super::*;

    fn subject(byte: u8) -> CanonicalSubjectKeyV1 { CanonicalSubjectKeyV1::try_new(vec![byte]).unwrap() }

    fn binding(seed: u8) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    fn intent(recipient_seed: u8, local_ordinal: u32, subject_byte: u8) -> SemanticSendIntentV1 {
        SemanticSendIntentV1 {
            recipient: binding(recipient_seed),
            semantic_stream: SemanticStreamIdV1::General,
            causality: SemanticCausalityV1 { producer_tick: None, snapshot: None },
            order_key: ServerSemanticOrderKeyV1 {
                source_tick: 0,
                phase_rank: 0,
                producer_rank: 0,
                payload_rank: 0,
                subject: subject(subject_byte),
                local_ordinal,
            },
            payload: Arc::new(ServerGeneral::UpdateRecipes),
        }
    }

    #[test]
    fn oversized_subject_is_rejected() {
        assert_eq!(
            CanonicalSubjectKeyV1::try_new(vec![0u8; CANONICAL_SUBJECT_KEY_MAX_BYTES + 1]).unwrap_err(),
            CanonicalSubjectKeyErrorV1::OversizedKey { len: CANONICAL_SUBJECT_KEY_MAX_BYTES + 1 }
        );
        // The boundary itself is accepted, not rejected off-by-one.
        assert!(CanonicalSubjectKeyV1::try_new(vec![0u8; CANONICAL_SUBJECT_KEY_MAX_BYTES]).is_ok());
    }

    #[test]
    fn immutable_payload_is_shared_not_cloned_across_intents() {
        let payload = Arc::new(ServerGeneral::UpdateRecipes);
        let a = SemanticSendIntentV1 {
            recipient: binding(1),
            semantic_stream: SemanticStreamIdV1::General,
            causality: SemanticCausalityV1 { producer_tick: None, snapshot: None },
            order_key: ServerSemanticOrderKeyV1 {
                source_tick: 0,
                phase_rank: 0,
                producer_rank: 0,
                payload_rank: 0,
                subject: subject(1),
                local_ordinal: 0,
            },
            payload: Arc::clone(&payload),
        };
        let b = SemanticSendIntentV1 { order_key: ServerSemanticOrderKeyV1 { local_ordinal: 1, ..a.order_key.clone() }, ..a.clone() };
        assert!(Arc::ptr_eq(&a.payload, &b.payload));
        assert_eq!(Arc::strong_count(&payload), 3); // payload + a.payload + b.payload
    }

    #[test]
    fn all_insertion_permutations_produce_the_same_sorted_result() {
        // Four intents, distinguishable only by their order-key fields
        // (same recipient/stream), so the total sort is exercised
        // purely by the fields the packet's own tuple names.
        let intents = vec![intent(1, 3, 1), intent(1, 1, 2), intent(1, 2, 1), intent(1, 0, 3)];

        let mut expected: Vec<_> = intents.clone();
        expected.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
        let expected_subjects_and_ordinals: Vec<(Vec<u8>, u32)> =
            expected.iter().map(|i| (i.order_key.subject.as_bytes().to_vec(), i.order_key.local_ordinal)).collect();

        for permutation in intents.iter().cloned().permutations(intents.len()) {
            let outbox = ServerSemanticOutboxV1::new();
            for i in permutation {
                outbox.enqueue(i);
            }
            let mut got = outbox.take_pending();
            got.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
            let got_subjects_and_ordinals: Vec<(Vec<u8>, u32)> =
                got.iter().map(|i| (i.order_key.subject.as_bytes().to_vec(), i.order_key.local_ordinal)).collect();
            assert_eq!(got_subjects_and_ordinals, expected_subjects_and_ordinals);
        }
    }

    #[test]
    fn identical_multiset_regardless_of_thread_order() {
        // The packet's own acceptance-gate wording: "regardless of
        // thread order" -- unlike the permutation test above (single-
        // threaded, deterministic call order), this drives concurrent
        // producers racing on the SAME outbox, proving the Mutex-backed
        // storage doesn't lose or reorder anything under real
        // contention either.
        let outbox = Arc::new(ServerSemanticOutboxV1::new());
        let handles: Vec<_> = (0u32..8)
            .map(|n| {
                let outbox = Arc::clone(&outbox);
                thread::spawn(move || outbox.enqueue(intent(1, n, (n % 3) as u8)))
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let mut got = outbox.take_pending();
        assert_eq!(got.len(), 8);
        got.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
        let ordinals: Vec<u32> = got.iter().map(|i| i.order_key.local_ordinal).collect();

        // Independently recompute the expected order from the same
        // multiset (order-independent construction, not copy-pasted
        // from the concurrent run) and compare.
        let mut expected: Vec<u32> = (0u32..8).collect();
        expected.sort_by_key(|&n| (n % 3, n)); // subject byte, then local_ordinal (the only other varying field)
        assert_eq!(ordinals, expected);
    }
}
