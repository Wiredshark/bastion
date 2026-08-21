//! The entity event log (DECISIONS #99, `ROW-ENTITY-EVENT-LOG-PACKET`).
//!
//! **Stage 1: the stream, the per-entity ring, the retention flag.** Gated
//! behind stage 1's paired determinism-floor gate going green (attested,
//! 2026-08-10, `dda63e376`): **stage 2 ground-phase producers have landed**
//! -- `record_pickup_event` (`server/src/events/inventory_manip.rs`,
//! `ItemEventKind::PickedUp` on the two state-changing verdicts only) and
//! the `to_release` drain choke point (`bastion_jobs.rs:14281`,
//! `ColonistEventKind::Released`, Measure 0's producer). Item scope is
//! GROUND-PHASE ONLY (`Created`/`Dropped`/`PickedUp`/`Reserved`/
//! `Released`/`Despawned`) -- `Consumed`/`Split`/`Merged` are deliberately
//! NOT wired: neither `Uid` nor `PickupItem`'s `ItemInstanceId` survives an
//! item's pickup (traced 2026-08-10; `ItemInstanceId` lives only on
//! `PickupItem`, is re-minted fresh on every re-drop, and is read nowhere
//! in the tree today), so there is no subject to key an in-inventory event
//! against until that identity gap closes (routed to Fable as its own row).
//! Persistence (stage 3 -- save/load across the retention boundary) is
//! still deliberately NOT part of this module.
//!
//! **Naming (Opus's ruling, 2026-08-10, packet `55cdeb003c`):** this
//! mechanism was originally called "promotion." Renamed to RETENTION before
//! stage 3 opens -- `comp::bastion::BastionColonist`'s own doc already uses
//! "promoted" for an unrelated fact (an rtsim `Npc` going from simulated-
//! but-unloaded to a loaded ECS entity), and a reader who knows that usage
//! would confidently misread "colonist promoted" here as a load event.
//! "Retained" names the MECHANISM (kept vs. ring-evicted), not a judgement
//! about the entity, so it can't drift back into the collision.
//!
//! Disabled unless `BASTION_ENTITY_EVENT_LOG` is set: no init, no
//! allocation beyond the (empty) process-global slot, no ECS mutation, no
//! scheduling change when off -- same posture as `bastion_flight_recorder`,
//! reused deliberately (see the design doc's "what is genuinely reusable").
//!
//! ## Chassis correction the design's first draft got wrong
//!
//! `bastion_flight_recorder` was proposed as this module's likely chassis.
//! It is not: it is cap-and-DROP-NEWEST, single-`uid_filter` (one entity),
//! JSONL-to-file. Drop-newest preserves a run's BEGINNING (right for a
//! startup bug, wrong for "what happened to this item"), and a global cap
//! would let one flooding entity starve every other entity's history. This
//! module is instead PER-ENTITY, oldest-out, multi-entity by construction --
//! the three things the design doc names as genuinely new here.

use crate::bastion_jobs::ReleaseReason;
use common::uid::Uid;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
};

/// Design #99 §2: ring size is TUNABLE and must be measured, not guessed --
/// the first fan after this lands reports bytes/sim-hour at this default.
const DEFAULT_RING_SIZE: usize = 64;

/// Explicit, VERSIONED wire copy of `bastion_jobs::ReleaseReason` (Opus's
/// catch, 2026-08-10): `JobBoard` is documented runtime-only (not
/// serialized, not recorder-sampled) and this enum is actively growing
/// (`TargetChanged` added 2026-08-04 as a 4th producer). Deriving serde on
/// the LIVE gameplay enum directly would freeze a still-discovered type
/// into a save-format schema the moment stage 3's retained-entity
/// persistence lands -- every future variant add/rename would become a
/// save migration on a type whose whole history has been "edit it when the
/// job board needs a new reason." This copy decouples the wire format:
/// adding `ReleaseReason::Whatever` next month is a one-line arm here, not
/// a migration. `#[non_exhaustive]` so a future gameplay variant that
/// hasn't been given a wire mapping yet fails to compile at the `From`
/// impl below rather than silently landing as `Other`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReleaseReasonV1 {
    Other,
    TimedOut,
    Completed,
    RemovedExternally,
    TargetChanged,
}

