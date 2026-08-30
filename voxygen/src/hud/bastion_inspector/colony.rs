//! bastion (INSPECTOR-M2): the **Colony** section view — THE TOWN AROUND
//! THIS COLONIST.
//!
//! ★ EVERY STOCK ROW IS A BREAKDOWN. The line this view exists to draw is
//!
//! ```text
//! common.items.tool.craftsman_hammer: 0 in stockpiles · 64 carried · 3 on ground (67 total)
//! ```
//!
//! because the number that shipped was the `0`, alone, under a label that
//! did not say it was stockpile-scoped — and a player reading it could not
//! tell a broken forge from broken hauling. Those are two entirely
//! different repairs, and the row above distinguishes them at a glance.
//!
//! ★ AND EVERY ROW SAYS WHICH FRAME IT IS IN. The board is RUNTIME-ONLY:
//! professions, the colony drive and the household derivation all read
//! empty after a restart until their own cadences run again. The roster and
//! the stock census are ECS. Rendering the two side by side without saying
//! which is which is the commonest defect in this subsystem.

use common::{
    bastion::WorkType,
    comp::bastion_inspect::{
        ColonySectionV1, FrameV1, InspectFramesV1, InspectRow, SectionPayloadV1, StockScopeV1,
    },
};

/// The drive's label. Exhaustive — a fifth drive fails to compile here.
fn drive_label(d: common::bastion::ColonyDrive) -> &'static str {
    use common::bastion::ColonyDrive as D;
    match d {
        D::Sustain => "Sustain (feed the colony first)",
        D::Defend => "Defend (hostiles perceived)",
        D::Grow => "Grow (housing short)",
        D::Expand => "Expand (satisfied — reach outward)",
    }
}

pub fn rows(payload: &SectionPayloadV1, frames: &InspectFramesV1) -> Vec<InspectRow> {
    let SectionPayloadV1::Colony(d) = payload else {
        return Vec::new();
    };
    let mut rows = Vec::with_capacity(48);

    push_drive(&mut rows, d, frames);
    push_households(&mut rows, d);
    push_professions(&mut rows, d);
    push_stock(&mut rows, d);
    push_jobs(&mut rows, d);
    rows
}

