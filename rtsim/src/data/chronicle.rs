//! bastion (HIST-0, row 39): the CHRONICLE — the world's PERMANENT memory,
//! the persistent player-facing twin of the ephemeral [`super::report`]
//! system (the DF "event log vs. legends" split; see
//! `readme/DF-HIST-design.md`). `Reports` decay by design (murder 15d,
//! theft 1.5d, then forgotten — the *recent-events feed*); the chronicle
//! remembers the ages: a fallen colony's ruin, a legendary death, the
//! god's deeds. This module is HIST-0's whole scope — the LOCKED schema,
//! the bounded store, and the ONE [`Chronicle::record`] entry point every
//! future emitter (HIST-1 rtsim rules AND server-side bastion systems)
//! calls, so nobody forks a private log. No emitters, no live feed, no
//! browser here.
//!
//! Retention is the LOD law applied to history (S5): each event carries an
//! importance BAND — `Routine` prunes fast on a short window, `Notable`
//! persists a long window, `Legendary` persists FOREVER — and each pruning
//! band has a hard CAP (the carrying capacity `Reports`' own TODO names),
//! so the log is bounded no matter how long the world runs.

use common::{
    resources::TimeOfDay,
    rtsim::{Actor, SiteId},
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use vek::Vec3;

/// The canonical deed vocabulary — LOCKED per the GAP-AUDIT ADDENDUM in
/// `readme/DF-HIST-design.md` (2026-07-10, architect-approved; the
/// `Purpose`-enum discipline). Extension = APPENDING variants as emitters
/// land (serde enums decode by name, so appends are save-compatible);
/// never re-shape an existing variant — chronicle entries are permanent
/// data, and per-deed detail belongs in the EVENT's fields (actors / site
/// / pos / attribution), not in kind payloads.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChronicleKind {
    // ── The original spec's core ten.
    Death,
    Theft,
    Founding,
    WarDeclared,
    Harvest,
    Masterwork,
    Famine,
    Siege,
    DivineAct,
    Birth,
    // ── Production / economy (DF-PRODUCTION, DF-TRADE, BUILD-FRAMEWORK).
    GreatWorkCompleted,
    CaravanArrived,
    CaravanLost,
    TradeDealStruck,
    // ── Faith / omens (DF-RELIGION, DF-OMEN, DF-FESTIVAL).
    TempleBuilt,
    ProphetArose,
    PrayerAnswered,
    TempleStoodEmpty,
    FestivalHeld,
    OmenSeen,
    ProphecyFulfilled,
    ProphecyFalse,
    // ── The remembering world — the 4 faces (REPUTATION, GOD-EPITHET,
    //    SACRED-SITES, COLLECTIVE-RENOWN).
    ReputationRose,
    ReputationFell,
    EpithetShifted,
    SacredSiteMade,
    SiteDesecrated,
    RenownEarned,
    ColonyBynamed,
    // ── Legendary figures — the triad (DF-VILLAIN, DIVINE-CHAMPION,
    //    DF-BEAST).
    NemesisRose,
    NemesisFell,
    ChampionAnointed,
    ChampionFell,
    BeastSlain,
    BeastNamed,
    // ── The dead (DF-ANCESTORS).
    HeroMartyred,
    AncestorVenerated,
    GhostRestless,
    SoulLaidToRest,
    // ── The divine hand (DF-CURSE, SACRED-SITES, GOD-POWERS).
    CurseLaid,
    CurseLifted,
    GeasBound,
    GeasBroken,
    Consecration,
    Miracle,
    // ── Knowledge (DF-KNOWLEDGE).
    TechDiscovered,
    KnowledgeLost,
    KnowledgeTaught,
    // ── The colony's life (DF-RECLAIM, milestones, hazards, DF-CAVERN).
    ColonyFell,
    /// First-birth / first-death / first-winter-survived /
    /// first-masterwork — the colony's firsts.
    MilestoneFirst,
    CaveIn,
    Breach,
    Flood,
    Migration,
    // ── bastion (B7-1): the sleep-quality thoughts (design §3's own
    //    list: "slept in own bed +, slept on the ground −") — appended
    //    per the lock's extension rule as their emitter landed.
    SleptInBed,
    SleptOnGround,
}

/// The importance BAND (S5) — a small canonical enum, `Purpose`-enum
/// discipline. Retention is per-band; see [`Chronicle`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Importance {
    /// Fine loaded-colony detail — feed only, prunes fast.
    Routine,
    /// The long window: wars, foundings, figures.
    Notable,
    /// Never pruned. The ages remember.
    Legendary,
}

