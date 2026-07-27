//! `APEX-T2.5.06` — expand claimed asset roots to EXACT keys, purely.
//!
//! Deployment compiles prefix claims (`claims.asset_roots`) plus the
//! admitted archive namespace into the sorted, exact list of asset IDs
//! this plugin publishes, so conflict policy (.07) decides over concrete
//! keys instead of prefixes and no publishable file is resolved by
//! container iteration order. Pure string-level validation: no cache, no
//! filesystem, no registration.

use serde::{Deserialize, Serialize};

/// Canonical archive layout: publishable assets live under this tree.
pub const PLUGIN_ASSET_TREE_PREFIX_V1: &str = "assets/";

/// One exact publishable key: `assets/example/thing.ron` under root
/// `assets/example` → id `example.thing`, extension `ron`.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PluginAssetKeyV1 {
    pub asset_id: String,
    pub extension: String,
    /// The archive path this key was derived from (evidence pointer;
    /// derivation is injective so this adds no identity).
    pub source_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginAssetKeyErrorV1 {
    /// A claimed root outside `assets/` can never publish.
    RootOutsideAssetTree { root: String },
    /// A file under `assets/` not covered by any claimed root — the claim
    /// inventory must cover every publishable file (fail closed, never
    /// silently publish or silently drop).
    UnclaimedAssetFile { path: String },
    /// Path cannot become an exact key: uppercase (case-insensitive
    /// filesystems would collide), dot in a directory/stem segment
    /// (ambiguous with the id separator), empty segment/stem/extension,
    /// or a byte outside the id alphabet.
    UnrepresentableKey { path: String, detail: &'static str },
    /// Two distinct archive paths deriving the same (id, extension).
    DuplicateKey { asset_id: String, extension: String },
}

fn segment_ok(seg: &str) -> Result<(), &'static str> {
    if seg.is_empty() {
        return Err("empty segment");
    }
    for b in seg.bytes() {
        match b {
            b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' => {},
            b'A'..=b'Z' => return Err("uppercase"),
            b'.' => return Err("dot inside segment"),
            _ => return Err("byte outside id alphabet"),
        }
    }
    Ok(())
}

