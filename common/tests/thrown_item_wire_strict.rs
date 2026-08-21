//! W5 leg forensics: the DET-NET-017 strict decode refused a live
//! `ServerGeneral::CompSync` frame carrying comp-packet variant 12
//! (ThrownItem) — consumed 56 of 64 bytes, in ONE process. This test
//! reproduces the exact wire config (bincode legacy, the network
//! crate's) on a ThrownItem and asserts full consumption — the
//! smallest harness that can hold the asymmetry still.

use veloren_common::comp::item::{Item, ThrownItem};

#[test]
fn thrown_item_bincode_legacy_roundtrip_consumes_fully() {
    let item = Item::new_from_asset("common.items.weapons.tool.throwable_stone")
        .or_else(|_| Item::new_from_asset("common.items.food.mushroom"))
        .expect("some test item loads");
    let thrown = ThrownItem(item);
    let bytes = bincode::serde::encode_to_vec(&thrown, bincode::config::legacy())
        .expect("encodes");
    let (_decoded, consumed): (ThrownItem, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::legacy())
            .expect("decodes");
    assert_eq!(
        consumed,
        bytes.len(),
        "ThrownItem wire asymmetry: encoded {} bytes, decode consumed {}",
        bytes.len(),
        consumed
    );
}
