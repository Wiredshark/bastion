use crate::{
    astar::{Astar, PathResult},
    resources::Time,
    terrain::{Block, SpriteKind},
    vol::{BaseVol, ReadVol},
};
use common_base::span;
use fxhash::FxBuildHasher;
#[cfg(feature = "rrt_pathfinding")]
use hashbrown::HashMap;
#[cfg(feature = "rrt_pathfinding")]
use kiddo::{SquaredEuclidean, float::kdtree::KdTree, nearest_neighbour::NearestNeighbour}; /* For RRT paths (disabled for now) */
use rand::{RngExt, SeedableRng, rng};
// RNG-DEEP-009 (determinism audit): ChaCha8Rng, not SmallRng — the hidden
// Chaser stream must be a portable named generator or its state transitions
// diverge cross-machine (SmallRng's algorithm is explicitly unstable).
use rand_chacha::ChaCha8Rng;
#[cfg(feature = "rrt_pathfinding")]
use rand::{
    distr::{Distribution, Uniform},
    prelude::IteratorRandom,
};
#[cfg(feature = "rrt_pathfinding")]
use std::f32::consts::PI;
use std::{collections::VecDeque, iter::FromIterator};
use vek::*;

// Path

#[derive(Clone, Debug)]
pub struct Path<T> {
    pub nodes: Vec<T>,
}

impl<T> Default for Path<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::default(),
        }
    }
}

impl<T> FromIterator<T> for Path<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            nodes: iter.into_iter().collect(),
        }
    }
}

impl<T> IntoIterator for Path<T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter { self.nodes.into_iter() }
}

impl<T> Path<T> {
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

    pub fn len(&self) -> usize { self.nodes.len() }

    pub fn iter(&self) -> impl Iterator<Item = &T> { self.nodes.iter() }

    pub fn start(&self) -> Option<&T> { self.nodes.first() }

    pub fn end(&self) -> Option<&T> { self.nodes.last() }

    pub fn nodes(&self) -> &[T] { &self.nodes }
}

// Route: A path that can be progressed along

#[derive(Default, Clone, Debug)]
pub struct Route {
    path: Path<Vec3<i32>>,
    next_idx: usize,
}

impl Route {
    pub fn get_path(&self) -> &Path<Vec3<i32>> { &self.path }

    pub fn next_idx(&self) -> usize { self.next_idx }
}

impl From<Path<Vec3<i32>>> for Route {
    fn from(path: Path<Vec3<i32>>) -> Self { Self { path, next_idx: 0 } }
}

pub struct TraversalConfig {
    /// The distance to a node at which node is considered visited.
    pub node_tolerance: f32,
    /// The slowdown factor when following corners.
    /// 0.0 = no slowdown on corners, 1.0 = total slowdown on corners.
    pub slow_factor: f32,
    /// Whether the agent is currently on the ground.
    pub on_ground: bool,
    /// Whether the agent is currently in water.
    pub in_liquid: bool,
    /// The distance to the target below which it is considered reached.
    pub min_tgt_dist: f32,
    /// Whether the agent can climb.
    pub can_climb: bool,
    /// bastion (B5.8): the tallest vertical face (blocks) this agent may
    /// path over beyond plain walking — 0 disables all bastion vertical
    /// edges (vanilla NPCs), 2 = novice colonist (jump range anyway), 3 =
    /// trained climber (unlocks the 3-up scramble edges). SKILL-DRIVEN:
    /// the agent system maps the colonist's `climbing` movement skill to
    /// this each tick, so reach GROWS with use (Ben's climbing-is-a-skill
    /// directive). Also gates ladder edges (any reach > 0). Deliberately
    /// separate from `can_climb` (humanoid body capability): execution
    /// rides the existing jump → wall-contact → auto-`Climb` chain.
    pub scramble_reach: u8,
    /// Whether the agent can fly.
    pub can_fly: bool,
    /// Whether the agent has vectored propulsion.
    pub vectored_propulsion: bool,
    /// Whether chunk containing target position is currently loaded
    pub is_target_loaded: bool,
    /// bastion (PATH-0): whether THIS chase call may run/resume the A*
    /// search inline. Colonists on job travel are SCHEDULED (the
    /// sequential budgeted path scheduler owns their searches; the agent
    /// tick only follows delivered routes) — they pass false. Vanilla
    /// NPCs (and colonists in non-Goto states, e.g. combat) pass true
    /// and search exactly as before. When false with no route, `chase`
    /// holds the Pending stance — byte-identical to a mid-search tick
    /// today — until the scheduler's `search_step` delivers.
    pub search_allowed: bool,
}

const DIAGONALS: [Vec2<i32>; 8] = [
    Vec2::new(1, 0),
    Vec2::new(1, 1),
    Vec2::new(0, 1),
    Vec2::new(-1, 1),
    Vec2::new(-1, 0),
    Vec2::new(-1, -1),
    Vec2::new(0, -1),
    Vec2::new(1, -1),
];

pub enum TraverseStop {
    Done,
    InvalidOutput,
    InvalidPath,
}

impl Route {
    pub fn path(&self) -> &Path<Vec3<i32>> { &self.path }

    pub fn next(&self, i: usize) -> Option<Vec3<i32>> {
        self.path.nodes.get(self.next_idx + i).copied()
    }

    pub fn is_finished(&self) -> bool { self.next(0).is_none() }

    /// Handles moving along a path.
    pub fn traverse<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        traversal_cfg: &TraversalConfig,
    ) -> Result<(Vec3<f32>, f32), TraverseStop>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        let (next0, next1, next_tgt, be_precise) = loop {
            // If we've reached the end of the path, stop
            let next0 = self.next(0).ok_or(TraverseStop::Done)?;
            let next1 = self.next(1).unwrap_or(next0);

            // Stop using obstructed paths
            if !walkable(vol, next0, traversal_cfg.is_target_loaded)
                || !walkable(vol, next1, traversal_cfg.is_target_loaded)
            {
                return Err(TraverseStop::InvalidPath);
            }

            // If, in any direction, there is a column of open air of several blocks
            let open_space_nearby = DIAGONALS.iter().any(|pos| {
                (-2..2).all(|z| {
                    vol.get(next0 + Vec3::new(pos.x, pos.y, z))
                        .map(|b| !b.is_solid())
                        .unwrap_or(false)
                })
            });

            // If, in any direction, there is a solid wall
            let wall_nearby = DIAGONALS.iter().any(|pos| {
                vol.get(next0 + Vec3::new(pos.x, pos.y, 1))
                    .map(|b| b.is_solid())
                    .unwrap_or(true)
            });

            // Unwalkable obstacles, such as walls or open space or stepping up blocks can
            // affect path-finding
            let be_precise =
                open_space_nearby || wall_nearby || (pos.z - next0.z as f32).abs() > 1.0;

            // If we're not being precise and the next next target is closer, go towards
            // that instead.
            if !be_precise
                && next0.as_::<f32>().distance_squared(pos)
                    > next1.as_::<f32>().distance_squared(pos)
            {
                self.next_idx += 1;
                continue;
            }

            // Map position of node to middle of block
            let next_tgt = next0.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0);
            let closest_tgt = next_tgt
                .map2(pos, |tgt, pos| pos.clamped(tgt.floor(), tgt.ceil()))
                .xy()
                .with_z(next_tgt.z);
            // Determine whether we're close enough to the next to to consider it completed
            let dist_sqrd = pos.xy().distance_squared(closest_tgt.xy());
            if dist_sqrd
                < (traversal_cfg.node_tolerance
                    * if be_precise {
                        0.5
                    } else if traversal_cfg.in_liquid {
                        2.5
                    } else {
                        1.0
                    })
                .powi(2)
                && ((-1.0..=2.25).contains(&(pos.z - closest_tgt.z))
                    || (traversal_cfg.in_liquid
                        && pos.z < closest_tgt.z + 0.8
                        && pos.z > closest_tgt.z))
            {
                // Node completed, move on to the next one
                self.next_idx += 1;
            } else {
                // The next node hasn't been reached yet, use it as a target
                break (next0, next1, next_tgt, be_precise);
            }
        };

        fn gradient(line: LineSegment2<f32>) -> f32 {
            let r = (line.start.y - line.end.y) / (line.start.x - line.end.x);
            if r.is_nan() { 100000.0 } else { r }
        }

        fn intersect(a: LineSegment2<f32>, b: LineSegment2<f32>) -> Option<Vec2<f32>> {
            let ma = gradient(a);
            let mb = gradient(b);

            let ca = a.start.y - ma * a.start.x;
            let cb = b.start.y - mb * b.start.x;

            if (ma - mb).abs() < 0.0001 || (ca - cb).abs() < 0.0001 {
                None
            } else {
                let x = (cb - ca) / (ma - mb);
                let y = ma * x + ca;

                Some(Vec2::new(x, y))
            }
        }

        let line_segments = [
            LineSegment3 {
                start: self
                    .next_idx
                    .checked_sub(2)
                    .and_then(|i| self.path().nodes().get(i))
                    .unwrap_or(&next0)
                    .as_()
                    + 0.5,
                end: self
                    .next_idx
                    .checked_sub(1)
                    .and_then(|i| self.path().nodes().get(i))
                    .unwrap_or(&next0)
                    .as_()
                    + 0.5,
            },
            LineSegment3 {
                start: self
                    .next_idx
                    .checked_sub(1)
                    .and_then(|i| self.path().nodes().get(i))
                    .unwrap_or(&next0)
                    .as_()
                    + 0.5,
                end: next0.as_() + 0.5,
            },
            LineSegment3 {
                start: next0.as_() + 0.5,
                end: next1.as_() + 0.5,
            },
        ];

        if line_segments
            .iter()
            .map(|ls| {
                if self.next_idx > 1 {
                    ls.projected_point(pos).distance_squared(pos)
                } else {
                    LineSegment2 {
                        start: ls.start.xy(),
                        end: ls.end.xy(),
                    }
                    .projected_point(pos.xy())
                    .distance_squared(pos.xy())
                }
            })
            .reduce(|a, b| a.min(b))
            .is_some_and(|d| {
                d > if traversal_cfg.in_liquid {
                    traversal_cfg.node_tolerance * 5.0
                } else {
                    traversal_cfg.node_tolerance * 2.0
                }
                .powi(2)
            })
        {
            return Err(TraverseStop::InvalidPath);
        }

        // We don't always want to aim for the centre of block since this can create
        // jerky zig-zag movement. This function attempts to find a position
        // inside a target block's area that aligned nicely with our velocity.
        // This has a twofold benefit:
        //
        // 1. Entities can move at any angle when
        // running on a flat surface
        //
        // 2. We don't have to search diagonals when
        // pathfinding - cartesian positions are enough since this code will
        // make the entity move smoothly along them
        let corners = [
            Vec2::new(0, 0),
            Vec2::new(1, 0),
            Vec2::new(1, 1),
            Vec2::new(0, 1),
            Vec2::new(0, 0), // Repeated start
        ];

        let vel_line = LineSegment2 {
            start: pos.xy(),
            end: pos.xy() + vel.xy() * 100.0,
        };

        let align = |block_pos: Vec3<i32>, precision: f32| {
            let lerp_block =
                |x, precision| Lerp::lerp(x, block_pos.xy().map(|e| e as f32), precision);

            (0..4)
                .filter_map(|i| {
                    let edge_line = LineSegment2 {
                        start: lerp_block(
                            (block_pos.xy() + corners[i]).map(|e| e as f32),
                            precision,
                        ),
                        end: lerp_block(
                            (block_pos.xy() + corners[i + 1]).map(|e| e as f32),
                            precision,
                        ),
                    };
                    intersect(vel_line, edge_line).filter(|intersect| {
                        intersect
                            .clamped(
                                block_pos.xy().map(|e| e as f32),
                                block_pos.xy().map(|e| e as f32 + 1.0),
                            )
                            .distance_squared(*intersect)
                            < 0.001
                    })
                })
                .min_by_key(|intersect: &Vec2<f32>| {
                    (intersect.distance_squared(vel_line.end) * 1000.0) as i32
                })
                .unwrap_or_else(|| {
                    (0..2)
                        .flat_map(|i| (0..2).map(move |j| Vec2::new(i, j)))
                        .map(|rpos| block_pos + rpos)
                        .map(|block_pos| {
                            let block_posf = block_pos.xy().map(|e| e as f32);
                            let proj = vel_line.projected_point(block_posf);
                            let clamped = lerp_block(
                                proj.clamped(
                                    block_pos.xy().map(|e| e as f32),
                                    block_pos.xy().map(|e| e as f32),
                                ),
                                precision,
                            );

                            (proj.distance_squared(clamped), clamped)
                        })
                        .min_by_key(|(d2, _)| (d2 * 1000.0) as i32)
                        .unwrap()
                        .1
                })
        };

        let bez = CubicBezier2 {
            start: pos.xy(),
            ctrl0: pos.xy() + vel.xy().try_normalized().unwrap_or_default() * 1.0,
            ctrl1: align(next0, 1.0),
            end: align(next1, 1.0),
        };

        // Use a cubic spline of the next few targets to come up with a sensible target
        // position. We want to use a position that gives smooth movement but is
        // also accurate enough to avoid the agent getting stuck under ledges or
        // falling off walls.
        let next_dir = bez
            .evaluate_derivative(0.85)
            .try_normalized()
            .unwrap_or_default();
        let straight_factor = next_dir
            .dot(vel.xy().try_normalized().unwrap_or(next_dir))
            .max(0.0)
            .powi(2);

        let bez = CubicBezier2 {
            start: pos.xy(),
            ctrl0: pos.xy() + vel.xy().try_normalized().unwrap_or_default() * 1.0,
            ctrl1: align(
                next0,
                (1.0 - if (next0.z as f32 - pos.z).abs() < 0.25 && !be_precise {
                    straight_factor
                } else {
                    0.0
                })
                .max(0.1),
            ),
            end: align(next1, 1.0),
        };

        let tgt2d = bez.evaluate(if (next0.z as f32 - pos.z).abs() < 0.25 {
            0.25
        } else {
            0.5
        });
        let tgt = if be_precise {
            next_tgt
        } else {
            Vec3::from(tgt2d) + Vec3::unit_z() * next_tgt.z
        };

        Some((
            tgt - pos,
            // Control the entity's speed to hopefully stop us falling off walls on sharp
            // corners. This code is very imperfect: it does its best but it
            // can still fail for particularly fast entities.
            1.0 - (traversal_cfg.slow_factor * (1.0 - straight_factor)).min(0.9),
        ))
        .filter(|(bearing, _)| bearing.z < 2.1)
        .ok_or(TraverseStop::InvalidOutput)
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// How long the path we're trying to compute should be.
pub enum PathLength {
    #[default]
    Small,
    Medium,
    Long,
    Longest,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathState {
    /// There is no path.
    #[default]
    None,
    /// A non-complete path.
    Exhausted,
    /// In progress of computing a path.
    Pending,
    /// A complete path.
    Path,
}

/// Read-only flight-recorder view of hidden Chaser state. This intentionally
/// exposes no mutation and is used only by env-gated Bastion diagnostics.
#[derive(Clone, Debug)]
pub struct ChaserDiagnosticSnapshot {
    pub last_search_target: Option<Vec3<f32>>,
    pub route_target: Option<Vec3<f32>>,
    pub route_complete: Option<bool>,
    pub route_head: Option<Vec3<i32>>,
    pub route_next_idx: Option<usize>,
    pub path_state: PathState,
    pub recent_state_count: usize,
}

/// A self-contained system that attempts to chase a moving target, only
/// performing pathfinding if necessary
impl TraversalConfig {
    /// bastion ledger #178: the SEARCH-PROFILE fingerprint — every field
    /// that changes which nodes/edges a search ADMITS or how it terminates.
    /// A retained search from a different profile is stale by construction:
    /// the falsifier shows a loaded=true continuation routing THROUGH an
    /// unloaded band its own profile forbids, on admission state carried
    /// from an optimistic (loaded=false) retained frontier. (Geometric-cost
    /// staleness self-heals post-ENGOPT2-reopen — every cfg read goes
    /// through current closures — but ADMISSION is baked into visited
    /// entries at insert time.) Floats hash by bit pattern.
    pub fn search_profile_key(&self) -> u64 {
        use core::hash::{Hash, Hasher};
        let mut h = fxhash::FxHasher64::default();
        self.node_tolerance.to_bits().hash(&mut h);
        self.in_liquid.hash(&mut h);
        self.can_climb.hash(&mut h);
        self.scramble_reach.hash(&mut h);
        self.can_fly.hash(&mut h);
        self.vectored_propulsion.hash(&mut h);
        self.is_target_loaded.hash(&mut h);
        h.finish()
    }
}

#[derive(Default, Clone, Debug)]
pub struct Chaser {
    last_search_tgt: Option<Vec3<f32>>,
    /// `bool` indicates whether the Route is a complete route to the target
    ///
    /// `Vec3` is the target end pos
    route: Option<(Route, bool, Vec3<f32>)>,
    /// We use this hasher (FxHash) because:
    /// (1) we don't care about DDOS attacks (We can use FxHash);
    /// (2) we want this to be constant across compiles because of hot-reloading
    /// (Ruling out AAHash);
    ///
    /// The Vec3 is the astar's start position.
    astar: Option<(Astar<Node, FxBuildHasher>, Vec3<f32>)>,
    /// bastion ledger #178: the search-profile key the retained `astar`
    /// was built under — a mismatch invalidates it (see
    /// `TraversalConfig::search_profile_key`).
    astar_profile: Option<u64>,
    /// bastion ledger #180: ACTUAL expansions consumed by the most recent
    /// search step (poll delta), as opposed to the [`Self::planned_iters`]
    /// estimate. The PATH-0 scheduler debits this against its tick budget.
    last_search_consumed: u64,
    flee_from: Option<Vec3<f32>>,
    /// Whether to allow consideration of longer paths, npc will stand still
    /// while doing this.
    path_length: PathLength,

