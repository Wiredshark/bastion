pub mod armor;
pub mod item_key;
pub mod modular;
pub mod tool;

// Reexports
pub use modular::{MaterialStatManifest, ModularBase, ModularComponent};
pub use tool::{AbilityMap, AbilitySet, AbilitySpec, Hands, Tool, ToolKind};

use crate::{
    assets::{self, Asset, AssetCache, AssetExt, BoxedError, Error, Ron, SharedString},
    comp::inventory::InvSlot,
    effect::Effect,
    lottery::LootSpec,
    recipe::RecipeInput,
    resources::ProgramTime,
    terrain::{Block, sprite::SpriteCfg},
};
use common_i18n::Content;
use core::{
    convert::TryFrom,
    mem,
    num::{NonZeroU32, NonZeroU64},
};
use crossbeam_utils::atomic::AtomicCell;
use hashbrown::{Equivalent, HashMap};
use item_key::ItemKey;
use serde::{Deserialize, Serialize, Serializer, de};
use specs::{Component, DenseVecStorage, DerefFlaggedStorage};
use std::{borrow::Cow, fmt, sync::Arc};
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};
use tracing::error;
use vek::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, strum::EnumString)]
pub enum Reagent {
    Blue,
    Green,
    Purple,
    Red,
    White,
    Yellow,
    FireRain,
    FireGigas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Utility {
    Coins,
    Collar,
    Key,
    AbilityReq,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lantern {
    color: Rgb<u32>,
    strength_thousandths: u32,
    flicker_thousandths: u32,
    pub dir: Option<(Vec3<f32>, f32)>,
}

impl Lantern {
    pub fn strength(&self) -> f32 { self.strength_thousandths as f32 / 1000_f32 }

    pub fn color(&self) -> Rgb<f32> { self.color.map(|c| c as f32 / 255.0) }

    pub fn flicker(&self) -> f32 { self.flicker_thousandths as f32 / 1000_f32 }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy, PartialOrd, Ord)]
pub enum Quality {
    Low,       // Grey
    Common,    // Light blue
    Moderate,  // Green
    High,      // Blue
    Epic,      // Purple
    Legendary, // Gold
    Artifact,  // Orange
    Debug,     // Red
}

impl Quality {
    pub const MIN: Self = Self::Low;
}

pub trait TagExampleInfo {
    fn name(&self) -> &str;
    /// What item to show in the crafting hud if the player has nothing with the
    /// tag
    fn exemplar_identifier(&self) -> Option<&str>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, IntoStaticStr)]
pub enum MaterialKind {
    Metal,
    Gem,
    Wood,
    Stone,
    Cloth,
    Hide,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumString,
    EnumIter,
)]
#[strum(serialize_all = "snake_case")]
pub enum Material {
    Bronze,
    Iron,
    Steel,
    Cobalt,
    Bloodsteel,
    Silver,
    Gold,
    Orichalcum,
    Topaz,
    Emerald,
    Sapphire,
    Amethyst,
    Ruby,
    Diamond,
    Twig,
    PlantFiber,
    Wood,
    Bamboo,
    Hardwood,
    Ironwood,
    Frostwood,
    Eldwood,
    Rock,
    Granite,
    Bone,
    Basalt,
    Obsidian,
    Velorite,
    Linen,
    RedLinen,
    Cotton,
    Wool,
    Silk,
    Lifecloth,
    Moonweave,
    Sunsilk,
    Rawhide,
    Leather,
    RigidLeather,
    Scale,
    Carapace,
    Serpentscale,
    Plate,
    Dragonscale,
}

impl Material {
    pub fn material_kind(&self) -> MaterialKind {
        match self {
            Material::Bronze
            | Material::Iron
            | Material::Steel
            | Material::Cobalt
            | Material::Bloodsteel
            | Material::Silver
            | Material::Gold
            | Material::Orichalcum => MaterialKind::Metal,
            Material::Topaz
            | Material::Emerald
            | Material::Sapphire
            | Material::Amethyst
            | Material::Ruby
            | Material::Diamond => MaterialKind::Gem,
            Material::Wood
            | Material::Twig
            | Material::PlantFiber
            | Material::Bamboo
            | Material::Hardwood
            | Material::Ironwood
            | Material::Frostwood
            | Material::Eldwood => MaterialKind::Wood,
            Material::Rock
            | Material::Granite
            | Material::Bone
            | Material::Basalt
            | Material::Obsidian
            | Material::Velorite => MaterialKind::Stone,
            Material::Linen
            | Material::RedLinen
            | Material::Cotton
            | Material::Wool
            | Material::Silk
            | Material::Lifecloth
            | Material::Moonweave
            | Material::Sunsilk => MaterialKind::Cloth,
            Material::Rawhide
            | Material::Leather
            | Material::RigidLeather
            | Material::Scale
            | Material::Carapace
            | Material::Serpentscale
            | Material::Plate
            | Material::Dragonscale => MaterialKind::Hide,
        }
    }

    pub fn asset_identifier(&self) -> Option<&'static str> {
        match self {
            Material::Bronze => Some("common.items.mineral.ingot.bronze"),
            Material::Iron => Some("common.items.mineral.ingot.iron"),
            Material::Steel => Some("common.items.mineral.ingot.steel"),
            Material::Cobalt => Some("common.items.mineral.ingot.cobalt"),
            Material::Bloodsteel => Some("common.items.mineral.ingot.bloodsteel"),
            Material::Silver => Some("common.items.mineral.ingot.silver"),
            Material::Gold => Some("common.items.mineral.ingot.gold"),
            Material::Orichalcum => Some("common.items.mineral.ingot.orichalcum"),
            Material::Topaz => Some("common.items.mineral.gem.topaz"),
            Material::Emerald => Some("common.items.mineral.gem.emerald"),
            Material::Sapphire => Some("common.items.mineral.gem.sapphire"),
            Material::Amethyst => Some("common.items.mineral.gem.amethyst"),
            Material::Ruby => Some("common.items.mineral.gem.ruby"),
            Material::Diamond => Some("common.items.mineral.gem.diamond"),
            Material::Twig => Some("common.items.crafting_ing.twigs"),
            Material::PlantFiber => Some("common.items.flowers.plant_fiber"),
            Material::Wood => Some("common.items.log.wood"),
            Material::Bamboo => Some("common.items.log.bamboo"),
            Material::Hardwood => Some("common.items.log.hardwood"),
            Material::Ironwood => Some("common.items.log.ironwood"),
            Material::Frostwood => Some("common.items.log.frostwood"),
            Material::Eldwood => Some("common.items.log.eldwood"),
            Material::Rock
            | Material::Granite
            | Material::Bone
            | Material::Basalt
            | Material::Obsidian
            | Material::Velorite => None,
            Material::Linen => Some("common.items.crafting_ing.cloth.linen"),
            Material::RedLinen => Some("common.items.crafting_ing.cloth.linen_red"),
            Material::Cotton => Some("common.items.crafting_ing.cloth.cotton"),
            Material::Wool => Some("common.items.crafting_ing.cloth.wool"),
            Material::Silk => Some("common.items.crafting_ing.cloth.silk"),
            Material::Lifecloth => Some("common.items.crafting_ing.cloth.lifecloth"),
            Material::Moonweave => Some("common.items.crafting_ing.cloth.moonweave"),
            Material::Sunsilk => Some("common.items.crafting_ing.cloth.sunsilk"),
            Material::Rawhide => Some("common.items.crafting_ing.leather.simple_leather"),
            Material::Leather => Some("common.items.crafting_ing.leather.thick_leather"),
            Material::RigidLeather => Some("common.items.crafting_ing.leather.rigid_leather"),
            Material::Scale => Some("common.items.crafting_ing.hide.scales"),
            Material::Carapace => Some("common.items.crafting_ing.hide.carapace"),
            Material::Serpentscale => Some("common.items.crafting_ing.hide.serpent_scale"),
            Material::Plate => Some("common.items.crafting_ing.hide.plate"),
            Material::Dragonscale => Some("common.items.crafting_ing.hide.dragon_scale"),
        }
    }
}

impl TagExampleInfo for Material {
    fn name(&self) -> &str { self.into() }

