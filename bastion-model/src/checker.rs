//! Exhaustive checker over the traversal-contract model: BFS reachability,
//! per-state/per-edge safety (S1-S5), backward-reachability never-stranded
//! skeleton (S6), and fair-closed SCC analysis for liveness (L1 starvation,
//! L2 abort→reacquire livelock) under weak fairness on SYSTEM actions.
//!
//! On violation: the MINIMAL action trace (BFS parent pointers; BFS order
//! makes safety traces shortest), plus the offending cycle for liveness.

use crate::model::{apply, enabled_actions, initial_state, Action, Config, Phase, State};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct Violation {
    pub property: &'static str,
    pub member: Option<u8>,
    /// Shortest action path from the initial state to the offending state
    /// (for liveness: to a state inside the offending SCC).
    pub trace: Vec<Action>,
    /// For liveness violations: one cycle inside the fair-closed SCC.
    pub cycle: Vec<Action>,
    pub state_repr: String,
}

pub struct Report {
    pub states: usize,
    pub edges: usize,
    pub violations: Vec<Violation>,
    pub truncated_at_depth: bool,
}

struct Graph {
    states: Vec<State>,
    /// Forward edges: (target-state index, action).
    succ: Vec<Vec<(usize, Action)>>,
    /// BFS tree parent: (parent index, action) — trace reconstruction.
    parent: Vec<Option<(usize, Action)>>,
    depth: Vec<u32>,
}

fn state_repr(s: &State) -> String {
    let members: Vec<String> = s
        .members
        .iter()
        .enumerate()
        .map(|(i, m)| {
            format!(
                "m{}:{:?}{}{}r{}",
                i,
                m.phase,
                if m.contact { "+c" } else { "" },
                if m.progress { "+p" } else { "" },
                m.reengage
            )
        })
        .collect();
    format!(
        "[{}] rsv={:?} q={:?} stale={:?} intr={:?}",
        members.join(" "),
        s.reservation,
        s.queue,
        s.stale,
        s.interrupt
    )
}

fn trace_to(g: &Graph, mut idx: usize) -> Vec<Action> {
    let mut rev = Vec::new();
    while let Some((p, a)) = g.parent[idx] {
        rev.push(a);
        idx = p;
    }
    rev.reverse();
    rev
}

/// Explore the full reachable space (BFS), checking per-edge safety (S3, S4)
/// as edges are generated.
fn explore(cfg: &Config, max_depth: Option<u32>, violations: &mut Vec<Violation>) -> (Graph, bool) {
    let init = initial_state(cfg);
    let mut index: HashMap<State, usize> = HashMap::new();
    let mut g = Graph { states: vec![init.clone()], succ: vec![Vec::new()], parent: vec![None], depth: vec![0] };
    index.insert(init, 0);
    let mut queue = VecDeque::new();
    queue.push_back(0usize);
    let mut truncated = false;
    let mut s3_reported = false;
    let mut s4_reported = false;

    while let Some(i) = queue.pop_front() {
        if let Some(cap) = max_depth {
            if g.depth[i] >= cap {
                truncated = true;
                continue;
            }
        }
        let s = g.states[i].clone();
        for a in enabled_actions(&s, cfg) {
            let t = apply(&s, a, cfg);

            // S3: a stale-epoch write must NEVER mutate member/link state
            // (the R10 fence). The consumed `stale` token itself is the only
            // permitted delta.
            if matches!(a, Action::StaleWrite) && !s3_reported {
                let mut expected = s.clone();
                expected.stale = None;
                if t != expected {
                    s3_reported = true;
                    let mut trace = trace_to(&g, i);
                    trace.push(a);
                    violations.push(Violation {
                        property: "S3 stale-epoch write mutated state (fence hole)",
                        member: s.stale,
                        trace,
                        cycle: Vec::new(),
                        state_repr: state_repr(&t),
                    });
                }
            }
            // S4: no traversal progress across a terrain-revision mismatch.
            if matches!(a, Action::EnterLink(_) | Action::FrontierComplete(_) | Action::TopExit(_))
                && s.reservation.map(|r| !r.revision_current).unwrap_or(true)
                && !s4_reported
            {
                s4_reported = true;
                let mut trace = trace_to(&g, i);
                trace.push(a);
                violations.push(Violation {
                    property: "S4 traversal progressed across a terrain-revision mismatch",
                    member: None,
                    trace,
                    cycle: Vec::new(),
                    state_repr: state_repr(&t),
                });
            }

            let j = match index.get(&t) {
                Some(&j) => j,
                None => {
                    let j = g.states.len();
                    index.insert(t.clone(), j);
                    g.states.push(t);
                    g.succ.push(Vec::new());
                    g.parent.push(Some((i, a)));
                    g.depth.push(g.depth[i] + 1);
                    queue.push_back(j);
                    j
                },
            };
            g.succ[i].push((j, a));
        }
    }
    (g, truncated)
}

