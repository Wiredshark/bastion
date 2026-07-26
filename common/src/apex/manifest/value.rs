//! The restricted `ManifestValueV1` data model (`APEX-T0.2`, packet
//! sections 5.2-5.3, 7.5).
//!
//! Only unsigned/negative 64-bit integers, byte strings, ASCII machine
//! text, bool, arrays, and field maps are constructible. There is no
//! `From<f64>`, `From<HashMap<_,_>>`, or `From<usize>` anywhere in this
//! module — a forbidden CBOR kind (float, tag, null, undefined, arbitrary
//! map key) cannot enter the encoder through the safe public API.

use super::error::{ManifestCodecErrorCodeV1, ManifestErrorV1};
use super::text::MachineTextV1;

/// A field ID: the `u16` key of a canonical field map. Not a general
/// integer — deliberately has no arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FieldIdV1(u16);

impl FieldIdV1 {
    pub const fn new(id: u16) -> Self { Self(id) }

    pub const fn get(self) -> u16 { self.0 }
}

/// An enum variant discriminant, encoded the same way as a field ID but
/// kept as a distinct nominal type so a variant tag can never be silently
/// used as a struct field ID or vice versa.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VariantTagV1(u16);

impl VariantTagV1 {
    pub const fn new(tag: u16) -> Self { Self(tag) }

    pub const fn get(self) -> u16 { self.0 }
}

/// The restricted CBOR value model. See module docs.
#[derive(Clone, Debug, PartialEq)]
pub enum ManifestValueV1 {
    Unsigned(u64),
    /// Invariant: constructed only for values `< 0`; see [`ManifestValueV1::negative`].
    Negative(i64),
    Bytes(Vec<u8>),
    MachineText(MachineTextV1),
    Bool(bool),
    Array(Vec<ManifestValueV1>),
    Map(CanonicalFieldMapV1),
}

impl ManifestValueV1 {
    /// Construct a `Negative` value, enforcing the invariant that it is
    /// actually negative. Nonnegative signed inputs must use `Unsigned`.
    pub fn negative(v: i64) -> Result<Self, ManifestErrorV1> {
        if v >= 0 {
            return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::MalformedCbor)
                .detail("Negative variant constructed with a nonnegative value"));
        }
        Ok(Self::Negative(v))
    }
}

/// An ordered list of `(FieldIdV1, ManifestValueV1)` entries, guaranteed at
/// construction time to be sorted by field ID with no duplicates.
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalFieldMapV1(Vec<(FieldIdV1, ManifestValueV1)>);

impl CanonicalFieldMapV1 {
    /// Sorts `entries` by field ID and rejects duplicates. Encode-side
    /// convenience only — a decoder must never call this to "fix up"
    /// noncanonical received bytes; see [`super::decode`].
    pub fn try_from_entries(mut entries: Vec<(FieldIdV1, ManifestValueV1)>) -> Result<Self, ManifestErrorV1> {
        entries.sort_by_key(|(id, _)| id.get());
        for w in entries.windows(2) {
            if w[0].0 == w[1].0 {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::DuplicateFieldId).field(w[0].0.get()));
            }
        }
        Ok(Self(entries))
    }

    /// Build from entries already known to be in strictly increasing order
    /// (used by the decoder, which must reject rather than sort).
    pub(super) fn from_strictly_increasing(entries: Vec<(FieldIdV1, ManifestValueV1)>) -> Self { Self(entries) }

    pub fn entries(&self) -> &[(FieldIdV1, ManifestValueV1)] { &self.0 }

    pub fn into_entries(self) -> Vec<(FieldIdV1, ManifestValueV1)> { self.0 }
}

/// Whether a schema-declared array is order-significant (`Sequence`) or a
/// canonically sorted, duplicate-rejecting set (`CanonicalSet`). The codec
/// itself never guesses which one a given array is (packet section 5.6).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArraySemanticsV1 {
    Sequence,
    CanonicalSet { sort_key_name: &'static str },
}

