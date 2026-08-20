//! bastion (Project Bastion): shared overseer-interaction types (B2a).
//!
//! These are the *plumbing* payloads for the overseer interaction surface —
//! designation regions, influence kinds, and context-menu verbs. In B2a the
//! server only validates and echoes them (no behavior); B4 (job board) and
//! B13 (divine influence) give them teeth. Everything is serde-ready by
//! construction (B10 persistence ground rule).

use rand::RngExt as _;
use serde::{Deserialize, Serialize};
use vek::*;

/// An axis-aligned block region, inclusive on both corners.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub min: Vec3<i32>,
    pub max: Vec3<i32>,
}

impl Region {
    /// Normalize so `min <= max` on every axis.
    pub fn normalized(self) -> Self {
        Self {
            min: Vec3::partial_min(self.min, self.max),
            max: Vec3::partial_max(self.min, self.max),
        }
    }

    pub fn volume(&self) -> i64 {
        let d = (self.max - self.min).map(|e| (e as i64 + 1).max(0));
        d.x * d.y * d.z
    }

    pub fn contains_point(&self, p: Vec3<i32>) -> bool {
        (self.min.x..=self.max.x).contains(&p.x)
            && (self.min.y..=self.max.y).contains(&p.y)
            && (self.min.z..=self.max.z).contains(&p.z)
    }

    /// XY-footprint containment, ignoring z (B5.6b-1). Zone interaction
    /// matches by footprint: a rect's z-band comes from the paint-time pick
    /// plane, so the clicked *surface* block's z routinely falls outside it
    /// on slopes — the same z-fragility the erase fix (`clip_xy`) addressed.
    pub fn contains_point_xy(&self, p: Vec3<i32>) -> bool {
        (self.min.x..=self.max.x).contains(&p.x) && (self.min.y..=self.max.y).contains(&p.y)
    }

    /// bastion (B5.6a): clip this region's XY footprint to `[min_xy, max_xy]`,
    /// KEEPING this region's own z-range. `None` if the XY footprints don't
    /// overlap. Used by the erase tool: the erase drag's z comes from the
    /// camera pick-plane, which need not align with where a designation was
    /// painted — so erase matches designations by XY and cancels the
    /// XY-intersection at the *designation's* z (can't silently miss in z,
    /// can't over-erase beyond the brush footprint).
    pub fn clip_xy(&self, min_xy: Vec2<i32>, max_xy: Vec2<i32>) -> Option<Region> {
        let nx = self.min.x.max(min_xy.x);
        let ny = self.min.y.max(min_xy.y);
        let xx = self.max.x.min(max_xy.x);
        let xy = self.max.y.min(max_xy.y);
        (nx <= xx && ny <= xy).then(|| Region {
            min: Vec3::new(nx, ny, self.min.z),
            max: Vec3::new(xx, xy, self.max.z),
        })
    }

    pub fn intersects(&self, other: &Region) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// The overlapping region, if any (both inputs assumed normalized).
    pub fn intersection(&self, other: &Region) -> Option<Region> {
        self.intersects(other).then(|| Region {
            min: Vec3::partial_max(self.min, other.min),
            max: Vec3::partial_min(self.max, other.max),
        })
    }

    /// `self` minus `other`, as up to 6 disjoint boxes exactly covering the
    /// remainder (B5.5 zone erase: the client overlay subtracts erased
    /// regions from stored designation rects). Volume-conserving:
    /// `vol(self) == vol(self ∩ other) + Σ vol(pieces)`.
    pub fn subtract(&self, other: &Region) -> Vec<Region> {
        let Some(o) = self.intersection(other) else {
            return vec![*self];
        };
        let mut pieces = Vec::new();
        // Below / above the overlap (full XY footprint of self).
        if self.min.z < o.min.z {
            pieces.push(Region {
                min: self.min,
                max: Vec3::new(self.max.x, self.max.y, o.min.z - 1),
            });
        }
        if self.max.z > o.max.z {
            pieces.push(Region {
                min: Vec3::new(self.min.x, self.min.y, o.max.z + 1),
                max: self.max,
            });
        }
        // Within the overlap's z-slab: south/north strips (full X of self).
        if self.min.y < o.min.y {
            pieces.push(Region {
                min: Vec3::new(self.min.x, self.min.y, o.min.z),
                max: Vec3::new(self.max.x, o.min.y - 1, o.max.z),
            });
        }
        if self.max.y > o.max.y {
            pieces.push(Region {
                min: Vec3::new(self.min.x, o.max.y + 1, o.min.z),
                max: Vec3::new(self.max.x, self.max.y, o.max.z),
            });
        }
        // Within the overlap's z- and y-slabs: west/east strips.
        if self.min.x < o.min.x {
            pieces.push(Region {
                min: Vec3::new(self.min.x, o.min.y, o.min.z),
                max: Vec3::new(o.min.x - 1, o.max.y, o.max.z),
            });
        }
        if self.max.x > o.max.x {
            pieces.push(Region {
                min: Vec3::new(o.max.x + 1, o.min.y, o.min.z),
                max: Vec3::new(self.max.x, o.max.y, o.max.z),
            });
        }
        pieces
    }
}

/// Max designation volume the server accepts (validation cap; keeps a stray
/// drag from queueing a mountain).
pub const MAX_DESIGNATION_VOLUME: i64 = 64 * 64 * 32;

/// bastion (B5.6b-2): a designation's vertical extent RELATIVE TO THE
/// PAINTED SURFACE, resolved per column — `down` levels below and `up`
/// levels above each cell's own terrain surface (so a zone painted across a
/// slope follows the slope instead of being cut by one flat plane, the
/// B5.MINE-COVERAGE root cause). Defaults preserve the pre-B5.6b-2
/// semantics exactly (`ZExtent::default_for`). The SAME field the §3v mine
/// framework ("8 levels down") and §3w boundary consumers expect — one
/// schema, locked here.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ZExtent {
    /// Levels below the per-cell surface (inclusive of the surface block).
    pub down: u16,
    /// Levels above the per-cell surface.
    pub up: u16,
    /// bastion (B5.6b-2.1, Ben's flat-floor mode): when `Some`, `down` is
    /// IGNORED and every column digs from its own surface to this shared
    /// ABSOLUTE z — the pit bottoms out FLAT AND SQUARE (quarry floors /
    /// foundations / plazas) instead of following the slope. Columns whose
    /// surface already sits at/below the floor get nothing. Identical to
    /// the relative mode on flat ground. `serde(default)` for pre-2.1
    /// stored copies; the WIRE requires client+server in step regardless
    /// (positional struct coding — same ship-together rule as every wire
    /// change this arc).
    #[serde(default)]
    pub floor_z: Option<i32>,
}

impl Default for ZExtent {
    fn default() -> Self {
        Self {
            down: 2,
            up: 0,
            floor_z: None,
        }
    }
}

impl ZExtent {
    /// The default extent per designation kind — matches the previous
    /// hardcoded paint depth (`plane-2 ..= plane`, i.e. down 2 / up 0) so
    /// behavior is unchanged until the UI sets a custom depth. B5.8's
    /// Ladder builds UPWARD from the surface instead (a 4-level rung
    /// column; scroll/stepper adjusts as usual).
    pub fn default_for(kind: DesignationKind) -> Self {
        match kind {
            DesignationKind::Ladder => Self {
                down: 0,
                up: 3,
                floor_z: None,
            },
            _ => Self::default(),
        }
    }

    /// Total levels spanned (down + surface-inclusive + up counting quirk is
    /// folded in: down already includes the surface block's own level).
    /// Flat-floor volumes are terrain-dependent — this is the RELATIVE
    /// span only (validation for flat mode bounds by footprint × a depth
    /// cap server-side).
    pub fn levels(&self) -> u32 { self.down as u32 + 1 + self.up as u32 }

    /// The dig range for one column: from the column's own `surface` down
    /// (or to the shared absolute floor in flat mode) up to `surface + up`.
    /// `None` when the column has nothing to dig (its surface is already
    /// at/below a flat floor). ONE authority — job gen, echo bounds, and
    /// the harness all call this.
    pub fn column_range(&self, surface: i32) -> Option<(i32, i32)> {
        let hi = surface + self.up as i32;
        let lo = match self.floor_z {
            Some(floor) => {
                if surface < floor {
                    return None;
                }
                floor
            },
            None => surface - self.down as i32,
        };
        Some((lo, hi))
    }
}

/// bastion (B5.6b-2, SCHEMA GUARD): THE canonical zone↔asset `purpose`
/// enumeration — locked verbatim from `readme/BASTION-SYSTEM-FRAMEWORKS.md`
/// §2 (the authoritative 8-kind list; other docs carry drifted 7/8/9-kind
/// copies and DEFER to §2). The classification is the zone↔asset matching
/// key ("what can be built in this zone?"). Zones use it as soft preference,
/// not iron law (§2). Do NOT re-derive or extend without an architect pass
/// on frameworks §2 itself.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Purpose {
    /// residential → housing
    Housing,
    /// industrial → production
    Production,
    /// commercial → commerce
    Commerce,
    /// religious → faith
    Faith,
    /// civic → social
    Social,
    /// defensive → defense
    Defense,
    /// storage → storage
    Storage,
    /// agricultural → farming
    Farming,
}

impl Purpose {
    pub fn label(&self) -> &'static str {
        match self {
            Purpose::Housing => "Housing",
            Purpose::Production => "Production",
            Purpose::Commerce => "Commerce",
            Purpose::Faith => "Faith",
            Purpose::Social => "Social",
            Purpose::Defense => "Defense",
            Purpose::Storage => "Storage",
            Purpose::Farming => "Farming",
        }
    }
}

/// bastion (ZONE-0, row 37): ACTIVITY-ZONE subtypes — each carries its
/// locked [`Purpose`]. A zone is a SOFT MAGNET (DF activity zones reframed
/// per the pillar): it RAISES an activity's utility within its footprint,
/// never forces — a colonist with a stronger drive always leaves freely.
/// APPEND-ONLY, wire-stable (the JobKind/Species discipline). ZONE-0 ships
/// Meeting (the proof zone biasing EXISTING idle behavior); further kinds
/// land with their owning blocks (Refuse/Gather = ZONE-1, needs-gated
/// kinds = ZONE-2+).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneKind {
    // `Default` so `DesignationKind` can derive `strum::EnumIter` (the
    // EXHAUSTIVENESS-ASSERTS iteration; a data variant needs a default to
    // enumerate). Meaningless as a "default zone" — only the iterator uses it.
    #[default]
    Meeting,
}

impl ZoneKind {
    pub fn purpose(&self) -> Purpose {
        match self {
            ZoneKind::Meeting => Purpose::Social,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ZoneKind::Meeting => "Meeting",
        }
    }
}

/// bastion (ZONE-0): the ACTIVITY-ZONE mirror — an ECS resource the job
/// board rewrites each arbitration pass (zones are few and tiny) so the
/// AGENT system can read footprints without seeing the board. The magnet
/// reads this; the board stays the single authority.
#[derive(Clone, Debug, Default)]
pub struct ActivityZones(pub Vec<(ZoneKind, Region)>);

/// bastion (ZONE-0): the soft magnet's pull weight on the idle-wander
/// BEARING — the same order as the vanilla patrol-origin pull (0.015 ×
/// wander factor), deliberately weak: a bias, never a command. Graduates
/// to a RON asset when zone kinds multiply (flagged in the Opus notes).
pub const ZONE_MAGNET_WEIGHT: f32 = 0.1;
/// bastion (ZONE-0): beyond this XY distance a zone exerts no pull (no
/// cross-map teleport-attraction; idle colonists drift in when nearby).
/// How many colonists a founding brings.
///
/// THE ONE DEFINITION. Before this there were three numbers with nothing
/// relating them: the shipped widget passed a bare `6`, every acceptance
/// script passed `8`, and the preset's bed plot happens to provide 8 cells.
/// Every scored bar in the program therefore ran at a population the shipped
/// action never produces, and nothing could notice the drift.
///
/// THE BASIS: the bed plot is the binding resource — the only preset element
/// sized per-colonist, and a colonist without a bed has no rest service. So
/// the count is pinned to bed capacity, and
/// `bed_capacity_covers_the_founding_count` (bastion-server) DERIVES that
/// capacity from `FOUNDING_PRESET_V1` and fails if the two part company.
///
/// It lives in `common` because both ends need it: the voxygen widget that
/// founds, and the server-side preset that beds them.
///
/// The VALUE is inherited from the scripts (8 — the saturated case every
/// scored run used), not from the widget's `6`. Changing it is a design call
/// rather than a refactor, which is exactly why it now sits in one place
/// where such a call is visible.
pub const FOUNDING_COLONIST_COUNT: u8 = 8;

pub const ZONE_MAGNET_RANGE: f32 = 48.0;

