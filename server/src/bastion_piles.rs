//! bastion (B5.5): persistent item-pile upkeep — tier-scale visuals.
//!
//! A colonist-produced pile ([`comp::bastion::BastionPile`], created by
//! `create_item_drop(persistent: true)`) is a single `PickupItem` entity
//! whose `amount()` grows as nearby drops merge into it (vanilla merge
//! machinery — see `sys::item` and the class gate in
//! `get_nearby_mergeable_items`). This system makes the pile *read* as a
//! heap: `comp::Scale` (synced) steps up with the amount tier, so a big
//! pile renders visibly bigger with zero client changes. A real heap mesh /
//! count label is future asset/B9 work (see the backlog).

use crate::Tick;
use common::comp;
use common_ecs::{Job as EcsJob, Origin, Phase, System};
use specs::{Entities, Join, ReadStorage, WriteStorage};

/// Upkeep cadence in ticks (~1 s at 30 tps): merges are not so frequent that
/// scale needs per-tick tracking.
const PILE_SCALE_INTERVAL: u64 = 30;

/// B5.6a: pile visual tiers — the mesh grows with the count through five
/// steps then PLATEAUS at a great-mound cap (the count keeps rising; the
/// visual stops so a 400-stone pile doesn't tower absurdly). Purely visual:
/// the scale never feeds back into the count (conservation stays exact — the
/// tier is read from `PickupItem::amount()`, never the reverse).
///
///   scatter (1–5) → small heap (6–20) → heap (21–60) → large mound (61–150)
///   → capped great-mound (150+, plateau)
fn tier_scale(amount: u32) -> f32 {
    match amount {
        0..=5 => 0.8,
        6..=20 => 1.1,
        21..=60 => 1.45,
        61..=150 => 1.8,
        _ => 2.15,
    }
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        specs::Read<'a, Tick>,
        ReadStorage<'a, comp::bastion::BastionPile>,
        ReadStorage<'a, comp::PickupItem>,
        WriteStorage<'a, comp::Scale>,
    );

    const NAME: &'static str = "bastion_piles";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut EcsJob<Self>,
        (entities, tick, piles, items, mut scales): Self::SystemData,
    ) {
        if tick.0 % PILE_SCALE_INTERVAL != 0 {
            return;
        }
        for (entity, _, item) in (&entities, &piles, &items).join() {
            let scale = tier_scale(item.amount());
            let current = scales.get(entity).map(|s| s.0);
            if current != Some(scale) {
                let _ = scales.insert(entity, comp::Scale(scale));
            }
        }
    }
}
