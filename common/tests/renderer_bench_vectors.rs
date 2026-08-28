//! renderer-bench W1 vector tests: the Rust encoder must reproduce, byte for
//! byte, the W0 canonical vectors and the W1 reviewed vectors — both read
//! from the checked-in JSON under `readme/renderer-bench/` (the production
//! encoder never regenerates or blesses expected bytes; these files are the
//! authority and Python produced them).

use veloren_common::renderer_bench::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn vectors(file: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../readme/renderer-bench")
        .join(file);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str::<Value>(&raw).expect("valid JSON")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex(&h.finalize())
}

/// Assert one produced byte-string against a vector entry (length + hex +
/// whichever digests the entry carries).
fn assert_vector(entry: &Value, produced: &[u8], name: &str) {
    if let Some(l) = entry.get("length").and_then(Value::as_u64) {
        assert_eq!(produced.len() as u64, l, "{name}: length");
    }
    if let Some(h) = entry.get("hex").and_then(Value::as_str) {
        assert_eq!(hex(produced), h, "{name}: bytes");
    }
    if let Some(s) = entry.get("sha256").and_then(Value::as_str) {
        assert_eq!(sha_hex(produced), s, "{name}: sha256");
    }
    if let Some(s) = entry.get("payload_sha256").and_then(Value::as_str) {
        assert_eq!(sha_hex(produced), s, "{name}: payload_sha256");
    }
    if let Some(s) = entry.get("domain_sha256").and_then(Value::as_str) {
        let d = FixtureManifestV1::domain_sha256(produced);
        assert_eq!(hex(&d), s, "{name}: domain_sha256");
    }
}

// ── Reference fixture builders (mirror the W0 producer scripts exactly) ──

fn humanoid() -> BenchBodyV1 {
    BenchBodyV1::Humanoid {
        species: 3,
        body_type: 1,
        hair_style: 2,
        beard: 0,
        eyes: 1,
        accessory: 0,
        hair_color: 4,
        skin: 7,
        eye_color: 11,
        height_scale: 128,
    }
}

fn humanoid_with(f: impl FnOnce(&mut BenchBodyV1)) -> BenchBodyV1 {
    let mut b = humanoid();
    f(&mut b);
    b
}

fn entity(id: u32, body: BenchBodyV1, loadout: Vec<LoadoutEntryV1>) -> FixtureEntityV1 {
    FixtureEntityV1 {
        semantic_id: id,
        per_entity_seed: 1000 + id as u64,
        body,
        loadout,
        spawn_position_mm: [id as i32 * 1000, 0, 0],
        orientation_turns_u32: 0,
        movement: MovementV1::None,
        animation: AnimationV1::None,
    }
}

fn base(scenario: &str, entities: Vec<FixtureEntityV1>) -> FixtureManifestV1 {
    FixtureManifestV1 {
        scenario_id: scenario.into(),
        scenario_seed: 1,
        worldgen_seed: 2,
        rtsim_seed: 3,
        simulation_tps: 30,
        arena_origin_mm: [0, 0, 0],
        camera_script_id: "static-origin".into(),
        graphics_manifest_version: 2,
        artifact_schema_version: 2,
        entities,
    }
}

fn empty_manifest() -> FixtureManifestV1 {
    FixtureManifestV1 {
        scenario_id: "empty-v1".into(),
        scenario_seed: 1,
        worldgen_seed: 2,
        rtsim_seed: 3,
        simulation_tps: 30,
        arena_origin_mm: [0, 0, 0],
        camera_script_id: "static-origin".into(),
        graphics_manifest_version: 2,
        artifact_schema_version: 2,
        entities: vec![],
    }
}

fn frame_token() -> SemanticFrameTokenV1 {
    let seq = |start: u8| -> [u8; 32] {
        let mut a = [0u8; 32];
        for (i, b) in a.iter_mut().enumerate() {
            *b = start + i as u8;
        }
        a
    };
    SemanticFrameTokenV1 {
        run_id: seq(0x00),
        frame_index: 7,
        sim_tick: 123,
        script_cursor: 9,
        readback_cursor: 9,
        manifest_sha256: seq(0x20),
        script_sha256: seq(0x40),
        parent_frame_sha256: seq(0x60),
    }
}

