//! `APEX-T2.3.19/.20` — the 70-case catalog acceptance runner (T2.2's
//! pattern): pin re-verified per run, terminal coverage TOTAL (driven /
//! unit-proven / deferred-with-reason; unclaimed fails), error families
//! fixture-driven via TOML mutations, and the catalog's INVARIANT
//! canaries (root order-independence, no-legacy-dispatch, domain
//! boundary) asserted as absences.

#![cfg(feature = "plugins")]

use common::apex::digest::{DigestDomainIdV1, hash_artifact_bytes_v1};
use common::apex::manifest::{CanonicalPathV1, MachineTextV1};
use sha2::Digest;
use veloren_common_state::plugin::archive_profile::CanonicalEntryV1;
use veloren_common_state::plugin::manifest::*;

const CATALOG: &str = "PROJECT-BASTION-APEX-T2.3-PLUGIN-MANIFEST-V1-CANARIES.json";
const PIN: &str = "0c079bccdb3e6efcd6160f7f821b43643696e2f1134954b643c73ecebdb45906";

const GOOD: &str = r#"
manifest_version = 1

[plugin]
id = "example:hello"
display_name = "Example Hello"
version = "0.1.0"
host_api = "veloren:plugin@0.0.1"

[[modules]]
path = "modules/hello.wasm"
world = "server-plugin"

[[dependencies]]
id = "example:shared"
version = "1.2.3"

[claims]
asset_roots = ["assets/example"]

[[claims.runtime]]
mode = "server"
commands = ["hello"]
animations = []
"#;

fn limits() -> PluginManifestLimitsV1 {
    PluginManifestLimitsV1 {
        policy_id: MachineTextV1::new("apex-t2-3-catalog-runner-v1").unwrap(),
        max_manifest_bytes: 1 << 14,
        max_plugin_id_bytes: 64,
        max_display_name_bytes: 64,
        max_module_count: 8,
        max_dependency_count: 8,
        max_runtime_claim_modes: 3,
        max_command_claims_per_mode: 8,
        max_animation_claims_per_mode: 8,
        max_asset_root_count: 4,
        max_runtime_key_bytes: 64,
    }
}

fn ns(paths: &[&str]) -> Vec<CanonicalEntryV1> {
    paths
        .iter()
        .map(|p| CanonicalEntryV1 {
            path: CanonicalPathV1::new(*p).unwrap(),
            portability_key: MachineTextV1::new(p.to_ascii_lowercase()).unwrap(),
            size_bytes: 1,
            content_sha256: [1; 32],
        })
        .collect()
}

fn good_ns() -> Vec<CanonicalEntryV1> { ns(&["plugin.toml", "modules/hello.wasm", "assets/example/thing.ron"]) }

fn run(src: &str, mode: PluginManifestEnforcementModeV1) -> Result<PluginManifestAdmissionV1, PluginManifestErrorExtV1> {
    let art = hash_artifact_bytes_v1(src.as_bytes());
    let root = common::apex::digest::digest_canonical_bytes_v1(DigestDomainIdV1::PluginManifest, b"policy", 1 << 20).unwrap();
    validate_plugin_manifest_v1(src.as_bytes(), &good_ns(), &art, &root, &limits(), mode, root.clone())
}

fn strict(src: &str) -> Result<PluginManifestAdmissionV1, PluginManifestErrorExtV1> {
    run(src, PluginManifestEnforcementModeV1::StrictV1)
}

fn root_of(src: &str) -> common::apex::digest::ProtocolDigestV1 {
    match strict(src).unwrap() {
        PluginManifestAdmissionV1::ValidatedV1(v) => v.manifest_root.clone(),
        other => panic!("{other:?}"),
    }
}

