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

// ─── B3: colonists ──────────────────────────────────────────────────────────

/// A work skill's progression. Levels rise as B5 grants completion XP.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillLevel {
    pub level: u16,
    pub xp: f32,
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
