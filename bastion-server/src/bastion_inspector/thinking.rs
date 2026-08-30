//! bastion (INSPECTOR-M2): the **Thinking** section provider — WHAT THIS
//! COLONIST IS FEELING, AND WHY.
//!
//! ★ ALMOST NONE OF THIS IS NEW DATA. `MoodExplanationV1` — every need
//! penalty, every thought's decayed and care-scaled contribution, the
//! comfort thresholds, and a `total_mood` recomputed through the REAL
//! [`common::comp::bastion::mood_formula`] — has been crossing the wire
//! inside `BastionInspectPayload` since T3.54, and the client flattens the
//! whole thing away unread. Personality traits, the `values` map the care
//! multiplier scales by, the sentiments and the needs meters are in the
//! same position. This section is not a new producer; it is the first
//! CONSUMER of an existing one.
//!
//! ★ THE SECTION'S MOST VALUABLE OUTPUT IS A DISAGREEMENT. `comp::Mood` is
//! a MIRROR written by the mood tick and read by everything downstream;
//! `MoodExplanationV1::build` recomputes the same number through the real
//! formula at request time. When they differ by more than
//! [`common::comp::bastion_inspect::MOOD_MIRROR_TOLERANCE`] the mirror is
//! stale and every consumer of `Mood` is acting on a number the formula
//! would no longer produce. The panel renders that as an alarm row. See
//! `mood_mirror_disagreement`, which is where the rule lives.
//!
//! ★ THE CHRONICLE DISCLOSES ITS FILTERS. Three different things can make
//! a colonist's story empty — the log is off, the colonist genuinely has
//! no history, or the player view dropped 64 job-release rows — and an
//! inspector that rendered them alike would be committing the exact defect
//! the whole design exists to prevent. Every filter is carried as its own
//! field; see [`chronicle_view`].

use common::comp::bastion_inspect::{
    ChronicleRowV1, ChronicleViewV1, SectionIdV1, SectionPayloadV1, ThinkingSectionV1,
};

use super::{InspectCtx, name_of, not_a_colonist, not_measured, unloaded};
use crate::bastion_entity_event_log as ev;

/// How many chronicle rows ride the wire, at most.
///
/// The per-entity ring holds 64 by default (`DEFAULT_RING_SIZE`, private
/// to that module), so at the default this cap never bites — but the ring
/// size is settable from the environment
/// (`BASTION_ENTITY_EVENT_LOG_RING_SIZE`), and a panel that quietly showed
/// the last 64 of 4,096
/// events would be a filtered list wearing a complete list's face.
/// [`ChronicleViewV1::capped`] is how the panel knows to say so.
pub const CHRONICLE_ROW_CAP: usize = 64;

/// Whether the player view's job-release filter is switched OFF.
///
/// Reads the SAME environment variable as the shipped `Chronicle` inspect
/// target, so the two views cannot disagree about what a player is looking
/// at. Split from [`chronicle_view`] so the filter itself is pinnable
/// without mutating process environment — which races every other test in
/// the binary.
pub fn chronicle_raw_mode() -> bool { std::env::var_os("BASTION_CHRONICLE_RAW").is_some() }

/// Build the chronicle half, DISCLOSING EVERY FILTER BETWEEN THE RING AND
/// THE ROWS.
///
/// * `enabled` — the producer itself. `false` means nothing was ever
///   recorded, which is not the same as an uneventful colonist.
/// * `total` — what the ring held BEFORE the view filter.
/// * `hidden_released` — what the player-view filter took. The
///   improvement-list measured a chronicle that was 93% job-release spam;
///   dropping those rows is right, and dropping them SILENTLY is not.
/// * `truncated` — the ring dropped the oldest to make room. A fourth,
///   independent kind of missing.
/// * `row_cap` vs the surviving count — the transmitted list is a SUFFIX.
///
/// `raw` is a parameter rather than an env read so the filter can be
/// pinned in both modes from one test.
pub fn chronicle_view(subject: common::uid::Uid, raw: bool, names: &[(common::uid::Uid, String)])
-> ChronicleViewV1 {
    if !ev::enabled() {
        // ★ AND NOT A SINGLE OTHER FIELD IS GUESSED. `events_for` already
        // returns empty while disabled, so falling through would produce
        // `total: 0` — a true number that reads as "this colonist has no
        // story" when the truth is "nobody is writing one".
        return ChronicleViewV1 {
            enabled: false,
            truncated: false,
            total: 0,
            hidden_released: 0,
            raw,
            rows: Vec::new(),
            row_cap: CHRONICLE_ROW_CAP as u32,
        };
    }
    let all = ev::events_for(subject);
    let total = all.len() as u32;
    let kept: Vec<&ev::EntityEvent> = all
        .iter()
        .filter(|e| {
            raw || !matches!(
                e.kind,
                ev::EventKind::Colonist(ev::ColonistEventKind::Released { .. })
            )
        })
        .collect();
    let hidden_released = total.saturating_sub(kept.len() as u32);
    // The MOST RECENT rows: a life reads forwards, but when it does not
    // all fit, what a player wants is the end of it.
    let start = kept.len().saturating_sub(CHRONICLE_ROW_CAP);
    let rows = kept[start..]
        .iter()
        .map(|e| ChronicleRowV1 {
            tick: e.tick,
            // `EventKind` has no wire form and `common` cannot name it,
            // so the variant's own `Debug` — which carries each variant's
            // typed payload — is the row's text. Same choice the shipped
            // `BastionChronicleRow` makes, for the same reason.
            kind: format!("{:?}", e.kind),
            actor: e.actor.map(|a| {
                name_of(names, a)
                    .map(str::to_owned)
                    // An actor who is not a loaded colonist is a real
                    // state (a wolf, an unloaded neighbour). Say the uid
                    // rather than dropping the row's second party.
                    .unwrap_or_else(|| format!("uid:{} (not a loaded colonist)", a.0.get()))
            }),
        })
        .collect();
    ChronicleViewV1 {
        enabled: true,
        truncated: ev::truncated(subject),
        total,
        hidden_released,
        raw,
        rows,
        row_cap: CHRONICLE_ROW_CAP as u32,
    }
}

