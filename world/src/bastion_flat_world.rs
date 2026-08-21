//! bastion (FLAT TOWN): level the terrain at world centre **before** civs are
//! placed, so Veloren's own generator builds a real village on flat ground.
//!
//! Ben, 2026-08-21: *"i think we should place the town on our flat arena"* and
//! *"remember veloren actual town support pretty much all these systems"*.
//!
//! Those two together rule out the obvious implementations:
//!
//! * **Not** hand-building a town. Veloren's villages already have doors, beds,
//!   roads, farm fields and workshops; re-creating them badly throws away the
//!   thing Ben is pointing at.
//! * **Not** `bastion_flat_arena`'s chunk override. That builds a slab and
//!   returns *before* `world.generate_chunk`, which is the call that renders
//!   site structures — so a village inside the arena radius is never drawn at
//!   all. The arena is a terrain replacement, not a terrain flattener.
//!
//! So: flatten the SIM, then let the real generator do everything else.
//!
//! # Order is the whole design
//!
//! `World::generate` runs `WorldSim::generate` and *then*
//! `civ::Civs::generate(seed, &mut sim, …)`. Site altitudes are derived during
//! civ placement. Flattening **after** placement would move the ground out from
//! under buildings whose heights are already baked — floating or buried houses.
//! This runs in the gap between the two, which is the only correct place for it.
//!
//! # Off by default, and it moves every seed
//!
//! This is a worldgen change: with it on, a seed produces different terrain and
//! therefore different everything. It must never be enabled for a run compared
//! against a banked baseline. Hence an env gate that is absent by default, and
//! a loud emit naming the radius and the altitude it chose.

use crate::sim::WorldSim;
use common::terrain::TerrainChunkSize;
use common::vol::RectVolSize;
use vek::*;

/// Radius, in chunks, of the flattened disc — `BASTION_FLAT_WORLD_RADIUS`.
/// Absent (or 0) disables the whole feature.
pub fn radius_chunks() -> i32 {
    std::env::var("BASTION_FLAT_WORLD_RADIUS")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .filter(|r| *r > 0)
        .unwrap_or(0)
}

/// Flatten a disc of sim chunks at world centre to a single altitude.
///
/// Returns the number of chunks changed, so a caller (and a test) can tell
/// "disabled" from "enabled and did nothing" — those are different facts and
/// they otherwise look identical from the outside.
pub fn flatten(sim: &mut WorldSim) -> usize {
    let radius = radius_chunks();
    if radius <= 0 {
        return 0;
    }

    let centre = sim.get_size().map(|e| e as i32) / 2;

    // THE TARGET ALTITUDE IS THE CENTRE'S OWN, not a constant. A hardcoded z
    // would be a cliff wherever the surrounding world happens to sit, and the
    // village would be generated at the bottom of a pit or the top of a mesa.
    // Taking the centre chunk's existing altitude means the disc is level
    // *with itself* and as close to its surroundings as one number can be.
    let Some(target) = sim.get(centre).map(|c| c.alt) else {
        tracing::warn!(
            ?centre,
            "bastion: FLAT TOWN — no sim chunk at world centre; flattened nothing"
        );
        return 0;
    };

    let mut changed = 0usize;
    let mut max_drop = 0.0f32;
    let mut max_lift = 0.0f32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            // A DISC, not a square: a square leaves four corners further from
            // centre than the edges, and the seam is visibly a rectangle.
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let key = centre + Vec2::new(dx, dy);
            let Some(chunk) = sim.get_mut(key) else {
                continue;
            };
            // ★ TAPER THE RIM, or the disc is a CLIFF (2026-08-21). The
            // first leg flattened radius 14 with max_drop=82: an
            // eighty-two-block vertical wall right around the edge. Nothing
            // about that looks like a real place, and it is a pathing hazard
            // of exactly the kind this work exists to remove -- a colonist
            // who wanders to the rim has an 82-block fall waiting.
            //
            // Blend over the outer fifth of the radius so the plain meets the
            // world it sits in. `t` is 1.0 through the inner core and eases to
            // 0.0 at the boundary, so the centre is genuinely flat (which is
            // the point) and only the margin slopes.
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            let inner = radius as f32 * 0.8;
            let t = if d <= inner {
                1.0
            } else {
                let u = ((radius as f32 - d) / (radius as f32 - inner)).clamp(0.0, 1.0);
                // Smoothstep, not linear: a linear blend leaves a visible
                // crease where the taper meets the flat core.
                u * u * (3.0 - 2.0 * u)
            };
            let blended = chunk.alt + (target - chunk.alt) * t;

            let delta = blended - chunk.alt;
            if delta < 0.0 {
                max_drop = max_drop.max(-delta);
            } else {
                max_lift = max_lift.max(delta);
            }
            let target = blended;
            chunk.alt = target;
            // `basement` is the rock floor BENEATH the surface. Leaving it
            // above the new altitude would put stone through the ground where
            // we lowered terrain, so it moves with `alt` — but only downward,
            // because raising it would push rock up into a village's cellars.
            chunk.basement = chunk.basement.min(target);
            // WATER LAST, and this is the branch that decides whether the
            // result is a plain or a lake. `water_alt` is the surface water
            // height; if the land drops to `target` while water stays where it
            // was, the whole disc floods and the village generates underwater.
            chunk.water_alt = chunk.water_alt.min(target);
            changed += 1;
        }
    }

    tracing::info!(
        radius_chunks = radius,
        chunks_flattened = changed,
        target_alt = target,
        max_drop,
        max_lift,
        centre_block = ?(centre * TerrainChunkSize::RECT_SIZE.map(|e| e as i32)),
        "bastion: FLAT TOWN — sim levelled BEFORE civ placement; villages here \
         are generated by the real generator on flat ground"
    );
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate reads its env var and distinguishes the three states that
    /// otherwise look alike: off, on-with-a-radius, and malformed.
    ///
    /// Pinned because `radius_chunks` returning 0 is the DISABLED signal, and
    /// a parse failure silently returning 0 would disable the feature while
    /// looking exactly like "the user didn't ask for it" — the same
    /// present-but-unreachable shape that cost this project four bugs in one
    /// session.
    #[test]
    fn a_malformed_radius_disables_rather_than_defaulting_to_something() {
        // SAFETY: single-threaded test, no other reader of this var.
        unsafe {
            std::env::remove_var("BASTION_FLAT_WORLD_RADIUS");
        }
        assert_eq!(radius_chunks(), 0, "absent must mean disabled");

        unsafe {
            std::env::set_var("BASTION_FLAT_WORLD_RADIUS", "0");
        }
        assert_eq!(radius_chunks(), 0, "an explicit 0 must mean disabled");

        unsafe {
            std::env::set_var("BASTION_FLAT_WORLD_RADIUS", "not-a-number");
        }
        assert_eq!(
            radius_chunks(),
            0,
            "a malformed radius must DISABLE, never fall back to a guessed \
             radius that silently rewrites the world"
        );

        unsafe {
            std::env::set_var("BASTION_FLAT_WORLD_RADIUS", "12");
        }
        assert_eq!(radius_chunks(), 12, "a valid radius must be honoured");

        unsafe {
            std::env::remove_var("BASTION_FLAT_WORLD_RADIUS");
        }
    }
}