/// Which tier RECORDED the event — the loaded↔simulated LOD law applied to
/// history: a watched colony records fine detail, the world tier records
/// coarse summaries (epochs and figures, never footsteps).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    Colony,
    World,
}

/// Divine attribution (God-Powers §1.2): how openly a god-act was signed.
/// Only god-acts carry one (`None` on mortal deeds); the faith layer later
/// FLIPS `Ambiguous` entries as revelation spreads (HIST-5) — the one
/// thread from the chronicle back into the control spectrum.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Attribution {
    Attributed,
    Ambiguous,
    Hidden,
}

/// One remembered deed — the LOCKED schema
/// `{ kind, actors, site, at_tod, importance, scope, attribution }` plus
/// the D7 spatial key: a bucketed `pos` (FR9-RESOLVED — every deed carries
/// the spot where it happened when there is one; `site` is a ROLLUP
/// derived by coarsening pos, never the other way). The D7 sphere-weight
/// field (GOD-DOMAIN) is deliberately ABSENT: no sphere vocabulary is
/// locked anywhere yet, and an `Option` field appends save-compatibly the
/// moment GOD-DOMAIN locks one — flagged to the architect at HIST-0's tag.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChronicleEvent {
    /// Monotonic capture order, stamped by [`Chronicle::record`] — stable
    /// across pruning (an append-log ordinal, usable as a cross-link key
    /// by the HIST-3 browser).
    pub seq: u64,
    pub kind: ChronicleKind,
    /// Who did it / who it happened to — the stable-ID vocabulary
    /// ([`Actor`] = NPC or character), possibly empty (a flood has no
    /// author).
    pub actors: Vec<Actor>,
    /// The site ROLLUP (derived from `pos` where both exist).
    pub site: Option<SiteId>,
    /// The bucketed world position of the deed (the canonical spatial
    /// key — scry-a-spot resolution), when it has one.
    pub pos: Option<Vec3<i32>>,
    pub at_tod: TimeOfDay,
    pub importance: Importance,
    pub scope: Scope,
    /// `Some` on god-acts only.
    pub attribution: Option<Attribution>,
}

/// Per-band carrying capacity + pruning windows. The caps bound the log no
/// matter how long the world runs; the windows are the decay every
/// accumulation must carry. Tunable constants — the BANDS are the locked
/// contract, these numbers are not.
pub const ROUTINE_CAP: usize = 512;
pub const NOTABLE_CAP: usize = 2048;
const DAYS: f64 = 60.0 * 60.0 * 24.0;
pub const ROUTINE_WINDOW: f64 = DAYS * 3.0;
pub const NOTABLE_WINDOW: f64 = DAYS * 90.0;

/// The persistent store — a new `#[serde(default)]` field on rtsim
/// [`super::Data`] (rides B10 persistence; no version bump, exactly like
/// its siblings). Internally banded so caps are O(1) at record-time:
/// bounded bands are deques (cap-evict from the front = oldest first),
/// `Legendary` is a plain append-only Vec.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Chronicle {
    next_seq: u64,
    routine: VecDeque<ChronicleEvent>,
    notable: VecDeque<ChronicleEvent>,
    legendary: Vec<ChronicleEvent>,
}

