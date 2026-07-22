//! DET-BLD-031(b) — POSITIVE CONTROL for the verify-profile guard layer.
//!
//! BLD-031(a) flipped `[profile.verify]` to `debug-assertions = true` +
//! `overflow-checks = true` so the cert lane actually runs its `debug_assert!`
//! invariant guards (double-reserve, completion-balance, decrement/drop, ECS
//! phase) and overflow panics. But "guard present in source" != "guard ran in
//! this build": a profile that silently reverts to release semantics would make
//! every guard inert AGAIN, and a guard-tripping bug would pass cert unseen —
//! the same silent-inert class as a SLOT_LOST empty-success.
//!
//! These tests are the standing POSITIVE CONTROL: they are DISCRIMINATING —
//! they PASS only under a guards-ON profile and FAIL under a guards-OFF one.
//!   * under `--profile verify` (and the default `test` profile): PASS.
//!   * under `--release` (debug-assertions off): FAIL.
//! Run them under `--profile verify` FIRST; only once they pass is a guards-on
//! cert produced under that profile trustworthy. If they ever fail under verify,
//! the guard layer is inert and the cert lane is blind — STOP.

/// Side-effect detection: a `debug_assert!` evaluates its body ONLY when
/// debug-assertions are compiled in. If `fired` stays false, the guard layer is
/// a no-op in this profile.
#[test]
fn bld031_guard_layer_is_live() {
    let mut fired = false;
    debug_assert!({
        fired = true;
        true
    });
    assert!(
        fired,
        "BLD-031 REGRESSION: debug-assertions are OFF in this profile — the \
         cert-lane guard layer is INERT. verify must set debug-assertions = true."
    );
}

/// Actual assert-fire: a deliberately-tripped `debug_assert!(false)` MUST panic
/// when guards are live. `#[should_panic]` passes iff the panic fires; under a
/// guards-off profile the assert is compiled out, no panic occurs, and this test
/// fails with "did not panic" — exactly the discrimination we want.
#[test]
#[should_panic(expected = "BLD-031 positive control")]
fn bld031_debug_assert_actually_panics() {
    debug_assert!(false, "BLD-031 positive control: the guard layer must HALT");
}

/// Overflow-checks control: BLD-031(a) also enabled `overflow-checks = true`.
/// A deliberate `u8` overflow MUST panic when overflow-checks are on. Guarded so
/// the multiply is not const-folded / optimized away.
#[test]
#[should_panic]
fn bld031_overflow_checks_are_live() {
    let x = std::hint::black_box(255u8);
    let y = std::hint::black_box(1u8);
    let _ = x + y; // panics under overflow-checks; wraps silently without them
}