/// The single-leaf hierarchy for one entity id (the W0 vector's exact
/// parameters: FigureIdentity / field 0x09000001 / ENUM / StableEntity /
/// payload = Body family Humanoid as u16 0).
fn hierarchy(entity_id: u32) -> ([u8; 32], [u8; 32], [u8; 32], [u8; 32], [u8; 32]) {
    let schema = oracle_schema_hash();
    let owner_key = entity_id.to_le_bytes();
    let payload = 0u16.to_le_bytes();
    let leaf = leaf_hash(
        &schema,
        Domain::FigureIdentity,
        0x0900_0001,
        WireType::Enum,
        OwnerKind::StableEntity,
        &owner_key,
        &payload,
    );
    let owner = owner_root(
        &schema,
        OwnerKind::StableEntity,
        &owner_key,
        &[(0x0900_0001, leaf)],
    );
    let composite = {
        let mut v = vec![OwnerKind::StableEntity as u8];
        v.extend_from_slice(&(owner_key.len() as u32).to_le_bytes());
        v.extend_from_slice(&owner_key);
        v
    };
    let domain = domain_root(&schema, Domain::FigureIdentity, &[(composite, owner)]);
    let token = frame_token().encode();
    let frame = frame_root(&schema, &token, &[(Domain::FigureIdentity, domain)]);
    let run = run_root(&schema, "run-vector-v1", &[(token, frame)], 0);
    (leaf, owner, domain, frame, run)
}

// ── Canonical vectors ──

#[test]
fn canonical_primitives() {
    let v = vectors("renderer-r0d-canonical-vectors-v1.json");
    let v = &v["vectors"];

    let mut w = CanonicalWriter::new();
    w.u32(42);
    assert_eq!(hex(&w.into_bytes()), v["primitive_nonzero_u32_42"]["hex"]);

    let mut w = CanonicalWriter::new();
    w.opt(None::<&u32>, |_, _| Ok(())).unwrap();
    assert_eq!(hex(&w.into_bytes()), v["primitive_option_none"]["hex"]);

    let mut w = CanonicalWriter::new();
    w.opt(Some(&42u32), |w, x| {
        w.u32(*x);
        Ok(())
    })
    .unwrap();
    assert_eq!(hex(&w.into_bytes()), v["primitive_option_some_u32_42"]["hex"]);

    let mut w = CanonicalWriter::new();
    w.seq(&[1u32, 2u32], |w, x| {
        w.u32(*x);
        Ok(())
    })
    .unwrap();
    assert_eq!(hex(&w.into_bytes()), v["primitive_sequence_u32_1_2"]["hex"]);

    let mut w = CanonicalWriter::new();
    w.f32_finite(-0.0).unwrap();
    assert_eq!(
        hex(&w.into_bytes()),
        v["primitive_f32_negative_zero_normalized"]["hex"],
        "-0.0 must normalize to +0.0"
    );

    let mut w = CanonicalWriter::new();
    w.f32_finite(1.5).unwrap();
    assert_eq!(hex(&w.into_bytes()), v["primitive_f32_1_5"]["hex"]);
}

#[test]
fn non_finite_f32_refuses() {
    let mut w = CanonicalWriter::new();
    assert_eq!(w.f32_finite(f32::NAN), Err(EncodeError::NonFiniteF32));
    assert_eq!(w.f32_finite(f32::INFINITY), Err(EncodeError::NonFiniteF32));
    assert_eq!(
        w.f32_finite(f32::NEG_INFINITY),
        Err(EncodeError::NonFiniteF32)
    );
}

#[test]
fn canonical_empty_manifest() {
    let v = vectors("renderer-r0d-canonical-vectors-v1.json");
    let payload = empty_manifest().encode().expect("encodes");
    assert_vector(
        &v["vectors"]["manifest_empty_corrected_v1"],
        &payload,
        "manifest_empty_corrected_v1",
    );
}

#[test]
fn canonical_frame_token() {
    let v = vectors("renderer-r0d-canonical-vectors-v1.json");
    let tok = frame_token().encode();
    assert_vector(&v["vectors"]["frame_token_v1"], &tok, "frame_token_v1");
    // Round-trip + strict tail.
    let back = SemanticFrameTokenV1::decode(&tok).expect("decodes");
    assert_eq!(back.encode(), tok);
    let mut trailing = tok.clone();
    trailing.push(0);
    assert_eq!(
        SemanticFrameTokenV1::decode(&trailing),
        Err(DecodeError::TrailingBytes),
        "trailing bytes must fail closed"
    );
}

