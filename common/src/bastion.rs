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

/// What a painted designation region means. B4 turns these into jobs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DesignationKind {
    Mine,
    Chop,
    Build,
    Stockpile,
}

impl DesignationKind {
    pub fn label(&self) -> &'static str {
        match self {
            DesignationKind::Mine => "Mine",
            DesignationKind::Chop => "Chop",
            DesignationKind::Build => "Build",
            DesignationKind::Stockpile => "Stockpile",
        }
    }
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
    pub fn stubbed(&self) -> bool {
        matches!(self, ContextVerb::Embody | ContextVerb::ForceAction)
    }
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
}

impl WorkType {
    pub fn label(&self) -> &'static str {
        match self {
            WorkType::Mine => "mine",
            WorkType::Chop => "chop",
            WorkType::Build => "build",
            WorkType::Haul => "haul",
            WorkType::Cook => "cook",
        }
    }
}

impl DesignationKind {
    /// The work-type this designation's jobs require. (Build/Stockpile job
    /// *generation* lands with B5 blueprints / B6 zones; the mapping exists
    /// now so priorities are honored from day one.)
    pub fn work_type(&self) -> WorkType {
        match self {
            DesignationKind::Mine => WorkType::Mine,
            DesignationKind::Chop => WorkType::Chop,
            DesignationKind::Build => WorkType::Build,
            DesignationKind::Stockpile => WorkType::Haul,
        }
    }
}

/// One unit of colonist work — a block-level task generated from a
/// designation (B4). Serde-ready (B10). `claimed_by` is a transient claim
/// (entity `Uid`); claims are released on cancel/failure/demote.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub kind: DesignationKind,
    pub work: WorkType,
    /// Target block.
    pub pos: Vec3<i32>,
    /// Minimum skill level required (0 = anyone). Unused by v1 generation.
    pub skill_floor: u16,
    pub claimed_by: Option<crate::uid::Uid>,
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
}

impl ColonistSkills {
    /// Route completion XP to the skill matching the work type (B5).
    pub fn grant_xp(&mut self, work: WorkType, xp: f32) {
        match work {
            WorkType::Mine => self.mining.add_xp(xp),
            WorkType::Chop => self.woodcutting.add_xp(xp),
            WorkType::Build => self.construction.add_xp(xp),
            WorkType::Haul => self.hauling.add_xp(xp),
            WorkType::Cook => self.cooking.add_xp(xp),
        }
    }

    /// The level of the skill tracking a work type — what arbitration gates
    /// `skill_floor` on and B5's work rate scales by.
    pub fn level_for(&self, work: WorkType) -> u16 {
        match work {
            WorkType::Mine => self.mining.level,
            WorkType::Chop => self.woodcutting.level,
            WorkType::Build => self.construction.level,
            WorkType::Haul => self.hauling.level,
            WorkType::Cook => self.cooking.level,
        }
    }

    /// Directly set the level of the skill tracking a work type (harness /
    /// scenario tooling; gameplay progression goes through `grant_xp`).
    pub fn set_level_for(&mut self, work: WorkType, level: u16) {
        let s = match work {
            WorkType::Mine => &mut self.mining,
            WorkType::Chop => &mut self.woodcutting,
            WorkType::Build => &mut self.construction,
            WorkType::Haul => &mut self.hauling,
            WorkType::Cook => &mut self.cooking,
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
}

impl Default for WorkPriorities {
    fn default() -> Self {
        Self {
            mine: 3,
            chop: 3,
            build: 3,
            haul: 3,
            cook: 3,
        }
    }
}

impl WorkPriorities {
    /// Priority for a work type: 0 = never do this work, 1..=4 rising.
    pub fn get(&self, work: WorkType) -> u8 {
        match work {
            WorkType::Mine => self.mine,
            WorkType::Chop => self.chop,
            WorkType::Build => self.build,
            WorkType::Haul => self.haul,
            WorkType::Cook => self.cook,
        }
    }

    pub fn set(&mut self, work: WorkType, priority: u8) {
        let p = priority.min(4);
        match work {
            WorkType::Mine => self.mine = p,
            WorkType::Chop => self.chop = p,
            WorkType::Build => self.build = p,
            WorkType::Haul => self.haul = p,
            WorkType::Cook => self.cook = p,
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
}

const COLONIST_FIRST_NAMES: &[&str] = &[
    "Awen", "Bram", "Cerys", "Doran", "Eira", "Fenn", "Gwil", "Hesta", "Ivo", "Jena", "Kell",
    "Lira", "Maddoc", "Nia", "Osric", "Peri", "Quill", "Rhosyn", "Sten", "Tegan", "Ulric", "Vada",
    "Wynn", "Yara",
];

const COLONIST_EPITHETS: &[&str] = &[
    "the Steady", "of the Vale", "Ironhand", "the Quiet", "Longstride", "the Younger", "Ashborn",
    "the Stout", "Brighteye", "of the Ford", "the Wary", "Oakenshield",
];

const COLONIST_BACKSTORIES: &[&str] = &[
    "farmhand", "quarry worker", "wandering tinker", "disgraced guard", "orchard keeper",
    "charcoal burner", "riverboat hand", "apprentice mason", "trapper", "camp cook",
];

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(desig.subtract(&erase_drag), vec![desig], "reproduces the bug");
        // The fix: clip the erase to the designation's XY at the DESIGNATION's
        // z, then subtract → fully removed.
        let clipped = desig
            .clip_xy(erase_drag.min.xy(), erase_drag.max.xy())
            .expect("xy overlaps");
        assert!(desig.subtract(&clipped).is_empty(), "full XY cover erases cleanly");
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
            r((3, 3, 3), (6, 6, 6)),   // interior hole → 6 pieces
            r((0, 0, 0), (4, 9, 9)),   // face slab
            r((5, 5, 5), (20, 20, 20)), // corner cut
            r((0, 4, 0), (9, 5, 9)),   // through-slab
            r((-5, -5, -5), (0, 0, 0)), // corner nick
        ] {
            let pieces = a.subtract(&b);
            let inter_vol = a.intersection(&b).map_or(0, |i| i.volume());
            let piece_vol: i64 = pieces.iter().map(|p| p.volume()).sum();
            assert_eq!(a.volume(), inter_vol + piece_vol, "volume not conserved vs {b:?}");
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
            skills: ColonistSkills {
                mining: skill(rng),
                woodcutting: skill(rng),
                construction: skill(rng),
                hauling: skill(rng),
                cooking: skill(rng),
                melee: skill(rng),
            },
            work_priorities: WorkPriorities::default(),
        }
    }
}
