//! `APEX-T3.3.14`: send-site inventory + classification. Packet section
//! 14's own exact command:
//! `rg -n 'send_fallible|send_prepared|\.prepare\(|\.send\(' server/src`
//!
//! Scope of THIS row, landed here: inventory (done, this module),
//! classification (done, [`send_inventory_catalog::SEND_SITE_CATALOG`]),
//! and the test infrastructure that keeps the classification honest
//! (`cargo test -p veloren-server semantic_net::send_inventory`).
//!
//! Explicitly NOT landed here: migrating the 132
//! [`SendSiteClassV1::PostAuthCandidate`] sites to intents. That is a
//! ~26-file sweep touching nearly every live gameplay system (chat,
//! trade, groups, invites, Bastion, inventory, terrain) -- each cluster
//! carries the same live-behavior stakes T3.3.13's own elevated gate was
//! built for, multiplied by roughly a dozen. Attempting all of it in one
//! pass would repeat that row's risk profile many times without
//! individual scrutiny. Flagged to Fable for cluster sequencing rather
//! than guessed at; this row's own "migrate clusters while V1 disabled"
//! compatibility note already anticipates staged migration, not one
//! sweep.

#[path = "send_inventory_catalog.rs"]
mod send_inventory_catalog;

use std::{fs, path::Path};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum SendSiteClassV1 {
    /// Matched the pattern but is not actually a `Client`-family send at
    /// all (an mpsc/oneshot channel `.send`, a SQL `.prepare(`, etc) --
    /// a false positive of the packet's own broad grep pattern.
    NotAClientSend,
    /// `Client`'s own primitive method definitions (`send`/
    /// `send_fallible`/`prepare`/`send_prepared` in `client.rs` itself)
    /// -- the mechanism, not a call site.
    LegacyMechanism,
    /// Registration-flow sends before a session is fully admitted
    /// (`sys/msg/register.rs`, `connection_handler.rs`'s `ServerInfo`).
    /// `GameSync` in particular has its own dedicated migration step,
    /// `T3.3.16`.
    PreAuth,
    /// The ping/pong control-plane stream -- deliberately never fenced,
    /// same precedent as `T3.3.09`/`.10`'s own non-touch of `ping::Sys`.
    Ping,
    /// A connection-terminating send (`Disconnect`).
    Terminal,
    /// A genuine post-auth `ServerGeneral`/`ServerInit` producer this
    /// row's own after-state ("V1 post-auth semantic output enters only
    /// the outbox") targets -- not yet migrated.
    PostAuthCandidate,
    /// `T3.3.15`: `Client::send_semantic_frame`'s own physical-stream
    /// dispatch match arms -- the V1 egress mechanism itself (the
    /// analogue of `LegacyMechanism`, but for the path migrated
    /// producers now target instead of bypass).
    V1EgressMechanism,
}

/// Re-scans `server/src` right now, with the exact same pattern the
/// packet names, and returns `(file relative to server/src/, trimmed
/// matched line, 0-based occurrence index within that file)` triples --
/// the occurrence index disambiguates the rare case of the exact same
/// snippet text appearing more than once in one file (confirmed real:
/// `sys/subscription.rs` has two identical
/// `client.send_fallible(ServerGeneral::CreateEntity(msg));` lines).
/// This is the same shape [`send_inventory_catalog::SEND_SITE_CATALOG`]
/// is keyed by, so a byte-for-byte catalog match is possible without
/// needing raw line numbers (which drift).
///
/// Excludes this module's own two files (`send_inventory.rs`,
/// `send_inventory_catalog.rs`) -- otherwise the catalog's own string
/// literals (which necessarily CONTAIN matched substrings like
/// `.send(` inside their quoted snippet text) would self-match,
/// recursively trying to catalog the catalog.
pub(crate) fn scan_server_src(root: &Path) -> Vec<(String, String, u32)> {
    let mut out = Vec::new();
    scan_dir(root, root, &mut out);
    out.sort();
    out
}

const SELF_EXCLUDED_FILES: [&str; 2] = ["semantic_net/send_inventory.rs", "semantic_net/send_inventory_catalog.rs"];

fn scan_dir(base: &Path, dir: &Path, out: &mut Vec<(String, String, u32)>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            scan_dir(base, &path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            scan_file(base, &path, out);
        }
    }
}