    /// The current state of the path.
    path_state: PathState,

    /// The last time the `chase` method was called.
    last_update_time: Option<Time>,

    /// (position, requested walk dir)
    recent_states: VecDeque<(Time, Vec3<f32>, Vec3<f32>)>,
    /// ARCH-003: a per-tick deterministic stream installed by the server's
    /// deterministic harness mode. Live mode leaves this as `None` and keeps
    /// the existing OS-seeded entropy.
    deterministic_rng: Option<ChaCha8Rng>,
}

impl Chaser {
    /// Select the random stream used by hidden Chaser state transitions.
    /// Reinstalling a seed once per agent tick makes those transitions a pure
    /// function of (world seed, tick, uid) in deterministic harness mode.
    pub fn set_deterministic_seed(&mut self, seed: Option<u64>) {
        self.deterministic_rng = seed.map(ChaCha8Rng::seed_from_u64);
    }

    fn stuck_check(
        &mut self,
        pos: Vec3<f32>,
        bearing: Vec3<f32>,
        speed: f32,
        time: &Time,
    ) -> (Vec3<f32>, f32, bool) {
        /// The min amount of cached items.
        const MIN_CACHED_STATES: usize = 3;
        /// The max amount of cached items.
        const MAX_CACHED_STATES: usize = 10;
        /// Cache over 1 second.
        const CACHED_TIME_SPAN: f64 = 1.0;
        const TOLERANCE: f32 = 0.2;

        // We pop the first until there is only one element which was over
        // `CACHED_TIME_SPAN` seconds ago.
        while self.recent_states.len() > MIN_CACHED_STATES
            && self
                .recent_states
                .get(1)
                .is_some_and(|(t, ..)| time.0 - t.0 > CACHED_TIME_SPAN)
        {
            self.recent_states.pop_front();
        }

        if self.recent_states.len() < MAX_CACHED_STATES {
            self.recent_states.push_back((*time, pos, bearing * speed));

            if self.recent_states.len() >= MIN_CACHED_STATES
                && self
                    .recent_states
                    .front()
                    .is_some_and(|(t, ..)| time.0 - t.0 > CACHED_TIME_SPAN)
                && (bearing * speed).magnitude_squared() > 0.01
            {
                let average_pos = self
                    .recent_states
                    .iter()
                    .map(|(_, pos, _)| *pos)
                    .sum::<Vec3<f32>>()
                    * (1.0 / self.recent_states.len() as f32);
                let max_distance_sqr = self
                    .recent_states
                    .iter()
                    .map(|(_, pos, _)| pos.distance_squared(average_pos))
                    .reduce(|a, b| a.max(b));

                let average_speed = self
                    .recent_states
                    .iter()
                    .zip(self.recent_states.iter().skip(1).map(|(t, ..)| *t))
                    .map(|((t0, _, bearing), t1)| {
                        bearing.magnitude_squared() * (t1.0 - t0.0).powi(2) as f32
                    })
                    .sum::<f32>()
                    * (1.0 / self.recent_states.len() as f32);

                let is_stuck =
                    max_distance_sqr.is_some_and(|d| d < (average_speed * TOLERANCE).powi(2));

                let bearing = if is_stuck {
                    let choice = self.deterministic_rng.as_mut().map_or_else(
                        || rng().random_range(0..100u32),
                        |rng| rng.random_range(0..100u32),
                    );
                    match choice {
                        0..10 => -bearing,
                        10..20 => Vec3::new(bearing.y, bearing.x, bearing.z),
                        20..30 => Vec3::new(-bearing.y, bearing.x, bearing.z),
                        30..50 => {
                            if let Some((route, ..)) = &mut self.route {
                                route.next_idx = route.next_idx.saturating_sub(1);
                            }

                            bearing
                        },
                        50..60 => {
                            if let Some((route, ..)) = &mut self.route {
                                route.next_idx = route.next_idx.saturating_sub(2);
                            }

                            bearing
                        },
                        _ => bearing,
                    }
                } else {
                    bearing
                };

                return (bearing, speed, is_stuck);
            }
        }
        (bearing, speed, false)
    }

    fn reset(&mut self) {
        self.route = None;
        self.astar = None;
        self.last_search_tgt = None;
        self.path_length = Default::default();
        self.flee_from = None;
    }

    /// Returns bearing and speed
    /// Bearing is a `Vec3<f32>` dictating the direction of movement
    /// Speed is an f32 between 0.0 and 1.0
    pub fn chase<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        tgt: Vec3<f32>,
        traversal_cfg: TraversalConfig,
        time: &Time,
    ) -> Option<(Vec3<f32>, f32, bool)>
    where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        span!(_guard, "chase", "Chaser::chase");
        self.last_update_time = Some(*time);
        // If we're already close to the target then there's nothing to do
        if ((pos - tgt) * Vec3::new(1.0, 1.0, 2.0)).magnitude_squared()
            < traversal_cfg.min_tgt_dist.powi(2)
        {
            self.reset();
            return None;
        }

        let d = tgt.distance_squared(pos);

        // Check if the current route is no longer valid.
        if let Some(end) = self.route.as_ref().map(|(_, _, end)| *end)
            && self.flee_from.is_none()
            && self.path_length < PathLength::Longest
            && d < tgt.distance_squared(end)
        {
            self.path_length = Default::default();
            self.route = None;
        }

