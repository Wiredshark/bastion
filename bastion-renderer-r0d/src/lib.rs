//! Renderer-owned, simulation-independent source admission primitives.

mod admission;

pub use admission::{
    AdmissionErrorV1, MAX_CORPUS_INPUT_BYTES_V1, MAX_CORPUS_INPUTS_V1,
    RENDERER_ADMISSION_V1_VERSION, RENDERER_SOURCE_EPOCH_V1_VERSION, RendererAdmissionV1,
    RendererCorpusInputV1, RendererCorpusRoleV1, RendererSourceEpochV1,
};

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT_A: &str = "f7b30de6d916930c96f181919160ff7839aa6d5b";
    const COMMIT_B: &str = "07b30de6d916930c96f181919160ff7839aa6d5b";

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

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

        let mut reversed = bytes;
        let width = RendererAdmissionV1::CORPUS_ENTRY_BYTES;
        let second_role_offset = first_role_offset + width;
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
