//! BUILD-007A10.4 (part 1 of 2) — stable entity/loadout/figure-key/draw
//! identity substrate (design §9).
//!
//! - §9.1 `RendererSemanticEntityDigestV1` = the full 256-bit domain-separated
//!   digest is authority; a run-local compact `RendererBenchEntityId` alias is
//!   assigned by a validated bijection (sort by 32 digest bytes, reject
//!   duplicates, contiguous `1..=N`, no truncation-as-collision).
//! - §9.2 semantic path grammar (`fixture/…`, `asset/…`, `figure-key/…`) over
//!   the §4.3 machine-id ASCII grammar; display text never enters identity.
//!
//! The live typed-fixture projection (§9.3) and atomic ECS commit (§9.5) touch
//! the real server/ECS and land in the integration packet; this module is the
//! self-contained digest/alias/grammar core with golden vectors.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// Typed identity failures (§9). Every one is terminal — a duplicate digest or
/// malformed path never produces a best-effort alias.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityError {
    /// A semantic path violated the §9.2 grammar.
    InvalidSemanticPath { path: String, reason: PathReason },
    /// Two entities produced the same full digest (§9.1 step 3).
    DuplicateDigest { digest: [u8; 32] },
    /// More than `u32::MAX - 1` entities — the compact alias space is exhausted.
    AliasSpaceExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathReason {
    UnknownForm,
    WrongArity { expected: usize, got: usize },
    EmptySegment,
    DotSegment,
    IllegalChar,
    BadBoundary,
    TooLong,
}

/// The three canonical semantic-path forms (§9.2). Each is exactly four
/// slash-separated fields including the leading form tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticPathKind {
    /// `fixture/<scenario-id>/<entity-kind>/<zero-padded-ordinal>`
    Fixture,
    /// `asset/<plugin-id>/<content-digest>/<asset-id>`
    Asset,
    /// `figure-key/<schema-major>/<entity-digest>/<presentation-digest>`
    FigureKey,
}

/// True if `seg` is a valid §4.3 machine identifier SEGMENT (no slash inside a
/// segment): `[a-z0-9](?:[a-z0-9._-]{0,94}[a-z0-9])?`, and not `.`/`..`.
fn valid_segment(seg: &str) -> Result<(), PathReason> {
    if seg.is_empty() {
        return Err(PathReason::EmptySegment);
    }
    if seg == "." || seg == ".." {
        return Err(PathReason::DotSegment);
    }
    if seg.len() > 96 {
        return Err(PathReason::TooLong);
    }
    let b = seg.as_bytes();
    let is_inner = |c: u8| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'_' | b'-');
    let is_edge = |c: u8| c.is_ascii_lowercase() || c.is_ascii_digit();
    for &c in b {
        if !is_inner(c) {
            return Err(PathReason::IllegalChar);
        }
    }
    if !is_edge(b[0]) || !is_edge(b[b.len() - 1]) {
        return Err(PathReason::BadBoundary);
    }
    Ok(())
}

/// Parse and validate a semantic path (§9.2). Returns the form kind, or a typed
/// reason. Display text is not accepted here — identity is machine-only.
pub fn parse_semantic_path(path: &str) -> Result<SemanticPathKind, IdentityError> {
    let err = |reason| IdentityError::InvalidSemanticPath {
        path: path.to_string(),
        reason,
    };
    let parts: Vec<&str> = path.split('/').collect();
    let kind = match parts.first() {
        Some(&"fixture") => SemanticPathKind::Fixture,
        Some(&"asset") => SemanticPathKind::Asset,
        Some(&"figure-key") => SemanticPathKind::FigureKey,
        _ => return Err(err(PathReason::UnknownForm)),
    };
    if parts.len() != 4 {
        return Err(err(PathReason::WrongArity {
            expected: 4,
            got: parts.len(),
        }));
    }
    // The leading form tag is fixed; validate the three identity segments.
    for seg in &parts[1..] {
        valid_segment(seg).map_err(err)?;
    }
    Ok(kind)
}

/// The full authoritative entity digest (§9.1 / §4.4): the domain-separated
/// length-framed hash of the validated canonical semantic path.
pub fn semantic_entity_digest(path: &str) -> Result<[u8; 32], IdentityError> {
    parse_semantic_path(path)?;
    Ok(crate::domain_hash("bastion/r0d/entity", 1, 0, path.as_bytes()))
}

/// A validated full-digest ↔ compact-alias bijection (§9.1).
#[derive(Clone, Debug)]
pub struct EntityBijectionV1 {
    /// `by_alias[i]` is the digest of alias `i + 1`.
    by_alias: Vec<[u8; 32]>,
    to_alias: BTreeMap<[u8; 32], NonZeroU32>,
}

