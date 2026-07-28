use common::{
    comp,
    event::EventBus,
    outcome::Outcome,
    resources::{DeltaTime, ProgramTime, TimeOfDay},
    slowjob::{SlowJob, SlowJobPool},
    weather::{SharedWeatherGrid, Weather, WeatherGrid},
};
use common_ecs::{Origin, Phase, System};
use common_net::msg::ServerGeneral;
use rand::{RngExt, seq::IteratorRandom};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, Write, WriteExpect};
use std::{mem, sync::Arc};
use vek::Vec2;
use world::World;

use crate::{Tick, client::Client};

use super::{
    WEATHER_DT,
    sim::{LightningCells, WeatherSim},
};

enum WeatherJobState {
    #[expect(dead_code)]
    Working(SlowJob),
    Idle(WeatherSim),
    None,
}

pub struct WeatherJob {
    last_update: ProgramTime,
    weather_tx: crossbeam_channel::Sender<(WeatherGrid, LightningCells, WeatherSim)>,
    weather_rx: crossbeam_channel::Receiver<(WeatherGrid, LightningCells, WeatherSim)>,
    state: WeatherJobState,
    qeued_zones: Vec<(Weather, Vec2<f32>, f32, f32)>,
    /// T0.87: the ContentEpoch pattern applied to weather's own background
    /// regeneration rather than to asset content -- a monotonic counter
    /// incremented exactly once at the single named adoption point, plus
    /// the server tick that adoption happened on. Under
    /// `SlowJobPool::new_inline` (already wired for
    /// `ExecutionMode::DeterministicSerial`, see `common/state/src/
    /// state.rs`), the background job runs synchronously inside `spawn`,
    /// so the result is already sitting in the channel by the time this
    /// system's NEXT tick polls it -- making `adopted_at_tick` a fixed
    /// +1-tick offset from the spawn tick, not a wall-clock race.
    epoch: u64,
    adopted_at_tick: u64,
}

impl WeatherJob {
    pub fn queue_zone(&mut self, weather: Weather, pos: Vec2<f32>, radius: f32, time: f32) {
        self.qeued_zones.push((weather, pos, radius, time))
    }

    /// The weather generation currently live. Starts at 1 (the boot-time
    /// initial generation counts as the first adoption).
    pub fn epoch(&self) -> u64 { self.epoch }

    /// The server tick the current generation was adopted on.
    pub fn adopted_at_tick(&self) -> u64 { self.adopted_at_tick }
}

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, TimeOfDay>,
        Read<'a, ProgramTime>,
        Read<'a, Tick>,
        Read<'a, DeltaTime>,
        Write<'a, LightningCells>,
        Write<'a, Option<WeatherJob>>,
        WriteExpect<'a, WeatherGrid>,
        WriteExpect<'a, SlowJobPool>,
        Read<'a, EventBus<Outcome>>,
        ReadExpect<'a, Arc<World>>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, comp::Pos>,
    );

    const NAME: &'static str = "weather::tick";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (
            entities,
            game_time,
            program_time,
            tick,
            delta_time,
            mut lightning_cells,
            mut weather_job,
            mut grid,
            slow_job_pool,
            outcomes,
            world,
            clients,
            positions,
        ): Self::SystemData,
    ) {
        let to_update = match &mut *weather_job {
            Some(weather_job) => (program_time.0 - weather_job.last_update.0 >= WEATHER_DT as f64)
                .then_some(weather_job),
            None => {
                let (weather_tx, weather_rx) = crossbeam_channel::bounded(1);

                let weather_size = world.sim().get_size() / common::weather::CHUNKS_PER_CELL;
                let mut sim = WeatherSim::new(weather_size, &world);
                *grid = WeatherGrid::new(sim.size());
                *lightning_cells = sim.tick(*game_time, &mut grid);

                *weather_job = Some(WeatherJob {
                    last_update: *program_time,
                    weather_tx,
                    weather_rx,
                    state: WeatherJobState::Idle(sim),
                    qeued_zones: Vec::new(),
                    epoch: 1,
                    adopted_at_tick: tick.0,
                });

                None
            },
        };

        if let Some(weather_job) = to_update {
            if matches!(weather_job.state, WeatherJobState::Working(_))
                && let Ok((new_grid, new_lightning_cells, sim)) = weather_job.weather_rx.try_recv()
            {
                *grid = new_grid;
                *lightning_cells = new_lightning_cells;
                weather_job.epoch += 1;
                weather_job.adopted_at_tick = tick.0;
                let mut lazy_msg = None;
                for client in clients.join() {
                    if lazy_msg.is_none() {
                        lazy_msg = Some(client.prepare(ServerGeneral::WeatherUpdate(
                            SharedWeatherGrid::from(&*grid),
                        )));
                    }
                    lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
                }
                weather_job.state = WeatherJobState::Idle(sim);
            }

            if matches!(weather_job.state, WeatherJobState::Idle(_)) {
                weather_job.last_update = *program_time;
                let old_state = mem::replace(&mut weather_job.state, WeatherJobState::None);

                let WeatherJobState::Idle(mut sim) = old_state else {
                    unreachable!()
                };

                let weather_tx = weather_job.weather_tx.clone();
                let game_time = *game_time;
                for (weather, pos, radius, time) in weather_job.qeued_zones.drain(..) {
                    sim.add_zone(weather, pos, radius, time)
                }
                let job = slow_job_pool.spawn("WEATHER", move || {
                    let mut grid = WeatherGrid::new(sim.size());
                    let lightning_cells = sim.tick(game_time, &mut grid);
                    let _ = weather_tx.send((grid, lightning_cells, sim));
                });

                weather_job.state = WeatherJobState::Working(job);
            }
        }

        // Chance to emit lightning every frame from one or more of the cells that
        // currently has the correct weather conditions.
        let mut outcome_emitter = outcomes.emitter();
        let mut rng = rand::rng();
        let num_cells = lightning_cells.cells.len() as f64 * 0.0015 * delta_time.0 as f64;
        let num_cells = num_cells.floor() as u32 + rng.random_bool(num_cells.fract()) as u32;

        for _ in 0..num_cells {
            let cell_pos = lightning_cells.cells.iter().choose(&mut rng).expect(
                "This is non-empty, since we multiply with its len for the chance to do a \
                 lightning strike.",
            );
            let wpos = cell_pos.map(|e| {
                (e as f32 + rng.random_range(0.0..1.0)) * common::weather::CELL_SIZE as f32
            });
            outcome_emitter.emit(Outcome::Lightning {
                pos: wpos.with_z(world.sim().get_alt_approx(wpos.as_()).unwrap_or(0.0)),
            });
        }

        for (entity, client, pos) in (&entities, &clients, &positions).join() {
            if entity.id() as u64 % 30 == tick.0 % 30 {
                let weather = grid.get_interpolated(pos.0.xy());
                client.send_fallible(ServerGeneral::LocalWindUpdate(weather.wind));
            }
        }
    }
}

