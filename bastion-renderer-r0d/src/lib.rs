//! Renderer-owned, simulation-independent source admission primitives.

use sha2::{Digest, Sha256};

mod admission;
pub mod agreement;
pub mod atlas;
pub mod bootstrap;
pub mod camera;
pub mod capture;
pub mod cbor;
pub mod cosmetic_rng;
pub mod cutaway;
pub mod draw_submission;
pub mod environment;
pub mod extract;
pub mod figure_asset;
pub mod figure_batch;
pub mod figure_gpu;
pub mod figure_package;
pub mod fog;
pub mod gpu_cull;
pub mod group_representation;
pub mod identity;
pub mod individual_tier;
pub mod interior;
pub mod island;
pub mod lens;
pub mod lighting;
pub mod material;
pub mod parallel;
pub mod pass_graph;
pub mod presentation;
pub mod publication;
pub mod r2_admission;
pub mod readiness;
pub mod replay;
pub mod selection;
pub mod shadow;
pub mod shared_adapter;
pub mod shutdown;
pub mod tape;
pub mod terrain_distance;
pub mod texture_payload;
pub mod visual_oracle;
pub mod weather;

pub use admission::{
    AdmissionErrorV1, MAX_CORPUS_INPUT_BYTES_V1, MAX_CORPUS_INPUTS_V1,
    RENDERER_ADMISSION_V1_VERSION, RENDERER_SOURCE_EPOCH_V1_VERSION, RendererAdmissionV1,
    RendererCorpusInputV1, RendererCorpusRoleV1, RendererSourceEpochV1,
};

pub const MAX_HASH_DOMAIN_BYTES_V1: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainHashErrorV1 {
    InvalidDomain,
    PayloadLengthOutOfRange,
}

