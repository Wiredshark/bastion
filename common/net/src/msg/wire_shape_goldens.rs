//! `APEX-WIRE-SHAPE-GOLDENS` (WSG-1) — per-variant frozen encode vectors.
//!
//! **The gap this closes, found in `T5.2`/O2.** The frozen
//! `NET_ENVELOPE_PROFILE` table digests the TAG VOCABULARY — directions,
//! streams, payload-schema ids, encodings, the causality profile. It does
//! NOT digest the payload schemas' CONTENTS: `ClientGeneral` and
//! `ServerGeneral` are opaque to it. During `T5.2` two message shapes
//! changed and a variant was added, and the profile golden **passed
//! unchanged**. An old client against a new server would mis-decode with
//! no digest disagreement anywhere to catch it.
//!
//! `T5.2` narrowed that by bumping the payload-schema LABELS `v1 -> v2`,
//! which does move the profile root. But the label bump is **voluntary**:
//! nothing forces a future author to remember it when they change a
//! message. These goldens make shape drift **mechanical** — one frozen
//! encode vector per variant, through the real encoder, so a changed
//! field type or order fails a test that names the variant.
//!
//! **Coverage is a PINNED OPEN SET, not a red build.** WSG-1 lands the
//! mechanism plus the four variants `T5.2` touched and could not detect;
//! the remaining variants are NAMED with a pinned count. A variant in
//! neither list fails immediately, so the set cannot grow silently, but
//! the interim is honest rather than noisy — a build left red from WSG-1
//! until WSG-2 closes would teach everyone to ignore it. WSG-2 burns the
//! uncovered count to zero and flips the assertion to all-covered.

use common::apex::digest::hash_artifact_bytes_v1;

/// One variant's frozen encode vector.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct WireShapeGoldenV1 {
    /// `"ClientGeneral"` or `"ServerGeneral"`.
    pub payload_schema: &'static str,
    /// The variant this vector is for. Named, so a drift report says
    /// WHICH message changed rather than that something did.
    pub variant: &'static str,
    /// `sha256` of the representative instance's encoding, through the
    /// same bincode config the wire uses.
    pub digest_hex: &'static str,
}

/// The frozen vectors.
///
/// Seeded with exactly the variants `T5.2` changed and the profile root
/// could not see: the two weather messages that gained a snapshot id, the
/// client physics report that gained a snapshot reference, and the
/// receipt variant that is entirely new.
pub const WIRE_SHAPE_GOLDENS: &[WireShapeGoldenV1] = &[
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "PlayerPhysics",
        digest_hex: "sha256:e6885cf72ccc0929d047e453c88e1d18ac697fe5595d449f583518e2f02b007b",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "WeatherUpdate",
        digest_hex: "sha256:de60d64c5c8654c91146e9ce8ab8dbf4d9e93b941a4b4a2d6240de221933fd99",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "LocalWindUpdate",
        digest_hex: "sha256:a30ae0f51538e9309084429c09ba88a91b80d49f81de8d9220d984369ae406e2",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "InputReceipt",
        digest_hex: "sha256:5d3eec1882c6064e2f508bb44a01dca36978d79e3c0a046e05f25233d4981b78",
    },
    // WSG-2 chunk 1: the server-authoritative core first — a mis-decode
    // of a sync package corrupts every entity on the client, which is the
    // largest blast radius on this wire.
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "EntitySync",
        digest_hex: "sha256:bdfa81ba96b618f3dad6255d94b9203f3be5372ec0893e7e385a72cfb0fb5939",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CompSync",
        digest_hex: "sha256:3f329c035f38e3993a88d4c4436a42a05af7265f3d38b03a11e6226daccd8dd9",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "DeleteEntity",
        digest_hex: "sha256:21d0ddef907a10b3b11a79410775293e3215a35c6eb7e5b8e04058afdfbb6dbd",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "SetPlayerEntity",
        digest_hex: "sha256:56dc33d03a09a24f01def8ab8d102d75e183f1814a7364aabb21f25470b055e1",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "Knockback",
        digest_hex: "sha256:7b5772a71a2cfac187e6220bfe8b7222d5bb81f460ed1372185fde692a0bb0aa",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "SpectatePosition",
        digest_hex: "sha256:b9d9f7761b80f5da88f47d872cf408bdf9351a4c94a2bfd3465fefb0a48b5141",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "SpectatorSuccess",
        digest_hex: "sha256:07f7341d09d9cdd6e70088970adc5f75a8e25fa2c124e9c14e85b44e708820ed",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "SetViewDistance",
        digest_hex: "sha256:48dad7f79489519389219a35e69b51d3d3710aa775edeb82803e227e8b1476cf",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "InvitePending",
        digest_hex: "sha256:6296f87eacce313a03299fbb7f5328a90c03ab542b08f56fb24f6bc6bbf241e0",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CharacterCreated",
        digest_hex: "sha256:22629bfd7924d1ca4897bab53d5f99123f5c351e49916becb0c1fb5033ad7c3a",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CharacterEdited",
        digest_hex: "sha256:ece617e99be7b1a6f6f80f67823cce2b64575a063039b32182db3f345ba37ade",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CharacterActionError",
        digest_hex: "sha256:1aa6c8c64dbdc3174f82999c0ac4ae2be07c231c0adc8aa049cbc7111b748773",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CharacterSuccess",
        digest_hex: "sha256:2594b6a92ebfb1c3312deb7d01c015fb95e9fbe9bd7bc6b527af07813ec7b910",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "ExitInGameSuccess",
        digest_hex: "sha256:42f4aeb81c1ef81f771f3de8abca9dcf66901c575530e7672e4b1146474ae650",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "UpdateRecipes",
        digest_hex: "sha256:28276425d45829d4e6f5e18aefbf1f62862f07260a904532fb6e2106dec973e6",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "SetPlayerRole",
        digest_hex: "sha256:fb9cca3fa5fafbd354aa4284804a0239a730dcbdd6459cddd0be75649a1a8dd5",
    },
];

