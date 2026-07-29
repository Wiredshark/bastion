//! `APEX-T3.5.19` — the bypass scanner. Under Enforce, a mutation that
//! reaches authoritative state WITHOUT passing the command journal is a
//! hole in exactly-once, and the catalog names three surfaces a naive
//! scan misses: character-updater actions (`CMD-156`), direct
//! `BlockChange` mutations (`CMD-157`), and `CommandEvent` (`CMD-158`).
//!
//! The scan runs at test time over `server/src` and classifies every
//! FILE that touches those surfaces. A file with no entry fails the
//! build ("unclaimed-name-fails", the `T3.3.20` standard), so a new
//! mutation site cannot appear quietly — someone has to say what it is.
//!
//! Counting files rather than lines is deliberate: line positions drift
//! with every unrelated edit, and a per-line catalog rots into noise. A
//! file's ROLE is stable, and it is the honest unit for "does this
//! surface bypass the journal".

use std::{fs, path::Path};

/// What a file that touches a bypass surface actually is.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum BypassRoleV1 {
    /// The mechanism itself (the updater, the block-change resource, the
    /// event plumbing) — not a call site.
    Mechanism,
    /// A mutation that a journaled command will own under Enforce. These
    /// are the sites the rollout has to move.
    CommandOwned,
    /// A mutation with no client command behind it: worldgen, rtsim, an
    /// admin console path, or the arena harness.
    ServerAuthored,
    /// Catalog/inventory bookkeeping that merely NAMES these surfaces.
    CatalogOnly,
}

/// Every `server/src` file touching a bypass surface, and what it is.
/// Sorted by path.
pub(crate) const BYPASS_SURFACE_ROLES: &[(&str, BypassRoleV1)] = &[
    ("server/src/bastion_arena.rs", BypassRoleV1::ServerAuthored),
    ("server/src/character_creator.rs", BypassRoleV1::CommandOwned),
    ("server/src/cmd.rs", BypassRoleV1::CommandOwned),
    ("server/src/events/entity_creation.rs", BypassRoleV1::ServerAuthored),
    ("server/src/events/entity_manipulation.rs", BypassRoleV1::ServerAuthored),
    ("server/src/events/event_types.rs", BypassRoleV1::Mechanism),
    ("server/src/events/interaction.rs", BypassRoleV1::CommandOwned),
    ("server/src/events/inventory_manip.rs", BypassRoleV1::CommandOwned),
    ("server/src/events/mod.rs", BypassRoleV1::Mechanism),
    ("server/src/events/player.rs", BypassRoleV1::CommandOwned),
    ("server/src/lib.rs", BypassRoleV1::Mechanism),
    ("server/src/persistence/character/mod.rs", BypassRoleV1::Mechanism),
    ("server/src/persistence/character_loader.rs", BypassRoleV1::Mechanism),
    ("server/src/persistence/character_updater.rs", BypassRoleV1::Mechanism),
    ("server/src/net_command_bypass.rs", BypassRoleV1::CatalogOnly),
    ("server/src/persistence/mod.rs", BypassRoleV1::Mechanism),
    ("server/src/rtsim/event.rs", BypassRoleV1::ServerAuthored),
    ("server/src/rtsim/mod.rs", BypassRoleV1::ServerAuthored),
    ("server/src/rtsim/rule/deplete_resources.rs", BypassRoleV1::ServerAuthored),
    // `APEX-T4.6` chunk 2: names `CharacterUpdater` once, in its module
    // doc comment (describing the failure surface T4.6 fixes) -- never
    // touches it as a mutation site.
    ("server/src/save_universe.rs", BypassRoleV1::CatalogOnly),
    ("server/src/semantic_net/receive_inventory_catalog.rs", BypassRoleV1::CatalogOnly),
    ("server/src/semantic_net/send_inventory_catalog.rs", BypassRoleV1::CatalogOnly),
    ("server/src/sys/msg/character_screen.rs", BypassRoleV1::CommandOwned),
    ("server/src/sys/msg/general.rs", BypassRoleV1::CommandOwned),
    ("server/src/sys/msg/in_game.rs", BypassRoleV1::CommandOwned),
    ("server/src/sys/persistence.rs", BypassRoleV1::Mechanism),
    ("server/src/sys/wiring.rs", BypassRoleV1::ServerAuthored),
    ("server/src/wiring.rs", BypassRoleV1::ServerAuthored),
];

