//! Scoped monotonic generation/epoch counters (`APEX-T0.4`, packet
//! section 7.3, 7.6).
//!
//! These are checked `u64` counters, never wrapping: `checked_next`
//! returns `CounterAdvanceErrorV1::Exhausted` at `u64::MAX` rather than
//! silently wrapping back to a value that has already been issued.

use super::error::{CounterAdvanceErrorV1, IdentityDecodeErrorV1};

/// `ConnectionEpoch(0)` is reserved (`INVALID`) and cannot be constructed
/// via `new` — only via the `INVALID` constant, so a caller can never
/// mistake "no epoch yet" for a real, issued epoch value.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ConnectionEpoch(u64);

impl ConnectionEpoch {
    pub const INVALID: Self = Self(0);
    pub const FIRST: Self = Self(1);

    pub fn new(value: u64) -> Result<Self, IdentityDecodeErrorV1> {
        if value == 0 {
            return Err(IdentityDecodeErrorV1::ZeroReserved);
        }
        Ok(Self(value))
    }

    pub fn checked_next(self) -> Result<Self, CounterAdvanceErrorV1> {
        self.0.checked_add(1).map(Self).ok_or(CounterAdvanceErrorV1::Exhausted)
    }

    pub const fn get(self) -> u64 { self.0 }
}

/// A counter family where zero is a legitimate initial value (owning
/// schema decides genesis/reserved-value policy, not this module).
macro_rules! zero_valid_counter {
    ($(#[$meta:meta])* $name:ident, $initial_doc:literal) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct $name(u64);

        impl $name {
            #[doc = $initial_doc]
            pub const INITIAL: Self = Self(0);

            pub const fn new(value: u64) -> Self { Self(value) }

            pub fn checked_next(self) -> Result<Self, CounterAdvanceErrorV1> {
                self.0.checked_add(1).map(Self).ok_or(CounterAdvanceErrorV1::Exhausted)
            }

            pub const fn get(self) -> u64 { self.0 }
        }
    };
}

zero_valid_counter!(
    /// Migrates the live `ForceUpdate.counter` into a typed protocol
    /// generation (owned/integrated by `T3.6`).
    PhysicsGeneration,
    "Zero is a legitimate starting generation."
);
zero_valid_counter!(
    /// A save/snapshot lineage epoch. Zero/genesis validity policy is
    /// owned by `T4`, not this module.
    SnapshotEpoch,
    "Zero-validity policy is owned by T4."
);
zero_valid_counter!(
    /// A save-store epoch. Zero/genesis validity policy is owned by `T4`.
    SaveEpoch,
    "Zero-validity policy is owned by T4."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_epoch_zero_is_reserved() {
        assert_eq!(ConnectionEpoch::new(0).unwrap_err(), IdentityDecodeErrorV1::ZeroReserved);
        assert_eq!(ConnectionEpoch::INVALID.get(), 0);
        assert_eq!(ConnectionEpoch::FIRST.get(), 1);
    }

    #[test]
    fn connection_epoch_advances_and_exhausts() {
        let one = ConnectionEpoch::new(1).unwrap();
        assert_eq!(one.checked_next().unwrap().get(), 2);
        let max = ConnectionEpoch::new(u64::MAX).unwrap();
        assert_eq!(max.checked_next().unwrap_err(), CounterAdvanceErrorV1::Exhausted);
    }

    #[test]
    fn physics_generation_zero_is_valid() {
        let zero = PhysicsGeneration::new(0);
        assert_eq!(zero.get(), 0);
        assert_eq!(zero.checked_next().unwrap().get(), 1);
        let one = PhysicsGeneration::new(1);
        assert_eq!(one.checked_next().unwrap().get(), 2);
    }

    #[test]
    fn physics_generation_exhausts() {
        let max = PhysicsGeneration::new(u64::MAX);
        assert_eq!(max.checked_next().unwrap_err(), CounterAdvanceErrorV1::Exhausted);
    }

    #[test]
    fn snapshot_and_save_epoch_zero_valid_and_checked() {
        assert_eq!(SnapshotEpoch::new(0).get(), 0);
        assert_eq!(SnapshotEpoch::new(u64::MAX).checked_next().unwrap_err(), CounterAdvanceErrorV1::Exhausted);
        assert_eq!(SaveEpoch::new(0).get(), 0);
        assert_eq!(SaveEpoch::new(u64::MAX).checked_next().unwrap_err(), CounterAdvanceErrorV1::Exhausted);
    }
}
