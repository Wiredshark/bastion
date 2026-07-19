//! bastion (B-ASSET1): the `--asset-arena` server side — a throwaway flat
//! test chamber for eyes-on asset inspection (spec Part 3).
//!
//! Activated ONLY by the `BASTION_ASSET_ARENA` env var (set by voxygen's
//! `--asset-arena` flag before it spawns the singleplayer server thread —
//! deliberately not a `server::Settings` field, so nothing persists into
//! settings.ron). Vanilla boots read one env var and change nothing else.
//!
//! Controls ride the existing chat-command channel (`/bastion_arena
//! next|prev|fixture|dismiss|info`) — a test chamber, not a feature;
//! keybindings are a backlog nicety.

use crate::{Server, SpawnPoint, bastion_assets};
use specs::WorldExt;
use std::path::PathBuf;
use tracing::{info, warn};
use vek::{Vec2, Vec3};

const PAD_HALF: i32 = 44;
const PAD_CLEAR: i32 = 28;
const ARENA_SEED: u32 = 1337;

/// ECS resource holding the arena's state; present only when the arena is
/// active.
pub struct BastionArenaState {
    pub entries: Vec<bastion_assets::AssetLabEntry>,
    pub idx: usize,
    /// Pad center (world), at pad ground level.
    pub origin: Vec3<i32>,
    pub pad_z: i32,
    /// Where the fixture colonist spawns / the player spawn point (pad edge).
    pub staging: Vec3<f32>,
    /// The fixture colonist's name, once spawned.
    pub fixture: Option<String>,
    /// A goto should be (re)issued once the fixture has promoted and the
    /// placement BlockChange has applied (both take a few ticks).
    pub fixture_goto_pending: bool,
}

impl Server {
    /// Called at the tail of `Server::new`. Inert unless
    /// `BASTION_ASSET_ARENA` is set.
    pub(crate) fn bastion_arena_init_from_env(&mut self) {
        let Ok(request) = std::env::var("BASTION_ASSET_ARENA") else {
            return;
        };
        let root = std::env::var("BASTION_ASSET_LAB_DIR").unwrap_or_else(|_| "asset-lab".into());
        let catalog = bastion_assets::AssetLabCatalog::scan(&PathBuf::from(&root));
        let entries: Vec<_> = catalog
            .entries
            .iter()
            .filter(|e| {
                // World-layer, placeable entries only (creatures are a later
                // rung; figure-layer props/items place wrong at 1 vox = 1
                // block but remain viewable — keep them cyclable for scale
                // eyeballing, EXCEPT creatures).
                !matches!(e.category, bastion_assets::AssetCategory::Creature)
            })
            .cloned()
            .collect();
        if entries.is_empty() {
            warn!(root, "asset-arena: empty catalog — arena disabled");
            return;
        }
        let idx = entries.iter().position(|e| e.id == request).unwrap_or(0);

        // Pad on the flattest dry ground near the world spawn (never AT the
        // spawn — that's a town; the probe also dodges cliffs the fixed
        // clear height can't absorb). The player spawn is moved to the pad.
        let spawn = self.state.ecs().read_resource::<SpawnPoint>().0;
        let anchor = bastion_assets::pick_flat_anchor(&self.world, Vec2::new(spawn.x, spawn.y));
        let cx = anchor.x as i32;
        let cy = anchor.y as i32;
        self.bastion_force_load_area(anchor, 5);
        let (min_gz, max_gz) = match bastion_assets::survey_pad(&self.state, cx, cy, PAD_HALF) {
            Some(v) => v,
            None => {
                warn!("asset-arena: no ground across pad footprint — arena disabled");
                return;
            },
        };
        let pad_z = min_gz;
        let clear = (max_gz - min_gz + 8).clamp(PAD_CLEAR, 64);
        let writes = bastion_assets::flatten_pad(&mut self.state, cx, cy, pad_z, PAD_HALF, clear);
        info!(
            writes,
            cx, cy, pad_z, clear, "asset-arena: pad flattened (buffered)"
        );

        let origin = Vec3::new(cx, cy, pad_z);
        let staging = Vec3::new(cx as f32 + 0.5, (cy - 30) as f32 + 0.5, (pad_z + 2) as f32);
        match self.bastion_asset_place(&entries[idx].clone(), origin, false, ARENA_SEED) {
            Ok((loaded, report)) => info!(
                asset = entries[idx].id,
                blocks = report.blocks_placed,
                fidelity_ok = loaded.fidelity_ok,
                "asset-arena: initial asset placed"
            ),
            Err(e) => warn!(
                asset = entries[idx].id,
                e, "asset-arena: initial placement failed"
            ),
        }

        // Spawn the player on the pad edge, facing the asset.
        self.state.ecs_mut().write_resource::<SpawnPoint>().0 = staging;

        self.state.ecs_mut().insert(BastionArenaState {
            entries,
            idx,
            origin,
            pad_z,
            staging,
            fixture: None,
            fixture_goto_pending: false,
        });
        info!("asset-arena: ACTIVE (controls: /bastion_arena next|prev|fixture|dismiss|info)");
    }

