use super::*;
use crate::{ServerConstants, sys::terrain::SpawnEntityData};
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
    uid::IdMaps,
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
    sync::{Arc, Mutex},
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
    arbiter: Option<&comp::bastion::Arbiter>,
    now: f64,
) -> common::bastion::BastionColonist {
    let mut rec = c.0.clone();
    // B7-0: mirror the live meters (same wholesale-Option semantics as
    // the bag below).
    rec.needs = needs.map(|n| (n.hunger, n.rest, n.recreation));
    rec.mood = mood.map(|m| m.0);
    // DET-COL-AUT-002: mirror the arbiter drive, storing the commitment as a
    // REMAINING duration (never the absolute `committed_until` sim-Time) so it
    // survives a reload where the clock resets or jumps. `last_scores` and
    // `activity` are REPORTED telemetry (recomputed next tick), not persisted.
    rec.arbiter = arbiter.map(|a| common::bastion::ArbiterStateV1 {
        current: a.current,
        commitment_remaining_secs: (a.committed_until - now).max(0.0),
    });
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
            // DET-COL-AUT-002: mirror the arbiter drive into the persistent
            // record every loaded tick and restore it on promote.
            WriteStorage<'a, comp::bastion::Arbiter>,
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
                mut bastion_arbiters,
                mut stats_storage,
                bastion_active_jobs,
                bastion_test_gotos,
                bastion_traversal_ownerships,
                mut job_board,
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

        // Perform a save if required
        if rtsim
            .last_saved
            .is_none_or(|ls| ls.elapsed() > Duration::from_secs(60))
        {
            // TODO: Use slow jobs
            let _ = slow_jobs;
            rtsim.save(/* &slow_jobs, */ false);
        }

        let chunk_states = rtsim.state.resource::<ChunkStates>();
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
        let mut pending_thoughts = std::mem::take(&mut job_board.pending_thoughts);
        pending_thoughts.sort_by_key(|(re, pos, kind)| (*re, pos.x, pos.y, pos.z, *kind));
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
                            let _ = colonists.insert(entity, comp::Colonist(colonist.clone()));
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
                            // DET-COL-AUT-002: RESTORE the persisted arbiter
                            // drive (`None` = never captured / old save → the
                            // default `Idle` arbiter, pre-AUT-002 behavior).
                            // The commitment deadline is reconstructed from the
                            // stored REMAINING duration against the current
                            // clock; last_scores/activity recompute next tick.
                            let _ = bastion_arbiters.insert(
                                entity,
                                colonist.arbiter.map_or_else(
                                    comp::bastion::Arbiter::default,
                                    |st| comp::bastion::Arbiter {
                                        current: st.current,
                                        committed_until: time.0 + st.commitment_remaining_secs,
                                        ..Default::default()
                                    },
                                ),
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
                                bastion_arbiters.get(entity),
                                time.0,
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
                                bastion_arbiters.get(entity),
                                time.0,
                            ));
                        }
                        delete_emitter.emit(DeleteEvent(entity));
                    },
                }
            }
        }
    }
}