/// T0.87: the WeatherJob adoption pattern (spawn on `SlowJobPool`, adopt on
/// the next `try_recv`) reduced to its essential shape -- a bounded
/// crossbeam channel fed by a spawned job, polled by the caller -- so the
/// inline-vs-async distinction can be tested in isolation, without standing
/// up a full ECS World/dispatcher.
#[cfg(test)]
mod tests {
    use super::{WeatherJob, WeatherJobState};
    use common::slowjob::SlowJobPool;
    use std::{thread, time::Duration};

    fn minimal_weather_job(epoch: u64, adopted_at_tick: u64) -> WeatherJob {
        let (weather_tx, weather_rx) = crossbeam_channel::bounded(1);
        WeatherJob {
            last_update: Default::default(),
            weather_tx,
            weather_rx,
            state: WeatherJobState::None,
            qeued_zones: Vec::new(),
            epoch,
            adopted_at_tick,
        }
    }

    #[test]
    fn epoch_and_adopted_at_tick_report_what_was_recorded() {
        let job = minimal_weather_job(3, 41);
        assert_eq!(job.epoch(), 3);
        assert_eq!(job.adopted_at_tick(), 41);
    }

    fn inline_pool() -> SlowJobPool {
        let threadpool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        SlowJobPool::new_inline(0, std::sync::Arc::new(threadpool))
    }

    fn async_pool() -> SlowJobPool {
        let threadpool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        let pool = SlowJobPool::new(1, 0, std::sync::Arc::new(threadpool));
        pool.configure("WEATHER", |_| 1);
        pool
    }

    /// The property `epoch`/`adopted_at_tick` rely on: under an INLINE
    /// pool, the job's result is already in the channel by the time
    /// `spawn` returns, even if the job artificially takes real wall-clock
    /// time -- so a poll immediately after spawn always succeeds. This is
    /// what makes `adopted_at_tick` a fixed +1-tick offset instead of a
    /// wall-clock race, under `ExecutionMode::DeterministicSerial`
    /// (which is exactly when `common/state/src/state.rs` selects
    /// `new_inline`).
    #[test]
    fn inline_pool_adoption_is_available_immediately_after_spawn() {
        let pool = inline_pool();
        let (tx, rx) = crossbeam_channel::bounded(1);
        pool.spawn("WEATHER", move || {
            thread::sleep(Duration::from_millis(50));
            let _ = tx.send(42u32);
        });
        // No sleep here: if this line raced the job, it would be empty.
        assert_eq!(
            rx.try_recv(),
            Ok(42),
            "inline pool must run the job before spawn() returns"
        );
    }

    /// Falsifier: the SAME artificial delay against the ASYNC pool must
    /// NOT be available immediately -- proving the test above actually
    /// discriminates inline-vs-async rather than passing vacuously (e.g.
    /// because the job happened to be trivially fast). If this test ever
    /// goes green, `new_inline` no longer means what `WeatherJob` assumes
    /// it means, and T0.87's whole determinism story is void.
    #[test]
    fn async_pool_adoption_is_not_available_immediately_after_spawn() {
        let pool = async_pool();
        let (tx, rx) = crossbeam_channel::bounded(1);
        pool.spawn("WEATHER", move || {
            thread::sleep(Duration::from_millis(50));
            let _ = tx.send(42u32);
        });
        assert_eq!(
            rx.try_recv(),
            Err(crossbeam_channel::TryRecvError::Empty),
            "async pool must NOT have the result ready immediately after spawn -- if it does, \
             the artificial delay isn't discriminating and this falsifier is worthless"
        );
        // Confirm it does eventually complete (the delay is real, not a
        // permanently-stuck job) -- and that it succeeds strictly LATER.
        assert_eq!(rx.recv_timeout(Duration::from_millis(500)), Ok(42));
    }
}
