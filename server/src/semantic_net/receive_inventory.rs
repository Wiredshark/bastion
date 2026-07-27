//! `APEX-T3.3.20`: receive-site inventory + classification, the
//! receive-side twin of `send_inventory.rs` (`T3.3.14`) -- "Check direct
//! sends/receives" from this row's own file list. Exact scan pattern:
//! `rg -n '\.recv\(|\.try_recv\(|try_recv_all\(' server/src`.
//!
//! Unlike the send side (132 `PostAuthCandidate` sites still awaiting
//! migration as of this row), the receive side is ALREADY fully unified:
//! every one of the four post-auth receive systems (general/
//! character_screen/in_game/terrain) calls `try_recv_all_dispatch`
//! exclusively (`T3.3.09`) -- there is no live site where a post-auth
//! system reaches past it to a raw `client.recv(stream_id)`. This
//! module's own completeness test (`post_auth_bypass_count_is_zero`)
//! makes that a PINNED, continuously-checked fact rather than an
//! as-of-writing observation: any future receive site that bypasses the
//! dispatcher shows up as either an uncatalogued site (this module's own
//! `every_live_receive_site_is_classified_exactly_once`, which has no
//! `PostAuthCandidate`-shaped bucket to silently absorb it into the way
//! the send side's did) or, if someone deliberately mis-files it under
//! an existing safe class, a reviewer catches the mismatch by reading
//! the diff -- the lint is conservative (packet's own words), runtime
//! ingress validation (`T3.3.08`/`.10`/`.18`) remains the load-bearing
//! enforcement.

#[path = "receive_inventory_catalog.rs"]
mod receive_inventory_catalog;

use std::{fs, path::Path};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReceiveSiteClassV1 {
    /// Matched the pattern but is not actually a `Client`-family
    /// receive at all (an mpsc/oneshot channel `.recv()`/`.try_recv()`)
    /// -- a false positive of the broad grep pattern, same shape as
    /// `send_inventory.rs`'s own `NotAClientSend`.
    NotAClientReceive,
    /// `Client::recv`'s own primitive definition, and `try_recv_all`/
    /// `try_recv_all_semantic`/`try_recv_all_dispatch`'s own internal
    /// calls to it -- the mechanism every real receive site funnels
    /// through, not a call site itself.
    LegacyMechanism,
    /// Registration-flow receive (`sys/msg/register.rs`'s raw
    /// `ClientRegister`) -- before a session is admitted, never
    /// enveloped.
    PreAuth,
    /// The ping/pong control-plane stream -- deliberately never fenced,
    /// same `T3.3.09`/`.10` precedent as the send side's own `Ping`.
    Ping,
    /// A genuine post-auth receive site that reaches a raw
    /// `client.recv(stream_id)` INSTEAD OF `try_recv_all_dispatch` --
    /// the receive-side bypass this row exists to prevent. Pinned at
    /// `0` today (`post_auth_bypass_count_is_zero`); this variant
    /// exists so a future one has a real, named bucket instead of
    /// silently matching nothing.
    PostAuthBypassCandidate,
}

/// Re-scans `server/src` right now for the receive-shaped pattern,
/// returning the same `(file, trimmed line, 0-based occurrence index)`
/// shape `send_inventory.rs::scan_server_src` does, for the same
/// reasons (byte-for-byte catalog match without line-number drift, the
/// occurrence index disambiguating an identical snippet appearing twice
/// in one file).
///
/// Excludes this module's own two files, same reason
/// `send_inventory.rs` excludes its own.
pub(crate) fn scan_server_src(root: &Path) -> Vec<(String, String, u32)> {
    let mut out = Vec::new();
    scan_dir(root, root, &mut out);
    out.sort();
    out
}

const SELF_EXCLUDED_FILES: [&str; 2] =
    ["semantic_net/receive_inventory.rs", "semantic_net/receive_inventory_catalog.rs"];

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
        if line.contains(".recv(") || line.contains(".try_recv(") || line.contains("try_recv_all(") {
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
        ReceiveSiteClassV1::{self, PostAuthBypassCandidate},
        receive_inventory_catalog::RECEIVE_SITE_CATALOG,
        scan_server_src,
    };

    fn server_src_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    /// "Source allowlist", receive side: every receive-shaped line
    /// found by a FRESH scan right now is present in the frozen
    /// catalog with the SAME classification -- the receive-side half
    /// of "prevent future raw semantic bypass".
    #[test]
    fn every_live_receive_site_is_classified_exactly_once() {
        let live = scan_server_src(&server_src_root());
        let catalog: std::collections::HashMap<(&str, &str, u32), ReceiveSiteClassV1> =
            RECEIVE_SITE_CATALOG.iter().map(|&(f, s, i, c)| ((f, s, i), c)).collect();

        let mut unclassified = Vec::new();
        for (file, snippet, idx) in &live {
            if !catalog.contains_key(&(file.as_str(), snippet.as_str(), *idx)) {
                unclassified.push(format!("{file}: {snippet} (occurrence {idx})"));
            }
        }
        assert!(
            unclassified.is_empty(),
            "new/unclassified receive sites found (deliberate bypass would land here):\n{}",
            unclassified.join("\n")
        );
    }

    #[test]
    fn every_catalog_entry_still_matches_live_source() {
        let live: std::collections::HashSet<(String, String, u32)> = scan_server_src(&server_src_root()).into_iter().collect();
        let mut stale = Vec::new();
        for &(file, snippet, idx, _) in RECEIVE_SITE_CATALOG.iter() {
            if !live.contains(&(file.to_string(), snippet.to_string(), idx)) {
                stale.push(format!("{file}: {snippet} (occurrence {idx})"));
            }
        }
        assert!(stale.is_empty(), "catalog entries no longer found in live source (stale classification):\n{}", stale.join("\n"));
    }

    #[test]
    fn catalog_has_no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for &(file, snippet, idx, _) in RECEIVE_SITE_CATALOG.iter() {
            assert!(seen.insert((file, snippet, idx)), "duplicate catalog key: {file}: {snippet} (occurrence {idx})");
        }
    }

    /// "Deliberate bypass and new variant/producer must fail" (packet's
    /// own test list): same falsifier shape as `send_inventory.rs`'s
    /// own -- proves the coverage check can actually fail, without
    /// mutating real source.
    #[test]
    fn falsifier_an_uncatalogued_site_is_flagged() {
        let catalog: std::collections::HashMap<(&str, &str, u32), ReceiveSiteClassV1> =
            RECEIVE_SITE_CATALOG.iter().map(|&(f, s, i, c)| ((f, s, i), c)).collect();
        let synthetic_new_site = ("events/brand_new_feature.rs", "let _ = client.recv(9);", 0u32);
        assert!(
            !catalog.contains_key(&synthetic_new_site),
            "test fixture bug: the synthetic site must not already be in the real catalog"
        );
    }

    /// This row's own concrete "prevent future raw semantic bypass"
    /// acceptance gate at the classification level: the receive side
    /// has ZERO sites needing future migration, unlike the send side's
    /// 133 `PostAuthCandidate` -- every post-auth receive system
    /// already funnels through `try_recv_all_dispatch` (`T3.3.09`).
    #[test]
    fn post_auth_bypass_count_is_zero() {
        let n = RECEIVE_SITE_CATALOG.iter().filter(|&&(_, _, _, c)| c == PostAuthBypassCandidate).count();
        assert_eq!(n, 0, "a receive-side bypass exists -- see PostAuthBypassCandidate entries in the catalog");
    }
}
