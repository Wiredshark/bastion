//! bastion (INSPECTOR-M1): the **Identity** section view — WHO THIS IS.

use common::{
    bastion::{WorkType, game_time},
    comp::bastion_inspect::{FrameV1, InspectFramesV1, InspectRow, SectionPayloadV1},
};

pub fn rows(payload: &SectionPayloadV1, frames: &InspectFramesV1) -> Vec<InspectRow> {
    let SectionPayloadV1::Identity(d) = payload else {
        return Vec::new();
    };
    let mut rows = Vec::with_capacity(10 + 2 * WorkType::COUNT);

    rows.push(InspectRow::new(
        "Name",
        d.name.clone(),
        "BastionColonist::name",
        "",
        FrameV1::RtsimRoster,
    ));

    rows.push(
        InspectRow::new(
            "Trade",
            match d.profession {
                Some(w) => w.label().to_string(),
                // NOT "no trade": the board derives professions once a
                // game-day from the rolling dominant lane, and it is
                // rebuilt from empty at every server start. "None" here
                // means NOT YET DERIVED, and saying "no trade" would
                // report a fresh restart as a town of layabouts.
                None => "not yet derived (day 0, or since restart)".to_string(),
            },
            "JobBoard::professions",
            "",
            FrameV1::JobBoard,
        )
        .scoped("runtime-only; does not survive a restart"),
    );
    // ZONE ASSIGNMENT (Ben: "we should be able to see that").
    rows.push(InspectRow::new(
        "Assigned to",
        match d.assigned_zone {
            Some((z, true)) => format!("zone {z} (set by hand)"),
            Some((z, false)) => format!("zone {z} (auto)"),
            None => "no zone (lane has none, or not yet assigned today)".to_string(),
        },
        "JobBoard::assignments",
        "",
        FrameV1::JobBoard,
    ));


    // ★ AGE COMES FROM `born_tick` AGAINST `rtsim_tick`, NEVER FROM
    // `born_day`. `born_day` is stamped from `TimeOfDay`, which the server
    // resets at every boot; measured on world 109, `today - born_day` went
    // NEGATIVE across a restart and no child would ever have come of age.
    // `game_time::age_days` takes only the persistent clock and refuses a
    // backwards one, so this row cannot be built from the wrong field.
    rows.push(
        InspectRow::new(
            "Age",
            match game_time::age_days(frames.rtsim_tick, d.born_tick, frames.ticks_per_game_day) {
                Some(days) => format!("{days:.1}"),
                None if d.born_tick.is_none() => {
                    "unknown (arrived grown — founder or settler)".to_string()
                },
                // The clock ran backwards: a rolled-back save. Refuse
                // rather than print a plausible wrong number.
                None => "unknown (record predates the current rtsim tick)".to_string(),
            },
            "game_time::age_days(born_tick, rtsim_tick)",
            "game days",
            FrameV1::Derived,
        )
        .scoped("persistent rtsim clock only"),
    );

    // Shown ONLY under a label that says what it is. It is what the
    // witness logs print, so a reader wants to see it — but it is not
    // what anything computes with.
    if let Some(day) = d.born_day_boot_relative {
        rows.push(
            InspectRow::new(
                "Born on day",
                format!("{day} (BOOT-RELATIVE — not comparable across restarts)"),
                "BastionColonist::born_day",
                "boot-relative game days",
                FrameV1::RtsimRoster,
            )
            .scoped("display only; never used to compute an age"),
        );
    }

    rows.push(InspectRow::new(
        "Parent",
        d.parent_name.clone().unwrap_or_else(|| "none recorded".to_string()),
        "BastionColonist::parent (NpcId, resolved server-side)",
        "",
        FrameV1::RtsimRoster,
    ));

    rows.push(InspectRow::new(
        "Backstory",
        d.backstory.clone(),
        "BastionColonist::backstory",
        "",
        FrameV1::RtsimRoster,
    ));

    rows.push(
        InspectRow::new(
            "Bed",
            match (d.owned_bed, d.bed_slot_agrees) {
                (None, _) => "none owned".to_string(),
                (Some(p), Some(true)) => format!("({}, {}, {})", p.x, p.y, p.z),
                // A real, reportable disagreement: the DURABLE record
                // claims a bed the RUNTIME slot table does not agree is
                // theirs. Surfaced rather than reconciled -- the panel
                // showing the mismatch is the only place it is visible.
                (Some(p), _) => format!(
                    "({}, {}, {}) — but the board's slot does not name them as owner",
                    p.x, p.y, p.z
                ),
            },
            "BastionColonist::owned_bed vs JobBoard::beds[pos].owner",
            "",
            FrameV1::RtsimRoster,
        )
        .scoped("record compared against the runtime slot table"),
    );

    rows.push(InspectRow::new(
        "Health",
        match d.health {
            Some(h) => format!("{:.0}%", h * 100.0),
            // The Option is load-bearing: absence is not zero.
            None => "unknown (not loaded)".to_string(),
        },
        "Health::fraction",
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

    // ★ THE FULL LANE TABLE, over `WorkType::ALL`.
    //
    // The rows are generated by iterating `ALL`, so a new work type shows
    // up here with no edit. The bug this replaces was two hand-written
    // seven-element arrays against a `COUNT` of 8: every blacksmith
    // inspected showed no craft skill and no craft desire.
    for w in WorkType::ALL {
        let i = w.lane_index();
        rows.push(
            InspectRow::new(
                format!("Skill: {}", w.label()),
                d.skills[i].to_string(),
                "ColonistSkills::level_for",
                "level",
                FrameV1::RtsimRoster,
            )
            .scoped("the same source the claim gate and work rate read"),
        );
    }
    for w in WorkType::ALL {
        let i = w.lane_index();
        rows.push(
            InspectRow::new(
                format!("Desire: {}", w.label()),
                format!("{:.2}", d.desires[i]),
                "WorkDesires::get",
                "claim-score multiplier",
                FrameV1::RtsimRoster,
            )
            .scoped("1.00 is neutral"),
        );
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::comp::bastion_inspect::IdentitySectionV1;

    fn frames(rtsim_tick: u64) -> InspectFramesV1 {
        InspectFramesV1 {
            server_tick: 10,
            rtsim_tick,
            time_of_day: 0.0,
            ticks_per_game_day: 54_000.0,
            schedule_offset_hours: 0,
        }
    }

    fn payload(born_tick: Option<u64>) -> SectionPayloadV1 {
        SectionPayloadV1::Identity(IdentitySectionV1 {
            name: "Hedda".into(),
            profession: None,
            assigned_zone: None,
            born_tick,
            born_day_boot_relative: Some(2),
            parent_name: None,
            backstory: "charcoal burner".into(),
            owned_bed: None,
            bed_slot_agrees: None,
            health: None,
            guard_bravery: 0.4,
            skills: [1, 2, 3, 4, 5, 6, 7, 8],
            desires: [1.0; WorkType::COUNT],
        })
    }

    /// ★ EVERY LANE GETS A ROW, `Craft` included.
    ///
    /// FALSIFIER: replace `WorkType::ALL` in either loop with the old
    /// seven-element literal and this goes RED on "craft".
    #[test]
    fn every_work_type_gets_a_skill_and_a_desire_row() {
        let r = rows(&payload(Some(0)), &frames(0));
        for w in WorkType::ALL {
            assert!(
                r.iter().any(|row| row.label() == format!("Skill: {}", w.label())),
                "{w:?} has no skill row"
            );
            assert!(
                r.iter().any(|row| row.label() == format!("Desire: {}", w.label())),
                "{w:?} has no desire row"
            );
        }
        // The lane the shipped bug dropped, with its real value.
        let craft = r
            .iter()
            .find(|row| row.label() == "Skill: craft")
            .expect("craft must have a row");
        assert_eq!(craft.value(), "8");
    }

    /// ★ THE AGE ROW NEVER USES `born_day`, AND SAYS SO WHEN IT CANNOT
    /// ANSWER.
    ///
    /// FALSIFIER: compute the age from `born_day_boot_relative` and the
    /// "arrived grown" case starts printing a number — RED.
    #[test]
    fn age_refuses_boot_relative_day() {
        // No born_tick: unknown, even though born_day is Some(2).
        let r = rows(&payload(None), &frames(54_000 * 9));
        let age = r.iter().find(|x| x.label() == "Age").expect("age row");
        assert!(age.value().contains("unknown"), "age was {}", age.value());
        assert!(age.value().contains("arrived grown"));
        // born_day is still SHOWN, but labelled.
        let born = r.iter().find(|x| x.label() == "Born on day").expect("born row");
        assert!(born.value().contains("BOOT-RELATIVE"));
        assert!(born.scope().is_some_and(|s| s.contains("never used to compute an age")));

        // A backwards clock refuses rather than printing a huge number.
        let rolled = rows(&payload(Some(54_000 * 9 + 1)), &frames(54_000 * 9));
        let age = rolled.iter().find(|x| x.label() == "Age").expect("age row");
        assert!(age.value().contains("unknown"), "age was {}", age.value());

        // And the ordinary case answers in game days.
        let ok = rows(&payload(Some(0)), &frames(54_000 * 4));
        let age = ok.iter().find(|x| x.label() == "Age").expect("age row");
        assert_eq!(age.value(), "4.0");
        assert_eq!(age.unit(), "game days");
    }

    /// A profession the board has not derived yet must not read as "no
    /// trade" — a fresh restart would otherwise report the whole town as
    /// unemployed.
    #[test]
    fn undrived_profession_says_so() {
        let r = rows(&payload(Some(0)), &frames(0));
        let trade = r.iter().find(|x| x.label() == "Trade").expect("trade row");
        assert!(trade.value().contains("not yet derived"), "trade was {}", trade.value());
        assert!(trade.scope().is_some_and(|s| s.contains("restart")));
    }
}