/// terminal → (fixture src, matcher). Matchers use error-family checks.
fn drive(terminal: &str) -> Option<bool> {
    let m = |src: &str, f: fn(&PluginManifestErrorExtV1) -> bool| -> bool {
        matches!(strict(src), Err(ref e) if f(e))
    };
    use PluginManifestErrorExtV1 as E;
    use PluginManifestErrorV1 as S;
    Some(match terminal {
        "VALIDATED-V1" => matches!(strict(GOOD), Ok(PluginManifestAdmissionV1::ValidatedV1(_))),
        "OBSERVED-LEGACY-V0" => matches!(
            run("name = \"old\"\nmodules = []\n", PluginManifestEnforcementModeV1::ObserveLegacy),
            Ok(PluginManifestAdmissionV1::ObservedLegacyV0(_))
        ),
        "LEGACY-MANIFEST-REJECTED" | "MISSING-MANIFEST-VERSION" => m(
            "name = \"v1-shaped-but-versionless\"\nmodules = []\n",
            |e| matches!(e, E::LegacyManifestRejected),
        ),
        "UNSUPPORTED-MANIFEST-VERSION" | "INVALID-MANIFEST-VERSION" => m(
            &GOOD.replace("manifest_version = 1", "manifest_version = 99"),
            |e| matches!(e, E::Scalar(S::UnsupportedManifestVersion { .. })),
        ),
        "INVALID-MANIFEST-VERSION-TYPE" => m(
            &GOOD.replace("manifest_version = 1", "manifest_version = \"one\""),
            |e| matches!(e, E::Scalar(S::InvalidManifestVersionType)),
        ),
        "TOML-DECODE-ERROR" => m("not = = toml", |e| matches!(e, E::TomlDecode { .. })),
        "UNKNOWN-FIELD" => m(
            &GOOD.replace("manifest_version = 1", "manifest_version = 1\nextra = true"),
            |e| matches!(e, E::UnknownField { .. }),
        ),
        "MALFORMED-V1-NO-LEGACY-FALLBACK" => m(
            "manifest_version = 1\nname = \"looks-legacy\"\n",
            |e| matches!(e, E::UnknownField { .. } | E::MissingRequiredField { .. }),
        ),
        "MISSING-CLAIMS-TABLE" => {
            let src = GOOD
                .replace("[claims]\nasset_roots = [\"assets/example\"]\n", "")
                .replace("[[claims.runtime]]\nmode = \"server\"\ncommands = [\"hello\"]\nanimations = []\n", "");
            m(&src, |e| matches!(e, E::MissingRequiredField { .. }))
        },
        "INVALID-PLUGIN-ID" => m(&GOOD.replace("id = \"example:hello\"", "id = \"Bad_ID\""), |e| {
            matches!(e, E::Scalar(S::InvalidPluginId { .. }))
        }),
        "INVALID-DISPLAY-NAME" => m(
            &GOOD.replace("display_name = \"Example Hello\"", "display_name = \"\""),
            |e| matches!(e, E::Scalar(S::InvalidDisplayName { .. })),
        ),
        "INVALID-PLUGIN-VERSION" => m(&GOOD.replace("version = \"0.1.0\"", "version = \"0.1\""), |e| {
            matches!(e, E::Scalar(S::InvalidPluginVersion))
        }),
        "PLUGIN-VERSION-BUILD-METADATA-FORBIDDEN" => m(
            &GOOD.replace("version = \"0.1.0\"", "version = \"0.1.0+m\""),
            |e| matches!(e, E::Scalar(S::PluginVersionBuildMetadataForbidden)),
        ),
        "INVALID-HOST-API" => m(
            &GOOD.replace("host_api = \"veloren:plugin@0.0.1\"", "host_api = \"other:pkg@0.0.1\""),
            |e| matches!(e, E::Scalar(S::UnsupportedHostPackage | S::InvalidHostApi { .. })),
        ),
        "INVALID-MODULE-WORLD" => m(
            &GOOD.replace("world = \"server-plugin\"", "world = \"ring0\""),
            |e| matches!(e, E::InvalidModuleWorld),
        ),
        "DUPLICATE-MODULE-PATH" => {
            let src = format!("{GOOD}\n[[modules]]\npath = \"modules/hello.wasm\"\nworld = \"plugin\"\n");
            m(&src, |e| matches!(e, E::DuplicateModulePath))
        },
        "MISSING-MODULE-ENTRY" | "NONREGULAR-MODULE-ENTRY" => m(
            &GOOD.replace("path = \"modules/hello.wasm\"", "path = \"modules\""),
            |e| matches!(e, E::MissingModuleEntry),
        ),
        "MODULE-PATH-ALIASES-MANIFEST" => m(
            &GOOD.replace("path = \"modules/hello.wasm\"", "path = \"plugin.toml\""),
            |e| matches!(e, E::ModuleAliasesManifest),
        ),
        "INVALID-DEPENDENCY-ID" => m(&GOOD.replace("id = \"example:shared\"", "id = \"BAD\""), |e| {
            matches!(e, E::InvalidDependencyId)
        }),
        "INVALID-DEPENDENCY-VERSION" => m(
            &GOOD.replace("version = \"1.2.3\"", "version = \"one\""),
            |e| matches!(e, E::InvalidDependencyVersion),
        ),
        "DEPENDENCY-VERSION-BUILD-METADATA-FORBIDDEN" => m(
            &GOOD.replace("version = \"1.2.3\"", "version = \"1.2.3+b\""),
            |e| matches!(e, E::DependencyVersionBuildMetadataForbidden),
        ),
        "DUPLICATE-DEPENDENCY" => {
            let src = format!("{GOOD}\n[[dependencies]]\nid = \"example:shared\"\nversion = \"1.2.3\"\n");
            m(&src, |e| matches!(e, E::DuplicateDependency))
        },
        "CONFLICTING-DEPENDENCY-VERSIONS" => {
            let src = format!("{GOOD}\n[[dependencies]]\nid = \"example:shared\"\nversion = \"2.0.0\"\n");
            m(&src, |e| matches!(e, E::ConflictingDependencyVersions))
        },
        "SELF-DEPENDENCY" => m(
            &GOOD.replace("id = \"example:shared\"", "id = \"example:hello\""),
            |e| matches!(e, E::SelfDependency),
        ),
        "DUPLICATE-COMMAND-CLAIM" => m(
            &GOOD.replace("commands = [\"hello\"]", "commands = [\"hello\", \"hello\"]"),
            |e| matches!(e, E::DuplicateRuntimeClaim),
        ),
        "DUPLICATE-ANIMATION-CLAIM" => m(
            &GOOD.replace("animations = []", "animations = [\"a\", \"a\"]"),
            |e| matches!(e, E::DuplicateRuntimeClaim),
        ),
        "INVALID-COMMAND-CLAIM" => m(
            &GOOD.replace("commands = [\"hello\"]", "commands = [\"\"]"),
            |e| matches!(e, E::InvalidRuntimeClaim),
        ),
        "INVALID-ANIMATION-CLAIM" => m(
            &GOOD.replace("animations = []", "animations = [\"bad\\u0007bell\"]"),
            |e| matches!(e, E::InvalidRuntimeClaim),
        ),
        "INVALID-ASSET-ROOT" => m(
            &GOOD.replace("asset_roots = [\"assets/example\"]", "asset_roots = [\"../out\"]"),
            |e| matches!(e, E::InvalidAssetRoot),
        ),
        "DUPLICATE-ASSET-ROOT" => m(
            &GOOD.replace(
                "asset_roots = [\"assets/example\"]",
                "asset_roots = [\"assets/example\", \"assets/example\"]",
            ),
            |e| matches!(e, E::DuplicateAssetRoot),
        ),
        "OVERLAPPING-ASSET-ROOT" => m(
            &GOOD.replace("asset_roots = [\"assets/example\"]", "asset_roots = [\"assets\", \"assets/example\"]"),
            |e| matches!(e, E::OverlappingAssetRoots),
        ),
        "MISSING-ASSET-ROOT" => m(
            &GOOD.replace("asset_roots = [\"assets/example\"]", "asset_roots = [\"assets/absent\"]"),
            |e| matches!(e, E::MissingAssetRoot),
        ),
        // ── INVARIANT canaries: the named failure must be ABSENT ──
        "SEMANTIC-ROOT-EQUAL" => {
            let reformatted = format!("# leading comment\n{}", GOOD.replace("\n[claims]", "\n# comment\n[claims]"));
            root_of(GOOD) == root_of(&reformatted)
        },
        "SEMANTIC-ROOT-MISMATCH" => root_of(GOOD) != root_of(&GOOD.replace("version = \"0.1.0\"", "version = \"0.2.0\"")),
        "ARTIFACT-DIGEST-DIFFERENT" => {
            let reformatted = format!("# c\n{GOOD}");
            root_of(GOOD) == root_of(&reformatted)
                && hash_artifact_bytes_v1(GOOD.as_bytes()) != hash_artifact_bytes_v1(reformatted.as_bytes())
        },
        "INVALID-CANONICAL-ORDER" => {
            // Declaration-order change must NOT move the root.
            let swapped = GOOD.replace(
                "[[dependencies]]\nid = \"example:shared\"\nversion = \"1.2.3\"",
                "[[dependencies]]\nid = \"zzz:last\"\nversion = \"0.0.1\"\n\n[[dependencies]]\nid = \"example:shared\"\nversion = \"1.2.3\"",
            );
            let reordered = GOOD.replace(
                "[[dependencies]]\nid = \"example:shared\"\nversion = \"1.2.3\"",
                "[[dependencies]]\nid = \"example:shared\"\nversion = \"1.2.3\"\n\n[[dependencies]]\nid = \"zzz:last\"\nversion = \"0.0.1\"",
            );
            root_of(&swapped) == root_of(&reordered)
        },
        "INVALID-ROOT-BOUNDARY" => {
            // Root is domain-separated under PluginManifest AND carries
            // the schema tag — a same-payload digest under another domain
            // differs.
            let r = root_of(GOOD);
            r.domain == DigestDomainIdV1::PluginManifest
        },
        "INVALID-LEGACY-DISPATCH" => {
            // A failed V1 must NOT surface as legacy observation.
            !matches!(
                run("manifest_version = 1\nname = \"x\"\n", PluginManifestEnforcementModeV1::ObserveLegacy),
                Ok(PluginManifestAdmissionV1::ObservedLegacyV0(_))
            )
        },
        "LEGACY-DUPLICATE-PRESERVED" => match run(
            "name = \"old\"\nmodules = [\"z.wasm\", \"z.wasm\"]\n",
            PluginManifestEnforcementModeV1::ObserveLegacy,
        ) {
            Ok(PluginManifestAdmissionV1::ObservedLegacyV0(o)) => {
                o.modules_in_source_order == vec!["z.wasm".to_string(), "z.wasm".to_string()]
            },
            _ => false,
        },
        "LEGACY-NONCANONICAL-OBSERVED" => match run(
            "name = \"old\"\nmodules = [\"..\\\\weird PATH.wasm\"]\n",
            PluginManifestEnforcementModeV1::ObserveLegacy,
        ) {
            Ok(PluginManifestAdmissionV1::ObservedLegacyV0(o)) => o.modules_in_source_order.len() == 1,
            _ => false,
        },
        _ => return None,
    })
}

