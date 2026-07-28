// Example for calculating a drop rate:
//
// On every roll an f32 between 0 and 1 is created.
// For every loot table a total range is created by the sum of the individual
// ranges per item.
//
// This range is the sum of all single ranges defined per item in a table.
//                                                   // Individual Range
// (3, "common.items.food.cheese"),                  // 0.0..3.0
// (3, "common.items.food.apple"),                   // 3.0..6.0
// (3, "common.items.food.mushroom"),                // 6.0..9.0
// (1, "common.items.food.coconut"),                 // 9.0..10.0
// (0.05, "common.items.food.apple_mushroom_curry"), // 10.0..10.05
// (0.10, "common.items.food.apple_stick"),          // 10.05..10.15
// (0.10, "common.items.food.mushroom_stick"),       // 10.15..10.25
//
// The f32 is multiplied by the max. value needed to drop an item in this
// particular table. X = max. value needed = 10.15
//
// Example roll
// [Random Value 0..1] * X = Number inside the table's total range
// 0.45777 * X = 4.65
// 4.65 is in the range of 3.0..6.0 => Apple drops
//
// Example drop chance calculation
// Cheese drop rate = 3/X = 29.6%
// Coconut drop rate = 1/X = 9.85%

use std::{borrow::Cow, hash::Hash};

use crate::{
    assets::{AssetExt, BoxedError, FileAsset, load_ron},
    comp::{Item, inventory::item},
    state_hash::DomainHasher,
};
use rand::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tracing::warn;

/// E6 (determinism audit): seeds a loot roll from the causal event that
/// triggered it (attacker/collector uid, world position, sim time — whatever
/// the call site actually has), so [`LootSpec::to_items`] never falls back to
/// ambient OS entropy. Each `field` is one already-canonical byte slice
/// (caller's responsibility, same length-prefixing contract as
/// [`DomainHasher::field`]).
pub fn seed_loot_roll(fields: &[&[u8]]) -> u64 {
    let mut h = DomainHasher::new("bastion/domain/loot-roll/v1/sha256");
    for f in fields {
        h.field(f);
    }
    u64::from_le_bytes(h.finish().0[..8].try_into().expect("sha256 digest >= 8 bytes"))
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Lottery<T> {
    items: Vec<(f32, T)>,
    total: f32,
}

impl<T: DeserializeOwned + Send + Sync + 'static> FileAsset for Lottery<T> {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        load_ron::<Vec<(f32, T)>>(&bytes).map(Vec::into)
    }
}

impl<T> From<Vec<(f32, T)>> for Lottery<T> {
    fn from(mut items: Vec<(f32, T)>) -> Lottery<T> {
        let mut total = 0.0;

        for (rate, _) in &mut items {
            total += *rate;
            *rate = total - *rate;
        }

        Self { items, total }
    }
}

impl<T> Lottery<T> {
    pub fn choose_seeded(&self, seed: u32) -> &T {
        // RNG-P3-003 (determinism audit): use the seed's FULL 32-bit range —
        // the old `% 65536` collapsed every seed into 65,536 outcomes before
        // scaling by the cumulative weights, so entries whose weight
        // boundaries were finer than total/65536 could never be selected.
        // f64 keeps all 32 bits exact through the division; the quotient is
        // strictly < 1.0 so x < total (the end-index case stays impossible).
        let x = ((seed as f64 / (u32::MAX as f64 + 1.0)) * self.total as f64) as f32;
        &self.items[self
            .items
            .binary_search_by(|(y, _)| y.partial_cmp(&x).unwrap())
            .unwrap_or_else(|i| i.saturating_sub(1))]
        .1
    }

    pub fn iter(&self) -> impl Iterator<Item = &(f32, T)> { self.items.iter() }

    pub fn total(&self) -> f32 { self.total }
}

