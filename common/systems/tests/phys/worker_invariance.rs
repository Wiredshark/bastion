//! `APEX-T6.3` — entity-pushback invariance under worker count.
//!
//! This row was specified as an ordering FIX, on the claim that
//! `apply_pushback` accumulates contributions under Rayon while
//! `DET-PHY-005` had only canonicalised which candidates exist. That
//! claim was wrong and is retracted in the T6 tier spec: `par_join()`
//! parallelises over ENTITIES, each task accumulates into a task-local
//! `vel_delta` and writes only its own velocity under specs' disjoint
//! mutable access, the neighbour walk is a nested-range cell traversal
//! using `grid.get(&cell)` lookups rather than map iteration, and each
//! cell's contents are already `Uid`-canonical. The only cross-task
//! reduction in the system reduces `PhysicsMetrics`, whose fields are
//! `u64` counters — associative, so no float hazard there either.
//!
//! So the row is a PINNING TEST. That is not a downgrade of it:
//!
//! - nothing in the tree currently prevents a future edit from turning
//!   the per-entity join into a shared accumulation, and the failure
//!   would present as rare cross-machine drift rather than a test break;
//! - the existing physics harness never varies worker count, so this is
//!   new coverage, not a duplicate;
//! - the candidate-PERMUTATION half of the row is already covered where
//!   it belongs, by `DET-PHY-005`'s own test on `SpatialGrid`. It is
//!   strengthened there with a non-vacuity check rather than rebuilt
//!   here.
//!
//! Worker-count invariance is the axis that actually catches a Rayon
//! dependence: a permutation test can pass while partitioning still
//! leaks.

use crate::utils;
use common::{
    comp::{Pos, Vel},
    uid::Uid,
};
use common_ecs::dispatch;
use specs::{Join, WorldExt};
use std::collections::BTreeMap;
use utils::DT;
use vek::Vec3;

/// Worker counts to compare. 1 removes work-stealing entirely; 48 is this
/// build's parallelism cap, so it is the widest partitioning the fixture
/// will realistically meet.
const WORKER_COUNTS: [usize; 4] = [1, 2, 8, 48];

const TICKS: usize = 30;
/// Enough bodies in one grid cell that a cell holds several candidates
/// and the pushback loop has something to order.
const BODIES: usize = 12;

/// Everything the fixture claims is reproducible, keyed by stable
/// identity rather than by ECS index. Raw bits, not floats: `==` on `f32`
/// hides `-0.0` and NaN payloads, which is exactly the class of
/// difference a partitioning leak produces first.
///
/// The worker count is deliberately NOT a field: it differs per leg by
/// construction, and a tape that carried it could never compare equal.
/// It is returned alongside and asserted separately.
#[derive(Debug, PartialEq, Eq)]
struct Tape {
    per_entity: BTreeMap<u64, [u32; 6]>,
    collisions: u64,
    checks: u64,
}

fn phys_only(dispatch_builder: &mut specs::DispatcherBuilder) {
    dispatch::<veloren_common_systems::phys::Sys>(dispatch_builder, &[]);
}

/// Returns the tape and the worker count the pool ACTUALLY had.
fn run_fixture(threads: usize) -> (Tape, usize) {
    let mut state = utils::setup_with_worker_count(threads, phys_only);
    let workers = state.thread_pool().current_num_threads();

    // A tight ring: every body overlaps its neighbours, so the pushback
    // path is engaged rather than merely reached.
    for i in 0..BODIES {
        let angle = i as f32 * std::f32::consts::TAU / BODIES as f32;
        utils::create_fixed_body(
            &mut state,
            Vec3::new(16.0 + 0.45 * angle.cos(), 16.0 + 0.45 * angle.sin(), 265.0),
        );
    }

    for _ in 0..TICKS {
        utils::tick(&mut state, DT);
    }

    let ecs = state.ecs();
    let uids = ecs.read_storage::<Uid>();
    let positions = ecs.read_storage::<Pos>();
    let velocities = ecs.read_storage::<Vel>();
    let per_entity = (&uids, &positions, &velocities)
        .join()
        .map(|(uid, pos, vel)| {
            (uid.0.get(), [
                pos.0.x.to_bits(),
                pos.0.y.to_bits(),
                pos.0.z.to_bits(),
                vel.0.x.to_bits(),
                vel.0.y.to_bits(),
                vel.0.z.to_bits(),
            ])
        })
        .collect();
    let metrics = ecs.read_resource::<common_ecs::PhysicsMetrics>();

    (
        Tape {
            per_entity,
            collisions: metrics.entity_entity_collisions,
            checks: metrics.entity_entity_collision_checks,
        },
        workers,
    )
}

/// `T6.3`: the physics tape is a function of the fixture, not of how
/// Rayon partitioned it.
#[test]
fn entity_pushback_is_invariant_to_worker_count() {
    let (baseline, baseline_workers) = run_fixture(WORKER_COUNTS[0]);

    // Preconditions, asserted rather than assumed. A pass means nothing
    // if the pushback path never ran, or if every leg used one pool.
    assert_eq!(
        baseline_workers, WORKER_COUNTS[0],
        "the fixture did not get the pool it asked for; the invariance claim would be vacuous"
    );
    assert!(
        baseline.collisions > 0,
        "no entity-entity collisions occurred, so apply_pushback's accumulation never ran: \
         {baseline:?}"
    );
    assert_eq!(baseline.per_entity.len(), BODIES, "fixture bodies went missing");
    assert!(
        baseline.per_entity.values().any(|bits| bits[3] != 0 || bits[4] != 0),
        "no body acquired horizontal velocity, so nothing was actually pushed back"
    );

    for threads in WORKER_COUNTS.into_iter().skip(1) {
        let (tape, workers) = run_fixture(threads);
        assert_eq!(
            workers, threads,
            "leg asked for {threads} workers and got {workers}; this leg proves nothing"
        );
        assert_eq!(
            tape, baseline,
            "the physics tape changed at {threads} workers. Rayon's partitioning is reaching the \
             simulation: check whether apply_pushback still accumulates task-locally and writes \
             only its own entity"
        );
    }
}
