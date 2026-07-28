//! `APEX-T6.4` — `NumericProfileV1`.
//!
//! A numeric profile is a precise tested tuple, not "uses IEEE floats".
//!
//! **What this row does NOT claim, stated first because the temptation is
//! to claim it.** Stable Rust's flags do not enforce complete strict
//! floating semantics, and nothing here says they do. A profile that
//! rested on that claim would be asserting something the toolchain does
//! not promise. **The golden conformance vectors are the authority** —
//! they are what is actually tested. The tuple is what is recorded, so
//! that when two machines' vectors disagree there is something to diff.
//!
//! **Two properties, two types, and the row is explicit that conflating
//! them certifies the wrong one:**
//!
//! - [`ArtifactReproducibilityV1`] — same inputs produce the same
//!   binary.
//! - [`ExecutionVectorEqualityV1`] — *different* binaries produce the
//!   same numeric results.
//!
//! Neither converts into the other, and neither is comparable to the
//! other. A reproducible build says nothing about cross-target numerics;
//! equal vectors say nothing about whether the build is reproducible.
//!
//! **Two prohibitions with teeth.** `target-cpu=native` makes the
//! binary's numerics a function of the build MACHINE, which destroys the
//! tuple's meaning entirely — a recorded profile would then describe a
//! machine that is not the one running the code. Undeclared codegen
//! features are the same failure in slower motion. Both are rejected at
//! construction, and a repository-level test scans the real
//! `.cargo/config.toml`.

use super::digest::{ArtifactIdentityV1, hash_artifact_bytes_v1};

/// The recorded tuple.
///
/// Every field is here because it can change numeric output. Adding a
/// field changes every profile's identity, which is correct: a profile
/// that recorded less was describing something else.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumericProfileV1 {
    pub rustc_version: String,
    pub llvm_version: String,
    pub target_triple: String,
    /// Explicitly declared CPU features, in the order declared. Order is
    /// preserved rather than sorted: a caller whose feature list is
    /// order-unstable has a problem this type should not hide.
    pub cpu_features: Vec<String>,
    pub profile: String,
    pub codegen_flags: Vec<String>,
    pub lto: String,
    pub codegen_units: u32,
    /// Native libraries whose implementations reach the simulation —
    /// `libm` above all, since `powf` is its function and not ours.
    pub native_libraries: Vec<String>,
    /// Rounding mode and subnormal handling ASSUMED by this profile.
    /// Assumed, not enforced: see the module doc.
    pub rounding_assumption: String,
    pub subnormal_assumption: String,
    /// Digest of the dependency set, which `T1.2`'s source closure
    /// already computes. Referenced rather than recomputed.
    pub dependency_set_root: [u8; 32],
}

/// Why a profile was rejected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NumericProfileErrorV1 {
    /// `target-cpu=native` (or `-C target-cpu=native`) appeared. The
    /// binary's numerics become a function of the build machine, so the
    /// recorded tuple describes a machine that is not the one running
    /// the code.
    NativeTargetCpu(String),
    /// A codegen flag enables a CPU feature that the profile does not
    /// declare. The tuple would then be incomplete in exactly the way
    /// that matters.
    UndeclaredFeature(String),
    /// A field that must be recorded was left empty. An empty field is
    /// not "unknown", it is a profile that quietly compares equal to
    /// another profile with the same gap.
    EmptyField(&'static str),
}

