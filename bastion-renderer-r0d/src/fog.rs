//! Generation-bound fog and visibility presentation policy.
//!
//! Gameplay visibility remains authoritative in [`EnvironmentProjectionV1`].
//! This module only derives a bounded visual response for the already-visible
//! scene. Decorative presentation may hide more, but can never expand the
//! declared terrain/entity visibility bounds.

use std::sync::Arc;

use crate::{
    domain_hash_v1,
    environment::{GameplayVisibilityV1, MAX_ENVIRONMENT_VISIBILITY_BLOCKS_V1},
};

pub const FOG_POLICY_SCHEMA_V1: u16 = 1;
pub const MAX_FOG_DISTANCE_BLOCKS_V1: u16 = MAX_ENVIRONMENT_VISIBILITY_BLOCKS_V1;
const FOG_MAGIC_V1: [u8; 8] = *b"R1FFOGV1";
const FOG_PAYLOAD_BYTES_V1: usize = 114;
const FOG_BYTES_V1: usize = FOG_PAYLOAD_BYTES_V1 + 32;

pub type FogDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FogModeV1 {
    Outdoor = 1,
    Underwater = 2,
    Underground = 3,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FogQualityV1 {
    Low = 1,
    Full = 2,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ShelterStateV1 {
    /// No authoritative shelter/room producer is bound.
    Unavailable = 0,
    /// Reserved for a future authoritative producer.
    Exposed = 1,
    /// Reserved for a future authoritative producer.
    Sheltered = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FogPolicyInputV1 {
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub environment_projection_digest: FogDigestV1,
    pub camera_token_digest: FogDigestV1,
    pub visibility: GameplayVisibilityV1,
    /// Number of world blocks represented by one visibility unit.
    pub visibility_unit_blocks: u16,
    pub mode: FogModeV1,
    pub quality: FogQualityV1,
    pub shelter: ShelterStateV1,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FogPolicyV1 {
    input: FogPolicyInputV1,
    near_blocks: u16,
    far_blocks: u16,
    color_milli: [u16; 3],
    policy_digest: FogDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FogErrorV1 {
    UnsealedOrPartial,
    InvalidGeneration,
    InvalidIdentity,
    VisibilityOutOfRange,
    InvalidDistanceRange,
    InvalidMagic,
    UnsupportedVersion(u16),
    NonzeroReserved,
    UnknownMode(u8),
    UnknownQuality(u8),
    UnknownShelter(u8),
    MalformedBoolean(u8),
    Truncated,
    TrailingBytes,
    DigestMismatch,
    HashFailure,
    StaleOrEqualGeneration { current: u64, offered: u64 },
    RollbackGenerationMismatch,
}

#[derive(Clone, Debug, Default)]
pub struct FogPolicyPublisherV1 {
    current: Option<Arc<FogPolicyV1>>,
    previous: Option<Arc<FogPolicyV1>>,
}

impl FogPolicyV1 {
    pub fn new(input: FogPolicyInputV1) -> Result<Self, FogErrorV1> {
        validate_input(&input)?;
        let (near_blocks, far_blocks, color_milli) = derive_response(&input)?;
        let payload = encode_payload(&input, near_blocks, far_blocks, color_milli);
        let policy_digest = domain_hash_v1("bastion/r1f/fog-policy", 1, 0, &payload)
            .map_err(|_| FogErrorV1::HashFailure)?;
        Ok(Self {
            input,
            near_blocks,
            far_blocks,
            color_milli,
            policy_digest,
        })
    }

    #[must_use]
    pub const fn presentation_generation(&self) -> u64 { self.input.presentation_generation }

    #[must_use]
    pub const fn simulation_tick(&self) -> u64 { self.input.simulation_tick }

    #[must_use]
    pub const fn environment_projection_digest(&self) -> FogDigestV1 {
        self.input.environment_projection_digest
    }

    #[must_use]
    pub const fn camera_token_digest(&self) -> FogDigestV1 { self.input.camera_token_digest }

    #[must_use]
    pub const fn visibility(&self) -> GameplayVisibilityV1 { self.input.visibility }

    #[must_use]
    pub const fn mode(&self) -> FogModeV1 { self.input.mode }

    #[must_use]
    pub const fn quality(&self) -> FogQualityV1 { self.input.quality }

    #[must_use]
    pub const fn shelter(&self) -> ShelterStateV1 { self.input.shelter }

    #[must_use]
    pub const fn near_blocks(&self) -> u16 { self.near_blocks }

    #[must_use]
    pub const fn far_blocks(&self) -> u16 { self.far_blocks }

    #[must_use]
    pub const fn color_milli(&self) -> [u16; 3] { self.color_milli }

    #[must_use]
    pub const fn policy_digest(&self) -> FogDigestV1 { self.policy_digest }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, FogErrorV1> {
        validate_input(&self.input)?;
        let (near, far, color) = derive_response(&self.input)?;
        if near != self.near_blocks || far != self.far_blocks || color != self.color_milli {
            return Err(FogErrorV1::DigestMismatch);
        }
        let mut bytes = encode_payload(&self.input, near, far, color);
        let digest = domain_hash_v1("bastion/r1f/fog-policy", 1, 0, &bytes)
            .map_err(|_| FogErrorV1::HashFailure)?;
        if digest != self.policy_digest {
            return Err(FogErrorV1::DigestMismatch);
        }
        bytes.extend_from_slice(&digest);
        Ok(bytes)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, FogErrorV1> {
        if bytes.len() < FOG_BYTES_V1 {
            return Err(FogErrorV1::Truncated);
        }
        if bytes.len() > FOG_BYTES_V1 {
            return Err(FogErrorV1::TrailingBytes);
        }
        let mut reader = Reader::new(bytes);
        if reader.array::<8>()? != FOG_MAGIC_V1 {
            return Err(FogErrorV1::InvalidMagic);
        }
        let version = reader.u16()?;
        if version != FOG_POLICY_SCHEMA_V1 {
            return Err(FogErrorV1::UnsupportedVersion(version));
        }
        if reader.u16()? != 0 {
            return Err(FogErrorV1::NonzeroReserved);
        }
        let presentation_generation = reader.u64()?;
        let simulation_tick = reader.u64()?;
        let environment_projection_digest = reader.array::<32>()?;
        let camera_token_digest = reader.array::<32>()?;
        let visibility = GameplayVisibilityV1 {
            terrain_blocks: reader.u16()?,
            entity_blocks: reader.u16()?,
        };
        let visibility_unit_blocks = reader.u16()?;
        let mode = match reader.u8()? {
            1 => FogModeV1::Outdoor,
            2 => FogModeV1::Underwater,
            3 => FogModeV1::Underground,
            value => return Err(FogErrorV1::UnknownMode(value)),
        };
        let quality = match reader.u8()? {
            1 => FogQualityV1::Low,
            2 => FogQualityV1::Full,
            value => return Err(FogErrorV1::UnknownQuality(value)),
        };
        let shelter = match reader.u8()? {
            0 => ShelterStateV1::Unavailable,
            1 => ShelterStateV1::Exposed,
            2 => ShelterStateV1::Sheltered,
            value => return Err(FogErrorV1::UnknownShelter(value)),
        };
        let complete = match reader.u8()? {
            0 => false,
            1 => true,
            value => return Err(FogErrorV1::MalformedBoolean(value)),
        };
        let encoded_near = reader.u16()?;
        let encoded_far = reader.u16()?;
        let encoded_color = [reader.u16()?, reader.u16()?, reader.u16()?];
        if reader.u16()? != 0 {
            return Err(FogErrorV1::NonzeroReserved);
        }
        let declared_digest = reader.array::<32>()?;
        if !reader.is_eof() {
            return Err(FogErrorV1::TrailingBytes);
        }
        let policy = Self::new(FogPolicyInputV1 {
            presentation_generation,
            simulation_tick,
            environment_projection_digest,
            camera_token_digest,
            visibility,
            visibility_unit_blocks,
            mode,
            quality,
            shelter,
            complete,
        })?;
        if policy.near_blocks != encoded_near
            || policy.far_blocks != encoded_far
            || policy.color_milli != encoded_color
            || policy.policy_digest != declared_digest
        {
            return Err(FogErrorV1::DigestMismatch);
        }
        Ok(policy)
    }
}

impl FogPolicyPublisherV1 {
    #[must_use]
    pub fn current(&self) -> Option<Arc<FogPolicyV1>> { self.current.clone() }

    pub fn publish(&mut self, policy: FogPolicyV1) -> Result<Arc<FogPolicyV1>, FogErrorV1> {
        if let Some(current) = &self.current
            && policy.presentation_generation() <= current.presentation_generation()
        {
            return Err(FogErrorV1::StaleOrEqualGeneration {
                current: current.presentation_generation(),
                offered: policy.presentation_generation(),
            });
        }
        let policy = Arc::new(policy);
        self.previous = self.current.replace(Arc::clone(&policy));
        Ok(policy)
    }

    pub fn rollback(&mut self, failed_generation: u64) -> Result<Arc<FogPolicyV1>, FogErrorV1> {
        if self
            .current
            .as_ref()
            .is_none_or(|current| current.presentation_generation() != failed_generation)
        {
            return Err(FogErrorV1::RollbackGenerationMismatch);
        }
        let previous = self
            .previous
            .take()
            .ok_or(FogErrorV1::RollbackGenerationMismatch)?;
        self.current = Some(Arc::clone(&previous));
        Ok(previous)
    }
}

fn validate_input(input: &FogPolicyInputV1) -> Result<(), FogErrorV1> {
    if !input.complete {
        return Err(FogErrorV1::UnsealedOrPartial);
    }
    if input.presentation_generation == 0 {
        return Err(FogErrorV1::InvalidGeneration);
    }
    if input.environment_projection_digest == [0; 32] || input.camera_token_digest == [0; 32] {
        return Err(FogErrorV1::InvalidIdentity);
    }
    if input.visibility.terrain_blocks == 0
        || input.visibility.entity_blocks == 0
        || input.visibility.entity_blocks > input.visibility.terrain_blocks
        || input.visibility.terrain_blocks > MAX_FOG_DISTANCE_BLOCKS_V1
        || input.visibility_unit_blocks == 0
        || input.visibility_unit_blocks > 64
    {
        return Err(FogErrorV1::VisibilityOutOfRange);
    }
    Ok(())
}

fn derive_response(input: &FogPolicyInputV1) -> Result<(u16, u16, [u16; 3]), FogErrorV1> {
    let terrain = input
        .visibility
        .terrain_blocks
        .checked_mul(input.visibility_unit_blocks)
        .ok_or(FogErrorV1::InvalidDistanceRange)?
        .min(MAX_FOG_DISTANCE_BLOCKS_V1);
    let (near, far, color) = match input.mode {
        FogModeV1::Outdoor => (terrain.saturating_mul(3) / 4, terrain, [720, 780, 840]),
        FogModeV1::Underwater => {
            let far = terrain.min(96);
            (far / 8, far, [35, 160, 250])
        },
        FogModeV1::Underground => {
            let far = terrain.min(128);
            (far / 4, far, [35, 40, 50])
        },
    };
    if far < 2 || near >= far {
        return Err(FogErrorV1::InvalidDistanceRange);
    }
    // Shelter is deliberately not consulted here. Until an authoritative
    // producer exists, an unavailable shelter field cannot change pixels or
    // gameplay visibility. Future support must version this policy.
    let _ = input.shelter;
    Ok((near, far, color))
}

fn encode_payload(
    input: &FogPolicyInputV1,
    near_blocks: u16,
    far_blocks: u16,
    color_milli: [u16; 3],
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(FOG_PAYLOAD_BYTES_V1);
    bytes.extend_from_slice(&FOG_MAGIC_V1);
    bytes.extend_from_slice(&FOG_POLICY_SCHEMA_V1.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&input.presentation_generation.to_le_bytes());
    bytes.extend_from_slice(&input.simulation_tick.to_le_bytes());
    bytes.extend_from_slice(&input.environment_projection_digest);
    bytes.extend_from_slice(&input.camera_token_digest);
    bytes.extend_from_slice(&input.visibility.terrain_blocks.to_le_bytes());
    bytes.extend_from_slice(&input.visibility.entity_blocks.to_le_bytes());
    bytes.extend_from_slice(&input.visibility_unit_blocks.to_le_bytes());
    bytes.push(input.mode as u8);
    bytes.push(input.quality as u8);
    bytes.push(input.shelter as u8);
    bytes.push(u8::from(input.complete));
    bytes.extend_from_slice(&near_blocks.to_le_bytes());
    bytes.extend_from_slice(&far_blocks.to_le_bytes());
    for value in color_milli {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    debug_assert_eq!(bytes.len(), FOG_PAYLOAD_BYTES_V1);
    bytes
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, cursor: 0 } }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], FogErrorV1> {
        let end = self.cursor.checked_add(N).ok_or(FogErrorV1::Truncated)?;
        let slice = self
            .bytes
            .get(self.cursor..end)
            .ok_or(FogErrorV1::Truncated)?;
        self.cursor = end;
        slice.try_into().map_err(|_| FogErrorV1::Truncated)
    }

    fn u8(&mut self) -> Result<u8, FogErrorV1> { Ok(self.array::<1>()?[0]) }

    fn u16(&mut self) -> Result<u16, FogErrorV1> { Ok(u16::from_le_bytes(self.array::<2>()?)) }

    fn u64(&mut self) -> Result<u64, FogErrorV1> { Ok(u64::from_le_bytes(self.array::<8>()?)) }

    fn is_eof(&self) -> bool { self.cursor == self.bytes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex32;

    fn input() -> FogPolicyInputV1 {
        FogPolicyInputV1 {
            presentation_generation: 7,
            simulation_tick: 300,
            environment_projection_digest: [1; 32],
            camera_token_digest: [2; 32],
            visibility: GameplayVisibilityV1 {
                terrain_blocks: 512,
                entity_blocks: 256,
            },
            visibility_unit_blocks: 1,
            mode: FogModeV1::Outdoor,
            quality: FogQualityV1::Full,
            shelter: ShelterStateV1::Unavailable,
            complete: true,
        }
    }

    #[test]
    fn canonical_round_trip_and_frozen_digest() {
        let policy = FogPolicyV1::new(input()).unwrap();
        let bytes = policy.canonical_bytes().unwrap();
        assert_eq!(bytes.len(), FOG_BYTES_V1);
        assert_eq!(FogPolicyV1::decode_exact(&bytes).unwrap(), policy);
        assert_eq!(
            hex32(&policy.policy_digest()),
            "19e4d2ff7a0cf90840f3c1dc6c665f10f7db75e099a5f5d2a654a8c42fbf174e"
        );
    }

    #[test]
    fn visibility_and_modes_have_bounded_source_backed_ranges() {
        let outdoor = FogPolicyV1::new(input()).unwrap();
        assert_eq!((outdoor.near_blocks(), outdoor.far_blocks()), (384, 512));

        let mut underwater = input();
        underwater.mode = FogModeV1::Underwater;
        let underwater = FogPolicyV1::new(underwater).unwrap();
        assert_eq!(
            (underwater.near_blocks(), underwater.far_blocks()),
            (12, 96)
        );

        let mut underground = input();
        underground.mode = FogModeV1::Underground;
        let underground = FogPolicyV1::new(underground).unwrap();
        assert_eq!(
            (underground.near_blocks(), underground.far_blocks()),
            (32, 128)
        );
        assert_ne!(outdoor.policy_digest(), underwater.policy_digest());
        assert_ne!(underwater.policy_digest(), underground.policy_digest());
    }

    #[test]
    fn quality_fallback_preserves_authoritative_visibility() {
        let full = FogPolicyV1::new(input()).unwrap();
        let mut low = input();
        low.quality = FogQualityV1::Low;
        let low = FogPolicyV1::new(low).unwrap();
        assert_eq!(full.near_blocks(), low.near_blocks());
        assert_eq!(full.far_blocks(), low.far_blocks());
        assert_ne!(full.policy_digest(), low.policy_digest());
    }

    #[test]
    fn unavailable_shelter_never_changes_response() {
        let unavailable = FogPolicyV1::new(input()).unwrap();
        let mut exposed = input();
        exposed.shelter = ShelterStateV1::Exposed;
        let exposed = FogPolicyV1::new(exposed).unwrap();
        assert_eq!(unavailable.near_blocks(), exposed.near_blocks());
        assert_eq!(unavailable.far_blocks(), exposed.far_blocks());
        assert_eq!(unavailable.color_milli(), exposed.color_milli());
    }

    #[test]
    fn malformed_partial_visibility_and_identity_fail_closed() {
        let mut partial = input();
        partial.complete = false;
        assert_eq!(
            FogPolicyV1::new(partial),
            Err(FogErrorV1::UnsealedOrPartial)
        );
        let mut invalid = input();
        invalid.visibility.entity_blocks = 513;
        assert_eq!(
            FogPolicyV1::new(invalid),
            Err(FogErrorV1::VisibilityOutOfRange)
        );
        let mut invalid = input();
        invalid.camera_token_digest = [0; 32];
        assert_eq!(FogPolicyV1::new(invalid), Err(FogErrorV1::InvalidIdentity));
    }

    #[test]
    fn decoder_rejects_unknowns_truncation_trailing_and_digest_damage() {
        let bytes = FogPolicyV1::new(input())
            .unwrap()
            .canonical_bytes()
            .unwrap();
        let mut unknown_mode = bytes.clone();
        unknown_mode[98] = 99;
        assert_eq!(
            FogPolicyV1::decode_exact(&unknown_mode),
            Err(FogErrorV1::UnknownMode(99))
        );
        let mut unknown_quality = bytes.clone();
        unknown_quality[99] = 99;
        assert_eq!(
            FogPolicyV1::decode_exact(&unknown_quality),
            Err(FogErrorV1::UnknownQuality(99))
        );
        let mut unknown_shelter = bytes.clone();
        unknown_shelter[100] = 99;
        assert_eq!(
            FogPolicyV1::decode_exact(&unknown_shelter),
            Err(FogErrorV1::UnknownShelter(99))
        );
        assert_eq!(
            FogPolicyV1::decode_exact(&bytes[..bytes.len() - 1]),
            Err(FogErrorV1::Truncated)
        );
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            FogPolicyV1::decode_exact(&trailing),
            Err(FogErrorV1::TrailingBytes)
        );
        let mut damaged = bytes;
        damaged[114] ^= 1;
        assert_eq!(
            FogPolicyV1::decode_exact(&damaged),
            Err(FogErrorV1::DigestMismatch)
        );
    }

    #[test]
    fn publication_is_monotonic_and_rollback_preserves_previous_arc() {
        let mut publisher = FogPolicyPublisherV1::default();
        let first = publisher
            .publish(FogPolicyV1::new(input()).unwrap())
            .unwrap();
        let held = Arc::clone(&first);
        let mut next = input();
        next.presentation_generation = 8;
        next.simulation_tick = 301;
        let second = publisher.publish(FogPolicyV1::new(next).unwrap()).unwrap();
        assert_eq!(held.presentation_generation(), 7);
        assert_eq!(second.presentation_generation(), 8);
        assert!(matches!(
            publisher.publish(FogPolicyV1::new(input()).unwrap()),
            Err(FogErrorV1::StaleOrEqualGeneration { .. })
        ));
        let restored = publisher.rollback(8).unwrap();
        assert_eq!(restored.presentation_generation(), 7);
    }
}