        // If we're closer than the designated `flee_from` position, we ignore
        // that.
        if self.flee_from.is_some_and(|p| d < p.distance_squared(tgt)) {
            self.route = None;
            self.flee_from = None;
            self.astar = None;
            self.path_length = Default::default();
        }

        // Find a route if we don't have one.
        if self.route.is_none() {
            if !traversal_cfg.search_allowed {
                // bastion (PATH-0): the search is deferred to the
                // sequential scheduler — hold the Pending stance (the
                // pre-existing mid-search behavior, no new movement
                // class); `search_step` delivers the route between
                // agent ticks.
                self.path_state = PathState::Pending;
            } else {
                self.search_step_inner(vol, pos, tgt, &traversal_cfg);
            }
        }

        if let Some((route, ..)) = &mut self.route {
            let res = route.traverse(vol, pos, vel, &traversal_cfg);

            // None either means we're done, or can't continue, either way we don't care
            // about that route anymore.
            if let Err(e) = &res {
                self.route = None;
                match e {
                    TraverseStop::InvalidOutput => {
                        return Some(self.stuck_check(
                            pos,
                            (tgt - pos).try_normalized().unwrap_or(Vec3::unit_x()),
                            1.0,
                            time,
                        ));
                    },
                    TraverseStop::InvalidPath => {
                        // If the path is invalid, blocks along the path have most likely changed,
                        // so reset the astar.
                        self.astar = None;
                    },
                    TraverseStop::Done => match self.path_state {
                        PathState::None => {
                            return Some(self.stuck_check(
                                pos,
                                (tgt - pos).try_normalized().unwrap_or_default(),
                                1.0,
                                time,
                            ));
                        },
                        PathState::Exhausted => {
                            // Upgrade path length if path is exhausted and we're at the same
                            // position.
                            if self.astar.as_ref().is_some_and(|(.., start)| {
                                start.distance_squared(pos) < traversal_cfg.node_tolerance.powi(2)
                            }) {
                                match self.path_length {
                                    PathLength::Small => {
                                        self.path_length = PathLength::Medium;
                                    },
                                    PathLength::Medium => {
                                        self.path_length = PathLength::Long;
                                    },
                                    PathLength::Long => {
                                        self.path_length = PathLength::Longest;
                                    },
                                    PathLength::Longest => {
                                        self.flee_from = Some(pos);
                                        self.astar = None;
                                    },
                                }
                            } else {
                                self.astar = None;
                            }
                        },
                        PathState::Pending | PathState::Path => {},
                    },
                }
            }

            let (bearing, speed) = res.ok()?;

            return Some(self.stuck_check(pos, bearing, speed, time));
        }

        None
    }

    /// bastion (PATH-0): the search half of [`Chaser::chase`], verbatim —
    /// run/resume the incremental A* toward `tgt` and store the result.
    /// Shared by the inline path (search_allowed) and the sequential
    /// scheduler's [`Chaser::search_step`]. No traverse, no stuck-shuffle
    /// rng — deterministic in scheduler context.
    fn search_step_inner<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        tgt: Vec3<f32>,
        traversal_cfg: &TraversalConfig,
    ) where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        // Reset astar if last tgt is too far from tgt.
        if self
            .last_search_tgt
            .is_some_and(|last_tgt| tgt.distance_squared(last_tgt) > 2.0)
        {
            self.astar = None;
        }
        // bastion ledger #178: reset the retained search on a PROFILE
        // change — admission staleness does not self-heal (see
        // search_profile_key's doc; falsifier: the loaded-flip band test).
        let profile = traversal_cfg.search_profile_key();
        if self.astar_profile.is_some_and(|p| p != profile) {
            self.astar = None;
        }
        self.astar_profile = Some(profile);
        let (result, consumed) = find_path(
            &mut self.astar,
            vol,
            pos,
            tgt,
            traversal_cfg,
            self.path_length,
            self.flee_from,
        );
        // bastion ledger #180: actual expansions this step spent — the
        // scheduler debits this, not its planned estimate.
        self.last_search_consumed = consumed;
        match result {
            PathResult::Pending => {
                self.path_state = PathState::Pending;
            },
            PathResult::None(path) => {
                self.path_state = PathState::None;
                self.route = Some((Route { path, next_idx: 0 }, false, tgt));
            },
            PathResult::Exhausted(path) => {
                self.path_state = PathState::Exhausted;
                self.route = Some((Route { path, next_idx: 0 }, false, tgt));
            },
            PathResult::Path(path, _) => {
                self.flee_from = None;
                self.path_state = PathState::Path;
                self.path_length = Default::default();
                self.route = Some((Route { path, next_idx: 0 }, true, tgt));
            },
        }

        self.last_search_tgt = Some(tgt);
    }

    /// bastion (PATH-0): does this chaser need the scheduler to run a
    /// search? Route presence is the exact condition `chase`'s own search
    /// arm keys on, read AFTER the agent tick applied its invalidations.
    /// bastion ledger #183 (REVERTED, floor-red M3A): a no-path negative
    /// cache here suppressed the scheduler's search cycle, and the
    /// organic-egress machinery turned out to depend on that cycle's side
    /// effects — the changed movement duty relocated waiting colonists,
    /// which flipped the feet-anchored egress-target computation to an
    /// unreachable elevated cell and stranded ladder-queue members
    /// ([66,null,null] vs baseline [66,82,94]). Do not re-land a cache
    /// until egress target selection is decoupled from search-cycle
    /// behavior.
    pub fn needs_search(&self) -> bool { self.route.is_none() }

    /// Test-only: the stored route's nodes (ledger #178 falsifier surface).
    #[cfg(test)]
    fn route_nodes(&self) -> Option<Vec<Vec3<i32>>> {
        self.route
            .as_ref()
            .map(|(route, ..)| route.path.nodes.clone())
    }

    /// bastion (PATH-0): the scheduler's search entry — one budgeted
    /// search/resume for `tgt`, storing the route the next agent tick
    /// follows. A no-op if a route already exists (grant raced an inline
    /// delivery — never double-spends budget on a routed chaser).
    pub fn search_step<V>(
        &mut self,
        vol: &V,
        pos: Vec3<f32>,
        tgt: Vec3<f32>,
        traversal_cfg: &TraversalConfig,
    ) where
        V: BaseVol<Vox = Block> + ReadVol,
    {
        if self.route.is_none() {
            self.search_step_inner(vol, pos, tgt, traversal_cfg);
        } else {
            // bastion ledger #180: the no-op arm spent nothing — never let
            // a stale delta from an earlier step be debited again.
            self.last_search_consumed = 0;
        }
    }

    /// bastion (PATH-0): the per-call iteration budget the NEXT search
    /// step hands `astar.poll` — `find_path`'s own [`PathLength`] map,
    /// exposed as the scheduler's budget-accounting unit.
    pub fn planned_iters(&self) -> u64 {
        match self.path_length {
            PathLength::Small => 250,
            PathLength::Medium => 400,
            PathLength::Long => 500,
            PathLength::Longest => 750,
        }
    }

    /// bastion ledger #180: ACTUAL expansions the most recent search step
    /// consumed — the scheduler debits this against its tick budget after
    /// the step (admission still uses the conservative
    /// [`Self::planned_iters`] estimate, so the cap cannot be exceeded:
    /// actual <= planned for every step).
    pub fn last_search_consumed(&self) -> u64 { self.last_search_consumed }

    pub fn get_route(&self) -> Option<&Route> { self.route.as_ref().map(|(r, ..)| r) }

    pub fn last_target(&self) -> Option<Vec3<f32>> { self.last_search_tgt }

    /// Start stuck detection with fresh observations after a higher-level
    /// movement owner deliberately replaces its target.
    ///
    /// This does not invalidate the cached route or disable stuck recovery:
    /// subsequent calls to [`Self::chase`] immediately begin accumulating a
    /// new history for the replacement target. Callers should invoke it once
    /// at the ownership/target handoff, not on every movement tick.
    pub fn rebase_stuck_history(&mut self) { self.recent_states.clear(); }

    /// Diagnostic view of the target stored with the currently cached route.
    ///
    /// This is intentionally read-only: Bastion's env-gated route-writer
    /// trace uses it to distinguish a newly installed `NpcActivity::Goto`
    /// target from a route that was computed for an earlier target.  It must
    /// not be used to mutate or invalidate normal Chaser behavior.
    pub fn route_target(&self) -> Option<Vec3<f32>> {
        self.route.as_ref().map(|(_, _, target)| *target)
    }

    /// Whether the cached route reaches its stored target. Read-only
    /// diagnostic companion to [`Self::route_target`].
    pub fn route_is_complete(&self) -> Option<bool> {
        self.route.as_ref().map(|(_, complete, _)| *complete)
    }

    pub fn diagnostic_snapshot(&self) -> ChaserDiagnosticSnapshot {
        ChaserDiagnosticSnapshot {
            last_search_target: self.last_search_tgt,
            route_target: self.route.as_ref().map(|(_, _, target)| *target),
            route_complete: self.route.as_ref().map(|(_, complete, _)| *complete),
            route_head: self
                .route
                .as_ref()
                .and_then(|(route, _, _)| route.get_path().nodes().get(route.next_idx()).copied()),
            route_next_idx: self.route.as_ref().map(|(route, _, _)| route.next_idx()),
            path_state: self.path_state,
            recent_state_count: self.recent_states.len(),
        }
    }

    pub fn state(&self) -> (PathLength, PathState) { (self.path_length, self.path_state) }

    pub fn last_update_time(&self) -> Time {
        self.last_update_time.unwrap_or(Time(f64::NEG_INFINITY))
    }
}

fn walkable<V>(vol: &V, pos: Vec3<i32>, is_target_loaded: bool) -> bool
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let mut below_z = 1;
    // We loop downwards
    let below = loop {
        if let Some(block) = vol.get(pos - Vec3::unit_z() * below_z).ok().copied() {
            if block.is_solid() || block.is_liquid() {
                break block;
            }

            below_z += 1;

            if below_z > Block::MAX_HEIGHT.ceil() as i32 {
                break Block::empty();
            }
        } else if is_target_loaded {
            break Block::empty();
        } else {
            // If not loaded assume we can walk there.
            break Block::new(crate::terrain::BlockKind::Misc, Default::default());
        }
    };

    let a = vol.get(pos).ok().copied().unwrap_or_else(Block::empty);
    let b = vol
        .get(pos + Vec3::unit_z())
        .ok()
        .copied()
        .unwrap_or_else(Block::empty);

    let on_ground = (below_z == 1 && below.is_filled())
        || below.get_sprite().is_some_and(|sprite| {
            sprite
                .solid_height()
                .is_some_and(|h| ((below_z - 1) as f32) < h && h <= below_z as f32)
        });
    let in_liquid = a.is_liquid();
    (on_ground || in_liquid) && !a.is_solid() && !b.is_solid()
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Node {
    pos: Vec3<i32>,
    last_dir: Vec2<i32>,
    last_dir_count: u32,
}

/// Attempt to search for a path to a target, returning the path (if one was
/// found) and whether it is complete (reaches the target)
///
/// If `flee_from` is `Some` this will attempt to both walk away from that
/// position and towards the target.
/// bastion ledger #180: also returns the ACTUAL expansions this call
/// consumed (the poll delta) so schedulers can debit real work instead of
/// their planned estimate.
/// bastion: the price of taking a 3-block vertical face instead of walking
/// around the obstacle. See the derivation at its use site in `transition`.
///
/// PINNED, not tuned by feel: `a_scramble_must_cost_more_than_rounding_a_house`
/// asserts the relationship this number exists to hold. A surcharge that merely
/// "feels high" would drift the first time someone edited the flat-move cost,
/// because every weight here is relative to a flat step and the flat step is
/// NOT 1.0.
const SCRAMBLE_SURCHARGE: f32 = 30.0;

