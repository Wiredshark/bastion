//! T2.13 + T2.19 + T2.21 (master build order; T2 lifecycle group): the
//! versioned projection schema — the RTSim (durable) ↔ ECS (loaded)
//! materialized view declared ONCE, per field, instead of scattered ad-hoc
//! copies at promotion / mirror / demotion sites.
//!
//! Each field declares its authoritative side and which transforms carry
//! it: promote (durable → loaded at activation), mirror (loaded → durable
//! each loaded tick), demote (loaded → durable at the final deactivation
//! transaction). The schema is VERSIONED (T2.19) and validated as a
//! fitness gate (T2.21): a field whose authority and transforms are
//! inconsistent — a loaded-authoritative field that never writes back, a
//! durable field never promoted — is a data-loss bug caught here.
//!
//! Determinism story (Ben's law): a pure declaration + pure validation; no
//! runtime effect, no RNG, no wall-clock.

use serde::{Deserialize, Serialize};

/// Which side owns the authoritative value of a field.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldAuthority {
    /// The durable RTSim record owns it; the loaded entity reads a copy.
    Durable,
    /// The loaded ECS entity owns it while loaded; it must be written back.
    Loaded,
    /// Both sides write it; requires both mirror and a merge rule (declared
    /// by `on_mirror` + `on_demote`).
    Bidirectional,
}

/// One projected field's transform declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionField {
    pub name: String,
    pub authority: FieldAuthority,
    /// Copied durable → loaded at PROMOTION.
    pub on_promote: bool,
    /// Copied loaded → durable each loaded TICK (mirror).
    pub on_mirror: bool,
    /// Copied loaded → durable at the final DEMOTION transaction.
    pub on_demote: bool,
}

/// T2.19: the versioned projection schema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionSchema {
    pub version: u16,
    pub fields: Vec<ProjectionField>,
}

/// A schema-consistency violation (T2.21 fitness gate).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionViolation {
    /// A loaded-authoritative field is never written back (mirror or demote)
    /// — its loaded changes would be silently LOST on unload.
    LoadedNeverPersisted(String),
    /// A durable-authoritative field is never promoted — the loaded entity
    /// never sees the durable value.
    DurableNeverPromoted(String),
    /// A bidirectional field lacks both mirror and demote (no write-back
    /// path for its loaded edits).
    BidirectionalNoWriteback(String),
    /// Two fields share a name (ambiguous projection).
    DuplicateField(String),
}

/// T2.21: validate schema consistency — every field's authority must have a
/// coherent transform set, or its loaded/durable edits are silently lost.
pub fn validate_schema(schema: &ProjectionSchema) -> Vec<ProjectionViolation> {
    let mut violations = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for field in &schema.fields {
        if !seen.insert(field.name.clone()) {
            violations.push(ProjectionViolation::DuplicateField(field.name.clone()));
        }
        match field.authority {
            FieldAuthority::Loaded => {
                if !field.on_mirror && !field.on_demote {
                    violations
                        .push(ProjectionViolation::LoadedNeverPersisted(field.name.clone()));
                }
            },
            FieldAuthority::Durable => {
                if !field.on_promote {
                    violations
                        .push(ProjectionViolation::DurableNeverPromoted(field.name.clone()));
                }
            },
            FieldAuthority::Bidirectional => {
                if !field.on_promote {
                    violations
                        .push(ProjectionViolation::DurableNeverPromoted(field.name.clone()));
                }
                if !field.on_mirror && !field.on_demote {
                    violations.push(ProjectionViolation::BidirectionalNoWriteback(
                        field.name.clone(),
                    ));
                }
            },
        }
    }
    violations
}

#[cfg(test)]
mod t2_13_tests {
    use super::*;

    fn field(
        name: &str,
        authority: FieldAuthority,
        promote: bool,
        mirror: bool,
        demote: bool,
    ) -> ProjectionField {
        ProjectionField {
            name: name.to_string(),
            authority,
            on_promote: promote,
            on_mirror: mirror,
            on_demote: demote,
        }
    }

    #[test]
    fn t2_13_starter_colonist_schema_is_consistent() {
        // A realistic colonist projection: needs/mood owned durable and
        // promoted; position owned loaded and demote-written; orientation
        // (T2.14) mirrored.
        let schema = ProjectionSchema {
            version: 1,
            fields: vec![
                field("needs", FieldAuthority::Durable, true, false, false),
                field("position", FieldAuthority::Loaded, false, false, true),
                field("dir", FieldAuthority::Loaded, false, true, true),
                field("inventory", FieldAuthority::Bidirectional, true, false, true),
            ],
        };
        assert_eq!(validate_schema(&schema), Vec::new());
    }

    #[test]
    fn t2_21_gate_catches_data_loss_shapes() {
        // Loaded field never written back → its edits are lost.
        let bad = ProjectionSchema {
            version: 1,
            fields: vec![field("position", FieldAuthority::Loaded, true, false, false)],
        };
        assert_eq!(validate_schema(&bad), vec![
            ProjectionViolation::LoadedNeverPersisted("position".to_string()),
        ]);
        // Durable field never promoted → loaded entity never sees it.
        let bad = ProjectionSchema {
            version: 1,
            fields: vec![field("needs", FieldAuthority::Durable, false, false, false)],
        };
        assert_eq!(validate_schema(&bad), vec![
            ProjectionViolation::DurableNeverPromoted("needs".to_string()),
        ]);
        // Duplicate field.
        let bad = ProjectionSchema {
            version: 1,
            fields: vec![
                field("x", FieldAuthority::Durable, true, false, false),
                field("x", FieldAuthority::Durable, true, false, false),
            ],
        };
        assert!(bad
            .fields
            .is_empty()
            .then_some(())
            .is_none() && validate_schema(&bad).contains(&ProjectionViolation::DuplicateField(
            "x".to_string()
        )));
    }
}
