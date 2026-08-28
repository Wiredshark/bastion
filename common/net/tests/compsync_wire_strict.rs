//! W5 leg forensics: a live 64-byte `ServerGeneral::CompSync` frame
//! (one entry, uid 633, 12-byte comp payload) decoded 56/64 in ONE
//! process. Reproduce the exact frame class through the exact wire
//! config and assert full consumption.

use common::uid::Uid;
use std::num::NonZeroU64;
use vek::Vec3;
use veloren_common_net::msg::{EcsCompPacket, ServerGeneral};
use veloren_common_net::sync::CompSyncPackage;

#[test]
fn one_entry_pos_compsync_roundtrips_fully() {
    let mut package: CompSyncPackage<EcsCompPacket> = CompSyncPackage::new();
    package.comp_inserted(
        Uid(NonZeroU64::new(633).unwrap()),
        common::comp::Pos(Vec3::new(1.0_f32, 2.0, 3.0)),
    );
    let msg = ServerGeneral::CompSync(
        package,
        common::apex::physics_generation::PhysicsGenerationV1::default(),
    );
    let bytes =
        bincode::serde::encode_to_vec(&msg, bincode::config::legacy()).expect("encodes");
    let (_decoded, consumed): (ServerGeneral, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::legacy()).expect("decodes");
    assert_eq!(
        consumed,
        bytes.len(),
        "CompSync wire asymmetry: encoded {} bytes ({:?}...), decode consumed {}",
        bytes.len(),
        &bytes[..bytes.len().min(24)],
        consumed
    );
}
