use bastion_renderer_r0d::{
    cbor::ValidatedCanonicalBytesV1,
    shared_adapter::renderer_corpus_digest_v1,
    tape::{DomainRank, FinalizedTapeV1, TapeKeyV1, TapeRecordV1},
};

fn record(tick: u64, domain: DomainRank, owner: u8, payload: u8) -> TapeRecordV1 {
    TapeRecordV1 {
        key: TapeKeyV1 {
            simulation_tick: tick,
            render_frame_or_zero: 0,
            domain_rank: domain as u16,
            authority_rank: 1,
            owner_digest: [owner; 32],
            leaf_kind_rank: 1,
            local_ordinal: 0,
        },
        payload: vec![payload],
    }
}

#[test]
fn r0d_fixture_is_producer_order_independent() {
    let record_a = record(1, DomainRank::SceneProjection, 1, 2);
    let record_b = record(2, DomainRank::RenderSelection, 3, 4);
    let first = FinalizedTapeV1::finalize(vec![record_a.clone(), record_b.clone()]).unwrap();
    let second = FinalizedTapeV1::finalize(vec![record_b, record_a]).unwrap();
    assert_eq!(first.final_root(), second.final_root());

    let canonical = ValidatedCanonicalBytesV1::validate(&[0x01]).unwrap();
    assert_eq!(
        renderer_corpus_digest_v1(&canonical).unwrap(),
        renderer_corpus_digest_v1(&canonical).unwrap()
    );
}
