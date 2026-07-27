//! Renderer-owned moving-local-space snapshot.
//!
//! This module never discovers gameplay membership. It validates and publishes
//! a bounded canonical projection supplied by an authoritative adapter.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::{DomainHashErrorV1, domain_hash_v1};

pub const RENDER_ISLAND_VERSION_V1: u16 = 1;
pub const MAX_RENDER_ISLAND_BYTES_V1: usize = 1024 * 1024;
pub const MAX_RENDER_ISLANDS_V1: usize = 128;
pub const MAX_RENDER_ISLAND_MEMBERS_V1: usize = 8_192;
pub const MAX_RENDER_ISLAND_PORTALS_V1: usize = 4_096;
pub const MAX_RENDER_ISLAND_COORDINATE_V1: i64 = 1_000_000_000_000;
pub const MAX_RENDER_ISLAND_PORTAL_COORDINATE_V1: i32 = 1_000_000;
pub const MAX_RENDER_ISLAND_SCALE_MILLI_V1: u32 = 100_000;
pub const Q30_ONE_V1: i32 = 1 << 30;
const Q30_NORM_SQUARED_V1: i128 = 1_i128 << 60;
const Q30_NORM_TOLERANCE_V1: i128 = 1_i128 << 52;
const MAX_SUPPORTED_NESTING_V1: usize = 1;
const MAGIC: &[u8; 8] = b"BASR1IS1";
const SEALED_TAG: u8 = 1;

