//! bastion (INSPECTOR-M1): the panel's SUBSCRIPTION STATE MACHINE.
//!
//! ★ WHY THIS IS IN `common` AND NOT IN `voxygen`. It is client POLICY but
//! it is pure — no ECS, no renderer, no `Instant`, just "given the time,
//! the selection and which panels are open, what should I ask for". Living
//! here means the bandwidth pins (`idle_sends_nothing`,
//! `one_request_in_flight`, `collapsed_sections_are_not_requested`) run in
//! a fast `-p veloren-common` test rather than behind a full voxygen link,
//! so they are pins that will actually be run. `voxygen::session::inspect_sub`
//! is the thin adapter that owns one of these and converts `Instant` to
//! seconds.
//!
//! ★ THE BANDWIDTH CONTRACT, which is a correctness property here and not
//! an optimisation:
//!
//! 1. Nothing selected => ZERO bytes. Not a keepalive, not an empty
//!    request.
//! 2. Detail for the SELECTED colonist only.
//! 3. A COLLAPSED section is never requested, so it is never computed
//!    server-side either.
//! 4. At most ONE request in flight. A reply whose `seq` does not match
//!    the outstanding request is DROPPED — late answers must not
//!    overwrite fresh ones, and a stale answer for a colonist the player
//!    already deselected must not repaint the panel.
//! 5. Live sections at ~2 Hz, slow sections at ~0.5 Hz, derived from
//!    `SectionIdV1::cadence()` rather than typed at the call site.
//!
//! The server enforces its own floor regardless
//! (`bastion_server::bastion_inspector::MIN_REQUEST_GAP_SECS`): the
//! client's cadence is a courtesy, and a rate limit that lives only in the
//! requester is not a rate limit.

use super::{SectionIdV1, SectionRequestV1, SectionSetV1, SectionedInspectV1};
use crate::uid::Uid;

/// The outstanding request, if any.
#[derive(Clone, Debug, PartialEq)]
struct InFlight {
    seq: u32,
    subject: Uid,
    sent_at: f64,
}

/// How long to wait for a reply before allowing another request.
///
/// Without this a dropped packet would wedge the panel forever: `one
/// request in flight` and "the reply never came" compose into "no request
/// ever again". A guard must not starve the thing it protects.
pub const IN_FLIGHT_TIMEOUT_SECS: f64 = 3.0;

/// The panel's request state for ONE subject.
#[derive(Clone, Debug, Default)]
pub struct InspectSubscription {
    /// Keyed on `Uid`, NEVER on `specs::Entity`.
    ///
    /// ★ THE DEFECT THIS FIXES. Selection was entity-keyed, so it
    /// silently vanished the moment a colonist unloaded — the panel went
    /// blank and the player was told nothing, which is indistinguishable
    /// from "the colonist died". A `Uid` survives the unload, the
    /// subscription keeps asking, and the roster-backed sections keep
    /// answering.
    subject: Option<Uid>,
    seq: u32,
    in_flight: Option<InFlight>,
    /// `sent_at` per section, for the per-cadence throttle. A fixed array
    /// indexed by `SectionIdV1::index()`, so it cannot fall behind the
    /// registry.
    last_sent: [Option<f64>; SectionIdV1::COUNT],
    latest: Option<SectionedInspectV1>,
}

impl InspectSubscription {
    pub fn new() -> Self { Self::default() }

    pub fn subject(&self) -> Option<Uid> { self.subject }

    /// The most recent ACCEPTED reply, or `None`.
    pub fn latest(&self) -> Option<&SectionedInspectV1> { self.latest.as_ref() }

    pub fn has_request_in_flight(&self) -> bool { self.in_flight.is_some() }

    /// Point the panel at a different colonist (or at nothing).
    ///
    /// Changing subject DISCARDS the cached reply and the throttle
    /// history immediately. Showing colonist A's rows under colonist B's
    /// name for even one frame is exactly the class of error the whole
    /// producer/frame discipline exists to prevent, so the panel goes
    /// empty until B answers rather than briefly lying.
    pub fn set_subject(&mut self, subject: Option<Uid>) {
        if self.subject == subject {
            return;
        }
        self.subject = subject;
        self.latest = None;
        self.last_sent = [None; SectionIdV1::COUNT];
        // The outstanding request is for the OLD subject. Forgetting it
        // here (rather than waiting for its reply) is what lets the new
        // subject be asked immediately; the reply itself is still
        // rejected on arrival because its subject no longer matches.
        self.in_flight = None;
    }

    /// Which of the expanded sections are DUE at `now`.
    ///
    /// A section is due if it has never been sent, or its cadence
    /// interval has elapsed. Collapsed sections are absent from
    /// `expanded` and are therefore never due.
    fn due(&self, now: f64, expanded: SectionSetV1) -> SectionSetV1 {
        expanded
            .sanitized()
            .iter()
            .filter(|id| match self.last_sent[id.index()] {
                None => true,
                // `now < sent` means the clock went backwards; treat it
                // as due rather than wedging the panel until it catches
                // up.
                Some(sent) => {
                    now < sent || now - sent >= f64::from(id.cadence().min_interval_secs())
                },
            })
            .collect()
    }