pub fn domain_hash_v1(
    domain: &str,
    schema_major: u16,
    schema_minor: u16,
    payload: &[u8],
) -> Result<[u8; 32], DomainHashErrorV1> {
    if domain.is_empty() || domain.len() > MAX_HASH_DOMAIN_BYTES_V1 || !domain.is_ascii() {
        return Err(DomainHashErrorV1::InvalidDomain);
    }
    let domain_len = u16::try_from(domain.len()).map_err(|_| DomainHashErrorV1::InvalidDomain)?;
    let payload_len =
        u64::try_from(payload.len()).map_err(|_| DomainHashErrorV1::PayloadLengthOutOfRange)?;

    let mut hasher = Sha256::new();
    hasher.update(domain_len.to_le_bytes());
    hasher.update(domain.as_bytes());
    hasher.update(schema_major.to_le_bytes());
    hasher.update(schema_minor.to_le_bytes());
    hasher.update(payload_len.to_le_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

/// Fixed internal R0D domain labels use the same checked protocol as every
/// public caller.  The `Result` is deliberately retained at port boundaries:
/// a future non-literal domain cannot become an unchecked fallback hash.
pub(crate) fn domain_hash(
    domain: &'static str,
    schema_major: u16,
    schema_minor: u16,
    payload: &[u8],
) -> [u8; 32] {
    // Crate-private fixed protocol labels only. Public/untrusted paths use
    // the fallible `domain_hash_v1` API above.
    match domain_hash_v1(domain, schema_major, schema_minor, payload) {
        Ok(digest) => digest,
        Err(_) => unreachable!("fixed renderer protocol label violates V1 bounds"),
    }
}

#[cfg(test)]
pub(crate) fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for &byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
pub(crate) fn hex32(bytes: &[u8; 32]) -> String { hex_bytes(bytes) }

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT_A: &str = "f7b30de6d916930c96f181919160ff7839aa6d5b";
    const COMMIT_B: &str = "07b30de6d916930c96f181919160ff7839aa6d5b";

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    #[test]
    fn frozen_domain_hash_vector_and_bounds() {
        assert_eq!(
            hex_bytes(&domain_hash_v1("bastion/r0d/test", 1, 0, b"abc").unwrap()),
            "91b5eab66fecc7f95e62afec1a4fc9674c5d2a06eddcd940c050682aea944b0d"
        );
        assert_eq!(
            domain_hash_v1("", 1, 0, b"abc"),
            Err(DomainHashErrorV1::InvalidDomain)
        );
        assert_eq!(
            domain_hash_v1("bastion/r0d/é", 1, 0, b"abc"),
            Err(DomainHashErrorV1::InvalidDomain)
        );
    }

    fn epoch(commit: &str) -> RendererSourceEpochV1 {
        RendererSourceEpochV1::from_hex(
            commit,
            digest(1),
            digest(2),
            digest(3),
            digest(4),
            digest(5),
        )
        .unwrap()
    }

    fn input(role: RendererCorpusRoleV1, byte: u8) -> RendererCorpusInputV1 {
        RendererCorpusInputV1::from_digest_slice(role, &digest(byte), 100 + u64::from(byte))
            .unwrap()
    }

    fn admission() -> RendererAdmissionV1 {
        RendererAdmissionV1::new(epoch(COMMIT_A), vec![
            input(RendererCorpusRoleV1::LivingWorldRedesign, 7),
            input(RendererCorpusRoleV1::CanonicalRendererCorpus, 6),
        ])
        .unwrap()
    }

    #[test]
    fn canonical_round_trip_orders_roles_and_is_byte_stable() {
        let value = admission();
        assert_eq!(
            RendererCorpusRoleV1::CanonicalRendererCorpus.stable_tag(),
            0
        );
        assert_eq!(
            RendererCorpusRoleV1::LivingWorldRedesign.stable_name(),
            "LIVING_WORLD_REDESIGN"
        );
        assert_eq!(
            value.corpus_inputs()[0].role(),
            RendererCorpusRoleV1::CanonicalRendererCorpus
        );
        assert_eq!(
            value.corpus_inputs()[1].role(),
            RendererCorpusRoleV1::LivingWorldRedesign
        );

        let first = value.canonical_bytes().unwrap();
        let decoded = RendererAdmissionV1::decode_exact(&first).unwrap();
        let second = decoded.canonical_bytes().unwrap();
        assert_eq!(value, decoded);
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_wrong_or_unknown_versions() {
        let mut admission_version = admission().canonical_bytes().unwrap();
        admission_version[8..10].copy_from_slice(&2_u16.to_le_bytes());
        assert_eq!(
            RendererAdmissionV1::decode_exact(&admission_version),
            Err(AdmissionErrorV1::UnsupportedAdmissionVersion(2))
        );

        let mut epoch_version = admission().canonical_bytes().unwrap();
        epoch_version[10..12].copy_from_slice(&2_u16.to_le_bytes());
        assert_eq!(
            RendererAdmissionV1::decode_exact(&epoch_version),
            Err(AdmissionErrorV1::UnsupportedSourceEpochVersion(2))
        );
    }

    #[test]
    fn rejects_malformed_commit_and_digest_lengths() {
        assert_eq!(
            RendererSourceEpochV1::from_hex(
                "f7b3",
                digest(1),
                digest(2),
                digest(3),
                digest(4),
                digest(5),
            ),
            Err(AdmissionErrorV1::InvalidSourceCommitLength(4))
        );
        assert_eq!(
            RendererSourceEpochV1::from_hex(
                "g7b30de6d916930c96f181919160ff7839aa6d5b",
                digest(1),
                digest(2),
                digest(3),
                digest(4),
                digest(5),
            ),
            Err(AdmissionErrorV1::InvalidHex)
        );
        assert_eq!(
            RendererCorpusInputV1::from_digest_slice(
                RendererCorpusRoleV1::CanonicalRendererCorpus,
                &[0; 31],
                1,
            ),
            Err(AdmissionErrorV1::InvalidDigestLength(31))
        );
    }

    #[test]
    fn non_ascii_commit_hex_returns_typed_error_without_unwinding() {
        let malformed = format!("aé{}", "b".repeat(37));
        assert_eq!(malformed.len(), 40);

        let outcome = std::panic::catch_unwind(|| {
            RendererSourceEpochV1::from_hex(
                &malformed,
                digest(1),
                digest(2),
                digest(3),
                digest(4),
                digest(5),
            )
        });
        match outcome {
            Ok(result) => assert_eq!(result, Err(AdmissionErrorV1::InvalidHex)),
            Err(_) => panic!("non-ASCII commit input unwound instead of returning InvalidHex"),
        }
    }

    #[test]
    fn rejects_duplicate_and_missing_required_roles() {
        assert_eq!(
            RendererAdmissionV1::new(epoch(COMMIT_A), vec![
                input(RendererCorpusRoleV1::CanonicalRendererCorpus, 6),
                input(RendererCorpusRoleV1::CanonicalRendererCorpus, 7),
            ],),
            Err(AdmissionErrorV1::DuplicateRole(
                RendererCorpusRoleV1::CanonicalRendererCorpus
            ))
        );
        assert_eq!(
            RendererAdmissionV1::new(epoch(COMMIT_A), vec![input(
                RendererCorpusRoleV1::CanonicalRendererCorpus,
                6
            )],),
            Err(AdmissionErrorV1::MissingRequiredRole(
                RendererCorpusRoleV1::LivingWorldRedesign
            ))
        );
    }

    #[test]
    fn rejects_unknown_role_and_noncanonical_serialized_order() {
        let bytes = admission().canonical_bytes().unwrap();
        let first_role_offset = RendererAdmissionV1::FIXED_HEADER_BYTES;

        let mut unknown = bytes.clone();
        unknown[first_role_offset..first_role_offset + 2].copy_from_slice(&99_u16.to_le_bytes());
        assert_eq!(
            RendererAdmissionV1::decode_exact(&unknown),
            Err(AdmissionErrorV1::UnknownRole(99))
        );

        let mut duplicate = bytes.clone();
        let width = RendererAdmissionV1::CORPUS_ENTRY_BYTES;
        let second_role_offset = first_role_offset + width;
        duplicate[second_role_offset..second_role_offset + 2].copy_from_slice(
            &RendererCorpusRoleV1::CanonicalRendererCorpus
                .stable_tag()
                .to_le_bytes(),
        );
        assert_eq!(
            RendererAdmissionV1::decode_exact(&duplicate),
            Err(AdmissionErrorV1::DuplicateRole(
                RendererCorpusRoleV1::CanonicalRendererCorpus
            ))
        );

        let mut reversed = bytes;
        let first = reversed[first_role_offset..first_role_offset + width].to_vec();
        let second = reversed[second_role_offset..second_role_offset + width].to_vec();
        reversed[first_role_offset..first_role_offset + width].copy_from_slice(&second);
        reversed[second_role_offset..second_role_offset + width].copy_from_slice(&first);
        assert_eq!(
            RendererAdmissionV1::decode_exact(&reversed),
            Err(AdmissionErrorV1::NonCanonicalRoleOrder)
        );
    }

    #[test]
    fn rejects_stale_source_epoch() {
        assert_eq!(
            admission().validate_against(&epoch(COMMIT_B)),
            Err(AdmissionErrorV1::SourceEpochMismatch)
        );
    }

    #[test]
    fn rejects_wrong_authoritative_corpus_input() {
        let expected = RendererAdmissionV1::new(epoch(COMMIT_A), vec![
            input(RendererCorpusRoleV1::CanonicalRendererCorpus, 42),
            input(RendererCorpusRoleV1::LivingWorldRedesign, 7),
        ])
        .unwrap();
        assert_eq!(
            admission().validate_authority(&expected),
            Err(AdmissionErrorV1::CorpusInputMismatch(
                RendererCorpusRoleV1::CanonicalRendererCorpus
            ))
        );
    }

    #[test]
    fn rejects_invalid_bounds() {
        assert_eq!(
            RendererCorpusInputV1::from_digest_slice(
                RendererCorpusRoleV1::CanonicalRendererCorpus,
                &digest(6),
                0,
            ),
            Err(AdmissionErrorV1::InvalidCorpusSize(0))
        );
        assert_eq!(
            RendererCorpusInputV1::from_digest_slice(
                RendererCorpusRoleV1::CanonicalRendererCorpus,
                &digest(6),
                MAX_CORPUS_INPUT_BYTES_V1 + 1,
            ),
            Err(AdmissionErrorV1::InvalidCorpusSize(
                MAX_CORPUS_INPUT_BYTES_V1 + 1
            ))
        );

        let mut excessive = admission().canonical_bytes().unwrap();
        excessive
            [RendererAdmissionV1::INPUT_COUNT_OFFSET..RendererAdmissionV1::INPUT_COUNT_OFFSET + 2]
            .copy_from_slice(&((MAX_CORPUS_INPUTS_V1 as u16) + 1).to_le_bytes());
        assert_eq!(
            RendererAdmissionV1::decode_exact(&excessive),
            Err(AdmissionErrorV1::TooManyCorpusInputs(
                MAX_CORPUS_INPUTS_V1 + 1
            ))
        );
    }

    #[test]
    fn rejects_truncated_or_trailing_serialized_bytes() {
        let mut truncated = admission().canonical_bytes().unwrap();
        truncated.pop();
        assert_eq!(
            RendererAdmissionV1::decode_exact(&truncated),
            Err(AdmissionErrorV1::Truncated)
        );

        let mut trailing = admission().canonical_bytes().unwrap();
        trailing.push(0);
        assert_eq!(
            RendererAdmissionV1::decode_exact(&trailing),
            Err(AdmissionErrorV1::TrailingBytes(1))
        );
    }
}