// DO NOT ADD A `_ => Self::Other` ARM (Opus's confirm, 2026-08-10): the
// compile-failure-on-new-variant guarantee comes ENTIRELY from this match
// being exhaustive with no wildcard, not from `#[non_exhaustive]` above
// (which only constrains external consumers of the wire type, forward-
// compat for THEM, not a guard on THIS mapping). A wildcard here would let
// a future `ReleaseReason::Whatever` compile silently and land as `Other`
// -- exactly the failure this type exists to prevent. If a build breaks
// here, that is the guard working: choose the new variant's wire mapping,
// don't silence it with `_`.
impl From<ReleaseReason> for ReleaseReasonV1 {
    fn from(reason: ReleaseReason) -> Self {
        match reason {
            ReleaseReason::Other => Self::Other,
            ReleaseReason::TimedOut => Self::TimedOut,
            ReleaseReason::Completed => Self::Completed,
            ReleaseReason::RemovedExternally => Self::RemovedExternally,
            ReleaseReason::TargetChanged => Self::TargetChanged,
        }
    }
}

/// ITEM class vocabulary (design #99 §2) -- closed, typed, never a free
/// string. `by`/`actor` semantics live on `EntityEvent::actor`, not
/// duplicated per-variant: an item's `Dropped` event names its subject (the
/// item) and its actor (who dropped it) via the one shared field, not two
/// copies of the same fact.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ItemEventKind {
    Created,
    Dropped,
    /// The item left the pile into the actor's possession. TWO producers by
    /// design, same meaning on both: the vanilla `InventoryManip::Pickup`
    /// handler (`server/src/events/inventory_manip.rs`'s
    /// `record_pickup_event`, `"partial"`/`"accepted"` verdicts) and the
    /// eat path's direct consumption (`bastion_jobs.rs`'s `EatFrom`
    /// completion, ITEM8-CRASH-FINDING.md's fix) -- a consumer reading this
    /// event answers "did this pile lose a unit to this actor," never
    /// "which code path moved it." Subject = the pile's own uid on both
    /// paths (never a synthetic per-pickup entity); actor = the recipient.
    PickedUp,
    Reserved,
    /// PROVISIONAL vocabulary (Opus's catch, 2026-08-10): an earlier draft
    /// reused `bastion_jobs::ReleaseReason` here, but that enum is
    /// JOB-CLAIM semantics (`Completed`, `TargetChanged`, ...) -- a
    /// different fact about a different subject than an ITEM's
    /// *reservation* being released. `ItemReleaseReason` below is a
    /// deliberately small, separately-named closed set, honest about not
    /// yet knowing every real reservation-release reason stage 2's actual
    /// producers will need; extend it there, do not borrow the colonist
    /// vocabulary to fill the gap.
    Released { reason: ItemReleaseReason },
    Consumed,
    Despawned { cause: DespawnCause },
    Split,
    Merged,
}

/// Closed, PROVISIONAL vocabulary for `ItemEventKind::Released` -- an
/// item's reservation being released, not a job claim. See that variant's
/// own doc for why this is a separate set from `ReleaseReasonV1`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ItemReleaseReason {
    /// The reserving colonist's claim ended (timeout, completion, or
    /// otherwise) without consuming the item.
    ReservingClaimEnded,
    Other,
}

/// Closed cause vocabulary for `ItemEventKind::Despawned`. Provisional set
/// covering what's known today; stage 2's real producers may need to add a
/// variant, never a free string in its place.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DespawnCause {
    Timeout,
    Consumed,
    Other,
}

/// COLONIST class vocabulary (design #99 §2) -- closed, typed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ColonistEventKind {
    Claimed { job: common::bastion::JobId },
    /// ★ Measure 0's REQUIRED field (architect-ruled, packet §Measure 0):
    /// `tick`/`actor` are the shared `EntityEvent` fields this variant
    /// rides on, not duplicated here -- `job` and `reason` are this
    /// variant's own data. `reason` carries the SAME vocabulary
    /// `bastion_jobs::ReleaseReason` already is -- `ReleaseReasonV1` is its
    /// versioned wire copy (see that enum's own doc for why a direct
    /// derive on the live gameplay type was wrong), constructed via
    /// `.into()` at the real producer site (stage 2), not a second
    /// definition of the same closed set. `queue_position` (Opus's
    /// request, 2026-08-10, seed 69's customer): whether the released
    /// entity was a member of an active traversal-queue link at the
    /// moment of release, and at what position (0 = head) -- a genuine
    /// `Option`, not a defaulted 0, since most `Released` events are never
    /// queue members at all; absence must not read as "queued at the
    /// front."
    Released {
        job: common::bastion::JobId,
        reason: ReleaseReasonV1,
        queue_position: Option<usize>,
    },
    Preempted { need: NeedKind },
    Teleported { cause: TeleportCause },
    NeedCrossed { need: NeedKind, dir: CrossDirection },
    Ate { item: Uid },
    Stuck { cause: StuckCause },
}

/// Closed vocabulary for which of a colonist's three tracked needs an event
/// concerns (`comp::bastion::Needs { hunger, rest, recreation }`'s own
/// fields, named).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NeedKind {
    Hunger,
    Rest,
    Recreation,
}

