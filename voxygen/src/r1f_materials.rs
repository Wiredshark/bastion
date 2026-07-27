//! Narrow production adapter from accepted figure-package provenance to the
//! renderer-owned R1F material table.
//!
//! This adapter intentionally does not alter shader response.  It binds the
//! current package material declarations into GPU-generation authority while
//! the legacy atlas/palette path remains the pixel-producing compatibility
//! seam.

use bastion_renderer_r0d::{
    domain_hash_v1,
    figure_asset::{CompiledFigurePackageV1, MaterialBindingV1, MaterialKindV1},
    material::{
        MaterialClassV1, MaterialEntryV1, MaterialResponseV1, MaterialTableInputV1, MaterialTableV1,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaterialProductionEvidenceV1 {
    pub generation: u64,
    pub package_digest: [u8; 32],
    pub package_authority_digest: [u8; 32],
    pub table_digest: [u8; 32],
    pub shader_interface_digest: [u8; 32],
    pub entry_count: u16,
    pub legacy_fallback_count: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterialAdapterErrorV1 {
    InvalidGeneration,
    Hash,
    Core(String),
    CounterOverflow,
    MissingPrimaryMaterial,
}

pub(crate) fn compile_figure_material_table(
    generation: u64,
    package: &CompiledFigurePackageV1,
) -> Result<MaterialTableV1, MaterialAdapterErrorV1> {
    if generation == 0 {
        return Err(MaterialAdapterErrorV1::InvalidGeneration);
    }
    let entries = package
        .materials()
        .iter()
        .map(|binding| entry(package, binding))
        .collect::<Result<Vec<_>, _>>()?;
    MaterialTableV1::new(MaterialTableInputV1 {
        generation,
        package_digest: package.package_digest(),
        package_authority_digest: package.authority_digest(),
        entries,
    })
    .map_err(|error| MaterialAdapterErrorV1::Core(format!("{error:?}")))
}

pub(crate) fn record_completed_table(
    table: &MaterialTableV1,
) -> Result<MaterialProductionEvidenceV1, MaterialAdapterErrorV1> {
    let entry_count = u16::try_from(table.entries().len())
        .map_err(|_| MaterialAdapterErrorV1::CounterOverflow)?;
    let legacy_fallback_count = u16::try_from(
        table
            .entries()
            .iter()
            .filter(|entry| entry.class == MaterialClassV1::LegacyFallback)
            .count(),
    )
    .map_err(|_| MaterialAdapterErrorV1::CounterOverflow)?;
    let evidence = MaterialProductionEvidenceV1 {
        generation: table.generation(),
        package_digest: table.package_digest(),
        package_authority_digest: table.package_authority_digest(),
        table_digest: table.table_digest(),
        shader_interface_digest: table
            .shader_interface_digest()
            .map_err(|error| MaterialAdapterErrorV1::Core(format!("{error:?}")))?,
        entry_count,
        legacy_fallback_count,
    };
    Ok(evidence)
}

pub(crate) fn primary_material_slot(
    table: &MaterialTableV1,
) -> Result<u16, MaterialAdapterErrorV1> {
    table
        .entries()
        .iter()
        .find(|entry| entry.slot == 1)
        .map(|entry| entry.slot)
        .ok_or(MaterialAdapterErrorV1::MissingPrimaryMaterial)
}

fn entry(
    package: &CompiledFigurePackageV1,
    binding: &MaterialBindingV1,
) -> Result<MaterialEntryV1, MaterialAdapterErrorV1> {
    let class = match binding.kind {
        MaterialKindV1::OpaqueVoxel => MaterialClassV1::OpaqueVoxel,
        MaterialKindV1::CutoutVoxel => MaterialClassV1::CutoutVoxel,
        MaterialKindV1::EmissiveVoxel => MaterialClassV1::EmissiveVoxel,
        MaterialKindV1::MetallicVoxel => MaterialClassV1::MetallicVoxel,
    };
    let mut source = Vec::with_capacity(32 + 2 + 2 + 4 + 2);
    source.extend_from_slice(&package.authority_digest());
    source.extend_from_slice(&binding.slot.to_le_bytes());
    source.extend_from_slice(&(binding.kind as u16).to_le_bytes());
    source.extend_from_slice(&binding.base_color_rgba);
    source.extend_from_slice(&binding.flags.to_le_bytes());
    let source_identity = domain_hash_v1("bastion/r1f/material-source", 1, 0, &source)
        .map_err(|_| MaterialAdapterErrorV1::Hash)?;
    let response = MaterialResponseV1 {
        base_color_rgba: binding.base_color_rgba,
        roughness_milli: if class == MaterialClassV1::MetallicVoxel {
            300
        } else {
            800
        },
        metallic_milli: if class == MaterialClassV1::MetallicVoxel {
            1_000
        } else {
            0
        },
        emission_milli: if class == MaterialClassV1::EmissiveVoxel {
            1_000
        } else {
            0
        },
        alpha_cutoff_milli: if class == MaterialClassV1::CutoutVoxel {
            500
        } else {
            0
        },
        flags: binding.flags & 0x000f,
    };
    Ok(MaterialEntryV1 {
        slot: binding.slot,
        source_identity,
        class,
        response,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::figure_asset::{
        FigureAssetRoleV1, FigurePackageTargetV1, FigureSourceInputV1,
    };

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn package(materials: Vec<MaterialBindingV1>) -> CompiledFigurePackageV1 {
        CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Composite,
            digest(1),
            digest(2),
            materials,
            vec![FigureSourceInputV1 {
                logical_path: "fixture/body.vox".to_owned(),
                role: FigureAssetRoleV1::CoreBody,
                material_slot: 1,
                bytes: b"body".to_vec(),
                deterministic_fixture: false,
            }],
        )
        .unwrap()
    }

    fn material(slot: u16, kind: MaterialKindV1, base: u8) -> MaterialBindingV1 {
        MaterialBindingV1 {
            slot,
            kind,
            base_color_rgba: [base, base + 1, base + 2, 255],
            flags: 1,
        }
    }

    #[test]
    fn package_material_order_compiles_to_one_canonical_table() {
        let a = package(vec![
            material(2, MaterialKindV1::MetallicVoxel, 20),
            material(1, MaterialKindV1::OpaqueVoxel, 10),
        ]);
        let b = package(vec![
            material(1, MaterialKindV1::OpaqueVoxel, 10),
            material(2, MaterialKindV1::MetallicVoxel, 20),
        ]);
        let first = compile_figure_material_table(5, &a).unwrap();
        let second = compile_figure_material_table(5, &b).unwrap();
        assert_eq!(a.package_digest(), b.package_digest());
        assert_eq!(first, second);
        assert_eq!(first.entries()[0].slot, 1);
        assert_eq!(first.entries()[1].class, MaterialClassV1::MetallicVoxel);
    }

    #[test]
    fn package_provenance_and_material_changes_change_table_authority() {
        let base = package(vec![material(1, MaterialKindV1::OpaqueVoxel, 10)]);
        let changed = package(vec![material(1, MaterialKindV1::OpaqueVoxel, 11)]);
        let first = compile_figure_material_table(5, &base).unwrap();
        let second = compile_figure_material_table(5, &changed).unwrap();
        assert_ne!(first.package_digest(), second.package_digest());
        assert_ne!(first.table_digest(), second.table_digest());
        assert!(
            first
                .validate_package(changed.package_digest(), changed.authority_digest())
                .is_err()
        );
    }

    #[test]
    fn known_package_kinds_map_to_explicit_response_without_pixel_cutover() {
        let value = package(vec![
            material(1, MaterialKindV1::OpaqueVoxel, 10),
            material(2, MaterialKindV1::CutoutVoxel, 20),
            material(3, MaterialKindV1::EmissiveVoxel, 30),
            material(4, MaterialKindV1::MetallicVoxel, 40),
        ]);
        let table = compile_figure_material_table(5, &value).unwrap();
        assert_eq!(table.entries()[0].response.roughness_milli, 800);
        assert_eq!(table.entries()[1].response.alpha_cutoff_milli, 500);
        assert_eq!(table.entries()[2].response.emission_milli, 1_000);
        assert_eq!(table.entries()[3].response.metallic_milli, 1_000);
        assert_eq!(table.entries()[3].response.roughness_milli, 300);
    }

    #[test]
    fn completion_evidence_binds_generation_package_table_and_shader_layout() {
        let value = package(vec![material(1, MaterialKindV1::OpaqueVoxel, 10)]);
        let table = compile_figure_material_table(5, &value).unwrap();
        let evidence = record_completed_table(&table).unwrap();
        assert_eq!(evidence.generation, 5);
        assert_eq!(evidence.package_digest, value.package_digest());
        assert_eq!(evidence.table_digest, table.table_digest());
        assert_eq!(evidence.entry_count, 1);
        assert_eq!(evidence.legacy_fallback_count, 0);
        assert_ne!(evidence.shader_interface_digest, [0; 32]);
    }
}