include!("wire_shape_uncovered.rs");

/// Digest a representative instance the way the wire encodes it.
///
/// Exposed so WSG-2 uses the SAME function rather than re-deriving one —
/// two ways of computing a golden is two goldens.
pub fn golden_digest_v1<T: serde::Serialize>(value: &T) -> String {
    let bytes = bincode::serde::encode_to_vec(value, bincode::config::legacy())
        .expect("a wire message encodes");
    hash_artifact_bytes_v1(&bytes).digest.bytes.to_human_v1()
}

#[cfg(test)]
mod wire_shape_goldens_v1 {
    use super::*;
    use crate::msg::{
        ClientGeneral, ServerGeneral,
        input_receipt_wire::{CorrectionReasonWireV1, InputReceiptWireV1, SequenceStateWireV1},
    };
    use common::apex::weather_snapshot::WeatherSnapshotIdV1;
    use vek::Vec2;

    /// The representative instances. Fixed values, never defaults that
    /// might change under us, and never randomised — a golden over a
    /// value that can move is not a golden.
    fn client_player_physics() -> ClientGeneral {
        ClientGeneral::PlayerPhysics {
            pos: common::comp::Pos(Vec2::new(1.0, 2.0).with_z(3.0)),
            vel: common::comp::Vel(Vec2::new(4.0, 5.0).with_z(6.0)),
            ori: common::comp::Ori::default(),
            physics_generation:
                common::apex::physics_generation::PhysicsGenerationV1::from_legacy_counter_v1(7),
            weather_snapshot: WeatherSnapshotIdV1::from_sequence_v1(11),
        }
    }

    fn server_weather_update() -> ServerGeneral {
        ServerGeneral::WeatherUpdate(
            common::weather::SharedWeatherGrid::new(Vec2::new(2, 2)),
            WeatherSnapshotIdV1::from_sequence_v1(13),
        )
    }

    fn server_local_wind_update() -> ServerGeneral {
        ServerGeneral::LocalWindUpdate(
            Vec2::new(0.5, -1.5),
            WeatherSnapshotIdV1::from_sequence_v1(17),
        )
    }

    fn server_input_receipt() -> ServerGeneral {
        ServerGeneral::InputReceipt(InputReceiptWireV1 {
            sequence: SequenceStateWireV1::Rejected {
                sequence: 19,
                reason: CorrectionReasonWireV1::StaleGeneration,
            },
            server_tick: 23,
            generation:
                common::apex::physics_generation::PhysicsGenerationV1::from_legacy_counter_v1(29),
            correction: CorrectionReasonWireV1::StaleGeneration,
            exact_digest: [3u8; 32],
            quantised_policy: 31,
            quantised_digest: [5u8; 32],
        })
    }


    fn uid(n: u64) -> common::uid::Uid {
        common::uid::Uid(std::num::NonZeroU64::new(n).expect("nonzero"))
    }

    fn server_entity_sync() -> ServerGeneral {
        ServerGeneral::EntitySync(crate::sync::EntitySyncPackage {
            created_entities: vec![uid(2), uid(3)],
            deleted_entities: vec![uid(5)],
            sync_tick: 41,
            sequence: 43,
        })
    }

