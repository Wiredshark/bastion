/// This file contains a single economy
/// and functions to simulate it
use crate::world_msg::EconomyInfo;
use crate::{
    sim::SimChunk,
    site::Site,
    util::{DHashMap, DHashSet, map_array::GenericIndex},
};
use common::{
    store::Id,
    terrain::BiomeKind,
    trade::{Good, SitePrices},
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use std::{cmp::Ordering::Less, convert::TryFrom};
use tracing::{debug, info, trace, warn};

use Good::*;
mod map_types;
pub use map_types::Labor;
use map_types::{GoodIndex, GoodMap, LaborIndex, LaborMap, NaturalResources};
mod context;
pub use context::simulate_economy;
mod cache;

const INTER_SITE_TRADE: bool = true;
const DAYS_PER_MONTH: f32 = 30.0;
const DAYS_PER_YEAR: f32 = 12.0 * DAYS_PER_MONTH;
const GENERATE_CSV: bool = false;

#[derive(Debug)]
pub struct TradeOrder {
    customer: Id<Site>,
    amount: GoodMap<f32>, // positive for orders, negative for exchange
}

#[derive(Debug)]
pub struct TradeDelivery {
    supplier: Id<Site>,
    amount: GoodMap<f32>, // positive for orders, negative for exchange
    prices: GoodMap<f32>, // at the time of interaction
    supply: GoodMap<f32>, // maximum amount available, at the time of interaction
}

#[derive(Debug, Default)]
pub struct TradeInformation {
    orders: DHashMap<Id<Site>, Vec<TradeOrder>>, // per provider
    deliveries: DHashMap<Id<Site>, Vec<TradeDelivery>>, // per receiver
}

#[derive(Debug)]
pub struct NeighborInformation {
    id: Id<Site>,
    //travel_distance: usize,

    // remembered from last interaction
    last_values: GoodMap<f32>,
    last_supplies: GoodMap<f32>,
}

lazy_static! {
    static ref COIN_INDEX: GoodIndex = Coin.try_into().unwrap_or_default();
    static ref FOOD_INDEX: GoodIndex = Good::Food.try_into().unwrap_or_default();
    static ref TRANSPORTATION_INDEX: GoodIndex = Transportation.try_into().unwrap_or_default();
}

#[derive(Debug)]
pub struct Economy {
    /// Population
    pop: f32,
    population_limited_by: GoodIndex,

    /// Total available amount of each good
    stocks: GoodMap<f32>,
    /// Surplus stock compared to demand orders
    surplus: GoodMap<f32>,
    /// change rate (derivative) of stock in the current situation
    marginal_surplus: GoodMap<f32>,
    /// amount of wares not needed by the economy (helps with trade planning)
    unconsumed_stock: GoodMap<f32>,
    /// Local availability of a good, 4.0 = starved, 2.0 = balanced, 0.1 =
    /// extra, NULL = way too much
    // For some goods, such a goods without any supply, it doesn't make sense to talk about value
    values: GoodMap<Option<f32>>,
    /// amount of goods exported/imported during the last cycle
    last_exports: GoodMap<f32>,
    active_exports: GoodMap<f32>, // unfinished trade (amount unconfirmed)
    //pub export_targets: GoodMap<f32>,
    /// amount of labor that went into a good, [1 man cycle=1.0]
    labor_values: GoodMap<Option<f32>>,
    // this assumes a single source, replace with LaborMap?
    material_costs: GoodMap<f32>,

    /// Proportion of individuals dedicated to an industry (sums to roughly 1.0)
    labors: LaborMap<f32>,
    // Per worker, per year, of their output good
    yields: LaborMap<f32>,
    /// [0.0..1.0]
    productivity: LaborMap<f32>,
    /// Missing raw material which limits production
    limited_by: LaborMap<GoodIndex>,

    natural_resources: NaturalResources,
    /// Neighboring sites to trade with
    neighbors: Vec<NeighborInformation>,

    /// outgoing trade, per provider
    orders: DHashMap<Id<Site>, Vec<TradeOrder>>,
    /// incoming trade - only towards this site
    deliveries: Vec<TradeDelivery>,
}

fn push_f32(buf: &mut Vec<u8>, v: f32) { buf.extend_from_slice(&v.to_be_bytes()); }
fn push_u64(buf: &mut Vec<u8>, v: u64) { buf.extend_from_slice(&v.to_be_bytes()); }
fn push_good_index(buf: &mut Vec<u8>, g: GoodIndex) { push_u64(buf, g.into_usize() as u64); }
fn push_option_f32(buf: &mut Vec<u8>, v: Option<f32>) {
    match v {
        Some(f) => {
            buf.push(1);
            push_f32(buf, f);
        },
        None => buf.push(0),
    }
}
fn push_good_map_f32(buf: &mut Vec<u8>, m: &GoodMap<f32>) {
    for (_, v) in m.iter() {
        push_f32(buf, *v);
    }
}
fn push_good_map_option_f32(buf: &mut Vec<u8>, m: &GoodMap<Option<f32>>) {
    for (_, v) in m.iter() {
        push_option_f32(buf, *v);
    }
}
fn push_labor_map_f32(buf: &mut Vec<u8>, m: &LaborMap<f32>) {
    for (_, v) in m.iter() {
        push_f32(buf, *v);
    }
}
fn push_labor_map_good_index(buf: &mut Vec<u8>, m: &LaborMap<GoodIndex>) {
    for (_, v) in m.iter() {
        push_good_index(buf, *v);
    }
}

impl Economy {
    /// `APEX-T4.3` chunk 2: the "economic baseline" component of
    /// `WorldBaselineManifestV1`. Every field is included, in DECLARATION
    /// order for the fixed-shape `GoodMap`/`LaborMap` array fields (whose
    /// own `.iter()` is index-ordered, not hash-ordered -- verified by
    /// reading their backing storage: `GoodMap`/`LaborMap` are `[V;
    /// LENGTH]`/`Vec<V>`, never a `HashMap`), and explicitly SORTED by
    /// `Id<Site>` for the two fields that genuinely are order-unstable
    /// (`orders`, a real `DHashMap`; `neighbors`/`deliveries`, plain
    /// `Vec`s whose own build order this function does not trust).
    /// `Id<Site>` is a `Store<T>` index -- confirmed stable/never-recycled
    /// at `E11-3b`'s own premise-check, safe to sort and hash directly.
    pub fn canonical_baseline_hash_v1(&self) -> common::apex::digest::ArtifactIdentityV1 {
        let mut buf = Vec::new();
        push_f32(&mut buf, self.pop);
        push_good_index(&mut buf, self.population_limited_by);
        push_good_map_f32(&mut buf, &self.stocks);
        push_good_map_f32(&mut buf, &self.surplus);
        push_good_map_f32(&mut buf, &self.marginal_surplus);
        push_good_map_f32(&mut buf, &self.unconsumed_stock);
        push_good_map_option_f32(&mut buf, &self.values);
        push_good_map_f32(&mut buf, &self.last_exports);
        push_good_map_f32(&mut buf, &self.active_exports);
        push_good_map_option_f32(&mut buf, &self.labor_values);
        push_good_map_f32(&mut buf, &self.material_costs);
        push_labor_map_f32(&mut buf, &self.labors);
        push_labor_map_f32(&mut buf, &self.yields);
        push_labor_map_f32(&mut buf, &self.productivity);
        push_labor_map_good_index(&mut buf, &self.limited_by);

        for area in &self.natural_resources.per_area {
            push_good_map_f32(&mut buf, &area.resource_sum);
            push_good_map_f32(&mut buf, &area.resource_chunks);
            push_u64(&mut buf, area.chunks as u64);
        }
        push_good_map_f32(&mut buf, &self.natural_resources.chunks_per_resource);
        push_good_map_f32(&mut buf, &self.natural_resources.average_yield_per_chunk);

        let mut neighbors: Vec<&NeighborInformation> = self.neighbors.iter().collect();
        neighbors.sort_unstable_by_key(|n| n.id.id());
        push_u64(&mut buf, neighbors.len() as u64);
        for n in neighbors {
            push_u64(&mut buf, n.id.id());
            push_good_map_f32(&mut buf, &n.last_values);
            push_good_map_f32(&mut buf, &n.last_supplies);
        }

        let mut orders: Vec<(&Id<Site>, &Vec<TradeOrder>)> = self.orders.iter().collect();
        orders.sort_unstable_by_key(|(id, _)| id.id());
        push_u64(&mut buf, orders.len() as u64);
        for (provider, provider_orders) in orders {
            push_u64(&mut buf, provider.id());
            push_u64(&mut buf, provider_orders.len() as u64);
            for order in provider_orders {
                push_u64(&mut buf, order.customer.id());
                push_good_map_f32(&mut buf, &order.amount);
            }
        }

        let mut deliveries: Vec<&TradeDelivery> = self.deliveries.iter().collect();
        deliveries.sort_unstable_by_key(|d| d.supplier.id());
        push_u64(&mut buf, deliveries.len() as u64);
        for delivery in deliveries {
            push_u64(&mut buf, delivery.supplier.id());
            push_good_map_f32(&mut buf, &delivery.amount);
            push_good_map_f32(&mut buf, &delivery.prices);
            push_good_map_f32(&mut buf, &delivery.supply);
        }

        common::apex::digest::hash_artifact_bytes_v1(&buf)
    }
}

impl Default for Economy {
    fn default() -> Self {
        let coin_index: GoodIndex = GoodIndex::try_from(Coin).unwrap_or_default();
        Self {
            pop: 32.0,
            population_limited_by: GoodIndex::default(),

            stocks: GoodMap::from_list(&[(coin_index, Economy::STARTING_COIN)], 100.0),
            surplus: Default::default(),
            marginal_surplus: Default::default(),
            values: GoodMap::from_list(&[(coin_index, Some(2.0))], None),
            last_exports: Default::default(),
            active_exports: Default::default(),

            labor_values: Default::default(),
            material_costs: Default::default(),

            labors: LaborMap::from_default(0.01),
            yields: LaborMap::from_default(1.0),
            productivity: LaborMap::from_default(1.0),
            limited_by: LaborMap::from_default(GoodIndex::default()),

            natural_resources: Default::default(),
            neighbors: Default::default(),
            unconsumed_stock: Default::default(),

            orders: Default::default(),
            deliveries: Default::default(),
        }
    }
}

impl Economy {
    // FIXME?: this is hit for (almost) every Good
    //
    // Which means that all goods in all cities have the same price.
    //
    // We could try to change that fact, but:
    // a) This would mean we need need to rebalance prices again, which
    // could probably be done quite easily with good_scaling, but still.
    // b) Making prices vary from town to town would lead to (expected)
    // scenarios where price of buying good in one town is less than
    // the price of selling good in another town.
    // Which we probably want, but the question is to what extent.
    // c) Traveling merchants fuck this anyway from a gameplay perspective,
    // since they have randomized origins anyway and don't conform to local
    // prices the way they are coded right now.
    const MINIMUM_PRICE: f32 = 0.1;
    const STARTING_COIN: f32 = 1000.0;
    const _NATURAL_RESOURCE_SCALE: f32 = 1.0 / 9.0;

    pub fn population(&self) -> f32 { self.pop }

    pub fn get_available_stock(&self) -> HashMap<Good, f32> {
        self.unconsumed_stock
            .iter()
            .map(|(g, a)| (g.into(), *a))
            .collect()
    }

    pub fn get_information(&self, id: Id<Site>) -> EconomyInfo {
        EconomyInfo {
            id: id.id(),
            population: self.pop.floor() as u32,
            stock: self
                .stocks
                .iter()
                .map(|(g, a)| (Good::from(g), *a))
                .collect(),
            labor_values: self
                .labor_values
                .iter()
                .filter_map(|(g, a)| a.map(|a| (Good::from(g), a)))
                .collect(),
            values: self
                .values
                .iter()
                .filter_map(|(g, a)| a.map(|a| (Good::from(g), a)))
                .collect(),
            labors: self.labors.iter().map(|(_, a)| *a).collect(),
            last_exports: self
                .last_exports
                .iter()
                .map(|(g, a)| (Good::from(g), *a))
                .collect(),
            resources: self
                .natural_resources
                .chunks_per_resource
                .iter()
                .map(|(g, a)| {
                    (
                        Good::from(g),
                        (*a) * self.natural_resources.average_yield_per_chunk[g],
                    )
                })
                .collect(),
        }
    }

    pub fn cache_economy(&mut self) {
        for g in good_list() {
            let amount: f32 = self
                .natural_resources
                .per_area
                .iter()
                .map(|a| a.resource_sum[g])
                .sum();
            let chunks = self
                .natural_resources
                .per_area
                .iter()
                .map(|a| a.resource_chunks[g])
                .sum();
            if chunks > 0.001 {
                self.natural_resources.chunks_per_resource[g] = chunks;
                self.natural_resources.average_yield_per_chunk[g] = amount / chunks;
            }
        }
    }

    /// orders per profession (excluding everyone)
    fn get_orders(&self) -> &'static LaborMap<Vec<(GoodIndex, f32)>> {
        lazy_static! {
            static ref ORDERS: LaborMap<Vec<(GoodIndex, f32)>> = {
                let mut res: LaborMap<Vec<(GoodIndex, f32)>> = LaborMap::default();
                res.iter_mut()
                    .for_each(|(i, e)| e.extend(i.orders().copied()));
                res
            };
        }
        &ORDERS
    }

    /// resources consumed by everyone (no matter which profession)
    fn get_orders_everyone(&self) -> impl Iterator<Item = &'static (GoodIndex, f32)> + use<> {
        Labor::orders_everyone()
    }

    fn get_production(&self) -> LaborMap<(GoodIndex, f32)> {
        // cache the site independent part of production
        lazy_static! {
            static ref PRODUCTS: LaborMap<(GoodIndex, f32)> = LaborMap::from_iter(
                Labor::list().map(|p| { (p, p.products(),) }),
                (GoodIndex::default(), 0.0),
            );
        }
        PRODUCTS.map(|l, vec| {
            //dbg!((l,vec));
            let labor_ratio = self.labors[l];
            let total_workers = labor_ratio * self.pop;
            // apply economy of scale (workers get more productive in numbers)
            let relative_scale = 1.0 + labor_ratio;
            let absolute_scale = (1.0 + total_workers / 100.0).min(3.0);
            let scale = relative_scale * absolute_scale;
            (vec.0, vec.1 * scale)
        })
    }

    fn replenish(&mut self, _time: f32) {
        for (good, &ch) in self.natural_resources.chunks_per_resource.iter() {
            let per_year = self.natural_resources.average_yield_per_chunk[good] * ch;
            self.stocks[good] = self.stocks[good].max(per_year);
        }
        // info!("resources {:?}", self.stocks);
    }

    pub fn add_chunk(&mut self, ch: &SimChunk, distance_squared: i64) {
        // let biome = ch.get_biome();
        // we don't scale by pi, although that would be correct
        let distance_bin = (distance_squared >> 16).min(64) as usize;
        if self.natural_resources.per_area.len() <= distance_bin {
            self.natural_resources
                .per_area
                .resize_with(distance_bin + 1, Default::default);
        }
        self.natural_resources.per_area[distance_bin].chunks += 1;

        let mut add_biome = |biome, amount| {
            if let Ok(idx) = GoodIndex::try_from(Terrain(biome)) {
                self.natural_resources.per_area[distance_bin].resource_sum[idx] += amount;
                self.natural_resources.per_area[distance_bin].resource_chunks[idx] += amount;
            }
        };
        if ch.river.is_ocean() {
            add_biome(BiomeKind::Ocean, 1.0);
        } else if ch.river.is_lake() {
            add_biome(BiomeKind::Lake, 1.0);
        } else {
            add_biome(BiomeKind::Forest, 0.5 + ch.tree_density);
            add_biome(BiomeKind::Grassland, 0.5 + ch.humidity);
            add_biome(BiomeKind::Jungle, 0.5 + ch.humidity * ch.temp.max(0.0));
            add_biome(BiomeKind::Mountain, 0.5 + (ch.alt / 4000.0).max(0.0));
            add_biome(
                BiomeKind::Desert,
                0.5 + (1.0 - ch.humidity) * ch.temp.max(0.0),
            );
            add_biome(BiomeKind::Snowland, 0.5 + (-ch.temp).max(0.0));
        }
    }

    pub fn add_neighbor(&mut self, id: Id<Site>, _distance: usize) {
        self.neighbors.push(NeighborInformation {
            id,
            //travel_distance: distance,
            last_values: GoodMap::from_default(Economy::MINIMUM_PRICE),
            last_supplies: Default::default(),
        });
    }

    pub fn get_site_prices(&self) -> SitePrices {
        let normalize = |xs: GoodMap<Option<f32>>| {
            let sum = xs
                .iter()
                .map(|(_, x)| (*x).unwrap_or(0.0))
                .sum::<f32>()
                .max(0.001);
            xs.map(|_, x| Some(x? / sum))
        };

        SitePrices {
            values: {
                let labor_values = normalize(self.labor_values);
                // Use labor values as prices. Not correct (doesn't care about exchange value)
                let prices = normalize(self.values).map(|good, value| {
                    ((labor_values[good].unwrap_or(Economy::MINIMUM_PRICE)
                        + value.unwrap_or(Economy::MINIMUM_PRICE))
                        * 0.5)
                        .max(Economy::MINIMUM_PRICE)
                });
                prices.iter().map(|(g, v)| (Good::from(g), *v)).collect()
            },
        }
    }

    /// plan the trading according to missing goods and prices at neighboring
    /// sites (1st step of trading)
    // returns wares spent (-) and procured (+)
    // potential_trade: positive = buy, (negative = sell, unused)
    fn plan_trade_for_site(
        // site: &mut Site,
        &mut self,
        site_id: &Id<Site>,
        transportation_capacity: f32,
        // external_orders: &mut DHashMap<Id<Site>, Vec<TradeOrder>>,
        potential_trade: &mut GoodMap<f32>,
    ) -> GoodMap<f32> {
        // TODO: Do we have some latency of information here (using last years
        // capacity?)
        //let total_transport_capacity = self.stocks[Transportation];
        // TODO: We don't count the capacity per site, but globally (so there might be
        // some imbalance in dispatch vs collection across sites (e.g. more dispatch
        // than collection at one while more collection than dispatch at another))
        // transport capacity works both ways (going there and returning)
        let mut dispatch_capacity = transportation_capacity;
        let mut collect_capacity = transportation_capacity;
        let mut missing_dispatch: f32 = 0.0;
        let mut missing_collect: f32 = 0.0;
        let mut result = GoodMap::default();
        const MIN_SELL_PRICE: f32 = 1.0;
        // value+amount per good
        let mut missing_goods: Vec<(GoodIndex, (f32, f32))> = self
            .surplus
            .iter()
            .filter(|(g, a)| **a < 0.0 && *g != *TRANSPORTATION_INDEX)
            .map(|(g, a)| (g, (self.values[g].unwrap_or(Economy::MINIMUM_PRICE), -*a)))
            .collect();
        missing_goods.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(Less));
        let mut extra_goods: GoodMap<f32> = GoodMap::from_iter(
            self.surplus
                .iter()
                .chain(core::iter::once((*COIN_INDEX, &self.stocks[*COIN_INDEX])))
                .filter(|(g, a)| **a > 0.0 && *g != *TRANSPORTATION_INDEX)
                .map(|(g, a)| (g, *a)),
            0.0,
        );
        // ratio+price per good and site
        type GoodRatioPrice = Vec<(GoodIndex, (f32, f32))>;
        let good_payment: DHashMap<Id<Site>, GoodRatioPrice> = self
            .neighbors
            .iter()
            .map(|n| {
                let mut rel_value = extra_goods
                    .iter()
                    .map(|(g, _)| (g, n.last_values[g]))
                    .filter(|(_, last_val)| *last_val >= MIN_SELL_PRICE)
                    .map(|(g, last_val)| {
                        (
                            g,
                            (
                                last_val
                                    / self.values[g].unwrap_or(-1.0).max(Economy::MINIMUM_PRICE),
                                last_val,
                            ),
                        )
                    })
                    .collect::<Vec<_>>();
                rel_value.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(Less));
                (n.id, rel_value)
            })
            .collect();
        // price+stock per site and good
        type SitePriceStock = Vec<(Id<Site>, (f32, f32))>;
        let mut good_price: DHashMap<GoodIndex, SitePriceStock> = missing_goods
            .iter()
            .map(|(g, _)| {
                (*g, {
                    let mut neighbor_prices: Vec<(Id<Site>, (f32, f32))> = self
                        .neighbors
                        .iter()
                        .filter(|n| n.last_supplies[*g] > 0.0)
                        .map(|n| (n.id, (n.last_values[*g], n.last_supplies[*g])))
                        .collect();
                    neighbor_prices.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(Less));
                    neighbor_prices
                })
            })
            .collect();
        // TODO: we need to introduce priority (according to available transportation
        // capacity)
        let mut neighbor_orders: DHashMap<Id<Site>, GoodMap<f32>> = self
            .neighbors
            .iter()
            .map(|n| (n.id, GoodMap::default()))
            .collect();
        if site_id.id() == 1 {
            // cut down number of lines printed
            trace!(
                "Site {} #neighbors {} Transport capacity {}",
                site_id.id(),
                self.neighbors.len(),
                transportation_capacity,
            );
            trace!("missing {:#?} extra {:#?}", missing_goods, extra_goods,);
            trace!("buy {:#?} pay {:#?}", good_price, good_payment);
        }
        // === the actual planning is here ===
        for (g, (_, a)) in missing_goods.iter() {
            let mut amount = *a;
            if let Some(site_price_stock) = good_price.get_mut(g) {
                for (s, (price, supply)) in site_price_stock.iter_mut() {
                    // how much to buy, limit by supply and transport budget
                    let mut buy_target = amount.min(*supply);
                    let effort = transportation_effort(*g);
                    let collect = buy_target * effort;
                    let mut potential_balance: f32 = 0.0;
                    if collect > collect_capacity && effort > 0.0 {
                        let transportable_amount = collect_capacity / effort;
                        let missing_trade = buy_target - transportable_amount;
                        potential_trade[*g] += missing_trade;
                        potential_balance += missing_trade * *price;
                        buy_target = transportable_amount; // (buy_target - missing_trade).max(0.0); // avoid negative buy target caused by numeric inaccuracies
                        missing_collect += collect - collect_capacity;
                        trace!(
                            "missing capacity {:?}/{:?} {:?}",
                            missing_trade, amount, potential_balance,
                        );
                        amount = (amount - missing_trade).max(0.0); // you won't be able to transport it from elsewhere either, so don't count multiple times
                    }
                    let mut balance: f32 = *price * buy_target;
                    trace!(
                        "buy {:?} at {:?} amount {:?} balance {:?}",
                        *g,
                        s.id(),
                        buy_target,
                        balance,
                    );
                    if let Some(neighbor_orders) = neighbor_orders.get_mut(s) {
                        // find suitable goods in exchange
                        let mut acute_missing_dispatch: f32 = 0.0; // only count the highest priority (not multiple times)
                        for (g2, (_, price2)) in good_payment[s].iter() {
                            let mut amount2 = extra_goods[*g2];
                            // good available for trading?
                            if amount2 > 0.0 {
                                amount2 = amount2.min(balance / price2); // pay until balance is even
                                let effort2 = transportation_effort(*g2);
                                let mut dispatch = amount2 * effort2;
                                // limit by separate transport budget (on way back)
                                if dispatch > dispatch_capacity && effort2 > 0.0 {
                                    let transportable_amount = dispatch_capacity / effort2;
                                    let missing_trade = amount2 - transportable_amount;
                                    amount2 = transportable_amount;
                                    if acute_missing_dispatch == 0.0 {
                                        acute_missing_dispatch = missing_trade * effort2;
                                    }
                                    trace!(
                                        "can't carry payment {:?} {:?} {:?}",
                                        g2, dispatch, dispatch_capacity
                                    );
                                    dispatch = dispatch_capacity;
                                }

                                extra_goods[*g2] -= amount2;
                                trace!("pay {:?} {:?} = {:?}", g2, amount2, balance);
                                balance -= amount2 * price2;
                                neighbor_orders[*g2] -= amount2;
                                dispatch_capacity = (dispatch_capacity - dispatch).max(0.0);
                                if balance == 0.0 {
                                    break;
                                }
                            }
                        }
                        missing_dispatch += acute_missing_dispatch;
                        // adjust order if we are unable to pay for it
                        buy_target -= balance / *price;
                        buy_target = buy_target.min(amount);
                        collect_capacity = (collect_capacity - buy_target * effort).max(0.0);
                        neighbor_orders[*g] += buy_target;
                        amount -= buy_target;
                        trace!(
                            "deal amount {:?} end_balance {:?} price {:?} left {:?}",
                            buy_target, balance, *price, amount
                        );
                    }
                }
            }
        }
        // if site_id.id() == 1 {
        //     // cut down number of lines printed
        //     info!("orders {:#?}", neighbor_orders,);
        // }
        // TODO: Use planned orders and calculate value, stock etc. accordingly
        for n in &self.neighbors {
            if let Some(orders) = neighbor_orders.get(&n.id) {
                for (g, a) in orders.iter() {
                    result[g] += *a;
                }
                let to = TradeOrder {
                    customer: *site_id,
                    amount: *orders,
                };
                if let Some(o) = self.orders.get_mut(&n.id) {
                    // this is just to catch unbound growth (happened in development)
                    if o.len() < 100 {
                        o.push(to);
                    } else {
                        warn!("overflow {:?}", o);
                    }
                } else {
                    self.orders.insert(n.id, vec![to]);
                }
            }
        }
        // return missing transport capacity
        //missing_collect.max(missing_dispatch)
        trace!(
            "Tranportation {:?} {:?} {:?} {:?} {:?}",
            transportation_capacity,
            collect_capacity,
            dispatch_capacity,
            missing_collect,
            missing_dispatch,
        );
        result[*TRANSPORTATION_INDEX] = -(transportation_capacity
            - collect_capacity.min(dispatch_capacity)
            + missing_collect.max(missing_dispatch));
        if site_id.id() == 1 {
            trace!("Trade {:?}", result);
        }
        result
    }

    /// perform trade using neighboring orders (2nd step of trading)
    pub fn trade_at_site(
        &mut self,
        site_id: Id<Site>,
        orders: &mut Vec<TradeOrder>,
        // economy: &mut Economy,
        deliveries: &mut DHashMap<Id<Site>, Vec<TradeDelivery>>,
    ) {
        // make sure that at least this amount of stock remains available
        // TODO: rework using economy.unconsumed_stock

        let internal_orders = self.get_orders();
        let mut next_demand = GoodMap::from_default(0.0);
        for (labor, orders) in internal_orders.iter() {
            let workers = self.labors[labor] * self.pop;
            for (good, amount) in orders {
                next_demand[*good] += *amount * workers;
                assert!(next_demand[*good] >= 0.0);
            }
        }
        for (good, amount) in self.get_orders_everyone() {
            next_demand[*good] += *amount * self.pop;
            assert!(next_demand[*good] >= 0.0);
        }
        //info!("Trade {} {}", site.id(), orders.len());
        let mut total_orders: GoodMap<f32> = GoodMap::from_default(0.0);
        for i in orders.iter() {
            for (g, &a) in i.amount.iter().filter(|(_, a)| **a > 0.0) {
                total_orders[g] += a;
            }
        }
        let order_stock_ratio: GoodMap<Option<f32>> = GoodMap::from_iter(
            self.stocks
                .iter()
                .map(|(g, a)| (g, *a, next_demand[g]))
                .filter(|(_, a, s)| *a > *s)
                .map(|(g, a, s)| (g, Some(total_orders[g] / (a - s)))),
            None,
        );
        trace!("trade {} {:?}", site_id.id(), order_stock_ratio);
        let prices = GoodMap::from_iter(
            self.values
                .iter()
                .map(|(g, o)| (g, o.unwrap_or(0.0).max(Economy::MINIMUM_PRICE))),
            0.0,
        );
        for o in orders.drain(..) {
            // amount, local value (sell low value, buy high value goods first (trading
            // town's interest))
            let mut sorted_sell: Vec<(GoodIndex, f32, f32)> = o
                .amount
                .iter()
                .filter(|&(_, &a)| a > 0.0)
                .map(|(g, a)| (g, *a, prices[g]))
                .collect();
            sorted_sell.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Less));
            let mut sorted_buy: Vec<(GoodIndex, f32, f32)> = o
                .amount
                .iter()
                .filter(|&(_, &a)| a < 0.0)
                .map(|(g, a)| (g, *a, prices[g]))
                .collect();
            sorted_buy.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Less));
            trace!(
                "with {} {:?} buy {:?}",
                o.customer.id(),
                sorted_sell,
                sorted_buy
            );
            let mut good_delivery = GoodMap::from_default(0.0);
            for (g, amount, price) in sorted_sell.iter() {
                if let Some(order_stock_ratio) = order_stock_ratio[*g] {
                    let allocated_amount = *amount / order_stock_ratio.max(1.0);
                    let mut balance = allocated_amount * *price;
                    for (g2, avail, price2) in sorted_buy.iter_mut() {
                        let amount2 = (-*avail).min(balance / *price2);
                        assert!(amount2 >= 0.0);
                        self.stocks[*g2] += amount2;
                        balance = (balance - amount2 * *price2).max(0.0);
                        *avail += amount2; // reduce (negative) brought stock
                        trace!("paid with {:?} {} {}", *g2, amount2, *price2);
                        if balance == 0.0 {
                            break;
                        }
                    }
                    let mut paid_amount =
                        (allocated_amount - balance / *price).min(self.stocks[*g]);
                    if paid_amount / allocated_amount < 0.95 {
                        trace!(
                            "Client {} is broke on {:?} : {} {} severity {}",
                            o.customer.id(),
                            *g,
                            paid_amount,
                            allocated_amount,
                            order_stock_ratio,
                        );
                    } else {
                        trace!("bought {:?} {} {}", *g, paid_amount, *price);
                    }
                    if self.stocks[*g] - paid_amount < 0.0 {
                        info!(
                            "BUG {:?} {:?} {} TO {:?} OSR {:?} ND {:?}",
                            self.stocks[*g],
                            *g,
                            paid_amount,
                            total_orders[*g],
                            order_stock_ratio,
                            next_demand[*g]
                        );
                        paid_amount = self.stocks[*g];
                    }
                    good_delivery[*g] += paid_amount;
                    self.stocks[*g] -= paid_amount;
                }
            }
            for (g, amount, _) in sorted_buy.drain(..) {
                if amount < 0.0 {
                    trace!("shipping back unsold {} of {:?}", amount, g);
                    good_delivery[g] += -amount;
                }
            }
            let delivery = TradeDelivery {
                supplier: site_id,
                prices,
                supply: GoodMap::from_iter(
                    self.stocks.iter().map(|(g, a)| {
                        (g, {
                            (a - next_demand[g] - total_orders[g]).max(0.0) + good_delivery[g]
                        })
                    }),
                    0.0,
                ),
                amount: good_delivery,
            };
            trace!(?delivery);
            if let Some(deliveries) = deliveries.get_mut(&o.customer) {
                deliveries.push(delivery);
            } else {
                deliveries.insert(o.customer, vec![delivery]);
            }
        }
        if !orders.is_empty() {
            info!("non empty orders {:?}", orders);
            orders.clear();
        }
    }

    /// 3rd step of trading
    fn collect_deliveries(
        // site: &mut Site,
        &mut self,
        // deliveries: &mut Vec<TradeDelivery>,
        // ctx: &mut vergleich::Context,
    ) {
        // collect all the goods we shipped
        let mut last_exports = GoodMap::from_iter(
            self.active_exports
                .iter()
                .filter(|(_g, a)| **a > 0.0)
                .map(|(g, a)| (g, *a)),
            0.0,
        );
        // TODO: properly rate benefits created by merchants (done below?)
        for mut d in self.deliveries.drain(..) {
            // let mut ictx = ctx.context(&format!("suppl {}", d.supplier.id()));
            for i in d.amount.iter() {
                last_exports[i.0] -= *i.1;
            }
            // remember price
            if let Some(n) = self.neighbors.iter_mut().find(|n| n.id == d.supplier) {
                // remember (and consume) last values
                std::mem::swap(&mut n.last_values, &mut d.prices);
                std::mem::swap(&mut n.last_supplies, &mut d.supply);
                // add items to stock
                for (g, a) in d.amount.iter() {
                    if *a < 0.0 {
                        // likely rounding error, ignore
                        trace!("Unexpected delivery for {:?} {}", g, *a);
                    } else {
                        self.stocks[g] += *a;
                    }
                }
            }
        }
        if !self.deliveries.is_empty() {
            info!("non empty deliveries {:?}", self.deliveries);
            self.deliveries.clear();
        }
        std::mem::swap(&mut last_exports, &mut self.last_exports);
        //self.active_exports.clear();
    }

    /// Simulate one step of economic interaction:
    /// - collect returned goods from trade
    /// - calculate demand, production and their ratio
    /// - reassign workers based on missing goods
    /// - change stock due to raw material use and production
    /// - send out traders with goods and orders
    /// - calculate good decay and population change
    ///
    /// Simulate a site's economy. This simulation is roughly equivalent to the
    /// Lange-Lerner model's solution to the socialist calculation problem. The
    /// simulation begins by assigning arbitrary values to each commodity and
    /// then incrementally updates them according to the final scarcity of
    /// the commodity at the end of the tick. This results in the
    /// formulation of values that are roughly analogous to prices for each
    /// commodity. The workforce is then reassigned according to the
    /// respective commodity values. The simulation also includes damping
    /// terms that prevent cyclical inconsistencies in value rationalisation
    /// magnifying enough to crash the economy. We also ensure that
    /// a small number of workers are allocated to every industry (even inactive
    /// ones) each tick. This is not an accident: a small amount of productive
    /// capacity in one industry allows the economy to quickly pivot to a
    /// different production configuration should an additional commodity
    /// that acts as production input become available. This means that the
    /// economy will dynamically react to environmental changes. If a
    /// product becomes available through a mechanism such as trade, an
    /// entire arm of the economy may materialise to take advantage of this.
    pub fn tick(&mut self, site_id: Id<Site>, dt: f32) {
        // collect goods from trading
        if INTER_SITE_TRADE {
            self.collect_deliveries();
        }

        let orders = self.get_orders();
        let production = self.get_production();

        // for i in production.iter() {
        //     vc.context("production")
        //         .value(&std::format!("{:?}{:?}", i.0, Good::from(i.1.0)), i.1.1);
        // }

        let mut demand = GoodMap::from_default(0.0);
        for (labor, orders) in orders.iter() {
            let workers = self.labors[labor] * self.pop;
            for (good, amount) in orders {
                demand[*good] += *amount * workers;
            }
        }
        for (good, amount) in self.get_orders_everyone() {
            demand[*good] += *amount * self.pop;
        }
        if INTER_SITE_TRADE {
            demand[*COIN_INDEX] += Economy::STARTING_COIN; // if we spend coin value increases
        }

        // which labor is the merchant
        let merchant_labor = production
            .iter()
            .find(|(_, v)| v.0 == *TRANSPORTATION_INDEX)
            .map(|(l, _)| l)
            .unwrap_or_default();

        let mut supply = self.stocks; //GoodMap::from_default(0.0);
        for (labor, goodvec) in production.iter() {
            //for (output_good, _) in goodvec.iter() {
            //info!("{} supply{:?}+={}", site_id.id(), Good::from(goodvec.0),
            // self.yields[labor] * self.labors[labor] * self.pop);
            supply[goodvec.0] += self.yields[labor] * self.labors[labor] * self.pop;
            // vc.context(&std::format!("{:?}-{:?}", Good::from(goodvec.0),
            // labor))     .value("yields", self.yields[labor]);
            // vc.context(&std::format!("{:?}-{:?}", Good::from(goodvec.0),
            // labor))     .value("labors", self.labors[labor]);
            //}
        }

        // for i in supply.iter() {
        //     vc.context("supply")
        //         .value(&std::format!("{:?}", Good::from(i.0)), *i.1);
        // }

        let stocks = &self.stocks;
        // for i in stocks.iter() {
        //     vc.context("stocks")
        //         .value(&std::format!("{:?}", Good::from(i.0)), *i.1);
        // }
        self.surplus = demand.map(|g, demand| supply[g] + stocks[g] - demand);
        self.marginal_surplus = demand.map(|g, demand| supply[g] - demand);

        // plan trading with other sites
        // let external_orders = &mut index.trade.orders;
        let mut potential_trade = GoodMap::from_default(0.0);
        // use last year's generated transportation for merchants (could we do better?
        // this is in line with the other professions)
        let transportation_capacity = self.stocks[*TRANSPORTATION_INDEX];
        let trade = if INTER_SITE_TRADE {
            let trade =
                self.plan_trade_for_site(&site_id, transportation_capacity, &mut potential_trade);
            self.active_exports = GoodMap::from_iter(trade.iter().map(|(g, a)| (g, -*a)), 0.0); // TODO: check for availability?

            // add the wares to sell to demand and the goods to buy to supply
            for (g, a) in trade.iter() {
                // vc.context("trade")
                //     .value(&std::format!("{:?}", Good::from(g)), *a);
                if *a > 0.0 {
                    supply[g] += *a;
                    assert!(supply[g] >= 0.0);
                } else {
                    demand[g] -= *a;
                    assert!(demand[g] >= 0.0);
                }
            }
            trade
        } else {
            GoodMap::default()
        };

        // Update values according to the surplus of each stock
        // Note that values are used for workforce allocation and are not the same thing
        // as price
        // fall back to old (less wrong than other goods) coin logic
        let old_coin_surplus = self.stocks[*COIN_INDEX] - demand[*COIN_INDEX];
        let values = &mut self.values;

        self.surplus.iter().for_each(|(good, surplus)| {
            let old_surplus = if good == *COIN_INDEX {
                old_coin_surplus
            } else {
                *surplus
            };
            // Value rationalisation
            // let goodname = std::format!("{:?}", Good::from(good));
            // vc.context("old_surplus").value(&goodname, old_surplus);
            // vc.context("demand").value(&goodname, demand[good]);
            let val = 2.0f32.powf(1.0 - old_surplus / demand[good]);
            let smooth = 0.8;
            values[good] = if val > 0.001 && val < 1000.0 {
                Some(
                    // vc.context("values").value(
                    // &goodname,
                    smooth * values[good].unwrap_or(val) + (1.0 - smooth) * val,
                )
            } else {
                None
            };
        });

        let all_trade_goods: DHashSet<GoodIndex> = trade
            .iter()
            .chain(potential_trade.iter())
            .filter(|(_, a)| **a > 0.0)
            .map(|(g, _)| g)
            .collect();
        //let empty_goods: DHashSet<GoodIndex> = DHashSet::default();
        // TODO: Does avg/max/sum make most sense for labors creating more than one good
        // summing favors merchants too much (as they will provide multiple
        // goods, so we use max instead)
        let labor_ratios: LaborMap<f32> = LaborMap::from_iter(
            production.iter().map(|(labor, goodvec)| {
                (
                    labor,
                    if labor == merchant_labor {
                        all_trade_goods
                            .iter()
                            .chain(std::iter::once(&goodvec.0))
                            .map(|&output_good| self.values[output_good].unwrap_or(0.0))
                            .max_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap_or(Less))
                    } else {
                        self.values[goodvec.0]
                    }
                    .unwrap_or(0.0)
                        * self.productivity[labor],
                )
            }),
            0.0,
        );
        trace!(?labor_ratios);

        let labor_ratio_sum = labor_ratios.iter().map(|(_, r)| *r).sum::<f32>().max(0.01);
        //let mut labor_context = vc.context("labor");
        production.iter().for_each(|(labor, _)| {
            let smooth = 0.8;
            self.labors[labor] =
            // labor_context.value(
            //     &format!("{:?}", labor),
                smooth * self.labors[labor]
                    + (1.0 - smooth)
                        * (labor_ratios[labor].max(labor_ratio_sum / 1000.0) / labor_ratio_sum);
            assert!(self.labors[labor] >= 0.0);
        });

        // Production
        let stocks_before = self.stocks;
        // TODO: Should we recalculate demand after labor reassignment?

        let direct_use = direct_use_goods();
        // Handle the stocks you can't pile (decay)
        for g in direct_use {
            self.stocks[*g] = 0.0;
        }

        let mut total_labor_values = GoodMap::<f32>::default();
        // TODO: trade
        let mut total_outputs = GoodMap::<f32>::default();
        for (labor, orders) in orders.iter() {
            let workers = self.labors[labor] * self.pop;
            assert!(workers >= 0.0);
            let is_merchant = merchant_labor == labor;

            // For each order, we try to find the minimum satisfaction rate - this limits
            // how much we can produce! For example, if we need 0.25 fish and
            // 0.75 oats to make 1 unit of food, but only 0.5 units of oats are
            // available then we only need to consume 2/3rds
            // of other ingredients and leave the rest in stock
            // In effect, this is the productivity
            let (labor_productivity, limited_by) = orders
                .iter()
                .map(|(good, amount)| {
                    // What quantity is this order requesting?
                    let _quantity = *amount * workers;
                    assert!(stocks_before[*good] >= 0.0);
                    assert!(demand[*good] >= 0.0);
                    // What proportion of this order is the economy able to satisfy?
                    ((stocks_before[*good] / demand[*good]).min(1.0), *good)
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Less))
                .unwrap_or_else(|| {
                    panic!("Industry {:?} requires at least one input order", labor)
                });
            assert!(labor_productivity >= 0.0);
            self.limited_by[labor] = if labor_productivity >= 1.0 {
                GoodIndex::default()
            } else {
                limited_by
            };

            let mut total_materials_cost = 0.0;
            for (good, amount) in orders {
                // What quantity is this order requesting?
                let quantity = *amount * workers;
                // What amount gets actually used in production?
                let used = quantity * labor_productivity;

                // Material cost of each factor of production
                total_materials_cost += used * self.labor_values[*good].unwrap_or(0.0);

                // Deplete stocks accordingly
                if !direct_use.contains(good) {
                    self.stocks[*good] = (self.stocks[*good] - used).max(0.0);
                }
            }
            let mut produced_goods: GoodMap<f32> = GoodMap::from_default(0.0);
            if INTER_SITE_TRADE && is_merchant {
                // TODO: replan for missing merchant productivity???
                for (g, a) in trade.iter() {
                    if !direct_use.contains(&g) {
                        if *a < 0.0 {
                            // take these goods to the road
                            if self.stocks[g] + *a < 0.0 {
                                // we have a problem: Probably due to a shift in productivity we
                                // have less goods available than
                                // planned, so we would need to
                                // reduce the amount shipped
                                debug!("NEG STOCK {:?} {} {}", g, self.stocks[g], *a);
                                let reduced_amount = self.stocks[g];
                                let planned_amount: f32 = self
                                    .orders
                                    .iter()
                                    .map(|i| {
                                        i.1.iter()
                                            .filter(|o| o.customer == site_id)
                                            .map(|j| j.amount[g])
                                            .sum::<f32>()
                                    })
                                    .sum();
                                let scale = reduced_amount / planned_amount.abs();
                                trace!("re-plan {} {} {}", reduced_amount, planned_amount, scale);
                                for k in self.orders.iter_mut() {
                                    for l in k.1.iter_mut().filter(|o| o.customer == site_id) {
                                        l.amount[g] *= scale;
                                    }
                                }
                                self.stocks[g] = 0.0;
                            }
                            //                    assert!(self.stocks[g] + *a >= 0.0);
                            else {
                                self.stocks[g] += *a;
                            }
                        }
                        total_materials_cost += (-*a) * self.labor_values[g].unwrap_or(0.0);
                    } else {
                        // count on receiving these
                        produced_goods[g] += *a;
                    }
                }
                trace!(
                    "merchant {} {}: {:?} {} {:?}",
                    site_id.id(),
                    self.pop,
                    produced_goods,
                    total_materials_cost,
                    trade
                );
            }

            // Industries produce things
            let work_products = &production[labor];
            self.yields[labor] = labor_productivity * work_products.1;
            self.productivity[labor] = labor_productivity;
            let (stock, rate) = work_products;
            let total_output = labor_productivity * *rate * workers;
            assert!(total_output >= 0.0);
            self.stocks[*stock] += total_output;
            produced_goods[*stock] += total_output;

            let produced_amount: f32 = produced_goods.iter().map(|(_, a)| *a).sum();
            for (stock, amount) in produced_goods.iter() {
                let cost_weight = amount / produced_amount.max(0.001);
                // Materials cost per unit
                // TODO: How to handle this reasonably for multiple producers (collect upper and
                // lower term separately)
                self.material_costs[stock] = total_materials_cost / amount.max(0.001) * cost_weight;
                // Labor costs
                let wages = 1.0;
                let total_labor_cost = workers * wages;

                total_labor_values[stock] +=
                    (total_materials_cost + total_labor_cost) * cost_weight;
                total_outputs[stock] += amount;
            }
        }
        // consume goods needed by everyone
        for &(good, amount) in self.get_orders_everyone() {
            let needed = amount * self.pop;
            let available = stocks_before[good];
            self.stocks[good] = (self.stocks[good] - needed.min(available)).max(0.0);
            //info!("Ev {:.1} {:?} {} - {:.1} {:.1}", self.pop, good,
            // self.stocks[good], needed, available);
        }

        // Update labour values per unit
        self.labor_values = total_labor_values.map(|stock, tlv| {
            let total_output = total_outputs[stock];
            if total_output > 0.01 {
                Some(tlv / total_output)
            } else {
                None
            }
        });

        // Decay stocks (the ones which totally decay are handled later)
        self.stocks
            .iter_mut()
            .map(|(c, v)| (v, 1.0 - decay_rate(c)))
            .for_each(|(v, factor)| *v *= factor);

        // Decay stocks
        self.replenish(dt);

        // Births/deaths
        const NATURAL_BIRTH_RATE: f32 = 0.05;
        const DEATH_RATE: f32 = 0.005;
        let population_growth = self.surplus[*FOOD_INDEX] > 0.0;
        let birth_rate = if population_growth {
            NATURAL_BIRTH_RATE
        } else {
            0.0
        };
        self.pop += //vc.value(
            //"pop",
            dt / DAYS_PER_YEAR * self.pop * (birth_rate - DEATH_RATE);
        //);
        self.population_limited_by = if population_growth {
            GoodIndex::default()
        } else {
            *FOOD_INDEX
        };

        // calculate the new unclaimed stock
        //let next_orders = self.get_orders();
        // orders are static
        let mut next_demand = GoodMap::from_default(0.0);
        for (labor, orders) in orders.iter() {
            let workers = self.labors[labor] * self.pop;
            for (good, amount) in orders {
                next_demand[*good] += *amount * workers;
                assert!(next_demand[*good] >= 0.0);
            }
        }
        for (good, amount) in self.get_orders_everyone() {
            next_demand[*good] += *amount * self.pop;
            assert!(next_demand[*good] >= 0.0);
        }
        //let mut us = vc.context("unconsumed");
        self.unconsumed_stock = GoodMap::from_iter(
            self.stocks.iter().map(|(g, a)| {
                (
                    g,
                    //us.value(&format!("{:?}", Good::from(g)),
                    *a - next_demand[g],
                )
            }),
            0.0,
        );
    }

    pub fn csv_entry(f: &mut std::fs::File, site: &Site) -> Result<(), std::io::Error> {
        use std::io::Write;
        let d = Economy::default();
        let economy = site.economy.as_deref().unwrap_or(&d);
        write!(
            *f,
            "{}, {}, {}, {:.1}, {},,",
            site.name().unwrap_or("<None>"),
            site.origin.x,
            site.origin.y,
            economy.pop,
            economy.neighbors.len(),
        )?;
        for g in good_list() {
            if let Some(value) = economy.values[g] {
                write!(*f, "{:.2},", value)?;
            } else {
                f.write_all(b",")?;
            }
        }
        f.write_all(b",")?;
        for g in good_list() {
            if let Some(labor_value) = economy.labor_values[g] {
                write!(f, "{:.2},", labor_value)?;
            } else {
                f.write_all(b",")?;
            }
        }
        f.write_all(b",")?;
        for g in good_list() {
            write!(f, "{:.1},", economy.stocks[g])?;
        }
        f.write_all(b",")?;
        for g in good_list() {
            write!(f, "{:.1},", economy.marginal_surplus[g])?;
        }
        f.write_all(b",")?;
        for l in LaborIndex::list() {
            write!(f, "{:.1},", economy.labors[l] * economy.pop)?;
        }
        f.write_all(b",")?;
        for l in LaborIndex::list() {
            write!(f, "{:.2},", economy.productivity[l])?;
        }
        f.write_all(b",")?;
        for l in LaborIndex::list() {
            write!(f, "{:.1},", economy.yields[l])?;
        }
        f.write_all(b",")?;
        for l in LaborIndex::list() {
            let limit = economy.limited_by[l];
            if limit == GoodIndex::default() {
                f.write_all(b",")?;
            } else {
                write!(f, "{:?},", limit)?;
            }
        }
        f.write_all(b",")?;
        for g in good_list() {
            if economy.last_exports[g] >= 0.1 || economy.last_exports[g] <= -0.1 {
                write!(f, "{:.1},", economy.last_exports[g])?;
            } else {
                f.write_all(b",")?;
            }
        }
        writeln!(f)
    }

    fn csv_header(f: &mut std::fs::File) -> Result<(), std::io::Error> {
        use std::io::Write;
        write!(f, "Site,PosX,PosY,Population,Neighbors,,")?;
        for g in good_list() {
            write!(f, "{:?} Value,", g)?;
        }
        f.write_all(b",")?;
        for g in good_list() {
            write!(f, "{:?} LaborVal,", g)?;
        }
        f.write_all(b",")?;
        for g in good_list() {
            write!(f, "{:?} Stock,", g)?;
        }
        f.write_all(b",")?;
        for g in good_list() {
            write!(f, "{:?} Surplus,", g)?;
        }
        f.write_all(b",")?;
        for l in LaborIndex::list() {
            write!(f, "{:?} Labor,", l)?;
        }
        f.write_all(b",")?;
        for l in LaborIndex::list() {
            write!(f, "{:?} Productivity,", l)?;
        }
        f.write_all(b",")?;
        for l in LaborIndex::list() {
            write!(f, "{:?} Yields,", l)?;
        }
        f.write_all(b",")?;
        for l in LaborIndex::list() {
            write!(f, "{:?} limit,", l)?;
        }
        f.write_all(b",")?;
        for g in good_list() {
            write!(f, "{:?} trade,", g)?;
        }
        writeln!(f)
    }

    pub fn csv_open() -> Option<std::fs::File> {
        if GENERATE_CSV {
            let mut f = std::fs::File::create("economy.csv").ok()?;
            if Self::csv_header(&mut f).is_err() {
                None
            } else {
                Some(f)
            }
        } else {
            None
        }
    }

    #[cfg(test)]
    fn print_details(&self) {
        fn print_sorted(
            prefix: &str,
            mut list: Vec<(String, f32)>,
            threshold: f32,
            decimals: usize,
        ) {
            print!("{}", prefix);
            list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Less));
            for i in list.iter() {
                if i.1 >= threshold {
                    print!("{}={:.*} ", i.0, decimals, i.1);
                }
            }
            println!();
        }

        print!(" Resources: ");
        for i in good_list() {
            let amount = self.natural_resources.chunks_per_resource[i];
            if amount > 0.0 {
                print!("{:?}={} ", i, amount);
            }
        }
        println!();
        println!(
            " Population {:.1}, limited by {:?}",
            self.pop, self.population_limited_by
        );
        let idle: f32 = self.pop * (1.0 - self.labors.iter().map(|(_, a)| *a).sum::<f32>());
        print_sorted(
            &format!(" Professions: idle={:.1} ", idle),
            self.labors
                .iter()
                .map(|(l, a)| (format!("{:?}", l), *a * self.pop))
                .collect(),
            self.pop * 0.05,
            1,
        );
        print_sorted(
            " Stock: ",
            self.stocks
                .iter()
                .map(|(l, a)| (format!("{:?}", l), *a))
                .collect(),
            1.0,
            0,
        );
        print_sorted(
            " Values: ",
            self.values
                .iter()
                .map(|(l, a)| {
                    (
                        format!("{:?}", l),
                        a.map(|v| if v > 3.9 { 0.0 } else { v }).unwrap_or(0.0),
                    )
                })
                .collect(),
            0.1,
            1,
        );
        print_sorted(
            " Labor Values: ",
            self.labor_values
                .iter()
                .map(|(l, a)| (format!("{:?}", l), a.unwrap_or(0.0)))
                .collect(),
            0.1,
            1,
        );
        print!(" Limited: ");
        for (limit, prod) in self.limited_by.iter().zip(self.productivity.iter()) {
            if (0.01..=0.99).contains(prod.1) {
                print!("{:?}:{:?}={:.2} ", limit.0, limit.1, *prod.1);
            }
        }
        println!();
        print!(" Trade({}): ", self.neighbors.len());
        for (g, &amt) in self.active_exports.iter() {
            if !(-0.1..=0.1).contains(&amt) {
                print!("{:?}={:.2} ", g, amt);
            }
        }
        println!();
    }
}

