use crate::{
    EditableSettings, Settings,
    client::Client,
    login_provider::{LoginProvider, PendingLogin},
    metrics::PlayerMetrics,
    session_registry::{AuthenticatedIntentV1, DEFAULT_DETACHED_RETENTION_CAP, SessionRegistry},
    settings::{BanOperation, banlist::NormalizedIpAddr},
    sys::sentinel::TrackedStorages,
};
use authc::Uuid;
use common::{
    comp::{self, Admin, AdminRole, Player, Stats},
    event::{ClientDisconnectEvent, EventBus, MakeAdminEvent},
    recipe::default_component_recipe_book,
    resources::TimeOfDay,
    shared_server_config::ServerConstants,
    uid::Uid,
};
use common::apex::identity::{OsRandomBytesSourceV1, ServerBootId};
use common_base::prof_span;

/// APEX-T3.1.09: the exact boot-scope admission check `ClientRegister`
/// must pass before `login_provider.verify` runs. Extracted to a free
/// function (rather than left inline) so `bastion-harness`'s T3.1.17
/// process-restart fixture can call the identical production code path,
/// not a copy of the comparison it re-derived itself.
pub fn check_register_boot_scope(
    expected: ServerBootId,
    current: ServerBootId,
) -> Result<(), RegisterError> {
    if expected != current {
        Err(RegisterError::ServerBootMismatch { current, received: expected })
    } else {
        Ok(())
    }
}

/// `APEX-T3.3.05`: the phase-1 "requested protocol is server-supported"
/// check (row status doc requirement 1's first insertion point). Extracted
/// to a free function for the same reason `check_register_boot_scope`
/// above is -- directly unit-testable without a full ECS harness, and a
/// single production code path instead of an inline comparison a test
/// would have to re-derive.
pub fn check_semantic_protocol_supported(
    requested: common_net::msg::envelope::SemanticProtocolIdV1,
    supported: &[common_net::msg::envelope::SemanticProtocolIdV1],
) -> Result<(), RegisterError> {
    if supported.contains(&requested) {
        Ok(())
    } else {
        Err(RegisterError::IncompatibleSemanticProtocol)
    }
}

#[cfg(test)]
mod semantic_protocol_negotiation_tests {
    use common_net::msg::envelope::SemanticProtocolIdV1;
    use common_net::msg::server_supported_semantic_protocols_v1;

    use super::check_semantic_protocol_supported;
    use common_net::msg::RegisterError;

    /// Packet section 5.9's own test list: "Legacy ordinary" and "V1".
    #[test]
    fn requested_protocol_in_supported_set_is_accepted() {
        let supported = server_supported_semantic_protocols_v1();
        assert!(check_semantic_protocol_supported(SemanticProtocolIdV1::Legacy, &supported).is_ok());
        assert!(check_semantic_protocol_supported(SemanticProtocolIdV1::NetEnvelopeV1, &supported).is_ok());
    }

    /// Packet section 5.9's own test list: "no-overlap".
    #[test]
    fn requested_protocol_outside_supported_set_is_rejected() {
        // A server operator restricting to certified-only (NetEnvelopeV1
        // alone) is exactly the "certified mode requires V1" policy the
        // packet describes -- expressed here as a supported-set value,
        // not a separate config toggle (see row status doc's note that
        // T4.1 owns the real config surface).
        let certified_only = vec![SemanticProtocolIdV1::NetEnvelopeV1];
        assert_eq!(
            check_semantic_protocol_supported(SemanticProtocolIdV1::Legacy, &certified_only),
            Err(RegisterError::IncompatibleSemanticProtocol)
        );
        assert!(check_semantic_protocol_supported(SemanticProtocolIdV1::NetEnvelopeV1, &certified_only).is_ok());
    }
}
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{
    CharacterInfo, ClientRegister, DisconnectReason, PlayerInfo, PlayerListUpdate,
    RegisterError, ServerGeneral, ServerInit, SessionAdmissionV1, WorldMapMsg,
    server::ServerDescription, server_supported_semantic_protocols_v1,
};
use hashbrown::HashMap;
use rayon::prelude::*;
use specs::{
    Entities, Entity, Join, LendJoin, ParJoin, Read, ReadExpect, ReadStorage, SystemData,
    WriteExpect, WriteStorage, shred,
};
use std::time::Instant;
use tracing::{debug, info, trace, warn};

