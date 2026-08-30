static PINNED_SO_FAR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

use super::*;
use crate::{ServerConstants, persistence::DatabaseSettings, sys::terrain::SpawnEntityData};
use common::{
    LoadoutBuilder,
    calendar::Calendar,
    comp::{
        self, Body, Item, Presence, PresenceKind,
        inventory::trade_pricing::TradePricing,
        item::{ItemDefinitionIdOwned, Quality},
        slot::ArmorSlot,
    },
    event::{CreateNpcEvent, CreateShipEvent, DeleteEvent, EventBus, NpcBuilder},
    generation::{BodyBuilder, EntityConfig, EntityInfo},
    resources::{DeltaTime, Time, TimeOfDay},
    rtsim::{Actor, NpcId, RtSimEntity},
    slowjob::SlowJobPool,
    terrain::CoordinateConversions,
    trade::{Good, SiteInformation},
    uid::{IdMaps, Uid},
    util::Dir,
    weather::WeatherGrid,
};
use common_ecs::{Job, Origin, Phase, System};
use rand::RngExt;
use rand_chacha::ChaCha8Rng;
use rtsim::{
    ai::NpcSystemData,
    data::{
        Npc, Sites,
        npc::{Profession, SimulationMode},
    },
};
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::{
    ops::Range,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use tracing::error;

/// ★ A DOMAIN ID IS NOT A NUMBER YOU MAY XOR (adversarial review, ROW 38/50).
///
/// Both population drains below need a per-cell stream — two settlers arriving
/// on the same tick must not be the same person — and both built it as
/// `DOMAIN ^ cell_salt`. `tick_rng` takes ONE u32 and feeds it to the hasher as
/// ONE field (rtsim/src/lib.rs:175, whose own comment is "distinct (seed, tick,
/// salt) can never alias"), so XOR-folding an unbounded cell hash onto the
/// domain id does not extend the domain: it ERASES it. The four ids are
/// 0xBA57_C013..0xBA57_C016 and differ only in their low three bits, so any two
/// cells whose salts differ in those bits alias ACROSS generators.
///
/// MEASURED, not feared. With the old salt
/// (`x*0x9E37_79B9 + y*0x85EB_CA6B + z`, `z` folded in by plain addition) a
/// settler bed at `(x, y, z)` and a birth corner at `(x, y, z+6)` land on the
/// SAME effective salt whenever the settler salt's low three bits are 0 or 1,
/// because `0xC013 ^ 0xC015 == 6`. Swept over a 40x40x40 town box that is
/// 16,000 of 64,000 cells — one quarter of the column pairs, not an
/// astronomical hash accident. Both drains take their epoch from the same
/// `data.tick` in the same `run()`, and the producer gates immigration and
/// births on two SEPARATE daily flags, so a day on which both fire queues both
/// into the same tick. The newborn and the newcomer then come out of a
/// byte-identical stream: same name, same backstory, same species, same face.
/// Two colonists sharing a name is not merely odd — `bastion_set_work_priority`
/// matches by name and writes to EVERY hit.
///
/// So the cell is folded into the domain the way rtsim folds its own inputs:
/// length-prefixed hashing, with the domain as a field of its own. Two distinct
/// `(domain, cell)` pairs now need a real 32-bit hash collision instead of a
/// z-offset of six.
///
/// BASELINE NOTE, stated plainly: this changes WHICH colonist a given
/// `(world, tick, cell)` mints. Colonists already in a save are untouched —
/// they are persisted records — but settlers and children minted after this
/// change carry different names and faces than the same world would have minted
/// before it.
fn bastion_drain_stream_salt(domain: u32, cell: Vec3<i32>) -> u32 {
    let mut h =
        common::state_hash::DomainHasher::new("bastion/domain/rtsim-population-drain/v1/sha256");
    h.field(&domain.to_le_bytes());
    h.field(&cell.x.to_le_bytes());
    h.field(&cell.y.to_le_bytes());
    h.field(&cell.z.to_le_bytes());
    let digest = h.finish().0;
    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}

/// ★ THE PROMOTE AND THE DEMOTE WERE NOT THE SAME TEST, AND THE TOWN SPUN.
///
/// MEASURED on the owner's live world (`.item29-wt/play-server.log`, a 33.8
/// minute window): 38,513 `colonist promoted to loaded entity` against 38,518
/// `colonist demoted to SimulationMode::Simulated` — ~38 entity spawn/despawn
/// transitions per second over only 21 distinct colonists, several cycling
/// ~3,500 times each. `colony_total` was pinned at 115 and `loaded_before`
/// returned to the same two values (98, 97) on every single cycle, so it was a
/// STABLE LIMIT CYCLE, not drift. In one 3 MB tail slice two colonists each
/// cycled 538 times with a promote→demote gap of 4 ms and a demote→promote gap
/// of one server tick (~130 ms on that world): one full spawn/despawn of a
/// fully-decorated humanoid — loadout, inventory restore, agent, chronicle
/// retention — per colonist per tick, forever.
///
/// The two directions were reading DIFFERENT AUTHORITIES about the same fact:
///
/// * PROMOTE (below, `impl System for Sys::run`, the "Load in NPCs" loop):
///   `chunk_states.0.get(npc.wpos → cpos).is_some_and(|c| c.is_some())` — the
///   rtsim-side `ChunkStates` grid, a CACHE maintained by
///   `RtSim::hook_load_chunk` / `RtSim::hook_unload_chunk`
///   (server/src/rtsim/mod.rs).
/// * DEMOTE (`Server::tick`, server/src/lib.rs, the "Remove NPCs that are
///   outside the view distances of all players" block): the entity is deleted,
///   and `RtSim::hook_rtsim_entity_unload` therefore flips it to
///   `SimulationMode::Simulated`, when
///   `terrain.get_key_real(pos_key(pos)).is_none()` — the authoritative
///   `TerrainGrid` itself.
///
/// The chunk KEY is the same on both sides (an rtsim npc is spawned at exactly
/// `npc.wpos` — `get_npc_entity_info` uses `comp::Pos(npc.wpos)` verbatim and
/// `to_npc_builder` passes it through; both sides then take the same xy
/// `div_euclid`). Nothing repositions it in between: `RepositionToFreeSpace`
/// only searches in Z. So the disagreement is purely CACHE vs TERRAIN, and
/// there was NO HYSTERESIS AND NO DEAD-BAND of any size in either direction —
/// both sides are a bare boolean with no margin and neither records anything
/// the other can read. A disagreement of one chunk is therefore not a blip: it
/// is an unbounded oscillator at tick rate, which is exactly what the world
/// showed.
///
/// THE FIX IS AGREEMENT BY CONSTRUCTION, NOT A COOLDOWN. A timer would have
/// divided the spin rate by a constant and left the two directions still
/// arguing. Instead the promote now asks the demote's own question of the
/// demote's own grid, so `bastion_may_promote_npc` and
/// `bastion_sweep_would_delete` can never both be true — pinned over the whole
/// input space in `bastion_promotion_pins` below. The `ChunkStates` test is
/// KEPT as well, as an AND: an extra conjunct can only ever refuse a promotion
/// that the sweep would have reversed within the same tick, never permit one.
///
/// The ordering that makes this exact rather than probable is declared in
/// `server/src/rtsim/mod.rs::add_server_systems`: this system now runs AFTER
/// `sys::terrain::Sys`, which is the only code that inserts or removes terrain
/// chunks. So the `TerrainGrid` this predicate reads is byte-for-byte the one
/// the deletion sweep will read later in the same `Server::tick`.
///
/// Applies to every rtsim npc, not only colonists — deliberately. The defect
/// is in the shared promote loop, and a fix left in one branch of a pair is a
/// fix that hides behind its sibling. On a world where the two authorities
/// agree (every healthy world) the added conjunct is true whenever the old one
/// was, so vanilla behaviour is unchanged.
///
/// NO DEAD-BAND, DELIBERATELY, and this is the interesting half. A settle-hold
/// ("do not re-promote for K ticks after a demotion") was written and then
/// REMOVED: with the root cause identified as an engine-level chunk thrash, a
/// constant in rtsim would not settle anything, it would divide the visible
/// spin rate by K while the chunk underneath went on loading and unloading —
/// the "widen it until the symptom disappears" mistake this program has
/// already paid for. The two directions do not need a margin between them
/// because they are no longer two directions: they are one test, read off one
/// registry, inside one tick. The generate-vs-retain annulus is a terrain
/// decision and is owned elsewhere.
pub(crate) fn bastion_may_promote_npc(
    is_simulated: bool,
    rtsim_chunk_state_loaded: bool,
    terrain_chunk_real: bool,
    is_mounted: bool,
) -> bool {
    is_simulated && rtsim_chunk_state_loaded && terrain_chunk_real && !is_mounted
}

/// ★ ONE CHUNK KEY, ONE ROUNDING RULE (latent frame bug, found alongside the
/// oscillator). The promote used `npc.wpos.xy().as_::<i32>()` — an `as` cast,
/// which TRUNCATES TOWARD ZERO — while the deletion sweep in `Server::tick`
/// uses `pos.0.map(|e| e.floor() as i32)`. For every positive coordinate those
/// agree, which is why no live world has shown it; for a block anywhere in
/// `[-1, 0)` they do not. Truncation puts wpos `-0.5` in chunk 0 and flooring
/// puts it in chunk -1, so the two sides would test DIFFERENT CHUNKS and the
/// promote/demote pair would disagree by construction for any colony west or
/// north of the world origin — the same defect that was measured here, waiting
/// on a map coordinate.
///
/// Flooring, because the demote's rule is the authoritative one: it is the
/// side that owns the entity's fate.
pub(crate) fn bastion_npc_chunk_key(wpos: Vec3<f32>) -> Vec2<i32> {
    wpos.xy().map(|e| e.floor() as i32).wpos_to_cpos()
}

/// The DEMOTE side of the pair above, written as the deletion sweep in
/// `Server::tick` (server/src/lib.rs, "Remove NPCs that are outside the view
/// distances of all players") actually computes it for an rtsim npc: an rtsim
/// npc carries no `Anchor` (`NpcBuilder` leaves it `None` and
/// `handle_create_npc` only attaches one when it is `Some`), so the sweep
/// always takes the `None` arm — delete iff the chunk under the entity is not
/// a real chunk.
///
/// It exists so the invariant that ties the two directions together can be
/// STATED AND PINNED rather than asserted in prose. If the sweep's rule ever
/// changes, this is the line that has to change with it, and the pin below
/// fails until it does.
///
/// Deliberately unused OUTSIDE the pins: it is a MODEL of code that lives in
/// another file, and calling it from production here would make the model and
/// the modelled thing the same expression, which is how a pin stops testing
/// anything.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn bastion_sweep_would_delete(terrain_chunk_real: bool) -> bool { !terrain_chunk_real }

/// The four population-drain streams, named once so the pin below and the two
/// drains cannot drift apart.
const BASTION_SETTLER_IDENTITY_DOMAIN: u32 = 0xBA57_C013;
const BASTION_SETTLER_PERSONALITY_DOMAIN: u32 = 0xBA57_C014;
const BASTION_BIRTH_IDENTITY_DOMAIN: u32 = 0xBA57_C015;
const BASTION_BIRTH_PERSONALITY_DOMAIN: u32 = 0xBA57_C016;

pub fn trader_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    permitted: impl FnMut(Good) -> bool,
    mut permitted_quality: impl FnMut(Quality) -> bool,
    coin_range: Range<f32>,
    rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    let mut backpack = Item::new_from_asset_expect("common.items.armor.misc.back.backpack");
    let mut bag1 = Item::new_from_asset_expect("common.items.armor.misc.bag.sturdy_red_backpack");
    let mut bag2 = Item::new_from_asset_expect("common.items.armor.misc.bag.sturdy_red_backpack");
    let mut bag3 = Item::new_from_asset_expect("common.items.armor.misc.bag.sturdy_red_backpack");
    let mut bag4 = Item::new_from_asset_expect("common.items.armor.misc.bag.sturdy_red_backpack");
    let slots = backpack.slots().len() + 4 * bag1.slots().len();
    let mut stockmap: hashbrown::HashMap<Good, f32> = economy
        .map(|e| e.unconsumed_stock.clone().into_iter().collect())
        .unwrap_or_default();
    // modify stock for better gameplay
    stockmap
        .entry(Good::Ingredients)
        .and_modify(|e| {
            *e = e.max(10_000.0);
            *e *= 100_000.0;
            *e = e.min(2_000_000.0);
        })
        .or_insert(1_000_000.0);

    // economy isn't economying sometimes
    stockmap
        .entry(Good::Wood)
        .and_modify(|e| {
            *e = e.max(10_000.0);
            *e *= 100_000.0;
            *e = e.min(2_000_000.0);
        })
        .or_insert(1_000_000.0);

    // econsim doesn't produce recipes at all
    stockmap
        .entry(Good::Recipe)
        .and_modify(|e| *e = e.max(10_000.0))
        .or_insert(10_000.0);

    // TODO: currently econsim spends all its food on population, resulting in none
    // for the players to buy; the `.max` is temporary to ensure that there's some
    // food for sale at every site, to be used until we have some solution like NPC
    // houses as a limit on econsim population growth
    stockmap
        .entry(Good::Food)
        .and_modify(|e| *e = e.max(10_000.0))
        .or_insert(10_000.0);
    // Reduce amount of potions so merchants do not oversupply potions.
    // TODO: Maybe remove when merchants and their inventories are rtsim?
    // Note: Likely without effect now that potions are counted as food
    stockmap
        .entry(Good::Potions)
        .and_modify(|e| *e = e.powf(0.25));
    stockmap
        .entry(Good::Coin)
        .and_modify(|e| *e = e.min(rng.random_range(coin_range)));

    // assume roughly 10 merchants sharing a town's stock (other logic for coins)
    stockmap
        .iter_mut()
        .filter(|(good, _amount)| **good != Good::Coin)
        .for_each(|(_good, amount)| *amount *= 0.1);
    // Fill bags with stuff according to unclaimed stock
    let ability_map = &comp::tool::AbilityMap::load().read();
    let msm = &comp::item::MaterialStatManifest::load().read();

    let mut allow_item = |n: ItemDefinitionIdOwned, a: &u32| -> Option<Item> {
        let i = Item::new_from_item_definition_id(n.as_ref(), ability_map, msm).ok();
        if !permitted_quality(i.as_ref()?.quality()) {
            return None;
        }
        i.map(|mut i| {
            i.set_amount(*a)
                .map_err(|_| tracing::error!("merchant loadout amount failure"))
                .ok();
            i
        })
    };

    let mut wares: Vec<Item> = TradePricing::random_items_with_rng(
        &mut stockmap,
        slots as u32,
        true,
        true,
        16,
        permitted,
        rng,
    )
    .into_iter()
    .filter_map(|(n, a)| allow_item(n, &a))
    .collect();
    sort_wares(&mut wares);
    transfer(&mut wares, &mut backpack);
    transfer(&mut wares, &mut bag1);
    transfer(&mut wares, &mut bag2);
    transfer(&mut wares, &mut bag3);
    transfer(&mut wares, &mut bag4);

    loadout_builder
        .back(Some(backpack))
        .bag(ArmorSlot::Bag1, Some(bag1))
        .bag(ArmorSlot::Bag2, Some(bag2))
        .bag(ArmorSlot::Bag3, Some(bag3))
        .bag(ArmorSlot::Bag4, Some(bag4))
}