/// Closed vocabulary for `NeedCrossed`'s direction -- crossed INTO the
/// interrupt band (need is now critical) or back OUT of it (satisfied).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrossDirection {
    Into,
    OutOf,
}

/// Closed cause vocabulary for `Teleported`. Provisional (see
/// `DespawnCause`'s own note on stage-2 producers extending, not
/// free-stringing, this set).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeleportCause {
    UltimateFailSafe,
    Other,
}

/// Closed cause vocabulary for `Stuck`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StuckCause {
    NoProgress,
    Other,
}

/// The one event kind, tagged by class. Deliberately NOT `AMBIENT` --
/// pilot scope is items + colonists only (design #99, "PILOT SCOPE").
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EventKind {
    Item(ItemEventKind),
    Colonist(ColonistEventKind),
}

/// `EntityEvent { tick, subject, kind, actor, data }` per the design --
/// `data` in the design's prose is each `EventKind` variant's OWN typed
/// payload (`job`, `reason`, `cause`, ...), not a separate field; storing
/// it twice would be exactly the duplicated-fact shape the design forbids.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityEvent {
    pub schema: String,
    pub tick: u64,
    pub subject: Uid,
    pub kind: EventKind,
    /// The second uid: the picker, the claimant, the killer. `None` when
    /// no actor applies (e.g. `Created`, `Stuck`).
    pub actor: Option<Uid>,
}

const SCHEMA: &str = "bastion.entity-event/v1";

/// Per-entity ring: bounded, OLDEST-OUT (not the flight recorder's
/// drop-newest -- see this module's own doc for why that policy is wrong
/// here). The truncation flag is carried per-entity, reusing the
/// self-accounting law `bastion_flight_recorder` already implements: a ring
/// that silently discarded history must never be indistinguishable from an
/// entity that had none.
#[derive(Debug, Default)]
struct EntityRing {
    events: VecDeque<EntityEvent>,
    truncated: bool,
}

impl EntityRing {
    fn push(&mut self, event: EntityEvent, cap: usize) {
        if self.events.len() >= cap {
            self.events.pop_front();
            self.truncated = true;
        }
        self.events.push_back(event);
    }
}

/// Stage 1's retention mechanism: a flag plus a move from ring to
/// permanent storage (design #99 §3, "Promotion is one-way and cheap" --
/// the design's original vocabulary; the MECHANISM is unchanged, only its
/// name here, per the rename above). Stage 1 builds the STORAGE branch and
/// the API to trigger it; stage 2/3 wire the actual retention TRIGGERS
/// (colonist named, item crafted/gifted/player-touched, ...) and
/// cross-restart persistence. An entity here with `retained == true` has
/// its full history in `permanent`, unbounded, and no longer touches its
/// (now-empty) ring.
#[derive(Debug, Default)]
struct EntityStore {
    rings: HashMap<Uid, EntityRing>,
    permanent: HashMap<Uid, Vec<EntityEvent>>,
    /// Value = whether this entity's ring had ALREADY truncated at the
    /// moment it was retained (Opus's catch, 2026-08-10): retaining an
    /// entity mid-truncation must carry that fact forward, or a retained
    /// entity -- exactly the one whose history someone will trust --
    /// silently claims complete history it doesn't have. The permanent
    /// store itself never truncates once retained; this records only the
    /// pre-retention gap, if any.
    retained: HashMap<Uid, bool>,
    ring_size: usize,
}

impl EntityStore {
    fn new(ring_size: usize) -> Self {
        Self { ring_size: ring_size.max(1), ..Default::default() }
    }

    fn record(&mut self, event: EntityEvent) {
        let subject = event.subject;
        if self.retained.contains_key(&subject) {
            self.permanent.entry(subject).or_default().push(event);
        } else {
            self.rings
                .entry(subject)
                .or_default()
                .push(event, self.ring_size);
        }
    }

    /// One-way: move this entity's ring contents into permanent storage
    /// and mark it so all future events for this uid go straight there,
    /// unbounded. Idempotent -- retaining an already-retained entity is a
    /// no-op past the first call.
    fn retain(&mut self, subject: Uid) {
        if self.retained.contains_key(&subject) {
            return;
        }
        let had_gap = if let Some(ring) = self.rings.remove(&subject) {
            let had_gap = ring.truncated;
            self.permanent
                .entry(subject)
                .or_default()
                .extend(ring.events);
            had_gap
        } else {
            false
        };
        self.retained.insert(subject, had_gap);
    }

    fn is_retained(&self, subject: Uid) -> bool { self.retained.contains_key(&subject) }