fn good_list() -> impl Iterator<Item = GoodIndex> {
    (0..GoodIndex::LENGTH).map(GoodIndex::from_usize)
}

fn transportation_effort(g: GoodIndex) -> f32 { cache::cache().transport_effort[g] }

fn decay_rate(g: GoodIndex) -> f32 { cache::cache().decay_rate[g] }

/** you can't accumulate or save these options/resources for later */
fn direct_use_goods() -> &'static [GoodIndex] { &cache::cache().direct_use_goods }

pub struct GraphInfo {
    dummy: Economy,
}

impl Default for GraphInfo {
    fn default() -> Self {
        // avoid economy of scale
        Self {
            dummy: Economy {
                pop: 0.0,
                labors: LaborMap::from_default(0.0),
                ..Default::default()
            },
        }
    }
}

impl GraphInfo {
    pub fn get_orders(&self) -> &'static LaborMap<Vec<(GoodIndex, f32)>> { self.dummy.get_orders() }

    pub fn get_orders_everyone(&self) -> impl Iterator<Item = &'static (GoodIndex, f32)> + use<> {
        self.dummy.get_orders_everyone()
    }

    pub fn get_production(&self) -> LaborMap<(GoodIndex, f32)> { self.dummy.get_production() }

    pub fn good_list(&self) -> impl Iterator<Item = GoodIndex> + use<> { good_list() }

    pub fn labor_list(&self) -> impl Iterator<Item = Labor> + use<> { Labor::list() }

    pub fn can_store(&self, g: &GoodIndex) -> bool { direct_use_goods().contains(g) }
}

