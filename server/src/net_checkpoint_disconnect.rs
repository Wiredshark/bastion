//! `APEX-T3.6.03` (`CKPT-174`) — the legacy `ServerGeneral::Disconnect`
//! source inventory.
//!
//! The canary's terminal is `BLOCK-LEGACY-DISCONNECT-SOURCE`: under the
//! checkpoint regime a session ends through the fenced control lane
//! (`SessionTerminateV1`, `T3.6.01`), not through a data-stream message.
//! The checkpoint path is still dormant, so **migrating these sites now
//! would change live disconnect behaviour on a premise that is not yet
//! active** — the wrong trade. What this row does instead is make the
//! migration a bounded, specified job and stop the surface growing:
//!
//! - every live send site is enumerated and classified,
//! - each is mapped to the `SessionTerminationReasonV1` it becomes,
//! - the set is pinned, so a NEW legacy disconnect site fails the build
//!   and has to be argued for rather than appearing quietly.
//!
//! That is the same shape as `T3.5.19`'s bypass scanner, and it is the
//! honest closure available while the target path is dormant: the case
//! is covered by a tripwire plus a written migration, not by a claim
//! that the legacy source is gone.

use common_net::msg::session_control::SessionTerminationReasonV1;

/// What a legacy disconnect site is, and what it becomes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct LegacyDisconnectSiteV1 {
    /// Repo-relative file, forward-slashed.
    pub(crate) file: &'static str,
    /// What the site does, in one phrase.
    pub(crate) role: &'static str,
    /// The control-lane reason this site maps to under `T3.6.01`.
    pub(crate) becomes: SessionTerminationReasonV1,
}

/// Every live `ServerGeneral::Disconnect` SEND site. Match arms and
/// catalog mentions are not sends and are excluded deliberately —
/// `server/src/client.rs`'s two occurrences are stream-routing arms, and
/// the `semantic_net` catalogs merely quote the sites below.
pub(crate) const LEGACY_DISCONNECT_SITES: &[LegacyDisconnectSiteV1] = &[
    LegacyDisconnectSiteV1 {
        file: "server/src/cmd.rs",
        role: "admin kick command",
        becomes: SessionTerminationReasonV1::Kicked,
    },
    LegacyDisconnectSiteV1 {
        file: "server/src/lib.rs",
        role: "server shutdown notice to all players",
        becomes: SessionTerminationReasonV1::ServerShutdown,
    },
    LegacyDisconnectSiteV1 {
        file: "server/src/sys/msg/network_events.rs",
        role: "ban enforcement on connect",
        becomes: SessionTerminationReasonV1::Banned,
    },
    LegacyDisconnectSiteV1 {
        file: "server/src/sys/msg/register.rs",
        role: "duplicate login displaces the older session",
        becomes: SessionTerminationReasonV1::Kicked,
    },
];

#[cfg(test)]
mod legacy_disconnect_inventory_v1 {
    use super::*;
    use std::{fs, path::Path};

    fn repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("server has a parent").to_path_buf()
    }

    /// Re-scans for real SEND sites now: `ServerGeneral::Disconnect(`
    /// preceded by a send call on the same line or the line above.
    fn scan_send_sites(root: &Path) -> std::collections::BTreeSet<String> {
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

        let mut hits = std::collections::BTreeSet::new();
        for path in files {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            let rel = match path.strip_prefix(root) {
                Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            // Files that QUOTE send sites rather than perform them: this
            // inventory, and the T3.3 send/receive catalogs which mirror
            // source text by design. Same Mechanism-vs-CatalogOnly
            // distinction T3.5.19's scanner draws.
            if rel.ends_with("net_checkpoint_disconnect.rs") || rel.ends_with("_catalog.rs") {
                continue;
            }
            let lines: Vec<&str> = text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if !line.contains("ServerGeneral::Disconnect(") {
                    continue;
                }
                let previous = if i > 0 { lines[i - 1] } else { "" };
                let context = format!("{previous}{line}");
                let is_send = context.contains(".send(")
                    || context.contains("send_fallible(")
                    || context.contains("notify_client(")
                    || context.contains("notify_players(");
                if is_send {
                    hits.insert(rel.clone());
                }
            }
        }
        hits
    }

    /// `CKPT-174`: the legacy source is enumerated, mapped, and pinned. A
    /// new site fails here rather than appearing quietly.
    #[test]
    fn every_legacy_disconnect_send_site_is_inventoried() {
        let scanned = scan_send_sites(&repo_root());
        assert!(!scanned.is_empty(), "the scan found nothing — it is broken, not the tree");

        let claimed: std::collections::BTreeSet<&str> =
            LEGACY_DISCONNECT_SITES.iter().map(|s| s.file).collect();
        let found: std::collections::BTreeSet<&str> = scanned.iter().map(String::as_str).collect();

        let unclaimed: Vec<&&str> = found.difference(&claimed).collect();
        assert!(
            unclaimed.is_empty(),
            "new legacy disconnect send sites (map them to a SessionTerminationReasonV1 or do not add \
             them):\n{unclaimed:#?}"
        );
        let vanished: Vec<&&str> = claimed.difference(&found).collect();
        assert!(vanished.is_empty(), "these sites are gone; drop their entries:\n{vanished:#?}");
    }

    /// Every site's migration target is stated, and the reasons actually
    /// used are the ones the control lane defines.
    #[test]
    fn every_site_maps_to_a_control_lane_reason() {
        for site in LEGACY_DISCONNECT_SITES {
            assert!(!site.role.is_empty(), "{} has no stated role", site.file);
            assert!(
                SessionTerminationReasonV1::ALL.contains(&site.becomes),
                "{} maps to a reason the control lane does not define",
                site.file
            );
        }
        // The shutdown path and the moderation paths are distinct
        // reasons; collapsing them would lose the distinction the control
        // lane exists to carry.
        let shutdown = LEGACY_DISCONNECT_SITES
            .iter()
            .filter(|s| s.becomes == SessionTerminationReasonV1::ServerShutdown)
            .count();
        let moderation = LEGACY_DISCONNECT_SITES
            .iter()
            .filter(|s| {
                matches!(
                    s.becomes,
                    SessionTerminationReasonV1::Kicked | SessionTerminationReasonV1::Banned
                )
            })
            .count();
        assert_eq!(shutdown, 1, "exactly one shutdown-notice site is expected");
        assert_eq!(moderation, 3, "three moderation-driven sites are expected");
    }
}
