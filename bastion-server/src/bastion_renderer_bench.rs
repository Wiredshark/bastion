//! renderer-bench W2 (fork build, Ben-directed): the LIVE half of the bench —
//! fixture loading, scripted driving, and the semantic tape.
//!
//! W1 (`common::renderer_bench`) is the canonical contract; this module is
//! its first producer. Env-gated and runtime-inert without the gate, fully
//! deterministic with it:
//! - `BASTION_RENDERER_BENCH_MANIFEST` — path to an RBDM fixture manifest
//!   (the W1 binary format, decoded FAIL-CLOSED: a bad manifest disables the
//!   bench loudly rather than running something plausible).
//! - `BASTION_RENDERER_BENCH_OUT` — tape artifact output path (default
//!   `renderer-bench-tape.json` beside the manifest).
//! - `BASTION_RENDERER_BENCH_TICKS` — run length in ticks (default 900).
//! - `BASTION_RENDERER_BENCH_CADENCE` — frame every N ticks (default 30).
//!
//! W2 SEMANTICS AUTHORED HERE (the W0 handoff left them to this wave; each
//! is a named decision, revisable by a future wave without byte breakage):
//! - `run_id` = the manifest's DOMAIN sha256 — run identity IS manifest
//!   identity, so two runs of one fixture chain to the same root ancestry.
//! - token `script_sha256` = the manifest domain sha as well (the script
//!   lives inside the manifest in V1); `parent_frame_sha256` chains to the
//!   previous frame's ROOT ([0; 32] for frame 0).
//! - Movement step `move_*_ppm` = milli-blocks per second of commanded
//!   velocity; a step holds from its tick until the next step's tick.
//!   `Target` movement drives at unit speed toward `target_mm` and stops
//!   inside 0.25 blocks. Bench entities spawn WITHOUT an Agent — nothing
//!   nondeterministic steers them; this module is their only driver.
//! - The tape's per-frame domains in V1: `FigureIdentity` (body family,
//!   the W0 leaf shape), `FigureSourceProjection` (position mm, FIXED_I32
//!   ×3), `ServerScriptState` (current step ordinal, U32). More domains are
//!   additive later waves.
//!
//! Artifact transaction: the tape is written tmp-then-rename (atomic on the
//! same filesystem), carrying the manifest digests + every (token, frame
//! root) + the run root. The GOLDEN comparison lives harness-side; the
//! producer never blesses its own output.

use crate::Tick;
use common::{
    comp,
    event::{CreateNpcEvent, EventBus, NpcBuilder},
    renderer_bench::{
        AnimationV1, BenchBodyV1, BenchFrameAnnounceV1, Domain, FixtureManifestV1, MovementV1,
        OwnerKind, RendererBenchClientSignals, RendererBenchNetOutbox, SemanticFrameTokenV1,
        WireType, domain_root, frame_root, leaf_hash, oracle_schema_hash, owner_root, run_root,
    },
};
use hashbrown::HashMap;
use common_ecs::{Job as EcsJob, Origin, Phase, System};
use specs::{Entities, Join, ReadStorage, WriteStorage};
use std::path::PathBuf;
use tracing::{info, warn};
use vek::Vec3;

/// One bench entity's live bookkeeping.
struct BenchEntitySlot {
    semantic_id: u32,
    /// Resolved once the spawned NPC is found by its deterministic name tag.
    entity: Option<specs::Entity>,
    /// Index into the manifest's (sorted) entity list.
    manifest_index: usize,
}

/// The bench's whole run state (a `Write<Option<…>>` resource, created on
/// the first gated tick).
pub struct RendererBenchRun {
    manifest: FixtureManifestV1,
    manifest_payload_sha: [u8; 32],
    manifest_domain_sha: [u8; 32],
    schema: [u8; 32],
    slots: Vec<BenchEntitySlot>,
    spawn_emitted: bool,
    start_tick: u64,
    run_ticks: u64,
    cadence: u64,
    frames: Vec<(u64, Vec<u8>, [u8; 32])>,
    parent_frame_root: [u8; 32],
    out_path: PathBuf,
    finished: bool,
}