/// Try to distribute stacked items fairly between weighted participants.
pub fn distribute_many<T: Copy + Eq + Hash, I>(
    participants: impl IntoIterator<Item = (f32, T)>,
    rng: &mut impl Rng,
    items: &[I],
    mut get_amount: impl FnMut(&I) -> u32,
    mut exec_item: impl FnMut(&I, T, u32),
) {
    struct Participant<T> {
        // weight / total
        weight: f32,
        sorted_weight: f32,
        data: T,
        recieved_count: u32,
        current_recieved_count: u32,
    }

    impl<T> Participant<T> {
        fn give(&mut self, amount: u32) {
            self.current_recieved_count += amount;
            self.recieved_count += amount;
        }
    }

    // Nothing to distribute, we can return early.
    if items.is_empty() {
        return;
    }

    let mut total_weight = 0.0;

    // RNG-P3-006 (determinism audit): CANONICAL participant order. The f32
    // cumulative prefix sums (and thus which participant each draw maps to)
    // depended on the caller's insertion order — the live caller iterates a
    // HashMap, so the same draws could award loot to different participants
    // run-to-run. Sort by (weight, stable identity hash) — a total order
    // using only the existing T: Hash bound (stable_hash_u64 is the
    // DET-ADD-008 version-stable hasher), making the interval assignment a
    // pure function of the participant SET.
    let mut canonical: Vec<(f32, T)> = participants.into_iter().collect();
    canonical.sort_by(|a, b| {
        a.0.total_cmp(&b.0).then_with(|| {
            crate::state_hash::stable_hash_u64("bastion/domain/loot-participant/v1", &a.1)
                .cmp(&crate::state_hash::stable_hash_u64(
                    "bastion/domain/loot-participant/v1",
                    &b.1,
                ))
        })
    });

    let mut participants = canonical
        .into_iter()
        .map(|(weight, participant)| Participant {
            weight,
            sorted_weight: {
                total_weight += weight;
                total_weight - weight
            },
            data: participant,
            recieved_count: 0,
            current_recieved_count: 0,
        })
        .collect::<Vec<_>>();

    let total_item_amount = items.iter().map(&mut get_amount).sum::<u32>();

    let mut current_total_weight = total_weight;

    for item in items.iter() {
        let amount = get_amount(item);
        let mut distributed = 0;

        let Some(mut give) = participants
            .iter()
            .map(|participant| {
                (total_item_amount as f32 * participant.weight / total_weight).ceil() as u32
                    - participant.recieved_count
            })
            .min()
        else {
            tracing::error!("Tried to distribute items to no participants.");
            return;
        };

        while distributed < amount {
            // Can't give more than amount, and don't give more than the average between all
            // to keep things well distributed.
            let max_give = (amount / participants.len() as u32).clamp(1, amount - distributed);
            give = give.clamp(1, max_give);
            let x = rng.random_range(0.0..=current_total_weight);

            let index = participants
                .binary_search_by(|item| item.sorted_weight.partial_cmp(&x).unwrap())
                .unwrap_or_else(|i| i.saturating_sub(1));

            let participant_count = participants.len();

            let Some(winner) = participants.get_mut(index) else {
                tracing::error!("Tried to distribute items to no participants.");
                return;
            };

            winner.give(give);
            distributed += give;

            // If a participant has received enough, remove it.
            if participant_count > 1
                && winner.recieved_count as f32 / total_item_amount as f32
                    >= winner.weight / total_weight
            {
                current_total_weight = index
                    .checked_sub(1)
                    .and_then(|i| Some(participants.get(i)?.sorted_weight))
                    .unwrap_or(0.0);
                let winner = participants.swap_remove(index);
                exec_item(item, winner.data, winner.current_recieved_count);

                // Keep participant weights correct so that we can binary search it.
                for participant in &mut participants[index..] {
                    current_total_weight += participant.weight;
                    participant.sorted_weight = current_total_weight - participant.weight;
                }

                // Update max item give amount.
                give = participants
                    .iter()
                    .map(|participant| {
                        (total_item_amount as f32 * participant.weight / total_weight).ceil() as u32
                            - participant.recieved_count
                    })
                    .min()
                    .unwrap_or(0);
            } else {
                give = give.min(
                    (total_item_amount as f32 * winner.weight / total_weight).ceil() as u32
                        - winner.recieved_count,
                );
            }
        }
        for participant in participants.iter_mut() {
            if participant.current_recieved_count != 0 {
                exec_item(item, participant.data, participant.current_recieved_count);
                participant.current_recieved_count = 0;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[rustfmt::skip] // breaks doc comments
#[derive(Default)]
pub enum LootSpec<T: AsRef<str>> {
    /// Asset specifier
    Item(T),
    /// Loot table
    LootTable(T),
    /// No loot given
    #[default]
    Nothing,
    /// Random modular weapon that matches requested restrictions
    ModularWeapon {
        tool: item::tool::ToolKind,
        material: item::Material,
        hands: Option<item::tool::Hands>,
    },
    /// Random primary modular weapon component that matches requested
    /// restrictions
    ModularWeaponPrimaryComponent {
        tool: item::tool::ToolKind,
        material: item::Material,
        hands: Option<item::tool::Hands>,
    },
    /// Dropping variable number of items at random from respective Category
    ///
    /// # Examples:
    /// ```text
    /// MultiDrop(Item("common.items.utility.coins"), 100, 250)
    /// ```
    /// Will drop 100-250 coins (250 coins is also possible).
    /// ```text
    /// MultiDrop(LootTable("common.loot_tables.food.prepared"), 1, 4)
    /// ```
    /// Will drop random item from food.prepared loot table one to four times.
    /// Each time the dice is thrown again, so items might get duplicated or
    /// not.
    MultiDrop(Box<LootSpec<T>>, u32, u32),
    /// Each category is evaluated, often used to have guaranteed quest item
    /// and random reward.
    ///
    /// # Examples:
    /// ```text
    /// All([
    ///     Item("common.items.keys.bone_key"),
    ///     MultiDrop(
    ///         Item("common.items.crafting_ing.mineral.gem.sapphire"),
    ///         0, 1,
    ///     ),
    /// ])
    /// ```
    /// Will always drop bone key, 1-2 furs, and may drop or not drop one
    /// sapphire.
    ///
    /// ```text
    /// All([
    ///     Item("common.items.armor.cultist.necklace"),
    ///     MultiDrop(Item("common.items.armor.cultist.ring"), 2, 2),
    /// ])
    /// ```
    /// Will always drop cultist necklace and two cultist rings.
    All(Vec<LootSpec<T>>),
    /// Like a `LootTable` but inline, most useful with `All([])`.
    ///
    /// # Examples:
    /// ```text
    /// All([
    ///     Item("common.items.keys.terracotta_key_door"),
    ///
    ///     Lottery([
    ///         // Weapons
    ///         (3.0, LootTable("common.loot_tables.weapons.tier-5")),
    ///         // Armor
    ///         (3.0, LootTable("common.loot_tables.armor.tier-5")),
    ///         // Misc
    ///         (0.25, Item("common.items.tool.instruments.steeltonguedrum")),
    ///     ]),
    /// ])
    /// ```
    /// Will always drop a terracotta key, and ONE of items defined in a lottery:
    /// * one random tier-5 weapon
    /// * one random tier-5 armour piece
    /// * Steeldrum
    Lottery(Vec<(f32, LootSpec<T>)>),
}

impl<T: AsRef<str>> LootSpec<T> {
    fn to_items_inner(&self, rng: &mut impl rand::Rng, amount: u32, items: &mut Vec<(u32, Item)>) {
        let convert_item = |item: &T| {
            Item::new_from_asset(item.as_ref()).map_or_else(
                |e| {
                    warn!(?e, "error while loading item: {}", item.as_ref());
                    None
                },
                Some,
            )
        };
        let mut push_item = |mut item: Item, count: u32| {
            let count = item.amount().saturating_mul(count);
            item.set_amount(1).expect("1 is always a valid amount.");
            let hash = item.item_hash();
            match items.binary_search_by_key(&hash, |(_, item)| item.item_hash()) {
                Ok(i) => {
                    // Since item hash can collide with other items, we search nearby items with the
                    // same hash.
                    // NOTE: The `ParitalEq` implementation for `Item` doesn't compare some data
                    // like durability, or wether slots contain anything. Although since these are
                    // Newly loaded items we don't care about comparing those for deduplication
                    // here.
                    let has_same_hash = |i: &usize| items[*i].1.item_hash() == hash;
                    if let Some(i) = (i..items.len())
                        .take_while(has_same_hash)
                        .chain((0..i).rev().take_while(has_same_hash))
                        .find(|i| items[*i].1 == item)
                    {
                        // We saturate at 4 billion items, could use u64 instead if this isn't
                        // desirable.
                        items[i].0 = items[i].0.saturating_add(count);
                    } else {
                        items.insert(i, (count, item));
                    }
                },
                Err(i) => items.insert(i, (count, item)),
            }
        };

        match self {
            Self::Item(item) => {
                if let Some(item) = convert_item(item) {
                    push_item(item, amount);
                }
            },
            Self::LootTable(table) => {
                let loot_spec = Lottery::<LootSpec<String>>::load_expect(table.as_ref()).read();
                for _ in 0..amount {
                    // RNG-P3-004 (determinism audit): draw from the CALLER'S
                    // rng, not the ambient OS-entropy `choose()` — the nested
                    // table severed stream ownership, so the parent's seeded
                    // stream no longer controlled nested loot.
                    loot_spec
                        .choose_seeded(rng.random())
                        .to_items_inner(rng, 1, items)
                }
            },
            Self::Lottery(table) => {
                let lottery = Lottery::from(
                    table
                        .iter()
                        .map(|(weight, spec)| (*weight, spec))
                        .collect::<Vec<_>>(),
                );

                for _ in 0..amount {
                    // RNG-P3-004: caller's stream, as above.
                    lottery
                        .choose_seeded(rng.random())
                        .to_items_inner(rng, 1, items)
                }
            },
            Self::Nothing => {},
            Self::ModularWeapon {
                tool,
                material,
                hands,
            } => {
                for _ in 0..amount {
                    match item::modular::random_weapon(*tool, *material, *hands, rng) {
                        Ok(item) => push_item(item, 1),
                        Err(e) => {
                            warn!(
                                ?e,
                                "error while creating modular weapon. Toolkind: {:?}, Material: \
                                 {:?}, Hands: {:?}",
                                tool,
                                material,
                                hands,
                            );
                        },
                    }
                }
            },
            Self::ModularWeaponPrimaryComponent {
                tool,
                material,
                hands,
            } => {
                for _ in 0..amount {
                    match item::modular::random_weapon(*tool, *material, *hands, rng) {
                        Ok(item) => push_item(item, 1),
                        Err(e) => {
                            warn!(
                                ?e,
                                "error while creating modular weapon primary component. Toolkind: \
                                 {:?}, Material: {:?}, Hands: {:?}",
                                tool,
                                material,
                                hands,
                            );
                        },
                    }
                }
            },
            Self::MultiDrop(loot_spec, lower, upper) => {
                let sub_amount = rng.random_range(*lower..=*upper);
                // We saturate at 4 billion items, could use u64 instead if this isn't
                // desirable.
                loot_spec.to_items_inner(rng, sub_amount.saturating_mul(amount), items);
            },
            Self::All(loot_specs) => {
                for loot_spec in loot_specs {
                    loot_spec.to_items_inner(rng, amount, items);
                }
            },
        }
    }

    /// E6 (determinism audit): `rng` must be the CALLER's stream, seeded
    /// from the causal event via [`seed_loot_roll`] — this used to reach
    /// ambient OS entropy (`rand::rng()`) internally, making every loot
    /// roll (on-transform, spawn pre-roll, block-harvest) nondeterministic
    /// cross-run even though the nested-table draws (RNG-P3-004) already
    /// respected a passed-in stream.
    pub fn to_items(&self, rng: &mut impl rand::Rng) -> Option<Vec<(u32, Item)>> {
        let mut items = Vec::new();
        self.to_items_inner(rng, 1, &mut items);

        if !items.is_empty() {
            // E6: amount alone left equal-amount items in `sort_unstable`'s
            // implementation-defined tie order; item_hash is a stable,
            // already-computed secondary key, so the full output order is
            // now a pure function of the input (no incidental sort-algorithm
            // dependence).
            items.sort_unstable_by_key(|(amount, item)| (*amount, item.item_hash()));

            Some(items)
        } else {
            None
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::borrow::Borrow;

    use super::*;
    use crate::{assets, comp::Item};
    use assets::AssetExt;

    #[cfg(test)]
    pub fn validate_loot_spec(item: &LootSpec<String>) {
        let mut rng = rand::rng();
        match item {
            LootSpec::Item(item) => {
                Item::new_from_asset_expect(item);
            },
            LootSpec::LootTable(loot_table) => {
                let loot_table = Lottery::<LootSpec<String>>::load_expect(loot_table).read();
                validate_table_contents(&loot_table);
            },
            LootSpec::Nothing => {},
            LootSpec::ModularWeapon {
                tool,
                material,
                hands,
            } => {
                item::modular::random_weapon(*tool, *material, *hands, &mut rng).unwrap_or_else(
                    |_| {
                        panic!(
                            "Failed to synthesize a modular {tool:?} made of {material:?} that \
                             had a hand restriction of {hands:?}."
                        )
                    },
                );
            },
            LootSpec::ModularWeaponPrimaryComponent {
                tool,
                material,
                hands,
            } => {
                item::modular::random_weapon_primary_component(*tool, *material, *hands, &mut rng)
                    .unwrap_or_else(|_| {
                        panic!(
                            "Failed to synthesize a modular weapon primary component: {tool:?} \
                             made of {material:?} that had a hand restriction of {hands:?}."
                        )
                    });
            },
            LootSpec::MultiDrop(loot_spec, lower, upper) => {
                assert!(
                    upper >= lower,
                    "Upper quantity must be at least the value of lower quantity. Upper value: \
                     {}, low value: {}.",
                    upper,
                    lower
                );
                validate_loot_spec(loot_spec);
            },
            LootSpec::All(loot_specs) => {
                for loot_spec in loot_specs {
                    validate_loot_spec(loot_spec);
                }
            },
            LootSpec::Lottery(table) => {
                let lottery = Lottery::from(
                    table
                        .iter()
                        .map(|(weight, spec)| (*weight, spec))
                        .collect::<Vec<_>>(),
                );

                validate_table_contents(&lottery);
            },
        }
    }

    fn validate_table_contents<T: Borrow<LootSpec<String>>>(table: &Lottery<T>) {
        for (_, item) in table.iter() {
            validate_loot_spec(item.borrow());
        }
    }

    #[test]
    fn test_loot_tables() {
        let loot_tables = assets::load_rec_dir::<Lottery<LootSpec<String>>>("common.loot_tables")
            .expect("load loot_tables");
        for loot_table in loot_tables.read().ids() {
            let loot_table = Lottery::<LootSpec<String>>::load_expect(loot_table);
            validate_table_contents(&loot_table.read());
        }
    }

    /// LOOT-01 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof):
    /// `Lottery::choose_seeded` is a pure, seed-deterministic selector that
    /// consumes the FULL 32-bit seed range (RNG-P3-003). The existing lottery
    /// tests only validate table CONTENTS and drive `distribute_many` off OS
    /// entropy — neither evidences the seeded-selection determinism contract.
    #[test]
    fn lottery_choose_seeded_is_deterministic_and_uses_full_seed_range() {
        // 26 equal-weight entries => total 26; choose_seeded maps a seed in
        // [0, 2^32) linearly onto [0, 26), so entry index = floor(seed/2^32 * 26).
        let lottery = Lottery::from((0u32..26).map(|i| (1.0f32, i)).collect::<Vec<_>>());

        // Determinism: choose_seeded is a pure function of the seed.
        for &s in &[0u32, 1, 12345, 0x1234_5678, u32::MAX] {
            assert_eq!(
                lottery.choose_seeded(s),
                lottery.choose_seeded(s),
                "choose_seeded must be a pure function of the seed"
            );
        }

        // Non-vacuity: distinct seeds reach distinct entries, so the outcome
        // genuinely depends on the seed rather than being constant.
        let reached: std::collections::BTreeSet<u32> = (0..26u64)
            .map(|k| *lottery.choose_seeded((k * (1u64 << 32) / 26) as u32))
            .collect();
        assert!(
            reached.len() > 1,
            "lottery outcome does not vary with the seed (only reached {:?})",
            reached
        );

        // RNG-P3-003 (full 32-bit range): the OLD `% 65536` discarded the HIGH
        // 16 seed bits, so any two seeds sharing their low 16 bits collapsed to
        // the same outcome. seed_a/seed_b share low16 = 0x1234 but differ in the
        // high bits and MUST be able to select different entries — direct
        // evidence the high seed bits are not discarded. (Under the old formula
        // both map to the same entry, so this assertion fails RED on a regression.)
        let seed_a = 0x0000_1234u32;
        let seed_b = 0x8000_1234u32;
        assert_ne!(
            lottery.choose_seeded(seed_a),
            lottery.choose_seeded(seed_b),
            "seeds differing only in their high 16 bits collapsed to the same outcome: the \
             RNG-P3-003 full-32-bit-range fix has regressed (high seed bits are being discarded)"
        );
    }

    /// LOOT-02 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof):
    /// `distribute_many` assigns stacked items to weighted participants in a
    /// CANONICAL participant order (RNG-P3-006), so with the same RNG seed the
    /// allocation is independent of the participant INPUT order. The existing
    /// `test_distribute_many` drives it off OS entropy and only checks a "known
    /// successful case" — it cannot evidence this order-independence contract.
    #[test]
    fn distribute_many_is_participant_order_independent() {
        use rand::SeedableRng;
        use std::collections::BTreeMap;

        // Distribute a fixed set of stacked items across weighted participants,
        // capturing each participant's TOTAL allocation. exec_item reports the
        // per-item-type count (then it is reset), so summing yields the total.
        fn run(seed: u64, participants: Vec<(f32, char)>) -> BTreeMap<char, u32> {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let items: Vec<u32> = vec![3, 2, 5, 1, 4];
            let mut got: BTreeMap<char, u32> = BTreeMap::new();
            distribute_many(
                participants,
                &mut rng,
                &items,
                |&amount| amount,
                |_item, who, amount| {
                    *got.entry(who).or_default() += amount;
                },
            );
            got
        }

        // Same seed, participants supplied FORWARD vs REVERSED. RNG-P3-006's
        // canonical ordering must make the allocation identical either way.
        let seed = 0x0D15_712B_u64;
        let forward = run(seed, vec![(0.5, 'a'), (0.3, 'b'), (0.2, 'c')]);
        let reversed = run(seed, vec![(0.2, 'c'), (0.3, 'b'), (0.5, 'a')]);
        assert_eq!(
            forward, reversed,
            "distribute_many allocation depends on participant INPUT order: the RNG-P3-006 \
             canonical participant ordering has regressed"
        );

        // Non-vacuity: the run actually distributed items (an empty distribution
        // would make the equality above trivially true).
        let total: u32 = forward.values().sum();
        assert!(
            total > 0,
            "distribute_many placed no items — the order-independence check is vacuous"
        );
    }

    /// E6 (determinism audit): `to_items` used to reach ambient OS entropy
    /// internally (`rand::rng()`) regardless of what the caller passed in
    /// for the nested-table draws. If any such ambient source were still
    /// reachable, at least one of these 20 independent fixed-seed
    /// re-derivations would diverge from the first.
    #[test]
    fn to_items_is_a_pure_function_of_its_rng_argument() {
        use rand::SeedableRng;
        let spec: LootSpec<String> = LootSpec::All(vec![
            LootSpec::Item("common.items.food.cheese".to_string()),
            LootSpec::Item("common.items.food.apple".to_string()),
            LootSpec::Item("common.items.food.mushroom".to_string()),
        ]);

        fn snapshot(items: &[(u32, Item)]) -> Vec<(u32, u64)> {
            items
                .iter()
                .map(|(amount, item)| (*amount, item.item_hash()))
                .collect()
        }

        let seed = 0xE6_0000_5EED_u64;
        let mut first_rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
        let first = snapshot(&spec.to_items(&mut first_rng).expect("spec yields items"));

        for _ in 0..20 {
            let mut rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
            let got = snapshot(&spec.to_items(&mut rng).expect("spec yields items"));
            assert_eq!(
                got, first,
                "to_items() diverged across re-derivations of the same seed — ambient entropy \
                 is reachable"
            );
        }
    }

    /// E6: equal-amount items must be ordered by a caller-recomputable
    /// secondary key (item_hash), not `sort_unstable`'s implementation-
    /// defined tie behavior.
    #[test]
    fn to_items_equal_amount_items_sort_by_item_hash() {
        use rand::SeedableRng;
        let spec: LootSpec<String> = LootSpec::All(vec![
            LootSpec::Item("common.items.food.cheese".to_string()),
            LootSpec::Item("common.items.food.apple".to_string()),
            LootSpec::Item("common.items.food.mushroom".to_string()),
        ]);
        let mut rng = rand_chacha::ChaChaRng::seed_from_u64(0xE6_0000_50E7);
        let items = spec.to_items(&mut rng).expect("spec yields items");

        // Non-vacuity: the fixture only exercises the tiebreak if at least
        // two entries actually share an amount.
        let amounts: Vec<u32> = items.iter().map(|(a, _)| *a).collect();
        assert!(
            amounts.windows(2).any(|w| w[0] == w[1]),
            "fixture does not produce equal-amount items — the tiebreak test is vacuous"
        );

        let actual: Vec<(u32, u64)> = items
            .iter()
            .map(|(a, item)| (*a, item.item_hash()))
            .collect();
        let mut expected = actual.clone();
        expected.sort_unstable();
        assert_eq!(
            actual, expected,
            "equal-amount items are not ordered by the documented (amount, item_hash) tiebreak"
        );
    }

    #[test]
    fn test_distribute_many() {
        let mut rng = rand::rng();

        // Known successful case
        for _ in 0..10 {
            distribute_many(
                vec![(0.4f32, "a"), (0.4, "b"), (0.2, "c")],
                &mut rng,
                &[("item", 10)],
                |(_, m)| *m,
                |_item, winner, count| match winner {
                    "a" | "b" => assert_eq!(count, 4),
                    "c" => assert_eq!(count, 2),
                    _ => unreachable!(),
                },
            );
        }
    }
}