impl Chronicle {
    /// THE capture entry point — every emitter calls this (rtsim rules and
    /// server-side bastion systems alike); no system keeps a private log.
    /// O(1) amortized: stamps the ordinal + timestamp, routes to the
    /// band, and cap-evicts the band's oldest entry on overflow
    /// (`Legendary` has no cap by design). Returns the stamped `seq`.
    #[expect(clippy::too_many_arguments, reason = "the locked schema, flat")]
    pub fn record(
        &mut self,
        now: TimeOfDay,
        kind: ChronicleKind,
        actors: Vec<Actor>,
        site: Option<SiteId>,
        pos: Option<Vec3<i32>>,
        importance: Importance,
        scope: Scope,
        attribution: Option<Attribution>,
    ) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        let event = ChronicleEvent {
            seq,
            kind,
            actors,
            site,
            pos,
            at_tod: now,
            importance,
            scope,
            attribution,
        };
        match importance {
            Importance::Routine => {
                self.routine.push_back(event);
                while self.routine.len() > ROUTINE_CAP {
                    self.routine.pop_front();
                }
            },
            Importance::Notable => {
                self.notable.push_back(event);
                while self.notable.len() > NOTABLE_CAP {
                    self.notable.pop_front();
                }
            },
            Importance::Legendary => self.legendary.push(event),
        }
        seq
    }

    /// The periodic decay sweep (the [`super::report::Reports::cleanup`]
    /// shape): expire pruning-band entries past their window. `Legendary`
    /// is NEVER touched — by construction, not by tuning.
    pub fn cleanup(&mut self, now: TimeOfDay) {
        let live = |window: f64| move |e: &ChronicleEvent| (now.0 - e.at_tod.0).max(0.0) < window;
        self.routine.retain(live(ROUTINE_WINDOW));
        self.notable.retain(live(NOTABLE_WINDOW));
    }

    /// Every remembered event, banded order (routine → notable →
    /// legendary); readers sort by `seq`/`at_tod` as needed (HIST-3's
    /// concern, not the store's).
    pub fn events(&self) -> impl Iterator<Item = &ChronicleEvent> {
        self.routine
            .iter()
            .chain(self.notable.iter())
            .chain(self.legendary.iter())
    }

    /// (routine, notable, legendary) live counts — the bounded-growth
    /// probe.
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.routine.len(), self.notable.len(), self.legendary.len())
    }

    /// The store's bytes under the SAME encoder `Data` persistence uses
    /// (rmp, named) — the byte-for-byte round-trip probe. Chronicle-scoped
    /// because whole-`Data` bytes are not order-stable (slotmap/hashmap
    /// siblings), while this store's banded deques/vec are.
    pub fn fingerprint(&self) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        rmp_serde::encode::write_named(&mut bytes, self)
            .ok()
            .map(|_| bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deed(c: &mut Chronicle, tod: f64, importance: Importance) -> u64 {
        c.record(
            TimeOfDay(tod),
            ChronicleKind::Founding,
            Vec::new(),
            None,
            Some(Vec3::new(100, 200, 50)),
            importance,
            Scope::Colony,
            None,
        )
    }

    /// The carrying capacity: a soak N ≫ cap holds every pruning band at
    /// its cap; `Legendary` grows unbounded by design and is NEVER pruned
    /// — not by cap, not by any window, however far time runs.
    #[test]
    fn bounded_growth_and_legendary_immortality() {
        let mut c = Chronicle::default();
        for i in 0..(ROUTINE_CAP * 4) {
            deed(&mut c, i as f64, Importance::Routine);
        }
        for i in 0..(NOTABLE_CAP * 2) {
            deed(&mut c, i as f64, Importance::Notable);
        }
        for i in 0..64 {
            deed(&mut c, i as f64, Importance::Legendary);
        }
        assert_eq!(
            c.counts(),
            (ROUTINE_CAP, NOTABLE_CAP, 64),
            "caps must hold under soak"
        );
        // Cap-eviction is oldest-first: the survivors are the newest.
        assert!(
            c.routine
                .iter()
                .all(|e| e.at_tod.0 >= (ROUTINE_CAP * 3) as f64)
        );
        // A cleanup at the end of time expires every windowed entry —
        // and not one Legendary.
        c.cleanup(TimeOfDay(f64::MAX / 2.0));
        assert_eq!(c.counts(), (0, 0, 64), "Legendary is never pruned");
    }

    /// `seq` is a monotonic capture ordinal, stable across band routing.
    #[test]
    fn seq_monotonic_across_bands() {
        let mut c = Chronicle::default();
        let a = deed(&mut c, 0.0, Importance::Routine);
        let b = deed(&mut c, 1.0, Importance::Legendary);
        let d = deed(&mut c, 2.0, Importance::Notable);
        assert!(a < b && b < d);
        assert_eq!(c.events().count(), 3);
    }

    /// The B10 boundary in miniature: the store round-trips through the
    /// SAME encoder `Data` persistence uses (rmp, named) byte-for-byte —
    /// no entry lost, duplicated, or reordered.
    #[test]
    fn roundtrip_byte_exact() {
        let mut c = Chronicle::default();
        for i in 0..100 {
            let band = match i % 3 {
                0 => Importance::Routine,
                1 => Importance::Notable,
                _ => Importance::Legendary,
            };
            deed(&mut c, i as f64, band);
        }
        let mut bytes = Vec::new();
        rmp_serde::encode::write_named(&mut bytes, &c).expect("encode");
        let c2: Chronicle = rmp_serde::decode::from_read(bytes.as_slice()).expect("decode");
        let mut bytes2 = Vec::new();
        rmp_serde::encode::write_named(&mut bytes2, &c2).expect("re-encode");
        assert_eq!(bytes, bytes2, "byte-for-byte across the round-trip");
        assert_eq!(c.counts(), c2.counts());
    }
}
