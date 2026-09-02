//! bastion (INSPECTOR-M1): the client half of the modular colonist
//! inspector — the SECTION VIEW REGISTRY.
//!
//! One view per [`SectionIdV1`], registered in [`view_for`], which is a
//! match with NO wildcard arm. Appending a section id is a compile error
//! here until a view exists for it, exactly as it is on the server for the
//! provider.
//!
//! ★ THE VIEWS ARE PURE. A view maps `SectionPayloadV1 -> Vec<InspectRow>`
//! and touches no conrod, no renderer and no ECS. Drawing is a separate,
//! generic pass over rows, so a new section needs no drawing code at all —
//! which is what "modular so we can keep expanding it" has to mean if it
//! is to survive more than two additions.
//!
//! ★ EVERY ROW NAMES ITS PRODUCER. `InspectRow` cannot be constructed
//! without one, so this is a property of the type rather than a habit of
//! the author.

use common::comp::bastion_inspect::{
    FrameV1, InspectFramesV1, InspectRow, RowSeverityV1, SectionIdV1, SectionPayloadV1,
};

pub mod colony;
pub mod identity;
pub mod path;
pub mod right_now;
pub mod thinking;
/// INSPECTOR-M2: the readable panel (scroll + fold) over this model.
pub mod panel;

/// A section view. Pure.
pub type ViewFn = fn(&SectionPayloadV1, &InspectFramesV1) -> Vec<InspectRow>;

/// ★ THE CLIENT REGISTRY. NO WILDCARD ARM.
pub const fn view_for(id: SectionIdV1) -> ViewFn {
    match id {
        SectionIdV1::Identity => identity::rows,
        SectionIdV1::RightNow => right_now::rows,
        SectionIdV1::Path => path::rows,
        SectionIdV1::Thinking => thinking::rows,
        SectionIdV1::Colony => colony::rows,
    }
}

/// Render one section's payload.
///
/// The `Unavailable` arm is handled HERE rather than in each view, so
/// every section refuses in the same words and no view can forget to
/// handle a refusal. A refusal still produces a ROW: a section that
/// silently rendered nothing would be indistinguishable from a section
/// with nothing to say, which is the ambiguity the whole design removes.
pub fn rows_for(payload: &SectionPayloadV1, frames: &InspectFramesV1) -> Vec<InspectRow> {
    if let SectionPayloadV1::Unavailable(id, reason) = payload {
        return vec![InspectRow::new(
            id.title(),
            reason.label(),
            "SectionPayloadV1::Unavailable",
            "",
            FrameV1::Derived,
        )];
    }
    view_for(payload.id())(payload, frames)
}

/// ★ THE TWO-CLOCK HEADER.
///
/// Both clocks, both named, every time — because they are not
/// interchangeable and the reader will otherwise assume they are:
///
/// * `server_tick` resets to 0 at every process boot.
/// * `rtsim_tick` survives restart and is the ONLY clock an age may be
///   computed against.
/// * `time_of_day` is game-seconds whose DAY INDEX is boot-relative.
///
/// The colonist's own hour is also shown whenever it differs from the wall
/// hour, because a night watchman's schedule is rotated by 14 hours and
/// reading the global clock for a rotated colonist has already shipped
/// once as a real defect.
pub fn header_rows(frames: &InspectFramesV1, loaded: bool) -> Vec<InspectRow> {
    use common::bastion::game_time;

    let wall_hour = game_time::hour_of_day(frames.time_of_day);
    let own_hour = game_time::colonist_hour(frames.time_of_day, frames.schedule_offset_hours);

    let mut rows = vec![
        InspectRow::new(
            if loaded { "Subject" } else { "Subject (unloaded)" },
            if loaded {
                "loaded — ECS and roster both readable"
            } else {
                "unloaded — showing roster state only"
            },
            "IdMaps::uid_entity",
            "",
            FrameV1::Derived,
        ),
        InspectRow::new(
            "Wall hour",
            format!("{wall_hour:02}:00"),
            "game_time::hour_of_day(TimeOfDay)",
            "game hour",
            FrameV1::Ecs,
        )
        .scoped("the colony's global clock"),
    ];

    // Only shown when it actually differs: a row that always reads the
    // same as the one above it trains the reader to skip both.
    if frames.schedule_offset_hours % 24 != 0 {
        rows.push(
            InspectRow::new(
                "Their hour",
                format!(
                    "{own_hour:02}:00  (schedule rotated {}h)",
                    frames.schedule_offset_hours % 24
                ),
                "game_time::colonist_hour",
                "game hour",
                FrameV1::Derived,
            )
            .scoped("this colonist's own schedule frame"),
        );
    }

    rows.push(
        InspectRow::new(
            "rtsim tick",
            frames.rtsim_tick.to_string(),
            "rtsim Data.tick",
            "rtsim ticks",
            FrameV1::RtsimRoster,
        )
        .scoped("persistent — survives restart"),
    );
    rows.push(
        InspectRow::new(
            "server tick",
            frames.server_tick.to_string(),
            "bastion_server::Tick",
            "server ticks",
            FrameV1::Ecs,
        )
        .scoped("BOOT-RELATIVE — resets to 0 every start"),
    );
    rows.push(
        InspectRow::new(
            "Game day",
            format!("{:.0} ticks", frames.ticks_per_game_day),
            "game_time::ticks_per_game_day(dt, day_cycle_coefficient)",
            "ticks/game-day",
            FrameV1::Derived,
        )
        .scoped("this server's settings, not a constant"),
    );
    rows
}