fn find_path<V>(
    astar: &mut Option<(Astar<Node, FxBuildHasher>, Vec3<f32>)>,
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    traversal_cfg: &TraversalConfig,
    path_length: PathLength,
    flee_from: Option<Vec3<f32>>,
) -> (PathResult<Vec3<i32>>, u64)
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let is_walkable = |pos: &Vec3<i32>| walkable(vol, *pos, traversal_cfg.is_target_loaded);
    let get_walkable_z = |pos| {
        let mut z_incr = 0;
        for _ in 0..32 {
            let test_pos = pos + Vec3::unit_z() * z_incr;
            if is_walkable(&test_pos) {
                return Some(test_pos);
            }
            z_incr = -z_incr + i32::from(z_incr <= 0);
        }
        None
    };

    // Find walkable ground for start and end.
    let (start, end) = match (
        get_walkable_z(startf.map(|e| e.floor() as i32)),
        get_walkable_z(endf.map(|e| e.floor() as i32)),
    ) {
        (Some(start), Some(end)) => (start, end),

        // Special case for partially loaded path finding
        (Some(start), None) if !traversal_cfg.is_target_loaded => {
            (start, endf.map(|e| e.floor() as i32))
        },

        _ => return (PathResult::None(Path::default()), 0),
    };

    let heuristic = |node: &Node| {
        let diff = end.as_::<f32>() - node.pos.as_::<f32>();
        let d = diff.magnitude();

        d - flee_from.map_or(0.0, |p| {
            let ndiff = p - node.pos.as_::<f32>() - 0.5;
            let nd = ndiff.magnitude();
            nd.sqrt() * ((diff / d).dot(ndiff / nd) + 0.1).max(0.0) * 10.0
        })
    };
    let transition = |a: Node, b: Node| {
        1.0
            // Discourage travelling in the same direction for too long: this encourages
            // turns to be spread out along a path, more closely approximating a straight
            // line toward the target.
            + b.last_dir_count as f32 * 0.01
            // Penalise jumping
            + (b.pos.z - a.pos.z + 1).max(0) as f32 * 2.0
            // bastion (B5.8): scrambles (3-up) cost more than the staircase
            // they replace (three 1-ups = 15) so carved/built stairs stay
            // preferred; a scramble is the fallback, not the highway.
            // ★ RE-PRICED 8.0 -> 30.0 (Ben, 2026-08-21: "climbing and falling
            // should be actively discouraged -- a colonist scaling a house wall
            // to reach a crate is a bug even when it works").
            //
            // The 8.0 was calibrated against a STAIRCASE: three 1-ups cost 15,
            // a scramble cost 17, so stairs won narrowly. That was the right
            // comparison for the job it was written for and the WRONG one for
            // a house. Do the arithmetic on the case Ben is complaining about:
            //
            //   a flat move costs 3.0, NOT 1.0 -- `(dz+1).max(0)*2.0` charges
            //   2.0 even when dz is zero, and every weight here is relative to
            //   that. A scramble at 17.0 is therefore only 5.7 flat steps.
            //
            //   over a 6-block wall = 2 scrambles (34) + roof (~8 flat, 24)
            //                       + fall (6)                  ~= 64
            //   around a 10x10 house = ~20 extra flat steps      ~= 60
            //
            // Effectively TIED, so the roof wins whenever the detour is a
            // little longer or the target is on the far side. That is the
            // route that produces the wall-run: `traverse` jumps when
            // `bearing.z > 1.5`, the jump puts the colonist airborne against
            // the wall, and `handle_wallrun` fires on `on_wall && !on_ground`.
            //
            // At 30.0 a scramble is 39.0 = 13 flat steps, so two of them
            // (26 steps) clearly exceed walking around. Stairs (1-up, 5.0) and
            // ladders (1.5) are UNTOUCHED -- this discourages exactly the thing
            // Ben called a bug and nothing else a colonist legitimately does.
            + if b.pos.z - a.pos.z >= 3 { SCRAMBLE_SURCHARGE } else { 0.0 }
    };
    let neighbors = |node: &Node| {
        let node = *node;
        let pos = node.pos;
        const DIRS: [Vec3<i32>; 9] = [
            Vec3::new(0, 1, 0), // Forward
            Vec3::new(0, 1, 1), // Forward upward
            // Vec3::new(0, 1, -1),  // Forward downward
            // Vec3::new(0, 1, -2),  // Forward downwardx2
            Vec3::new(1, 0, 0), // Right
            Vec3::new(1, 0, 1), // Right upward
            // Vec3::new(1, 0, -1),  // Right downward
            // Vec3::new(1, 0, -2),  // Right downwardx2
            Vec3::new(0, -1, 0), // Backwards
            Vec3::new(0, -1, 1), // Backward Upward
            // Vec3::new(0, -1, -1), // Backward downward
            // Vec3::new(0, -1, -2), // Backward downwardx2
            Vec3::new(-1, 0, 0), // Left
            Vec3::new(-1, 0, 1), // Left upward
            // Vec3::new(-1, 0, -1), // Left downward
            // Vec3::new(-1, 0, -2), // Left downwardx2
            Vec3::new(0, 0, -1), // Downwards
        ];

        const JUMPS: [Vec3<i32>; 4] = [
            Vec3::new(0, 1, 2),  // Forward Upwardx2
            Vec3::new(1, 0, 2),  // Right Upwardx2
            Vec3::new(0, -1, 2), // Backward Upwardx2
            Vec3::new(-1, 0, 2), // Left Upwardx2
        ];

        // bastion (B5.8): 3-up scramble edges — a short vertical face taken
        // by jump + auto-climb. Gated on `can_scramble` (colony workers
        // only; see TraversalConfig) and priced above an equivalent
        // staircase in `transition`, so carved/built stairs stay preferred
        // wherever they exist.
        const SCRAMBLES: [Vec3<i32>; 4] = [
            Vec3::new(0, 1, 3),  // Forward Upwardx3
            Vec3::new(1, 0, 3),  // Right Upwardx3
            Vec3::new(0, -1, 3), // Backward Upwardx3
            Vec3::new(-1, 0, 3), // Left Upwardx3
        ];

        /// The cost of falling a block.
        const FALL_COST: f32 = 1.5;

        let walkable = [
            (is_walkable(&(pos + Vec3::new(1, 0, 0))), Vec3::new(1, 0, 0)),
            (
                is_walkable(&(pos + Vec3::new(-1, 0, 0))),
                Vec3::new(-1, 0, 0),
            ),
            (is_walkable(&(pos + Vec3::new(0, 1, 0))), Vec3::new(0, 1, 0)),
            (
                is_walkable(&(pos + Vec3::new(0, -1, 0))),
                Vec3::new(0, -1, 0),
            ),
        ];

        // Discourage walking alog walls/edges.
        let edge_cost = if path_length < PathLength::Medium {
            walkable.iter().any(|(w, _)| !*w) as i32 as f32
        } else {
            0.0
        };

        // const DIAGONALS: [(Vec3<i32>, [usize; 2]); 8] = [
        //     (Vec3::new(1, 1, 0), [0, 2]),
        //     (Vec3::new(-1, 1, 0), [1, 2]),
        //     (Vec3::new(1, -1, 0), [0, 3]),
        //     (Vec3::new(-1, -1, 0), [1, 3]),
        //     (Vec3::new(1, 1, 1), [0, 2]),
        //     (Vec3::new(-1, 1, 1), [1, 2]),
        //     (Vec3::new(1, -1, 1), [0, 3]),
        //     (Vec3::new(-1, -1, 1), [1, 3]),
        // ];

        DIRS.iter()
            .chain(
                (vol.get(pos - Vec3::unit_z())
                    .map(|b| !b.is_liquid())
                    .unwrap_or(traversal_cfg.is_target_loaded)
                    || traversal_cfg.can_climb
                    || traversal_cfg.can_fly).then_some(JUMPS.iter())
                    .into_iter().flatten()
            )
            .chain(
                (traversal_cfg.scramble_reach >= 3)
                    .then_some(SCRAMBLES.iter())
                    .into_iter()
                    .flatten(),
            )
            .map(move |dir| (pos, dir))
            .filter(move |(pos, dir)| {
                (traversal_cfg.can_fly || is_walkable(pos) && is_walkable(&(*pos + **dir)))
                    && ((dir.z < 1
                        || vol
                            .get(pos + Vec3::unit_z() * 2)
                            .map(|b| !b.is_solid())
                            .unwrap_or(traversal_cfg.is_target_loaded))
                        && (dir.z < 2
                            || vol
                                .get(pos + Vec3::unit_z() * 3)
                                .map(|b| !b.is_solid())
                                .unwrap_or(traversal_cfg.is_target_loaded))
                        // bastion (B5.8): scramble corridor — one more block
                        // of clearance above the start so the body can rise
                        // 3 along the face before topping out.
                        && (dir.z < 3
                            || vol
                                .get(pos + Vec3::unit_z() * 4)
                                .map(|b| !b.is_solid())
                                .unwrap_or(traversal_cfg.is_target_loaded))
                        && (dir.z >= 0
                            || vol
                                .get(pos + *dir + Vec3::unit_z() * 2)
                                .map(|b| !b.is_solid())
                                .unwrap_or(traversal_cfg.is_target_loaded)))
            })
            .map(move |(pos, dir)| {
                let next_node = Node {
                    pos: pos + dir,
                    last_dir: dir.xy(),
                    last_dir_count: if node.last_dir == dir.xy() {
                        node.last_dir_count + 1
                    } else {
                        0
                    },
                };

                (
                    next_node,
                    transition(node, next_node) + if dir.z == 0 { edge_cost } else { 0.0 },
                )
            })
            // Falls
            .chain(walkable.into_iter().filter_map(move |(w, dir)| {
                let pos = pos + dir;
                if w ||
                    vol.get(pos).map(|b| b.is_solid()).unwrap_or(true) ||
                    vol.get(pos + Vec3::unit_z()).map(|b| b.is_solid()).unwrap_or(true) {
                    return None;
                }

                let down = (1..12).find(|i| is_walkable(&(pos - Vec3::unit_z() * *i)))?;

                let next_node = Node {
                    pos: pos - Vec3::unit_z() * down,
                    last_dir: dir.xy(),
                    last_dir_count: 0,
                };

                // Falling costs a lot.
                Some((next_node, match down {
                    1..=2 => {
                        transition(node, next_node)
                    }
                    _ => FALL_COST * (down - 2) as f32,
                }))
            }))
            // bastion (B5.8): LADDER edges — vertical moves in a cell
            // beside a `SpriteKind::Ladder` column (the ladder block itself
            // is solid; you climb its face). Bypasses the walkable filter
            // (mid-climb cells have no floor); requires body space at the
            // destination and a ladder beside BOTH ends so the column is
            // continuous. Gated like scrambles (colony workers) so vanilla
            // NPCs don't start using dungeon ladders. Cheap: below-normal
            // vertical cost — a placed ladder should beat a scramble.
            .chain(
                (traversal_cfg.scramble_reach > 0)
                    .then(|| {
                        let beside_ladder = move |p: Vec3<i32>| {
                            [
                                Vec2::new(1, 0),
                                Vec2::new(-1, 0),
                                Vec2::new(0, 1),
                                Vec2::new(0, -1),
                            ]
                            .into_iter()
                            .any(|d| {
                                vol.get(p + Vec3::new(d.x, d.y, 0)).is_ok_and(|b| {
                                    b.get_sprite() == Some(SpriteKind::Ladder)
                                })
                            })
                        };
                        [Vec3::unit_z(), -Vec3::unit_z()]
                            .into_iter()
                            .filter_map(move |dir| {
                                let next = pos + dir;
                                let clear = |p: Vec3<i32>| {
                                    vol.get(p).map(|b| !b.is_solid()).unwrap_or(false)
                                };
                                // OR, not AND: the mount edge starts on
                                // ground BELOW the bottom rung (no ladder
                                // beside the ground cell) and the top-out
                                // edge rises one past the top rung — one
                                // end beside the column suffices.
                                (clear(next)
                                    && clear(next + Vec3::unit_z())
                                    && (beside_ladder(pos) || beside_ladder(next)))
                                .then(|| {
                                    let next_node = Node {
                                        pos: next,
                                        last_dir: Vec2::zero(),
                                        last_dir_count: 0,
                                    };
                                    (next_node, 1.5)
                                })
                            })
                            // DISMOUNT: step off the climb onto an adjacent
                            // walkable ledge (mid-climb cells have no floor,
                            // so the normal DIRS edges never fire there).
                            // This is why a ladder must be built one block
                            // ABOVE the ledge it serves.
                            .chain(
                                [
                                    Vec3::new(1, 0, 0),
                                    Vec3::new(-1, 0, 0),
                                    Vec3::new(0, 1, 0),
                                    Vec3::new(0, -1, 0),
                                ]
                                .into_iter()
                                .filter_map(move |dir| {
                                    let next = pos + dir;
                                    (beside_ladder(pos) && is_walkable(&next)).then(
                                        || {
                                            let next_node = Node {
                                                pos: next,
                                                last_dir: dir.xy(),
                                                last_dir_count: 0,
                                            };
                                            (next_node, 1.5)
                                        },
                                    )
                                }),
                            )
                    })
                    .into_iter()
                    .flatten(),
            )
        // .chain(
        //     DIAGONALS
        //         .iter()
        //         .filter(move |(dir, [a, b])| {
        //             is_walkable(&(pos + *dir)) && walkable[*a] &&
        // walkable[*b]         })
        //         .map(move |(dir, _)| pos + *dir),
        // )
    };

    let satisfied = |node: &Node| node.pos == end;

    if astar
        .as_ref()
        .is_some_and(|(_, start)| start.distance_squared(startf) > 4.0)
    {
        *astar = None;
    }
    let max_iters = match path_length {
        PathLength::Small => 500,
        PathLength::Medium => 5000,
        PathLength::Long => 25_000,
        PathLength::Longest => 75_000,
    };

    let (astar, _) = astar.get_or_insert_with(|| {
        (
            Astar::new(
                max_iters,
                Node {
                    pos: start,
                    last_dir: Vec2::zero(),
                    last_dir_count: 0,
                },
                FxBuildHasher::default(),
            ),
            startf,
        )
    });

    astar.set_max_iters(max_iters);

    // bastion ledger #180: expansions are counted as a delta around the
    // poll — correct across resumed searches (a retained astar keeps its
    // running total) and fresh creations (total starts at zero).
    let consumed_before = astar.iters_consumed();
    let path_result = astar.poll(
        match path_length {
            PathLength::Small => 250,
            PathLength::Medium => 400,
            PathLength::Long => 500,
            PathLength::Longest => 750,
        },
        heuristic,
        neighbors,
        satisfied,
    );
    let consumed = (astar.iters_consumed() - consumed_before) as u64;

    (
        path_result.map(|path| path.nodes.into_iter().map(|n| n.pos).collect()),
        consumed,
    )
}