/// What a painted designation region means. B4 turns these into jobs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, strum::EnumIter)]
pub enum DesignationKind {
    Mine,
    Chop,
    Build,
    Stockpile,
    /// bastion (B5.8): a buildable vertical link — jobs place
    /// `SpriteKind::Ladder` blocks (the native climbable sprite) bottom-up;
    /// pathfinding treats the ladder column's side as vertical edges.
    /// Appended LAST: `DesignationKind` is on the wire (client+server
    /// recompile together; see the b-2 net-protocol ledger note).
    Ladder,
    /// bastion (ZONE-0): an activity zone — a SOFT MAGNET carrying its
    /// [`ZoneKind`] (which carries its [`Purpose`]). Generates NO jobs;
    /// registered as a footprint the utility magnet reads. Appended last
    /// (wire rule as above).
    Zone(ZoneKind),
    /// bastion (GATHER, row 38): forage — the FOOD-LOOP verb. One job per
    /// collectible plant sprite in the painted footprint (the
    /// `TerrainResource` food allowlist); execution is the VANILLA sprite
    /// interaction (`InventoryManip::Collect` — loot tables, capacity and
    /// overflow all owned by the authoritative handler). Appended last
    /// (wire rule as above).
    Gather,
    /// bastion (B7-1, row 44): a BED — placed like a Ladder (a designation
    /// with its own completion arm placing a specific named sprite;
    /// vanilla ships the sprites), registered as a [`BedSlot`] on
    /// completion. The rest-loop venue B7-2's preemption targets.
    /// Appended last (wire rule as above).
    Bed,
    /// bastion (ITEM 14, axis 3 — POST): a guard station. A designation like
    /// a [`Bed`](DesignationKind::Bed) — a named place a colonist is assigned
    /// to hold, not a block to mine. Appended last (wire rule as above).
    GuardPost,
    /// bastion (ITEM 14, axis 3 — PATROL): one waypoint of a patrol route.
    /// ★ Post and patrol are BOTH assignment types per Ben's ruling; shipping
    /// only one would collapse a parameterised axis into a policy, which is
    /// the exact thing the ruling forbids. Appended last (wire rule as above).
    PatrolPoint,
    /// bastion (FARM/PROD-2, row 46): a PERSISTENT farm footprint — the
    /// paint registers the plot (the Stockpile-registration precedent)
    /// and generates NO jobs itself; the farm trigger pass reads each
    /// cell's state (raw -> till, tilled -> sow, mature -> harvest) and
    /// generates the right job, forever (cells CYCLE — unlike Mine/
    /// Gather cells, a farm cell never completes out of the footprint).
    /// Appended LAST (wire rule).
    Farm,
}

/// bastion (task #64, KindAffordance): what `Job::pos` PHYSICALLY MEANS —
/// the stance a colonist must commit to reach it. Stamped at CREATION, not
/// derived from `kind` at lookup time: `DesignationKind::Farm` alone can't
/// answer this (see `AtTarget` below) because the SAME kind spans two
/// different physical shapes depending on which phase created the job
/// instance — the creator knows which shape it built; a table keyed on
/// kind alone cannot recover that after the fact. NO `Default`: every
/// `Job` construction site must choose explicitly (this campaign's
/// original bug was Mine's stance logic silently inherited by every other
/// kind that never opted in — a missing arm here is now a compile error,
/// not a corpus failure discovered later).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AffordanceClass {
    /// `job.pos` IS solid pre-completion (removal-shaped): on-top is the
    /// preferred stance (real support exists), falling back to an
    /// adjacent-ground stance only when on-top is physically unusable.
    /// Mine, Chop, Gather.
    SolidTarget,
    /// `job.pos` is EMPTY pre-completion and becomes solid ONLY on
    /// completion (construction-shaped): on-top has no support by
    /// definition — commit to a cardinal-adjacent stance at `job.pos`'s
    /// own level, on real ground. Built, evidenced-plausible, and
    /// CORRECTLY-TYPED for Build/Bed/a Ladder base — but NOT currently
    /// stamped on any live `Job` (task #64, DECISIONS #45, pure-refactor
    /// scope): Ladder's own control (see `OnTopAlways`'s doc) falsified
    /// the physical-support argument as a PREDICTOR in a system with no
    /// execution-proximity check, and Build/Bed never got their own
    /// control before that finding landed — shipping this on the same
    /// unproven argument for three more kinds would repeat exactly what
    /// just got reverted for Ladder. Reserved for whichever of Build/Bed
    /// earns its own evidence-gated row (change `designation_affordance`'s
    /// match arm, nothing else needs new plumbing).
    AdjacentToBase,
    /// Unconditional on-top — no terrain read, no possibility of refusing
    /// a stance. Mine/Chop's stance function (`SolidTarget`) can fall back
    /// or return `None`; this can't, by construction. Covers Build, Bed,
    /// Ladder, and all three Farm phases (till/sow/harvest) — every kind
    /// whose pre-task-#64 behaviour was the blind
    /// `standable.get(&job_id).unwrap_or(Vec3::unit_z())` default, now
    /// DECLARED rather than silently inherited (task #64, DECISIONS #45:
    /// pure refactor, zero behaviour change from pre-#64 for any of them).
    /// The name records a real, evidenced finding for Ladder specifically:
    /// a controlled A/B (DECISIONS #44) falsified an EARLIER,
    /// terrain-conditional version of Ladder's rule (on-top only once the
    /// rung below read solid, then named `LadderContinuation` for that
    /// now-deleted mechanism) — the conditional version REGRESSED a
    /// previously-working placement rate (5/5 -> 2/5) because nothing
    /// downstream enforces the stance for a placement kind, so a stance
    /// that can refuse is strictly worse than one that always answers —
    /// see `ladder_stance`'s own doc for the full A/B. Build/Bed/Farm
    /// share the VALUE (on-top always) but not yet the EVIDENCE; their own
    /// controls are filed as separate future rows, not assumed by
    /// association with Ladder's.
    OnTopAlways,
    /// `job.pos` is the STAND cell itself, not a thing to reach onto or
    /// beside — the colonist's feet land AT `job.pos`, support comes from
    /// the (already solid) cell below it. Neither on-top nor adjacent.
    /// Built and correctly-typed for Farm's SOW/HARVEST sub-jobs
    /// (job.pos = the crop cell one above tilled ground, which IS the
    /// working position — a real semantic difference from TILL's job.pos,
    /// preserved in the farm pass's own comments) but NOT currently
    /// stamped on any live `Job`, same reasoning as `AdjacentToBase`: no
    /// demonstrated failure this fixes (this session's counter-control
    /// showed `farm_tilled:false` is unexplained under either stance), so
    /// it ships as a behaviour change only once Farm's own control
    /// demonstrates a sow/harvest failure the split actually fixes.
    AtTarget,
    /// No terrain-edit stance requirement — the target is wherever the
    /// referenced entity/zone/self actually is (Haul, DepositRun, RestAt,
    /// EatFrom, Despond). Resolves to the pre-existing on-top default,
    /// preserving all currently-working self-job behavior unchanged.
    Untargeted,
}

impl DesignationKind {
    pub fn label(&self) -> &'static str {
        match self {
            DesignationKind::GuardPost => "GuardPost",
            DesignationKind::PatrolPoint => "PatrolPoint",
            DesignationKind::Mine => "Mine",
            DesignationKind::Chop => "Chop",
            DesignationKind::Build => "Build",
            DesignationKind::Stockpile => "Stockpile",
            DesignationKind::Ladder => "Ladder",
            DesignationKind::Zone(z) => z.label(),
            DesignationKind::Gather => "Gather",
            DesignationKind::Bed => "Bed",
            DesignationKind::Farm => "Farm",
        }
    }

    /// bastion (EXHAUSTIVENESS-ASSERTS, row 51.52): is this kind placed by
    /// the player PAINTING it with the overseer area toolbar (a
    /// `ToolMode::Designate` button)? An EXHAUSTIVE match — no wildcard — so
    /// a NEW `DesignationKind` variant fails to COMPILE here until it's
    /// categorized. Paired with the voxygen parity test, this is the
    /// compile-time guard the FARM-PALETTE bug lacked (Farm was a real
    /// paintable kind silently missing from the hand-listed `ToolMode::ALL`;
    /// the append-only-enum-vs-hand-mirrored-array trap). Every `true` kind
    /// MUST have a `ToolMode::ALL` entry; every `ToolMode::Designate(_)`
    /// MUST be `true` here.
    pub fn is_tool_paintable(&self) -> bool {
        match self {
            // ITEM 14: both are player-painted assignments.
            DesignationKind::GuardPost | DesignationKind::PatrolPoint => true,
            DesignationKind::Mine
            | DesignationKind::Chop
            | DesignationKind::Gather
            | DesignationKind::Build
            | DesignationKind::Stockpile
            | DesignationKind::Farm
            | DesignationKind::Ladder
            // Bed: RULED a confirmed bug (architect, this pass) — 3rd
            // instance of the missing-wiring class (Mine-legend/Farm/Bed):
            // Bed has a reserved palette color (voxygen tools.rs) + a
            // completion arm placing a Bedroll sprite ("placed like a
            // Ladder", which IS paintable), yet had no ToolMode::ALL button.
            // Categorized paintable here + given its button in the same pass;
            // EXHAUSTIVENESS surfaced it exactly as intended.
            | DesignationKind::Bed => true,
            // NON-paintable: Zone(_) is placed via the activity-zone UX, not
            // the paint toolbar.
            DesignationKind::Zone(_) => false,
        }
    }

    /// bastion (CHOP redesign, FR10 — the first AREA-kind, classified not
    /// special-cased): how a designation's painted footprint resolves.
    /// `Volume` kinds paint a 3D slab (the `ZExtent`/flat-floor model);
    /// `Area2D` kinds paint a PURE XY footprint — no depth stepper, no
    /// z-extent on the wire (`z_extent: None`), and the server resolves
    /// content from the footprint itself (Chop: whole trees rooted in it).
    /// The UI (hide the stepper), the paint path (2D vs volume), and the
    /// server (area vs slab job-gen) all branch off this ONE flag, so a
    /// future Gather/Forage/surface-zone kind gets the branch free.
    pub fn footprint_mode(&self) -> FootprintMode {
        match self {
            // ITEM 14: a station/waypoint is a SURFACE place, like a zone.
            DesignationKind::GuardPost | DesignationKind::PatrolPoint => FootprintMode::Area2D,
            // ZONE-0: zones are surface activity areas — pure XY.
            // GATHER: forage sweeps a surface footprint (the branch this
            // doc-comment promised it would get free).
            // FARM (row 46): a field is a surface plot (the same free
            // branch this doc-comment promised surface kinds).
            DesignationKind::Chop
            | DesignationKind::Zone(_)
            | DesignationKind::Gather
            | DesignationKind::Farm => FootprintMode::Area2D,
            DesignationKind::Mine
            | DesignationKind::Build
            | DesignationKind::Stockpile
            | DesignationKind::Ladder
            | DesignationKind::Bed => FootprintMode::Volume,
        }
    }

    /// bastion (B5.6b-2): the canonical [`Purpose`] a designation maps to,
    /// for zone↔asset matching (frameworks §2). `Build` is `None` — a build
    /// designation constructs a structure whose OWN asset purpose applies;
    /// the designation itself carries none.
    pub fn purpose(&self) -> Option<Purpose> {
        match self {
            // ITEM 14: an assignment place carries no ASSET purpose.
            DesignationKind::GuardPost | DesignationKind::PatrolPoint => None,
            DesignationKind::Mine
            | DesignationKind::Chop
            | DesignationKind::Gather
            | DesignationKind::Farm => Some(Purpose::Production),
            DesignationKind::Stockpile => Some(Purpose::Storage),
            // ZONE-0: the zone kind carries its own locked Purpose.
            DesignationKind::Zone(z) => Some(z.purpose()),
            // B7-1: a bed IS its purpose (unlike Build's blank canvas).
            DesignationKind::Bed => Some(Purpose::Housing),
            // Structures carry their asset's purpose, not the designation's.
            DesignationKind::Build | DesignationKind::Ladder => None,
        }
    }
}

/// bastion (CHOP redesign, FR10): a designation's footprint semantics — see
/// [`DesignationKind::footprint_mode`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FootprintMode {
    /// A 3D slab: the painted XY × a per-column [`ZExtent`] (surface-relative
    /// or flat-floor).
    Volume,
    /// A pure XY footprint — no depth; the server resolves the content.
    Area2D,
}

/// bastion (B5.8): headroom cleared above each ramp step — the step's own
/// air block plus enough clearance for the pathfinder's 1-up edge (which
/// requires `pos+2z` non-solid at the source step).
pub const CARVE_STEP_CLEARANCE: i32 = 3;

