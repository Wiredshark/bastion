//! Stable error codes for `common::apex::digest` (`APEX-T0.3`, packet
//! section 7.5).

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestErrorCodeV1 {
    UnsupportedAlgorithm = 1,
    InvalidDigestLength = 2,
    UnknownDomain = 3,
    DomainRegistryMismatch = 4,
    InputTooLarge = 5,
    SizeOverflow = 6,
    InvalidDigestText = 7,
    ManifestEncodeFailed = 8,
    SemanticSchemaUnavailable = 9,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigestErrorV1 {
    pub code: DigestErrorCodeV1,
    pub detail: &'static str,
}

impl DigestErrorV1 {
    pub const fn new(code: DigestErrorCodeV1) -> Self { Self { code, detail: "" } }

    pub const fn detail(mut self, detail: &'static str) -> Self {
        self.detail = detail;
        self
    }
}

impl core::fmt::Display for DigestErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.code)?;
        if !self.detail.is_empty() {
            write!(f, ": {}", self.detail)?;
        }
        Ok(())
    }
}

impl std::error::Error for DigestErrorV1 {}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactVerificationErrorCodeV1 {
    SizeMismatch = 100,
    DigestMismatch = 101,
    InputTooLarge = 102,
    IoFailure = 103,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactVerificationErrorV1 {
    pub code: ArtifactVerificationErrorCodeV1,
    pub detail: &'static str,
}

impl ArtifactVerificationErrorV1 {
    pub const fn new(code: ArtifactVerificationErrorCodeV1) -> Self { Self { code, detail: "" } }

    pub const fn detail(mut self, detail: &'static str) -> Self {
        self.detail = detail;
        self
    }
}

impl core::fmt::Display for ArtifactVerificationErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.code)?;
        if !self.detail.is_empty() {
            write!(f, ": {}", self.detail)?;
        }
        Ok(())
    }
}

impl std::error::Error for ArtifactVerificationErrorV1 {}