    fn server_comp_sync() -> ServerGeneral {
        ServerGeneral::CompSync(
            crate::sync::CompSyncPackage { comp_updates: Vec::new(), sync_tick: 47, sequence: 53 },
            common::apex::physics_generation::PhysicsGenerationV1::from_legacy_counter_v1(59),
        )
    }

    fn server_delete_entity() -> ServerGeneral { ServerGeneral::DeleteEntity(uid(61)) }

    fn server_set_player_entity() -> ServerGeneral { ServerGeneral::SetPlayerEntity(uid(67)) }

    fn server_knockback() -> ServerGeneral {
        ServerGeneral::Knockback(Vec2::new(1.5, -2.5).with_z(3.5))
    }

    fn server_spectate_position() -> ServerGeneral {
        ServerGeneral::SpectatePosition(Vec2::new(7.0, 8.0).with_z(9.0))
    }

    fn server_spectator_success() -> ServerGeneral {
        ServerGeneral::SpectatorSuccess(Vec2::new(10.0, 11.0).with_z(12.0))
    }

    fn server_set_view_distance() -> ServerGeneral { ServerGeneral::SetViewDistance(71) }

    fn server_invite_pending() -> ServerGeneral { ServerGeneral::InvitePending(uid(73)) }

    fn server_character_created() -> ServerGeneral {
        ServerGeneral::CharacterCreated(common::character::CharacterId(79))
    }

    fn server_character_edited() -> ServerGeneral {
        ServerGeneral::CharacterEdited(common::character::CharacterId(83))
    }

    fn server_character_action_error() -> ServerGeneral {
        ServerGeneral::CharacterActionError("wsg-2 fixture".to_owned())
    }

    fn server_character_success() -> ServerGeneral { ServerGeneral::CharacterSuccess }

    fn server_exit_in_game_success() -> ServerGeneral { ServerGeneral::ExitInGameSuccess }

    fn server_update_recipes() -> ServerGeneral { ServerGeneral::UpdateRecipes }

    fn server_set_player_role() -> ServerGeneral {
        ServerGeneral::SetPlayerRole(Some(common::comp::AdminRole::Moderator))
    }

    fn actual(variant: &str) -> String {
        match variant {
            "PlayerPhysics" => golden_digest_v1(&client_player_physics()),
            "WeatherUpdate" => golden_digest_v1(&server_weather_update()),
            "LocalWindUpdate" => golden_digest_v1(&server_local_wind_update()),
            "InputReceipt" => golden_digest_v1(&server_input_receipt()),
            "EntitySync" => golden_digest_v1(&server_entity_sync()),
            "CompSync" => golden_digest_v1(&server_comp_sync()),
            "DeleteEntity" => golden_digest_v1(&server_delete_entity()),
            "SetPlayerEntity" => golden_digest_v1(&server_set_player_entity()),
            "Knockback" => golden_digest_v1(&server_knockback()),
            "SpectatePosition" => golden_digest_v1(&server_spectate_position()),
            "SpectatorSuccess" => golden_digest_v1(&server_spectator_success()),
            "SetViewDistance" => golden_digest_v1(&server_set_view_distance()),
            "InvitePending" => golden_digest_v1(&server_invite_pending()),
            "CharacterCreated" => golden_digest_v1(&server_character_created()),
            "CharacterEdited" => golden_digest_v1(&server_character_edited()),
            "CharacterActionError" => golden_digest_v1(&server_character_action_error()),
            "CharacterSuccess" => golden_digest_v1(&server_character_success()),
            "ExitInGameSuccess" => golden_digest_v1(&server_exit_in_game_success()),
            "UpdateRecipes" => golden_digest_v1(&server_update_recipes()),
            "SetPlayerRole" => golden_digest_v1(&server_set_player_role()),
            other => panic!("{other} has a golden entry but no representative instance"),
        }
    }

    /// Every golden still matches its variant's encoding. A changed field
    /// type or order fails HERE, naming the variant — which is the whole
    /// point, because the profile root cannot see it.
    #[test]
    fn every_golden_still_matches_its_variants_encoding() {
        for golden in WIRE_SHAPE_GOLDENS {
            assert_eq!(
                actual(golden.variant),
                golden.digest_hex,
                "{}::{} changed shape on the wire. The envelope profile root CANNOT see this — \
                 that is why this table exists. If the change is deliberate, recompute this \
                 golden AND bump the payload-schema label, because old peers will mis-decode.",
                golden.payload_schema,
                golden.variant
            );
        }
    }