#[cfg(test)]
mod canonical_baseline_hash_v1_tests {
    use super::*;

    #[test]
    fn the_same_economy_hashes_the_same_twice() {
        let e = Economy::default();
        assert_eq!(e.canonical_baseline_hash_v1().digest, e.canonical_baseline_hash_v1().digest);
    }

    #[test]
    fn a_changed_field_moves_the_hash() {
        let base = Economy::default();
        let mut changed = Economy::default();
        changed.pop = base.pop + 1.0;
        assert_ne!(
            base.canonical_baseline_hash_v1().digest,
            changed.canonical_baseline_hash_v1().digest
        );
    }

    /// `E11-3b`-class falsifier: the same neighbor/order/delivery entries,
    /// inserted in a different order, must hash identically -- proves the
    /// sort-before-hash canonicalization is real, not decorative.
    #[test]
    fn permuted_neighbor_order_does_not_move_the_hash() {
        let n1 = NeighborInformation {
            id: Id::new(1),
            last_values: GoodMap::default(),
            last_supplies: GoodMap::default(),
        };
        let n2 = NeighborInformation {
            id: Id::new(2),
            last_values: GoodMap::default(),
            last_supplies: GoodMap::default(),
        };

        let forward = Economy { neighbors: vec![n1, n2], ..Default::default() };
        let n1b = NeighborInformation {
            id: Id::new(1),
            last_values: GoodMap::default(),
            last_supplies: GoodMap::default(),
        };
        let n2b = NeighborInformation {
            id: Id::new(2),
            last_values: GoodMap::default(),
            last_supplies: GoodMap::default(),
        };
        let reversed = Economy { neighbors: vec![n2b, n1b], ..Default::default() };

        assert_eq!(
            forward.canonical_baseline_hash_v1().digest,
            reversed.canonical_baseline_hash_v1().digest
        );
    }