/// Per-state safety: S1 reservation consistency (capacity=1 is structural:
/// the reservation is a single Option), S2 single owned-moving member which
/// must be the reservation holder, S5 dead members hold nothing.
fn check_state_safety(g: &Graph, violations: &mut Vec<Violation>) {
    let mut done: HashSet<&'static str> = HashSet::new();
    for (i, s) in g.states.iter().enumerate() {
        if let Some(r) = s.reservation {
            let holder = &s.members[r.member as usize];
            if holder.phase == Phase::Dead && !done.contains("S5") {
                done.insert("S5");
                violations.push(Violation {
                    property: "S5 dead member holds the reservation",
                    member: Some(r.member),
                    trace: trace_to(g, i),
                    cycle: Vec::new(),
                    state_repr: state_repr(s),
                });
            } else if !holder.phase.owned() && holder.phase != Phase::Dead && !done.contains("S1") {
                done.insert("S1");
                violations.push(Violation {
                    property: "S1 reservation held by a non-owned-phase member (leak)",
                    member: Some(r.member),
                    trace: trace_to(g, i),
                    cycle: Vec::new(),
                    state_repr: state_repr(s),
                });
            }
        }
        let moving: Vec<u8> = s
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.phase.owned_moving())
            .map(|(i, _)| i as u8)
            .collect();
        let s2_bad = moving.len() > 1
            || moving
                .iter()
                .any(|&m| s.reservation.map(|r| r.member) != Some(m));
        if s2_bad && !done.contains("S2") {
            done.insert("S2");
            violations.push(Violation {
                property: "S2 owned-moving member(s) without sole valid reservation",
                member: moving.first().copied(),
                trace: trace_to(g, i),
                cycle: Vec::new(),
                state_repr: state_repr(s),
            });
        }
    }
}

/// S6 (never-stranded skeleton): for every reachable state, every live
/// non-terminal member can still reach Delivered or Netted (deliverability
/// must not depend on dying — death paths cannot satisfy the target).
fn check_s6(g: &Graph, cfg: &Config, violations: &mut Vec<Violation>) {
    let n = cfg.n_members;
    // Reverse edges once.
    let mut pred: Vec<Vec<usize>> = vec![Vec::new(); g.states.len()];
    for (i, outs) in g.succ.iter().enumerate() {
        for &(j, _) in outs {
            pred[j].push(i);
        }
    }
    for m in 0..n {
        let mut can = vec![false; g.states.len()];
        let mut queue = VecDeque::new();
        for (i, s) in g.states.iter().enumerate() {
            if matches!(s.members[m].phase, Phase::Delivered | Phase::Netted) {
                can[i] = true;
                queue.push_back(i);
            }
        }
        while let Some(i) = queue.pop_front() {
            for &p in &pred[i] {
                if !can[p] {
                    can[p] = true;
                    queue.push_back(p);
                }
            }
        }
        for (i, s) in g.states.iter().enumerate() {
            if !s.members[m].phase.terminal() && !can[i] {
                violations.push(Violation {
                    property: "S6 live member permanently un-deliverable (stranded)",
                    member: Some(m as u8),
                    trace: trace_to(g, i),
                    cycle: Vec::new(),
                    state_repr: state_repr(&g.states[i]),
                });
                break; // one witness per member
            }
        }
    }
}

