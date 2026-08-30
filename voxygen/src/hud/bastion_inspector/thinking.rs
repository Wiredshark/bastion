//! bastion (INSPECTOR-M2): the **Thinking** section view — the MOOD
//! WATERFALL and everything that weights it.
//!
//! ★ THE SHAPE IS A WATERFALL because that is what `mood_formula` is:
//! `clamp01(base + Σ w·shortfall(need) + Σ thought)`. Rendering the total
//! alone would be the paraphrase this whole panel exists to replace — the
//! player asks "why is he miserable", and the only answer worth giving is
//! the formula's own terms, in the formula's own order, with the same
//! numbers.
//!
//! Every term carries its own producer, so a reader can grep from a row on
//! screen to the line that computed it.

use common::comp::bastion_inspect::{
    FrameV1, InspectFramesV1, InspectRow, SectionPayloadV1, ThinkingSectionV1,
    mood_mirror_disagreement,
};

/// The label for a need in the waterfall. A wildcard-free match, kept
/// HERE rather than on `MoodNeedId` because `common::comp::bastion` is not
/// this section's file to edit — but it is still exhaustive, so a fourth
/// need cannot be added without visiting this view.
fn need_label(n: common::comp::bastion::MoodNeedId) -> &'static str {
    use common::comp::bastion::MoodNeedId as N;
    match n {
        N::Hunger => "hunger",
        N::Rest => "rest",
        N::Recreation => "recreation",
    }
}

/// Same discipline for `Value`: an exhaustive match, so appending a value
/// to the vocabulary fails to compile here first.
fn value_label(v: common::bastion::Value) -> &'static str {
    use common::bastion::Value as V;
    match v {
        V::Glory => "glory",
        V::Tradition => "tradition",
        V::Kin => "kin",
        V::Wealth => "wealth",
        V::Piety => "piety",
        V::Nature => "nature",
        V::Craft => "craft",
        V::Freedom => "freedom",
    }
}

