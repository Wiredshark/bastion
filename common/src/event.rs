use crate::{
    Explosion,
    character::CharacterId,
    combat::{AttackSource, AttackTarget, CombatEffect, DeathEffects, RiderEffects},
    comp::{
        self, ArcProperties, DisconnectReason, LootOwner, Ori, Pos, UnresolvedChatMsg, Vel,
        ability::Dodgeable,
        agent::Sound,
        beam,
        invite::{InviteKind, InviteResponse},
        slot::EquipSlot,
    },
    generation::{EntityInfo, SpecialEntity},
    interaction::Interaction,
    lottery::LootSpec,
    mounting::VolumePos,
    outcome::Outcome,
    resources::{BattleMode, Secs},
    rtsim::{self, RtSimEntity},
    states::basic_summon::BeamPillarIndicatorSpecifier,
    terrain::SpriteKind,
    trade::{TradeAction, TradeId},
    uid::Uid,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use specs::Entity as EcsEntity;
use std::{collections::VecDeque, sync::Mutex, time::Duration};
use uuid::Uuid;
use vek::*;

pub type SiteId = u64;
/// Plugin identifier (sha256)
pub type PluginHash = [u8; 32];

pub enum LocalEvent {
    /// Applies upward force to entity's `Vel`
    Jump(EcsEntity, f32),
    /// Applies the `impulse` to `entity`'s `Vel`
    ApplyImpulse {
        entity: EcsEntity,
        impulse: Vec3<f32>,
    },
    /// Applies `vel` velocity to `entity`
    Boost { entity: EcsEntity, vel: Vec3<f32> },
    /// Creates an outcome
    CreateOutcome(Outcome),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UpdateCharacterMetadata {
    pub skill_set_persistence_load_error: Option<comp::skillset::SkillsPersistenceError>,
}

pub struct NpcBuilder {
    pub stats: comp::Stats,
    pub skill_set: comp::SkillSet,
    pub health: Option<comp::Health>,
    pub poise: comp::Poise,
    pub inventory: comp::inventory::Inventory,
    pub body: comp::Body,
    pub agent: Option<comp::Agent>,
    pub alignment: comp::Alignment,
    pub scale: comp::Scale,
    pub anchor: Option<comp::Anchor>,
    pub loot: LootSpec<String>,
    pub pets: Vec<(NpcBuilder, Vec3<f32>)>,
    pub rtsim_entity: Option<RtSimEntity>,
    pub projectile: Option<comp::Projectile>,
    pub heads: Option<comp::body::parts::Heads>,
    pub death_effects: Option<DeathEffects>,
    pub rider_effects: Option<RiderEffects>,
    pub rider: Option<Box<Self>>,
}

impl NpcBuilder {
    pub fn new(stats: comp::Stats, body: comp::Body, alignment: comp::Alignment) -> Self {
        Self {
            stats,
            skill_set: comp::SkillSet::default(),
            health: None,
            poise: comp::Poise::new(body),
            inventory: comp::Inventory::with_empty(),
            body,
            agent: None,
            alignment,
            scale: comp::Scale(1.0),
            anchor: None,
            loot: LootSpec::Nothing,
            rtsim_entity: None,
            projectile: None,
            pets: Vec::new(),
            heads: None,
            death_effects: None,
            rider_effects: None,
            rider: None,
        }
    }

    pub fn with_rider(mut self, rider: impl Into<Option<NpcBuilder>>) -> Self {
        let rider: Option<NpcBuilder> = rider.into();
        self.rider = rider.map(Box::new);
        self
    }

    pub fn with_heads(mut self, heads: impl Into<Option<comp::body::parts::Heads>>) -> Self {
        self.heads = heads.into();
        self
    }

    pub fn with_health(mut self, health: impl Into<Option<comp::Health>>) -> Self {
        self.health = health.into();
        self
    }

    pub fn with_poise(mut self, poise: comp::Poise) -> Self {
        self.poise = poise;
        self
    }

    pub fn with_agent(mut self, agent: impl Into<Option<comp::Agent>>) -> Self {
        self.agent = agent.into();
        self
    }

    pub fn with_anchor(mut self, anchor: comp::Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    pub fn with_rtsim(mut self, rtsim: RtSimEntity) -> Self {
        self.rtsim_entity = Some(rtsim);
        self
    }

    pub fn with_projectile(mut self, projectile: impl Into<Option<comp::Projectile>>) -> Self {
        self.projectile = projectile.into();
        self
    }

    pub fn with_scale(mut self, scale: comp::Scale) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_inventory(mut self, inventory: comp::Inventory) -> Self {
        self.inventory = inventory;
        self
    }

    pub fn with_skill_set(mut self, skill_set: comp::SkillSet) -> Self {
        self.skill_set = skill_set;
        self
    }

    pub fn with_loot(mut self, loot: LootSpec<String>) -> Self {
        self.loot = loot;
        self
    }

    pub fn with_pets(mut self, pets: Vec<(NpcBuilder, Vec3<f32>)>) -> Self {
        self.pets = pets;
        self
    }

    pub fn with_death_effects(mut self, death_effects: Option<DeathEffects>) -> Self {
        self.death_effects = death_effects;
        self
    }

    pub fn with_rider_effects(mut self, rider_effects: Option<RiderEffects>) -> Self {
        self.rider_effects = rider_effects;
        self
    }
}

// These events are generated only by server systems
//
// TODO: we may want to move these into the server crate, this may allow moving
// other types out of `common` and would also narrow down where we know specific
// events will be emitted (if done it should probably be setup so they can
// easily be moved back here if needed).

pub struct ClientDisconnectEvent(pub EcsEntity, pub DisconnectReason);

pub struct ClientDisconnectWithoutPersistenceEvent(pub EcsEntity);

pub struct CommandEvent(pub EcsEntity, pub String, pub Vec<String>);

pub struct CreateSpecialEntityEvent {
    pub pos: Vec3<f32>,
    pub entity: SpecialEntity,
}

pub struct CreateShipEvent {
    pub pos: Pos,
    pub ori: Ori,
    pub ship: comp::ship::Body,
    pub rtsim_entity: Option<RtSimEntity>,
    pub driver: Option<NpcBuilder>,
}

pub struct CreateItemDropEvent {
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub item: comp::PickupItem,
    pub loot_owner: Option<LootOwner>,
    /// bastion (B5.5): colonist-produced drops are player resources — they
    /// never get a despawn timer and aggregate into piles (marked with
    /// `comp::bastion::BastionPile`; merges stay within the persistent
    /// class). Vanilla emitters pass `false` — behavior unchanged.
    pub persistent: bool,
}

pub struct CreateObjectEvent {
    pub pos: Pos,
    pub vel: Vel,
    pub body: comp::object::Body,
    pub object: Option<comp::Object>,
    pub item: Option<comp::PickupItem>,
    pub light_emitter: Option<comp::LightEmitter>,
    pub stats: Option<comp::Stats>,
}

/// Inserts default components for a character when loading into the game.
pub struct InitializeCharacterEvent {
    pub entity: EcsEntity,
    pub character_id: CharacterId,
    pub requested_view_distances: crate::ViewDistances,
}

pub struct InitializeSpectatorEvent(pub EcsEntity, pub crate::ViewDistances);

pub struct UpdateCharacterDataEvent {
    pub entity: EcsEntity,
    pub components: (
        comp::Body,
        Option<comp::Hardcore>,
        comp::Stats,
        comp::SkillSet,
        comp::Inventory,
        Option<comp::Waypoint>,
        Vec<(comp::Pet, comp::Body, comp::Stats)>,
        comp::ActiveAbilities,
        Option<comp::MapMarker>,
    ),
    pub metadata: UpdateCharacterMetadata,
}

pub struct ExitIngameEvent {
    pub entity: EcsEntity,
}

pub struct RequestSiteInfoEvent {
    pub entity: EcsEntity,
    pub id: SiteId,
}

pub struct TamePetEvent {
    pub pet_entity: EcsEntity,
    pub owner_entity: EcsEntity,
}

pub struct UpdateMapMarkerEvent {
    pub entity: EcsEntity,
    pub update: comp::MapMarkerChange,
}

pub struct MakeAdminEvent {
    pub entity: EcsEntity,
    pub admin: comp::Admin,
    pub uuid: Uuid,
}

pub struct DeleteCharacterEvent {
    pub entity: EcsEntity,
    pub requesting_player_uuid: String,
    pub character_id: CharacterId,
}

pub struct TeleportToPositionEvent {
    pub entity: EcsEntity,
    pub position: Vec3<f32>,
}

#[cfg(feature = "plugins")]
pub struct RequestPluginsEvent {
    pub entity: EcsEntity,
    pub plugins: Vec<PluginHash>,
}

/// APEX-T2.5.11: the typed artifact request (root + exact ordinals),
/// served from the server's compiled deployment. Sibling of the legacy
/// `RequestPluginsEvent`, which remains for explicit legacy mode.
#[cfg(feature = "plugins")]
pub struct RequestPluginArtifactsEvent {
    pub entity: EcsEntity,
    pub deployment_root: [u8; 32],
    pub ordinals: Vec<u32>,
}

pub struct SetBattleModeEvent {
    pub entity: EcsEntity,
    pub battle_mode: BattleMode,
}

// These events are generated in common systems in addition to server systems
// (but note on the client the event buses aren't registered and these events
// aren't actually emitted).

pub struct ChatEvent {
    pub msg: UnresolvedChatMsg,
    // We warn when the server tries to generate non plain `Content` messags
    // that appear from a player since we currently filter those out.
    //
    // But we don't want to spam warnings if this is from a client, so track that here.
    pub from_client: bool,
}

pub struct CreateNpcEvent {
    pub pos: Pos,
    pub ori: Ori,
    pub npc: NpcBuilder,
}

pub struct CreateNpcGroupEvent {
    pub npcs: Vec<CreateNpcEvent>,
}

pub struct CreateAuraEntityEvent {
    pub auras: comp::Auras,
    pub pos: Pos,
    pub creator_uid: Uid,
    pub duration: Option<Secs>,
}

pub struct ExplosionEvent {
    pub pos: Vec3<f32>,
    pub explosion: Explosion,
    pub owner: Option<Uid>,
}

pub struct ArcingEvent {
    pub arc: ArcProperties,
    pub owner: Option<Uid>,
    pub target: Uid,
    pub pos: Pos,
}

pub struct CreatePoolEvent {
    pub properties: comp::pool::PoolProperties,
    pub owner: Option<Uid>,
    pub pos: Pos,
    pub ori: Ori,
}

pub struct BonkEvent {
    pub pos: Vec3<f32>,
    pub owner: Option<Uid>,
    pub target: Option<Uid>,
}

pub struct HealthChangeEvent {
    pub entity: EcsEntity,
    pub change: comp::HealthChange,
}

pub struct KillEvent {
    pub entity: EcsEntity,
}

pub struct HelpDownedEvent {
    pub helper: Option<Uid>,
    pub target: Uid,
}

pub struct DownedEvent {
    pub entity: EcsEntity,
}

pub struct PoiseChangeEvent {
    pub entity: EcsEntity,
    pub change: comp::PoiseChange,
}

pub struct DeleteEvent(pub EcsEntity);

pub struct DestroyEvent {
    pub entity: EcsEntity,
    pub cause: comp::HealthChange,
}

pub struct InventoryManipEvent(pub EcsEntity, pub comp::InventoryManip);

pub struct GroupManipEvent(pub EcsEntity, pub comp::GroupManip);

pub struct RespawnEvent(pub EcsEntity);

pub struct ShootEvent {
    // This should be the owner entity
    pub entity: Option<EcsEntity>,
    pub source_vel: Option<Vel>,
    pub pos: Pos,
    pub dir: Dir,
    pub body: comp::Body,
    pub light: Option<comp::LightEmitter>,
    pub projectile: comp::Projectile,
    pub speed: f32,
    pub object: Option<comp::Object>,
    pub marker: Option<comp::FrontendMarker>,
}

pub struct ThrowEvent {
    pub entity: EcsEntity,
    pub pos: Pos,
    pub dir: Dir,
    pub light: Option<comp::LightEmitter>,
    pub projectile: comp::Projectile,
    pub speed: f32,
    pub object: Option<comp::Object>,
    pub equip_slot: EquipSlot,
}

pub struct ShockwaveEvent {
    pub properties: comp::shockwave::Properties,
    pub pos: Pos,
    pub ori: Ori,
}

pub struct KnockbackEvent {
    pub entity: EcsEntity,
    pub impulse: Vec3<f32>,
}

pub struct LandOnGroundEvent {
    pub entity: EcsEntity,
    pub vel: Vec3<f32>,
    pub surface_normal: Vec3<f32>,
}

pub struct SetLanternEvent(pub EcsEntity, pub bool);

pub struct NpcInteractEvent(pub EcsEntity, pub EcsEntity);

pub struct DialogueEvent(pub EcsEntity, pub EcsEntity, pub rtsim::Dialogue);

pub struct InviteResponseEvent(pub EcsEntity, pub InviteResponse);

pub struct InitiateInviteEvent(pub EcsEntity, pub Uid, pub InviteKind);

pub struct ProcessTradeActionEvent(pub EcsEntity, pub TradeId, pub TradeAction);

pub enum MountEvent {
    MountEntity(EcsEntity, EcsEntity),
    MountVolume(EcsEntity, VolumePos),
    Unmount(EcsEntity),
}

pub struct SetPetStayEvent(pub EcsEntity, pub EcsEntity, pub bool);

pub struct PossessEvent(pub Uid, pub Uid);

pub struct TransformEvent {
    pub target_entity: Uid,
    pub entity_info: EntityInfo,
    /// If set to false, players wont be transformed unless with a Possessor
    /// presence kind
    pub allow_players: bool,
    /// Whether the entity should be deleted if transforming fails (only applies
    /// to non-players)
    pub delete_on_failure: bool,
}

pub struct StartInteractionEvent(pub Interaction);

pub struct AuraEvent {
    pub entity: EcsEntity,
    pub aura_change: comp::AuraChange,
}

pub struct BuffEvent {
    pub entity: EcsEntity,
    pub buff_change: comp::BuffChange,
}

pub struct EnergyChangeEvent {
    pub entity: EcsEntity,
    pub change: f32,
    pub reset_rate: bool,
}

pub struct ComboChangeEvent {
    pub entity: EcsEntity,
    pub change: i32,
}

pub struct ParryHookEvent {
    pub defender: EcsEntity,
    pub attacker: Option<EcsEntity>,
    pub source: AttackSource,
    pub poise_multiplier: f32,
}

/// Attempt to mine a block, turning it into an item.
pub struct MineBlockEvent {
    pub entity: EcsEntity,
    pub pos: Vec3<i32>,
    pub tool: Option<comp::tool::ToolKind>,
}

pub struct TeleportToEvent {
    pub entity: EcsEntity,
    pub target: Uid,
    pub max_range: Option<f32>,
}

pub struct SoundEvent {
    pub sound: Sound,
}

pub struct CreateSpriteEvent {
    pub pos: Vec3<i32>,
    pub sprite: SpriteKind,
    pub del_timeout: Option<(f32, f32)>,
}

pub struct EntityAttackedHookEvent {
    pub entity: EcsEntity,
    pub attacker: Option<EcsEntity>,
    pub attack_dir: Dir,
    pub damage_dealt: f32,
    pub attack_source: AttackSource,
}

pub struct ChangeAbilityEvent {
    pub entity: EcsEntity,
    pub slot: usize,
    pub auxiliary_key: comp::ability::AuxiliaryKey,
    pub new_ability: comp::ability::AuxiliaryAbility,
}

pub struct ChangeStanceEvent {
    pub entity: EcsEntity,
    pub stance: comp::Stance,
}

pub struct PermanentChange {
    pub expected_old_body: comp::Body,
}

pub struct ChangeBodyEvent {
    pub entity: EcsEntity,
    pub new_body: comp::Body,
    /// Is Some if this change should be persisted.
    ///
    /// Only applies to player characters.
    pub permanent_change: Option<PermanentChange>,
}

pub struct RemoveLightEmitterEvent {
    pub entity: EcsEntity,
}

pub struct StartTeleportingEvent {
    pub entity: EcsEntity,
    pub portal: EcsEntity,
}

pub struct ToggleSpriteLightEvent {
    pub entity: EcsEntity,
    pub pos: Vec3<i32>,
    pub enable: bool,
}

pub struct RegrowHeadEvent {
    pub entity: EcsEntity,
}

pub struct SummonBeamPillarsEvent {
    pub summoner: EcsEntity,
    pub target: AttackTarget,
    pub buildup_duration: Duration,
    pub attack_duration: Duration,
    pub beam_duration: Duration,
    pub radius: f32,
    pub height: f32,
    pub damage: f32,
    pub damage_effect: Option<CombatEffect>,
    pub dodgeable: Dodgeable,
    pub tick_rate: f32,
    pub specifier: beam::FrontendSpecifier,
    pub indicator_specifier: BeamPillarIndicatorSpecifier,
}

/// T0.29-31 (master build order; T0-003; Sonnet's ruling): the per-event
/// stamp — assigned by the EMITTER at bus append, invisible to consumers
/// (drains strip it after the stable merge).
#[derive(Clone, Debug)]
pub struct EventStamp {
    /// The bus's drain epoch when this event was staged — the recorded
    /// ORIGIN FRAME (the ruling's keep-tick addition; the bus-side
    /// tick-analog: epoch N = consumed by drain N+1). Phase is subsumed by
    /// `producer` (the call site IS the system).
    pub epoch: u64,
    /// Producer identity: the emitter's creation call site — static,
    /// unique per system site, same-binary-stable. The declared rank the
    /// stable merge sorts by (never mutex arrival order).
    pub producer: &'static core::panic::Location<'static>,
    /// Producer-local sequence within the batch.
    pub seq: u32,
    /// T0.31: relation identities — machinery present, unpopulated until a
    /// producer opts in (no fake total chronology).
    pub causation: Option<u64>,
    pub correlation: Option<u64>,
    pub idempotency: Option<u64>,
}

struct Stamped<E> {
    stamp: EventStamp,
    event: E,
}

struct EventBusInner<E> {
    queue: VecDeque<Stamped<E>>,
    /// Drain epoch — incremented by every recv; stamps record it.
    epoch: u64,
    /// Saturates to u8::MAX and is never reset.
    ///
    /// Used in the first tick to check for if certain event types are handled
    /// and only handled once.
    ///
    /// T0.26: lives in ALL builds — release topology validation reads it
    /// (one u8 per bus, saturating adds; the check itself runs once).
    recv_count: u8,
}

pub struct EventBus<E> {
    inner: Mutex<EventBusInner<E>>,
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self {
            inner: Mutex::new(EventBusInner {
                queue: VecDeque::new(),
                epoch: 0,
                recv_count: 0,
            }),
        }
    }
}

impl<E> EventBus<E> {
    #[track_caller]
    pub fn emitter(&self) -> Emitter<'_, E> {
        Emitter {
            bus: self,
            events: VecDeque::new(),
            producer: core::panic::Location::caller(),
        }
    }

    #[track_caller]
    pub fn emit_now(&self, event: E) {
        let producer = core::panic::Location::caller();
        let mut guard = self.inner.lock().expect("Poisoned");
        let epoch = guard.epoch;
        guard.queue.push_back(Stamped {
            stamp: EventStamp {
                epoch,
                producer,
                seq: 0,
                causation: None,
                correlation: None,
                idempotency: None,
            },
            event,
        });
    }

    /// T0.30: every drain performs the STABLE MERGE — sort by
    /// (epoch, producer site, producer-local seq); batch arrival (mutex)
    /// order is never authoritative. Stamps are stripped here; consumers
    /// see plain events.
    fn merge_sorted(mut queue: VecDeque<Stamped<E>>) -> impl ExactSizeIterator<Item = E> {
        queue.make_contiguous().sort_by(|a, b| {
            (
                a.stamp.epoch,
                a.stamp.producer.file(),
                a.stamp.producer.line(),
                a.stamp.producer.column(),
                a.stamp.seq,
            )
                .cmp(&(
                    b.stamp.epoch,
                    b.stamp.producer.file(),
                    b.stamp.producer.line(),
                    b.stamp.producer.column(),
                    b.stamp.seq,
                ))
        });
        queue.into_iter().map(|stamped| stamped.event)
    }

    pub fn recv_all(&self) -> impl ExactSizeIterator<Item = E> + use<E> {
        Self::merge_sorted({
            let mut guard = self.inner.lock().expect("Poisoned");
            guard.recv_count = guard.recv_count.saturating_add(1);
            guard.epoch += 1;
            core::mem::take(&mut guard.queue)
        })
    }

    pub fn recv_all_mut(&mut self) -> impl ExactSizeIterator<Item = E> + use<E> {
        let inner = self.inner.get_mut().expect("Poisoned");
        inner.recv_count = inner.recv_count.saturating_add(1);
        inner.epoch += 1;
        Self::merge_sorted(core::mem::take(&mut inner.queue))
    }

    pub fn recv_count(&mut self) -> u8 { self.inner.get_mut().expect("Poisoned").recv_count }
}

pub struct Emitter<'a, E> {
    bus: &'a EventBus<E>,
    pub events: VecDeque<E>,
    /// T0.29: the producer identity stamped onto this batch at append.
    producer: &'static core::panic::Location<'static>,
}