/// bastion (B5.8, SHARED LIBRARY — design law from DF-DIG-VERBS §2: this is
/// THE ramp/stair decomposition; B5.8's auto-carve and DIG-1's player Ramp
/// verb are two callers of this one routine. Do NOT build a second one.):
/// decompose a walkable stepped stair from `from` (lower, e.g. a trapped
/// digger's feet) up to `to.z` (upper, e.g. the rim), heading toward `to`'s
/// XY, into the ordered list of blocks to DIG.
///
/// Geometry: one step per level, advancing one column per step. Steps
/// follow the remaining delta toward `to` (voxel Bresenham); when the
/// preferred column is NOT `allowed` (outside the colony claim mask, the
/// DIG-1 designation box, …) the stair SWITCHES BACK — it tries the
/// perpendicular headings, then the reverse, snaking up inside the allowed
/// region like a real stairwell. A column is never reused (a switchback
/// directly above an earlier step would have its floor dug out from under
/// it). Per step column, [`CARVE_STEP_CLEARANCE`] blocks of air space are
/// cleared; only currently-solid blocks (per `is_solid`) are emitted.
///
/// ORDERING INVARIANT (the reachability law): emission is bottom-up,
/// column by column — every dig position is adjacent-reachable from the
/// standing set established by the previous steps (the digger carves step
/// k standing on/beside step k-1; nothing is removed from beneath it).
///
/// Returns `None` when the full rise cannot be routed (no allowed column
/// at some level — the caller falls back to a ladder or gives up), and
/// `Some(digs)` when it can (digs may be sparse where the route crosses
/// already-open air).
pub fn carve_ramp(
    from: Vec3<i32>,
    to: Vec3<i32>,
    is_solid: &dyn Fn(Vec3<i32>) -> bool,
    allowed: &dyn Fn(Vec3<i32>) -> bool,
) -> Option<Vec<Vec3<i32>>> {
    let rise = to.z - from.z;
    if rise <= 0 {
        return None;
    }
    let delta = to.xy() - from.xy();
    let mut remaining = delta;
    let mut col = from.xy();
    let mut heading = Vec2::zero();
    let mut used: Vec<Vec2<i32>> = vec![col];
    let mut digs = Vec::new();
    for k in 1..=rise {
        let feet_z = from.z + k;
        // Preferred step: toward the target; then current heading; then
        // the perpendiculars; reversal last (a straight-back reversal can
        // only work after a sideways jog — the used-column check enforces
        // that automatically).
        let toward = if remaining == Vec2::zero() {
            heading
        } else if remaining.x.abs() >= remaining.y.abs() {
            Vec2::new(remaining.x.signum(), 0)
        } else {
            Vec2::new(0, remaining.y.signum())
        };
        let mut candidates = vec![toward];
        if heading != Vec2::zero() {
            candidates.push(heading);
            let perp = Vec2::new(-heading.y, heading.x);
            candidates.push(perp);
            candidates.push(-perp);
            candidates.push(-heading);
        } else {
            candidates.extend([
                Vec2::new(1, 0),
                Vec2::new(-1, 0),
                Vec2::new(0, 1),
                Vec2::new(0, -1),
            ]);
        }
        let step = candidates.into_iter().find(|s| {
            *s != Vec2::zero()
                && !used.contains(&(col + *s))
                && allowed(Vec3::new(col.x + s.x, col.y + s.y, feet_z))
                // FLOOR RULE: the block under the step's feet must be solid
                // (it lies below the cleared range, so it survives the dig).
                // A stair cannot route through already-open space — that is
                // the ladder's job (the caller's fallback).
                && is_solid(Vec3::new(col.x + s.x, col.y + s.y, feet_z - 1))
        })?;
        if step == toward && remaining != Vec2::zero() {
            remaining -= step;
        }
        heading = step;
        col += step;
        used.push(col);
        for dz in 0..CARVE_STEP_CLEARANCE {
            let p = Vec3::new(col.x, col.y, feet_z + dz);
            if is_solid(p) {
                digs.push(p);
            }
        }
    }
    Some(digs)
}

/// A divine influence applied at/around a point. B13 implements these.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InfluenceKind {
    Bless,
    Rain,
}

impl InfluenceKind {
    pub fn label(&self) -> &'static str {
        match self {
            InfluenceKind::Bless => "Bless",
            InfluenceKind::Rain => "Rain",
        }
    }
}

/// A context-menu verb aimed at a target (entity or block). B2a: server-echo
/// stub. B3/B4/B12/B2b give the entity verbs behavior; note that force-action
/// and possession are deliberately *not* free verbs here — they are metered
/// god powers (B2b).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextVerb {
    /// Designate the targeted block/feature for mining.
    Mine,
    /// Designate the targeted tree for chopping.
    Chop,
    /// Place a build marker at the target.
    Build,
    /// Mark a stockpile at the target.
    Stockpile,
    /// Open/inspect the target (client-side affordance, echoed for the log).
    Inspect,
    /// Set a colonist policy (B3+; stub).
    SetPolicy,
    /// Embody the target (B12; shown greyed, stub).
    Embody,
    /// Force an action (B2b; shown greyed, stub — metered god power).
    ForceAction,
    /// Found the player colony here (B3): spawns the starting band.
    FoundColony,
}

impl ContextVerb {
    pub fn label(&self) -> &'static str {
        match self {
            ContextVerb::Mine => "Mine",
            ContextVerb::Chop => "Chop",
            ContextVerb::Build => "Build",
            ContextVerb::Stockpile => "Stockpile",
            ContextVerb::Inspect => "Inspect",
            ContextVerb::SetPolicy => "Set policy",
            ContextVerb::Embody => "Embody",
            ContextVerb::ForceAction => "Force action",
            ContextVerb::FoundColony => "Found colony",
        }
    }

    /// Verbs that exist on the menu but are stubbed/greyed until a later
    /// block gives them rules (B2b metering, B12 possession).
    pub fn stubbed(&self) -> bool { matches!(self, ContextVerb::Embody | ContextVerb::ForceAction) }
}

/// A context-action target: an entity (by Uid) or a world block.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ContextTarget {
    Entity(crate::uid::Uid),
    Block(Vec3<i32>),
}

// ─── B4: jobs ───────────────────────────────────────────────────────────────

/// Job identifier (board-scoped, monotonically allocated).
pub type JobId = u64;

/// The kind of work a job requires — maps onto [`WorkPriorities`] fields.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkType {
    Mine,
    Chop,
    Build,
    Haul,
    Cook,
    /// bastion (FARM/PROD-2, row 46): field work — till, sow, harvest.
    /// Appended LAST (wire rule).
    Farm,
    /// bastion (ITEM 14): standing watch. Its own work type because guarding
    /// is neither hauling nor building — a guard occupies a colonist without
    /// producing, which every throughput measure must be able to see
    /// separately. Appended LAST (wire rule).
    Guard,
}

impl WorkType {
    pub fn label(&self) -> &'static str {
        match self {
            WorkType::Guard => "guard",
            WorkType::Mine => "mine",
            WorkType::Chop => "chop",
            WorkType::Build => "build",
            WorkType::Haul => "haul",
            WorkType::Cook => "cook",
            WorkType::Farm => "farm",
        }
    }
}

impl DesignationKind {
    /// The work-type this designation's jobs require. (Build/Stockpile job
    /// *generation* lands with B5 blueprints / B6 zones; the mapping exists
    /// now so priorities are honored from day one.)
    pub fn work_type(&self) -> WorkType {
        match self {
            // ITEM 14: both assignment types map to the same work; the
            // DIFFERENCE between post and patrol is the job, not the labour.
            DesignationKind::GuardPost | DesignationKind::PatrolPoint => WorkType::Guard,
            DesignationKind::Mine => WorkType::Mine,
            DesignationKind::Chop => WorkType::Chop,
            // B5.8: placing a ladder is construction work. B7-1: so is a
            // bed.
            DesignationKind::Build | DesignationKind::Ladder | DesignationKind::Bed => {
                WorkType::Build
            },
            // ZONE-0: zones generate no jobs; Haul is the inert mapping
            // (same as Stockpile's pre-B6 stance — priorities stay honored
            // if a zone kind ever emits work).
            DesignationKind::Stockpile | DesignationKind::Zone(_) => WorkType::Haul,
            // GATHER: foraging is item logistics — Haul skill/priorities
            // apply, no tool required (bare-hand tool_factor = base rate).
            // A dedicated Forage skill is a designer-lane taxonomy call.
            DesignationKind::Gather => WorkType::Haul,
            DesignationKind::Farm => WorkType::Farm,
        }
    }
}

/// bastion (B6 JOB-CORE): a painted stockpile zone's stable id (the board
/// hands them out; `Haul.destination` references one).
/// bastion (B7-0, row 44): one survival need's tuning — decay drains the
/// meter per game-second toward 0.0; the need penalizes mood only BELOW
/// its comfort band; `weight` is NEGATIVE (a shortfall subtracts).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct NeedTuning {
    pub decay_per_sec: f32,
    pub comfort: f32,
    pub weight: f32,
    /// bastion (B7-2): below this the need PREEMPTS work (the low edge
    /// of the hysteresis band; the high edge is `comfort + SLEEP_MARGIN`
    /// — B7-1's wake threshold IS the doc's NEED_SATISFIED). Wide band =
    /// no work/need flicker. serde-default keeps older RONs parseable.
    #[serde(default = "default_need_interrupt")]
    pub interrupt: f32,
}

fn default_need_interrupt() -> f32 { 0.2 }

/// bastion (ITEM 14, AXIS 2 — HOLD-vs-FLEE). Ben's ruling: *"a per-colonist
/// threshold (bravery), not a global rule: guarding outranks flee up to a
/// breaking point that varies by the individual"*.
///
/// Deliberately the SAME shape as [`default_need_interrupt`] — the ruling names
/// that pattern by hand ("the same per-colonist clamp pattern
/// `default_need_interrupt()` already uses"), and matching it means personality
/// and veterancy can move this later without a second mechanism.
///
/// Semantics: a guarding colonist holds while `health.fraction() >= bravery`.
/// **Lower = braver** (holds down to a worse wound), so the field reads the
/// same direction as `flee_health`, which it competes with.
///
/// serde-defaulted so pre-item-14 saves parse and get the neutral value.
pub fn default_guard_bravery() -> f32 { 0.5 }

/// bastion (ITEM 14, AXIS 1 — RESPONSE MODE). Ben's ruling: *"none of these are
/// mutually exclusive, it just depends on the situation, NPCs, civs"* — so the
/// mode is a **parameter carried by the assignment**, never a global policy.
///
/// v1 ships the two ENDS of the escalation the ruling describes
/// (watch → alarm → fight). Two values, because a parameter with ONE exercised
/// value is a constant wearing a parameter's name — and the row's bar 1 is
/// exactly that both are observed live, each changing behaviour by name.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardMode {
    /// Watch and raise the alarm; do NOT engage. The de-escalated end.
    Alarm,
    /// Engage the hostile. The escalated end.
    Fight,
}

/// T0.4 (master build order; ledger #54): a SIM-clock duration in seconds —
/// the units-of-measure boundary for tuning fields. Transparent for RON
/// compat; arithmetic goes through [`crate::resources::Time`] explicitly so
/// a wall-clock or tick-count value cannot be mixed in silently (the
/// LootOwner/ENGOPT6 class, closed at the type level).
#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct SimSecs(pub f64);

impl SimSecs {
    /// The sim instant this duration ends if started `now`.
    pub fn after(self, now: crate::resources::Time) -> crate::resources::Time {
        crate::resources::Time(now.0 + self.0)
    }

    /// Whether this duration has fully elapsed between a stored sim instant
    /// and `now`.
    pub fn has_elapsed(self, since: f64, now: crate::resources::Time) -> bool {
        now.0 - since >= self.0
    }
}

/// bastion (B7-0): the needs/mood tuning — RON
/// (`assets/common/bastion_mood.ron`), graceful default (the
/// `SeasonConfig` idiom). Holds the base and the three bodily-need
/// tunings ONLY: the thought table keys on `ChronicleKind`, which lives
/// in the rtsim crate common cannot depend on — it ships as its own
/// server-side asset, and [`crate::comp::bastion::mood_formula`] takes
/// the summed thought term as a plain input (the formula is
/// layering-agnostic by construction).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MoodConfig {
    pub mood_base: f32,
    pub hunger: NeedTuning,
    pub rest: NeedTuning,
    pub recreation: NeedTuning,
    /// bastion (B7-3): mood below this arms the breakdown staircase
    /// (despondent-only v1). serde-defaulted for older RONs.
    #[serde(default = "default_break_minor")]
    pub break_minor: f32,
    /// Sustained-below window before the per-cadence roll starts.
    #[serde(default = "default_break_sustain")]
    pub break_sustain_secs: SimSecs,
    /// Per-cadence break chance once sustained (not an instant flip —
    /// forgiving, per the prior art).
    #[serde(default = "default_break_chance")]
    pub break_chance: f32,
    /// How long a despondent colonist stays down.
    #[serde(default = "default_despond_secs")]
    pub despond_secs: SimSecs,
}

fn default_break_minor() -> f32 { 0.25 }
fn default_break_sustain() -> SimSecs { SimSecs(30.0) }
fn default_break_chance() -> f32 { 0.15 }
fn default_despond_secs() -> SimSecs { SimSecs(60.0) }

impl Default for MoodConfig {
    fn default() -> Self {
        Self {
            mood_base: 0.6,
            hunger: NeedTuning {
                // AUTON-2 STEP-3 RE-TUNE (2026-08-08): matches the shipped
                // asset (assets/common/bastion_mood.ron). Was 0.0004 --
                // kept in sync by bastion_mood_config_matches_shipped_asset
                // (this file's test module) so a future retune that edits
                // the RON without touching this copy goes red immediately.
                decay_per_sec: 0.000889,
                comfort: 0.5,
                weight: -0.5,
                interrupt: 0.2,
            },
            rest: NeedTuning {
                // Was 0.0003 -- see the hunger field's comment above.
                decay_per_sec: 0.000444,
                comfort: 0.5,
                weight: -0.4,
                interrupt: 0.2,
            },
            recreation: NeedTuning {
                decay_per_sec: 0.0002,
                comfort: 0.4,
                weight: -0.15,
                interrupt: 0.0,
            },
            break_minor: 0.25,
            break_sustain_secs: SimSecs(30.0),
            break_chance: 0.15,
            despond_secs: SimSecs(60.0),
        }
    }
}

