//! Stable error types for `common::apex::identity` (`APEX-T0.4`, packet
//! section 7.7).

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityGenerationErrorV1 {
    EntropyUnavailable,
    GeneratedInvariantViolation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityDecodeErrorV1 {
    WrongByteLength { actual: u64 },
    NilUuid,
    WrongUuidVersion { actual: Option<u8> },
    WrongUuidVariant,
    WrongTextPrefix,
    InvalidText,
    ZeroReserved,
}

impl IdentityDecodeErrorV1 {
    /// Coarser terminal class matching the golden-vector corpus's
    /// `terminal` strings (same pattern as `ManifestCodecErrorCodeV1::
    /// terminal_class` in `APEX-T0.2`: several fine-grained internal
    /// variants can share one externally-frozen terminal label).
    pub const fn terminal_class(&self) -> &'static str {
        match self {
            Self::WrongByteLength { .. } => "WRONG_BYTE_LENGTH",
            Self::NilUuid | Self::WrongUuidVersion { .. } | Self::WrongUuidVariant => "INVALID_UUID_VERSION_VARIANT",
            Self::WrongTextPrefix => "WRONG_TYPE_PREFIX",
            Self::InvalidText => "INVALID_TEXT",
            Self::ZeroReserved => "ZERO_RESERVED",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterAdvanceErrorV1 {
    Exhausted,
}

impl CounterAdvanceErrorV1 {
    pub const fn terminal_class(&self) -> &'static str {
        match self {
            Self::Exhausted => "COUNTER_EXHAUSTED",
        }
    }
}