    /// Non-vacuity companion: a genuinely DIFFERENT neighbor set (not
    /// just reordered) DOES move the hash -- proves the ordering test
    /// isn't passing because neighbors are ignored entirely.
    #[test]
    fn a_genuinely_different_neighbor_set_moves_the_hash() {
        let n1 = NeighborInformation {
            id: Id::new(1),
            last_values: GoodMap::default(),
            last_supplies: GoodMap::default(),
        };
        let with_one = Economy { neighbors: vec![n1], ..Default::default() };
        let with_none = Economy { neighbors: Vec::new(), ..Default::default() };

        assert_ne!(
            with_one.canonical_baseline_hash_v1().digest,
            with_none.canonical_baseline_hash_v1().digest
        );
    }

    /// Same falsifier for `orders`, the one field that is a REAL
    /// `DHashMap` (not just a plain `Vec` this function chooses to sort
    /// defensively) -- its own hash-bucket iteration order must not leak
    /// into the baseline hash.
    #[test]
    fn permuted_orders_insertion_order_does_not_move_the_hash() {
        let order_for = |customer: u64| TradeOrder { customer: Id::new(customer), amount: GoodMap::default() };

        let mut forward_orders: DHashMap<Id<Site>, Vec<TradeOrder>> = DHashMap::default();
        forward_orders.insert(Id::new(10), vec![order_for(100)]);
        forward_orders.insert(Id::new(20), vec![order_for(200)]);
        let forward = Economy { orders: forward_orders, ..Default::default() };

        let mut reversed_orders: DHashMap<Id<Site>, Vec<TradeOrder>> = DHashMap::default();
        reversed_orders.insert(Id::new(20), vec![order_for(200)]);
        reversed_orders.insert(Id::new(10), vec![order_for(100)]);
        let reversed = Economy { orders: reversed_orders, ..Default::default() };

        assert_eq!(
            forward.canonical_baseline_hash_v1().digest,
            reversed.canonical_baseline_hash_v1().digest
        );
    }
}