fn sort_wares(bag: &mut [Item]) {
    use common::comp::item::TagExampleInfo;

    bag.sort_by(|a, b| {
        a.quality()
            .cmp(&b.quality())
        // sort by kind
        .then(
            Ord::cmp(
                a.tags().first().map_or("", |tag| tag.name()),
                b.tags().first().map_or("", |tag| tag.name()),
            )
        )
        // sort by name
        // TODO: figure out the better way here
        .then(#[expect(deprecated)] Ord::cmp(&a.legacy_name(), &b.legacy_name()))
    });
}

fn transfer(wares: &mut Vec<Item>, bag: &mut Item) {
    let capacity = bag.slots().len();
    for (s, w) in bag
        .slots_mut()
        .iter_mut()
        .zip(wares.drain(0..wares.len().min(capacity)))
    {
        *s = Some(w);
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Class7ItemObservation {
    pub slot: u32,
    pub definition_id: ItemDefinitionIdOwned,
    pub item_hash: u64,
    pub amount: u32,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Class7ItemFixtureResult {
    pub schema: &'static str,
    pub npc_seed: u32,
    pub lazy_loadout_seed: u64,
    pub inventory: Vec<Class7ItemObservation>,
    pub selected_use_item: Option<Class7ItemObservation>,
}

pub fn class7_item_observation(
    slot: common::comp::inventory::slot::InvSlotId,
    item: &Item,
) -> Class7ItemObservation {
    Class7ItemObservation {
        slot: slot.idx(),
        definition_id: item.item_definition_id().to_owned(),
        item_hash: item.item_hash(),
        amount: item.amount(),
    }
}

pub fn class7_inventory_observations(
    inventory: &comp::Inventory,
) -> Vec<Class7ItemObservation> {
    let mut observations = inventory
        .slots_with_id()
        .filter_map(|(slot, item)| item.as_ref().map(|item| class7_item_observation(slot, item)))
        .collect::<Vec<_>>();
    observations.sort_by_key(|item| item.slot);
    observations
}

/// Millisecond-scale production-seam fixture for registry class 7. This runs
/// the actual lazy farmer loadout, `SpawnEntityData` inventory construction,
/// and production healing-slot selector without starting the server loop.
pub fn bastion_class7_item_fixture(npc_seed: u32) -> Class7ItemFixtureResult {
    let lazy_loadout_seed = npc_seed.wrapping_add(Npc::PERM_LAZY_LOADOUT) as u64;
    let body = Body::Humanoid(
        comp::humanoid::Body::iter()
            .next()
            .expect("humanoid body corpus is non-empty"),
    );
    let info = EntityInfo::at(
        Vec3::zero(),
        &mut <rand_chacha::ChaCha8Rng as rand::SeedableRng>::seed_from_u64(lazy_loadout_seed),
    )
        .with_body(body)
        .with_lazy_loadout(farmer_loadout, lazy_loadout_seed);
    let SpawnEntityData::Npc(data) = SpawnEntityData::from_entity_info(info) else {
        unreachable!("ordinary EntityInfo creates an NPC")
    };
    let inventory = class7_inventory_observations(&data.inventory);
    let selected_use_item =
        crate::sys::agent::action_nodes::select_healing_item(&data.inventory, true, 1.0)
            .and_then(|slot| {
                data.inventory
                    .get(slot)
                    .map(|item| class7_item_observation(slot, item))
            });
    Class7ItemFixtureResult {
        schema: "bastion.class7-item-identity/v1",
        npc_seed,
        lazy_loadout_seed,
        inventory,
        selected_use_item,
    }
}

fn humanoid_config(profession: &Profession) -> &'static str {
    match profession {
        Profession::Farmer => "common.entity.village.farmer",
        Profession::Hunter => "common.entity.village.hunter",
        Profession::Herbalist => "common.entity.village.herbalist",
        Profession::Captain => "common.entity.village.captain",
        Profession::Merchant => "common.entity.village.merchant",
        Profession::Guard => "common.entity.village.guard",
        Profession::Adventurer(rank) => match rank {
            0 => "common.entity.world.traveler0",
            1 => "common.entity.world.traveler1",
            2 => "common.entity.world.traveler2",
            3 => "common.entity.world.traveler3",
            _ => {
                error!(
                    "Tried to get configuration for invalid adventurer rank {}",
                    rank
                );
                "common.entity.world.traveler3"
            },
        },
        Profession::Blacksmith => "common.entity.village.blacksmith",
        Profession::Chef => "common.entity.village.chef",
        Profession::Alchemist => "common.entity.village.alchemist",
        Profession::Pirate(leader) => match leader {
            false => "common.entity.spot.pirate",
            true => "common.entity.spot.buccaneer",
        },
        Profession::Cultist => "common.entity.dungeon.cultist.cultist",
    }
}

fn loadout_default(
    loadout: LoadoutBuilder,
    _economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
    _rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    loadout
}

fn merchant_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
    rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    trader_loadout(
        loadout_builder,
        economy,
        |good| {
            !matches!(
                good,
                Good::Ingredients | Good::Tools | Good::Armor | Good::Wood
            )
        },
        |quality| matches!(quality, Quality::Low | Quality::Common | Quality::Moderate),
        1000.0..3000.0,
        rng,
    )
}

fn farmer_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
    rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    trader_loadout(
        loadout_builder,
        economy,
        |good| matches!(good, Good::Food | Good::Coin),
        |quality| matches!(quality, Quality::Low | Quality::Common | Quality::Moderate),
        500.0..1400.0,
        rng,
    )
}

fn herbalist_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
    rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    trader_loadout(
        loadout_builder,
        economy,
        |good| {
            matches!(
                good,
                Good::Potions | Good::Stone | Good::Wood | Good::Ingredients | Good::Coin
            )
        },
        |quality| matches!(quality, Quality::Low | Quality::Common | Quality::Moderate),
        500.0..1400.0,
        rng,
    )
}

