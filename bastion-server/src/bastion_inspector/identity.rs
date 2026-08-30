//! bastion (INSPECTOR-M1): the **Identity** section provider — WHO THIS IS.
//!
//! Everything here except `health` comes off the PERSISTENT colonist record
//! or the job board, so this section answers for an unloaded colonist too
//! (`SectionIdV1::available_unloaded` is `true`). That is the point of
//! keying selection on `Uid`: a colonist walking out of view should not
//! blank the panel that was describing them.

use common::comp::bastion_inspect::{IdentitySectionV1, SectionIdV1, SectionPayloadV1};

use super::{InspectCtx, lane_tables, not_a_colonist, unloaded};

/// Read the identity of the subject.
///
/// ★ NO HASHMAP ITERATION. Every board lookup here is a KEYED `get`
/// (`professions[uid]`, `beds[bed_pos]`), never a scan. A scan would make
/// the reply depend on `HashMap` iteration order, which differs between
/// map instances, so two servers answering the same question would emit
/// different bytes. The determinism pin in the parent module is what holds
/// this.
pub fn provide(ctx: &InspectCtx<'_>) -> SectionPayloadV1 {
    let Some(rec) = ctx.record else {
        // Two different absences, told apart rather than collapsed.
        //
        // ★ A KNOWN LIMIT OF THE CURRENT WIRING, stated here because it is
        // the kind of thing that otherwise gets rediscovered as a bug.
        // This provider can answer from the roster record ALONE — that is
        // what makes Identity `available_unloaded()`, and the provider
        // test proves it. But today the caller cannot HAND it a record for
        // an unloaded subject: `IdMaps::uid_entity` resolves only loaded
        // entities, and `rtsim::Npc::uid` is rtsim's own counter, not the
        // ECS `Uid`, so there is no `Uid -> NpcId` index to look the
        // record up with. Until one exists, an unloaded subject reaches
        // here with `record: None` and is told it is unloaded — which is
        // true, and is still strictly better than the pre-existing
        // behaviour of silently blanking the panel.
        return if ctx.loaded.is_none() {
            unloaded(SectionIdV1::Identity)
        } else {
            not_a_colonist(SectionIdV1::Identity)
        };
    };

    let (skills, desires) = lane_tables(rec);

    // The DURABLE record says which bed is theirs; the RUNTIME board says
    // who the slot thinks its owner is. Report whether the two agree
    // rather than picking a winner: a disagreement is a real finding (the
    // board is rebuilt from scratch at every server start, the record is
    // not), and an inspector that silently reconciled it would erase the
    // only symptom.
    let bed_slot_agrees = rec.owned_bed.map(|p| {
        ctx.board.beds.get(&p).is_some_and(|slot| slot.owner == Some(ctx.subject))
    });

    SectionPayloadV1::Identity(IdentitySectionV1 {
        name: rec.name.clone(),
        // RUNTIME-ONLY (`JobBoard::professions`): `None` on day 0, before
        // any lane work, or after a restart until the daily derivation
        // runs again. Not "no trade" — "not yet derived".
        profession: ctx.board.professions.get(&ctx.subject).copied(),
        born_tick: rec.born_tick,
        // Carried ONLY so the panel can show it under a boot-relative
        // label. Never used to compute an age.
        born_day_boot_relative: rec.born_day,
        parent_name: ctx.parent_name.clone(),
        backstory: rec.backstory.clone(),
        owned_bed: rec.owned_bed,
        bed_slot_agrees,
        // `None` = no ECS entity, or an entity with no `Health`. NOT zero
        // health: the `Option` is the difference between "unknown" and
        // "dying", which is not a distinction to lose in a UI.
        health: ctx.loaded.as_ref().and_then(|l| l.health),
        guard_bravery: rec.guard_bravery,
        skills,
        desires,
    })
}
