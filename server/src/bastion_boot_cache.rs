//! Opt-in, process-resident boot templates for the Bastion headless harness.
//!
//! This module never serializes a live [`crate::Server`]. It retains only the
//! immutable generated world views, pristine pre-setup RTSim data, and pristine
//! harness force-loaded chunks. Every hit still constructs a new ECS, runtime,
//! network, dispatcher and mutable RTSim state.

use crate::{CalendarMode, Settings, lod::Lod};
use common::{
    rtsim::TerrainResource,
    terrain::{TerrainChunk, TerrainGrid},
};
use common_net::msg::WorldMapMsg;
use common_state::ExecutionMode;
use enum_map::EnumMap;
use rtsim::data::Data;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};
use vek::Vec2;
use world::{IndexOwned, World};

const SCHEMA: &str = "bastion.boot-template/v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Disabled,
    Fresh,
    Restored,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Status {
    pub origin: Origin,
    pub key_sha256: Option<String>,
    pub refusal: Option<String>,
}

impl Status {
    pub fn disabled(reason: impl Into<String>) -> Self {
        Self {
            origin: Origin::Disabled,
            key_sha256: None,
            refusal: Some(reason.into()),
        }
    }

    fn fresh(key: &Request) -> Self {
        Self {
            origin: Origin::Fresh,
            key_sha256: Some(key.key_sha256.clone()),
            refusal: None,
        }
    }

