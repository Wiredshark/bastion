pub mod airship;
pub mod architect;
pub mod chronicle;
pub mod faction;
pub mod nature;
pub mod npc;
pub mod quest;
pub mod report;
pub mod sentiment;
pub mod site;

pub use self::{
    chronicle::{Attribution, Chronicle, ChronicleEvent, ChronicleKind, Importance, Scope},
    faction::{Faction, FactionId, Factions},
    nature::Nature,
    npc::{Npc, NpcId, Npcs},
    quest::Quests,
    report::{KnownReports, Report, ReportId, ReportKind, Reports},
    sentiment::{Sentiment, Sentiments},
    site::{Site, SiteId, Sites},
};
use airship::AirshipSim;
use architect::Architect;
use common::resources::TimeOfDay;
use enum_map::{EnumArray, EnumMap, enum_map};
use serde::{Deserialize, Serialize, de, ser};
use std::{
    cmp::PartialEq,
    fmt,
    io::{Read, Write},
    marker::PhantomData,
};

/// The current version of rtsim data.
///
/// Note that this number does *not* need incrementing on every change: most
/// field removals/additions are fine. This number should only be incremented
/// when we wish to perform a *hard purge* of rtsim data.
pub const CURRENT_VERSION: u32 = 10;

#[derive(Clone, Serialize, Deserialize)]
pub struct Data {
    // Absence of field just implied version = 0
    #[serde(default)]
    pub version: u32,

    pub nature: Nature,
    #[serde(default)]
    pub npcs: Npcs,
    #[serde(default)]
    pub sites: Sites,
    #[serde(default)]
    pub factions: Factions,
    #[serde(default)]
    pub reports: Reports,
    #[serde(default)]
    pub architect: Architect,
    #[serde(default)]
    pub quests: Quests,
    /// T0.49 (master build order; T0-003): the persistent item-instance
    /// allocator — lives in THE authoritative world save (sibling
    /// `#[serde(default)]` pattern, no version bump).
    #[serde(default)]
    pub item_instance_allocator: ItemInstanceAllocator,
    /// bastion (HIST-0): the world's PERMANENT memory — the persistent
    /// twin of the fading `reports` feed (see [`chronicle`]). Sibling
    /// pattern: `#[serde(default)]`, no version bump.
    #[serde(default)]
    pub chronicle: Chronicle,

    /// bastion (IDLE-HOME-LEASH): the colony's idle-orbit anchor — the first
    /// stockpile's centroid, overridden by a painted Meeting zone (explicit
    /// beats implicit). EPHEMERAL: recomputed every server tick by the rtsim
    /// bridge from live designations; `#[serde(skip)]` = never persisted,
    /// `None` on load / when no stockpile or Meeting zone exists (leash
    /// inactive).
    #[serde(skip)]
    pub bastion_home_anchor: Option<vek::Vec3<f32>>,

    #[serde(default)]
    pub tick: u64,
    #[serde(default)]
    pub time_of_day: TimeOfDay,

    // If true, rtsim data will be ignored (and, hence, overwritten on next save) on load.
    #[serde(default)]
    pub should_purge: bool,

    #[serde(skip)]
    pub airship_sim: AirshipSim,
}

pub enum ReadError {
    Load(rmp_serde::decode::Error),
    // Preserve old data
    VersionMismatch(Box<Data>),
}

impl fmt::Debug for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Load(err) => err.fmt(f),
            Self::VersionMismatch(_) => write!(f, "VersionMismatch"),
        }
    }
}

pub type WriteError = rmp_serde::encode::Error;

impl Data {
    pub fn spawn_npc(&mut self, npc: Npc) -> NpcId {
        let home = npc.home;
        let id = self.npcs.create_npc(npc);
        if let Some(home) = home.and_then(|home| self.sites.get_mut(home)) {
            home.population.insert(id);
        }
        id
    }

    pub fn from_reader<R: Read>(reader: R) -> Result<Box<Self>, ReadError> {
        rmp_serde::decode::from_read(reader)
            .map_err(ReadError::Load)
            .and_then(|data: Data| {
                if data.version == CURRENT_VERSION {
                    Ok(Box::new(data))
                } else {
                    Err(ReadError::VersionMismatch(Box::new(data)))
                }
            })
    }

    pub fn write_to<W: Write>(&self, mut writer: W) -> Result<(), WriteError> {
        rmp_serde::encode::write_named(&mut writer, self)
    }

    /// Perform whatever initial preparation is required for rtsim data to be
    /// ready for simulation.
    ///
    /// This might include populating caches, normalising data, etc.
    pub fn prepare(&mut self) { self.quests.prepare(); }
}

fn rugged_ser_enum_map<
    K: EnumArray<V> + Serialize,
    V: From<i16> + PartialEq + Serialize,
    S: ser::Serializer,
    const DEFAULT: i16,
>(
    map: &EnumMap<K, V>,
    ser: S,
) -> Result<S::Ok, S::Error> {
    ser.collect_map(map.iter().filter(|(_, v)| v != &&V::from(DEFAULT)))
}