impl crate::assets::FileAsset for MoodConfig {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Result<Self, crate::assets::BoxedError> {
        crate::assets::load_ron(&bytes)
    }
}

impl MoodConfig {
    /// The loaded tuning (asset-backed; "hot-reloadable" only in a build
    /// that compiles the `hot-reloading` cargo feature -- `bastion-harness`
    /// does NOT, so its asset content is effectively FROZEN for the whole
    /// process: `assets_manager` caches on first load, and without the
    /// feature-gated file watcher a later rewrite is invisible by
    /// construction, not by timing. Found the hard way (AUTON-2 Step 1,
    /// 2026-08-08): a 500ms real-wall-clock sleep after rewriting the
    /// asset changed nothing, because the code path that would have
    /// noticed isn't compiled into that binary at all. Anyone tempted to
    /// mutate `assets/common/bastion_mood.ron` under a running harness
    /// scenario expecting a live pickup will lose the same hours.
    ///
    /// For test-only tuning, `BASTION_AUTON2_MOOD_OVERRIDE` below is the
    /// actual mechanism: it bypasses the asset pipeline entirely (Opus's
    /// ruling, 2026-08-08 -- routing a needs test through the asset/hot-
    /// reload machinery would make it depend on caching/timing/watcher
    /// behavior that has nothing to do with needs; a planted case should
    /// fail for exactly one reason).
    ///
    /// Compiled default on a missing/broken asset — graceful, never a
    /// panic.
    pub fn current() -> Self {
        // AUTON-2 Step 1 (2026-08-08, Opus-directed): test-only override,
        // env-gated, off by default. REPLACES the config wholesale when
        // set -- never shadows/merges with the shipped asset (a partial
        // override would be a second source of truth for tuning). The
        // asset path below is completely untouched when unset: same
        // call, same behaviour, checkable by reading it back (this is
        // the "prove the negative" half of the acceptance criteria).
        if let Ok(ron) = std::env::var("BASTION_AUTON2_MOOD_OVERRIDE")
            && let Ok(cfg) = crate::assets::load_ron::<Self>(ron.as_bytes())
        {
            return cfg;
        }
        use crate::assets::AssetExt;
        Self::load("common.bastion_mood")
            .map(|h| h.read().clone())
            .unwrap_or_default()
    }
}

/// bastion (FOCUS-0, row 43): the PERSONAL-NEED vocabulary — the locked
/// venue interface (frameworks §2-adjacent, the `Purpose`/`ChronicleKind`
/// discipline: append-only, never reorder — wire- and save-stable).
/// Venues declare what they satisfy against THIS enum (DF-RELIGION's
/// temple satisfies `Pray`, the tavern `Drink`/`Socialize`, …); the
/// facet-derived per-colonist WEIGHTS are B-AG3-dependent and explicitly
/// NOT built yet (deferred with it), as are self-generated need-jobs
/// (FOCUS-1) and the focus→work_rate hook (FOCUS-2). This block is the
/// vocabulary + the save-shape ONLY.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Need {
    Pray,
    Socialize,
    Drink,
    Craft,
    Family,
    SeeAnimals,
    AdmireArt,
    Learn,
    Acquire,
    Fight,
}

/// bastion (B-AG3 slice 1): the VALUE vocabulary — what a colonist
/// believes in / prioritizes. DISTINCT from vanilla's Big-Five
/// [`crate::rtsim::Personality`] (temperament — HOW one reacts): a value
/// is WHAT one holds dear, the axis the culture-keying tables (CHAR-1)
/// and the chronicle-thought care weighting read. Same discipline as
/// [`Need`]: append-only, never reorder — wire- and save-stable. The
/// starting set pulls from the build report's culture examples
/// (glory/tradition/kin/wealth/piety/nature) + DF's ethic/value list
/// (craftsmanship, independence) — not invented from scratch.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Value {
    Glory,
    Tradition,
    Kin,
    Wealth,
    Piety,
    Nature,
    Craft,
    Freedom,
}

/// bastion (B7-1, row 44): what kind of bed a [`BedSlot`] is — drives the
/// completion sprite and the quality scalar (a frame rests better than a
/// bedroll). v1 places bedrolls; frames are a data extension (vanilla
/// ships both sprite families).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BedKind {
    Bedroll,
    Frame,
}

impl BedKind {
    /// The rest-recovery quality multiplier (design §4).
    pub fn quality(&self) -> f32 {
        match self {
            BedKind::Bedroll => 0.6,
            BedKind::Frame => 1.0,
        }
    }
}

/// bastion (B7-1): one bed's slot state — the reservations-table shape
/// (capacity-1 occupancy, insert on claim, remove on release/death),
/// keyed by block position on the board. OWNERSHIP truth persists on
/// `BastionColonist::owned_bed` (the LOD-0 mirror pattern — the board is
/// session-state); the slot's `owner` is the fast lookup written at
/// assignment. `occupant` is transient by nature (a claim, not a fact
/// about the world).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BedSlot {
    pub kind: BedKind,
    pub owner: Option<crate::uid::Uid>,
    pub occupant: Option<crate::uid::Uid>,
}

pub type ZoneId = u64;
/// bastion (B6 JOB-CORE): a reservation's stable id — ONE item can be
/// reserved by ONE job (the double-spend guard); the table lives on the
/// board (single authority, D2 — stock itself stays DERIVED from physical
/// items, never a second mutable count).
pub type ReservationId = u64;

/// bastion (B6 JOB-CORE): the job's TYPE. `Designated` wraps every pre-B6
/// designation job unchanged (a type change, not a behavior change);
/// `Haul` carries a loose item into a stockpile zone. APPEND-ONLY, never
/// reorder (wire-stable — the NightHorror Species discipline). Later
/// variants land only with their owning block (Gather = row 38).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum JobKind {
    Designated(DesignationKind),
    Haul {
        /// The loose `PickupItem` entity to carry.
        item: crate::uid::Uid,
        /// The stockpile zone to carry it into.
        destination: ZoneId,
    },
    /// bastion (GATHER deposit ruling): the ONE end-of-forage stockpile
    /// trip — created PRE-CLAIMED for a specific colonist when it runs out
    /// of claimable Gather targets while still carrying forage (the bag is
    /// the batch unit; no per-sprite round-trips). Rides the whole proven
    /// job pipeline (travel/watchdog/stuck-economy) instead of a bespoke
    /// steer — the FR15 lesson. Appended last (wire rule as above).
    DepositRun {
        /// The stockpile zone to empty the forage bag into.
        destination: ZoneId,
    },
    /// bastion (B7-1, row 44): SLEEP at a bed — travel to the bed cell
    /// (the proven pipeline), occupy its [`BedSlot`] (capacity-1), and
    /// restore `rest` per tick scaled by bed quality until the comfort
    /// band, depositing a sleep-quality thought at completion. Created
    /// PRE-CLAIMED (the DepositRun shape); the automatic
    /// preempt-on-threshold trigger is B7-2's. Appended last (wire rule
    /// as above).
    RestAt {
        /// The bed's block position ([`BedSlot`] key).
        bed_pos: Vec3<i32>,
    },
    /// bastion (B7-3, row 44): EAT a food item — the hunger need-job
    /// (the RestAt shape: pre-claimed by the NEED-CHECK pass, rides the
    /// pipeline). Targets a loose/stockpiled food ITEM by Uid (items
    /// move — the Haul leg-1 vanish-confirm pattern) with a B6
    /// reservation (the double-spend guard applies to food exactly as
    /// to build materials). Appended last (wire rule).
    EatFrom {
        /// The food item entity.
        item: crate::uid::Uid,
    },
    /// bastion (B7-3): the BREAKDOWN state (design §3, despondent-only
    /// v1) — "the break is itself a top-tier job in the same preemption
    /// frame": a pre-claimed self-job at the colonist's own feet that
    /// idles until `until`, blocking all claims (an honest visible
    /// collapse, never a frozen sim). Appended last (wire rule).
    Despond {
        /// Sim time when the despondency lifts.
        until: f64,
    },
    /// bastion (ITEM 11, row 2026-08-16): RECREATION — the need's first
    /// PRODUCER. Same shape as [`JobKind::Despond`]: a pre-claimed
    /// self-job at the colonist's own feet that idles until `until`,
    /// restoring `recreation` while it runs.
    ///
    /// WHY THIS EXISTS. Recreation was a ONE-WAY RATCHET: it decays at
    /// `decay_per_sec`, feeds a mood penalty through `shortfall`, and
    /// NOTHING in the codebase raised it — hunger has `EatFrom`, rest has
    /// `RestAt`, and recreation had no counterpart, so every colony ran
    /// an unopposed mood drag that no fixture was long enough to see
    /// (3000 sim-seconds to cross comfort from 1.0, against 2400–3600
    /// total run length). Measured, not assumed: `PendingNeed` had no
    /// Recreate arm and recreation's interrupt is 0 = never preempts.
    ///
    /// LOWEST PRIORITY BY CONSTRUCTION. Despond/RestAt/EatFrom answer
    /// collapse, exhaustion and starvation; this answers boredom. It is
    /// created only when nothing more urgent is pending, so a hungry
    /// colonist never relaxes instead of eating.
    ///
    /// Appended last (the wire rule every self-job above follows).
    Recreate {
        /// Sim time when the break ends.
        until: f64,
    },
    /// bastion (ITEM 14): a GUARD assignment. Carries its own
    /// [`GuardMode`] (axis 1) rather than reading a global setting, so two
    /// guards in one colony can hold different postures — which is the
    /// ruling's whole point and is what bar 1 scores.
    ///
    /// `route` is axis 3: a POST is a route of length 1, a PATROL is length
    /// >= 2. Representing both in ONE variant keeps them the same kind of
    /// thing, so nothing downstream can grow a post-only assumption.
    /// Appended last (wire rule).
    Guard {
        /// Axis 1 — this assignment's response mode.
        mode: GuardMode,
        /// Axis 3 — the station, and the patrol's first point.
        post: Vec3<i32>,
        /// Axis 3 — `Some` makes this a PATROL between `post` and here;
        /// `None` makes it a POST. Two points, not an arbitrary route:
        /// `JobKind` is `Copy` (a wire type matched in dozens of places), so a
        /// `Vec` here would strip `Copy` from the whole enum for a v1 that
        /// only has to prove patrol is a real assignment TYPE, not that routes
        /// can be long. The ruling asks for "the smallest value set that
        /// EXERCISES it" — two points visit >= 2 distinct waypoints, which is
        /// exactly what bar 2 scores. A longer route becomes a board-side
        /// list keyed by `JobId` when something needs one.
        patrol_to: Option<Vec3<i32>>,
        /// Which end the colonist is currently heading for: `false` = `post`,
        /// `true` = `patrol_to`. Ignored for a post.
        at_far_end: bool,
    },
}

impl JobKind {
    /// The wrapped designation kind, `None` for non-designation jobs —
    /// the compat shim for the many call sites that match on it.
    pub fn designation(&self) -> Option<DesignationKind> {
        match self {
            JobKind::Designated(d) => Some(*d),
            // ITEM 14: a Guard JOB carries its own place (post/patrol_to), so
            // it is not "a job on a designation" the way Mine/Build are. The
            // GuardPost/PatrolPoint DESIGNATIONS are what the player paints;
            // the JOB is what a colonist holds.
            JobKind::Guard { .. } => None,
            JobKind::Haul { .. }
            | JobKind::DepositRun { .. }
            | JobKind::RestAt { .. }
            | JobKind::EatFrom { .. }
            | JobKind::Despond { .. }
            | JobKind::Recreate { .. } => None,
        }
    }

    /// Is this a designation job of the given kind?
    pub fn is(&self, kind: DesignationKind) -> bool { self.designation() == Some(kind) }
}