/// bastion (FR15 fix-1): compute a COMPLETE path ONCE — no per-call budget,
/// no reset-on-move — for the bastion job-travel WAYPOINT COMMIT. The
/// incremental machinery above resets its search whenever the agent moves
/// >2 blocks from the search anchor (`start.distance_squared(startf) > 4.0`),
/// which is exactly the beeline-then-bob at corners (FR15 Bug A): the agent
/// moves, the search restarts, the turn is never found. This wrapper drives
/// the SAME [`find_path`] (one pathfinder, identical walkable/transition
/// semantics — B17 one-implementation) with a fresh one-shot search polled
/// to completion. Bounded by [`PathLength::Medium`] (5000 iters total —
/// tight-dig paths are short); returns `None` on unreachable/exhausted (the
/// caller falls back to the plain steer + watchdog pipeline, unchanged).
pub fn bastion_full_path<V>(
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    traversal_cfg: &TraversalConfig,
) -> Option<Vec<Vec3<i32>>>
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    match bastion_full_path_ext(vol, startf, endf, traversal_cfg, PathLength::Medium) {
        FullPathOutcome::Path(nodes) => Some(nodes),
        FullPathOutcome::Unreachable | FullPathOutcome::BudgetExhausted => None,
    }
}

/// bastion (ITEM 29, wall-detour row): why the two failure arms are kept
/// APART here. [`bastion_full_path`] collapses `PathResult::None` and
/// `PathResult::Exhausted` into one `None`, so a search that merely ran out
/// of this tier's `max_iters` is indistinguishable from a frontier that
/// emptied — a budget failure recorded as a geography fact. The colony-side
/// detour has to count those separately: `Unreachable` is a POSITIVE proof
/// that no route exists at this tier's admission rules and the colonist
/// should stop paying for detours, while `BudgetExhausted` says only that
/// the tier was too small and the next rung is worth trying.
pub enum FullPathOutcome {
    Path(Vec<Vec3<i32>>),
    /// The frontier emptied: `find_path` proved no route under this
    /// config's `walkable`/`neighbors` admission.
    Unreachable,
    /// The tier's cumulative `max_iters` ran out before either terminal
    /// result. Says NOTHING about reachability.
    BudgetExhausted,
}

/// bastion (ITEM 29): [`bastion_full_path`] with the tier as a parameter and
/// the failure arms separated. Same one-shot drive of the SAME [`find_path`]
/// (B17 one-implementation), so walkable/transition semantics cannot drift
/// between the incremental scheduler path and this one.
pub fn bastion_full_path_ext<V>(
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    traversal_cfg: &TraversalConfig,
    path_length: PathLength,
) -> FullPathOutcome
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let mut astar = None;
    // `find_path` polls a bounded slice per call (250/400/500/750) against
    // the tier's own cumulative `max_iters` (500/5000/25_000/75_000), so
    // ceil(max_iters / slice) + 1 calls always reaches a terminal result.
    // Kept as the same conservative over-estimates the Medium case has
    // always used (16 where 14 would do) rather than tightened here — a
    // too-small bound would silently report `BudgetExhausted` on a search
    // that was one poll from a route.
    let calls = match path_length {
        PathLength::Small => 4,
        PathLength::Medium => 16,
        PathLength::Long => 52,
        PathLength::Longest => 102,
    };
    for _ in 0..calls {
        match find_path(
            &mut astar,
            vol,
            startf,
            endf,
            traversal_cfg,
            path_length,
            None,
        )
        .0
        {
            PathResult::Pending => continue,
            PathResult::Path(path, _cost) => {
                return FullPathOutcome::Path(path.nodes.into_iter().collect());
            },
            PathResult::None(_) => return FullPathOutcome::Unreachable,
            PathResult::Exhausted(_) => return FullPathOutcome::BudgetExhausted,
        }
    }
    FullPathOutcome::BudgetExhausted
}
// Enable when airbraking/sensible flight is a thing
#[cfg(feature = "rrt_pathfinding")]
fn find_air_path<V>(
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    traversal_cfg: &TraversalConfig,
) -> (Option<Path<Vec3<i32>>>, bool)
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    let radius = traversal_cfg.node_tolerance;
    let total_dist_sqrd = startf.distance_squared(endf);
    // First check if a straight line path works
    if vol
        .ray(startf + Vec3::unit_z(), endf + Vec3::unit_z())
        .until(Block::is_opaque)
        .cast()
        .0
        .powi(2)
        >= total_dist_sqrd
    {
        let path = vec![endf.map(|e| e.floor() as i32)];
        let connect = true;
        (Some(path.into_iter().collect()), connect)
    // Else use RRTs
    } else {
        let is_traversable = |start: &Vec3<f32>, end: &Vec3<f32>| {
            vol.ray(*start, *end)
                .until(Block::is_solid)
                .cast()
                .0
                .powi(2)
                > (*start).distance_squared(*end)
            //vol.get(*pos).ok().copied().unwrap_or_else(Block::empty).
            // is_fluid();
        };
        informed_rrt_connect(vol, startf, endf, is_traversable, radius)
    }
}