#[test]
fn canonical_schema_hash_and_hierarchy() {
    let v = vectors("renderer-r0d-canonical-vectors-v1.json");
    let v = &v["vectors"];
    assert_eq!(
        hex(&oracle_schema_hash()),
        v["synthetic_oracle_schema_hash"].as_str().unwrap()
    );
    let (leaf, owner, domain, frame, run) = hierarchy(42);
    let e = &v["single_leaf_hierarchy_entity_42"];
    assert_eq!(hex(&leaf), e["leaf"].as_str().unwrap(), "leaf");
    assert_eq!(hex(&owner), e["owner"].as_str().unwrap(), "owner");
    assert_eq!(hex(&domain), e["domain"].as_str().unwrap(), "domain");
    assert_eq!(hex(&frame), e["frame"].as_str().unwrap(), "frame");
    assert_eq!(hex(&run), e["run"].as_str().unwrap(), "run");
}

#[test]
fn hierarchy_mutation_changes_every_root() {
    // The W0 mutation vector: entity 43 must change leaf, owner, domain,
    // frame AND run (changed_root_mask = all five).
    let v = vectors("renderer-r0d-canonical-vectors-v1.json");
    let e = &v["vectors"]["single_leaf_hierarchy_entity_43_mutation"];
    let (leaf, owner, domain, frame, run) = hierarchy(43);
    assert_eq!(hex(&leaf), e["leaf"].as_str().unwrap());
    assert_eq!(hex(&owner), e["owner"].as_str().unwrap());
    assert_eq!(hex(&domain), e["domain"].as_str().unwrap());
    assert_eq!(hex(&frame), e["frame"].as_str().unwrap());
    assert_eq!(hex(&run), e["run"].as_str().unwrap());
    let a = hierarchy(42);
    assert!(
        a.0 != leaf && a.1 != owner && a.2 != domain && a.3 != frame && a.4 != run,
        "every root must move when the entity id moves"
    );
}

// ── W1 reviewed vectors ──

const SLOT_NAMES: [&str; 22] = [
    "lantern",
    "glider",
    "shoulders",
    "chest",
    "belt",
    "hands",
    "legs",
    "feet",
    "back",
    "ring1",
    "ring2",
    "neck",
    "head",
    "tabard",
    "bag1",
    "bag2",
    "bag3",
    "bag4",
    "active-main",
    "active-off",
    "inactive-main",
    "inactive-off",
];

#[test]
fn reviewed_manifests() {
    let v = vectors("renderer-r0d-w1-reviewed-vectors-v1.json");
    let v = &v["vectors"];

    let m = base("humanoid-empty-v1", vec![entity(1, humanoid(), vec![])]);
    assert_vector(
        &v["manifest_humanoid_empty_v1"],
        &m.encode().unwrap(),
        "manifest_humanoid_empty_v1",
    );

    let all_slots: Vec<LoadoutEntryV1> = SLOT_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| LoadoutEntryV1 {
            slot: i as u8,
            item: ItemDefV1::Simple(format!("test.r0d.slot.{name}")),
        })
        .collect();
    let m = base(
        "humanoid-all-slots-v1",
        vec![entity(
            1,
            humanoid_with(|b| {
                if let BenchBodyV1::Humanoid { body_type, .. } = b {
                    *body_type = 0;
                }
            }),
            all_slots,
        )],
    );
    assert_vector(
        &v["manifest_humanoid_all_slots_v1"],
        &m.encode().unwrap(),
        "manifest_humanoid_all_slots_v1",
    );

    let m = base(
        "qsmall-pig-v1",
        vec![entity(
            7,
            BenchBodyV1::QuadrupedSmall {
                species: 0,
                body_type: 0,
            },
            vec![],
        )],
    );
    assert_vector(
        &v["manifest_nonhumanoid_quadruped_small_v1"],
        &m.encode().unwrap(),
        "manifest_nonhumanoid_quadruped_small_v1",
    );

    // Author-order normalization: reversed author order and sorted control
    // must produce IDENTICAL bytes.
    let hair = |style: u8| {
        humanoid_with(|b| {
            if let BenchBodyV1::Humanoid { hair_style, .. } = b {
                *hair_style = style;
            }
        })
    };
    let reversed = base(
        "reverse-author-order-v1",
        vec![entity(2, hair(8), vec![]), entity(1, hair(3), vec![])],
    );
    let sorted = base(
        "reverse-author-order-v1",
        vec![entity(1, hair(3), vec![]), entity(2, hair(8), vec![])],
    );
    let reversed_bytes = reversed.encode().unwrap();
    let sorted_bytes = sorted.encode().unwrap();
    assert_eq!(
        reversed_bytes, sorted_bytes,
        "author order must normalize away"
    );
    assert_vector(
        &v["manifest_reverse_author_order_v1"],
        &reversed_bytes,
        "manifest_reverse_author_order_v1",
    );
    assert_vector(
        &v["manifest_reverse_author_order_sorted_control_v1"],
        &sorted_bytes,
        "manifest_reverse_author_order_sorted_control_v1",
    );
}