fn rugged_de_enum_map<
    'a,
    K: EnumArray<V> + EnumArray<Option<V>> + Deserialize<'a>,
    V: From<i16> + Deserialize<'a>,
    D: de::Deserializer<'a>,
    const DEFAULT: i16,
>(
    de: D,
) -> Result<EnumMap<K, V>, D::Error> {
    struct Visitor<K, V, const DEFAULT: i16>(PhantomData<(K, V)>);

    impl<'de, K, V, const DEFAULT: i16> de::Visitor<'de> for Visitor<K, V, DEFAULT>
    where
        K: EnumArray<V> + EnumArray<Option<V>> + Deserialize<'de>,
        V: From<i16> + Deserialize<'de>,
    {
        type Value = EnumMap<K, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a map")
        }

        fn visit_map<M: de::MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            let mut entries = EnumMap::default();
            while let Some((key, value)) = access.next_entry()? {
                entries[key] = Some(value);
            }
            Ok(enum_map! { key => entries[key].take().unwrap_or_else(|| V::from(DEFAULT)) })
        }
    }

    de.deserialize_map(Visitor::<_, _, DEFAULT>(PhantomData))
}


/// T0.49: the per-world item-instance identity allocator. The namespace is
/// a ONE-TIME nonce minted at world creation (OS entropy is fine here — the
/// packet rejects randomness as ongoing PRIMARY identity, not as a one-time
/// per-world seed component; two saves sharing a worldgen seed must not
/// alias instance ids). The sequence is a single persisted monotonic
/// counter, incremented only at the authoritative creation commit
/// (`create_item_drop`); full retry-safe RANGE reservation is Tier-1
/// transaction scope (T1.17/T1.24), deliberately not built here.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemInstanceAllocator {
    pub world_namespace: u64,
    pub next_sequence: u64,
}

impl Default for ItemInstanceAllocator {
    fn default() -> Self {
        Self {
            world_namespace: rand::random::<u64>(),
            next_sequence: 0,
        }
    }
}

impl ItemInstanceAllocator {
    /// Allocate the next instance identity — synchronous at the
    /// non-yielding construction point, so no reserve-without-commit gap.
    pub fn allocate(&mut self) -> common::comp::item::ItemInstanceId {
        let id = common::comp::item::ItemInstanceId {
            world_namespace: self.world_namespace,
            creation_sequence: self.next_sequence,
        };
        self.next_sequence += 1;
        id
    }
}

// T0.49: monotonicity + namespace-constancy pin.
#[cfg(test)]
mod t0_49_tests {
    use super::ItemInstanceAllocator;

    #[test]
    fn t0_49_allocator_is_monotonic_within_a_constant_namespace() {
        let mut alloc = ItemInstanceAllocator {
            world_namespace: 7,
            next_sequence: 0,
        };
        let a = alloc.allocate();
        let b = alloc.allocate();
        assert_eq!(a.world_namespace, 7);
        assert_eq!(b.world_namespace, 7);
        assert_eq!(a.creation_sequence, 0);
        assert_eq!(b.creation_sequence, 1);
        assert!(a < b, "identity is totally ordered by (namespace, sequence)");
    }
}

#[cfg(test)]
mod det_rng_009_gate {
    //! DET-RNG-009 (determinism audit) — the persisted-RNG-cursor gate.
    //!
    //! Confirmed moot by construction: all authoritative sim RNG is
    //! re-derived every tick from `crate::tick_rng(world_seed, tick, salt)`
    //! (counter-RNG), so nothing carries stream state across a save/reload —
    //! there is no cursor to fork. This gate PINS that: no field of a
    //! persisted rtsim `data` struct may be an RNG cursor type (which WOULD
    //! serialize/restore a stream position and silently fork post-reload).
    //! The only live RNG values are the per-tick `NpcCtx` fields in
    //! `ai/mod.rs` — a transient context, never persisted, out of scope.

    #[test]
    fn no_persisted_rng_cursor_in_rtsim_data() {
        // Every source file that defines serialized rtsim `data` state.
        let sources: &[&str] = &[
            include_str!("mod.rs"),
            include_str!("npc.rs"),
            include_str!("site.rs"),
            include_str!("faction.rs"),
            include_str!("nature.rs"),
            include_str!("sentiment.rs"),
            include_str!("quest.rs"),
            include_str!("report.rs"),
            include_str!("chronicle.rs"),
            include_str!("architect.rs"),
            include_str!("airship.rs"),
        ];
        // RNG cursor types that would carry a stream position across a save.
        // Needles are built at RUNTIME so this test's own source (scanned via
        // include_str!("mod.rs")) never self-matches.
        for ty in ["ChaChaRng", "ChaCha20Rng", "StdRng", "SmallRng", "Pcg64", "Pcg32"] {
            let field_needle = format!(": {ty}");
            for src in sources {
                assert_eq!(
                    src.matches(&field_needle).count(),
                    0,
                    "a persisted RNG cursor field (`{field_needle}`) appeared in rtsim \
                     `data` — authoritative sim RNG must be re-derived per tick from \
                     tick_rng (counter-RNG), never STORED, or it forks across save/reload \
                     (DET-RNG-009). If this is genuinely a transient non-persisted field, \
                     move it off the data struct."
                );
            }
        }
    }
}
