//! BUILD-007A10.11 (crate-side substrate) — immutable renderer extraction and
//! setup/run epoch split (design §5.1/§5.1A).
//!
//! The renderer consumes ONLY `RendererExtractSnapshotV1`, created in one named
//! `RendererExtractPhaseV1` after the authoritative tick commits (Bevy's
//! read-only ExtractSchedule pattern; Bastion schema/ordering project-owned).
//!
//! Crate-side here: the snapshot schema, canonical entity ordering, the
//! extraction root, the double-buffered publication slot ("renderer workers can
//! never observe a partially updated snapshot" — enforced by construction: the
//! slot swaps a complete immutable `Arc`, there is no field-level mutation),
//! and the `RendererSetupEpochV1` → `StartCanonicalRunV1` split (§5.1A: setup
//! latency can never alter tick count, frame zero, or capture identity).
//! The wiring into the real dispatcher after the authoritative tick is the
//! voxygen/server integration surface.

use std::sync::Arc;

use common::state_hash::{DomainHash, DomainHasher, FinalStateCertificate};

/// One extracted entity: full semantic digest + the renderer-visible canonical
/// fields (integer/fixed representation; presentation floats never enter).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererEntitySnapshotV1 {
    pub entity_digest: [u8; 32],
    pub figure_key_digest: [u8; 32],
    pub position_mm: [i64; 3],
    /// Orientation as the canonical integer representation (frozen tag + payload).
    pub orientation_q: [i32; 4],
    pub character_state_tag: u16,
    pub scale_milli: u32,
}

impl RendererEntitySnapshotV1 {
    fn encode(&self, h: &mut DomainHasher) {
        h.field(&self.entity_digest);
        h.field(&self.figure_key_digest);
        for c in self.position_mm {
            h.field(&c.to_le_bytes());
        }
        for c in self.orientation_q {
            h.field(&c.to_le_bytes());
        }
        h.field(&self.character_state_tag.to_le_bytes());
        h.field(&self.scale_milli.to_le_bytes());
    }
}

/// Weather visual snapshot (§5.1): immutable copy of authoritative weather; the
/// renderer can never write weather state back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WeatherVisualSnapshotV1 {
    pub cloud_milli: u32,
    pub rain_milli: u32,
    pub wind_mm_s: [i32; 2],
}

/// The immutable extraction snapshot (§5.1). Constructed complete via
/// [`RendererExtractSnapshotV1::build`]; entities are sorted by full semantic
/// digest at construction and the extraction root is computed once — there is
/// no post-construction mutation path.
#[derive(Clone, Debug)]
pub struct RendererExtractSnapshotV1 {
    pub run_epoch: u64,
    pub simulation_tick: u64,
    pub source_phase_tag: u16,
    pub authoritative: FinalStateCertificate,
    pub content_root: DomainHash,
    sorted_entities: Vec<RendererEntitySnapshotV1>,
    pub weather: WeatherVisualSnapshotV1,
    extraction_root: DomainHash,
}

impl RendererExtractSnapshotV1 {
    /// Build the complete snapshot: canonicalize entity order (full-digest
    /// sort), then bind every field into the extraction root. Gather-sort-
    /// commit — producer order cannot leak (§5.3, DETERMINISM IMPACT ordering).
    #[must_use]
    pub fn build(
        run_epoch: u64,
        simulation_tick: u64,
        source_phase_tag: u16,
        authoritative: FinalStateCertificate,
        content_root: DomainHash,
        mut entities: Vec<RendererEntitySnapshotV1>,
        weather: WeatherVisualSnapshotV1,
    ) -> Self {
        entities.sort_by(|a, b| a.entity_digest.cmp(&b.entity_digest));
        let mut h = DomainHasher::new("bastion/r0d/extract-snapshot/v1/sha256");
        h.field(&run_epoch.to_le_bytes());
        h.field(&simulation_tick.to_le_bytes());
        h.field(&source_phase_tag.to_le_bytes());
        h.field(&authoritative.world_seed.to_le_bytes());
        h.field(&authoritative.tick.to_le_bytes());
        h.field(&authoritative.durable_composite.0);
        h.field(&content_root.0);
        h.field(&(entities.len() as u64).to_le_bytes());
        for e in &entities {
            e.encode(&mut h);
        }
        h.field(&weather.cloud_milli.to_le_bytes());
        h.field(&weather.rain_milli.to_le_bytes());
        for c in weather.wind_mm_s {
            h.field(&c.to_le_bytes());
        }
        let extraction_root = h.finish();
        Self {
            run_epoch,
            simulation_tick,
            source_phase_tag,
            authoritative,
            content_root,
            sorted_entities: entities,
            weather,
            extraction_root,
        }
    }

