//! bastion (INSPECTOR-M1): the **Path** section view — the retained route,
//! as rows and as world-space line geometry.

use common::comp::bastion_inspect::{
    FrameV1, InspectFramesV1, InspectRow, PathSectionV1, SectionPayloadV1,
};

pub fn rows(payload: &SectionPayloadV1, _frames: &InspectFramesV1) -> Vec<InspectRow> {
    let SectionPayloadV1::Path(d) = payload else {
        return Vec::new();
    };
    let mut rows = Vec::with_capacity(6);

    rows.push(
        InspectRow::new(
            "Route",
            if d.total_nodes == 0 {
                if d.needs_search {
                    "none retained — a search is pending (replanning)".to_string()
                } else {
                    // A chaser holding no route while claiming not to
                    // need one is a genuine oddity, and the panel says so
                    // rather than rendering the ordinary "replanning".
                    "none retained, and no search pending".to_string()
                }
            } else {
                format!("{} nodes", d.total_nodes)
            },
            "Chaser::get_route().get_path().nodes",
            "nodes",
            FrameV1::Ecs,
        )
        .scoped("BLOCK coordinates"),
    );

    if d.total_nodes > 0 {
        let walked = d.next_idx as usize;
        let shown = d.nodes.len();
        rows.push(
            InspectRow::new(
                "Progress",
                format!("{walked} walked, {} remaining", shown.saturating_sub(walked)),
                "Route::next_idx",
                "nodes",
                FrameV1::Ecs,
            )
            // Scoped because on a truncated route the remaining count is
            // over the SENT prefix, not over the whole route -- exactly
            // the kind of silently-narrowed number this discipline exists
            // to catch.
            .scoped(if d.truncated {
                "counts are over the transmitted prefix, not the whole route"
            } else {
                "the whole route"
            }),
        );

        if d.truncated {
            rows.push(
                InspectRow::new(
                    "Truncated",
                    format!("showing the first {shown} of {} nodes", d.total_nodes),
                    "bastion_inspect::PATH_NODE_CAP",
                    "nodes",
                    FrameV1::Derived,
                )
                .scoped("the drawn line is a PREFIX, not the route"),
            );
        }

        if let Some(next) = d.nodes.get(walked) {
            rows.push(InspectRow::new(
                "Next node",
                format!("({}, {}, {})", next.x, next.y, next.z),
                "Route::get_path().nodes[next_idx]",
                "blocks",
                FrameV1::Ecs,
            ));
        }
    }

    rows.push(InspectRow::new(
        "Searching",
        if d.needs_search { "yes — no route held" } else { "no — a route is held" },
        "Chaser::needs_search",
        "",
        FrameV1::Ecs,
    ));

    rows
}

/// One drawable segment of the route in world space.
pub struct PathSegment {
    pub from: vek::Vec3<f32>,
    pub to: vek::Vec3<f32>,
    /// `true` for the already-walked prefix (drawn dim), `false` for the
    /// remaining suffix (drawn bright).
    pub walked: bool,
}

/// Colour for a segment. The walked prefix is dimmed rather than hidden:
/// where a colonist HAS been is what makes an oscillation visible, and an
/// oscillation is invisible in the remaining suffix alone.
pub const WALKED_COLOR: [f32; 4] = [0.35, 0.40, 0.45, 0.55];
pub const REMAINING_COLOR: [f32; 4] = [0.30, 0.95, 0.55, 0.95];

/// Convert the route into world-space segments.
///
/// Nodes are BLOCK coordinates, so each is offset to the block CENTRE in
/// XY and lifted slightly in Z — a line drawn through block corners at
/// exactly floor level is mostly inside the ground and reads as a dashed
/// line even when the route is continuous.
pub fn segments(d: &PathSectionV1) -> Vec<PathSegment> {
    let walked_upto = d.next_idx as usize;
    d.nodes
        .windows(2)
        .enumerate()
        .map(|(i, w)| PathSegment {
            from: node_to_world(w[0]),
            to: node_to_world(w[1]),
            // Segment `i` spans nodes i..i+1, so it is walked when its
            // FAR end has been reached.
            walked: i + 1 <= walked_upto,
        })
        .collect()
}

