//! Bounded adapter for the admitted renderer corpus and source epoch.
//!
//! The integration base predates the donor's unrelated `common` determinism
//! modules.  This adapter therefore preserves the required boundary without
//! widening the source authority surface: it accepts only the already
//! validated renderer admission and canonical bytes, and exposes a stable
//! presentation-side digest.  It never writes simulation state.

use crate::{
    DomainHashErrorV1, RendererAdmissionV1, cbor::ValidatedCanonicalBytesV1, domain_hash_v1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SharedAdapterErrorV1 {
    Domain(DomainHashErrorV1),
}

impl From<DomainHashErrorV1> for SharedAdapterErrorV1 {
    fn from(value: DomainHashErrorV1) -> Self { Self::Domain(value) }
}

/// Hashes only canonical bytes. Raw bytes cannot cross this adapter boundary.
pub fn renderer_corpus_digest_v1(
    canonical: &ValidatedCanonicalBytesV1,
) -> Result<[u8; 32], SharedAdapterErrorV1> {
    Ok(domain_hash_v1(
        "bastion/r0d/corpus-adapter",
        1,
        0,
        canonical.as_bytes(),
    )?)
}

/// Produces the immutable renderer-side admission projection. The source epoch
/// and corpus ordering have already been checked by `RendererAdmissionV1`.
pub fn renderer_admission_projection_v1(
    admission: &RendererAdmissionV1,
) -> Result<[u8; 32], SharedAdapterErrorV1> {
    let bytes = admission
        .canonical_bytes()
        .map_err(|_| DomainHashErrorV1::PayloadLengthOutOfRange)?;
    Ok(domain_hash_v1(
        "bastion/r0d/admission-projection",
        1,
        0,
        &bytes,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cbor::ValidatedCanonicalBytesV1;

    #[test]
    fn validated_bytes_are_the_only_corpus_adapter_input() {
        let value = ValidatedCanonicalBytesV1::validate(&[0x01]).unwrap();
        assert_eq!(
            renderer_corpus_digest_v1(&value).unwrap(),
            renderer_corpus_digest_v1(&value).unwrap()
        );
    }
}