#[cfg(feature = "plugins")]
use common_state::plugin::PluginMgr;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    stats: ReadStorage<'a, Stats>,
    uids: ReadStorage<'a, Uid>,
    /// APEX-T3.1.09: compared against `ClientRegister.expected_server_boot_id`
    /// before any `login_provider.verify` call.
    server_boot_id: ReadExpect<'a, ServerBootId>,
    client_disconnect_events: Read<'a, EventBus<ClientDisconnectEvent>>,
    make_admin_events: Read<'a, EventBus<MakeAdminEvent>>,
    login_provider: ReadExpect<'a, LoginProvider>,
    player_metrics: ReadExpect<'a, PlayerMetrics>,
    settings: ReadExpect<'a, Settings>,
    time_of_day: Read<'a, TimeOfDay>,
    material_stats: ReadExpect<'a, comp::item::MaterialStatManifest>,
    ability_map: ReadExpect<'a, comp::item::tool::AbilityMap>,
    recipe_book: ReadExpect<'a, common::recipe::RecipeBookManifest>,
    map: ReadExpect<'a, WorldMapMsg>,
    trackers: TrackedStorages<'a>,
    #[cfg(feature = "plugins")]
    plugin_mgr: Read<'a, PluginMgr>,
    #[cfg(feature = "plugins")]
    plugin_deployment: ReadExpect<'a, crate::plugin_deployment_policy::PluginDeploymentStateV1>,
    data_dir: ReadExpect<'a, crate::DataDir>,
}