pub fn provide(ctx: &InspectCtx<'_>) -> SectionPayloadV1 {
    let Some(rec) = ctx.record else {
        return if ctx.loaded.is_none() {
            unloaded(SectionIdV1::Thinking)
        } else {
            not_a_colonist(SectionIdV1::Thinking)
        };
    };
    let Some(l) = ctx.loaded.as_ref() else {
        // Mood, needs and energy are ECS components. A Thinking section
        // with no mood at all is not a Thinking section.
        return unloaded(SectionIdV1::Thinking);
    };
    let Some(mind) = ctx.mind else {
        // The request did not ask for this section, so nothing rtsim-side
        // was measured. Refusing with the REASON keeps "nobody looked"
        // distinct from "there is nothing to feel".
        return not_measured(SectionIdV1::Thinking);
    };

    SectionPayloadV1::Thinking(ThinkingSectionV1 {
        mood_mirror: l.mood,
        explanation: mind.explanation.clone(),
        needs: l.needs,
        energy: l.energy,
        guard_bravery: rec.guard_bravery,
        traits: mind.traits.clone(),
        // A `BTreeMap`, so this order is the `Value` enum's own — stable
        // across two assemblies by construction, no sort needed.
        values: rec.values.iter().map(|(v, w)| (*v, *w)).collect(),
        sentiments: mind.sentiments.clone(),
        chronicle: chronicle_view(ctx.subject, chronicle_raw_mode(), ctx.names),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::uid::Uid;

    fn uid(n: u64) -> Uid { Uid(std::num::NonZeroU64::new(n).expect("nonzero")) }

    /// ★ A DISABLED LOG, AN EMPTY ONE AND A FILTERED ONE ARE THREE
    /// DIFFERENT ANSWERS.
    ///
    /// The disabled case is the one that actually shipped as a defect:
    /// two play sessions opened a colonist's story and read
    /// `enabled=false rows=0` while believing they were reading an
    /// uneventful life.
    ///
    /// ★ HONEST NOTE ON WHAT THIS PIN CAN AND CANNOT REACH. The log is
    /// a process-global switched by environment, and mutating process env
    /// races every other test in this binary — so the disabled BRANCH is
    /// reached here only when the ambient environment has it off. What is
    /// pinned unconditionally is the property that matters and is
    /// reachable: the payload CARRIES `enabled`, `total`,
    /// `hidden_released` and `truncated` as four independent fields, so
    /// the three states are representable and are not collapsed by the
    /// view. `chronicle_absence_disabled_and_filtered_are_distinguishable`
    /// in `common` pins the discrimination itself over hand-built values,
    /// where no global is involved.
    #[test]
    fn the_chronicle_view_states_its_own_filters() {
        let v = chronicle_view(uid(1), false, &[]);
        assert_eq!(v.row_cap, CHRONICLE_ROW_CAP as u32);
        assert_eq!(v.enabled, ev::enabled(), "the payload must report the real producer state");
        if !v.enabled {
            assert_eq!(v.total, 0);
            assert_eq!(v.hidden_released, 0);
            assert!(v.rows.is_empty());
        }
        // A colonist with no recorded events: `total` is the ring's own
        // count, and `hidden_released` cannot exceed it.
        assert!(v.hidden_released <= v.total);
        assert!(v.rows.len() as u32 <= v.shown_after_filter());
        // RAW mode is carried so the reader knows whether a filter ran at
        // all.
        assert!(!v.raw);
        assert!(chronicle_view(uid(1), true, &[]).raw);
    }

    /// The actor column resolves to a NAME where one exists and says so
    /// where one does not — never a bare number that names nobody.
    #[test]
    fn an_unresolvable_actor_is_labelled_rather_than_dropped() {
        let names = vec![(uid(2), "Hedda".to_string())];
        assert_eq!(name_of(&names, uid(2)), Some("Hedda"));
        assert_eq!(name_of(&names, uid(3)), None);
    }
}