fn node_to_world(n: vek::Vec3<i32>) -> vek::Vec3<f32> {
    vek::Vec3::new(n.x as f32 + 0.5, n.y as f32 + 0.5, n.z as f32 + 0.15)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames() -> InspectFramesV1 {
        InspectFramesV1 {
            server_tick: 0,
            rtsim_tick: 0,
            time_of_day: 0.0,
            ticks_per_game_day: 54_000.0,
            schedule_offset_hours: 0,
        }
    }

    fn path(nodes: Vec<vek::Vec3<i32>>, next_idx: u32, total: u32, truncated: bool) -> PathSectionV1 {
        PathSectionV1 {
            nodes_hash: PathSectionV1::hash_nodes(&nodes),
            nodes,
            next_idx,
            total_nodes: total,
            truncated,
            needs_search: false,
        }
    }

    /// The walked/remaining split lands on the right segments, in both
    /// degenerate positions.
    ///
    /// FALSIFIER: change `i + 1 <= walked_upto` to `i <= walked_upto` and
    /// the "nothing walked" case marks a segment walked — RED.
    #[test]
    fn segments_split_walked_from_remaining() {
        let nodes: Vec<vek::Vec3<i32>> = (0..5).map(|i| vek::Vec3::new(i, 0, 0)).collect();

        // Nothing walked yet.
        let s = segments(&path(nodes.clone(), 0, 5, false));
        assert_eq!(s.len(), 4, "n nodes make n-1 segments");
        assert!(s.iter().all(|x| !x.walked), "nothing is walked at next_idx 0");

        // Half walked.
        let s = segments(&path(nodes.clone(), 2, 5, false));
        assert_eq!(s.iter().filter(|x| x.walked).count(), 2);
        assert_eq!(s.iter().filter(|x| !x.walked).count(), 2);

        // Finished.
        let s = segments(&path(nodes.clone(), 5, 5, false));
        assert!(s.iter().all(|x| x.walked), "a finished route is all walked");

        // A one-node or empty route draws nothing rather than panicking.
        assert!(segments(&path(vec![vek::Vec3::zero()], 0, 1, false)).is_empty());
        assert!(segments(&path(Vec::new(), 0, 0, false)).is_empty());
    }

    /// Nodes are drawn at block CENTRES, lifted off the floor.
    #[test]
    fn nodes_are_centred_in_their_block() {
        let s = segments(&path(
            vec![vek::Vec3::new(3, 4, 5), vek::Vec3::new(4, 4, 5)],
            0,
            2,
            false,
        ));
        assert_eq!(s[0].from, vek::Vec3::new(3.5, 4.5, 5.15));
        assert!(s[0].to.z > 5.0, "the line must sit above the floor");
    }

    /// ★ A TRUNCATED ROUTE SAYS SO, and its progress row says its counts
    /// are over the PREFIX.
    ///
    /// A "3 remaining" that silently meant "3 remaining of the 96 I chose
    /// to send" is precisely the narrowed-scope defect this project has
    /// already shipped once.
    ///
    /// FALSIFIER: drop the `truncated` branch of the scope and this goes
    /// RED.
    #[test]
    fn a_truncated_route_is_labelled_as_a_prefix() {
        let nodes: Vec<vek::Vec3<i32>> = (0..96).map(|i| vek::Vec3::new(i, 0, 0)).collect();
        let p = SectionPayloadV1::Path(path(nodes, 10, 400, true));
        let r = rows(&p, &frames());
        let trunc = r.iter().find(|x| x.label() == "Truncated").expect("truncated row");
        assert!(trunc.value().contains("96 of 400"));
        assert!(trunc.scope().is_some_and(|s| s.contains("PREFIX")));
        let prog = r.iter().find(|x| x.label() == "Progress").expect("progress row");
        assert!(
            prog.scope().is_some_and(|s| s.contains("transmitted prefix")),
            "a prefix-scoped count must say so"
        );
    }

    /// An empty route reads as replanning, not as an error, and the two
    /// empty cases are told apart.
    #[test]
    fn an_empty_route_reads_as_replanning() {
        let mut p = path(Vec::new(), 0, 0, false);
        p.needs_search = true;
        let r = rows(&SectionPayloadV1::Path(p.clone()), &frames());
        let route = r.iter().find(|x| x.label() == "Route").expect("route row");
        assert!(route.value().contains("replanning"));

        p.needs_search = false;
        let r = rows(&SectionPayloadV1::Path(p), &frames());
        let route = r.iter().find(|x| x.label() == "Route").expect("route row");
        assert!(route.value().contains("no search pending"), "the odd case must be distinct");
    }
}