fn chef_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
    rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    trader_loadout(
        loadout_builder,
        economy,
        |good| matches!(good, Good::Food | Good::Coin),
        |quality| matches!(quality, Quality::Low | Quality::Common | Quality::Moderate),
        500.0..1400.0,
        rng,
    )
}

fn blacksmith_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
    rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    trader_loadout(
        loadout_builder,
        economy,
        |good| matches!(good, Good::Armor | Good::Coin),
        |quality| matches!(quality, Quality::Low | Quality::Common | Quality::Moderate),
        500.0..1400.0,
        rng,
    )
}

fn hunter_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
    rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    trader_loadout(
        loadout_builder,
        economy,
        |good| matches!(good, Good::Tools | Good::Coin),
        |quality| matches!(quality, Quality::Low | Quality::Common | Quality::Moderate),
        500.0..1400.0,
        rng,
    )
}

fn alchemist_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
    rng: &mut ChaCha8Rng,
) -> LoadoutBuilder {
    trader_loadout(
        loadout_builder,
        economy,
        |good| {
            matches!(
                good,
                Good::Potions | Good::Stone | Good::Wood | Good::Ingredients | Good::Coin
            )
        },
        |quality| matches!(quality, Quality::Low | Quality::Common | Quality::Moderate),
        500.0..1400.0,
        rng,
    )
}

fn profession_extra_loadout(
    profession: Option<&Profession>,
) -> common::generation::LazyLoadoutCreator {
    match profession {
        Some(Profession::Merchant) => merchant_loadout,
        Some(Profession::Farmer) => farmer_loadout,
        Some(Profession::Herbalist) => herbalist_loadout,
        Some(Profession::Chef) => chef_loadout,
        Some(Profession::Blacksmith) => blacksmith_loadout,
        Some(Profession::Alchemist) => alchemist_loadout,
        Some(Profession::Hunter) => hunter_loadout,
        _ => loadout_default,
    }
}

fn profession_agent_mark(profession: Option<&Profession>) -> Option<comp::agent::Mark> {
    match profession {
        Some(
            Profession::Merchant
            | Profession::Farmer
            | Profession::Herbalist
            | Profession::Chef
            | Profession::Blacksmith
            | Profession::Hunter
            | Profession::Alchemist,
        ) => Some(comp::agent::Mark::Merchant),
        Some(Profession::Guard) => Some(comp::agent::Mark::Guard),
        _ => None,
    }
}

fn get_npc_entity_info(
    npc: &Npc,
    sites: &Sites,
    index: IndexRef,
    time: Option<&(TimeOfDay, Calendar)>,
) -> EntityInfo {
    let pos = comp::Pos(npc.wpos);

    let mut rng = npc.rng(Npc::PERM_ENTITY_CONFIG);
    if let Some(profession) = npc.profession() {
        let economy = npc.home.and_then(|home| {
            let site = sites.get(home)?.world_site?;
            index.sites.get(site).trade_information(site)
        });

        let config_asset = humanoid_config(&profession);

        let entity_config = EntityConfig::from_asset_expect_owned(config_asset)
            .with_body(BodyBuilder::Exact(npc.body));
        EntityInfo::at(pos.0, &mut rng)
            .with_entity_config(entity_config, Some(config_asset), &mut rng, time)
            .with_alignment(
                if matches!(profession, Profession::Cultist | Profession::Pirate(_)) {
                    comp::Alignment::Enemy
                } else {
                    comp::Alignment::Npc
                },
            )
            .with_economy(economy.as_ref())
            .with_lazy_loadout(
                profession_extra_loadout(Some(&profession)),
                npc.seed.wrapping_add(Npc::PERM_LAZY_LOADOUT) as u64,
            )
            .with_alias(npc.get_name())
            .with_agent_mark(profession_agent_mark(Some(&profession)))
    } else {
        let config_asset = match npc.body {
            Body::BirdLarge(body) => match body.species {
                comp::bird_large::Species::Phoenix => "common.entity.wild.aggressive.phoenix",
                comp::bird_large::Species::Cockatrice => "common.entity.wild.aggressive.cockatrice",
                comp::bird_large::Species::Roc => "common.entity.wild.aggressive.roc",
                comp::bird_large::Species::CloudWyvern => {
                    "common.entity.wild.aggressive.cloudwyvern"
                },
                comp::bird_large::Species::FlameWyvern => {
                    "common.entity.wild.aggressive.flamewyvern"
                },
                comp::bird_large::Species::FrostWyvern => {
                    "common.entity.wild.aggressive.frostwyvern"
                },
                comp::bird_large::Species::SeaWyvern => "common.entity.wild.aggressive.seawyvern",
                comp::bird_large::Species::WealdWyvern => {
                    "common.entity.wild.aggressive.wealdwyvern"
                },
            },
            Body::BipedLarge(body) => match body.species {
                comp::biped_large::Species::Ogre => "common.entity.wild.aggressive.ogre",
                comp::biped_large::Species::Cyclops => "common.entity.wild.aggressive.cyclops",
                comp::biped_large::Species::Wendigo => "common.entity.wild.aggressive.wendigo",
                // bastion (NIGHT_HORROR, FR14): the wendigo-lineage stalker.
                comp::biped_large::Species::NightHorror => {
                    "common.entity.wild.aggressive.night_horror"
                },
                comp::biped_large::Species::Werewolf => "common.entity.wild.aggressive.werewolf",
                comp::biped_large::Species::Cavetroll => "common.entity.wild.aggressive.cave_troll",
                comp::biped_large::Species::Mountaintroll => {
                    "common.entity.wild.aggressive.mountain_troll"
                },
                comp::biped_large::Species::Swamptroll => {
                    "common.entity.wild.aggressive.swamp_troll"
                },
                comp::biped_large::Species::Blueoni => "common.entity.wild.aggressive.blue_oni",
                comp::biped_large::Species::Redoni => "common.entity.wild.aggressive.red_oni",
                comp::biped_large::Species::Tursus => "common.entity.wild.aggressive.tursus",
                comp::biped_large::Species::Gigasfrost => {
                    "common.entity.world.world_bosses.gigas_frost"
                },
                comp::biped_large::Species::Gigasfire => {
                    "common.entity.world.world_bosses.gigas_fire"
                },
                species => unimplemented!("rtsim spawning for {:?}", species),
            },
            body => unimplemented!("rtsim spawning for {:?}", body),
        };
        let entity_config = EntityConfig::from_asset_expect_owned(config_asset)
            .with_body(BodyBuilder::Exact(npc.body));

        EntityInfo::at(pos.0, &mut rng)
            .with_entity_config(entity_config, Some(config_asset), &mut rng, time)
    }
}