/// One completed, banned/whitelist/client-type-checked authentication,
/// collected during the parallel phase and committed against
/// `SessionRegistry` only in the single sequential pass afterward (spec
/// section 2.2 item 3: registry mutation never happens inside the
/// parallel phase, canaries SES-024/065).
struct CollectedAdmissionV1 {
    entity: Entity,
    uid: Uid,
    principal: Uuid,
    intent: AuthenticatedIntentV1,
    player: Player,
    admin_role: Option<AdminRole>,
    player_list_update_msg: Option<crate::client::PreparedMsg>,
}

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, PendingLogin>,
        WriteExpect<'a, EditableSettings>,
        WriteExpect<'a, SessionRegistry>,
    );

    const NAME: &'static str = "msg::register";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (read_data, mut clients, mut players, mut pending_logins, mut editable_settings, mut session_registry): Self::SystemData,
    ) {
        let mut make_admin_emitter = read_data.make_admin_events.emitter();
        // Player list to send new players, and lookup from UUID to entity (so we don't
        // have to do a linear scan over all entities on each login to see if
        // it's a duplicate).
        //
        // NOTE: For this to work as desired, we must maintain the invariant that there
        // is just one player per UUID!
        let (player_list, old_players_by_uuid): (HashMap<_, _>, HashMap<_, _>) = (
            &read_data.entities,
            &read_data.uids,
            clients.maybe(),
            &players,
            read_data.stats.maybe(),
            read_data.trackers.admin.maybe(),
        )
            .join()
            .filter(|(_, _, client, _, _, _)| {
                client.is_none_or(|client| client.client_type.emit_login_events())
            })
            .map(|(entity, uid, _, player, stats, admin)| {
                (
                    (*uid, PlayerInfo {
                        is_online: true,
                        is_moderator: admin.is_some(),
                        player_alias: player.alias.clone(),
                        character: stats.map(|stats| CharacterInfo {
                            name: stats.name.clone(),
                            // NOTE: hack, read docs for body::Gender for more
                            gender: stats.original_body.humanoid_gender(),
                            battle_mode: player.battle_mode,
                        }),
                        uuid: player.uuid(),
                    }),
                    (player.uuid(), entity),
                )
            })
            .unzip();
        let max_players = usize::from(read_data.settings.max_players);

        // Phase 1 (sequential, unchanged in spirit): drain ClientRegister,
        // allocate this entity's SessionAttemptSeqV1 HERE -- before the
        // awaited auth race below even begins (spec section 2.2 item 1,
        // canary SES-051) -- and kick off LoginProvider::verify.
        for (entity, client) in (&read_data.entities, &mut clients).join() {
            let mut locale = None;

            let _ = super::try_recv_all(client, 0, |_, msg: ClientRegister| {
                trace!(?msg.token_or_username, "defer auth lockup");
                // Opus 5's T3.2 boundary-review finding: never default a
                // failed allocation -- a defaulted attempt_seq is
                // indistinguishable from a real `0`, which would let two
                // same-principal registrations in one pass collide on the
                // exact attempt_seq value `admit_sorted`'s own doc comment
                // requires the caller to prevent (`BLOCK-AMBIGUOUS-ATTEMPT`).
                // `Exhausted` here means this process has issued
                // `u64::MAX` registration attempts since it started (this
                // counter is process-lifetime, never persisted or
                // restored) -- unreachable in practice, and by the time it
                // happens process state has far bigger problems than one
                // rejected login, so a loud panic is correct: there is no
                // well-formed `RegisterError` response to build without a
                // valid attempt_seq to build it from.
                let attempt_seq = session_registry
                    .allocate_attempt_seq()
                    .expect("SessionAttemptSeqV1 (u64, process-lifetime) exhausted -- process state is unrecoverable");
                // APEX-T3.1.09: compare before authentication -- a stale
                // registration from a client that observed a prior server
                // process's ServerInfo must never reach login_provider.verify.
                let current = *read_data.server_boot_id;
                if let Err(err) = check_register_boot_scope(msg.expected_server_boot_id, current) {
                    debug!("Rejecting ClientRegister: server boot ID mismatch (client observed a prior server process)");
                    let pending = crate::login_provider::PendingLogin::new_failure(
                        err,
                        msg.session_request,
                        attempt_seq,
                        msg.requested_semantic_protocol,
                    );
                    let _ = pending_logins.insert(entity, pending);
                    return Ok(());
                }
                // APEX-T3.3.05: sequential-phase-confinement requirement 1
                // (row status doc, readme/apex/APEX-T3.3.05-ROW-STATUS-v1.md)
                // -- this check reads only the just-drained message against
                // a fixed constant set, no shared mutable state, placed as
                // a sibling to the boot-scope check above and before
                // `login_provider.verify` is ever invoked, same as it is.
                if let Err(err) =
                    check_semantic_protocol_supported(msg.requested_semantic_protocol, &server_supported_semantic_protocols_v1())
                {
                    debug!(?msg.requested_semantic_protocol, "Rejecting ClientRegister: unsupported semantic protocol");
                    let pending = crate::login_provider::PendingLogin::new_failure(
                        err,
                        msg.session_request,
                        attempt_seq,
                        msg.requested_semantic_protocol,
                    );
                    let _ = pending_logins.insert(entity, pending);
                    return Ok(());
                }
                let pending = read_data.login_provider.verify(
                    &msg.token_or_username,
                    msg.session_request,
                    attempt_seq,
                    msg.requested_semantic_protocol,
                );
                locale = msg.locale;
                let _ = pending_logins.insert(entity, pending);
                Ok(())
            });

            // Update locale
            if let Some(locale) = locale {
                client.locale = Some(locale);
            }
        }

        // NOTE: this is just default value.
        //
        // It will be overwritten in ServerExt::update_character_data.
        let battle_mode = read_data.settings.gameplay.battle_mode.default_mode();
        let mut upgradeable_bans: EventBus<(NormalizedIpAddr, Uuid, String)> = EventBus::default();

        // Phase 2 (parallel): resolve auth (ban/whitelist/client-type only
        // -- NO capacity check, NO SessionRegistry access here, per spec
        // section 2.2 item 3/5, canaries SES-023/024/065/083). Collect
        // every ban/whitelist/client-type-passed authentication for the
        // sequential commit pass below.
        let collected: parking_lot::Mutex<Vec<CollectedAdmissionV1>> = parking_lot::Mutex::new(Vec::new());
        let finished_pending: parking_lot::Mutex<Vec<Entity>> = parking_lot::Mutex::new(Vec::new());

        (
            &read_data.entities,
            &read_data.uids,
            &clients,
            !players.mask(),
            &mut pending_logins,
        )
            .join()
            // NOTE: Required because Specs has very poor work splitting for sparse joins.
            .par_bridge()
            .for_each_init(
                || (read_data.client_disconnect_events.emitter(), upgradeable_bans.emitter()),
                |(client_disconnect_emitter, upgradeable_ban_emitter), (entity, uid, client, _, pending)| {
                    prof_span!("msg::register login");
                    // No capacity gate here -- always "not exceeded"; capacity is
                    // decided once, in the sequential SessionRegistry commit pass
                    // (spec section 4 policy 1, canary SES-083).
                    let player_count_exceeded = |username: String, uuid: authc::Uuid| (false, (username, uuid));

                    match LoginProvider::login(
                        pending,
                        client,
                        &editable_settings.admins,
                        &editable_settings.whitelist,
                        &editable_settings.banlist,
                        player_count_exceeded,
                        |ip, uuid, username| upgradeable_ban_emitter.emit((ip, uuid, username)),
                    ) {
                        None => {},
                        Some(Err(e)) => {
                            finished_pending.lock().push(entity);
                            trace!(?e, "pending login returned error");
                            client_disconnect_emitter.emit(ClientDisconnectEvent(entity, common::comp::DisconnectReason::Kicked));
                            let _ = client.send(Err::<SessionAdmissionV1, _>(e));
                        },
                        Some(Ok((username, uuid))) => {
                            finished_pending.lock().push(entity);
                            let admin = editable_settings.admins.get(&uuid);
                            if !client.client_type.is_valid_for_role(admin.map(|admin| admin.role.into())) {
                                client_disconnect_emitter.emit(ClientDisconnectEvent(entity, common::comp::DisconnectReason::InvalidClientType));
                                return;
                            }
                            let player = Player::new(username, battle_mode, uuid, None);
                            let player_list_update_msg = player.is_valid().then(|| {
                                client.prepare(ServerGeneral::PlayerListUpdate(PlayerListUpdate::Add(
                                    *uid,
                                    PlayerInfo {
                                        player_alias: player.alias.clone(),
                                        is_online: true,
                                        is_moderator: admin.is_some(),
                                        character: None,
                                        uuid: player.uuid(),
                                    },
                                )))
                            });
                            if player_list_update_msg.is_none() {
                                let _ = client.send(Err::<SessionAdmissionV1, _>(RegisterError::InvalidCharacter));
                                return;
                            }
                            collected.lock().push(CollectedAdmissionV1 {
                                entity,
                                uid: *uid,
                                principal: uuid,
                                intent: AuthenticatedIntentV1 {
                                    principal: uuid,
                                    client_type: client.client_type,
                                    attempt_seq: pending.attempt_seq,
                                    request: pending.session_request,
                                    capacity_exempt: admin.is_some(),
                                    // APEX-T3.3.05: already validated against
                                    // the server's advertised set in phase 1
                                    // above; this parallel phase only carries
                                    // it forward, deciding nothing new with it.
                                    requested_semantic_protocol: pending.requested_semantic_protocol,
                                },
                                player,
                                admin_role: admin.map(|a| a.role.into()),
                                player_list_update_msg,
                            });
                        },
                    }
                },
            );

        let finished_pending = finished_pending.into_inner();
        let mut collected = collected.into_inner();

        finished_pending.into_iter().for_each(|e| {
            // Remove all entities in finished_pending from pending_logins.
            pending_logins.remove(e);
        });

        // Phase 3 (sequential): the ONLY place `SessionRegistry` is
        // mutated. Sorts and commits `collected` canonically (spec
        // section 2.2), then applies each outcome's ECS-lifecycle
        // consequence (fresh insert, same-principal takeover with a
        // retry-or-kick of the old entity exactly as before this row, or
        // a typed rejection) sequentially -- no race, no mutex needed
        // here at all.
        let intents: Vec<(Entity, AuthenticatedIntentV1)> = collected.iter().map(|c| (c.entity, c.intent.clone())).collect();
        let mut random_source = OsRandomBytesSourceV1;
        let outcomes = session_registry.admit_sorted(intents, max_players, Instant::now(), DEFAULT_DETACHED_RETENTION_CAP, &mut random_source);

        // Index collected by entity for the outcome pass below (collected
        // is consumed by value as we go, order doesn't matter anymore).
        collected.sort_by_key(|c| c.entity);
        let mut new_players: HashMap<Uuid, (Entity, Player, Option<AdminRole>, Option<crate::client::PreparedMsg>)> = HashMap::new();

        for (entity, outcome) in outcomes {
            let Ok(idx) = collected.binary_search_by_key(&entity, |c| c.entity) else { continue };
            let admission = collected.remove(idx);

            match outcome {
                Err(RegisterError::OlderAttemptSuperseded) => {
                    // Lost a same-phase race to a newer attempt from the
                    // same principal (SES-054/055) -- distinct from a kick,
                    // this connection simply never becomes a session.
                    let _ = clients.get(admission.entity).map(|c| c.send(Err::<SessionAdmissionV1, _>(RegisterError::OlderAttemptSuperseded)));
                    read_data.client_disconnect_events.emitter().emit(ClientDisconnectEvent(admission.entity, common::comp::DisconnectReason::Kicked));
                },
                Err(e) => {
                    let _ = clients.get(admission.entity).map(|c| c.send(Err::<SessionAdmissionV1, _>(e)));
                    read_data.client_disconnect_events.emitter().emit(ClientDisconnectEvent(admission.entity, common::comp::DisconnectReason::Kicked));
                },
                Ok(admitted @ (SessionAdmissionV1::Replaced { .. } | SessionAdmissionV1::Resumed { .. })) => {
                    // Same-principal takeover of a PRE-EXISTING (prior-tick,
                    // already ECS-committed) session. Unlike the pre-T3.2
                    // code, there is no same-tick retry-defer case to
                    // handle here: `SessionRegistry` already resolved any
                    // same-phase collision via `OlderAttemptSuperseded`
                    // above, so a `Replaced`/`Resumed` outcome can only
                    // ever correlate to an `old_players_by_uuid` entry from
                    // a genuinely earlier tick, whose `Player` component is
                    // already committed ECS storage -- no insert-ordering
                    // hazard, so no deferral is needed.
                    if let Some(&old_entity) = old_players_by_uuid.get(&admission.principal) {
                        match clients.get(old_entity) {
                            Some(old_client) => {
                                let _ = old_client.send(ServerGeneral::Disconnect(DisconnectReason::Kicked(String::from(
                                    "You have logged in from another location.",
                                ))));
                            },
                            None => {
                                warn!("Player without client detected for entity {:?}", old_entity);
                            },
                        }
                        read_data.client_disconnect_events.emitter().emit(ClientDisconnectEvent(old_entity, common::comp::DisconnectReason::NewerLogin));
                    }
                    finalize_admission(&read_data, &mut clients, &editable_settings, &player_list, &mut new_players, admission, admitted);
                },
                Ok(admitted @ SessionAdmissionV1::Created { .. }) => {
                    finalize_admission(&read_data, &mut clients, &editable_settings, &player_list, &mut new_players, admission, admitted);
                },
            }
        }

        // Handle new players.
        let msgs = new_players
            .into_values()
            .filter_map(|(entity, player, admin_role, msg)| {
                let username = &player.alias;
                let uuid = player.uuid();
                info!(?username, "New User");
                // Add Player component to this client.
                players
                    .insert(entity, player)
                    .expect("The entity was joined against in the same system, so it exists");

                // Give the Admin component to the player if their name exists in
                // admin list
                if let Some(role) = admin_role {
                    // We need to defer writing to the Admin storage since it's borrowed immutably
                    // by this system via TrackedStorages.
                    make_admin_emitter.emit(MakeAdminEvent { entity, admin: Admin(role), uuid });
                }
                msg
            })
            .collect::<Vec<_>>();

        // Tell all clients to add the new players to the player list, in parallel.
        (players.mask(), &clients)
            .par_join()
            .for_each(|(_, client)| {
                // Send messages sequentially within each client; by the time we have enough
                // players to make parallelizing useful, we will have way more
                // players than cores.
                msgs.iter().for_each(|msg| {
                    let _ = client.send_prepared(msg);
                });
            });

        for (ip, uuid, username) in upgradeable_bans.recv_all_mut() {
            if let Err(error) = editable_settings.banlist.ban_operation(
                read_data.data_dir.as_ref(),
                chrono::Utc::now(),
                uuid,
                username,
                BanOperation::UpgradeToIpBan { ip },
                false,
            ) {
                warn!(?error, ?uuid, "Upgrading ban to IP ban failed");
            }
        }
    }
}

