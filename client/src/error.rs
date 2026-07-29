use authc::AuthClientError;
use common::apex::identity::{ConnectionEpoch, ServerBootId};
use common_net::msg::ClientType;
use common_net::msg::server::{BanInfo, SessionBindingV1};
pub use network::{InitProtocolError, NetworkConnectError, NetworkError};
use network::{ParticipantError, StreamError};
use rustls::Error as RustlsError;
use specs::error::Error as SpecsError;

#[derive(Debug)]
pub enum Error {
    Kicked(String),
    /// APEX-T3.1.10/.12: a boot ID mismatch was detected, either by the
    /// server rejecting `ClientRegister.expected_server_boot_id` (packet
    /// section 7.5's `RegisterError::ServerBootMismatch` mapping) or by
    /// this client comparing `ServerInit::GameSync`'s boot ID against its
    /// own `ServerInfo` observation before constructing `State`. Both are
    /// "this client observed a prior server-process incarnation" -- one
    /// shared variant, field names per the packet's exact contract.
    /// `server_info` is the boot ID this client observed/echoed;
    /// `game_sync` is the boot ID the other side reported back (the
    /// server's current ID at register time, or GameSync's ID at
    /// bootstrap time).
    ServerBootMismatch {
        server_info: ServerBootId,
        game_sync: ServerBootId,
    },
    NetworkErr(NetworkError),
    ParticipantErr(ParticipantError),
    StreamErr(StreamError),
    ServerTimeout,
    ServerShutdown,
    TooManyPlayers,
    NotOnWhitelist,
    AuthErr(String),
    AuthClientError(AuthClientError),
    AuthServerUrlInvalid(String),
    AuthServerNotTrusted,
    HostnameLookupFailed(std::io::Error),
    Banned(BanInfo),
    /// Persisted character data is invalid or missing
    InvalidCharacter,
    //TODO: InvalidAlias,
    Other(String),
    SpecsErr(SpecsError),
    RustlsErr(RustlsError),
    /// APEX-T3.2: `RegisterAnswer` and `GameSync` carried different
    /// `SessionBindingV1`s -- checked before `State` construction, same
    /// shape as `ServerBootMismatch` above (spec section 3.5, canaries
    /// SES-045/046).
    SessionBindingMismatch {
        register_answer: SessionBindingV1,
        game_sync: SessionBindingV1,
    },
    /// APEX-T3.2 (`UNKNOWN-SESSION`).
    UnknownSession,
    /// APEX-T3.2 (`SESSION-PRINCIPAL-MISMATCH`).
    SessionPrincipalMismatch,
    /// APEX-T3.2 (`SESSION-EXPIRED`).
    SessionExpired,
    /// APEX-T3.2 (`STALE-CONNECTION-EPOCH`/`FUTURE-CONNECTION-EPOCH`).
    ConnectionEpochMismatch {
        current: ConnectionEpoch,
        expected: ConnectionEpoch,
    },
    /// APEX-T3.2 (`CONNECTION-EPOCH-EXHAUSTED`).
    ConnectionEpochExhausted,
    /// APEX-T3.2 (`SESSION-CLIENT-TYPE-MISMATCH`).
    SessionClientTypeMismatch {
        session: ClientType,
        requested: ClientType,
    },
    /// APEX-T3.2 (`OLDER-ATTEMPT-SUPERSEDED`): this registration attempt
    /// lost a same-phase race to a newer attempt from the same principal
    /// (e.g. a rapid double-click reconnect) -- not a credential/capacity
    /// failure, just a stale loser. The caller should typically retry.
    OlderAttemptSuperseded,
    /// APEX-T3.3.05 (`INCOMPATIBLE-SEMANTIC-PROTOCOL`).
    IncompatibleSemanticProtocol,
    /// APEX-T3.3.05 (`SEMANTIC-PROTOCOL-MODE-SWITCH`).
    SemanticProtocolModeSwitch,
    /// `T4.1` chunk 2b (`BOOT-005`): the message immediately following
    /// registration on the register stream was not a usable
    /// `ServerGeneral::BootstrapManifest` -- either a different message
    /// arrived (an ordering violation: e.g. `ServerInit::GameSync` sent
    /// directly, a legacy/buggy server skipping the manifest step) or the
    /// bytes failed to decode. Both are "no valid manifest was obtained",
    /// refused before `State::client` construction; `detail` names which.
    BootstrapManifestMissing { detail: String },
    /// `T4.1` chunk 2b (`BOOT-006`): the manifest decoded, but at least
    /// one slot failed T0.5's `evaluate_compatibility_v1` -- a TOTAL
    /// report, every mismatched slot named, not just the first found.
    BootstrapManifestIncompatible { mismatches: Vec<String> },
}

