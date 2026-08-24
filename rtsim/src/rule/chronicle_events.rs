use crate::{
    RtState, Rule, RuleError,
    data::{ChronicleKind, Importance, Scope},
    event::{EventCtx, OnDeath, OnHealthChange, OnTheft},
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
        rtstate.bind::<Self, OnHealthChange>(on_wounded);
        rtstate.bind::<Self, OnTheft>(on_theft);

        Ok(Self)
    }
}

/// bastion (looking-sweep row): the diary SEES violence now. One record
/// per wound-line CROSSING (prev >= 0.5 > new, hostile cause required), so
/// a flurry of blows in one fight records once on the way down — recover
/// above the line and a later mauling records again. Witnesses ride the
/// same actors list as Death (the sweep's cook was mauled twice with no
/// record and no one to remember it).
fn on_wounded(ctx: EventCtx<ChronicleEvents, OnHealthChange>) {
    const WOUND_LINE: f32 = 0.5;
    let ev = &ctx.event;
    let Some(cause) = ev.cause else { return };
    if !(ev.change < 0.0
        && ev.old_health_fraction >= WOUND_LINE
        && ev.new_health_fraction < WOUND_LINE)
    {
        return;
    }
    let data = &mut *ctx.state.data_mut();
    let now = data.time_of_day;
    let wpos = match ev.actor {
        common::rtsim::Actor::Npc(id) => data.npcs.get(id).map(|n| n.wpos),
        _ => None,
    };
    let mut actors = vec![ev.actor, cause];
    if let Some(wpos) = wpos {
        actors.extend(witness_actors(
            data.npcs.nearby(None, wpos, 32.0),
            ev.actor,
            Some(cause),
        ));
    }
    data.chronicle.record(
        now,
        ChronicleKind::Wounded,
        actors,
        None,
        wpos.map(|p| p.map(|e| e.floor() as i32)),
        Importance::Routine,
        Scope::World,
        None,
    );
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
    // ★ ITEM 36 v1 (witnesses): who SAW it dies a little too. The Death
    // thought is filtered by `actors.contains`, so before this only the
    // victim and the killer carried it — a colonist could watch a friend
    // fall two blocks away and feel nothing. Witnesses ride the same
    // actors list (the locked schema's one per-figure channel; "I saw
    // them fall" becoming part of a witness's own history is the DF
    // engraving shape, not an accident). The 32-block radius is the
    // sibling report rule's own witness scan, reused verbatim. Sorted
    // (Actor is Ord for exactly this) and capped so a crowd cannot
    // bloat the record: the six nearest-by-id is a deterministic set,
    // where "whichever six the grid yielded first" is not.
    if let Some(wpos) = ctx.event.wpos {
        actors.extend(witness_actors(
            data.npcs.nearby(None, wpos, 32.0),
            ctx.event.actor,
            ctx.event.killer,
        ));
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

/// ITEM 36 v1: the witness set for a death record — everyone nearby except
/// the victim and the killer (both already lead the actors list), SORTED
/// (`Actor` is `Ord` for exactly this) and capped, so the set is a
/// deterministic function of who was there, never of grid iteration order.
/// Capped at six: a crowd must not bloat a locked-schema record.
fn witness_actors(
    nearby: impl Iterator<Item = common::rtsim::Actor>,
    victim: common::rtsim::Actor,
    killer: Option<common::rtsim::Actor>,
) -> Vec<common::rtsim::Actor> {
    let mut witnesses: Vec<_> = nearby
        .filter(|a| *a != victim && Some(*a) != killer)
        .collect();
    witnesses.sort();
    witnesses.truncate(6);
    witnesses
}

#[cfg(test)]
mod tests {
    use super::witness_actors;
    use common::rtsim::Actor;

    /// ★ ITEM 36 v1: witnesses are DERIVED, deterministic, and never
    /// double-count the principals. The victim and killer already lead the
    /// actors list — re-adding them would double their Death thought; the
    /// sort makes the record independent of spatial-grid yield order; the
    /// cap keeps a crowd from bloating the locked schema.
    #[test]
    fn witnesses_exclude_principals_sort_and_cap() {
        // NpcIds are slotmap keys; fabricate distinct ones via a real map.
        let mut sm: slotmap::SlotMap<crate::data::NpcId, ()> = slotmap::SlotMap::with_key();
        let ids: Vec<_> = (0..10).map(|_| sm.insert(())).collect();
        let victim = Actor::Npc(ids[0]);
        let killer = Actor::Npc(ids[1]);

        // Fed in reverse order: the output must not care.
        let out = witness_actors(
            ids.iter().rev().map(|i| Actor::Npc(*i)),
            victim,
            Some(killer),
        );
        assert!(!out.contains(&victim), "the victim is not their own witness");
        assert!(!out.contains(&killer), "the killer already leads the actors list");
        assert_eq!(out.len(), 6, "a crowd of 8 eligible caps at 6");
        let mut sorted = out.clone();
        sorted.sort();
        assert_eq!(out, sorted, "the set is ordered, not grid-yield-ordered");

        // A death with no killer excludes only the victim.
        let out = witness_actors(ids[..3].iter().map(|i| Actor::Npc(*i)), victim, None);
        assert_eq!(out.len(), 2);
    }
}
