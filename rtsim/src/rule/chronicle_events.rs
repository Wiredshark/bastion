use crate::{
    RtState, Rule, RuleError,
    data::{ChronicleKind, Importance, Scope},
    event::{EventCtx, OnDeath, OnTheft},
};

/// bastion (HIST-1, row 54): the Chronicle's first event-bus emitters — a
/// SIBLING of [`super::report::ReportEvents`]: the SAME `OnDeath`/`OnTheft`
/// events, a second destination. `Reports` stays the ephemeral NPC-gossip
/// sink (witness-gated, decays); the `Chronicle` is the persistent
/// history. The two handlers fire independently off the bus and neither
/// touches the other — same event, two sinks, no new capture mechanism.
pub struct ChronicleEvents;

impl Rule for ChronicleEvents {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnDeath>(on_death);
        rtstate.bind::<Self, OnTheft>(on_theft);

        Ok(Self)
    }
}

fn on_death(ctx: EventCtx<ChronicleEvents, OnDeath>) {
    let data = &mut *ctx.state.data_mut();
    let now = data.time_of_day;
    // One event → one record (the conservation invariant). NO witness
    // gate here on purpose — Reports keep theirs (gossip needs someone
    // to gossip), but a death nobody saw still HAPPENED; history does
    // not need witnesses. The killer rides the actors list so the deed
    // appears in BOTH figures' histories (the thought-sum and the
    // legends browser both filter by `actors.contains`).
    let mut actors = vec![ctx.event.actor];
    if let Some(killer) = ctx.event.killer {
        actors.push(killer);
    }
    data.chronicle.record(
        now,
        ChronicleKind::Death,
        actors,
        None,
        ctx.event.wpos.map(|p| p.map(|e| e.floor() as i32)),
        Importance::Notable,
        Scope::World,
        None,
    );
}

fn on_theft(ctx: EventCtx<ChronicleEvents, OnTheft>) {
    let data = &mut *ctx.state.data_mut();
    let now = data.time_of_day;
    data.chronicle.record(
        now,
        ChronicleKind::Theft,
        vec![ctx.event.actor],
        ctx.event.site,
        Some(ctx.event.wpos),
        Importance::Notable,
        Scope::World,
        None,
    );
}
