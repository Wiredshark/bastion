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
}
