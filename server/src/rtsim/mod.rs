pub mod event;
pub mod rule;
pub mod tick;

use atomicwrites::{AtomicFile, OverwriteBehavior};
use common::{
    grid::Grid,
    mounting::VolumePos,
    rtsim::{Actor, NpcId, RtSimEntity, TerrainResource, WorldSettings},
    terrain::{CoordinateConversions, SpriteKind},
};
use common_ecs::{System, dispatch};
use common_state::BlockDiff;
use crossbeam_channel::{Receiver, Sender, unbounded};
use enum_map::EnumMap;
use rtsim::{
    RtState,
    data::{Data, ReadError, npc::SimulationMode},
    event::{OnDeath, OnHealthChange, OnHelped, OnMountVolume, OnSetup, OnTheft},
};
use specs::DispatcherBuilder;
use std::{
    fs::{self, File},
    io,
    path::PathBuf,
    thread::{self, JoinHandle},
    time::Instant,
};
use tracing::{debug, error, info, trace, warn};
use vek::*;
use world::{IndexRef, World};

/// Translate DETRNG's boot-time flag into the server/common-state execution
/// policy. Keeping this adapter here avoids a dependency from common-state
/// back into rtsim while making the live default explicit.
pub(crate) fn execution_mode() -> common_state::ExecutionMode {
    if ::rtsim::deterministic_rtsim_enabled() {
        common_state::ExecutionMode::DeterministicSerial
    } else {
        common_state::ExecutionMode::Parallel
    }
}

pub struct RtSim {
    file_path: PathBuf,
    world_seed: u32,
    last_saved: Option<Instant>,
    state: RtState,
    save_thread: Option<(Sender<Data>, JoinHandle<()>)>,
    // `APEX-T4.6` chunk 3b: the staged multi-store epoch commit's own
    // state, separate from the pre-existing rtsim-file save machinery
    // above (which this row does not replace, only supplements).
    save_universe_layout: crate::save_universe::SaveUniverseLayoutV1,
    save_epoch_ledger: common::apex::save_universe::SaveEpochLedgerV1,
}

impl RtSim {
    pub fn new(
        settings: &WorldSettings,
        world_seed: u32,
        index: IndexRef,
        world: &World,
        data_dir: PathBuf,
        // `APEX-T4.3` chunk 2: the caller (`server/src/lib.rs`) already
        // builds `WorldMapMsg` via `World::get_map_data` for the real
        // bootstrap send -- passed in rather than re-derived here, one
        // computation, two consumers. See
        // `common_net::msg::world_msg::world_map_geometry_root_v1`.
        map_geometry_root: common::apex::digest::ArtifactIdentityV1,
        // `APEX-T4-PV`: derived by the caller from the ACTUAL WorldOpts
        // this server generated with -- same one-computation-two-consumers
        // reason as `map_geometry_root`. `None` only when the derivation
        // itself failed, which is recorded as absent rather than faked.
        worldgen_protocol_root: Option<
            common::apex::subsystem::descriptor::WorldgenProtocolVersion,
        >,
        // `APEX-T4.1-CONTENT-LIVE`: derived by the caller from a REAL,
        // once-at-boot asset-tree walk (`common::content_manifest::
        // build_from_asset_tree_v1`) -- same one-computation-two-
        // consumers reason as `map_geometry_root`/`worldgen_protocol_root`
        // (the other consumer being `bootstrap_manifest_v1`'s own Content
        // descriptor). `None` only when the walk itself failed, recorded
        // as absent rather than faked.
        content_protocol_root: Option<
            common::apex::subsystem::descriptor::ContentProtocolVersion,
        >,
    ) -> Result<Self, ron::Error> {
        // `APEX-T4.6` chunk 3a: `get_file_path` consumes `data_dir` below
        // (it may push "rtsim" onto it), so the save-universe layout
        // root -- a SIBLING of `rtsim/`, not nested under it -- is
        // derived first, borrowing rather than needing its own clone.
        let save_universe_layout = crate::save_universe::SaveUniverseLayoutV1::new(data_dir.join("save_universe"));
        let file_path = Self::get_file_path(data_dir);

        info!("Looking for rtsim data at {}...", file_path.display());
        let mut data = 'load: {
            if std::env::var("RTSIM_NOLOAD").map_or(true, |v| v != "1") {
                match File::open(&file_path) {
                    Ok(file) => {
                        info!("Rtsim data found. Attempting to load...");

                        let ignore_version = std::env::var("RTSIM_IGNORE_VERSION").is_ok();
                        // `APEX-T4.5-FIXTURES`: the exact decision, extracted
                        // so the offline-recovery proof calls the real
                        // function rather than a duplicate of this guard.
                        let load_unmigrated = matches!(
                            crate::save_migration::rtsim_version_mismatch_disposition_v1(ignore_version),
                            crate::save_migration::RtsimVersionMismatchDispositionV1::LoadUnmigrated
                        );

                        match Data::from_reader(io::BufReader::new(file)) {
                            Err(ReadError::VersionMismatch(_)) if !load_unmigrated => {
                                warn!(
                                    "Rtsim data version mismatch (implying a breaking change), \
                                     rtsim data will be purged"
                                );
                            },
                            Ok(data) | Err(ReadError::VersionMismatch(data)) => {
                                info!("Rtsim data loaded.");
                                if data.should_purge {
                                    warn!(
                                        "The should_purge flag was set on the rtsim data, \
                                         generating afresh"
                                    );
                                } else {
                                    break 'load *data;
                                }
                            },
                            Err(ReadError::Load(err)) => {
                                error!("Rtsim data failed to load: {}", err);
                                info!("Old rtsim data will now be moved to a backup file");
                                let mut i = 0;
                                loop {
                                    let mut backup_path = file_path.clone();
                                    backup_path.set_extension(if i == 0 {
                                        "ron_backup".to_string()
                                    } else {
                                        format!("ron_backup_{}", i)
                                    });
                                    if !backup_path.exists() {
                                        fs::rename(&file_path, &backup_path)?;
                                        warn!(
                                            "Failed rtsim data was moved to {}",
                                            backup_path.display()
                                        );
                                        info!("A fresh rtsim data will now be generated.");
                                        break;
                                    } else {
                                        info!(
                                            "Backup file {} already exists, trying another name...",
                                            backup_path.display()
                                        );
                                    }
                                    i += 1;
                                }
                            },
                        }
                    },
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        info!("No rtsim data found. Generating from world...")
                    },
                    Err(e) => return Err(e.into()),
                }
            } else {
                warn!(
                    "'RTSIM_NOLOAD' is set, skipping loading of rtsim state (old state will be \
                     overwritten)."
                );
            }

