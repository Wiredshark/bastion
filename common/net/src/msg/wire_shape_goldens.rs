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
//! **Coverage started as a PINNED OPEN SET, not a red build.** WSG-1
//! landed the mechanism plus the four variants `T5.2` touched and could
//! not detect; the remaining variants were NAMED with a pinned count. A
//! variant in neither list failed immediately, so the set could not grow
//! silently, but the interim was honest rather than noisy — a build left
//! red from WSG-1 until WSG-2 closed would have taught everyone to ignore
//! it.
//!
//! **WSG-2 is CLOSED.** Both `UNCOVERED_CLIENTGENERAL_V1` and
//! `UNCOVERED_SERVERGENERAL_V1` are now empty — every `ClientGeneral` and
//! `ServerGeneral` variant has a golden. `coverage_is_all_covered` asserts
//! this directly; `a_new_variant_in_neither_list_fails_immediately` is the
//! ongoing tripwire that keeps it that way — a variant added to either
//! enum without a matching golden now fails immediately rather than
//! silently joining an "uncovered" list, because there is no longer an
//! uncovered list for it to join.

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
    // WSG-2 chunk 3: the empty-collection and byte-payload tail — the
    // variants whose representative instance needs no type exploration.
    // Taken as a deliberately small chunk against a tight capacity gate
    // rather than a large one taken on optimism.
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "PluginData",
        digest_hex: "sha256:9c5928befe60cf973f2e36b8fc1e247ffcf561177fe2c395c0a61d81b10ff587",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "Outcomes",
        digest_hex: "sha256:92ee0b61bd440fd8cd31a7b58b3a591d5f088e9bc165200777a958dbb18c520e",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "Gizmos",
        digest_hex: "sha256:f2ba5375533463b6340cb716006f2a57fa64a3702f1ba01bf1cb5a8df81de8ca",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CharacterListUpdate",
        digest_hex: "sha256:ca888f40c3caca805b37a5434c75de5550616e0795e7602fb91156f22dd90851",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CharacterDataLoadResult",
        digest_hex: "sha256:c4a5fe46107c963b2596af886c3b6f5241e1f0e035d9dd63b54a1438dcd20cc0",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "RequestPlugins",
        digest_hex: "sha256:6c9c17b5859fc171b4c361e705f5623506b1690c5fb8b246ed91de7c545cb520",
    },
    // WSG-2 chunk 2: InventoryUpdate (the deferred priority payload)
    // plus the client request surface — every one of these is a message
    // the SERVER decodes, so a shape drift is a server mis-parse of
    // player intent.
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "InventoryUpdate",
        digest_hex: "sha256:5395a375894799a81d560b5fba6459cfa9dd41736f2e676354cfc0d829f32c3d",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "RequestCharacterList",
        digest_hex: "sha256:df3f619804a92fdb4057192dc43dd748ea778adc52bc498ce80524c014b81119",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "DeleteCharacter",
        digest_hex: "sha256:2003bb45cdb4b56ec6860df6445d62a7da98aaf68581e8960391921723c0c256",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "ExitInGame",
        digest_hex: "sha256:42f4aeb81c1ef81f771f3de8abca9dcf66901c575530e7672e4b1146474ae650",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "Terminate",
        digest_hex: "sha256:8d71b3faab8201459ad37ef499beb336ba88bdcfa0f51ee6f0a46ec3192d750a",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "BreakBlock",
        digest_hex: "sha256:4afa94bbc07036aacc88bd0dc35b52a66b8fa9af23126068e0063eeb55d469ce",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "SpectatePosition",
        digest_hex: "sha256:9b321da81cf82c0407b29b9f7001d59e1a298d65a64ee54114db6968d70730ef",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "SpectateEntity",
        digest_hex: "sha256:ec9b7b0e1d6df8afa378eee5ca11e10939987a3fb318c82cffa5ab09277636c1",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "BastionCameraAnchor",
        digest_hex: "sha256:80ad81204480bb13c7417820c4005912fff164310b19dc8451ca6dd42d20f0f7",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "BastionSpawnColony",
        digest_hex: "sha256:2046395ff8b5288743451fa87c3de847209f8bf76a06b50a0b3bfe56ca1b0d62",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "TerrainChunkRequest",
        digest_hex: "sha256:aeeb4eff4dbce87dc72939cfbfa109d1ab3862001aaba05eae600b27d7ff5d55",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "LodZoneRequest",
        digest_hex: "sha256:08da0f328c7edf5fd56ce2281110639d9ae2a37703642954b601d31976f20524",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "Command",
        digest_hex: "sha256:001cf414c45355f55930c6cbd443c24f77243818c47fb30ed1898017275475a5",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "RequestPlayerPhysics",
        digest_hex: "sha256:c66d621939ede1014e7d9c99e8c0d25d07ecfecc71a51a0c8338c6f9b1d55a26",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "RequestLossyTerrainCompression",
        digest_hex: "sha256:49b53a82dec31dbcb3b3f4276f2996e695546482e961558b6f6d7e5d9273ffef",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "RequestSiteInfo",
        digest_hex: "sha256:f1f2e3db5bdef2590c86e96b6c2827cc1b71100f02fdb6265eeafd03267971b7",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "SetBattleMode",
        digest_hex: "sha256:a58049314b30ea940a92301f86b139ff225b2c094461b087a7105f88d8967590",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "Character",
        digest_hex: "sha256:63cefb587fd281aea30bf77ec9e2d34f4d768651bcf2ffd28c9b2895248b2f69",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "Spectate",
        digest_hex: "sha256:e1b03ca36251051fad146b0f3f515e3782b433377dc8d6c6bf28a7c559056a7c",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "SetViewDistance",
        digest_hex: "sha256:54c756fe3530d4553ff39be325acd408bd34a415f93dd8cc08e1ffd52b40e015",
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
    // WSG-2 chunk 4 (Sonnet lane): the remaining unit/single-simple-field
    // shapes -- picked from the 42-item remainder after confirming each
    // one's inner type by reading its definition first (Skill and
    // Content, both multi-crate enums, deliberately deferred rather than
    // guessed at).
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "ControlAction",
        digest_hex: "sha256:1f6f4b0d2ba528a06eb08eeb78503461eb4ac68a19abec2e33cd5bea1255f040",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "ControlEvent",
        digest_hex: "sha256:aae89fc0f03e2959ae4d701a80cc3915918c950b159f6abb6c92c1433b1a8534",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "UpdateMapMarker",
        digest_hex: "sha256:c4b0abe54ae451ca314522244a354d1ceca4772379b7a566e241b29c6d301bdc",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "GroupUpdate",
        digest_hex: "sha256:04b37e303aab5f9b4a180ace8b192b9123431e203b06864fa0eddfe7a1e63650",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "ChatMode",
        digest_hex: "sha256:449498db0a357e972ff02bc0a1c339f8493c59b77d27e2251136fe5e9ed93eaf",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "Disconnect",
        digest_hex: "sha256:74b4dda3624aed85d808e91d84b08aad88563b02fe290e0d327865c33d2bafbd",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "MapMarker",
        digest_hex: "sha256:76d25faac67d849dc025fd6cad30b7d377431bfddf5c2d998cda3d5feeee3b5d",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CreateEntity",
        digest_hex: "sha256:2d073538b2f92f89df9bda81ecadbb8ff22ff79000e79968501c9e4fd145eec0",
    },
    // WSG-2 chunk 5 (Sonnet lane): PlaceBlock/ControllerInputs (simple
    // wrapper types already used elsewhere in this file), the invite
    // pair (InviteKind/InviteAnswer are both plain small enums), a
    // Bastion region variant, and the calendar/time tuple.
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "PlaceBlock",
        digest_hex: "sha256:e00ce9a7cb9c0b819aa9cbf2cda616c2343f8fa0b372d647fcbf57511f7659b9",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "ControllerInputs",
        digest_hex: "sha256:b84359184b258e2805e81ec0a26a833d93aa222a1584a438f8b17d3a982ffd99",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "BastionCancelDesignation",
        digest_hex: "sha256:744d534cc11ac7ed083f292af2d313ef6987afc9234be0e003375d6b53fb904d",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "Notification",
        digest_hex: "sha256:1a4347f9664b7dbc5262b69f3dc1ba49c40b42ba3bf662af6bd475ac39e52eb2",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "FinishedTrade",
        digest_hex: "sha256:dd3168a635e962767dc4501dcef1d88bdfd9825d9c5a55d01dff715e7da9b5ec",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "TimeOfDay",
        digest_hex: "sha256:3f8b8591cc5cd792125bcbb873bc632eb76f38cb58de593997e9e7e5b1ce4cba",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "Invite",
        digest_hex: "sha256:a2a56ce3829f51202d577a34f6c4ee05f08f48daf7d9e1de7c51c4f3ee874779",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "InviteComplete",
        digest_hex: "sha256:ab8b19515d90969149f91829ea114691d40e4e41c2b1be1f1e0b9ed3bd339a77",
    },
    // WSG-2 chunk 6 (Sonnet lane): the Bastion cluster -- all seven
    // remaining Bastion variants at once, since Region/DesignationKind/
    // InfluenceKind/ContextTarget/ContextVerb/BastionInspectTarget are
    // all small enums or plain structs once read, and
    // BastionInspectKind's variants all wrap complex payloads that
    // `payload: None` avoids needing to construct at all.
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "BastionPlaceDesignation",
        digest_hex: "sha256:fcc1f63e0572110e743c0e9ff427f9ade12216a50d4743189cc2e907751756d2",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "BastionApplyInfluence",
        digest_hex: "sha256:904aa691bb97cca28899c8ceed6cb8b5dcd6b16c7dce8e83ed629ef6978a3843",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "BastionContextAction",
        digest_hex: "sha256:e01f756a5374f037e7b536fc778a91ea6064c3b2e0a9b1c0360d3330ddd1a4bd",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "BastionInspect",
        digest_hex: "sha256:ba9ddffe6a58fc41ea6e156e9f8b2c9ed5a1a7c2215f0b0f5f01304bfb1b4634",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "BastionDesignation",
        digest_hex: "sha256:3b51d462c438fe4a1a7f16f98b563238f2651f0fdcaf2c9ef90513c9903c5053",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "BastionDesignationRemoved",
        digest_hex: "sha256:2423fa1c035e2855a60008f5e8f7a18d9cb44066837dfaca03b94960fd764294",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "BastionInspectInfo",
        digest_hex: "sha256:fb7a8e0ac12bfddf5ccf533e2fe75f14b7ea383ef4452a7a44efe7c5497c7c8f",
    },
    // WSG-2 chunk 7 (Sonnet lane): the character-creation pair, the two
    // small self-describing client acks/requests, and the two simplest
    // remaining server payloads (LodZoneUpdate wraps an empty-vec Zone,
    // TerrainBlockUpdates wraps an empty-vec CompressedData -- both
    // constructible without touching the asset-manager or the checkpoint
    // identity machinery).
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "CreateCharacter",
        digest_hex: "sha256:7a296fe72321ce4b7973c57c476c6977076ec6e9091a7c3554f0c05c7fd91893",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "EditCharacter",
        digest_hex: "sha256:0dd3205a6bd76540697c8e012fb8acb5a65c33e8a68d89d7e824693807d2e61f",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "CheckpointCommitAck",
        digest_hex: "sha256:f21ae366e93ddd9c1ae1b460bcae2fb2c9a81f07c868fbf151912d01b9db691f",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "RequestPluginArtifacts",
        digest_hex: "sha256:01fbc944f1ba49e09446b214c05238b9e2e28342f12475ae68ab5e6db9de4af3",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "LodZoneUpdate",
        digest_hex: "sha256:781d556993f0e12cdf186679b7e44297a858dcb9a92253413cc01e113123e79f",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "TerrainBlockUpdates",
        digest_hex: "sha256:15ce278a47bfeec9447ac5bf9183cf122d31a3b635f3423a1f9c88151552c3a0",
    },
    // WSG-2 chunk 8 (Sonnet lane): CheckpointBarrier is a self-contained
    // struct (no CheckpointDescriptorV1 dependency, unlike CheckpointBegin);
    // CommandResult needs only CommandPublicationV1's receipt+sequence,
    // built via the same FixedRandomBytesSourceV1 deterministic-generate
    // pattern command.rs's own tests already use for CommandId.
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CheckpointBarrier",
        digest_hex: "sha256:e52653806d67b65733d298fcd7703ec077e8632dd40b34c2f35daefb7c593d94",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CommandResult",
        digest_hex: "sha256:a6c0dec0b9b3ebed6d7044aa8191c667b174bb8fdf470c2aa48d98a7fddc9bf4",
    },
    // WSG-2 chunk 9 (Sonnet lane): PluginArtifactData is a small
    // self-describing struct (no asset dependency); GroupInventoryUpdate
    // is the one variant in this pass needing the asset-manager-backed
    // FrontendItem (Item::new_from_asset_expect + AbilityMap::load() +
    // MaterialStatManifest::load(), the same pattern common's own
    // test_assets_items uses).
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "PluginArtifactData",
        digest_hex: "sha256:30caf704bf9df7ef32de4304136d54b38d698e7eb39eda7f1519b60df4ff5218",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "GroupInventoryUpdate",
        digest_hex: "sha256:edb5fe6e3ebc25e1177110c254bbe8bfc5562efe5d8b4c6c60e981c9e1be98f7",
    },
    // WSG-2 chunk 10 (Sonnet lane): UnlockSkill and both ChatMsg variants
    // (client sends comp::Content, server sends comp::ChatMsg) -- all
    // three are constructible from small enums/structs once read, closing
    // out the client side entirely.
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "UnlockSkill",
        digest_hex: "sha256:1e99261c7e6000f1081f2e8556637d48aa59f94b39b74eaae1fc9b6901e4ac1e",
    },
    WireShapeGoldenV1 {
        payload_schema: "ClientGeneral",
        variant: "ChatMsg",
        digest_hex: "sha256:32fc3efabf5b48c84514571b7e6c169aaf533d323b95ae96bfb9fc36ede8724c",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "ChatMsg",
        digest_hex: "sha256:f376fb99c30adf28e49b56dd8a0994484505cdc7deffa859469a73d8164e0a66",
    },
    // WSG-2 chunk 11 (Sonnet lane): UpdatePendingTrade and SiteEconomy.
    // TradeId's inner field is private outside common::trade -- a
    // bincode roundtrip through the same newtype-transparent encoding
    // golden_digest_v1 itself uses builds one from an external crate
    // without exposing the field.
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "UpdatePendingTrade",
        digest_hex: "sha256:d1a606dac9ee281ea856271948e6dfef68948b40f6283508057bfbf16b992f70",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "SiteEconomy",
        digest_hex: "sha256:06fda32836156e704143736fd84f56b4ac1b8b59119282e000a7296c5ee241c2",
    },
    // WSG-2 chunk 12 (Sonnet lane): the final four -- closes WSG-2.
    // Dialogue wraps a small rtsim::Dialogue<true>. TerrainChunkUpdate
    // uses `chunk: Err(())`, the wire's own explicit "chunk unavailable"
    // case, avoiding SerializedTerrainChunk construction entirely.
    // PlayerListUpdate::Remove(Uid) is the simplest of its eight variants.
    // CheckpointBegin needs the full CheckpointDescriptorV1 tree, built
    // field-by-field from its own definition (5-stream plan array,
    // ActiveSessionBindingV1 via the established FixedRandomBytesSourceV1
    // deterministic-generate pattern).
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "Dialogue",
        digest_hex: "sha256:d62ea8b0a2c9fcbf99991501d396e5333aa90f25d7ac12e90e56c755c6ab4717",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "TerrainChunkUpdate",
        digest_hex: "sha256:7a5886f5fb4b7943cfa43e13a90d97aab4c506287ab64a88b8e4636f70a1da8e",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "PlayerListUpdate",
        digest_hex: "sha256:634e4859d1bde17fd638fe1ff1ed58e6329f742bff84fbb6c188b66660c5bca7",
    },
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "CheckpointBegin",
        digest_hex: "sha256:4e9beeec9db3a10c7b1fdffda4a43501cf2669688b943c856cc8f507a26c4adc",
    },
    // APEX-T4.1 chunk 2a: the new BootstrapManifest variant, added the
    // same landing day it was introduced -- the rail (this file's own
    // growth tripwire) demanded it before the build would go green.
    WireShapeGoldenV1 {
        payload_schema: "ServerGeneral",
        variant: "BootstrapManifest",
        digest_hex: "sha256:595809ef3f70cc71dc5124ff08e4200054d1ced2aafac04eafb15fa329f39fad",
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
        ClientGeneral, DisconnectReason, ServerGeneral,
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


    fn view_distances() -> common::ViewDistances {
        common::ViewDistances { terrain: 9, entity: 5 }
    }

    fn server_inventory_update() -> ServerGeneral {
        ServerGeneral::InventoryUpdate(
            common::comp::Inventory::with_empty(),
            vec![
                common::comp::InventoryUpdateEvent::Init,
                common::comp::InventoryUpdateEvent::Used,
            ],
        )
    }

    fn client_request_character_list() -> ClientGeneral { ClientGeneral::RequestCharacterList }

    fn client_delete_character() -> ClientGeneral {
        ClientGeneral::DeleteCharacter(common::character::CharacterId(101))
    }

    fn client_exit_in_game() -> ClientGeneral { ClientGeneral::ExitInGame }

    fn client_terminate() -> ClientGeneral { ClientGeneral::Terminate }

    fn client_break_block() -> ClientGeneral {
        ClientGeneral::BreakBlock(Vec2::new(1, 2).with_z(3))
    }

    fn client_spectate_position() -> ClientGeneral {
        ClientGeneral::SpectatePosition(Vec2::new(4.0, 5.0).with_z(6.0))
    }

    fn client_spectate_entity() -> ClientGeneral {
        ClientGeneral::SpectateEntity(Some(uid(103)))
    }

    fn client_bastion_camera_anchor() -> ClientGeneral {
        ClientGeneral::BastionCameraAnchor(Some(Vec2::new(7.0, 8.0).with_z(9.0)))
    }

    fn client_bastion_spawn_colony() -> ClientGeneral {
        ClientGeneral::BastionSpawnColony { pos: Vec2::new(10.0, 11.0).with_z(12.0), count: 4 }
    }

    fn client_terrain_chunk_request() -> ClientGeneral {
        ClientGeneral::TerrainChunkRequest { key: Vec2::new(13, 14) }
    }

    fn client_lod_zone_request() -> ClientGeneral {
        ClientGeneral::LodZoneRequest { key: Vec2::new(15, 16) }
    }

    fn client_command() -> ClientGeneral {
        ClientGeneral::Command("wsg".to_owned(), vec!["a".to_owned(), "b".to_owned()])
    }

    fn client_request_player_physics() -> ClientGeneral {
        ClientGeneral::RequestPlayerPhysics { server_authoritative: true }
    }

    fn client_request_lossy() -> ClientGeneral {
        ClientGeneral::RequestLossyTerrainCompression { lossy_terrain_compression: false }
    }

    fn client_request_site_info() -> ClientGeneral { ClientGeneral::RequestSiteInfo(107) }

    fn client_set_battle_mode() -> ClientGeneral {
        ClientGeneral::SetBattleMode(common::resources::BattleMode::PvE)
    }

    fn client_character() -> ClientGeneral {
        ClientGeneral::Character(common::character::CharacterId(109), view_distances())
    }

    fn client_spectate() -> ClientGeneral { ClientGeneral::Spectate(view_distances()) }

    fn client_set_view_distance() -> ClientGeneral {
        ClientGeneral::SetViewDistance(view_distances())
    }

    /// Keyed by (schema, variant), NOT by variant alone.
    ///
    /// `SetViewDistance`, `SpectatePosition` and `SpectateEntity` exist
    /// in BOTH enums. Dispatching on the name alone silently returns the
    /// wrong enum's fixture — the same duplicate-name hazard that broke
    /// the uncovered list in chunk 1, biting the DISPATCH this time.
    fn server_plugin_data() -> ServerGeneral { ServerGeneral::PluginData(vec![7, 8, 9]) }

    fn server_outcomes() -> ServerGeneral { ServerGeneral::Outcomes(Vec::new()) }

    fn server_gizmos() -> ServerGeneral { ServerGeneral::Gizmos(Vec::new()) }

    fn server_character_list_update() -> ServerGeneral {
        ServerGeneral::CharacterListUpdate(Vec::new())
    }

    fn server_character_data_load_result() -> ServerGeneral {
        ServerGeneral::CharacterDataLoadResult(Err("wsg-3 fixture".to_owned()))
    }

    fn client_request_plugins() -> ClientGeneral { ClientGeneral::RequestPlugins(Vec::new()) }

    // WSG-2 chunk 4 fixtures.

    fn client_control_action() -> ClientGeneral {
        ClientGeneral::ControlAction(common::comp::ControlAction::Stand)
    }

    fn client_control_event() -> ClientGeneral {
        ClientGeneral::ControlEvent(common::comp::ControlEvent::EnableLantern)
    }

    fn client_update_map_marker() -> ClientGeneral {
        ClientGeneral::UpdateMapMarker(common::comp::MapMarkerChange::Remove)
    }

    fn server_group_update() -> ServerGeneral {
        ServerGeneral::GroupUpdate(common::comp::group::ChangeNotification::NoGroup)
    }

    fn server_chat_mode() -> ServerGeneral { ServerGeneral::ChatMode(common::comp::ChatMode::Say) }

    fn server_disconnect() -> ServerGeneral { ServerGeneral::Disconnect(DisconnectReason::Shutdown) }

    fn server_map_marker() -> ServerGeneral {
        ServerGeneral::MapMarker(common::comp::MapMarkerUpdate::ClearGroup)
    }

    fn server_create_entity() -> ServerGeneral {
        ServerGeneral::CreateEntity(crate::sync::EntityPackage { uid: uid(113), comps: Vec::new() })
    }

    // WSG-2 chunk 5 fixtures.

    fn client_place_block() -> ClientGeneral {
        ClientGeneral::PlaceBlock(
            Vec2::new(17, 18).with_z(19),
            common::terrain::Block::new(
                common::terrain::BlockKind::Air,
                vek::Rgb::new(0u8, 0u8, 0u8),
            ),
        )
    }

    fn client_controller_inputs() -> ClientGeneral {
        ClientGeneral::ControllerInputs(Box::new(common::comp::ControllerInputs::default()))
    }

    fn client_bastion_cancel_designation() -> ClientGeneral {
        ClientGeneral::BastionCancelDesignation {
            region: common::bastion::Region {
                min: Vec2::new(0, 0).with_z(0),
                max: Vec2::new(1, 1).with_z(1),
            },
        }
    }

    fn server_notification() -> ServerGeneral {
        ServerGeneral::Notification(crate::msg::Notification::WaypointSaved {
            location_name: "wsg5".to_owned(),
        })
    }

    fn server_finished_trade() -> ServerGeneral {
        ServerGeneral::FinishedTrade(common::trade::TradeResult::Completed)
    }

    fn server_time_of_day() -> ServerGeneral {
        ServerGeneral::TimeOfDay(
            common::resources::TimeOfDay::new(100.0),
            common::calendar::Calendar::default(),
            common::resources::Time(50.0),
            common::resources::TimeScale(1.0),
        )
    }

    fn server_invite() -> ServerGeneral {
        ServerGeneral::Invite {
            inviter: uid(119),
            timeout: std::time::Duration::from_secs(30),
            kind: common::comp::invite::InviteKind::Group,
        }
    }

    fn server_invite_complete() -> ServerGeneral {
        ServerGeneral::InviteComplete {
            target: uid(127),
            answer: crate::msg::InviteAnswer::Accepted,
            kind: common::comp::invite::InviteKind::Trade,
        }
    }

    // WSG-2 chunk 6 fixtures: the Bastion cluster.

    fn region() -> common::bastion::Region {
        common::bastion::Region { min: Vec2::new(0, 0).with_z(0), max: Vec2::new(1, 1).with_z(1) }
    }

    fn client_bastion_place_designation() -> ClientGeneral {
        ClientGeneral::BastionPlaceDesignation {
            region: region(),
            kind: common::bastion::DesignationKind::Mine,
            z_extent: None,
        }
    }

    fn client_bastion_apply_influence() -> ClientGeneral {
        ClientGeneral::BastionApplyInfluence {
            target: Vec2::new(20.0, 21.0).with_z(22.0),
            kind: common::bastion::InfluenceKind::Bless,
        }
    }

    fn client_bastion_context_action() -> ClientGeneral {
        ClientGeneral::BastionContextAction {
            target: common::bastion::ContextTarget::Block(Vec2::new(23, 24).with_z(25)),
            verb: common::bastion::ContextVerb::Mine,
        }
    }

    fn client_bastion_inspect() -> ClientGeneral {
        ClientGeneral::BastionInspect {
            target: common::comp::bastion::BastionInspectTarget::Cell(
                Vec2::new(26, 27).with_z(28),
            ),
        }
    }

    fn server_bastion_designation() -> ServerGeneral {
        ServerGeneral::BastionDesignation {
            region: region(),
            kind: common::bastion::DesignationKind::Chop,
            z_extent: None,
        }
    }

    fn server_bastion_designation_removed() -> ServerGeneral {
        ServerGeneral::BastionDesignationRemoved { region: region() }
    }

    fn server_bastion_inspect_info() -> ServerGeneral {
        ServerGeneral::BastionInspectInfo {
            target: common::comp::bastion::BastionInspectTarget::Cell(
                Vec2::new(29, 30).with_z(31),
            ),
            payload: None,
        }
    }

    // WSG-2 chunk 7 fixtures.

    fn client_create_character() -> ClientGeneral {
        ClientGeneral::CreateCharacter {
            alias: "chunk7".to_string(),
            mainhand: None,
            offhand: None,
            body: common::comp::Body::Humanoid(common::comp::body::humanoid::Body {
                species: common::comp::body::humanoid::Species::Human,
                body_type: common::comp::body::humanoid::BodyType::Male,
                hair_style: 0,
                beard: 0,
                eyes: 0,
                accessory: 0,
                hair_color: 0,
                skin: 0,
                eye_color: 0,
                height_scale: 0,
            }),
            hardcore: false,
            start_site: None,
        }
    }

    fn client_edit_character() -> ClientGeneral {
        ClientGeneral::EditCharacter {
            id: common::character::CharacterId(32),
            alias: "chunk7-edit".to_string(),
            body: common::comp::Body::Humanoid(common::comp::body::humanoid::Body {
                species: common::comp::body::humanoid::Species::Dwarf,
                body_type: common::comp::body::humanoid::BodyType::Female,
                hair_style: 1,
                beard: 1,
                eyes: 1,
                accessory: 1,
                hair_color: 1,
                skin: 1,
                eye_color: 1,
                height_scale: 1,
            }),
        }
    }

    fn client_checkpoint_commit_ack() -> ClientGeneral {
        ClientGeneral::CheckpointCommitAck(crate::msg::checkpoint::CheckpointCommitReceiptV1 {
            epoch: 33,
            parent_epoch: 32,
            descriptor_root: [34u8; 32],
            applied_records: 35,
        })
    }

    fn client_request_plugin_artifacts() -> ClientGeneral {
        ClientGeneral::RequestPluginArtifacts(crate::msg::plugin_artifact::PluginArtifactRequestV1 {
            deployment_root: [36u8; 32],
            ordinals: vec![1, 2, 3],
        })
    }

    fn server_lod_zone_update() -> ServerGeneral {
        ServerGeneral::LodZoneUpdate {
            key: Vec2::new(37, 38),
            zone: common::lod::Zone { objects: Vec::new() },
        }
    }

    fn server_terrain_block_updates() -> ServerGeneral {
        ServerGeneral::TerrainBlockUpdates(crate::msg::compression::CompressedData::compress(
            &vec![(
                Vec2::new(39, 40).with_z(41),
                common::terrain::Block::air(common::terrain::SpriteKind::Empty),
            )],
            1,
        ))
    }

    // WSG-2 chunk 8 fixtures.

    fn server_checkpoint_barrier() -> ServerGeneral {
        ServerGeneral::CheckpointBarrier(crate::msg::checkpoint::CheckpointBarrierV1 {
            epoch: 42,
            stream: crate::msg::envelope::SemanticStreamIdV1::General,
            descriptor_root: [43u8; 32],
            data_record_count: 44,
            payload_bytes: 45,
            last_data_sequence: Some(46),
            stream_transcript_root: [47u8; 32],
        })
    }

    fn server_command_result() -> ServerGeneral {
        let command_id = common::apex::identity::CommandId::generate(
            &mut common::apex::identity::FixedRandomBytesSourceV1([48; 16]),
        )
        .unwrap();
        ServerGeneral::CommandResult(crate::msg::command::CommandPublicationV1 {
            receipt: crate::msg::command::CommandReceiptV1 {
                command_id,
                identity_root: [49u8; 32],
                outcome: crate::msg::command::CommandOutcomeV1::Applied { result_digest: [50u8; 32] },
                effect_epoch: 51,
            },
            sequence: 52,
        })
    }

    // WSG-2 chunk 9 fixtures.

    fn server_plugin_artifact_data() -> ServerGeneral {
        ServerGeneral::PluginArtifactData(crate::msg::plugin_artifact::PluginArtifactResponseV1 {
            descriptor: crate::msg::plugin_artifact::PluginArtifactDescriptorV1 {
                deployment_root: [53u8; 32],
                ordinal: 54,
                digest: [55u8; 32],
                size_bytes: 56,
            },
            bytes: vec![57, 58, 59],
        })
    }

    fn server_group_inventory_update() -> ServerGeneral {
        let item = common::comp::Item::new_from_asset_expect("common.items.weapons.empty.empty");
        let ability_map = common::comp::inventory::item::AbilityMap::load();
        let msm = common::comp::inventory::item::MaterialStatManifest::load();
        let frontend_item = item.frontend_item(&ability_map.read(), &msm.read());
        ServerGeneral::GroupInventoryUpdate(frontend_item, uid(146))
    }

    // WSG-2 chunk 10 fixtures.

    fn client_unlock_skill() -> ClientGeneral {
        ClientGeneral::UnlockSkill(common::comp::Skill::Sword(
            common::comp::skillset::skills::SwordSkill::CrescentSlash,
        ))
    }

    fn client_chat_msg() -> ClientGeneral {
        ClientGeneral::ChatMsg(common::comp::Content::Plain("chunk10".to_string()))
    }

    fn server_chat_msg() -> ServerGeneral {
        ServerGeneral::ChatMsg(common::comp::ChatMsg::npc_say(
            uid(147),
            common::comp::Content::Plain("chunk10-npc".to_string()),
        ))
    }

    // WSG-2 chunk 11 fixtures.

    fn trade_id(n: usize) -> common::trade::TradeId {
        let bytes =
            bincode::serde::encode_to_vec(&n, bincode::config::legacy()).expect("encode usize");
        bincode::serde::decode_from_slice(&bytes, bincode::config::legacy())
            .expect("decode as TradeId (newtype-transparent)")
            .0
    }

    fn server_update_pending_trade() -> ServerGeneral {
        ServerGeneral::UpdatePendingTrade(
            trade_id(60),
            common::trade::PendingTrade {
                parties: [uid(61), uid(62)],
                offers: [hashbrown::HashMap::new(), hashbrown::HashMap::new()],
                phase: common::trade::TradePhase::Mutate,
                accept_flags: [false, false],
            },
            None,
        )
    }

    fn server_site_economy() -> ServerGeneral {
        ServerGeneral::SiteEconomy(crate::msg::world_msg::EconomyInfo {
            id: 63,
            population: 64,
            stock: std::collections::HashMap::new(),
            labor_values: std::collections::HashMap::new(),
            values: std::collections::HashMap::new(),
            labors: Vec::new(),
            last_exports: std::collections::HashMap::new(),
            resources: std::collections::HashMap::new(),
        })
    }

    // WSG-2 chunk 12 fixtures.

    fn server_dialogue() -> ServerGeneral {
        ServerGeneral::Dialogue(
            uid(148),
            common::rtsim::Dialogue::<true> {
                id: common::rtsim::DialogueId(149),
                kind: common::rtsim::DialogueKind::Start,
            },
        )
    }

    fn server_terrain_chunk_update() -> ServerGeneral {
        ServerGeneral::TerrainChunkUpdate { key: Vec2::new(150, 151), chunk: Err(()) }
    }

    fn server_player_list_update() -> ServerGeneral {
        ServerGeneral::PlayerListUpdate(crate::msg::server::PlayerListUpdate::Remove(uid(152)))
    }

    fn stream_plan(stream: crate::msg::envelope::SemanticStreamIdV1) -> crate::msg::checkpoint::StreamCheckpointPlanV1 {
        crate::msg::checkpoint::StreamCheckpointPlanV1 {
            stream,
            begin_sequence: 1,
            first_data_sequence: None,
            last_data_sequence: None,
            barrier_sequence: 2,
            data_record_count: 0,
            payload_bytes: 0,
            stream_transcript_root: [153u8; 32],
        }
    }

    fn server_checkpoint_begin() -> ServerGeneral {
        let binding = crate::msg::envelope::ActiveSessionBindingV1 {
            server_boot_id: common::apex::identity::ServerBootId::generate(
                &mut common::apex::identity::FixedRandomBytesSourceV1([154; 16]),
            )
            .unwrap(),
            session_id: common::apex::identity::SessionId::generate(
                &mut common::apex::identity::FixedRandomBytesSourceV1([155; 16]),
            )
            .unwrap(),
            epoch: common::apex::identity::ConnectionEpoch::FIRST,
        };
        ServerGeneral::CheckpointBegin(Box::new(crate::msg::checkpoint::CheckpointStreamOpenV1 {
            begin: crate::msg::checkpoint::CheckpointBeginV1 {
                epoch: 156,
                stream: crate::msg::envelope::SemanticStreamIdV1::General,
                descriptor_root: [157u8; 32],
            },
            descriptor: crate::msg::checkpoint::CheckpointDescriptorV1 {
                schema_version: 1,
                binding,
                epoch: 156,
                parent_epoch: 155,
                resource_profile_root: [158u8; 32],
                apply_policy_root: [159u8; 32],
                egress_order_policy_root: [160u8; 32],
                data_record_count: 0,
                ordinal_max: 0,
                payload_bytes: 0,
                global_transcript_root: [161u8; 32],
                streams: [
                    stream_plan(crate::msg::envelope::SemanticStreamIdV1::Bootstrap),
                    stream_plan(crate::msg::envelope::SemanticStreamIdV1::CharacterScreen),
                    stream_plan(crate::msg::envelope::SemanticStreamIdV1::InGame),
                    stream_plan(crate::msg::envelope::SemanticStreamIdV1::General),
                    stream_plan(crate::msg::envelope::SemanticStreamIdV1::Terrain),
                ],
                bootstrap_manifest_root: None,
            },
        }))
    }

    // APEX-T4.1 chunk 2a fixture.

    fn server_bootstrap_manifest() -> ServerGeneral {
        let manifest = common::apex::bootstrap_manifest::BootstrapManifestV1 {
            descriptors: vec![common::apex::subsystem::SubsystemDescriptorV1 {
                slot: common::apex::subsystem::SubsystemSlotIdV1::NetEnvelope,
                schema: common::apex::scalar::SchemaVersion::new(1),
                content: common::apex::digest::ContentIdentityV1 {
                    artifact: common::apex::digest::hash_artifact_bytes_v1(b"wire-schema-fixture"),
                    semantic: None,
                },
            }],
            peer_selector: None,
            peer_capabilities: Vec::new(),
            freshness: None,
        };
        ServerGeneral::BootstrapManifest(
            crate::msg::bootstrap_manifest_wire::BootstrapManifestWireV1::from_typed_v1(&manifest)
                .expect("fixture manifest encodes"),
        )
    }

    fn actual(schema: &str, variant: &str) -> String {
        match (schema, variant) {
            ("ClientGeneral", "PlayerPhysics") => golden_digest_v1(&client_player_physics()),
            ("ServerGeneral", "WeatherUpdate") => golden_digest_v1(&server_weather_update()),
            ("ServerGeneral", "LocalWindUpdate") => golden_digest_v1(&server_local_wind_update()),
            ("ServerGeneral", "InputReceipt") => golden_digest_v1(&server_input_receipt()),
            ("ServerGeneral", "PluginData") => golden_digest_v1(&server_plugin_data()),
            ("ServerGeneral", "Outcomes") => golden_digest_v1(&server_outcomes()),
            ("ServerGeneral", "Gizmos") => golden_digest_v1(&server_gizmos()),
            ("ServerGeneral", "CharacterListUpdate") => golden_digest_v1(&server_character_list_update()),
            ("ServerGeneral", "CharacterDataLoadResult") => golden_digest_v1(&server_character_data_load_result()),
            ("ClientGeneral", "RequestPlugins") => golden_digest_v1(&client_request_plugins()),
            ("ServerGeneral", "InventoryUpdate") => golden_digest_v1(&server_inventory_update()),
            ("ClientGeneral", "RequestCharacterList") => golden_digest_v1(&client_request_character_list()),
            ("ClientGeneral", "DeleteCharacter") => golden_digest_v1(&client_delete_character()),
            ("ClientGeneral", "ExitInGame") => golden_digest_v1(&client_exit_in_game()),
            ("ClientGeneral", "Terminate") => golden_digest_v1(&client_terminate()),
            ("ClientGeneral", "BreakBlock") => golden_digest_v1(&client_break_block()),
            ("ClientGeneral", "SpectatePosition") => golden_digest_v1(&client_spectate_position()),
            ("ClientGeneral", "SpectateEntity") => golden_digest_v1(&client_spectate_entity()),
            ("ClientGeneral", "BastionCameraAnchor") => golden_digest_v1(&client_bastion_camera_anchor()),
            ("ClientGeneral", "BastionSpawnColony") => golden_digest_v1(&client_bastion_spawn_colony()),
            ("ClientGeneral", "TerrainChunkRequest") => golden_digest_v1(&client_terrain_chunk_request()),
            ("ClientGeneral", "LodZoneRequest") => golden_digest_v1(&client_lod_zone_request()),
            ("ClientGeneral", "Command") => golden_digest_v1(&client_command()),
            ("ClientGeneral", "RequestPlayerPhysics") => golden_digest_v1(&client_request_player_physics()),
            ("ClientGeneral", "RequestLossyTerrainCompression") => golden_digest_v1(&client_request_lossy()),
            ("ClientGeneral", "RequestSiteInfo") => golden_digest_v1(&client_request_site_info()),
            ("ClientGeneral", "SetBattleMode") => golden_digest_v1(&client_set_battle_mode()),
            ("ClientGeneral", "Character") => golden_digest_v1(&client_character()),
            ("ClientGeneral", "Spectate") => golden_digest_v1(&client_spectate()),
            ("ClientGeneral", "SetViewDistance") => golden_digest_v1(&client_set_view_distance()),
            ("ServerGeneral", "EntitySync") => golden_digest_v1(&server_entity_sync()),
            ("ServerGeneral", "CompSync") => golden_digest_v1(&server_comp_sync()),
            ("ServerGeneral", "DeleteEntity") => golden_digest_v1(&server_delete_entity()),
            ("ServerGeneral", "SetPlayerEntity") => golden_digest_v1(&server_set_player_entity()),
            ("ServerGeneral", "Knockback") => golden_digest_v1(&server_knockback()),
            ("ServerGeneral", "SpectatePosition") => golden_digest_v1(&server_spectate_position()),
            ("ServerGeneral", "SpectatorSuccess") => golden_digest_v1(&server_spectator_success()),
            ("ServerGeneral", "SetViewDistance") => golden_digest_v1(&server_set_view_distance()),
            ("ServerGeneral", "InvitePending") => golden_digest_v1(&server_invite_pending()),
            ("ServerGeneral", "CharacterCreated") => golden_digest_v1(&server_character_created()),
            ("ServerGeneral", "CharacterEdited") => golden_digest_v1(&server_character_edited()),
            ("ServerGeneral", "CharacterActionError") => golden_digest_v1(&server_character_action_error()),
            ("ServerGeneral", "CharacterSuccess") => golden_digest_v1(&server_character_success()),
            ("ServerGeneral", "ExitInGameSuccess") => golden_digest_v1(&server_exit_in_game_success()),
            ("ServerGeneral", "UpdateRecipes") => golden_digest_v1(&server_update_recipes()),
            ("ServerGeneral", "SetPlayerRole") => golden_digest_v1(&server_set_player_role()),
            ("ClientGeneral", "ControlAction") => golden_digest_v1(&client_control_action()),
            ("ClientGeneral", "ControlEvent") => golden_digest_v1(&client_control_event()),
            ("ClientGeneral", "UpdateMapMarker") => golden_digest_v1(&client_update_map_marker()),
            ("ServerGeneral", "GroupUpdate") => golden_digest_v1(&server_group_update()),
            ("ServerGeneral", "ChatMode") => golden_digest_v1(&server_chat_mode()),
            ("ServerGeneral", "Disconnect") => golden_digest_v1(&server_disconnect()),
            ("ServerGeneral", "MapMarker") => golden_digest_v1(&server_map_marker()),
            ("ServerGeneral", "CreateEntity") => golden_digest_v1(&server_create_entity()),
            ("ClientGeneral", "PlaceBlock") => golden_digest_v1(&client_place_block()),
            ("ClientGeneral", "ControllerInputs") => golden_digest_v1(&client_controller_inputs()),
            ("ClientGeneral", "BastionCancelDesignation") => {
                golden_digest_v1(&client_bastion_cancel_designation())
            },
            ("ServerGeneral", "Notification") => golden_digest_v1(&server_notification()),
            ("ServerGeneral", "FinishedTrade") => golden_digest_v1(&server_finished_trade()),
            ("ServerGeneral", "TimeOfDay") => golden_digest_v1(&server_time_of_day()),
            ("ServerGeneral", "Invite") => golden_digest_v1(&server_invite()),
            ("ServerGeneral", "InviteComplete") => golden_digest_v1(&server_invite_complete()),
            ("ClientGeneral", "BastionPlaceDesignation") => {
                golden_digest_v1(&client_bastion_place_designation())
            },
            ("ClientGeneral", "BastionApplyInfluence") => {
                golden_digest_v1(&client_bastion_apply_influence())
            },
            ("ClientGeneral", "BastionContextAction") => {
                golden_digest_v1(&client_bastion_context_action())
            },
            ("ClientGeneral", "BastionInspect") => golden_digest_v1(&client_bastion_inspect()),
            ("ServerGeneral", "BastionDesignation") => golden_digest_v1(&server_bastion_designation()),
            ("ServerGeneral", "BastionDesignationRemoved") => {
                golden_digest_v1(&server_bastion_designation_removed())
            },
            ("ServerGeneral", "BastionInspectInfo") => golden_digest_v1(&server_bastion_inspect_info()),
            ("ClientGeneral", "CreateCharacter") => golden_digest_v1(&client_create_character()),
            ("ClientGeneral", "EditCharacter") => golden_digest_v1(&client_edit_character()),
            ("ClientGeneral", "CheckpointCommitAck") => golden_digest_v1(&client_checkpoint_commit_ack()),
            ("ClientGeneral", "RequestPluginArtifacts") => {
                golden_digest_v1(&client_request_plugin_artifacts())
            },
            ("ServerGeneral", "LodZoneUpdate") => golden_digest_v1(&server_lod_zone_update()),
            ("ServerGeneral", "TerrainBlockUpdates") => golden_digest_v1(&server_terrain_block_updates()),
            ("ServerGeneral", "CheckpointBarrier") => golden_digest_v1(&server_checkpoint_barrier()),
            ("ServerGeneral", "CommandResult") => golden_digest_v1(&server_command_result()),
            ("ServerGeneral", "PluginArtifactData") => golden_digest_v1(&server_plugin_artifact_data()),
            ("ServerGeneral", "GroupInventoryUpdate") => golden_digest_v1(&server_group_inventory_update()),
            ("ClientGeneral", "UnlockSkill") => golden_digest_v1(&client_unlock_skill()),
            ("ClientGeneral", "ChatMsg") => golden_digest_v1(&client_chat_msg()),
            ("ServerGeneral", "ChatMsg") => golden_digest_v1(&server_chat_msg()),
            ("ServerGeneral", "UpdatePendingTrade") => golden_digest_v1(&server_update_pending_trade()),
            ("ServerGeneral", "SiteEconomy") => golden_digest_v1(&server_site_economy()),
            ("ServerGeneral", "Dialogue") => golden_digest_v1(&server_dialogue()),
            ("ServerGeneral", "TerrainChunkUpdate") => golden_digest_v1(&server_terrain_chunk_update()),
            ("ServerGeneral", "PlayerListUpdate") => golden_digest_v1(&server_player_list_update()),
            ("ServerGeneral", "CheckpointBegin") => golden_digest_v1(&server_checkpoint_begin()),
            ("ServerGeneral", "BootstrapManifest") => golden_digest_v1(&server_bootstrap_manifest()),
            (schema, other) => panic!("{schema}::{other} has a golden entry but no representative instance"),
        }
    }

    /// Every golden still matches its variant's encoding. A changed field
    /// type or order fails HERE, naming the variant — which is the whole
    /// point, because the profile root cannot see it.
    #[test]
    fn every_golden_still_matches_its_variants_encoding() {
        for golden in WIRE_SHAPE_GOLDENS {
            assert_eq!(
                actual(golden.payload_schema, golden.variant),
                golden.digest_hex,
                "{}::{} changed shape on the wire. The envelope profile root CANNOT see this — \
                 that is why this table exists. If the change is deliberate, recompute this \
                 golden AND bump the payload-schema label, because old peers will mis-decode.",
                golden.payload_schema,
                golden.variant
            );
        }
    }

    /// WSG-2 CLOSED: coverage is no longer an open set. Every
    /// `ClientGeneral` and `ServerGeneral` variant has a golden -- both
    /// uncovered lists are pinned empty, and the covered count is pinned
    /// against the enums' own real variant totals (37 ClientGeneral, 51
    /// ServerGeneral, counted from the enums at 71b1c87ca7).
    #[test]
    fn coverage_is_all_covered() {
        assert_eq!(WIRE_SHAPE_GOLDENS.len(), 89, "the covered set changed");
        assert_eq!(UNCOVERED_CLIENTGENERAL_V1.len(), 0, "WSG-2 closed this at zero");
        assert_eq!(UNCOVERED_SERVERGENERAL_V1.len(), 0, "WSG-2 closed this at zero");
        let covered_client =
            WIRE_SHAPE_GOLDENS.iter().filter(|g| g.payload_schema == "ClientGeneral").count();
        let covered_server =
            WIRE_SHAPE_GOLDENS.iter().filter(|g| g.payload_schema == "ServerGeneral").count();
        assert_eq!(covered_client, 37, "every ClientGeneral variant must have a golden");
        assert_eq!(covered_server, 52, "every ServerGeneral variant must have a golden");
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
            52,
            "ServerGeneral gained or lost a variant. Add it to WIRE_SHAPE_GOLDENS or to \
             UNCOVERED_SERVERGENERAL_V1."
        );
    }

    /// WSG-2 chunk 5's falsifier: `TimeOfDay` has four fields, the
    /// richest shape this chunk added -- perturbing the second (a whole
    /// different type, `Calendar`, not just a numeric tweak) proves the
    /// mechanism sees a change anywhere in a multi-field tuple, not just
    /// the first field.
    #[test]
    fn chunk_5_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&server_time_of_day());
        let perturbed = golden_digest_v1(&ServerGeneral::TimeOfDay(
            common::resources::TimeOfDay::new(100.0),
            common::calendar::Calendar::from_events(vec![common::calendar::CalendarEvent::Easter]),
            common::resources::Time(50.0),
            common::resources::TimeScale(1.0),
        ));
        assert_ne!(
            base, perturbed,
            "changing TimeOfDay's Calendar field did not move the digest -- the golden \
             mechanism is blind to this chunk's payload"
        );
    }

    /// WSG-2 chunk 6's falsifier: perturbing `BastionPlaceDesignation`'s
    /// `Region.max` (a nested struct field, not a top-level scalar) proves
    /// the mechanism sees a change buried inside a chunk-added payload.
    #[test]
    fn chunk_6_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&client_bastion_place_designation());
        let perturbed = golden_digest_v1(&ClientGeneral::BastionPlaceDesignation {
            region: common::bastion::Region {
                min: Vec2::new(0, 0).with_z(0),
                max: Vec2::new(9, 9).with_z(9),
            },
            kind: common::bastion::DesignationKind::Mine,
            z_extent: None,
        });
        assert_ne!(
            base, perturbed,
            "changing BastionPlaceDesignation's region.max did not move the digest -- the \
             golden mechanism is blind to this chunk's payload"
        );
    }

    /// WSG-2 chunk 7's falsifier: perturbing `TerrainBlockUpdates`'s inner
    /// block content proves the mechanism sees a change inside a
    /// `CompressedData<T>` payload -- not just T's own top-level fields,
    /// but the compressed bytes derived from them.
    #[test]
    fn chunk_7_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&server_terrain_block_updates());
        let perturbed = golden_digest_v1(&ServerGeneral::TerrainBlockUpdates(
            crate::msg::compression::CompressedData::compress(
                &vec![(
                    Vec2::new(39, 40).with_z(41),
                    common::terrain::Block::new(common::terrain::BlockKind::Rock, vek::Rgb::new(1, 2, 3)),
                )],
                1,
            ),
        ));
        assert_ne!(
            base, perturbed,
            "changing TerrainBlockUpdates's inner block did not move the digest -- the golden \
             mechanism is blind to this chunk's payload"
        );
    }

    /// WSG-2 chunk 8's falsifier: perturbing `CommandResult`'s outcome
    /// digest (a field inside the nested `CommandReceiptV1` inside
    /// `CommandPublicationV1`, two levels deep) proves the mechanism sees
    /// a change buried past a struct-in-struct boundary.
    #[test]
    fn chunk_8_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&server_command_result());
        let command_id = common::apex::identity::CommandId::generate(
            &mut common::apex::identity::FixedRandomBytesSourceV1([48; 16]),
        )
        .unwrap();
        let perturbed = golden_digest_v1(&ServerGeneral::CommandResult(
            crate::msg::command::CommandPublicationV1 {
                receipt: crate::msg::command::CommandReceiptV1 {
                    command_id,
                    identity_root: [49u8; 32],
                    outcome: crate::msg::command::CommandOutcomeV1::Applied {
                        result_digest: [99u8; 32],
                    },
                    effect_epoch: 51,
                },
                sequence: 52,
            },
        ));
        assert_ne!(
            base, perturbed,
            "changing CommandResult's nested result_digest did not move the digest -- the \
             golden mechanism is blind to this chunk's payload"
        );
    }

    /// WSG-2 chunk 9's falsifier: perturbing `PluginArtifactData`'s
    /// `bytes` payload proves the mechanism sees a change in the raw
    /// `Vec<u8>` field, not just the descriptor's fixed-size fields.
    #[test]
    fn chunk_9_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&server_plugin_artifact_data());
        let perturbed = golden_digest_v1(&ServerGeneral::PluginArtifactData(
            crate::msg::plugin_artifact::PluginArtifactResponseV1 {
                descriptor: crate::msg::plugin_artifact::PluginArtifactDescriptorV1 {
                    deployment_root: [53u8; 32],
                    ordinal: 54,
                    digest: [55u8; 32],
                    size_bytes: 56,
                },
                bytes: vec![57, 58, 60],
            },
        ));
        assert_ne!(
            base, perturbed,
            "changing PluginArtifactData's bytes payload did not move the digest -- the golden \
             mechanism is blind to this chunk's payload"
        );
    }

    /// WSG-2 chunk 10's falsifier: perturbing `UnlockSkill`'s inner
    /// `SwordSkill` variant proves the mechanism sees a change inside a
    /// doubly-nested enum (`Skill::Sword(SwordSkill)`), not just the
    /// outer discriminant.
    #[test]
    fn chunk_10_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&client_unlock_skill());
        let perturbed = golden_digest_v1(&ClientGeneral::UnlockSkill(common::comp::Skill::Sword(
            common::comp::skillset::skills::SwordSkill::FellStrike,
        )));
        assert_ne!(
            base, perturbed,
            "changing UnlockSkill's inner SwordSkill did not move the digest -- the golden \
             mechanism is blind to this chunk's payload"
        );
    }

    /// WSG-2 chunk 11's falsifier: perturbing `UpdatePendingTrade`'s
    /// `phase` field (the tuple's middle element, inside the nested
    /// `PendingTrade` struct) proves the mechanism sees a change in a
    /// multi-arity tuple variant's non-first field.
    #[test]
    fn chunk_11_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&server_update_pending_trade());
        let perturbed = golden_digest_v1(&ServerGeneral::UpdatePendingTrade(
            trade_id(60),
            common::trade::PendingTrade {
                parties: [uid(61), uid(62)],
                offers: [hashbrown::HashMap::new(), hashbrown::HashMap::new()],
                phase: common::trade::TradePhase::Review,
                accept_flags: [false, false],
            },
            None,
        ));
        assert_ne!(
            base, perturbed,
            "changing UpdatePendingTrade's phase did not move the digest -- the golden \
             mechanism is blind to this chunk's payload"
        );
    }

    /// WSG-2 chunk 12's falsifier (the closing chunk): perturbing
    /// `CheckpointBegin`'s `streams` array -- one entry three levels deep
    /// inside `Box<CheckpointStreamOpenV1> -> CheckpointDescriptorV1 ->
    /// [StreamCheckpointPlanV1; 5]` -- proves the mechanism sees a change
    /// at the bottom of the deepest payload WSG-2 covers.
    #[test]
    fn chunk_12_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&server_checkpoint_begin());
        let binding = crate::msg::envelope::ActiveSessionBindingV1 {
            server_boot_id: common::apex::identity::ServerBootId::generate(
                &mut common::apex::identity::FixedRandomBytesSourceV1([154; 16]),
            )
            .unwrap(),
            session_id: common::apex::identity::SessionId::generate(
                &mut common::apex::identity::FixedRandomBytesSourceV1([155; 16]),
            )
            .unwrap(),
            epoch: common::apex::identity::ConnectionEpoch::FIRST,
        };
        let mut perturbed_terrain_plan =
            stream_plan(crate::msg::envelope::SemanticStreamIdV1::Terrain);
        perturbed_terrain_plan.data_record_count = 99;
        let perturbed = golden_digest_v1(&ServerGeneral::CheckpointBegin(Box::new(
            crate::msg::checkpoint::CheckpointStreamOpenV1 {
                begin: crate::msg::checkpoint::CheckpointBeginV1 {
                    epoch: 156,
                    stream: crate::msg::envelope::SemanticStreamIdV1::General,
                    descriptor_root: [157u8; 32],
                },
                descriptor: crate::msg::checkpoint::CheckpointDescriptorV1 {
                    schema_version: 1,
                    binding,
                    epoch: 156,
                    parent_epoch: 155,
                    resource_profile_root: [158u8; 32],
                    apply_policy_root: [159u8; 32],
                    egress_order_policy_root: [160u8; 32],
                    data_record_count: 0,
                    ordinal_max: 0,
                    payload_bytes: 0,
                    global_transcript_root: [161u8; 32],
                    streams: [
                        stream_plan(crate::msg::envelope::SemanticStreamIdV1::Bootstrap),
                        stream_plan(crate::msg::envelope::SemanticStreamIdV1::CharacterScreen),
                        stream_plan(crate::msg::envelope::SemanticStreamIdV1::InGame),
                        stream_plan(crate::msg::envelope::SemanticStreamIdV1::General),
                        perturbed_terrain_plan,
                    ],
                    bootstrap_manifest_root: None,
                },
            },
        )));
        assert_ne!(
            base, perturbed,
            "changing CheckpointBegin's nested streams[Terrain].data_record_count did not move \
             the digest -- the golden mechanism is blind to this chunk's payload"
        );
    }

    /// `APEX-T4.1` chunk 2a's falsifier: perturbing the manifest's sole
    /// descriptor content (still wrapped through the REAL
    /// `BootstrapManifestWireV1::from_typed_v1` encoder, not a hand-built
    /// byte string) moves the golden.
    #[test]
    fn t4_1_chunk_2a_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&server_bootstrap_manifest());
        let perturbed_manifest = common::apex::bootstrap_manifest::BootstrapManifestV1 {
            descriptors: vec![common::apex::subsystem::SubsystemDescriptorV1 {
                slot: common::apex::subsystem::SubsystemSlotIdV1::NetEnvelope,
                schema: common::apex::scalar::SchemaVersion::new(1),
                content: common::apex::digest::ContentIdentityV1 {
                    artifact: common::apex::digest::hash_artifact_bytes_v1(b"DIFFERENT-wire-schema-fixture"),
                    semantic: None,
                },
            }],
            peer_selector: None,
            peer_capabilities: Vec::new(),
            freshness: None,
        };
        let perturbed = golden_digest_v1(&ServerGeneral::BootstrapManifest(
            crate::msg::bootstrap_manifest_wire::BootstrapManifestWireV1::from_typed_v1(&perturbed_manifest)
                .expect("perturbed fixture manifest encodes"),
        ));
        assert_ne!(
            base, perturbed,
            "changing BootstrapManifest's descriptor content did not move the digest -- the \
             golden mechanism is blind to this chunk's payload"
        );
    }

    /// WSG-2 chunk 4's falsifier: a perturbation of one of this chunk's
    /// new fixtures reaches the golden and moves the digest, proven
    /// against a `ServerGeneral` variant this chunk added (`CreateEntity`,
    /// the one with actual field structure rather than a bare unit
    /// enum-of-enums) rather than the pre-existing `LocalWindUpdate`
    /// example.
    #[test]
    fn chunk_4_fixture_perturbation_moves_the_digest() {
        let base = golden_digest_v1(&server_create_entity());
        let perturbed = golden_digest_v1(&ServerGeneral::CreateEntity(crate::sync::EntityPackage {
            uid: uid(114),
            comps: Vec::new(),
        }));
        assert_ne!(
            base, perturbed,
            "changing CreateEntity's uid did not move the digest -- the golden mechanism is \
             blind to this chunk's payload"
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
