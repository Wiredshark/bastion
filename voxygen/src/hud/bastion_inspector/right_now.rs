//! bastion (INSPECTOR-M1): the **Right Now** section view — WHAT THEY ARE
//! DOING THIS INSTANT.

use common::comp::bastion_inspect::{
    FrameV1, InspectFramesV1, InspectRow, SectionPayloadV1,
};

/// `Drive` has no `Display` and no `label()`, so the naming happens here.
/// No wildcard arm: a new drive must be named rather than silently
/// rendering as the last one.
const fn drive_label(d: common::comp::bastion::Drive) -> &'static str {
    use common::comp::bastion::Drive as D;
    match d {
        D::Work => "Working",
        D::Flee => "Fleeing",
        D::Idle => "Idle",
        D::Personal => "Personal (rest / eat / recreation)",
    }
}

/// No wildcard arm, same reason.
const fn state_label(s: common::comp::bastion::ActiveJobState) -> &'static str {
    use common::comp::bastion::ActiveJobState as S;
    match s {
        S::Traveling => "Traveling — walking to the site",
        S::Arrived => "Arrived — at the site",
        S::Waiting => "Waiting — queued at a single-file link (not stuck)",
    }
}

pub fn rows(payload: &SectionPayloadV1, _frames: &InspectFramesV1) -> Vec<InspectRow> {
    let SectionPayloadV1::RightNow(d) = payload else {
        return Vec::new();
    };
    let mut rows = Vec::with_capacity(14);

    rows.push(
        InspectRow::new(
            "Drive",
            drive_label(d.drive),
            "Arbiter::current",
            "",
            FrameV1::Ecs,
        )
        .scoped("Idle is also the default when no Arbiter exists"),
    );

    rows.push(
        InspectRow::new(
            "Urgencies",
            format!(
                "work {:.2} | flee {:.2} | idle {:.2}",
                d.last_scores.0, d.last_scores.1, d.last_scores.2
            ),
            "Arbiter::last_scores",
            "post-modulation urgency",
            FrameV1::Ecs,
        )
        .scoped("reported telemetry; the sim does not read these back"),
    );

    rows.push(
        InspectRow::new(
            "Activity",
            match d.activity {
                // ★ 0% DOES NOT MEAN STALLED. A claimant still walking to
                // the site reads `Some((work, 0.0))` -- the walk is part
                // of the work's story. The job phase row below is what
                // separates "on the way" from "arrived and not moving".
                Some((w, p)) => format!("{} {:.0}%", w.label(), p * 100.0),
                None => "not on a progress-bearing work job".to_string(),
            },
            "Arbiter::activity",
            "normalized 0..1",
            FrameV1::Ecs,
        )
        .scoped("normalized per job kind (Chop scales by wood count)"),
    );

    rows.push(
        InspectRow::new(
            "Status",
            match d.status {
                Some(s) => format!("{s:?}"),
                None => "none stamped in the last ~2s".to_string(),
            },
            "bastion_jobs::colonist_status",
            "",
            FrameV1::JobBoard,
        )
        // HONEST LIMIT, carried on the row itself rather than left for
        // the reader to discover: the stamp's TTL is about two seconds,
        // so a 2 Hz panel misses most occurrences. "None" is NOT "not
        // happening".
        .scoped("~2s TTL — absence is not evidence of absence"),
    );

    rows.push(InspectRow::new(
        "Position",
        match d.pos {
            Some(p) => format!("({:.1}, {:.1}, {:.1})", p.x, p.y, p.z),
            None => "unknown".to_string(),
        },
        "Pos",
        "blocks",
        FrameV1::Ecs,
    ));

    match &d.job {
        None => {
            rows.push(InspectRow::new(
                "Job",
                "none — no ActiveJob component",
                "ActiveJob",
                "",
                FrameV1::Ecs,
            ));
        },
        Some(j) => {
            rows.push(InspectRow::new(
                "Job id",
                j.id.to_string(),
                "ActiveJob::job (the JobBoard::jobs key)",
                "",
                FrameV1::Ecs,
            ));
            rows.push(InspectRow::new(
                "Job phase",
                state_label(j.state),
                "ActiveJob::state",
                "",
                FrameV1::Ecs,
            ));
            rows.push(
                InspectRow::new(
                    "Job kind",
                    match (&j.kind, j.work) {
                        (Some(k), Some(w)) => format!("{k:?} ({})", w.label()),
                        (Some(k), None) => format!("{k:?}"),
                        // A REPORTABLE state, not an error: the colonist
                        // still points at a job the board has completed
                        // or reaped.
                        (None, _) => {
                            "id no longer on the board (completed or reaped)".to_string()
                        },
                    },
                    "JobBoard::jobs[id].kind / .work",
                    "",
                    FrameV1::JobBoard,
                )
                .scoped("runtime-only board"),
            );
            rows.push(InspectRow::new(
                "Job position",
                match j.pos {
                    Some(p) => format!("({}, {}, {})", p.x, p.y, p.z),
                    None => "unknown".to_string(),
                },
                "JobBoard::jobs[id].pos",
                "blocks",
                FrameV1::JobBoard,
            ));
            rows.push(
                InspectRow::new(
                    "Distance",
                    match j.distance {
                        Some(dist) => format!(
                            "{dist:.1} (arrives within {:.1})",
                            j.arrive_dist
                        ),
                        None => "unknown".to_string(),
                    },
                    "|Pos - (job.pos + ActiveJob::stance)|",
                    "blocks",
                    FrameV1::Derived,
                )
                // The stance offset is why this is not simply the
                // distance to `job.pos`; saying so keeps a reader from
                // "correcting" it against their own mental model.
                .scoped("to the STANCE cell, not to job.pos"),
            );
            if let Some(p) = j.raw_progress {
                rows.push(
                    InspectRow::new(
                        "Raw progress",
                        format!("{p:.2}"),
                        "JobBoard::jobs[id].progress",
                        "raw work units",
                        FrameV1::JobBoard,
                    )
                    .scoped("NOT a fraction — the denominator differs by job kind"),
                );
            }
            if j.unreachable == Some(true) {
                rows.push(InspectRow::new(
                    "Reachability",
                    "marked UNREACHABLE",
                    "JobBoard::jobs[id].unreachable",
                    "",
                    FrameV1::JobBoard,
                ));
            }
            if j.needs_materials == Some(true) {
                rows.push(InspectRow::new(
                    "Blocked",
                    "waiting on materials",
                    "JobBoard::jobs[id].needs_materials",
                    "",
                    FrameV1::JobBoard,
                ));
            }
            rows.push(
                InspectRow::new(
                    "Claimed by",
                    match j.claimed_by {
                        Some(u) => u.to_string(),
                        None => "unclaimed".to_string(),
                    },
                    "JobBoard::jobs[id].claimed_by",
                    "uid",
                    FrameV1::JobBoard,
                )
                // Can legitimately differ from the subject, and that
                // difference is a finding rather than a glitch.
                .scoped("may differ from this colonist — the claim can move"),
            );
        },
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{
        bastion::WorkType,
        comp::bastion::{ActiveJobState, Drive},
        comp::bastion_inspect::{ActiveJobViewV1, RightNowSectionV1},
    };

    fn frames() -> InspectFramesV1 {
        InspectFramesV1 {
            server_tick: 0,
            rtsim_tick: 0,
            time_of_day: 0.0,
            ticks_per_game_day: 54_000.0,
            schedule_offset_hours: 0,
        }
    }

    fn job_at(state: ActiveJobState) -> ActiveJobViewV1 {
        ActiveJobViewV1 {
            id: 7,
            state,
            kind: None,
            work: Some(WorkType::Chop),
            pos: Some(vek::Vec3::new(9, 9, 9)),
            distance: Some(12.5),
            arrive_dist: 2.5,
            raw_progress: Some(0.0),
            unreachable: None,
            needs_materials: None,
            claimed_by: None,
        }
    }

    fn with_job(job: Option<ActiveJobViewV1>) -> SectionPayloadV1 {
        SectionPayloadV1::RightNow(RightNowSectionV1 {
            drive: Drive::Work,
            last_scores: (1.0, 0.0, 0.25),
            activity: Some((WorkType::Chop, 0.0)),
            status: None,
            pos: Some(vek::Vec3::new(1.0, 2.0, 3.0)),
            job,
        })
    }

    /// The phase row is what separates "on the way" from "arrived and
    /// stuck" — the pair of facts a 0% activity bar alone cannot
    /// distinguish.
    ///
    /// FALSIFIER: drop the phase row and this goes RED.
    #[test]
    fn traveling_at_zero_percent_is_distinguishable_from_stalled() {
        let r = rows(&with_job(Some(job_at(ActiveJobState::Traveling))), &frames());
        let act = r.iter().find(|x| x.label() == "Activity").expect("activity row");
        assert_eq!(act.value(), "chop 0%");
        let phase = r.iter().find(|x| x.label() == "Job phase").expect("phase row");
        assert!(phase.value().contains("Traveling"));

        // Arrived at the same 0% reads differently.
        let r2 = rows(&with_job(Some(job_at(ActiveJobState::Arrived))), &frames());
        let phase2 = r2.iter().find(|x| x.label() == "Job phase").expect("phase row");
        assert!(phase2.value().contains("Arrived"));
        assert_ne!(phase.value(), phase2.value());
    }

    /// A job id that no longer resolves is REPORTED, not hidden behind a
    /// default.
    ///
    /// FALSIFIER: render `None` kind as an empty string and this goes RED.
    #[test]
    fn a_reaped_job_id_is_reported() {
        let mut j = job_at(ActiveJobState::Arrived);
        // The board no longer resolves the id.
        j.kind = None;
        j.work = None;
        j.pos = None;
        let r = rows(&with_job(Some(j)), &frames());
        let kind = r.iter().find(|x| x.label() == "Job kind").expect("kind row");
        assert!(kind.value().contains("no longer on the board"));
    }

    /// The status row carries its own TTL caveat, so a reader cannot take
    /// `None` as "not happening".
    #[test]
    fn status_none_carries_its_ttl_caveat() {
        let r = rows(&with_job(None), &frames());
        let s = r.iter().find(|x| x.label() == "Status").expect("status row");
        assert!(s.value().contains("none stamped"));
        assert!(s.scope().is_some_and(|x| x.contains("absence is not evidence")));
    }
}