            let data = Data::generate(settings, world, index);
            info!("Rtsim data generated.");
            data
        };

        // `APEX-T4.3` chunk 2: verify the freshly-generated world's
        // baseline against whatever this rtsim data was last checked
        // against, BEFORE wiring it into live simulation (`RtState::new`
        // below is the reconciliation-commit point this row's own
        // architecture note names -- world generation is already
        // complete by this line, `world`/`index` are the finished
        // product). Trailing expression: `APEX-T4.6` chunk 3b's own
        // seeded (or fresh) epoch ledger, so `save_epoch_ledger_seed`
        // (computed inside this block, from the SAME `recover_v1` call
        // the baseline check already needs) doesn't need to escape it
        // via a second mutable local.
        let save_epoch_ledger = {
            let baseline_input = common::apex::world_baseline::WorldBaselineInputV1 {
                world_seed,
                // `T4-PV`: the worldgen slot is DERIVED, from the frozen
                // vocabulary this row's survey settled (see
                // `world::apex_worldgen_vocabulary`).
                // `APEX-T4.1-CONTENT-LIVE`: `content` is now DERIVED too,
                // from the caller's real, once-at-boot asset-tree walk.
                // `numeric` stays undescribed rather than fabricated --
                // its own premise-check (same row) found no compile-time
                // toolchain/codegen introspection exists yet to derive it
                // honestly (a `build.rs`-class addition, not this row's
                // "one incision"); recorded as absent, not faked.
                worldgen: worldgen_protocol_root,
                content: content_protocol_root,
                numeric: None,
                map_geometry_root: map_geometry_root.digest.bytes,
                sites: world.civs().baseline_site_graph_v1(),
                economy_root: index.world_economy_root_v1().digest.bytes,
            };
            let fresh_root = common::apex::world_baseline::compute_world_baseline_root_v1(&baseline_input)
                .expect("a locally-constructed baseline input always encodes under the domain's own limit");
            let fresh_root_bytes: [u8; 32] = *fresh_root.bytes.as_array();

            // `APEX-T4.6` chunk 3a: subsumption, read side. Once a
            // durable save-universe manifest has been published, IT is
            // the real reader per the orchestrator's own ruling ("never
            // remove the old path before the new one is the actual
            // reader") -- `data.world_baseline_root` stays the fallback
            // for the `EpochZero`/pre-adoption case (no manifest exists
            // yet) and keeps being written below either way, so an old
            // save is never worse off. A recovery ERROR (corrupt
            // manifest/pointer) is logged and treated the same as
            // `EpochZero` here: this comparison is advisory, not
            // authoritative for anything else in this chunk, so it must
            // not block startup over a manifest-layer read failure.
            //
            // chunk 3b also needs this SAME recovery result to seed the
            // in-process epoch ledger below -- one call, two consumers,
            // rather than recovering twice.
            let (recovered_world_baseline_root, save_epoch_ledger_seed): (
                Option<[u8; 32]>,
                Option<(common::apex::identity::SaveEpoch, common::apex::digest::ArtifactDigestV1, Option<common::apex::identity::UniverseBranchId>)>,
            ) = match crate::save_universe::recover_v1(&save_universe_layout) {
                Ok(crate::save_universe::SaveUniverseRecoveryV1::Recovered { manifest, manifest_identity }) => (
                    manifest.world_baseline_root.map(|d| *d.bytes.as_array()),
                    Some((manifest.lineage.epoch, manifest_identity.digest, manifest.lineage.branch)),
                ),
                Ok(crate::save_universe::SaveUniverseRecoveryV1::EpochZero) => (None, None),
                Err(e) => {
                    error!(
                        ?e,
                        "failed to recover save-universe manifest (falling back to data.world_baseline_root, starting a fresh epoch ledger)"
                    );
                    (None, None)
                },
            };
            let world_baseline_root_source = recovered_world_baseline_root.or(data.world_baseline_root);

            if let Some(stored_root_bytes) = world_baseline_root_source
                && stored_root_bytes != fresh_root_bytes
            {
                // `RESOLUTION_LAW_V1` ("loss is recorded"): write the
                // sidecar BEFORE any purge below, so the fact of the
                // mismatch survives even though `data.dat` itself is
                // about to be overwritten with fresh data. Best-effort:
                // a write failure here must not block startup, since the
                // mismatch disposition itself (purge/ignore) still has
                // to happen either way.
                #[derive(serde::Serialize)]
                struct WorldBaselineMismatchRecordV1 {
                    stored_root: Vec<u8>,
                    observed_root: Vec<u8>,
                    detected_at_unix_seconds: u64,
                }
                let record = WorldBaselineMismatchRecordV1 {
                    stored_root: stored_root_bytes.to_vec(),
                    observed_root: fresh_root_bytes.to_vec(),
                    detected_at_unix_seconds: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                };
                let sidecar_path = file_path.with_file_name("world_baseline_mismatch.json");
                match serde_json::to_vec_pretty(&record) {
                    Ok(bytes) => {
                        if let Err(e) = fs::write(&sidecar_path, bytes) {
                            error!(?e, "failed to write world-baseline-mismatch sidecar (proceeding anyway)");
                        }
                    },
                    Err(e) => error!(?e, "failed to encode world-baseline-mismatch sidecar (proceeding anyway)"),
                }

                let ignore_baseline = std::env::var("RTSIM_IGNORE_WORLD_BASELINE").is_ok();
                if ignore_baseline {
                    warn!(
                        "Rtsim data's recorded world baseline does not match this world \
                         (RTSIM_IGNORE_WORLD_BASELINE set, loading unmigrated -- the ExplicitRecoveryOnly path)"
                    );
                } else {
                    warn!(
                        "Rtsim data's recorded world baseline does not match this world \
                         (worldgen/content/economy changed since this save was written); \
                         rtsim data will be purged and regenerated"
                    );
                    data = Data::generate(settings, world, index);
                }
            }

            // Stamp the current baseline as the new floor -- covers both
            // the first-ever check (`None`) and every check that agreed.
            data.world_baseline_root = Some(fresh_root_bytes);

            match save_epoch_ledger_seed {
                Some((epoch, root, branch)) => common::apex::save_universe::SaveEpochLedgerV1::seeded_from_recovery_v1(epoch, root, branch),
                None => common::apex::save_universe::SaveEpochLedgerV1::new(),
            }
        };

        let mut this = Self {
            last_saved: None,
            world_seed,
            state: RtState::new(data).with_resource(ChunkStates(Grid::populate_from(
                world.sim().get_size().as_(),
                |_| None,
            ))),
            file_path,
            save_thread: None,
            save_universe_layout,
            save_epoch_ledger,
        };

        rule::start_rules(&mut this.state);

        this.state.emit(OnSetup, &mut (), world, index);

        Ok(this)
    }

    fn get_file_path(mut data_dir: PathBuf) -> PathBuf {
        let mut path = std::env::var("VELOREN_RTSIM")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                data_dir.push("rtsim");
                data_dir
            });
        path.push("data.dat");
        path
    }

    pub fn hook_character_mount_volume(
        &mut self,
        world: &World,
        index: IndexRef,
        pos: VolumePos<NpcId>,
        actor: Actor,
    ) {
        self.state
            .emit(OnMountVolume { actor, pos }, &mut (), world, index)
    }

    pub fn hook_pickup_owned_sprite(
        &mut self,
        world: &World,
        index: IndexRef,
        sprite: SpriteKind,
        wpos: Vec3<i32>,
        actor: Actor,
    ) {
        let site = world.sim().get(wpos.xy().wpos_to_cpos()).and_then(|chunk| {
            chunk
                .sites
                .iter()
                .find_map(|site| self.state.data().sites.world_site_map.get(site).copied())
        });

        self.state.emit(
            OnTheft {
                actor,
                wpos,
                sprite,
                site,
            },
            &mut (),
            world,
            index,
        )
    }

    /// T0.49 (master build order; T0-003): allocate the next persistent
    /// item-instance identity from the world-save allocator — called only
    /// at the authoritative creation commit (`create_item_drop`).
    pub fn allocate_item_instance_id(&mut self) -> common::comp::item::ItemInstanceId {
        self.state.get_data_mut().item_instance_allocator.allocate()
    }

    pub fn hook_load_chunk(
        &mut self,
        key: Vec2<i32>,
        max_res: EnumMap<TerrainResource, usize>,
        world: &World,
    ) {
        if let Some(chunk_state) = self.state.get_resource_mut::<ChunkStates>().0.get_mut(key) {
            *chunk_state = Some(LoadedChunkState { max_res });
        }

        if let Some(chunk) = world.sim().get(key) {
            let data = self.state.get_data_mut();
            for site in chunk.sites.iter() {
                let Some(site) = data.sites.world_site_map.get(site) else {
                    continue;
                };

                let site = *site;
                let Some(site) = data.sites.get_mut(site) else {
                    continue;
                };

                site.count_loaded_chunks += 1;
            }
        }
    }

    pub fn hook_unload_chunk(&mut self, key: Vec2<i32>, world: &World) {
        if let Some(chunk_state) = self.state.get_resource_mut::<ChunkStates>().0.get_mut(key) {
            *chunk_state = None;
        }

        if let Some(chunk) = world.sim().get(key) {
            let data = self.state.get_data_mut();
            for site in chunk.sites.iter() {
                let Some(site) = data.sites.world_site_map.get(site) else {
                    continue;
                };

                let site = *site;
                let Some(site) = data.sites.get_mut(site) else {
                    continue;
                };

                site.count_loaded_chunks = site.count_loaded_chunks.saturating_sub(1);
            }
        }
    }

    // Note that this hook only needs to be invoked if the block change results in a
    // change to the rtsim resource produced by [`Block::get_rtsim_resource`].
    pub fn hook_block_update(&mut self, world: &World, index: IndexRef, changes: Vec<BlockDiff>) {
        self.state
            .emit(event::OnBlockChange { changes }, &mut (), world, index);
    }

    /// bastion (B3): spawn the player-colony starting band near `wpos` as
    /// ordinary rtsim NPCs carrying a colonist record — they promote/demote
    /// through the standard loaded↔simulated machinery. Returns the roster
    /// names.
    pub fn bastion_spawn_colony(&mut self, wpos: Vec3<f32>, count: u8) -> Vec<String> {
        // Live founding seeds from the current rtsim tick — each founding is a
        // unique roll (intended gameplay).
        let seed_tick = self.state.get_data_mut().tick;
        self.bastion_spawn_colony_seeded(wpos, count, seed_tick)
    }

    /// Deterministic-seed variant of [`Self::bastion_spawn_colony`]: seeds the
    /// colony-generation RNG from an EXPLICIT tick instead of the live
    /// `data.tick`. For reproducible founding in determinism captures
    /// (`BASTION_AUTOFOUND_COLONY`) — the live `data.tick` is NOT deterministic
    /// at boot in a real server (rtsim generation advances it a variable amount
    /// before the colony is founded), so a fixed `seed_tick` pins colonist
    /// identities and spawn positions across runs.
    pub fn bastion_spawn_colony_seeded(
        &mut self,
        wpos: Vec3<f32>,
        count: u8,
        seed_tick: u64,
    ) -> Vec<String> {
        use common::rtsim::{Profession, Role};
        use rand::{RngExt as _, prelude::IndexedRandom};
        use rtsim::data::npc::Npc;

        let data = self.state.get_data_mut();
        // DETRNG/ARCH-003: colony generation is simulation input, not
        // cosmetic entropy. Reuse the one rtsim RNG authority.
        let mut rng = ::rtsim::tick_rng(self.world_seed, seed_tick, 0xBA57_C010);
        // Home = nearest site, so simulated-mode AI keeps them local.
        let home = data
            .sites
            .iter()
            .min_by_key(|(_, site)| {
                site.wpos
                    .map(|e| e as i64)
                    .distance_squared(wpos.xy().map(|e| e as i64))
            })
            .map(|(id, _)| id);
        let professions = [
            Profession::Farmer,
            Profession::Hunter,
            Profession::Blacksmith,
            Profession::Chef,
        ];
        let mut names = Vec::new();
        for i in 0..count {
            let colonist = common::bastion::BastionColonist::generate(&mut rng);
            names.push(colonist.name.clone());
            let offset = Vec3::new(
                rng.random_range(-5.0..5.0),
                rng.random_range(-5.0..5.0),
                0.0,
            );
            let species = *common::comp::humanoid::ALL_SPECIES
                .choose(&mut rng)
                .expect("humanoid species catalog must not be empty");
            let body = common::comp::Body::Humanoid(common::comp::humanoid::Body::random_with(
                &mut rng, &species,
            ));
            let mut npc = Npc::new(
                rng.random(),
                wpos + offset,
                body,
                Role::Civilised(Some(professions[i as usize % professions.len()])),
            )
            .with_bastion_colonist(colonist);
            npc.home = home;
            data.npcs.create_npc(npc);
        }
        info!(?names, count, "bastion: spawned starting colony");
        names
    }

    /// bastion (B4): set a work priority on a colonist's rtsim record by
    /// name. Returns whether any record matched.
    pub fn bastion_set_work_priority(
        &mut self,
        name: &str,
        work: common::bastion::WorkType,
        priority: u8,
    ) -> bool {
        let data = self.state.get_data_mut();
        let mut found = false;
        for (_, npc) in data.npcs.npcs.iter_mut() {
            if let Some(colonist) = &mut npc.bastion_colonist
                && colonist.name == name
            {
                colonist.work_priorities.set(work, priority);
                found = true;
            }
        }
        found
    }

    /// bastion (B5.5, harness): set a colonist's skill level for a work type
    /// on the rtsim record (the ECS mirror is handled by the Server hook).
    pub fn bastion_set_colonist_skill(
        &mut self,
        name: &str,
        work: common::bastion::WorkType,
        level: u16,
    ) -> bool {
        let data = self.state.get_data_mut();
        let mut found = false;
        for (_, npc) in data.npcs.npcs.iter_mut() {
            if let Some(colonist) = &mut npc.bastion_colonist
                && colonist.name == name
            {
                colonist.skills.set_level_for(work, level);
                found = true;
            }
        }
        found
    }

    /// bastion (LOD-0, harness): force-DEMOTE a loaded colonist by flipping
    /// its rtsim mode to Simulated — the sync loop's demote arm FLUSHES the
    /// live state into the persistent record and deletes the entity; the
    /// loaded-chunk spawn machinery then RE-PROMOTES it (the chunk stays
    /// loaded), exercising the REAL unload/re-promote cycle end-to-end.
    /// Returns whether a matching loaded colonist was found.
    pub fn bastion_force_demote(&mut self, name: &str) -> bool {
        let data = self.state.get_data_mut();
        let mut found = false;
        for (_, npc) in data.npcs.npcs.iter_mut() {
            if let Some(colonist) = &npc.bastion_colonist
                && colonist.name == name
                && matches!(npc.mode, ::rtsim::data::npc::SimulationMode::Loaded)
            {
                npc.mode = ::rtsim::data::npc::SimulationMode::Simulated;
                found = true;
            }
        }
        found
    }

    /// bastion (B3): the colony roster (headless harness dump + inspectors).
    pub fn bastion_colony_roster(&self) -> Vec<common::bastion::BastionColonist> {
        self.state
            .data()
            .npcs
            .npcs
            .values()
            .filter_map(|npc| npc.bastion_colonist.clone())
            .collect()
    }

    /// bastion (FOUNDING PRESET v1, packet §4 + review §8 B6): does a
    /// colony already live in this world?
    ///
    /// TEMPORAL SHAPE: **SNAPSHOT** (PACKET-CRAFT-CHECKLIST entry 1) — the
    /// answer is "right now", never "ever". An extinct colony leaves no
    /// records, so re-founding is permitted by construction; that is the
    /// ruled behaviour, not an accident of the read.
    ///
    /// WHY THE RTSIM RECORDS AND NOTHING ELSE: they are the only part of a
    /// colony that survives a server restart. The JobBoard and its
    /// designations do NOT persist (found live restarting the celebration
    /// world: colonists came back, the zones did not), and the colony
    /// presence entity is not persistence-backed either. A boundary check
    /// reading either of those would answer "no colony here" after any
    /// restart WHILE THE FIRST COLONY'S COLONISTS ARE STILL STANDING IN
    /// THE WORLD — and would then bless exactly the second founding whose
    /// cross-country leash-march the one-colony boundary exists to make
    /// impossible. The predicate has to outlive a restart because the
    /// failure it prevents does.
    pub fn bastion_colony_exists(&self) -> bool {
        self.state
            .data()
            .npcs
            .npcs
            .values()
            .any(|npc| npc.bastion_colonist.is_some())
    }

    /// bastion (B-AG2, harness): how many rtsim NPCs carry each CONVERTED
    /// archetype's profession (herbalist, hunter, guard) — evidence the
    /// table applies to a real generated population, not just test keys.
    pub fn bastion_profession_census(&self) -> (usize, usize, usize) {
        use common::rtsim::{Profession, Role};
        let data = self.state.data();
        let mut census = (0, 0, 0);
        for (_, npc) in data.npcs.npcs.iter() {
            if let Role::Civilised(Some(p)) = &npc.role {
                match p {
                    Profession::Herbalist => census.0 += 1,
                    Profession::Hunter => census.1 += 1,
                    Profession::Guard => census.2 += 1,
                    _ => {},
                }
            }
        }
        census
    }

    /// bastion (HIST-0, harness): soak-record `n` chronicle test events at
    /// an importance band (0 = Routine, 1 = Notable, other = Legendary)
    /// through THE ONE capture entry point. Returns the last stamped seq.
    pub fn bastion_chronicle_record_test(&mut self, band: u8, n: u32) -> u64 {
        use ::rtsim::data::{ChronicleKind, Importance, Scope};
        let data = self.state.get_data_mut();
        let now = data.time_of_day;
        let importance = match band {
            0 => Importance::Routine,
            1 => Importance::Notable,
            _ => Importance::Legendary,
        };
        let mut last = 0;
        for i in 0..n {
            last = data.chronicle.record(
                now,
                ChronicleKind::Founding,
                Vec::new(),
                None,
                Some(Vec3::new(i as i32, 0, 0)),
                importance,
                Scope::Colony,
                None,
            );
        }
        last
    }

    /// bastion (HIST-0, harness): (routine, notable, legendary) live
    /// counts — the bounded-growth probe.
    pub fn bastion_chronicle_counts(&self) -> (usize, usize, usize) {
        self.state.data().chronicle.counts()
    }

    /// bastion (HIST-0, harness): the B10 boundary round-trip + the
    /// immortality sweep, in vivo. (1) An end-of-time cleanup must not
    /// touch a single Legendary entry; (2) the LIVE `Data` encodes through
    /// the exact persistence encoder (`Data::write_to`) and decodes back
    /// (`Data::from_reader`, version-checked) with the chronicle surviving
    /// BYTE-FOR-BYTE (fingerprint equality) and counts intact.
    pub fn bastion_chronicle_roundtrip(&mut self) -> bool {
        let data = self.state.get_data_mut();
        let legendary_before = data.chronicle.counts().2;
        let end_of_time = common::resources::TimeOfDay(data.time_of_day.0 + 1.0e12);
        data.chronicle.cleanup(end_of_time);
        if data.chronicle.counts().2 != legendary_before {
            return false;
        }
        let mut bytes = Vec::new();
        if data.write_to(&mut bytes).is_err() {
            return false;
        }
        let decoded = match ::rtsim::data::Data::from_reader(bytes.as_slice()) {
            Ok(d) => d,
            Err(_) => return false,
        };
        match (
            data.chronicle.fingerprint(),
            decoded.chronicle.fingerprint(),
        ) {
            (Some(a), Some(b)) => a == b && data.chronicle.counts() == decoded.chronicle.counts(),
            _ => false,
        }
    }

    pub fn hook_rtsim_entity_unload(&mut self, entity: RtSimEntity) {
        let data = self.state.get_data_mut();

        if let Some(npc) = data.npcs.get_mut(entity) {
            if matches!(npc.mode, SimulationMode::Simulated) {
                error!("Unloaded already unloaded entity");
            }
            // bastion (B3): the loaded↔simulated boundary, log-verified.
            if let Some(colonist) = &npc.bastion_colonist {
                tracing::info!(
                    name = colonist.name.as_str(),
                    "bastion: colonist demoted to SimulationMode::Simulated"
                );
            }
            npc.mode = SimulationMode::Simulated;
        }
    }

    pub fn hook_rtsim_actor_hp_change(
        &mut self,
        world: &World,
        index: IndexRef,
        actor: Actor,
        cause: Option<Actor>,
        new_hp_fraction: f32,
        change: f32,
    ) {
        self.state.emit(
            OnHealthChange {
                actor,
                cause,
                new_health_fraction: new_hp_fraction,
                change,
            },
            &mut (),
            world,
            index,
        )
    }

    pub fn hook_rtsim_actor_death(
        &mut self,
        world: &World,
        index: IndexRef,
        actor: Actor,
        wpos: Option<Vec3<f32>>,
        killer: Option<Actor>,
    ) {
        self.state.emit(
            OnDeath {
                wpos,
                actor,
                killer,
            },
            &mut (),
            world,
            index,
        );
    }

    pub fn hook_rtsim_actor_helped(
        &mut self,
        world: &World,
        index: IndexRef,
        actor: Actor,
        saver: Option<Actor>,
    ) {
        self.state
            .emit(OnHelped { actor, saver }, &mut (), world, index);
    }

    // `APEX-T4.6` chunk 3b: `character_db_dir` is the character DB's own
    // directory (`DatabaseSettings::db_dir`) -- neither call site had a
    // reason to know it before this row; both are threaded now
    // (`rtsim/tick.rs`'s periodic save, `lib.rs`'s shutdown save).
    pub fn save(&mut self, wait_until_finished: bool, character_db_dir: &std::path::Path) {
        debug!("Saving rtsim data...");

        // Create the save thread if it doesn't already exist
        // We're not using the slow job pool here for two reasons:
        // 1) The thread is mostly blocked on IO, not compute
        // 2) We need to synchronise saves to ensure monotonicity, which slow jobs
        // aren't designed to allow
        let (tx, _) = self.save_thread.get_or_insert_with(|| {
            trace!("Starting rtsim data save thread...");
            let (tx, rx) = unbounded();
            let file_path = self.file_path.clone();
            (tx, thread::spawn(move || save_thread(file_path, rx)))
        });

        // Send rtsim data to the save thread
        if let Err(err) = tx.send(self.state.data().clone()) {
            error!("Failed to perform rtsim save: {}", err);
        }

        // If we need to wait until the save thread has done its work (due to, for
        // example, server shutdown) then do that.
        if wait_until_finished && let Some((tx, handle)) = self.save_thread.take() {
            drop(tx);
            info!("Waiting for rtsim save thread to finish...");
            handle.join().expect("Save thread failed to join");
            info!("Rtsim save thread finished.");
        }

        self.last_saved = Some(Instant::now());

        // `APEX-T4.6` chunk 3b: the staged multi-store epoch commit, run
        // SYNCHRONOUSLY and best-effort, strictly ADDITIVE to the
        // existing rtsim-file save above (never blocks or fails it -- a
        // staged-commit failure only ever prevents THIS epoch's commit,
        // logged loudly, not the primary save this function already
        // promised). A future perf pass can move this to its own
        // thread; correctness-first for this landing.
        self.commit_save_universe_epoch_v1(character_db_dir);
    }

    /// See [`Self::save`]'s own doc comment for why this runs where it
    /// does and why it is best-effort.
    fn commit_save_universe_epoch_v1(&mut self, character_db_dir: &std::path::Path) {
        let (frozen_tick, world_baseline_root_bytes) = {
            let data = self.state.data();
            (data.tick, data.world_baseline_root)
        };
        let mut rtsim_bytes = Vec::new();
        if let Err(e) = self.state.data().write_to(&mut rtsim_bytes) {
            error!(?e, "failed to encode rtsim payload for save-universe staging (skipping this epoch's commit)");
            return;
        }

        let world_baseline_root = world_baseline_root_bytes.map(|bytes| common::apex::digest::ArtifactDigestV1 {
            algorithm: common::apex::digest::DigestAlgorithmIdV1::Sha256,
            bytes: common::apex::digest::DigestBytes32V1::from_array(bytes),
        });

        let candidate_epoch = common::apex::identity::SaveEpoch::new(self.save_epoch_ledger.current_epoch().get() + 1);
        let lineage = common::apex::save_universe::SaveEpochLineageV1 {
            epoch: candidate_epoch,
            predecessor_root: self.save_epoch_ledger.current_root(),
            // Carry forward whatever branch the ledger already tracks
            // (`None` for a lineage never branched, unchanged behavior;
            // `Some(id)` once `APEX-T9.2` branching creates one) -- an
            // ordinary forward save must not silently drop a branch
            // identity, or the very next `admit_v1` would refuse it as a
            // `BranchMismatch`.
            branch: self.save_epoch_ledger.current_branch(),
        };

        let rtsim_payload = match crate::save_universe::stage_payload_v1(
            &self.save_universe_layout,
            candidate_epoch,
            common::apex::save_universe::SaveStoreIdV1::RtsimData,
            |f| std::io::Write::write_all(f, &rtsim_bytes),
        ) {
            Ok(p) => p,
            Err(e) => {
                error!(?e, "failed to stage rtsim payload for save-universe epoch (skipping this epoch's commit)");
                return;
            },
        };

        let character_db_payload = match crate::save_universe::stage_character_db_v1(&self.save_universe_layout, candidate_epoch, character_db_dir) {
            Ok(p) => p,
            Err(e) => {
                error!(?e, "failed to stage character-db payload for save-universe epoch (skipping this epoch's commit)");
                return;
            },
        };

        let manifest = common::apex::save_universe::SaveUniverseManifestV1 {
            lineage,
            frozen_tick,
            // Canonical store order (`SaveStoreIdV1`'s own discriminant
            // order: `CharacterDb` then `RtsimData`) -- the type's own
            // "caller supplies sorted order for reproducibility" doc note.
            stores: vec![character_db_payload, rtsim_payload],
            world_baseline_root,
            // `T4-PV` (parked, orchestrator-ruled): same undescribed-
            // rather-than-fabricated discipline as the world-baseline
            // check above -- no honest frozen-vocabulary derivation
            // exists yet for content/build/numeric/schedule identity.
            descriptors: Vec::new(),
            // `T4.5`'s confirmed-EMPTY rtsim migration graph -- nothing
            // to journal yet.
            migration_journal_digest: None,
        };

        match crate::save_universe::commit_epoch_v1(&self.save_universe_layout, &manifest) {
            Ok(pointer) => {
                if let Err(e) = self.save_epoch_ledger.admit_v1(manifest.lineage, pointer.manifest_identity.digest) {
                    error!(
                        ?e,
                        "save-universe epoch committed to disk but the in-process ledger refused to admit it -- internal inconsistency, investigate"
                    );
                }
            },
            Err(e) => error!(?e, "failed to commit save-universe epoch (rtsim/character-db payloads staged but not published)"),
        }
    }

    // TODO: Clean up this API a bit
    pub fn get_chunk_resources(&self, key: Vec2<i32>) -> EnumMap<TerrainResource, f32> {
        self.state
            .data()
            .nature
            .chunk_resources(key)
            .copied()
            .unwrap_or_default()
    }

    pub fn state(&self) -> &RtState { &self.state }

    pub fn set_should_purge(&mut self, should_purge: bool) {
        self.state.data_mut().should_purge = should_purge;
    }
}

