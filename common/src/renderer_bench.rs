//! renderer-bench (R0D wave W1): the canonical semantic-encoding contract.
//!
//! PROVENANCE. This module implements, byte-for-byte, the encoding contract
//! fixed by the R0D wave-0 admission bundle
//! (`bastion-test-evidence/renderer-r0d-w0/`, published 2026-07-21):
//! - `renderer-r0d-canonical-vectors-v1.json` — primitives, the empty
//!   manifest, the frame token, and the leaf→run hash hierarchy;
//! - `renderer-r0d-w1-reviewed-vectors-v1.json` — fixture manifests, item
//!   identity, presentation state, and the humanoid figure-key projection;
//! - two producer scripts + one independent verifier (Python), whose byte
//!   output THIS module must reproduce. The Rust side never regenerates or
//!   blesses expected bytes: tests compare against the checked-in JSON
//!   (mirrored under `readme/renderer-bench/`), so the production encoder
//!   cannot certify itself (W0 contract rule).
//!
//! CONTRACT RULES (from `renderer-r0d-handoff-w1-v1.json`), restated where
//! they bind this file:
//! - explicit exhaustive matches for every projected enum — no wildcard,
//!   debug-string, or source-order cast anywhere in encode/decode;
//! - manifest entities sort by `semantic_id`, equipment by `slot` (stable
//!   semantic identity), while recursive item COMPONENT order is preserved
//!   verbatim (it is semantically significant);
//! - unknown tags and trailing bytes fail closed on decode;
//! - always compiled, runtime-inert: no Cargo feature gates this module.
//!
//! NAMING CAVEAT. W0's frame-token vector is synthetic: its byte LAYOUT is
//! contractual, but the field names used here for the three trailing
//! 32-byte digests and the two u64 cursors are W1-chosen labels, to be
//! bound (or renamed) by the W2 semantics handoff. The bytes cannot change;
//! the names may.

use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Little-endian canonical writer. Every multi-byte integer is LE; strings
/// and byte-strings are u32-length-prefixed; options are a u8 0/1 prefix;
/// sequences are a u32 count then elements; bools are a u8 0/1.
#[derive(Default)]
pub struct CanonicalWriter {
    buf: Vec<u8>,
}

/// Encoding refusals. Fail-closed by construction: an invalid value refuses
/// to encode rather than encoding something plausible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// A float that is not finite (NaN or ±inf) — the canonical form has no
    /// representation for these, deliberately.
    NonFiniteF32,
    /// Manifest entity ids must be nonzero and unique.
    InvalidSemanticIds,
    /// Equipment slots within one loadout must be unique.
    DuplicateEquipmentSlot,
}

/// Decoding refusals. Unknown tags and trailing bytes fail closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated,
    BadMagic,
    UnsupportedVersion,
    UnknownTag { context: &'static str, tag: u8 },
    InvalidUtf8,
    InvalidBool,
    InvalidOptionPrefix,
    /// Decoded payload violates a semantic invariant (ids, slots, order).
    InvalidSemantics,
    TrailingBytes,
}

impl CanonicalWriter {
    pub fn new() -> Self { Self::default() }

    pub fn into_bytes(self) -> Vec<u8> { self.buf }

    pub fn raw(&mut self, b: &[u8]) { self.buf.extend_from_slice(b); }

    pub fn u8(&mut self, v: u8) { self.buf.push(v); }

    pub fn u16(&mut self, v: u16) { self.raw(&v.to_le_bytes()); }

    pub fn u32(&mut self, v: u32) { self.raw(&v.to_le_bytes()); }

    pub fn u64(&mut self, v: u64) { self.raw(&v.to_le_bytes()); }

    pub fn i32(&mut self, v: i32) { self.raw(&v.to_le_bytes()); }

    pub fn i64(&mut self, v: i64) { self.raw(&v.to_le_bytes()); }

    pub fn bool(&mut self, v: bool) { self.u8(if v { 1 } else { 0 }); }

    /// u32-length-prefixed bytes.
    pub fn lp(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.raw(b);
    }

    /// u32-length-prefixed UTF-8 string.
    pub fn text(&mut self, s: &str) { self.lp(s.as_bytes()); }

    /// Canonical finite f32: -0.0 normalizes to +0.0; NaN/inf refuse.
    pub fn f32_finite(&mut self, v: f32) -> Result<(), EncodeError> {
        if !v.is_finite() {
            return Err(EncodeError::NonFiniteF32);
        }
        let v = if v == 0.0 { 0.0 } else { v }; // -0.0 == 0.0 → writes +0.0
        self.raw(&v.to_le_bytes());
        Ok(())
    }

    pub fn opt<T>(
        &mut self,
        v: Option<&T>,
        f: impl FnOnce(&mut Self, &T) -> Result<(), EncodeError>,
    ) -> Result<(), EncodeError> {
        match v {
            None => {
                self.u8(0);
                Ok(())
            },
            Some(x) => {
                self.u8(1);
                f(self, x)
            },
        }
    }