/// The pure expansion. `asset_roots` are the manifest's claimed roots
/// (e.g. `assets/example`); `entry_paths` is the FULL admitted archive
/// namespace (manifest/modules included — non-asset entries are not
/// publishable and are skipped; asset-tree entries must be claimed).
pub fn plugin_asset_keys_v1(
    asset_roots: &[&str],
    entry_paths: &[&str],
) -> Result<Vec<PluginAssetKeyV1>, PluginAssetKeyErrorV1> {
    for root in asset_roots {
        let under = root
            .strip_prefix(PLUGIN_ASSET_TREE_PREFIX_V1)
            .filter(|rest| !rest.is_empty() && !rest.ends_with('/'));
        if under.is_none() {
            return Err(PluginAssetKeyErrorV1::RootOutsideAssetTree { root: root.to_string() });
        }
    }

    let mut keys: Vec<PluginAssetKeyV1> = Vec::new();
    for path in entry_paths {
        let Some(in_tree) = path.strip_prefix(PLUGIN_ASSET_TREE_PREFIX_V1) else {
            continue; // manifest/modules/etc: not publishable
        };
        // Root coverage is by whole path segments — `assets/exampleX/…`
        // is NOT under the claimed root `assets/example`.
        let claimed = asset_roots.iter().any(|root| {
            let rest = &root[PLUGIN_ASSET_TREE_PREFIX_V1.len()..];
            in_tree.strip_prefix(rest).is_some_and(|tail| tail.starts_with('/'))
        });
        if !claimed {
            return Err(PluginAssetKeyErrorV1::UnclaimedAssetFile { path: path.to_string() });
        }

        let unrep = |detail| PluginAssetKeyErrorV1::UnrepresentableKey { path: path.to_string(), detail };
        let mut segments: Vec<&str> = in_tree.split('/').collect();
        let file = segments.pop().expect("split is never empty");
        let (stem, extension) = file.rsplit_once('.').ok_or_else(|| unrep("missing extension"))?;
        for seg in segments.iter().chain([&stem]) {
            segment_ok(seg).map_err(unrep)?;
        }
        if extension.is_empty() {
            return Err(unrep("empty extension"));
        }
        segment_ok(extension).map_err(unrep)?;

        let mut asset_id = String::with_capacity(in_tree.len());
        for seg in segments.iter().chain([&stem]) {
            if !asset_id.is_empty() {
                asset_id.push('.');
            }
            asset_id.push_str(seg);
        }
        keys.push(PluginAssetKeyV1 { asset_id, extension: extension.to_string(), source_path: path.to_string() });
    }

    keys.sort();
    for pair in keys.windows(2) {
        if pair[0].asset_id == pair[1].asset_id && pair[0].extension == pair[1].extension {
            return Err(PluginAssetKeyErrorV1::DuplicateKey {
                asset_id: pair[0].asset_id.clone(),
                extension: pair[0].extension.clone(),
            });
        }
    }
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_asset_keys_v1_exact_expansion_and_boundaries() {
        // Happy path: exact keys, sorted, input order irrelevant.
        let keys = plugin_asset_keys_v1(
            &["assets/example", "assets/other"],
            &[
                "plugin.toml",
                "modules/hello.wasm",
                "assets/other/z.ron",
                "assets/example/deep/thing.ron",
                "assets/example/thing.png",
            ],
        )
        .unwrap();
        let flat: Vec<(&str, &str)> = keys.iter().map(|k| (k.asset_id.as_str(), k.extension.as_str())).collect();
        assert_eq!(flat, vec![
            ("example.deep.thing", "ron"),
            ("example.thing", "png"),
            ("other.z", "ron"),
        ]);

        // Root boundary: sibling-prefix directory is NOT covered.
        assert_eq!(
            plugin_asset_keys_v1(&["assets/example"], &["assets/examplex/a.ron"]),
            Err(PluginAssetKeyErrorV1::UnclaimedAssetFile { path: "assets/examplex/a.ron".into() })
        );
        // Unclaimed publishable file fails closed.
        assert!(matches!(
            plugin_asset_keys_v1(&["assets/example"], &["assets/rogue/a.ron"]),
            Err(PluginAssetKeyErrorV1::UnclaimedAssetFile { .. })
        ));
        // A root outside the asset tree can never publish.
        assert!(matches!(
            plugin_asset_keys_v1(&["modules"], &[]),
            Err(PluginAssetKeyErrorV1::RootOutsideAssetTree { .. })
        ));

        // Case boundary: uppercase is unrepresentable (case-insensitive
        // filesystems would collide two distinct keys).
        assert!(matches!(
            plugin_asset_keys_v1(&["assets/example"], &["assets/example/Thing.ron"]),
            Err(PluginAssetKeyErrorV1::UnrepresentableKey { detail: "uppercase", .. })
        ));
        // Dot in stem is ambiguous with the id separator.
        assert!(matches!(
            plugin_asset_keys_v1(&["assets/example"], &["assets/example/a.b.ron"]),
            Err(PluginAssetKeyErrorV1::UnrepresentableKey { detail: "dot inside segment", .. })
        ));
        // Extension boundaries.
        assert!(matches!(
            plugin_asset_keys_v1(&["assets/example"], &["assets/example/noext"]),
            Err(PluginAssetKeyErrorV1::UnrepresentableKey { detail: "missing extension", .. })
        ));
        assert!(matches!(
            plugin_asset_keys_v1(&["assets/example"], &["assets/example/dot."]),
            Err(PluginAssetKeyErrorV1::UnrepresentableKey { detail: "empty extension", .. })
        ));

        // Two claimed roots where one contains the other still map each
        // file exactly once (coverage is a predicate, not a partition).
        let nested = plugin_asset_keys_v1(
            &["assets/example", "assets/example/deep"],
            &["assets/example/deep/thing.ron"],
        )
        .unwrap();
        assert_eq!(nested.len(), 1);

        // Duplicate exact key across distinct paths is refused. Distinct
        // archive paths can only collide via dot-vs-slash ambiguity, which
        // is already unrepresentable — prove the duplicate arm directly on
        // identical-path doubling.
        assert!(matches!(
            plugin_asset_keys_v1(&["assets/example"], &["assets/example/a.ron", "assets/example/a.ron"]),
            Err(PluginAssetKeyErrorV1::DuplicateKey { .. })
        ));
    }
}