// Crate-split seam: the bastion job system (now in the `bastion-server` leaf)
// reads rtsim state through this one-method trait instead of naming `RtSim`
// directly; `sys/mod.rs` registers `bastion_jobs::Sys<RtSim>`.
impl bastion_server::bastion_jobs::RtSimAccess for RtSim {
    fn rt_state(&self) -> &RtState { self.state() }
}

fn save_thread(file_path: PathBuf, rx: Receiver<Data>) {
    if let Some(dir) = file_path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    let atomic_file = AtomicFile::new(file_path, OverwriteBehavior::AllowOverwrite);
    while let Ok(data) = rx.recv() {
        debug!("Writing rtsim data to file...");
        match atomic_file.write(move |file| data.write_to(io::BufWriter::new(file))) {
            Ok(_) => debug!("Rtsim data saved."),
            Err(e) => error!("Saving rtsim data failed: {}", e),
        }
    }
}

pub struct ChunkStates(pub Grid<Option<LoadedChunkState>>);

pub struct LoadedChunkState {
    // The maximum possible number of each resource in this chunk
    pub max_res: EnumMap<TerrainResource, usize>,
}

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    // T0.16 (master build order; ledger #67): the jobs -> RTSim THOUGHT
    // OUTBOX edge, DECLARED — bastion_jobs pushes `pending_thoughts` that
    // this system drains; without the explicit dependency their order was
    // implicit shred staging (deterministic per build, but an undeclared
    // contract a registration shuffle could silently flip, moving thought
    // delivery by a tick).
    dispatch::<tick::Sys>(dispatch_builder, &[
        &common_systems::phys::Sys::sys_name(),
        &crate::bastion_jobs::Sys::<crate::rtsim::RtSim>::sys_name(),
    ]);
}