impl NumericProfileV1 {
    /// Validate and take identity. There is no way to obtain a
    /// [`NumericProfileIdV1`] without passing here, so an unvalidated
    /// profile cannot be recorded anywhere.
    pub fn validated_v1(self) -> Result<ValidatedNumericProfileV1, NumericProfileErrorV1> {
        for (name, value) in [
            ("rustc_version", &self.rustc_version),
            ("llvm_version", &self.llvm_version),
            ("target_triple", &self.target_triple),
            ("profile", &self.profile),
            ("lto", &self.lto),
            ("rounding_assumption", &self.rounding_assumption),
            ("subnormal_assumption", &self.subnormal_assumption),
        ] {
            if value.trim().is_empty() {
                return Err(NumericProfileErrorV1::EmptyField(name));
            }
        }

        for flag in &self.codegen_flags {
            if let Some(err) = reject_native_target_cpu_v1(flag) {
                return Err(err);
            }
            // `-C target-feature=+avx2` must have `avx2` declared.
            if let Some(features) = flag.split("target-feature=").nth(1) {
                for feature in features.split(',') {
                    let name = feature.trim_start_matches(['+', '-']).trim();
                    if name.is_empty() {
                        continue;
                    }
                    if !self.cpu_features.iter().any(|declared| declared == name) {
                        return Err(NumericProfileErrorV1::UndeclaredFeature(name.to_owned()));
                    }
                }
            }
        }

        Ok(ValidatedNumericProfileV1 { profile: self })
    }
}

/// Rejects `target-cpu=native` in any spelling that reaches rustc.
///
/// Exposed on its own so the repository-level scan and profile
/// validation cannot drift apart — one predicate, two callers.
pub fn reject_native_target_cpu_v1(flag: &str) -> Option<NumericProfileErrorV1> {
    let normalised = flag.replace(char::is_whitespace, "").to_ascii_lowercase();
    normalised
        .contains("target-cpu=native")
        .then(|| NumericProfileErrorV1::NativeTargetCpu(flag.to_owned()))
}

/// A profile that passed validation. The only thing that has an identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedNumericProfileV1 {
    profile: NumericProfileV1,
}

impl ValidatedNumericProfileV1 {
    pub const fn profile_v1(&self) -> &NumericProfileV1 { &self.profile }

    /// Identity over every recorded field.
    ///
    /// Field values are length-prefixed so that moving a character from
    /// one field into the next cannot produce the same bytes — the
    /// classic concatenation collision, which here would make two
    /// genuinely different toolchains share a profile.
    pub fn id_v1(&self) -> NumericProfileIdV1 {
        let p = &self.profile;
        let mut bytes = Vec::new();
        let mut push = |value: &str| {
            bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
            bytes.extend_from_slice(value.as_bytes());
        };
        push(&p.rustc_version);
        push(&p.llvm_version);
        push(&p.target_triple);
        push(&p.profile);
        push(&p.lto);
        push(&p.rounding_assumption);
        push(&p.subnormal_assumption);
        for feature in &p.cpu_features {
            push(feature);
        }
        for flag in &p.codegen_flags {
            push(flag);
        }
        for library in &p.native_libraries {
            push(library);
        }
        bytes.extend_from_slice(&p.codegen_units.to_be_bytes());
        bytes.extend_from_slice(&p.dependency_set_root);
        NumericProfileIdV1(hash_artifact_bytes_v1(&bytes))
    }
}

/// A validated profile's identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumericProfileIdV1(ArtifactIdentityV1);

impl NumericProfileIdV1 {
    pub const fn identity_v1(&self) -> &ArtifactIdentityV1 { &self.0 }
}

/// Same inputs produce the same binary.
///
/// ```compile_fail
/// # use veloren_common::apex::numeric_profile::*;
/// let artifact = ArtifactReproducibilityV1::established_v1();
/// // T6.4: a reproducible BUILD says nothing about cross-target
/// // NUMERICS. Conflating them certifies the wrong property.
/// let execution: ExecutionVectorEqualityV1 = artifact.into();
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ArtifactReproducibilityV1 {
    established: bool,
}

impl ArtifactReproducibilityV1 {
    pub const fn established_v1() -> Self { Self { established: true } }

    pub const fn unestablished_v1() -> Self { Self { established: false } }

    pub const fn is_established_v1(self) -> bool { self.established }
}

/// Different binaries produce the same numeric results.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ExecutionVectorEqualityV1 {
    established: bool,
}

impl ExecutionVectorEqualityV1 {
    pub const fn established_v1() -> Self { Self { established: true } }

    pub const fn unestablished_v1() -> Self { Self { established: false } }

    pub const fn is_established_v1(self) -> bool { self.established }
}

