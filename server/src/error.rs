use crate::persistence::error::PersistenceError;
use common::apex::identity::IdentityGenerationErrorV1;
use network::{NetworkError, ParticipantError, StreamError};
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    NetworkErr(NetworkError),
    ParticipantErr(ParticipantError),
    StreamErr(StreamError),
    DatabaseErr(rusqlite::Error),
    PersistenceErr(PersistenceError),
    RtsimError(ron::Error),
    /// APEX-T3.1.03: `ServerBootId` generation failed before any durable or
    /// externally visible startup work began. Typed rather than
    /// `Other(String)` so callers/tests can distinguish this terminal from
    /// every other startup failure (packet section 3.8/10.12: entropy
    /// unavailability must fail closed, never fall back to a
    /// timestamp/PID/zero substitute).
    BootIdentity(IdentityGenerationErrorV1),
    Other(String),
}

impl From<NetworkError> for Error {
    fn from(err: NetworkError) -> Self { Error::NetworkErr(err) }
}

impl From<ParticipantError> for Error {
    fn from(err: ParticipantError) -> Self { Error::ParticipantErr(err) }
}

impl From<StreamError> for Error {
    fn from(err: StreamError) -> Self { Error::StreamErr(err) }
}

// TODO: Don't expose rusqlite::Error from persistence module
impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self { Error::DatabaseErr(err) }
}

impl From<PersistenceError> for Error {
    fn from(err: PersistenceError) -> Self { Error::PersistenceErr(err) }
}

impl From<IdentityGenerationErrorV1> for Error {
    fn from(err: IdentityGenerationErrorV1) -> Self { Error::BootIdentity(err) }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NetworkErr(err) => write!(f, "Network Error: {}", err),
            Self::ParticipantErr(err) => write!(f, "Participant Error: {}", err),
            Self::StreamErr(err) => write!(f, "Stream Error: {}", err),
            Self::DatabaseErr(err) => write!(f, "Database Error: {}", err),
            Self::PersistenceErr(err) => write!(f, "Persistence Error: {}", err),
            Self::RtsimError(err) => write!(f, "Rtsim Error: {}", err),
            Self::BootIdentity(err) => write!(f, "Server boot identity generation failed: {}", err),
            Self::Other(err) => write!(f, "Error: {}", err),
        }
    }
}