pub fn rows(payload: &SectionPayloadV1, _frames: &InspectFramesV1) -> Vec<InspectRow> {
    let SectionPayloadV1::Thinking(d) = payload else {
        return Vec::new();
    };
    let mut rows = Vec::with_capacity(32);

    // ── THE MIRROR CHECK, FIRST ────────────────────────────────────────
    //
    // ★ THIS ROW IS THE SECTION'S MOST VALUABLE OUTPUT. `Mood(f32)` is
    // what every downstream consumer reads; `total_mood` is what the real
    // `mood_formula` produces from the same tables right now. A
    // disagreement means the mirror is stale and the sim is acting on a
    // number the formula would no longer give.
    let explained = d.explanation.as_ref().map(|e| e.total_mood);
    let drift = mood_mirror_disagreement(d.mood_mirror, explained);
    rows.push(
        InspectRow::new(
            "Mood",
            match (d.mood_mirror, explained) {
                (Some(m), Some(e)) => match drift {
                    Some(delta) => format!(
                        "{m:.3}  ✗ STALE MIRROR — the formula says {e:.3} (drift {delta:+.3})"
                    ),
                    None => format!("{m:.3}  (formula agrees: {e:.3})"),
                },
                // Absence is NOT agreement, and it is not zero either.
                (Some(m), None) => {
                    format!("{m:.3}  (no explanation to check it against)")
                },
                (None, Some(e)) => {
                    format!("no Mood component — the formula would say {e:.3}")
                },
                (None, None) => "unknown (no Mood component, no explanation)".to_string(),
            },
            "comp::Mood vs MoodExplanationV1::total_mood (mood_formula)",
            "0..1",
            FrameV1::Ecs,
        )
        .scoped("the ECS mirror compared against a live recomputation")
        .alarming_if(drift.is_some()),
    );

    let Some(exp) = d.explanation.as_ref() else {
        rows.push(
            InspectRow::new(
                "Why",
                "no explanation — the subject has no rtsim entity, so the \
                 chronicle-backed thought half cannot be built",
                "MoodExplanationV1::build",
                "",
                FrameV1::Derived,
            )
            .scoped("absent, NOT empty"),
        );
        push_tail(&mut rows, d);
        return rows;
    };

    // ── THE WATERFALL ──────────────────────────────────────────────────
    rows.push(
        InspectRow::new(
            "  base",
            format!("{:+.3}", base_of(exp)),
            "MoodConfig::mood_base",
            "mood",
            FrameV1::Derived,
        )
        .scoped("the formula's starting point, back-derived from its own terms"),
    );
    for n in &exp.needs {
        rows.push(
            InspectRow::new(
                format!("  need: {}", need_label(n.need)),
                format!(
                    "{:+.3}   ({:.2} vs comfort {:.2}, weight {:.2})",
                    n.penalty, n.value, n.comfort, n.weight
                ),
                "NeedPenaltyV1 = weight * shortfall(value, comfort)",
                "mood",
                FrameV1::Ecs,
            )
            // A need AT or above comfort contributes nothing; only a
            // shortfall does. Saying so beside the number stops a reader
            // treating `+0.000` as a missing measurement.
            .scoped(if n.penalty == 0.0 {
                "no shortfall — this need is at or above comfort"
            } else {
                "a shortfall, weighted"
            }),
        );
    }
    // ★ ONE ROW PER THOUGHT, weighted by how much THIS colonist cares.
    // The care multiplier without its thought is meaningless, which is
    // why `ThoughtContributionV1` carries them together.
    if exp.thoughts.is_empty() {
        rows.push(
            InspectRow::new(
                "  thoughts",
                "none active",
                "bastion_mood::thought_contributions",
                "",
                FrameV1::RtsimRoster,
            )
            .scoped("no qualifying chronicle event is still within its lifetime"),
        );
    }
    for t in &exp.thoughts {
        rows.push(
            InspectRow::new(
                format!("  thought #{}", t.thought_id),
                format!(
                    "{:+.3}   (base {:+.3} × care {:.2}, from event {})",
                    t.contribution, t.base_magnitude, t.care_multiplier, t.source_event_id
                ),
                "ThoughtContributionV1 (decayed base × care_multiplier)",
                "mood",
                FrameV1::RtsimRoster,
            )
            .scoped("care is this colonist's own values weighting the event"),
        );
    }
    rows.push(
        InspectRow::new(
            "  = total",
            format!("{:.3}", exp.total_mood),
            "mood_formula(cfg, needs, thought_sum)",
            "0..1",
            FrameV1::Derived,
        )
        .scoped("clamped to 0..1 — the terms above may sum outside it"),
    );
    rows.push(
        InspectRow::new(
            "  snapshot",
            exp.snapshot_tick.to_string(),
            "MoodExplanationV1::snapshot_tick",
            "server ticks",
            FrameV1::Ecs,
        )
        .scoped("BOOT-RELATIVE — the tick this waterfall was computed at"),
    );

    push_tail(&mut rows, d);
    rows
}

/// The formula's base term, back-derived: `total` is
/// `clamp01(base + Σ penalties + thought_sum)`, and every other term is
/// carried, so `base` is what is left over.
///
/// ★ WHY DERIVE IT RATHER THAN SEND IT. `MoodConfig::mood_base` is server
/// state and the payload does not carry it; adding a field for a number the
/// payload already determines would be a second producer of one quantity.
/// The honest limit is stated in the row's own scope: when the total
/// CLAMPED, the leftover is not the base but the base plus whatever the
/// clamp removed. The clamp is visible right beside it (a total sitting
/// exactly on 0.000 or 1.000), which is the reader's cue.
fn base_of(exp: &common::comp::bastion::MoodExplanationV1) -> f32 {
    let needs: f32 = exp.needs.iter().map(|n| n.penalty).sum();
    let thoughts: f32 = exp.thoughts.iter().map(|t| t.contribution).sum();
    exp.total_mood - needs - thoughts
}

