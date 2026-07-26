//! Digest algorithm registry and fixed-length digest bytes (`APEX-T0.3`,
//! packet sections 5.1, 7.2).

use super::error::{DigestErrorCodeV1, DigestErrorV1};

/// The V1 algorithm registry. ID 1 is permanently `Sha256`; a future
/// algorithm requires a new registered ID, new golden vectors, and
/// explicit compatibility rules — it may not silently replace ID 1.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestAlgorithmIdV1 {
    Sha256 = 1,
}

impl DigestAlgorithmIdV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const fn digest_bytes(self) -> usize {
        match self {
            Self::Sha256 => 32,
        }
    }

    pub fn try_from_u16(v: u16) -> Result<Self, DigestErrorV1> {
        match v {
            1 => Ok(Self::Sha256),
            _ => Err(DigestErrorV1::new(DigestErrorCodeV1::UnsupportedAlgorithm)),
        }
    }
}

/// Exactly 32 raw digest bytes. Truncation is forbidden in V1 — there is
/// no constructor that accepts fewer or more bytes.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DigestBytes32V1([u8; 32]);

impl DigestBytes32V1 {
    pub const fn from_array(bytes: [u8; 32]) -> Self { Self(bytes) }

    pub const fn as_array(&self) -> &[u8; 32] { &self.0 }

    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, DigestErrorV1> {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| DigestErrorV1::new(DigestErrorCodeV1::InvalidDigestLength).detail("expected exactly 32 bytes"))?;
        Ok(Self(arr))
    }

    /// Strict `sha256:<64 lowercase hex>` parser (packet section 7.6).
    /// Rejects uppercase, separators, whitespace, missing/wrong prefix,
    /// wrong length, and non-hex characters.
    pub fn parse_human_v1(s: &str) -> Result<(DigestAlgorithmIdV1, Self), DigestErrorV1> {
        let rest = s
            .strip_prefix("sha256:")
            .ok_or_else(|| DigestErrorV1::new(DigestErrorCodeV1::InvalidDigestText).detail("missing 'sha256:' prefix"))?;
        if rest.len() != 64 {
            return Err(DigestErrorV1::new(DigestErrorCodeV1::InvalidDigestText).detail("wrong hex length"));
        }
        if !rest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(DigestErrorV1::new(DigestErrorCodeV1::InvalidDigestText).detail("non-lowercase-hex byte"));
        }
        let mut out = [0u8; 32];
        for i in 0..32 {
            let hi = hex_nibble(rest.as_bytes()[i * 2])?;
            let lo = hex_nibble(rest.as_bytes()[i * 2 + 1])?;
            out[i] = (hi << 4) | lo;
        }
        Ok((DigestAlgorithmIdV1::Sha256, Self(out)))
    }

    pub fn to_human_v1(&self) -> String {
        let mut s = String::with_capacity(7 + 64);
        s.push_str("sha256:");
        for b in self.0 {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

fn hex_nibble(b: u8) -> Result<u8, DigestErrorV1> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(DigestErrorV1::new(DigestErrorCodeV1::InvalidDigestText).detail("non-lowercase-hex byte")),
    }
}

impl core::fmt::Debug for DigestBytes32V1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{}", self.to_human_v1()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_ids() {
        assert_eq!(DigestAlgorithmIdV1::Sha256.as_u16(), 1);
        assert_eq!(DigestAlgorithmIdV1::Sha256.digest_bytes(), 32);
        assert!(DigestAlgorithmIdV1::try_from_u16(1).is_ok());
        assert!(DigestAlgorithmIdV1::try_from_u16(2).is_err());
    }

    #[test]
    fn exact_32_byte_construction() {
        assert!(DigestBytes32V1::try_from_slice(&[0u8; 32]).is_ok());
        assert!(DigestBytes32V1::try_from_slice(&[0u8; 31]).is_err());
        assert!(DigestBytes32V1::try_from_slice(&[0u8; 33]).is_err());
    }

    #[test]
    fn human_round_trip() {
        let (_, digest) = DigestBytes32V1::parse_human_v1(
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .unwrap();
        assert_eq!(
            digest.to_human_v1(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn human_parser_rejects_malformed() {
        assert!(DigestBytes32V1::parse_human_v1("sha256:").is_err(), "empty");
        assert!(
            DigestBytes32V1::parse_human_v1("SHA256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .is_err(),
            "uppercase prefix"
        );
        assert!(
            DigestBytes32V1::parse_human_v1("sha256:E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855")
                .is_err(),
            "uppercase hex"
        );
        assert!(DigestBytes32V1::parse_human_v1("sha256:abcd").is_err(), "too short");
        assert!(
            DigestBytes32V1::parse_human_v1(
                "sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            )
            .is_err(),
            "whitespace"
        );
        assert!(
            DigestBytes32V1::parse_human_v1("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .is_err(),
            "missing algorithm prefix"
        );
    }
}