    pub fn seq<T>(
        &mut self,
        vs: &[T],
        mut f: impl FnMut(&mut Self, &T) -> Result<(), EncodeError>,
    ) -> Result<(), EncodeError> {
        self.u32(vs.len() as u32);
        for v in vs {
            f(self, v)?;
        }
        Ok(())
    }
}

/// Strict little-endian reader for the fail-closed decode direction.
pub struct CanonicalReader<'a> {
    buf: &'a [u8],
    at: usize,
}

impl<'a> CanonicalReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self { Self { buf, at: 0 } }

    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.at.checked_add(n).ok_or(DecodeError::Truncated)?;
        if end > self.buf.len() {
            return Err(DecodeError::Truncated);
        }
        let s = &self.buf[self.at..end];
        self.at = end;
        Ok(s)
    }

    pub fn u8(&mut self) -> Result<u8, DecodeError> { Ok(self.take(1)?[0]) }

    pub fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("len 2")))
    }

    pub fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("len 4")))
    }

    pub fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("len 8")))
    }

    pub fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().expect("len 4")))
    }

    pub fn bool(&mut self) -> Result<bool, DecodeError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::InvalidBool),
        }
    }

    pub fn text(&mut self) -> Result<String, DecodeError> {
        let n = self.u32()? as usize;
        let raw = self.take(n)?;
        String::from_utf8(raw.to_vec()).map_err(|_| DecodeError::InvalidUtf8)
    }

    pub fn opt<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<Option<T>, DecodeError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(f(self)?)),
            _ => Err(DecodeError::InvalidOptionPrefix),
        }
    }

    /// The trailing-bytes gate: a top-level decode MUST end exactly at the
    /// buffer's end or the whole decode fails closed.
    pub fn finish(self) -> Result<(), DecodeError> {
        if self.at == self.buf.len() {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Tag tables (W0 canonical — numeric values are contractual).
// ─────────────────────────────────────────────────────────────────────────

/// Canonical wire TYPE tags (W0 `tag_tables.type`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum WireType {
    U8 = 1,
    Bool = 2,
    U16 = 3,
    U32 = 4,
    U64 = 5,
    I32 = 6,
    I64 = 7,
    Bytes = 8,
    Utf8 = 9,
    Option = 10,
    Sequence = 11,
    Enum = 12,
    Sha256 = 13,
    FiniteF32 = 14,
    Struct = 15,
    FixedI32 = 16,
}

/// Semantic DOMAIN tags (W0 `tag_tables.domain`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Domain {
    ManifestIdentity = 1,
    ServerFixture = 2,
    ServerScriptState = 3,
    ReplicationProjection = 4,
    ClientProjection = 5,
    CameraFrame = 6,
    SceneEnvironment = 7,
    FigureSourceProjection = 8,
    FigureIdentity = 9,
    FigureDecision = 10,
    AssetMeshContent = 11,
    PassDraw = 12,
    VisualStructure = 13,
    ReadbackLifecycle = 14,
    ArtifactLifecycle = 15,
    RunTerminal = 16,
}

/// Owner-kind tags (W0 `tag_tables.owner_kind`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OwnerKind {
    Run = 1,
    Frame = 2,
    StableEntity = 3,
    FigureKey = 4,
    AssetRequest = 5,
    Pass = 6,
    Readback = 7,
    Artifact = 8,
}

// ─────────────────────────────────────────────────────────────────────────
// Fixture manifest ("RBDM" v1.0) — the scenario the bench replays.
// ─────────────────────────────────────────────────────────────────────────

pub const MANIFEST_MAGIC: &[u8; 4] = b"RBDM";
pub const MANIFEST_DOMAIN_SEP: &[u8] = b"BASTION:R0D:MANIFEST:V1\0";
pub const MANIFEST_VERSION: (u16, u16) = (1, 0);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FixtureManifestV1 {
    pub scenario_id: String,
    pub scenario_seed: u64,
    pub worldgen_seed: u64,
    pub rtsim_seed: u64,
    pub simulation_tps: u32,
    pub arena_origin_mm: [i32; 3],
    pub camera_script_id: String,
    pub graphics_manifest_version: u32,
    pub artifact_schema_version: u32,
    pub entities: Vec<FixtureEntityV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FixtureEntityV1 {
    /// Stable nonzero semantic id — the manifest's sort key and the sync
    /// component's payload ([`crate::comp::bastion::RendererBenchEntityId`]).
    pub semantic_id: u32,
    pub per_entity_seed: u64,
    pub body: BenchBodyV1,
    pub loadout: Vec<LoadoutEntryV1>,
    pub spawn_position_mm: [i32; 3],
    pub orientation_turns_u32: u32,
    pub movement: MovementV1,
    pub animation: AnimationV1,
}

/// Body projection (W1 supports exactly the two families the reviewed
/// vectors exercise; further families arrive with their own vectors).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BenchBodyV1 {
    /// tag 0 — ten u8 fields, field order contractual.
    Humanoid {
        species: u8,
        body_type: u8,
        hair_style: u8,
        beard: u8,
        eyes: u8,
        accessory: u8,
        hair_color: u8,
        skin: u8,
        eye_color: u8,
        height_scale: u8,
    },
    /// tag 1 — two u8 fields.
    QuadrupedSmall { species: u8, body_type: u8 },
}

