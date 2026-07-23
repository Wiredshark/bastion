use common::{
    grid::Grid,
    resources::TimeOfDay,
    weather::{CELL_SIZE, CHUNKS_PER_CELL, Weather, WeatherGrid},
};
use noise::{NoiseFn, Perlin, Seedable, SuperSimplex, Turbulence};
use vek::*;
use world::World;

use crate::weather::WEATHER_DT;

fn cell_to_wpos_center(p: Vec2<i32>) -> Vec2<i32> { p * CELL_SIZE as i32 + CELL_SIZE as i32 / 2 }

#[derive(Clone)]
struct WeatherZone {
    weather: Weather,
    /// Time, in seconds this zone lives.
    time_to_live: f32,
}

struct CellConsts {
    humidity: f32,
}

pub struct WeatherSim {
    size: Vec2<u32>,
    consts: Grid<CellConsts>,
    zones: Grid<Option<WeatherZone>>,
    /// DET-WTH-001 (v8 weather): the world seed, so the weather noise is a
    /// function of the world rather than fixed to zero.
    seed: u32,
}

/// A list of weather cells where lightning has a chance to strike.
#[derive(Default)]
pub struct LightningCells {
    pub cells: Vec<Vec2<i32>>,
}

impl WeatherSim {
    pub fn new(size: Vec2<u32>, world: &World) -> Self {
        Self {
            size,
            consts: Grid::from_raw(
                size.as_(),
                (0..size.x * size.y)
                    .map(|i| Vec2::new(i % size.x, i / size.x))
                    .map(|p| {
                        let mut humid_sum = 0.0;

                        for y in 0..CHUNKS_PER_CELL {
                            for x in 0..CHUNKS_PER_CELL {
                                let chunk_pos = p * CHUNKS_PER_CELL + Vec2::new(x, y);
                                if let Some(chunk) = world.sim().get(chunk_pos.as_()) {
                                    let env = chunk.get_environment();
                                    humid_sum += env.humid;
                                }
                            }
                        }
                        let average_humid = humid_sum / (CHUNKS_PER_CELL * CHUNKS_PER_CELL) as f32;
                        CellConsts {
                            humidity: average_humid.powf(0.2).min(1.0),
                        }
                    })
                    .collect::<Vec<_>>(),
            ),
            zones: Grid::new(size.as_(), None),
            seed: world.sim().seed,
        }
    }

    /// Adds a weather zone as a circle at a position, with a given radius. Both
    /// of which should be in weather cell units
    pub fn add_zone(&mut self, weather: Weather, pos: Vec2<f32>, radius: f32, time: f32) {
        let min: Vec2<i32> = (pos - radius).as_::<i32>().map(|e| e.max(0));
        let max: Vec2<i32> = (pos + radius)
            .ceil()
            .as_::<i32>()
            .map2(self.size.as_::<i32>(), |a, b| a.min(b));
        for y in min.y..max.y {
            for x in min.x..max.x {
                let ipos = Vec2::new(x, y);
                let p = ipos.as_::<f32>();

                if p.distance_squared(pos) < radius.powi(2) {
                    self.zones[ipos] = Some(WeatherZone {
                        weather,
                        time_to_live: time,
                    });
                }
            }
        }
    }

    // Time step is cell size / maximum wind speed.
    pub fn tick(&mut self, time_of_day: TimeOfDay, out: &mut WeatherGrid) -> LightningCells {
        let time = time_of_day.0;

        // DET-WTH-001 (v8 weather, High): the weather noise was seeded with a
        // literal 0 (both the SuperSimplex cores and the Turbulence Perlin
        // displacements defaulted to a fixed seed), so EVERY world generated
        // the identical weather base pattern independent of its world seed.
        // Derive each generator's seed from the world seed through the shared
        // DomainHasher (label-separated), matching worldgen's `noise_seed`.
        let weather_seed = self.seed;
        let noise_seed = |name: &str| -> u32 {
            let mut h =
                common::state_hash::DomainHasher::new("bastion/domain/weather-noise/v1/sha256");
            h.field(&weather_seed.to_le_bytes());
            h.field(name.as_bytes());
            u32::from_le_bytes(h.finish().0[..4].try_into().expect("sha256 >= 4 bytes"))
        };

        let base_nz: Turbulence<Turbulence<SuperSimplex, Perlin>, Perlin> = Turbulence::new(
            Turbulence::new(SuperSimplex::new(noise_seed("base")))
                .set_seed(noise_seed("base_turb_inner"))
                .set_frequency(0.2)
                .set_power(1.5),
        )
        .set_seed(noise_seed("base_turb_outer"))
        .set_frequency(2.0)
        .set_power(0.2);

        let rain_nz = SuperSimplex::new(noise_seed("rain"));

        let mut lightning_cells = Vec::new();
        for (point, cell) in out.iter_mut() {
            if let Some(zone) = &mut self.zones[point] {
                *cell = zone.weather;
                zone.time_to_live -= WEATHER_DT;
                if zone.time_to_live <= 0.0 {
                    self.zones[point] = None;
                }
            } else {
                let wpos = cell_to_wpos_center(point);

                let pos = wpos.as_::<f64>() + time * 0.1;

                let space_scale = 7_500.0;
                let time_scale = 100_000.0;
                let spos = (pos / space_scale).with_z(time / time_scale);

                let avg_scale = 30_000.0;
                let avg_delay = 250_000.0;
                let pressure = ((base_nz
                    .get((pos / avg_scale).with_z(time / avg_delay).into_array())
                    + base_nz.get(
                        (pos / (avg_scale * 0.25))
                            .with_z(time / (avg_delay * 0.25))
                            .into_array(),
                    ) * 0.5)
                    * 0.5
                    + 1.0)
                    .clamped(0.0, 1.0) as f32
                    + 0.55
                    - self.consts[point].humidity * 0.6;

                const RAIN_CLOUD_THRESHOLD: f32 = 0.25;
                // DET-WTH-004 (v8 weather, Critical — contract half): clamp the
                // generated cloud/rain into their declared 0..1 range. The raw
                // expressions overshoot (cloud is scaled ×4, rain is a powf of
                // an unbounded product), and Weather::get_kind then classifies
                // by thresholds — an out-of-contract value can split
                // authoritative classification. Bounding at the source enforces
                // the contract for every downstream consumer and shrinks the
                // cross-platform-float divergence surface. (The remaining
                // cross-platform-bit-identity concern over this pipeline is
                // DET-WTH-003, held.) The threshold checks below (0.2 / 0.15)
                // and is-raining are all < 1.0, so authoritative classification
                // is unchanged.
                cell.cloud = ((1.0 - pressure).max(0.0).powi(2) * 4.0).min(1.0);
                cell.rain = ((1.0 - pressure - RAIN_CLOUD_THRESHOLD).max(0.0)
                    * self.consts[point].humidity
                    * 2.5)
                    .powf(0.75)
                    .min(1.0);
                cell.wind = Vec2::new(
                    rain_nz.get(spos.into_array()).powi(3) as f32,
                    rain_nz.get((spos + 1.0).into_array()).powi(3) as f32,
                ) * 200.0
                    * (1.0 - pressure);
            }

            if cell.rain > 0.2 && cell.cloud > 0.15 {
                lightning_cells.push(point);
            }
        }
        LightningCells {
            cells: lightning_cells,
        }
    }