    fn exemplar_identifier(&self) -> Option<&str> { self.asset_identifier() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemTag {
    /// Used to indicate that an item is composed of this material
    Material(Material),
    /// Used to indicate that an item is composed of this material kind
    MaterialKind(MaterialKind),
    Cultist,
    Gnarling,
    Potion,
    Charm,
    Food,
    BaseMaterial, // Cloth-scraps, Leather...
    CraftingTool, // Pickaxe, Craftsman-Hammer, Sewing-Set
    Utility,
    Bag,
    SalvageInto(Material, u32),
    Witch,
    Pirate,
}

impl TagExampleInfo for ItemTag {
    fn name(&self) -> &str {
        match self {
            ItemTag::Material(material) => material.name(),
            ItemTag::MaterialKind(material_kind) => material_kind.into(),
            ItemTag::Cultist => "cultist",
            ItemTag::Gnarling => "gnarling",
            ItemTag::Potion => "potion",
            ItemTag::Charm => "charm",
            ItemTag::Food => "food",
            ItemTag::BaseMaterial => "basemat",
            ItemTag::CraftingTool => "tool",
            ItemTag::Utility => "utility",
            ItemTag::Bag => "bag",
            ItemTag::SalvageInto(_, _) => "salvage",
            ItemTag::Witch => "witch",
            ItemTag::Pirate => "pirate",
        }
    }

    // TODO: Autogenerate these?
    fn exemplar_identifier(&self) -> Option<&str> {
        match self {
            ItemTag::Material(material) => material.exemplar_identifier(),
            ItemTag::Cultist => Some("common.items.tag_examples.cultist"),
            ItemTag::Gnarling => Some("common.items.tag_examples.gnarling"),
            ItemTag::Witch => Some("common.items.tag_examples.witch"),
            ItemTag::Pirate => Some("common.items.tag_examples.pirate"),
            ItemTag::MaterialKind(_)
            | ItemTag::Potion
            | ItemTag::Food
            | ItemTag::Charm
            | ItemTag::BaseMaterial
            | ItemTag::CraftingTool
            | ItemTag::Utility
            | ItemTag::Bag
            | ItemTag::SalvageInto(_, _) => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effects {
    Any(Vec<Effect>),
    All(Vec<Effect>),
    One(Effect),
}

impl Effects {
    pub fn effects(&self) -> &[Effect] {
        match self {
            Effects::Any(effects) => effects,
            Effects::All(effects) => effects,
            Effects::One(effect) => std::slice::from_ref(effect),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ItemKind {
    /// Something wieldable
    Tool(Tool),
    ModularComponent(ModularComponent),
    Lantern(Lantern),
    Armor(armor::Armor),
    Glider,
    Consumable {
        kind: ConsumableKind,
        effects: Effects,
        #[serde(default)]
        container: Option<ItemDefinitionIdOwned>,
    },
    Utility {
        kind: Utility,
    },
    Ingredient {
        /// Used to generate names for modular items composed of this ingredient
        // I think we can actually remove it now?
        #[deprecated = "since item i18n"]
        descriptor: String,
    },
    TagExamples {
        /// A list of item names to lookup the appearences of and animate
        /// through
        item_ids: Vec<String>,
    },
    RecipeGroup {
        recipes: Vec<String>,
    },
    Quest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsumableKind {
    Drink,
    Food,
    ComplexFood,
    Charm,
    Recipe,
}

impl ItemKind {
    pub fn is_equippable(&self) -> bool {
        matches!(
            self,
            ItemKind::Tool(_) | ItemKind::Armor { .. } | ItemKind::Glider | ItemKind::Lantern(_)
        )
    }

    // Used for inventory sorting, what comes before the first colon (:) is used as
    // a broader category
    pub fn get_itemkind_string(&self) -> String {
        match self {
            // Using tool and toolkind to sort tools by kind
            ItemKind::Tool(tool) => format!("Tool: {:?}", tool.kind),
            ItemKind::ModularComponent(modular_component) => {
                format!("ModularComponent: {:?}", modular_component.toolkind())
            },
            ItemKind::Lantern(lantern) => format!("Lantern: {:?}", lantern),
            ItemKind::Armor(armor) => format!("Armor: {:?}", armor.stats),
            ItemKind::Glider => "Glider:".to_string(),
            ItemKind::Consumable { kind, .. } => {
                format!("Consumable: {:?}", kind)
            },
            ItemKind::Utility { kind } => format!("Utility: {:?}", kind),
            #[expect(deprecated)]
            ItemKind::Ingredient { descriptor } => format!("Ingredient: {}", descriptor),
            ItemKind::TagExamples { item_ids } => format!("TagExamples: {:?}", item_ids),
            ItemKind::RecipeGroup { .. } => String::from("Recipes:"),
            ItemKind::Quest => String::from("Quest:"),
        }
    }

    pub fn has_durability(&self) -> bool {
        match self {
            ItemKind::Tool(Tool { kind, .. }) => !matches!(kind, ToolKind::Throwable),
            ItemKind::Armor(armor) => armor.kind.has_durability(),
            ItemKind::ModularComponent(_)
            | ItemKind::Lantern(_)
            | ItemKind::Quest
            | ItemKind::Glider
            | ItemKind::Consumable { .. }
            | ItemKind::Utility { .. }
            | ItemKind::Ingredient { .. }
            | ItemKind::TagExamples { .. }
            | ItemKind::RecipeGroup { .. } => false,
        }
    }
}

pub type ItemId = AtomicCell<Option<NonZeroU64>>;

/* /// The only way to access an item id outside this module is to mutably, atomically update it using
/// this structure.  It has a single method, `try_assign_id`, which attempts to set the id if and
/// only if it's not already set.
pub struct CreateDatabaseItemId {
    item_id: Arc<ItemId>,
}*/

/// NOTE: Do not call `Item::clone` without consulting the core devs!  It only
/// exists due to being required for message serialization at the moment, and
/// should not be used for any other purpose.
///
/// FIXME: Turn on a Clippy lint forbidding the use of `Item::clone` using the
/// `disallowed_method` feature.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    /// item_id is hidden because it represents the persistent, storage entity
    /// ID for any item that has been saved to the database.  Additionally,
    /// it (currently) holds interior mutable state, making it very
    /// dangerous to expose.  We will work to eliminate this issue soon; for
    /// now, we try to make the system as foolproof as possible by greatly
    /// restricting opportunities for cloning the item_id.
    #[serde(skip)]
    item_id: Arc<ItemId>,
    /// item_def is hidden because changing the item definition for an item
    /// could change invariants like whether it was stackable (invalidating
    /// the amount).
    item_base: ItemBase,
    /// components is hidden to maintain the following invariants:
    /// - It should only contain modular components (and enhancements, once they
    ///   exist)
    /// - Enhancements (once they exist) should be compatible with the available
    ///   slot shapes
    /// - Modular components should agree with the tool kind
    /// - There should be exactly one damage component and exactly one held
    ///   component for modular weapons
    components: Vec<Item>,
    /// amount is hidden because it needs to maintain the invariant that only
    /// stackable items can have > 1 amounts.
    amount: NonZeroU32,
    /// The slots for items that this item has
    slots: Vec<InvSlot>,
    item_config: Option<Box<ItemConfig>>,
    hash: u64,
    /// Tracks how many deaths occurred while item was equipped, which is
    /// converted into the items durability. Only tracked for tools and armor
    /// currently.
    durability_lost: Option<u32>,
}

/// Newtype around [`Item`] used for frontend events to prevent it accidentally
/// being used for anything other than frontend events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrontendItem(Item);

// An item that is dropped into the world an can be picked up. It can stack with
// other items of the same type regardless of the stack limit, when picked up
// the last item from the list is popped
//
// NOTE: Never call PickupItem::clone, it is only used for network
// synchronization
//
// Invariants:
//  - Any item that is not the last one must have an amount equal to its
//    `max_amount()`
//  - All items must be equal and have a zero amount of slots
//  - The Item list must not be empty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickupItem {
    items: Vec<Item>,
    /// This [`ProgramTime`] only makes sense on the server
    created_at: ProgramTime,
    /// This [`ProgramTime`] only makes sense on the server
    next_merge_check: ProgramTime,
    /// When set to `true`, this item will actively try to be merged into nearby
    /// items of the same kind (see [`Item::can_merge`]). Currently only used
    /// for inventory dropped items to prevent entity DoS.
    pub should_merge: bool,
    /// T0.49 (master build order; T0-003): the persistent instance identity —
    /// stamped ONCE at the authoritative creation commit
    /// (`create_item_drop`); `None` for instances predating the field or not
    /// yet committed. Field-first per the ruling: no consumer switches yet
    /// (harness item hashes keep their current mechanism).
    #[serde(default)]
    instance_id: Option<ItemInstanceId>,
}

/// T0.49: the packet-specified persistent item-instance identity. The
/// world namespace is a one-time per-world NONCE (minted at world
/// creation — two saves sharing a worldgen seed must not alias), and the
/// creation sequence is allocated only at the authoritative creation
/// commit. Content hashes are definition/migration fingerprints, never
/// instance identity; UUIDs/pointers/entity ids rejected as primary
/// identity per the packet.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ItemInstanceId {
    pub world_namespace: u64,
    pub creation_sequence: u64,
}

/// Newtype around [`Item`] so that thrown projectiles can track which item
/// they represent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrownItem(pub Item);

use std::hash::{Hash, Hasher};

// Used to find inventory item corresponding to hotbar slot
impl Hash for Item {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.item_definition_id().hash(state);
        self.components.iter().for_each(|comp| comp.hash(state));
    }
}

// at the time of writing, we use Fluent, which supports attributes
// and we can get both name and description using them
type I18nId = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
// TODO: probably make a Resource if used outside of voxygen
// TODO: add hot-reloading similar to how ItemImgs does it?
// TODO: make it work with plugins (via Concatenate?)
/// To be used with ItemDesc::i18n
///
/// NOTE: there is a limitation to this manifest, as it uses ItemKey and
/// ItemKey isn't uniquely identifies Item, when it comes to modular items.
///
/// If modular weapon has the same primary component and the same hand-ness,
/// we use the same model EVEN IF it has different secondary components, like
/// Staff with Heavy core or Light core.
///
/// Translations currently do the same, but *maybe* they shouldn't in which case
/// we should either extend ItemKey or use new identifier. We could use
/// ItemDefinitionId, but it's very generic and cumbersome.
pub struct ItemI18n {
    /// maps ItemKey to i18n identifier
    map: HashMap<ItemKey, I18nId>,
    /// maps FragmentKey to i18n identifier
    ///
    /// Used for optional templating for languages that can stomach them
    fragments: HashMap<FragmentKey, I18nId>,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum FragmentKey {
    // path to ingredient
    Ingredient(String),
    // path to primary component and hand-ness required
    WeaponPrimaryComponent(String, Hands),
}

impl ItemI18n {
    pub fn new_expect() -> Self {
        Ron::load_expect("common.item_i18n_manifest")
            .read()
            .clone()
            .into_inner()
    }

    /// Returns (name, description) in Content form.
    // TODO: after we remove legacy text from ItemDef, consider making this
    // function non-fallible?
    fn item_text_opt(&self, item_key: &ItemKey) -> Option<(Content, Content)> {
        let key = self.try_key(item_key);
        key.map(|key| {
            (
                Content::Key(key.to_owned()),
                Content::Attr(key.to_owned(), "desc".to_owned()),
            )
        })
    }

    /// Tries to fetch a fragment from i18n manifest
    // TODO: potentially should just return a string as well?
    fn try_fragment(&self, fragment_key: &FragmentKey) -> Option<Content> {
        self.fragments
            .get(fragment_key)
            .map(|key| Content::Key(key.to_owned()))
    }

    /// Tries to fetch a key from i18n manifest, returns a i18n string,
    /// do with it what you need.
    fn try_key(&self, item_key: &ItemKey) -> Option<&I18nId> {
        // We don't put TagExamples into manifest.
        // Instead they are marked as Simple.
        let key;
        let item_key = if let ItemKey::TagExamples(_, id) = item_key {
            key = ItemKey::Simple(id.to_string());
            &key
        } else {
            item_key
        };

        self.map.get(item_key)
    }

    /// Returns all fragments, mainly for testing
    pub fn all_fragments(&self) -> impl Iterator<Item = (&FragmentKey, &I18nId)> {
        self.fragments.iter()
    }
}

#[derive(Clone, Debug)]
pub enum ItemBase {
    Simple(Arc<ItemDef>),
    Modular(ModularBase),
}

impl Serialize for ItemBase {
    // Custom serialization for ItemDef, we only want to send the item_definition_id
    // over the network, the client will use deserialize_item_def to fetch the
    // ItemDef from assets.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.serialization_item_id())
    }
}

impl<'de> Deserialize<'de> for ItemBase {
    // Custom de-serialization for ItemBase to retrieve the ItemBase from assets
    // using its asset specifier (item_definition_id)
    fn deserialize<D>(deserializer: D) -> Result<ItemBase, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ItemBaseStringVisitor;

        impl de::Visitor<'_> for ItemBaseStringVisitor {
            type Value = ItemBase;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("item def string")
            }

            fn visit_str<E>(self, serialized_item_base: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                ItemBase::from_item_id_string(serialized_item_base)
                    .map_err(|err| E::custom(err.to_string()))
            }
        }

        deserializer.deserialize_str(ItemBaseStringVisitor)
    }
}

impl ItemBase {
    fn num_slots(&self) -> u16 {
        match self {
            ItemBase::Simple(item_def) => item_def.num_slots(),
            ItemBase::Modular(_) => 0,
        }
    }

    // Should be kept the same as the persistence_item_id function in Item
    // TODO: Maybe use Cow?
    fn serialization_item_id(&self) -> String {
        match &self {
            ItemBase::Simple(item_def) => item_def.item_definition_id.clone(),
            ItemBase::Modular(mod_base) => String::from(mod_base.pseudo_item_id()),
        }
    }

    fn from_item_id_string(item_id_string: &str) -> Result<Self, Error> {
        if item_id_string.starts_with(crate::modular_item_id_prefix!()) {
            Ok(ItemBase::Modular(ModularBase::load_from_pseudo_id(
                item_id_string,
            )))
        } else {
            Ok(ItemBase::Simple(Arc::<ItemDef>::load_cloned(
                item_id_string,
            )?))
        }
    }
}

// TODO: could this theorectically hold a ref to the actual components and
// lazily get their IDs for hash/partialeq/debug/to_owned/etc? (i.e. eliminating
// `Vec`s)
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemDefinitionId<'a> {
    Simple(Cow<'a, str>),
    Modular {
        pseudo_base: &'a str,
        components: Vec<ItemDefinitionId<'a>>,
    },
    Compound {
        simple_base: &'a str,
        components: Vec<ItemDefinitionId<'a>>,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ItemDefinitionIdOwned {
    Simple(String),
    Modular {
        pseudo_base: String,
        components: Vec<ItemDefinitionIdOwned>,
    },
    Compound {
        simple_base: String,
        components: Vec<ItemDefinitionIdOwned>,
    },
}

impl ItemDefinitionIdOwned {
    pub fn as_ref(&self) -> ItemDefinitionId<'_> {
        match *self {
            Self::Simple(ref id) => ItemDefinitionId::Simple(Cow::Borrowed(id)),
            Self::Modular {
                ref pseudo_base,
                ref components,
            } => ItemDefinitionId::Modular {
                pseudo_base,
                components: components.iter().map(|comp| comp.as_ref()).collect(),
            },
            Self::Compound {
                ref simple_base,
                ref components,
            } => ItemDefinitionId::Compound {
                simple_base,
                components: components.iter().map(|comp| comp.as_ref()).collect(),
            },
        }
    }
}

impl ItemDefinitionId<'_> {
    pub fn itemdef_id(&self) -> Option<&str> {
        match self {
            Self::Simple(id) => Some(id),
            Self::Modular { .. } => None,
            Self::Compound { simple_base, .. } => Some(simple_base),
        }
    }

    pub fn to_owned(&self) -> ItemDefinitionIdOwned {
        match self {
            Self::Simple(id) => ItemDefinitionIdOwned::Simple(String::from(&**id)),
            Self::Modular {
                pseudo_base,
                components,
            } => ItemDefinitionIdOwned::Modular {
                pseudo_base: String::from(*pseudo_base),
                components: components.iter().map(|comp| comp.to_owned()).collect(),
            },
            Self::Compound {
                simple_base,
                components,
            } => ItemDefinitionIdOwned::Compound {
                simple_base: String::from(*simple_base),
                components: components.iter().map(|comp| comp.to_owned()).collect(),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemDef {
    #[serde(default)]
    /// The string that refers to the filepath to the asset, relative to the
    /// assets folder, which the ItemDef is loaded from. The name space
    /// prepended with `veloren.core` is reserved for veloren functions.
    item_definition_id: String,
    #[deprecated = "since item i18n"]
    legacy_name: String,
    pub kind: ItemKind,
    pub quality: Quality,
    pub tags: Vec<ItemTag>,
    #[serde(default)]
    pub slots: u16,
    /// Used to specify a custom ability set for a weapon. Leave None (or don't
    /// include field in ItemDef) to use default ability set for weapon kind.
    pub ability_spec: Option<AbilitySpec>,
}

impl PartialEq for ItemDef {
    fn eq(&self, other: &Self) -> bool { self.item_definition_id == other.item_definition_id }
}

// TODO: Look into removing ItemConfig and just using AbilitySet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemConfig {
    pub abilities: AbilitySet<tool::AbilityItem>,
}

#[derive(Debug)]
pub enum ItemConfigError {
    BadItemKind,
}

impl TryFrom<(&Item, &AbilityMap, &MaterialStatManifest)> for ItemConfig {
    type Error = ItemConfigError;

    fn try_from(
        // TODO: Either remove msm or use it as argument in fn kind
        (item, ability_map, _msm): (&Item, &AbilityMap, &MaterialStatManifest),
    ) -> Result<Self, Self::Error> {
        match &*item.kind() {
            ItemKind::Tool(tool) => {
                // If no custom ability set is specified, fall back to abilityset of tool kind.
                let tool_default = |tool_kind| {
                    let key = &AbilitySpec::Tool(tool_kind);
                    ability_map.get_ability_set(key)
                };
                let abilities = if let Some(set_key) = item.ability_spec() {
                    if let Some(set) = ability_map.get_ability_set(&set_key) {
                        set.clone()
                            .modified_by_tool(tool, item.stats_durability_multiplier())
                    } else {
                        error!(
                            "Custom ability set: {:?} references non-existent set, falling back \
                             to default ability set.",
                            set_key
                        );
                        tool_default(tool.kind).cloned().unwrap_or_default()
                    }
                } else if let Some(set) = tool_default(tool.kind) {
                    set.clone()
                        .modified_by_tool(tool, item.stats_durability_multiplier())
                } else {
                    error!(
                        "No ability set defined for tool: {:?}, falling back to default ability \
                         set.",
                        tool.kind
                    );
                    Default::default()
                };

                Ok(ItemConfig { abilities })
            },
            ItemKind::Glider => item
                .ability_spec()
                .and_then(|set_key| ability_map.get_ability_set(&set_key))
                .map(|abilities| ItemConfig {
                    abilities: abilities.clone(),
                })
                .ok_or(ItemConfigError::BadItemKind),
            _ => Err(ItemConfigError::BadItemKind),
        }
    }
}

impl ItemDef {
    pub fn is_stackable(&self) -> bool {
        matches!(
            self.kind,
            ItemKind::Consumable { .. }
                | ItemKind::Quest
                | ItemKind::Ingredient { .. }
                | ItemKind::Utility { .. }
                | ItemKind::Tool(Tool {
                    kind: ToolKind::Throwable,
                    ..
                })
        )
    }

    /// NOTE: invariant that amount() ≤ max_amount(), 1 ≤ max_amount(),
    /// and if !self.is_stackable(), self.max_amount() = 1.
    pub fn max_amount(&self) -> u32 { if self.is_stackable() { u32::MAX } else { 1 } }

    // currently needed by trade_pricing
    pub fn id(&self) -> &str { &self.item_definition_id }

    #[cfg(test)]
    pub fn new_test(
        item_definition_id: String,
        kind: ItemKind,
        quality: Quality,
        tags: Vec<ItemTag>,
        slots: u16,
    ) -> Self {
        #[expect(deprecated)]
        Self {
            item_definition_id,
            legacy_name: "test item name".to_owned(),
            kind,
            quality,
            tags,
            slots,
            ability_spec: None,
        }
    }

    #[cfg(test)]
    pub fn create_test_itemdef_from_kind(kind: ItemKind) -> Self {
        #[expect(deprecated)]
        Self {
            item_definition_id: "test.item".to_string(),
            legacy_name: "test item name".to_owned(),
            kind,
            quality: Quality::Common,
            tags: vec![],
            slots: 0,
            ability_spec: None,
        }
    }
}

/// NOTE: This PartialEq instance is pretty broken!  It doesn't check item
/// amount or any child items (and, arguably, doing so should be able to ignore
/// things like item order within the main inventory or within each bag, and
/// possibly even coalesce amounts, though these may be more controversial).
/// Until such time as we find an actual need for a proper PartialEq instance,
/// please don't rely on this for anything!
impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        (match (&self.item_base, &other.item_base) {
            (ItemBase::Simple(our_def), ItemBase::Simple(other_def)) => {
                our_def.item_definition_id == other_def.item_definition_id
            },
            (ItemBase::Modular(our_base), ItemBase::Modular(other_base)) => our_base == other_base,
            _ => false,
        }) && self.components() == other.components()
    }
}

impl Asset for ItemDef {
    fn load(cache: &AssetCache, specifier: &SharedString) -> Result<Self, BoxedError> {
        if specifier.starts_with("veloren.core.") {
            return Err(format!(
                "Attempted to load an asset from a specifier reserved for core veloren functions. \
                 Specifier: {}",
                specifier
            )
            .into());
        }

        let RawItemDef {
            legacy_name,
            legacy_description: _,
            kind,
            quality,
            tags,
            slots,
            ability_spec,
        } = cache.load::<Ron<_>>(specifier)?.cloned().into_inner();

        // Some commands like /give_item provide the asset specifier separated with \
        // instead of .
        //
        // TODO: This probably does not belong here
        let item_definition_id = specifier.replace('\\', ".");

        Ok(ItemDef {
            item_definition_id,
            #[expect(deprecated)]
            legacy_name,
            kind,
            quality,
            tags,
            slots,
            ability_spec,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename = "ItemDef", deny_unknown_fields)]
struct RawItemDef {
    legacy_name: String,
    legacy_description: String,
    kind: ItemKind,
    quality: Quality,
    tags: Vec<ItemTag>,
    #[serde(default)]
    slots: u16,
    ability_spec: Option<AbilitySpec>,
}

#[derive(Debug)]
pub struct OperationFailure;

impl Item {
    pub const MAX_DURABILITY: u32 = 12;

    // TODO: consider alternatives such as default abilities that can be added to a
    // loadout when no weapon is present
    pub fn empty() -> Self { Item::new_from_asset_expect("common.items.weapons.empty.empty") }

    pub fn new_from_item_base(
        inner_item: ItemBase,
        components: Vec<Item>,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Self {
        let mut item = Item {
            item_id: Arc::new(AtomicCell::new(None)),
            amount: NonZeroU32::new(1).unwrap(),
            components,
            slots: vec![None; inner_item.num_slots() as usize],
            item_base: inner_item,
            // These fields are updated immediately below
            item_config: None,
            hash: 0,
            durability_lost: None,
        };
        item.durability_lost = item.has_durability().then_some(0);
        item.update_item_state(ability_map, msm);
        item
    }

    pub fn new_from_item_definition_id(
        item_definition_id: ItemDefinitionId<'_>,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Result<Self, Error> {
        let (base, components) = match item_definition_id {
            ItemDefinitionId::Simple(spec) => {
                let base = ItemBase::Simple(Arc::<ItemDef>::load_cloned(&spec)?);
                (base, Vec::new())
            },
            ItemDefinitionId::Modular {
                pseudo_base,
                components,
            } => {
                let base = ItemBase::Modular(ModularBase::load_from_pseudo_id(pseudo_base));
                let components = components
                    .into_iter()
                    .map(|id| Item::new_from_item_definition_id(id, ability_map, msm))
                    .collect::<Result<Vec<_>, _>>()?;
                (base, components)
            },
            ItemDefinitionId::Compound {
                simple_base,
                components,
            } => {
                let base = ItemBase::Simple(Arc::<ItemDef>::load_cloned(simple_base)?);
                let components = components
                    .into_iter()
                    .map(|id| Item::new_from_item_definition_id(id, ability_map, msm))
                    .collect::<Result<Vec<_>, _>>()?;
                (base, components)
            },
        };
        Ok(Item::new_from_item_base(base, components, ability_map, msm))
    }

    /// Creates a new instance of an `Item` from the provided asset identifier
    /// Panics if the asset does not exist.
    pub fn new_from_asset_expect(asset_specifier: &str) -> Self {
        Item::new_from_asset(asset_specifier).unwrap_or_else(|err| {
            panic!(
                "Expected asset to exist: {}, instead got error {:?}",
                asset_specifier, err
            );
        })
    }

    /// Creates a Vec containing one of each item that matches the provided
    /// asset glob pattern
    pub fn new_from_asset_glob(asset_glob: &str) -> Result<Vec<Self>, Error> {
        let specifier = asset_glob.strip_suffix(".*").unwrap_or(asset_glob);
        let defs = assets::load_rec_dir::<Ron<RawItemDef>>(specifier)?;
        defs.read()
            .ids()
            .map(|id| Item::new_from_asset(id))
            .collect()
    }

    /// Creates a new instance of an `Item from the provided asset identifier if
    /// it exists
    pub fn new_from_asset(asset: &str) -> Result<Self, Error> {
        let inner_item = ItemBase::from_item_id_string(asset)?;
        // TODO: Get msm and ability_map less hackily
        let msm = &MaterialStatManifest::load().read();
        let ability_map = &AbilityMap::load().read();
        Ok(Item::new_from_item_base(
            inner_item,
            Vec::new(),
            ability_map,
            msm,
        ))
    }

    /// Creates a [`FrontendItem`] out of this item for frontend use
    #[must_use]
    pub fn frontend_item(
        &self,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> FrontendItem {
        FrontendItem(self.duplicate(ability_map, msm))
    }

    /// Duplicates an item, creating an exact copy but with a new item ID
    #[must_use]
    pub fn duplicate(&self, ability_map: &AbilityMap, msm: &MaterialStatManifest) -> Self {
        let duplicated_components = self
            .components
            .iter()
            .map(|comp| comp.duplicate(ability_map, msm))
            .collect();
        let mut new_item = Item::new_from_item_base(
            match &self.item_base {
                ItemBase::Simple(item_def) => ItemBase::Simple(Arc::clone(item_def)),
                ItemBase::Modular(mod_base) => ItemBase::Modular(mod_base.clone()),
            },
            duplicated_components,
            ability_map,
            msm,
        );
        new_item.set_amount(self.amount()).expect(
            "`new_item` has the same `item_def` and as an invariant, \
             self.set_amount(self.amount()) should always succeed.",
        );
        new_item.slots_mut().iter_mut().zip(self.slots()).for_each(
            |(new_item_slot, old_item_slot)| {
                *new_item_slot = old_item_slot
                    .as_ref()
                    .map(|old_item| old_item.duplicate(ability_map, msm));
            },
        );
        new_item
    }

    pub fn stacked_duplicates<'a>(
        &'a self,
        ability_map: &'a AbilityMap,
        msm: &'a MaterialStatManifest,
        count: u32,
    ) -> impl Iterator<Item = Self> + 'a {
        let max_stack_count = count / self.max_amount();
        let rest = count % self.max_amount();

        (0..max_stack_count)
            .map(|_| {
                let mut item = self.duplicate(ability_map, msm);

                item.set_amount(item.max_amount())
                    .expect("max_amount() is always a valid amount.");

                item
            })
            .chain((rest > 0).then(move || {
                let mut item = self.duplicate(ability_map, msm);

                item.set_amount(rest)
                    .expect("anything less than max_amount() is always a valid amount.");

                item
            }))
    }

    /// FIXME: HACK: In order to set the entity ID asynchronously, we currently
    /// start it at None, and then atomically set it when it's saved for the
    /// first time in the database.  Because this requires shared mutable
    /// state if these aren't synchronized by the program structure,
    /// currently we use an Atomic inside an Arc; this is clearly very
    /// dangerous, so in the future we will hopefully have a better way of
    /// dealing with this.
    #[doc(hidden)]
    pub fn get_item_id_for_database(&self) -> Arc<ItemId> { Arc::clone(&self.item_id) }

    /// Resets the item's item ID to None, giving it a new identity. Used when
    /// dropping items into the world so that a new database record is
    /// created when they are picked up again.
    ///
    /// NOTE: The creation of a new `Arc` when resetting the item ID is critical
    /// because every time a new `Item` instance is created, it is cloned from
    /// a single asset which results in an `Arc` pointing to the same value in
    /// memory. Therefore, every time an item instance is created this
    /// method must be called in order to give it a unique identity.
    fn reset_item_id(&mut self) {
        if let Some(item_id) = Arc::get_mut(&mut self.item_id) {
            *item_id = AtomicCell::new(None);
        } else {
            self.item_id = Arc::new(AtomicCell::new(None));
        }
        // Reset item id for every component of an item too
        for component in self.components.iter_mut() {
            component.reset_item_id();
        }
    }

    /// Removes the unique identity of an item - used when dropping an item on
    /// the floor. In the future this will need to be changed if we want to
    /// maintain a unique ID for an item even when it's dropped and picked
    /// up by another player.
    pub fn put_in_world(&mut self) { self.reset_item_id() }

    pub fn increase_amount(&mut self, increase_by: u32) -> Result<(), OperationFailure> {
        let amount = u32::from(self.amount);
        self.amount = amount
            .checked_add(increase_by)
            .filter(|&amount| amount <= self.max_amount())
            .and_then(NonZeroU32::new)
            .ok_or(OperationFailure)?;
        Ok(())
    }

    pub fn decrease_amount(&mut self, decrease_by: u32) -> Result<(), OperationFailure> {
        let amount = u32::from(self.amount);
        self.amount = amount
            .checked_sub(decrease_by)
            .and_then(NonZeroU32::new)
            .ok_or(OperationFailure)?;
        Ok(())
    }

    pub fn set_amount(&mut self, give_amount: u32) -> Result<(), OperationFailure> {
        if give_amount <= self.max_amount() {
            self.amount = NonZeroU32::new(give_amount).ok_or(OperationFailure)?;
            Ok(())
        } else {
            Err(OperationFailure)
        }
    }

    pub fn persistence_access_add_component(&mut self, component: Item) {
        self.components.push(component);
    }

    pub fn persistence_access_mutable_component(&mut self, index: usize) -> Option<&mut Self> {
        self.components.get_mut(index)
    }

    /// Updates state of an item (important for creation of new items,
    /// persistence, and if components are ever added to items after initial
    /// creation)
    pub fn update_item_state(&mut self, ability_map: &AbilityMap, msm: &MaterialStatManifest) {
        // Updates item config of an item
        if let Ok(item_config) = ItemConfig::try_from((&*self, ability_map, msm)) {
            self.item_config = Some(Box::new(item_config));
        }
        // Updates hash of an item.
        // DET-ADD-008 (determinism audit): stable across toolchain/library
        // upgrades — was std::hash::DefaultHasher (SipHash), which is NOT a
        // stable algorithm across Rust versions, so this semantic id could
        // silently shift on an upgrade. Same Sha256 primitive as DomainHasher.
        self.hash = crate::state_hash::stable_hash_u64("bastion/domain/item-hash/v1", self);
    }

    /// Returns an iterator that drains items contained within the item's slots
    pub fn drain(&mut self) -> impl Iterator<Item = Item> + '_ {
        self.slots.iter_mut().filter_map(mem::take)
    }

    pub fn item_definition_id(&self) -> ItemDefinitionId<'_> {
        match &self.item_base {
            ItemBase::Simple(item_def) => {
                if self.components.is_empty() {
                    ItemDefinitionId::Simple(Cow::Borrowed(&item_def.item_definition_id))
                } else {
                    ItemDefinitionId::Compound {
                        simple_base: &item_def.item_definition_id,
                        components: self
                            .components
                            .iter()
                            .map(|item| item.item_definition_id())
                            .collect(),
                    }
                }
            },
            ItemBase::Modular(mod_base) => ItemDefinitionId::Modular {
                pseudo_base: mod_base.pseudo_item_id(),
                components: self
                    .components
                    .iter()
                    .map(|item| item.item_definition_id())
                    .collect(),
            },
        }
    }

    pub fn is_same_item_def(&self, item_def: &ItemDef) -> bool {
        if let ItemBase::Simple(self_def) = &self.item_base {
            self_def.item_definition_id == item_def.item_definition_id
        } else {
            false
        }
    }

    pub fn matches_recipe_input(&self, recipe_input: &RecipeInput, amount: u32) -> bool {
        match recipe_input {
            RecipeInput::Item(item_def) => self.is_same_item_def(item_def),
            RecipeInput::Tag(tag) => self.tags().contains(tag),
            RecipeInput::TagSameItem(tag) => {
                self.tags().contains(tag) && u32::from(self.amount) >= amount
            },
            RecipeInput::ListSameItem(item_defs) => item_defs.iter().any(|item_def| {
                self.is_same_item_def(item_def) && u32::from(self.amount) >= amount
            }),
        }
    }

    pub fn is_salvageable(&self) -> bool {
        self.tags()
            .iter()
            .any(|tag| matches!(tag, ItemTag::SalvageInto(_, _)))
    }

    pub fn salvage_output(&self) -> impl Iterator<Item = (&str, u32)> {
        self.tags().into_iter().filter_map(|tag| {
            if let ItemTag::SalvageInto(material, quantity) = tag {
                material
                    .asset_identifier()
                    .map(|material_id| (material_id, quantity))
            } else {
                None
            }
        })
    }

    #[deprecated = "since item i18n"]
    pub fn legacy_name(&self) -> Cow<'_, str> {
        match &self.item_base {
            ItemBase::Simple(item_def) => {
                if self.components.is_empty() {
                    #[expect(deprecated)]
                    Cow::Borrowed(&item_def.legacy_name)
                } else {
                    #[expect(deprecated)]
                    modular::modify_name(&item_def.legacy_name, self)
                }
            },
            #[expect(deprecated, reason = "since item i18n")]
            ItemBase::Modular(mod_base) => mod_base.generate_name(self.components()),
        }
    }

    pub fn kind(&self) -> Cow<'_, ItemKind> {
        match &self.item_base {
            ItemBase::Simple(item_def) => Cow::Borrowed(&item_def.kind),
            ItemBase::Modular(mod_base) => {
                // TODO: Try to move further upward
                let msm = &MaterialStatManifest::load().read();
                mod_base.kind(self.components(), msm, self.stats_durability_multiplier())
            },
        }
    }