    /// Coverage is a pinned OPEN set: 4 covered, the rest named, and the
    /// counts pinned so neither list can drift silently.
    #[test]
    fn coverage_is_a_pinned_open_set() {
        assert_eq!(WIRE_SHAPE_GOLDENS.len(), 20, "the covered set changed");
        assert_eq!(UNCOVERED_CLIENTGENERAL_V1.len(), 36);
        assert_eq!(UNCOVERED_SERVERGENERAL_V1.len(), 32);
        // 1 + 36 = 37 ClientGeneral, 3 + 48 = 51 ServerGeneral, counted
        // from the enums at 71b1c87ca7.
        let covered_client =
            WIRE_SHAPE_GOLDENS.iter().filter(|g| g.payload_schema == "ClientGeneral").count();
        let covered_server =
            WIRE_SHAPE_GOLDENS.iter().filter(|g| g.payload_schema == "ServerGeneral").count();
        assert_eq!(covered_client + UNCOVERED_CLIENTGENERAL_V1.len(), 37);
        assert_eq!(covered_server + UNCOVERED_SERVERGENERAL_V1.len(), 51);
    }

    /// A variant may not be in BOTH lists, and the uncovered lists carry
    /// no duplicates. Either would make the pinned count a lie.
    #[test]
    fn no_variant_is_both_covered_and_uncovered() {
        for golden in WIRE_SHAPE_GOLDENS {
            let uncovered = match golden.payload_schema {
                "ClientGeneral" => UNCOVERED_CLIENTGENERAL_V1.as_slice(),
                "ServerGeneral" => UNCOVERED_SERVERGENERAL_V1.as_slice(),
                other => panic!("unknown payload schema {other}"),
            };
            assert!(
                !uncovered.contains(&golden.variant),
                "{} is listed as both covered and uncovered",
                golden.variant
            );
        }
        for list in [UNCOVERED_CLIENTGENERAL_V1.as_slice(), UNCOVERED_SERVERGENERAL_V1.as_slice()] {
            let mut sorted = list.to_vec();
            sorted.sort_unstable();
            let before = sorted.len();
            sorted.dedup();
            assert_eq!(sorted.len(), before, "an uncovered list has a duplicate");
        }
    }

    /// The pinned totals are checked against the ENUMS, not against
    /// themselves. A new variant added to either enum and to neither list
    /// fails here immediately — that is the tripwire the open set needs
    /// to stay honest during the WSG-1 -> WSG-2 interval.
    #[test]
    fn a_new_variant_in_neither_list_fails_immediately() {
        use std::{fs, path::Path};
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("common/net has a grandparent");

        let count_variants = |file: &str, enum_name: &str| -> usize {
            let text = fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
            let start = text
                .find(&format!("pub enum {enum_name}"))
                .unwrap_or_else(|| panic!("{enum_name} not found"));
            let open = text[start..].find('{').expect("enum has a body") + start;
            let mut depth = 0usize;
            let mut end = open;
            for (offset, ch) in text[open..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = open + offset;
                            break;
                        }
                    },
                    _ => {},
                }
            }
            text[open + 1..end]
                .lines()
                .filter(|line| {
                    let trimmed = line.trim_start();
                    line.len() - trimmed.len() == 4
                        && trimmed.chars().next().is_some_and(char::is_uppercase)
                })
                .count()
        };

        assert_eq!(
            count_variants("common/net/src/msg/client.rs", "ClientGeneral"),
            37,
            "ClientGeneral gained or lost a variant. Add it to WIRE_SHAPE_GOLDENS or to \
             UNCOVERED_CLIENTGENERAL_V1 — a variant in neither is a message whose shape nothing \
             is watching."
        );
        assert_eq!(
            count_variants("common/net/src/msg/server.rs", "ServerGeneral"),
            51,
            "ServerGeneral gained or lost a variant. Add it to WIRE_SHAPE_GOLDENS or to \
             UNCOVERED_SERVERGENERAL_V1."
        );
    }

    /// The digest helper is the one WSG-2 must use. Two ways of computing
    /// a golden is two goldens, and the second one is always the wrong
    /// one.
    #[test]
    fn the_digest_helper_is_stable_and_shape_sensitive() {
        let a = golden_digest_v1(&server_local_wind_update());
        assert_eq!(a, golden_digest_v1(&server_local_wind_update()));
        let different = ServerGeneral::LocalWindUpdate(
            Vec2::new(0.5, -1.5),
            WeatherSnapshotIdV1::from_sequence_v1(18),
        );
        assert_ne!(a, golden_digest_v1(&different), "the helper is blind to a payload change");
    }
}
