use crate::path::Path;
use core::{
    cmp::Ordering::{self, Equal},
    fmt,
    hash::{BuildHasher, Hash},
};
use hashbrown::HashMap;
use std::collections::BinaryHeap;

#[derive(Copy, Clone, Debug)]
pub struct PathEntry<S> {
    // f = cost so far + heuristic (g + h)
    cost_estimate: f32,
    // h alone: the frontier tie-break's second component.
    heuristic: f32,
    // g alone: the third component.
    cost: f32,
    // Deterministic hash of the node's identity (FxHasher64, zero seed —
    // the same fixed-seed hasher the civ layer already uses for
    // cross-machine determinism): the tie-break's node-identity component,
    // making the frontier order INSERTION-INDEPENDENT — equal-(f,h,g)
    // entries order the same way regardless of the order neighbors were
    // pushed (architect-ruled option (c): `S` deliberately carries no
    // `Ord` — vek refuses it on vectors — and this rides the existing
    // `Hash` bound with zero public-API change).
    node_hash: u64,
    // Insertion sequence number: unique per pushed entry, so the key below
    // is a TOTAL order and Eq/Ord stay coherent (`Equal` only vs self); it
    // also breaks the measure-zero 64-bit hash-collision case
    // (bastion ENGINE-OPT-1 item 177; prior art: lockstep-deterministic
    // priority queues + van Dijk's secondary-h comparator). The heap's
    // previous order compared f alone via `partial_cmp(..).unwrap_or(Equal)`
    // — equal-f ties (ubiquitous on uniform-cost voxel grids) then resolved
    // by BinaryHeap's internal sift order, i.e. non-deterministically
    // w.r.t. anything the caller controls, making the expansion order and
    // the resulting path non-reproducible run-to-run.
    seq: u64,
    node: S,
}

/// The frontier key's node-identity component (see `PathEntry::node_hash`).
fn frontier_node_hash<S: core::hash::Hash>(node: &S) -> u64 {
    use core::hash::Hasher;
    let mut hasher = fxhash::FxHasher64::default();
    node.hash(&mut hasher);
    hasher.finish()
}