    pub fn amount(&self) -> u32 { u32::from(self.amount) }

    pub fn is_stackable(&self) -> bool {
        match &self.item_base {
            ItemBase::Simple(item_def) => item_def.is_stackable(),
            // TODO: Let whoever implements stackable modular items deal with this
            ItemBase::Modular(_) => false,
        }
    }

    /// NOTE: invariant that amount() ≤ max_amount(), 1 ≤ max_amount(),
    /// and if !self.is_stackable(), self.max_amount() = 1.
    pub fn max_amount(&self) -> u32 {
        match &self.item_base {
            ItemBase::Simple(item_def) => item_def.max_amount(),
            ItemBase::Modular(_) => {
                debug_assert!(!self.is_stackable());
                1
            },
        }
    }

    pub fn num_slots(&self) -> u16 { self.item_base.num_slots() }

    pub fn quality(&self) -> Quality {
        match &self.item_base {
            ItemBase::Simple(item_def) => item_def.quality.max(
                self.components
                    .iter()
                    .fold(Quality::MIN, |a, b| a.max(b.quality())),
            ),
            ItemBase::Modular(mod_base) => mod_base.compute_quality(self.components()),
        }
    }

    pub fn components(&self) -> &[Item] { &self.components }

    pub fn slots(&self) -> &[InvSlot] { &self.slots }

