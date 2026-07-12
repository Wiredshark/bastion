//! bastion (B-AG5-CORE, row 36): the colony's ACTION VERBS as standalone,
//! callable helpers — the "approach a target → do the verb → produce the
//! outcome" shape Mine/Chop/Build/Ladder/Haul already execute inside
//! `bastion_jobs::Sys::run`, extracted so a SECOND caller can compose it
//! without fabricating a `Job` to ride the ECS pipeline. GATHER (row 38) is
//! the first such caller (approach via the standable solver → execute via
//! `ControlAction::Collect` → authoritative interaction creates the item →
//! ordinary deposit); NPC self-drives (B-AG3) come later and call the same
//! verbs. Prior art: RimWorld's Toil/JobDriver split, DF's shared job types
//! across player- and dwarf-initiated labor — ONE execution mechanism,
//! multiple selection/trigger paths (the shape B4's single JobBoard already
//! chose).
//!
//! BOUNDARY (deliberate, documented per the packet): the TRAVEL/steer chain
//! (anchor staging, watchdog rebases, queue discipline) stays in
//! `bastion_jobs` — it is drive machinery around `NpcActivity::Goto`, not
//! verb machinery; every caller supplies its own drive. What lives here is
//! the verb-side: the arrive-target formula, work-progress accrual, the
//! completion terrain-edit, and the item-flow trio (pickup / drop / deposit)
//! that B6-HAUL proved.

use common::{
    bastion::{DesignationKind, JobKind, WorkType, tool_factor},
    comp::{self, PickupItem},
    event::{CreateItemDropEvent, InventoryManipEvent},
    resources::ProgramTime,
    terrain::{Block, BlockKind, SpriteKind},
    uid::Uid,
};
use rand::RngExt;
use specs::Entity;
use vek::{Rgb, Vec2, Vec3};

/// The canonical ARRIVE TARGET for working a block from a committed stance
/// (B15/FR12): the stance FEET cell's center. `(0,0,1)` reproduces the
/// classic on-top target; a cardinal stance stands beside the block.
pub fn approach_target(job_pos: Vec3<i32>, stance: Vec3<i32>) -> Vec3<f32> {
    (job_pos + stance).map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)
}

/// One tick of WORK toward completion — skill- and tool-scaled (TOOL-0:
/// both axes pay). Pure; the caller owns the accumulator.
pub fn work_progress(
    dt: f32,
    skill_level: u16,
    work: WorkType,
    tool: Option<(comp::item::tool::ToolKind, comp::item::Quality)>,
) -> f32 {
    dt * crate::bastion_jobs::work_rate(skill_level) * tool_factor(work, tool)
}

/// The COMPLETION terrain edit for a designation verb — what the target
/// block becomes. `None` = this kind completes without editing terrain
/// (Stockpile zones, Haul — and GATHER, whose sprite is consumed by the
/// authoritative interaction, never deleted here).
pub fn completion_block(kind: JobKind) -> Option<Block> {
    match kind {
        JobKind::Designated(d) => match d {
            DesignationKind::Mine | DesignationKind::Chop => Some(Block::empty()),
            DesignationKind::Build => {
                Some(Block::new(BlockKind::Rock, Rgb::new(150, 150, 150)))
            },
            // B5.8: the native climbable ladder sprite — the vertical link
            // pathfinding knows about.
            DesignationKind::Ladder => Some(Block::air(SpriteKind::Ladder)),
            DesignationKind::Stockpile => None,
        },
        JobKind::Haul { .. } => None,
    }
}

/// Emit a colonist-output DROP (B5.5 semantics: persistent, mergeable —
/// burst output aggregates into piles). The gentle deterministic toss keeps
/// spawn-time merging effective (DETRNG: the rng is the caller's
/// tick-seeded one).
pub fn emit_drop(
    emitter: &mut common::event::Emitter<CreateItemDropEvent>,
    pos: Vec3<i32>,
    item: comp::Item,
    program_time: ProgramTime,
    rng: &mut impl RngExt,
) {
    emitter.emit(CreateItemDropEvent {
        pos: comp::Pos(pos.map(|e| e as f32) + Vec3::broadcast(0.5)),
        vel: comp::Vel(
            (Vec2::unit_x()
                .rotated_z(rng.random::<f32>() * std::f32::consts::TAU)
                * 0.5)
                .with_z(rng.random_range(2.0..4.0)),
        ),
        ori: comp::Ori::default(),
        item: PickupItem::new(item, program_time, true),
        loot_owner: None,
        persistent: true,
    });
}

/// COLLECT a loose item entity through the VANILLA pickup path (capacity
/// checks, despawn, inventory insert all owned by the authoritative
/// handler — never a second pickup mechanism). Idempotent-safe: a re-emit
/// against a consumed uid no-ops in the handler; the ITEM ENTITY VANISHING
/// is the caller's confirmation signal.
pub fn emit_pickup(
    emitter: &mut common::event::Emitter<InventoryManipEvent>,
    collector: Entity,
    item: Uid,
) {
    emitter.emit(InventoryManipEvent(
        collector,
        comp::InventoryManip::Pickup(item),
    ));
}

/// DEPOSIT: drain every bag stack of `def` from the inventory and drop it
/// at `pos` (a stockpile cell — pile merging re-aggregates). Returns the
/// total amount deposited. The B6-HAUL leg-2 shape, callable.
pub fn deposit_all_of(
    inv: &mut comp::Inventory,
    def: &str,
    pos: Vec3<i32>,
    emitter: &mut common::event::Emitter<CreateItemDropEvent>,
    program_time: ProgramTime,
) -> u32 {
    let slots: Vec<_> = inv
        .slots_with_id()
        .filter_map(|(slot, i)| {
            i.as_ref()
                .is_some_and(|i| i.item_definition_id().itemdef_id() == Some(def))
                .then_some(slot)
        })
        .collect();
    let mut deposited = 0u32;
    for slot in slots {
        if let Some(item_out) = inv.remove(slot) {
            deposited += item_out.amount();
            emitter.emit(CreateItemDropEvent {
                pos: comp::Pos(pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 1.0)),
                vel: comp::Vel(Vec3::zero()),
                ori: comp::Ori::default(),
                item: PickupItem::new(item_out, program_time, true),
                loot_owner: None,
                persistent: true,
            });
        }
    }
    deposited
}