/// Implemented by a domain DTO's schema-specific sort key when it
/// participates in a `CanonicalSet` array.
pub trait CanonicalSortKeyV1 {
    type Key: Ord;

    fn canonical_sort_key_v1(&self) -> Self::Key;
}

/// Required decode limits — there is deliberately no `Default`. Each root
/// schema owns named constants (packet section 5.7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestDecodeLimitsV1 {
    pub max_input_bytes: u64,
    pub max_depth: u16,
    pub max_nodes: u64,
    pub max_array_items: u64,
    pub max_map_entries: u64,
    pub max_machine_text_bytes: u64,
    pub max_byte_string_bytes: u64,
}

/// Struct-field extraction helper: default typed decoding rejects every
/// unconsumed field (packet section 7.5).
pub struct StructFieldsV1 {
    remaining: std::collections::VecDeque<(FieldIdV1, ManifestValueV1)>,
}

impl StructFieldsV1 {
    pub fn new(map: CanonicalFieldMapV1) -> Self { Self { remaining: map.into_entries().into() } }

    /// Takes a required field by ID. Fields must be requested in ascending
    /// ID order (matching the wire's own canonical order) — this walks the
    /// remaining deque forward rather than searching, so an out-of-order
    /// request against present-but-earlier fields correctly reports
    /// `MissingRequiredField` rather than silently reordering.
    pub fn take_required(&mut self, id: FieldIdV1) -> Result<ManifestValueV1, ManifestErrorV1> {
        self.take_optional(id)?
            .ok_or_else(|| ManifestErrorV1::new(ManifestCodecErrorCodeV1::MissingRequiredField).field(id.get()))
    }

    pub fn take_optional(&mut self, id: FieldIdV1) -> Result<Option<ManifestValueV1>, ManifestErrorV1> {
        if let Some((front_id, _)) = self.remaining.front() {
            if *front_id == id {
                return Ok(self.remaining.pop_front().map(|(_, v)| v));
            }
        }
        Ok(None)
    }

    pub fn finish_no_unknown(self) -> Result<(), ManifestErrorV1> {
        if let Some((id, _)) = self.remaining.front() {
            return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::UnknownField).field(id.get()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_rejects_nonnegative_input() {
        assert!(ManifestValueV1::negative(-1).is_ok());
        assert!(ManifestValueV1::negative(0).is_err());
        assert!(ManifestValueV1::negative(5).is_err());
    }

    #[test]
    fn field_map_sorts_and_rejects_duplicates() {
        let ok = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(2), ManifestValueV1::Unsigned(0)),
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(1)),
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(2)),
        ])
        .unwrap();
        let ids: Vec<u16> = ok.entries().iter().map(|(id, _)| id.get()).collect();
        assert_eq!(ids, vec![0, 1, 2]);

        let dup = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(0)),
            (FieldIdV1::new(1), ManifestValueV1::Unsigned(1)),
        ]);
        assert_eq!(dup.unwrap_err().code, ManifestCodecErrorCodeV1::DuplicateFieldId);
    }

    #[test]
    fn struct_fields_rejects_unknown() {
        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(1)),
            (FieldIdV1::new(5), ManifestValueV1::Bool(true)),
        ])
        .unwrap();
        let mut fields = StructFieldsV1::new(map);
        assert!(matches!(fields.take_required(FieldIdV1::new(0)), Ok(ManifestValueV1::Unsigned(1))));
        assert_eq!(fields.finish_no_unknown().unwrap_err().code, ManifestCodecErrorCodeV1::UnknownField);
    }

    #[test]
    fn struct_fields_missing_required() {
        let map = CanonicalFieldMapV1::try_from_entries(vec![]).unwrap();
        let mut fields = StructFieldsV1::new(map);
        assert_eq!(
            fields.take_required(FieldIdV1::new(0)).unwrap_err().code,
            ManifestCodecErrorCodeV1::MissingRequiredField
        );
    }
}