    /// All recorded events for `subject`, in tick order, from whichever
    /// store currently holds them (ring for un-retained, permanent for
    /// retained -- transparent to the caller). This is the Voonoo query's
    /// underlying read: `events_for(uid).filter(|e| e.actor.is_some())`
    /// answers "every actor that touched it, in order" directly, since
    /// both stores are already tick-ordered by construction (append-only).
    fn events_for(&self, subject: Uid) -> Vec<EntityEvent> {
        if let Some(events) = self.permanent.get(&subject) {
            events.clone()
        } else if let Some(ring) = self.rings.get(&subject) {
            ring.events.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// True if `subject` has EVER dropped an event to make room for a
    /// newer one -- either its live (un-retained) ring right now, or the
    /// gap it carried across retention. A retained entity's own permanent
    /// store never truncates again once retained; this is the pre-
    /// retention history, not an ongoing one.
    fn truncated(&self, subject: Uid) -> bool {
        if let Some(&had_gap) = self.retained.get(&subject) {
            had_gap
        } else {
            self.rings.get(&subject).is_some_and(|ring| ring.truncated)
        }
    }

    /// Total events currently held, ring + permanent, across every entity.
    /// Pure summation, split out from `event_count()` so the arithmetic is
    /// unit-testable directly against a constructed store, the same pattern
    /// every other method on this type already uses.
    fn event_count(&self) -> u64 {
        let rings: usize = self.rings.values().map(|r| r.events.len()).sum();
        let permanent: usize = self.permanent.values().map(|v| v.len()).sum();
        (rings + permanent) as u64
    }

    /// Every `ColonistEventKind::Released` event across EVERY entity (ring
    /// and retained alike), tick-ordered, capped at `cap` most-recent
    /// (oldest-out, matching the ring's own eviction policy for
    /// consistency) -- Opus's request, 2026-08-10, "the aggregate-late law
    /// landing on the instrument built to escape it": a run-total count
    /// answers neither of the pilot's two registered customers (Measure
    /// 0's cluster-vs-uniform question over ticks; seed 69's per-event
    /// queue-position detail), so this crosses subjects deliberately --
    /// unlike `events_for`, which is intentionally per-entity. Returns
    /// `(records, truncated)`; `truncated` is true when the true count
    /// exceeds `cap`, so a capped list can never render identically to a
    /// complete one -- the same self-accounting law the per-entity ring
    /// already carries, applied here.
    fn released_records(&self, cap: usize) -> (Vec<ReleasedRecord>, bool) {
        let mut all: Vec<ReleasedRecord> = Vec::new();
        let mut collect = |subject: Uid, event: &EntityEvent| {
            if let EventKind::Colonist(ColonistEventKind::Released { job, reason, queue_position }) =
                &event.kind
            {
                all.push(ReleasedRecord {
                    subject,
                    tick: event.tick,
                    job: *job,
                    reason: *reason,
                    queue_position: *queue_position,
                });
            }
        };
        for (&subject, ring) in &self.rings {
            for event in &ring.events {
                collect(subject, event);
            }
        }
        for (&subject, events) in &self.permanent {
            for event in events {
                collect(subject, event);
            }
        }
        // ORDER (#84, 2026-08-17): sorted by a TOTAL key, not by `tick` alone.
        //
        // `tick` is NOT unique -- two subjects can be released on the same
        // tick -- and `sort_by_key` is STABLE, so equal ticks keep their input
        // order. That input order comes from iterating `self.rings` and
        // `self.permanent`, both `HashMap`s, so the tie-break was hash order
        // and this list reached the harness JSON (`b5_eelog_released_events`)
        // as an array that `holdcheck` compares WHOLE.
        //
        // ★ Found by applying the rule written for the travel_timeout fix an
        // hour earlier -- "a non-unique sort key leaves the tie hash-dependent,
        // the same defect one step further in" -- to the one list field that
        // was VACUOUS (always empty) in the seeds that scored that fix. An
        // empty list hides this perfectly: it cannot move, so no wave has ever
        // reported it, and it would have surfaced only once some scenario
        // released two colonists on one tick.
        all.sort_by_key(|r| (r.tick, r.subject.0.get(), r.job));
        let truncated = all.len() > cap;
        if truncated {
            let start = all.len() - cap;
            all = all.split_off(start);
        }
        (all, truncated)
    }
}

/// One `ColonistEventKind::Released` event, flattened for cross-entity
/// export -- see [`EntityStore::released_records`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReleasedRecord {
    pub subject: Uid,
    pub tick: u64,
    pub job: common::bastion::JobId,
    pub reason: ReleaseReasonV1,
    pub queue_position: Option<usize>,
}

static LOG: OnceLock<Mutex<Option<EntityStore>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<EntityStore>> { LOG.get_or_init(|| Mutex::new(None)) }

fn ring_size_from_env() -> usize {
    std::env::var("BASTION_ENTITY_EVENT_LOG_RING_SIZE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_RING_SIZE)
        .max(1)
}

/// ON by default since 2026-08-21; `BASTION_ENTITY_EVENT_LOG=0` (or
/// `off`/`false`/`no`) is the kill switch. Checking this must not itself
/// initialize anything -- matches `bastion_flight_recorder::enabled()`'s own
/// contract (an env read, no lock taken unless a prior call already
/// initialized the slot).
pub fn enabled() -> bool {
    // ★ DEFAULT-ON WITH A NAMED KILL SWITCH (2026-08-21, found by playing):
    // two independent play sessions opened a colonist's story and got
    // `CHRONICLE enabled=false rows=0` for every single colonist — the one
    // surface that could tell a player WHY anything happened shipped switched
    // off, so a smiting, a starvation and a night in a bed all recorded
    // nothing. The cost is bounded and known: a 64-event ring per entity
    // (DEFAULT_RING_SIZE), which is why default-on is affordable at all.
    // `BASTION_ENTITY_EVENT_LOG=0` (or `off`/`false`) is the kill switch —
    // the same on-by-default-with-a-named-switch shape the haul-deadlock fix
    // was ruled into.
    !matches!(
        std::env::var("BASTION_ENTITY_EVENT_LOG").as_deref().map(str::trim),
        Ok("0") | Ok("off") | Ok("false") | Ok("no")
    )
}

fn with_store(f: impl FnOnce(&mut EntityStore)) {
    let Ok(mut guard) = slot().lock() else {
        return;
    };
    if guard.is_none() {
        *guard = Some(EntityStore::new(ring_size_from_env()));
    }
    if let Some(store) = guard.as_mut() {
        f(store);
    }
}

/// Event-driven ONLY -- never call this from a per-tick sweep (design #99
/// §1, "the density budget is non-negotiable"). Callers must capture the
/// values `kind`/`actor` describe BEFORE the mutation they narrate, not
/// after (design #99 §5) -- this function takes owned values, not
/// references into live ECS state, specifically so a caller cannot
/// accidentally read post-mutation state through it.
pub fn record_event(tick: u64, subject: Uid, kind: EventKind, actor: Option<Uid>) {
    if !enabled() {
        return;
    }
    with_store(|store| {
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick,
            subject,
            kind,
            actor,
        });
    });
}