impl EntityBijectionV1 {
    /// Assign contiguous compact aliases (§9.1 steps 1-4): sort digests
    /// lexicographically by their 32 bytes, reject any duplicate, assign
    /// `1..=N` with no gaps. Truncation is never treated as collision authority.
    pub fn assign(mut digests: Vec<[u8; 32]>) -> Result<Self, IdentityError> {
        digests.sort();
        if digests.len() > (u32::MAX - 1) as usize {
            return Err(IdentityError::AliasSpaceExhausted);
        }
        let mut to_alias = BTreeMap::new();
        for (i, d) in digests.iter().enumerate() {
            let alias = NonZeroU32::new((i + 1) as u32).expect("i+1 >= 1");
            if to_alias.insert(*d, alias).is_some() {
                return Err(IdentityError::DuplicateDigest { digest: *d });
            }
        }
        Ok(Self {
            by_alias: digests,
            to_alias,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_alias.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_alias.is_empty()
    }

    /// Full digest for a compact alias.
    #[must_use]
    pub fn digest_of(&self, alias: NonZeroU32) -> Option<&[u8; 32]> {
        self.by_alias.get((alias.get() - 1) as usize)
    }

    /// Compact alias for a full digest.
    #[must_use]
    pub fn alias_of(&self, digest: &[u8; 32]) -> Option<NonZeroU32> {
        self.to_alias.get(digest).copied()
    }

    /// Length-framed digest over the whole bijection table (in alias order), so
    /// two runs that produced the same entity set share one table digest
    /// regardless of insertion order.
    #[must_use]
    pub fn table_digest(&self) -> [u8; 32] {
        let mut payload = Vec::with_capacity(8 + self.by_alias.len() * 32);
        payload.extend_from_slice(&(self.by_alias.len() as u64).to_le_bytes());
        for d in &self.by_alias {
            payload.extend_from_slice(d);
        }
        crate::domain_hash("bastion/r0d/entity-bijection", 1, 0, &payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    #[test]
    fn accepts_each_canonical_form() {
        assert_eq!(
            parse_semantic_path("fixture/arena-01/humanoid/0007").unwrap(),
            SemanticPathKind::Fixture
        );
        assert_eq!(
            parse_semantic_path("asset/core-plugin/deadbeef/head").unwrap(),
            SemanticPathKind::Asset
        );
        assert_eq!(
            parse_semantic_path("figure-key/1/abcd/ef01").unwrap(),
            SemanticPathKind::FigureKey
        );
    }

    #[test]
    fn rejects_malformed_paths() {
        use PathReason::*;
        let r = |p: &str| match parse_semantic_path(p) {
            Err(IdentityError::InvalidSemanticPath { reason, .. }) => reason,
            other => panic!("expected path error, got {other:?}"),
        };
        assert_eq!(r("weird/a/b/c"), UnknownForm);
        assert_eq!(r("fixture/a/b"), WrongArity { expected: 4, got: 3 });
        assert_eq!(r("fixture/a/b/c/d"), WrongArity { expected: 4, got: 5 });
        assert_eq!(r("fixture//b/c"), EmptySegment);
        assert_eq!(r("fixture/./b/c"), DotSegment);
        assert_eq!(r("fixture/a/UP/c"), IllegalChar);
        assert_eq!(r("fixture/a/-b/c"), BadBoundary);
    }

    #[test]
    fn display_text_cannot_enter_identity() {
        // A path with spaces / punctuation (display-style) is rejected outright.
        assert!(parse_semantic_path("fixture/arena 01/humanoid/1").is_err());
    }

    #[test]
    fn frozen_entity_digest_vector() {
        let d = semantic_entity_digest("fixture/arena-01/humanoid/0007").unwrap();
        assert_eq!(
            hex_bytes(&d),
            "03494581497d9e468c74f774a02a5c13e96dd914cb65768238d72124b765884d",
            "frozen entity digest drift"
        );
    }

    fn dig(b: u8) -> [u8; 32] {
        [b; 32]
    }

    #[test]
    fn bijection_is_sorted_contiguous_and_order_independent() {
        let a = EntityBijectionV1::assign(vec![dig(3), dig(1), dig(2)]).unwrap();
        // Aliases assigned in sorted-digest order, 1..=3.
        assert_eq!(a.digest_of(NonZeroU32::new(1).unwrap()), Some(&dig(1)));
        assert_eq!(a.digest_of(NonZeroU32::new(3).unwrap()), Some(&dig(3)));
        assert_eq!(a.alias_of(&dig(2)), NonZeroU32::new(2));
        // Insertion order cannot change the table digest.
        let b = EntityBijectionV1::assign(vec![dig(1), dig(2), dig(3)]).unwrap();
        assert_eq!(a.table_digest(), b.table_digest());
    }

    #[test]
    fn duplicate_digest_is_typed_failure() {
        assert_eq!(
            EntityBijectionV1::assign(vec![dig(5), dig(5)]).unwrap_err(),
            IdentityError::DuplicateDigest { digest: dig(5) }
        );
    }
}