impl<E> Emitter<'_, E> {
    pub fn emit(&mut self, event: E) { self.events.push_back(event); }

    pub fn emit_many(&mut self, events: impl IntoIterator<Item = E>) { self.events.extend(events); }

    pub fn append(&mut self, other: &mut VecDeque<E>) { self.events.append(other) }

    pub fn append_vec(&mut self, vec: Vec<E>) {
        if self.events.is_empty() {
            self.events = vec.into();
        } else {
            self.events.extend(vec);
        }
    }
}

impl<E> Drop for Emitter<'_, E> {
    fn drop(&mut self) {
        if !self.events.is_empty() {
            let mut guard = self.bus.inner.lock().expect("Poision");
            let epoch = guard.epoch;
            for (index, event) in self.events.drain(..).enumerate() {
                guard.queue.push_back(Stamped {
                    stamp: EventStamp {
                        epoch,
                        producer: self.producer,
                        seq: index as u32,
                        causation: None,
                        correlation: None,
                        idempotency: None,
                    },
                    event,
                });
            }
        }
    }
}

pub trait EmitExt<E> {
    fn emit(&mut self, event: E);
    fn emit_many(&mut self, events: impl IntoIterator<Item = E>);
}

/// Define ecs read data for event busses. And a way to convert them all to
/// emitters.
///
/// # Example:
/// ```
/// mod some_mod_is_necessary_for_the_test {
///     use veloren_common::event_emitters;
///     pub struct Foo;
///     pub struct Bar;
///     pub struct Baz;
///     event_emitters!(
///       pub struct ReadEvents[EventEmitters] {
///           foo: Foo, bar: Bar, baz: Baz,
///       }
///     );
/// }
/// ```
#[macro_export]
macro_rules! event_emitters {
    // `APEX-T7.3b`: the 3-bracket form, opted into per call site (only
    // `CharacterStateEvents` needs it today) rather than added to the
    // 2-bracket form below, so the other 21 call sites of this macro
    // are untouched -- zero blast radius on anything but the one
    // struct that actually needs a replay sink.
    ($($vis:vis struct $read_data:ident[$emitters:ident][$sink:ident] { $($(#[$($tt:tt)*])? $ev_ident:ident: $ty:ty),+ $(,)? })+) => {
        mod event_emitters {
            use super::*;
            use specs::shred;
            $(
            #[derive(specs::SystemData)]
            pub struct $read_data<'a> {
                $($(#[$($tt)*])? $ev_ident: Option<specs::Read<'a, $crate::event::EventBus<$ty>>>),+
            }

            impl<'a> $read_data<'a> {
                pub fn get_emitters(&self) -> $emitters<'_> {
                    $emitters {
                        $($(#[$($tt)*])? $ev_ident: self.$ev_ident.as_ref().map(|e| e.emitter())),+
                    }
                }
            }

            pub struct $emitters<'a> {
                $($(#[$($tt)*])? $ev_ident: Option<$crate::event::Emitter<'a, $ty>>),+
            }

            impl<'a> $emitters<'a> {
                #[expect(unused)]
                pub fn append(&mut self, mut other: Self) {
                    $(
                        $(#[$($tt)*])?
                        {self.$ev_ident.as_mut().zip(other.$ev_ident).map(|(a, mut b)| a.append(&mut b.events));}
                    )+
                }
            }

            $(
                $(#[$($tt)*])?
                impl<'a> $crate::event::EmitExt<$ty> for $emitters<'a> {
                    fn emit(&mut self, event: $ty) { self.$ev_ident.as_mut().map(|e| e.emit(event)); }
                    fn emit_many(&mut self, events: impl IntoIterator<Item = $ty>) { self.$ev_ident.as_mut().map(|e| e.emit_many(events)); }
                }
            )+

            /// `APEX-T7.3b`: a throwaway sink for [`$emitters`]. Every
            /// channel here is a fresh, unshared `EventBus` -- emitters
            /// borrowed from this sink (via [`Self::emitters`]) write
            /// into THESE buses, never the live ones a real system
            /// drains. A replayed frame's events already fired during
            /// the original predicted pass; replay recomputes STATE,
            /// and re-delivering its events into a live bus would be
            /// the double-fire hazard Decision 4 exists to prevent --
            /// so discarding here is the correct semantics, not a
            /// compromise standing in for one. Never silent about it:
            /// [`Self::drain_counts_v1`] reports what landed and where,
            /// per channel, so a replay caller can assert both that a
            /// known-emitting state DID get captured here and that the
            /// live buses stayed untouched.
            pub struct $sink {
                $($(#[$($tt)*])? $ev_ident: $crate::event::EventBus<$ty>),+
            }

            impl Default for $sink {
                fn default() -> Self {
                    Self {
                        $($(#[$($tt)*])? $ev_ident: $crate::event::EventBus::default()),+
                    }
                }
            }

            impl $sink {
                /// Construct a fresh `Self` per replay call -- reusing
                /// one sink across calls would accumulate counts from
                /// every earlier call into the next one's assertions.
                pub fn emitters(&self) -> $emitters<'_> {
                    $emitters {
                        $($(#[$($tt)*])? $ev_ident: Some(self.$ev_ident.emitter())),+
                    }
                }

                /// Per-channel captured-and-discarded counts. Call
                /// AFTER the `$emitters` borrowing this sink has been
                /// dropped -- `Emitter` flushes into its bus on drop,
                /// so counts read before that drop would undercount.
                /// Draining (not just measuring queue length) is
                /// deliberate: it is also the proof the events were
                /// real entries, not an artifact of a queue nothing
                /// ever reads.
                pub fn drain_counts_v1(&self) -> Vec<(&'static str, usize)> {
                    vec![
                        $((stringify!($ev_ident), self.$ev_ident.recv_all().count())),+
                    ]
                }
            }
            )+
        }
        $(
            $vis use event_emitters::{$read_data, $emitters, $sink};
        )+
    };
    ($($vis:vis struct $read_data:ident[$emitters:ident] { $($(#[$($tt:tt)*])? $ev_ident:ident: $ty:ty),+ $(,)? })+) => {
        mod event_emitters {
            use super::*;
            use specs::shred;
            $(
            #[derive(specs::SystemData)]
            pub struct $read_data<'a> {
                $($(#[$($tt)*])? $ev_ident: Option<specs::Read<'a, $crate::event::EventBus<$ty>>>),+
            }

            impl<'a> $read_data<'a> {
                pub fn get_emitters(&self) -> $emitters<'_> {
                    $emitters {
                        $($(#[$($tt)*])? $ev_ident: self.$ev_ident.as_ref().map(|e| e.emitter())),+
                    }
                }
            }

            pub struct $emitters<'a> {
                $($(#[$($tt)*])? $ev_ident: Option<$crate::event::Emitter<'a, $ty>>),+
            }

            impl<'a> $emitters<'a> {
                #[expect(unused)]
                pub fn append(&mut self, mut other: Self) {
                    $(
                        $(#[$($tt)*])?
                        {self.$ev_ident.as_mut().zip(other.$ev_ident).map(|(a, mut b)| a.append(&mut b.events));}
                    )+
                }
            }

            $(
                $(#[$($tt)*])?
                impl<'a> $crate::event::EmitExt<$ty> for $emitters<'a> {
                    fn emit(&mut self, event: $ty) { self.$ev_ident.as_mut().map(|e| e.emit(event)); }
                    fn emit_many(&mut self, events: impl IntoIterator<Item = $ty>) { self.$ev_ident.as_mut().map(|e| e.emit_many(events)); }
                }
            )+
            )+
        }
        $(
            $vis use event_emitters::{$read_data, $emitters};
        )+
    }
}


// T0.29-31: the stable merge is producer-declared order, never mutex
// arrival order — two batches appended reversed still drain in call-site
// order, with origin epochs recorded.
#[cfg(test)]
mod t0_29_tests {
    use super::*;

    #[test]
    fn t0_30_drain_orders_by_producer_site_not_arrival() {
        let bus: EventBus<u32> = EventBus::default();
        // Site A (earlier line) buffered first but DROPPED SECOND.
        let mut emitter_a = bus.emitter();
        emitter_a.emit(1);
        emitter_a.emit(2);
        {
            let mut emitter_b = bus.emitter();
            emitter_b.emit(10);
            drop(emitter_b); // arrives FIRST
        }
        drop(emitter_a); // arrives second
        let drained: Vec<u32> = bus.recv_all().collect();
        // Producer-site order (a's site is the earlier line), local seq
        // within each batch.
        assert_eq!(drained, vec![1, 2, 10]);

        // Epoch advances per drain; a post-drain emit lands in the next
        // epoch and still drains cleanly.
        bus.emit_now(7);
        let again: Vec<u32> = bus.recv_all().collect();
        assert_eq!(again, vec![7]);
    }
}
