//! Canonical relative path grammar (`APEX-T0.2`, packet section 5.5).
//!
//! Case-fold collision rejection (e.g. `Foo` vs `foo` colliding on a
//! case-insensitive filesystem) is explicitly NOT this codec's job — it
//! belongs to the owning schema (e.g. `APEX-T2.2`). This module only
//! enforces the byte grammar.

use super::error::{ManifestCodecErrorCodeV1, ManifestErrorV1};
use super::text::MachineTextV1;

/// A relative, `/`-separated, normalized path: not empty, no leading or
/// trailing slash, no empty component, no `.`/`..` component, no backslash.
/// Case is preserved and compared bytewise.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CanonicalPathV1(MachineTextV1);

impl CanonicalPathV1 {
    pub fn new(s: impl Into<String>) -> Result<Self, ManifestErrorV1> {
        let text = MachineTextV1::new(s.into())
            .map_err(|_| ManifestErrorV1::new(ManifestCodecErrorCodeV1::InvalidCanonicalPath).detail("non-ASCII path"))?;
        let raw = text.as_str();

        if raw.is_empty() {
            return Err(err("empty path"));
        }
        if raw.contains('\\') {
            return Err(err("backslash forbidden"));
        }
        if raw.starts_with('/') || raw.ends_with('/') {
            return Err(err("leading or trailing slash"));
        }
        for component in raw.split('/') {
            if component.is_empty() {
                return Err(err("empty path component"));
            }
            if component == "." || component == ".." {
                return Err(err("'.' or '..' component"));
            }
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str { self.0.as_str() }
}

fn err(detail: &'static str) -> ManifestErrorV1 {
    ManifestErrorV1::new(ManifestCodecErrorCodeV1::InvalidCanonicalPath).detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_relative_paths() {
        assert!(CanonicalPathV1::new("a/b/c").is_ok());
        assert!(CanonicalPathV1::new("single").is_ok());
    }

    #[test]
    fn rejects_all_aliases() {
        assert!(CanonicalPathV1::new("").is_err(), "empty");
        assert!(CanonicalPathV1::new("/a/b").is_err(), "leading slash");
        assert!(CanonicalPathV1::new("a/b/").is_err(), "trailing slash");
        assert!(CanonicalPathV1::new("a//b").is_err(), "empty component");
        assert!(CanonicalPathV1::new("./a").is_err(), "dot component");
        assert!(CanonicalPathV1::new("a/../b").is_err(), "dotdot component");
        assert!(CanonicalPathV1::new("a\\b").is_err(), "backslash");
        assert!(CanonicalPathV1::new("a/b\u{e9}").is_err(), "non-ascii");
    }

    #[test]
    fn case_is_preserved_and_distinct() {
        let a = CanonicalPathV1::new("Foo/Bar").unwrap();
        let b = CanonicalPathV1::new("foo/bar").unwrap();
        assert_ne!(a, b);
        assert_eq!(a.as_str(), "Foo/Bar");
    }
}