/// ★ THE DRIVE, ITS REASON, ITS MAGNITUDE AND ITS AGE.
///
/// `colony_drive_for` returns `(drive, reason, value)` and the board keeps
/// only the drive — so the reason has never been visible anywhere but a log
/// line. The verdict block below is the ladder re-run over freshly measured
/// inputs, with every input printed beside the answer.
fn push_drive(rows: &mut Vec<InspectRow>, d: &ColonySectionV1, frames: &InspectFramesV1) {
    let days = if frames.ticks_per_game_day > 0.0 {
        d.drive_held_ticks as f64 / frames.ticks_per_game_day
    } else {
        0.0
    };
    rows.push(
        InspectRow::new(
            "Colony drive",
            format!(
                "{}   held {} ticks ({days:.2} game days)",
                drive_label(d.drive),
                d.drive_held_ticks
            ),
            "JobBoard::colony_drive",
            "",
            FrameV1::JobBoard,
        )
        .scoped("RUNTIME-ONLY — resets to the default at every server start"),
    );
    let Some(v) = d.verdict.as_ref() else {
        rows.push(
            InspectRow::new(
                "  ladder",
                "not measured this request",
                "colony_drive_for",
                "",
                FrameV1::Derived,
            )
            .scoped("NOT the same as 'the ladder is satisfied'"),
        );
        return;
    };
    rows.push(
        InspectRow::new(
            "  because",
            format!("{} = {:.2}   (bar {:.2})", v.deciding, v.value, v.bar),
            "colony_drive_for -> (drive, deciding, value)",
            "",
            FrameV1::Derived,
        )
        // ★ THE SUSTAIN BAND. The threshold is not a constant — leaving
        // Sustain needs a higher bar than entering it — so a value without
        // its bar is unreadable, and `colony_sustain_bar` is the ONE
        // producer the gate itself reads.
        .scoped("re-run at inspect time; the bar depends on where the drive stands"),
    );
    // ★ THE DWELL-SUPPRESSED STATE, MADE VISIBLE. When the ladder wants a
    // different drive than the one held, the colony is either
    // mid-transition or the dwell timer is eating the transition — a state
    // that previously existed only in a decimated log line.
    if v.want != d.drive {
        rows.push(
            InspectRow::new(
                "  wants",
                format!(
                    "{}  — the ladder disagrees with the held drive (mid-transition, \
                     or the dwell timer is suppressing it)",
                    drive_label(v.want)
                ),
                "colony_drive_for(current = the held drive)",
                "",
                FrameV1::Derived,
            )
            .scoped("held is JobBoard-frame; wanted is measured NOW")
            .alarming(),
        );
    }
    rows.push(
        InspectRow::new(
            "  food",
            format!(
                "{:.2}/head   ({} total, of which {} in stockpiles, {} held)",
                v.food_per_cap,
                v.food_total,
                v.food_pantry,
                v.food_total.saturating_sub(v.food_pantry)
            ),
            "colony_drive_food_per_cap(colony_food_stock, colony_food_total)",
            "meals per colonist",
            FrameV1::Ecs,
        )
        // The frame fix that shipped in the producer: the TOTAL decides
        // (it is what `EatFrom` draws from) and the pantry witnesses. A
        // colony of 46 once read `food_per_cap=0.0` while its people
        // carried and ate their dinner.
        .scoped("the TOTAL decides; the pantry is the witness"),
    );
    rows.push(
        InspectRow::new(
            "  threats",
            v.threats.to_string(),
            "Agent::target.hostile over the loaded roster",
            "colonists perceiving a hostile",
            FrameV1::Ecs,
        )
        .scoped("the colony's OWN perception — not every nearby stranger"),
    );
    rows.push(
        InspectRow::new(
            "  beds vs pop",
            format!("{} beds / {} colonists", v.beds, v.pop),
            "JobBoard::beds.len() vs the loaded roster",
            "",
            FrameV1::Derived,
        )
        .scoped("beds are JobBoard-frame, the roster is ECS — two stores, one row"),
    );
}