    /// Decide what to send this frame.
    ///
    /// Returns `None` for "send nothing", which is the answer whenever
    /// nothing is selected, nothing is expanded, a request is already in
    /// flight, or no expanded section is due yet.
    pub fn poll(&mut self, now: f64, expanded: SectionSetV1) -> Option<SectionRequestV1> {
        // (1) Nothing selected => ZERO bytes.
        let subject = self.subject?;

        // (4) At most one in flight -- but not forever: a lost reply must
        // not wedge the panel.
        if let Some(f) = &self.in_flight {
            if now >= f.sent_at && now - f.sent_at < IN_FLIGHT_TIMEOUT_SECS {
                return None;
            }
            self.in_flight = None;
        }

        // (3) Only what is expanded, and (5) only what is due.
        let sections = self.due(now, expanded);
        if sections.is_empty() {
            return None;
        }

        self.seq = self.seq.wrapping_add(1);
        for id in sections.iter() {
            self.last_sent[id.index()] = Some(now);
        }
        self.in_flight = Some(InFlight {
            seq: self.seq,
            subject,
            sent_at: now,
        });
        Some(SectionRequestV1 {
            subject,
            seq: self.seq,
            sections,
        })
    }

    /// Offer a reply. Returns whether it was ACCEPTED.
    ///
    /// A reply is rejected unless it matches BOTH the outstanding seq and
    /// the current subject. The subject check is not redundant: seq wraps
    /// at `u32::MAX`, and a reply that arrives after the player has
    /// re-selected the previous colonist could otherwise match by
    /// coincidence.
    pub fn accept(&mut self, reply: SectionedInspectV1) -> bool {
        let Some(f) = &self.in_flight else {
            return false;
        };
        if f.seq != reply.seq || Some(reply.subject) != self.subject || f.subject != reply.subject {
            return false;
        }
        self.in_flight = None;
        self.latest = Some(reply);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> SectionSetV1 { SectionSetV1::all() }

    fn uid(n: u64) -> Uid { Uid(std::num::NonZeroU64::new(n).expect("nonzero")) }

    fn reply_to(req: &SectionRequestV1) -> SectionedInspectV1 {
        SectionedInspectV1 {
            subject: req.subject,
            seq: req.seq,
            loaded: true,
            frames: super::super::InspectFramesV1 {
                server_tick: 0,
                rtsim_tick: 0,
                time_of_day: 0.0,
                ticks_per_game_day: 54_000.0,
                schedule_offset_hours: 0,
            },
            sections: Vec::new(),
        }
    }

    /// ★ NOTHING SELECTED => ZERO BYTES. Not a keepalive, not an empty
    /// request, at any time, however many sections are expanded.
    ///
    /// FALSIFIER: change `let subject = self.subject?` to default the
    /// subject and this goes RED.
    #[test]
    fn idle_sends_nothing() {
        let mut s = InspectSubscription::new();
        for t in 0..100 {
            assert!(
                s.poll(f64::from(t) * 0.1, all()).is_none(),
                "an unselected panel must not send"
            );
        }
        // Selected but nothing expanded is also zero bytes.
        s.set_subject(Some(uid(1)));
        for t in 0..100 {
            assert!(
                s.poll(f64::from(t) * 0.1, SectionSetV1::empty()).is_none(),
                "a panel with every section collapsed must not send"
            );
        }
    }

    /// ★ AT MOST ONE REQUEST IN FLIGHT, and the reply is what releases
    /// the next one.
    ///
    /// FALSIFIER: delete the `in_flight` early return in `poll` and this
    /// goes RED at the second poll.
    #[test]
    fn one_request_in_flight() {
        let mut s = InspectSubscription::new();
        s.set_subject(Some(uid(1)));

        let first = s.poll(0.0, all()).expect("the first poll must send");
        assert!(s.has_request_in_flight());
        // Nothing else goes out while it is outstanding, however long the
        // panel spins -- right up to the timeout.
        for t in 1..=20 {
            assert!(
                s.poll(f64::from(t) * 0.1, all()).is_none(),
                "a second request went out with one already in flight"
            );
        }
        // The reply releases it.
        assert!(s.accept(reply_to(&first)));
        assert!(!s.has_request_in_flight());
        assert!(s.latest().is_some());
    }

    /// A lost reply must not wedge the panel forever. A guard must not
    /// starve the thing it protects.
    ///
    /// FALSIFIER: remove the `IN_FLIGHT_TIMEOUT_SECS` arm and the panel
    /// never sends again after a dropped packet — this goes RED.
    #[test]
    fn a_lost_reply_does_not_wedge_the_panel() {
        let mut s = InspectSubscription::new();
        s.set_subject(Some(uid(1)));
        let _ = s.poll(0.0, all()).expect("first send");
        assert!(s.poll(IN_FLIGHT_TIMEOUT_SECS - 0.01, all()).is_none(), "too early");
        assert!(
            s.poll(IN_FLIGHT_TIMEOUT_SECS + 0.01, all()).is_some(),
            "a dropped reply must not silence the panel forever"
        );
    }

    /// ★ COLLAPSED SECTIONS ARE NOT REQUESTED. The request carries
    /// exactly the expanded set — never a helpful extra, because an extra
    /// section is server work and packet bytes for something nobody is
    /// looking at.
    ///
    /// FALSIFIER: make `due` iterate `SectionIdV1::ALL` without the
    /// `expanded.contains` filter and this goes RED.
    #[test]
    fn collapsed_sections_are_not_requested() {
        let mut s = InspectSubscription::new();
        s.set_subject(Some(uid(1)));
        let req = s.poll(0.0, SectionSetV1::empty().with(SectionIdV1::Identity)).expect("send");
        assert_eq!(req.sections.len(), 1);
        assert!(req.sections.contains(SectionIdV1::Identity));
        assert!(!req.sections.contains(SectionIdV1::Path), "a collapsed section was requested");
        assert!(!req.sections.contains(SectionIdV1::RightNow));
    }

    /// Live sections come round again before slow ones do, and the
    /// intervals come from `cadence()` rather than from a literal here.
    ///
    /// FALSIFIER: give `Identity` the `Live` cadence and the "Identity is
    /// not yet due" assertion goes RED.
    #[test]
    fn live_sections_refresh_faster_than_slow_ones() {
        let mut s = InspectSubscription::new();
        s.set_subject(Some(uid(1)));

        let first = s.poll(0.0, all()).expect("send");
        assert_eq!(first.sections, all(), "the first request asks for everything expanded");
        assert!(s.accept(reply_to(&first)));

        // At 0.6s the two Live sections are due; the Slow one is not.
        let second = s.poll(0.6, all()).expect("the live sections are due");
        assert!(second.sections.contains(SectionIdV1::RightNow));
        assert!(second.sections.contains(SectionIdV1::Path));
        assert!(
            !second.sections.contains(SectionIdV1::Identity),
            "a slow section must not ride along on the live cadence"
        );
        assert!(s.accept(reply_to(&second)));

        // Past the slow interval it comes back.
        let third = s.poll(2.5, all()).expect("everything is due again");
        assert!(third.sections.contains(SectionIdV1::Identity));
    }

    /// ★ A REPLY WHOSE SEQ DOES NOT MATCH IS DROPPED, and so is one for a
    /// colonist the player has moved on from. A late answer must never
    /// repaint the panel with someone else's rows.
    ///
    /// FALSIFIER: drop the seq comparison in `accept` and the stale-seq
    /// case goes RED; drop the subject comparison and the re-selection
    /// case goes RED.
    #[test]
    fn stale_replies_are_dropped() {
        let mut s = InspectSubscription::new();
        s.set_subject(Some(uid(1)));
        let req = s.poll(0.0, all()).expect("send");

        // Wrong seq.
        let mut wrong_seq = reply_to(&req);
        wrong_seq.seq = req.seq.wrapping_add(7);
        assert!(!s.accept(wrong_seq), "a mismatched seq must be dropped");
        assert!(s.latest().is_none());

        // Wrong subject.
        let mut wrong_subject = reply_to(&req);
        wrong_subject.subject = uid(999);
        assert!(!s.accept(wrong_subject), "a reply for another colonist must be dropped");
        assert!(s.latest().is_none());

        // The right one still lands.
        assert!(s.accept(reply_to(&req)));
        assert!(s.latest().is_some());

        // And a reply arriving after the player switched away is dropped
        // even though it was legitimately requested.
        let req2 = s.poll(10.0, all()).expect("send again");
        s.set_subject(Some(uid(2)));
        assert!(!s.accept(reply_to(&req2)), "a reply for the previous subject must be dropped");
    }

    /// Switching subject clears the cached reply IMMEDIATELY, so the
    /// panel never shows one colonist's rows under another's name.
    #[test]
    fn switching_subject_clears_the_cache() {
        let mut s = InspectSubscription::new();
        s.set_subject(Some(uid(1)));
        let req = s.poll(0.0, all()).expect("send");
        assert!(s.accept(reply_to(&req)));
        assert!(s.latest().is_some());

        s.set_subject(Some(uid(2)));
        assert!(s.latest().is_none(), "the previous colonist's rows must not survive the switch");
        // And the new subject is askable at once, not after the old
        // cadence expires.
        let next = s.poll(0.01, all()).expect("the new subject is asked immediately");
        assert_eq!(next.subject, uid(2));
        assert_eq!(next.sections, all());

        // Re-setting the SAME subject is a no-op and must not reset the
        // throttle (a player wiggling the mouse must not spam the server).
        s.set_subject(Some(uid(2)));
        assert!(s.has_request_in_flight(), "an identical set_subject must not clear state");
    }

    /// Deselecting stops the traffic and clears the panel.
    #[test]
    fn deselect_stops_traffic() {
        let mut s = InspectSubscription::new();
        s.set_subject(Some(uid(1)));
        let req = s.poll(0.0, all()).expect("send");
        assert!(s.accept(reply_to(&req)));

        s.set_subject(None);
        assert!(s.latest().is_none());
        for t in 0..50 {
            assert!(s.poll(10.0 + f64::from(t) * 0.1, all()).is_none());
        }
    }
}
