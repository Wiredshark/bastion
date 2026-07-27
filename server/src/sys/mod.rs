pub mod agent;
pub mod chunk_send;
pub mod chunk_serialize;
pub mod entity_sync;
pub mod invite_timeout;
pub mod item;
pub mod loot;
pub mod metrics;
pub mod msg;
pub mod object;
pub mod persistence;
pub mod pets;
pub mod semantic_egress;
pub mod sentinel;
pub mod server_info;
pub mod subscription;
pub mod teleporter;
pub mod terrain;
pub mod terrain_sync;
pub mod waypoint;
pub mod wiring;

use common_ecs::{System, dispatch, run_now};
use common_systems::{melee, projectile};
use specs::DispatcherBuilder;
use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

pub type PersistenceScheduler = SysScheduler<persistence::Sys>;

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<melee::Sys>(dispatch_builder, &[&projectile::Sys::sys_name()]);
    //Note: server should not depend on interpolation system
    // T0.20 (master build order; ledger #150): the Controller phase
    // contract, DECLARED — `controller::Sys` = ConsumePreviousCommands
    // (drains the batch agents wrote LAST tick), `agent::Sys` =
    // AgentPlanNextCommands (writes the batch consumed NEXT tick). The
    // order was implicit registration staging (common systems register
    // before server systems); the explicit edge makes the double-buffered
    // command frame un-shufflable.
    dispatch::<agent::Sys>(dispatch_builder, &[
        &common_systems::controller::Sys::sys_name(),
    ]);
    // bastion (PATH-0): the sequential path scheduler runs AFTER the
    // agent tick — the tick surfaces routeless Goto colonists (holding
    // the Pending stance), this system searches under the global budget,
    // and the NEXT agent tick follows the delivered route (1-tick
    // latency; the packet's enqueue/consume-last-result shape in pull
    // form).
    dispatch::<crate::bastion_path::Sys>(dispatch_builder, &[&agent::Sys::sys_name()]);
    // Stage-1 B5.8: make the Agent -> PATH-0 -> Bastion handoff explicit.
    // Agent alone writes normal approach intent; Bastion may acquire the link
    // owner only after that pass and projects the exclusion used next tick.
    dispatch::<crate::bastion_jobs::Sys<crate::rtsim::RtSim>>(dispatch_builder, &[
        &agent::Sys::sys_name(),
        &crate::bastion_path::Sys::sys_name(),
    ]);
    dispatch::<crate::bastion_piles::Sys>(dispatch_builder, &[]);
    dispatch::<terrain::Sys>(dispatch_builder, &[&msg::terrain::Sys::sys_name()]);
    dispatch::<waypoint::Sys>(dispatch_builder, &[]);
    dispatch::<teleporter::Sys>(dispatch_builder, &[]);
    dispatch::<invite_timeout::Sys>(dispatch_builder, &[]);
    dispatch::<persistence::Sys>(dispatch_builder, &[]);
    dispatch::<object::Sys>(dispatch_builder, &[]);
    dispatch::<wiring::Sys>(dispatch_builder, &[]);
    // no dependency, as we only work once per sec anyway.
    dispatch::<chunk_serialize::Sys>(dispatch_builder, &[]);
    // don't depend on chunk_serialize, as we assume everything is done in a SlowJow
    dispatch::<chunk_send::Sys>(dispatch_builder, &[]);
    dispatch::<item::Sys>(dispatch_builder, &[]);
    dispatch::<server_info::Sys>(dispatch_builder, &[]);
}

pub fn run_sync_systems(ecs: &mut specs::World) {
    // Setup for entity sync
    // If I'm not mistaken, these two could be ran in parallel
    run_now::<sentinel::Sys>(ecs);
    run_now::<subscription::Sys>(ecs);

    // Sync
    run_now::<terrain_sync::Sys>(ecs);
    run_now::<entity_sync::Sys>(ecs);

    // APEX-T3.3.15: the single canonical semantic egress owner, invoked
    // explicitly last -- every current semantic producer (entity_sync,
    // subscription) already ran above in this same strictly-sequential
    // function; see semantic_egress.rs's own module doc for the one
    // known future wrinkle (a rare post-flush terrain::Sys re-run
    // elsewhere in the tick, inert until terrain.rs is migrated).
    run_now::<semantic_egress::Sys>(ecs);
}

/// Used to schedule systems to run at an interval
pub struct SysScheduler<S> {
    interval: Duration,
    last_run: Instant,
    /// T0.1 (master build order; ledger #5): sim-TICK cadence for
    /// deterministic mode. A wall-anchored interval lands on a
    /// machine-throughput-dependent sim tick — the same class the ENGOPT6
    /// recorder pair pinned for `LootOwner` (a 45-wall-second timeout
    /// resolving at tick 3960 on one VM and ~3976 on the other). Derived
    /// from the wall interval at the standard 30 tps fixed step.
    interval_ticks: u64,
    last_run_tick: Option<u64>,
    _phantom: PhantomData<S>,
}

impl<S> SysScheduler<S> {
    pub fn every(interval: Duration) -> Self {
        Self {
            interval,
            last_run: Instant::now(),
            interval_ticks: (interval.as_secs_f64() * bastion_server::SIM_TPS as f64).max(1.0)
                as u64,
            last_run_tick: None,
            _phantom: PhantomData,
        }
    }

    /// Interval check: wall-clock in live mode (original behavior),
    /// sim-tick in deterministic mode (lockstep fixed-step scheduling —
    /// two same-seed runs must fire on the same tick regardless of
    /// machine throughput). Both paths wait one full interval from
    /// construction/first-call before the first fire.
    pub fn should_run_at(&mut self, tick: u64, deterministic: bool) -> bool {
        if deterministic {
            match self.last_run_tick {
                None => {
                    self.last_run_tick = Some(tick);
                    false
                },
                Some(last) if tick.saturating_sub(last) > self.interval_ticks => {
                    self.last_run_tick = Some(tick);
                    true
                },
                Some(_) => false,
            }
        } else if self.last_run.elapsed() > self.interval {
            self.last_run = Instant::now();

            true
        } else {
            false
        }
    }
}

impl<S> Default for SysScheduler<S> {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            last_run: Instant::now(),
            interval_ticks: 900,
            last_run_tick: None,
            _phantom: PhantomData,
        }
    }
}

// T0.1: the tick path is a pure function of (tick, interval_ticks) — pinned
// so the deterministic cadence can never silently re-anchor to the wall.
#[cfg(test)]
mod sys_scheduler_tests {
    use super::SysScheduler;
    use std::time::Duration;

    #[test]
    fn t0_1_deterministic_cadence_is_tick_pure() {
        let mut sched: SysScheduler<()> = SysScheduler::every(Duration::from_secs(30));
        // First observation anchors, never fires.
        assert!(!sched.should_run_at(5, true));
        // Within the 900-tick interval: silent.
        assert!(!sched.should_run_at(904, true));
        assert!(!sched.should_run_at(905, true));
        // Past it: exactly one fire, then re-anchored.
        assert!(sched.should_run_at(906, true));
        assert!(!sched.should_run_at(907, true));
        assert!(!sched.should_run_at(1806, true));
        assert!(sched.should_run_at(1807, true));
        // Wall time never advanced in this test — the wall path could not
        // have produced these fires (the property the old API could not
        // even express).
    }
}
