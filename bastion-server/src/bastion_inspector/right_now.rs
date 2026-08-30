//! bastion (INSPECTOR-M1): the **Right Now** section provider — WHAT THEY
//! ARE DOING THIS INSTANT.
//!
//! Pure ECS frame. Every field here lives on a component of a LOADED
//! entity, so this section refuses for an unloaded subject rather than
//! inventing a default — `Drive::Idle` for someone the server cannot see
//! would be a lie with the shape of an answer.

use common::comp::bastion_inspect::{
    ActiveJobViewV1, RightNowSectionV1, SectionIdV1, SectionPayloadV1,
};

use super::{InspectCtx, not_a_colonist, unloaded};

pub fn provide(ctx: &InspectCtx<'_>) -> SectionPayloadV1 {
    if ctx.record.is_none() {
        return not_a_colonist(SectionIdV1::RightNow);
    }
    let Some(l) = ctx.loaded.as_ref() else {
        return unloaded(SectionIdV1::RightNow);
    };

    // `Drive::Idle` IS the documented default when no `Arbiter` exists
    // (the existing wire fill does the same), so this collapse is
    // faithful rather than a papered-over absence.
    let drive = l.arbiter.map_or(common::comp::bastion::Drive::Idle, |a| a.current);

    let job = l.active_job.map(|aj| {
        // The id is the `JobBoard::jobs` KEY -- `Job` has no id field.
        // `None` here is a REPORTABLE state, not an error: the colonist
        // still points at a job that has completed or been reaped.
        let job = ctx.board.jobs.get(&aj.job);

        // ★ THE STANCE OFFSET IS LOAD-BEARING. The arrival target is
        // `job.pos + ActiveJob::stance`, not `job.pos`. The default
        // stance is `(0,0,1)` (stand on top), but Farm's sow/harvest and
        // the ladder mounts use others -- assuming the default would
        // misreport the distance for exactly the jobs whose distance is
        // interesting.
        let target = job.map(|j| {
            (j.pos + aj.stance).map(|e| e as f32) + vek::Vec3::new(0.5, 0.5, 0.0)
        });
        let distance = match (l.pos, target) {
            (Some(p), Some(t)) => Some(p.distance(t)),
            _ => None,
        };

        ActiveJobViewV1 {
            id: aj.job,
            state: aj.state,
            kind: job.map(|j| common::comp::bastion::JobKindTagV1::from(&j.kind)),
            work: job.map(|j| j.work),
            pos: job.map(|j| j.pos),
            distance,
            // Sent rather than hard-coded client-side so the panel cannot
            // hold a rival copy of the arrival threshold that drifts from
            // the one the sim actually applies.
            arrive_dist: crate::bastion_jobs::ARRIVE_DIST,
            // RAW work, not a fraction: the denominator differs by job
            // kind (Chop scales by the tree's wood count). The normalized
            // bar is `activity` below.
            raw_progress: job.map(|j| j.progress),
            unreachable: job.map(|j| j.unreachable),
            needs_materials: job.map(|j| j.needs_materials),
            // Can legitimately differ from the subject: an `ActiveJob`
            // can outlive the claim it refers to.
            claimed_by: job.and_then(|j| j.claimed_by),
        }
    });

    SectionPayloadV1::RightNow(RightNowSectionV1 {
        drive,
        last_scores: l.arbiter.map_or((0.0, 0.0, 0.0), |a| a.last_scores),
        // Normalized 0..1. A TRAVELING claimant reads `Some((work, 0.0))`
        // -- "Hauling 0%" means "on the way", which is why the job phase
        // beside it is what disambiguates progress from stalling.
        activity: l.arbiter.and_then(|a| a.activity),
        // HONEST LIMIT, restated at the producer: the stamp lives about
        // two seconds, so a 2 Hz panel reads `None` most of the time even
        // while the state is occurring. `None` is "not stamped
        // recently", never "not happening".
        status: crate::bastion_jobs::colonist_status(
            ctx.board,
            ctx.subject,
            ctx.frames.server_tick,
        ),
        pos: l.pos,
        job,
    })
}