    pub fn slots_mut(&mut self) -> &mut [InvSlot] { &mut self.slots }

    pub fn item_config(&self) -> Option<&ItemConfig> { self.item_config.as_deref() }

    pub fn free_slots(&self) -> usize { self.slots.iter().filter(|x| x.is_none()).count() }

    pub fn populated_slots(&self) -> usize { self.slots().len().saturating_sub(self.free_slots()) }

    pub fn slot(&self, slot: usize) -> Option<&InvSlot> { self.slots.get(slot) }

    pub fn slot_mut(&mut self, slot: usize) -> Option<&mut InvSlot> { self.slots.get_mut(slot) }

    pub fn try_reclaim_from_block(
        block: Block,
        sprite_cfg: Option<&SpriteCfg>,
        rng: &mut impl rand::Rng,
    ) -> Option<Vec<(u32, Self)>> {
        if let Some(loot_spec) = sprite_cfg.and_then(|sprite_cfg| sprite_cfg.loot_table.as_ref()) {
            LootSpec::LootTable(loot_spec).to_items(rng)
        } else {
            block.get_sprite()?.default_loot_spec()??.to_items(rng)
        }
    }

    pub fn ability_spec(&self) -> Option<Cow<'_, AbilitySpec>> {
        match &self.item_base {
            ItemBase::Simple(item_def) => {
                item_def.ability_spec.as_ref().map(Cow::Borrowed).or({
                    // If no custom ability set is specified, fall back to abilityset of tool
                    // kind.
                    if let ItemKind::Tool(tool) = &item_def.kind {
                        Some(Cow::Owned(AbilitySpec::Tool(tool.kind)))
                    } else {
                        None
                    }
                })
            },
            ItemBase::Modular(mod_base) => mod_base.ability_spec(self.components()),
        }
    }

    // TODO: Maybe try to make slice again instead of vec? Could also try to make an
    // iterator?
    pub fn tags(&self) -> Vec<ItemTag> {
        match &self.item_base {
            ItemBase::Simple(item_def) => item_def.tags.to_vec(),
            // TODO: Do this properly. It'll probably be important at some point.
            ItemBase::Modular(mod_base) => mod_base.generate_tags(self.components()),
        }
    }

    pub fn is_modular(&self) -> bool {
        match &self.item_base {
            ItemBase::Simple(_) => false,
            ItemBase::Modular(_) => true,
        }
    }

    pub fn item_hash(&self) -> u64 { self.hash }

    pub fn persistence_item_id(&self) -> String {
        match &self.item_base {
            ItemBase::Simple(item_def) => item_def.item_definition_id.clone(),
            ItemBase::Modular(mod_base) => String::from(mod_base.pseudo_item_id()),
        }
    }

    pub fn durability_lost(&self) -> Option<u32> {
        self.durability_lost.map(|x| x.min(Self::MAX_DURABILITY))
    }

    pub fn stats_durability_multiplier(&self) -> DurabilityMultiplier {
        let durability_lost = self.durability_lost.unwrap_or(0);
        debug_assert!(durability_lost <= Self::MAX_DURABILITY);
        // How much durability must be lost before stats start to decay
        const DURABILITY_THRESHOLD: u32 = 9;
        const MIN_FRAC: f32 = 0.25;
        let mult = (1.0
            - durability_lost.saturating_sub(DURABILITY_THRESHOLD) as f32
                / (Self::MAX_DURABILITY - DURABILITY_THRESHOLD) as f32)
            * (1.0 - MIN_FRAC)
            + MIN_FRAC;
        DurabilityMultiplier(mult)
    }

    pub fn has_durability(&self) -> bool {
        self.kind().has_durability() && self.quality() != Quality::Debug
    }

    pub fn increment_damage(&mut self, ability_map: &AbilityMap, msm: &MaterialStatManifest) {
        if let Some(durability_lost) = &mut self.durability_lost
            && *durability_lost < Self::MAX_DURABILITY
        {
            *durability_lost += 1;
        }
        // Update item state after applying durability because stats have potential to
        // change from different durability
        self.update_item_state(ability_map, msm);
    }

    pub fn persistence_durability(&self) -> Option<NonZeroU32> {
        self.durability_lost.and_then(NonZeroU32::new)
    }

    pub fn persistence_set_durability(&mut self, value: Option<NonZeroU32>) {
        // If changes have been made so that item no longer needs to track durability,
        // set to None
        if !self.has_durability() {
            self.durability_lost = None;
        } else {
            // Set durability to persisted value, and if item previously had no durability,
            // set to Some(0) so that durability will be tracked
            self.durability_lost = Some(value.map_or(0, NonZeroU32::get));
        }
    }

    pub fn reset_durability(&mut self, ability_map: &AbilityMap, msm: &MaterialStatManifest) {
        self.durability_lost = self.has_durability().then_some(0);
        // Update item state after applying durability because stats have potential to
        // change from different durability
        self.update_item_state(ability_map, msm);
    }

    /// If an item is stackable and has an amount greater than the requested
    /// amount, decreases the amount of the original item by the same
    /// quantity and return a copy of the item with the taken amount.
    #[must_use = "Returned items will be lost if not used"]
    pub fn take_amount(
        &mut self,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
        returning_amount: u32,
    ) -> Option<Item> {
        if self.is_stackable() && self.amount() > 1 && returning_amount < self.amount() {
            let mut return_item = self.duplicate(ability_map, msm);
            self.decrease_amount(returning_amount).ok()?;
            return_item.set_amount(returning_amount).expect(
                "return_item.amount() = returning_amount < self.amount() (since self.amount() ≥ \
                 1) ≤ self.max_amount() = return_item.max_amount(), since return_item is a \
                 duplicate of item",
            );
            Some(return_item)
        } else {
            None
        }
    }

    /// If an item is stackable and has an amount greater than 1, creates a new
    /// item with half the amount (rounded down), and decreases the amount of
    /// the original item by the same quantity.
    #[must_use = "Returned items will be lost if not used"]
    pub fn take_half(
        &mut self,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Option<Item> {
        self.take_amount(ability_map, msm, self.amount() / 2)
    }

    #[cfg(test)]
    pub fn create_test_item_from_kind(kind: ItemKind) -> Self {
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();
        Self::new_from_item_base(
            ItemBase::Simple(Arc::new(ItemDef::create_test_itemdef_from_kind(kind))),
            Vec::new(),
            ability_map,
            msm,
        )
    }

    /// Checks if this item and another are suitable for grouping into the same
    /// [`PickupItem`].
    ///
    /// Also see [`Item::try_merge`].
    pub fn can_merge(&self, other: &Self) -> bool {
        if self.amount() > self.max_amount() || other.amount() > other.max_amount() {
            error!("An item amount is over max_amount!");
            return false;
        }

        (self == other)
            && self.slots().iter().all(Option::is_none)
            && other.slots().iter().all(Option::is_none)
            && self.durability_lost() == other.durability_lost()
    }

    /// Checks if this item and another are suitable for grouping into the same
    /// [`PickupItem`] and combines stackable items if possible.
    ///
    /// If the sum of both amounts is larger than their max amount, a remainder
    /// item is returned as `Ok(Some(remainder))`. A remainder item will
    /// always be produced for non-stackable items.
    ///
    /// If the items are not suitable for grouping `Err(other)` will be
    /// returned.
    pub fn try_merge(&mut self, mut other: Self) -> Result<Option<Self>, Self> {
        if self.can_merge(&other) {
            let max_amount = self.max_amount();
            debug_assert_eq!(
                max_amount,
                other.max_amount(),
                "Mergeable items must have the same max_amount()"
            );

            // Additional amount `self` can hold
            // For non-stackable items this is always zero
            let to_fill_self = max_amount
                .checked_sub(self.amount())
                .expect("can_merge should ensure that amount() <= max_amount()");

            if let Some(remainder) = other.amount().checked_sub(to_fill_self).filter(|r| *r > 0) {
                self.set_amount(max_amount)
                    .expect("max_amount() is always a valid amount.");
                other.set_amount(remainder).expect(
                    "We know remainder is more than 0 and less than or equal to max_amount()",
                );
                Ok(Some(other))
            } else {
                // If there would be no remainder, add the amounts!
                self.increase_amount(other.amount())
                    .expect("We know that we can at least add other.amount() to this item");
                drop(other);
                Ok(None)
            }
        } else {
            Err(other)
        }
    }

    // Probably doesn't need to be limited to persistence, but nothing else should
    // really need to look at item base
    pub fn persistence_item_base(&self) -> &ItemBase { &self.item_base }
}