#[test]
fn reviewed_presentation_state() {
    let v = vectors("renderer-r0d-w1-reviewed-vectors-v1.json");
    let p = CharacterPresentationStateV1 {
        class: 13,
        stage: Some(4),
        input: Some(InputKindV1::Ability { ability_index: 7 }),
        is_riding: true,
        is_gliding: true,
        is_dead: false,
    };
    assert_vector(
        &v["vectors"]["character_presentation_nested_options_v1"],
        &p.encode(),
        "character_presentation_nested_options_v1",
    );
}

fn nested_item() -> ItemDefV1 {
    ItemDefV1::Modular {
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
    }
}

#[test]
fn reviewed_item_identity_preserves_component_order() {
    let v = vectors("renderer-r0d-w1-reviewed-vectors-v1.json");
    let v = &v["vectors"];

    let encode = |item: &ItemDefV1| -> Vec<u8> { item.encode_canonical().unwrap() };

    let nested = nested_item();
    let nested_bytes = encode(&nested);
    assert_vector(
        &v["item_definition_nested_v1"],
        &nested_bytes,
        "item_definition_nested_v1",
    );

    let reversed = match nested_item() {
        ItemDefV1::Modular { base, mut components } => {
            components.reverse();
            ItemDefV1::Modular { base, components }
        },
        _ => unreachable!(),
    };
    let reversed_bytes = encode(&reversed);
    assert_vector(
        &v["item_definition_nested_reversed_components_v1"],
        &reversed_bytes,
        "item_definition_nested_reversed_components_v1",
    );
    assert_ne!(
        nested_bytes, reversed_bytes,
        "component order is semantic and must NOT normalize away"
    );
}

#[test]
fn reviewed_complete_figure_key() {
    let v = vectors("renderer-r0d-w1-reviewed-vectors-v1.json");
    let f = FigureKeyProjectionV1 {
        body: humanoid(),
        item_key: None,
        extra: Some(FigureCacheKeyV1 {
            third_person: Some(ThirdPersonKeyV1 {
                head: Some("test.armor.head".into()),
                shoulder: Some("test.armor.shoulder".into()),
                chest: Some("test.armor.chest".into()),
                belt: Some("test.armor.belt".into()),
                back: Some("test.armor.back".into()),
                pants: Some("test.armor.legs".into()),
            }),
            tool: Some(ToolSlotsV1 {
                active: Some(ToolKeyV1::Simple("test.weapon.sword".into())),
                second: Some(ToolKeyV1::Modular {
                    a: "test.weapon.bow".into(),
                    b: "test.material.oak".into(),
                    hands: 1,
                }),
            }),
            lantern: Some("test.lantern".into()),
            glider: Some("test.glider".into()),
            foot: Some("test.armor.feet".into()),
            head: Some("test.armor.head".into()),
            hand: None,
        }),
    };
    assert_vector(
        &v["vectors"]["complete_humanoid_figure_key_v1"],
        &f.encode().unwrap(),
        "complete_humanoid_figure_key_v1",
    );
}

// ── Fail-closed semantics ──