/// The three surfaces the catalog names, as the substrings a scan looks
/// for. Kept together so adding a surface is one edit.
pub(crate) const BYPASS_SURFACE_PATTERNS: [&str; 3] =
    ["character_updater", "block_change", "CommandEvent"];

/// Re-scans `server/src` now and returns the files that touch any bypass
/// surface, repo-relative and forward-slashed, sorted.
pub(crate) fn scan_bypass_surfaces_v1(root: &Path) -> Vec<String> {
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
    walk(&root.join("server/src"), &mut files);
    let mut hits: Vec<String> = files
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path).is_ok_and(|text| {
                BYPASS_SURFACE_PATTERNS.iter().any(|pattern| {
                    // `CharacterUpdater`/`BlockChange` are the type names
                    // of the same two surfaces; match either spelling.
                    text.contains(pattern)
                        || (*pattern == "character_updater" && text.contains("CharacterUpdater"))
                        || (*pattern == "block_change" && text.contains("BlockChange"))
                })
            })
        })
        .filter_map(|path| {
            let rel = path.strip_prefix(root).ok()?;
            Some(rel.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    hits.sort();
    hits
}

#[cfg(test)]
mod command_bypass_scan_v1 {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        // `CARGO_MANIFEST_DIR` is `<root>/server`.
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("server has a parent").to_path_buf()
    }

    /// `CMD-156`/`CMD-157`/`CMD-158`: the scan sees every file touching a
    /// bypass surface, and every one of them is classified. A new
    /// mutation site fails the build rather than slipping in unnamed.
    #[test]
    fn every_bypass_surface_file_is_classified() {
        let scanned = scan_bypass_surfaces_v1(&repo_root());
        assert!(!scanned.is_empty(), "the scan found nothing — it is broken, not the tree");

        let claimed: std::collections::BTreeSet<&str> =
            BYPASS_SURFACE_ROLES.iter().map(|(path, _)| *path).collect();
        let found: std::collections::BTreeSet<&str> = scanned.iter().map(String::as_str).collect();

        let unclaimed: Vec<&&str> = found.difference(&claimed).collect();
        assert!(
            unclaimed.is_empty(),
            "unclassified bypass-surface files (say what they are in BYPASS_SURFACE_ROLES):\n{unclaimed:#?}"
        );

        let vanished: Vec<&&str> = claimed.difference(&found).collect();
        assert!(
            vanished.is_empty(),
            "these files no longer touch a bypass surface; drop their entries:\n{vanished:#?}"
        );
    }

    /// The three surfaces the catalog names are all actually searched
    /// for — a scan that quietly dropped one would still pass the test
    /// above.
    #[test]
    fn all_three_named_surfaces_are_scanned() {
        assert_eq!(BYPASS_SURFACE_PATTERNS.len(), 3);
        for surface in ["character_updater", "block_change", "CommandEvent"] {
            assert!(BYPASS_SURFACE_PATTERNS.contains(&surface), "{surface} is not scanned for");
        }
    }

    /// The rollout's real question: which files must move under Enforce.
    /// Naming them here means the number cannot drift silently.
    #[test]
    fn the_command_owned_set_is_pinned() {
        let owned: Vec<&str> = BYPASS_SURFACE_ROLES
            .iter()
            .filter(|(_, role)| *role == BypassRoleV1::CommandOwned)
            .map(|(path, _)| *path)
            .collect();
        assert_eq!(
            owned,
            vec![
                "server/src/character_creator.rs",
                "server/src/cmd.rs",
                "server/src/events/interaction.rs",
                "server/src/events/inventory_manip.rs",
                "server/src/events/player.rs",
                "server/src/sys/msg/character_screen.rs",
                "server/src/sys/msg/general.rs",
                "server/src/sys/msg/in_game.rs",
            ],
            "the set of files Enforce has to move changed — update this pin deliberately"
        );
    }
}