/// Recursive item identity. COMPONENT ORDER IS PRESERVED VERBATIM — the
/// reviewed vectors prove reversed components produce different bytes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemDefV1 {
    /// tag 0
    Simple(String),
    /// tag 1
    Modular {
        base: String,
        components: Vec<ItemDefV1>,
    },
    /// tag 2
    Compound {
        base: String,
        components: Vec<ItemDefV1>,
    },
}

impl ItemDefV1 {
    /// Bare canonical encoding of one item identity (the W1 item vectors
    /// are unwrapped — no manifest framing).
    pub fn encode_canonical(&self) -> Result<Vec<u8>, EncodeError> {
        let mut w = CanonicalWriter::new();
        encode_item(&mut w, self)?;
        Ok(w.into_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoadoutEntryV1 {
    pub slot: u8,
    pub item: ItemDefV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveStepV1 {
    pub tick: u64,
    pub move_x_ppm: i32,
    pub move_y_ppm: i32,
    pub look_turns_u32: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MovementV1 {
    /// tag 0
    None,
    /// tag 1
    Steps(Vec<MoveStepV1>),
    /// tag 2
    Target {
        target_mm: [i32; 3],
        earliest_terminal_tick: u64,
        latest_terminal_tick: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AnimationV1 {
    /// tag 0
    None,
    /// tag 1
    Script(Vec<ScriptActionV1>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptActionV1 {
    pub tick: u64,
    pub sequence: u32,
    pub kind: ScriptActionKindV1,
}

/// Script action tags 0–11. W0 leaves tags 0–8 semantically opaque (they
/// carry no payload on the wire); their W1 names are placeholders bound at
/// the W2 semantics handoff. Tags 9/10/11 carry the payloads the reference
/// encoder defines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScriptActionKindV1 {
    Action0,
    Action1,
    Action2,
    Action3,
    Action4,
    Action5,
    Action6,
    Action7,
    Action8,
    /// tag 9 — stable-target talk.
    Talk { target: Option<u64> },
    /// tag 10 — start input.
    StartInput {
        input: InputKindV1,
        target: Option<u64>,
        select_pos_mm: Option<[i32; 3]>,
    },
    /// tag 11 — cancel input.
    CancelInput { input: InputKindV1 },
}

impl ScriptActionKindV1 {
    fn tag(&self) -> u8 {
        match self {
            ScriptActionKindV1::Action0 => 0,
            ScriptActionKindV1::Action1 => 1,
            ScriptActionKindV1::Action2 => 2,
            ScriptActionKindV1::Action3 => 3,
            ScriptActionKindV1::Action4 => 4,
            ScriptActionKindV1::Action5 => 5,
            ScriptActionKindV1::Action6 => 6,
            ScriptActionKindV1::Action7 => 7,
            ScriptActionKindV1::Action8 => 8,
            ScriptActionKindV1::Talk { .. } => 9,
            ScriptActionKindV1::StartInput { .. } => 10,
            ScriptActionKindV1::CancelInput { .. } => 11,
        }
    }
}

/// Input kinds. Tags 0–2 carry no payload; tag 3 carries an ability index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputKindV1 {
    Input0,
    Input1,
    Input2,
    /// tag 3
    Ability { ability_index: u32 },
}

impl InputKindV1 {
    fn tag(&self) -> u8 {
        match self {
            InputKindV1::Input0 => 0,
            InputKindV1::Input1 => 1,
            InputKindV1::Input2 => 2,
            InputKindV1::Ability { .. } => 3,
        }
    }
}

fn encode_body(w: &mut CanonicalWriter, b: &BenchBodyV1) {
    match b {
        BenchBodyV1::Humanoid {
            species,
            body_type,
            hair_style,
            beard,
            eyes,
            accessory,
            hair_color,
            skin,
            eye_color,
            height_scale,
        } => {
            w.u8(0);
            w.u8(*species);
            w.u8(*body_type);
            w.u8(*hair_style);
            w.u8(*beard);
            w.u8(*eyes);
            w.u8(*accessory);
            w.u8(*hair_color);
            w.u8(*skin);
            w.u8(*eye_color);
            w.u8(*height_scale);
        },
        BenchBodyV1::QuadrupedSmall { species, body_type } => {
            w.u8(1);
            w.u8(*species);
            w.u8(*body_type);
        },
    }
}

fn decode_body(r: &mut CanonicalReader) -> Result<BenchBodyV1, DecodeError> {
    match r.u8()? {
        0 => Ok(BenchBodyV1::Humanoid {
            species: r.u8()?,
            body_type: r.u8()?,
            hair_style: r.u8()?,
            beard: r.u8()?,
            eyes: r.u8()?,
            accessory: r.u8()?,
            hair_color: r.u8()?,
            skin: r.u8()?,
            eye_color: r.u8()?,
            height_scale: r.u8()?,
        }),
        1 => Ok(BenchBodyV1::QuadrupedSmall {
            species: r.u8()?,
            body_type: r.u8()?,
        }),
        tag => Err(DecodeError::UnknownTag { context: "body", tag }),
    }
}

fn encode_item(w: &mut CanonicalWriter, item: &ItemDefV1) -> Result<(), EncodeError> {
    match item {
        ItemDefV1::Simple(id) => {
            w.u8(0);
            w.text(id);
            Ok(())
        },
        ItemDefV1::Modular { base, components } => {
            w.u8(1);
            w.text(base);
            w.seq(components, encode_item)
        },
        ItemDefV1::Compound { base, components } => {
            w.u8(2);
            w.text(base);
            w.seq(components, encode_item)
        },
    }
}

fn decode_item(r: &mut CanonicalReader) -> Result<ItemDefV1, DecodeError> {
    match r.u8()? {
        0 => Ok(ItemDefV1::Simple(r.text()?)),
        tag @ (1 | 2) => {
            let base = r.text()?;
            let n = r.u32()? as usize;
            let mut components = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                components.push(decode_item(r)?);
            }
            if tag == 1 {
                Ok(ItemDefV1::Modular { base, components })
            } else {
                Ok(ItemDefV1::Compound { base, components })
            }
        },
        tag => Err(DecodeError::UnknownTag { context: "item", tag }),
    }
}

fn encode_input(w: &mut CanonicalWriter, i: &InputKindV1) {
    w.u8(i.tag());
    match i {
        InputKindV1::Input0 | InputKindV1::Input1 | InputKindV1::Input2 => {},
        InputKindV1::Ability { ability_index } => w.u32(*ability_index),
    }
}

fn decode_input(r: &mut CanonicalReader) -> Result<InputKindV1, DecodeError> {
    match r.u8()? {
        0 => Ok(InputKindV1::Input0),
        1 => Ok(InputKindV1::Input1),
        2 => Ok(InputKindV1::Input2),
        3 => Ok(InputKindV1::Ability {
            ability_index: r.u32()?,
        }),
        tag => Err(DecodeError::UnknownTag {
            context: "input_kind",
            tag,
        }),
    }
}

fn encode_action(w: &mut CanonicalWriter, a: &ScriptActionV1) -> Result<(), EncodeError> {
    w.u64(a.tick);
    w.u32(a.sequence);
    w.u8(a.kind.tag());
    match &a.kind {
        ScriptActionKindV1::Action0
        | ScriptActionKindV1::Action1
        | ScriptActionKindV1::Action2
        | ScriptActionKindV1::Action3
        | ScriptActionKindV1::Action4
        | ScriptActionKindV1::Action5
        | ScriptActionKindV1::Action6
        | ScriptActionKindV1::Action7
        | ScriptActionKindV1::Action8 => Ok(()),
        ScriptActionKindV1::Talk { target } => {
            w.opt(target.as_ref(), |w, t| {
                w.u64(*t);
                Ok(())
            })
        },
        ScriptActionKindV1::StartInput {
            input,
            target,
            select_pos_mm,
        } => {
            encode_input(w, input);
            w.opt(target.as_ref(), |w, t| {
                w.u64(*t);
                Ok(())
            })?;
            w.opt(select_pos_mm.as_ref(), |w, p| {
                for c in p.iter() {
                    w.i32(*c);
                }
                Ok(())
            })
        },
        ScriptActionKindV1::CancelInput { input } => {
            encode_input(w, input);
            Ok(())
        },
    }
}

fn encode_movement(w: &mut CanonicalWriter, m: &MovementV1) -> Result<(), EncodeError> {
    match m {
        MovementV1::None => {
            w.u8(0);
            Ok(())
        },
        MovementV1::Steps(steps) => {
            w.u8(1);
            w.seq(steps, |w, s| {
                w.u64(s.tick);
                w.i32(s.move_x_ppm);
                w.i32(s.move_y_ppm);
                w.u32(s.look_turns_u32);
                Ok(())
            })
        },
        MovementV1::Target {
            target_mm,
            earliest_terminal_tick,
            latest_terminal_tick,
        } => {
            w.u8(2);
            for c in target_mm.iter() {
                w.i32(*c);
            }
            w.u64(*earliest_terminal_tick);
            w.u64(*latest_terminal_tick);
            Ok(())
        },
    }
}

fn decode_movement(r: &mut CanonicalReader) -> Result<MovementV1, DecodeError> {
    match r.u8()? {
        0 => Ok(MovementV1::None),
        1 => {
            let n = r.u32()? as usize;
            let mut steps = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                steps.push(MoveStepV1 {
                    tick: r.u64()?,
                    move_x_ppm: r.i32()?,
                    move_y_ppm: r.i32()?,
                    look_turns_u32: r.u32()?,
                });
            }
            Ok(MovementV1::Steps(steps))
        },
        2 => Ok(MovementV1::Target {
            target_mm: [r.i32()?, r.i32()?, r.i32()?],
            earliest_terminal_tick: r.u64()?,
            latest_terminal_tick: r.u64()?,
        }),
        tag => Err(DecodeError::UnknownTag {
            context: "movement",
            tag,
        }),
    }
}

fn encode_animation(w: &mut CanonicalWriter, a: &AnimationV1) -> Result<(), EncodeError> {
    match a {
        AnimationV1::None => {
            w.u8(0);
            Ok(())
        },
        AnimationV1::Script(actions) => {
            w.u8(1);
            w.seq(actions, encode_action)
        },
    }
}

fn decode_animation(r: &mut CanonicalReader) -> Result<AnimationV1, DecodeError> {
    match r.u8()? {
        0 => Ok(AnimationV1::None),
        1 => {
            let n = r.u32()? as usize;
            let mut actions = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                let tick = r.u64()?;
                let sequence = r.u32()?;
                let kind = match r.u8()? {
                    0 => ScriptActionKindV1::Action0,
                    1 => ScriptActionKindV1::Action1,
                    2 => ScriptActionKindV1::Action2,
                    3 => ScriptActionKindV1::Action3,
                    4 => ScriptActionKindV1::Action4,
                    5 => ScriptActionKindV1::Action5,
                    6 => ScriptActionKindV1::Action6,
                    7 => ScriptActionKindV1::Action7,
                    8 => ScriptActionKindV1::Action8,
                    9 => ScriptActionKindV1::Talk {
                        target: r.opt(|r| r.u64())?,
                    },
                    10 => ScriptActionKindV1::StartInput {
                        input: decode_input(r)?,
                        target: r.opt(|r| r.u64())?,
                        select_pos_mm: r.opt(|r| Ok([r.i32()?, r.i32()?, r.i32()?]))?,
                    },
                    11 => ScriptActionKindV1::CancelInput {
                        input: decode_input(r)?,
                    },
                    tag => {
                        return Err(DecodeError::UnknownTag {
                            context: "script_action",
                            tag,
                        });
                    },
                };
                actions.push(ScriptActionV1 {
                    tick,
                    sequence,
                    kind,
                });
            }
            Ok(AnimationV1::Script(actions))
        },
        tag => Err(DecodeError::UnknownTag {
            context: "animation",
            tag,
        }),
    }
}

impl FixtureManifestV1 {
    /// Canonical encode. Entities are SORTED by `semantic_id` (author order
    /// is normalized away — proven by the reverse-author-order vector pair);
    /// ids must be nonzero and unique; loadout slots are sorted and must be
    /// unique; item component order is preserved verbatim.
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut entities: Vec<&FixtureEntityV1> = self.entities.iter().collect();
        entities.sort_by_key(|e| e.semantic_id);
        let mut seen = HashSet::new();
        for e in &entities {
            if e.semantic_id == 0 || !seen.insert(e.semantic_id) {
                return Err(EncodeError::InvalidSemanticIds);
            }
        }
        let mut w = CanonicalWriter::new();
        w.raw(MANIFEST_MAGIC);
        w.u16(MANIFEST_VERSION.0);
        w.u16(MANIFEST_VERSION.1);
        w.text(&self.scenario_id);
        w.u64(self.scenario_seed);
        w.u64(self.worldgen_seed);
        w.u64(self.rtsim_seed);
        w.u32(self.simulation_tps);
        for c in self.arena_origin_mm.iter() {
            w.i32(*c);
        }
        w.text(&self.camera_script_id);
        w.u32(self.graphics_manifest_version);
        w.u32(self.artifact_schema_version);
        w.u32(entities.len() as u32);
        for e in entities {
            w.u32(e.semantic_id);
            w.u64(e.per_entity_seed);
            encode_body(&mut w, &e.body);
            // Loadout: sorted by slot, duplicate slots refuse.
            let mut load: Vec<&LoadoutEntryV1> = e.loadout.iter().collect();
            load.sort_by_key(|l| l.slot);
            let mut slots = HashSet::new();
            for l in &load {
                if !slots.insert(l.slot) {
                    return Err(EncodeError::DuplicateEquipmentSlot);
                }
            }
            w.u32(load.len() as u32);
            for l in load {
                w.u8(l.slot);
                encode_item(&mut w, &l.item)?;
            }
            for c in e.spawn_position_mm.iter() {
                w.i32(*c);
            }
            w.u32(e.orientation_turns_u32);
            encode_movement(&mut w, &e.movement)?;
            encode_animation(&mut w, &e.animation)?;
        }
        Ok(w.into_bytes())
    }

    /// The manifest's domain-separated digest
    /// (`sha256(MANIFEST_DOMAIN_SEP ‖ payload)`).
    pub fn domain_sha256(payload: &[u8]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(MANIFEST_DOMAIN_SEP);
        h.update(payload);
        h.finalize().into()
    }

    /// Strict decode: unknown tags, bad magic/version, semantic violations
    /// (zero/dup ids, unsorted entities, dup slots) and trailing bytes ALL
    /// fail closed.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = CanonicalReader::new(buf);
        let magic = r.take(4)?;
        if magic != MANIFEST_MAGIC {
            return Err(DecodeError::BadMagic);
        }
        let (maj, min) = (r.u16()?, r.u16()?);
        if (maj, min) != MANIFEST_VERSION {
            return Err(DecodeError::UnsupportedVersion);
        }
        let scenario_id = r.text()?;
        let scenario_seed = r.u64()?;
        let worldgen_seed = r.u64()?;
        let rtsim_seed = r.u64()?;
        let simulation_tps = r.u32()?;
        let arena_origin_mm = [r.i32()?, r.i32()?, r.i32()?];
        let camera_script_id = r.text()?;
        let graphics_manifest_version = r.u32()?;
        let artifact_schema_version = r.u32()?;
        let n = r.u32()? as usize;
        let mut entities = Vec::with_capacity(n.min(4096));
        let mut prev_id = 0u32;
        for _ in 0..n {
            let semantic_id = r.u32()?;
            if semantic_id == 0 || semantic_id <= prev_id {
                return Err(DecodeError::InvalidSemantics);
            }
            prev_id = semantic_id;
            let per_entity_seed = r.u64()?;
            let body = decode_body(&mut r)?;
            let ln = r.u32()? as usize;
            let mut loadout = Vec::with_capacity(ln.min(256));
            let mut prev_slot: Option<u8> = None;
            for _ in 0..ln {
                let slot = r.u8()?;
                if prev_slot.is_some_and(|p| slot <= p) {
                    return Err(DecodeError::InvalidSemantics);
                }
                prev_slot = Some(slot);
                loadout.push(LoadoutEntryV1 {
                    slot,
                    item: decode_item(&mut r)?,
                });
            }
            let spawn_position_mm = [r.i32()?, r.i32()?, r.i32()?];
            let orientation_turns_u32 = r.u32()?;
            let movement = decode_movement(&mut r)?;
            let animation = decode_animation(&mut r)?;
            entities.push(FixtureEntityV1 {
                semantic_id,
                per_entity_seed,
                body,
                loadout,
                spawn_position_mm,
                orientation_turns_u32,
                movement,
                animation,
            });
        }
        r.finish()?;
        Ok(FixtureManifestV1 {
            scenario_id,
            scenario_seed,
            worldgen_seed,
            rtsim_seed,
            simulation_tps,
            arena_origin_mm,
            camera_script_id,
            graphics_manifest_version,
            artifact_schema_version,
            entities,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Semantic frame token ("RBFT" v1.0, fixed 164 bytes).
// ─────────────────────────────────────────────────────────────────────────

pub const FRAME_TOKEN_MAGIC: &[u8; 4] = b"RBFT";
pub const FRAME_TOKEN_VERSION: (u16, u16) = (1, 0);
pub const FRAME_TOKEN_LEN: usize = 164;

/// One frame's identity token. Layout contractual (W0 vector, 164 bytes);
/// the trailing digest/cursor NAMES are provisional W1 labels (see module
/// docs) — W2 binds them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticFrameTokenV1 {
    pub run_id: [u8; 32],
    pub frame_index: u32,
    pub sim_tick: u64,
    pub script_cursor: u64,
    pub readback_cursor: u64,
    pub manifest_sha256: [u8; 32],
    pub script_sha256: [u8; 32],
    pub parent_frame_sha256: [u8; 32],
}

impl SemanticFrameTokenV1 {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = CanonicalWriter::new();
        w.raw(FRAME_TOKEN_MAGIC);
        w.u16(FRAME_TOKEN_VERSION.0);
        w.u16(FRAME_TOKEN_VERSION.1);
        w.raw(&self.run_id);
        w.u32(self.frame_index);
        w.u64(self.sim_tick);
        w.u64(self.script_cursor);
        w.u64(self.readback_cursor);
        w.raw(&self.manifest_sha256);
        w.raw(&self.script_sha256);
        w.raw(&self.parent_frame_sha256);
        let out = w.into_bytes();
        debug_assert_eq!(out.len(), FRAME_TOKEN_LEN);
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut r = CanonicalReader::new(buf);
        let magic = r.take(4)?;
        if magic != FRAME_TOKEN_MAGIC {
            return Err(DecodeError::BadMagic);
        }
        if (r.u16()?, r.u16()?) != FRAME_TOKEN_VERSION {
            return Err(DecodeError::UnsupportedVersion);
        }
        let mut run_id = [0u8; 32];
        run_id.copy_from_slice(r.take(32)?);
        let frame_index = r.u32()?;
        let sim_tick = r.u64()?;
        let script_cursor = r.u64()?;
        let readback_cursor = r.u64()?;
        let mut manifest_sha256 = [0u8; 32];
        manifest_sha256.copy_from_slice(r.take(32)?);
        let mut script_sha256 = [0u8; 32];
        script_sha256.copy_from_slice(r.take(32)?);
        let mut parent_frame_sha256 = [0u8; 32];
        parent_frame_sha256.copy_from_slice(r.take(32)?);
        r.finish()?;
        Ok(Self {
            run_id,
            frame_index,
            sim_tick,
            script_cursor,
            readback_cursor,
            manifest_sha256,
            script_sha256,
            parent_frame_sha256,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The semantic-tape hash hierarchy (leaf → owner → domain → frame → run).
// ─────────────────────────────────────────────────────────────────────────

pub const ORACLE_SCHEMA_DOMAIN_SEP: &[u8] = b"BASTION:R0D:ORACLE-SCHEMA:VECTOR:V1\0";
const LEAF_SEP: &[u8] = b"bastion.renderer.leaf.v1\0";
const OWNER_SEP: &[u8] = b"bastion.renderer.owner.v1\0";
const DOMAIN_SEP: &[u8] = b"bastion.renderer.domain.v1\0";
const FRAME_SEP: &[u8] = b"bastion.renderer.frame.v1\0";
const RUN_SEP: &[u8] = b"bastion.renderer.run.v1\0";

/// `sha256("BASTION:R0D:ORACLE-SCHEMA:VECTOR:V1\0")` — the schema hash every
/// hierarchy preimage binds.
pub fn oracle_schema_hash() -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(ORACLE_SCHEMA_DOMAIN_SEP);
    h.finalize().into()
}

fn sha(pre: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(pre);
    h.finalize().into()
}

/// Leaf: one (domain, field, type) value under one owner.
pub fn leaf_hash(
    schema: &[u8; 32],
    domain: Domain,
    field_id: u32,
    ty: WireType,
    owner: OwnerKind,
    owner_key: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    let mut w = CanonicalWriter::new();
    w.raw(LEAF_SEP);
    w.raw(schema);
    w.u16(domain as u16);
    w.u32(field_id);
    w.u16(ty as u16);
    w.u8(owner as u8);
    w.lp(owner_key);
    w.lp(payload);
    sha(&w.into_bytes())
}

/// Owner root: the sorted (field_id, leaf) set for one owner.
pub fn owner_root(
    schema: &[u8; 32],
    owner: OwnerKind,
    owner_key: &[u8],
    leaves: &[(u32, [u8; 32])],
) -> [u8; 32] {
    let mut w = CanonicalWriter::new();
    w.raw(OWNER_SEP);
    w.raw(schema);
    w.u8(owner as u8);
    w.lp(owner_key);
    w.u32(leaves.len() as u32);
    for (field_id, leaf) in leaves {
        w.u32(*field_id);
        w.raw(leaf);
    }
    sha(&w.into_bytes())
}

/// Domain root: the sorted (owner_key, owner_root) set for one domain.
/// `owner_key` here is the composite `u8(owner_kind) ‖ lp(owner_key_bytes)`.
pub fn domain_root(
    schema: &[u8; 32],
    domain: Domain,
    owners: &[(Vec<u8>, [u8; 32])],
) -> [u8; 32] {
    let mut w = CanonicalWriter::new();
    w.raw(DOMAIN_SEP);
    w.raw(schema);
    w.u16(domain as u16);
    w.u32(owners.len() as u32);
    for (owner_key, oroot) in owners {
        w.lp(owner_key);
        w.raw(oroot);
    }
    sha(&w.into_bytes())
}

/// Frame root: the frame token binding the sorted (domain, domain_root) set.
pub fn frame_root(
    schema: &[u8; 32],
    token_bytes: &[u8],
    domains: &[(Domain, [u8; 32])],
) -> [u8; 32] {
    let mut w = CanonicalWriter::new();
    w.raw(FRAME_SEP);
    w.raw(schema);
    w.lp(token_bytes);
    w.u32(domains.len() as u32);
    for (domain, droot) in domains {
        w.u16(*domain as u16);
        w.raw(droot);
    }
    sha(&w.into_bytes())
}

/// Run root: run id + ordered (token, frame_root) pairs + terminal count.
pub fn run_root(
    schema: &[u8; 32],
    run_id: &str,
    frames: &[(Vec<u8>, [u8; 32])],
    terminal_count: u32,
) -> [u8; 32] {
    let mut w = CanonicalWriter::new();
    w.raw(RUN_SEP);
    w.raw(schema);
    // W0 quirk, contractual: the run id is DOUBLE length-prefixed —
    // `lp(text(id))` in the producer — so the outer prefix covers the inner
    // prefixed string. Reproduced verbatim (bytes are the contract).
    let mut inner = CanonicalWriter::new();
    inner.text(run_id);
    w.lp(&inner.into_bytes());
    w.u32(frames.len() as u32);
    for (token, froot) in frames {
        w.lp(token);
        w.raw(froot);
    }
    w.u32(terminal_count);
    sha(&w.into_bytes())
}

// ─────────────────────────────────────────────────────────────────────────
// Character presentation state (client projection, W1 vector).
// ─────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterPresentationStateV1 {
    pub class: u8,
    pub stage: Option<u8>,
    pub input: Option<InputKindV1>,
    pub is_riding: bool,
    pub is_gliding: bool,
    pub is_dead: bool,
}

impl CharacterPresentationStateV1 {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = CanonicalWriter::new();
        w.u8(self.class);
        let _ = w.opt(self.stage.as_ref(), |w, s| {
            w.u8(*s);
            Ok(())
        });
        let _ = w.opt(self.input.as_ref(), |w, i| {
            encode_input(w, i);
            Ok(())
        });
        w.bool(self.is_riding);
        w.bool(self.is_gliding);
        w.bool(self.is_dead);
        w.into_bytes()
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Figure-key projection (the humanoid render-identity cache key, W1 vector).
// ─────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToolKeyV1 {
    /// tag 0
    Simple(String),
    /// tag 1
    Modular { a: String, b: String, hands: u8 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemKeyV1 {
    /// tag 0
    Simple(String),
    /// tag 1
    Tool { a: String, b: String, hands: u8 },
    /// tag 2
    Pair { a: String, b: String },
    /// tag 3
    Set {
        items: Vec<ItemKeyV1>,
        fallback: String,
    },
    /// tag 4
    Empty,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThirdPersonKeyV1 {
    pub head: Option<String>,
    pub shoulder: Option<String>,
    pub chest: Option<String>,
    pub belt: Option<String>,
    pub back: Option<String>,
    pub pants: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSlotsV1 {
    pub active: Option<ToolKeyV1>,
    pub second: Option<ToolKeyV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FigureCacheKeyV1 {
    pub third_person: Option<ThirdPersonKeyV1>,
    pub tool: Option<ToolSlotsV1>,
    pub lantern: Option<String>,
    pub glider: Option<String>,
    pub foot: Option<String>,
    pub head: Option<String>,
    /// Double option — outer presence, inner value — preserved as-is (the
    /// vector pins the nested encoding).
    pub hand: Option<Option<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FigureKeyProjectionV1 {
    pub body: BenchBodyV1,
    pub item_key: Option<ItemKeyV1>,
    pub extra: Option<FigureCacheKeyV1>,
}

fn encode_tool_key(w: &mut CanonicalWriter, t: &ToolKeyV1) {
    match t {
        ToolKeyV1::Simple(id) => {
            w.u8(0);
            w.text(id);
        },
        ToolKeyV1::Modular { a, b, hands } => {
            w.u8(1);
            w.text(a);
            w.text(b);
            w.u8(*hands);
        },
    }
}

fn encode_item_key(w: &mut CanonicalWriter, k: &ItemKeyV1) -> Result<(), EncodeError> {
    match k {
        ItemKeyV1::Simple(id) => {
            w.u8(0);
            w.text(id);
            Ok(())
        },
        ItemKeyV1::Tool { a, b, hands } => {
            w.u8(1);
            w.text(a);
            w.text(b);
            w.u8(*hands);
            Ok(())
        },
        ItemKeyV1::Pair { a, b } => {
            w.u8(2);
            w.text(a);
            w.text(b);
            Ok(())
        },
        ItemKeyV1::Set { items, fallback } => {
            w.u8(3);
            w.seq(items, encode_item_key)?;
            w.text(fallback);
            Ok(())
        },
        ItemKeyV1::Empty => {
            w.u8(4);
            Ok(())
        },
    }
}

impl FigureKeyProjectionV1 {
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut w = CanonicalWriter::new();
        encode_body(&mut w, &self.body);
        w.opt(self.item_key.as_ref(), |w, k| encode_item_key(w, k))?;
        w.opt(self.extra.as_ref(), |w, e| {
            w.opt(e.third_person.as_ref(), |w, tp| {
                for f in [&tp.head, &tp.shoulder, &tp.chest, &tp.belt, &tp.back, &tp.pants] {
                    w.opt(f.as_ref(), |w, s| {
                        w.text(s);
                        Ok(())
                    })?;
                }
                Ok(())
            })?;
            w.opt(e.tool.as_ref(), |w, t| {
                w.opt(t.active.as_ref(), |w, k| {
                    encode_tool_key(w, k);
                    Ok(())
                })?;
                w.opt(t.second.as_ref(), |w, k| {
                    encode_tool_key(w, k);
                    Ok(())
                })
            })?;
            for f in [&e.lantern, &e.glider, &e.foot, &e.head] {
                w.opt(f.as_ref(), |w, s| {
                    w.text(s);
                    Ok(())
                })?;
            }
            w.opt(e.hand.as_ref(), |w, inner| {
                w.opt(inner.as_ref(), |w, s| {
                    w.text(s);
                    Ok(())
                })
            })
        })?;
        Ok(w.into_bytes())
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The exactly-once readback registry (interface `readback_terminal`).
// ─────────────────────────────────────────────────────────────────────────

/// "One exactly-once readback registry" (W0 invariant): every visual
/// readback id may complete AT MOST once per run; a second completion is a
/// contract violation the caller must treat as a hard error, never a
/// duplicate to ignore silently. Background completion is NOT semantic
/// authority — this registry is bookkeeping for the semantic path only.
#[derive(Default, Debug)]
pub struct RendererBenchReadbacks {
    claimed: HashSet<u64>,
}

impl RendererBenchReadbacks {
    /// Claim a readback id. `true` exactly on the first claim.
    pub fn claim(&mut self, readback_id: u64) -> bool { self.claimed.insert(readback_id) }

    pub fn is_claimed(&self, readback_id: u64) -> bool { self.claimed.contains(&readback_id) }

    pub fn len(&self) -> usize { self.claimed.len() }

    pub fn is_empty(&self) -> bool { self.claimed.is_empty() }
}