/// One golden conformance vector: an input and the bits the kernel must
/// return for it.
///
/// Bits, not floats — the whole point is to catch a last-place
/// difference, and `==` on floats is what hides one.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct GoldenVectorV1 {
    pub function: &'static str,
    pub input_bits: u32,
    pub expected_bits: u32,
}

/// What a golden-vector run found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GoldenVerdictV1 {
    /// Every vector matched. The ONLY statement this row is entitled to
    /// make about numeric conformance, and it is a statement about the
    /// vectors, not about the toolchain's guarantees.
    AllVectorsMatch,
    /// The first vector that did not match, with what was returned.
    FirstMismatch {
        function: &'static str,
        input_bits: u32,
        expected_bits: u32,
        actual_bits: u32,
    },
    /// No vectors were supplied. Reported rather than passing: a run
    /// over an empty set matching everything is exactly the shape of a
    /// broken harness, and `AllVectorsMatch` would be a lie in the most
    /// convincing form.
    NoVectors,
}

/// Run the vectors against a kernel.
pub fn verify_golden_vectors_v1(
    vectors: &[GoldenVectorV1],
    kernel: fn(f32) -> f32,
) -> GoldenVerdictV1 {
    if vectors.is_empty() {
        return GoldenVerdictV1::NoVectors;
    }
    for vector in vectors {
        let actual_bits = kernel(f32::from_bits(vector.input_bits)).to_bits();
        if actual_bits != vector.expected_bits {
            return GoldenVerdictV1::FirstMismatch {
                function: vector.function,
                input_bits: vector.input_bits,
                expected_bits: vector.expected_bits,
                actual_bits,
            };
        }
    }
    GoldenVerdictV1::AllVectorsMatch
}

/// What the toolchain actually promises, recorded so nobody has to
/// re-derive it from optimism.
pub const TOOLCHAIN_DOES_NOT_PROMISE: &[&str] = &[
    "stable Rust exposes no flag enforcing complete strict floating-point semantics; the golden \
     vectors are the authority, not the flag set",
    "powf/sin/cos/ln are the platform libm's and carry no correct-rounding requirement; only \
     sqrt does (IEEE 754 §5.4.1) — see T6.1's numeric surface",
    "LLVM may contract a multiply-add into an FMA where the target allows it; the vectors are \
     what detects the difference, the tuple is what explains it",
];

#[cfg(test)]
mod numeric_profile_v1 {
    use super::*;

    fn profile() -> NumericProfileV1 {
        NumericProfileV1 {
            rustc_version: "1.90.0".to_owned(),
            llvm_version: "20.1.4".to_owned(),
            target_triple: "x86_64-pc-windows-gnu".to_owned(),
            cpu_features: vec!["sse2".to_owned()],
            profile: "dev".to_owned(),
            codegen_flags: vec!["-C target-feature=+sse2".to_owned()],
            lto: "off".to_owned(),
            codegen_units: 16,
            native_libraries: vec!["libm".to_owned()],
            rounding_assumption: "round-to-nearest-even".to_owned(),
            subnormal_assumption: "subnormals preserved (no flush-to-zero)".to_owned(),
            dependency_set_root: [7u8; 32],
        }
    }

    fn id(p: NumericProfileV1) -> NumericProfileIdV1 {
        p.validated_v1().expect("fixture profile is valid").id_v1()
    }