    #[must_use]
    pub fn entities(&self) -> &[RendererEntitySnapshotV1] {
        &self.sorted_entities
    }

    #[must_use]
    pub fn extraction_root(&self) -> DomainHash {
        self.extraction_root
    }
}

/// Double-buffered publication slot (§5.3). The authoritative owner publishes a
/// COMPLETE immutable snapshot; renderer workers receive an `Arc` handle. A
/// partially updated snapshot is unobservable by construction — the slot swap
/// is whole-`Arc`, never field-level. Old snapshots are reclaimed when the last
/// handle drops; reclamation timing is nonsemantic.
#[derive(Debug, Default)]
pub struct SnapshotSlotV1 {
    current: Option<Arc<RendererExtractSnapshotV1>>,
}

/// Publication failures (§5.3: publication is by-tick monotonic).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishError {
    /// A snapshot for an equal-or-older tick was offered after a newer one.
    NonMonotonicTick { current: u64, offered: u64 },
}

impl SnapshotSlotV1 {
    /// Publish a complete snapshot (release semantics in the live wiring; here
    /// the whole-`Arc` swap is the same guarantee). Ticks must be monotonic.
    pub fn publish(&mut self, snap: RendererExtractSnapshotV1) -> Result<(), PublishError> {
        if let Some(cur) = &self.current {
            if snap.simulation_tick <= cur.simulation_tick {
                return Err(PublishError::NonMonotonicTick {
                    current: cur.simulation_tick,
                    offered: snap.simulation_tick,
                });
            }
        }
        self.current = Some(Arc::new(snap));
        Ok(())
    }

    /// Acquire a read handle to the latest complete snapshot. Never partial.
    #[must_use]
    pub fn acquire(&self) -> Option<Arc<RendererExtractSnapshotV1>> {
        self.current.clone()
    }
}

/// §5.1A: the setup/run epoch split. Setup (asset load, pipeline compile,
/// agreement, readiness) has no canonical camera frame and emits no scenario
/// input; once agreement + readiness pass, BOTH graphics-on and graphics-off
/// runners receive the same start token. Host setup latency cannot alter it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartCanonicalRunV1 {
    pub start_tick: u64,
    pub frame_zero_token: u64,
}

/// Setup-epoch state machine (§5.1A): readiness and agreement must BOTH pass
/// before the start token exists; a host timeout during setup is typed invalid
/// infrastructure evidence, never a semantic scenario advance/failure.
#[derive(Clone, Copy, Debug, Default)]
pub struct RendererSetupEpochV1 {
    agreement_committed: bool,
    readiness_exact: bool,
}

/// Typed setup terminal (§5.1A).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupError {
    /// `R0D_INVALID_EVIDENCE_INFRA_TIMEOUT`: infrastructure, not semantics.
    InfraTimeout,
    /// Start requested before agreement + exact readiness both passed.
    NotReady { agreement: bool, readiness: bool },
}

impl RendererSetupEpochV1 {
    pub fn record_agreement_committed(&mut self) {
        self.agreement_committed = true;
    }

    pub fn record_readiness_exact(&mut self) {
        self.readiness_exact = true;
    }