/// ★ "ONE COLONIST PER HOUSE", MADE VISIBLE.
fn push_households(rows: &mut Vec<InspectRow>, d: &ColonySectionV1) {
    let occupied = d.households.iter().filter(|h| !h.members.is_empty()).count();
    rows.push(
        InspectRow::new(
            "Households",
            format!(
                "{} derived — {occupied} occupied, {} vacant",
                d.households.len(),
                d.households.len() - occupied
            ),
            "derive_households(JobBoard::designated, JobBoard::beds)",
            "houses",
            FrameV1::JobBoard,
        )
        .scoped("re-derived every sweep; never a stored ledger"),
    );
    rows.push(
        InspectRow::new(
            "  beds",
            format!(
                "{} total, {} in no household",
                d.beds_total, d.beds_outside_households
            ),
            "JobBoard::beds",
            "bed slots",
            FrameV1::JobBoard,
        )
        // A bedroll on open ground houses nobody as far as the population
        // loop is concerned, and the loop's growth gate reads bed counts.
        .scoped("a bed outside every Bed region is an open-ground bedroll"),
    );
    for (i, h) in d.households.iter().enumerate() {
        let who = if h.members.is_empty() {
            "vacant".to_string()
        } else {
            h.members
                .iter()
                .map(|m| match &m.name {
                    Some(n) => n.clone(),
                    // A bed owned by someone who has unloaded. Real, and
                    // said rather than hidden behind the number.
                    None => format!("uid:{} (unloaded)", m.uid.0.get()),
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        rows.push(
            InspectRow::new(
                format!("  house {i}"),
                format!(
                    "({},{},{})-({},{},{})   {} bed(s), capacity {} — {who}",
                    h.min.x, h.min.y, h.min.z, h.max.x, h.max.y, h.max.z, h.beds, h.capacity
                ),
                "HouseholdView + household_capacity(beds)",
                "",
                FrameV1::JobBoard,
            )
            .scoped("members are the BED OWNERS, uid-sorted; [0] is the head")
            // Ben's ruling is 1..6 to a house. More owners than beds is
            // the invariant breaking, not a crowded family.
            .alarming_if(h.members.len() as u32 > h.capacity),
        );
    }
}

/// ★ THE OWNER'S ACCEPTANCE CRITERION AS A NUMBER: "name someone's job
/// from an hour of watching". The bucket that has to fall is `unnamed`.
fn push_professions(rows: &mut Vec<InspectRow>, d: &ColonySectionV1) {
    rows.push(
        InspectRow::new(
            "Professions",
            format!(
                "{} of {} loaded colonists named",
                d.roster_loaded.saturating_sub(d.profession_unnamed),
                d.roster_loaded
            ),
            "JobBoard::professions, keyed over the loaded roster",
            "colonists",
            FrameV1::Derived,
        )
        .scoped("board table (runtime-only) read against the ECS roster"),
    );
    // Over `WorkType::ALL` by construction: the payload is a fixed array
    // sized by `COUNT`, so a new lane appears here with no edit. Three
    // hand-written variant lists in this codebase have already fallen
    // behind exactly this enum.
    for w in WorkType::ALL {
        rows.push(
            InspectRow::new(
                format!("  {}", w.label()),
                d.professions[w.lane_index()].to_string(),
                "JobBoard::professions",
                "colonists",
                FrameV1::JobBoard,
            )
            .scoped("the rolling dominant lane, with hysteresis, recomputed daily"),
        );
    }
    rows.push(
        InspectRow::new(
            "  unnamed",
            d.profession_unnamed.to_string(),
            "loaded colonists absent from JobBoard::professions",
            "colonists",
            FrameV1::Derived,
        )
        // NOT "unemployed": the board derives professions once a game-day
        // and starts empty at every boot, so on day 0 everyone is here.
        .scoped("not yet derived — day 0, or since the last restart"),
    );
    if d.professions_board_entries != d.roster_loaded.saturating_sub(d.profession_unnamed) {
        rows.push(
            InspectRow::new(
                "  board entries",
                format!(
                    "{} — the board names colonists the ECS roster does not currently hold",
                    d.professions_board_entries
                ),
                "JobBoard::professions.len()",
                "entries",
                FrameV1::JobBoard,
            )
            // Reported, never subtracted: the board outlives the loaded
            // roster, and subtracting one frame from another is how rows
            // get lost here.
            .scoped("two frames, reported side by side and NOT differenced"),
        );
    }
}

/// ★ EVERY STOCK ROW IS A BREAKDOWN, NEVER A SCALAR.
fn push_stock(rows: &mut Vec<InspectRow>, d: &ColonySectionV1) {
    rows.push(
        InspectRow::new(
            "Stock",
            format!(
                "{} distinct item definitions{}",
                d.stock_distinct,
                if d.stock_truncated { " (list capped — heaviest shown)" } else { "" }
            ),
            "PickupItem + Inventory census, scoped by JobBoard::stockpile_at",
            "item definitions",
            FrameV1::Ecs,
        )
        .scoped("three DISJOINT scopes: stockpiles, carried, ground"),
    );
    // The payload is a flat `(label, scope, count)` list in heaviest-first
    // order with the four scopes adjacent, so one pass groups it without
    // sorting or a map.
    let mut i = 0;
    while i < d.stock.len() {
        let label = &d.stock[i].item_label;
        let mut parts: Vec<String> = Vec::with_capacity(3);
        let mut total = 0u32;
        while i < d.stock.len() && &d.stock[i].item_label == label {
            let r = &d.stock[i];
            if r.scope == StockScopeV1::Total {
                total = r.count;
            } else {
                parts.push(format!("{} {}", r.count, r.scope.label()));
            }
            i += 1;
        }
        rows.push(
            InspectRow::new(
                format!("  {label}"),
                format!("{} ({total} total)", parts.join(" · ")),
                "PickupItem::amount / Inventory slots, by scope",
                "units",
                FrameV1::Ecs,
            )
            // THE row this whole type exists for. A bare `0` here reads as
            // "the forge is broken"; `0 · 64 · 3` reads as "hauling is
            // broken", and they are different repairs.
            .scoped("units, not stacks — a 64-stone pile is 64"),
        );
    }
}

fn push_jobs(rows: &mut Vec<InspectRow>, d: &ColonySectionV1) {
    let j = &d.jobs;
    rows.push(
        InspectRow::new(
            "Jobs",
            format!("{} on the board, {} claimed", j.total, j.claimed),
            "JobBoard::jobs (one scan — bastion_inspector::tally_jobs)",
            "jobs",
            FrameV1::JobBoard,
        )
        .scoped("claimed means ACTIVELY HELD, not merely owned"),
    );
    for (label, n, producer, scope) in [
        (
            "  blocked: no stance",
            j.blocked_stance,
            "job_stance_missing (the claim gate's own predicate)",
            "unclaimed jobs with no cell a colonist can stand in",
        ),
        (
            "  unreachable",
            j.unreachable,
            "Job::unreachable",
            "counted regardless of claim",
        ),
        (
            "  blocked: materials",
            j.blocked_materials,
            "stockpile_has_material (the fetch leg's own rule)",
            "per JOB, never per (job, colonist) pair; Haul jobs excluded",
        ),
    ] {
        rows.push(
            InspectRow::new(label, n.to_string(), producer, "jobs", FrameV1::JobBoard)
                .scoped(scope),
        );
    }
    rows.push(
        InspectRow::new(
            "Designations",
            d.designations.to_string(),
            "JobBoard::designated_regions",
            "regions",
            FrameV1::JobBoard,
        )
        .scoped("standing paint orders, not jobs"),
    );
    rows.push(
        InspectRow::new(
            "Sampled at",
            d.tick.to_string(),
            "bastion_server::Tick",
            "server ticks",
            FrameV1::JobBoard,
        )
        .scoped("BOOT-RELATIVE — resets to 0 every start"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::comp::bastion_inspect::{
        ColonyDriveVerdictV1, HouseholdMemberV1, HouseholdRowV1, JobTallyV1, RowSeverityV1,
        StockRowV1,
    };

    fn frames() -> InspectFramesV1 {
        InspectFramesV1 {
            server_tick: 10_000,
            rtsim_tick: 1,
            time_of_day: 0.0,
            ticks_per_game_day: 54_000.0,
            schedule_offset_hours: 0,
        }
    }

    fn uid(n: u64) -> common::uid::Uid {
        common::uid::Uid(std::num::NonZeroU64::new(n).expect("nonzero"))
    }

    fn hammer(scope: StockScopeV1, count: u32) -> StockRowV1 {
        StockRowV1 {
            item_label: "common.items.tool.craftsman_hammer".into(),
            count,
            scope,
        }
    }

    fn payload() -> SectionPayloadV1 {
        let mut professions = [0u32; WorkType::COUNT];
        professions[WorkType::Farm.lane_index()] = 3;
        professions[WorkType::Craft.lane_index()] = 1;
        SectionPayloadV1::Colony(ColonySectionV1 {
            drive: common::bastion::ColonyDrive::Sustain,
            drive_since_tick: 1_000,
            drive_held_ticks: 9_000,
            verdict: Some(ColonyDriveVerdictV1 {
                want: common::bastion::ColonyDrive::Sustain,
                deciding: "food_per_cap".into(),
                value: 1.5,
                bar: 3.0,
                food_per_cap: 1.5,
                food_pantry: 2,
                food_total: 9,
                threats: 0,
                beds: 4,
                pop: 6,
            }),
            households: vec![
                HouseholdRowV1 {
                    min: vek::Vec3::new(0, 0, 5),
                    max: vek::Vec3::new(4, 4, 8),
                    beds: 2,
                    capacity: 2,
                    members: vec![
                        HouseholdMemberV1 { uid: uid(1), name: Some("Hedda".into()) },
                        HouseholdMemberV1 { uid: uid(2), name: None },
                    ],
                },
                HouseholdRowV1 {
                    min: vek::Vec3::new(9, 0, 5),
                    max: vek::Vec3::new(13, 4, 8),
                    beds: 1,
                    capacity: 1,
                    members: Vec::new(),
                },
            ],
            beds_total: 4,
            beds_outside_households: 1,
            professions,
            profession_unnamed: 2,
            roster_loaded: 6,
            professions_board_entries: 4,
            stock: vec![
                hammer(StockScopeV1::InStockpileRegions, 0),
                hammer(StockScopeV1::CarriedByColonists, 64),
                hammer(StockScopeV1::OnGroundAnywhere, 3),
                hammer(StockScopeV1::Total, 67),
            ],
            stock_distinct: 1,
            stock_truncated: false,
            jobs: JobTallyV1 {
                total: 30,
                claimed: 4,
                blocked_stance: 5,
                unreachable: 2,
                blocked_materials: 7,
            },
            designations: 11,
            tick: 10_000,
        })
    }

    /// ★ THE STOCK LINE IS A BREAKDOWN AND ITS PARTS ADD UP.
    ///
    /// This is the row the whole `StockRowV1` type exists for: `0` alone
    /// says "the forge is broken", and `0 in stockpiles · 64 carried · 3
    /// on ground` says "hauling is broken".
    ///
    /// FALSIFIER: render only the `Total` scope and the three-scope
    /// assertion goes RED; drop the `StockScopeV1::Total` skip in the
    /// grouping loop and the total appears twice.
    #[test]
    fn a_stock_row_shows_every_scope_and_its_total() {
        let r = rows(&payload(), &frames());
        let row = r
            .iter()
            .find(|x| x.label().contains("craftsman_hammer"))
            .expect("the hammer must have a row");
        let v = row.value();
        assert!(v.contains("0 in stockpiles"), "got {v}");
        assert!(v.contains("64 carried"), "got {v}");
        assert!(v.contains("3 on ground"), "got {v}");
        assert!(v.contains("(67 total)"), "got {v}");
        // Exactly one line per item, not one per scope.
        assert_eq!(
            r.iter().filter(|x| x.label().contains("craftsman_hammer")).count(),
            1,
            "the four scopes must collapse into ONE line"
        );
    }

    /// The professions histogram covers every lane INCLUDING the one three
    /// hand-written lists dropped, and the unnamed bucket is a row of its
    /// own that does not read as "unemployed".
    ///
    /// FALSIFIER: iterate a hand-written lane list instead of
    /// `WorkType::ALL` and the `craft` assertion goes RED.
    #[test]
    fn every_lane_gets_a_row_and_unnamed_is_its_own_bucket() {
        let r = rows(&payload(), &frames());
        for w in WorkType::ALL {
            assert!(
                r.iter().any(|x| x.label() == format!("  {}", w.label())),
                "{w:?} has no row"
            );
        }
        assert_eq!(
            r.iter().find(|x| x.label() == "  craft").expect("craft row").value(),
            "1"
        );
        let unnamed = r.iter().find(|x| x.label() == "  unnamed").expect("unnamed row");
        assert_eq!(unnamed.value(), "2");
        assert!(
            unnamed.scope().is_some_and(|s| s.contains("not yet derived")),
            "an underived profession must not read as unemployment"
        );
        // The histogram plus the unnamed bucket accounts for the roster.
        let named: u32 = WorkType::ALL
            .iter()
            .map(|w| {
                r.iter()
                    .find(|x| x.label() == format!("  {}", w.label()))
                    .unwrap()
                    .value()
                    .parse::<u32>()
                    .unwrap()
            })
            .sum();
        assert_eq!(named + 2, 6, "the histogram must conserve the roster");
    }

    /// ★ THE DRIVE CARRIES ITS REASON, ITS BAR AND ITS AGE, and a ladder
    /// that disagrees with the held drive is an ALARM — the
    /// dwell-suppressed state, which had no player-facing witness at all.
    ///
    /// FALSIFIER: drop the `v.want != d.drive` guard and the agreeing case
    /// grows a "wants" row — RED.
    #[test]
    fn the_drive_row_names_its_reason_and_flags_a_disagreement() {
        let r = rows(&payload(), &frames());
        let drive = r.iter().find(|x| x.label() == "Colony drive").expect("drive row");
        assert!(drive.value().contains("Sustain"), "got {}", drive.value());
        assert!(drive.value().contains("9000 ticks"), "got {}", drive.value());
        assert!(drive.value().contains("0.17 game days"), "got {}", drive.value());
        let why = r.iter().find(|x| x.label() == "  because").expect("reason row");
        assert!(why.value().contains("food_per_cap"), "got {}", why.value());
        assert!(why.value().contains("bar 3.00"), "the BAND, not a constant: {}", why.value());
        assert!(
            !r.iter().any(|x| x.label() == "  wants"),
            "an agreeing ladder must not add a row"
        );

        // Now make the ladder disagree.
        let mut p = payload();
        if let SectionPayloadV1::Colony(c) = &mut p {
            c.verdict.as_mut().unwrap().want = common::bastion::ColonyDrive::Defend;
        }
        let r = rows(&p, &frames());
        let wants = r.iter().find(|x| x.label() == "  wants").expect("disagreement row");
        assert_eq!(wants.severity(), RowSeverityV1::Alarm);
        assert!(wants.value().contains("Defend"), "got {}", wants.value());
        assert!(wants.value().contains("dwell"), "got {}", wants.value());
    }

    /// Household members are NAMED, and an owner the roster cannot resolve
    /// says so rather than disappearing.
    #[test]
    fn households_name_their_members_and_admit_the_ones_they_cannot() {
        let r = rows(&payload(), &frames());
        let h0 = r.iter().find(|x| x.label() == "  house 0").expect("house 0");
        assert!(h0.value().contains("Hedda"), "got {}", h0.value());
        assert!(h0.value().contains("uid:2 (unloaded)"), "got {}", h0.value());
        assert!(h0.value().contains("capacity 2"), "got {}", h0.value());
        let h1 = r.iter().find(|x| x.label() == "  house 1").expect("house 1");
        assert!(h1.value().contains("vacant"), "got {}", h1.value());
        // A house over capacity is an alarm; these are not.
        assert_eq!(h0.severity(), RowSeverityV1::Normal);

        let mut p = payload();
        if let SectionPayloadV1::Colony(c) = &mut p {
            c.households[0].capacity = 1;
        }
        let r = rows(&p, &frames());
        assert_eq!(
            r.iter().find(|x| x.label() == "  house 0").unwrap().severity(),
            RowSeverityV1::Alarm,
            "more owners than the house holds is the invariant breaking"
        );
    }

    /// A section that was not measured refuses in the ladder row rather
    /// than reading as a satisfied colony.
    #[test]
    fn an_unmeasured_verdict_says_so_rather_than_reading_as_satisfied() {
        let mut p = payload();
        if let SectionPayloadV1::Colony(c) = &mut p {
            c.verdict = None;
        }
        let r = rows(&p, &frames());
        let ladder = r.iter().find(|x| x.label() == "  ladder").expect("ladder row");
        assert!(ladder.value().contains("not measured"), "got {}", ladder.value());
        assert!(!r.iter().any(|x| x.label() == "  because"));
    }

    /// Every row names a producer and a frame, across both branches.
    #[test]
    fn every_row_names_a_producer() {
        let mut all = rows(&payload(), &frames());
        let mut bare = payload();
        if let SectionPayloadV1::Colony(c) = &mut bare {
            c.verdict = None;
            c.households.clear();
            c.stock.clear();
        }
        all.extend(rows(&bare, &frames()));
        assert!(all.len() > 30, "the fixture must exercise the view");
        for r in &all {
            assert!(!r.producer().is_empty(), "row '{}' names no producer", r.label());
            assert!(!r.provenance().is_empty());
        }
        // The frame discipline: the board rows and the ECS rows are
        // labelled differently, so a reader cannot take them for one
        // store.
        assert!(all.iter().any(|r| r.frame() == FrameV1::JobBoard));
        assert!(all.iter().any(|r| r.frame() == FrameV1::Ecs));
    }
}
