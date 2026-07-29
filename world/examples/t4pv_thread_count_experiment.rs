//! `APEX-T4-PV-EXP` — the confirming experiment for the Q3 survey.
//!
//! The `T4-PV` survey established BY READING that no generation stage's
//! output depends on the order its parallel work completes, and stated
//! its own bound: that is a claim about SHAPE, stronger than a passing
//! test but different from a measurement. This is the measurement.
//!
//! **If this disagrees with the survey, the survey is wrong.** That was
//! written into the survey deliberately so the experiment could not be
//! argued away later.
//!
//! Usage: `cargo run --release --example t4pv_thread_count_experiment -- <threads>`
//! Run it at several thread counts and compare the printed root.
//!
//! What is digested: the canonical map-geometry root
//! (`world_map_geometry_root_v1`), which is `T4.3`'s OWN identity for a
//! generated map — reusing the program's existing canonical digest
//! rather than inventing a comparison, so a difference here is a
//! difference the rest of the program would also see.
//!
//! Note the world is GENERATED, not loaded: `FileOpts::LoadAsset` would
//! read a prebuilt map off disk and measure nothing about generation.

use veloren_world::{
    World,
    sim::{FileOpts, GenOpts, WorldOpts},
};

fn main() {
    let threads: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .expect("usage: t4pv_thread_count_experiment <threads>");

    let threadpool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("thread pool");

    // A smaller map than the 10/10 default, disclosed rather than
    // hidden: order-sensitivity in a stage would show at any size, and
    // this keeps a three-point experiment affordable. The stages under
    // test (erosion, civ placement, site generation, economy) all run.
    let gen_opts = GenOpts {
        x_lg: 9,
        y_lg: 9,
        ..Default::default()
    };

    let (world, index) = World::generate(
        1337,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::Generate(gen_opts),
            calendar: None,
        },
        &threadpool,
        &|_| {},
    );

    let map = world.get_map_data(index.as_index_ref(), &threadpool);
    let root = common_net::msg::world_msg::world_map_geometry_root_v1(&map);
    let hex: String = root
        .digest
        .bytes
        .as_array()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    println!(
        "T4PV-EXP threads={} actual={} map_geometry_root={}",
        threads,
        threadpool.current_num_threads(),
        hex
    );
}