/// One unit of colonist work — a block-level task generated from a
/// designation (B4). Serde-ready (B10). `claimed_by` is a transient claim
/// (entity `Uid`); claims are released on cancel/failure/demote.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub kind: JobKind,
    pub work: WorkType,
    /// Target block.
    pub pos: Vec3<i32>,
    /// Minimum skill level required (0 = anyone). Unused by v1 generation.
    pub skill_floor: u16,
    /// bastion (AUTON-0..): who ACTIVELY holds this job right now — set
    /// exactly when an `ActiveJob` component points at it, cleared
    /// exactly when that stops being true (completion, cancellation,
    /// travel-timeout release, suspension). This is the field every
    /// pre-AUTON-2-unification consumer means by "claimed": arbiter
    /// selection, the claim-loop, `JobAudit::claims_distinct` (which
    /// requires each uid claim AT MOST ONE job — a genuine board-
    /// conservation invariant, not a convention).
    pub claimed_by: Option<crate::uid::Uid>,
    /// bastion (AUTON-2 unification, site 4/6, row 50, 2026-08-09):
    /// OWNERSHIP-ACROSS-RELEASE — a colonist's SELF-job (RestAt/EatFrom/
    /// Despond) that got preempted (a higher-priority self-job, an
    /// unreachable target) but should be RECLAIMED verbatim once nothing
    /// outranks it, rather than destroyed-and-recreated (which for
    /// Despond specifically would lose `until`, the breakdown deadline,
    /// or force a fresh RNG roll). Distinct from `claimed_by` on
    /// purpose, never the same field: `claimed_by` means ACTIVELY HELD
    /// (governs selection/audit/conservation) — a job can be
    /// `claimed_by: None, suspended_for: Some(uid)` (suspended, still
    /// owned, invisible to `claims_distinct`) at the same moment another
    /// job is `claimed_by: Some(uid)` (the colonist's CURRENT active
    /// job) without violating "one active claim per colonist." Reading
    /// one field for the other's question is the reservation-vs-arrival
    /// class of bug on a determinism-critical struct — don't. The orphan
    /// sweep discriminates removal on OWNER LIVENESS via this field (a
    /// suspended job with a live, loaded owner survives; one whose owner
    /// died or was never loaded gets swept, same as any other orphan) —
    /// never on `claimed_by` alone, which is `None` for both cases.
    pub suspended_for: Option<crate::uid::Uid>,
    /// Set when a claimant repeatedly failed to reach the site; unreachable
    /// jobs are skipped by arbitration and logged.
    pub unreachable: bool,
    /// B5: work-in-progress toward completion, 0.0..=1.0.
    pub progress: f32,
    /// B5 Build jobs only: the material item asset id required to complete
    /// (a stand-in for a real blueprint's bill of materials — B6 owns real
    /// recipes/hauling). `None` for Mine/Chop (no material needed).
    pub required_item: Option<&'static str>,
    /// B5: true when no currently-loaded colonist carries `required_item` —
    /// i.e. the job is stalled pending B6 hauling. Informational only
    /// (arbitration eligibility is the real gate); recomputed each cycle.
    pub needs_materials: bool,
    /// bastion (B5.8): a carve-steps self-rescue was already attempted for
    /// this job — the watchdog degrades straight to `unreachable` next time
    /// instead of carving again (one attempt per job; no carve loops).
    pub carve_attempted: bool,
    /// bastion (B5.8): this job IS part of an auto-access plan (a stair
    /// step or ladder rung the colony carves for itself). Access jobs never
    /// spawn further access, and while ANY access job is pending no new
    /// plan is emitted — overlapping plans dig through each other's floors
    /// (the b58 run-7 gallery-of-chaos finding); one stair serves everyone.
    #[serde(default)]
    pub is_access: bool,
    /// bastion (B5.8-E, Ben's anti-loop invariant): how many times a
    /// claimant stuck-timed-out on this job. Grows the job's arrival
    /// tolerance (a bounded REMOTE-WORK reach extension, ~6 blocks at 3+
    /// strikes), so a colonist that can't physically stand at an awkward
    /// block eventually works it from below/afar instead of looping
    /// claim→stuck→unreachable→retry forever.
    #[serde(default)]
    pub stuck_strikes: u8,
    /// bastion (ROW B′, 2026-08-04, replaces the withdrawn Row B): the sim
    /// tick this job becomes eligible for the amnesty grant again, once
    /// `stuck_strikes` crosses `PERSIST_ESCALATE_STRIKES`. `None` = not
    /// currently benched (the overwhelming majority, including every job
    /// while `BASTION_ROWB_BENCH` is unset).
    ///
    /// WHY A TICK, NOT A GRANT COUNT: `amnesty_set_quiet` (JobBoard) is a
    /// quiet-STREAK, not a cumulative grants-issued count — it resets on
    /// `world_changed` and on its own dormant-cycle catch-all, so nothing
    /// in the amnesty system counts "which grant this is." `tick.0` does
    /// (the sim tick, destructured once at the top of the enclosing
    /// `System::run` and already in scope at both the site that sets this
    /// field and the site that reads it) -- a genuinely free lever, not
    /// an invented one.
    ///
    /// GRADUATION IS A CONJUNCTION, not a timer: the amnesty loop only
    /// resets `unreachable` when a grant fires (`world_changed`, a real
    /// terrain-change signal, or the dormant-cycle catch-all) AND this
    /// tick has passed. A benched job cannot be re-offered on the clock
    /// alone, and cannot be re-offered instantly just because a neighbour
    /// dug once. Do not "simplify" this into a plain deadline check
    /// outside the amnesty loop -- that would drop the terrain-change
    /// half of the condition the Haul-drop arm's own comment protects.
    ///
    /// `ROWB_BENCH_TICKS` (the increment added to `tick.0` here) is an
    /// UNVALIDATED DEFAULT -- same discipline as the withdrawn Row B's
    /// `BENCH_AMNESTY_GRANTS_OWED`, which the corpus judged and rejected
    /// on cost grounds, not on this number. No corpus evidence yet
    /// justifies any particular graduation delay; the paired A/B is what
    /// judges it.
    ///
    /// SAVE/LOAD: moot, verified not assumed. `JobBoard` is never
    /// persisted -- created fresh via `JobBoard::default()` each server
    /// start (see `JobBoard`'s own doc, `bastion_jobs.rs`, and this
    /// struct's `affordance` field's doc, both independently stating the
    /// same fact) -- so an absolute tick stored here never survives past
    /// the run that wrote it. `#[serde(default)]` kept for
    /// harness/test-fixture JSON round-tripping only, not save
    /// migration.
    #[serde(default)]
    pub benched_until_tick: Option<u64>,
    /// bastion (B5.8-E, Ben's ACCESS-BEFORE-DESCENT): this dig cell's depth
    /// below its own column's surface AT PLACEMENT (0 = the surface layer).
    /// The descent gate holds Mine claims deeper than novice reach until
    /// return-access exists nearby — access LEADS the dig down instead of
    /// trailing it, so an inescapable hole is never created in the first
    /// place (the reactive egress becomes the rare backstop).
    #[serde(default)]
    pub depth: u8,
    /// bastion (B6 JOB-CORE): the item reservation this job holds (a Haul's
    /// cargo, a Build's material) — released on completion/cancel/release.
    /// serde-default: pre-B6 saves have none.
    #[serde(default)]
    pub reservation: Option<ReservationId>,
    /// bastion (task #64): what `pos` physically means — see
    /// [`AffordanceClass`]. Deliberately NO `#[serde(default)]`: `Job`
    /// never crosses the wire and `JobBoard` is never persisted (created
    /// fresh via `JobBoard::default()` each server start, confirmed
    /// premise-checked — see the task #64 packet), so there is no old-save
    /// migration story to protect and no reason to let a construction site
    /// skip choosing.
    pub affordance: AffordanceClass,
}

impl Job {
    /// bastion (item 16): is this job a candidate for the CLAIM SELECTOR —
    /// the loop that ranks unheld work and, crucially, the ONLY place
    /// `WorkPriorities` is consulted?
    ///
    /// This exists as one named predicate because the answer decides whether
    /// a player-set work priority can reach a job at all, and that turned out
    /// to be load-bearing in a way nothing named it:
    ///
    /// **`insert_eat_job` files eating under [`WorkType::Haul`].** Read
    /// naively, `bastion_priority haul 0` should therefore stop the colony
    /// EATING and starve it. It does not — but only because self-jobs
    /// (`EatFrom`/`RestAt`/`Despond`) are inserted PRE-CLAIMED, so they fail
    /// this predicate and never reach the priority gate downstream of it.
    ///
    /// So the safety is a consequence of claim-ordering, not of intent. If a
    /// future change ever lets a self-job enter the selector unclaimed — or
    /// adds a second priority check keyed on `job.work` somewhere else — then
    /// disabling Haul silently becomes a starvation command. Route any new
    /// priority gate through THIS predicate so the coupling stays visible,
    /// and see `an_eat_job_is_invisible_to_the_work_priority_gate`.
    pub fn is_claim_candidate(&self) -> bool {
        self.claimed_by.is_none() && !self.unreachable
    }
}

/// bastion (TOOL-0, TOOLS-UPGRADE §3): the work-tick's TOOL factor — a
/// multiplier on the server's `work_rate`. The verb↔tool map rides the
/// shipped `ToolKind`s (Mine→Pick, Chop→Axe, Build→Hammer; Haul/Cook have
/// no tool gate yet). NO or WRONG tool = 1.0 — the deliberately slow base
/// (the "slow mining" home: upgrades must mean something); a MATCHING tool
/// speeds work up, scaled by the LOCKED `item::Quality` (DF-QUALITY —
/// reuse, never fork). Deterministic and pure — the curve is unit-pinned
/// below. TOOL-1 adds the material-tier ladder + min-tier gating on hard
/// blocks; TOOL-2 adds auto-equip-best + craft-quality stamps.
pub fn tool_factor(
    work: WorkType,
    tool: Option<(
        crate::comp::item::tool::ToolKind,
        crate::comp::item::Quality,
    )>,
) -> f32 {
    use crate::comp::item::{Quality, tool::ToolKind};
    let wanted = match work {
        // ITEM 14 v1: no tool requirement. A weapon axis is a separate
        // parameter and inventing one here would be a policy, not a value.
        WorkType::Guard => None,
        WorkType::Mine => Some(ToolKind::Pick),
        WorkType::Chop => Some(ToolKind::Axe),
        WorkType::Build => Some(ToolKind::Hammer),
        // FARM (row 46): bare-hand base rate v1 — vanilla ships no
        // dedicated farm tool tier; a Hoe tier is TOOLS-UPGRADE data
        // when it lands.
        WorkType::Haul | WorkType::Cook | WorkType::Farm => None,
    };
    match (wanted, tool) {
        (Some(w), Some((k, q))) if k == w => {
            // Quality ladder: a crude matching tool is a real relief over
            // bare hands (1.5×); the artifact apex is 3.5×. Bounded so
            // skill (+20%/level) stays a co-equal axis — both multiply.
            1.5 + match q {
                Quality::Low => 0.0,
                Quality::Common => 0.25,
                Quality::Moderate => 0.5,
                Quality::High => 1.0,
                Quality::Epic => 1.5,
                Quality::Legendary | Quality::Artifact | Quality::Debug => 2.0,
            }
        },
        // Verb has no tool gate, or bare hands / wrong tool: the slow base.
        _ => 1.0,
    }
}

/// The material B5's minimal Build path requires (single hardcoded material;
/// B6 gives Build real per-blueprint recipes). Deliberately the same item
/// Mine drops, so mine → build closes into a loop even before B6 hauling.
pub const BUILD_MATERIAL_ITEM: &str = "common.items.crafting_ing.stones";
/// What a completed Mine job drops (B5 v1: flat stones for any mined block;
/// a per-block-type loot mapping is future work).
pub const MINE_DROP_ITEM: &str = "common.items.crafting_ing.stones";
/// What a completed Chop job drops.
pub const CHOP_DROP_ITEM: &str = "common.items.log.wood";

/// Aggregate job-board audit for tests/inspectors (B4 harness gate).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JobAudit {
    pub total: usize,
    pub claimed: usize,
    pub unreachable: usize,
    /// True iff no two claimed jobs share a claimant and no claimant appears
    /// twice (each colonist works at most one job).
    pub claims_distinct: bool,
    /// T1.16 (conservation cluster): the number of item entities reserved by
    /// MORE THAN ONE job (the T1.13 reservation-ledger bidirectional-
    /// uniqueness audit). Non-zero = a double-spend — two jobs believe they
    /// own the same physical item.
    pub reservation_conflicts: usize,
}

impl JobAudit {
    /// T1.16: the single board-conservation verdict — claims are distinct
    /// (one colonist per job) AND no item is double-reserved. A colony that
    /// fails this has leaked a claim ticket or a reservation.
    pub fn conserved(&self) -> bool {
        self.claims_distinct && self.reservation_conflicts == 0
    }
}

// ─── B3: colonists ──────────────────────────────────────────────────────────

/// A work skill's progression. Levels rise as B5 grants completion XP.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillLevel {
    pub level: u16,
    pub xp: f32,
}

impl SkillLevel {
    /// Flat per-level XP curve — plenty for B5's "does XP feed back into
    /// rate" loop; a real curve is a B-AG/balance concern, not this block's.
    const XP_PER_LEVEL: f32 = 20.0;

    pub fn add_xp(&mut self, xp: f32) {
        self.xp += xp;
        while self.xp >= Self::XP_PER_LEVEL {
            self.xp -= Self::XP_PER_LEVEL;
            self.level += 1;
        }
    }
}

/// The colonist work skills (B4 arbitration reads these; B5 trains them).
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ColonistSkills {
    pub mining: SkillLevel,
    pub woodcutting: SkillLevel,
    pub construction: SkillLevel,
    pub hauling: SkillLevel,
    pub cooking: SkillLevel,
    pub melee: SkillLevel,
    /// bastion (B5.8, Ben's directive: climbing is a SKILL): a MOVEMENT
    /// skill, deliberately not a [`WorkType`] — it gates scramble reach
    /// (novice: 2-block faces; level 1+: 3-block) and accrues XP while
    /// actually climbing (the bastion job system's climb-state accrual).
    /// The same movement-skill shape extends to flying entities later —
    /// don't fold it into the work-skill enum. `serde(default)` so rtsim
    /// colonist records saved before B5.8 still load.
    #[serde(default)]
    pub climbing: SkillLevel,
    /// bastion (FARM/PROD-2): field work. `serde(default)` — records
    /// saved before row 46 load with an untrained farmer.
    #[serde(default)]
    pub farming: SkillLevel,
}

