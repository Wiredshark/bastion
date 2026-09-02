//! bastion (INSPECTOR-M1): the session-side adapter for the modular
//! colonist inspector.
//!
//! Owns one [`InspectSubscription`] (the pure state machine lives in
//! `common::comp::bastion_inspect::subscription`, where its bandwidth
//! pins run without a voxygen link) and does the three things that need a
//! client: turn `Instant` into seconds, remember which sections the panel
//! has expanded, and cache the world-space route geometry so the drawn
//! line is rebuilt only when the route actually changes.
//!
//! ★ FALLBACK IS IDENTITY. With nothing selected this object holds no
//! subject, [`InspectSubscription::poll`] returns `None`, and not one byte
//! goes to the server — the same as before this feature existed.

use common::{
    comp::bastion_inspect::{
        SectionIdV1, SectionRequestV1, SectionSetV1, SectionedInspectV1,
        subscription::InspectSubscription,
    },
    uid::Uid,
};

use crate::{
    hud::bastion_inspector::{self, path::PathSegment},
    scene::debug::DebugShapeId,
};

/// What the route overlay should do this frame.
pub enum PathDraw {
    /// Nothing to draw — remove whatever is on screen.
    Clear,
    /// The route is unchanged; leave the existing shapes alone.
    Unchanged,
    /// Rebuild from these segments, and record this node hash.
    Rebuild(Vec<PathSegment>, u64),
}

pub struct InspectSubState {
    sub: InspectSubscription,
    /// Which section panels are open. Everything, by default: phase 1
    /// ships the collapse MECHANISM (the request carries the set, the
    /// server computes only what is in it, and both ends are pinned)
    /// without yet spending a keybind on the affordance.
    expanded: SectionSetV1,
    epoch: std::time::Instant,
    /// The route's world-space line shapes, and the node hash they were
    /// built from.
    path_shapes: Vec<DebugShapeId>,
    path_hash: Option<u64>,
    /// Append each row's producer/unit/scope/frame to its line.
    verbose: bool,
}

impl Default for InspectSubState {
    fn default() -> Self {
        Self {
            sub: InspectSubscription::new(),
            expanded: SectionSetV1::all(),
            epoch: std::time::Instant::now(),
            path_shapes: Vec::new(),
            path_hash: None,
            verbose: false,
        }
    }
}

impl InspectSubState {
    fn now(&self) -> f64 { self.epoch.elapsed().as_secs_f64() }

    pub fn subject(&self) -> Option<Uid> { self.sub.subject() }

    pub fn expanded(&self) -> SectionSetV1 { self.expanded }

    /// Open or close one section. A closed section stops being requested
    /// on the very next poll, so closing it stops the server computing it
    /// too.
    pub fn toggle(&mut self, id: SectionIdV1) { self.expanded = self.expanded.toggled(id); }

    pub fn set_verbose(&mut self, v: bool) { self.verbose = v; }

    /// Point the panel at a colonist, or at nothing.
    pub fn set_subject(&mut self, subject: Option<Uid>) { self.sub.set_subject(subject); }

    /// What to ask for this frame, if anything.
    pub fn poll(&mut self) -> Option<SectionRequestV1> {
        let now = self.now();
        self.sub.poll(now, self.expanded)
    }

    /// Offer a reply; `false` means it was stale and was dropped.
    pub fn accept(&mut self, reply: SectionedInspectV1) -> bool { self.sub.accept(reply) }

    pub fn latest(&self) -> Option<&SectionedInspectV1> { self.sub.latest() }

    /// The panel's text lines, or empty when there is nothing to show.
    pub fn lines(&self) -> Vec<String> {
        let Some(reply) = self.sub.latest() else {
            return Vec::new();
        };
        // The two clocks first, where a reader looks: every number
        // below them belongs to one frame or the other, and a panel that
        // did not say which would be inviting the reader to assume.
        let mut out =
            bastion_inspector::header_lines(&reply.frames, reply.loaded, self.verbose);
        // ★ EACH SECTION'S OWN AGE. Sections refresh at different
        // cadences and the subscription CARRIES FORWARD one the newest
        // reply did not answer, so a slow section's rows are routinely
        // older than the clocks printed above them. The heading says how
        // much older; a panel that did not would be putting two frames on
        // one screen unlabelled.
        out.extend(bastion_inspector::to_lines(
            &bastion_inspector::render(reply, |id| self.sub.section_age_ticks(id)),
            self.verbose,
        ));
        out
    }

    /// INSPECTOR-M2: the same reply as `lines()`, projected for the
    /// conrod panel -- every section id, folded or not, so a folded
    /// heading stays clickable. `None` = nothing to show.
    pub fn panel(&self) -> Option<bastion_inspector::panel::InspectPanel> {
        let reply = self.sub.latest()?;
        Some(bastion_inspector::panel::build(
            reply,
            |id| self.sub.section_age_ticks(id),
            self.expanded,
            self.verbose,
        ))
    }

    /// What the route overlay should do this frame.
    ///
    /// Returns OWNED geometry rather than a borrow into `self`: the caller
    /// needs `&mut self.scene` at the same moment, and handing it a
    /// reference into the subscription would make the two borrows fight
    /// for no benefit.
    pub fn path_draw(&self) -> PathDraw {
        use common::comp::bastion_inspect::SectionPayloadV1;
        let Some(reply) = self.sub.latest() else {
            return PathDraw::Clear;
        };
        let path = reply.sections.iter().find_map(|p| match p {
            SectionPayloadV1::Path(d) => Some(d),
            _ => None,
        });
        let Some(path) = path else {
            // The Path section is collapsed, refused, or simply not in
            // this reply. Either way there is no line to draw.
            return PathDraw::Clear;
        };
        // ★ REBUILD ONLY ON A REAL CHANGE. A colonist walking a stable
        // route re-sends the same nodes twice a second; rebuilding ~96
        // debug shapes at 2 Hz for a line that has not moved is exactly
        // the sort of cost an inspector must not impose on the thing it
        // is observing.
        if self.path_hash == Some(path.nodes_hash) {
            return PathDraw::Unchanged;
        }
        PathDraw::Rebuild(bastion_inspector::path::segments(path), path.nodes_hash)
    }

    /// Take ownership of the previous shapes so the caller can delete
    /// them, and record the hash the new ones were built from.
    pub fn swap_path_shapes(
        &mut self,
        new_shapes: Vec<DebugShapeId>,
        hash: u64,
    ) -> Vec<DebugShapeId> {
        self.path_hash = Some(hash);
        std::mem::replace(&mut self.path_shapes, new_shapes)
    }

    /// Drop every drawn shape (deselection, or the panel closing).
    pub fn take_path_shapes(&mut self) -> Vec<DebugShapeId> {
        self.path_hash = None;
        std::mem::take(&mut self.path_shapes)
    }
}