impl From<SpecsError> for Error {
    fn from(err: SpecsError) -> Self { Self::SpecsErr(err) }
}

impl From<RustlsError> for Error {
    fn from(err: RustlsError) -> Self { Self::RustlsErr(err) }
}

impl From<NetworkError> for Error {
    fn from(err: NetworkError) -> Self { Self::NetworkErr(err) }
}

impl From<ParticipantError> for Error {
    fn from(err: ParticipantError) -> Self { Self::ParticipantErr(err) }
}

impl From<StreamError> for Error {
    fn from(err: StreamError) -> Self { Self::StreamErr(err) }
}

impl From<AuthClientError> for Error {
    fn from(err: AuthClientError) -> Self { Self::AuthClientError(err) }
}

/// APEX-T3.1.12: the exact boot-scope check `ServerInit::GameSync` must
/// pass before `State::client` construction. Extracted (not left inline)
/// so `bastion-harness`'s T3.1.17 process-restart fixture exercises the
/// identical production code path.
pub fn check_game_sync_boot_scope(server_info: ServerBootId, game_sync: ServerBootId) -> Result<(), Error> {
    if server_info != game_sync {
        Err(Error::ServerBootMismatch { server_info, game_sync })
    } else {
        Ok(())
    }
}

/// APEX-T3.2: `RegisterAnswer`'s admitted `SessionBindingV1` must equal
/// `GameSync`'s repeated one before `State::client` construction -- the
/// session-layer twin of [`check_game_sync_boot_scope`] above.
pub fn check_session_binding_equality(register_answer: SessionBindingV1, game_sync: SessionBindingV1) -> Result<(), Error> {
    if register_answer != game_sync {
        Err(Error::SessionBindingMismatch { register_answer, game_sync })
    } else {
        Ok(())
    }
}

/// `T4.1` chunk 2b: the ordering gate itself -- the message received in
/// the manifest step must actually BE `ServerGeneral::BootstrapManifest`.
/// Extracted so the ordering invariant ("GameSync before a manifest is
/// refused") is provable from the function's own signature: it consumes
/// a `ServerGeneral`, so a caller that passed it a `ServerInit::GameSync`
/// directly never even type-checks -- the invariant this exists to test
/// is "whatever DID arrive on the wire decodes as a `ServerGeneral` other
/// than `BootstrapManifest`, or fails to decode at all", both `BOOT-005`.
pub fn expect_bootstrap_manifest(
    msg: common_net::msg::ServerGeneral,
) -> Result<common_net::msg::bootstrap_manifest_wire::BootstrapManifestWireV1, Error> {
    match msg {
        common_net::msg::ServerGeneral::BootstrapManifest(wire) => Ok(wire),
        other => Err(Error::BootstrapManifestMissing { detail: format!("expected BootstrapManifest, got {other:?}") }),
    }
}