    fn restored(key: &Request) -> Self {
        Self {
            origin: Origin::Restored,
            key_sha256: Some(key.key_sha256.clone()),
            refusal: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Request {
    key_sha256: String,
}

#[derive(Clone)]
pub(crate) struct Template {
    pub world: Arc<World>,
    pub index: IndexOwned,
    pub map: WorldMapMsg,
    pub lod: Lod,
    pub rtsim_data: Data,
}

struct Entry {
    key_sha256: String,
    template: Template,
    chunks: HashMap<Vec2<i32>, (Arc<TerrainChunk>, EnumMap<TerrainResource, usize>)>,
}

static CACHE: OnceLock<Mutex<Option<Entry>>> = OnceLock::new();
static OPT_IN: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn cache() -> &'static Mutex<Option<Entry>> { CACHE.get_or_init(|| Mutex::new(None)) }
fn opt_in() -> &'static Mutex<Option<String>> { OPT_IN.get_or_init(|| Mutex::new(None)) }

#[derive(Serialize)]
struct CanonicalKey<'a> {
    schema: &'static str,
    code_sha256: &'a str,
    target_arch: &'static str,
    target_os: &'static str,
    world_seed: u32,
    world: &'a common::rtsim::WorldSettings,
    map_file: &'a Option<world::sim::FileOpts>,
    calendar_mode: &'a CalendarMode,
    execution_mode: &'static str,
    deterministic_worldgen: bool,
    boot_environment: BTreeMap<String, String>,
}

fn boot_environment() -> BTreeMap<String, String> {
    std::env::vars()
        .filter(|(name, _)| {
            (name.starts_with("BASTION_") || name.starts_with("RTSIM_") || name == "VELOREN_RTSIM")
                && !name.starts_with("BASTION_FLIGHT_RECORDER_")
        })
        .collect()
}

/// Build an exact request or classify why this server must remain fresh.
pub(crate) fn request(
    settings: &Settings,
    data_dir: &Path,
    execution_mode: ExecutionMode,
) -> Result<Option<Request>, Status> {
    let code_sha256 = opt_in()
        .lock()
        .map_err(|_| Status::disabled("boot-cache opt-in lock poisoned"))?
        .clone();
    request_with_code(settings, data_dir, execution_mode, code_sha256)
}

fn valid_code_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn request_with_code(
    settings: &Settings,
    data_dir: &Path,
    execution_mode: ExecutionMode,
    code_sha256: Option<String>,
) -> Result<Option<Request>, Status> {
    let Some(code_sha256) = code_sha256 else {
        return Ok(None);
    };
    if !valid_code_sha256(&code_sha256) {
        return Err(Status::disabled("invalid executable SHA-256 opt-in"));
    }
    if !execution_mode.is_deterministic() {
        return Err(Status::disabled("parallel execution mode is not cacheable"));
    }
    if !common::deterministic_worldgen_enabled() {
        return Err(Status::disabled(
            "entropy-seeded world generation is not cacheable",
        ));
    }
    if !matches!(settings.calendar_mode, CalendarMode::None) {
        return Err(Status::disabled("only CalendarMode::None is cacheable"));
    }
    if settings.map_file.is_some() {
        return Err(Status::disabled(
            "only the bundled default map input is cacheable",
        ));
    }
    if settings.experimental_terrain_persistence {
        return Err(Status::disabled("terrain persistence is not cacheable"));
    }
    if data_dir.join("rtsim").join("data.dat").exists() {
        return Err(Status::disabled(
            "pre-existing RTSim input is not cacheable",
        ));
    }

    let key = CanonicalKey {
        schema: SCHEMA,
        code_sha256: &code_sha256,
        target_arch: std::env::consts::ARCH,
        target_os: std::env::consts::OS,
        world_seed: settings.world_seed,
        world: &settings.world,
        map_file: &settings.map_file,
        calendar_mode: &settings.calendar_mode,
        execution_mode: "deterministic_serial",
        deterministic_worldgen: true,
        boot_environment: boot_environment(),
    };
    let encoded = serde_json::to_vec(&key).expect("boot-cache key is serializable");
    let key_sha256 = hex::encode(Sha256::digest(encoded));
    Ok(Some(Request { key_sha256 }))
}

pub(crate) fn lookup(request: &Request) -> Option<(Template, Status)> {
    let guard = cache().lock().ok()?;
    let entry = guard.as_ref()?;
    (entry.key_sha256 == request.key_sha256)
        .then(|| (entry.template.clone(), Status::restored(request)))
}

pub(crate) fn publish(request: &Request, template: Template) -> Status {
    let Ok(mut guard) = cache().lock() else {
        return Status::disabled("boot-cache template lock poisoned");
    };
    *guard = Some(Entry {
        key_sha256: request.key_sha256.clone(),
        template,
        chunks: HashMap::new(),
    });
    Status::fresh(request)
}

pub(crate) fn lookup_chunk(
    request: &Request,
    key: Vec2<i32>,
) -> Option<(Arc<TerrainChunk>, EnumMap<TerrainResource, usize>)> {
    let guard = cache().lock().ok()?;
    let entry = guard.as_ref()?;
    (entry.key_sha256 == request.key_sha256)
        .then(|| entry.chunks.get(&key).cloned())
        .flatten()
}

pub(crate) fn publish_chunk(
    request: &Request,
    key: Vec2<i32>,
    chunk: Arc<TerrainChunk>,
    resources: EnumMap<TerrainResource, usize>,
) {
    if let Ok(mut guard) = cache().lock()
        && let Some(entry) = guard.as_mut()
        && entry.key_sha256 == request.key_sha256
    {
        entry.chunks.entry(key).or_insert((chunk, resources));
    }
}

/// Canonical block/resource fingerprint of the currently cached pristine
/// chunks. The harness uses this only to prove that scenario terrain
/// copy-on-write cannot mutate the template.
pub fn cached_chunks_sha256() -> Option<String> {
    let guard = cache().lock().ok()?;
    let entry = guard.as_ref()?;
    let mut keys = entry.chunks.keys().copied().collect::<Vec<_>>();
    keys.sort_by_key(|key| (key.x, key.y));
    let mut digest = Sha256::new();
    for key in keys {
        let (chunk, resources) = entry.chunks.get(&key)?;
        update_chunk_digest(&mut digest, key, chunk);
        for (_, amount) in resources.iter() {
            digest.update(amount.to_le_bytes());
        }
    }
    Some(hex::encode(digest.finalize()))
}

fn update_chunk_digest(digest: &mut Sha256, key: Vec2<i32>, chunk: &TerrainChunk) {
    digest.update(key.x.to_le_bytes());
    digest.update(key.y.to_le_bytes());
    digest.update(chunk.get_min_z().to_le_bytes());
    digest.update(chunk.get_max_z().to_le_bytes());
    for (position, block) in chunk.iter_changed() {
        digest.update(position.x.to_le_bytes());
        digest.update(position.y.to_le_bytes());
        digest.update(position.z.to_le_bytes());
        digest.update(block.to_u32().to_le_bytes());
    }
}

/// Canonical loaded-terrain block fingerprint for fresh/restored proof legs.
pub fn terrain_grid_sha256(terrain: &TerrainGrid) -> String {
    let mut chunks = terrain.iter().collect::<Vec<_>>();
    chunks.sort_by_key(|(key, _)| (key.x, key.y));
    let mut digest = Sha256::new();
    for (key, chunk) in chunks {
        update_chunk_digest(&mut digest, key, chunk);
    }
    hex::encode(digest.finalize())
}

/// Canonical persistence-format fingerprint of complete mutable RTSim data.
pub fn rtsim_data_sha256(data: &Data) -> Result<String, ron::Error> {
    rtsim_data_ron(data).map(|encoded| hex::encode(Sha256::digest(&encoded)))
}

/// Exact persistence-format bytes for first-divergence diagnosis. This is
/// evidence-only; restore continues to use the typed `Data` clone.
pub fn rtsim_data_ron(data: &Data) -> Result<Vec<u8>, ron::Error> {
    ron::ser::to_string(data).map(String::into_bytes)
}

/// Test/harness lifecycle control. This never enables caching by itself.
pub fn clear() {
    if let Ok(mut guard) = cache().lock() {
        *guard = None;
    }
}

/// Enable process-local caching for this exact executable. The harness calls
/// this before starting any runtime threads; normal server binaries never do.
pub fn enable(code_sha256: String) -> Result<(), &'static str> {
    if !valid_code_sha256(&code_sha256) {
        return Err("code SHA-256 must be 64 hexadecimal characters");
    }
    let mut guard = opt_in()
        .lock()
        .map_err(|_| "boot-cache opt-in lock poisoned")?;
    *guard = Some(code_sha256.to_ascii_lowercase());
    Ok(())
}

pub fn disable() {
    if let Ok(mut guard) = opt_in().lock() {
        *guard = None;
    }
    clear();
}

/// Canonical executable digest used by the harness opt-in.
pub fn executable_sha256(path: &Path) -> std::io::Result<String> {
    Ok(hex::encode(Sha256::digest(fs::read(path)?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn supported_settings(seed: u32) -> Settings {
        Settings {
            world_seed: seed,
            calendar_mode: CalendarMode::None,
            map_file: None,
            experimental_terrain_persistence: false,
            ..Settings::default()
        }
    }

    fn unused_data_dir(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "bastion-boot-cache-unit-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn invalid_code_digest_fails_closed() {
        assert!(!valid_code_sha256("not-a-digest"));
        assert!(valid_code_sha256(&"0".repeat(64)));
    }

    #[test]
    fn disabled_default_and_unsupported_modes_fail_closed() {
        common::enable_deterministic_worldgen();
        let data_dir = unused_data_dir("disabled");
        let settings = supported_settings(21);
        assert_eq!(
            request_with_code(
                &settings,
                &data_dir,
                ExecutionMode::DeterministicSerial,
                None,
            )
            .unwrap(),
            None
        );
        let parallel = request_with_code(
            &settings,
            &data_dir,
            ExecutionMode::Parallel,
            Some("0".repeat(64)),
        )
        .unwrap_err();
        assert_eq!(
            parallel.refusal.as_deref(),
            Some("parallel execution mode is not cacheable")
        );

        let mut calendar = settings.clone();
        calendar.calendar_mode = CalendarMode::Auto;
        let calendar = request_with_code(
            &calendar,
            &data_dir,
            ExecutionMode::DeterministicSerial,
            Some("0".repeat(64)),
        )
        .unwrap_err();
        assert_eq!(
            calendar.refusal.as_deref(),
            Some("only CalendarMode::None is cacheable")
        );
    }

    #[test]
    fn seed_and_existing_rtsim_input_change_or_refuse_the_key() {
        common::enable_deterministic_worldgen();
        let data_dir = unused_data_dir("key");
        let code = Some("a".repeat(64));
        let seed_21 = request_with_code(
            &supported_settings(21),
            &data_dir,
            ExecutionMode::DeterministicSerial,
            code.clone(),
        )
        .unwrap()
        .unwrap();
        let seed_22 = request_with_code(
            &supported_settings(22),
            &data_dir,
            ExecutionMode::DeterministicSerial,
            code.clone(),
        )
        .unwrap()
        .unwrap();
        assert_ne!(seed_21, seed_22);

        std::fs::create_dir_all(data_dir.join("rtsim")).unwrap();
        std::fs::write(data_dir.join("rtsim").join("data.dat"), b"existing").unwrap();
        let persisted = request_with_code(
            &supported_settings(21),
            &data_dir,
            ExecutionMode::DeterministicSerial,
            code,
        )
        .unwrap_err();
        assert_eq!(
            persisted.refusal.as_deref(),
            Some("pre-existing RTSim input is not cacheable")
        );
        std::fs::remove_dir_all(data_dir).unwrap();
    }
}