impl ColonistSkills {
    /// Route completion XP to the skill matching the work type (B5).
    pub fn grant_xp(&mut self, work: WorkType, xp: f32) {
        match work {
            // ITEM 14: guarding trains MELEE — the skill already exists, so
            // v1 adds no skill field it would have to justify.
            WorkType::Guard => self.melee.add_xp(xp),
            WorkType::Mine => self.mining.add_xp(xp),
            WorkType::Chop => self.woodcutting.add_xp(xp),
            WorkType::Build => self.construction.add_xp(xp),
            WorkType::Haul => self.hauling.add_xp(xp),
            WorkType::Cook => self.cooking.add_xp(xp),
            WorkType::Farm => self.farming.add_xp(xp),
        }
    }

    /// The level of the skill tracking a work type — what arbitration gates
    /// `skill_floor` on and B5's work rate scales by.
    pub fn level_for(&self, work: WorkType) -> u16 {
        match work {
            WorkType::Guard => self.melee.level,
            WorkType::Mine => self.mining.level,
            WorkType::Chop => self.woodcutting.level,
            WorkType::Build => self.construction.level,
            WorkType::Haul => self.hauling.level,
            WorkType::Cook => self.cooking.level,
            WorkType::Farm => self.farming.level,
        }
    }

    /// Directly set the level of the skill tracking a work type (harness /
    /// scenario tooling; gameplay progression goes through `grant_xp`).
    pub fn set_level_for(&mut self, work: WorkType, level: u16) {
        let s = match work {
            WorkType::Guard => &mut self.melee,
            WorkType::Mine => &mut self.mining,
            WorkType::Chop => &mut self.woodcutting,
            WorkType::Build => &mut self.construction,
            WorkType::Haul => &mut self.hauling,
            WorkType::Cook => &mut self.cooking,
            WorkType::Farm => &mut self.farming,
        };
        s.level = level;
    }
}

/// RimWorld-style per-work-type priority: 0 = never, 1..=4 with 4 highest.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkPriorities {
    pub mine: u8,
    pub chop: u8,
    pub build: u8,
    pub haul: u8,
    pub cook: u8,
    /// bastion (FARM/PROD-2): serde-defaulted to the standard 3 so
    /// pre-row-46 saves farm at normal priority, not never.
    #[serde(default = "default_work_priority")]
    pub farm: u8,
    /// bastion (ITEM 14): serde-defaulted to 3 so pre-item-14 saves guard at
    /// normal priority rather than NEVER — a 0 default would make every
    /// existing colony silently refuse guard work and read as "guards do not
    /// work", which is the kind of default that gets diagnosed as a bug.
    #[serde(default = "default_work_priority")]
    pub guard: u8,
}

fn default_work_priority() -> u8 { 3 }

impl Default for WorkPriorities {
    fn default() -> Self {
        Self {
            mine: 3,
            chop: 3,
            build: 3,
            haul: 3,
            cook: 3,
            farm: 3,
            // ITEM 14: normal priority, not 0 — see the field doc.
            guard: 3,
        }
    }
}

impl WorkPriorities {
    /// Priority for a work type: 0 = never do this work, 1..=4 rising.
    pub fn get(&self, work: WorkType) -> u8 {
        match work {
            WorkType::Guard => self.guard,
            WorkType::Mine => self.mine,
            WorkType::Chop => self.chop,
            WorkType::Build => self.build,
            WorkType::Haul => self.haul,
            WorkType::Cook => self.cook,
            WorkType::Farm => self.farm,
        }
    }

    pub fn set(&mut self, work: WorkType, priority: u8) {
        let p = priority.min(4);
        match work {
            WorkType::Guard => self.guard = p,
            WorkType::Mine => self.mine = p,
            WorkType::Chop => self.chop = p,
            WorkType::Build => self.build = p,
            WorkType::Haul => self.haul = p,
            WorkType::Cook => self.cook = p,
            WorkType::Farm => self.farm = p,
        }
    }
}

/// The per-colonist record. Lives in the rtsim `Npc` (persisted, works
/// headlessly) and is mirrored into the ECS `comp::Colonist` when the NPC is
/// promoted to a loaded entity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionColonist {
    pub name: String,
    pub backstory: String,
    pub skills: ColonistSkills,
    pub work_priorities: WorkPriorities,
    /// bastion (ITEM 14, AXIS 2 — HOLD-vs-FLEE). Ben's ruling: guarding
    /// outranks flee **up to a breaking point that varies by the
    /// individual**. This is that point, PER COLONIST.
    ///
    /// A guarding colonist holds while `health.fraction() >= guard_bravery`.
    /// **Lower = braver.** Same direction as `Psyche::flee_health`, which it
    /// competes with, so the two can be compared without a sign flip — a
    /// reversed sense here would make the "brave" pin flee FIRST while every
    /// number still looked plausible.
    ///
    /// Lives on the PERSISTED colonist record (not the ECS mirror) so it
    /// survives promote/demote — a bravery that reset when a colonist
    /// unloaded would make axis 2 unmeasurable across a long run.
    /// serde-defaulted so pre-item-14 saves parse.
    #[serde(default = "default_guard_bravery")]
    pub guard_bravery: f32,
    /// bastion (B6 SOFT-0): transient SOFT-COLLISION state — while sim
    /// `Time` < this, the phys colonist↔colonist push is SOFTENED so this
    /// colonist can squeeze past another in a chokepoint (terrain stays
    /// hard; see SOFT-COLLISION-design §0). 0.0 = off. Set by the server
    /// triggers (watchdog grace window / local density); expiry IS the
    /// hysteresis. serde-default: absent in old rtsim saves → off.
    #[serde(default)]
    pub soft_until: f64,
    /// REQ-0052: sub-second terrain-collider squeeze used only for an
    /// already-validated adjacent emergency-route mount. Physics reduces the
    /// horizontal capsule radius while preserving full height and terrain
    /// collision; expiry restores the normal body automatically.
    #[serde(default)]
    pub route_squeeze_until: f64,
    /// bastion (B-LIVE3, Ben's UNIVERSAL CLIMB-OUT fail-safe): while sim
    /// `Time` < this, the climb assist lifts this colonist up ANY wall
    /// contact — no ladder, no reach cap ("climb out of anywhere, as a
    /// FINAL fail-safe, not a preference"). Granted only by the trapped
    /// verdict (egress/churn no-egress) and on mine-done dispersal; the
    /// teleport-to-ground ultimate backstop fires if this too fails.
    /// 0.0 = off; serde-default for old saves.
    #[serde(default)]
    pub climb_free_until: f64,
    /// bastion (LOD-0, the save-back): the colonist's BAG-SLOT inventory as
    /// `(itemdef id, amount)` pairs — the persistent truth mirrored from the
    /// live ECS `Inventory` every loaded tick and restored on promote, so
    /// carried items survive unload/re-promote and save/reload with no loss
    /// and no dupe (registry B11's inventory half). Sorted by id with
    /// duplicate stacks merged (a canonical form, so equality means
    /// equality). `None` = never captured (a FIRST promote keeps the spawn
    /// loadout); `Some` = the truth, INCLUDING a legitimately empty bag —
    /// promote then REPLACES the fresh spawn-default bag wholesale, which is
    /// what kills the dupe (re-created entities roll a NEW random villager
    /// loadout; restoring on top of it doubled food/coins in the first
    /// scenario run). serde-default: absent in old saves → `None`.
    #[serde(default)]
    pub inventory: Option<Vec<(String, u32)>>,
    /// bastion (FOCUS-0): per-colonist PERSONAL-NEED state keyed by the
    /// locked [`Need`] vocabulary — a serde-defaulted COLLECTION, not one
    /// struct field per need (the Playbook's explicit shape: future
    /// `Need` variants join without a struct migration; old saves default
    /// EMPTY). Same 1.0-satisfied semantics as the bodily `Needs` comp.
    /// Empty = no tracked personal-need state yet — FOCUS-1 populates it;
    /// the B-AG3-deferred facet-derivation later sets per-colonist
    /// weights. Schema only this block: nothing reads or writes it.
    #[serde(default)]
    pub personal_needs: std::collections::BTreeMap<Need, f32>,
    /// bastion (B-AG3 slice 1): per-colonist VALUE weights on the locked
    /// [`Value`] vocabulary — ±50 (the agency bible's scale: positive =
    /// holds the value, negative = scorns it). Same serde-defaulted
    /// COLLECTION shape as `personal_needs` (future variants join without
    /// a migration; old saves + fresh colonists default EMPTY = a care
    /// factor of exactly 1.0 = the pre-B-AG3 mood behavior bit-for-bit).
    /// Value ROLLING (culture keying / facet derivation) is a later
    /// slice — this block reads the map in the thought-care weighting
    /// only; the test hook is the sole writer. `BTreeMap` makes persisted
    /// iteration follow [`Value`]'s append-only, save-stable enum order.
    #[serde(default)]
    pub values: std::collections::BTreeMap<Value, i8>,
    /// bastion (B7-0): the persistent mirror of the bodily `Needs` comp
    /// `(hunger, rest, recreation)` — captured from the live ECS every
    /// loaded tick, restored WHOLESALE on promote (the LOD-0 inventory
    /// semantics: `None` = never captured, a first promote keeps the
    /// fresh defaults; `Some` replaces them). serde-default: old saves →
    /// `None`.
    #[serde(default)]
    pub needs: Option<(f32, f32, f32)>,
    /// bastion (B7-0): the persistent mirror of `Mood` — same capture/
    /// restore semantics as `needs`.
    #[serde(default)]
    pub mood: Option<f32>,
    /// bastion (RUN-0, row 47): the emergency-run gait flag — walk
    /// (TRAVEL_SPEED) is every colonist's default; true feeds RUN_SPEED
    /// into the SAME Goto call sites (no parallel movement path). Set by
    /// urgency triggers (RUN-1's job; a test hook this block), CLEARED
    /// by the energy governor when Energy crosses the low threshold —
    /// resource-governed, not timer-governed (the design's framing).
    /// serde-default: old saves walk.
    #[serde(default)]
    pub running: bool,
    /// bastion (B7-1): the bed this colonist OWNS (its [`BedSlot`] key) —
    /// the persistent ownership truth (the board's slot table is
    /// session-state; its `owner` field is the runtime lookup). Assigned
    /// by the assignment hook this block, by B7-2's auto-assignment
    /// later. serde-default: old saves → `None`.
    #[serde(default)]
    pub owned_bed: Option<Vec3<i32>>,
}

/// bastion (CASE-003 belt): count of per-tick CENTER-SAFETY-NET fires — a
/// colonist's torso-center was found inside solid terrain after physics
/// integration and was relocated to the nearest standable cell. The net is
/// the "entombment impossible by construction" guarantee; the counter is
/// REPORTED telemetry (harness + ops visibility): with the writer-side bugs
/// fixed it should sit at 0, and any climb marks a NEW embedding writer to
/// hunt (never gate on it — the net firing is the invariant HOLDING).
pub static CENTER_NET_FIRES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// CAVE-IN v1 (FR11, reviewer R8/F-CAVE-1) + CASE-003: the nearest TRUE
/// STANDABLE cell for relocating a colonist — air at the feet AND head with a
/// solid floor, preferring a LATERAL step-out at the victim's own level and
/// rising/dropping only as needed. Deliberately NOT a surface search: a
/// surface-window scan deep underground is ALL ROCK and returns the window
/// top — teleporting a deep-mine victim INTO solid stone (the exact
/// entombment the invariant forbids; R8 F-CAVE-1). Returns `None` when no
/// safe cell exists in range — the caller leaves the victim IN PLACE.
///
/// ONE implementation for every relocation caller (cave-in eject, the phys
/// center-safety-net) — identity by construction (B17). The closure-based
/// core is the testable engine ([`floating_chunk`]'s pattern); the
/// [`TerrainGrid`](crate::terrain::TerrainGrid) wrapper is the shipping path.
pub fn eject_dest_impl(
    solid: impl Fn(Vec3<i32>) -> bool,
    feet: Vec3<i32>,
    crush_xy: &hashbrown::HashSet<Vec2<i32>>,
) -> Option<Vec3<i32>> {
    let standable =
        |c: Vec3<i32>| !solid(c) && !solid(c + Vec3::unit_z()) && solid(c - Vec3::unit_z());
    // Nearest ring out; within a ring, smallest |dz| first (lateral step-out
    // beats climbing), searching a modest vertical band around the victim.
    for r in 1..=8i32 {
        for dz_abs in 0..=4i32 {
            let dzs: &[i32] = if dz_abs == 0 {
                &[0]
            } else {
                &[dz_abs, -dz_abs]
            };
            for &dz in dzs {
                for dx in -r..=r {
                    for dy in -r..=r {
                        if dx.abs().max(dy.abs()) != r {
                            continue;
                        }
                        let (x, y) = (feet.x + dx, feet.y + dy);
                        if crush_xy.contains(&Vec2::new(x, y)) {
                            continue; // never eject INTO the falling footprint
                        }
                        let c = Vec3::new(x, y, feet.z + dz);
                        if standable(c) {
                            return Some(c);
                        }
                    }
                }
            }
        }
    }
    None
}