/// Iterative Tarjan SCC over an arbitrary edge relation.
fn tarjan(n: usize, succ: &[Vec<(usize, Action)>]) -> Vec<Vec<usize>> {
    let mut index_of = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut sccs: Vec<Vec<usize>> = Vec::new();
    let mut next_index = 0usize;
    // Explicit DFS frames: (node, child-iterator position).
    let mut frames: Vec<(usize, usize)> = Vec::new();

    for root in 0..n {
        if index_of[root] != usize::MAX {
            continue;
        }
        frames.push((root, 0));
        index_of[root] = next_index;
        low[root] = next_index;
        next_index += 1;
        stack.push(root);
        on_stack[root] = true;

        while let Some(&mut (v, ref mut ci)) = frames.last_mut() {
            if *ci < succ[v].len() {
                let (w, _) = succ[v][*ci];
                *ci += 1;
                if index_of[w] == usize::MAX {
                    index_of[w] = next_index;
                    low[w] = next_index;
                    next_index += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    frames.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index_of[w]);
                }
            } else {
                frames.pop();
                if let Some(&(p, _)) = frames.last() {
                    low[p] = low[p].min(low[v]);
                }
                if low[v] == index_of[v] {
                    let mut scc = Vec::new();
                    loop {
                        let w = stack.pop().expect("tarjan stack");
                        on_stack[w] = false;
                        scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    sccs.push(scc);
                }
            }
        }
    }
    sccs
}

/// One cycle inside an SCC starting/ending at `entry`, via intra-SCC BFS.
fn find_cycle(
    succ: &[Vec<(usize, Action)>],
    scc: &HashSet<usize>,
    entry: usize,
) -> Vec<Action> {
    let mut parent: HashMap<usize, (usize, Action)> = HashMap::new();
    let mut queue = VecDeque::new();
    queue.push_back(entry);
    while let Some(i) = queue.pop_front() {
        for &(j, a) in &succ[i] {
            if j == entry {
                let mut rev = vec![a];
                let mut cur = i;
                while cur != entry {
                    let (p, pa) = parent[&cur];
                    rev.push(pa);
                    cur = p;
                }
                rev.reverse();
                return rev;
            }
            if scc.contains(&j) && !parent.contains_key(&j) {
                parent.insert(j, (i, a));
                queue.push_back(j);
            }
        }
    }
    Vec::new()
}

/// Fair-closure test: a cycling SCC is a valid WEAKLY-FAIR counterexample
/// only if it does NOT contain a system action enabled in EVERY state of the
/// SCC whose edges all exit the given edge relation — weak fairness forces a
/// continuously-enabled action eventually, ejecting the run. Environment
/// actions carry no fairness (they may never fire). Intermittently-enabled
/// actions (e.g. Reserve at free-link states only) force nothing.
fn fair_closed(
    g: &Graph,
    cfg: &Config,
    succ: &[Vec<(usize, Action)>],
    scc: &[usize],
    set: &HashSet<usize>,
) -> bool {
    let mut common: Option<HashSet<Action>> = None;
    for &i in scc {
        let en: HashSet<Action> = enabled_actions(&g.states[i], cfg)
            .into_iter()
            .filter(|a| a.is_system())
            .collect();
        common = Some(match common {
            None => en,
            Some(c) => c.intersection(&en).copied().collect(),
        });
    }
    let intra: HashSet<Action> = scc
        .iter()
        .flat_map(|&i| succ[i].iter())
        .filter(|(j, _)| set.contains(j))
        .map(|&(_, a)| a)
        .collect();
    common.unwrap_or_default().iter().all(|a| intra.contains(a))
}

