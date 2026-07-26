//! Canonical machine text (`APEX-T0.2`, packet section 5.4).
//!
//! V1 identity-bearing text is ASCII-only. A future Unicode identity type
//! must pin its own normalization profile rather than have this type
//! silently rewrite signed/hashed bytes (packet external evidence 4.5).

use super::error::{ManifestCodecErrorCodeV1, ManifestErrorV1};

/// A `String` guaranteed, at construction time, to contain only ASCII bytes
/// with no NUL and no C0/C1 control characters.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MachineTextV1(String);

impl MachineTextV1 {
    /// Construct from an already-validated UTF-8 Rust `String`, checking the
    /// ASCII/control-character policy. Used both by the public constructor
    /// and by the decoder (which has already proven the bytes are valid
    /// UTF-8 before calling this).
    pub fn new(s: impl Into<String>) -> Result<Self, ManifestErrorV1> {
        let s = s.into();
        for b in s.bytes() {
            if b == 0 {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::MachineTextNonAscii)
                    .detail("NUL byte is forbidden"));
            }
            if b > 0x7F {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::MachineTextNonAscii)
                    .detail("non-ASCII byte"));
            }
            // C0 controls are 0x00-0x1F (0x00 handled above as NUL) plus
            // 0x7F (DEL). C1 controls (0x80-0x9F) are unreachable here
            // because we already rejected b > 0x7F above.
            if b < 0x20 || b == 0x7F {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::MachineTextNonAscii)
                    .detail("C0 control character"));
            }
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str { &self.0 }

    pub fn into_string(self) -> String { self.0 }
}

impl core::fmt::Display for MachineTextV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{}", self.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_empty_and_ascii() {
        assert!(MachineTextV1::new("").is_ok());
        assert!(MachineTextV1::new("bastion.manifest/v1").is_ok());
        assert!(MachineTextV1::new("A-Za-z0-9_./").is_ok());
    }

    #[test]
    fn rejects_non_ascii() {
        let err = MachineTextV1::new("caf\u{e9}").unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::MachineTextNonAscii);
    }

    #[test]
    fn rejects_nul_and_control_chars() {
        assert!(MachineTextV1::new("a\0b").is_err());
        assert!(MachineTextV1::new("a\tb").is_err());
        assert!(MachineTextV1::new("a\nb").is_err());
        assert!(MachineTextV1::new("a\x7fb").is_err());
    }

    #[test]
    fn boundary_bytes() {
        assert!(MachineTextV1::new("\x20").is_ok()); // space, first allowed
        assert!(MachineTextV1::new("\x7e").is_ok()); // '~', last allowed
        assert!(MachineTextV1::new("\x1f").is_err()); // last C0 control
        assert!(MachineTextV1::new("\x7f").is_err()); // DEL
    }
}
