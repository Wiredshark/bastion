//! Project Bastion R0D deterministic renderer substrate.
//!
//! BUILD-007A10.0 — W0 source-authority admission. `RendererW0AdmissionV2`
//! binds the immutable source-authority facts of the clean integration base
//! (design §3.3). Validating a candidate source state against it produces a
//! distinct typed [`R0dSourceAuthorityMismatch`] on any divergence — dirty
//! file, extra path, changed base blob, branch drift, or lease collision —
//! with no best-effort continuation. Determinism: paths are UTF-8 byte sorted
//! after slash normalization, no floats, no RNG.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub mod admission;
pub mod agreement;
pub mod atlas;
pub mod capture;
pub mod bootstrap;
pub mod camera;
pub mod cbor;
pub mod cosmetic_rng;
pub mod extract;
pub mod figure_package;
pub mod identity;
pub mod parallel;
pub mod pass_graph;
pub mod publication;
pub mod readiness;
pub mod replay;
pub mod selection;
pub mod shared_adapter;
pub mod shutdown;
pub mod tape;
pub mod visual_oracle;

pub use admission::{CandidateSourceState, R0dSourceAuthorityMismatch, RendererW0AdmissionV2};
pub use bootstrap::{
    BootstrapError, PluginIdentityV1, SeedDomainDeclarationV1, SeedRegistryV1, TickContractV1,
    canonicalize_plugins, hkdf_expand, hkdf_extract, hmac_sha256,
};
pub use cbor::{
    CanonicalDecodeError, CanonicalEnvelopeV1, CborValue, ValidatedCanonicalBytesV1, int_map,
};

/// Design §4.4 length-framed, domain-separated hash. Every R0D hash frames its
/// domain label and schema so a domain/schema change can never alias.
///
/// `SHA256( u16_le(domain_len) || domain || u16_le(major) || u16_le(minor) ||
///          u64_le(payload_len) || payload )`
#[must_use]
pub fn domain_hash(domain: &str, schema_major: u16, schema_minor: u16, payload: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update((domain.len() as u16).to_le_bytes());
    h.update(domain.as_bytes());
    h.update(schema_major.to_le_bytes());
    h.update(schema_minor.to_le_bytes());
    h.update((payload.len() as u64).to_le_bytes());
    h.update(payload);
    h.finalize().into()
}

/// Lowercase-hex of a 32-byte digest (stable, no allocation surprises).
#[must_use]
pub fn hex32(d: &[u8; 32]) -> String {
    hex_bytes(d)
}

/// Lowercase-hex of an arbitrary byte slice.
#[must_use]
pub fn hex_bytes(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// Length-framed string field.
pub(crate) fn push_str(b: &mut Vec<u8>, x: &str) {
    b.extend_from_slice(&(x.len() as u64).to_le_bytes());
    b.extend_from_slice(x.as_bytes());
}

/// Length-framed, byte-sorted string vector (order can never leak into the hash).
pub(crate) fn push_sorted_vec(b: &mut Vec<u8>, v: &[String]) {
    let mut v: Vec<&String> = v.iter().collect();
    v.sort();
    b.extend_from_slice(&(v.len() as u64).to_le_bytes());
    for x in v {
        push_str(b, x);
    }
}

pub(crate) type Sorted<'a> = BTreeSet<&'a String>;
pub(crate) type BlobMap = BTreeMap<String, String>;