/// `T8.1` chunk 1's remaining required test. Lives here rather than
/// alongside `context`'s own phase-evidence tests because it needs to
/// perturb `Economy::stocks` directly, and that field is private to
/// THIS module -- `context` is a sibling, not a descendant, of the
/// module `stocks` is declared in.
#[cfg(test)]
mod t8_1_phase_localization_tests {
    use super::{GoodIndex, context};
    use common::trade::Good;

    fn minimal_fixture_v1(seed: u32, n: usize) -> crate::index::Index {
        let mut index = crate::index::Index::new(seed);
        for _ in 0..n {
            let mut site = crate::site::Site::default();
            site.kind = Some(crate::site::SiteKind::Refactor);
            let _ = site.economy_mut();
            index.sites.insert(site);
        }
        index
    }

    /// Required test: a deliberately perturbed phase is localised to
    /// THAT phase, not to the endpoint -- runs two identical fixtures in
    /// lockstep, one phase at a time, and injects a one-ULP nudge to one
    /// site's coin stock right after a chosen phase. Every phase before
    /// the perturbation must match; the perturbation itself must be the
    /// FIRST phase that doesn't (not the endpoint, and not a phase
    /// before it either -- a real localization, not a coincidence).
    #[test]
    fn a_deliberately_perturbed_phase_is_localized_to_that_phase() {
        context::enable_economy_phase_evidence_mode_v1();
        let perturb_at_phase: u32 = 17;
        let total = context::total_phase_count_v1();
        assert!(perturb_at_phase + 5 < total, "fixture premise: room to observe post-perturbation phases");

        let mut index_a = minimal_fixture_v1(11, 2);
        let mut index_b = minimal_fixture_v1(11, 2);
        let mut env_a = context::Environment::new().unwrap();
        let mut env_b = context::Environment::new().unwrap();
        let coin_index = GoodIndex::try_from(Good::Coin).expect("Coin is a valid Good");

        let mut first_divergence: Option<u32> = None;
        for phase in 0..total {
            let ev_a = context::tick_with_phase_evidence_v1(&mut index_a, phase, &mut env_a);
            if phase == perturb_at_phase
                && let Some(id) = index_b.sites.ids().next()
            {
                // A real state difference (the smallest representable
                // f32 nudge), not a rounding artifact -- unambiguous.
                let economy = index_b.sites.get_mut(id).economy_mut();
                let stock = &mut economy.stocks[coin_index];
                *stock = f32::from_bits(stock.to_bits() + 1);
            }
            let ev_b = context::tick_with_phase_evidence_v1(&mut index_b, phase, &mut env_b);

            if ev_a.root != ev_b.root {
                first_divergence = Some(phase);
                break;
            }
        }

        assert_eq!(
            first_divergence,
            Some(perturb_at_phase),
            "expected the perturbation to be localized to phase {perturb_at_phase}, got {first_divergence:?}"
        );
    }
}

/// `T8.3` chunk 1 (Lane B, order sensitivity): the provider/customer
/// pairing axis. `Economy::trade_at_site` (this module, `mod.rs`)
/// processes one provider's customer orders SEQUENTIALLY
/// (`for o in orders.drain(..)`), each mutating the SAME shared,
/// depleting `self.stocks` -- structurally the exact shape T8.3's own
/// doc names as the transactional-non-commutativity risk ("a stock is
/// consumed, a customer is served first"). This module lives here (not
/// in `context`) because it needs `Economy`'s own private fields
/// (`trade_at_site`/`TradeOrder`/`TradeDelivery`) directly -- a minimal,
/// reproducible fixture at the `trade_at_site` level, not a full
/// 2000-phase simulation, per the row's own "reproducible minimal
/// fixture" acceptance criterion. Evidence-only: this measures and
/// classifies, it does not canonicalize an order or otherwise fix
/// anything found here.
///
/// **Finding, CLASSIFIED NEGATIVE (not the positive the structural read
/// predicted) -- and holds BY CONSTRUCTION, not merely for the fixtures
/// tested.** Every test below permutes processing order across symmetric,
/// asymmetric, and cross-paying scarce-stock scenarios; in every case the
/// scarce-stock customer received the BIT-IDENTICAL amount regardless of
/// which order was processed first. `order_stock_ratio[g]` is computed
/// ONCE, from `total_orders[g]` (summed BEFORE the per-order loop starts)
/// against the ORIGINAL stock captured before any order is processed, as
/// `total_orders[g] / (stock[g] - next_demand[g])` -- so
/// `allocated_amount = amount / order_stock_ratio.max(1.0)` is a pure,
/// proportional function of each order's OWN amount and a ratio fixed for
/// the whole call.
///
/// The proof the `.min(self.stocks[*g])` clamp on `paid_amount` (which
/// DOES read the live, depleting stock) can never actually bite, for ANY
/// input, not just the ones tested: summing `allocated_amount` over every
/// order sharing good `g` telescopes to exactly `total_orders[g] /
/// order_stock_ratio[g].max(1.0)`, which is `(stock[g] - next_demand[g])`
/// when the ratio rations (`order_stock_ratio[g] > 1`, by direct
/// substitution) and at most that same bound when it doesn't (the ratio's
/// own `<= 1` condition is `total_orders[g] <= stock[g] - next_demand[g]`
/// by definition). Since `paid_amount <= allocated_amount` for every
/// individual order (the payment-balance subtraction only ever shrinks
/// it, never grows it), the cumulative amount drawn from `self.stocks[g]`
/// by any PREFIX of the processing order, plus the next order's own
/// `allocated_amount`, never exceeds `stock[g] - next_demand[g] <=
/// stock[g]` (`next_demand` is asserted `>= 0.0` at construction, twice,
/// above) -- so the live stock the clamp reads is always still large
/// enough. A cross-payment credit (`self.stocks[*g2] += amount2`, the one
/// path that could raise live stock mid-call instead of only depleting
/// it) only ever makes the bound MORE slack, never less. This is the same
/// class of result `T8.2`'s `world/src/lib.rs` chunk found for worldgen
/// RNG: the STRUCTURAL read (raw code shape) suggested a hazard; TRACING
/// the actual arithmetic proved it structurally absent, not merely absent
/// from the cases exercised.
///
/// **Cross-review addition** (Opus 5, `bastion/apex-t34`): the
/// cross-paying-customer fixture below, and the arithmetic proof above,
/// are the review's own finding -- flagged rather than silently folded
/// in, and independently re-derived and confirmed here before being
/// adopted into this doc, per this program's own "never trust a claim
/// without independent re-verification" discipline. The chunk's original
/// "not yet tested" list is now closed: the payment-side inner loop is
/// covered by `cross_paying_orders_are_also_order_independent` below, and
/// `collect_deliveries`'s reduction/last-writer sites were covered by
/// this row's own later chunks 2-3 (`collect_deliveries_last_writer_
/// keeps_the_last_delivery_not_the_first`,
/// `collect_deliveries_stock_accumulation_is_order_independent`).
#[cfg(test)]
mod t8_3_order_sensitivity_tests {
    use super::*;

    /// A provider whose stock of `good` is deliberately too small to
    /// satisfy every customer order that will be submitted against it --
    /// the scarcity `trade_at_site`'s own `order_stock_ratio` rationing
    /// exists to handle, and the mechanism through which processing
    /// order could in principle matter.
    fn scarce_provider_v1(good: GoodIndex, stock: f32) -> Economy {
        let mut economy = Economy::default();
        economy.stocks[good] = stock;
        economy
    }

    /// One customer's order: wants `want` units of `good` (positive =
    /// order, per `TradeOrder::amount`'s own doc comment), offers `pay`
    /// units of `coin` in exchange (negative = exchange).
    fn hungry_order_v1(customer: u64, good: GoodIndex, coin: GoodIndex, want: f32, pay: f32) -> TradeOrder {
        let mut amount = GoodMap::from_default(0.0);
        amount[good] = want;
        amount[coin] = -pay;
        TradeOrder { customer: Id::new(customer), amount }
    }

    fn delivered_amount_v1(
        deliveries: &DHashMap<Id<Site>, Vec<TradeDelivery>>,
        customer: u64,
        good: GoodIndex,
    ) -> f32 {
        deliveries
            .get(&Id::new(customer))
            .and_then(|v| v.first())
            .map(|d| d.amount[good])
            .unwrap_or(0.0)
    }

    /// Runs `trade_at_site` for two customer orders against a fresh
    /// provider with the given stock, in the given processing order, and
    /// returns what customer 100 received.
    fn customer_100_delivery_v1(stock: f32, orders_100_then_200: bool) -> f32 {
        let flour = GoodIndex::try_from(Good::Flour).expect("Flour is a valid Good");
        let coin = GoodIndex::try_from(Good::Coin).expect("Coin is a valid Good");
        let order_100 = hungry_order_v1(100, flour, coin, 50.0, 100.0);
        let order_200 = hungry_order_v1(200, flour, coin, 1.0, 2.0);
        let mut orders = if orders_100_then_200 { vec![order_100, order_200] } else { vec![order_200, order_100] };
        let mut economy = scarce_provider_v1(flour, stock);
        let mut deliveries: DHashMap<Id<Site>, Vec<TradeDelivery>> = DHashMap::default();
        economy.trade_at_site(Id::new(1), &mut orders, &mut deliveries);
        delivered_amount_v1(&deliveries, 100, flour)
    }

    /// REVIEW ADDITION (`T8.3` cross-review): the path the chunk's own
    /// doc named as untested -- a customer who PAYS in the same good
    /// another customer is BUYING.
    ///
    /// Why this is the axis that could still break the negative: the
    /// payment loop does `self.stocks[*g2] += amount2`, mutating LIVE
    /// stock mid-call, and `paid_amount` is clamped by
    /// `.min(self.stocks[*g])` -- the one place live, depleting stock
    /// reaches the delivered amount. If a payment credit lands in the
    /// same good slot a later customer draws from, the clamp could see
    /// a different stock depending on who was processed first. The
    /// chunk's fixtures all used distinct good/coin, so none of them
    /// could exercise it.
    ///
    /// Customer 100 buys Flour and pays Coin. Customer 200 buys Coin and
    /// pays FLOUR -- so 200's payment credits `stocks[Flour]`, the very
    /// slot 100 draws from.
    fn cross_paying_customer_100_delivery_v1(orders_100_then_200: bool) -> f32 {
        let flour = GoodIndex::try_from(Good::Flour).expect("Flour is a valid Good");
        let coin = GoodIndex::try_from(Good::Coin).expect("Coin is a valid Good");
        // 100 wants Flour, pays Coin. 200 wants Coin, pays Flour.
        let order_100 = hungry_order_v1(100, flour, coin, 50.0, 100.0);
        let order_200 = hungry_order_v1(200, coin, flour, 20.0, 40.0);
        let mut orders = if orders_100_then_200 {
            vec![order_100, order_200]
        } else {
            vec![order_200, order_100]
        };
        let mut economy = scarce_provider_v1(flour, 5.0);
        // The provider must also hold some Coin, or 200's order has no
        // ratio and the cross-payment never runs.
        economy.stocks[coin] = 30.0;
        let mut deliveries: DHashMap<Id<Site>, Vec<TradeDelivery>> = DHashMap::default();
        economy.trade_at_site(Id::new(1), &mut orders, &mut deliveries);

        // PRECONDITION, asserted rather than assumed: customer 200's
        // order must actually have been processed through the ratio
        // path, or no Flour payment was ever credited and this fixture
        // proves nothing about the clamp. A cross-payment test that
        // silently never cross-pays is a vacuous green -- the exact
        // shape this program keeps catching.
        assert!(
            delivered_amount_v1(&deliveries, 200, coin) > 0.0,
            "customer 200 received no Coin, so its Flour payment never ran and this fixture did              not exercise the cross-payment path it exists to test"
        );

        delivered_amount_v1(&deliveries, 100, flour)
    }