    pub fn size(&self) -> Vec2<u32> { self.size }
}

#[cfg(test)]
impl WeatherSim {
    /// WTH-01 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof): a
    /// worldgen-free constructor for the determinism fixture — a synthetic
    /// uniform-humidity arena at an EXPLICIT world seed. The determinism-critical
    /// surface (the seed-derived noise of DET-WTH-001 plus the DET-WTH-003/004
    /// clamps) is fully exercised without a `World`. Only the humidity `consts`
    /// are stubbed uniform; those are themselves a deterministic worldgen
    /// function of the seed and thus can never be a non-determinism SOURCE, so
    /// stubbing them faithfully isolates the seed -> weather contract under test.
    fn from_seed_for_test(size: Vec2<u32>, seed: u32, humidity: f32) -> Self {
        let n = (size.x * size.y) as usize;
        Self {
            size,
            consts: Grid::from_raw(
                size.as_(),
                (0..n).map(|_| CellConsts { humidity }).collect::<Vec<_>>(),
            ),
            zones: Grid::new(size.as_(), None),
            seed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `WeatherSim::tick` over a full-day TimeOfDay sweep and capture the
    /// entire weather field as RAW f32 BITS (exact / ULP-sensitive — the "raw
    /// bits, never approximate" discipline), plus the derived lightning-cell
    /// set, as one flat signature. Grid iteration is row-major (deterministic).
    fn run_sequence(seed: u32) -> Vec<u32> {
        let size = Vec2::new(6u32, 5u32);
        let mut sim = WeatherSim::from_seed_for_test(size, seed, 0.5);
        let mut grid = WeatherGrid::new(size);
        let mut sig = Vec::new();
        // 48 half-hour steps = one in-game day, so the time-dependent noise axis
        // (advection + pressure delay) is genuinely swept, not a single frame.
        for step in 0..48u64 {
            let tod = TimeOfDay((step as f64) * 1800.0);
            let lightning = sim.tick(tod, &mut grid);
            for (_p, cell) in grid.iter() {
                sig.push(cell.cloud.to_bits());
                sig.push(cell.rain.to_bits());
                sig.push(cell.wind.x.to_bits());
                sig.push(cell.wind.y.to_bits());
            }
            // Fold the authoritative lightning-cell classification into the
            // signature too (it is the downstream consumer of the thresholds).
            sig.push(lightning.cells.len() as u32);
            for c in &lightning.cells {
                sig.push(c.x as u32);
                sig.push(c.y as u32);
            }
        }
        sig
    }

    /// DETERMINISM: two independent WeatherSim runs at the SAME world seed must
    /// produce byte-identical weather over a full-day sweep. No ambient entropy
    /// (thread_rng / wall-clock / HashMap order) may reach the weather pipeline.
    /// Guards DET-WTH-001 (seed-derived noise) and DET-WTH-003/004 (the clamps).
    #[test]
    fn weather_sim_is_seed_deterministic() {
        let a = run_sequence(1000);
        let b = run_sequence(1000);
        assert_eq!(
            a, b,
            "same-seed weather diverged: a non-deterministic input reached WeatherSim::tick"
        );
    }

    /// NON-VACUITY: different world seeds MUST produce different weather. This is
    /// the exact DET-WTH-001 regression — pre-fix the noise cores were seeded
    /// with a literal 0, so every world got identical weather regardless of its
    /// seed. Without this assertion the determinism test above would pass even
    /// if the seed were ignored entirely, making the proof vacuous.
    #[test]
    fn weather_sim_seed_is_non_vacuous() {
        let a = run_sequence(1000);
        let c = run_sequence(2024);
        assert_ne!(
            a, c,
            "different seeds produced identical weather: the seed -> noise \
             derivation (DET-WTH-001) is not actually wired"
        );
    }
}
