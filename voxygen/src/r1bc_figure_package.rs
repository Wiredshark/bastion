//! Narrow R1BC adapter from current Voxygen `.vox` sources into the
//! renderer-owned reproducible figure package.
//!
//! This is a package/load seam only. GPU allocation and draw conversion belong
//! to later R1BC batches.

use std::{
    fs,
    path::{Path, PathBuf},
};

use bastion_renderer_r0d::{
    figure_asset::{
        CachePublicationPolicyV1, CachePublicationTerminalV1, CompiledFigurePackageV1,
        FigureAssetRoleV1, FigurePackageCacheV1, FigurePackageTargetV1, FigureSourceInputV1,
        MaterialBindingV1, MaterialKindV1, PackageReceiptV1, completion_from_package_receipts,
    },
    presentation::{
        PresentationEnvironmentV1, PresentationFrameDraftV1, PresentationGenerationV1,
        PresentationVisualPolicyV1,
    },
};
use sha2::{Digest, Sha256};

const CORE_BODY_PATH: &str = "voxygen/voxel/figure/head/dwarf/female.vox";
const EQUIPMENT_PATH: &str = "voxygen/voxel/armor/warlord/chest.vox";
const ATTACHMENT_PATH: &str = "voxygen/voxel/weapon/sword/starter.vox";
const CORPUS_PATHS: &[&str] = &[
    "voxygen/voxel/humanoid_head_manifest.ron",
    "voxygen/voxel/humanoid_armor_chest_manifest.ron",
    "voxygen/voxel/biped_weapon_manifest.ron",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealFigureSourceRecordV1 {
    pub logical_path: String,
    pub sha256: [u8; 32],
    pub bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealFigurePackageSmokeV1 {
    pub package_sha256: [u8; 32],
    pub authority_sha256: [u8; 32],
    pub asset_epoch: [u8; 32],
    pub corpus_epoch: [u8; 32],
    pub required_section_identities: Vec<[u8; 32]>,
    pub real_sources: Vec<RealFigureSourceRecordV1>,
    pub cold_terminal: CachePublicationTerminalV1,
    pub warm_terminal: CachePublicationTerminalV1,
    pub receipt_completion_generation: u64,
    pub receipt_completed_resources: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RealFigurePackageErrorV1 {
    Io { path: PathBuf, message: String },
    InvalidVox { path: String, message: String },
    Renderer(String),
    PackageMismatch,
}

pub fn compile_real_figure_package(
    assets_root: &Path,
) -> Result<(CompiledFigurePackageV1, Vec<RealFigureSourceRecordV1>), RealFigurePackageErrorV1> {
    let definitions = [
        (CORE_BODY_PATH, FigureAssetRoleV1::CoreBody, 1_u16),
        (EQUIPMENT_PATH, FigureAssetRoleV1::Equipment, 1),
        (ATTACHMENT_PATH, FigureAssetRoleV1::Attachment, 2),
    ];
    let mut records = Vec::new();
    let mut inputs = Vec::new();
    for (path, role, material_slot) in definitions {
        let bytes = read_asset(assets_root, path)?;
        dot_vox::load_bytes(&bytes).map_err(|error| RealFigurePackageErrorV1::InvalidVox {
            path: path.to_string(),
            message: error.to_string(),
        })?;
        records.push(RealFigureSourceRecordV1 {
            logical_path: path.to_string(),
            sha256: Sha256::digest(&bytes).into(),
            bytes: u64::try_from(bytes.len())
                .map_err(|error| RealFigurePackageErrorV1::Renderer(error.to_string()))?,
        });
        inputs.push(FigureSourceInputV1 {
            logical_path: path.to_string(),
            role,
            material_slot,
            bytes,
            deterministic_fixture: false,
        });
    }
    records.sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
    let asset_epoch = inventory_digest(&records)?;
    let mut corpus_records = Vec::new();
    for path in CORPUS_PATHS {
        let bytes = read_asset(assets_root, path)?;
        corpus_records.push(RealFigureSourceRecordV1 {
            logical_path: (*path).to_string(),
            sha256: Sha256::digest(&bytes).into(),
            bytes: u64::try_from(bytes.len())
                .map_err(|error| RealFigurePackageErrorV1::Renderer(error.to_string()))?,
        });
    }
    corpus_records.sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
    let corpus_epoch = inventory_digest(&corpus_records)?;

    for (logical_path, role, bytes) in [
        (
            "r1bc/fixtures/humanoid/lod-v1.bin",
            FigureAssetRoleV1::Lod,
            b"R1BC deterministic LOD fixture v1".as_slice(),
        ),
        (
            "r1bc/fixtures/humanoid/shadow-proxy-v1.bin",
            FigureAssetRoleV1::ShadowProxy,
            b"R1BC deterministic shadow proxy fixture v1".as_slice(),
        ),
        (
            "r1bc/fixtures/humanoid/impostor-v1.bin",
            FigureAssetRoleV1::Impostor,
            b"R1BC deterministic impostor fixture v1".as_slice(),
        ),
    ] {
        inputs.push(FigureSourceInputV1 {
            logical_path: logical_path.to_string(),
            role,
            material_slot: 1,
            bytes: bytes.to_vec(),
            deterministic_fixture: true,
        });
    }

    let package = CompiledFigurePackageV1::compile(
        FigurePackageTargetV1::Composite,
        asset_epoch,
        corpus_epoch,
        vec![
            MaterialBindingV1 {
                slot: 1,
                kind: MaterialKindV1::OpaqueVoxel,
                base_color_rgba: [255, 255, 255, 255],
                flags: 0,
            },
            MaterialBindingV1 {
                slot: 2,
                kind: MaterialKindV1::MetallicVoxel,
                base_color_rgba: [255, 255, 255, 255],
                flags: 1,
            },
        ],
        inputs,
    )
    .map_err(|error| RealFigurePackageErrorV1::Renderer(format!("{error:?}")))?;
    Ok((package, records))
}

pub fn run_real_figure_package_smoke(
    assets_root: &Path,
    cache_root: &Path,
) -> Result<RealFigurePackageSmokeV1, RealFigurePackageErrorV1> {
    let (package, real_sources) = compile_real_figure_package(assets_root)?;
    let cache = FigurePackageCacheV1::new(cache_root);
    let cold = cache
        .publish(&package, CachePublicationPolicyV1::Commit)
        .map_err(|error| RealFigurePackageErrorV1::Renderer(format!("{error:?}")))?;
    let warm = cache
        .publish(&package, CachePublicationPolicyV1::Commit)
        .map_err(|error| RealFigurePackageErrorV1::Renderer(format!("{error:?}")))?;
    let loaded = cache
        .load(package.authority_digest())
        .map_err(|error| RealFigurePackageErrorV1::Renderer(format!("{error:?}")))?;
    if loaded.canonical_bytes() != package.canonical_bytes() {
        return Err(RealFigurePackageErrorV1::PackageMismatch);
    }
    let frame = PresentationFrameDraftV1 {
        generation: PresentationGenerationV1 {
            run_epoch: 1,
            client_applied_generation: 1,
            simulation_tick: 0,
            coherent_snapshot_root: package.authority_digest(),
        },
        entities: Vec::new(),
        groups: Vec::new(),
        events: Vec::new(),
        environment: PresentationEnvironmentV1 {
            terrain_root: [1; 32],
            environment_digest: [2; 32],
            cloud_milli: 0,
            rain_milli: 0,
            wind_mm_s: [0, 0],
            daylight_milli: 500,
        },
        visual_policy: PresentationVisualPolicyV1 {
            policy_digest: [3; 32],
            terrain_view_distance: 1,
            entity_view_distance: 1,
            figure_lod_distance: 1,
            sprite_distance: 1,
            particles_enabled: false,
            weapon_trails_enabled: false,
            flashing_lights_enabled: false,
        },
        renderer_required_resources: vec![package.package_digest()],
        complete: true,
    }
    .seal()
    .map_err(|error| RealFigurePackageErrorV1::Renderer(format!("{error:?}")))?;
    let receipt = PackageReceiptV1::from_publication(&frame, &package, &cold)
        .map_err(|error| RealFigurePackageErrorV1::Renderer(format!("{error:?}")))?;
    let completion = completion_from_package_receipts(&frame, &[(&package, &receipt)])
        .map_err(|error| RealFigurePackageErrorV1::Renderer(format!("{error:?}")))?;
    Ok(RealFigurePackageSmokeV1 {
        package_sha256: package.package_digest(),
        authority_sha256: package.authority_digest(),
        asset_epoch: package.authority().asset_epoch,
        corpus_epoch: package.authority().corpus_epoch,
        required_section_identities: package.required_section_identities().to_vec(),
        real_sources,
        cold_terminal: cold.terminal,
        warm_terminal: warm.terminal,
        receipt_completion_generation: completion.client_applied_generation,
        receipt_completed_resources: completion.completed_resources,
    })
}

fn read_asset(assets_root: &Path, relative: &str) -> Result<Vec<u8>, RealFigurePackageErrorV1> {
    let path = assets_root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    fs::read(&path).map_err(|error| RealFigurePackageErrorV1::Io {
        path,
        message: error.to_string(),
    })
}

fn inventory_digest(
    records: &[RealFigureSourceRecordV1],
) -> Result<[u8; 32], RealFigurePackageErrorV1> {
    let mut hasher = Sha256::new();
    hasher.update(b"BSTRINV1");
    hasher.update(
        u32::try_from(records.len())
            .map_err(|error| RealFigurePackageErrorV1::Renderer(error.to_string()))?
            .to_le_bytes(),
    );
    for record in records {
        hasher.update(
            u16::try_from(record.logical_path.len())
                .map_err(|error| RealFigurePackageErrorV1::Renderer(error.to_string()))?
                .to_le_bytes(),
        );
        hasher.update(record.logical_path.as_bytes());
        hasher.update(record.sha256);
        hasher.update(record.bytes.to_le_bytes());
    }
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    static ORDINAL: AtomicUsize = AtomicUsize::new(0);

    fn hex_bytes(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }

    fn assets_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("assets")
    }

    fn cache_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "bastion-r1bc-voxygen-{label}-{}-{}",
            std::process::id(),
            ORDINAL.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn real_body_equipment_attachment_package_load_smoke() {
        let root = cache_root("smoke");
        let _ = fs::remove_dir_all(&root);
        let smoke = run_real_figure_package_smoke(&assets_root(), &root).unwrap();
        assert_eq!(smoke.real_sources.len(), 3);
        assert_eq!(smoke.cold_terminal, CachePublicationTerminalV1::Published);
        assert_eq!(
            smoke.warm_terminal,
            CachePublicationTerminalV1::ExistingIdentical
        );
        assert_eq!(smoke.required_section_identities.len(), 6);
        assert_eq!(smoke.receipt_completion_generation, 1);
        assert_eq!(smoke.receipt_completed_resources, vec![
            smoke.package_sha256
        ]);
        assert_eq!(
            hex_bytes(&smoke.package_sha256),
            "ac47dfe3102670fe535c194fd06863836afec194f2ceca5263979b056b78426d"
        );
        println!(
            "R1BC_PACKAGE package_sha256={} authority_sha256={} asset_epoch={} corpus_epoch={} \
             cold={:?} warm={:?}",
            hex_bytes(&smoke.package_sha256),
            hex_bytes(&smoke.authority_sha256),
            hex_bytes(&smoke.asset_epoch),
            hex_bytes(&smoke.corpus_epoch),
            smoke.cold_terminal,
            smoke.warm_terminal,
        );
        for source in &smoke.real_sources {
            println!(
                "R1BC_SOURCE path={} bytes={} sha256={}",
                source.logical_path,
                source.bytes,
                hex_bytes(&source.sha256),
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn real_source_mutation_changes_package_address() {
        let (package, _) = compile_real_figure_package(&assets_root()).unwrap();
        let scratch = cache_root("sensitivity");
        let copy = scratch.join("assets");
        fs::create_dir_all(copy.join("voxygen/voxel/figure/head/dwarf")).unwrap();
        fs::create_dir_all(copy.join("voxygen/voxel/armor/warlord")).unwrap();
        fs::create_dir_all(copy.join("voxygen/voxel/weapon/sword")).unwrap();
        fs::create_dir_all(copy.join("voxygen/voxel")).unwrap();
        for path in [CORE_BODY_PATH, EQUIPMENT_PATH, ATTACHMENT_PATH] {
            let source = assets_root().join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
            let target = copy.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
            fs::copy(source, target).unwrap();
        }
        for path in CORPUS_PATHS {
            let source = assets_root().join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
            let target = copy.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
            fs::copy(source, target).unwrap();
        }
        let body = copy.join(CORE_BODY_PATH.replace('/', std::path::MAIN_SEPARATOR_STR));
        let mut bytes = fs::read(&body).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(body, bytes).unwrap();
        let changed = compile_real_figure_package(&copy);
        assert!(
            changed
                .as_ref()
                .map(|(value, _)| value.package_digest() != package.package_digest())
                .unwrap_or(true),
            "source mutation must either invalidate VOX or change the package"
        );
        fs::remove_dir_all(scratch).unwrap();
    }
}