    /// The cross-review's own falsification target. A DIFFERENCE here
    /// would bound `T8.3` chunk 1's negative to the distinct-good case;
    /// equality extends it to the mixed case the chunk left open.
    #[test]
    fn cross_paying_orders_are_also_order_independent() {
        let when_first = cross_paying_customer_100_delivery_v1(true);
        let when_second = cross_paying_customer_100_delivery_v1(false);
        assert_eq!(
            when_first, when_second,
            "a customer paying in the good another customer buys routes a payment credit into              the live stock slot the clamp reads -- if processing order reached the delivered              amount anywhere, it would be here"
        );
    }

    /// Required test (this axis): a scarce-good order, permuted, is
    /// localized to a minimal `trade_at_site` fixture and classified.
    /// Two customers (IDs 100 and 200) with deliberately ASYMMETRIC
    /// wants (50 vs 1 units) against scarce stock (5.0, satisfying
    /// neither in full) -- run once with customer 100 processed first,
    /// once with 200 processed first, nothing else varied (the row's
    /// own "permute separately" discipline). RESULT: bit-identical
    /// (0.5098039 both times) -- CLASSIFIED NEGATIVE. See the module
    /// doc for why: `order_stock_ratio` is computed once, before any
    /// order is processed, making the allocation formula itself a pure
    /// function of the FIXED ratio rather than of processing order.
    #[test]
    fn a_scarce_good_allocation_is_order_independent_asymmetric() {
        let when_first = customer_100_delivery_v1(5.0, true);
        let when_second = customer_100_delivery_v1(5.0, false);
        assert_eq!(
            when_first, when_second,
            "trade_at_site's scarce-good allocation is expected to be bit-identical regardless \
             of processing order (order_stock_ratio is computed once, before any order runs) -- \
             a mismatch here would be a genuine transactional-non-commutativity finding, worth \
             re-flagging"
        );
    }

    /// Non-vacuity companion: the SYMMETRIC case (equal wants) is the
    /// simplest possible scarcity fixture -- confirms the same
    /// order-independence holds there too, not only in the asymmetric
    /// case above.
    #[test]
    fn a_scarce_good_allocation_is_order_independent_symmetric() {
        let flour = GoodIndex::try_from(Good::Flour).expect("Flour is a valid Good");
        let coin = GoodIndex::try_from(Good::Coin).expect("Coin is a valid Good");
        let make_orders = || vec![hungry_order_v1(100, flour, coin, 10.0, 20.0), hungry_order_v1(200, flour, coin, 10.0, 20.0)];

        let mut economy_a = scarce_provider_v1(flour, 5.0);
        let mut orders_a = make_orders();
        let mut deliveries_a: DHashMap<Id<Site>, Vec<TradeDelivery>> = DHashMap::default();
        economy_a.trade_at_site(Id::new(1), &mut orders_a, &mut deliveries_a);

        let mut economy_b = scarce_provider_v1(flour, 5.0);
        let mut orders_b = make_orders();
        orders_b.reverse();
        let mut deliveries_b: DHashMap<Id<Site>, Vec<TradeDelivery>> = DHashMap::default();
        economy_b.trade_at_site(Id::new(1), &mut orders_b, &mut deliveries_b);

        assert_eq!(
            delivered_amount_v1(&deliveries_a, 100, flour),
            delivered_amount_v1(&deliveries_b, 100, flour)
        );
    }

    /// Companion at the OTHER end: when the provider's stock is ample
    /// (comfortably exceeds both orders combined), processing order also
    /// must not matter -- the expected, unsurprising case, checked so
    /// the scarce-case findings above are read against a real contrast
    /// rather than assumed to differ from something untested.
    #[test]
    fn an_ample_good_order_is_order_independent() {
        let when_first = customer_100_delivery_v1(1000.0, true);
        let when_second = customer_100_delivery_v1(1000.0, false);
        assert_eq!(when_first, when_second);
    }
}

/// `T8.3` chunk 2 (Lane B, order sensitivity): the site-order axis
/// (`context.rs`'s `index.sites.par_iter_mut()`). The orchestrator's own
/// ruling for this axis: "prove the null the way this program proves
/// negatives -- an EXPERIMENT... not a reading; a null established by
/// test survives a future edit that a reading doesn't." Lives here (not
/// `context`) because it needs `Economy::stocks` directly, to give
/// otherwise-identical sites a distinguishing starting value.
#[cfg(test)]
mod t8_3_site_order_tests {
    use super::*;

    /// A fixture of sites with DISTINGUISHABLE starting flour stocks
    /// (so each site can be told apart after insertion order permutes
    /// which raw `Id` it's assigned), inserted in the given order.
    /// Returns the index plus a `(distinguishing stock, assigned Id)`
    /// map -- comparing "the same logical site" across two differently-
    /// ordered fixtures requires this map, since raw `Id` assignment
    /// itself depends on insertion order.
    fn fixture_with_order_v1(flour: GoodIndex, seed: u32, stocks_in_order: &[f32]) -> (crate::index::Index, Vec<(f32, Id<Site>)>) {
        let mut index = crate::index::Index::new(seed);
        let mut assigned = Vec::new();
        for &stock in stocks_in_order {
            let mut site = Site::default();
            site.kind = Some(crate::site::SiteKind::Refactor);
            site.economy_mut().stocks[flour] = stock;
            let id = index.sites.insert(site);
            assigned.push((stock, id));
        }
        (index, assigned)
    }

    /// Required test (site-order axis), PROVEN not assumed: the same
    /// three sites, inserted in reverse order (permuting which raw `Id`
    /// each is assigned, and therefore the order
    /// `index.sites.par_iter_mut()` visits them in), produce IDENTICAL
    /// per-phase economy state for each logical site across all 2000
    /// phases -- an experiment on the real 500-year simulation via
    /// `T8.1`'s own evidence collection, not a structural reading of
    /// `tick()`'s body.
    #[test]
    fn site_processing_order_does_not_change_any_sites_outcome() {
        context::enable_economy_phase_evidence_mode_v1();
        let flour = GoodIndex::try_from(Good::Flour).expect("Flour is a valid Good");

        let stocks = [10.0_f32, 25.0, 50.0];
        let mut reversed = stocks;
        reversed.reverse();

        let (mut index_forward, assigned_forward) = fixture_with_order_v1(flour, 7, &stocks);
        let (mut index_reversed, assigned_reversed) = fixture_with_order_v1(flour, 7, &reversed);

        let evidence_forward = context::simulate_with_phase_evidence_v1(&mut index_forward);
        let evidence_reversed = context::simulate_with_phase_evidence_v1(&mut index_reversed);
        assert_eq!(evidence_forward.len(), evidence_reversed.len());

        for &(stock, id_forward) in &assigned_forward {
            let id_reversed = assigned_reversed
                .iter()
                .find(|&&(s, _)| s == stock)
                .map(|&(_, id)| id)
                .expect("every logical site (by its distinguishing stock) exists in both fixtures");

            for (phase_forward, phase_reversed) in evidence_forward.iter().zip(evidence_reversed.iter()) {
                let digest_forward =
                    phase_forward.per_site.iter().find(|(id, _)| *id == id_forward.id()).map(|(_, d)| d);
                let digest_reversed =
                    phase_reversed.per_site.iter().find(|(id, _)| *id == id_reversed.id()).map(|(_, d)| d);
                assert_eq!(
                    digest_forward, digest_reversed,
                    "site starting with flour stock={stock} diverged at phase {} depending on \
                     insertion/processing order",
                    phase_forward.phase
                );
            }
        }
    }
}

/// `T8.3` chunk 3 (Lane B, order sensitivity): `collect_deliveries`'
/// reduction and last-writer sites.
///
/// **Reachability, checked FIRST per the orchestrator's own ruling
/// ("the reachability check comes first and its answer is a finding
/// either way").** `collect_deliveries` iterates `self.deliveries:
/// Vec<TradeDelivery>` and, for any two deliveries sharing the same
/// `supplier`, the SECOND overwrites the first's remembered
/// `last_values`/`last_supplies` via `mem::swap` (mod.rs:852-853) -- a
/// genuine last-writer field IF that scenario is reachable. Traced the
/// only path that populates `self.deliveries` for one site: `context.rs`'s
/// `tick()` (1) calls every site's `Economy::tick()` (which calls
/// `plan_trade_for_site` EXACTLY ONCE, pushing AT MOST one `TradeOrder`
/// per neighbor into `self.orders[neighbor]` -- `neighbor_orders` there
/// is a `GoodMap` per neighbor, not a multi-map, so one call cannot
/// itself create two orders to the same neighbor), THEN (2) drains
/// EVERY site's `self.orders` UNCONDITIONALLY in the same phase
/// (`context.rs:219`), before `trade_at_site` (which is what eventually
/// produces `TradeDelivery`s) ever runs on the drained result. Populate
/// and drain happen once per phase, in that order, for every site --
/// there is no live path today that lets `self.orders[neighbor]`
/// (and therefore a provider's incoming order list, and therefore
/// `self.deliveries` for a receiver) accumulate more than once per
/// supplier per phase. **CLASSIFIED DEAD-CODE-TODAY**, not
/// unreachable-forever: the `if o.len() < 100 { o.push(to) } else {
/// warn!(...) }` overflow guard at mod.rs:652-656 ("this is just to
/// catch unbound growth (happened in development)") is itself evidence
/// this WAS reachable under some prior calling shape. **Revival
/// precondition, named so a future edit can recognize it**: any change
/// that calls `Economy::tick()` (or `plan_trade_for_site` directly)
/// more than once per site per phase without an intervening
/// `self.orders.drain()`, or that removes the drain-every-phase
/// discipline in `context.rs::tick()`, re-opens this path.
///
/// The mechanism itself is tested directly below (bypassing the
/// unreachable-today call path, constructing the scenario by hand) --
/// proving what WOULD happen if the precondition above is ever met,
/// since "unreachable" describes today's callers, not the code's own
/// behavior.
#[cfg(test)]
mod t8_3_delivery_collection_tests {
    use super::*;

    /// Direct test of the last-writer mechanism at `collect_deliveries`
    /// mod.rs:852-853: two deliveries from the SAME supplier, pushed
    /// directly into `self.deliveries` (bypassing the unreachable-today
    /// call path per this module's own reachability finding), confirm
    /// the SECOND delivery's prices/supply win -- LAST-WRITER, not
    /// first, not merged.
    #[test]
    fn collect_deliveries_last_writer_keeps_the_last_delivery_not_the_first() {
        let flour = GoodIndex::try_from(Good::Flour).expect("Flour is a valid Good");
        let supplier = Id::new(1);

        let mut economy = Economy { neighbors: vec![NeighborInformation { id: supplier, last_values: GoodMap::default(), last_supplies: GoodMap::default() }], ..Default::default() };

        let mut first_prices = GoodMap::from_default(0.0);
        first_prices[flour] = 1.0;
        let mut second_prices = GoodMap::from_default(0.0);
        second_prices[flour] = 99.0;

        economy.deliveries.push(TradeDelivery { supplier, amount: GoodMap::from_default(0.0), prices: first_prices, supply: GoodMap::default() });
        economy.deliveries.push(TradeDelivery { supplier, amount: GoodMap::from_default(0.0), prices: second_prices, supply: GoodMap::default() });

        economy.collect_deliveries();

        let remembered = economy.neighbors.iter().find(|n| n.id == supplier).expect("neighbor still present");
        assert_eq!(
            remembered.last_values[flour], 99.0,
            "expected the SECOND (last-processed) delivery's price to win, not the first"
        );
    }