/// `APEX-T3.3.16`: builds and sends `ServerInit::GameSync` (the only
/// caller, though genericized over the payload type the same way
/// `build_semantic_frame_v1` is -- lets the pure wiring here get
/// direct unit coverage with a lightweight `ServerGeneral` stand-in
/// instead of constructing a full `ServerInit::GameSync`, which has no
/// lightweight constructor in this crate) as a V1 envelope on the
/// Bootstrap stream iff the client has a live V1 attachment -- called
/// immediately after `reset_semantic_state`, so this is always the
/// FIRST `allocate_sequence(Bootstrap)` call on this attachment and
/// therefore always returns `1` ("Bootstrap sequence 1" falls out of
/// that reset; no special-casing needed here). Returns `false` (never
/// sent, no state mutated beyond the sequence allocation itself) for a
/// `Legacy` session or on any allocation/encode/send failure -- the
/// caller keeps its exact existing raw legacy send call as a fallback,
/// same dual-path shape as every other `T3.3.13`/`.14a`
/// producer. GameSync has no producer contention (one send per
/// successful admission, never batched with anything else), so unlike
/// `ServerGeneral` replication traffic this intentionally does NOT go
/// through `ServerSemanticOutboxV1` -- `SemanticSendIntentV1::payload`
/// is `Arc<ServerGeneral>` only (`server/src/semantic_net/outbox.rs`'s
/// own `ServerSemanticPayloadV1` alias), and widening that shared type
/// to carry `ServerInit` too would ripple into every existing producer
/// for no benefit here.
/// `APEX-T4.1` chunk 2a: the manifest this server actually sends,
/// immediately before `GameSync`.
///
/// Deliberately minimal today: only the `NetEnvelope` slot, using the
/// already-frozen `net_envelope_profile_root_v1()` (`T3.3`'s own
/// registered content root) as real, honestly-computed content -- not a
/// placeholder. Content/plugin/schedule/numeric/world identity descriptors
/// need live server-state wiring `T4.1`'s own spec names as future work
/// within this row (content via `ContentManifest`, plugin via the active
/// deployment, schedule/numeric/world via rows that have not built their
/// identity-root functions yet); adding them here ahead of that wiring
/// would be fabricated content pretending to be checked, which this
/// program's own standing rule forbids. `freshness_reserved` stays `None`
/// -- `T4.2`'s reservation, not yet populated.
fn bootstrap_manifest_v1() -> common::apex::bootstrap_manifest::BootstrapManifestV1 {
    use common::apex::{
        digest::{ContentIdentityV1, hash_artifact_bytes_v1},
        scalar::SchemaVersion,
        subsystem::{SubsystemDescriptorV1, SubsystemSlotIdV1},
    };

    common::apex::bootstrap_manifest::BootstrapManifestV1 {
        descriptors: vec![SubsystemDescriptorV1 {
            slot: SubsystemSlotIdV1::NetEnvelope,
            schema: SchemaVersion::new(1),
            content: ContentIdentityV1 {
                artifact: hash_artifact_bytes_v1(common_net::msg::envelope::net_envelope_profile_root_v1().as_array()),
                semantic: None,
            },
        }],
        peer_selector: None,
        peer_capabilities: Vec::new(),
        freshness_reserved: None,
    }
}