/// One rendered section: its heading, its rows, and how OLD its payload
/// is against the reply's own clock.
pub struct RenderedSection {
    pub id: SectionIdV1,
    pub rows: Vec<InspectRow>,
    /// Server ticks between this section's answer and the header's clocks.
    /// `Some(0)` is fresh; `None` is "unknown", which happens only when
    /// nothing is tracking ages.
    ///
    /// ★ WHY THIS IS NOT COSMETIC. Sections refresh at different cadences
    /// and the subscription CARRIES FORWARD a section the newest reply did
    /// not answer (otherwise every slow section blinks out three refreshes
    /// in four). A carried section's rows were computed at an earlier tick
    /// than the clocks above them — two frames on one screen, the defect
    /// class this subsystem loses most rows to — so the age is printed
    /// rather than left for the reader to assume away.
    pub age_ticks: Option<u64>,
}

/// Render a whole reply into headed sections, in registry order.
///
/// `age_of` supplies each section's staleness; pass `|_| None` when
/// nothing is tracking it.
pub fn render(
    reply: &common::comp::bastion_inspect::SectionedInspectV1,
    age_of: impl Fn(SectionIdV1) -> Option<u64>,
) -> Vec<RenderedSection> {
    reply
        .sections
        .iter()
        .map(|p| RenderedSection {
            id: p.id(),
            rows: rows_for(p, &reply.frames),
            age_ticks: age_of(p.id()),
        })
        .collect()
}

/// The clock header as text lines, under its own heading.
///
/// The header is NOT a section: it has no id, no provider and no view. It
/// is rendered separately so it cannot be mistaken for one — a fake
/// `SectionIdV1` for the header would have had to be given a provider and
/// a view that answer nothing, which is exactly the kind of always-passing
/// stub a compiler-enforced registry exists to make impossible.
pub fn header_lines(
    frames: &InspectFramesV1,
    loaded: bool,
    verbose: bool,
) -> Vec<String> {
    let mut out = vec!["- CLOCKS -".to_string()];
    out.extend(row_lines(&header_rows(frames, loaded), verbose));
    out
}

/// The prefix an [`RowSeverityV1::Alarm`] row carries in the text block.
///
/// ★ A SEVERITY, NOT A COLOUR — and a marker rather than nothing. The HUD
/// draws one unstyled text block today, so an alarm row that rendered
/// identically to its forty neighbours would be invisible in exactly the
/// case it exists for (a stale `Mood` mirror, a disabled chronicle, a
/// household over capacity). When this block grows real conrod styling the
/// marker becomes a colour and this constant goes away; until then the
/// severity is on the screen rather than only in the type.
pub const ALARM_PREFIX: &str = "!! ";

/// One row per line. Shared by the header and by every section, so
/// provenance renders identically wherever it appears.
pub(crate) fn row_lines(rows: &[InspectRow], verbose: bool) -> Vec<String> {
    rows.iter()
        .map(|r| {
            let unit = if r.unit().is_empty() {
                String::new()
            } else {
                format!(" {}", r.unit())
            };
            let mark = match r.severity() {
                RowSeverityV1::Normal => "",
                RowSeverityV1::Alarm => ALARM_PREFIX,
            };
            if verbose {
                format!("{mark}{}: {}{}   [{}]", r.label(), r.value(), unit, r.provenance())
            } else {
                format!("{mark}{}: {}{}", r.label(), r.value(), unit)
            }
        })
        .collect()
}