/// Env gate, read once. `None` = bench inert (the always-compiled,
/// runtime-inert W0 rule).
fn env_config() -> Option<(PathBuf, PathBuf, u64, u64)> {
    let manifest = std::env::var("BASTION_RENDERER_BENCH_MANIFEST").ok()?;
    let manifest = PathBuf::from(manifest);
    let out = std::env::var("BASTION_RENDERER_BENCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            manifest
                .parent()
                .map(|p| p.join("renderer-bench-tape.json"))
                .unwrap_or_else(|| PathBuf::from("renderer-bench-tape.json"))
        });
    let ticks = std::env::var("BASTION_RENDERER_BENCH_TICKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(900u64);
    let cadence = std::env::var("BASTION_RENDERER_BENCH_CADENCE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30u64)
        .max(1);
    Some((manifest, out, ticks, cadence))
}

/// Map a W1 body to a live `comp::Body`, fail-closed on out-of-range
/// indices (a refused manifest, not a clamped one).
fn build_body(b: &BenchBodyV1) -> Option<comp::Body> {
    match b {
        BenchBodyV1::Humanoid {
            species,
            body_type,
            hair_style,
            beard,
            eyes,
            accessory,
            hair_color,
            skin,
            eye_color,
            height_scale,
        } => {
            use comp::body::humanoid;
            let species = *humanoid::ALL_SPECIES.get(*species as usize)?;
            let body_type = *humanoid::ALL_BODY_TYPES.get(*body_type as usize)?;
            Some(comp::Body::Humanoid(humanoid::Body {
                species,
                body_type,
                hair_style: *hair_style,
                beard: *beard,
                eyes: *eyes,
                accessory: *accessory,
                hair_color: *hair_color,
                skin: *skin,
                eye_color: *eye_color,
                height_scale: *height_scale,
            }))
        },
        BenchBodyV1::QuadrupedSmall { species, body_type } => {
            use comp::body::quadruped_small;
            let species = *quadruped_small::ALL_SPECIES.get(*species as usize)?;
            let body_type = *quadruped_small::ALL_BODY_TYPES.get(*body_type as usize)?;
            Some(comp::Body::QuadrupedSmall(quadruped_small::Body {
                species,
                body_type,
            }))
        },
    }
}

fn mm_to_blocks(mm: [i32; 3]) -> Vec3<f32> {
    Vec3::new(mm[0] as f32, mm[1] as f32, mm[2] as f32) / 1000.0
}

/// The deterministic name tag the spawner stamps and the resolver scans for.
fn bench_name(semantic_id: u32) -> String { format!("rbench:{semantic_id}") }

/// The commanded velocity for one entity at `rel_tick` (see module docs for
/// the W2 movement semantics). Also returns the current step ordinal for
/// the ServerScriptState leaf.
fn commanded_velocity(
    movement: &MovementV1,
    rel_tick: u64,
    pos: Vec3<f32>,
    origin: Vec3<f32>,
) -> (Vec3<f32>, u32) {
    match movement {
        MovementV1::None => (Vec3::zero(), 0),
        MovementV1::Steps(steps) => {
            let mut vel = Vec3::zero();
            let mut ordinal = 0u32;
            for (i, s) in steps.iter().enumerate() {
                if s.tick <= rel_tick {
                    vel = Vec3::new(
                        s.move_x_ppm as f32 / 1000.0,
                        s.move_y_ppm as f32 / 1000.0,
                        0.0,
                    );
                    ordinal = i as u32 + 1;
                } else {
                    break;
                }
            }
            (vel, ordinal)
        },
        MovementV1::Target {
            target_mm,
            earliest_terminal_tick: _,
            latest_terminal_tick,
        } => {
            if rel_tick > *latest_terminal_tick {
                return (Vec3::zero(), 2);
            }
            let target = origin + mm_to_blocks(*target_mm);
            let delta = target - pos;
            if delta.magnitude_squared() < 0.25 * 0.25 {
                (Vec3::zero(), 2)
            } else {
                (delta.normalized(), 1)
            }
        },
    }
}

/// The bench system. Registered unconditionally; a single env check makes
/// the ungated path one `Option::is_none` test per tick.
#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        specs::Read<'a, Tick>,
        specs::Write<'a, Option<RendererBenchRun>>,
        specs::Read<'a, RendererBenchClientSignals>,
        specs::Write<'a, RendererBenchNetOutbox>,
        specs::Read<'a, EventBus<CreateNpcEvent>>,
        ReadStorage<'a, comp::Stats>,
        ReadStorage<'a, comp::Pos>,
        WriteStorage<'a, comp::Vel>,
        WriteStorage<'a, comp::bastion::RendererBenchEntityId>,
    );

    const NAME: &'static str = "bastion_renderer_bench";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut EcsJob<Self>,
        (
            entities,
            tick,
            mut run_res,
            signals,
            mut outbox,
            create_npc_bus,
            stats,
            positions,
            mut velocities,
            mut bench_ids,
        ): Self::SystemData,
    ) {
        let tick = tick.0;
        // ── Lazy init (once): read + decode the manifest, fail closed. ──
        if run_res.is_none() {
            let Some((mpath, out_path, run_ticks, cadence)) = env_config() else {
                return; // ungated: inert
            };
            // W3: with WAIT_CLIENT=1 the run does not START (init, spawn,
            // frame 0) until at least one client sent RendererBenchReady —
            // so a client leg covers the whole run. Default off: the
            // headless twin legs are byte-identical to W2.
            if std::env::var("BASTION_RENDERER_BENCH_WAIT_CLIENT").as_deref() == Ok("1")
                && signals.ready_count.load(std::sync::atomic::Ordering::Relaxed) == 0
            {
                use std::sync::atomic::{AtomicBool, Ordering};
                static WAIT_LOGGED: AtomicBool = AtomicBool::new(false);
                if !WAIT_LOGGED.swap(true, Ordering::Relaxed) {
                    info!("bastion: renderer-bench WAITING for client readiness");
                }
                return;
            }
            // A once-latched sentinel so a bad manifest logs ONCE, not per
            // tick: a poisoned run entry with `finished = true`.
            let poisoned = |reason: &str| {
                warn!(%reason, path = %mpath.display(), "bastion: renderer-bench REFUSED");
                RendererBenchRun {
                    manifest: FixtureManifestV1 {
                        scenario_id: String::new(),
                        scenario_seed: 0,
                        worldgen_seed: 0,
                        rtsim_seed: 0,
                        simulation_tps: 0,
                        arena_origin_mm: [0; 3],
                        camera_script_id: String::new(),
                        graphics_manifest_version: 0,
                        artifact_schema_version: 0,
                        entities: vec![],
                    },
                    manifest_payload_sha: [0; 32],
                    manifest_domain_sha: [0; 32],
                    schema: [0; 32],
                    slots: vec![],
                    spawn_emitted: true,
                    start_tick: tick,
                    run_ticks: 0,
                    cadence: 1,
                    frames: vec![],
                    parent_frame_root: [0; 32],
                    out_path: PathBuf::new(),
                    finished: true,
                }
            };
            let run = match std::fs::read(&mpath) {
                Err(e) => poisoned(&format!("manifest unreadable: {e}")),
                Ok(bytes) => match FixtureManifestV1::decode(&bytes) {
                    Err(e) => poisoned(&format!("manifest decode refused: {e:?}")),
                    Ok(manifest) => {
                        use sha2::{Digest, Sha256};
                        let payload: [u8; 32] = {
                            let mut h = Sha256::new();
                            h.update(&bytes);
                            h.finalize().into()
                        };
                        let domain = FixtureManifestV1::domain_sha256(&bytes);
                        let slots = manifest
                            .entities
                            .iter()
                            .enumerate()
                            .map(|(i, e)| BenchEntitySlot {
                                semantic_id: e.semantic_id,
                                entity: None,
                                manifest_index: i,
                            })
                            .collect();
                        info!(
                            scenario = %manifest.scenario_id,
                            entities = manifest.entities.len(),
                            run_ticks,
                            cadence,
                            "bastion: renderer-bench ARMED"
                        );
                        RendererBenchRun {
                            manifest,
                            manifest_payload_sha: payload,
                            manifest_domain_sha: domain,
                            schema: oracle_schema_hash(),
                            slots,
                            spawn_emitted: false,
                            start_tick: tick,
                            run_ticks,
                            cadence,
                            frames: vec![],
                            parent_frame_root: [0; 32],
                            out_path,
                            finished: false,
                        }
                    },
                },
            };
            *run_res = Some(run);
        }
        let Some(run) = run_res.as_mut() else { return };
        if run.finished {
            return;
        }
        let rel_tick = tick.saturating_sub(run.start_tick);
        let origin = mm_to_blocks(run.manifest.arena_origin_mm);

        // ── Spawn (once): emit CreateNpcEvent per manifest entity. ──
        if !run.spawn_emitted {
            let mut emitter = create_npc_bus.emitter();
            for e in &run.manifest.entities {
                let Some(body) = build_body(&e.body) else {
                    warn!(
                        semantic_id = e.semantic_id,
                        "bastion: renderer-bench body indices out of range — entity SKIPPED"
                    );
                    continue;
                };
                let name = bench_name(e.semantic_id);
                let stats = comp::Stats::new(
                    common::comp::Content::Plain(name),
                    body,
                );
                let pos = origin + mm_to_blocks(e.spawn_position_mm);
                info!(
                    semantic_id = e.semantic_id,
                    x = pos.x, y = pos.y, z = pos.z,
                    "bastion: renderer-bench SPAWN"
                );
                emitter.emit(CreateNpcEvent {
                    pos: comp::Pos(pos),
                    ori: comp::Ori::default(),
                    // No Agent: this module is the ONLY driver (determinism).
                    npc: NpcBuilder::new(stats, body, comp::Alignment::Npc),
                });
            }
            run.spawn_emitted = true;
        }

        // ── Resolve (until bound): find spawned NPCs by name tag, stamp
        // the synced semantic id. ──
        let mut unresolved: HashMap<String, usize> = HashMap::new();
        for (i, s) in run.slots.iter().enumerate() {
            if s.entity.is_none() {
                unresolved.insert(bench_name(s.semantic_id), i);
            }
        }
        if !unresolved.is_empty() {
            for (entity, stats) in (&entities, &stats).join() {
                if let common::comp::Content::Plain(name) = &stats.name {
                    if let Some(&i) = unresolved.get(name.as_str()) {
                        let sid = run.slots[i].semantic_id;
                        run.slots[i].entity = Some(entity);
                        let _ = bench_ids
                            .insert(entity, comp::bastion::RendererBenchEntityId(sid));
                    }
                }
            }
        }

        // ── Drive: commanded velocity per entity, every tick. ──
        let mut ordinals: HashMap<u32, u32> = HashMap::new();
        for s in &run.slots {
            let Some(entity) = s.entity else {
                ordinals.insert(s.semantic_id, 0);
                continue;
            };
            let pos = positions
                .get(entity)
                .map(|p| p.0)
                .unwrap_or_default();
            let ent = &run.manifest.entities[s.manifest_index];
            let (vel, ordinal) = commanded_velocity(&ent.movement, rel_tick, pos, origin);
            ordinals.insert(s.semantic_id, ordinal);
            if let Some(v) = velocities.get_mut(entity) {
                v.0.x = vel.x;
                v.0.y = vel.y;
            }
            // Animation scripts: V1 records ordinals only (actions carry no
            // live consumer yet — a later wave binds them; exhaustive match
            // deliberately NOT wildcarded away here since nothing branches).
            let _ = &ent.animation;
        }

        // ── Tape: one frame every cadence ticks (and always frame 0). ──
        if rel_tick % run.cadence == 0 {
            let frame_index = (rel_tick / run.cadence) as u32;
            let token = SemanticFrameTokenV1 {
                run_id: run.manifest_domain_sha,
                frame_index,
                // W3 revision (reserved by the W2 doc): run identity is
                // RUN-RELATIVE. An absolute boot tick would make two
                // otherwise-identical runs differ by operator timing —
                // exactly the wall-coupling the project law forbids in
                // deterministic identity.
                sim_tick: rel_tick,
                script_cursor: 0,
                readback_cursor: 0,
                manifest_sha256: run.manifest_payload_sha,
                script_sha256: run.manifest_domain_sha,
                parent_frame_sha256: run.parent_frame_root,
            }
            .encode();
            let mut fig_owners: Vec<(Vec<u8>, [u8; 32])> = vec![];
            let mut src_owners: Vec<(Vec<u8>, [u8; 32])> = vec![];
            let mut script_owners: Vec<(Vec<u8>, [u8; 32])> = vec![];
            for s in &run.slots {
                let owner_key = s.semantic_id.to_le_bytes();
                // W3: the composite shape moved to the ONE shared
                // implementation client and server both call.
                let composite =
                    |_key: &[u8]| common::renderer_bench::stable_entity_composite(s.semantic_id);
                // FigureIdentity: body family (the W0 leaf shape verbatim).
                let family: u16 = match &run.manifest.entities[s.manifest_index].body {
                    BenchBodyV1::Humanoid { .. } => 0,
                    BenchBodyV1::QuadrupedSmall { .. } => 1,
                };
                let leaf = leaf_hash(
                    &run.schema,
                    Domain::FigureIdentity,
                    0x0900_0001,
                    WireType::Enum,
                    OwnerKind::StableEntity,
                    &owner_key,
                    &family.to_le_bytes(),
                );
                fig_owners.push((
                    composite(&owner_key),
                    owner_root(
                        &run.schema,
                        OwnerKind::StableEntity,
                        &owner_key,
                        &[(0x0900_0001, leaf)],
                    ),
                ));
                // FigureSourceProjection: live position, mm, FIXED_I32 ×3.
                let pos = s
                    .entity
                    .and_then(|e| positions.get(e))
                    .map(|p| p.0)
                    .unwrap_or_default();
                let mm = (pos - origin) * 1000.0;
                let mut payload = Vec::with_capacity(12);
                for c in [mm.x, mm.y, mm.z] {
                    payload.extend_from_slice(&(c as i32).to_le_bytes());
                }
                let leaf = leaf_hash(
                    &run.schema,
                    Domain::FigureSourceProjection,
                    0x0800_0001,
                    WireType::FixedI32,
                    OwnerKind::StableEntity,
                    &owner_key,
                    &payload,
                );
                src_owners.push((
                    composite(&owner_key),
                    owner_root(
                        &run.schema,
                        OwnerKind::StableEntity,
                        &owner_key,
                        &[(0x0800_0001, leaf)],
                    ),
                ));
                // ServerScriptState: the current step ordinal.
                let ordinal = ordinals.get(&s.semantic_id).copied().unwrap_or(0);
                let leaf = leaf_hash(
                    &run.schema,
                    Domain::ServerScriptState,
                    0x0300_0001,
                    WireType::U32,
                    OwnerKind::StableEntity,
                    &owner_key,
                    &ordinal.to_le_bytes(),
                );
                script_owners.push((
                    composite(&owner_key),
                    owner_root(
                        &run.schema,
                        OwnerKind::StableEntity,
                        &owner_key,
                        &[(0x0300_0001, leaf)],
                    ),
                ));
            }
            let domains = [
                (
                    Domain::ServerScriptState,
                    domain_root(&run.schema, Domain::ServerScriptState, &script_owners),
                ),
                (
                    Domain::FigureSourceProjection,
                    domain_root(&run.schema, Domain::FigureSourceProjection, &src_owners),
                ),
                (
                    Domain::FigureIdentity,
                    domain_root(&run.schema, Domain::FigureIdentity, &fig_owners),
                ),
            ];
            let froot = frame_root(&run.schema, &token, &domains);
            run.parent_frame_root = froot;
            run.frames.push((rel_tick, token, froot));
            // W3: announce the frame to every in-game client (drained by
            // the server crate's net sys — bastion-server cannot see
            // `Client`). Ack content is observational; run_root is closed
            // over server frames only.
            outbox.announces.push(BenchFrameAnnounceV1 {
                run_id: run.manifest_domain_sha,
                frame_index,
                sim_tick: rel_tick,
                frame_root: froot,
                cadence: run.cadence as u32,
                run_ticks: run.run_ticks as u32,
                arena_origin_mm: run.manifest.arena_origin_mm,
                entity_count: run.manifest.entities.len() as u32,
            });
        }

        // ── Terminal: run root + atomic artifact write. ──
        if rel_tick >= run.run_ticks {
            let frames_ref: Vec<(Vec<u8>, [u8; 32])> = run
                .frames
                .iter()
                .map(|(_, t, f)| (t.clone(), *f))
                .collect();
            let rroot = run_root(&run.schema, &run.manifest.scenario_id, &frames_ref, 0);
            let hex = |b: &[u8]| -> String { b.iter().map(|x| format!("{x:02x}")).collect() };
            let frames_json: Vec<String> = run
                .frames
                .iter()
                .map(|(t, tok, fr)| {
                    format!(
                        "{{\"tick\":{t},\"token\":\"{}\",\"frame_root\":\"{}\"}}",
                        hex(tok),
                        hex(fr)
                    )
                })
                .collect();
            // W3 sidecar: client acks (wall-coupled observations; NEVER
            // part of run_root). echo_match verifies the announced root
            // came back verbatim for the frame index it names.
            let acks = std::mem::take(
                &mut *signals.acks.lock().expect("bench signals mutex never poisons"),
            );
            let acks_json: Vec<String> = acks
                .iter()
                .map(|a| {
                    let echo_match = run
                        .frames
                        .get(a.frame_index as usize)
                        .map(|(_, _, fr)| *fr == a.frame_root_echo)
                        .unwrap_or(false);
                    format!(
                        "{{\"frame_index\":{},\"sim_tick\":{},\"echo_match\":{},\"client_projection_root\":\"{}\",\"entities_resolved\":{}}}",
                        a.frame_index,
                        a.sim_tick,
                        echo_match,
                        hex(&a.client_projection_root),
                        a.entities_resolved
                    )
                })
                .collect();
            let body = format!(
                "{{\n\"schema\":\"renderer-bench-tape-v1\",\n\"scenario_id\":\"{}\",\n\"manifest_payload_sha256\":\"{}\",\n\"manifest_domain_sha256\":\"{}\",\n\"cadence\":{},\n\"frames\":[\n{}\n],\n\"client_acks\":[{}],\n\"ready_count\":{},\n\"run_root\":\"{}\",\n\"terminal_count\":0\n}}\n",
                run.manifest.scenario_id,
                hex(&run.manifest_payload_sha),
                hex(&run.manifest_domain_sha),
                run.cadence,
                frames_json.join(",\n"),
                acks_json.join(",\n"),
                signals.ready_count.load(std::sync::atomic::Ordering::Relaxed),
                hex(&rroot),
            );
            // artifact_transaction: tmp + rename (atomic on one filesystem).
            let tmp = run.out_path.with_extension("tmp");
            let ok = std::fs::write(&tmp, body.as_bytes())
                .and_then(|()| std::fs::rename(&tmp, &run.out_path));
            match ok {
                Ok(()) => info!(
                    path = %run.out_path.display(),
                    frames = run.frames.len(),
                    run_root = %hex(&rroot),
                    "bastion: renderer-bench TAPE WRITTEN"
                ),
                Err(e) => warn!(%e, "bastion: renderer-bench tape write FAILED"),
            }
            run.finished = true;
        }
    }
}