fn try_send_gamesync_v1<T>(client: &mut Client, payload: &T) -> bool
where
    T: common_net::msg::envelope::SemanticRouteV1 + serde::Serialize,
{
    use common_net::msg::envelope::SemanticCausalityV1;

    let Some((binding, sequence)) = client.semantic_send_state_mut().and_then(|send_state| {
        let binding = send_state.binding();
        let stream = payload.semantic_stream();
        send_state.allocate_sequence(stream).ok().map(|seq| (binding, seq))
    }) else {
        return false;
    };
    let causality = SemanticCausalityV1 { producer_tick: None, snapshot: None };
    match crate::sys::semantic_egress::build_semantic_frame_v1(payload, binding, causality, sequence) {
        Ok((_, _, frame_bytes)) => client.send_semantic_frame(payload.semantic_stream(), frame_bytes).is_ok(),
        Err(e) => {
            warn!(?e, "gamesync V1 encode failure");
            false
        },
    }
}

/// Sends `RegisterAnswer`/`GameSync`/the initial player-list sync for a
/// successfully-committed admission (any of `Created`/`Replaced`/
/// `Resumed`) and stages the entity for the final `Player`-component
/// insertion pass. Mirrors the pre-`T3.2` success path exactly (same
/// three messages, same ordering), only the `RegisterAnswer`/`GameSync`
/// payloads themselves are new.
fn finalize_admission(
    read_data: &ReadData,
    clients: &mut WriteStorage<Client>,
    editable_settings: &EditableSettings,
    player_list: &HashMap<Uid, PlayerInfo>,
    new_players: &mut HashMap<Uuid, (Entity, Player, Option<AdminRole>, Option<crate::client::PreparedMsg>)>,
    admission: CollectedAdmissionV1,
    admitted: SessionAdmissionV1,
) {
    let Some(client) = clients.get_mut(admission.entity) else { return };
    read_data.player_metrics.players_connected.inc();

    // Tell the client its request was successful.
    if client.send(Ok::<_, RegisterError>(admitted)).is_err() {
        return;
    }

    #[cfg(feature = "plugins")]
    // APEX-T2.5.22: on a GOVERNED deployment the hash vector carries NO
    // bootstrap authority — the summary's roots + requirements are the
    // whole contract, so the legacy vector goes out EMPTY (structural,
    // not merely unread). Legacy sessions keep the exact old hash list.
    let active_plugins = if read_data.plugin_deployment.summary().is_some() {
        Vec::new()
    } else {
        read_data.plugin_mgr.plugin_list()
    };
    #[cfg(not(feature = "plugins"))]
    let active_plugins = Vec::default();

    let server_descriptions = &editable_settings.server_description;
    let description = ServerDescription {
        motd: server_descriptions.get(client.locale.as_deref()).map(|d| d.motd.clone()).unwrap_or_default(),
        rules: server_descriptions.get_rules(client.locale.as_deref()).map(str::to_string),
    };

    // Send client all the tracked components currently attached to its
    // entity as well as synced resources (currently only `TimeOfDay`).
    debug!("Starting initial sync with client.");
    let session_binding = admitted.binding();
    // APEX-T3.3.06: "accepted binding initializes; higher epoch replaces"
    // -- a fresh reset via the constructor every time admission succeeds,
    // never a partial update. Legacy carries no V1 state (packet's own
    // words), so this only fires for a NetEnvelopeV1 selection.
    if session_binding.selected_semantic_protocol == common_net::msg::SemanticProtocolIdV1::NetEnvelopeV1 {
        client.reset_semantic_state(common_net::msg::ActiveSessionBindingV1 {
            server_boot_id: *read_data.server_boot_id,
            session_id: session_binding.session_id,
            epoch: session_binding.epoch,
        });
    }
    // `APEX-T4.1` chunk 2a: send the bootstrap compatibility manifest
    // immediately before `GameSync`, in the same admission call, one
    // send call earlier than everything below -- order is this row's
    // whole mechanism, so this call sits directly ahead of `game_sync`'s
    // own construction rather than anywhere else in this function.
    // Encode failure is logged and the send is skipped, never a reason
    // to fail the admission this manifest is meant to protect.
    match common_net::msg::bootstrap_manifest_wire::BootstrapManifestWireV1::from_typed_v1(&bootstrap_manifest_v1()) {
        Ok(wire) => client.send_fallible(ServerGeneral::BootstrapManifest(wire)),
        Err(e) => warn!(?e, "bootstrap manifest encode failure"),
    }
    let game_sync = ServerInit::GameSync {
        server_boot_id: *read_data.server_boot_id,
        entity_package: read_data.trackers.create_entity_package_with_uid(admission.entity, admission.uid, None, None, None),
        role: admission.admin_role,
        time_of_day: *read_data.time_of_day,
        max_group_size: read_data.settings.max_player_group_size,
        client_timeout: read_data.settings.client_timeout,
        world_map: (*read_data.map).clone(),
        recipe_book: (*read_data.recipe_book).clone(),
        component_recipe_book: default_component_recipe_book().cloned(),
        material_stats: (*read_data.material_stats).clone(),
        ability_map: (*read_data.ability_map).clone(),
        server_constants: ServerConstants { day_cycle_coefficient: read_data.settings.day_cycle_coefficient() },
        description,
        active_plugins,
        // APEX-T2.5.11: Some only when a strict deployment compiled at
        // startup (policy-file opt-in); Legacy state = None = old path.
        #[cfg(feature = "plugins")]
        plugin_deployment: read_data.plugin_deployment.summary(),
        #[cfg(not(feature = "plugins"))]
        plugin_deployment: None,
        session_binding,
    };
    // APEX-T3.3.16: V1-envelope GameSync iff this attachment is V1
    // (the `reset_semantic_state` call above just made that true or
    // stayed `None`) -- `Legacy` keeps the exact original raw send.
    let sent_v1 = try_send_gamesync_v1(client, &game_sync);
    if !sent_v1 && client.send(game_sync).is_err() {
        return;
    }
    debug!("Done initial sync with client.");

    // Send initial player list.
    // DET-NET-015: build Init through the canonical helper so the wire
    // bytes are Uid-sorted and the client initializes deterministically.
    let _ = client.send(ServerGeneral::PlayerListUpdate(PlayerListUpdate::init_canonical(
        player_list.iter().map(|(uid, info)| (*uid, info.clone())),
    )));

    new_players.insert(admission.principal, (admission.entity, admission.player, admission.admin_role, admission.player_list_update_msg));
}

