//! bastion (INSPECTOR-M1): the **Path** section provider — the RETAINED
//! route.
//!
//! ★ READ-ONLY, ENFORCED BY THE BORROW CHECKER. This provider receives
//! `Option<&Chaser>`. Every route-mutating entry point on `Chaser`
//! (`chase`, the search step, the stuck-history rebase) takes `&mut self`,
//! so this function CANNOT trigger a path search however it is edited.
//! That matters more than usual here: a path search is expensive and
//! scheduled against a tick budget, so an inspector that provoked one
//! would not merely observe the colony, it would change how the colony
//! moves — and the observation would be of the inspector.
//!
//! The shaping is split out of [`provide`] into [`build`] deliberately.
//! `Chaser`'s route field is private and it exposes no way to install one,
//! so a route-bearing `Chaser` cannot be constructed in a test — and the
//! answer to that is to make the part worth testing not need one, rather
//! than to open a hole in `common::path` for the benefit of a test.
//! [`provide`] is then a three-line adapter over a fully pinned builder.

use common::comp::bastion_inspect::{
    PATH_NODE_CAP, PathSectionV1, SectionIdV1, SectionPayloadV1, UnavailableReasonV1,
};

use super::{InspectCtx, unloaded};

/// Shape a retained route into the section payload. PURE.
pub(crate) fn build(nodes: &[vek::Vec3<i32>], next_idx: usize, needs_search: bool) -> PathSectionV1 {
    let total = nodes.len();
    let sent: Vec<vek::Vec3<i32>> = nodes.iter().take(PATH_NODE_CAP).copied().collect();
    PathSectionV1 {
        // The hash is over the FULL list, before the cap: two routes that
        // differ only past node 96 are genuinely different routes, and a
        // hash over the truncated prefix would call them equal, so the
        // client would never rebuild the drawn line.
        nodes_hash: PathSectionV1::hash_nodes(nodes),
        // Clamped into the TRANSMITTED range. `next_idx` can legitimately
        // sit at `len` (the route is finished) and can exceed the cap on a
        // long route; an unclamped value would index past the end of the
        // client's node list when it splits the walked prefix from the
        // remaining suffix.
        next_idx: next_idx.min(sent.len()) as u32,
        total_nodes: total as u32,
        truncated: total > sent.len(),
        needs_search,
        nodes: sent,
    }
}