#[test]
fn encode_refusals() {
    // Zero id.
    let m = base("bad", vec![entity(0, humanoid(), vec![])]);
    assert_eq!(m.encode(), Err(EncodeError::InvalidSemanticIds));
    // Duplicate id.
    let m = base(
        "bad",
        vec![entity(5, humanoid(), vec![]), entity(5, humanoid(), vec![])],
    );
    assert_eq!(m.encode(), Err(EncodeError::InvalidSemanticIds));
    // Duplicate slot.
    let dup = vec![
        LoadoutEntryV1 {
            slot: 3,
            item: ItemDefV1::Simple("a".into()),
        },
        LoadoutEntryV1 {
            slot: 3,
            item: ItemDefV1::Simple("b".into()),
        },
    ];
    let m = base("bad", vec![entity(1, humanoid(), dup)]);
    assert_eq!(m.encode(), Err(EncodeError::DuplicateEquipmentSlot));
}

#[test]
fn decode_round_trip_and_fail_closed() {
    let m = base(
        "roundtrip-v1",
        vec![
            entity(2, humanoid(), vec![LoadoutEntryV1 {
                slot: 1,
                item: nested_item(),
            }]),
            entity(1, BenchBodyV1::QuadrupedSmall { species: 0, body_type: 0 }, vec![]),
        ],
    );
    let bytes = m.encode().unwrap();
    let back = FixtureManifestV1::decode(&bytes).expect("round-trips");
    // Decode returns the NORMALIZED (sorted) form; re-encode is identical.
    assert_eq!(back.encode().unwrap(), bytes);

    // Trailing byte fails closed.
    let mut t = bytes.clone();
    t.push(0xAA);
    assert_eq!(FixtureManifestV1::decode(&t), Err(DecodeError::TrailingBytes));

    // Unknown body tag fails closed: corrupt the first entity's body tag.
    // (Find it structurally: header is magic4+ver4+lp(id)+8*3+4+4*3+lp(cam)
    // +4+4+count4, then id4+seed8 → body tag offset.)
    let header = 4 + 4 + (4 + "roundtrip-v1".len()) + 24 + 4 + 12 + (4 + "static-origin".len()) + 8 + 4;
    let body_tag_at = header + 4 + 8;
    let mut u = bytes.clone();
    u[body_tag_at] = 0xEE;
    assert!(matches!(
        FixtureManifestV1::decode(&u),
        Err(DecodeError::UnknownTag { context: "body", .. })
    ));

    // Bad magic fails closed.
    let mut b = bytes.clone();
    b[0] = b'X';
    assert_eq!(FixtureManifestV1::decode(&b), Err(DecodeError::BadMagic));
}

#[test]
fn readback_registry_is_exactly_once() {
    let mut r = RendererBenchReadbacks::default();
    assert!(r.claim(7), "first claim wins");
    assert!(!r.claim(7), "second claim must refuse");
    assert!(r.is_claimed(7));
    assert!(!r.is_claimed(8));
}

// ── W3: ClientProjection vectors (independent Python producer:
// readme/renderer-bench/w3_client_projection_vectors_v1.py) ──