/// `APEX-T3.3.16` tests for `try_send_gamesync_v1`'s own wiring
/// (allocate sequence -> build frame -> send). Uses `ServerGeneral::
/// UpdateRecipes` as a lightweight stand-in payload (the function is
/// genericized exactly so this is possible -- `ServerInit::GameSync`
/// has no lightweight constructor in this crate, matching the packet's
/// own "no post-auth V1 payload accepted before correctly bound
/// GameSync" gate being about the SEQUENCE/BINDING plumbing, not about
/// this one payload type's own field values, which `build_semantic_
/// frame_v1`'s own T3.3.15 integration test already proves end-to-end
/// for real). Packet's own test list ("First sequence 1, duplicate,
/// old epoch, raw V1, registration success plus send failure"): "old
/// epoch"/"raw V1" are RECEIVE-side (client) concerns, closed by
/// genericizing `Client::validate_semantic_frame_v1` in `client/src/
/// lib.rs` instead (inherits its existing `StaleEpoch`/decode-failure
/// coverage for any payload type, including `ServerInit`).
#[cfg(test)]
mod gamesync_v1_tests {
    use common::apex::identity::{FixedRandomBytesSourceV1, ServerBootId};
    use common_net::msg::{ClientType, ServerGeneral, envelope::ActiveSessionBindingV1};
    use network::{ConnectAddr, ListenAddr, Network, Participant, Pid, Promises, Stream};
    use tokio::runtime::Runtime;