pub fn provide(ctx: &InspectCtx<'_>) -> SectionPayloadV1 {
    let Some(l) = ctx.loaded.as_ref() else {
        return unloaded(SectionIdV1::Path);
    };
    // No `Agent` component at all: the route OWNER is missing, which is a
    // different fact from "has an owner, no route right now". Kept
    // distinct so the panel does not report a colonist with no pathfinder
    // as merely between plans.
    let Some(chaser) = l.chaser else {
        return SectionPayloadV1::Unavailable(SectionIdV1::Path, UnavailableReasonV1::NoRoute);
    };
    // A retained-route owner with no route is NORMAL -- it is what being
    // between plans looks like. It answers with an empty path and the
    // `needs_search` flag rather than refusing, so the panel can say
    // "replanning" instead of "unknown".
    match chaser.get_route() {
        Some(route) => SectionPayloadV1::Path(build(
            &route.get_path().nodes,
            route.next_idx(),
            chaser.needs_search(),
        )),
        None => SectionPayloadV1::Path(build(&[], 0, chaser.needs_search())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(n: u64) -> common::uid::Uid {
        common::uid::Uid(std::num::NonZeroU64::new(n).expect("nonzero"))
    }

    fn ctx_with<'a>(
        board: &'a crate::bastion_jobs::JobBoard,
        chaser: Option<&'a common::path::Chaser>,
    ) -> InspectCtx<'a> {
        InspectCtx {
            subject: uid(1),
            frames: super::super::frames(0, 0, 0.0, 1.0 / 30.0, 48.0, 0),
            record: None,
            parent_name: None,
            board,
            loaded: Some(super::super::LoadedCtx {
                pos: None,
                health: None,
                arbiter: None,
                active_job: None,
                chaser,
                mood: None,
                needs: None,
                energy: None,
            }),
            names: &[],
            mind: None,
            colony: None,
        }
    }

    /// The cap holds, the `truncated` flag is honest, and the hash is
    /// taken over the FULL route rather than the transmitted prefix.
    ///
    /// FALSIFIER: hash `sent` instead of `nodes` in `build`, and the last
    /// assertion (two routes differing only past the cap must hash
    /// differently) goes RED.
    #[test]
    fn long_routes_are_capped_and_flagged() {
        let long: Vec<vek::Vec3<i32>> =
            (0..PATH_NODE_CAP as i32 + 40).map(|i| vek::Vec3::new(i, 0, 0)).collect();
        let p = build(&long, 5, false);
        assert_eq!(p.nodes.len(), PATH_NODE_CAP, "the cap must bound the packet");
        assert_eq!(p.total_nodes, long.len() as u32, "the real length must still be reported");
        assert!(p.truncated, "a capped route must say so");
        assert_eq!(p.next_idx, 5);

        // Two routes identical up to the cap but differing past it are
        // DIFFERENT routes and must not share a hash.
        let mut other = long.clone();
        *other.last_mut().unwrap() = vek::Vec3::new(-99, -99, -99);
        let p2 = build(&other, 5, false);
        assert_eq!(p.nodes, p2.nodes, "the fixture must be identical within the cap");
        assert_ne!(p.nodes_hash, p2.nodes_hash, "the hash must cover the whole route");
    }

    /// `next_idx` is clamped into the SENT list. An out-of-range split
    /// point would panic or mis-slice the drawn line client-side.
    ///
    /// FALSIFIER: drop the `.min(sent.len())` in `build` and this goes
    /// RED.
    #[test]
    fn next_idx_is_clamped_into_the_sent_nodes() {
        let long: Vec<vek::Vec3<i32>> =
            (0..PATH_NODE_CAP as i32 + 40).map(|i| vek::Vec3::new(i, 0, 0)).collect();
        // Walked past the cap.
        let p = build(&long, PATH_NODE_CAP + 30, false);
        assert_eq!(p.next_idx as usize, p.nodes.len(), "must clamp to the sent list");
        // A finished short route sits legitimately AT len.
        let short = vec![vek::Vec3::new(0, 0, 0), vek::Vec3::new(1, 0, 0)];
        assert_eq!(build(&short, 2, false).next_idx, 2);
        // And an empty route cannot produce a nonzero split.
        assert_eq!(build(&[], 9, true).next_idx, 0);
    }

    /// A short route is sent whole and NOT flagged truncated.
    #[test]
    fn short_routes_are_sent_whole() {
        let nodes = vec![vek::Vec3::new(1, 1, 1), vek::Vec3::new(2, 2, 1)];
        let p = build(&nodes, 1, false);
        assert_eq!(p.nodes, nodes);
        assert!(!p.truncated);
        assert_eq!(p.total_nodes, 2);
    }

    /// The two "no line to draw" cases are DISTINGUISHABLE: no pathfinder
    /// at all refuses with a reason; a pathfinder between plans answers
    /// with an empty route and `needs_search`.
    ///
    /// FALSIFIER: collapse both arms of `provide` to the same return and
    /// this goes RED.
    #[test]
    fn no_route_and_no_chaser_are_different_answers() {
        let board = crate::bastion_jobs::JobBoard::default();

        // A default `Chaser` holds no route -- exactly the between-plans
        // state, and the one route state a test can construct.
        let empty = common::path::Chaser::default();
        assert!(empty.needs_search(), "the fixture must be a routeless chaser");
        let SectionPayloadV1::Path(p) = provide(&ctx_with(&board, Some(&empty))) else {
            panic!("a chaser between plans must still answer");
        };
        assert!(p.nodes.is_empty());
        assert!(p.needs_search, "no retained route means a search is pending");
        assert!(!p.truncated);

        match provide(&ctx_with(&board, None)) {
            SectionPayloadV1::Unavailable(id, reason) => {
                assert_eq!(id, SectionIdV1::Path);
                assert_eq!(reason, UnavailableReasonV1::NoRoute);
            },
            other => panic!("no chaser must refuse, got {:?}", other.id()),
        }
    }

    /// An unloaded subject refuses with the UNLOAD reason, not the
    /// no-route one — the panel must be able to say "unloaded" rather
    /// than implying the colonist is standing still.
    #[test]
    fn unloaded_subject_refuses_with_the_unload_reason() {
        let board = crate::bastion_jobs::JobBoard::default();
        let ctx = InspectCtx {
            subject: uid(1),
            frames: super::super::frames(0, 0, 0.0, 1.0 / 30.0, 48.0, 0),
            record: None,
            parent_name: None,
            board: &board,
            loaded: None,
            names: &[],
            mind: None,
            colony: None,
        };
        match provide(&ctx) {
            SectionPayloadV1::Unavailable(id, reason) => {
                assert_eq!(id, SectionIdV1::Path);
                assert_eq!(reason, UnavailableReasonV1::SubjectUnloaded);
            },
            other => panic!("expected a refusal, got {:?}", other.id()),
        }
    }
}