/// Attempts to find a path from a start to the end using an informed
/// RRT-Connect algorithm. A point is sampled from a bounding spheroid
/// between the start and end. Two separate rapidly exploring random
/// trees extend toward the sampled point. Nodes are stored in k-d trees
/// for quicker nearest node calculations. Points are sampled until the
/// trees connect. A final path is then reconstructed from the nodes.
/// This pathfinding algorithm is more appropriate for 3D pathfinding
/// with wider gaps, such as flying through a forest than for terrain
/// with narrow gaps, such as navigating a maze.
/// Returns a path and whether that path is complete or not.
#[cfg(feature = "rrt_pathfinding")]
fn informed_rrt_connect<V>(
    vol: &V,
    startf: Vec3<f32>,
    endf: Vec3<f32>,
    is_valid_edge: impl Fn(&Vec3<f32>, &Vec3<f32>) -> bool,
    radius: f32,
) -> (Option<Path<Vec3<i32>>>, bool)
where
    V: BaseVol<Vox = Block> + ReadVol,
{
    const MAX_POINTS: usize = 7000;
    // RNG-P3-016 (determinism audit): ONE deterministic stream for the whole
    // RRT search, keyed by the search's intrinsic identity (start, end) —
    // replaces ambient OS-entropy draws (spheroid sampler + parent re-pick),
    // so an identical query explores identically.
    let mut rrt_rng = ChaCha8Rng::seed_from_u64(
        (startf.x.to_bits() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (startf.y.to_bits() as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ (startf.z.to_bits() as u64).wrapping_mul(0x1656_67B1_9E37_79F9)
            ^ (endf.x.to_bits() as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
            ^ (((endf.y.to_bits() as u64) << 32) | endf.z.to_bits() as u64)
            ^ 0x4247_0016,
    );
    let mut path = Vec::new();

    // Each tree has a vector of nodes
    let mut node_index1: usize = 0;
    let mut node_index2: usize = 0;
    let mut nodes1 = Vec::new();
    let mut nodes2 = Vec::new();

    // The parents hashmap stores nodes and their parent nodes as pairs to
    // retrace the complete path once the two RRTs connect
    let mut parents1 = HashMap::new();
    let mut parents2 = HashMap::new();

    // The path vector stores the path from the appropriate terminal to the
    // connecting node or vice versa
    let mut path1 = Vec::new();
    let mut path2 = Vec::new();

    // K-d trees are used to find the closest nodes rapidly
    let mut kdtree1: KdTree<f32, usize, 3, 32, u32> = KdTree::with_capacity(MAX_POINTS);
    let mut kdtree2: KdTree<f32, usize, 3, 32, u32> = KdTree::with_capacity(MAX_POINTS);

    // Add the start as the first node of the first k-d tree
    kdtree1.add(&[startf.x, startf.y, startf.z], node_index1);
    nodes1.push(startf);
    node_index1 += 1;

    // Add the end as the first node of the second k-d tree
    kdtree2.add(&[endf.x, endf.y, endf.z], node_index2);
    nodes2.push(endf);
    node_index2 += 1;

    let mut connection1_idx = 0;
    let mut connection2_idx = 0;

    let mut connect = false;

    // Scalar non-dimensional value that is proportional to the size of the
    // sample spheroid volume. This increases in value until a path is found.
    let mut search_parameter = 0.01;

    // Maximum of MAX_POINTS iterations
    for _i in 0..MAX_POINTS {
        if connect {
            break;
        }

        // Sample a point on the bounding spheroid
        let (sampled_point1, sampled_point2) = {
            let point = point_on_prolate_spheroid(startf, endf, search_parameter, &mut rrt_rng);
            (point, point)
        };

        // Find the nearest nodes to the the sampled point
        let nearest_index1 = kdtree1
            .nearest_one::<SquaredEuclidean>(&[
                sampled_point1.x,
                sampled_point1.y,
                sampled_point1.z,
            ])
            .item;
        let nearest_index2 = kdtree2
            .nearest_one::<SquaredEuclidean>(&[
                sampled_point2.x,
                sampled_point2.y,
                sampled_point2.z,
            ])
            .item;
        let nearest1 = nodes1[nearest_index1];
        let nearest2 = nodes2[nearest_index2];

        // Extend toward the sampled point from the nearest node of each tree
        let new_point1 = nearest1 + (sampled_point1 - nearest1).normalized().map(|a| a * radius);
        let new_point2 = nearest2 + (sampled_point2 - nearest2).normalized().map(|a| a * radius);

        // Ensure the new nodes are valid/traversable
        if is_valid_edge(&nearest1, &new_point1) {
            kdtree1.add(&[new_point1.x, new_point1.y, new_point1.z], node_index1);
            nodes1.push(new_point1);
            parents1.insert(node_index1, nearest_index1);
            node_index1 += 1;
            // Check if the trees connect
            let NearestNeighbour {
                distance: check,
                item: index,
            } = kdtree2.nearest_one::<SquaredEuclidean>(&[
                new_point1.x,
                new_point1.y,
                new_point1.z,
            ]);
            if check < radius {
                let connection = nodes2[index];
                connection2_idx = index;
                nodes1.push(connection);
                connection1_idx = nodes1.len() - 1;
                parents1.insert(node_index1, node_index1 - 1);
                connect = true;
            }
        }

        // Repeat the validity check for the second tree
        if is_valid_edge(&nearest2, &new_point2) {
            kdtree2.add(&[new_point2.x, new_point2.y, new_point1.z], node_index2);
            nodes2.push(new_point2);
            parents2.insert(node_index2, nearest_index2);
            node_index2 += 1;
            // Again check for a connection
            let NearestNeighbour {
                distance: check,
                item: index,
            } = kdtree1.nearest_one::<SquaredEuclidean>(&[
                new_point2.x,
                new_point2.y,
                new_point1.z,
            ]);
            if check < radius {
                let connection = nodes1[index];
                connection1_idx = index;
                nodes2.push(connection);
                connection2_idx = nodes2.len() - 1;
                parents2.insert(node_index2, node_index2 - 1);
                connect = true;
            }
        }
        // Increase the search parameter to widen the sample volume
        search_parameter += 0.02;
    }

    if connect {
        // Construct paths from the connection node to the start and end
        let mut current_node_index1 = connection1_idx;
        while current_node_index1 > 0 {
            current_node_index1 = *parents1.get(&current_node_index1).unwrap_or(&0);
            path1.push(nodes1[current_node_index1].map(|e| e.floor() as i32));
        }
        let mut current_node_index2 = connection2_idx;
        while current_node_index2 > 0 {
            current_node_index2 = *parents2.get(&current_node_index2).unwrap_or(&0);
            path2.push(nodes2[current_node_index2].map(|e| e.floor() as i32));
        }
        // Join the two paths together in the proper order and remove duplicates
        path1.pop();
        path1.reverse();
        path.append(&mut path1);
        path.append(&mut path2);
        path.dedup();
    } else {
        // If the trees did not connect, construct a path from the start to
        // the closest node to the end
        let mut current_node_index1 = kdtree1
            .nearest_one::<SquaredEuclidean>(&[endf.x, endf.y, endf.z])
            .item;
        // Attempt to pick a node other than the start node
        for _i in 0..3 {
            if current_node_index1 == 0
                || nodes1[current_node_index1].distance_squared(startf) < 4.0
            {
                // RNG-P3-016: sort the candidate set (HashMap::values order
                // is process-seeded) and draw from the search's keyed stream.
                let mut candidates: Vec<_> = parents1.values().copied().collect();
                candidates.sort_unstable();
                if let Some(index) = candidates.iter().choose(&mut rrt_rng) {
                    current_node_index1 = *index;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        path1.push(nodes1[current_node_index1].map(|e| e.floor() as i32));
        // Construct the path
        while current_node_index1 != 0 && nodes1[current_node_index1].distance_squared(startf) > 4.0
        {
            current_node_index1 = *parents1.get(&current_node_index1).unwrap_or(&0);
            path1.push(nodes1[current_node_index1].map(|e| e.floor() as i32));
        }

        path1.reverse();
        path.append(&mut path1);
    }
    let mut new_path = Vec::new();
    let mut node = path[0];
    new_path.push(node);
    let mut node_idx = 0;
    let num_nodes = path.len();
    let end = path[num_nodes - 1];
    while node != end {
        let next_idx = if node_idx + 4 > num_nodes - 1 {
            num_nodes - 1
        } else {
            node_idx + 4
        };
        let next_node = path[next_idx];
        let start_pos = node.map(|e| e as f32 + 0.5);
        let end_pos = next_node.map(|e| e as f32 + 0.5);
        if vol
            .ray(start_pos, end_pos)
            .until(Block::is_solid)
            .cast()
            .0
            .powi(2)
            > (start_pos).distance_squared(end_pos)
        {
            node_idx = next_idx;
            new_path.push(next_node);
        } else {
            node_idx += 1;
        }
        node = path[node_idx];
    }
    path = new_path;
    (Some(path.into_iter().collect()), connect)
}

// bastion (B5.8): graph-level tests for the vertical-mobility edges — a
// mock volume pins `find_path` behavior in milliseconds instead of full sim
// runs (the b58 scenario iterations that motivated these took ~8 min each).
#[cfg(test)]
mod bastion_vertical_tests {
    use super::*;
    use crate::terrain::{BlockKind, SpriteKind};
    use hashbrown::HashMap as StdHashMap;
    use vek::Rgb;

    pub(super) struct MockVol {
        blocks: StdHashMap<Vec3<i32>, Block>,
        air: Block,
    }

    impl MockVol {
        pub(super) fn from_parts(blocks: StdHashMap<Vec3<i32>, Block>, air: Block) -> Self {
            Self { blocks, air }
        }
    }

    impl BaseVol for MockVol {
        type Error = ();
        type Vox = Block;
    }

    impl ReadVol for MockVol {
        fn get(&self, pos: Vec3<i32>) -> Result<&Block, ()> {
            Ok(self.blocks.get(&pos).unwrap_or(&self.air))
        }
    }

    /// Flat ground (solid z ≤ 0), a 4-high wall+plateau for x ≥ 10, and —
    /// optionally — a ladder column against the wall face at (9, 0) with
    /// rungs z 1..=5 (one above the ledge, per the dismount rule). Mirrors
    /// the b58 part-(c) geometry.
    pub(super) fn wall_world(with_ladder: bool) -> MockVol {
        let rock = Block::new(BlockKind::Rock, Rgb::new(120, 120, 120));
        let mut blocks = StdHashMap::new();
        for x in -2..=20 {
            for y in -6..=6 {
                blocks.insert(Vec3::new(x, y, 0), rock);
            }
        }
        for x in 10..=20 {
            for y in -6..=6 {
                for z in 1..=4 {
                    blocks.insert(Vec3::new(x, y, z), rock);
                }
            }
        }
        if with_ladder {
            for z in 1..=5 {
                blocks.insert(Vec3::new(9, 0, z), Block::air(SpriteKind::Ladder));
            }
        }
        MockVol {
            blocks,
            air: Block::empty(),
        }
    }

    pub(super) fn worker_cfg() -> TraversalConfig {
        TraversalConfig {
            node_tolerance: 1.5,
            slow_factor: 0.0,
            on_ground: true,
            in_liquid: false,
            min_tgt_dist: 1.0,
            can_climb: true,
            scramble_reach: 3,
            can_fly: false,
            vectored_propulsion: false,
            is_target_loaded: true,
            search_allowed: true,
        }
    }

    #[test]
    fn chaser_stuck_history_rebase_requires_fresh_samples() {
        let mut chaser = Chaser::default();
        chaser.set_deterministic_seed(Some(21));
        let pos = Vec3::zero();
        let bearing = Vec3::unit_x();

        assert!(!chaser.stuck_check(pos, bearing, 1.0, &Time(0.0)).2);
        assert!(!chaser.stuck_check(pos, bearing, 1.0, &Time(0.6)).2);
        assert!(chaser.stuck_check(pos, bearing, 1.0, &Time(1.2)).2);

        chaser.rebase_stuck_history();
        assert!(chaser.recent_states.is_empty());
        assert!(
            !chaser.stuck_check(pos, bearing, 1.0, &Time(1.3)).2,
            "replacement target must accumulate fresh history before stuck recovery"
        );
        assert_eq!(chaser.recent_states.len(), 1);
        assert!(!chaser.stuck_check(pos, bearing, 1.0, &Time(1.9)).2);
        assert!(
            chaser.stuck_check(pos, bearing, 1.0, &Time(2.5)).2,
            "genuine no-progress must reactivate stuck recovery after fresh history"
        );
    }

    #[test]
    fn route_local_endpoint_tolerance_keeps_normal_done_band_pending() {
        let vol = wall_world(false);
        let endpoint = Vec3::new(0, 0, 1);
        // The actor is 0.99 beyond the endpoint cell boundary: ordinary
        // tolerance 1.5 completes, while the emergency corridor contract
        // 0.75 must keep producing target-directed traversal.
        let pos = Vec3::new(1.99, 0.5, 1.0);
        let path = || Path::from_iter([endpoint]);

        let mut ordinary = Route::from(path());
        assert!(matches!(
            ordinary.traverse(&vol, pos, Vec3::zero(), &worker_cfg()),
            Err(TraverseStop::Done)
        ));

        let mut corridor = Route::from(path());
        let strict = TraversalConfig {
            node_tolerance: 0.75,
            ..worker_cfg()
        };
        let Ok((bearing, _)) = corridor.traverse(&vol, pos, Vec3::zero(), &strict) else {
            panic!("strict endpoint must remain pending in the ordinary Done band");
        };
        assert!(bearing.dot((Vec3::new(0.5, 0.5, 1.0) - pos).normalized()) > 0.0);
        assert_eq!(corridor.next_idx(), 0, "strict cursor advanced above 0.75");
    }

    /// Poll to completion — `find_path` yields `Pending` every ~400
    /// iterations (the Chaser normally resumes it across ticks).
    fn route_to(vol: &MockVol, cfg: &TraversalConfig, end: Vec3<f32>) -> PathResult<Vec3<i32>> {
        let mut astar = None;
        for _ in 0..64 {
            match find_path(
                &mut astar,
                vol,
                Vec3::new(4.5, 0.5, 1.0),
                end,
                cfg,
                PathLength::Medium,
                None,
            )
            .0
            {
                PathResult::Pending => continue,
                r => return r,
            }
        }
        panic!("pathfinding never completed (still Pending after 64 polls)");
    }

    fn route(vol: &MockVol, cfg: &TraversalConfig) -> PathResult<Vec3<i32>> {
        route_to(vol, cfg, Vec3::new(12.5, 0.5, 5.0))
    }

    #[test]
    fn ladder_column_routes_up_a_tall_wall() {
        let vol = wall_world(true);
        let r = route(&vol, &worker_cfg());
        let PathResult::Path(path, _cost) = r else {
            panic!("no route via the ladder (mount/climb/dismount edges broken)");
        };
        // The route must actually use the climb line beside the ladder —
        // some node adjacent to the ladder column above ground level.
        assert!(
            path.iter()
                .any(|n| n.z > 2 && (n.xy() - Vec2::new(9, 0)).map(|e: i32| e.abs()).sum() <= 1),
            "path exists but skips the ladder line: {:?}",
            path.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn tall_wall_without_ladder_has_no_route() {
        // 4 blocks exceeds scramble reach (3): with no ladder there must be
        // NO full path — if this ever passes, an edge leaked past the reach
        // model.
        let vol = wall_world(false);
        assert!(
            !matches!(route(&vol, &worker_cfg()), PathResult::Path(..)),
            "a 4-high wall was routed without a ladder (reach model leak)"
        );
    }

    #[test]
    fn scramble_reach_gates_three_up_edges() {
        // A 3-high wall IS routable at reach 3 (the scramble edge) and NOT
        // at reach 2 (novice).
        let rock = Block::new(BlockKind::Rock, Rgb::new(120, 120, 120));
        let mut blocks = StdHashMap::new();
        for x in -2..=20 {
            for y in -6..=6 {
                blocks.insert(Vec3::new(x, y, 0), rock);
            }
        }
        for x in 10..=20 {
            for y in -6..=6 {
                for z in 1..=3 {
                    blocks.insert(Vec3::new(x, y, z), rock);
                }
            }
        }
        let vol = MockVol {
            blocks,
            air: Block::empty(),
        };
        let skilled = route_to(&vol, &worker_cfg(), Vec3::new(12.5, 0.5, 4.0));
        assert!(
            matches!(skilled, PathResult::Path(..)),
            "reach 3 must route a 3-up scramble"
        );
        let novice = TraversalConfig {
            scramble_reach: 2,
            ..worker_cfg()
        };
        let blocked = route_to(&vol, &novice, Vec3::new(12.5, 0.5, 4.0));
        assert!(
            !matches!(blocked, PathResult::Path(..)),
            "reach 2 must NOT route a 3-up face"
        );
    }
}

/// Returns a random point within a radially symmetrical ellipsoid with given
/// foci and a `search parameter` to determine the size of the ellipse beyond
/// the foci. Technically the point is within a prolate spheroid translated and
/// rotated to the proper place in cartesian space.
/// The search_parameter is a float that relates to the length of the string for
/// a two dimensional ellipse or the size of the ellipse beyond the foci. In
/// this case that analogy still holds as the ellipse is radially symmetrical
/// along the axis between the foci. The value of the search parameter must be
/// greater than zero. In order to increase the sample area, the
/// search_parameter should be increased linearly as the search continues.
#[cfg(feature = "rrt_pathfinding")]
pub fn point_on_prolate_spheroid(
    focus1: Vec3<f32>,
    focus2: Vec3<f32>,
    search_parameter: f32,
    // RNG-P3-016: the caller's stream, not ambient OS entropy.
    rng: &mut impl RngExt,
) -> Vec3<f32> {
    // Uniform distribution
    let range = Uniform::new(0.0, 1.0).unwrap();

    // Midpoint is used as the local origin
    let midpoint = 0.5 * (focus1 + focus2);
    // Radius between the start and end of the path
    let radius: f32 = focus1.distance(focus2);
    // The linear eccentricity of an ellipse is the distance from the origin to a
    // focus A prolate spheroid is a half-ellipse rotated for a full revolution
    // which is why ellipse variables are used frequently in this function
    let linear_eccentricity: f32 = 0.5 * radius;

    // For an ellipsoid, three variables determine the shape: a, b, and c.
    // These are the distance from the center/origin to the surface on the
    // x, y, and z axes, respectively.
    // For a prolate spheroid a and b are equal.
    // c is determined by adding the search parameter to the linear eccentricity.
    // As the search parameter increases the size of the spheroid increases
    let c: f32 = linear_eccentricity + search_parameter;
    // The width is calculated to prioritize increasing width over length of
    // the ellipsoid
    let a: f32 = (c.powi(2) - linear_eccentricity.powi(2)).powf(0.5);
    // The width should be the same in both the x and y directions
    let b: f32 = a;

    // The parametric spherical equation for an ellipsoid measuring from the
    // center point is as follows:
    // x = a * cos(theta) * cos(lambda)
    // y = b * cos(theta) * sin(lambda)
    // z = c * sin(theta)
    //
    // where     -0.5 * PI <= theta <= 0.5 * PI
    // and       0.0 <= lambda < 2.0 * PI
    //
    // Select these two angles using the uniform distribution defined at the
    // beginning of the function from 0.0 to 1.0
    let rtheta: f32 = PI * range.sample(&mut *rng) - 0.5 * PI;
    let lambda: f32 = 2.0 * PI * range.sample(&mut *rng);
    // Select a point on the surface of the ellipsoid
    let point = Vec3::new(
        a * rtheta.cos() * lambda.cos(),
        b * rtheta.cos() * lambda.sin(),
        c * rtheta.sin(),
    );
    // NOTE: Theoretically we should sample a point within the spheroid
    // requiring selecting a point along the radius. In my tests selecting
    // a point *on the surface* of the spheroid results in sampling that is
    // "good enough". The following code is commented out to reduce expense.
    //let surface_point = Vec3::new(a * rtheta.cos() * lambda.cos(), b *
    // rtheta.cos() * lambda.sin(), c * rtheta.sin()); let magnitude =
    // surface_point.magnitude(); let direction = surface_point.normalized();
    //// Randomly select a point along the vector to the previously selected surface
    //// point using the uniform distribution
    //let point = magnitude * range.sample(&mut rng) * direction;

    // Now that a point has been selected in local space, it must be rotated and
    // translated into global coordinates
    // NOTE: Don't rotate about the z axis as the point is already randomly
    // selected about the z axis
    //let dx = focus2.x - focus1.x;
    //let dy = focus2.y - focus1.y;
    let dz = focus2.z - focus1.z;
    // Phi and theta are the angles from the x axis in the x-y plane and from
    // the z axis, respectively. (As found in spherical coordinates)
    // These angles are used to rotate the random point in the spheroid about
    // the local origin
    //
    // Rotate about z axis by phi
    //let phi: f32 = if dx.abs() > 0.0 {
    //    (dy / dx).atan()
    //} else {
    //    0.5 * PI
    //};
    // This is unnecessary as rtheta is randomly selected between 0.0 and 2.0 * PI
    // let rot_z_mat = Mat3::new(phi.cos(), -1.0 * phi.sin(), 0.0, phi.sin(),
    // phi.cos(), 0.0, 0.0, 0.0, 1.0);

    // Rotate about perpendicular vector in the xy plane by theta
    let theta: f32 = if radius > 0.0 {
        (dz / radius).acos()
    } else {
        0.0
    };
    // Vector from focus1 to focus2
    let r_vec = focus2 - focus1;
    // Perpendicular vector in xy plane
    let perp_vec = Vec3::new(-1.0 * r_vec.y, r_vec.x, 0.0).normalized();
    let l = perp_vec.x;
    let m = perp_vec.y;
    let n = perp_vec.z;
    // Rotation matrix for rotation about a vector
    let rot_2_mat = Mat3::new(
        l * l * (1.0 - theta.cos()),
        m * l * (1.0 - theta.cos()) - n * theta.sin(),
        n * l * (1.0 - theta.cos()) + m * theta.sin(),
        l * m * (1.0 - theta.cos()) + n * theta.sin(),
        m * m * (1.0 - theta.cos()) + theta.cos(),
        n * m * (1.0 - theta.cos()) - l * theta.sin(),
        l * n * (1.0 - theta.cos()) - m * theta.sin(),
        m * n * (1.0 - theta.cos()) + l * theta.sin(),
        n * n * (1.0 - theta.cos()) + theta.cos(),
    );

    // Get the global coordinates of the point by rotating and adding the origin
    // rot_z_mat is unneeded due to the random rotation defined by lambda
    // let global_coords = midpoint + rot_2_mat * (rot_z_mat * point);
    midpoint + rot_2_mat * point
}


// bastion ledger #178 falsifier: a retained (Pending) search MUST NOT survive
// a traversal-profile change — continuing a no-climb frontier under a climb
// config yields a stale-profile result that diverges from a fresh search.
#[cfg(test)]
mod ledger_178_tests {
    use super::{*, bastion_vertical_tests::{MockVol, wall_world, worker_cfg}};

    /// A volume with a genuinely UNLOADED band (get() errors for x in the
    /// band): `is_target_loaded=false` treats unloaded cells as optimistically
    /// walkable (see `walkable`), `true` forbids them — the profile flag that
    /// changes node ADMISSION itself.
    pub(super) struct BandUnloadedVol {
        pub inner: MockVol,
        pub unloaded_x: core::ops::Range<i32>,
    }

    impl BaseVol for BandUnloadedVol {
        type Error = ();
        type Vox = Block;
    }

    impl ReadVol for BandUnloadedVol {
        fn get(&self, pos: Vec3<i32>) -> Result<&Block, ()> {
            if self.unloaded_x.contains(&pos.x) {
                Err(())
            } else {
                self.inner.get(pos)
            }
        }
    }

    /// THE SHARP FALSIFIER (iteration 2 — the broad profile-change version
    /// went GREEN on unfixed code: every cfg read happens through CURRENT
    /// closures per poll, and ENGOPT2's reopen makes stale-frontier
    /// continuations self-heal for geometric costs; documented, kept below).
    /// The RESIDUAL hole is ADMISSION staleness: nodes admitted under
    /// loaded=false optimism sit in the retained visited/frontier with live
    /// g-values; continuing under loaded=true pops them and routes THROUGH
    /// terrain the current profile forbids. A fresh loaded=true search
    /// cannot cross the band.
    #[test]
    fn ledger_178_loaded_flip_must_not_route_through_forbidden_band() {
        let vol = BandUnloadedVol {
            inner: wall_world(false),
            unloaded_x: 4..7,
        };
        let start = Vec3::new(0.0, 0.5, 1.0);
        let tgt = Vec3::new(8.5, 0.5, 1.0);
        let loaded = TraversalConfig {
            is_target_loaded: true,
            can_climb: false,
            scramble_reach: 0,
            ..worker_cfg()
        };
        let optimistic = TraversalConfig {
            is_target_loaded: false,
            ..loaded
        };

        // Seed a retained search under OPTIMISM (one budgeted step —
        // Pending or routed-through-band), then continue under loaded=true.
        let mut stale = Chaser::default();
        stale.search_step(&vol, start, tgt, &optimistic);
        for _ in 0..400 {
            stale.search_step(&vol, start, tgt, &loaded);
            if !stale.needs_search() {
                break;
            }
        }
        if let Some(nodes) = stale.route_nodes() {
            assert!(
                !nodes.iter().any(|n| (4..7).contains(&n.x)),
                "a loaded=true continuation must not route through the band its profile                  forbids (stale-ADMISSION carryover from the optimistic retained search):                  {nodes:?}"
            );
        }
    }

    #[test]
    fn ledger_178_profile_change_must_invalidate_retained_search() {
        let vol = wall_world(false);
        let start = Vec3::new(0.0, 0.5, 1.0);
        let tgt = Vec3::new(12.5, 0.5, 5.0);
        let climb = worker_cfg();
        let no_climb = TraversalConfig {
            can_climb: false,
            scramble_reach: 0,
            ..worker_cfg()
        };

        // Reference: a FRESH search under the climb profile.
        let mut fresh = Chaser::default();
        for _ in 0..400 {
            fresh.search_step(&vol, start, tgt, &climb);
            if !fresh.needs_search() {
                break;
            }
        }
        let reference = fresh.route_nodes().expect("fresh climb search must route");

        // Stale-continuation: seed a retained search under NO-CLIMB, then
        // continue under CLIMB. Pre-#178 the retained frontier carries the
        // no-climb constraint forward; post-#178 the profile change drops it
        // and the result equals the fresh reference.
        let mut stale = Chaser::default();
        stale.search_step(&vol, start, tgt, &no_climb);
        for _ in 0..400 {
            stale.search_step(&vol, start, tgt, &climb);
            if !stale.needs_search() {
                break;
            }
        }
        let continued = stale.route_nodes().expect("continued search must route");
        assert_eq!(
            continued, reference,
            "a traversal-profile change must invalidate the retained search (ledger #178)"
        );
    }
}

// bastion ledger #180: the scheduler debits ACTUAL search work (poll
// deltas), not its planned estimate — these pin the delta semantics the
// debit relies on (trivial searches are cheap, no-op grants are free,
// exhausted slices bill the full slice, and actual never exceeds planned).
#[cfg(test)]
mod ledger_180_tests {
    use super::{
        *,
        bastion_vertical_tests::{wall_world, worker_cfg},
    };

    #[test]
    fn ledger_180_actual_consumption_tracks_real_work() {
        let vol = wall_world(false);
        let cfg = TraversalConfig {
            search_allowed: false,
            ..worker_cfg()
        };
        let pos = Vec3::new(0.0, 0.5, 1.0);
        let tgt = Vec3::new(3.5, 0.5, 1.0);

        // A trivial 3-cell route must cost far less than the planned slice.
        let mut chaser = Chaser::default();
        chaser.search_step(&vol, pos, tgt, &cfg);
        let trivial = chaser.last_search_consumed();
        assert!(trivial > 0, "a real search consumes at least one expansion");
        assert!(
            trivial < chaser.planned_iters(),
            "trivial search must undercut the planned estimate: {trivial} vs {}",
            chaser.planned_iters()
        );

        // A granted step that races an existing route is a no-op — zero.
        chaser.search_step(&vol, pos, tgt, &cfg);
        assert_eq!(
            chaser.last_search_consumed(),
            0,
            "the no-op grant arm must never re-bill a stale delta"
        );

        // A slice that ends mid-search (Pending) bills exactly the slice.
        let mut long = Chaser::default();
        let far_tgt = Vec3::new(12.5, 0.5, 5.0);
        long.search_step(&vol, Vec3::new(0.0, 0.5, 1.0), far_tgt, &TraversalConfig {
            can_climb: false,
            scramble_reach: 0,
            ..cfg
        });
        let (_, state) = long.state();
        if state == PathState::Pending {
            assert_eq!(long.last_search_consumed(), 250, "a full slice bills 250");
        }
        assert!(
            long.last_search_consumed() <= long.planned_iters(),
            "actual must never exceed planned (the cap-holding invariant)"
        );
    }
}

// bastion ledger #179: `find_path`'s edge-cost policy is a function of
// `path_length` (Small discourages wall-adjacent cells via `edge_cost`;
// Medium+ does not). The Exhausted-upgrade ladder retains the search across
// the Small→Medium boundary (`set_max_iters` just raises the cap), so every
// visited g-value keeps Small's wall penalties baked in — a mixed-policy
// search. The live window is exactly the boxed-in case: an exhausted search
// whose best-progress node is within the anchor tolerance (otherwise the
// agent walks the partial route and the >2-block anchor-move wipe restarts
// the search anyway).
#[cfg(test)]
mod ledger_179_tests {
    use super::{
        *,
        bastion_vertical_tests::{MockVol, worker_cfg},
    };
    use crate::terrain::BlockKind;
    use hashbrown::HashMap as StdHashMap;
    use vek::Rgb;

    /// Two-door variant: a DEEP narrow tunnel (2-thick wall, 1-wide slot at
    /// x = 5 — every through-step is wall-flanked and pays Small's
    /// `edge_cost`) vs a wide penalty-free door centered at x = -6, one cell
    /// longer by geometry. Fresh Medium (no edge tax) takes the short
    /// tunnel; a frontier whose tunnel g-values were baked under Small's
    /// tax prefers the wide door — the mixed-cost-model flip.
    fn two_door_world() -> MockVol {
        let rock = Block::new(BlockKind::Rock, Rgb::new(120, 120, 120));
        let mut blocks = StdHashMap::new();
        for x in -10..=10 {
            for y in -3..=12 {
                blocks.insert(Vec3::new(x, y, 0), rock);
            }
        }
        for x in -10..=10 {
            for y in [4, 5] {
                // Narrow tunnel slot at x = 5; wide door at x in [-7, -5].
                if x == 5 || (-7..=-5).contains(&x) {
                    continue;
                }
                for z in 1..=4 {
                    blocks.insert(Vec3::new(x, y, z), rock);
                }
            }
        }
        MockVol::from_parts(blocks, Block::empty())
    }

    fn trap_cfg() -> TraversalConfig {
        TraversalConfig {
            can_climb: false,
            scramble_reach: 0,
            search_allowed: false,
            ..worker_cfg()
        }
    }

    /// THE FALSIFIER: the continued (Small-exhausted → Medium) route must
    /// equal a fresh Medium-policy search — a mixed-cost-model frontier is
    /// the defect (search-epoch invalidation; LPA*-style repair is
    /// deliberately not attempted).
    #[test]
    /// ★ A SCRAMBLE MUST COST MORE THAN WALKING ROUND THE HOUSE.
    ///
    /// Ben: "a colonist scaling a house wall to reach a crate is a bug even
    /// when it works." This pins the RELATIONSHIP that makes that true, not
    /// the constant, because every weight in `transition` is relative to a
    /// flat step and a flat step is NOT 1.0 -- it is 3.0, since
    /// `(dz+1).max(0)*2.0` charges 2.0 even when dz is zero. A test on the
    /// number alone would still pass if someone changed the flat cost and
    /// silently made roofs cheap again.
    ///
    /// BOTH DIRECTIONS, because a surcharge that refuses every vertical move
    /// also stops the bug reproducing: it must beat the detour AND must still
    /// leave stairs and ladders cheaper than it.
    #[test]
    fn a_scramble_must_cost_more_than_rounding_a_house() {
        // The cost model, reproduced from `transition` in `find_path`. Kept in
        // one expression so a change there that this does not follow shows up
        // as a failure rather than as a stale duplicate.
        let step = |dz: i32| {
            1.0 + (dz + 1).max(0) as f32 * 2.0
                + if dz >= 3 { SCRAMBLE_SURCHARGE } else { 0.0 }
        };
        let flat = step(0);
        assert_eq!(flat, 3.0, "a flat move costs 3.0, not 1.0 - every weight below is relative to it");

        // Over a modest house: up, across the roof, down the far side.
        let over_the_roof = step(3) * 2.0 + flat * 8.0 + 6.0;
        // Around the same house: the extra flat steps of the detour.
        let round_the_house = flat * 20.0;
        // ★ THE MARGIN IS THE ASSERTION, NOT THE SIGN. My first version of this
        // test compared them with `>` and PASSED ON THE OLD VALUE -- at 8.0 the
        // roof cost 64 against a 60-step detour, a 6% edge. "Technically more
        // expensive" is exactly the tie that made the router pick roofs
        // whenever the detour ran slightly longer or the target sat on the far
        // side. A one-sided `>` was a green bar with the wrong numbers beside
        // it, and only planting the old constant exposed it.
        //
        // Requiring a HALF-AGAIN margin means a detour can be 50% longer than
        // this idealised one and walking around still wins.
        assert!(
            over_the_roof > round_the_house * 1.5,
            "climbing the wall ({over_the_roof}) must cost HALF AGAIN more than              walking around it ({round_the_house}). Merely costing more is not enough:              at the old surcharge it was 64 vs 60, and the router chose roofs whenever              the real detour was a little longer than the ideal one"
        );

        // ...and the surcharge must not have swallowed legitimate verticality.
        assert!(
            step(1) < step(3) && step(2) < step(3),
            "stairs and jumps must stay cheaper than a scramble: a surcharge that              refuses ALL vertical movement also stops the bug reproducing, and would              pass a one-sided test while stranding colonists at every doorstep"
        );
        assert!(
            step(1) < flat * 3.0,
            "a single step up must stay cheaper than three flat steps, or colonists              will refuse stairs and ladders they are supposed to use"
        );
    }

    fn ledger_179_policy_boundary_must_not_mix_cost_models() {
        let vol = two_door_world();
        let pos = Vec3::new(0.5, 3.5, 1.0);
        let tgt = Vec3::new(0.5, 8.5, 1.0);

        // Reference: a FRESH search that runs under the Medium policy from
        // its first expansion.
        let mut fresh_astar = None;
        let mut reference = None;
        for _ in 0..64 {
            match find_path(
                &mut fresh_astar,
                &vol,
                pos,
                tgt,
                &trap_cfg(),
                PathLength::Medium,
                None,
            )
            .0
            {
                PathResult::Pending => continue,
                PathResult::Path(path, _) => {
                    reference = Some(path.nodes.clone());
                    break;
                },
                PathResult::None(_) => panic!("fresh Medium search must not report no-path"),
                PathResult::Exhausted(_) => panic!("fresh Medium search must not exhaust"),
            }
        }
        let reference = reference.expect("fresh Medium search must terminate");

        // Continued: drive the REAL Chaser ladder — Small exhausts in the
        // trap bowl, the trivial partial route Done-s in place (pos static,
        // within the anchor tolerance), the upgrade arm fires and the
        // retained frontier continues under Medium.
        let mut chaser = Chaser::default();
        chaser.set_deterministic_seed(Some(7));
        let mut continued = None;
        let mut saw_exhausted = false;
        let mut saw_upgrade = false;
        for i in 0..600 {
            let _ = chaser.chase(&vol, pos, Vec3::zero(), tgt, trap_cfg(), &Time(i as f64 * 0.1));
            let (length, state) = chaser.state();
            saw_exhausted |= state == PathState::Exhausted;
            if !saw_upgrade && length > PathLength::Small {
                saw_upgrade = true;
                // PRECONDITION (the falsifier is evidence only if the
                // mechanism engaged where it matters): at the upgrade
                // instant — BEFORE the first Medium poll extends it — the
                // retained visited set must already cover the narrow-tunnel
                // decision surface, i.e. carry Small-taxed g-values there.
                let touched_tunnel = chaser.astar.as_ref().is_some_and(|(astar, _)| {
                    astar
                        .visited()
                        .any(|node| node.pos.x == 5 && (4..=5).contains(&node.pos.y))
                });
                assert!(
                    touched_tunnel,
                    "precondition unmet: the Small phase never reached the tunnel — the \
                     mixed-cost comparison below would be vacuous"
                );
            }
            if chaser.needs_search() {
                chaser.search_step(&vol, pos, tgt, &trap_cfg());
            }
            if chaser.route_is_complete() == Some(true) {
                continued = chaser.route_nodes();
                break;
            }
        }
        assert!(
            saw_exhausted && saw_upgrade,
            "precondition unmet: Small must exhaust (saw_exhausted={saw_exhausted}) and the \
             ladder must upgrade in place (saw_upgrade={saw_upgrade}) for the policy-boundary \
             comparison to be evidence at all"
        );
        let continued = continued.expect(
            "the upgrade ladder must eventually deliver a complete route around the wall",
        );
        assert_eq!(
            continued, reference,
            "a Small-exhausted frontier continued under Medium must equal the fresh \
             Medium-policy search (ledger #179: edge-cost policy epoch)"
        );
    }
}