/// Flatten to plain text lines, for the placeholder-first text block the
/// HUD draws today.
///
/// `verbose` appends each row's provenance. It is off by default because
/// provenance beside every row is unreadable in normal play — but it is
/// one flag away, and it is the difference between "tools: 0" and "tools:
/// 0 | scope: this stockpile", which is the difference between a player
/// concluding the forge is broken and knowing to look elsewhere.
pub fn to_lines(sections: &[RenderedSection], verbose: bool) -> Vec<String> {
    let mut out = Vec::new();
    for s in sections {
        // ★ A CARRIED SECTION SAYS SO. Sections refresh at different
        // cadences, so a slow one is routinely older than the clocks in
        // the header. Printing the gap is the difference between two
        // frames on one screen and two frames LABELLED on one screen.
        out.push(match s.age_ticks {
            Some(0) | None => format!("- {} -", s.id.title()),
            Some(age) => format!("- {} -   (as of {age} server ticks ago)", s.id.title()),
        });
        out.extend(row_lines(&s.rows, verbose));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{
        bastion::WorkType,
        comp::bastion_inspect::{
            IdentitySectionV1, PathSectionV1, RightNowSectionV1, SectionedInspectV1,
            UnavailableReasonV1,
        },
        uid::Uid,
    };

    fn frames(offset: u32) -> InspectFramesV1 {
        InspectFramesV1 {
            server_tick: 900,
            rtsim_tick: 54_000 * 5 + 100,
            time_of_day: 2.0 * 3600.0,
            ticks_per_game_day: 54_000.0,
            schedule_offset_hours: offset,
        }
    }

    fn identity() -> SectionPayloadV1 {
        SectionPayloadV1::Identity(IdentitySectionV1 {
            name: "Hedda".into(),
            profession: Some(WorkType::Craft),
            assigned_zone: None,
            born_tick: Some(54_000),
            born_day_boot_relative: Some(0),
            parent_name: Some("Orm".into()),
            backstory: "charcoal burner".into(),
            owned_bed: Some(vek::Vec3::new(1, 2, 3)),
            bed_slot_agrees: Some(true),
            health: Some(0.75),
            guard_bravery: 0.4,
            skills: [1, 2, 3, 4, 5, 6, 7, 8],
            desires: [1.0; WorkType::COUNT],
        })
    }

    fn every_payload() -> Vec<SectionPayloadV1> {
        vec![
            identity(),
            SectionPayloadV1::RightNow(RightNowSectionV1 {
                drive: common::comp::bastion::Drive::Work,
                last_scores: (1.0, 0.0, 0.5),
                activity: Some((WorkType::Chop, 0.45)),
                status: None,
                pos: Some(vek::Vec3::new(10.0, 20.0, 30.0)),
                job: None,
            }),
            SectionPayloadV1::Path(PathSectionV1 {
                nodes: vec![vek::Vec3::new(0, 0, 0), vek::Vec3::new(1, 0, 0)],
                next_idx: 1,
                total_nodes: 2,
                truncated: false,
                needs_search: false,
                nodes_hash: 7,
            }),
            SectionPayloadV1::Unavailable(
                SectionIdV1::RightNow,
                UnavailableReasonV1::SubjectUnloaded,
            ),
            SectionPayloadV1::Thinking(
                common::comp::bastion_inspect::ThinkingSectionV1 {
                    mood_mirror: Some(0.6),
                    explanation: None,
                    needs: Some((0.4, 0.4, 0.4)),
                    energy: Some(0.9),
                    guard_bravery: 0.4,
                    traits: vec!["Stable".into()],
                    values: vec![(common::bastion::Value::Craft, -10)],
                    sentiments: Vec::new(),
                    chronicle: common::comp::bastion_inspect::ChronicleViewV1 {
                        enabled: true,
                        truncated: false,
                        total: 0,
                        hidden_released: 0,
                        raw: false,
                        rows: Vec::new(),
                        row_cap: 64,
                    },
                },
            ),
            SectionPayloadV1::Colony(common::comp::bastion_inspect::ColonySectionV1 {
                drive: common::bastion::ColonyDrive::Grow,
                drive_since_tick: 10,
                drive_held_ticks: 890,
                verdict: None,
                households: Vec::new(),
                beds_total: 0,
                beds_outside_households: 0,
                professions: [0; WorkType::COUNT],
                profession_unnamed: 0,
                roster_loaded: 0,
                professions_board_entries: 0,
                stock: Vec::new(),
                stock_distinct: 0,
                stock_truncated: false,
                jobs: common::comp::bastion_inspect::JobTallyV1::default(),
                designations: 0,
                tick: 900,
            }),
        ]
    }

    /// ★ EVERY SECTION ID HAS A VIEW, and each view answers for the
    /// section it was registered under.
    ///
    /// FALSIFIER: swap two arms in `view_for` and the round-trip below
    /// produces the wrong rows — caught by the label check.
    #[test]
    fn inspect_section_ids_are_total() {
        let f = frames(0);
        for id in SectionIdV1::ALL {
            let _view: ViewFn = view_for(id);
        }
        // Miswiring check: each real payload must produce rows whose
        // content belongs to ITS section.
        let rows = rows_for(&identity(), &f);
        assert!(rows.iter().any(|r| r.value().contains("Hedda")), "Identity rendered the wrong view");
    }

    /// ★ EVERY ROW NAMES A PRODUCER — walked over every view, for every
    /// payload shape including the refusal.
    ///
    /// FALSIFIER: delete the `assert!` in `InspectRow::new` AND pass `""`
    /// as a producer somewhere in a view; this then goes RED instead of
    /// panicking at construction.
    #[test]
    fn every_row_names_a_producer() {
        for offset in [0u32, 14] {
            let f = frames(offset);
            let mut all: Vec<InspectRow> = header_rows(&f, true);
            all.extend(header_rows(&f, false));
            for p in every_payload() {
                all.extend(rows_for(&p, &f));
            }
            assert!(all.len() > 20, "the fixture must actually exercise the views");
            for r in &all {
                assert!(!r.producer().is_empty(), "row '{}' names no producer", r.label());
                assert!(!r.label().is_empty(), "a row has no label");
                assert!(
                    !r.provenance().is_empty(),
                    "row '{}' renders no provenance",
                    r.label()
                );
            }
        }
    }

    /// The header names BOTH clocks and marks the boot-relative one as
    /// such, and it shows the colonist's own hour ONLY when rotated.
    ///
    /// FALSIFIER: drop the `schedule_offset_hours != 0` guard and the
    /// unrotated case grows a "Their hour" row — RED.
    #[test]
    fn header_names_both_clocks_and_the_rotation() {
        let plain = header_rows(&frames(0), true);
        assert!(plain.iter().any(|r| r.label() == "rtsim tick"));
        assert!(plain.iter().any(|r| r.label() == "server tick"));
        assert!(
            plain
                .iter()
                .find(|r| r.label() == "server tick")
                .expect("server tick row")
                .scope()
                .is_some_and(|s| s.contains("BOOT-RELATIVE")),
            "the boot-relative clock must be labelled as such"
        );
        assert!(
            !plain.iter().any(|r| r.label() == "Their hour"),
            "an unrotated colonist needs no second hour row"
        );

        // A night watchman: wall 02, own 12.
        let watch = header_rows(&frames(14), true);
        let own = watch.iter().find(|r| r.label() == "Their hour").expect("rotated row");
        assert!(own.value().starts_with("12:00"), "wall 02 must rotate to own hour 12");
        let wall = watch.iter().find(|r| r.label() == "Wall hour").expect("wall row");
        assert!(wall.value().starts_with("02:00"));
    }

    /// An unloaded subject is SAID to be unloaded, in the header and in
    /// each refusing section.
    #[test]
    fn unloaded_is_stated_not_blanked() {
        let f = frames(0);
        let h = header_rows(&f, false);
        assert!(
            h.iter().any(|r| r.value().contains("unloaded")),
            "the header must say the subject is unloaded"
        );
        let refusal = rows_for(
            &SectionPayloadV1::Unavailable(SectionIdV1::Path, UnavailableReasonV1::SubjectUnloaded),
            &f,
        );
        assert_eq!(refusal.len(), 1, "a refusal must still produce a row");
        assert!(refusal[0].value().contains("unloaded"));
    }

    /// The text flattening keeps every row and heads every section.
    #[test]
    fn to_lines_keeps_every_row() {
        let f = frames(0);
        let reply = SectionedInspectV1 {
            subject: Uid(std::num::NonZeroU64::new(1).expect("nonzero")),
            seq: 1,
            loaded: true,
            frames: f,
            sections: every_payload(),
        };
        let rendered = render(&reply, |_| None);
        let plain = to_lines(&rendered, false);
        let verbose = to_lines(&rendered, true);
        let row_count: usize = rendered.iter().map(|s| s.rows.len()).sum();
        assert_eq!(plain.len(), row_count + rendered.len(), "one heading per section");
        assert_eq!(verbose.len(), plain.len());
        assert!(
            verbose.iter().any(|l| l.contains("frame: ")),
            "verbose mode must carry provenance"
        );
        assert!(
            !plain.iter().any(|l| l.contains("frame: ")),
            "normal mode must not"
        );
    }

    /// ★ A CARRIED-FORWARD SECTION IS LABELLED WITH ITS AGE.
    ///
    /// The subscription retains a section the newest reply did not answer
    /// (otherwise every slow section blinks out three refreshes in four),
    /// so the panel routinely shows rows computed at an earlier tick than
    /// the clocks in its own header. TWO FRAMES COMPARED AS ONE is the
    /// defect class this subsystem loses most rows to; the heading is
    /// where it gets named.
    ///
    /// FALSIFIER: drop the `Some(age)` arm in `to_lines` and the stale
    /// heading becomes indistinguishable from the fresh one.
    #[test]
    fn a_stale_section_heading_says_how_old_it_is() {
        let f = frames(0);
        let reply = SectionedInspectV1 {
            subject: Uid(std::num::NonZeroU64::new(1).expect("nonzero")),
            seq: 1,
            loaded: true,
            frames: f,
            sections: vec![identity()],
        };
        let fresh = to_lines(&render(&reply, |_| Some(0)), false);
        assert_eq!(fresh[0], "- Identity -", "a fresh section carries no age note");

        let stale = to_lines(&render(&reply, |_| Some(42)), false);
        assert!(stale[0].contains("42 server ticks ago"), "got {}", stale[0]);

        // Unknown age renders like fresh rather than inventing a number.
        let unknown = to_lines(&render(&reply, |_| None), false);
        assert_eq!(unknown[0], "- Identity -");
    }

    /// ★ AN ALARM ROW IS VISIBLE IN THE TEXT BLOCK.
    ///
    /// The HUD draws one unstyled block, so a severity that lived only in
    /// the type would be invisible in exactly the cases it exists for.
    ///
    /// FALSIFIER: delete the `mark` from `row_lines` and this goes RED.
    #[test]
    fn an_alarm_row_is_marked_in_the_rendered_text() {
        let quiet = InspectRow::new("a", "b", "P", "", FrameV1::Derived);
        let loud = InspectRow::new("a", "b", "P", "", FrameV1::Derived).alarming();
        let lines = row_lines(&[quiet, loud], false);
        assert!(!lines[0].starts_with(ALARM_PREFIX), "a normal row must be quiet");
        assert!(lines[1].starts_with(ALARM_PREFIX), "an alarm row must be marked");
        // And the marking survives verbose mode, where a reader is
        // scanning provenance and needs the alarm even more.
        let loud = InspectRow::new("a", "b", "P", "", FrameV1::Derived).alarming();
        assert!(row_lines(&[loud], true)[0].starts_with(ALARM_PREFIX));
    }

    /// ★ EVERY REGISTERED SECTION HAS A VIEW THAT PRODUCES ROWS FOR ITS
    /// OWN PAYLOAD — the miswiring check, extended to phase 2.
    ///
    /// A registry can be total and still be MISWIRED (Colony's slot
    /// returning the Thinking view). That compiles; only a round-trip
    /// catches it, and each view returns an EMPTY vec for a payload that
    /// is not its own, so a swap shows up as a section with no rows.
    ///
    /// FALSIFIER: swap the `Thinking` and `Colony` arms in `view_for` and
    /// this goes RED on both.
    #[test]
    fn every_registered_section_renders_its_own_payload() {
        let f = frames(0);
        let mut covered = 0;
        for p in every_payload() {
            if matches!(p, SectionPayloadV1::Unavailable(..)) {
                continue;
            }
            let rows = rows_for(&p, &f);
            assert!(
                !rows.is_empty(),
                "{:?} rendered no rows — its view is wired to another section",
                p.id()
            );
            covered += 1;
        }
        assert_eq!(
            covered,
            SectionIdV1::COUNT,
            "the fixture must carry one real payload per registered section"
        );
    }

    // ----- INSPECTOR-M2: the panel is a projection of the same reply -----

    fn full_reply() -> SectionedInspectV1 {
        SectionedInspectV1 {
            subject: Uid(std::num::NonZeroU64::new(1).expect("nonzero")),
            seq: 1,
            loaded: true,
            frames: frames(0),
            sections: every_payload(),
        }
    }

    /// A folded section is still a heading (so it can be unfolded) but
    /// carries no rows; an open one carries its rows; a fresh section
    /// carries no age in its title. Every registered id is present in
    /// registry order, answered or not.
    #[test]
    fn a_folded_section_keeps_its_heading_and_loses_its_rows() {
        use common::comp::bastion_inspect::SectionSetV1;
        let r = full_reply();
        let folded = SectionSetV1::all().toggled(SectionIdV1::Identity);
        let p = panel::build(&r, |_| Some(0), folded, false);
        assert_eq!(
            p.sections.iter().map(|s| s.id).collect::<Vec<_>>(),
            SectionIdV1::ALL.to_vec(),
            "every registered section is a heading, folded or not, in registry order"
        );
        let id = p.sections.iter().find(|s| s.id == SectionIdV1::Identity).unwrap();
        assert!(!id.expanded && id.rows.is_empty(), "folded: heading only, got {:?}", id.rows);
        let rn = p.sections.iter().find(|s| s.id == SectionIdV1::RightNow).unwrap();
        assert!(rn.expanded && !rn.rows.is_empty(), "open: heading and rows");
        assert_eq!(id.title, SectionIdV1::Identity.title(), "a fresh section carries no age");
        // The false direction: unfolding it brings the rows back.
        let p2 = panel::build(&r, |_| Some(0), SectionSetV1::all(), false);
        let id2 = p2.sections.iter().find(|s| s.id == SectionIdV1::Identity).unwrap();
        assert!(id2.expanded && !id2.rows.is_empty(), "unfolded: rows return");
    }

    /// The panel and the flat `to_lines` view are two renderings of ONE
    /// reply: same rows, same text, same order within a section, the same
    /// rows marked alarm, and a carried-forward section's age in its title.
    #[test]
    fn the_panel_and_the_flat_view_agree_on_every_row_alarm_and_age() {
        use common::comp::bastion_inspect::SectionSetV1;
        let r = full_reply();
        let age = |id: SectionIdV1| if id == SectionIdV1::RightNow { Some(30) } else { Some(0) };
        let p = panel::build(&r, age, SectionSetV1::all(), true);
        let rendered = render(&r, age);
        let mut compared = 0;
        for s in &p.sections {
            let rs = rendered
                .iter()
                .find(|x| x.id == s.id)
                .expect("the fixture answers every registered section");
            let flat = row_lines(&rs.rows, true);
            let panel_rows: Vec<&str> = s.rows.iter().map(|x| x.text.as_str()).collect();
            assert_eq!(panel_rows, flat.iter().map(String::as_str).collect::<Vec<_>>(), "{:?}", s.id);
            let flat_alarms: Vec<bool> = flat.iter().map(|l| l.starts_with(ALARM_PREFIX)).collect();
            let panel_alarms: Vec<bool> = s.rows.iter().map(|x| x.alarm).collect();
            assert_eq!(panel_alarms, flat_alarms, "alarm flags for {:?}", s.id);
            compared += s.rows.len();
        }
        assert!(compared > 0, "the fixture must produce rows to compare");
        let rn = p.sections.iter().find(|s| s.id == SectionIdV1::RightNow).unwrap();
        assert!(rn.title.contains("as of 30 server ticks ago"), "{}", rn.title);
        let fresh = p.sections.iter().find(|s| s.id == SectionIdV1::Path).unwrap();
        assert!(!fresh.title.contains("as of"), "{}", fresh.title);
    }
}
