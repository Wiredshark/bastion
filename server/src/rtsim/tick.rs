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
        let pending_immigrants = std::mem::take(&mut job_board.pending_immigrants);
        for (cell, day, prof_idx) in pending_immigrants {
            use common::rtsim::{Profession, Role};
            use rand::{RngExt as _, prelude::IndexedRandom};
            // u32 salt (tick_rng's own width): the bed cell, so two settlers
            // arriving on the same tick are still different people.
            let cell_salt = (cell.x as u32)
                .wrapping_mul(0x9E37_79B9)
                .wrapping_add((cell.y as u32).wrapping_mul(0x85EB_CA6B))
                .wrapping_add(cell.z as u32);
            let mut rng = ::rtsim::tick_rng(
                bastion_world_seed,
                settler_epoch,
                0xBA57_C013u32 ^ cell_salt,
            );
            let mut personality_rng = ::rtsim::tick_rng(
                bastion_world_seed,
                settler_epoch,
                0xBA57_C014u32 ^ cell_salt,
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
            npc.personality = common::rtsim::Personality::random(&mut personality_rng);
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
                "bastion: ★ A SETTLER ARRIVES — a vacant house drew a new colonist"
            );
        }

        // ── ROW 50: CHILDREN ARE BORN ───────────────────────────────────
        // Same producer/drain contract as the settler above: bastion_jobs
        // decided WHETHER, WHOSE and WHERE while the world was readable;
        // this side only applies it, because `data_mut()` is legal in this
        // one place. Sorted before applying so two births queued in one
        // tick land in a fixed order regardless of how the producer's maps
        // iterated.
        let mut pending_births = std::mem::take(&mut job_board.pending_births);
        pending_births.sort_by_key(|(cell, day, parent)| {
            (*day, cell.x, cell.y, cell.z, parent.0)
        });
        let birth_epoch = data.tick;
        for (cell, day, parent_uid) in pending_births {
            use common::rtsim::Role;
            use rand::{RngExt as _, prelude::IndexedRandom};
            // The parent, as a durable rtsim id. Resolved through the ECS
            // because a Uid only means anything for a loaded entity — and
            // the parent necessarily IS loaded: the producer read them out
            // of a household this same tick.
            // `id_maps` moved into `npc_system_data` above; the struct is
            // only partially moved (its `inventories` field), so the map is
            // still readable here.
            let parent_id: Option<NpcId> = npc_system_data
                .id_maps
                .uid_entity(parent_uid)
                .and_then(|e| rtsim_entities.get(e))
                .copied();
            let parent_profession = parent_id
                .and_then(|id| data.npcs.get(id))
                .and_then(|n| match n.role {
                    Role::Civilised(p) => p,
                    _ => None,
                });
            let cell_salt = (cell.x as u32)
                .wrapping_mul(0x9E37_79B9)
                .wrapping_add((cell.y as u32).wrapping_mul(0x85EB_CA6B))
                .wrapping_add(cell.z as u32);
            let mut rng =
                ::rtsim::tick_rng(bastion_world_seed, birth_epoch, 0xBA57_C015u32 ^ cell_salt);
            let mut personality_rng =
                ::rtsim::tick_rng(bastion_world_seed, birth_epoch, 0xBA57_C016u32 ^ cell_salt);
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
            npc.personality = common::rtsim::Personality::random(&mut personality_rng);
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

        // Load in mounted npcs and their riders
        for mount in data.npcs.mounts.iter_mounts() {
            let mount_npc = data.npcs.npcs.get_mut(mount).expect("This should exist");
            let chunk = mount_npc.wpos.xy().as_::<i32>().wpos_to_cpos();

            if matches!(mount_npc.mode, SimulationMode::Simulated)
                && chunk_states.0.get(chunk).is_some_and(|c| c.is_some())
            {
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
            let chunk = npc.wpos.xy().as_::<i32>().wpos_to_cpos();

            // Load the NPC into the world if it's in a loaded chunk and is not already
            // loaded
            if matches!(npc.mode, SimulationMode::Simulated)
                && chunk_states.0.get(chunk).is_some_and(|c| c.is_some())
                // Riding npcs will be spawned by the vehicle.
                && data.npcs.mounts.get_mount_link(npc_id).is_none()
            {
                npc.mode = SimulationMode::Loaded;
                create_event(npc_id, npc, None);
            }
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
