//! Stable error codes for `BastionManifestEncodingV1` (`APEX-T0.2`, packet
//! section 7.3). Diagnostic context (byte offsets, field IDs) is carried
//! alongside the code but is never part of canonical bytes.

/// Every terminal failure a V1 codec operation can produce. Numeric values
/// are frozen once referenced by evidence/logs; do not renumber.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestCodecErrorCodeV1 {
    MalformedCbor = 100,
    TrailingData = 101,
    NonPreferredEncoding = 102,
    IndefiniteLengthForbidden = 103,
    UnsupportedMajorType = 104,
    FloatForbidden = 105,
    TagForbidden = 106,
    NullForbidden = 107,
    SimpleValueForbidden = 108,
    FieldKeyType = 109,
    FieldIdOutOfRange = 110,
    FieldIdOrder = 111,
    DuplicateFieldId = 112,
    MalformedText = 113,
    MachineTextNonAscii = 114,
    InvalidCanonicalPath = 115,
    InputByteLimit = 120,
    DepthLimit = 121,
    NodeLimit = 122,
    ArrayItemLimit = 123,
    MapEntryLimit = 124,
    TextLimit = 125,
    ByteStringLimit = 126,
    EncodeLimit = 127,
    UnknownField = 130,
    MissingRequiredField = 131,
    ArrayOrder = 132,
    DuplicateArrayKey = 133,
}

impl ManifestCodecErrorCodeV1 {
    /// The coarser terminal class used by the golden-vector corpus (which
    /// intentionally freezes "what kind of thing went wrong", not which of
    /// several equally-valid internal codes fired for a given malformed
    /// input) — see `PROJECT-BASTION-APEX-MANIFEST-CBOR-GOLDEN-VECTORS-v1.json`'s
    /// own note: "Invalid vectors freeze the required terminal class, not
    /// dependency-specific error text."
    pub const fn terminal_class(self) -> &'static str {
        use ManifestCodecErrorCodeV1::*;
        match self {
            MalformedCbor => "MALFORMED_CBOR",
            TrailingData => "TRAILING_DATA",
            NonPreferredEncoding => "NON_PREFERRED_ENCODING",
            IndefiniteLengthForbidden => "INDEFINITE_LENGTH_FORBIDDEN",
            UnsupportedMajorType | FloatForbidden | TagForbidden | NullForbidden | SimpleValueForbidden => {
                "TYPE_FORBIDDEN"
            },
            FieldKeyType => "FIELD_KEY_TYPE",
            FieldIdOutOfRange => "FIELD_KEY_RANGE",
            FieldIdOrder => "FIELD_ORDER",
            DuplicateFieldId => "DUPLICATE_FIELD",
            MalformedText => "INVALID_UTF8",
            MachineTextNonAscii => "MACHINE_TEXT_NON_ASCII",
            InvalidCanonicalPath => "INVALID_CANONICAL_PATH",
            InputByteLimit => "INPUT_BYTE_LIMIT",
            DepthLimit => "DEPTH_LIMIT",
            NodeLimit => "NODE_LIMIT",
            ArrayItemLimit => "ARRAY_ITEM_LIMIT",
            MapEntryLimit => "MAP_ENTRY_LIMIT",
            TextLimit => "TEXT_LIMIT",
            ByteStringLimit => "BYTE_STRING_LIMIT",
            EncodeLimit => "ENCODE_LIMIT",
            UnknownField => "UNKNOWN_FIELD",
            MissingRequiredField => "MISSING_REQUIRED_FIELD",
            ArrayOrder => "ARRAY_ORDER",
            DuplicateArrayKey => "DUPLICATE_ARRAY_KEY",
        }
    }
}

/// Diagnostic wrapper carrying a stable code plus non-canonical context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestErrorV1 {
    pub code: ManifestCodecErrorCodeV1,
    pub byte_offset: Option<usize>,
    pub field_id: Option<u16>,
    pub detail: &'static str,
}

impl ManifestErrorV1 {
    pub const fn new(code: ManifestCodecErrorCodeV1) -> Self {
        Self { code, byte_offset: None, field_id: None, detail: "" }
    }

    pub const fn at(mut self, byte_offset: usize) -> Self {
        self.byte_offset = Some(byte_offset);
        self
    }

    pub const fn field(mut self, field_id: u16) -> Self {
        self.field_id = Some(field_id);
        self
    }

    pub const fn detail(mut self, detail: &'static str) -> Self {
        self.detail = detail;
        self
    }
}

impl core::fmt::Display for ManifestErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.code.terminal_class())?;
        if let Some(off) = self.byte_offset {
            write!(f, " at byte {off}")?;
        }
        if let Some(fid) = self.field_id {
            write!(f, " (field {fid})")?;
        }
        if !self.detail.is_empty() {
            write!(f, ": {}", self.detail)?;
        }
        Ok(())
    }
}

impl std::error::Error for ManifestErrorV1 {}

/// Encoding-side error (construction/budget only — no parsing concerns).
pub type ManifestCodecErrorV1 = ManifestErrorV1;
/// Decoding-side error (parsing/budget/canonicalization).
pub type ManifestDecodeErrorV1 = ManifestErrorV1;
/// Typed-schema error (field presence/shape, on top of a decoded value tree).
pub type ManifestSchemaErrorV1 = ManifestErrorV1;
