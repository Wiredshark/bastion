use authc::AuthClientError;
use common::apex::identity::ServerBootId;
use common_net::msg::server::BanInfo;
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