pub type RenderIslandDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RenderIslandTransformV1 {
    pub translation_mm: [i64; 3],
    pub orientation_q30: [i32; 4],
    pub scale_milli: u32,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RenderIslandNodeV1 {
    pub semantic_id: RenderIslandDigestV1,
    pub parent_island: Option<RenderIslandDigestV1>,
    pub parent_transform: RenderIslandTransformV1,
    pub member_ids: Vec<RenderIslandDigestV1>,
    pub portal_cells: Vec<[i32; 3]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderIslandInputV1 {
    pub presentation_generation: u64,
    pub island_generation: u64,
    pub publication_sequence: u64,
    pub presentation_frame_digest: RenderIslandDigestV1,
    pub interior_snapshot_digest: RenderIslandDigestV1,
    pub cutaway_policy_digest: RenderIslandDigestV1,
    pub nodes: Vec<RenderIslandNodeV1>,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderIslandV1 {
    presentation_generation: u64,
    island_generation: u64,
    publication_sequence: u64,
    presentation_frame_digest: RenderIslandDigestV1,
    interior_snapshot_digest: RenderIslandDigestV1,
    cutaway_policy_digest: RenderIslandDigestV1,
    nodes: Vec<RenderIslandNodeV1>,
    canonical_bytes: Vec<u8>,
    snapshot_digest: RenderIslandDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderIslandErrorV1 {
    UnsupportedVersion(u16),
    InvalidMagic,
    UnsealedOrPartial,
    InvalidGeneration,
    InvalidBinding,
    InvalidIslandId,
    InvalidTransform,
    InvalidPortalCell,
    TooManyIslands(usize),
    TooManyMembers(usize),
    TooManyPortals(usize),
    DuplicateIsland(RenderIslandDigestV1),
    DuplicateMember(RenderIslandDigestV1),
    DuplicatePortal([i32; 3]),
    MissingParent(RenderIslandDigestV1),
    Cycle(RenderIslandDigestV1),
    UnsupportedNesting(usize),
    NonCanonicalOrder,
    Truncated,
    TrailingBytes(usize),
    MalformedOptionalTag(u8),
    DigestMismatch,
    EncodedSizeExceeded(usize),
    SizeOverflow,
    AllocationFailure,
    StaleOrEqualPublication {
        current_generation: u64,
        current_sequence: u64,
        offered_generation: u64,
        offered_sequence: u64,
    },
    Hash(DomainHashErrorV1),
}

#[derive(Clone, Debug, Default)]
pub struct RenderIslandPublicationV1 {
    current: Option<Arc<RenderIslandV1>>,
}

impl RenderIslandV1 {
    pub fn seal(mut input: RenderIslandInputV1) -> Result<Self, RenderIslandErrorV1> {
        validate_header(&input)?;
        validate_counts(&input.nodes)?;
        for node in &mut input.nodes {
            validate_node(node)?;
            node.member_ids.sort_unstable();
            reject_duplicate_members(&node.member_ids)?;
            node.portal_cells.sort_unstable();
            reject_duplicate_portals(&node.portal_cells)?;
        }
        input.nodes.sort_unstable_by_key(|node| node.semantic_id);
        reject_duplicate_nodes(&input.nodes)?;
        validate_unique_members(&input.nodes)?;
        validate_parent_graph(&input.nodes)?;

        let mut value = Self {
            presentation_generation: input.presentation_generation,
            island_generation: input.island_generation,
            publication_sequence: input.publication_sequence,
            presentation_frame_digest: input.presentation_frame_digest,
            interior_snapshot_digest: input.interior_snapshot_digest,
            cutaway_policy_digest: input.cutaway_policy_digest,
            nodes: input.nodes,
            canonical_bytes: Vec::new(),
            snapshot_digest: [0; 32],
        };
        let prefix = value.encode_prefix()?;
        value.snapshot_digest = domain_hash_v1("bastion/r1e/render-island", 1, 0, &prefix)
            .map_err(RenderIslandErrorV1::Hash)?;
        value.canonical_bytes = prefix;
        value
            .canonical_bytes
            .extend_from_slice(&value.snapshot_digest);
        if value.canonical_bytes.len() > MAX_RENDER_ISLAND_BYTES_V1 {
            return Err(RenderIslandErrorV1::EncodedSizeExceeded(
                value.canonical_bytes.len(),
            ));
        }
        Ok(value)
    }

    #[must_use]
    pub const fn presentation_generation(&self) -> u64 { self.presentation_generation }

    #[must_use]
    pub const fn island_generation(&self) -> u64 { self.island_generation }

    #[must_use]
    pub const fn publication_sequence(&self) -> u64 { self.publication_sequence }

    #[must_use]
    pub const fn presentation_frame_digest(&self) -> RenderIslandDigestV1 {
        self.presentation_frame_digest
    }

    #[must_use]
    pub const fn interior_snapshot_digest(&self) -> RenderIslandDigestV1 {
        self.interior_snapshot_digest
    }

    #[must_use]
    pub const fn cutaway_policy_digest(&self) -> RenderIslandDigestV1 { self.cutaway_policy_digest }

    #[must_use]
    pub fn nodes(&self) -> &[RenderIslandNodeV1] { &self.nodes }

    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] { &self.canonical_bytes }

    #[must_use]
    pub const fn snapshot_digest(&self) -> RenderIslandDigestV1 { self.snapshot_digest }

    #[must_use]
    pub fn island_for_member(&self, member: RenderIslandDigestV1) -> Option<RenderIslandDigestV1> {
        self.nodes.iter().find_map(|node| {
            node.member_ids
                .binary_search(&member)
                .is_ok()
                .then_some(node.semantic_id)
        })
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, RenderIslandErrorV1> {
        if bytes.len() > MAX_RENDER_ISLAND_BYTES_V1 {
            return Err(RenderIslandErrorV1::EncodedSizeExceeded(bytes.len()));
        }
        let mut reader = Reader::new(bytes);
        if reader.take(8)? != MAGIC {
            return Err(RenderIslandErrorV1::InvalidMagic);
        }
        let version = reader.u16()?;
        if version != RENDER_ISLAND_VERSION_V1 {
            return Err(RenderIslandErrorV1::UnsupportedVersion(version));
        }
        if reader.u8()? != SEALED_TAG {
            return Err(RenderIslandErrorV1::UnsealedOrPartial);
        }
        if reader.u8()? != 0 {
            return Err(RenderIslandErrorV1::NonCanonicalOrder);
        }
        let presentation_generation = reader.u64()?;
        let island_generation = reader.u64()?;
        let publication_sequence = reader.u64()?;
        let presentation_frame_digest = reader.digest()?;
        let interior_snapshot_digest = reader.digest()?;
        let cutaway_policy_digest = reader.digest()?;
        let node_count = reader.count(MAX_RENDER_ISLANDS_V1)?;
        let mut nodes = try_vec(node_count)?;
        for _ in 0..node_count {
            let semantic_id = reader.digest()?;
            let parent_island = match reader.u8()? {
                0 => None,
                1 => Some(reader.digest()?),
                value => return Err(RenderIslandErrorV1::MalformedOptionalTag(value)),
            };
            let parent_transform = RenderIslandTransformV1 {
                translation_mm: [reader.i64()?, reader.i64()?, reader.i64()?],
                orientation_q30: [reader.i32()?, reader.i32()?, reader.i32()?, reader.i32()?],
                scale_milli: reader.u32()?,
            };
            let member_count = reader.count(MAX_RENDER_ISLAND_MEMBERS_V1)?;
            let mut member_ids = try_vec(member_count)?;
            for _ in 0..member_count {
                member_ids.push(reader.digest()?);
            }
            let portal_count = reader.count(MAX_RENDER_ISLAND_PORTALS_V1)?;
            let mut portal_cells = try_vec(portal_count)?;
            for _ in 0..portal_count {
                portal_cells.push([reader.i32()?, reader.i32()?, reader.i32()?]);
            }
            nodes.push(RenderIslandNodeV1 {
                semantic_id,
                parent_island,
                parent_transform,
                member_ids,
                portal_cells,
            });
        }
        let encoded_digest = reader.digest()?;
        reader.finish()?;
        let rebuilt = Self::seal(RenderIslandInputV1 {
            presentation_generation,
            island_generation,
            publication_sequence,
            presentation_frame_digest,
            interior_snapshot_digest,
            cutaway_policy_digest,
            nodes,
            complete: true,
        })?;
        if rebuilt.snapshot_digest != encoded_digest {
            return Err(RenderIslandErrorV1::DigestMismatch);
        }
        if rebuilt.canonical_bytes != bytes {
            return Err(RenderIslandErrorV1::NonCanonicalOrder);
        }
        Ok(rebuilt)
    }

    fn encode_prefix(&self) -> Result<Vec<u8>, RenderIslandErrorV1> {
        let mut output = Vec::new();
        output
            .try_reserve(196)
            .map_err(|_| RenderIslandErrorV1::AllocationFailure)?;
        output.extend_from_slice(MAGIC);
        put_u16(&mut output, RENDER_ISLAND_VERSION_V1);
        output.push(SEALED_TAG);
        output.push(0);
        put_u64(&mut output, self.presentation_generation);
        put_u64(&mut output, self.island_generation);
        put_u64(&mut output, self.publication_sequence);
        output.extend_from_slice(&self.presentation_frame_digest);
        output.extend_from_slice(&self.interior_snapshot_digest);
        output.extend_from_slice(&self.cutaway_policy_digest);
        put_count(&mut output, self.nodes.len())?;
        for node in &self.nodes {
            output.extend_from_slice(&node.semantic_id);
            match node.parent_island {
                None => output.push(0),
                Some(parent) => {
                    output.push(1);
                    output.extend_from_slice(&parent);
                },
            }
            for value in node.parent_transform.translation_mm {
                output.extend_from_slice(&value.to_le_bytes());
            }
            for value in node.parent_transform.orientation_q30 {
                output.extend_from_slice(&value.to_le_bytes());
            }
            put_u32(&mut output, node.parent_transform.scale_milli);
            put_count(&mut output, node.member_ids.len())?;
            for member in &node.member_ids {
                output.extend_from_slice(member);
            }
            put_count(&mut output, node.portal_cells.len())?;
            for cell in &node.portal_cells {
                for value in cell {
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
            if output.len() > MAX_RENDER_ISLAND_BYTES_V1.saturating_sub(32) {
                return Err(RenderIslandErrorV1::EncodedSizeExceeded(output.len()));
            }
        }
        Ok(output)
    }
}

impl RenderIslandPublicationV1 {
    pub fn publish(
        &mut self,
        snapshot: RenderIslandV1,
    ) -> Result<Arc<RenderIslandV1>, RenderIslandErrorV1> {
        if let Some(current) = &self.current {
            let current_key = (current.island_generation, current.publication_sequence);
            let offered_key = (snapshot.island_generation, snapshot.publication_sequence);
            if offered_key <= current_key {
                return Err(RenderIslandErrorV1::StaleOrEqualPublication {
                    current_generation: current_key.0,
                    current_sequence: current_key.1,
                    offered_generation: offered_key.0,
                    offered_sequence: offered_key.1,
                });
            }
        }
        let snapshot = Arc::new(snapshot);
        self.current = Some(Arc::clone(&snapshot));
        Ok(snapshot)
    }

    #[must_use]
    pub fn acquire(&self) -> Option<Arc<RenderIslandV1>> { self.current.clone() }
}

fn validate_header(input: &RenderIslandInputV1) -> Result<(), RenderIslandErrorV1> {
    if !input.complete {
        return Err(RenderIslandErrorV1::UnsealedOrPartial);
    }
    if input.presentation_generation == 0
        || input.island_generation == 0
        || input.publication_sequence == 0
    {
        return Err(RenderIslandErrorV1::InvalidGeneration);
    }
    if is_zero(&input.presentation_frame_digest)
        || is_zero(&input.interior_snapshot_digest)
        || is_zero(&input.cutaway_policy_digest)
    {
        return Err(RenderIslandErrorV1::InvalidBinding);
    }
    Ok(())
}

fn validate_counts(nodes: &[RenderIslandNodeV1]) -> Result<(), RenderIslandErrorV1> {
    if nodes.len() > MAX_RENDER_ISLANDS_V1 {
        return Err(RenderIslandErrorV1::TooManyIslands(nodes.len()));
    }
    let members = nodes.iter().try_fold(0_usize, |total, node| {
        total
            .checked_add(node.member_ids.len())
            .ok_or(RenderIslandErrorV1::SizeOverflow)
    })?;
    if members > MAX_RENDER_ISLAND_MEMBERS_V1 {
        return Err(RenderIslandErrorV1::TooManyMembers(members));
    }
    let portals = nodes.iter().try_fold(0_usize, |total, node| {
        total
            .checked_add(node.portal_cells.len())
            .ok_or(RenderIslandErrorV1::SizeOverflow)
    })?;
    if portals > MAX_RENDER_ISLAND_PORTALS_V1 {
        return Err(RenderIslandErrorV1::TooManyPortals(portals));
    }
    Ok(())
}

fn validate_node(node: &RenderIslandNodeV1) -> Result<(), RenderIslandErrorV1> {
    if is_zero(&node.semantic_id) {
        return Err(RenderIslandErrorV1::InvalidIslandId);
    }
    if node.member_ids.iter().any(is_zero) {
        return Err(RenderIslandErrorV1::InvalidIslandId);
    }
    validate_transform(node.parent_transform)?;
    if node.portal_cells.iter().flatten().any(|value| {
        value
            .checked_abs()
            .is_none_or(|value| value > MAX_RENDER_ISLAND_PORTAL_COORDINATE_V1)
    }) {
        return Err(RenderIslandErrorV1::InvalidPortalCell);
    }
    Ok(())
}

fn validate_transform(value: RenderIslandTransformV1) -> Result<(), RenderIslandErrorV1> {
    if value.translation_mm.iter().any(|component| {
        component
            .checked_abs()
            .is_none_or(|component| component > MAX_RENDER_ISLAND_COORDINATE_V1)
    }) || value.scale_milli == 0
        || value.scale_milli > MAX_RENDER_ISLAND_SCALE_MILLI_V1
        || value.orientation_q30.iter().any(|component| {
            component
                .checked_abs()
                .is_none_or(|value| value > Q30_ONE_V1)
        })
    {
        return Err(RenderIslandErrorV1::InvalidTransform);
    }
    let norm = value
        .orientation_q30
        .iter()
        .map(|component| i128::from(*component) * i128::from(*component))
        .sum::<i128>();
    if (norm - Q30_NORM_SQUARED_V1).abs() > Q30_NORM_TOLERANCE_V1 {
        return Err(RenderIslandErrorV1::InvalidTransform);
    }
    Ok(())
}

fn reject_duplicate_nodes(nodes: &[RenderIslandNodeV1]) -> Result<(), RenderIslandErrorV1> {
    if let Some(pair) = nodes
        .windows(2)
        .find(|pair| pair[0].semantic_id == pair[1].semantic_id)
    {
        return Err(RenderIslandErrorV1::DuplicateIsland(pair[0].semantic_id));
    }
    Ok(())
}

fn reject_duplicate_members(members: &[RenderIslandDigestV1]) -> Result<(), RenderIslandErrorV1> {
    if let Some(pair) = members.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(RenderIslandErrorV1::DuplicateMember(pair[0]));
    }
    Ok(())
}

fn reject_duplicate_portals(portals: &[[i32; 3]]) -> Result<(), RenderIslandErrorV1> {
    if let Some(pair) = portals.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(RenderIslandErrorV1::DuplicatePortal(pair[0]));
    }
    Ok(())
}

fn validate_unique_members(nodes: &[RenderIslandNodeV1]) -> Result<(), RenderIslandErrorV1> {
    let mut seen = BTreeSet::new();
    for member in nodes.iter().flat_map(|node| &node.member_ids) {
        if !seen.insert(*member) {
            return Err(RenderIslandErrorV1::DuplicateMember(*member));
        }
    }
    Ok(())
}

fn validate_parent_graph(nodes: &[RenderIslandNodeV1]) -> Result<(), RenderIslandErrorV1> {
    let by_id = nodes
        .iter()
        .map(|node| (node.semantic_id, node.parent_island))
        .collect::<BTreeMap<_, _>>();
    for node in nodes {
        let mut cursor = node.semantic_id;
        let mut visited = BTreeSet::new();
        let mut depth = 0_usize;
        loop {
            if !visited.insert(cursor) {
                return Err(RenderIslandErrorV1::Cycle(cursor));
            }
            let parent = by_id
                .get(&cursor)
                .copied()
                .ok_or(RenderIslandErrorV1::MissingParent(cursor))?;
            let Some(parent) = parent else {
                break;
            };
            if !by_id.contains_key(&parent) {
                return Err(RenderIslandErrorV1::MissingParent(parent));
            }
            depth = depth
                .checked_add(1)
                .ok_or(RenderIslandErrorV1::SizeOverflow)?;
            cursor = parent;
        }
        if depth > MAX_SUPPORTED_NESTING_V1 {
            return Err(RenderIslandErrorV1::UnsupportedNesting(depth));
        }
    }
    Ok(())
}

fn is_zero(value: &RenderIslandDigestV1) -> bool { *value == [0; 32] }

fn try_vec<T>(count: usize) -> Result<Vec<T>, RenderIslandErrorV1> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| RenderIslandErrorV1::AllocationFailure)?;
    Ok(values)
}

fn put_u16(output: &mut Vec<u8>, value: u16) { output.extend_from_slice(&value.to_le_bytes()); }

fn put_u32(output: &mut Vec<u8>, value: u32) { output.extend_from_slice(&value.to_le_bytes()); }

fn put_u64(output: &mut Vec<u8>, value: u64) { output.extend_from_slice(&value.to_le_bytes()); }

fn put_count(output: &mut Vec<u8>, value: usize) -> Result<(), RenderIslandErrorV1> {
    let value = u32::try_from(value).map_err(|_| RenderIslandErrorV1::SizeOverflow)?;
    put_u32(output, value);
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, cursor: 0 } }

    fn take(&mut self, length: usize) -> Result<&'a [u8], RenderIslandErrorV1> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(RenderIslandErrorV1::SizeOverflow)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(RenderIslandErrorV1::Truncated)?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, RenderIslandErrorV1> {
        Ok(*self
            .take(1)?
            .first()
            .ok_or(RenderIslandErrorV1::Truncated)?)
    }

    fn u16(&mut self) -> Result<u16, RenderIslandErrorV1> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| RenderIslandErrorV1::Truncated)?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, RenderIslandErrorV1> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| RenderIslandErrorV1::Truncated)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn i32(&mut self) -> Result<i32, RenderIslandErrorV1> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| RenderIslandErrorV1::Truncated)?;
        Ok(i32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, RenderIslandErrorV1> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| RenderIslandErrorV1::Truncated)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn i64(&mut self) -> Result<i64, RenderIslandErrorV1> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| RenderIslandErrorV1::Truncated)?;
        Ok(i64::from_le_bytes(bytes))
    }

    fn digest(&mut self) -> Result<RenderIslandDigestV1, RenderIslandErrorV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| RenderIslandErrorV1::Truncated)
    }

    fn count(&mut self, maximum: usize) -> Result<usize, RenderIslandErrorV1> {
        let count = usize::try_from(self.u32()?).map_err(|_| RenderIslandErrorV1::SizeOverflow)?;
        if count > maximum {
            return Err(RenderIslandErrorV1::EncodedSizeExceeded(count));
        }
        Ok(count)
    }

    fn finish(self) -> Result<(), RenderIslandErrorV1> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(RenderIslandErrorV1::TrailingBytes(
                self.bytes.len() - self.cursor,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> RenderIslandDigestV1 { [value; 32] }

    fn transform(x: i64) -> RenderIslandTransformV1 {
        RenderIslandTransformV1 {
            translation_mm: [x, 2_000, 3_000],
            orientation_q30: [0, 0, 0, Q30_ONE_V1],
            scale_milli: 1_000,
        }
    }

    fn node(id: u8, members: &[u8]) -> RenderIslandNodeV1 {
        RenderIslandNodeV1 {
            semantic_id: digest(id),
            parent_island: None,
            parent_transform: transform(i64::from(id)),
            member_ids: members.iter().map(|value| digest(*value)).collect(),
            portal_cells: vec![[i32::from(id), 2, 3]],
        }
    }

    fn input() -> RenderIslandInputV1 {
        RenderIslandInputV1 {
            presentation_generation: 7,
            island_generation: 9,
            publication_sequence: 1,
            presentation_frame_digest: digest(1),
            interior_snapshot_digest: digest(2),
            cutaway_policy_digest: digest(3),
            nodes: vec![node(20, &[41, 40]), node(10, &[31, 30])],
            complete: true,
        }
    }

    fn hex(value: RenderIslandDigestV1) -> String {
        value.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn frozen_snapshot_digest() {
        assert_eq!(
            hex(RenderIslandV1::seal(input()).unwrap().snapshot_digest()),
            "3575abb65a6b73e3f88e9cd51e9f71a67b2a3a39ba50ba8907b2f5aa2f4ac7a8"
        );
    }

    #[test]
    fn canonical_bytes_are_permutation_independent_and_exactly_decodable() {
        let first = RenderIslandV1::seal(input()).unwrap();
        let mut permuted = input();
        permuted.nodes.reverse();
        permuted.nodes[0].member_ids.reverse();
        let second = RenderIslandV1::seal(permuted).unwrap();
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(
            RenderIslandV1::decode_exact(first.canonical_bytes()).unwrap(),
            first
        );
    }

    #[test]
    fn movement_generation_and_membership_change_the_snapshot() {
        let base = RenderIslandV1::seal(input()).unwrap();
        let mut moved = input();
        moved.nodes[0].parent_transform.translation_mm[0] += 1;
        let moved = RenderIslandV1::seal(moved).unwrap();
        let mut generation = input();
        generation.island_generation += 1;
        let generation = RenderIslandV1::seal(generation).unwrap();
        let mut membership = input();
        membership.nodes[0].member_ids.push(digest(42));
        let membership = RenderIslandV1::seal(membership).unwrap();
        assert_ne!(base.snapshot_digest(), moved.snapshot_digest());
        assert_ne!(base.snapshot_digest(), generation.snapshot_digest());
        assert_ne!(base.snapshot_digest(), membership.snapshot_digest());
    }

    #[test]
    fn duplicate_and_invalid_inputs_fail_closed() {
        let mut duplicate_member = input();
        duplicate_member.nodes[1].member_ids.push(digest(30));
        assert!(matches!(
            RenderIslandV1::seal(duplicate_member),
            Err(RenderIslandErrorV1::DuplicateMember(value)) if value == digest(30)
        ));
        let mut duplicate_island = input();
        duplicate_island.nodes[1].semantic_id = duplicate_island.nodes[0].semantic_id;
        assert!(matches!(
            RenderIslandV1::seal(duplicate_island),
            Err(RenderIslandErrorV1::DuplicateIsland(_))
        ));
        let mut duplicate_portal = input();
        let repeated_portal = duplicate_portal.nodes[0].portal_cells[0];
        duplicate_portal.nodes[0].portal_cells.push(repeated_portal);
        assert!(matches!(
            RenderIslandV1::seal(duplicate_portal),
            Err(RenderIslandErrorV1::DuplicatePortal(_))
        ));
        let mut invalid_transform = input();
        invalid_transform.nodes[0].parent_transform.orientation_q30 = [0; 4];
        assert_eq!(
            RenderIslandV1::seal(invalid_transform),
            Err(RenderIslandErrorV1::InvalidTransform)
        );
        let mut invalid_portal = input();
        invalid_portal.nodes[0].portal_cells[0][0] = MAX_RENDER_ISLAND_PORTAL_COORDINATE_V1 + 1;
        assert_eq!(
            RenderIslandV1::seal(invalid_portal),
            Err(RenderIslandErrorV1::InvalidPortalCell)
        );
        let mut partial = input();
        partial.complete = false;
        assert_eq!(
            RenderIslandV1::seal(partial),
            Err(RenderIslandErrorV1::UnsealedOrPartial)
        );
    }

    #[test]
    fn missing_parent_cycle_and_unsupported_nesting_reject() {
        let mut missing = input();
        missing.nodes[0].parent_island = Some(digest(99));
        assert_eq!(
            RenderIslandV1::seal(missing),
            Err(RenderIslandErrorV1::MissingParent(digest(99)))
        );

        let mut cycle = input();
        cycle.nodes[0].parent_island = Some(cycle.nodes[1].semantic_id);
        cycle.nodes[1].parent_island = Some(cycle.nodes[0].semantic_id);
        assert!(matches!(
            RenderIslandV1::seal(cycle),
            Err(RenderIslandErrorV1::Cycle(_))
        ));

        let mut nested = input();
        nested.nodes.push(node(30, &[50]));
        nested
            .nodes
            .iter_mut()
            .find(|node| node.semantic_id == digest(20))
            .unwrap()
            .parent_island = Some(digest(10));
        nested
            .nodes
            .iter_mut()
            .find(|node| node.semantic_id == digest(30))
            .unwrap()
            .parent_island = Some(digest(20));
        assert_eq!(
            RenderIslandV1::seal(nested),
            Err(RenderIslandErrorV1::UnsupportedNesting(2))
        );
    }

    #[test]
    fn decode_rejects_truncation_trailing_and_noncanonical_order() {
        let snapshot = RenderIslandV1::seal(input()).unwrap();
        for length in 0..snapshot.canonical_bytes().len() {
            assert!(RenderIslandV1::decode_exact(&snapshot.canonical_bytes()[..length]).is_err());
        }
        let mut trailing = snapshot.canonical_bytes().to_vec();
        trailing.push(0);
        assert_eq!(
            RenderIslandV1::decode_exact(&trailing),
            Err(RenderIslandErrorV1::TrailingBytes(1))
        );

        let mut raw = snapshot.canonical_bytes().to_vec();
        let first_node = 8 + 2 + 1 + 1 + 8 * 3 + 32 * 3 + 4;
        raw[first_node..first_node + 32].copy_from_slice(&digest(30));
        assert!(RenderIslandV1::decode_exact(&raw).is_err());
    }

    #[test]
    fn selection_identity_survives_transform_movement() {
        let first = RenderIslandV1::seal(input()).unwrap();
        let mut moved = input();
        moved.nodes[0].parent_transform.translation_mm[1] += 5_000;
        moved.publication_sequence = 2;
        let moved = RenderIslandV1::seal(moved).unwrap();
        assert_eq!(first.island_for_member(digest(40)), Some(digest(20)));
        assert_eq!(moved.island_for_member(digest(40)), Some(digest(20)));
    }

    #[test]
    fn publication_is_monotonic_and_failed_publish_rolls_back() {
        let mut publication = RenderIslandPublicationV1::default();
        let first = publication
            .publish(RenderIslandV1::seal(input()).unwrap())
            .unwrap();
        let held = Arc::clone(&first);
        assert!(matches!(
            publication.publish(RenderIslandV1::seal(input()).unwrap()),
            Err(RenderIslandErrorV1::StaleOrEqualPublication { .. })
        ));
        assert_eq!(
            publication.acquire().unwrap().snapshot_digest(),
            first.snapshot_digest()
        );
        let mut next = input();
        next.publication_sequence = 2;
        next.nodes[0].parent_transform.translation_mm[0] += 1;
        let next = publication
            .publish(RenderIslandV1::seal(next).unwrap())
            .unwrap();
        assert_ne!(next.snapshot_digest(), first.snapshot_digest());
        assert_eq!(held.snapshot_digest(), first.snapshot_digest());
    }

    #[test]
    fn zero_and_oversize_bindings_fail_without_panicking() {
        let mut invalid = input();
        invalid.presentation_frame_digest = [0; 32];
        assert_eq!(
            RenderIslandV1::seal(invalid),
            Err(RenderIslandErrorV1::InvalidBinding)
        );
        let mut too_many = input();
        too_many.nodes = (0..=MAX_RENDER_ISLANDS_V1)
            .map(|index| {
                let value = u8::try_from(index + 1).unwrap_or(255);
                node(value, &[])
            })
            .collect();
        assert_eq!(
            RenderIslandV1::seal(too_many),
            Err(RenderIslandErrorV1::TooManyIslands(
                MAX_RENDER_ISLANDS_V1 + 1
            ))
        );
        let mut too_many_members = input();
        too_many_members.nodes = vec![node(10, &[])];
        too_many_members.nodes[0].member_ids = vec![digest(70); MAX_RENDER_ISLAND_MEMBERS_V1 + 1];
        assert_eq!(
            RenderIslandV1::seal(too_many_members),
            Err(RenderIslandErrorV1::TooManyMembers(
                MAX_RENDER_ISLAND_MEMBERS_V1 + 1
            ))
        );
        let mut too_many_portals = input();
        too_many_portals.nodes = vec![node(10, &[])];
        too_many_portals.nodes[0].portal_cells = vec![[1, 2, 3]; MAX_RENDER_ISLAND_PORTALS_V1 + 1];
        assert_eq!(
            RenderIslandV1::seal(too_many_portals),
            Err(RenderIslandErrorV1::TooManyPortals(
                MAX_RENDER_ISLAND_PORTALS_V1 + 1
            ))
        );
    }
}
