//! Stable renderer-owned semantic identity.
//!
//! Full 256-bit digests are authority. Compact aliases are a checked,
//! contiguous run-local projection and never participate in semantic identity.

use std::{collections::BTreeMap, num::NonZeroU32};

use crate::{DomainHashErrorV1, domain_hash_v1};

pub const MAX_SEMANTIC_PATH_BYTES_V1: usize = 384;
pub const MAX_SEMANTIC_SEGMENT_BYTES_V1: usize = 96;
pub const MAX_RENDERER_ENTITIES_V1: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPathKindV1 {
    Fixture,
    Asset,
    FigureKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPathReasonV1 {
    Empty,
    NonAscii,
    TooLong,
    UnknownKind,
    WrongArity,
    EmptySegment,
    DotSegment,
    SegmentTooLong,
    IllegalCharacter,
    BadBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityErrorV1 {
    InvalidSemanticPath(SemanticPathReasonV1),
    HashFailure(DomainHashErrorV1),
    TooManyEntities { count: usize, cap: usize },
    DuplicateDigest([u8; 32]),
    AliasOutOfRange(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererSemanticIdentityV1 {
    path: String,
    kind: SemanticPathKindV1,
    digest: [u8; 32],
}

impl RendererSemanticIdentityV1 {
    pub fn parse(path: &str) -> Result<Self, IdentityErrorV1> {
        let kind = validate_path(path)?;
        let digest = domain_hash_v1("bastion/r0d/semantic-entity", 1, 0, path.as_bytes())
            .map_err(IdentityErrorV1::HashFailure)?;
        Ok(Self {
            path: path.to_owned(),
            kind,
            digest,
        })
    }

    #[must_use]
    pub fn path(&self) -> &str { &self.path }

    #[must_use]
    pub const fn kind(&self) -> SemanticPathKindV1 { self.kind }

    #[must_use]
    pub const fn digest(&self) -> [u8; 32] { self.digest }
}

fn validate_path(path: &str) -> Result<SemanticPathKindV1, IdentityErrorV1> {
    use SemanticPathReasonV1 as Reason;

    let invalid = |reason| IdentityErrorV1::InvalidSemanticPath(reason);
    if path.is_empty() {
        return Err(invalid(Reason::Empty));
    }
    if !path.is_ascii() {
        return Err(invalid(Reason::NonAscii));
    }
    if path.len() > MAX_SEMANTIC_PATH_BYTES_V1 {
        return Err(invalid(Reason::TooLong));
    }

    let mut segments = path.split('/');
    let kind = match segments.next() {
        Some("fixture") => SemanticPathKindV1::Fixture,
        Some("asset") => SemanticPathKindV1::Asset,
        Some("figure-key") => SemanticPathKindV1::FigureKey,
        _ => return Err(invalid(Reason::UnknownKind)),
    };
    let mut count = 1_usize;
    for segment in segments {
        count = count.saturating_add(1);
        validate_segment(segment).map_err(invalid)?;
    }
    if count != 4 {
        return Err(invalid(Reason::WrongArity));
    }
    Ok(kind)
}

fn validate_segment(segment: &str) -> Result<(), SemanticPathReasonV1> {
    use SemanticPathReasonV1 as Reason;

    if segment.is_empty() {
        return Err(Reason::EmptySegment);
    }
    if segment == "." || segment == ".." {
        return Err(Reason::DotSegment);
    }
    if segment.len() > MAX_SEMANTIC_SEGMENT_BYTES_V1 {
        return Err(Reason::SegmentTooLong);
    }
    let bytes = segment.as_bytes();
    let edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    let inner = |byte: u8| edge(byte) || matches!(byte, b'.' | b'_' | b'-');
    if !edge(bytes[0]) || !edge(bytes[bytes.len() - 1]) {
        return Err(Reason::BadBoundary);
    }
    if bytes.iter().copied().any(|byte| !inner(byte)) {
        return Err(Reason::IllegalCharacter);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererEntityAliasTableV1 {
    by_alias: Vec<[u8; 32]>,
    by_digest: BTreeMap<[u8; 32], NonZeroU32>,
    table_digest: [u8; 32],
}

impl RendererEntityAliasTableV1 {
    pub fn assign(mut digests: Vec<[u8; 32]>) -> Result<Self, IdentityErrorV1> {
        if digests.len() > MAX_RENDERER_ENTITIES_V1 {
            return Err(IdentityErrorV1::TooManyEntities {
                count: digests.len(),
                cap: MAX_RENDERER_ENTITIES_V1,
            });
        }
        digests.sort_unstable();
        if let Some(duplicate) = digests.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(IdentityErrorV1::DuplicateDigest(duplicate[0]));
        }

        let mut by_digest = BTreeMap::new();
        for (index, digest) in digests.iter().copied().enumerate() {
            let one_based = index
                .checked_add(1)
                .and_then(|value| u32::try_from(value).ok())
                .and_then(NonZeroU32::new)
                .ok_or(IdentityErrorV1::TooManyEntities {
                    count: digests.len(),
                    cap: MAX_RENDERER_ENTITIES_V1,
                })?;
            by_digest.insert(digest, one_based);
        }

        let mut payload = Vec::with_capacity(8 + digests.len().saturating_mul(32));
        payload.extend_from_slice(
            &u64::try_from(digests.len())
                .map_err(|_| IdentityErrorV1::TooManyEntities {
                    count: digests.len(),
                    cap: MAX_RENDERER_ENTITIES_V1,
                })?
                .to_le_bytes(),
        );
        for digest in &digests {
            payload.extend_from_slice(digest);
        }
        let table_digest = domain_hash_v1("bastion/r0d/entity-alias-table", 1, 0, &payload)
            .map_err(IdentityErrorV1::HashFailure)?;

        Ok(Self {
            by_alias: digests,
            by_digest,
            table_digest,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize { self.by_alias.len() }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.by_alias.is_empty() }

    pub fn digest_for_alias(&self, alias: u32) -> Result<[u8; 32], IdentityErrorV1> {
        let nonzero = NonZeroU32::new(alias).ok_or(IdentityErrorV1::AliasOutOfRange(alias))?;
        let index = usize::try_from(nonzero.get() - 1)
            .map_err(|_| IdentityErrorV1::AliasOutOfRange(alias))?;
        self.by_alias
            .get(index)
            .copied()
            .ok_or(IdentityErrorV1::AliasOutOfRange(alias))
    }

    #[must_use]
    pub fn alias_for_digest(&self, digest: &[u8; 32]) -> Option<NonZeroU32> {
        self.by_digest.get(digest).copied()
    }

    #[must_use]
    pub const fn table_digest(&self) -> [u8; 32] { self.table_digest }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    #[test]
    fn path_grammar_accepts_exact_machine_forms() {
        let fixture = RendererSemanticIdentityV1::parse("fixture/arena-01/humanoid/0007").unwrap();
        assert_eq!(fixture.kind(), SemanticPathKindV1::Fixture);
        assert_eq!(fixture.path(), "fixture/arena-01/humanoid/0007");
        assert_eq!(
            RendererSemanticIdentityV1::parse("asset/core-plugin/deadbeef/head")
                .unwrap()
                .kind(),
            SemanticPathKindV1::Asset
        );
        assert_eq!(
            RendererSemanticIdentityV1::parse("figure-key/1/abcd/ef01")
                .unwrap()
                .kind(),
            SemanticPathKindV1::FigureKey
        );
    }

    #[test]
    fn path_grammar_rejects_every_invalid_class() {
        use SemanticPathReasonV1 as Reason;
        let reason = |path: &str| match RendererSemanticIdentityV1::parse(path) {
            Err(IdentityErrorV1::InvalidSemanticPath(value)) => value,
            other => panic!("expected path error, got {other:?}"),
        };
        assert_eq!(reason(""), Reason::Empty);
        assert_eq!(reason("fixture/a/é/1"), Reason::NonAscii);
        assert_eq!(reason("other/a/b/c"), Reason::UnknownKind);
        assert_eq!(reason("fixture/a/b"), Reason::WrongArity);
        assert_eq!(reason("fixture//b/c"), Reason::EmptySegment);
        assert_eq!(reason("fixture/./b/c"), Reason::DotSegment);
        assert_eq!(reason("fixture/a/b!d/c"), Reason::IllegalCharacter);
        assert_eq!(reason("fixture/a/-bad/c"), Reason::BadBoundary);
        assert_eq!(
            reason(&format!("fixture/{}/b/c", "a".repeat(97))),
            Reason::SegmentTooLong
        );
        assert_eq!(
            reason(&format!(
                "fixture/{}/{}/{}/{}",
                "a".repeat(96),
                "b".repeat(96),
                "c".repeat(96),
                "d".repeat(96)
            )),
            Reason::TooLong
        );
    }

    #[test]
    fn frozen_semantic_digest_vector() {
        let identity = RendererSemanticIdentityV1::parse("fixture/arena-01/humanoid/0007").unwrap();
        assert_eq!(
            hex_bytes(&identity.digest()),
            "76f96adf310f66d9b17d5e00dddb2c3c7b01c6ce3d143a0d572137d7ca7f349b"
        );
    }

    #[test]
    fn alias_table_is_a_stable_contiguous_bijection() {
        let a = RendererEntityAliasTableV1::assign(vec![digest(3), digest(1), digest(2)]).unwrap();
        let b = RendererEntityAliasTableV1::assign(vec![digest(2), digest(3), digest(1)]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.digest_for_alias(1), Ok(digest(1)));
        assert_eq!(a.digest_for_alias(2), Ok(digest(2)));
        assert_eq!(a.digest_for_alias(3), Ok(digest(3)));
        assert_eq!(a.alias_for_digest(&digest(2)).map(NonZeroU32::get), Some(2));
        assert_eq!(a.table_digest(), b.table_digest());
        assert_eq!(
            hex_bytes(&a.table_digest()),
            "0e3a8b3cba7b9cdb085d34329f7df8a4b29030d2d2e914d0e7f93452ebef2438"
        );
        assert_eq!(
            a.digest_for_alias(0),
            Err(IdentityErrorV1::AliasOutOfRange(0))
        );
        assert_eq!(
            a.digest_for_alias(4),
            Err(IdentityErrorV1::AliasOutOfRange(4))
        );
    }

    #[test]
    fn duplicate_and_entity_count_bounds_fail_closed() {
        assert_eq!(
            RendererEntityAliasTableV1::assign(vec![digest(1), digest(1)]),
            Err(IdentityErrorV1::DuplicateDigest(digest(1)))
        );
        assert_eq!(
            RendererEntityAliasTableV1::assign(vec![[0; 32]; MAX_RENDERER_ENTITIES_V1 + 1]),
            Err(IdentityErrorV1::TooManyEntities {
                count: MAX_RENDERER_ENTITIES_V1 + 1,
                cap: MAX_RENDERER_ENTITIES_V1
            })
        );
    }

    #[test]
    fn compact_alias_never_replaces_full_digest_authority() {
        let left = [7; 32];
        let mut right = [7; 32];
        right[31] = 8;
        let table = RendererEntityAliasTableV1::assign(vec![right, left]).unwrap();
        assert_ne!(left, right);
        assert_ne!(
            table.alias_for_digest(&left),
            table.alias_for_digest(&right)
        );
        assert_eq!(
            table.digest_for_alias(table.alias_for_digest(&left).unwrap().get()),
            Ok(left)
        );
    }
}