impl FrontendItem {
    /// See [`Item::duplicate`], the returned item will still be a
    /// [`FrontendItem`]
    #[must_use]
    pub fn duplicate(&self, ability_map: &AbilityMap, msm: &MaterialStatManifest) -> Self {
        FrontendItem(self.0.duplicate(ability_map, msm))
    }

    pub fn set_amount(&mut self, amount: u32) -> Result<(), OperationFailure> {
        self.0.set_amount(amount)
    }
}

impl PickupItem {
    pub fn new(item: Item, time: ProgramTime, should_merge: bool) -> Self {
        Self {
            items: vec![item],
            created_at: time,
            next_merge_check: time,
            should_merge,
            instance_id: None,
        }
    }

    /// T0.49: stamp the persistent instance identity at the authoritative
    /// creation commit — set-once (a re-stamp is a logic error).
    pub fn set_instance_id(&mut self, id: ItemInstanceId) {
        debug_assert!(
            self.instance_id.is_none(),
            "item instance identity re-stamped"
        );
        self.instance_id = Some(id);
    }

    pub fn instance_id(&self) -> Option<ItemInstanceId> { self.instance_id }

    /// Get a reference to the last item in this stack
    ///
    /// The amount of this item should *not* be used.
    pub fn item(&self) -> &Item {
        self.items
            .last()
            .expect("PickupItem without at least one item is an invariant")
    }

