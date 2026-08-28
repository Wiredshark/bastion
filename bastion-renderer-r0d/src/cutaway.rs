//! Deterministic, presentation-only vertical terrain cutaway policy and
//! geometry.

use std::{collections::BTreeMap, sync::Arc};

use crate::domain_hash_v1;

pub const CUTAWAY_POLICY_V1: (u16, u16) = (1, 0);
pub const CUTAWAY_POLICY_MAGIC_V1: [u8; 8] = *b"BCUTAWY1";
pub const MAX_CUTAWAY_CELLS_V1: usize = 32_768;
pub const MAX_CUTAWAY_CAP_FACES_V1: usize = MAX_CUTAWAY_CELLS_V1 * 6;
pub const MAX_CUTAWAY_COORDINATE_V1: i32 = 1_000_000;
pub const MAX_CUTAWAY_TRANSITION_STEPS_V1: u16 = 240;
pub const MAX_CUTAWAY_POLICY_BYTES_V1: usize = 1_048_576;

pub type CutawayDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CutawayAxisV1 {
    X = 1,
    Y = 2,
}

impl TryFrom<u8> for CutawayAxisV1 {
    type Error = CutawayErrorV1;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::X),
            2 => Ok(Self::Y),
            _ => Err(CutawayErrorV1::UnknownMode),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CutawayModeV1 {
    Off,
    Layer {
        maximum_visible_z: i32,
    },
    VerticalPlane {
        axis: CutawayAxisV1,
        coordinate: i32,
        retain_less_equal: bool,
    },
    BoundedVolume {
        minimum: CellPositionV1,
        maximum: CellPositionV1,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CutawayTransitionKindV1 {
    Off = 0,
    Entering = 1,
    Held = 2,
    Restoring = 3,
}

impl TryFrom<u8> for CutawayTransitionKindV1 {
    type Error = CutawayErrorV1;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Off),
            1 => Ok(Self::Entering),
            2 => Ok(Self::Held),
            3 => Ok(Self::Restoring),
            _ => Err(CutawayErrorV1::InvalidTransition),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CutawayTransitionV1 {
    pub kind: CutawayTransitionKindV1,
    pub step: u16,
    pub total_steps: u16,
}

impl CutawayTransitionV1 {
    pub fn validate(self) -> Result<(), CutawayErrorV1> {
        if self.kind == CutawayTransitionKindV1::Off {
            return (self.step == 0 && self.total_steps == 0)
                .then_some(())
                .ok_or(CutawayErrorV1::InvalidTransition);
        }
        if self.total_steps == 0
            || self.total_steps > MAX_CUTAWAY_TRANSITION_STEPS_V1
            || self.step > self.total_steps
        {
            return Err(CutawayErrorV1::InvalidTransition);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CellPositionV1 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl CellPositionV1 {
    pub const fn new(x: i32, y: i32, z: i32) -> Self { Self { x, y, z } }

    fn checked_neighbor(self, direction: FaceDirectionV1) -> Result<Self, CutawayErrorV1> {
        let [dx, dy, dz] = direction.normal();
        Ok(Self {
            x: self
                .x
                .checked_add(i32::from(dx))
                .ok_or(CutawayErrorV1::Overflow)?,
            y: self
                .y
                .checked_add(i32::from(dy))
                .ok_or(CutawayErrorV1::Overflow)?,
            z: self
                .z
                .checked_add(i32::from(dz))
                .ok_or(CutawayErrorV1::Overflow)?,
        })
    }

    fn validate(self) -> Result<(), CutawayErrorV1> {
        [self.x, self.y, self.z]
            .into_iter()
            .all(|component| {
                component
                    .checked_abs()
                    .is_some_and(|absolute| absolute <= MAX_CUTAWAY_COORDINATE_V1)
            })
            .then_some(())
            .ok_or(CutawayErrorV1::CoordinateOutOfRange)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutawayPolicyV1 {
    presentation_generation: u64,
    terrain_generation: CutawayDigestV1,
    camera_token: CutawayDigestV1,
    camera_sequence: u64,
    mode: CutawayModeV1,
    transition: CutawayTransitionV1,
    cap_material: u16,
    reveal_authority: CutawayDigestV1,
    authorized_cells: Vec<CellPositionV1>,
    policy_digest: CutawayDigestV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CutawayPolicyInputV1 {
    pub presentation_generation: u64,
    pub terrain_generation: CutawayDigestV1,
    pub camera_token: CutawayDigestV1,
    pub camera_sequence: u64,
    pub mode: CutawayModeV1,
    pub transition: CutawayTransitionV1,
    pub cap_material: u16,
    pub reveal_authority: CutawayDigestV1,
}

impl CutawayPolicyV1 {
    pub fn new(
        input: CutawayPolicyInputV1,
        mut authorized_cells: Vec<CellPositionV1>,
    ) -> Result<Self, CutawayErrorV1> {
        if input.presentation_generation == 0
            || input.terrain_generation == [0; 32]
            || input.camera_token == [0; 32]
            || input.reveal_authority == [0; 32]
            || input.cap_material == 0
            || authorized_cells.len() > MAX_CUTAWAY_CELLS_V1
        {
            return Err(CutawayErrorV1::InvalidPolicy);
        }
        input.transition.validate()?;
        validate_mode(input.mode)?;
        if matches!(input.mode, CutawayModeV1::Off)
            != (input.transition.kind == CutawayTransitionKindV1::Off)
        {
            return Err(CutawayErrorV1::InvalidTransition);
        }
        for cell in &authorized_cells {
            cell.validate()?;
        }
        authorized_cells.sort_unstable();
        if authorized_cells.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(CutawayErrorV1::DuplicateAuthorization);
        }
        let mut value = Self {
            presentation_generation: input.presentation_generation,
            terrain_generation: input.terrain_generation,
            camera_token: input.camera_token,
            camera_sequence: input.camera_sequence,
            mode: input.mode,
            transition: input.transition,
            cap_material: input.cap_material,
            reveal_authority: input.reveal_authority,
            authorized_cells,
            policy_digest: [0; 32],
        };
        value.policy_digest = domain_hash_v1(
            "bastion/r1e/cutaway-policy",
            CUTAWAY_POLICY_V1.0,
            CUTAWAY_POLICY_V1.1,
            &value.canonical_payload()?,
        )
        .map_err(|_| CutawayErrorV1::Hash)?;
        Ok(value)
    }

    #[must_use]
    pub const fn presentation_generation(&self) -> u64 { self.presentation_generation }

    #[must_use]
    pub const fn terrain_generation(&self) -> CutawayDigestV1 { self.terrain_generation }

    #[must_use]
    pub const fn camera_token(&self) -> CutawayDigestV1 { self.camera_token }

    #[must_use]
    pub const fn camera_sequence(&self) -> u64 { self.camera_sequence }

    #[must_use]
    pub const fn mode(&self) -> CutawayModeV1 { self.mode }

    #[must_use]
    pub const fn transition(&self) -> CutawayTransitionV1 { self.transition }

    #[must_use]
    pub const fn cap_material(&self) -> u16 { self.cap_material }

    #[must_use]
    pub const fn reveal_authority(&self) -> CutawayDigestV1 { self.reveal_authority }

    #[must_use]
    pub fn authorized_cells(&self) -> &[CellPositionV1] { &self.authorized_cells }

    #[must_use]
    pub const fn policy_digest(&self) -> CutawayDigestV1 { self.policy_digest }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CutawayErrorV1> {
        let payload = self.canonical_payload()?;
        let mut output = Vec::with_capacity(
            CUTAWAY_POLICY_MAGIC_V1
                .len()
                .checked_add(4)
                .and_then(|length| length.checked_add(payload.len()))
                .and_then(|length| length.checked_add(32))
                .ok_or(CutawayErrorV1::Overflow)?,
        );
        output.extend_from_slice(&CUTAWAY_POLICY_MAGIC_V1);
        output.extend_from_slice(&CUTAWAY_POLICY_V1.0.to_le_bytes());
        output.extend_from_slice(&CUTAWAY_POLICY_V1.1.to_le_bytes());
        output.extend_from_slice(&payload);
        output.extend_from_slice(&self.policy_digest);
        if output.len() > MAX_CUTAWAY_POLICY_BYTES_V1 {
            return Err(CutawayErrorV1::OversizedInput);
        }
        Ok(output)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, CutawayErrorV1> {
        if bytes.len() > MAX_CUTAWAY_POLICY_BYTES_V1 {
            return Err(CutawayErrorV1::OversizedInput);
        }
        let mut reader = ReaderV1::new(bytes);
        if reader.take(8)? != CUTAWAY_POLICY_MAGIC_V1 {
            return Err(CutawayErrorV1::MalformedEncoding);
        }
        let major = reader.u16()?;
        let minor = reader.u16()?;
        if (major, minor) != CUTAWAY_POLICY_V1 {
            return Err(CutawayErrorV1::UnsupportedVersion);
        }
        let presentation_generation = reader.u64()?;
        let terrain_generation = reader.digest()?;
        let camera_token = reader.digest()?;
        let camera_sequence = reader.u64()?;
        let mode = match reader.u8()? {
            0 => CutawayModeV1::Off,
            1 => CutawayModeV1::Layer {
                maximum_visible_z: reader.i32()?,
            },
            2 => CutawayModeV1::VerticalPlane {
                axis: CutawayAxisV1::try_from(reader.u8()?)?,
                coordinate: reader.i32()?,
                retain_less_equal: match reader.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(CutawayErrorV1::MalformedEncoding),
                },
            },
            3 => CutawayModeV1::BoundedVolume {
                minimum: reader.position()?,
                maximum: reader.position()?,
            },
            _ => return Err(CutawayErrorV1::UnknownMode),
        };
        let transition = CutawayTransitionV1 {
            kind: CutawayTransitionKindV1::try_from(reader.u8()?)?,
            step: reader.u16()?,
            total_steps: reader.u16()?,
        };
        let cap_material = reader.u16()?;
        let reveal_authority = reader.digest()?;
        let count = usize::try_from(reader.u32()?).map_err(|_| CutawayErrorV1::OversizedInput)?;
        if count > MAX_CUTAWAY_CELLS_V1 {
            return Err(CutawayErrorV1::OversizedInput);
        }
        let mut authorized_cells = Vec::with_capacity(count);
        for _ in 0..count {
            authorized_cells.push(reader.position()?);
        }
        let encoded_digest = reader.digest()?;
        if !reader.at_end() {
            return Err(CutawayErrorV1::TrailingBytes);
        }
        let value = Self::new(
            CutawayPolicyInputV1 {
                presentation_generation,
                terrain_generation,
                camera_token,
                camera_sequence,
                mode,
                transition,
                cap_material,
                reveal_authority,
            },
            authorized_cells,
        )?;
        if encoded_digest != value.policy_digest {
            return Err(CutawayErrorV1::DigestMismatch);
        }
        Ok(value)
    }

    fn canonical_payload(&self) -> Result<Vec<u8>, CutawayErrorV1> {
        let mut output = Vec::new();
        output.extend_from_slice(&self.presentation_generation.to_le_bytes());
        output.extend_from_slice(&self.terrain_generation);
        output.extend_from_slice(&self.camera_token);
        output.extend_from_slice(&self.camera_sequence.to_le_bytes());
        match self.mode {
            CutawayModeV1::Off => output.push(0),
            CutawayModeV1::Layer { maximum_visible_z } => {
                output.push(1);
                output.extend_from_slice(&maximum_visible_z.to_le_bytes());
            },
            CutawayModeV1::VerticalPlane {
                axis,
                coordinate,
                retain_less_equal,
            } => {
                output.push(2);
                output.push(axis as u8);
                output.extend_from_slice(&coordinate.to_le_bytes());
                output.push(u8::from(retain_less_equal));
            },
            CutawayModeV1::BoundedVolume { minimum, maximum } => {
                output.push(3);
                encode_position(&mut output, minimum);
                encode_position(&mut output, maximum);
            },
        }
        output.push(self.transition.kind as u8);
        output.extend_from_slice(&self.transition.step.to_le_bytes());
        output.extend_from_slice(&self.transition.total_steps.to_le_bytes());
        output.extend_from_slice(&self.cap_material.to_le_bytes());
        output.extend_from_slice(&self.reveal_authority);
        output.extend_from_slice(
            &u32::try_from(self.authorized_cells.len())
                .map_err(|_| CutawayErrorV1::OversizedInput)?
                .to_le_bytes(),
        );
        for cell in &self.authorized_cells {
            encode_position(&mut output, *cell);
        }
        Ok(output)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TerrainCellV1 {
    pub position: CellPositionV1,
    pub material: u16,
    pub filled: bool,
    pub reveal_eligible: bool,
    pub content_digest: CutawayDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerrainSliceInputV1 {
    pub presentation_generation: u64,
    pub terrain_generation: CutawayDigestV1,
    pub terrain_revision: u64,
    pub camera_token: CutawayDigestV1,
    pub camera_sequence: u64,
    pub bounds_minimum: CellPositionV1,
    pub bounds_maximum: CellPositionV1,
    pub cells: Vec<TerrainCellV1>,
}

impl TerrainSliceInputV1 {
    pub fn validate_and_sort(&mut self) -> Result<(), CutawayErrorV1> {
        if self.presentation_generation == 0
            || self.terrain_generation == [0; 32]
            || self.camera_token == [0; 32]
            || self.cells.len() > MAX_CUTAWAY_CELLS_V1
        {
            return Err(CutawayErrorV1::InvalidTerrain);
        }
        validate_bounds(self.bounds_minimum, self.bounds_maximum)?;
        for cell in &self.cells {
            cell.position.validate()?;
            if !inside(cell.position, self.bounds_minimum, self.bounds_maximum)
                || (cell.filled && (cell.material == 0 || cell.content_digest == [0; 32]))
            {
                return Err(CutawayErrorV1::InvalidTerrain);
            }
        }
        self.cells.sort_unstable_by_key(|cell| cell.position);
        if self
            .cells
            .windows(2)
            .any(|pair| pair[0].position == pair[1].position)
        {
            return Err(CutawayErrorV1::DuplicateCell);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FaceDirectionV1 {
    NegativeX = 0,
    PositiveX = 1,
    NegativeY = 2,
    PositiveY = 3,
    NegativeZ = 4,
    PositiveZ = 5,
}

impl FaceDirectionV1 {
    pub const ALL: [Self; 6] = [
        Self::NegativeX,
        Self::PositiveX,
        Self::NegativeY,
        Self::PositiveY,
        Self::NegativeZ,
        Self::PositiveZ,
    ];

    #[must_use]
    pub const fn normal(self) -> [i8; 3] {
        match self {
            Self::NegativeX => [-1, 0, 0],
            Self::PositiveX => [1, 0, 0],
            Self::NegativeY => [0, -1, 0],
            Self::PositiveY => [0, 1, 0],
            Self::NegativeZ => [0, 0, -1],
            Self::PositiveZ => [0, 0, 1],
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CapFaceV1 {
    pub retained_cell: CellPositionV1,
    pub direction: FaceDirectionV1,
    pub vertices: [[i32; 3]; 4],
    pub normal: [i8; 3],
    pub material: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutawayGeometryV1 {
    pub presentation_generation: u64,
    pub terrain_generation: CutawayDigestV1,
    pub terrain_revision: u64,
    pub camera_token: CutawayDigestV1,
    pub camera_sequence: u64,
    pub policy_digest: CutawayDigestV1,
    pub terrain_root: CutawayDigestV1,
    pub removed_cells: Vec<CellPositionV1>,
    pub cap_faces: Vec<CapFaceV1>,
    pub roof_removals: u32,
    pub wall_removals: u32,
    pub surface_passthrough: bool,
    pub geometry_digest: CutawayDigestV1,
}

impl CutawayGeometryV1 {
    #[must_use]
    pub fn key(&self) -> CutawayWorkKeyV1 {
        CutawayWorkKeyV1 {
            presentation_generation: self.presentation_generation,
            terrain_revision: self.terrain_revision,
            camera_sequence: self.camera_sequence,
            policy_digest: self.policy_digest,
        }
    }

    pub fn validate_structure(&self) -> Result<(), CutawayErrorV1> {
        if self.removed_cells.len() > MAX_CUTAWAY_CELLS_V1
            || self.cap_faces.len() > MAX_CUTAWAY_CAP_FACES_V1
            || self.removed_cells.windows(2).any(|pair| pair[0] >= pair[1])
            || self.cap_faces.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(CutawayErrorV1::InvalidGeometry);
        }
        for position in &self.removed_cells {
            position.validate()?;
        }
        for face in &self.cap_faces {
            face.retained_cell.validate()?;
            if face.normal != face.direction.normal()
                || face.vertices != face_vertices(face.retained_cell, face.direction)?
            {
                return Err(CutawayErrorV1::InvalidGeometry);
            }
        }
        let removal_count =
            u32::try_from(self.removed_cells.len()).map_err(|_| CutawayErrorV1::Overflow)?;
        if self
            .roof_removals
            .checked_add(self.wall_removals)
            .ok_or(CutawayErrorV1::Overflow)?
            != removal_count
            || (self.surface_passthrough
                && (!self.removed_cells.is_empty() || !self.cap_faces.is_empty()))
            || (!self.surface_passthrough
                && (self.removed_cells.is_empty() || self.cap_faces.is_empty()))
        {
            return Err(CutawayErrorV1::InvalidGeometry);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CutawayWorkKeyV1 {
    pub presentation_generation: u64,
    pub terrain_revision: u64,
    pub camera_sequence: u64,
    pub policy_digest: CutawayDigestV1,
}

pub fn derive_cutaway_geometry_v1(
    policy: &CutawayPolicyV1,
    mut terrain: TerrainSliceInputV1,
) -> Result<CutawayGeometryV1, CutawayErrorV1> {
    terrain.validate_and_sort()?;
    if policy.presentation_generation != terrain.presentation_generation
        || policy.terrain_generation != terrain.terrain_generation
        || policy.camera_token != terrain.camera_token
        || policy.camera_sequence != terrain.camera_sequence
    {
        return Err(CutawayErrorV1::StaleGeneration);
    }
    let cells = terrain
        .cells
        .iter()
        .map(|cell| (cell.position, *cell))
        .collect::<BTreeMap<_, _>>();
    for position in &policy.authorized_cells {
        let cell = cells
            .get(position)
            .ok_or(CutawayErrorV1::UnauthorizedReveal)?;
        if !cell.reveal_eligible {
            return Err(CutawayErrorV1::UnauthorizedReveal);
        }
    }
    let terrain_root = terrain_root(&terrain.cells)?;
    let mut removed_cells = Vec::new();
    for cell in terrain.cells.iter().filter(|cell| cell.filled) {
        if policy
            .authorized_cells
            .binary_search(&cell.position)
            .is_ok()
            && mode_removes(policy.mode, cell.position)
        {
            removed_cells.push(cell.position);
        }
    }
    removed_cells.sort_unstable();
    let mut cap_faces = Vec::new();
    for cell in terrain
        .cells
        .iter()
        .filter(|cell| cell.filled && removed_cells.binary_search(&cell.position).is_err())
    {
        for direction in FaceDirectionV1::ALL {
            let neighbor = cell.position.checked_neighbor(direction)?;
            if removed_cells.binary_search(&neighbor).is_ok() {
                cap_faces.push(CapFaceV1 {
                    retained_cell: cell.position,
                    direction,
                    vertices: face_vertices(cell.position, direction)?,
                    normal: direction.normal(),
                    material: policy.cap_material,
                });
            }
        }
    }
    cap_faces.sort_unstable();
    if cap_faces.len() > MAX_CUTAWAY_CAP_FACES_V1
        || cap_faces.windows(2).any(|pair| {
            pair[0].retained_cell == pair[1].retained_cell && pair[0].direction == pair[1].direction
        })
    {
        return Err(CutawayErrorV1::InvalidGeometry);
    }
    let roof_removals = u32::try_from(
        removed_cells
            .iter()
            .filter(|position| matches!(policy.mode, CutawayModeV1::Layer { .. }) && position.z > 0)
            .count(),
    )
    .map_err(|_| CutawayErrorV1::Overflow)?;
    let wall_removals = u32::try_from(removed_cells.len())
        .map_err(|_| CutawayErrorV1::Overflow)?
        .checked_sub(roof_removals)
        .ok_or(CutawayErrorV1::Overflow)?;
    let surface_passthrough = matches!(policy.mode, CutawayModeV1::Off);
    if (surface_passthrough && (!removed_cells.is_empty() || !cap_faces.is_empty()))
        || (!surface_passthrough && (removed_cells.is_empty() || cap_faces.is_empty()))
    {
        return Err(CutawayErrorV1::InvalidGeometry);
    }
    let geometry_digest = geometry_digest(
        policy,
        &terrain,
        terrain_root,
        &removed_cells,
        &cap_faces,
        roof_removals,
        wall_removals,
    )?;
    let geometry = CutawayGeometryV1 {
        presentation_generation: terrain.presentation_generation,
        terrain_generation: terrain.terrain_generation,
        terrain_revision: terrain.terrain_revision,
        camera_token: terrain.camera_token,
        camera_sequence: terrain.camera_sequence,
        policy_digest: policy.policy_digest,
        terrain_root,
        removed_cells,
        cap_faces,
        roof_removals,
        wall_removals,
        surface_passthrough,
        geometry_digest,
    };
    geometry.validate_structure()?;
    Ok(geometry)
}

#[derive(Clone, Debug, Default)]
pub struct CutawayPublisherV1 {
    latest_key: Option<CutawayWorkKeyV1>,
    visible: Option<Arc<CutawayGeometryV1>>,
}

impl CutawayPublisherV1 {
    pub fn publish(
        &mut self,
        expected: CutawayWorkKeyV1,
        geometry: CutawayGeometryV1,
    ) -> Result<Arc<CutawayGeometryV1>, CutawayErrorV1> {
        geometry.validate_structure()?;
        if expected != geometry.key() {
            return Err(CutawayErrorV1::StaleGeneration);
        }
        if let Some(latest) = self.latest_key {
            let offered_order = (
                expected.presentation_generation,
                expected.terrain_revision,
                expected.camera_sequence,
            );
            let latest_order = (
                latest.presentation_generation,
                latest.terrain_revision,
                latest.camera_sequence,
            );
            if offered_order <= latest_order {
                return Err(CutawayErrorV1::SupersededWork);
            }
        }
        let published = Arc::new(geometry);
        self.latest_key = Some(expected);
        self.visible = Some(published.clone());
        Ok(published)
    }

    #[must_use]
    pub fn acquire(&self) -> Option<Arc<CutawayGeometryV1>> { self.visible.clone() }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CutawayErrorV1 {
    UnsupportedVersion,
    UnknownMode,
    InvalidPolicy,
    InvalidTransition,
    InvalidBounds,
    CoordinateOutOfRange,
    DuplicateAuthorization,
    DuplicateCell,
    UnauthorizedReveal,
    InvalidTerrain,
    InvalidGeometry,
    StaleGeneration,
    SupersededWork,
    OversizedInput,
    MalformedEncoding,
    TrailingBytes,
    DigestMismatch,
    Overflow,
    Hash,
}

fn validate_mode(mode: CutawayModeV1) -> Result<(), CutawayErrorV1> {
    match mode {
        CutawayModeV1::Off => Ok(()),
        CutawayModeV1::Layer { maximum_visible_z } => {
            CellPositionV1::new(0, 0, maximum_visible_z).validate()
        },
        CutawayModeV1::VerticalPlane { coordinate, .. } => {
            CellPositionV1::new(coordinate, 0, 0).validate()
        },
        CutawayModeV1::BoundedVolume { minimum, maximum } => validate_bounds(minimum, maximum),
    }
}

fn validate_bounds(minimum: CellPositionV1, maximum: CellPositionV1) -> Result<(), CutawayErrorV1> {
    minimum.validate()?;
    maximum.validate()?;
    (minimum.x <= maximum.x && minimum.y <= maximum.y && minimum.z <= maximum.z)
        .then_some(())
        .ok_or(CutawayErrorV1::InvalidBounds)
}

fn inside(position: CellPositionV1, minimum: CellPositionV1, maximum: CellPositionV1) -> bool {
    (minimum.x..=maximum.x).contains(&position.x)
        && (minimum.y..=maximum.y).contains(&position.y)
        && (minimum.z..=maximum.z).contains(&position.z)
}

fn mode_removes(mode: CutawayModeV1, position: CellPositionV1) -> bool {
    match mode {
        CutawayModeV1::Off => false,
        CutawayModeV1::Layer { maximum_visible_z } => position.z > maximum_visible_z,
        CutawayModeV1::VerticalPlane {
            axis,
            coordinate,
            retain_less_equal,
        } => {
            let component = match axis {
                CutawayAxisV1::X => position.x,
                CutawayAxisV1::Y => position.y,
            };
            if retain_less_equal {
                component > coordinate
            } else {
                component < coordinate
            }
        },
        CutawayModeV1::BoundedVolume { minimum, maximum } => inside(position, minimum, maximum),
    }
}

fn terrain_root(cells: &[TerrainCellV1]) -> Result<CutawayDigestV1, CutawayErrorV1> {
    let mut payload = Vec::new();
    payload.extend_from_slice(
        &u32::try_from(cells.len())
            .map_err(|_| CutawayErrorV1::OversizedInput)?
            .to_le_bytes(),
    );
    for cell in cells {
        encode_position(&mut payload, cell.position);
        payload.extend_from_slice(&cell.material.to_le_bytes());
        payload.push(u8::from(cell.filled));
        payload.push(u8::from(cell.reveal_eligible));
        payload.extend_from_slice(&cell.content_digest);
    }
    domain_hash_v1("bastion/r1e/terrain-slice-input", 1, 0, &payload)
        .map_err(|_| CutawayErrorV1::Hash)
}

fn geometry_digest(
    policy: &CutawayPolicyV1,
    terrain: &TerrainSliceInputV1,
    terrain_root: CutawayDigestV1,
    removed: &[CellPositionV1],
    faces: &[CapFaceV1],
    roof_removals: u32,
    wall_removals: u32,
) -> Result<CutawayDigestV1, CutawayErrorV1> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&policy.policy_digest);
    payload.extend_from_slice(&terrain_root);
    payload.extend_from_slice(&terrain.terrain_revision.to_le_bytes());
    payload.extend_from_slice(&roof_removals.to_le_bytes());
    payload.extend_from_slice(&wall_removals.to_le_bytes());
    payload.extend_from_slice(
        &u32::try_from(removed.len())
            .map_err(|_| CutawayErrorV1::OversizedInput)?
            .to_le_bytes(),
    );
    for position in removed {
        encode_position(&mut payload, *position);
    }
    payload.extend_from_slice(
        &u32::try_from(faces.len())
            .map_err(|_| CutawayErrorV1::OversizedInput)?
            .to_le_bytes(),
    );
    for face in faces {
        encode_position(&mut payload, face.retained_cell);
        payload.push(face.direction as u8);
        payload.extend_from_slice(&face.material.to_le_bytes());
        for vertex in face.vertices {
            for component in vertex {
                payload.extend_from_slice(&component.to_le_bytes());
            }
        }
    }
    domain_hash_v1("bastion/r1e/cutaway-geometry", 1, 0, &payload).map_err(|_| CutawayErrorV1::Hash)
}

fn face_vertices(
    cell: CellPositionV1,
    direction: FaceDirectionV1,
) -> Result<[[i32; 3]; 4], CutawayErrorV1> {
    let x0 = cell.x;
    let y0 = cell.y;
    let z0 = cell.z;
    let x1 = x0.checked_add(1).ok_or(CutawayErrorV1::Overflow)?;
    let y1 = y0.checked_add(1).ok_or(CutawayErrorV1::Overflow)?;
    let z1 = z0.checked_add(1).ok_or(CutawayErrorV1::Overflow)?;
    Ok(match direction {
        FaceDirectionV1::NegativeX => [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
        FaceDirectionV1::PositiveX => [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
        FaceDirectionV1::NegativeY => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        FaceDirectionV1::PositiveY => [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
        FaceDirectionV1::NegativeZ => [[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
        FaceDirectionV1::PositiveZ => [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
    })
}

fn encode_position(output: &mut Vec<u8>, position: CellPositionV1) {
    output.extend_from_slice(&position.x.to_le_bytes());
    output.extend_from_slice(&position.y.to_le_bytes());
    output.extend_from_slice(&position.z.to_le_bytes());
}

struct ReaderV1<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ReaderV1<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, offset: 0 } }

    fn take(&mut self, count: usize) -> Result<&'a [u8], CutawayErrorV1> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(CutawayErrorV1::Overflow)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(CutawayErrorV1::MalformedEncoding)?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, CutawayErrorV1> { Ok(self.take(1)?[0]) }

    fn u16(&mut self) -> Result<u16, CutawayErrorV1> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| CutawayErrorV1::MalformedEncoding)?,
        ))
    }

    fn u32(&mut self) -> Result<u32, CutawayErrorV1> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| CutawayErrorV1::MalformedEncoding)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, CutawayErrorV1> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| CutawayErrorV1::MalformedEncoding)?,
        ))
    }

    fn i32(&mut self) -> Result<i32, CutawayErrorV1> {
        Ok(i32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| CutawayErrorV1::MalformedEncoding)?,
        ))
    }

    fn digest(&mut self) -> Result<CutawayDigestV1, CutawayErrorV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| CutawayErrorV1::MalformedEncoding)
    }

    fn position(&mut self) -> Result<CellPositionV1, CutawayErrorV1> {
        Ok(CellPositionV1::new(self.i32()?, self.i32()?, self.i32()?))
    }

    fn at_end(&self) -> bool { self.offset == self.bytes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn positions() -> Vec<CellPositionV1> {
        let mut cells = Vec::new();
        for z in 0..=2 {
            for y in 0..=1 {
                for x in 0..=1 {
                    cells.push(CellPositionV1::new(x, y, z));
                }
            }
        }
        cells
    }

    fn transition(kind: CutawayTransitionKindV1) -> CutawayTransitionV1 {
        if kind == CutawayTransitionKindV1::Off {
            CutawayTransitionV1 {
                kind,
                step: 0,
                total_steps: 0,
            }
        } else {
            CutawayTransitionV1 {
                kind,
                step: 2,
                total_steps: 4,
            }
        }
    }

    fn policy(mode: CutawayModeV1) -> CutawayPolicyV1 {
        CutawayPolicyV1::new(
            CutawayPolicyInputV1 {
                presentation_generation: 7,
                terrain_generation: digest(2),
                camera_token: digest(3),
                camera_sequence: 11,
                mode,
                transition: transition(if matches!(mode, CutawayModeV1::Off) {
                    CutawayTransitionKindV1::Off
                } else {
                    CutawayTransitionKindV1::Held
                }),
                cap_material: 9,
                reveal_authority: digest(4),
            },
            positions(),
        )
        .unwrap()
    }

    fn terrain() -> TerrainSliceInputV1 {
        TerrainSliceInputV1 {
            presentation_generation: 7,
            terrain_generation: digest(2),
            terrain_revision: 19,
            camera_token: digest(3),
            camera_sequence: 11,
            bounds_minimum: CellPositionV1::new(0, 0, 0),
            bounds_maximum: CellPositionV1::new(1, 1, 2),
            cells: positions()
                .into_iter()
                .enumerate()
                .map(|(index, position)| TerrainCellV1 {
                    position,
                    material: 1,
                    filled: true,
                    reveal_eligible: true,
                    content_digest: digest(u8::try_from(index + 10).unwrap()),
                })
                .collect(),
        }
    }

    #[test]
    fn policy_bytes_are_frozen_order_independent_and_exact() {
        let a = policy(CutawayModeV1::Layer {
            maximum_visible_z: 1,
        });
        let mut reversed = positions();
        reversed.reverse();
        let b = CutawayPolicyV1::new(
            CutawayPolicyInputV1 {
                presentation_generation: 7,
                terrain_generation: digest(2),
                camera_token: digest(3),
                camera_sequence: 11,
                mode: CutawayModeV1::Layer {
                    maximum_visible_z: 1,
                },
                transition: transition(CutawayTransitionKindV1::Held),
                cap_material: 9,
                reveal_authority: digest(4),
            },
            reversed,
        )
        .unwrap();
        assert_eq!(a, b);
        let bytes = a.canonical_bytes().unwrap();
        assert_eq!(CutawayPolicyV1::decode_exact(&bytes).unwrap(), a);
        assert_eq!(
            crate::hex32(&a.policy_digest()),
            "bdf2a6fe83a2f6698e6be6a4221fc500034d40ac78d0625eace975859453d078"
        );
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            CutawayPolicyV1::decode_exact(&trailing),
            Err(CutawayErrorV1::TrailingBytes)
        );
        let truncated = &bytes[..bytes.len() - 1];
        assert_eq!(
            CutawayPolicyV1::decode_exact(truncated),
            Err(CutawayErrorV1::MalformedEncoding)
        );
    }

    #[test]
    fn mode_boundaries_and_off_parity_are_exact() {
        let off = derive_cutaway_geometry_v1(&policy(CutawayModeV1::Off), terrain()).unwrap();
        assert!(off.surface_passthrough);
        assert!(off.removed_cells.is_empty());
        assert!(off.cap_faces.is_empty());
        let layer = derive_cutaway_geometry_v1(
            &policy(CutawayModeV1::Layer {
                maximum_visible_z: 1,
            }),
            terrain(),
        )
        .unwrap();
        assert_eq!(layer.removed_cells.len(), 4);
        assert!(layer.removed_cells.iter().all(|cell| cell.z == 2));
        assert_eq!(layer.cap_faces.len(), 4);
        assert!(
            layer
                .cap_faces
                .iter()
                .all(|face| face.direction == FaceDirectionV1::PositiveZ)
        );
        assert_eq!(layer.roof_removals, 4);
        assert_eq!(layer.wall_removals, 0);
    }

    #[test]
    fn vertical_plane_and_bounded_volume_vectors_are_canonical() {
        let plane = derive_cutaway_geometry_v1(
            &policy(CutawayModeV1::VerticalPlane {
                axis: CutawayAxisV1::X,
                coordinate: 0,
                retain_less_equal: true,
            }),
            terrain(),
        )
        .unwrap();
        assert_eq!(plane.removed_cells.len(), 6);
        assert_eq!(plane.cap_faces.len(), 6);
        assert!(
            plane
                .cap_faces
                .iter()
                .all(|face| face.direction == FaceDirectionV1::PositiveX)
        );
        let volume = derive_cutaway_geometry_v1(
            &policy(CutawayModeV1::BoundedVolume {
                minimum: CellPositionV1::new(1, 0, 1),
                maximum: CellPositionV1::new(1, 1, 2),
            }),
            terrain(),
        )
        .unwrap();
        assert_eq!(volume.removed_cells.len(), 4);
        assert_eq!(volume.cap_faces.len(), 6);
    }

    #[test]
    fn input_permutation_and_replay_are_identical() {
        let policy = policy(CutawayModeV1::Layer {
            maximum_visible_z: 1,
        });
        let a = derive_cutaway_geometry_v1(&policy, terrain()).unwrap();
        let mut reverse = terrain();
        reverse.cells.reverse();
        let b = derive_cutaway_geometry_v1(&policy, reverse).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            crate::hex32(&a.geometry_digest),
            "a78ce4e1998941534a3151609d6178d333380693bc7171d0e50f1c7f91697fd9"
        );
    }

    #[test]
    fn cap_winding_normals_material_and_uniqueness_are_exact() {
        let result = derive_cutaway_geometry_v1(
            &policy(CutawayModeV1::Layer {
                maximum_visible_z: 1,
            }),
            terrain(),
        )
        .unwrap();
        for face in &result.cap_faces {
            assert_eq!(face.material, 9);
            assert_eq!(face.normal, [0, 0, 1]);
            let a = face.vertices[0];
            let b = face.vertices[1];
            let c = face.vertices[2];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let cross = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            assert_eq!(cross, [0, 0, 1]);
        }
        assert!(result.cap_faces.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn unauthorized_duplicate_invalid_and_oversize_inputs_fail_closed() {
        let mut duplicate = positions();
        duplicate.push(duplicate[0]);
        assert_eq!(
            CutawayPolicyV1::new(
                CutawayPolicyInputV1 {
                    presentation_generation: 7,
                    terrain_generation: digest(2),
                    camera_token: digest(3),
                    camera_sequence: 11,
                    mode: CutawayModeV1::Layer {
                        maximum_visible_z: 1
                    },
                    transition: transition(CutawayTransitionKindV1::Held),
                    cap_material: 9,
                    reveal_authority: digest(4),
                },
                duplicate,
            ),
            Err(CutawayErrorV1::DuplicateAuthorization)
        );
        let mut unauthorized = terrain();
        unauthorized.cells[0].reveal_eligible = false;
        assert_eq!(
            derive_cutaway_geometry_v1(
                &policy(CutawayModeV1::Layer {
                    maximum_visible_z: 1
                }),
                unauthorized,
            ),
            Err(CutawayErrorV1::UnauthorizedReveal)
        );
        let mut duplicate_cell = terrain();
        duplicate_cell.cells.push(duplicate_cell.cells[0]);
        assert_eq!(
            derive_cutaway_geometry_v1(
                &policy(CutawayModeV1::Layer {
                    maximum_visible_z: 1
                }),
                duplicate_cell,
            ),
            Err(CutawayErrorV1::DuplicateCell)
        );
        assert_eq!(
            validate_bounds(CellPositionV1::new(2, 0, 0), CellPositionV1::new(1, 1, 1)),
            Err(CutawayErrorV1::InvalidBounds)
        );
    }

    #[test]
    fn stale_generation_terrain_edit_and_camera_supersession_reject() {
        let policy = policy(CutawayModeV1::Layer {
            maximum_visible_z: 1,
        });
        let result = derive_cutaway_geometry_v1(&policy, terrain()).unwrap();
        let mut publisher = CutawayPublisherV1::default();
        let held = publisher.publish(result.key(), result.clone()).unwrap();
        let mut stale = terrain();
        stale.terrain_revision = 18;
        let stale_result = derive_cutaway_geometry_v1(&policy, stale).unwrap();
        assert_eq!(
            publisher.publish(stale_result.key(), stale_result),
            Err(CutawayErrorV1::SupersededWork)
        );
        let mut wrong_generation = terrain();
        wrong_generation.terrain_generation = digest(99);
        assert_eq!(
            derive_cutaway_geometry_v1(&policy, wrong_generation),
            Err(CutawayErrorV1::StaleGeneration)
        );
        let mut wrong_camera = terrain();
        wrong_camera.camera_sequence = 12;
        assert_eq!(
            derive_cutaway_geometry_v1(&policy, wrong_camera),
            Err(CutawayErrorV1::StaleGeneration)
        );
        assert_eq!(publisher.acquire().unwrap(), held);
    }

    #[test]
    fn publisher_rejects_incomplete_or_duplicate_geometry_without_replacing_visible() {
        let policy = policy(CutawayModeV1::Layer {
            maximum_visible_z: 1,
        });
        let result = derive_cutaway_geometry_v1(&policy, terrain()).unwrap();
        let mut publisher = CutawayPublisherV1::default();
        let held = publisher.publish(result.key(), result.clone()).unwrap();

        let mut incomplete = result.clone();
        incomplete.presentation_generation += 1;
        incomplete.removed_cells.clear();
        assert_eq!(
            publisher.publish(incomplete.key(), incomplete),
            Err(CutawayErrorV1::InvalidGeometry)
        );

        let mut duplicate = result;
        duplicate.presentation_generation += 2;
        duplicate.cap_faces.push(duplicate.cap_faces[0].clone());
        duplicate.cap_faces.sort_unstable();
        assert_eq!(
            publisher.publish(duplicate.key(), duplicate),
            Err(CutawayErrorV1::InvalidGeometry)
        );
        assert_eq!(publisher.acquire().unwrap(), held);
    }

    #[test]
    fn multi_layer_transition_changes_policy_and_geometry_without_clock_input() {
        let low = policy(CutawayModeV1::Layer {
            maximum_visible_z: 0,
        });
        let high = policy(CutawayModeV1::Layer {
            maximum_visible_z: 1,
        });
        assert_ne!(low.policy_digest(), high.policy_digest());
        let a = derive_cutaway_geometry_v1(&low, terrain()).unwrap();
        let b = derive_cutaway_geometry_v1(&high, terrain()).unwrap();
        assert_eq!(a.removed_cells.len(), 8);
        assert_eq!(b.removed_cells.len(), 4);
        assert_ne!(a.geometry_digest, b.geometry_digest);
    }
}