    /// **The row's first required test.** A profile differing in exactly
    /// one recorded field is a DIFFERENT profile — checked field by
    /// field, so a field accidentally left out of the identity is caught
    /// rather than assumed present.
    #[test]
    fn changing_any_single_recorded_field_changes_the_profile() {
        let base = id(profile());

        let mutations: Vec<(&str, NumericProfileV1)> = vec![
            ("rustc_version", NumericProfileV1 { rustc_version: "1.91.0".to_owned(), ..profile() }),
            ("llvm_version", NumericProfileV1 { llvm_version: "21.0.0".to_owned(), ..profile() }),
            ("target_triple", NumericProfileV1 {
                target_triple: "aarch64-unknown-linux-gnu".to_owned(),
                ..profile()
            }),
            ("cpu_features", NumericProfileV1 {
                cpu_features: vec!["sse2".to_owned(), "avx2".to_owned()],
                codegen_flags: vec!["-C target-feature=+sse2,+avx2".to_owned()],
                ..profile()
            }),
            ("profile", NumericProfileV1 { profile: "release".to_owned(), ..profile() }),
            ("codegen_flags", NumericProfileV1 {
                codegen_flags: vec!["-C target-feature=+sse2".to_owned(), "-C opt-level=3".to_owned()],
                ..profile()
            }),
            ("lto", NumericProfileV1 { lto: "thin".to_owned(), ..profile() }),
            ("codegen_units", NumericProfileV1 { codegen_units: 1, ..profile() }),
            ("native_libraries", NumericProfileV1 {
                native_libraries: vec!["libm".to_owned(), "openlibm".to_owned()],
                ..profile()
            }),
            ("rounding_assumption", NumericProfileV1 {
                rounding_assumption: "round-toward-zero".to_owned(),
                ..profile()
            }),
            ("subnormal_assumption", NumericProfileV1 {
                subnormal_assumption: "flush-to-zero".to_owned(),
                ..profile()
            }),
            ("dependency_set_root", NumericProfileV1 {
                dependency_set_root: [8u8; 32],
                ..profile()
            }),
        ];

        assert_eq!(mutations.len(), 12, "a recorded field is not exercised here");
        for (field, mutated) in mutations {
            assert_ne!(
                id(mutated),
                base,
                "changing {field} did not change the profile identity, so it is recorded in the \
                 struct but not in its identity"
            );
        }
    }

    /// Field values are length-prefixed, so moving a character across a
    /// field boundary cannot produce the same identity. Without this two
    /// genuinely different toolchains could share a profile.
    #[test]
    fn adjacent_fields_cannot_collide_by_concatenation() {
        let a = NumericProfileV1 {
            rustc_version: "1.90".to_owned(),
            llvm_version: "0.20".to_owned(),
            ..profile()
        };
        let b = NumericProfileV1 {
            rustc_version: "1.900".to_owned(),
            llvm_version: ".20".to_owned(),
            ..profile()
        };
        assert_ne!(id(a), id(b));
    }

    /// **The row's second required test.** `target-cpu=native` is
    /// rejected, in every spelling that reaches rustc.
    #[test]
    fn target_cpu_native_is_rejected() {
        for spelling in [
            "-C target-cpu=native",
            "-Ctarget-cpu=native",
            "-C  TARGET-CPU=NATIVE",
            "--codegen target-cpu=native",
        ] {
            let rejected = NumericProfileV1 {
                codegen_flags: vec![spelling.to_owned()],
                ..profile()
            }
            .validated_v1();
            assert!(
                matches!(rejected, Err(NumericProfileErrorV1::NativeTargetCpu(_))),
                "{spelling:?} was accepted; the profile would describe the build machine rather \
                 than the running one"
            );
        }
    }

