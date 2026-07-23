//! Immutable, renderer-owned extraction snapshots and setup-to-run transition.
//!
//! This module consumes only explicit digests and integer renderer fields. It
//! has no simulation-owned dependency and exposes no writeback path.

use std::sync::Arc;

use crate::{
    DomainHashErrorV1, bootstrap::V1_TICK_CAP, domain_hash_v1, identity::MAX_RENDERER_ENTITIES_V1,
};

pub const MAX_ABS_POSITION_MM_V1: i64 = 9_000_000_000_000;
pub const MAX_ABS_ORIENTATION_COMPONENT_V1: i32 = 1 << 30;
pub const MAX_SCALE_MILLI_V1: u32 = 100_000;
pub const MAX_WEATHER_MILLI_V1: u32 = 1_000;
pub const MAX_ABS_WIND_MM_S_V1: i32 = 1_000_000;
pub const MAX_SOURCE_PHASE_TAG_V1: u16 = 4_095;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererEntitySnapshotV1 {
    pub semantic_digest: [u8; 32],
    pub figure_key_digest: [u8; 32],
    pub position_mm: [i64; 3],
    pub orientation_q30: [i32; 4],
    pub character_state_tag: u16,
    pub scale_milli: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RendererWeatherSnapshotV1 {
    pub cloud_milli: u32,
    pub rain_milli: u32,
    pub wind_mm_s: [i32; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtractionErrorV1 {
    InvalidRunEpoch,
    InvalidSimulationTick(u64),
    InvalidSourcePhase(u16),
    TooManyEntities { count: usize, cap: usize },
    DuplicateSemanticDigest([u8; 32]),
    PositionOutOfRange,
    OrientationOutOfRange,
    InvalidOrientation,
    InvalidCharacterState(u16),
    InvalidScale(u32),
    InvalidWeather,
    InvalidWind,
    SizeOverflow,
    AllocationFailure,
    HashFailure(DomainHashErrorV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererExtractSnapshotV1 {
    run_epoch: u64,
    simulation_tick: u64,
    source_phase_tag: u16,
    authoritative_state_digest: [u8; 32],
    content_root: [u8; 32],
    entities: Vec<RendererEntitySnapshotV1>,
    weather: RendererWeatherSnapshotV1,
    extraction_root: [u8; 32],
}

impl RendererExtractSnapshotV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        run_epoch: u64,
        simulation_tick: u64,
        source_phase_tag: u16,
        authoritative_state_digest: [u8; 32],
        content_root: [u8; 32],
        mut entities: Vec<RendererEntitySnapshotV1>,
        weather: RendererWeatherSnapshotV1,
    ) -> Result<Self, ExtractionErrorV1> {
        if run_epoch == 0 {
            return Err(ExtractionErrorV1::InvalidRunEpoch);
        }
        if simulation_tick >= V1_TICK_CAP {
            return Err(ExtractionErrorV1::InvalidSimulationTick(simulation_tick));
        }
        if source_phase_tag == 0 || source_phase_tag > MAX_SOURCE_PHASE_TAG_V1 {
            return Err(ExtractionErrorV1::InvalidSourcePhase(source_phase_tag));
        }
        if entities.len() > MAX_RENDERER_ENTITIES_V1 {
            return Err(ExtractionErrorV1::TooManyEntities {
                count: entities.len(),
                cap: MAX_RENDERER_ENTITIES_V1,
            });
        }
        validate_weather(weather)?;
        for entity in &entities {
            validate_entity(entity)?;
        }
        entities.sort_unstable_by_key(|entity| entity.semantic_digest);
        if let Some(duplicate) = entities
            .windows(2)
            .find(|pair| pair[0].semantic_digest == pair[1].semantic_digest)
        {
            return Err(ExtractionErrorV1::DuplicateSemanticDigest(
                duplicate[0].semantic_digest,
            ));
        }

        let entity_bytes = entities
            .len()
            .checked_mul(110)
            .ok_or(ExtractionErrorV1::SizeOverflow)?;
        let capacity = 8_usize
            .checked_add(8)
            .and_then(|value| value.checked_add(2))
            .and_then(|value| value.checked_add(32 + 32 + 8))
            .and_then(|value| value.checked_add(entity_bytes))
            .and_then(|value| value.checked_add(4 + 4 + 4 + 4))
            .ok_or(ExtractionErrorV1::SizeOverflow)?;
        let mut payload = Vec::new();
        payload
            .try_reserve_exact(capacity)
            .map_err(|_| ExtractionErrorV1::AllocationFailure)?;
        payload.extend_from_slice(&run_epoch.to_le_bytes());
        payload.extend_from_slice(&simulation_tick.to_le_bytes());
        payload.extend_from_slice(&source_phase_tag.to_le_bytes());
        payload.extend_from_slice(&authoritative_state_digest);
        payload.extend_from_slice(&content_root);
        payload.extend_from_slice(
            &u64::try_from(entities.len())
                .map_err(|_| ExtractionErrorV1::SizeOverflow)?
                .to_le_bytes(),
        );
        for entity in &entities {
            encode_entity(entity, &mut payload);
        }
        payload.extend_from_slice(&weather.cloud_milli.to_le_bytes());
        payload.extend_from_slice(&weather.rain_milli.to_le_bytes());
        for component in weather.wind_mm_s {
            payload.extend_from_slice(&component.to_le_bytes());
        }
        let extraction_root = domain_hash_v1("bastion/r0d/extraction-snapshot", 1, 0, &payload)
            .map_err(ExtractionErrorV1::HashFailure)?;

        Ok(Self {
            run_epoch,
            simulation_tick,
            source_phase_tag,
            authoritative_state_digest,
            content_root,
            entities,
            weather,
            extraction_root,
        })
    }

    #[must_use]
    pub const fn run_epoch(&self) -> u64 { self.run_epoch }

    #[must_use]
    pub const fn simulation_tick(&self) -> u64 { self.simulation_tick }

    #[must_use]
    pub const fn source_phase_tag(&self) -> u16 { self.source_phase_tag }

    #[must_use]
    pub const fn authoritative_state_digest(&self) -> [u8; 32] { self.authoritative_state_digest }

    #[must_use]
    pub const fn content_root(&self) -> [u8; 32] { self.content_root }

    #[must_use]
    pub fn entities(&self) -> &[RendererEntitySnapshotV1] { &self.entities }

    #[must_use]
    pub const fn weather(&self) -> RendererWeatherSnapshotV1 { self.weather }

    #[must_use]
    pub const fn extraction_root(&self) -> [u8; 32] { self.extraction_root }
}

fn validate_entity(entity: &RendererEntitySnapshotV1) -> Result<(), ExtractionErrorV1> {
    if entity
        .position_mm
        .iter()
        .copied()
        .any(|component| component < -MAX_ABS_POSITION_MM_V1 || component > MAX_ABS_POSITION_MM_V1)
    {
        return Err(ExtractionErrorV1::PositionOutOfRange);
    }
    if entity.orientation_q30.iter().copied().any(|component| {
        component < -MAX_ABS_ORIENTATION_COMPONENT_V1
            || component > MAX_ABS_ORIENTATION_COMPONENT_V1
    }) {
        return Err(ExtractionErrorV1::OrientationOutOfRange);
    }
    if entity.orientation_q30 == [0; 4] {
        return Err(ExtractionErrorV1::InvalidOrientation);
    }
    if entity.character_state_tag == 0 {
        return Err(ExtractionErrorV1::InvalidCharacterState(0));
    }
    if entity.scale_milli == 0 || entity.scale_milli > MAX_SCALE_MILLI_V1 {
        return Err(ExtractionErrorV1::InvalidScale(entity.scale_milli));
    }
    Ok(())
}

fn validate_weather(weather: RendererWeatherSnapshotV1) -> Result<(), ExtractionErrorV1> {
    if weather.cloud_milli > MAX_WEATHER_MILLI_V1 || weather.rain_milli > MAX_WEATHER_MILLI_V1 {
        return Err(ExtractionErrorV1::InvalidWeather);
    }
    if weather
        .wind_mm_s
        .iter()
        .copied()
        .any(|component| component < -MAX_ABS_WIND_MM_S_V1 || component > MAX_ABS_WIND_MM_S_V1)
    {
        return Err(ExtractionErrorV1::InvalidWind);
    }
    Ok(())
}

fn encode_entity(entity: &RendererEntitySnapshotV1, output: &mut Vec<u8>) {
    output.extend_from_slice(&entity.semantic_digest);
    output.extend_from_slice(&entity.figure_key_digest);
    for component in entity.position_mm {
        output.extend_from_slice(&component.to_le_bytes());
    }
    for component in entity.orientation_q30 {
        output.extend_from_slice(&component.to_le_bytes());
    }
    output.extend_from_slice(&entity.character_state_tag.to_le_bytes());
    output.extend_from_slice(&entity.scale_milli.to_le_bytes());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishErrorV1 {
    StaleOrEqual {
        current_run_epoch: u64,
        current_tick: u64,
        offered_run_epoch: u64,
        offered_tick: u64,
    },
}

#[derive(Debug, Default)]
pub struct RendererSnapshotSlotV1 {
    current: Option<Arc<RendererExtractSnapshotV1>>,
}

impl RendererSnapshotSlotV1 {
    pub fn publish(&mut self, snapshot: RendererExtractSnapshotV1) -> Result<(), PublishErrorV1> {
        if let Some(current) = &self.current {
            let current_key = (current.run_epoch, current.simulation_tick);
            let offered_key = (snapshot.run_epoch, snapshot.simulation_tick);
            if offered_key <= current_key {
                return Err(PublishErrorV1::StaleOrEqual {
                    current_run_epoch: current.run_epoch,
                    current_tick: current.simulation_tick,
                    offered_run_epoch: snapshot.run_epoch,
                    offered_tick: snapshot.simulation_tick,
                });
            }
        }
        self.current = Some(Arc::new(snapshot));
        Ok(())
    }

    #[must_use]
    pub fn acquire(&self) -> Option<Arc<RendererExtractSnapshotV1>> { self.current.clone() }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RendererSetupEpochV1 {
    agreement_committed: bool,
    readiness_committed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupErrorV1 {
    NotReady {
        agreement_committed: bool,
        readiness_committed: bool,
    },
    InvalidRunEpoch,
    InvalidStartTick(u64),
    HashFailure(DomainHashErrorV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StartCanonicalRunV1 {
    pub run_epoch: u64,
    pub start_tick: u64,
    pub frame_zero_token: [u8; 32],
}

impl RendererSetupEpochV1 {
    pub fn commit_agreement(&mut self) { self.agreement_committed = true; }

    pub fn commit_readiness(&mut self) { self.readiness_committed = true; }

    pub fn start(
        &self,
        run_epoch: u64,
        declared_start_tick: u64,
    ) -> Result<StartCanonicalRunV1, SetupErrorV1> {
        if !(self.agreement_committed && self.readiness_committed) {
            return Err(SetupErrorV1::NotReady {
                agreement_committed: self.agreement_committed,
                readiness_committed: self.readiness_committed,
            });
        }
        if run_epoch == 0 {
            return Err(SetupErrorV1::InvalidRunEpoch);
        }
        if declared_start_tick >= V1_TICK_CAP {
            return Err(SetupErrorV1::InvalidStartTick(declared_start_tick));
        }
        let mut payload = [0_u8; 16];
        payload[..8].copy_from_slice(&run_epoch.to_le_bytes());
        payload[8..].copy_from_slice(&declared_start_tick.to_le_bytes());
        let frame_zero_token = domain_hash_v1("bastion/r0d/frame-zero", 1, 0, &payload)
            .map_err(SetupErrorV1::HashFailure)?;
        Ok(StartCanonicalRunV1 {
            run_epoch,
            start_tick: declared_start_tick,
            frame_zero_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    fn entity(byte: u8) -> RendererEntitySnapshotV1 {
        RendererEntitySnapshotV1 {
            semantic_digest: [byte; 32],
            figure_key_digest: [byte.wrapping_add(10); 32],
            position_mm: [i64::from(byte), -i64::from(byte), 0],
            orientation_q30: [0, 0, 0, 1 << 30],
            character_state_tag: 1,
            scale_milli: 1_000,
        }
    }

    fn weather() -> RendererWeatherSnapshotV1 {
        RendererWeatherSnapshotV1 {
            cloud_milli: 200,
            rain_milli: 300,
            wind_mm_s: [-400, 500],
        }
    }

    fn snapshot(
        run_epoch: u64,
        tick: u64,
        entities: Vec<RendererEntitySnapshotV1>,
    ) -> Result<RendererExtractSnapshotV1, ExtractionErrorV1> {
        RendererExtractSnapshotV1::build(run_epoch, tick, 3, [1; 32], [2; 32], entities, weather())
    }

    #[test]
    fn producer_order_is_irrelevant_and_root_is_frozen() {
        let a = snapshot(1, 10, vec![entity(3), entity(1), entity(2)]).unwrap();
        let b = snapshot(1, 10, vec![entity(2), entity(3), entity(1)]).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a.entities()
                .iter()
                .map(|value| value.semantic_digest[0])
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            hex_bytes(&a.extraction_root()),
            "018a25924eede8b7bcf5321c961514a97be1f279c12e2171220c31a96f4f2f29"
        );
    }

    #[test]
    fn duplicate_semantic_identity_is_rejected() {
        assert_eq!(
            snapshot(1, 10, vec![entity(1), entity(1)]),
            Err(ExtractionErrorV1::DuplicateSemanticDigest([1; 32]))
        );
    }

    #[test]
    fn every_snapshot_field_changes_the_root() {
        let base = snapshot(1, 10, vec![entity(1)]).unwrap();
        let root = base.extraction_root();
        assert_ne!(
            snapshot(2, 10, vec![entity(1)]).unwrap().extraction_root(),
            root
        );
        assert_ne!(
            snapshot(1, 11, vec![entity(1)]).unwrap().extraction_root(),
            root
        );
        assert_ne!(
            RendererExtractSnapshotV1::build(
                1,
                10,
                4,
                [1; 32],
                [2; 32],
                vec![entity(1)],
                weather()
            )
            .unwrap()
            .extraction_root(),
            root
        );
        assert_ne!(
            RendererExtractSnapshotV1::build(
                1,
                10,
                3,
                [3; 32],
                [2; 32],
                vec![entity(1)],
                weather()
            )
            .unwrap()
            .extraction_root(),
            root
        );
        assert_ne!(
            RendererExtractSnapshotV1::build(
                1,
                10,
                3,
                [1; 32],
                [3; 32],
                vec![entity(1)],
                weather()
            )
            .unwrap()
            .extraction_root(),
            root
        );
        let mut variants = Vec::new();
        let mut changed = entity(1);
        changed.semantic_digest = [9; 32];
        variants.push(changed);
        let mut changed = entity(1);
        changed.figure_key_digest = [9; 32];
        variants.push(changed);
        let mut changed = entity(1);
        changed.position_mm[0] += 1;
        variants.push(changed);
        let mut changed = entity(1);
        changed.orientation_q30[0] += 1;
        variants.push(changed);
        let mut changed = entity(1);
        changed.character_state_tag += 1;
        variants.push(changed);
        let mut changed = entity(1);
        changed.scale_milli += 1;
        variants.push(changed);
        for changed in variants {
            assert_ne!(
                snapshot(1, 10, vec![changed]).unwrap().extraction_root(),
                root
            );
        }
        let mut changed_weather = weather();
        changed_weather.cloud_milli += 1;
        assert_ne!(
            RendererExtractSnapshotV1::build(
                1,
                10,
                3,
                [1; 32],
                [2; 32],
                vec![entity(1)],
                changed_weather
            )
            .unwrap()
            .extraction_root(),
            root
        );
        changed_weather = weather();
        changed_weather.rain_milli += 1;
        assert_ne!(
            RendererExtractSnapshotV1::build(
                1,
                10,
                3,
                [1; 32],
                [2; 32],
                vec![entity(1)],
                changed_weather
            )
            .unwrap()
            .extraction_root(),
            root
        );
        changed_weather = weather();
        changed_weather.wind_mm_s[0] += 1;
        assert_ne!(
            RendererExtractSnapshotV1::build(
                1,
                10,
                3,
                [1; 32],
                [2; 32],
                vec![entity(1)],
                changed_weather
            )
            .unwrap()
            .extraction_root(),
            root
        );
    }

    #[test]
    fn snapshot_bounds_and_ranges_fail_closed() {
        assert_eq!(
            snapshot(0, 0, vec![]),
            Err(ExtractionErrorV1::InvalidRunEpoch)
        );
        assert_eq!(
            RendererExtractSnapshotV1::build(1, 0, 0, [1; 32], [2; 32], vec![], weather()),
            Err(ExtractionErrorV1::InvalidSourcePhase(0))
        );
        assert_eq!(
            snapshot(1, V1_TICK_CAP, vec![]),
            Err(ExtractionErrorV1::InvalidSimulationTick(V1_TICK_CAP))
        );
        assert_eq!(
            snapshot(1, 0, vec![entity(1); MAX_RENDERER_ENTITIES_V1 + 1]),
            Err(ExtractionErrorV1::TooManyEntities {
                count: MAX_RENDERER_ENTITIES_V1 + 1,
                cap: MAX_RENDERER_ENTITIES_V1
            })
        );
        let mut invalid = entity(1);
        invalid.position_mm[0] = MAX_ABS_POSITION_MM_V1 + 1;
        assert_eq!(
            snapshot(1, 0, vec![invalid]),
            Err(ExtractionErrorV1::PositionOutOfRange)
        );
        let mut invalid = entity(1);
        invalid.orientation_q30 = [0; 4];
        assert_eq!(
            snapshot(1, 0, vec![invalid]),
            Err(ExtractionErrorV1::InvalidOrientation)
        );
        let mut invalid = entity(1);
        invalid.orientation_q30[0] = MAX_ABS_ORIENTATION_COMPONENT_V1 + 1;
        assert_eq!(
            snapshot(1, 0, vec![invalid]),
            Err(ExtractionErrorV1::OrientationOutOfRange)
        );
        let mut invalid = entity(1);
        invalid.character_state_tag = 0;
        assert_eq!(
            snapshot(1, 0, vec![invalid]),
            Err(ExtractionErrorV1::InvalidCharacterState(0))
        );
        let mut invalid = entity(1);
        invalid.scale_milli = 0;
        assert_eq!(
            snapshot(1, 0, vec![invalid]),
            Err(ExtractionErrorV1::InvalidScale(0))
        );
        let mut invalid_weather = weather();
        invalid_weather.rain_milli = MAX_WEATHER_MILLI_V1 + 1;
        assert_eq!(
            RendererExtractSnapshotV1::build(1, 0, 3, [1; 32], [2; 32], vec![], invalid_weather),
            Err(ExtractionErrorV1::InvalidWeather)
        );
        invalid_weather = weather();
        invalid_weather.wind_mm_s[0] = MAX_ABS_WIND_MM_S_V1 + 1;
        assert_eq!(
            RendererExtractSnapshotV1::build(1, 0, 3, [1; 32], [2; 32], vec![], invalid_weather),
            Err(ExtractionErrorV1::InvalidWind)
        );
    }

    #[test]
    fn publication_is_lexicographic_and_held_readers_are_immutable() {
        let mut slot = RendererSnapshotSlotV1::default();
        slot.publish(snapshot(1, 10, vec![entity(1)]).unwrap())
            .unwrap();
        let held = slot.acquire().unwrap();
        slot.publish(snapshot(1, 11, vec![entity(2)]).unwrap())
            .unwrap();
        assert_eq!(held.simulation_tick(), 10);
        assert_eq!(held.entities()[0].semantic_digest, [1; 32]);
        assert_eq!(slot.acquire().unwrap().simulation_tick(), 11);
        assert_eq!(
            slot.publish(snapshot(1, 10, vec![]).unwrap()),
            Err(PublishErrorV1::StaleOrEqual {
                current_run_epoch: 1,
                current_tick: 11,
                offered_run_epoch: 1,
                offered_tick: 10
            })
        );
        slot.publish(snapshot(2, 0, vec![entity(3)]).unwrap())
            .unwrap();
        assert_eq!(slot.acquire().unwrap().run_epoch(), 2);
        assert_eq!(slot.acquire().unwrap().simulation_tick(), 0);
        assert_eq!(
            slot.publish(snapshot(2, 0, vec![]).unwrap()),
            Err(PublishErrorV1::StaleOrEqual {
                current_run_epoch: 2,
                current_tick: 0,
                offered_run_epoch: 2,
                offered_tick: 0
            })
        );
        assert_eq!(
            slot.publish(snapshot(1, 99, vec![]).unwrap()),
            Err(PublishErrorV1::StaleOrEqual {
                current_run_epoch: 2,
                current_tick: 0,
                offered_run_epoch: 1,
                offered_tick: 99
            })
        );
    }

    #[test]
    fn setup_requires_both_commits_and_start_is_idempotent() {
        let mut setup = RendererSetupEpochV1::default();
        assert_eq!(
            setup.start(7, 100),
            Err(SetupErrorV1::NotReady {
                agreement_committed: false,
                readiness_committed: false
            })
        );
        setup.commit_agreement();
        assert_eq!(
            setup.start(7, 100),
            Err(SetupErrorV1::NotReady {
                agreement_committed: true,
                readiness_committed: false
            })
        );
        let mut readiness_only = RendererSetupEpochV1::default();
        readiness_only.commit_readiness();
        assert_eq!(
            readiness_only.start(7, 100),
            Err(SetupErrorV1::NotReady {
                agreement_committed: false,
                readiness_committed: true
            })
        );
        setup.commit_readiness();
        let first = setup.start(7, 100).unwrap();
        let after_arbitrary_host_latency = setup.start(7, 100).unwrap();
        assert_eq!(first, after_arbitrary_host_latency);
        assert_eq!(first.run_epoch, 7);
        assert_eq!(first.start_tick, 100);
        assert_eq!(
            hex_bytes(&first.frame_zero_token),
            "047f93e8301117476f88fd825d16f1a10f24f2158cf207a1542fd6d931e29ca2"
        );
        assert_eq!(setup.start(0, 100), Err(SetupErrorV1::InvalidRunEpoch));
        assert_eq!(
            setup.start(7, V1_TICK_CAP),
            Err(SetupErrorV1::InvalidStartTick(V1_TICK_CAP))
        );
    }
}