impl<S> PathEntry<S> {
    /// The reverse-lexicographic total key `(f, h, g, seq)` compared with
    /// `f32::total_cmp` (a total order — no NaN-collapse-to-Equal), reversed
    /// so the max-heap `BinaryHeap` pops the LOWEST key first. `seq` is
    /// unique per entry, so `Equal` occurs only for an entry vs itself —
    /// `Eq`/`Ord`/`PartialOrd` below are coherent by construction.
    ///
    /// The node-identity component is `fxhash64(node)` rather than the
    /// packet's literal node-coord (`S` carries no `Ord` bound — vek
    /// deliberately refuses `Ord` on vectors); architect-ruled option (c):
    /// insertion-independent like coord, zero public-API change, and
    /// cross-machine deterministic (fixed-seed FxHasher64, the civ layer's
    /// own cross-machine hasher).
    fn key_cmp(&self, other: &Self) -> Ordering {
        other
            .cost_estimate
            .total_cmp(&self.cost_estimate)
            .then_with(|| other.heuristic.total_cmp(&self.heuristic))
            .then_with(|| other.cost.total_cmp(&self.cost))
            .then_with(|| other.node_hash.cmp(&self.node_hash))
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

impl<S> PartialEq for PathEntry<S> {
    fn eq(&self, other: &PathEntry<S>) -> bool { self.key_cmp(other) == Equal }
}

impl<S> Eq for PathEntry<S> {}

impl<S> Ord for PathEntry<S> {
    // Reverse ordering (see `key_cmp`), so that the lowest cost is ordered
    // first.
    fn cmp(&self, other: &PathEntry<S>) -> Ordering { self.key_cmp(other) }
}

impl<S> PartialOrd for PathEntry<S> {
    fn partial_cmp(&self, other: &PathEntry<S>) -> Option<Ordering> { Some(self.cmp(other)) }

    // This is particularly hot in `BinaryHeap::pop`, so we provide this
    // implementation. Unlike the pre-177 version (a bare f-compare that
    // disagreed with `Ord` on ties), this is exactly `cmp`-consistent.
    //
    // See note about reverse ordering above.
    fn le(&self, other: &PathEntry<S>) -> bool {
        matches!(self.key_cmp(other), Ordering::Less | Equal)
    }
}

pub enum PathResult<T> {
    /// No reachable nodes were satisfactory.
    ///
    /// Contains path to node with the lowest heuristic value (out of the
    /// explored nodes).
    None(Path<T>),
    /// Either max_iters or max_cost was reached.
    ///
    /// Contains path to node with the lowest heuristic value (out of the
    /// explored nodes).
    Exhausted(Path<T>),
    /// Path succefully found.
    ///
    /// Second field is cost.
    Path(Path<T>, f32),
    Pending,
}

impl<T> PathResult<T> {
    /// Returns `Some((path, cost))` if a path reaching the target was
    /// successfully found.
    pub fn into_path(self) -> Option<(Path<T>, f32)> {
        match self {
            PathResult::Path(path, cost) => Some((path, cost)),
            _ => None,
        }
    }

    pub fn map<U>(self, f: impl FnOnce(Path<T>) -> Path<U>) -> PathResult<U> {
        match self {
            PathResult::None(p) => PathResult::None(f(p)),
            PathResult::Exhausted(p) => PathResult::Exhausted(f(p)),
            PathResult::Path(p, cost) => PathResult::Path(f(p), cost),
            PathResult::Pending => PathResult::Pending,
        }
    }
}

// If node entry exists, this was visited!
#[derive(Clone, Debug)]
struct NodeEntry<S> {
    /// Previous node in the cheapest path (known so far) that goes from the
    /// start to this node.
    ///
    /// If `came_from == self` this is the start node! (to avoid inflating the
    /// size with `Option`)
    came_from: S,
    /// Cost to reach this node from the start by following the cheapest path
    /// known so far. This is the sum of the transition costs between all the
    /// nodes on this path.
    cost: f32,
}

#[derive(Clone)]
pub struct Astar<S, Hasher> {
    iter: usize,
    max_iters: usize,
    max_cost: f32,
    potential_nodes: BinaryHeap<PathEntry<S>>, // cost, node pairs
    visited_nodes: HashMap<S, NodeEntry<S>, Hasher>,
    /// Node with the lowest heuristic value so far.
    ///
    /// (node, heuristic value)
    closest_node: Option<(S, f32)>,
    /// Next insertion sequence number for the frontier's total-order
    /// tie-break (item 177). Monotone; never reused.
    next_seq: u64,
}

/// NOTE: Must manually derive since Hasher doesn't implement it.
impl<S: Clone + Eq + Hash + fmt::Debug, H: BuildHasher> fmt::Debug for Astar<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Astar")
            .field("iter", &self.iter)
            .field("max_iters", &self.max_iters)
            .field("potential_nodes", &self.potential_nodes)
            .field("visited_nodes", &self.visited_nodes)
            .field("closest_node", &self.closest_node)
            .finish()
    }
}

impl<S: Clone + Eq + Hash, H: BuildHasher + Clone> Astar<S, H> {
    pub fn new(max_iters: usize, start: S, hasher: H) -> Self {
        Self {
            max_iters,
            max_cost: f32::MAX,
            iter: 0,
            potential_nodes: core::iter::once(PathEntry {
                cost_estimate: 0.0,
                // h/g are unknowable before the first `poll` supplies the
                // heuristic; a single-entry heap needs no ordering, and the
                // start entry is popped first regardless.
                heuristic: 0.0,
                cost: 0.0,
                node_hash: frontier_node_hash(&start),
                seq: 0,
                node: start.clone(),
            })
            .collect(),
            visited_nodes: {
                let mut s = HashMap::with_capacity_and_hasher(1, hasher);
                s.extend(core::iter::once((start.clone(), NodeEntry {
                    came_from: start,
                    cost: 0.0,
                })));
                s
            },
            closest_node: None,
            next_seq: 1,
        }
    }

