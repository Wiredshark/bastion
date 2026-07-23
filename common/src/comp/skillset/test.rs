use super::*;

// Unneeded cfg(test) here keeps rust-analyzer happy
#[cfg(test)]
use petgraph::{algo::is_cyclic_directed, graph::DiGraph};

#[test]
fn check_cyclic_skill_deps() {
    let skill_prereqs: HashMap<Skill, SkillPrerequisite> =
        Ron::load_expect_cloned("common.skill_trees.skill_prerequisites").0;
    let mut graph = DiGraph::new();
    let mut nodes = HashMap::<Skill, _>::new();
    let mut add_node = |graph: &mut DiGraph<Skill, _>, node: Skill| {
        *nodes.entry(node).or_insert_with(|| graph.add_node(node))
    };

    for (skill, prereqs) in skill_prereqs.iter() {
        let skill_node = add_node(&mut graph, *skill);
        let prereqs = match prereqs {
            SkillPrerequisite::Any(skills) => skills,
            SkillPrerequisite::All(skills) => skills,
        };
        for (prereq, _) in prereqs.iter() {
            let prereq_node = add_node(&mut graph, *prereq);
            graph.add_edge(prereq_node, skill_node, ());
        }
    }

    assert!(!is_cyclic_directed(&graph));
}

/// DET-SKL-003 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof): the
/// group-unlock selection in `load_from_database` picks the CANONICAL (smallest
/// `SkillGroupKind`) unlockable group — a pure function of the candidate SET,
/// never the `hashbrown` / `RandomState` iteration order. This directly guards
/// Builder 4's fix (`canonical_next_unlockable_group`): a revert to
/// `.next()` / `.find()` (first-in-hash) would RED cases (1) and (2), because
/// when the RandomState iterates `Weapon(Pick)` before `General`, first-in-hash
/// != min.
///
/// (A result-level "final SkillSet invariant across seeds" test would be
/// TAUTOLOGICAL here: all 80 skill prerequisites are intra-group and General /
/// Pick have none, so group-unlock order can't change the RESULT with today's
/// data — the fix is a defensive guard on a latent hole. So the contract is
/// pinned at the SELECTION level, which is directly falsifiable.)
#[test]
fn det_skl_003_canonical_group_selection_is_min_and_seed_independent() {
    // `default()` has UnlockGroup(General) + UnlockGroup(Weapon(Pick)) in its
    // skills, so both those groups are "unlockable" (has_skill(UnlockGroup)).
    let skillset = SkillSet::default();
    let general = SkillGroupKind::General;
    let pick = SkillGroupKind::Weapon(ToolKind::Pick);
    // Weapon(Sword) has NO UnlockGroup skill in the default skillset -> it is
    // filtered out of the candidate set (never selected).
    let sword = SkillGroupKind::Weapon(ToolKind::Sword);

    // (1) Canonicality + non-vacuity: with General AND Weapon(Pick) both
    // unlockable, the selection is the MIN, General (General < Weapon(_) by the
    // derived Ord); the non-unlockable Weapon(Sword) is ignored.
    let mut m: HashMap<SkillGroupKind, ()> = HashMap::new();
    m.insert(pick, ());
    m.insert(general, ());
    m.insert(sword, ()); // not unlockable -> filtered
    assert_eq!(
        SkillSet::canonical_next_unlockable_group(&m, &skillset),
        Some(general),
        "must pick the canonical min unlockable group, not a hash-order entry"
    );

    // (2) Seed-independence: a fresh `hashbrown::HashMap` per iteration gets a
    // fresh RandomState (varied internal iteration order). `.min` is invariant;
    // `.next` / `.find` would vary and fail here across enough seeds.
    for _ in 0..128 {
        let mut mm: HashMap<SkillGroupKind, ()> = HashMap::new();
        mm.insert(pick, ());
        mm.insert(general, ());
        assert_eq!(
            SkillSet::canonical_next_unlockable_group(&mm, &skillset),
            Some(general),
            "canonical selection must not depend on the RandomState hash seed"
        );
    }

    // (3) A different candidate set selects a different min; none-unlockable
    // (or empty) selects None.
    let mut only_pick: HashMap<SkillGroupKind, ()> = HashMap::new();
    only_pick.insert(pick, ());
    only_pick.insert(sword, ()); // not unlockable
    assert_eq!(
        SkillSet::canonical_next_unlockable_group(&only_pick, &skillset),
        Some(pick),
    );
    let empty: HashMap<SkillGroupKind, ()> = HashMap::new();
    assert_eq!(
        SkillSet::canonical_next_unlockable_group(&empty, &skillset),
        None,
    );
    let mut none_unlockable: HashMap<SkillGroupKind, ()> = HashMap::new();
    none_unlockable.insert(sword, ()); // only a non-unlockable group present
    assert_eq!(
        SkillSet::canonical_next_unlockable_group(&none_unlockable, &skillset),
        None,
    );
}