#[test]
fn w3_client_projection_reproduces_python_vectors() {
    let v = vectors("w3-client-projection-vectors-v1.json");
    let schema = oracle_schema_hash();

    // The contractual tags themselves, pinned against the JSON.
    let tags = &v["tags"];
    assert_eq!(Domain::ClientProjection as u64, tags["domain"].as_u64().unwrap());
    assert_eq!(CLIENT_PROJECTION_LEAF as u64, tags["leaf_id"].as_u64().unwrap());
    assert_eq!(WireType::FixedI32 as u64, tags["wire_type"].as_u64().unwrap());
    assert_eq!(OwnerKind::StableEntity as u64, tags["owner_kind"].as_u64().unwrap());

    // Single entity: leaf, owner root, composite, one-entry domain root.
    let s = &v["single"];
    let id = s["semantic_id"].as_u64().unwrap() as u32;
    let mm: Vec<i64> = s["mm"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
    let mm = [mm[0] as i32, mm[1] as i32, mm[2] as i32];
    let (composite, oroot) = client_projection_owner(&schema, id, mm);
    assert_eq!(hex(&composite), s["composite"].as_str().unwrap(), "composite");
    assert_eq!(hex(&oroot), s["owner_root"].as_str().unwrap(), "owner_root");
    let leaf = leaf_hash(
        &schema,
        Domain::ClientProjection,
        CLIENT_PROJECTION_LEAF,
        WireType::FixedI32,
        OwnerKind::StableEntity,
        &id.to_le_bytes(),
        &{
            let mut p = Vec::new();
            for c in mm {
                p.extend_from_slice(&c.to_le_bytes());
            }
            p
        },
    );
    assert_eq!(hex(&leaf), s["leaf"].as_str().unwrap(), "leaf");
    let droot = domain_root(&schema, Domain::ClientProjection, &[(composite, oroot)]);
    assert_eq!(hex(&droot), s["domain_root"].as_str().unwrap(), "single domain_root");

    // Three entities, sorted by semantic id (incl i32 extremes).
    let t = &v["triple"];
    let owners: Vec<(Vec<u8>, [u8; 32])> = t["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let id = e["semantic_id"].as_u64().unwrap() as u32;
            let mm: Vec<i64> =
                e["mm"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
            client_projection_owner(&schema, id, [mm[0] as i32, mm[1] as i32, mm[2] as i32])
        })
        .collect();
    let droot = domain_root(&schema, Domain::ClientProjection, &owners);
    assert_eq!(hex(&droot), t["domain_root"].as_str().unwrap(), "triple domain_root");

    // The empty domain (a client that resolved nothing yet).
    let droot = domain_root(&schema, Domain::ClientProjection, &[]);
    assert_eq!(hex(&droot), v["empty_domain_root"].as_str().unwrap(), "empty domain_root");
}

// ── W4: PassDraw + VisualStructure vectors (independent producer:
// readme/renderer-bench/w4_visual_domain_vectors_v1.py) ──

#[test]
fn w4_visual_domains_reproduce_python_vectors() {
    let v = vectors("w4-visual-domain-vectors-v1.json");
    let schema = oracle_schema_hash();

    // Pin the contractual tags against the JSON.
    assert_eq!(Domain::PassDraw as u64, v["tags"]["pass_draw"]["domain"].as_u64().unwrap());
    assert_eq!(PASS_DRAW_LEAF as u64, v["tags"]["pass_draw"]["leaf_id"].as_u64().unwrap());
    assert_eq!(
        Domain::VisualStructure as u64,
        v["tags"]["visual_structure"]["domain"].as_u64().unwrap()
    );
    assert_eq!(
        VISUAL_STRUCTURE_LEAF as u64,
        v["tags"]["visual_structure"]["leaf_id"].as_u64().unwrap()
    );
    assert_eq!(WireType::Struct as u64, v["tags"]["wire_type"].as_u64().unwrap());

    let s = &v["stats"];
    let stats = BenchSceneStatsV1 {
        pass_count: s["pass_count"].as_u64().unwrap() as u32,
        draw_count: s["draw_count"].as_u64().unwrap() as u32,
        instances: s["instances"].as_u64().unwrap() as u32,
        geometry_units: s["geometry_units"].as_u64().unwrap(),
        terrain_chunks: s["terrain_chunks"].as_u64().unwrap() as u32,
        visible_terrain_chunks: s["visible_terrain_chunks"].as_u64().unwrap() as u32,
        shadow_terrain_chunks: s["shadow_terrain_chunks"].as_u64().unwrap() as u32,
        figure_draw_count: s["figure_draw_count"].as_u64().unwrap() as u32,
    };
    let frame_index = v["frame_index"].as_u64().unwrap() as u32;

    let (_, pd_owner) = pass_draw_owner(&schema, 0, &stats);
    assert_eq!(hex(&pd_owner), v["pass_draw"]["owner_root"].as_str().unwrap(), "pd owner");
    let (_, vs_owner) = visual_structure_owner(&schema, frame_index, &stats);
    assert_eq!(
        hex(&vs_owner),
        v["visual_structure"]["owner_root"].as_str().unwrap(),
        "vs owner"
    );
    let domains = visual_domains(&schema, frame_index, &stats);
    assert_eq!(
        hex(&domains.pass_draw_root),
        v["pass_draw"]["domain_root"].as_str().unwrap(),
        "pd domain"
    );
    assert_eq!(
        hex(&domains.visual_structure_root),
        v["visual_structure"]["domain_root"].as_str().unwrap(),
        "vs domain"
    );
}

#[test]
fn w3_shared_composite_matches_python_shape() {
    // The ONE owner-key implementation both sides call: kind byte then
    // length-prefixed LE id — pinned against the independent producer.
    let c = stable_entity_composite(0x01020304);
    assert_eq!(c, vec![3u8, 4, 0, 0, 0, 0x04, 0x03, 0x02, 0x01]);
}