    /// Per-tick arena upkeep (deferred fixture goto — promote + BlockChange
    /// application both lag the command by a few ticks). Near-zero cost when
    /// the arena is inactive or idle.
    pub(crate) fn bastion_arena_tick(&mut self) {
        // Take the state out to sidestep borrow juggling on &mut self.
        let Some(mut arena) = self.state.ecs_mut().remove::<BastionArenaState>() else {
            return;
        };
        if arena.fixture_goto_pending
            && let Some(name) = arena.fixture.clone()
        {
            // Target: the current asset's interior, else the pad center.
            let bounds = vek::Aabb {
                min: arena.origin - Vec3::new(24, 24, 0),
                max: arena.origin + Vec3::new(24, 24, PAD_CLEAR),
            };
            let target = bastion_assets::interior_target(&self.state, bounds, arena.pad_z)
                .unwrap_or_else(|| arena.origin.map(|e| e as f32) + Vec3::new(0.5, 0.5, 1.0));
            // bastion_goto only succeeds once the colonist has promoted.
            if self.bastion_goto(&name, target) {
                info!(name, ?target, "asset-arena: fixture walking in");
                arena.fixture_goto_pending = false;
            }
        }
        self.state.ecs_mut().insert(arena);
    }

    /// The `/bastion_arena` chat-command backend. Returns the feedback line.
    pub fn bastion_arena_command(&mut self, action: &str) -> String {
        let Some(mut arena) = self.state.ecs_mut().remove::<BastionArenaState>() else {
            return "Asset arena is not active (boot voxygen with --asset-arena).".into();
        };
        let feedback = match action {
            "next" | "prev" => {
                let delta: i64 = if action == "next" { 1 } else { -1 };
                arena.idx =
                    ((arena.idx as i64 + delta).rem_euclid(arena.entries.len() as i64)) as usize;
                bastion_assets::flatten_pad(
                    &mut self.state,
                    arena.origin.x,
                    arena.origin.y,
                    arena.pad_z,
                    PAD_HALF,
                    PAD_CLEAR,
                );
                let entry = arena.entries[arena.idx].clone();
                match self.bastion_asset_place(&entry, arena.origin, false, ARENA_SEED) {
                    Ok((loaded, report)) => format!(
                        "[{}/{}] {} — {} blocks, fidelity {}{}",
                        arena.idx + 1,
                        arena.entries.len(),
                        entry.id,
                        report.blocks_placed,
                        if loaded.fidelity_ok {
                            "OK"
                        } else {
                            "MISMATCH (see log)"
                        },
                        if report.sprite_cfgs_dropped > 0 {
                            format!(", {} sprite cfgs dropped", report.sprite_cfgs_dropped)
                        } else {
                            String::new()
                        }
                    ),
                    Err(e) => format!("{}: load failed — {e}", entry.id),
                }
            },
            "fixture" => {
                if arena.fixture.is_none() {
                    let names = self.bastion_spawn_colony(arena.staging, 1);
                    arena.fixture = names.first().cloned();
                }
                arena.fixture_goto_pending = true;
                match &arena.fixture {
                    Some(name) => {
                        format!("Fixture {name} ordered in — watch it path to the interior.")
                    },
                    None => "Fixture spawn failed (see server log).".into(),
                }
            },
            "dismiss" => {
                if let Some(name) = &arena.fixture {
                    self.bastion_goto_clear(Some(name));
                    arena.fixture_goto_pending = false;
                    format!("Fixture {name} stood down (idles on the pad).")
                } else {
                    "No fixture to dismiss.".into()
                }
            },
            _ => format!(
                "Asset arena: [{}/{}] {} at ({}, {}, {}). Actions: next prev fixture dismiss.",
                arena.idx + 1,
                arena.entries.len(),
                arena.entries[arena.idx].id,
                arena.origin.x,
                arena.origin.y,
                arena.origin.z
            ),
        };
        self.state.ecs_mut().insert(arena);
        feedback
    }
}
