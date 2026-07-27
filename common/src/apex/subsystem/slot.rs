//! Frozen subsystem-slot vocabulary (`APEX-T0.5`, spec section 3.1).
//!
//! Same shape as [`crate::apex::digest::DigestDomainIdV1`]: a closed,
//! `u16`-tagged enum with an explicit numeric discriminant per variant
//! (never derived from declaration order), an `ALL` const array, and a
//! self-test proving no duplicate ID or label. Not an opaque UUID
//! (`opaque_lifecycle_id!`) — slot identity is build-time-frozen
//! vocabulary, not a runtime-generated instance identity.

/// A registered subsystem slot. Every variant traces to a named consumer
/// row in the registry (`APEX-T4.1`, `APEX-T4.3{a,b}`, `APEX-T4.4`,
/// `APEX-T6.1`) — no slot is invented speculatively.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SubsystemSlotIdV1 {
    /// `APEX-T4.3`: world seed / worldgen protocol root.
    Worldgen = 1,
    /// `APEX-T4.1`/`APEX-T4.3`: content protocol root, `ContentIdentityV1` reuse.
    Content = 2,
    /// `APEX-T4.3`/`APEX-T6.1`: numeric protocol root, numeric attack surface.
    Numeric = 3,
    /// `APEX-T4.1`: schedule identity.
    Schedule = 4,
    /// `APEX-T4.1`: plugin activation plan.
    Plugin = 5,
    /// `APEX-T4.3b`: economy baseline root.
    Economy = 6,
    /// `APEX-T4.4`: non-authoritative existing-save inventory.
    SaveInventory = 7,
    /// `APEX-T4.1`: build identity.
    Build = 8,
}

impl SubsystemSlotIdV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Worldgen => "bastion/subsystem-slot/worldgen/v1",
            Self::Content => "bastion/subsystem-slot/content/v1",
            Self::Numeric => "bastion/subsystem-slot/numeric/v1",
            Self::Schedule => "bastion/subsystem-slot/schedule/v1",
            Self::Plugin => "bastion/subsystem-slot/plugin/v1",
            Self::Economy => "bastion/subsystem-slot/economy/v1",
            Self::SaveInventory => "bastion/subsystem-slot/save-inventory/v1",
            Self::Build => "bastion/subsystem-slot/build/v1",
        }
    }

    pub const ALL: [SubsystemSlotIdV1; 8] = [
        Self::Worldgen,
        Self::Content,
        Self::Numeric,
        Self::Schedule,
        Self::Plugin,
        Self::Economy,
        Self::SaveInventory,
        Self::Build,
    ];

    pub fn try_from_u16(raw: u16) -> Option<Self> { Self::ALL.into_iter().find(|s| s.as_u16() == raw) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registry_ids_and_labels_are_unique() {
        let ids: HashSet<u16> = SubsystemSlotIdV1::ALL.iter().map(|s| s.as_u16()).collect();
        assert_eq!(ids.len(), SubsystemSlotIdV1::ALL.len(), "duplicate slot ID");
        let labels: HashSet<&str> = SubsystemSlotIdV1::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(labels.len(), SubsystemSlotIdV1::ALL.len(), "duplicate slot label");
    }

    #[test]
    fn labels_are_ascii() {
        for s in SubsystemSlotIdV1::ALL {
            assert!(s.label().is_ascii(), "{:?} label must be ASCII", s);
        }
    }

    #[test]
    fn exact_registered_table() {
        assert_eq!(SubsystemSlotIdV1::Worldgen.as_u16(), 1);
        assert_eq!(SubsystemSlotIdV1::Content.as_u16(), 2);
        assert_eq!(SubsystemSlotIdV1::Numeric.as_u16(), 3);
        assert_eq!(SubsystemSlotIdV1::Schedule.as_u16(), 4);
        assert_eq!(SubsystemSlotIdV1::Plugin.as_u16(), 5);
        assert_eq!(SubsystemSlotIdV1::Economy.as_u16(), 6);
        assert_eq!(SubsystemSlotIdV1::SaveInventory.as_u16(), 7);
        assert_eq!(SubsystemSlotIdV1::Build.as_u16(), 8);
    }

    #[test]
    fn try_from_u16_round_trips_and_rejects_unknown() {
        for s in SubsystemSlotIdV1::ALL {
            assert_eq!(SubsystemSlotIdV1::try_from_u16(s.as_u16()), Some(s));
        }
        assert_eq!(SubsystemSlotIdV1::try_from_u16(0), None);
        assert_eq!(SubsystemSlotIdV1::try_from_u16(9), None);
        assert_eq!(SubsystemSlotIdV1::try_from_u16(u16::MAX), None);
    }

    #[test]
    fn ord_matches_tag_order_not_declaration_order() {
        let mut shuffled = vec![SubsystemSlotIdV1::Build, SubsystemSlotIdV1::Worldgen, SubsystemSlotIdV1::Numeric];
        shuffled.sort();
        assert_eq!(shuffled, vec![SubsystemSlotIdV1::Worldgen, SubsystemSlotIdV1::Numeric, SubsystemSlotIdV1::Build]);
    }
}