    pub fn with_max_cost(mut self, max_cost: f32) -> Self {
        self.max_cost = max_cost;
        self
    }

    pub fn set_max_iters(&mut self, max_iters: usize) { self.max_iters = max_iters; }

    /// To guarantee an optimal path the heuristic function needs to be
    /// [admissible](https://en.wikipedia.org/wiki/A*_search_algorithm#Admissibility).
    pub fn poll<I>(
        &mut self,
        iters: usize,
        // Estimate how far we are from the target.
        mut heuristic: impl FnMut(&S) -> f32,
        // get neighboring nodes
        mut neighbors: impl FnMut(&S) -> I,
        // have we reached target?
        mut satisfied: impl FnMut(&S) -> bool,
    ) -> PathResult<S>
    where
        I: Iterator<Item = (S, f32)>, // (node, transition cost)
    {
        let iter_limit = self.max_iters.min(self.iter + iters);
        while self.iter < iter_limit {
            if let Some(PathEntry {
                node,
                cost_estimate,
                ..
            }) = self.potential_nodes.pop()
            {
                let (node_cost, came_from) = self
                    .visited_nodes
                    .get(&node)
                    .map(|n| (n.cost, n.came_from.clone()))
                    .expect("All nodes in the queue should be included in visisted_nodes");

                // Item 175: seed best-so-far with the START node on the
                // first pop, so an exhausted/unreachable search falls back
                // to the provably-closest node instead of an EMPTY path
                // when no neighbor ever improved on the start.
                if self.closest_node.is_none() {
                    let h = heuristic(&node);
                    self.closest_node = Some((node.clone(), h));
                }

                if satisfied(&node) {
                    return PathResult::Path(self.reconstruct_path_to(node), node_cost);
                // Note, we assume that cost_estimate isn't an overestimation
                // (i.e. that `heuristic` doesn't overestimate).
                } else if cost_estimate > self.max_cost {
                    return PathResult::Exhausted(
                        self.closest_node
                            .clone()
                            .map(|(lc, _)| self.reconstruct_path_to(lc))
                            .unwrap_or_default(),
                    );
                } else {
                    for (neighbor, transition_cost) in neighbors(&node) {
                        if neighbor == came_from {
                            continue;
                        }
                        let neighbor_cost = self
                            .visited_nodes
                            .get(&neighbor)
                            .map_or(f32::MAX, |n| n.cost);

                        // compute cost to traverse to each neighbor
                        let cost = node_cost + transition_cost;

                        if cost < neighbor_cost {
                            let previously_visited = self
                                .visited_nodes
                                .insert(neighbor.clone(), NodeEntry {
                                    came_from: node.clone(),
                                    cost,
                                })
                                .is_some();
                            let h = heuristic(&neighbor);
                            // note that `cost` field does not include the heuristic
                            // priority queue does include heuristic
                            let cost_estimate = cost + h;

                            // Item 175 FIX: store the NEIGHBOR — the node
                            // whose heuristic `h` actually is — not its
                            // parent. The pre-fix code paired the parent
                            // with the neighbor's h, so the exhaustion/
                            // partial fallback reconstructed a path to the
                            // WRONG node (the best node's parent — or an
                            // arbitrary parent whose other child scored
                            // well). Strict `<` keeps the FIRST node seen
                            // at the best h: deterministic under 177's
                            // total-order expansion.
                            if self
                                .closest_node
                                .as_ref()
                                .map(|&(_, ch)| h < ch)
                                .unwrap_or(true)
                            {
                                self.closest_node = Some((neighbor.clone(), h));
                            };

                            // We don't need to reconsider already visited nodes as astar finds the
                            // shortest path to a node the first time it's visited, assuming the
                            // heuristic function is admissible.
                            if !previously_visited {
                                let seq = self.next_seq;
                                self.next_seq += 1;
                                self.potential_nodes.push(PathEntry {
                                    cost_estimate,
                                    heuristic: h,
                                    cost,
                                    node_hash: frontier_node_hash(&neighbor),
                                    seq,
                                    node: neighbor,
                                });
                            }
                        }
                    }
                }
            } else {
                return PathResult::None(
                    self.closest_node
                        .clone()
                        .map(|(lc, _)| self.reconstruct_path_to(lc))
                        .unwrap_or_default(),
                );
            }

            self.iter += 1
        }

        if self.iter >= self.max_iters {
            PathResult::Exhausted(
                self.closest_node
                    .clone()
                    .map(|(lc, _)| self.reconstruct_path_to(lc))
                    .unwrap_or_default(),
            )
        } else {
            PathResult::Pending
        }
    }

