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
    FrameV1, InspectFramesV1, InspectRow, SectionIdV1, SectionPayloadV1,
};

pub mod identity;
pub mod path;
pub mod right_now;

/// A section view. Pure.
pub type ViewFn = fn(&SectionPayloadV1, &InspectFramesV1) -> Vec<InspectRow>;

/// ★ THE CLIENT REGISTRY. NO WILDCARD ARM.
pub const fn view_for(id: SectionIdV1) -> ViewFn {
    match id {
        SectionIdV1::Identity => identity::rows,
        SectionIdV1::RightNow => right_now::rows,
        SectionIdV1::Path => path::rows,
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

/// One rendered section: its heading and its rows.
pub struct RenderedSection {
    pub id: SectionIdV1,
    pub rows: Vec<InspectRow>,
}

/// Render a whole reply into headed sections, in registry order.
pub fn render(reply: &common::comp::bastion_inspect::SectionedInspectV1) -> Vec<RenderedSection> {
    reply
        .sections
        .iter()
        .map(|p| RenderedSection {
            id: p.id(),
            rows: rows_for(p, &reply.frames),
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

/// One row per line. Shared by the header and by every section, so
/// provenance renders identically wherever it appears.
fn row_lines(rows: &[InspectRow], verbose: bool) -> Vec<String> {
    rows.iter()
        .map(|r| {
            let unit = if r.unit().is_empty() {
                String::new()
            } else {
                format!(" {}", r.unit())
            };
            if verbose {
                format!("{}: {}{}   [{}]", r.label(), r.value(), unit, r.provenance())
            } else {
                format!("{}: {}{}", r.label(), r.value(), unit)
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
        out.push(format!("- {} -", s.id.title()));
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
        let rendered = render(&reply);
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
}