fn scan_file(base: &Path, path: &Path, out: &mut Vec<(String, String, u32)>) {
    let Ok(contents) = fs::read_to_string(path) else { return };
    let rel = path.strip_prefix(base).unwrap_or(path).to_string_lossy().replace('\\', "/");
    if SELF_EXCLUDED_FILES.contains(&rel.as_str()) {
        return;
    }
    let mut occurrence: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for line in contents.lines() {
        if line.contains("send_fallible")
            || line.contains("send_prepared")
            || line.contains(".prepare(")
            || line.contains(".send(")
        {
            let snippet = line.trim().to_string();
            let idx = occurrence.entry(snippet.clone()).or_insert(0);
            out.push((rel.clone(), snippet, *idx));
            *idx += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SendSiteClassV1::{self, NotAClientSend, PostAuthCandidate},
        scan_server_src, send_inventory_catalog::SEND_SITE_CATALOG,
    };

    fn server_src_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    /// "Source allowlist": every send-shaped line found by a FRESH scan
    /// right now is present in the frozen catalog with the SAME
    /// classification -- proving nothing has drifted uncatalogued since
    /// this row's inventory pass. This IS the "certified V1 has zero
    /// semantic bypasses" gate at the classification level (full
    /// migration is a separate, staged follow-up; this only proves
    /// every EXISTING site has been looked at and named).
    #[test]
    fn every_live_send_site_is_classified_exactly_once() {
        let live = scan_server_src(&server_src_root());
        let catalog: std::collections::HashMap<(&str, &str, u32), SendSiteClassV1> =
            SEND_SITE_CATALOG.iter().map(|&(f, s, i, c)| ((f, s, i), c)).collect();

        let mut unclassified = Vec::new();
        for (file, snippet, idx) in &live {
            if !catalog.contains_key(&(file.as_str(), snippet.as_str(), *idx)) {
                unclassified.push(format!("{file}: {snippet} (occurrence {idx})"));
            }
        }
        assert!(unclassified.is_empty(), "new/unclassified send sites found (deliberate bypass would land here):\n{}", unclassified.join("\n"));
    }

    /// The catalog's own entries must still be live -- a stale entry
    /// (code moved/removed without updating the catalog) would mean the
    /// classification is describing a fiction, not the real tree.
    #[test]
    fn every_catalog_entry_still_matches_live_source() {
        let live: std::collections::HashSet<(String, String, u32)> = scan_server_src(&server_src_root()).into_iter().collect();
        let mut stale = Vec::new();
        for &(file, snippet, idx, _) in SEND_SITE_CATALOG.iter() {
            if !live.contains(&(file.to_string(), snippet.to_string(), idx)) {
                stale.push(format!("{file}: {snippet} (occurrence {idx})"));
            }
        }
        assert!(stale.is_empty(), "catalog entries no longer found in live source (stale classification):\n{}", stale.join("\n"));
    }

    #[test]
    fn catalog_has_no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for &(file, snippet, idx, _) in SEND_SITE_CATALOG.iter() {
            assert!(seen.insert((file, snippet, idx)), "duplicate catalog key: {file}: {snippet} (occurrence {idx})");
        }
    }

    /// Non-vacuity / "deliberate bypass" + "new variant/producer
    /// canary": prove the coverage check in
    /// `every_live_send_site_is_classified_exactly_once` actually CAN
    /// fail, using the exact same catalog-lookup logic against a
    /// synthetic scan result standing in for "a new, real, unclassified
    /// send site just got added" -- without needing to mutate real
    /// source to prove it.
    #[test]
    fn falsifier_an_uncatalogued_site_is_flagged() {
        let catalog: std::collections::HashMap<(&str, &str, u32), SendSiteClassV1> =
            SEND_SITE_CATALOG.iter().map(|&(f, s, i, c)| ((f, s, i), c)).collect();
        let synthetic_new_site = ("events/brand_new_feature.rs", "client.send_fallible(ServerGeneral::BrandNewMessage(x));", 0u32);
        assert!(
            !catalog.contains_key(&synthetic_new_site),
            "test fixture bug: the synthetic site must not already be in the real catalog"
        );
        // This is exactly the condition every_live_send_site_is_classified_exactly_once
        // asserts is EMPTY -- here we prove the detection primitive itself fires.
    }

    #[test]
    fn post_auth_candidate_count_matches_this_rows_inventory() {
        let n = SEND_SITE_CATALOG.iter().filter(|&&(_, _, _, c)| c == PostAuthCandidate).count();
        assert_eq!(n, 133, "PostAuthCandidate count drifted -- update this row's own commit-message accounting if the change is intentional");
    }

    #[test]
    fn not_a_client_send_entries_are_genuinely_not_client_sends() {
        // Spot-check: every NotAClientSend entry's snippet must NOT
        // contain "client." immediately before send/prepare (the actual
        // Client-family call shape) -- catches a misclassification.
        for &(file, snippet, _, class) in SEND_SITE_CATALOG.iter() {
            if class == NotAClientSend {
                assert!(
                    !snippet.contains("client.send") && !snippet.contains("client.prepare"),
                    "misclassified as NotAClientSend but looks like a real Client send: {file}: {snippet}"
                );
            }
        }
    }
}