/// bastion (LOD-0, the save-back): the CANONICAL persistent record for a
/// loaded colonist — the live ECS `Colonist` comp plus the bag-slot
/// inventory snapshot in canonical form (sorted by id, duplicate stacks
/// merged, so record equality is state equality). ONE builder for the
/// per-tick mirror AND the demote flush (B17).
pub(crate) fn colonist_record(
    c: &comp::Colonist,
    inv: Option<&comp::Inventory>,
    needs: Option<&comp::bastion::Needs>,
    mood: Option<&comp::bastion::Mood>,
) -> common::bastion::BastionColonist {
    let mut rec = c.0.clone();
    // B7-0: mirror the live meters (same wholesale-Option semantics as
    // the bag below).
    rec.needs = needs.map(|n| (n.hunger, n.rest, n.recreation));
    rec.mood = mood.map(|m| m.0);
    rec.inventory = inv.map(|inv| {
        let mut items: Vec<(String, u32)> = Vec::new();
        for item in inv.slots().flatten() {
            if let Some(id) = item.item_definition_id().itemdef_id() {
                match items.iter_mut().find(|(i, _)| i == id) {
                    Some((_, n)) => *n += item.amount(),
                    None => items.push((id.to_string(), item.amount())),
                }
            }
        }
        items.sort();
        items
    });
    rec
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, Time>,
        Read<'a, TimeOfDay>,
        Read<'a, EventBus<CreateShipEvent>>,
        Read<'a, EventBus<CreateNpcEvent>>,
        Read<'a, EventBus<DeleteEvent>>,
        WriteExpect<'a, RtSim>,
        ReadExpect<'a, Arc<world::World>>,
        ReadExpect<'a, world::IndexOwned>,
        ReadExpect<'a, SlowJobPool>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, RtSimEntity>,
        WriteStorage<'a, comp::Agent>,
        ReadStorage<'a, Presence>,
        ReadExpect<'a, Calendar>,
        Read<'a, IdMaps>,
        ReadExpect<'a, ServerConstants>,
        ReadExpect<'a, WeatherGrid>,
        WriteStorage<'a, comp::Inventory>,
        WriteExpect<'a, comp::gizmos::RtsimGizmos>,
        ReadExpect<'a, comp::tool::AbilityMap>,
        ReadExpect<'a, comp::item::MaterialStatManifest>,
        // ★ THE PROMOTE MUST READ THE AUTHORITY THE DEMOTE READS. See
        // `bastion_may_promote_npc` — the demotion sweep in `Server::tick`
        // deletes on `TerrainGrid::get_key_real`, so the promotion has to
        // ask the same grid the same question or the two run open-loop
        // against each other. This is the only new resource the fix needs.
        ReadExpect<'a, common::terrain::TerrainGrid>,
        // bastion (B3): colonist decoration on promote; (B4) job ownership
        // gate for the controller sync.
        (
            WriteStorage<'a, comp::Colonist>,
            WriteStorage<'a, comp::PlayerColony>,
            WriteStorage<'a, comp::bastion::Needs>,
            WriteStorage<'a, comp::bastion::Mood>,
            WriteStorage<'a, comp::Stats>,
            ReadStorage<'a, comp::bastion::ActiveJob>,
            // bastion (B-ASSET1): test-goto fixtures own their activity too.
            ReadStorage<'a, comp::bastion::BastionTestGoto>,
            // Stage-1 B5.8: RTSim activity and action queues defer while the
            // route-owned off-mesh link has exclusive intent ownership.
            ReadStorage<'a, comp::bastion::BastionTraversalOwnership>,
            // bastion (B7-0): the cave-in fear queue lives on the board —
            // this system drains it into the chronicle (it owns the rtsim
            // data mutably; bastion_jobs holds a long-lived read guard).
            specs::Write<'a, crate::bastion_jobs::JobBoard>,
            // `APEX-T4.6` chunk 3b: `RtSim::save`'s staged-commit path
            // needs the character DB's directory for its own read-only
            // `VACUUM INTO` connection -- the same `Arc` `lib.rs`'s
            // construction already inserts for `CharacterUpdater`/
            // `CharacterLoader`, read here rather than a new resource.
            ReadExpect<'a, Arc<RwLock<DatabaseSettings>>>,
            // Entity-event-log stage 3 (colonist retention, 2026-08-10,
            // Opus's ruling -- retain every colonist, no significance
            // criterion at this population): needed only at the
            // freshly-promoted-colonist site below, not the wider system.
            ReadStorage<'a, Uid>,
        ),
    );

    const NAME: &'static str = "rtsim::tick";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            dt,
            time,
            time_of_day,
            create_ship_events,
            create_npc_events,
            delete_events,
            mut rtsim,
            world,
            index,
            slow_jobs,
            positions,
            rtsim_entities,
            mut agents,
            presences,
            calendar,
            id_maps,
            server_constants,
            weather_grid,
            mut inventories,
            rtsim_gizmos,
            ability_map,
            msm,
            terrain,
            (
                mut colonists,
                mut player_colony,
                mut bastion_needs,
                mut bastion_moods,
                mut stats_storage,
                bastion_active_jobs,
                bastion_test_gotos,
                bastion_traversal_ownerships,
                mut job_board,
                database_settings,
                uids,
            ),
        ): Self::SystemData,
    ) {
        let mut create_ship_emitter = create_ship_events.emitter();
        let mut create_npc_emitter = create_npc_events.emitter();
        let mut delete_emitter = delete_events.emitter();
        let rtsim = &mut *rtsim;
        let calendar_data = (*time_of_day, (*calendar).clone());

        // Set up rtsim inputs
        {
            let mut data = rtsim.state.data_mut();

            // Update time of day
            data.time_of_day = *time_of_day;

            // Update character map (i.e: so that rtsim knows where players are)
            // TODO: Other entities too like animals? Or do we now care about that?
            data.npcs.character_map.clear();
            for (presence, wpos) in (&presences, &positions).join() {
                if let PresenceKind::Character(character) = &presence.kind {
                    let chunk_pos = wpos.0.xy().as_().wpos_to_cpos();
                    data.npcs
                        .character_map
                        .entry(chunk_pos)
                        .or_default()
                        .push((*character, wpos.0));
                }
            }

            // bastion (IDLE-HOME-LEASH): resolve the colony's idle-orbit
            // anchor for the brain's colonist idle selector — a painted
            // Meeting zone overrides (explicit beats implicit), else the
            // FIRST stockpile's centroid, else None (leash inactive).
            // Recomputed from live designation state every tick, so an
            // erased zone/stockpile drops out on the next compute (no
            // stale-slot risk, registry B25 class) and the field never
            // needs persisting.
            data.bastion_home_anchor = job_board
                .activity_zones
                .iter()
                .find(|(_, kind, _)| matches!(kind, common::bastion::ZoneKind::Meeting))
                .map(|(_, _, region)| region)
                .or_else(|| job_board.stockpiles.first().map(|(_, region)| region))
                .map(|region| {
                    (region.min.map(|e| e as f32) + region.max.map(|e| e as f32)) * 0.5
                        + vek::Vec3::broadcast(0.5)
                });
        }

        // Tick rtsim
        // bastion (LOD-0): the system data is BOUND (not a dropped
        // temporary) so the inventory storage can be RECLAIMED from its
        // Mutex after the tick — the colonist save-back below snapshots
        // bag slots into the persistent rtsim record.
        let mut npc_system_data = NpcSystemData {
            positions: positions.clone(),
            id_maps,
            server_constants,
            weather_grid,
            inventories: Mutex::new(inventories),
            rtsim_gizmos,
            ability_map,
            msm,
        };
        rtsim.state.tick(
            &mut npc_system_data,
            &world,
            index.as_index_ref(),
            *time_of_day,
            *time,
            dt.0,
        );
        let mut inventories = npc_system_data
            .inventories
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // COLONY PERSISTENCE, THE SEED — once per server lifetime, and
        // BEFORE the save block below, so the first save of a restarted
        // world already carries the orders it is about to restore.
        //
        // This system can see the save but not the terrain, so it can only
        // hand the orders over; `place_designation` needs a `TerrainGrid`
        // and no chunks are loaded yet. The bastion tick drains the queue
        // once each region's terrain exists.
        if !job_board.restore_seeded {
            job_board.restore_seeded = true;
            let orders = rtsim.state.data().bastion_designations.clone();
            if !orders.is_empty() {
                tracing::info!(
                    orders = orders.len(),
                    "bastion: colony orders read from save, awaiting terrain to replay"
                );
                job_board.pending_restore = orders;
            }
        }

        // Perform a save if required
        if rtsim
            .last_saved
            .is_none_or(|ls| ls.elapsed() > Duration::from_secs(60))
        {
            // TODO: Use slow jobs
            let _ = slow_jobs;
            // `APEX-T4.6` chunk 3b: `db_dir` cloned out and the read
            // guard dropped before `save` (which takes `&mut rtsim`)
            // needs no lock held across the call.
            let db_dir = database_settings
                .read()
                .expect("DatabaseSettings RwLock was poisoned")
                .db_dir
                .clone();
            // COLONY PERSISTENCE: the colony's standing ORDERS join the
            // save. Written HERE, immediately before `save`, so the file
            // can never carry a stale set from an earlier tick.
            //
            // THE UNION IS DELIBERATE AND LOAD-BEARING. After a restart the
            // board is EMPTY until the restore finds its terrain, so saving
            // `designated` alone would overwrite the very orders being
            // restored with nothing — the save would eat them, and a
            // 60-second window is more than long enough for that to happen.
            // An order awaiting replay is still an order.
            //
            // The guard is dropped at the end of this statement — `save`
            // takes `&mut rtsim` and must not have a data lock held across
            // it (the same constraint `db_dir` is cloned out for above).
            let mut orders = job_board.designated.clone();
            orders.extend(job_board.pending_restore.iter().copied());
            rtsim.state.data_mut().bastion_designations = orders;
            rtsim.save(/* &slow_jobs, */ false, &db_dir);
        }

        let chunk_states = rtsim.state.resource::<ChunkStates>();
        // ROW 38: the world seed, copied out BEFORE the data guard is taken
        // — the settler drain below seeds its identity streams from
        // (world_seed, game_day, salt) and must not reach back through
        // `rtsim` while the data borrow is live.
        let bastion_world_seed = rtsim.world_seed;
        let data = &mut *rtsim.state.data_mut();

        // B7-0/B7-1 (the thought queue): drain what bastion_jobs queued
        // last tick into the chronicle — decaying THOUGHTS the mood
        // recompute reads (a direct Mood write would be overwritten
        // within a cadence). Stamped on the chronicle's own clock.
        // Emitters: cave-in fear, sleep quality.
        // DET-MOOD-003: drain the queued thoughts in a canonical total order
        // (source NPC id, cell x/y/z, kind) before recording. bastion_jobs
        // populates pending_thoughts during its per-colonist pass, and the
        // chronicle stamps a monotonic seq and cap-evicts the oldest band
        // entry on overflow — so the queue's push order would otherwise become
        // the authoritative, persisted chronicle seq / eviction order. Sorting
        // at the drain makes that order independent of the producer pass.
        // DET-MOOD-003: drain through the canonical-order helper (sorted by
        // source NPC id, cell x/y/z, kind) so the chronicle seq / eviction order
        // is independent of the producer pass. The ordering contract is
        // unit-tested in bastion_jobs (det_mood_003_tests).
        let pending_thoughts = crate::bastion_jobs::canonical_thought_drain_order(
            std::mem::take(&mut job_board.pending_thoughts),
        );
        for (re, pos, kind) in pending_thoughts {
            let now = data.time_of_day;
            data.chronicle.record(
                now,
                kind,
                vec![common::rtsim::Actor::Npc(re)],
                None,
                Some(pos),
                ::rtsim::data::Importance::Notable,
                ::rtsim::data::Scope::Colony,
                None,
            );
        }

        // ITEM 22 (relationships): apply queued co-work sentiment deltas —
        // the same one-tick deferral as thoughts (bastion_jobs can't write
        // rtsim data under its read guard). Sorted before applying: deltas
        // of one sign commute through change_by's saturation, but the
        // persisted BTreeMap insert order must not inherit producer pass
        // order (DET-MOOD-003's rule, applied at this second seam).
        let mut pending_sentiments = std::mem::take(&mut job_board.pending_sentiments);
        pending_sentiments
            .sort_by_key(|(subj, obj, _, _)| (*subj, *obj));
        for (subj, obj, change, cap) in pending_sentiments {
            if let Some(npc) = data.npcs.get_mut(subj) {
                npc.sentiments.toward_mut(obj).change_by(change, cap);
            }
        }

        // ── ROW 38: A SETTLER ARRIVES ──────────────────────────────────
        // The town's population could only ever fall: the founding count
        // was a one-shot cap and nothing since could add a soul. The
        // housing gate (bastion_jobs, beside the households census) decides
        // WHETHER and WHERE — an empty house while the colony's own drive
        // reads Expand — and queues the bed cell here. This drain is a DUMB
        // APPLIER: it makes no choices, exactly like the sentiment drain
        // above, because the producer holds the housing picture and this
        // side holds only the mutable rtsim data.
        //
        // The record is built like the SPAWN path, never the adoption path:
        // adoption omits `.personality`, which leaves every axis at MID and
        // makes trait-carrying impossible (measured: 0 of 348). Identity
        // and personality draw from SEPARATE salted streams so neither can
        // shift the other. The settler lands ON the vacant bed, where the
        // existing B7-2 assigner houses them on its next sweep — nothing
        // new decides where they live.
        //
        // ROW 50 FIX (review rank 8): the streams are keyed on `data.tick`,
        // not on the day index. The day index is BOOT-RELATIVE — the server
        // recomputes it from the session clock — so keying identity on it
        // meant a house vacated twice across a restart re-minted the DEAD
        // colonist byte-identically: same name, same face, same backstory,
        // walking back into the house he died in. `Data.tick` is a
        // persisted monotonic sim counter (rtsim/src/lib.rs:388), so it
        // never rewinds and never repeats, and it is a SIM clock — no
        // wall-clock enters a decision. The cell salt stays: it separates
        // two settlers minted on the same tick, which is what it was
        // actually good for.
        let settler_epoch = data.tick;
        let mut pending_immigrants = std::mem::take(&mut job_board.pending_immigrants);
        // ORDERED BEFORE APPLYING, exactly like the birth drain below. Today
        // the producer's `immigration_day` gate means this vector is never
        // longer than one, so the comparator never runs — it is here because a
        // DRAIN MUST NOT INHERIT ITS PRODUCER'S CAP. `data.spawn_npc` allocates
        // NpcIds in call order, and an NpcId is what `BastionColonist.parent`
        // persists forever; the first row that lifts the one-per-day cap would
        // otherwise hand two runs of one seed different lineages. The key is
        // the world cell, which is stable across restarts — deliberately NOT a
        // Uid, which is reassigned every time an npc is promoted to an entity.
        pending_immigrants.sort_by_key(|(cell, day, _)| (*day, cell.x, cell.y, cell.z));
        let settler_pin_trait = super::bastion_pin_trait();
        for (cell, day, prof_idx) in pending_immigrants {
            use common::rtsim::{Profession, Role};
            use rand::{RngExt as _, prelude::IndexedRandom};
            // The bed cell, so two settlers arriving on the same tick are still
            // different people — HASHED WITH the domain id, never XORed onto
            // it (see `bastion_drain_stream_salt`: the XOR made a settler and a
            // newborn six blocks apart the same person).
            let mut rng = ::rtsim::tick_rng(
                bastion_world_seed,
                settler_epoch,
                bastion_drain_stream_salt(BASTION_SETTLER_IDENTITY_DOMAIN, cell),
            );
            let mut personality_rng = ::rtsim::tick_rng(
                bastion_world_seed,
                settler_epoch,
                bastion_drain_stream_salt(BASTION_SETTLER_PERSONALITY_DOMAIN, cell),
            );
            let mut colonist = common::bastion::BastionColonist::generate(&mut rng);
            let species = *common::comp::humanoid::ALL_SPECIES
                .choose(&mut rng)
                .expect("humanoid species catalog must not be empty");
            let body = common::comp::Body::Humanoid(
                common::comp::humanoid::Body::random_with(&mut rng, &species),
            );
            // A settler arrives WITH A TRADE. An arrival with no lane is
            // the "colony of 32 employs the same 3 people" failure by
            // construction — they would not be stuck, they would be
            // unemployed. Same seeding as adoption: lane priorities plus
            // the starting XP that makes the lane real.
            //
            // ROW 50 FIX (review rank 2 — this one was killing colonists):
            // the map MUST mirror the founding path at server/src/rtsim/
            // mod.rs:696, which reads `Hunter | Guard => Guard`. The drain
            // had `_ => Haul`, so every arriving hunter got
            // `in_lane(Haul)` — and `in_lane` leaves guard at 3 (common/
            // src/bastion.rs:1986) while the muster, the alarm's run-toward
            // exemption and the night-watch roster all test guard >= 4.
            // Nothing anywhere writes work_priorities again, so a settler
            // was a permanent civilian: the town's defence froze at the
            // founding count while its population, its buildings and its
            // raid surface all grew. Guard is in the array too, so the town
            // can actually draw a soldier.
            let professions = [
                Profession::Farmer,
                Profession::Hunter,
                Profession::Blacksmith,
                Profession::Chef,
                Profession::Guard,
            ];
            let profession = professions[prof_idx % professions.len()];
            let work = common::bastion::WorkPriorities::work_for_profession(profession)
                .unwrap_or(common::bastion::WorkType::Haul);
            colonist.skills.grant_xp(work, common::bastion::ADOPTED_TRADE_XP);
            colonist.work_priorities = common::bastion::WorkPriorities::in_lane(work);
            let name = colonist.name.clone();
            let wpos = cell.map(|e| e as f32) + Vec3::new(0.5, 0.5, 1.0);
            let home = data
                .sites
                .iter()
                .min_by_key(|(_, site)| {
                    site.wpos
                        .map(|e| e as i64)
                        .distance_squared(wpos.xy().map(|e| e as i64))
                })
                .map(|(id, _)| id);
            let mut npc = ::rtsim::data::npc::Npc::new(
                rng.random(),
                wpos,
                body,
                Role::Civilised(Some(profession)),
            )
            .with_bastion_colonist(colonist);
            npc.home = home;
            // Through the ONE helper: this site used to call
            // `Personality::random` directly, so BASTION_PIN_TRAIT was inert
            // for every colonist the town grew rather than founded.
            npc.personality =
                super::bastion_colonist_personality(settler_pin_trait, &mut personality_rng);
            let carries_a_trait = super::bastion_carries_a_trait(&npc.personality);
            // spawn_npc, not create_npc: it registers the id into the
            // home site's population, so the world's own census counts
            // the newcomer too.
            let id = data.spawn_npc(npc);
            tracing::info!(
                ?id,
                name,
                ?wpos,
                ?profession,
                day,
                carries_a_trait,
                "bastion: ★ A SETTLER ARRIVES — a vacant house drew a new colonist"
            );
        }

        // ── ROW 50: CHILDREN ARE BORN ───────────────────────────────────
        // Same producer/drain contract as the settler above: bastion_jobs
        // decided WHETHER, WHOSE and WHERE while the world was readable;
        // this side only applies it, because `data_mut()` is legal in this
        // one place. Sorted before applying so two births queued in one
        // tick land in a fixed order regardless of how the producer's maps
        // iterated. As with the settler drain above, the producer's
        // `birth_day` gate means the vector is never longer than one today —
        // the sort is a guard against a future producer, and the key is the
        // world cell BECAUSE a Uid is reassigned on every promotion and would
        // reorder across a restart.
        let mut pending_births = std::mem::take(&mut job_board.pending_births);
        pending_births.sort_by_key(|(cell, day, _)| (*day, cell.x, cell.y, cell.z));
        let birth_epoch = data.tick;
        let birth_pin_trait = super::bastion_pin_trait();
        for (cell, day, parent_uid) in pending_births {
            use common::rtsim::Role;
            use rand::{RngExt as _, prelude::IndexedRandom};
            // The parent, as a durable rtsim id, resolved through the ECS
            // because a Uid only means anything for a loaded entity.
            //
            // ★ AND IT CAN FAIL, so it says so. This used to read "the parent
            // necessarily IS loaded: the producer read them out of a household
            // this same tick" — which is false, and false in a way that costs
            // the child its whole lineage. The producer picks the parent from
            // `HouseholdView.members`, and those are BED OWNERS: persistent
            // Uids written by the bed assigner, not an ECS join. A colonist who
            // has walked out to an unloaded chunk, or been demoted to
            // `SimulationMode::Simulated`, still owns their bed and is still
            // eligible to be named parent — while `uid_entity` has nothing to
            // return for them. The birth then proceeded with `parent: None` and
            // `Role::Civilised(None)`, i.e. a child with no lineage and no
            // heritage, in two fields nothing ever rewrites. TWO FRAMES
            // COMPARED AS ONE: persistent bed ownership against the loaded ECS.
            //
            // The birth is not refused — a town that loses a person because
            // their parent stepped outside is worse than a child with an
            // unknown parent — but it is WITNESSED, loudly, because a silent
            // `None` here is indistinguishable from an orphan by design.
            // `id_maps` moved into `npc_system_data` above; the struct is
            // only partially moved (its `inventories` field), so the map is
            // still readable here.
            let parent_id: Option<NpcId> = npc_system_data
                .id_maps
                .uid_entity(parent_uid)
                .and_then(|e| rtsim_entities.get(e))
                .copied();
            if parent_id.is_none() {
                tracing::warn!(
                    parent = %parent_uid,
                    ?cell,
                    day,
                    "bastion: A CHILD IS BORN WITHOUT A PARENT — the producer named a bed owner \
                     the ECS cannot resolve (unloaded or demoted), so this child gets no lineage \
                     and no heritage; the birth gate is counting a household in the persistent \
                     frame and resolving it in the loaded one"
                );
            }
            let parent_profession = parent_id
                .and_then(|id| data.npcs.get(id))
                .and_then(|n| match n.role {
                    Role::Civilised(p) => p,
                    _ => None,
                });
            let mut rng = ::rtsim::tick_rng(
                bastion_world_seed,
                birth_epoch,
                bastion_drain_stream_salt(BASTION_BIRTH_IDENTITY_DOMAIN, cell),
            );
            let mut personality_rng = ::rtsim::tick_rng(
                bastion_world_seed,
                birth_epoch,
                bastion_drain_stream_salt(BASTION_BIRTH_PERSONALITY_DOMAIN, cell),
            );
            let mut colonist = common::bastion::BastionColonist::generate(&mut rng);
            // ★ CHILDHOOD IS AN EMPTY WORK PROFILE, not a small body. The
            // claim loop's priority gate already reads "0" as "not this
            // person's work", so a child is out of the labour force
            // through the one door the town already has — while eating,
            // sleeping and the evening palette, which are need-minted
            // self-jobs, go on as normal. A child is a resident with no
            // trade, which is exactly what a child is.
            colonist.work_priorities = common::bastion::WorkPriorities::childhood();
            colonist.parent = parent_id;
            colonist.born_day = Some(day);
            // ★ THE GATE'S CLOCK: persistent and monotonic. `day` above is
            // boot-relative and resets to settings.world.start_time on every
            // restart, so it can only be a label, never a deadline.
            colonist.born_tick = Some(birth_epoch);
            let species = *common::comp::humanoid::ALL_SPECIES
                .choose(&mut rng)
                .expect("humanoid species catalog must not be empty");
            let body = common::comp::Body::Humanoid(
                common::comp::humanoid::Body::random_with(&mut rng, &species),
            );
            let name = colonist.name.clone();
            let wpos = cell.map(|e| e as f32) + Vec3::new(0.5, 0.5, 1.0);
            let home = data
                .sites
                .iter()
                .min_by_key(|(_, site)| {
                    site.wpos
                        .map(|e| e as i64)
                        .distance_squared(wpos.xy().map(|e| e as i64))
                })
                .map(|(id, _)| id);
            // ★ HERITAGE (Ben, chartered): the child carries no trade, but
            // it carries where it grew up. The role holds the parent's
            // profession as an INCLINATION — coming of age reads it as a
            // first preference, not as a destiny, so the smith's daughter
            // is likelier to smith and free not to.
            let mut npc = ::rtsim::data::npc::Npc::new(
                rng.random(),
                wpos,
                body,
                Role::Civilised(parent_profession),
            )
            .with_bastion_colonist(colonist);
            npc.home = home;
            // Through the ONE helper: this site used to call
            // `Personality::random` directly, so BASTION_PIN_TRAIT was inert
            // for every colonist the town bore rather than founded.
            npc.personality =
                super::bastion_colonist_personality(birth_pin_trait, &mut personality_rng);
            let carries_a_trait = super::bastion_carries_a_trait(&npc.personality);
            let id = data.spawn_npc(npc);
            // ChronicleKind::Birth has existed since the chronicle landed
            // and has never once been emitted. A town whose records hold
            // only deaths is not keeping a history, it is keeping a
            // casualty list.
            let now = data.time_of_day;
            let mut actors = vec![Actor::Npc(id)];
            if let Some(p) = parent_id {
                actors.push(Actor::Npc(p));
            }
            data.chronicle.record(
                now,
                ::rtsim::data::ChronicleKind::Birth,
                actors,
                home,
                Some(cell),
                ::rtsim::data::chronicle::Importance::Notable,
                ::rtsim::data::chronicle::Scope::Colony,
                None,
            );
            tracing::info!(
                ?id,
                name,
                ?wpos,
                ?parent_id,
                ?parent_profession,
                day,
                carries_a_trait,
                "bastion: ★ A CHILD IS BORN — the town makes its own people now"
            );
        }

        // Row 11, second faucet: the cork's footprint, read once (Copy).
        let bastion_settlement_bounds = job_board.settlement_bounds;
        let mut create_event = |id: NpcId, npc: &Npc, steering: Option<NpcBuilder>| match npc.body {
            Body::Ship(body) => {
                create_ship_emitter.emit(CreateShipEvent {
                    pos: comp::Pos(npc.wpos),
                    ori: comp::Ori::from(Dir::new(npc.dir.with_z(0.0))),
                    ship: body,
                    rtsim_entity: Some(id),
                    driver: steering,
                });
            },
            _ => {
                let entity_info = get_npc_entity_info(
                    npc,
                    &data.sites,
                    index.as_index_ref(),
                    Some(&calendar_data),
                );

                let (mut npc_builder, pos) = SpawnEntityData::from_entity_info(entity_info)
                    .into_npc_data_inner()
                    .expect("Entity loaded from assets cannot be special")
                    .to_npc_builder();

                // ★ THE SECOND FAUCET (bastion row 11): the chunk-
                // supplement cork held and the SAME fixed street
                // coordinate kept spawning — a resident rtsim monster
                // whose den predates the city, re-manifested through THIS
                // path every time its chunk loads. Same rule, same
                // bounds: hostile/wild rtsim residents inside the
                // settlement stay in rtsim (persistent, unmanifested);
                // citizens and visitors pass.
                // ★ PACED DANGER (see the terrain cork's twin): flat-world
                // Enemy manifestations end globally; raids own danger.
                if (matches!(npc_builder.alignment, comp::Alignment::Enemy)
                    || (matches!(npc_builder.alignment, comp::Alignment::Wild)
                        && npc_builder
                            .agent
                            .as_ref()
                            .is_some_and(|a| a.psyche.aggro_dist.is_some())))
                    && std::env::var_os("BASTION_FLAT_WORLD").is_some()
                {
                    // Predators are Wild (see the terrain twin).
                    return;
                }
                if matches!(
                    npc_builder.alignment,
                    comp::Alignment::Enemy | comp::Alignment::Wild
                ) && bastion_settlement_bounds.is_some_and(|(bmin, bmax)| {
                    let p = pos.0.xy().map(|e| e.floor() as i32);
                    p.x >= bmin.x && p.y >= bmin.y && p.x <= bmax.x && p.y <= bmax.y
                }) {
                    return;
                }

                if let Some(agent) = &mut npc_builder.agent {
                    agent.rtsim_outbox = Some(Default::default());
                }

                if let Some(health) = &mut npc_builder.health {
                    health.set_fraction(npc.health_fraction);
                }

                create_npc_emitter.emit(CreateNpcEvent {
                    pos,
                    ori: comp::Ori::from(Dir::new(npc.dir.with_z(0.0))),
                    npc: npc_builder.with_rtsim(id).with_rider(steering),
                });
            },
        };

        // ★ THE SPIN COUNTER (see `bastion_may_promote_npc`): how many
        // promotions the terrain refused this tick that the rtsim chunk cache
        // would have allowed. On a healthy world this is zero every tick and
        // costs one comparison; on the world that spun it would have read 2
        // every tick for 33 minutes and named the chunk in one leg instead of
        // two million log lines that name nothing.
        let mut promote_refused_stale_chunk = 0usize;
        let mut promote_refused_sample: Option<Vec2<i32>> = None;

        // Load in mounted npcs and their riders
        for mount in data.npcs.mounts.iter_mounts() {
            let mount_npc = data.npcs.npcs.get_mut(mount).expect("This should exist");
            let chunk = bastion_npc_chunk_key(mount_npc.wpos);

            if bastion_may_promote_npc(
                matches!(mount_npc.mode, SimulationMode::Simulated),
                chunk_states.0.get(chunk).is_some_and(|c| c.is_some()),
                terrain.get_key_real(chunk).is_some(),
                false,
            ) {
                mount_npc.mode = SimulationMode::Loaded;

                let mut actor_info = |actor: Actor| {
                    let npc_id = actor.npc()?;
                    let npc = data.npcs.npcs.get_mut(npc_id)?;
                    if matches!(npc.mode, SimulationMode::Simulated) {
                        npc.mode = SimulationMode::Loaded;
                        let entity_info = get_npc_entity_info(
                            npc,
                            &data.sites,
                            index.as_index_ref(),
                            Some(&calendar_data),
                        );

                        let mut npc_builder = SpawnEntityData::from_entity_info(entity_info)
                            .into_npc_data_inner()
                            // EntityConfig can't represent Waypoints at all
                            // as of now, and if someone will try to spawn
                            // rtsim waypoint it is definitely error.
                            .expect("Entity loaded from assets cannot be special")
                            .to_npc_builder()
                            .0
                            .with_rtsim(npc_id);

                        if let Some(agent) = &mut npc_builder.agent {
                            agent.rtsim_outbox = Some(Default::default());
                        }

                        Some(npc_builder)
                    } else {
                        error!("Npc is loaded but vehicle is unloaded");
                        None
                    }
                };

                let steerer = data
                    .npcs
                    .mounts
                    .get_steerer_link(mount)
                    .and_then(|link| actor_info(link.rider));

                let mount_npc = data.npcs.npcs.get(mount).expect("This should exist");
                create_event(mount, mount_npc, steerer);
            }
        }

        // Load in NPCs
        for (npc_id, npc) in data.npcs.npcs.iter_mut() {
            let chunk = bastion_npc_chunk_key(npc.wpos);

            // Load the NPC into the world if it's in a loaded chunk and is not already
            // loaded. ★ "A loaded chunk" now means what the DELETION SWEEP means by
            // it — see `bastion_may_promote_npc` for the 38-transitions-per-second
            // limit cycle the old, cache-only test produced.
            let is_simulated = matches!(npc.mode, SimulationMode::Simulated);
            let cache_says_loaded = chunk_states.0.get(chunk).is_some_and(|c| c.is_some());
            let terrain_says_real = terrain.get_key_real(chunk).is_some();
            // Riding npcs will be spawned by the vehicle.
            let is_mounted = data.npcs.mounts.get_mount_link(npc_id).is_some();
            if bastion_may_promote_npc(
                is_simulated,
                cache_says_loaded,
                terrain_says_real,
                is_mounted,
            ) {
                npc.mode = SimulationMode::Loaded;
                create_event(npc_id, npc, None);
            } else if is_simulated && cache_says_loaded && !terrain_says_real && !is_mounted {
                // The exact disagreement that spun the world: the rtsim chunk
                // cache says this npc's chunk is loaded and the terrain says it
                // does not exist. Counted, not logged per npc — a witness that
                // printed here would reproduce the two-million-line flood it was
                // written to replace.
                promote_refused_stale_chunk += 1;
                promote_refused_sample.get_or_insert(chunk);
            }
        }
        if promote_refused_stale_chunk > 0 {
            tracing::warn!(
                refused = promote_refused_stale_chunk,
                sample_chunk = ?promote_refused_sample,
                "bastion: PROMOTE REFUSED — the rtsim ChunkStates cache says these npcs' chunks \
                 are loaded and the TerrainGrid says they are not. The promotion is held (it \
                 would have been reversed by the entity-cleanup sweep in the same tick); the \
                 cache is stale for the sampled chunk"
            );
        }

        // Synchronise rtsim NPC with entity data
        for (entity, pos, rtsim_entity, agent) in (
            &entities,
            &positions,
            &rtsim_entities,
            (&mut agents).maybe(),
        )
            .join()
        {
            if let Some(npc) = data.npcs.get_mut(*rtsim_entity) {
                match npc.mode {
                    SimulationMode::Loaded => {
                        // bastion (B3): decorate a freshly-promoted colonist —
                        // mirror the rtsim record into ECS comps once, and
                        // override the display name so the roster and the
                        // in-world nametag agree.
                        if let Some(colonist) = &npc.bastion_colonist
                            && !colonists.contains(entity)
                        {
                            // ★ ITEM 14 axis 2: pin on the ECS MIRROR, which is
                            // what the flee site reads (`colonists.get(entity)`).
                            // The rtsim record behind `colonist` is a `&` here,
                            // and pinning it would also outlive the fixture by
                            // persisting into the save — a fixture lever must
                            // not rewrite the world's stored state.
                            let mut mirrored = colonist.clone();
                            if let Some((timid, brave)) =
                                bastion_server::bastion_jobs::guard_bravery_pins()
                            {
                                let n = PINNED_SO_FAR
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                // ★ ALTERNATE, don't pin only the first.
                                // Leg 3 wounded guards to 0.5 and every
                                // evaluation read bravery=0.8: the ONE timid
                                // colonist happened to be the only guard
                                // claimant, so the brave contrast never
                                // existed. Which colonist CLAIMS a guard job is
                                // independent of promotion order, so pinning
                                // "the first" cannot guarantee both values
                                // reach the guard population. Alternating puts
                                // half the colony at each value.
                                mirrored.guard_bravery =
                                    if n % 2 == 0 { timid } else { brave };
                                tracing::info!(
                                    name = mirrored.name.as_str(),
                                    guard_bravery = mirrored.guard_bravery,
                                    nth = n,
                                    "bastion: ITEM 14 axis 2 -- guard_bravery PINNED at promotion"
                                );
                            }
                            let _ = colonists.insert(entity, comp::Colonist(mirrored));
                            // Entity-event-log stage 3 (2026-08-10, Opus's
                            // ruling): retain every colonist -- no
                            // significance criterion at this population (a
                            // colony's `count` is single digits, not the
                            // thousands-scale item/ambient-NPC problem
                            // retention exists to solve). This exact guard
                            // (`!colonists.contains(entity)` above) already
                            // fires exactly once per colonist's true first
                            // promote, so `retain()`'s own idempotence is a
                            // belt-and-braces guard, not the thing doing the
                            // work here.
                            if let Some(uid) = uids.get(entity).copied() {
                                bastion_server::bastion_entity_event_log::retain(uid);
                            }
                            let _ = player_colony.insert(entity, comp::PlayerColony);
                            let _ = bastion_needs.insert(
                                entity,
                                // B7-0: RESTORE the persisted meters —
                                // wholesale-replace like the bag (None =
                                // a genuine first promote keeps defaults).
                                colonist.needs.map_or_else(
                                    comp::bastion::Needs::default,
                                    |(hunger, rest, recreation)| comp::bastion::Needs {
                                        hunger,
                                        rest,
                                        recreation,
                                    },
                                ),
                            );
                            let _ = bastion_moods.insert(
                                entity,
                                colonist
                                    .mood
                                    .map_or_else(comp::bastion::Mood::default, comp::bastion::Mood),
                            );
                            if let Some(mut stats) = stats_storage.get_mut(entity) {
                                stats.name = comp::Content::Plain(colonist.name.clone());
                            }
                            // LOD-0: RESTORE the persisted bag — REPLACE,
                            // don't add: a re-created entity rolls a FRESH
                            // random spawn loadout, and restoring on top of
                            // it DOUBLES food/coins (the first scenario
                            // run's dupe). `None` = never captured (a
                            // genuine first promote) → keep the spawn
                            // default; `Some` (even empty) = the truth,
                            // wholesale. An id that no longer resolves
                            // degrades to a warn (old save, renamed asset),
                            // never a panic.
                            if let Some(persisted) = &colonist.inventory
                                && let Some(mut inv) = inventories.get_mut(entity)
                            {
                                // InvSlotId is ECS-lifetime only; never retain it across demotion.
                                inv.drain().for_each(drop);
                                for (id, amount) in persisted {
                                    match comp::Item::new_from_asset(id) {
                                        Ok(mut item) => {
                                            let n = (*amount).max(1);
                                            if n > 1 && item.set_amount(n).is_err() {
                                                // Non-stackable: push singles.
                                                for _ in 1..n {
                                                    if let Ok(extra) =
                                                        comp::Item::new_from_asset(id)
                                                    {
                                                        let _ = inv.push(extra);
                                                    }
                                                }
                                            }
                                            let _ = inv.push(item);
                                        },
                                        Err(e) => tracing::warn!(
                                            id = id.as_str(),
                                            ?e,
                                            "bastion LOD-0: persisted item id no longer resolves \
                                             — dropped on promote"
                                        ),
                                    }
                                }
                            }
                            tracing::info!(
                                name = colonist.name.as_str(),
                                "bastion: colonist promoted to loaded entity"
                            );
                        }
                        // Update rtsim NPC state
                        npc.wpos = pos.0;
                        // bastion (LOD-0, the save-back): mirror the LIVE
                        // colonist state into the persistent rtsim record
                        // EVERY loaded tick — the ECS comp was a one-time
                        // CLONE, so XP/inventory mutations never reached
                        // the record and a leveled colonist came back
                        // DE-LEVELED after unload or save/reload (registry
                        // B11). With the record save-ready every tick,
                        // demotion and periodic rtsim saves lose nothing.
                        if let Some(c) = colonists.get(entity) {
                            npc.bastion_colonist = Some(colonist_record(
                                c,
                                inventories.get(entity),
                                bastion_needs.get(entity),
                                bastion_moods.get(entity),
                            ));
                        }

                        // Update entity state
                        if let Some(agent) = agent {
                            agent.rtsim_controller.personality = npc.personality;
                            agent.rtsim_controller.look_dir = npc.controller.look_dir;
                            // bastion (B4): while a colonist works a job, the
                            // job system owns its activity — the rtsim brain
                            // must not clobber the travel intent. (B-ASSET1):
                            // same for test-goto fixture orders.
                            let link_owned = bastion_traversal_ownerships
                                .get(entity)
                                .is_some_and(|ownership| ownership.mode.owns_movement_intent());
                            if !link_owned
                                && !bastion_active_jobs.contains(entity)
                                && !bastion_test_gotos.contains(entity)
                            {
                                agent.rtsim_controller.activity = npc.controller.activity;
                            }
                            if !link_owned {
                                agent
                                    .rtsim_controller
                                    .actions
                                    .extend(std::mem::take(&mut npc.controller.actions));
                            }
                            if let Some(rtsim_outbox) = &mut agent.rtsim_outbox {
                                npc.inbox.append(rtsim_outbox);
                            }
                        }
                    },
                    SimulationMode::Simulated => {
                        // bastion (LOD-0): the DEMOTE FLUSH — the entity is
                        // about to be deleted; capture its final state into
                        // the persistent record (the per-tick mirror covers
                        // every earlier tick; this covers the last one).
                        if let Some(c) = colonists.get(entity) {
                            npc.bastion_colonist = Some(colonist_record(
                                c,
                                inventories.get(entity),
                                bastion_needs.get(entity),
                                bastion_moods.get(entity),
                            ));
                        }
                        delete_emitter.emit(DeleteEvent(entity));
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod bastion_promotion_pins {
    use super::*;

    /// ★ THE INVARIANT, SWEPT OVER THE WHOLE INPUT SPACE. A promotion that the
    /// deletion sweep would reverse in the same tick is an oscillator, and the
    /// live world ran one at ~38 transitions per second for 33 minutes. There
    /// are only 16 states here, so this is not a sample: it is the whole space.
    ///
    /// PLANT-AND-PROVE: this is the shipped defect written out. Drop
    /// `terrain_chunk_real` from `bastion_may_promote_npc` (i.e. restore
    /// `is_simulated && rtsim_chunk_state_loaded && !is_mounted`, the cache-only
    /// test) and the row `(simulated=true, cache=true, terrain=false,
    /// mounted=false)` fails immediately with the two colonists' exact
    /// situation named in the message.
    #[test]
    fn a_promotion_the_sweep_would_reverse_is_never_permitted() {
        for is_simulated in [false, true] {
            for cache in [false, true] {
                for terrain_real in [false, true] {
                    for mounted in [false, true] {
                        let promotes =
                            bastion_may_promote_npc(is_simulated, cache, terrain_real, mounted);
                        let deletes = bastion_sweep_would_delete(terrain_real);
                        assert!(
                            !(promotes && deletes),
                            "promote and demote disagree at (simulated={is_simulated}, \
                             chunk_states={cache}, terrain_real={terrain_real}, \
                             mounted={mounted}) — this npc would be spawned and deleted in the \
                             same server tick, for ever"
                        );
                    }
                }
            }
        }
    }

    /// ★ WHAT HAPPENS WHEN A PROMOTED ENTITY'S CHUNK IS ABSENT FROM THE
    /// TERRAIN GRID — the question the fix has to answer out loud, because the
    /// wrong answer (keep it loaded anyway) would be a worse defect than the
    /// spin: an entity standing in a chunk that does not exist.
    ///
    /// The answer is that the state is UNREACHABLE, not tolerated. The demote
    /// is untouched — it still deletes on `terrain_chunk_real == false`,
    /// immediately, exactly as vanilla does — and the promote simply never
    /// creates that entity in the first place. So the only steady state for a
    /// chunkless npc is `Simulated`, which is what `Simulated` is for.
    #[test]
    fn a_chunkless_npc_is_never_promoted_and_is_always_swept() {
        assert!(
            !bastion_may_promote_npc(true, true, false, false),
            "an npc whose chunk the terrain does not have must not be given an entity"
        );
        assert!(
            bastion_sweep_would_delete(false),
            "and if one somehow exists, the sweep must still take it — the fix must not have \
             taught anything to keep it"
        );
    }

    /// The two sides must agree on WHICH CHUNK, not merely on the question.
    /// `as_::<i32>()` truncates toward zero and `.floor() as i32` does not, so
    /// they part company for every block in `[-1, 0)` — a colony west or north
    /// of the origin would have had the promote testing one chunk while the
    /// sweep tested its neighbour.
    ///
    /// PLANT-AND-PROVE: put `wpos.xy().as_::<i32>().wpos_to_cpos()` back in
    /// `bastion_npc_chunk_key` and the negative rows fail.
    #[test]
    fn the_promote_and_the_sweep_round_the_same_way() {
        // The sweep's rule, transcribed from `Server::tick`:
        // `terrain.pos_key(pos.0.map(|e| e.floor() as i32))`.
        let sweep_key =
            |wpos: Vec3<f32>| wpos.xy().map(|e| e.floor() as i32).wpos_to_cpos();
        for x in [-64.5f32, -32.0, -1.5, -0.5, -0.001, 0.0, 0.5, 31.9, 32.0, 1000.25] {
            for y in [-33.75f32, -0.25, 0.0, 17.5, 64.0] {
                let wpos = Vec3::new(x, y, 42.0);
                assert_eq!(
                    bastion_npc_chunk_key(wpos),
                    sweep_key(wpos),
                    "promote and sweep disagree about which chunk holds {wpos:?}"
                );
            }
        }
        // And the disagreement it removes is real, not theoretical: the old
        // truncating rule really does name a different chunk here.
        let wpos = Vec3::new(-0.5f32, -0.5, 0.0);
        assert_ne!(
            wpos.xy().as_::<i32>().wpos_to_cpos(),
            bastion_npc_chunk_key(wpos),
            "if these ever agree, this pin has stopped testing anything"
        );
    }

    /// ★ THE PIN THAT PROTECTS AGAINST THE CLASS RETURNING: a colonist sitting
    /// EXACTLY on the boundary. The chunk under it is the thing that flickers —
    /// terrain generates in a ring wider than it retains, so an annulus of
    /// chunks is created on demand and dropped on the 16-tick unload scan,
    /// forever — and the rtsim question is what the colonist does about it.
    ///
    /// The answer this fix commits to: the colonist tracks its chunk EXACTLY,
    /// one transition per chunk transition, and never more. Walking 80 ticks of
    /// a chunk that flickers every 16 ticks must give 5 promotions — one per
    /// appearance — not 40. Under the shipped cache-only test the promote fires
    /// on every tick the stale cache still says "loaded", which is what the 2:1
    /// group-alternation and the 3,544-cycles-in-34-minutes measurement show.
    ///
    /// PLANT-AND-PROVE: drop `terrain_chunk_real` from the promote and this
    /// reads 40 promotions against a chunk that only existed 5 times.
    #[test]
    fn a_colonist_tracks_its_chunk_exactly_once_per_flicker() {
        let mut is_simulated = true;
        let mut promotions = 0u64;
        let mut appearances = 0u64;
        let ticks = 80u64;
        let mut was_real = false;
        for now in 0..ticks {
            // The annulus: the chunk exists for 16 ticks, then not, then again.
            let terrain_real = (now / 16) % 2 == 0;
            // The stale cache the old promote trusted: it lags a tick behind the
            // terrain on the way down, which is the spurious-promote window.
            let cache_says_loaded = terrain_real || (now % 16 != 0);
            if terrain_real && !was_real {
                appearances += 1;
            }
            was_real = terrain_real;
            if bastion_may_promote_npc(is_simulated, cache_says_loaded, terrain_real, false) {
                is_simulated = false;
                promotions += 1;
            }
            // The sweep, later in the same tick.
            if !is_simulated && bastion_sweep_would_delete(terrain_real) {
                is_simulated = true;
            }
        }
        assert_eq!(
            promotions, appearances,
            "the colonist promoted {promotions} times for {appearances} appearances of its \
             chunk — anything above 1:1 is the spin"
        );
        assert!(
            promotions > 0,
            "and it must still be promoted when its chunk really is there — a guard that \
             starves the thing it protects is not a fix"
        );
    }

    /// DIRECTION 1 — IT PROMOTES WHEN IT SHOULD. The fix must not be a guard
    /// that starves the thing it protects: a simulated, unmounted npc standing
    /// in a chunk BOTH authorities agree is loaded still promotes, which is the
    /// entire normal path (a player walks back into town and the colony comes
    /// alive).
    #[test]
    fn an_agreed_loaded_chunk_still_promotes() {
        assert!(bastion_may_promote_npc(true, true, true, false));
        assert!(!bastion_sweep_would_delete(true));
    }

    /// DIRECTION 2 — IT DOES NOT PROMOTE WHEN IT SHOULD NOT, and each refusal
    /// names its own reason, so a future edit cannot delete one conjunct and
    /// still pass. The third row is the live defect: the cache says loaded, the
    /// terrain says the chunk does not exist.
    #[test]
    fn each_refusal_has_its_own_reason() {
        // Already loaded — nothing to promote.
        assert!(!bastion_may_promote_npc(false, true, true, false));
        // The rtsim cache has not seen this chunk load (pre-existing rule).
        assert!(!bastion_may_promote_npc(true, false, true, false));
        // ★ THE ONE THAT SPUN: stale cache against a chunk the terrain does not
        // have. The sweep would delete it in the same tick.
        assert!(!bastion_may_promote_npc(true, true, false, false));
        assert!(bastion_sweep_would_delete(false));
        // Riders are spawned by their vehicle, never independently.
        assert!(!bastion_may_promote_npc(true, true, true, true));
    }

    /// THE DEAD-BAND, STATED AS A SEQUENCE rather than as a number. There is no
    /// cooldown and none is needed: the two directions are now the same test on
    /// the same grid, so the state that used to cycle is simply STABLE. Walking
    /// the disagreement state forward twenty times must produce twenty
    /// refusals and zero transitions — under the shipped cache-only test the
    /// same twenty steps are twenty promote/delete pairs.
    #[test]
    fn the_disagreement_state_is_stable_not_oscillating() {
        let (cache_loaded, terrain_real) = (true, false);
        let mut transitions = 0u32;
        let mut is_simulated = true;
        for _ in 0..20 {
            if bastion_may_promote_npc(is_simulated, cache_loaded, terrain_real, false) {
                is_simulated = false;
                transitions += 1;
            }
            // The sweep, later in the same tick.
            if !is_simulated && bastion_sweep_would_delete(terrain_real) {
                is_simulated = true;
                transitions += 1;
            }
        }
        assert_eq!(
            transitions, 0,
            "the promote/demote pair is still an oscillator — this is the 38-per-second spin"
        );
        assert!(is_simulated, "the npc must simply stay simulated");
    }
}

#[cfg(test)]
mod bastion_population_drain_pins {
    use super::*;

    /// The exact structural collision the review found, pinned as a pair of
    /// real town cells rather than as an abstract property.
    ///
    /// Under the shipped `DOMAIN ^ cell_salt` these two salts were EQUAL:
    /// `cell_salt(27400, 18320, 40)` has its low three bits clear, so
    /// `cell_salt(27400, 18320, 46)` is that salt `+ 6`, which for those bits
    /// is also that salt `^ 6` — and `0xBA57_C013 ^ 0xBA57_C015 == 6` cancels
    /// it exactly. A settler bed on the ground floor and a birth corner six
    /// blocks up therefore drew the SAME name, backstory, species and face out
    /// of `BastionColonist::generate` on the same `data.tick`.
    ///
    /// PLANT-AND-PROVE: replace the body of `bastion_drain_stream_salt` with
    /// `domain ^ (cell.x as u32).wrapping_mul(0x9E37_79B9)`
    /// `.wrapping_add((cell.y as u32).wrapping_mul(0x85EB_CA6B))`
    /// `.wrapping_add(cell.z as u32)` and this assertion fails on its first
    /// line.
    #[test]
    fn a_settler_and_a_newborn_six_blocks_apart_are_not_the_same_person() {
        let settler_bed = Vec3::new(27400, 18320, 40);
        let birth_corner = Vec3::new(27400, 18320, 46);
        assert_ne!(
            bastion_drain_stream_salt(BASTION_SETTLER_IDENTITY_DOMAIN, settler_bed),
            bastion_drain_stream_salt(BASTION_BIRTH_IDENTITY_DOMAIN, birth_corner),
            "the settler identity stream and the birth identity stream aliased — this is the \
             XOR-onto-the-domain-id defect, and the two colonists minted on this tick are the \
             same person"
        );
        // The origin cell makes the same point with no arithmetic at all:
        // the old salt of (0,0,0) was 0, so `0xBA57_C013 ^ 0` and
        // `0xBA57_C015 ^ 6` were both literally 0xBA57_C013.
        assert_ne!(
            bastion_drain_stream_salt(BASTION_SETTLER_IDENTITY_DOMAIN, Vec3::new(0, 0, 0)),
            bastion_drain_stream_salt(BASTION_BIRTH_IDENTITY_DOMAIN, Vec3::new(0, 0, 6)),
        );
    }

    /// The property behind the case above: over a swept block of cells, no two
    /// distinct `(domain, cell)` pairs may share a stream. Sweeps all four
    /// drain domains, so the settler's own identity-vs-personality pair (which
    /// aliased at an XOR distance of 7) is covered too.
    ///
    /// A LOOP, NOT A LITERAL: a pin over one hand-picked pair certifies one
    /// hand-picked pair. Under the shipped XOR this sweep collides on its first
    /// cross-domain cell; the panic names the colliding pair so the failure is
    /// diagnosable rather than merely red.
    #[test]
    fn no_two_drain_streams_alias_across_a_swept_town_block() {
        let domains = [
            BASTION_SETTLER_IDENTITY_DOMAIN,
            BASTION_SETTLER_PERSONALITY_DOMAIN,
            BASTION_BIRTH_IDENTITY_DOMAIN,
            BASTION_BIRTH_PERSONALITY_DOMAIN,
        ];
        let mut seen: std::collections::HashMap<u32, (u32, Vec3<i32>)> =
            std::collections::HashMap::new();
        let mut swept = 0usize;
        for domain in domains {
            for x in 27400..27404 {
                for y in 18320..18324 {
                    for z in 40..56 {
                        let cell = Vec3::new(x, y, z);
                        let salt = bastion_drain_stream_salt(domain, cell);
                        swept += 1;
                        if let Some((other_domain, other_cell)) = seen.insert(salt, (domain, cell))
                        {
                            panic!(
                                "stream {salt:#010x} is shared by (domain {domain:#010x}, \
                                 {cell:?}) and (domain {other_domain:#010x}, {other_cell:?}) — \
                                 two colonists minted on one tick would be the same person"
                            );
                        }
                    }
                }
            }
        }
        assert_eq!(
            swept,
            4 * 4 * 4 * 16,
            "the sweep itself must cover what it claims to"
        );
        assert_eq!(
            seen.len(),
            swept,
            "every swept (domain, cell) must own its own stream"
        );
    }

    /// The salt is a PURE function of `(domain, cell)` — the same world, tick
    /// and cell must mint the same colonist after a reload. A witness for the
    /// determinism claim the doc comment makes, not a restatement of it.
    #[test]
    fn the_drain_salt_is_reproducible_and_position_sensitive() {
        let cell = Vec3::new(-1337, 42, 7);
        assert_eq!(
            bastion_drain_stream_salt(BASTION_BIRTH_PERSONALITY_DOMAIN, cell),
            bastion_drain_stream_salt(BASTION_BIRTH_PERSONALITY_DOMAIN, cell),
        );
        // Negative coordinates are ordinary world coordinates, not an edge
        // case to be excluded: half the map has them.
        assert_ne!(
            bastion_drain_stream_salt(BASTION_BIRTH_PERSONALITY_DOMAIN, cell),
            bastion_drain_stream_salt(BASTION_BIRTH_PERSONALITY_DOMAIN, Vec3::new(1337, 42, 7)),
        );
    }
}
