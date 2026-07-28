//! `APEX-T6.1a` — numeric attack-surface inventory: the SCAN and the
//! classification.
//!
//! T6's premise is that determinism stops being a property of our code
//! and becomes a property of the machine the moment a transcendental
//! result drives a branch. Before anything can be certified or replaced,
//! the surface has to be known and prevented from growing silently.
//!
//! This row delivers the tripwire half, in the shape `T3.5.19`'s bypass
//! scanner proved: every file in the authoritative simulation crates
//! that performs a root, power or trigonometric operation is classified,
//! and an unclassified one fails the build.
//!
//! **What this row does NOT deliver**, stated so the coverage map can
//! say it too: per-SITE owner and protocol status for the branch-driving
//! operations (`T6.1b`). The class list below is the surface; assigning
//! an owner to each branch-driving call is the follow-on, and the row's
//! acceptance criterion is not met until it exists.
//!
//! Granularity is per FILE, for the reason the disconnect inventory
//! gives: line positions drift with every unrelated edit and rot into
//! noise, while a file's ROLE is stable.

use std::{fs, path::Path};

/// What a numeric-surface file is, for determinism purposes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum NumericRoleV1 {
    /// Authoritative simulation: its results reach state the server owns
    /// or that crosses a network, save or hash boundary.
    Authoritative,
    /// Presentation, tooling or diagnostics. Excluded WITH EVIDENCE —
    /// the `T5.4` finding (a "presentational" wind reaching glider
    /// steering) is why an assertion alone is not enough.
    PresentationOrTooling,
    /// Test/fixture support, not a live path.
    TestSupport,
}

/// Every file in the authoritative crates touching a root, power or
/// trigonometric operation, with what it is and why.
pub(crate) const NUMERIC_SURFACE_ROLES: &[(&str, NumericRoleV1, &str)] = &[
    ("common/src/apex/source_closure.rs", NumericRoleV1::TestSupport, "build-provenance hashing, no simulation arithmetic"),
    ("common/src/clock.rs", NumericRoleV1::PresentationOrTooling, "frame pacing and stats; sim time comes from State::tick's clamp"),
    ("common/src/combat.rs", NumericRoleV1::Authoritative, "damage/knockback scaling reaches health and physics"),
    ("common/src/comp/ability.rs", NumericRoleV1::Authoritative, "ability scaling feeds combat"),
    ("common/src/comp/buff.rs", NumericRoleV1::Authoritative, "buff strength curve (powf) feeds combat and movement"),
    ("common/src/comp/fluid_dynamics.rs", NumericRoleV1::Authoritative, "drag/lift powf drives glider and projectile motion"),
    ("common/src/comp/ori.rs", NumericRoleV1::Authoritative, "orientation normalisation is synced state"),
    ("common/src/comp/projectile.rs", NumericRoleV1::Authoritative, "projectile kinematics"),
    ("common/src/comp/skillset/mod.rs", NumericRoleV1::Authoritative, "skill-point curve is persisted state"),
    ("common/src/path.rs", NumericRoleV1::Authoritative, "pathfinding heuristics decide NPC movement"),
    ("common/src/region.rs", NumericRoleV1::Authoritative, "region membership decides what is synced to whom"),
    ("common/src/resources.rs", NumericRoleV1::Authoritative, "time/scale resources feed every tick"),
    ("common/src/states/basic_aura.rs", NumericRoleV1::Authoritative, "aura radius decides who is affected"),
    ("common/src/states/basic_summon.rs", NumericRoleV1::Authoritative, "summon placement is authoritative spawn position"),
    ("common/src/states/dash_melee.rs", NumericRoleV1::Authoritative, "dash kinematics"),
    ("common/src/states/glide_wield.rs", NumericRoleV1::Authoritative, "glider orientation feeds flight"),
    ("common/src/states/rapid_ranged.rs", NumericRoleV1::Authoritative, "projectile launch parameters"),
    ("common/src/states/utils.rs", NumericRoleV1::Authoritative, "movement scaling powf reaches position"),
    ("common/src/terrain/map.rs", NumericRoleV1::PresentationOrTooling, "map image sampling for the client map view; worldgen owns the authoritative geometry"),
    ("common/src/time.rs", NumericRoleV1::Authoritative, "calendar/day-cycle arithmetic is synced"),
    ("common/src/util/color.rs", NumericRoleV1::PresentationOrTooling, "colour space conversion, rendering only"),
    ("common/src/util/dir.rs", NumericRoleV1::Authoritative, "Dir normalisation is used by orientation and aiming"),
    ("common/src/util/find_dist.rs", NumericRoleV1::Authoritative, "distance predicates gate interactions"),
    ("common/systems/src/phys/collision.rs", NumericRoleV1::Authoritative, "collision resolution"),
    ("common/systems/src/phys/mod.rs", NumericRoleV1::Authoritative, "the physics tick itself; T6.3's ordering row lives here"),
    ("common/systems/src/phys/weather.rs", NumericRoleV1::Authoritative, "wind forces reach flight; see T5.4 on the presentation/authority split"),
    ("common/systems/src/projectile.rs", NumericRoleV1::Authoritative, "projectile system"),
    ("common/systems/src/shockwave.rs", NumericRoleV1::Authoritative, "shockwave geometry decides who is hit"),
];