    /// The repository's own `.cargo/config.toml` is scanned, so the
    /// prohibition applies to the real build and not only to profiles
    /// someone remembers to validate.
    #[test]
    fn the_repositorys_cargo_config_does_not_enable_native_target_cpu() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("common has a parent");
        let config = root.join(".cargo").join("config.toml");
        let Ok(text) = std::fs::read_to_string(&config) else {
            // No config is fine; a config that enables native is not.
            return;
        };
        for (number, line) in text.lines().enumerate() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            assert!(
                reject_native_target_cpu_v1(line).is_none(),
                ".cargo/config.toml:{} enables target-cpu=native, which makes every numeric \
                 profile a description of the build machine:\n{line}",
                number + 1
            );
        }
    }

    /// A codegen flag enabling a feature the profile does not declare is
    /// rejected: the tuple would be incomplete exactly where it matters.
    #[test]
    fn an_undeclared_feature_is_rejected() {
        let rejected = NumericProfileV1 {
            cpu_features: vec!["sse2".to_owned()],
            codegen_flags: vec!["-C target-feature=+sse2,+fma".to_owned()],
            ..profile()
        }
        .validated_v1();
        assert_eq!(
            rejected,
            Err(NumericProfileErrorV1::UndeclaredFeature("fma".to_owned())),
            "an undeclared feature was accepted"
        );
    }

    /// An empty required field is rejected. An empty field is not
    /// "unknown" — it is a profile that quietly compares equal to another
    /// with the same gap.
    #[test]
    fn an_empty_required_field_is_rejected() {
        let rejected =
            NumericProfileV1 { llvm_version: "   ".to_owned(), ..profile() }.validated_v1();
        assert_eq!(rejected, Err(NumericProfileErrorV1::EmptyField("llvm_version")));
    }

    /// **The row's third required test.** The golden vectors fail on a
    /// deliberately perturbed kernel — and pass on the real one, so the
    /// failure is attributable to the perturbation rather than to the
    /// vectors being wrong.
    #[test]
    fn the_golden_vectors_fail_on_a_perturbed_kernel() {
        fn real(x: f32) -> f32 { x.sqrt() }
        fn perturbed(x: f32) -> f32 { f32::from_bits(x.sqrt().to_bits().wrapping_add(1)) }

        // sqrt is IEEE-754 correctly rounded, so these bits are the same
        // on every conforming target — which is what makes it a usable
        // vector source for this test. See T6.1.
        let vectors: Vec<GoldenVectorV1> = [0.5_f32, 1.0, 2.0, 1234.5]
            .into_iter()
            .map(|x| GoldenVectorV1 {
                function: "sqrt",
                input_bits: x.to_bits(),
                expected_bits: x.sqrt().to_bits(),
            })
            .collect();

        assert_eq!(
            verify_golden_vectors_v1(&vectors, real),
            GoldenVerdictV1::AllVectorsMatch,
            "the vectors do not pass on the real kernel, so a failure below would prove nothing"
        );

        let verdict = verify_golden_vectors_v1(&vectors, perturbed);
        let GoldenVerdictV1::FirstMismatch { function, expected_bits, actual_bits, .. } = verdict
        else {
            panic!("a one-ulp perturbation was not detected: {verdict:?}");
        };
        assert_eq!(function, "sqrt");
        assert_eq!(actual_bits, expected_bits.wrapping_add(1));
    }

    /// An empty vector set is `NoVectors`, not `AllVectorsMatch`. A run
    /// over nothing matching everything is the shape of a broken
    /// harness, and the passing verdict would be a lie in its most
    /// convincing form.
    #[test]
    fn an_empty_vector_set_does_not_pass() {
        assert_eq!(verify_golden_vectors_v1(&[], |x| x), GoldenVerdictV1::NoVectors);
    }

    /// The two properties are distinct types with no conversion, so a
    /// profile cannot certify the wrong one. The `compile_fail` doctest
    /// on `ArtifactReproducibilityV1` pins the missing conversion; this
    /// pins that they are genuinely independent values.
    #[test]
    fn artifact_reproducibility_and_execution_equality_are_independent() {
        let reproducible = ArtifactReproducibilityV1::established_v1();
        let vectors_disagree = ExecutionVectorEqualityV1::unestablished_v1();
        assert!(reproducible.is_established_v1());
        assert!(!vectors_disagree.is_established_v1());

        // And the other way round: two different binaries can agree on
        // every vector while neither build is reproducible.
        let not_reproducible = ArtifactReproducibilityV1::unestablished_v1();
        let vectors_agree = ExecutionVectorEqualityV1::established_v1();
        assert!(!not_reproducible.is_established_v1());
        assert!(vectors_agree.is_established_v1());
    }

    /// The toolchain's non-promises are recorded, and each says
    /// something specific enough to act on.
    #[test]
    fn the_toolchains_non_promises_are_recorded_specifically() {
        assert_eq!(TOOLCHAIN_DOES_NOT_PROMISE.len(), 3);
        for claim in TOOLCHAIN_DOES_NOT_PROMISE {
            assert!(claim.len() > 60, "too vague to act on: {claim:?}");
        }
    }
}