    pub fn created(&self) -> ProgramTime { self.created_at }

    pub fn next_merge_check(&self) -> ProgramTime { self.next_merge_check }

    pub fn next_merge_check_mut(&mut self) -> &mut ProgramTime { &mut self.next_merge_check }

    // Get the total amount of items in here
    pub fn amount(&self) -> u32 {
        self.items
            .iter()
            .map(Item::amount)
            .fold(0, |total, amount| total.saturating_add(amount))
    }

    /// Remove any debug items if this is a container, used before dropping an
    /// item from an inventory
    pub fn remove_debug_items(&mut self) {
        for item in self.items.iter_mut() {
            item.slots_mut().iter_mut().for_each(|container_slot| {
                container_slot
                    .take_if(|contained_item| matches!(contained_item.quality(), Quality::Debug));
            });
        }
    }

    pub fn can_merge(&self, other: &PickupItem) -> bool {
        let self_item = self.item();
        let other_item = other.item();

        self.should_merge && other.should_merge && self_item.can_merge(other_item)
    }

    // Attempt to merge another PickupItem into this one, can only fail if
    // `can_merge` returns false
    pub fn try_merge(&mut self, mut other: PickupItem) -> Result<(), PickupItem> {
        if self.can_merge(&other) {
            // Pop the last item from `self` and `other` to merge them, as only the last
            // items can have an amount != max_amount()
            let mut self_last = self
                .items
                .pop()
                .expect("PickupItem without at least one item is an invariant");
            let other_last = other
                .items
                .pop()
                .expect("PickupItem without at least one item is an invariant");

            // Merge other_last into self_last
            let merged = self_last
                .try_merge(other_last)
                .expect("We know these items can be merged");

            debug_assert!(
                other
                    .items
                    .iter()
                    .chain(self.items.iter())
                    .all(|item| item.amount() == item.max_amount()),
                "All items before the last in `PickupItem` should have a full amount"
            );

            // We know all items except the last have a full amount, so we can safely append
            // them here
            self.items.append(&mut other.items);

            debug_assert!(
                merged.is_none() || self_last.amount() == self_last.max_amount(),
                "Merged can only be `Some` if the origin was set to `max_amount()`"
            );

            // Push the potentially not fully-stacked item at the end
            self.items.push(self_last);

            // Push the remainder, merged is only `Some` if self_last was set to
            // `max_amount()`
            if let Some(remainder) = merged {
                self.items.push(remainder);
            }

            Ok(())
        } else {
            Err(other)
        }
    }

    pub fn pick_up(mut self) -> (Item, Option<Self>) {
        (
            self.items
                .pop()
                .expect("PickupItem without at least one item is an invariant"),
            (!self.items.is_empty()).then_some(self),
        )
    }

    /// bastion (DECISIONS #89, Option B -- reservation capacity; FIXED
    /// 2026-08-11 per ITEM8-CRASH-FINDING.md): split ONE unit off this
    /// stack for a per-unit consumer (the eat path), returned to the
    /// caller as a value -- **never pushed into `self.items`**.
    ///
    /// The struct's own documented invariant ("any item that is not the
    /// last one must have an amount equal to its `max_amount()`") is
    /// unenforceable for stackables by CONSTRUCTION: `max_amount() ==
    /// u32::MAX` for a stackable (`Item::max_amount`), so a decremented
    /// stackable entry can never equal it. No ORDERING of a decremented
    /// entry and a fresh single ever satisfies the invariant -- reordering
    /// is not available as a fix, only never letting `self.items` grow
    /// past one entry is. This method's post-condition, proven by
    /// `split_off_one_never_grows_the_stack` below: `self.items.len()` is
    /// unchanged by every call, `Some` or `None`.
    ///
    /// **This used to push the split single as a new LAST entry** (so the
    /// existing, unmodified `pick_up()` would pop exactly it) -- creating
    /// `[Stack(39), Item(1)]`, a shape `try_merge`'s own debug_assert
    /// polices and this shape permanently violates. That comment named
    /// the exact failure ("`try_merge`'s own debug_assert on that
    /// invariant could in principle fire if this entity is merge-checked
    /// against a fresh drop of the same item while already split") and
    /// scoped it out of #89's own row; it detonated during item 8's
    /// endurance run (tick 45000, ~23.6 min in, ITEM8-CRASH-FINDING.md) —
    /// a scoped-out failure mode is a scheduled crash unless it's
    /// tracked, and a doc comment cannot page anyone when its own stated
    /// precondition becomes true.
    ///
    /// Finds the FIRST entry with `amount() >= 2` -- deliberately NOT
    /// blindly the last entry, matching the original reasoning: a caller
    /// that calls this twice in the same tick (two eaters) must find the
    /// real stack both times, not an already-split single. Since neither
    /// call ever grows `self.items`, both calls simply decrement the SAME
    /// single entry in sequence -- interleaving is free, not earned by
    /// index selection alone.
    ///
    /// Returns `None` (no mutation) if every entry is already down to
    /// amount 1 -- the caller's existing `pick_up()` already handles that
    /// case correctly on its own (consuming the whole remaining entity),
    /// no split needed.
    ///
    /// The returned single is `duplicate()`d -- a fresh item id, NEVER
    /// `Item::clone` (see that method's own warning: cloning shares the
    /// persistent database identity `Arc`, which two independently
    /// pickup-able units must not do).
    pub fn split_off_one(
        &mut self,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Option<Item> {
        let idx = self.items.iter().position(|it| it.amount() >= 2)?;
        let mut single = self.items[idx].duplicate(ability_map, msm);
        single
            .set_amount(1)
            .expect("amount 1 is always <= max_amount for anything with amount() >= 2");
        self.items[idx]
            .decrease_amount(1)
            .expect("just checked amount() >= 2, so decrease_amount(1) cannot underflow");
        Some(single)
    }
}

pub fn flatten_counted_items<'a>(
    items: &'a [(u32, Item)],
    ability_map: &'a AbilityMap,
    msm: &'a MaterialStatManifest,
) -> impl Iterator<Item = Item> + 'a {
    items
        .iter()
        .flat_map(|(count, item)| item.stacked_duplicates(ability_map, msm, *count))
}