/// The operations that make a file part of the surface.
pub(crate) const NUMERIC_SURFACE_PATTERNS: [&str; 6] =
    ["powf", "sqrt()", ".sin()", ".cos()", ".ln()", "hypot"];

/// Branch-driving `powf` call sites, seeded from the T6 tier spec's own
/// reads. This is the START of `T6.1b`'s owned inventory, not its
/// completion — see the module doc.
pub(crate) const BRANCH_DRIVING_SEED: &[(&str, &str)] = &[
    ("common/src/comp/fluid_dynamics.rs", "drag coefficient: ar.powf(0.68)"),
    ("common/src/comp/fluid_dynamics.rs", "scale.powf(2.0) in the force sum"),
    ("common/src/comp/fluid_dynamics.rs", "(PI/6 * dim).powf(2.0/3.0)"),
    ("common/src/states/utils.rs", "scale.powf(13.0).powf(0.25) movement scaling"),
    ("common/src/comp/buff.rs", "f32::powf(1.0 - nn_scaling(strength), 1.1)"),
];

pub(crate) fn scan_numeric_surface_v1(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(&root.join("common/src"), &mut files);
    walk(&root.join("common/systems/src"), &mut files);

    let mut hits: Vec<String> = files
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .is_ok_and(|text| NUMERIC_SURFACE_PATTERNS.iter().any(|p| text.contains(p)))
        })
        .filter_map(|path| {
            let rel = path.strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/");
            // This inventory NAMES the operations; it does not perform
            // them. Same quoter-not-doer rule the disconnect scanner uses.
            (!rel.ends_with("numeric_surface.rs")).then_some(rel)
        })
        .collect();
    hits.sort();
    hits
}

#[cfg(test)]
mod numeric_surface_v1 {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        // CARGO_MANIFEST_DIR is <root>/common.
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("common has a parent").to_path_buf()
    }

    /// `T6.1`: the surface is fully classified, and a new numeric site
    /// fails the build rather than appearing quietly.
    #[test]
    fn every_numeric_surface_file_is_classified() {
        let scanned = scan_numeric_surface_v1(&repo_root());
        assert!(!scanned.is_empty(), "the scan found nothing — it is broken, not the tree");

        let claimed: std::collections::BTreeSet<&str> =
            NUMERIC_SURFACE_ROLES.iter().map(|(f, _, _)| *f).collect();
        let found: std::collections::BTreeSet<&str> = scanned.iter().map(String::as_str).collect();

        let unclaimed: Vec<&&str> = found.difference(&claimed).collect();
        assert!(
            unclaimed.is_empty(),
            "unclassified numeric-surface files (say what they are, with evidence for any \
             presentation-only exclusion):\n{unclaimed:#?}"
        );
        let vanished: Vec<&&str> = claimed.difference(&found).collect();
        assert!(vanished.is_empty(), "these files no longer touch the surface; drop them:\n{vanished:#?}");
    }

    /// Every exclusion carries evidence. `T5.4` is why: a value that
    /// looked presentational reached glider steering, so "it's only for
    /// display" is a claim that has to be argued, not asserted.
    #[test]
    fn presentation_exclusions_carry_evidence() {
        for (file, role, why) in NUMERIC_SURFACE_ROLES {
            assert!(!why.trim().is_empty(), "{file} has no stated reason");
            if *role == NumericRoleV1::PresentationOrTooling {
                assert!(
                    why.len() > 20,
                    "{file} is excluded from authority on a one-word claim: {why:?}"
                );
            }
        }
    }

    /// The authoritative set is the majority of the surface, and the
    /// physics tick is in it — T6.3's ordering row depends on that being
    /// true.
    #[test]
    fn the_authoritative_set_is_pinned() {
        let authoritative = NUMERIC_SURFACE_ROLES
            .iter()
            .filter(|(_, role, _)| *role == NumericRoleV1::Authoritative)
            .count();
        assert_eq!(authoritative, 24, "the authoritative surface changed — re-derive T6.1b's owners");
        assert!(
            NUMERIC_SURFACE_ROLES.iter().any(|(f, role, _)| *f == "common/systems/src/phys/mod.rs"
                && *role == NumericRoleV1::Authoritative),
            "the physics tick must be authoritative or T6.3 is aimed at nothing"
        );
    }

    /// `T6.1b`'s seed is real: every branch-driving file named is one the
    /// scan actually classifies Authoritative.
    #[test]
    fn the_branch_driving_seed_sits_inside_the_authoritative_set() {
        for (file, what) in BRANCH_DRIVING_SEED {
            assert!(!what.trim().is_empty(), "{file} seed entry says nothing");
            let role = NUMERIC_SURFACE_ROLES
                .iter()
                .find(|(f, _, _)| f == file)
                .map(|(_, role, _)| *role)
                .unwrap_or_else(|| panic!("{file} is seeded but not classified"));
            assert_eq!(role, NumericRoleV1::Authoritative, "{file} is seeded but not authoritative");
        }
    }
}