    /// Reduction-order test: THREE deliveries from three different
    /// suppliers, all crediting the same good's stock (`self.stocks[g]
    /// += *a`, mod.rs:860, a SEQUENTIAL left-fold over `self.deliveries`
    /// -- `((start + a) + b) + c`), run in two different orders --
    /// classifies whether float summation ASSOCIATIVITY (not just
    /// commutativity) produces a bit-exact-identical result or a
    /// ULP-scale divergence (REDUCTION-ROUNDING, distinct from the
    /// transactional class chunk 1 tested for). Deliberately three
    /// terms of wildly different magnitude (1e7, 1.0, 1e-7): a
    /// two-term test would be vacuous -- IEEE-754 addition is exactly
    /// commutative for a single pair (A+B == B+A always), so only
    /// association order (which pair adds first) can expose rounding,
    /// and that needs three-plus terms with enough magnitude spread for
    /// the smallest term to be at risk of absorption depending on when
    /// it's added.
    #[test]
    fn collect_deliveries_stock_accumulation_is_order_independent() {
        let flour = GoodIndex::try_from(Good::Flour).expect("Flour is a valid Good");
        let suppliers: Vec<Id<Site>> = (1..=3).map(Id::new).collect();
        let amounts = [1.0e7_f32, 1.0_f32, 1.0e-7_f32];

        let delivery = |supplier: Id<Site>, amount: f32| {
            let mut good_amount = GoodMap::from_default(0.0);
            good_amount[flour] = amount;
            TradeDelivery { supplier, amount: good_amount, prices: GoodMap::default(), supply: GoodMap::default() }
        };
        let neighbors = || {
            suppliers
                .iter()
                .map(|&id| NeighborInformation { id, last_values: GoodMap::default(), last_supplies: GoodMap::default() })
                .collect::<Vec<_>>()
        };

        let mut economy_forward = Economy { neighbors: neighbors(), ..Default::default() };
        for (&supplier, &amount) in suppliers.iter().zip(amounts.iter()) {
            economy_forward.deliveries.push(delivery(supplier, amount));
        }
        economy_forward.collect_deliveries();

        let mut economy_reversed = Economy { neighbors: neighbors(), ..Default::default() };
        for (&supplier, &amount) in suppliers.iter().rev().zip(amounts.iter().rev()) {
            economy_reversed.deliveries.push(delivery(supplier, amount));
        }
        economy_reversed.collect_deliveries();

        assert_eq!(
            economy_forward.stocks[flour], economy_reversed.stocks[flour],
            "expected stock accumulation across suppliers of wildly different magnitude to be \
             bit-exact order-independent; forward={} reversed={}",
            economy_forward.stocks[flour], economy_reversed.stocks[flour]
        );
    }
}

/// `T8.4` chunk 1 (Lane C, model sensitivity): the ULP-sensitivity
/// sweep, first field (`stocks`) -- perturb ONE site's flour stock by
/// one ULP at phase 0, hold executable and traversal fixed (single
/// fixture, single-threaded reasoning -- the whole point is isolating
/// MODEL sensitivity from the order/platform questions Lanes A and B
/// already own), and record the resulting SENSITIVITY CURVE: the raw
/// magnitude of divergence at every subsequent phase, not just whether
/// two runs' hashes differ (`T8.1`'s evidence gives phase-LOCALIZATION;
/// this lane needs phase-by-phase MAGNITUDE, which a hash cannot give
/// -- hence reading `Economy::stocks`/`values` directly here rather
/// than through `PhaseEconomyEvidenceV1`). Extends the same two-fixture
/// lockstep harness `T8.1` chunk 1's own perturbation test and `T8.3`'s
/// axis tests already established.
///
/// Also tracks the first BRANCH CROSSING: `Economy::tick()`'s value-
/// rationalisation step (mod.rs, `values[good] = if val > 0.001 &&
/// val < 1000.0 { Some(...) } else { None }`) is a REAL conditional
/// whose taken/not-taken outcome is directly observable as a
/// `Some`/`None` transition -- the first phase where the perturbed and
/// baseline runs disagree on that Option's variant is a genuine branch
/// crossing, not a proxy for one.
///
/// Evidence-only: this measures and records sensitivity for `T8.5`'s
/// later remedy ladder. Nothing here stabilizes, quantizes, or
/// otherwise changes the model.
#[cfg(test)]
mod t8_4_model_sensitivity_tests {
    use super::*;

    fn fixture_v1(seed: u32) -> crate::index::Index {
        let mut index = crate::index::Index::new(seed);
        let mut site = Site::default();
        site.kind = Some(crate::site::SiteKind::Refactor);
        let _ = site.economy_mut();
        index.sites.insert(site);
        index
    }

    /// One phase's raw observation: the flour stock and whether flour
    /// has a `Some` value (the branch outcome).
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct RawObservationV1 {
        stock: f32,
        has_value: bool,
    }

    fn observe_v1(index: &crate::index::Index, site_id: Id<Site>, flour: GoodIndex) -> RawObservationV1 {
        let economy = index.sites.get(site_id).economy.as_ref().expect("fixture site always has an Economy");
        RawObservationV1 { stock: economy.stocks[flour], has_value: economy.values[flour].is_some() }
    }

    /// Runs a baseline and (optionally) one-ULP-perturbed fixture in
    /// lockstep for `phases` phases, returning: the per-phase magnitude
    /// curve (`|perturbed.stock - baseline.stock|`), and the first phase
    /// (if any) where the two runs' `values[flour]` Option variant
    /// diverges (a branch crossing).
    fn sensitivity_curve_v1(seed: u32, perturb: bool, phases: u32) -> (Vec<f32>, Option<u32>) {
        context::enable_economy_phase_evidence_mode_v1();
        let flour = GoodIndex::try_from(Good::Flour).expect("Flour is a valid Good");

        let mut index_baseline = fixture_v1(seed);
        let mut index_perturbed = fixture_v1(seed);
        let site_id = *index_baseline.sites.ids().next().as_ref().expect("fixture has one site");
        assert_eq!(site_id, *index_perturbed.sites.ids().next().as_ref().unwrap(), "both fixtures assign the same Id from the same seed/insertion sequence");

        if perturb {
            let economy = index_perturbed.sites.get_mut(site_id).economy_mut();
            economy.stocks[flour] = f32::from_bits(economy.stocks[flour].to_bits() + 1);
        }

        let mut env_baseline = context::Environment::new().unwrap();
        let mut env_perturbed = context::Environment::new().unwrap();
        let mut curve = Vec::new();
        let mut first_branch_crossing = None;

        for phase in 0..phases {
            context::tick_with_phase_evidence_v1(&mut index_baseline, phase, &mut env_baseline);
            context::tick_with_phase_evidence_v1(&mut index_perturbed, phase, &mut env_perturbed);

            let baseline = observe_v1(&index_baseline, site_id, flour);
            let perturbed = observe_v1(&index_perturbed, site_id, flour);
            curve.push((perturbed.stock - baseline.stock).abs());
            if first_branch_crossing.is_none() && baseline.has_value != perturbed.has_value {
                first_branch_crossing = Some(phase);
            }
        }
        (curve, first_branch_crossing)
    }

    /// Required test: perturbation harness reproducibility -- the same
    /// perturbation, run twice, gives the same curve. The economy
    /// simulation itself reads no RNG in its hot path (`tick`,
    /// `plan_trade_for_site`, `trade_at_site`, `collect_deliveries` --
    /// checked, only `#[cfg(test)]` full-worldgen scaffolding uses
    /// `rand`), so this is expected to hold exactly, not approximately;
    /// still worth proving rather than assuming, since a harness that
    /// can't reproduce its own curve can't be trusted for the rest of
    /// this lane.
    #[test]
    fn the_same_perturbation_twice_gives_the_same_curve() {
        let (curve_a, crossing_a) = sensitivity_curve_v1(21, true, 50);
        let (curve_b, crossing_b) = sensitivity_curve_v1(21, true, 50);
        assert_eq!(curve_a, curve_b);
        assert_eq!(crossing_a, crossing_b);
    }

    /// Required test: a null perturbation (the flag is false, so the
    /// "perturbed" fixture is built identically to the baseline)
    /// produces a zero curve -- the harness's own sanity floor.
    #[test]
    fn a_null_perturbation_produces_a_zero_curve() {
        let (curve, crossing) = sensitivity_curve_v1(21, false, 50);
        assert!(curve.iter().all(|&d| d == 0.0), "expected an all-zero curve for a null perturbation, got {curve:?}");
        assert_eq!(crossing, None);
    }

    /// Required test: at least one known-unstable threshold is found,
    /// or the sweep's own coverage is proven insufficient rather than
    /// assumed adequate. Runs the real one-ULP perturbation over a
    /// window long enough to observe the tier's own central
    /// measurement: does a one-ULP difference at phase 0 stay BOUNDED
    /// (the cheapest T8.5 remedies suffice) or grow UNBOUNDED (the
    /// model is chaotic, no ordering fix saves it)? Records which,
    /// with the actual curve as evidence either way -- this test
    /// cannot fail by construction (both outcomes are informative), it
    /// can only fail to have RUN, which the two required tests above
    /// already guard against.
    #[test]
    fn a_one_ulp_perturbation_produces_a_recorded_sensitivity_verdict() {
        let phases = 200;
        let (curve, first_branch_crossing) = sensitivity_curve_v1(21, true, phases);

        let first_nonzero_phase = curve.iter().position(|&d| d > 0.0);
        let final_magnitude = *curve.last().expect("phases > 0");
        let max_magnitude = curve.iter().cloned().fold(0.0_f32, f32::max);

        println!(
            "T8.4 chunk 1 sensitivity verdict (seed=21, site flour stock, {phases} phases): \
             first_nonzero_phase={first_nonzero_phase:?} first_branch_crossing={first_branch_crossing:?} \
             final_magnitude={final_magnitude} max_magnitude={max_magnitude}"
        );

        // The sweep's own coverage claim: a one-ULP perturbation must
        // become OBSERVABLE somewhere in this window, or this fixture
        // (single site, no trade partners, INTER_SITE_TRADE's cross-
        // site paths never engaged) is too inert to be evidence at all
        // -- proven insufficient rather than assumed adequate, per the
        // required test's own wording.
        assert!(
            first_nonzero_phase.is_some(),
            "the one-ULP perturbation never became observable in the flour stock over {phases} \
             phases on this single-site fixture -- coverage insufficient, NOT a stability \
             finding: a fixture with no trade partners never re-reads its own perturbed stock \
             through any state-dependent branch, so this sweep needs a multi-site fixture (T8.1/ \
             T8.3's own 2-3 site shape) to actually exercise the model"
        );
    }

    /// Chunk 2: the sweep's second named field, `population` (`pop`) --
    /// a genuinely different state VARIABLE from `stocks` (population
    /// feeds `demand`/`supply` as a multiplier, `self.labors[labor] *
    /// self.pop`, rather than being read/written as a stock itself), so
    /// its sensitivity is not assumed to match `stocks`' -- checked
    /// independently, its own mechanism, its own verdict.
    fn population_sensitivity_curve_v1(seed: u32, perturb: bool, phases: u32) -> (Vec<f32>, ()) {
        context::enable_economy_phase_evidence_mode_v1();

        let mut index_baseline = fixture_v1(seed);
        let mut index_perturbed = fixture_v1(seed);
        let site_id = *index_baseline.sites.ids().next().as_ref().expect("fixture has one site");

        if perturb {
            let economy = index_perturbed.sites.get_mut(site_id).economy_mut();
            economy.pop = f32::from_bits(economy.pop.to_bits() + 1);
        }

        let mut env_baseline = context::Environment::new().unwrap();
        let mut env_perturbed = context::Environment::new().unwrap();
        let mut curve = Vec::new();
        for phase in 0..phases {
            context::tick_with_phase_evidence_v1(&mut index_baseline, phase, &mut env_baseline);
            context::tick_with_phase_evidence_v1(&mut index_perturbed, phase, &mut env_perturbed);
            let pop_baseline = index_baseline.sites.get(site_id).economy.as_ref().unwrap().pop;
            let pop_perturbed = index_perturbed.sites.get(site_id).economy.as_ref().unwrap().pop;
            curve.push((pop_perturbed - pop_baseline).abs());
        }
        (curve, ())
    }

    #[test]
    fn population_sensitivity_reproducibility_and_null_and_verdict() {
        // Reproducibility.
        let (curve_a, _) = population_sensitivity_curve_v1(33, true, 50);
        let (curve_b, _) = population_sensitivity_curve_v1(33, true, 50);
        assert_eq!(curve_a, curve_b);

        // Null perturbation.
        let (null_curve, _) = population_sensitivity_curve_v1(33, false, 50);
        assert!(null_curve.iter().all(|&d| d == 0.0), "expected an all-zero population curve for a null perturbation, got {null_curve:?}");

        // Verdict, same coverage discipline as the stock sweep.
        let phases = 200;
        let (curve, _) = population_sensitivity_curve_v1(33, true, phases);
        let first_nonzero_phase = curve.iter().position(|&d| d > 0.0);
        let final_magnitude = *curve.last().expect("phases > 0");
        let max_magnitude = curve.iter().cloned().fold(0.0_f32, f32::max);
        println!(
            "T8.4 chunk 2 sensitivity verdict (seed=33, site population, {phases} phases): \
             first_nonzero_phase={first_nonzero_phase:?} final_magnitude={final_magnitude} \
             max_magnitude={max_magnitude}"
        );
        assert!(
            first_nonzero_phase.is_some(),
            "the one-ULP population perturbation never became observable over {phases} phases -- \
             coverage insufficient, not a stability finding"
        );
    }

    /// Generic two-fixture, lockstep sensitivity curve: `setup` prepares
    /// BOTH fixtures identically (e.g. give a field a real starting
    /// value before perturbing it -- several of the remaining swept
    /// fields default to `None`/`0.0`, which can't be meaningfully
    /// ULP-nudged), `perturb` nudges only the perturbed fixture by one
    /// ULP, `observe` reads the tracked value back out each phase.
    /// Chunks 1-2 (stock, population) predate this generic form and are
    /// left as-is rather than retrofitted -- their own tests already
    /// pass and a retrofit risks silently changing what they measure.
    fn generic_sensitivity_curve_v1(
        seed: u32,
        perturb: bool,
        phases: u32,
        setup: impl Fn(&mut Economy),
        perturb_fn: impl Fn(&mut Economy),
        observe: impl Fn(&Economy) -> f32,
    ) -> Vec<f32> {
        context::enable_economy_phase_evidence_mode_v1();
        let mut index_baseline = fixture_v1(seed);
        let mut index_perturbed = fixture_v1(seed);
        let site_id = *index_baseline.sites.ids().next().as_ref().expect("fixture has one site");

        setup(index_baseline.sites.get_mut(site_id).economy_mut());
        setup(index_perturbed.sites.get_mut(site_id).economy_mut());
        if perturb {
            perturb_fn(index_perturbed.sites.get_mut(site_id).economy_mut());
        }

        let mut env_baseline = context::Environment::new().unwrap();
        let mut env_perturbed = context::Environment::new().unwrap();
        let mut curve = Vec::new();
        for phase in 0..phases {
            context::tick_with_phase_evidence_v1(&mut index_baseline, phase, &mut env_baseline);
            context::tick_with_phase_evidence_v1(&mut index_perturbed, phase, &mut env_perturbed);
            let a = observe(index_baseline.sites.get(site_id).economy.as_ref().unwrap());
            let b = observe(index_perturbed.sites.get(site_id).economy.as_ref().unwrap());
            curve.push((b - a).abs());
        }
        curve
    }