/// `T4.1` chunk 2b (`BOOT-006`): decode the wire carrier, build this
/// client's own local compatibility profile (today just the `NetEnvelope`
/// slot, matching the server's own minimal-but-real manifest -- see
/// `server/src/sys/msg/register.rs::bootstrap_manifest_v1`), and evaluate
/// through T0.5's `evaluate_compatibility_v1` (never short-circuited, so
/// every slot is checked). Fails closed with every mismatched slot named,
/// not just the first.
pub fn validate_bootstrap_manifest_v1(
    wire: &common_net::msg::bootstrap_manifest_wire::BootstrapManifestWireV1,
) -> Result<(), Error> {
    use common::apex::{
        digest::{ContentIdentityV1, hash_artifact_bytes_v1},
        subsystem::{
            profile::CompatibilityProfileV1,
            report::{CompatibilityOutcomeV1, evaluate_compatibility_v1},
            rule::CompatibilityRuleV1,
            slot::SubsystemSlotIdV1,
            transform::TransformRegistryV1,
        },
    };

    let manifest = wire
        .to_typed_v1()
        .map_err(|e| Error::BootstrapManifestMissing { detail: format!("manifest decode failed: {e:?}") })?;
    let input = manifest.to_evaluation_input_v1(None, TransformRegistryV1::new());

    let local_net_envelope_content = ContentIdentityV1 {
        artifact: hash_artifact_bytes_v1(common_net::msg::envelope::net_envelope_profile_root_v1().as_array()),
        semantic: None,
    };
    let profile = CompatibilityProfileV1::new(vec![(
        SubsystemSlotIdV1::NetEnvelope,
        CompatibilityRuleV1::Exact { content: local_net_envelope_content },
    )])
    .expect("a single-entry profile can never exceed MAX_PROFILE_ENTRIES or duplicate a slot");

    let report = evaluate_compatibility_v1(&profile, &input);
    if report.is_fully_compatible() {
        Ok(())
    } else {
        let mismatches: Vec<String> = report
            .results()
            .iter()
            .filter(|r| !matches!(r.outcome, CompatibilityOutcomeV1::Compatible))
            .map(|r| format!("{:?}: {:?}", r.slot, r.outcome))
            .collect();
        Err(Error::BootstrapManifestIncompatible { mismatches })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apex::identity::FixedRandomBytesSourceV1;

    fn boot_id(seed: u8) -> ServerBootId { ServerBootId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap() }

    /// APEX-T3.1.17 (client-side twin of the harness's server-side reboot
    /// fixture, `bastion-harness --t3-1-17-scenario`): the exact production
    /// function rejects a stale GameSync boot ID and accepts a matching one.
    #[test]
    fn check_game_sync_boot_scope_rejects_stale_and_accepts_same_boot() {
        let boot_a = boot_id(0xA1);
        let boot_b = boot_id(0xB2);
        assert_ne!(boot_a, boot_b);

        // Stale: client observed boot A's ServerInfo but received a GameSync
        // carrying a different (B's) ID.
        match check_game_sync_boot_scope(boot_a, boot_b) {
            Err(Error::ServerBootMismatch { server_info, game_sync }) => {
                assert_eq!(server_info, boot_a);
                assert_eq!(game_sync, boot_b);
            },
            other => panic!("expected ServerBootMismatch, got {other:?}"),
        }

        // Positive control: matching IDs must not be rejected.
        assert!(check_game_sync_boot_scope(boot_b, boot_b).is_ok());
    }

    fn binding(seed: u8, epoch: u64) -> SessionBindingV1 {
        use common::apex::identity::SessionId;
        SessionBindingV1 {
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            epoch: ConnectionEpoch::new(epoch).unwrap(),
            selected_semantic_protocol: common_net::msg::envelope::SemanticProtocolIdV1::Legacy,
        }
    }

    /// APEX-T3.2 (client-side twin of the boot-scope test above, spec
    /// section 3.5): `RegisterAnswer` and `GameSync` disagreeing on the
    /// session binding must be rejected before `State` construction, and
    /// matching bindings must not be.
    #[test]
    fn check_session_binding_equality_rejects_mismatch_and_accepts_same_binding() {
        let a = binding(0x11, 1);
        let b = binding(0x22, 1);
        assert_ne!(a, b);

        match check_session_binding_equality(a, b) {
            Err(Error::SessionBindingMismatch { register_answer, game_sync }) => {
                assert_eq!(register_answer, a);
                assert_eq!(game_sync, b);
            },
            other => panic!("expected SessionBindingMismatch, got {other:?}"),
        }

        assert!(check_session_binding_equality(a, a).is_ok());
    }

    /// `T4.1` chunk 2b, `BOOT-005`, ordering half: whatever arrives in the
    /// manifest step that is NOT `ServerGeneral::BootstrapManifest` -- the
    /// shape a legacy/buggy server takes if it sends `GameSync` directly,
    /// skipping the manifest -- must be refused, never silently accepted
    /// as if it were a (missing) manifest.
    #[test]
    fn expect_bootstrap_manifest_rejects_any_other_variant() {
        match expect_bootstrap_manifest(common_net::msg::ServerGeneral::CharacterSuccess) {
            Err(Error::BootstrapManifestMissing { .. }) => {},
            other => panic!("expected BootstrapManifestMissing, got {other:?}"),
        }
    }

    fn empty_manifest_wire() -> common_net::msg::bootstrap_manifest_wire::BootstrapManifestWireV1 {
        common_net::msg::bootstrap_manifest_wire::BootstrapManifestWireV1::from_typed_v1(
            &common::apex::bootstrap_manifest::BootstrapManifestV1::default(),
        )
        .unwrap()
    }

    /// Positive control for `expect_bootstrap_manifest`: the real variant,
    /// wrapping the real wire value, passes through unchanged.
    #[test]
    fn expect_bootstrap_manifest_accepts_the_real_variant() {
        let wire = empty_manifest_wire();
        let msg = common_net::msg::ServerGeneral::BootstrapManifest(wire.clone());
        assert_eq!(expect_bootstrap_manifest(msg).unwrap(), wire);
    }

    fn manifest_with_net_envelope_content(
        content: common::apex::digest::ContentIdentityV1,
    ) -> common_net::msg::bootstrap_manifest_wire::BootstrapManifestWireV1 {
        use common::apex::{
            bootstrap_manifest::BootstrapManifestV1,
            scalar::SchemaVersion,
            subsystem::{descriptor::SubsystemDescriptorV1, slot::SubsystemSlotIdV1},
        };
        let manifest = BootstrapManifestV1 {
            descriptors: vec![SubsystemDescriptorV1 { slot: SubsystemSlotIdV1::NetEnvelope, schema: SchemaVersion::new(1), content }],
            peer_selector: None,
            peer_capabilities: Vec::new(),
            freshness_reserved: None,
        };
        common_net::msg::bootstrap_manifest_wire::BootstrapManifestWireV1::from_typed_v1(&manifest).unwrap()
    }

    /// `T4.1` chunk 2b, `BOOT-006`, content half: a manifest that decodes
    /// fine but whose `NetEnvelope` content identity does NOT match this
    /// client's own locally-computed root must be refused, with the
    /// mismatched slot named in the error (total-report: not just a bare
    /// boolean failure).
    #[test]
    fn validate_bootstrap_manifest_v1_rejects_net_envelope_content_mismatch() {
        use common::apex::digest::{ContentIdentityV1, hash_artifact_bytes_v1};
        let wrong_content = ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"not-the-real-net-envelope-root"), semantic: None };
        let wire = manifest_with_net_envelope_content(wrong_content);

        match validate_bootstrap_manifest_v1(&wire) {
            Err(Error::BootstrapManifestIncompatible { mismatches }) => {
                assert!(!mismatches.is_empty(), "total-report refusal must name at least one mismatch");
                assert!(
                    mismatches.iter().any(|m| m.contains("NetEnvelope")),
                    "the mismatched slot must be named: {mismatches:?}"
                );
            },
            other => panic!("expected BootstrapManifestIncompatible, got {other:?}"),
        }
    }

    /// Positive control: a manifest whose `NetEnvelope` content identity
    /// genuinely matches this client's own locally-computed root passes.
    #[test]
    fn validate_bootstrap_manifest_v1_accepts_the_real_net_envelope_root() {
        use common::apex::digest::{ContentIdentityV1, hash_artifact_bytes_v1};
        let real_content = ContentIdentityV1 {
            artifact: hash_artifact_bytes_v1(common_net::msg::envelope::net_envelope_profile_root_v1().as_array()),
            semantic: None,
        };
        let wire = manifest_with_net_envelope_content(real_content);
        assert!(validate_bootstrap_manifest_v1(&wire).is_ok());
    }

    /// A manifest with NO `NetEnvelope` descriptor at all (the slot this
    /// client's profile requires is simply absent) must also refuse --
    /// `InvalidInput(NoDescriptorForSlot)` is not `Compatible` either.
    #[test]
    fn validate_bootstrap_manifest_v1_rejects_a_manifest_missing_the_required_slot() {
        let wire = empty_manifest_wire();
        match validate_bootstrap_manifest_v1(&wire) {
            Err(Error::BootstrapManifestIncompatible { mismatches }) => assert!(!mismatches.is_empty()),
            other => panic!("expected BootstrapManifestIncompatible, got {other:?}"),
        }
    }
}
