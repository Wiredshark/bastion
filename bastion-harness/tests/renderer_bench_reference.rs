//! renderer-bench W1 reference test (harness side).
//!
//! Two jobs, per the W1 handoff:
//! 1. FRESHNESS BINDING — the mirrored contract artifacts under
//!    `readme/renderer-bench/` must hash-match the sha256 the handoff bundle
//!    pinned (`prerequisite_artifact_sha256`). A drifted vector file is a
//!    voided contract, and this test is the machine check that refuses it —
//!    the freshness rule as a test rather than a ritual.
//! 2. REFERENCE RECOMPUTATION — the harness package independently drives the
//!    production encoder against the reviewed vectors (the same bytes the
//!    Python independent verifier reproduced), so a green here means
//!    harness-side consumers see the exact contract common's own tests see.

use common::renderer_bench::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn bench_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../readme/renderer-bench")
}

fn read(file: &str) -> Vec<u8> {
    let p = bench_dir().join(file);
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn sha_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn mirrored_artifacts_match_handoff_pins() {
    let handoff: Value =
        serde_json::from_slice(&read("renderer-r0d-handoff-w1-v1.json")).expect("handoff JSON");
    let pins = handoff["prerequisite_artifact_sha256"]
        .as_object()
        .expect("pin table");
    // Every mirrored prerequisite present in the repo must match its pin.
    for file in [
        "renderer-r0d-canonical-vectors-v1.json",
        "renderer-r0d-w1-reviewed-vectors-v1.json",
        "renderer-r0d-contradiction-ledger-v1.json",
    ] {
        let expected = pins[file].as_str().expect("pin");
        let actual = sha_hex(&read(file));
        assert_eq!(actual, expected, "{file}: drifted from the W0 pin — the contract is void");
    }
}

#[test]
fn reference_recomputation_spot_vectors() {
    let v: Value = serde_json::from_slice(&read("renderer-r0d-w1-reviewed-vectors-v1.json"))
        .expect("vectors JSON");
    let v = &v["vectors"];

    // Quadruped manifest through the production encoder.
    let m = FixtureManifestV1 {
        scenario_id: "qsmall-pig-v1".into(),
        scenario_seed: 1,
        worldgen_seed: 2,
        rtsim_seed: 3,
        simulation_tps: 30,
        arena_origin_mm: [0, 0, 0],
        camera_script_id: "static-origin".into(),
        graphics_manifest_version: 2,
        artifact_schema_version: 2,
        entities: vec![FixtureEntityV1 {
            semantic_id: 7,
            per_entity_seed: 1007,
            body: BenchBodyV1::QuadrupedSmall {
                species: 0,
                body_type: 0,
            },
            loadout: vec![],
            spawn_position_mm: [7000, 0, 0],
            orientation_turns_u32: 0,
            movement: MovementV1::None,
            animation: AnimationV1::None,
        }],
    };
    let bytes = m.encode().expect("encodes");
    let e = &v["manifest_nonhumanoid_quadruped_small_v1"];
    assert_eq!(
        sha_hex(&bytes),
        e["payload_sha256"].as_str().unwrap(),
        "payload digest"
    );
    let domain = FixtureManifestV1::domain_sha256(&bytes);
    let domain_hex: String = domain.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(domain_hex, e["domain_sha256"].as_str().unwrap(), "domain digest");

    // Item identity: component order is semantic (reversed ≠ forward).
    let nested = ItemDefV1::Modular {
        base: "test.modular.weapon".into(),
        components: vec![
            ItemDefV1::Simple("test.material.iron".into()),
            ItemDefV1::Compound {
                base: "test.component.hilt".into(),
                components: vec![
                    ItemDefV1::Simple("test.material.oak".into()),
                    ItemDefV1::Simple("test.material.leather".into()),
                ],
            },
        ],
    };
    let fwd = nested.encode_canonical().unwrap();
    assert_eq!(
        sha_hex(&fwd),
        v["item_definition_nested_v1"]["sha256"].as_str().unwrap()
    );
}