/// [`eject_dest_impl`] over live terrain (the shipping path). An errored
/// terrain read (unloaded chunk) counts as NOT solid for the feet/head test
/// and NOT solid for the floor test — i.e. an unloaded cell can never be
/// accepted as standable (the floor check fails), so the search never
/// relocates anyone into unknown space.
pub fn eject_dest(
    terrain: &crate::terrain::TerrainGrid,
    feet: Vec3<i32>,
    crush_xy: &hashbrown::HashSet<Vec2<i32>>,
) -> Option<Vec3<i32>> {
    use crate::vol::ReadVol;
    eject_dest_impl(
        |p| terrain.get(p).map(|b| b.is_filled()).unwrap_or(false),
        feet,
        crush_xy,
    )
}

/// [`eject_dest`] with no crush exclusion — the CENTER-SAFETY-NET's form
/// (callers outside this crate may not depend on hashbrown directly).
pub fn eject_dest_free(
    terrain: &crate::terrain::TerrainGrid,
    feet: Vec3<i32>,
) -> Option<Vec3<i32>> {
    eject_dest(terrain, feet, &hashbrown::HashSet::new())
}

const COLONIST_FIRST_NAMES: &[&str] = &[
    "Awen", "Bram", "Cerys", "Doran", "Eira", "Fenn", "Gwil", "Hesta", "Ivo", "Jena", "Kell",
    "Lira", "Maddoc", "Nia", "Osric", "Peri", "Quill", "Rhosyn", "Sten", "Tegan", "Ulric", "Vada",
    "Wynn", "Yara",
];

const COLONIST_EPITHETS: &[&str] = &[
    "the Steady",
    "of the Vale",
    "Ironhand",
    "the Quiet",
    "Longstride",
    "the Younger",
    "Ashborn",
    "the Stout",
    "Brighteye",
    "of the Ford",
    "the Wary",
    "Oakenshield",
];