/// Stage 1's retention trigger surface. Stage 2/3 call this from the real
/// trigger sites (colonist named, item crafted/gifted/player-touched, ...);
/// stage 1 exposes it so those sites have something to call and so this
/// module's own tests can exercise the ring->permanent move without
/// inventing a second mechanism later.
pub fn retain(subject: Uid) {
    if !enabled() {
        return;
    }
    with_store(|store| store.retain(subject));
}

/// Cross-entity export for the pilot's two registered customers -- see
/// [`EntityStore::released_records`] for why this differs deliberately
/// from `events_for` (per-entity) and `event_count` (a bare total,
/// which "the aggregate-late law landing on the instrument built to
/// escape it" -- Opus, 2026-08-10 -- correctly named as unable to
/// answer either customer). `cap` bounds the returned list; the `bool`
/// is true when the true count exceeded it (truncated, oldest dropped
/// first, matching every other truncation flag in this module).
pub fn released_events(cap: usize) -> (Vec<ReleasedRecord>, bool) {
    if !enabled() {
        return (Vec::new(), false);
    }
    slot()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|store| store.released_records(cap)))
        .unwrap_or((Vec::new(), false))
}

/// The Voonoo query (measure 1): every recorded event for `subject`, in
/// tick order. `event.actor` names who touched it at each step -- this is
/// deliberately the WHOLE answer to "given a drop's uid, return every actor
/// that touched it, in order," not a stepping stone to one; filtering to
/// `actor.is_some()` events is the caller's one-line follow-up, not a
/// second query.
pub fn events_for(subject: Uid) -> Vec<EntityEvent> {
    if !enabled() {
        return Vec::new();
    }
    let mut result = Vec::new();
    if let Ok(guard) = slot().lock()
        && let Some(store) = guard.as_ref()
    {
        result = store.events_for(subject);
    }
    result
}

/// Whether `subject`'s ring has ever dropped an event to make room for a
/// newer one -- the per-entity self-accounting flag (design #99, "the
/// truncation flag... carried forward per-entity").
pub fn truncated(subject: Uid) -> bool {
    if !enabled() {
        return false;
    }
    slot()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|store| store.truncated(subject)))
        .unwrap_or(false)
}

pub fn is_retained(subject: Uid) -> bool {
    if !enabled() {
        return false;
    }
    slot()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|store| store.is_retained(subject)))
        .unwrap_or(false)
}

/// Test-tooling introspection for the disabled-by-default lifecycle proof,
/// same shape as `bastion_flight_recorder::global_slot_initialized`.
#[doc(hidden)]
pub fn global_slot_initialized() -> bool { LOG.get().is_some() }