/// Provides common methods providing details about an item definition
/// for either an `Item` containing the definition, or the actual `ItemDef`
pub trait ItemDesc {
    #[deprecated = "since item i18n"]
    fn legacy_name(&self) -> Cow<'_, str>;
    fn kind(&self) -> Cow<'_, ItemKind>;
    fn amount(&self) -> NonZeroU32;
    fn quality(&self) -> Quality;
    fn num_slots(&self) -> u16;
    fn item_definition_id(&self) -> ItemDefinitionId<'_>;
    fn tags(&self) -> Vec<ItemTag>;
    fn is_modular(&self) -> bool;
    fn components(&self) -> &[Item];
    fn has_durability(&self) -> bool;
    fn durability_lost(&self) -> Option<u32>;
    fn stats_durability_multiplier(&self) -> DurabilityMultiplier;

    fn tool_info(&self) -> Option<ToolKind> {
        if let ItemKind::Tool(tool) = &*self.kind() {
            Some(tool.kind)
        } else {
            None
        }
    }

    /// Return name's and description's localization descriptors
    fn i18n(&self, i18n: &ItemI18n) -> (Content, Content) {
        let item_key: ItemKey = self.into();

        let (name, description) = i18n.item_text_opt(&item_key).unwrap_or_else(|| {
            (
                #[expect(deprecated)]
                Content::Plain(self.legacy_name().to_string()),
                Content::Plain(String::new()),
            )
        });

        let b = |x| Box::new(x);
        if let ItemKey::ModularWeapon((comp_id, ing_id, hands)) = item_key {
            // the name template
            let title_fallback = Content::localized("weapon-modular-fallback-template")
                .with_arg(
                    "material-fragment",
                    i18n.try_fragment(&FragmentKey::Ingredient(ing_id))
                        // use Key instead of Plain here, so it's marked as
                        // "dirty" during get_content() and attempts English
                        .unwrap_or_else(|| Content::Key("Modular".to_owned())),
                )
                .with_arg(
                    "weapon",
                    i18n.try_fragment(&FragmentKey::WeaponPrimaryComponent(comp_id, hands))
                        // use Key instead of Plain here, so it's marked as
                        // "dirty" during get_content() and attempts English
                        .unwrap_or_else(|| Content::Key("Weapon".to_owned())),
                );

            (
                Content::WithFallback(b(name), b(title_fallback)),
                // no fallback for description, yet?
                description,
            )
        } else if let ItemKey::ModularWeaponComponent((comp_id, ing_id)) = item_key {
            // the name template
            let title_fallback = Content::localized("weapon-modular-comp-fallback-template")
                .with_arg(
                    "material-fragment",
                    i18n.try_fragment(&FragmentKey::Ingredient(ing_id))
                        // use Key instead of Plain here, so it's marked as
                        // "dirty" during get_content() and attempts English
                        .unwrap_or_else(|| Content::Key("Modular".to_owned())),
                )
                .with_arg(
                    "component",
                    i18n.try_key(&ItemKey::Simple(comp_id))
                        .map(|k| Content::Key(k.to_owned()))
                        // use Key instead of Plain here, so it's marked as
                        // "dirty" during get_content() and attempts English
                        .unwrap_or_else(|| Content::Key("Component".to_owned())),
                );

            (
                Content::WithFallback(b(name), b(title_fallback)),
                // no fallback for description, yet?
                description,
            )
        } else {
            (name, description)
        }
    }
}

impl ItemDesc for Item {
    fn legacy_name(&self) -> Cow<'_, str> {
        #[expect(deprecated)]
        self.legacy_name()
    }

    fn kind(&self) -> Cow<'_, ItemKind> { self.kind() }

    fn amount(&self) -> NonZeroU32 { self.amount }

    fn quality(&self) -> Quality { self.quality() }

    fn num_slots(&self) -> u16 { self.num_slots() }

    fn item_definition_id(&self) -> ItemDefinitionId<'_> { self.item_definition_id() }

    fn tags(&self) -> Vec<ItemTag> { self.tags() }

    fn is_modular(&self) -> bool { self.is_modular() }

    fn components(&self) -> &[Item] { self.components() }

    fn has_durability(&self) -> bool { self.has_durability() }

    fn durability_lost(&self) -> Option<u32> { self.durability_lost() }

    fn stats_durability_multiplier(&self) -> DurabilityMultiplier {
        self.stats_durability_multiplier()
    }
}

impl ItemDesc for FrontendItem {
    fn legacy_name(&self) -> Cow<'_, str> {
        #[expect(deprecated)]
        self.0.legacy_name()
    }

    fn kind(&self) -> Cow<'_, ItemKind> { self.0.kind() }

    fn amount(&self) -> NonZeroU32 { self.0.amount }

    fn quality(&self) -> Quality { self.0.quality() }

    fn num_slots(&self) -> u16 { self.0.num_slots() }

    fn item_definition_id(&self) -> ItemDefinitionId<'_> { self.0.item_definition_id() }

    fn tags(&self) -> Vec<ItemTag> { self.0.tags() }

    fn is_modular(&self) -> bool { self.0.is_modular() }

    fn components(&self) -> &[Item] { self.0.components() }

    fn has_durability(&self) -> bool { self.0.has_durability() }

    fn durability_lost(&self) -> Option<u32> { self.0.durability_lost() }

    fn stats_durability_multiplier(&self) -> DurabilityMultiplier {
        self.0.stats_durability_multiplier()
    }
}

impl ItemDesc for ItemDef {
    fn legacy_name(&self) -> Cow<'_, str> {
        #[expect(deprecated)]
        Cow::Borrowed(&self.legacy_name)
    }

    fn kind(&self) -> Cow<'_, ItemKind> { Cow::Borrowed(&self.kind) }

    fn amount(&self) -> NonZeroU32 { NonZeroU32::new(1).unwrap() }

    fn quality(&self) -> Quality { self.quality }

    fn num_slots(&self) -> u16 { self.slots }

    fn item_definition_id(&self) -> ItemDefinitionId<'_> {
        ItemDefinitionId::Simple(Cow::Borrowed(&self.item_definition_id))
    }

    fn tags(&self) -> Vec<ItemTag> { self.tags.to_vec() }

    fn is_modular(&self) -> bool { false }

    fn components(&self) -> &[Item] { &[] }

    fn has_durability(&self) -> bool {
        self.kind().has_durability() && self.quality != Quality::Debug
    }

    fn durability_lost(&self) -> Option<u32> { None }

    fn stats_durability_multiplier(&self) -> DurabilityMultiplier { DurabilityMultiplier(1.0) }
}

impl ItemDesc for PickupItem {
    fn legacy_name(&self) -> Cow<'_, str> {
        #[expect(deprecated)]
        self.item().legacy_name()
    }

    fn kind(&self) -> Cow<'_, ItemKind> { self.item().kind() }

    fn amount(&self) -> NonZeroU32 {
        NonZeroU32::new(self.amount()).expect("Item having amount of 0 is invariant")
    }

    fn quality(&self) -> Quality { self.item().quality() }

    fn num_slots(&self) -> u16 { self.item().num_slots() }

    fn item_definition_id(&self) -> ItemDefinitionId<'_> { self.item().item_definition_id() }

    fn tags(&self) -> Vec<ItemTag> { self.item().tags() }

    fn is_modular(&self) -> bool { self.item().is_modular() }

    fn components(&self) -> &[Item] { self.item().components() }

    fn has_durability(&self) -> bool { self.item().has_durability() }

    fn durability_lost(&self) -> Option<u32> { self.item().durability_lost() }

    fn stats_durability_multiplier(&self) -> DurabilityMultiplier {
        self.item().stats_durability_multiplier()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDrops(pub Vec<(u32, Item)>);

impl Component for ItemDrops {
    type Storage = DenseVecStorage<Self>;
}

impl Component for PickupItem {
    type Storage = DerefFlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl Component for ThrownItem {
    type Storage = DerefFlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Copy, Clone, Debug)]
pub struct DurabilityMultiplier(pub f32);

impl<T: ItemDesc + ?Sized> ItemDesc for &T {
    fn legacy_name(&self) -> Cow<'_, str> {
        #[expect(deprecated)]
        (*self).legacy_name()
    }

    fn kind(&self) -> Cow<'_, ItemKind> { (*self).kind() }

    fn amount(&self) -> NonZeroU32 { (*self).amount() }

    fn quality(&self) -> Quality { (*self).quality() }

    fn num_slots(&self) -> u16 { (*self).num_slots() }

    fn item_definition_id(&self) -> ItemDefinitionId<'_> { (*self).item_definition_id() }

    fn tags(&self) -> Vec<ItemTag> { (*self).tags() }

    fn is_modular(&self) -> bool { (*self).is_modular() }

    fn components(&self) -> &[Item] { (*self).components() }

    fn has_durability(&self) -> bool { (*self).has_durability() }

    fn durability_lost(&self) -> Option<u32> { (*self).durability_lost() }

    fn stats_durability_multiplier(&self) -> DurabilityMultiplier {
        (*self).stats_durability_multiplier()
    }
}

/// Returns all item asset specifiers
///
/// Panics in case of filesystem errors
pub fn all_item_defs_expect() -> Vec<String> {
    try_all_item_defs().expect("Failed to access items directory")
}

/// Returns all item asset specifiers
pub fn try_all_item_defs() -> Result<Vec<String>, Error> {
    let defs = assets::load_rec_dir::<Ron<RawItemDef>>("common.items")?;
    Ok(defs.read().ids().map(|id| id.to_string()).collect())
}