const COLONIST_BACKSTORIES: &[&str] = &[
    "farmhand",
    "quarry worker",
    "wandering tinker",
    "disgraced guard",
    "orchard keeper",
    "charcoal burner",
    "riverboat hand",
    "apprentice mason",
    "trapper",
    "camp cook",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// FOCUS-0 pin: the Need-keyed collection round-trips serde, and —
    /// the lock's whole point — a payload WITHOUT the field decodes with
    /// an EMPTY default (old saves load fine; the "not fixed fields"
    /// shape means future variants join without migration). Encoder:
    /// `ron` (common's in-crate encoder); `#[serde(default)]` semantics
    /// are encoder-independent, and the persistence-encoder-faithful
    /// (rmp) round-trip already rides the rtsim-level tests + the lod0
    /// gate leg's in-vivo whole-record round-trip.
    #[test]
    fn bastion_need_collection_serde_shape() {
        use std::collections::{BTreeMap, HashMap};
        #[derive(serde::Deserialize)]
        struct New {
            #[expect(dead_code, reason = "decode-shape witness")]
            name: String,
            #[serde(default)]
            personal_needs: BTreeMap<Need, f32>,
        }
        // An old-shape payload (no field) -> empty default.
        let decoded: New = ron::from_str(r#"(name: "Trell")"#).expect("decode old shape");
        assert!(decoded.personal_needs.is_empty());
        // A populated map round-trips exactly.
        let mut needs = HashMap::new();
        needs.insert(Need::Pray, 0.3f32);
        needs.insert(Need::Socialize, 1.0);
        needs.insert(Need::Fight, 0.75);
        let text = ron::to_string(&needs).expect("encode");
        let back: BTreeMap<Need, f32> = ron::from_str(&text).expect("decode");
        assert_eq!(needs.into_iter().collect::<BTreeMap<_, _>>(), back);
    }

    /// #62 (2026-08-09, Fable/Opus-directed): `MoodConfig::default()` is a
    /// COPY of the shipped RON's values, not a shared source of truth --
    /// the identity-or-loud law's "fresh copy" risk named explicitly. A
    /// retune that edits `assets/common/bastion_mood.ron` without also
    /// touching this file's `impl Default` leaves an asset-load failure
    /// silently reverting to the STALE compiled rates. This test converts
    /// that risk from vigilance into a red suite: it loads the real
    /// shipped asset (failing loudly, not gracefully, if the load itself
    /// fails -- `MoodConfig::current()`'s `unwrap_or_default()` would make
    /// this test vacuous, comparing Default() to itself on any load
    /// failure) and asserts full structural equality against Default().
    #[test]
    fn bastion_mood_config_matches_shipped_asset() {
        use crate::assets::AssetExt;
        let shipped = MoodConfig::load("common.bastion_mood")
            .expect("load assets/common/bastion_mood.ron")
            .read()
            .clone();
        assert_eq!(shipped, MoodConfig::default());
    }

    #[test]
    fn bastion_personal_needs_persistence_bytes_are_stable() {
        use std::collections::{BTreeMap, BTreeSet};

        let entries = [
            (Need::Pray, 0.1f32),
            (Need::Socialize, 0.2),
            (Need::Drink, 0.3),
            (Need::AdmireArt, 0.4),
            (Need::Craft, 0.5),
            (Need::Family, 0.6),
            (Need::Fight, 0.7),
            (Need::Learn, 0.8),
        ];
        let mut encodings = BTreeSet::new();
        for shift in 0..entries.len() {
            let mut needs = BTreeMap::new();
            for offset in 0..entries.len() {
                let (need, value) = entries[(shift + offset) % entries.len()];
                needs.insert(need, value);
            }
            encodings.insert(ron::to_string(&needs).expect("encode personal needs"));
        }
        println!(
            "personal_needs distinct persistence encodings={}",
            encodings.len()
        );
        if let Some(first) = encodings.first() {
            println!("personal_needs representative_ron={first}");
        }
        assert_eq!(
            encodings.len(),
            1,
            "equal personal-needs state must have one persisted representation"
        );
    }

    /// bastion (B-AG3 slice 1): the `values` collection carries the same
    /// wire guarantees as `personal_needs` — absent field decodes to an
    /// empty default (old saves), a populated ±50 map round-trips exactly.
    #[test]
    fn bastion_value_collection_serde_shape() {
        use std::collections::BTreeMap;
        #[derive(serde::Deserialize)]
        struct New {
            #[expect(dead_code, reason = "decode-shape witness")]
            name: String,
            #[serde(default)]
            values: BTreeMap<Value, i8>,
        }
        // An old-shape payload (no field) -> empty default.
        let decoded: New = ron::from_str(r#"(name: "Trell")"#).expect("decode old shape");
        assert!(decoded.values.is_empty());
        // A populated map round-trips exactly, negatives included
        // (scorned values are first-class).
        let mut values = BTreeMap::new();
        values.insert(Value::Glory, 50i8);
        values.insert(Value::Kin, -35);
        values.insert(Value::Piety, 10);
        let text = ron::to_string(&values).expect("encode");
        let back: BTreeMap<Value, i8> = ron::from_str(&text).expect("decode");
        assert_eq!(values, back);

        // Persistence must not depend on insertion order. `Value` is an
        // append-only enum, so its derived order is the save-stable order.
        let reverse = [(Value::Piety, 10), (Value::Kin, -35), (Value::Glory, 50)]
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        assert_eq!(ron::to_string(&reverse).expect("encode reverse"), text);
    }

    fn r(min: (i32, i32, i32), max: (i32, i32, i32)) -> Region {
        Region {
            min: Vec3::new(min.0, min.1, min.2),
            max: Vec3::new(max.0, max.1, max.2),
        }
    }

    #[test]
    fn subtract_disjoint_returns_self() {
        let a = r((0, 0, 0), (3, 3, 3));
        let b = r((10, 10, 10), (12, 12, 12));
        assert_eq!(a.subtract(&b), vec![a]);
    }

    #[test]
    fn erase_by_xy_removes_regardless_of_z_misalignment() {
        // The erase bug: a designation painted at ground z=[397,399]; the
        // erase drag's z came from a DIFFERENT camera height, z=[403,405].
        // A naive subtract(erase) misses in z → overlay + jobs persist.
        let desig = r((10, 10, 397), (15, 15, 399));
        let erase_drag = r((8, 8, 403), (20, 20, 405)); // XY-covers, z-misaligned
        // Naive (the bug): no z overlap → nothing removed.
        assert_eq!(
            desig.subtract(&erase_drag),
            vec![desig],
            "reproduces the bug"
        );
        // The fix: clip the erase to the designation's XY at the DESIGNATION's
        // z, then subtract → fully removed.
        let clipped = desig
            .clip_xy(erase_drag.min.xy(), erase_drag.max.xy())
            .expect("xy overlaps");
        assert!(
            desig.subtract(&clipped).is_empty(),
            "full XY cover erases cleanly"
        );
    }

    #[test]
    fn erase_partial_xy_leaves_remainder_at_correct_z() {
        // Erase only the +x half; z-misaligned drag. The remainder must stay,
        // at the designation's own z.
        let desig = r((10, 10, 397), (19, 15, 399));
        let erase_drag = r((15, 8, 500), (30, 20, 502));
        let clipped = desig
            .clip_xy(erase_drag.min.xy(), erase_drag.max.xy())
            .expect("xy overlaps");
        // Clipped keeps the designation's z, not the drag's.
        assert_eq!(clipped.min.z, 397);
        assert_eq!(clipped.max.z, 399);
        let remainder = desig.subtract(&clipped);
        let remainder_vol: i64 = remainder.iter().map(|r| r.volume()).sum();
        assert_eq!(remainder_vol, desig.volume() - clipped.volume());
        assert!(remainder.iter().all(|p| p.max.x < 15)); // only the un-erased -x half
    }

    #[test]
    fn clip_xy_no_overlap_is_none() {
        let a = r((0, 0, 0), (3, 3, 3));
        assert!(a.clip_xy(Vec2::new(10, 10), Vec2::new(12, 12)).is_none());
    }

    #[test]
    fn subtract_full_cover_returns_empty() {
        let a = r((1, 1, 1), (3, 3, 3));
        let b = r((0, 0, 0), (5, 5, 5));
        assert!(a.subtract(&b).is_empty());
    }

    #[test]
    fn subtract_conserves_volume_and_is_disjoint() {
        // A center hole and several offset overlaps, incl. edge/corner cuts.
        let a = r((0, 0, 0), (9, 9, 9));
        for b in [
            r((3, 3, 3), (6, 6, 6)),    // interior hole → 6 pieces
            r((0, 0, 0), (4, 9, 9)),    // face slab
            r((5, 5, 5), (20, 20, 20)), // corner cut
            r((0, 4, 0), (9, 5, 9)),    // through-slab
            r((-5, -5, -5), (0, 0, 0)), // corner nick
        ] {
            let pieces = a.subtract(&b);
            let inter_vol = a.intersection(&b).map_or(0, |i| i.volume());
            let piece_vol: i64 = pieces.iter().map(|p| p.volume()).sum();
            assert_eq!(
                a.volume(),
                inter_vol + piece_vol,
                "volume not conserved vs {b:?}"
            );
            // Pieces must be pairwise disjoint and inside `a`, outside `b`.
            for (i, p) in pieces.iter().enumerate() {
                assert!(p.volume() > 0);
                assert!(a.intersection(p) == Some(*p), "piece escapes a");
                assert!(!p.intersects(&b), "piece overlaps the subtrahend");
                for q in &pieces[i + 1..] {
                    assert!(!p.intersects(q), "pieces overlap each other");
                }
            }
        }
    }

    #[test]
    fn z_extent_default_preserves_legacy_paint_depth() {
        // The old client pre-expansion was `plane-2 ..= plane` (3 levels).
        // Every kind's default must reproduce it exactly, or existing
        // paint behavior changes under users' feet.
        for kind in [
            DesignationKind::Mine,
            DesignationKind::Chop,
            DesignationKind::Build,
            DesignationKind::Stockpile,
        ] {
            let e = ZExtent::default_for(kind);
            assert_eq!((e.down, e.up), (2, 0), "{kind:?} default drifted");
            assert_eq!(e.levels(), 3);
        }
        // B5.8: Ladder is the one upward kind (a rung column, not a dig).
        let l = ZExtent::default_for(DesignationKind::Ladder);
        assert_eq!((l.down, l.up), (0, 3));
        // B5.6b-2.1: flat-floor is opt-in per paint — never a default.
        assert!(l.floor_z.is_none());
        assert!(ZExtent::default().floor_z.is_none());
    }

    #[test]
    fn column_range_relative_and_flat() {
        // Relative: surface-follow (the b-2 model, unchanged).
        let rel = ZExtent {
            down: 2,
            up: 0,
            floor_z: None,
        };
        assert_eq!(rel.column_range(100), Some((98, 100)));
        assert_eq!(rel.column_range(105), Some((103, 105)));
        // Flat: every column bottoms at the SAME absolute z.
        let flat = ZExtent {
            down: 2,
            up: 0,
            floor_z: Some(98),
        };
        assert_eq!(flat.column_range(100), Some((98, 100)));
        assert_eq!(flat.column_range(105), Some((98, 105))); // deeper cut uphill
        // A column already at/below the floor digs nothing.
        assert_eq!(flat.column_range(97), None);
        // Floor at exactly the surface: the surface block itself goes.
        assert_eq!(flat.column_range(98), Some((98, 98)));
    }

    #[test]
    fn tool_factor_curve() {
        use crate::comp::item::{Quality, tool::ToolKind};
        // TOOL-0 CURVE PIN: bare hands / wrong tool = the slow base (1.0);
        // a MATCHING tool is a real relief (≥1.5×); quality is monotonic;
        // ungated verbs ignore tools. If this needs editing, the tuning
        // was deliberate (TOOLS-UPGRADE §2) — update the doc first.
        assert_eq!(tool_factor(WorkType::Mine, None), 1.0);
        assert_eq!(
            tool_factor(WorkType::Mine, Some((ToolKind::Axe, Quality::High))),
            1.0
        );
        assert_eq!(
            tool_factor(WorkType::Mine, Some((ToolKind::Pick, Quality::Low))),
            1.5
        );
        assert_eq!(
            tool_factor(WorkType::Chop, Some((ToolKind::Axe, Quality::Low))),
            1.5
        );
        assert_eq!(
            tool_factor(WorkType::Build, Some((ToolKind::Hammer, Quality::Low))),
            1.5
        );
        // Quality strictly climbs Low → Artifact for the matching tool.
        let ladder = [
            Quality::Low,
            Quality::Common,
            Quality::Moderate,
            Quality::High,
            Quality::Epic,
            Quality::Artifact,
        ];
        let factors: Vec<f32> = ladder
            .iter()
            .map(|q| tool_factor(WorkType::Mine, Some((ToolKind::Pick, *q))))
            .collect();
        assert!(factors.windows(2).all(|w| w[0] < w[1] || w[0] == w[1]));
        assert!(factors.windows(2).any(|w| w[0] < w[1]));
        assert_eq!(*factors.last().unwrap(), 3.5); // the apex
        // Haul/Cook: no tool gate yet — always the base.
        assert_eq!(
            tool_factor(WorkType::Haul, Some((ToolKind::Pick, Quality::Epic))),
            1.0
        );
    }

    #[test]
    fn purpose_enum_is_the_canonical_eight() {
        // SCHEMA GUARD (B5.6b-2): frameworks §2's zone↔asset purpose list is
        // canonical — 8 kinds, these labels. Other docs carry drifted 7/8/9-
        // kind copies; if this test needs editing, frameworks §2 must have
        // been deliberately changed FIRST (architect pass), not the reverse.
        let all = [
            Purpose::Housing,
            Purpose::Production,
            Purpose::Commerce,
            Purpose::Faith,
            Purpose::Social,
            Purpose::Defense,
            Purpose::Storage,
            Purpose::Farming,
        ];
        let labels: Vec<_> = all.iter().map(|p| p.label()).collect();
        assert_eq!(labels, vec![
            "Housing",
            "Production",
            "Commerce",
            "Faith",
            "Social",
            "Defense",
            "Storage",
            "Farming",
        ]);
        // Designation → purpose mapping: extraction/storage designations
        // classify; Build carries its asset's own purpose (None here).
        assert_eq!(DesignationKind::Mine.purpose(), Some(Purpose::Production));
        assert_eq!(DesignationKind::Chop.purpose(), Some(Purpose::Production));
        assert_eq!(DesignationKind::Stockpile.purpose(), Some(Purpose::Storage));
        assert_eq!(DesignationKind::Build.purpose(), None);
        assert_eq!(DesignationKind::Ladder.purpose(), None);
    }

    #[test]
    fn carve_ramp_shape_and_reachability_order() {
        // Fully solid mass (a pit wall). Rise 5, straight +x approach.
        let solid = |_: Vec3<i32>| true;
        let open = |_: Vec3<i32>| true;
        let digs = carve_ramp(Vec3::new(0, 0, 0), Vec3::new(5, 0, 5), &solid, &open)
            .expect("straight stair through solid must route");
        assert_eq!(digs.len(), (5 * CARVE_STEP_CLEARANCE) as usize);
        for k in 0..5i32 {
            let base = digs[(k * CARVE_STEP_CLEARANCE) as usize];
            // Step k+1: one block over, one block up (feet at from.z+k+1).
            assert_eq!(base, Vec3::new(k + 1, 0, k + 1));
            for dz in 1..CARVE_STEP_CLEARANCE {
                assert_eq!(
                    digs[(k * CARVE_STEP_CLEARANCE + dz) as usize],
                    base + Vec3::unit_z() * dz
                );
            }
        }
        // The reachability law: emission is bottom-up — a later column's
        // feet are strictly above every earlier column's feet (the climbing
        // digger never digs beneath its own established steps).
        let mut prev_feet = i32::MIN;
        for step in digs.chunks(CARVE_STEP_CLEARANCE as usize) {
            assert!(step[0].z > prev_feet, "emission not bottom-up");
            prev_feet = step[0].z;
        }
    }

    #[test]
    fn carve_ramp_short_xy_keeps_heading_into_face() {
        // Rise 5 but the rim is only 2 columns away: the stair keeps its
        // heading and cuts deeper into the face (no oscillation — the
        // remaining-delta bug this test pins).
        let solid = |_: Vec3<i32>| true;
        let open = |_: Vec3<i32>| true;
        let digs = carve_ramp(Vec3::new(0, 0, 0), Vec3::new(2, 0, 5), &solid, &open).unwrap();
        let xs: Vec<i32> = digs
            .chunks(CARVE_STEP_CLEARANCE as usize)
            .map(|c| c[0].x)
            .collect();
        assert_eq!(xs, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn carve_ramp_switchbacks_inside_a_narrow_mask() {
        // Rise 6 inside a mask only 4 columns wide in x (0..=3, all y): the
        // stair must SNAKE (switchback via a perpendicular jog) instead of
        // leaving the mask, and never reuse a column (a reused column's
        // floor was already dug out).
        let solid = |_: Vec3<i32>| true;
        let mask = |p: Vec3<i32>| (0..=3).contains(&p.x);
        let digs = carve_ramp(Vec3::new(0, 0, 0), Vec3::new(6, 0, 6), &solid, &mask)
            .expect("switchback stair must route inside the mask");
        let cols: Vec<(i32, i32)> = digs
            .chunks(CARVE_STEP_CLEARANCE as usize)
            .map(|c| (c[0].x, c[0].y))
            .collect();
        assert_eq!(cols.len(), 6);
        // All columns inside the mask, all distinct.
        assert!(cols.iter().all(|(x, _)| (0..=3).contains(x)));
        for (i, a) in cols.iter().enumerate() {
            assert!(!cols[i + 1..].contains(a), "column reused: {a:?}");
        }
    }

    #[test]
    fn carve_ramp_degenerate_inputs_refuse() {
        let solid = |_: Vec3<i32>| true;
        let open = |_: Vec3<i32>| true;
        // No rise → nothing to carve.
        assert!(carve_ramp(Vec3::new(0, 0, 5), Vec3::new(3, 0, 5), &solid, &open).is_none());
        // Fully-disallowed mask → cannot route.
        let never = |_: Vec3<i32>| false;
        assert!(carve_ramp(Vec3::new(0, 0, 0), Vec3::new(5, 0, 5), &solid, &never).is_none());
    }

    #[test]
    fn carve_ramp_refuses_floorless_routes() {
        // Only z <= 2 is solid: steps above have no floor to stand on — a
        // stair cannot route through open space (that's the ladder's job).
        let solid = |p: Vec3<i32>| p.z <= 2;
        let open = |_: Vec3<i32>| true;
        assert!(carve_ramp(Vec3::new(0, 0, 0), Vec3::new(5, 0, 5), &solid, &open).is_none());
    }
}

impl BastionColonist {
    /// Randomized starting colonist: name, backstory, skills 0..=5.
    pub fn generate(rng: &mut impl rand::Rng) -> Self {
        fn pick(list: &[&str], rng: &mut impl rand::Rng) -> String {
            list[rng.random_range(0..list.len())].to_string()
        }
        fn skill(rng: &mut impl rand::Rng) -> SkillLevel {
            SkillLevel {
                level: rng.random_range(0..=5),
                xp: 0.0,
            }
        }
        let name = format!(
            "{} {}",
            pick(COLONIST_FIRST_NAMES, rng),
            pick(COLONIST_EPITHETS, rng)
        );
        let backstory = pick(COLONIST_BACKSTORIES, rng);
        Self {
            name,
            backstory,
            // ITEM 14 axis 2: the NEUTRAL default, not an invented spread.
            // The ruling says bravery varies by the individual
            // (personality/veterancy) — but the DISTRIBUTION is a balance
            // choice, and `rng` is right here, so it would have been one line
            // to invent one. Banked for Ben instead; the fixture pins two
            // distinct values via BASTION_GUARD_BRAVERY to score bar 1.
            guard_bravery: default_guard_bravery(),
            skills: ColonistSkills {
                mining: skill(rng),
                woodcutting: skill(rng),
                construction: skill(rng),
                hauling: skill(rng),
                cooking: skill(rng),
                melee: skill(rng),
                farming: skill(rng),
                // B5.8: most settlers start a poor climber (0..=1 — reach
                // gating makes 3-block scrambles a TRAINED capability).
                climbing: SkillLevel {
                    level: rng.random_range(0..=1),
                    xp: 0.0,
                },
            },
            work_priorities: WorkPriorities::default(),
            soft_until: 0.0,
            route_squeeze_until: 0.0,
            climb_free_until: 0.0,
            inventory: None,
            // FOCUS-0: fresh settlers start with no tracked personal-need
            // state (FOCUS-1 populates it).
            personal_needs: Default::default(),
            // FOCUS-0-DERIVE (43.1): the REAL generation-time value roll
            // (architect's ruling — a feature, not test-only): every
            // value gets a ±50 weight from the SAME rng thread as
            // skills/name/backstory (the 0..=5 precedent), so rosters
            // carry genuine variance at spawn, deterministic under a
            // seeded caller. Old saves keep their serde-default (empty =
            // baseline) — the roll only shapes NEW colonists.
            values: {
                let mut v = std::collections::BTreeMap::new();
                for value in [
                    Value::Glory,
                    Value::Tradition,
                    Value::Kin,
                    Value::Wealth,
                    Value::Piety,
                    Value::Nature,
                    Value::Craft,
                    Value::Freedom,
                ] {
                    v.insert(value, rng.random_range(-50i8..=50));
                }
                v
            },
            // B7-0: never-captured until the first loaded tick mirrors
            // the live meters (LOD-0 semantics).
            needs: None,
            mood: None,
            // B7-1: no bed until one is assigned.
            owned_bed: None,
            // RUN-0: everyone walks until an urgency trigger says
            // otherwise.
            running: false,
        }
    }
}

#[cfg(test)]
mod t1_16_tests {
    use super::*;

    #[test]
    fn t1_16_board_conservation_verdict() {
        let base = JobAudit {
            total: 3,
            claimed: 2,
            unreachable: 0,
            claims_distinct: true,
            reservation_conflicts: 0,
        };
        assert!(base.conserved());
        // A double-reserved item breaks conservation.
        assert!(!JobAudit { reservation_conflicts: 1, ..base }.conserved());
        // A shared claim ticket breaks conservation.
        assert!(!JobAudit { claims_distinct: false, ..base }.conserved());
    }
}

#[cfg(test)]
mod eject_tests {
    use super::*;

    /// CASE-003 class pin: a cell whose FLOOR is solid but whose feet or
    /// HEAD cell is occupied (a tree trunk standing on the surface, a 1-high
    /// cave crack) is NOT standable — the search must skip it and land on a
    /// genuinely open cell. This is the exact hole `surface_teleport_dest`
    /// had: a surface scan that sees THROUGH non-surface solids accepted the
    /// inside of a trunk as a destination.
    #[test]
    fn eject_skips_occupied_and_one_high_cells() {
        // Flat ground: everything z <= 10 is solid. A trunk occupies
        // (1,0,11) and (1,0,12) — feet AND head of the r=1 cell east.
        // A "crack" at (0,1,·): solid ceiling at (0,1,12) → head blocked.
        let solid = |p: Vec3<i32>| {
            p.z <= 10
                || (p.xy() == Vec2::new(1, 0) && (p.z == 11 || p.z == 12))
                || (p.xy() == Vec2::new(0, 1) && p.z == 12)
        };
        let none = hashbrown::HashSet::new();
        let dest =
            eject_dest_impl(solid, Vec3::new(0, 0, 11), &none).expect("open cells exist in ring 1");
        // Must not be the trunk cell nor the crack cell; must be standable.
        assert_ne!(dest.xy(), Vec2::new(1, 0), "landed inside the trunk");
        assert_ne!(dest.xy(), Vec2::new(0, 1), "landed in the 1-high crack");
        assert!(!solid(dest) && !solid(dest + Vec3::unit_z()) && solid(dest - Vec3::unit_z()));
    }

    /// No standable cell in range → None (caller leaves the victim in
    /// place); and the crush exclusion is honoured.
    #[test]
    fn eject_none_when_sealed_and_crush_excluded() {
        // Solid EVERYWHERE: nothing standable.
        assert_eq!(
            eject_dest_impl(|_| true, Vec3::new(0, 0, 5), &hashbrown::HashSet::new()),
            None
        );
        // Exactly one open column at (1,0) — but it is in the crush set.
        let solid = |p: Vec3<i32>| !(p.xy() == Vec2::new(1, 0) && p.z > 10);
        let crush: hashbrown::HashSet<Vec2<i32>> = [Vec2::new(1, 0)].into_iter().collect();
        assert_eq!(eject_dest_impl(solid, Vec3::new(0, 0, 11), &crush), None);
    }
}