/// The floor-gate self-attestation field (Opus's catch, 2026-08-10): a
/// paired determinism-floor run with the chassis enabled but zero producers
/// wired changes no gameplay-visible field by construction, so a bare `u64`
/// count would read `0` in BOTH the env-unset and env-set arms and prove
/// nothing about whether `BASTION_ENTITY_EVENT_LOG` actually reached the
/// harness process. `Option` makes presence itself the witness: `None`
/// (disabled -- the field renders absent/null in the harness's JSON) vs.
/// `Some(0)` (enabled, zero producers -- present with a real, if currently
/// zero, value) are distinguishable even though the *number* doesn't yet
/// differ. Becomes genuinely informative, not just self-attesting, the
/// moment stage 2 wires a real producer and the count leaves zero.
pub fn event_count() -> Option<u64> {
    if !enabled() {
        return None;
    }
    Some(
        slot()
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(EntityStore::event_count))
            .unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;

    fn uid(n: u64) -> Uid { Uid(NonZeroU64::new(n).unwrap()) }

    /// The single most likely thing to be got wrong, per the packet: a
    /// flooding entity must NOT starve any other entity's history. Tests
    /// the per-entity `EntityStore` directly (bypassing the env-gated
    /// process-global slot) so this is deterministic and doesn't depend on
    /// process env state.
    #[test]
    fn flooding_entity_does_not_starve_others_history() {
        let mut store = EntityStore::new(4);
        let flooder = uid(1);
        let quiet = uid(2);
        for i in 0..100u64 {
            store.record(EntityEvent {
                schema: SCHEMA.to_owned(),
                tick: i,
                subject: flooder,
                kind: EventKind::Item(ItemEventKind::Created),
                actor: None,
            });
        }
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 5,
            subject: quiet,
            kind: EventKind::Item(ItemEventKind::Created),
            actor: None,
        });
        assert_eq!(
            store.events_for(quiet).len(),
            1,
            "the flooding entity must not evict or crowd out an unrelated entity's own ring"
        );
        assert!(!store.truncated(quiet), "the quiet entity was never truncated");
        assert!(store.truncated(flooder), "the flooder's own ring should be truncated");
        assert_eq!(store.events_for(flooder).len(), 4, "flooder's ring stays at its own cap");
    }

    /// Silent truncation: a ring that dropped history must never be
    /// indistinguishable from an entity that had none.
    #[test]
    fn overflowing_a_ring_sets_and_surfaces_the_truncation_flag_for_that_entity() {
        let mut store = EntityStore::new(2);
        let subject = uid(7);
        let untouched = uid(8);
        for i in 0..5u64 {
            store.record(EntityEvent {
                schema: SCHEMA.to_owned(),
                tick: i,
                subject,
                kind: EventKind::Item(ItemEventKind::Created),
                actor: None,
            });
        }
        assert!(store.truncated(subject));
        assert!(
            !store.truncated(untouched),
            "an entity with no ring at all is NOT truncated -- absence and truncation must not \
             render identically"
        );
    }

    /// Oldest-out, not drop-newest: this module's chassis correction
    /// against the flight recorder's own policy, verified directly.
    #[test]
    fn ring_keeps_the_newest_events_oldest_out() {
        let mut store = EntityStore::new(3);
        let subject = uid(1);
        for i in 0..5u64 {
            store.record(EntityEvent {
                schema: SCHEMA.to_owned(),
                tick: i,
                subject,
                kind: EventKind::Item(ItemEventKind::Created),
                actor: None,
            });
        }
        let ticks: Vec<u64> = store.events_for(subject).iter().map(|e| e.tick).collect();
        assert_eq!(
            ticks,
            vec![2, 3, 4],
            "oldest-out: the ring must keep the most RECENT events, not the first ones written"
        );
    }

    /// Retention: one-way, moves ring contents to permanent, and all
    /// FUTURE events for that uid go straight to permanent (unbounded).
    #[test]
    fn retention_moves_ring_to_permanent_and_stops_bounding_future_events() {
        let mut store = EntityStore::new(2);
        let subject = uid(3);
        for i in 0..2u64 {
            store.record(EntityEvent {
                schema: SCHEMA.to_owned(),
                tick: i,
                subject,
                kind: EventKind::Item(ItemEventKind::Created),
                actor: None,
            });
        }
        assert!(!store.is_retained(subject));
        store.retain(subject);
        assert!(store.is_retained(subject));
        // Past the old ring cap: proves permanent storage is unbounded,
        // not just a bigger ring.
        for i in 2..10u64 {
            store.record(EntityEvent {
                schema: SCHEMA.to_owned(),
                tick: i,
                subject,
                kind: EventKind::Item(ItemEventKind::Created),
                actor: None,
            });
        }
        let ticks: Vec<u64> = store.events_for(subject).iter().map(|e| e.tick).collect();
        assert_eq!(
            ticks,
            (0..10).collect::<Vec<_>>(),
            "retained history is complete, ring-era events included, unbounded past the cap"
        );
        assert!(
            !store.truncated(subject),
            "a retained entity's ring is gone; nothing left there to be truncated"
        );
    }

    /// Opus's addition to the planted set (2026-08-10): retaining an
    /// entity WHILE its ring is mid-truncation must carry that gap
    /// forward. A retained entity is exactly the one whose history someone
    /// will trust -- if retention silently cleared the flag, it would
    /// claim complete history it doesn't have.
    #[test]
    fn retaining_a_ring_that_already_truncated_carries_the_gap_forward() {
        let mut store = EntityStore::new(2);
        let subject = uid(11);
        // Overflow the ring BEFORE retaining -- ticks 0,1,2 pushed into a
        // cap-2 ring truncates tick 0.
        for i in 0..3u64 {
            store.record(EntityEvent {
                schema: SCHEMA.to_owned(),
                tick: i,
                subject,
                kind: EventKind::Item(ItemEventKind::Created),
                actor: None,
            });
        }
        assert!(store.truncated(subject), "sanity: the ring did truncate before retention");
        store.retain(subject);
        assert!(
            store.truncated(subject),
            "retention must carry the pre-existing gap forward, not silently clear it"
        );
        // And a NEW entity retained with a clean (never-truncated) ring
        // must read as NOT truncated -- the flag is per-entity fact, not a
        // side effect of retention itself.
        let clean = uid(12);
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 0,
            subject: clean,
            kind: EventKind::Item(ItemEventKind::Created),
            actor: None,
        });
        store.retain(clean);
        assert!(
            !store.truncated(clean),
            "retention itself must not manufacture a truncation that never happened"
        );
    }

    /// Retaining an already-retained entity is a no-op, not a double-move
    /// or a data loss.
    #[test]
    fn retaining_twice_is_idempotent() {
        let mut store = EntityStore::new(4);
        let subject = uid(9);
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 1,
            subject,
            kind: EventKind::Item(ItemEventKind::Created),
            actor: None,
        });
        store.retain(subject);
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 2,
            subject,
            kind: EventKind::Item(ItemEventKind::Created),
            actor: None,
        });
        store.retain(subject); // second call: must not clear or duplicate
        assert_eq!(store.events_for(subject).len(), 2);
    }

    fn released_event(tick: u64, subject: Uid, job: u64, queue_position: Option<usize>) -> EntityEvent {
        EntityEvent {
            schema: SCHEMA.to_owned(),
            tick,
            subject,
            kind: EventKind::Colonist(ColonistEventKind::Released {
                job,
                reason: ReleaseReasonV1::TimedOut,
                queue_position,
            }),
            actor: None,
        }
    }

    /// `released_records` (Opus's request, 2026-08-10): crosses entities
    /// deliberately, unlike `events_for` -- a `Released` from a live ring
    /// AND one from a retained (permanent) entity both appear, merged and
    /// tick-ordered, with non-`Released` events excluded and every field
    /// (job/reason/queue_position, including the `None` case) carried
    /// through unchanged.
    #[test]
    fn released_records_crosses_entities_tick_ordered_with_fields_intact() {
        let mut store = EntityStore::new(8);
        let colonist_a = uid(1);
        let colonist_b = uid(2);
        store.retain(colonist_b);
        // Out of tick order on purpose -- the query must sort, not trust
        // insertion order.
        store.record(released_event(20, colonist_a, 501, Some(3)));
        store.record(released_event(10, colonist_b, 502, None));
        // A non-Released event must not leak into the export.
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 15,
            subject: colonist_a,
            kind: EventKind::Colonist(ColonistEventKind::Claimed { job: 503 }),
            actor: None,
        });
        let (records, truncated) = store.released_records(10);
        assert!(!truncated);
        assert_eq!(records.len(), 2, "only the two Released events, not the Claimed one");
        assert_eq!(records[0].subject, colonist_b, "tick-ordered: 10 before 20");
        assert_eq!(records[0].tick, 10);
        assert_eq!(records[0].job, 502);
        assert_eq!(records[0].queue_position, None, "not every Released event is a queue member");
        assert_eq!(records[1].subject, colonist_a);
        assert_eq!(records[1].tick, 20);
        assert_eq!(records[1].job, 501);
        assert_eq!(records[1].queue_position, Some(3));
    }

    /// The cap must behave exactly like the per-entity ring's own policy:
    /// oldest-out, with the truncation flag set -- a silently-capped list
    /// is the same censoring trap the ring already guards against, in a
    /// new place.
    #[test]
    fn released_records_caps_oldest_out_and_flags_truncation() {
        let mut store = EntityStore::new(16);
        let subject = uid(1);
        for tick in 0..5u64 {
            store.record(released_event(tick, subject, tick, None));
        }
        let (records, truncated) = store.released_records(3);
        assert!(truncated, "5 recorded, capped at 3 -- must report truncation");
        let ticks: Vec<u64> = records.iter().map(|r| r.tick).collect();
        assert_eq!(ticks, vec![2, 3, 4], "oldest-out: keeps the most RECENT, drops the earliest");

        let (records_uncapped, not_truncated) = store.released_records(5);
        assert!(!not_truncated, "cap exactly equal to the true count is not truncation");
        assert_eq!(records_uncapped.len(), 5);
    }

    /// The Voonoo query itself, end to end: given a drop's uid, every
    /// actor that touched it, in order.
    #[test]
    fn voonoo_query_returns_every_actor_in_tick_order() {
        let mut store = EntityStore::new(8);
        let item = uid(100);
        let picker_a = uid(1);
        let picker_b = uid(2);
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 10,
            subject: item,
            kind: EventKind::Item(ItemEventKind::Created),
            actor: None,
        });
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 12,
            subject: item,
            kind: EventKind::Item(ItemEventKind::PickedUp),
            actor: Some(picker_a),
        });
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 15,
            subject: item,
            kind: EventKind::Item(ItemEventKind::Dropped),
            actor: Some(picker_a),
        });
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 20,
            subject: item,
            kind: EventKind::Item(ItemEventKind::PickedUp),
            actor: Some(picker_b),
        });
        let actors: Vec<Uid> = store
            .events_for(item)
            .into_iter()
            .filter_map(|e| e.actor)
            .collect();
        assert_eq!(actors, vec![picker_a, picker_a, picker_b]);
    }

    /// The floor-gate self-attestation arithmetic, tested directly against
    /// a constructed store (bypassing the env-gated wrapper, same reason as
    /// every other test in this module): a ring's events and a retained
    /// entity's permanent events both count, and an empty store counts as
    /// zero -- the "zero producers" case the floor gate actually runs
    /// against, distinct from "never initialized."
    #[test]
    fn event_count_sums_rings_and_permanent_across_every_entity() {
        let mut store = EntityStore::new(4);
        assert_eq!(store.event_count(), 0, "an empty store counts as zero, not absent");
        let a = uid(1);
        let b = uid(2);
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 1,
            subject: a,
            kind: EventKind::Item(ItemEventKind::Created),
            actor: None,
        });
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 2,
            subject: b,
            kind: EventKind::Item(ItemEventKind::Created),
            actor: None,
        });
        assert_eq!(store.event_count(), 2, "counts across distinct entities' rings");
        store.retain(a);
        store.record(EntityEvent {
            schema: SCHEMA.to_owned(),
            tick: 3,
            subject: a,
            kind: EventKind::Item(ItemEventKind::Dropped),
            actor: None,
        });
        assert_eq!(
            store.event_count(),
            3,
            "a retained entity's permanent events count too, not just live rings"
        );
    }

    /// The self-attestation shape itself (Opus's catch, 2026-08-10): when
    /// disabled, `event_count()` must return `None` -- absent, not `Some(0)`
    /// -- so a paired floor run can tell "the env var never arrived" apart
    /// from "it arrived and zero events were produced," which a bare `u64`
    /// defaulting to zero could never distinguish.
    #[test]
    fn event_count_is_none_when_disabled_not_some_zero() {
        if std::env::var_os("BASTION_ENTITY_EVENT_LOG").is_some() {
            return; // Environment-contaminated test run; skip rather than false-fail.
        }
        assert_eq!(
            event_count(),
            None,
            "disabled must render as absent, not a zero count that looks like a real reading"
        );
    }

    /// Disabled emission must be free: no I/O, no ECS mutation (n/a at
    /// this layer), no allocation of the process-global store, no state
    /// change at all when `BASTION_ENTITY_EVENT_LOG` is unset.
    #[test]
    fn disabled_emission_does_not_initialize_or_record_anything() {
        // Uses the module-level API (not a fresh EntityStore) specifically
        // to exercise the env-gated `enabled()`/`record_event` path this
        // test is about. Relies on the env var being unset in the test
        // process (the default; no test in this crate sets it).
        if std::env::var_os("BASTION_ENTITY_EVENT_LOG").is_some() {
            return; // Environment-contaminated test run; skip rather than false-fail.
        }
        assert!(!enabled());
        record_event(1, uid(1), EventKind::Item(ItemEventKind::Created), None);
        assert!(
            !global_slot_initialized(),
            "recording while disabled must not initialize the process-global slot at all"
        );
        assert!(events_for(uid(1)).is_empty());
    }
}