    fn reconstruct_path_to(&mut self, end: S) -> Path<S> {
        let mut path = vec![end.clone()];
        let mut cnode = &end;
        while let Some(node) = self
            .visited_nodes
            .get(cnode)
            .map(|n| &n.came_from)
            .filter(|n| *n != cnode)
        {
            path.push(node.clone());
            cnode = node;
        }
        path.into_iter().rev().collect()
    }
}

// bastion ENGINE-OPT-1 (ledger items 175 + 177) property tests. The grid
// fixtures are deliberately TIE-HEAVY (uniform transition costs on a
// lattice): equal-f frontier ties are exactly where the pre-177 order was
// non-deterministic.
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::RandomState;

    type Node = (i32, i32);

    const W: i32 = 16;
    const H: i32 = 16;

    /// A 16x16 lattice with a vertical wall at x==8 pierced at y==12, plus
    /// an optional full seal around the goal quadrant (for the
    /// unreachable-goal fixtures).
    fn passable(node: Node, seal_goal: bool) -> bool {
        let (x, y) = node;
        if !(0..W).contains(&x) || !(0..H).contains(&y) {
            return false;
        }
        if x == 8 && y != 12 {
            return false;
        }
        if seal_goal && x == 8 && y == 12 {
            return false;
        }
        true
    }

    fn manhattan(a: Node, b: Node) -> f32 { ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as f32 }

    /// Run a full search, recording the pop (expansion) order via the
    /// `satisfied` closure (called exactly once per popped node, in order).
    /// `neighbor_perm` rotates the neighbor-iteration order — simulating a
    /// caller whose neighbor enumeration order differs run-to-run (the
    /// HashMap-iteration class): the frontier's tie-break must make the
    /// expansion order and path INSERTION-INDEPENDENT of it.
    fn run_search_perm(seal_goal: bool, neighbor_perm: usize) -> (Vec<Node>, PathResult<Node>) {
        let start: Node = (2, 2);
        let goal: Node = (13, 3);
        // A FRESH RandomState per run: the expansion order must not depend
        // on the hasher (visited_nodes is only ever get/inserted, never
        // iterated — this pins that property).
        let mut astar = Astar::new(10_000, start, RandomState::new());
        let mut expansions = Vec::new();
        let result = astar.poll(
            10_000,
            |&node| manhattan(node, goal),
            |&(x, y)| {
                let mut dirs = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];
                dirs.rotate_left(neighbor_perm % 4);
                if neighbor_perm >= 4 {
                    dirs.reverse();
                }
                dirs.into_iter()
                    .filter(move |&n| passable(n, seal_goal))
                    .map(|n| (n, 1.0))
            },
            |&node| {
                expansions.push(node);
                node == goal
            },
        );
        (expansions, result)
    }

    fn run_search(seal_goal: bool) -> (Vec<Node>, PathResult<Node>) {
        run_search_perm(seal_goal, 0)
    }

    #[test]
    fn item_177_tie_break_is_insertion_order_independent() {
        // THE FALSIFIER (architect-conditioned): the reference run and the
        // permuted runs push equal-key frontier entries in DIFFERENT orders.
        // A seq-only tie-break provably fails this (seq assignment follows
        // insertion order); the node-identity hash component makes the
        // order a pure function of the key set. Verified RED on the
        // seq-only build before the hash component landed.
        let (ref_exp, ref_res) = run_search_perm(false, 0);
        let ref_path = match ref_res {
            PathResult::Path(p, cost) => (p.nodes, cost),
            _ => panic!("fixture path must be reachable"),
        };
        for perm in 1..8 {
            let (exp, res) = run_search_perm(false, perm);
            assert_eq!(
                exp, ref_exp,
                "expansion order must be independent of neighbor-iteration order (perm {perm})"
            );
            let path = match res {
                PathResult::Path(p, cost) => (p.nodes, cost),
                _ => panic!("fixture path must be reachable"),
            };
            assert_eq!(
                path, ref_path,
                "path must be independent of neighbor-iteration order (perm {perm})"
            );
        }
    }

    #[test]
    fn item_177_expansion_order_and_path_identical_across_runs() {
        let (first_exp, first_res) = run_search(false);
        let first_path = match first_res {
            PathResult::Path(p, cost) => (p.nodes, cost),
            _ => panic!("fixture path must be reachable"),
        };
        for _ in 0..3 {
            let (exp, res) = run_search(false);
            assert_eq!(exp, first_exp, "expansion order must be reproducible");
            let path = match res {
                PathResult::Path(p, cost) => (p.nodes, cost),
                _ => panic!("fixture path must be reachable"),
            };
            assert_eq!(path, first_path, "resulting path must be reproducible");
        }
    }

    #[test]
    fn item_175_unreachable_goal_falls_back_to_provably_closest_node() {
        let (_, res) = run_search(true);
        let path = match res {
            PathResult::None(p) => p.nodes,
            other => panic!(
                "sealed goal must exhaust the reachable set, got {:?}",
                match other {
                    PathResult::Path(..) => "Path",
                    PathResult::Exhausted(_) => "Exhausted",
                    PathResult::Pending => "Pending",
                    PathResult::None(_) => unreachable!(),
                }
            ),
        };
        let goal: Node = (13, 3);
        // Brute-force the reachable component's true minimum heuristic.
        let mut min_h = f32::MAX;
        for x in 0..W {
            for y in 0..H {
                if passable((x, y), true) && x < 8 {
                    min_h = min_h.min(manhattan((x, y), goal));
                }
            }
        }
        let end = *path.last().expect("fallback path must be non-empty");
        assert!(passable(end, true) && end.0 < 8, "endpoint must be reachable");
        assert_eq!(
            manhattan(end, goal),
            min_h,
            "fallback endpoint must be the provably-closest reachable node"
        );
    }

    #[test]
    fn item_175_best_so_far_is_monotone_and_exhaustion_returns_it() {
        let start: Node = (2, 2);
        let goal: Node = (13, 3);
        let mut astar = Astar::new(120, start, RandomState::new());
        let mut last_best = f32::MAX;
        let mut result = PathResult::Pending;
        for _ in 0..1_000 {
            result = astar.poll(
                1,
                |&node| manhattan(node, goal),
                |&(x, y)| {
                    [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
                        .into_iter()
                        .filter(move |&n| passable(n, true))
                        .map(|n| (n, 1.0))
                },
                |&node| node == goal,
            );
            if let Some((_, h)) = astar.closest_node {
                assert!(
                    h <= last_best,
                    "best-so-far heuristic must never worsen (was {last_best}, now {h})"
                );
                last_best = h;
            }
            if !matches!(result, PathResult::Pending) {
                break;
            }
        }
        let path = match result {
            PathResult::Exhausted(p) => p.nodes,
            _ => panic!("max_iters=120 on the sealed grid must exhaust"),
        };
        let (best_node, _) = astar.closest_node.clone().expect("best-so-far must be seeded");
        assert_eq!(
            *path.last().expect("exhausted fallback must be non-empty"),
            best_node,
            "exhausted fallback must end at the recorded best-so-far node"
        );
    }
}