    use super::*;

    fn boot_id() -> ServerBootId { ServerBootId::generate(&mut FixedRandomBytesSourceV1([21; 16])).unwrap() }

    fn binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: boot_id(),
            session_id: common::apex::identity::SessionId::generate(&mut FixedRandomBytesSourceV1([22; 16])).unwrap(),
            epoch: common::apex::identity::ConnectionEpoch::new(1).unwrap(),
        }
    }

    /// Same shape as `sys/semantic_egress.rs`'s own `LiveClientHarness`
    /// (a real `network::Participant` over an in-process `Mpsc`
    /// transport, the exact 6 streams `connection_handler.rs` opens) --
    /// duplicated rather than shared across the two files' `#[cfg(test)]`
    /// modules, since this crate has no established shared-test-support
    /// location yet and the two harnesses' needs already diverge slightly
    /// (this one only ever reads back the `general`/`in_game` peer
    /// stream, since the stand-in payload routes to `InGame`).
    struct LiveClientHarness {
        _server_net: Network,
        _peer_net: Network,
        _peer_participant: Participant,
        _peer_general: Stream,
        _peer_ping: Stream,
        _peer_register: Stream,
        _peer_character_screen: Stream,
        peer_in_game: Stream,
        _peer_terrain: Stream,
        client: Option<Client>,
    }

    fn build_live_client_harness(mpsc_port: u64, runtime: &Runtime) -> LiveClientHarness {
        let reliable = Promises::ORDERED | Promises::CONSISTENCY;
        let reliablec = reliable | Promises::COMPRESSED;

        runtime.block_on(async {
            let mut server_net = Network::new(Pid::fake(0), runtime);
            let peer_net = Network::new(Pid::fake(1), runtime);
            server_net.listen(ListenAddr::Mpsc(mpsc_port)).await.unwrap();
            let mut peer_participant = peer_net.connect(ConnectAddr::Mpsc(mpsc_port)).await.unwrap();
            let server_participant = server_net.connected().await.unwrap();

            let general_stream = server_participant.open(3, reliablec, 500).await.unwrap();
            let ping_stream = server_participant.open(2, reliable, 500).await.unwrap();
            let register_stream = server_participant.open(3, reliablec, 500).await.unwrap();
            let character_screen_stream = server_participant.open(3, reliablec, 500).await.unwrap();
            let in_game_stream = server_participant.open(3, reliablec, 100_000).await.unwrap();
            let terrain_stream = server_participant.open(4, reliable, 20_000).await.unwrap();

            let peer_general = peer_participant.opened().await.unwrap();
            let peer_ping = peer_participant.opened().await.unwrap();
            let peer_register = peer_participant.opened().await.unwrap();
            let peer_character_screen = peer_participant.opened().await.unwrap();
            let peer_in_game = peer_participant.opened().await.unwrap();
            let peer_terrain = peer_participant.opened().await.unwrap();

            let client = Client::new(
                ClientType::Game,
                server_participant,
                ConnectAddr::Mpsc(mpsc_port),
                0.0,
                None,
                general_stream,
                ping_stream,
                register_stream,
                character_screen_stream,
                in_game_stream,
                terrain_stream,
            );

            LiveClientHarness {
                _server_net: server_net,
                _peer_net: peer_net,
                _peer_participant: peer_participant,
                _peer_general: peer_general,
                _peer_ping: peer_ping,
                _peer_register: peer_register,
                _peer_character_screen: peer_character_screen,
                peer_in_game,
                _peer_terrain: peer_terrain,
                client: Some(client),
            }
        })
    }

    #[test]
    fn v1_session_sends_as_sequence_one_and_decodes() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let mut harness = build_live_client_harness(45_201, &runtime);
        let mut client = harness.client.take().unwrap();
        client.reset_semantic_state(binding());

        assert!(try_send_gamesync_v1(&mut client, &ServerGeneral::UpdateRecipes));

        let frame_bytes: Vec<u8> = runtime.block_on(harness.peer_in_game.recv::<Vec<u8>>()).unwrap();
        let limits = common::apex::manifest::ManifestDecodeLimitsV1 {
            max_input_bytes: 1 << 20,
            max_depth: 8,
            max_nodes: 64,
            max_array_items: 16,
            max_map_entries: 16,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 1 << 20,
        };
        let frame: common_net::msg::envelope::SemanticWireFrameV1 =
            common::apex::manifest::decode_manifest_v1(&frame_bytes, &limits).unwrap();
        assert_eq!(frame.header.sequence.get(), 1);
        assert_eq!(frame.header.session_id, binding().session_id);
    }

    #[test]
    fn second_send_advances_to_sequence_two_not_a_duplicate() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let mut harness = build_live_client_harness(45_202, &runtime);
        let mut client = harness.client.take().unwrap();
        client.reset_semantic_state(binding());

        assert!(try_send_gamesync_v1(&mut client, &ServerGeneral::UpdateRecipes));
        assert!(try_send_gamesync_v1(&mut client, &ServerGeneral::UpdateRecipes));

        let limits = common::apex::manifest::ManifestDecodeLimitsV1 {
            max_input_bytes: 1 << 20,
            max_depth: 8,
            max_nodes: 64,
            max_array_items: 16,
            max_map_entries: 16,
            max_machine_text_bytes: 256,
            max_byte_string_bytes: 1 << 20,
        };
        let first: Vec<u8> = runtime.block_on(harness.peer_in_game.recv::<Vec<u8>>()).unwrap();
        let second: Vec<u8> = runtime.block_on(harness.peer_in_game.recv::<Vec<u8>>()).unwrap();
        let first: common_net::msg::envelope::SemanticWireFrameV1 = common::apex::manifest::decode_manifest_v1(&first, &limits).unwrap();
        let second: common_net::msg::envelope::SemanticWireFrameV1 = common::apex::manifest::decode_manifest_v1(&second, &limits).unwrap();
        assert_eq!(first.header.sequence.get(), 1);
        assert_eq!(second.header.sequence.get(), 2, "sequence must advance, never repeat/duplicate");
    }

    #[test]
    fn legacy_session_never_sends_v1() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let mut harness = build_live_client_harness(45_203, &runtime);
        let mut client = harness.client.take().unwrap();
        // No `reset_semantic_state` call -- `Legacy`, matching what
        // `T3.3.05`'s negotiation always resolves today.
        assert!(!try_send_gamesync_v1(&mut client, &ServerGeneral::UpdateRecipes));
    }
}