/// The rows that do not depend on the explanation being present: the raw
/// meters, the temperament, the values, the sentiments and the chronicle.
fn push_tail(rows: &mut Vec<InspectRow>, d: &ThinkingSectionV1) {
    if let Some((hunger, rest, recreation)) = d.needs {
        for (label, v) in
            [("hunger", hunger), ("rest", rest), ("recreation", recreation)]
        {
            rows.push(
                InspectRow::new(
                    format!("Meter: {label}"),
                    format!("{v:.2}"),
                    "comp::bastion::Needs",
                    "0..1",
                    FrameV1::Ecs,
                )
                .scoped("1.00 is fully satisfied"),
            );
        }
    } else {
        rows.push(InspectRow::new(
            "Meters",
            "unknown (no Needs component)",
            "comp::bastion::Needs",
            "",
            FrameV1::Ecs,
        ));
    }
    rows.push(InspectRow::new(
        "Energy",
        match d.energy {
            Some(e) => format!("{:.0}%", e * 100.0),
            None => "unknown (not loaded)".to_string(),
        },
        "Energy::fraction",
        "",
        FrameV1::Ecs,
    ));
    rows.push(
        InspectRow::new(
            "Guard bravery",
            format!("{:.2}", d.guard_bravery),
            "BastionColonist::guard_bravery",
            "health fraction",
            FrameV1::RtsimRoster,
        )
        .scoped("holds while health >= this; LOWER is braver"),
    );
    rows.push(
        InspectRow::new(
            "Temperament",
            if d.traits.is_empty() {
                "no traits satisfied".to_string()
            } else {
                d.traits.join(", ")
            },
            "rtsim::Personality::is (vanilla Big-Five)",
            "",
            FrameV1::RtsimRoster,
        )
        .scoped("HOW they react — distinct from the values below"),
    );

    // ★ THE VALUES ARE THE MAP `care_multiplier` SCALES BY. Without them
    // the per-thought care numbers above are unexplained magic.
    if d.values.is_empty() {
        rows.push(
            InspectRow::new(
                "Values",
                "none rolled — every thought lands at care 1.00",
                "BastionColonist::values",
                "",
                FrameV1::RtsimRoster,
            )
            .scoped("an empty map is the pre-B-AG3 formula, bit for bit"),
        );
    }
    for (v, w) in &d.values {
        rows.push(
            InspectRow::new(
                format!("Value: {}", value_label(*v)),
                format!("{w:+}"),
                "BastionColonist::values",
                "±50 weight",
                FrameV1::RtsimRoster,
            )
            .scoped("what care_multiplier scales each thought by"),
        );
    }

    if d.sentiments.is_empty() {
        rows.push(
            InspectRow::new(
                "Sentiments",
                "none held",
                "Sentiments::iter_held",
                "",
                FrameV1::RtsimRoster,
            )
            .scoped("nobody has made an impression yet"),
        );
    }
    for s in &d.sentiments {
        rows.push(
            InspectRow::new(
                // Resolved server-side. The shipped payload printed
                // "uid:N" here, which names nobody.
                format!("Feels about {}", s.who),
                format!("{:+.2}   ({})", s.value, s.kind.label()),
                "Sentiments::iter_held, target resolved via the rtsim roster",
                "sentiment",
                FrameV1::RtsimRoster,
            )
            .scoped("the same scale gameplay consumes"),
        );
    }

    push_chronicle(rows, d);
}