/// Designed to return all possible items, including modulars.
/// And some impossible too, like ItemKind::TagExamples.
pub fn all_items_expect() -> Vec<Item> {
    let defs = assets::load_rec_dir::<Ron<RawItemDef>>("common.items")
        .expect("failed to load item asset directory");

    // Grab all items from assets
    let mut asset_items: Vec<Item> = defs
        .read()
        .ids()
        .map(|id| Item::new_from_asset_expect(id))
        .collect();

    let mut material_parse_table = HashMap::new();
    for mat in Material::iter() {
        if let Some(id) = mat.asset_identifier() {
            material_parse_table.insert(id.to_owned(), mat);
        }
    }

    let primary_comp_pool = modular::PRIMARY_COMPONENT_POOL.clone();

    // Grab weapon primary components
    let mut primary_comps: Vec<Item> = primary_comp_pool
        .values()
        .flatten()
        .map(|(item, _hand_rules)| item.clone())
        .collect();

    // Grab modular weapons
    let mut modular_items: Vec<Item> = primary_comp_pool
        .keys()
        .flat_map(|(tool, mat_id)| {
            let mat = material_parse_table
                .get(mat_id)
                .expect("unexpected material ident");

            // get all weapons without imposing additional hand restrictions
            modular::generate_weapons(*tool, *mat, None)
                .expect("failure during modular weapon generation")
        })
        .collect();

    // 1. Append asset items, that should include pretty much everything,
    // except modular items
    // 2. Append primary weapon components, which are modular as well.
    // 3. Finally append modular weapons that are made from (1) and (2)
    // extend when we get some new exotic stuff
    //
    // P. s. I still can't wrap my head around the idea that you can put
    // tag example into your inventory.
    let mut all = Vec::new();
    all.append(&mut asset_items);
    all.append(&mut primary_comps);
    all.append(&mut modular_items);

    all
}

impl PartialEq<ItemDefinitionId<'_>> for ItemDefinitionIdOwned {
    fn eq(&self, other: &ItemDefinitionId<'_>) -> bool {
        use ItemDefinitionId as DefId;
        match self {
            Self::Simple(simple) => {
                matches!(other, DefId::Simple(other_simple) if simple == other_simple)
            },
            Self::Modular {
                pseudo_base,
                components,
            } => matches!(
                other,
                DefId::Modular { pseudo_base: other_base, components: other_comps }
                if pseudo_base == other_base && components == other_comps
            ),
            Self::Compound {
                simple_base,
                components,
            } => matches!(
                other,
                DefId::Compound { simple_base: other_base, components: other_comps }
                if simple_base == other_base && components == other_comps
            ),
        }
    }
}

impl PartialEq<ItemDefinitionIdOwned> for ItemDefinitionId<'_> {
    #[inline]
    fn eq(&self, other: &ItemDefinitionIdOwned) -> bool { other == self }
}

impl Equivalent<ItemDefinitionIdOwned> for ItemDefinitionId<'_> {
    fn equivalent(&self, key: &ItemDefinitionIdOwned) -> bool { self == key }
}

impl From<&ItemDefinitionId<'_>> for ItemDefinitionIdOwned {
    fn from(value: &ItemDefinitionId<'_>) -> Self { value.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hashbrown::HashSet;

    #[test]
    fn test_assets_items() {
        let ids = all_item_defs_expect();
        for item in ids.iter().map(|id| Item::new_from_asset_expect(id)) {
            if let ItemKind::Consumable {
                container: Some(container),
                ..
            } = item.kind().as_ref()
            {
                Item::new_from_item_definition_id(
                    container.as_ref(),
                    &AbilityMap::load().read(),
                    &MaterialStatManifest::load().read(),
                )
                .unwrap();
            }
            drop(item)
        }
    }

    /// bastion (DECISIONS #89, ROW69-OPTION-B-PACKET; REWRITTEN 2026-08-11
    /// per ITEM8-CRASH-FINDING.md): `split_off_one`'s ordering guarantee
    /// for two eaters in the same tick, re-proven against the FIXED
    /// (never-grows-`items`) implementation. The original version of this
    /// test asserted `items.len()` growing to 2 then 3 as EXPECTED,
    /// CORRECT behavior -- that was the shape that crashed live. Same
    /// scenario, corrected expectation: both splits decrement the SAME
    /// single entry in sequence; `items.len()` never leaves 1.
    #[test]
    fn bastion_split_off_one_two_eaters_same_tick() {
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();
        let mut mushroom = Item::new_from_asset_expect("common.items.food.mushroom");
        mushroom.set_amount(40).expect("mushroom is stackable");
        let mut stack = PickupItem::new(mushroom, ProgramTime(0.0), true);
        assert_eq!(stack.amount(), 40);
        assert_eq!(stack.items.len(), 1, "starts as one entry");

        // Split A: decrements the sole entry, returns the single as a
        // VALUE -- never pushed into `items`.
        let picked_a = stack
            .split_off_one(ability_map, msm)
            .expect("40 >= 2, must split");
        assert_eq!(picked_a.amount(), 1);
        assert_eq!(stack.amount(), 39, "the returned single is no longer counted in the stack");
        assert_eq!(stack.items.len(), 1, "split_off_one must never grow items -- this is its post-condition");

        // Split B, same tick, same entity: must still find the real
        // stack (now at 39) and decrement it again -- the interleaving
        // guarantee holds without ever growing `items`.
        let picked_b = stack
            .split_off_one(ability_map, msm)
            .expect("39 >= 2, must split");
        assert_eq!(picked_b.amount(), 1);
        assert_eq!(stack.amount(), 38);
        assert_eq!(stack.items.len(), 1);

        // Different underlying `Arc<ItemId>` allocations -- never a
        // shared `Item::clone` (the persistent-identity hazard
        // `split_off_one`'s own doc warns about; `AtomicCell` doesn't
        // implement `PartialEq`, so pointer identity is the correct
        // check here, not value equality).
        assert!(!std::sync::Arc::ptr_eq(&picked_a.item_id, &picked_b.item_id));

        // The stack itself is still a normal, single-entry PickupItem --
        // pick_up() consumes the whole remainder exactly as it always
        // has, untouched by any of this.
        let (picked_rest, remainder) = stack.pick_up();
        assert_eq!(picked_rest.amount(), 38);
        assert!(remainder.is_none(), "the entity is now empty");
    }

    /// The no-op case: every entry already down to amount 1 -- nothing
    /// to split, `pick_up()` already handles it correctly on its own.
    #[test]
    fn bastion_split_off_one_no_op_when_all_singles() {
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();
        let mut mushroom = Item::new_from_asset_expect("common.items.food.mushroom");
        mushroom.set_amount(1).expect("mushroom is stackable");
        let mut stack = PickupItem::new(mushroom, ProgramTime(0.0), true);
        assert!(stack.split_off_one(ability_map, msm).is_none());
        assert_eq!(stack.items.len(), 1, "no mutation on the no-op path");
        assert_eq!(stack.amount(), 1);
    }

    /// ITEM8-CRASH-FINDING.md: the planted reproduction. `split_off_one`'s
    /// ORIGINAL implementation pushed the split single as a new LAST
    /// entry, constructing `[Stack(39), Item(1)]` -- a shape whose FIRST
    /// entry (a decremented stackable, `max_amount() == u32::MAX`) can
    /// never satisfy `try_merge`'s "non-last entries must be at
    /// `max_amount()`" invariant. That shape, merge-checked against a
    /// fresh drop of the same item, is exactly what crashed item 8's
    /// endurance run at tick 45000. This test constructs the scenario
    /// directly and merge-checks it: RED against the pre-fix
    /// implementation (the debug_assert in `try_merge` fires), GREEN
    /// against the fix (the shape is now unconstructable via the public
    /// API, so nothing to merge-check panics).
    #[test]
    fn split_off_one_never_grows_the_stack_even_under_repeated_splits_then_merge() {
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();
        let mut mushroom = Item::new_from_asset_expect("common.items.food.mushroom");
        mushroom.set_amount(40).expect("mushroom is stackable");
        let mut stack = PickupItem::new(mushroom, ProgramTime(0.0), true);

        // Split repeatedly -- the exact "already split once, split again"
        // sequence the original comment flagged as the trap. Every call
        // must leave `items.len() == 1`.
        for expected_amount in (1..=39).rev() {
            let single = stack.split_off_one(ability_map, msm);
            assert!(single.is_some(), "40 units means 39 possible splits before exhaustion");
            assert_eq!(
                stack.items.len(),
                1,
                "invariant post-condition: items never grows, at any split depth"
            );
            assert_eq!(stack.amount(), expected_amount);
        }

        // The would-be-violating merge, exercised for real: a fresh drop
        // of the same item, merge-checked against the already-split
        // entity. This is the EXACT operation that panicked live. Must
        // not panic.
        let fresh_drop = PickupItem::new(
            Item::new_from_asset_expect("common.items.food.mushroom"),
            ProgramTime(0.0),
            true,
        );
        stack
            .try_merge(fresh_drop)
            .expect("same item, both should_merge=true, must be mergeable");
        assert_eq!(stack.items.len(), 1, "merging two single-entry stackables stays single-entry");
        assert_eq!(stack.amount(), 2, "1 remaining unit + the fresh drop's 1 unit");
    }

    #[test]
    fn test_item_i18n() { let _ = ItemI18n::new_expect(); }

    #[test]
    // Probably can't fail, but better safe than crashing production server
    fn test_all_items() { let _ = all_items_expect(); }

    #[test]
    // All items in Veloren should have localization.
    // If no, add some common dummy i18n id.
    fn ensure_item_localization() {
        let manifest = ItemI18n::new_expect();
        let items = all_items_expect();
        let mut errs = vec![];
        for item in items {
            let item_key: ItemKey = (&item).into();
            if manifest.item_text_opt(&item_key.clone()).is_none() {
                errs.push(item_key)
            }
        }
        if !errs.is_empty() {
            panic!("item i18n manifest misses translation-id for following items {errs:#?}")
        }
    }

    #[test]
    // This exists to make translators' lives easier when translating
    // modulars.
    fn ensure_modular_fragments() {
        let manifest = ItemI18n::new_expect();
        let items = all_items_expect();
        let mut errs = HashSet::new();

        for item in items {
            let item_key: ItemKey = (&item).into();
            if let ItemKey::ModularWeapon((comp_id, ing_id, hands)) = item_key {
                if manifest
                    .try_fragment(&FragmentKey::Ingredient(ing_id.clone()))
                    .is_none()
                {
                    errs.insert(FragmentKey::Ingredient(ing_id));
                }
                if manifest
                    .try_fragment(&FragmentKey::WeaponPrimaryComponent(comp_id.clone(), hands))
                    .is_none()
                {
                    errs.insert(FragmentKey::WeaponPrimaryComponent(comp_id, hands));
                }
            }
        }
        if !errs.is_empty() {
            panic!("item i18n manifest missing fragment-id for following items {errs:#?}")
        }
    }
}