    fn verdict_v1(label: &str, curve: &[f32]) -> (Option<usize>, f32, f32) {
        let first_nonzero_phase = curve.iter().position(|&d| d > 0.0);
        let final_magnitude = *curve.last().expect("phases > 0");
        let max_magnitude = curve.iter().cloned().fold(0.0_f32, f32::max);
        println!(
            "T8.4 sensitivity verdict ({label}, {} phases): first_nonzero_phase={first_nonzero_phase:?} \
             final_magnitude={final_magnitude} max_magnitude={max_magnitude}",
            curve.len()
        );
        (first_nonzero_phase, final_magnitude, max_magnitude)
    }

    /// A perturbation applied AFTER a `warmup` of identical, unperturbed
    /// phases -- required for fields that start `None`/at a degenerate
    /// value and only reach a real, perturbable state after the model
    /// has run a few phases (found empirically for `price`: `Good::
    /// Flour`'s `values` entry stays `None` for this fixture's whole
    /// run -- the value-rationalisation `val` never lands in
    /// `(0.001, 1000.0)` for a good nobody's demand table references
    /// directly -- while `Good::Food`, which IS in `get_orders_everyone
    /// ()`'s "everyone needs this" list, reaches `Some` from phase 1
    /// onward; traced via a direct multi-phase dump before committing to
    /// this shape, not assumed).
    fn warmup_then_perturb_curve_v1(
        seed: u32,
        warmup_phases: u32,
        perturb: bool,
        post_phases: u32,
        perturb_fn: impl Fn(&mut Economy),
        observe: impl Fn(&Economy) -> f32,
    ) -> Vec<f32> {
        context::enable_economy_phase_evidence_mode_v1();
        let mut index_baseline = fixture_v1(seed);
        let mut index_perturbed = fixture_v1(seed);
        let site_id = *index_baseline.sites.ids().next().as_ref().expect("fixture has one site");

        let mut env_baseline = context::Environment::new().unwrap();
        let mut env_perturbed = context::Environment::new().unwrap();
        for phase in 0..warmup_phases {
            context::tick_with_phase_evidence_v1(&mut index_baseline, phase, &mut env_baseline);
            context::tick_with_phase_evidence_v1(&mut index_perturbed, phase, &mut env_perturbed);
        }
        if perturb {
            perturb_fn(index_perturbed.sites.get_mut(site_id).economy_mut());
        }

        let mut curve = Vec::new();
        for phase in warmup_phases..(warmup_phases + post_phases) {
            context::tick_with_phase_evidence_v1(&mut index_baseline, phase, &mut env_baseline);
            context::tick_with_phase_evidence_v1(&mut index_perturbed, phase, &mut env_perturbed);
            let a = observe(index_baseline.sites.get(site_id).economy.as_ref().unwrap());
            let b = observe(index_perturbed.sites.get(site_id).economy.as_ref().unwrap());
            curve.push((b - a).abs());
        }
        curve
    }

    /// Chunk 3: `price` (`values[good]`). Uses `Good::Food` rather than
    /// `Good::Flour` (chunks 1/2/5's good) -- traced first: `Flour`'s
    /// `values` entry stays `None` for this fixture's entire run (no
    /// direct consumer demand references it), so ANY perturbation is
    /// discarded identically to `surplus`'s wholesale overwrite, which
    /// would test the SAME finding twice under a different name rather
    /// than price's own dynamics. `Food` (in everyone's base demand)
    /// reaches a real, evolving `Some` price from phase 1 onward. A
    /// literal one-ULP nudge (tried first) never became observable --
    /// same magnitude-dominance rounding-absorption chunk 4 (labors)
    /// found and named; a 1e-3 quantisation unit (this chunk's own
    /// smoothing sibling's own size) propagates cleanly.
    #[test]
    fn price_sensitivity_reproducibility_and_null_and_verdict() {
        let food = GoodIndex::try_from(Good::Food).expect("Food is a valid Good");
        let perturb = move |e: &mut Economy| {
            let v = e.values[food].expect("warmed up: food has a real price by phase 2");
            e.values[food] = Some(v + 0.001);
        };
        let observe = move |e: &Economy| e.values[food].unwrap_or(0.0);

        let curve_a = warmup_then_perturb_curve_v1(41, 2, true, 50, perturb, observe);
        let curve_b = warmup_then_perturb_curve_v1(41, 2, true, 50, perturb, observe);
        assert_eq!(curve_a, curve_b, "reproducibility");

        let null_curve = warmup_then_perturb_curve_v1(41, 2, false, 50, perturb, observe);
        assert!(null_curve.iter().all(|&d| d == 0.0), "expected an all-zero price curve for a null perturbation, got {null_curve:?}");

        let curve = warmup_then_perturb_curve_v1(41, 2, true, 200, perturb, observe);
        let (first_nonzero_phase, _final, _max) = verdict_v1("seed=41, site food price (warmed up 2 phases), 1e-3 unit", &curve);
        assert!(first_nonzero_phase.is_some(), "the food price quantisation-unit perturbation never became observable -- coverage insufficient, not a stability finding");
    }

    /// Chunk 4: `demand`. `demand: GoodMap<f32>` in `Economy::tick()` is
    /// an EPHEMERAL local, recomputed from `labors`/`pop`/`orders`
    /// every phase -- not persisted state, so there is no `self.demand`
    /// field to perturb directly (a structural finding in itself,
    /// disclosed rather than silently substituting something else).
    /// Swept via `labors[labor]`, the persistent field demand is
    /// DERIVED FROM (`demand[good] += *amount * workers` where
    /// `workers = self.labors[labor] * self.pop`) -- perturbing the
    /// source and observing propagation is the only way to test this
    /// named quantity's sensitivity at all.
    ///
    /// A literal one-ULP nudge to `labors` (tried first) never became
    /// observable -- traced, not assumed: `labors[labor]` itself moves
    /// by ~0.01-0.03 EVERY phase from its own smoothing recombination
    /// (`smooth * OLD + (1-smooth) * fresh_ratio_term`), so a
    /// ~3e-9-scale ULP addition to the OLD term is rounded away by
    /// floating-point addition against the much larger fresh term
    /// before the result is even stored -- a genuine numerical-
    /// precision finding, not a coverage gap. Re-run with a 1e-3
    /// "quantisation unit" (the tier's own named alternative to a bare
    /// ULP) DOES propagate, with a clean decaying curve close to the
    /// formula's own `smooth=0.8` damping rate.
    #[test]
    fn demand_sensitivity_reproducibility_and_null_and_verdict() {
        let some_labor = Labor::list().next().expect("at least one labor exists");
        let setup = move |_: &mut Economy| {};
        let perturb = move |e: &mut Economy| {
            let l = e.labors[some_labor];
            e.labors[some_labor] = l + 0.001;
        };
        let observe = move |e: &Economy| e.labors[some_labor];

        let curve_a = generic_sensitivity_curve_v1(53, true, 50, setup, perturb, observe);
        let curve_b = generic_sensitivity_curve_v1(53, true, 50, setup, perturb, observe);
        assert_eq!(curve_a, curve_b, "reproducibility");

        let null_curve = generic_sensitivity_curve_v1(53, false, 50, setup, perturb, observe);
        assert!(null_curve.iter().all(|&d| d == 0.0), "expected an all-zero labor curve for a null perturbation, got {null_curve:?}");

        let curve = generic_sensitivity_curve_v1(53, true, 200, setup, perturb, observe);
        let (first_nonzero_phase, _final, _max) = verdict_v1("seed=53, site labors (demand's source field), 1e-3 quantisation unit", &curve);
        assert!(first_nonzero_phase.is_some(), "the labors quantisation-unit perturbation never became observable -- coverage insufficient, not a stability finding");
    }

    /// Chunk 5: `surplus` (`self.surplus: GoodMap<f32>`) -- a STRUCTURAL
    /// finding, not a sensitivity curve. Read the code before assuming
    /// this field carries state the way `stocks`/`pop` do: `Economy::
    /// tick()`'s first act on `surplus` is `self.surplus =
    /// demand.map(|g, demand| supply[g] + stocks[g] - demand)` -- a
    /// WHOLESALE assignment via `GoodMap::map`, not `+=` or any other
    /// form that reads the field's own prior value. Nothing earlier in
    /// `tick()` reads `self.surplus` either (the similarly-named
    /// `old_coin_surplus` is a FRESH local computed from
    /// `stocks`/`demand`, not from `self.surplus`). So `self.surplus`
    /// is a per-tick CACHE, fully recomputed from other state every
    /// phase, not an integrator a perturbation could ride forward in --
    /// a direct ULP perturbation to it is discarded before anything
    /// else in `tick()` ever reads it, by construction, not because the
    /// model damped it. Proven by TEST below (not just cited), since
    /// this module's own discipline is trace-and-verify, not read-and-
    /// assert.
    #[test]
    fn surplus_perturbation_is_discarded_by_the_unconditional_per_tick_overwrite() {
        let flour = GoodIndex::try_from(Good::Flour).expect("Flour is a valid Good");
        let setup = move |e: &mut Economy| e.surplus[flour] = 1.0;
        let perturb = move |e: &mut Economy| {
            let s = e.surplus[flour];
            e.surplus[flour] = f32::from_bits(s.to_bits() + 1);
        };
        let observe = move |e: &Economy| e.surplus[flour];

        let curve = generic_sensitivity_curve_v1(67, true, 5, setup, perturb, observe);
        verdict_v1("seed=67, site flour surplus (expected: discarded on tick 1)", &curve);
        assert!(
            curve.iter().all(|&d| d == 0.0),
            "expected surplus's perturbation to be discarded by tick()'s unconditional \
             self.surplus = demand.map(..) overwrite on the VERY FIRST phase; a nonzero curve \
             here would mean surplus is NOT wholesale-recomputed after all, contradicting the \
             code read -- worth re-checking immediately if this ever fails: {curve:?}"
        );
    }

    /// Chunk 6 (LAST, extra scrutiny per the orchestrator's own ruling):
    /// `smoothing` -- `values[good] = smooth * values[good].unwrap_or(val)
    /// + (1.0 - smooth) * val` (mod.rs, `smooth = 0.8`) is a first-order
    /// IIR filter on `values`, the SAME field `price` (chunk 3) swept --
    /// but this chunk asks a DIFFERENT question: not "does a price
    /// perturbation become observable" (already answered), but "does
    /// the smoothing recurrence ITSELF damp a perturbation at
    /// (approximately) its own designed rate, or does it integrate one
    /// upward". A geometric damping filter with factor `smooth` should
    /// roughly halve-life the perturbation roughly every
    /// `ln(2)/ln(1/smooth) ~= 3.1` phases if `val` (this phase's fresh
    /// signal) does not itself depend on the OLD smoothed value in a
    /// way that re-injects the perturbation -- checked over a LONG
    /// window (500 phases, not 200) specifically to give slow
    /// amplification room to show up if present, read with more
    /// suspicion than the other five fields per the ruling.
    #[test]
    fn smoothing_sensitivity_reproducibility_and_null_and_long_horizon_verdict() {
        // Good::Food, not Flour, for the same traced reason as chunk 3
        // (price): Flour's values entry never leaves None in this
        // fixture, and needs a 1e-3 quantisation unit rather than a
        // bare ULP for the same reason chunk 4 (labors) did -- the
        // smoothing recombination's fresh term dominates a literal
        // ULP-scale addition into rounding-away.
        let food = GoodIndex::try_from(Good::Food).expect("Food is a valid Good");
        let perturb = move |e: &mut Economy| {
            let v = e.values[food].expect("warmed up: food has a real price by phase 1");
            e.values[food] = Some(v + 0.001);
        };
        let observe = move |e: &Economy| e.values[food].unwrap_or(0.0);

        let curve_a = warmup_then_perturb_curve_v1(79, 2, true, 50, perturb, observe);
        let curve_b = warmup_then_perturb_curve_v1(79, 2, true, 50, perturb, observe);
        assert_eq!(curve_a, curve_b, "reproducibility");

        let null_curve = warmup_then_perturb_curve_v1(79, 2, false, 50, perturb, observe);
        assert!(null_curve.iter().all(|&d| d == 0.0), "expected an all-zero smoothing curve for a null perturbation, got {null_curve:?}");

        let phases = 500;
        let curve = warmup_then_perturb_curve_v1(79, 2, true, phases, perturb, observe);
        let (first_nonzero_phase, final_magnitude, max_magnitude) = verdict_v1("seed=79, site food price/smoothing, 1e-3 unit, LONG HORIZON", &curve);
        assert!(first_nonzero_phase.is_some(), "the smoothing quantisation-unit perturbation never became observable -- coverage insufficient, not a stability finding");

        // The suspicion this chunk was assigned, made concrete: a
        // healthy damping filter's magnitude should be MONOTONE
        // non-increasing after its initial transient, and its FINAL
        // magnitude must not exceed its early-window magnitude --
        // amplification would show up as final_magnitude growing past
        // an early checkpoint, which a bare "is it bounded" check
        // (comparing only to the injected ULP) would not catch if the
        // whole curve drifted upward together.
        let early_checkpoint = curve[9.min(curve.len() - 1)];
        assert!(
            final_magnitude <= early_checkpoint.max(max_magnitude.min(early_checkpoint * 2.0)),
            "SUSPECTED AMPLIFICATION: smoothing curve's final magnitude ({final_magnitude}) \
             exceeds its early-window magnitude ({early_checkpoint}) by more than the sweep's \
             own tolerance -- this is the finding T8.5 needs escalated, not smoothed over"
        );
    }
}