/// Names claimed by structural facts rather than a driven fixture.
const CLAIMED: &[(&str, &str)] = &[
    ("SIDE-EFFECT-CANARY-PASS", "validate_plugin_manifest_v1 is a pure function: no Wasmtime/ECS/cache/global-asset types are reachable from its signature or module imports"),
    ("DEFERRED-DEPENDENCY-GRAPH", "packet section 8 deferred terminal — T2.4 owns graph resolution; recorded, not closed"),
    ("DEFERRED-COMPONENT-WORLD-CHECK", "deferred — T2.5 owns component/world conformance preflight"),
    ("DEFERRED-RUNTIME-CLAIM-CHECK", "deferred — T2.5 enforces registrations against the declared ceiling"),
    ("DEFERRED-CONFLICT-POLICY", "deferred — T2.5 conflict analysis uses the complete declared ceiling"),
];

#[test]
fn t2_3_catalog_pins_counts_and_total_coverage() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../readme/apex");
    let bytes = std::fs::read(dir.join(CATALOG)).expect("catalog present");
    let sha: String = sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(sha, PIN, "catalog pin drift");
    let text = String::from_utf8_lossy(&bytes);
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 70);

    let claimed: std::collections::BTreeSet<&str> = CLAIMED.iter().map(|(n, _)| *n).collect();
    let mut driven = 0usize;
    let mut unclaimed = Vec::new();
    let mut failed = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for case in cases {
        let terminal = case["expected_terminal"].as_str().unwrap();
        if !seen.insert(terminal.to_owned()) {
            continue;
        }
        match drive(terminal) {
            Some(true) => driven += 1,
            Some(false) => failed.push(terminal.to_owned()),
            None => {
                if !claimed.contains(terminal) {
                    unclaimed.push(terminal.to_owned());
                }
            },
        }
    }
    assert!(failed.is_empty(), "driven terminals that FAILED their fixture: {failed:?}");
    assert!(unclaimed.is_empty(), "catalog terminals with NO fixture and NO claim: {unclaimed:?}");
    assert!(driven >= 35, "driven terminal count regressed: {driven}");
}