fn has_cycle(succ: &[Vec<(usize, Action)>], scc: &[usize], set: &HashSet<usize>) -> bool {
    scc.len() > 1
        || succ[scc[0]]
            .iter()
            .any(|&(j, _)| j == scc[0] && set.contains(&j))
}

/// L1 starvation + L2 livelock under weak fairness on system actions.
fn check_liveness(g: &Graph, cfg: &Config, violations: &mut Vec<Violation>) {
    // ── L1: over the FULL graph — a fair-closed cycling SCC in which some
    // live member is Queued in every state. Weak fairness cannot rescue the
    // member: Reserve(m) is at best intermittently enabled (free-link states
    // only), so runs that always race another action past it are fair.
    let mut l1_done: HashSet<u8> = HashSet::new();
    for scc in &tarjan(g.states.len(), &g.succ) {
        let set: HashSet<usize> = scc.iter().copied().collect();
        if !has_cycle(&g.succ, scc, &set) || !fair_closed(g, cfg, &g.succ, scc, &set) {
            continue;
        }
        for m in 0..cfg.n_members as u8 {
            if l1_done.contains(&m) {
                continue;
            }
            let queued_throughout = scc
                .iter()
                .all(|&i| g.states[i].members[m as usize].phase == Phase::Queued);
            if queued_throughout {
                l1_done.insert(m);
                let entry = *scc.iter().min_by_key(|&&i| g.depth[i]).expect("scc");
                violations.push(Violation {
                    property: "L1 starvation: fair cycle keeps member queued forever",
                    member: Some(m),
                    trace: trace_to(g, entry),
                    cycle: find_cycle(&g.succ, &set, entry),
                    state_repr: state_repr(&g.states[entry]),
                });
            }
        }
    }

    // ── L2: over the NO-PROGRESS subgraph (progress edges removed): a
    // fair-closed cycling SCC containing a Reacquire edge is an unbounded
    // abort→reacquire livelock — the reengage bound must make this
    // impossible by terminating the cycle into net-delivery (a progress
    // action, hence outside this subgraph).
    let sub: Vec<Vec<(usize, Action)>> = g
        .succ
        .iter()
        .map(|outs| {
            outs.iter()
                .copied()
                .filter(|(_, a)| !a.is_progress())
                .collect()
        })
        .collect();
    for scc in &tarjan(g.states.len(), &sub) {
        let set: HashSet<usize> = scc.iter().copied().collect();
        if !has_cycle(&sub, scc, &set) || !fair_closed(g, cfg, &sub, scc, &set) {
            continue;
        }
        let intra_reacquire = scc.iter().any(|&i| {
            sub[i]
                .iter()
                .any(|(j, a)| set.contains(j) && matches!(a, Action::Reacquire(_)))
        });
        if intra_reacquire {
            let entry = *scc.iter().min_by_key(|&&i| g.depth[i]).expect("scc");
            violations.push(Violation {
                property: "L2 livelock: unbounded abort/reacquire cycle without progress",
                member: None,
                trace: trace_to(g, entry),
                cycle: find_cycle(&sub, &set, entry),
                state_repr: state_repr(&g.states[entry]),
            });
            break; // one witness suffices
        }
    }
}

pub fn check(cfg: &Config, max_depth: Option<u32>) -> Report {
    let mut violations = Vec::new();
    let (g, truncated) = explore(cfg, max_depth, &mut violations);
    check_state_safety(&g, &mut violations);
    // Liveness/S6 verdicts are only sound over the FULL space.
    if !truncated {
        check_s6(&g, cfg, &mut violations);
        check_liveness(&g, cfg, &mut violations);
    }
    let edges = g.succ.iter().map(|v| v.len()).sum();
    Report { states: g.states.len(), edges, violations, truncated_at_depth: truncated }
}