    /// Issue the canonical start token (§5.1A). The token is a pure function of
    /// the DECLARED start tick — setup duration appears nowhere, so worker/GPU
    /// latency cannot alter tick count, scenario phase, camera frame zero, or
    /// capture identity.
    pub fn start(&self, declared_start_tick: u64) -> Result<StartCanonicalRunV1, SetupError> {
        if !(self.agreement_committed && self.readiness_exact) {
            return Err(SetupError::NotReady {
                agreement: self.agreement_committed,
                readiness: self.readiness_exact,
            });
        }
        Ok(StartCanonicalRunV1 {
            start_tick: declared_start_tick,
            frame_zero_token: declared_start_tick, // frame zero is tick-anchored, not wall-anchored
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::state_hash::IntegrityHash;

    fn cert(tick: u64) -> FinalStateCertificate {
        FinalStateCertificate {
            schema: "bastion/final-state-certificate/v1".to_string(),
            world_seed: 7,
            tick,
            durable_composite: DomainHash([1; 32]),
            rebuildable_integrity: IntegrityHash([0; 32]),
        }
    }

    fn ent(d: u8) -> RendererEntitySnapshotV1 {
        RendererEntitySnapshotV1 {
            entity_digest: [d; 32],
            figure_key_digest: [d; 32],
            position_mm: [i64::from(d), 0, 0],
            orientation_q: [0, 0, 0, 1 << 30],
            character_state_tag: 1,
            scale_milli: 1000,
        }
    }

    fn snap(tick: u64, ents: Vec<RendererEntitySnapshotV1>) -> RendererExtractSnapshotV1 {
        RendererExtractSnapshotV1::build(1, tick, 3, cert(tick), DomainHash([2; 32]), ents, WeatherVisualSnapshotV1::default())
    }

    #[test]
    fn producer_order_cannot_leak_into_extraction_root() {
        let a = snap(10, vec![ent(3), ent(1), ent(2)]);
        let b = snap(10, vec![ent(1), ent(2), ent(3)]);
        assert_eq!(a.extraction_root(), b.extraction_root());
        // Entities exposed in canonical full-digest order.
        let order: Vec<u8> = a.entities().iter().map(|e| e.entity_digest[0]).collect();
        assert_eq!(order, vec![1, 2, 3]);
    }

    #[test]
    fn extraction_root_is_field_sensitive() {
        let a = snap(10, vec![ent(1)]);
        let mut moved = ent(1);
        moved.position_mm = [999, 0, 0];
        let b = snap(10, vec![moved]);
        assert_ne!(a.extraction_root(), b.extraction_root());
    }

    #[test]
    fn slot_swaps_whole_snapshots_and_rejects_nonmonotonic_ticks() {
        let mut slot = SnapshotSlotV1::default();
        assert!(slot.acquire().is_none());
        slot.publish(snap(10, vec![ent(1)])).unwrap();
        let h1 = slot.acquire().unwrap();
        assert_eq!(h1.simulation_tick, 10);
        // Publishing tick 11 does not disturb the held handle (old snapshot
        // lives until the reader drops it — reclamation nonsemantic).
        slot.publish(snap(11, vec![ent(1), ent(2)])).unwrap();
        assert_eq!(h1.simulation_tick, 10, "held handle immutable");
        assert_eq!(slot.acquire().unwrap().simulation_tick, 11);
        // Equal or older tick is a typed publication failure.
        assert_eq!(
            slot.publish(snap(11, vec![ent(1)])),
            Err(PublishError::NonMonotonicTick { current: 11, offered: 11 })
        );
    }

    #[test]
    fn start_token_requires_agreement_and_readiness_and_ignores_latency() {
        let mut setup = RendererSetupEpochV1::default();
        assert_eq!(
            setup.start(100),
            Err(SetupError::NotReady { agreement: false, readiness: false })
        );
        setup.record_agreement_committed();
        assert!(setup.start(100).is_err(), "agreement alone insufficient");
        setup.record_readiness_exact();
        let t = setup.start(100).unwrap();
        assert_eq!(t, StartCanonicalRunV1 { start_tick: 100, frame_zero_token: 100 });
        // The token is a pure function of the declared tick — issuing it again
        // (any amount of setup latency later) yields the identical token.
        assert_eq!(setup.start(100).unwrap(), t);
    }
}