/// ★ THE CHRONICLE, WITH ITS FILTERS ON THE SCREEN.
///
/// An empty list, a switched-off producer and a filtered feed are three
/// different states and must not render alike. The producer state comes
/// first, because if the log is off nothing below it means anything.
fn push_chronicle(rows: &mut Vec<InspectRow>, d: &ThinkingSectionV1) {
    let c = &d.chronicle;
    if !c.enabled {
        rows.push(
            InspectRow::new(
                "Chronicle",
                "LOG DISABLED (BASTION_ENTITY_EVENT_LOG=0) — no history is \
                 being recorded, which is NOT the same as an uneventful life",
                "bastion_entity_event_log::enabled",
                "",
                FrameV1::Derived,
            )
            .scoped("the producer itself is off")
            .alarming(),
        );
        return;
    }
    let shown = c.shown_after_filter();
    rows.push(
        InspectRow::new(
            "Chronicle",
            if c.total == 0 {
                "no events recorded for this colonist (the log IS running)".to_string()
            } else {
                let mut s = format!("{} of {} shown", c.rows.len(), c.total);
                if c.hidden_released > 0 {
                    s.push_str(&format!(
                        " — {} job-release rows hidden by the player view",
                        c.hidden_released
                    ));
                }
                if c.raw {
                    s.push_str(" — BASTION_CHRONICLE_RAW is set, nothing filtered");
                }
                if c.capped() {
                    s.push_str(&format!(
                        " — showing the last {} of {shown} (cap {})",
                        c.rows.len(),
                        c.row_cap
                    ));
                }
                if c.truncated {
                    s.push_str(" — the ring DROPPED older events to make room");
                }
                s
            },
            "bastion_entity_event_log::events_for + the player-view filter",
            "events",
            FrameV1::Derived,
        )
        .scoped("every filter between the ring and the rows below")
        // The ring having eaten history is a real loss and worth
        // noticing; a routine filter is not.
        .alarming_if(c.truncated),
    );
    for r in &c.rows {
        rows.push(
            InspectRow::new(
                format!("  t{}", r.tick),
                match &r.actor {
                    Some(a) => format!("{}   (with {a})", r.kind),
                    None => r.kind.clone(),
                },
                "EntityEvent",
                "",
                FrameV1::Derived,
            )
            .scoped("server ticks, BOOT-RELATIVE"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::comp::bastion_inspect::{
        ChronicleRowV1, ChronicleViewV1, RowSeverityV1, SentimentRowV1, SentimentTargetKindV1,
    };

    fn frames() -> InspectFramesV1 {
        InspectFramesV1 {
            server_tick: 900,
            rtsim_tick: 1,
            time_of_day: 0.0,
            ticks_per_game_day: 54_000.0,
            schedule_offset_hours: 0,
        }
    }

    fn explanation(total: f32) -> common::comp::bastion::MoodExplanationV1 {
        use common::comp::bastion::{
            MoodNeedId, MoodThresholdV1, NeedPenaltyV1, ThoughtContributionV1,
        };
        common::comp::bastion::MoodExplanationV1 {
            snapshot_tick: 900,
            // `NpcId` is a slotmap key with no public constructor
            // reachable from voxygen; the actor is not what this view
            // renders, so the other variant stands in.
            actor: common::rtsim::Actor::Character(common::character::CharacterId(1)),
            needs: vec![
                NeedPenaltyV1 {
                    need: MoodNeedId::Hunger,
                    value: 0.2,
                    comfort: 0.5,
                    weight: -0.4,
                    penalty: -0.12,
                },
                NeedPenaltyV1 {
                    need: MoodNeedId::Rest,
                    value: 0.9,
                    comfort: 0.5,
                    weight: -0.4,
                    penalty: 0.0,
                },
                NeedPenaltyV1 {
                    need: MoodNeedId::Recreation,
                    value: 0.9,
                    comfort: 0.5,
                    weight: -0.2,
                    penalty: 0.0,
                },
            ],
            thoughts: vec![ThoughtContributionV1 {
                source_event_id: 77,
                thought_id: 3,
                base_magnitude: -0.10,
                care_multiplier: 1.5,
                contribution: -0.15,
            }],
            thresholds: vec![MoodThresholdV1 { need: MoodNeedId::Hunger, comfort: 0.5 }],
            total_mood: total,
        }
    }

    fn payload(mirror: Option<f32>, total: f32) -> SectionPayloadV1 {
        SectionPayloadV1::Thinking(ThinkingSectionV1 {
            mood_mirror: mirror,
            explanation: Some(explanation(total)),
            needs: Some((0.2, 0.9, 0.9)),
            energy: Some(0.8),
            guard_bravery: 0.4,
            traits: vec!["Neurotic".into()],
            values: vec![(common::bastion::Value::Kin, 30)],
            sentiments: vec![SentimentRowV1 {
                who: "Orm".into(),
                kind: SentimentTargetKindV1::Colonist,
                value: 0.4,
            }],
            chronicle: ChronicleViewV1 {
                enabled: true,
                truncated: false,
                total: 476,
                hidden_released: 64,
                raw: false,
                rows: vec![ChronicleRowV1 {
                    tick: 12,
                    kind: "Colonist(Slept { owned: true, healed: false })".into(),
                    actor: None,
                }],
                row_cap: 64,
            },
        })
    }

    /// ★ THE STALE-MIRROR ROW IS AN ALARM, AND ONLY WHEN IT SHOULD BE.
    ///
    /// A row that is always loud teaches the reader to skip it, so the
    /// agreeing case must be quiet — and the disagreeing case must name
    /// BOTH numbers, because "the mood is wrong" without the two values
    /// is not actionable.
    ///
    /// FALSIFIER: replace `alarming_if(drift.is_some())` with
    /// `alarming()` and the agreeing case goes RED; drop the `{e:.3}` from
    /// the disagreeing branch and the both-numbers assertion goes RED.
    #[test]
    fn a_stale_mood_mirror_renders_as_an_alarm() {
        // Agreement: quiet, and still shows the check happened.
        let ok = rows(&payload(Some(0.53), 0.53), &frames());
        let mood = ok.iter().find(|r| r.label() == "Mood").expect("a mood row");
        assert_eq!(mood.severity(), RowSeverityV1::Normal, "agreement must be quiet");
        assert!(mood.value().contains("agrees"), "got {}", mood.value());

        // Disagreement: alarming, and both numbers on screen.
        let bad = rows(&payload(Some(0.90), 0.53), &frames());
        let mood = bad.iter().find(|r| r.label() == "Mood").expect("a mood row");
        assert_eq!(mood.severity(), RowSeverityV1::Alarm);
        assert!(mood.value().contains("0.900"), "the mirror: {}", mood.value());
        assert!(mood.value().contains("0.530"), "the formula: {}", mood.value());
        assert!(mood.value().contains("STALE"), "got {}", mood.value());

        // ★ A MISSING MIRROR IS NOT A DISAGREEMENT AND NOT A ZERO.
        let none = rows(&payload(None, 0.53), &frames());
        let mood = none.iter().find(|r| r.label() == "Mood").expect("a mood row");
        assert_eq!(mood.severity(), RowSeverityV1::Normal);
        assert!(mood.value().contains("no Mood component"), "got {}", mood.value());
        assert!(!mood.value().contains("0.000"), "absence must not read as zero");
    }

    /// The waterfall shows every term the formula sums, with each term's
    /// own inputs beside it.
    ///
    /// FALSIFIER: drop either loop (needs or thoughts) and the row-presence
    /// assertions go RED; point `need_label` at the wrong variant and the
    /// hunger check goes RED.
    ///
    /// ★ HONEST NOTE ON THE LAST THIRD OF THIS TEST. The
    /// `base + Σneeds + Σthoughts == total` assertion CANNOT FAIL:
    /// `base_of` is *defined* as that residual, so the identity holds
    /// arithmetically whatever the payload contains. It is kept because it
    /// documents what the displayed `base` row means — and it is named
    /// here as tautological rather than left to look like a check. The
    /// assertions that can actually fail are the row-presence and
    /// row-content ones above it.
    #[test]
    fn the_waterfall_shows_every_term_and_reconciles() {
        let r = rows(&payload(Some(0.53), 0.53), &frames());
        for need in ["hunger", "rest", "recreation"] {
            assert!(
                r.iter().any(|x| x.label() == format!("  need: {need}")),
                "{need} has no row"
            );
        }
        assert!(r.iter().any(|x| x.label() == "  thought #3"), "the thought has no row");
        let hunger = r
            .iter()
            .find(|x| x.label() == "  need: hunger")
            .expect("hunger row");
        // The shortfall's own numbers, not a paraphrase.
        assert!(hunger.value().contains("0.20"), "the meter: {}", hunger.value());
        assert!(hunger.value().contains("0.50"), "the comfort: {}", hunger.value());

        // base + Σ needs + Σ thoughts == total, which is what makes it a
        // waterfall rather than a list.
        let exp = explanation(0.53);
        let sum: f32 = base_of(&exp)
            + exp.needs.iter().map(|n| n.penalty).sum::<f32>()
            + exp.thoughts.iter().map(|t| t.contribution).sum::<f32>();
        assert!((sum - exp.total_mood).abs() < 1e-6, "the waterfall must add up: {sum}");

        // A need at comfort contributes nothing and SAYS so, rather than
        // leaving the reader to wonder whether it was measured.
        let rest = r.iter().find(|x| x.label() == "  need: rest").expect("rest row");
        assert!(rest.scope().is_some_and(|s| s.contains("at or above comfort")));
    }

    /// ★ THE CHRONICLE DISCLOSES ITS FILTERS — and a disabled log, an
    /// empty one and a filtered one read differently.
    ///
    /// FALSIFIER: render the disabled case through the same branch as the
    /// empty one and the first two assertions collapse.
    #[test]
    fn the_chronicle_row_states_what_was_hidden() {
        let filtered = rows(&payload(Some(0.53), 0.53), &frames());
        let row = filtered.iter().find(|r| r.label() == "Chronicle").expect("chronicle row");
        assert!(row.value().contains("of 476"), "got {}", row.value());
        assert!(row.value().contains("64 job-release rows hidden"), "got {}", row.value());
        assert_eq!(row.severity(), RowSeverityV1::Normal, "a routine filter is not an alarm");

        let mut off = payload(Some(0.53), 0.53);
        if let SectionPayloadV1::Thinking(t) = &mut off {
            t.chronicle.enabled = false;
            t.chronicle.total = 0;
            t.chronicle.hidden_released = 0;
            t.chronicle.rows.clear();
        }
        let r = rows(&off, &frames());
        let row = r.iter().find(|r| r.label() == "Chronicle").expect("chronicle row");
        assert!(row.value().contains("LOG DISABLED"), "got {}", row.value());
        assert_eq!(row.severity(), RowSeverityV1::Alarm, "a dead producer is worth noticing");

        let mut empty = payload(Some(0.53), 0.53);
        if let SectionPayloadV1::Thinking(t) = &mut empty {
            t.chronicle.total = 0;
            t.chronicle.hidden_released = 0;
            t.chronicle.rows.clear();
        }
        let r = rows(&empty, &frames());
        let row = r.iter().find(|r| r.label() == "Chronicle").expect("chronicle row");
        assert!(row.value().contains("the log IS running"), "got {}", row.value());
        assert_eq!(row.severity(), RowSeverityV1::Normal);
        // All three are genuinely different strings -- the property the
        // whole disclosure exists for.
        assert_ne!(
            rows(&off, &frames())
                .iter()
                .find(|r| r.label() == "Chronicle")
                .unwrap()
                .value(),
            row.value()
        );

        // A truncated ring is a fourth state, and IS an alarm: history
        // was destroyed, not merely filtered.
        let mut lost = payload(Some(0.53), 0.53);
        if let SectionPayloadV1::Thinking(t) = &mut lost {
            t.chronicle.truncated = true;
        }
        let r = rows(&lost, &frames());
        let row = r.iter().find(|r| r.label() == "Chronicle").expect("chronicle row");
        assert!(row.value().contains("DROPPED older events"), "got {}", row.value());
        assert_eq!(row.severity(), RowSeverityV1::Alarm);
    }

    /// A sentiment names a PERSON, and the resolution kind rides with it
    /// so an unresolved target cannot pass for a name.
    #[test]
    fn sentiments_name_people_not_uids() {
        let r = rows(&payload(Some(0.53), 0.53), &frames());
        let s = r
            .iter()
            .find(|x| x.label() == "Feels about Orm")
            .expect("the sentiment must be labelled with a NAME");
        assert!(s.value().contains("colonist"), "the resolution kind must ride along");
        assert!(
            !r.iter().any(|x| x.label().contains("uid:")),
            "no row may label a person by uid alone"
        );
    }

    /// Every row names a producer, over every branch of this view.
    #[test]
    fn every_row_names_a_producer() {
        let mut all = rows(&payload(Some(0.9), 0.53), &frames());
        all.extend(rows(&payload(None, 0.53), &frames()));
        let mut bare = payload(Some(0.5), 0.5);
        if let SectionPayloadV1::Thinking(t) = &mut bare {
            t.explanation = None;
            t.needs = None;
            t.energy = None;
            t.traits.clear();
            t.values.clear();
            t.sentiments.clear();
        }
        all.extend(rows(&bare, &frames()));
        assert!(all.len() > 25, "the fixture must exercise the view");
        for r in &all {
            assert!(!r.producer().is_empty(), "row '{}' names no producer", r.label());
            assert!(!r.label().is_empty());
        }
    }
}