pub fn print_report(name: &str, r: &Report) {
    println!("== {name}: {} states, {} edges{} ==", r.states, r.edges,
        if r.truncated_at_depth { " (TRUNCATED at max-depth: liveness/S6 skipped)" } else { "" });
    if r.violations.is_empty() {
        println!("PASS: all safety (S1-S6) and liveness (L1-L2) properties hold");
        return;
    }
    for v in &r.violations {
        println!("VIOLATION {}{}", v.property,
            v.member.map(|m| format!(" [member {m}]")).unwrap_or_default());
        println!("  state: {}", v.state_repr);
        println!("  trace ({} steps): {:?}", v.trace.len(), v.trace);
        if !v.cycle.is_empty() {
            println!("  cycle: {:?}", v.cycle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(r: &Report) -> Vec<&'static str> {
        r.violations.iter().map(|v| v.property).collect()
    }

    fn has(r: &Report, prefix: &str) -> bool {
        r.violations.iter().any(|v| v.property.starts_with(prefix))
    }

    /// The faithful contract, 2 members: every property must hold. If this
    /// ever fails, that is a CONTRACT FINDING for the architect — do not
    /// remodel it away.
    #[test]
    fn faithful_two_members_all_pass() {
        let r = check(&Config::faithful(2), None);
        assert!(r.violations.is_empty(), "contract violations: {:?}", props(&r));
        assert!(r.states > 100, "suspiciously small space: {}", r.states);
    }

    /// 3-member contention config: same bar.
    #[test]
    fn faithful_three_members_all_pass() {
        let r = check(&Config::faithful(3), None);
        assert!(r.violations.is_empty(), "contract violations: {:?}", props(&r));
    }

    /// Falsifier: disable the R10 epoch fence — the checker MUST re-detect
    /// the stale-writer class (S3, and the resulting double-owner S2).
    #[test]
    fn broken_fence_detected() {
        let mut cfg = Config::faithful(2);
        cfg.epoch_fence = false;
        let r = check(&cfg, None);
        assert!(has(&r, "S3"), "fence hole not detected: {:?}", props(&r));
        assert!(has(&r, "S2"), "double-owner consequence not detected: {:?}", props(&r));
    }

    /// Falsifier: degrade the fair queue to min-UID selection (the live
    /// anti-pattern R9 kills) — the checker MUST re-detect starvation.
    #[test]
    fn broken_queue_starves() {
        let mut cfg = Config::faithful(2);
        cfg.fair_queue = false;
        let r = check(&cfg, None);
        assert!(has(&r, "L1"), "min-UID starvation not detected: {:?}", props(&r));
    }

    /// Falsifier: remove the reengage bound — the checker MUST re-detect the
    /// abort→reacquire livelock class (the class-12 pin's target).
    #[test]
    fn broken_bound_livelocks() {
        let mut cfg = Config::faithful(2);
        cfg.reengage_bound = false;
        let r = check(&cfg, None);
        assert!(has(&r, "L2"), "unbounded reengage not detected: {:?}", props(&r));
    }

    /// Falsifier: disable terrain-revision validation — S4 must fire.
    #[test]
    fn broken_revision_guard_detected() {
        let mut cfg = Config::faithful(2);
        cfg.revision_guard = false;
        let r = check(&cfg, None);
        assert!(has(&r, "S4"), "revision hole not detected: {:?}", props(&r));
    }

    /// Falsifier: remove the despawn advance-site — S5 must fire.
    #[test]
    fn broken_death_release_detected() {
        let mut cfg = Config::faithful(2);
        cfg.death_releases = false;
        let r = check(&cfg, None);
        assert!(has(&r, "S5"), "dead-holder not detected: {:?}", props(&r));
    }

    /// Safety traces are minimal: the S5 witness must be reachable in few
    /// steps (enqueue→reserve→death is 3).
    #[test]
    fn violation_traces_are_minimal() {
        let mut cfg = Config::faithful(2);
        cfg.death_releases = false;
        let r = check(&cfg, None);
        let s5 = r.violations.iter().find(|v| v.property.starts_with("S5")).expect("S5");
        assert!(s5.trace.len() <= 3, "non-minimal trace: {:?}", s5.trace);
    }
}
