use common_net::msg::{ActiveSessionBindingV1, ClientType, SemanticReceiveStateV1, SemanticSendStateV1, ServerGeneral, ServerMsg};
use network::{ConnectAddr, Message, Participant, Stream, StreamError, StreamParams};
use serde::{Serialize, de::DeserializeOwned};
use specs::Component;
use std::{net::SocketAddr, sync::atomic::AtomicBool};

/// Client handles ALL network related information of everything that connects
/// to the server Client DOES NOT handle game states
/// Client DOES NOT handle network information that is only relevant to some
/// "things" connecting to the server (there is currently no such case). First a
/// Client connects to the game, when it registers, it gets the `Player`
/// component, when it enters the game it gets the `InGame` component.
pub struct Client {
    pub client_type: ClientType,
    pub participant: Option<Participant>,
    pub current_ip_addrs: Vec<SocketAddr>,
    connected_from_addr: ConnectAddr,
    pub last_ping: f64,
    pub login_msg_sent: AtomicBool,
    pub locale: Option<String>,

    //TODO: Consider splitting each of these out into their own components so all the message
    //processing systems can run in parallel with each other (though it may turn out not to
    //matter that much).
    general_stream: Stream,
    ping_stream: Stream,
    register_stream: Stream,
    character_screen_stream: Stream,
    in_game_stream: Stream,
    terrain_stream: Stream,

    general_stream_params: StreamParams,
    ping_stream_params: StreamParams,
    register_stream_params: StreamParams,
    character_screen_stream_params: StreamParams,
    in_game_stream_params: StreamParams,
    terrain_stream_params: StreamParams,

    /// `APEX-T3.3.06`: `Some` only while a `NetEnvelopeV1` attachment is
    /// active -- `None` for `Legacy` sessions (packet: "Legacy carries no
    /// V1 state") and while detached (packet: "detach disables semantic
    /// access"). Server-to-client direction (this server sending
    /// `ServerGeneral`/`ServerInit` to this client). Dormant: nothing
    /// reads or advances this yet (`T3.3.11`+ owns that).
    semantic_send_state: Option<SemanticSendStateV1>,
    /// Client-to-server direction (this client sending `ClientGeneral` to
    /// this server). Same lifecycle as `semantic_send_state` above.
    /// Dormant: nothing reads or advances this yet (`T3.3.08`+ owns that).
    semantic_receive_state: Option<SemanticReceiveStateV1>,
}

pub struct PreparedMsg {
    stream_id: u8,
    message: Message,
}

impl Component for Client {
    type Storage = specs::DenseVecStorage<Self>;
}

impl Client {
    pub(crate) fn new(
        client_type: ClientType,
        participant: Participant,
        connected_from: ConnectAddr,
        last_ping: f64,
        locale: Option<String>,
        general_stream: Stream,
        ping_stream: Stream,
        register_stream: Stream,
        character_screen_stream: Stream,
        in_game_stream: Stream,
        terrain_stream: Stream,
    ) -> Self {
        let general_stream_params = general_stream.params();
        let ping_stream_params = ping_stream.params();
        let register_stream_params = register_stream.params();
        let character_screen_stream_params = character_screen_stream.params();
        let in_game_stream_params = in_game_stream.params();
        let terrain_stream_params = terrain_stream.params();
        Client {
            client_type,
            participant: Some(participant),
            current_ip_addrs: connected_from.socket_addr().into_iter().collect(),
            connected_from_addr: connected_from,
            last_ping,
            locale,
            login_msg_sent: AtomicBool::new(false),
            general_stream,
            ping_stream,
            register_stream,
            character_screen_stream,
            in_game_stream,
            terrain_stream,
            general_stream_params,
            ping_stream_params,
            register_stream_params,
            character_screen_stream_params,
            in_game_stream_params,
            terrain_stream_params,
            semantic_send_state: None,
            semantic_receive_state: None,
        }
    }

    pub(crate) fn connected_from_addr(&self) -> &ConnectAddr { &self.connected_from_addr }

    /// `APEX-T3.3.06`: called on a freshly-accepted `NetEnvelopeV1`
    /// binding, and again whenever the epoch advances ("higher epoch
    /// replaces") -- always a full reset via the reset constructors,
    /// never a partial update. Never called for a `Legacy` selection
    /// (caller's job to gate on `SessionBindingV1.selected_semantic_protocol`
    /// -- this method itself doesn't know about negotiation). "Detach
    /// disables semantic access" (the packet's third case) needs no
    /// separate clear here: `events/player.rs::handle_client_disconnect`
    /// already removes the whole `Client` component (owning this state)
    /// from ECS storage on every disconnect reason, detach included --
    /// confirmed by reading that function, not assumed. A resumed
    /// connection gets a brand-new `Client` component (`semantic_*_state:
    /// None` from `Client::new`) and this method resets it fresh via
    /// `finalize_admission`, matching "per-attachment" (this row's own
    /// title): cursor state is scoped to one epoch's live attachment,
    /// never carried across a detach/reattach.
    pub(crate) fn reset_semantic_state(&mut self, binding: ActiveSessionBindingV1) {
        self.semantic_send_state = Some(SemanticSendStateV1::new(binding));
        self.semantic_receive_state = Some(SemanticReceiveStateV1::new(binding));
    }

    /// `APEX-T3.3.08`: the ingress validation pipeline's own read/commit
    /// access to receive-side cursor state -- `None` for `Legacy`
    /// sessions and while detached, same lifecycle as `reset_semantic_state`.
    pub(crate) fn semantic_receive_state(&self) -> Option<&SemanticReceiveStateV1> { self.semantic_receive_state.as_ref() }

    pub(crate) fn semantic_receive_state_mut(&mut self) -> Option<&mut SemanticReceiveStateV1> {
        self.semantic_receive_state.as_mut()
    }

    /// `APEX-T3.3.13`: a semantic-send producer's read-only access to the
    /// current intended binding -- `None` for `Legacy` sessions and while
    /// detached, same lifecycle as `reset_semantic_state`. Producers use
    /// this only for `SemanticSendIntentV1::recipient`; sequence
    /// allocation stays `SemanticSendStateV1`'s own cursor fields,
    /// touched only by `T3.3.15`'s egress owner, never by a producer.
    pub(crate) fn semantic_send_state(&self) -> Option<&SemanticSendStateV1> { self.semantic_send_state.as_ref() }

    /// `APEX-T3.3.15`: the egress owner's own mutable access to the
    /// send-side cursor -- the ONLY place `SemanticSendStateV1::
    /// allocate_sequence` is ever called from, matching the packet's
    /// own "allocate checked sequence" step (5 of the 9-step egress
    /// algorithm). No producer touches this; producers only ever read
    /// the binding via `semantic_send_state()` above.
    pub(crate) fn semantic_send_state_mut(&mut self) -> Option<&mut SemanticSendStateV1> {
        self.semantic_send_state.as_mut()
    }

    /// `APEX-T3.3.15`: sends one already-encoded `SemanticWireFrameV1`'s
    /// bytes on the physical stream `semantic_stream` maps to -- the
    /// server-side mirror of `client/src/lib.rs::send_semantic_v1`'s own
    /// physical-stream match, which the client already established this
    /// exact routing table for (T3.3.07). `Bootstrap` routes to the
    /// register stream for structural completeness matching that
    /// mirror, though nothing enqueues a `Bootstrap`-classified intent
    /// yet -- `GameSync`'s own semantic envelope is `T3.3.16`'s job, not
    /// this row's.
    pub(crate) fn send_semantic_frame(
        &self,
        semantic_stream: common_net::msg::envelope::SemanticStreamIdV1,
        frame_bytes: Vec<u8>,
    ) -> Result<(), StreamError> {
        use common_net::msg::envelope::SemanticStreamIdV1;
        match semantic_stream {
            SemanticStreamIdV1::Bootstrap => self.register_stream.send(frame_bytes),
            SemanticStreamIdV1::CharacterScreen => self.character_screen_stream.send(frame_bytes),
            SemanticStreamIdV1::InGame => self.in_game_stream.send(frame_bytes),
            SemanticStreamIdV1::General => self.general_stream.send(frame_bytes),
            SemanticStreamIdV1::Terrain => self.terrain_stream.send(frame_bytes),
        }
    }

    pub(crate) fn send<M: Into<ServerMsg>>(&self, msg: M) -> Result<(), StreamError> {
        // TODO: hack to avoid locking stream mutex while serializing the message,
        // remove this when the mutexes on the Streams are removed
        let prepared = self.prepare(msg);
        self.send_prepared(&prepared)
        /*match msg.into() {
            ServerMsg::Info(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::Init(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::RegisterAnswer(m) => self.register_stream.lock().unwrap().send(m),
            ServerMsg::General(g) => {
                match g {
                    //Character Screen related
                    ServerGeneral::CharacterDataLoadResult(_)
                    | ServerGeneral::CharacterListUpdate(_)
                    | ServerGeneral::CharacterActionError(_)
                    | ServerGeneral::CharacterCreated(_)
                    | ServerGeneral::CharacterEdited(_)
                    | ServerGeneral::CharacterSuccess => {
                        self.character_screen_stream.lock().unwrap().send(g)
                    },
                    //In-game related
                    ServerGeneral::GroupUpdate(_)
                    | ServerGeneral::Invite { .. }
                    | ServerGeneral::InvitePending(_)
                    | ServerGeneral::InviteComplete { .. }
                    | ServerGeneral::ExitInGameSuccess
                    | ServerGeneral::InventoryUpdate(_, _)
                    | ServerGeneral::SetViewDistance(_)
                    | ServerGeneral::SiteEconomy(_)
                    | ServerGeneral::Outcomes(_)
                    | ServerGeneral::Knockback(_)
                    | ServerGeneral::UpdatePendingTrade(_, _, _)
                    | ServerGeneral::FinishedTrade(_)
                    | ServerGeneral::WeatherUpdate(_) => {
                        self.in_game_stream.lock().unwrap().send(g)
                    },
                    //Ingame related, terrain
                    ServerGeneral::TerrainChunkUpdate { .. }
                    | ServerGeneral::LodZoneUpdate { .. }
                    | ServerGeneral::TerrainBlockUpdates(_) => {
                        self.terrain_stream.lock().unwrap().send(g)
                    },
                    // Always possible
                    ServerGeneral::PlayerListUpdate(_)
                    | ServerGeneral::ChatMsg(_)
                    | ServerGeneral::ChatMode(_)
                    | ServerGeneral::SetPlayerEntity(_)
                    | ServerGeneral::TimeOfDay(_, _)
                    | ServerGeneral::EntitySync(_)
                    | ServerGeneral::CompSync(_)
                    | ServerGeneral::CreateEntity(_)
                    | ServerGeneral::DeleteEntity(_)
                    | ServerGeneral::Disconnect(_)
                    | ServerGeneral::Notification(_) => self.general_stream.lock().unwrap().send(g),
                }
            },
            ServerMsg::Ping(m) => self.ping_stream.lock().unwrap().send(m),
        }*/
    }

    /// Like `send` but any errors are explicitly ignored.
    pub(crate) fn send_fallible<M: Into<ServerMsg>>(&self, msg: M) { let _ = self.send(msg); }

    pub(crate) fn send_prepared(&self, msg: &PreparedMsg) -> Result<(), StreamError> {
        match msg.stream_id {
            0 => self.register_stream.send_raw(&msg.message),
            1 => self.character_screen_stream.send_raw(&msg.message),
            2 => self.in_game_stream.send_raw(&msg.message),
            3 => self.general_stream.send_raw(&msg.message),
            4 => self.ping_stream.send_raw(&msg.message),
            5 => self.terrain_stream.send_raw(&msg.message),
            _ => unreachable!("invalid stream id"),
        }
    }

    pub(crate) fn prepare<M: Into<ServerMsg>>(&self, msg: M) -> PreparedMsg {
        match msg.into() {
            ServerMsg::Info(m) => PreparedMsg::new(0, &m, &self.register_stream_params),
            ServerMsg::Init(m) => PreparedMsg::new(0, &m, &self.register_stream_params),
            ServerMsg::RegisterAnswer(m) => PreparedMsg::new(0, &m, &self.register_stream_params),
            ServerMsg::General(g) => {
                match g {
                    // Character Screen related
                    ServerGeneral::CharacterDataLoadResult(_)
                    | ServerGeneral::CharacterListUpdate(_)
                    | ServerGeneral::CharacterActionError(_)
                    | ServerGeneral::CharacterCreated(_)
                    | ServerGeneral::CharacterEdited(_)
                    | ServerGeneral::CharacterSuccess
                    | ServerGeneral::SpectatorSuccess(_) => {
                        PreparedMsg::new(1, &g, &self.character_screen_stream_params)
                    },
                    // In-game related
                    ServerGeneral::GroupUpdate(_)
                    | ServerGeneral::Invite { .. }
                    | ServerGeneral::InvitePending(_)
                    | ServerGeneral::InviteComplete { .. }
                    | ServerGeneral::ExitInGameSuccess
                    | ServerGeneral::InventoryUpdate(_, _)
                    | ServerGeneral::GroupInventoryUpdate(_, _)
                    | ServerGeneral::Dialogue(_, _)
                    | ServerGeneral::SetViewDistance(_)
                    | ServerGeneral::Outcomes(_)
                    | ServerGeneral::Knockback(_)
                    | ServerGeneral::SiteEconomy(_)
                    | ServerGeneral::UpdatePendingTrade(_, _, _)
                    | ServerGeneral::FinishedTrade(_)
                    | ServerGeneral::MapMarker(_)
                    | ServerGeneral::WeatherUpdate(_)
                    | ServerGeneral::LocalWindUpdate(_)
                    | ServerGeneral::SpectatePosition(_)
                    | ServerGeneral::UpdateRecipes
                    | ServerGeneral::Gizmos(_)
                    | ServerGeneral::BastionDesignation { .. }
                    | ServerGeneral::BastionDesignationRemoved { .. }
                    | ServerGeneral::BastionInspectInfo { .. } => {
                        PreparedMsg::new(2, &g, &self.in_game_stream_params)
                    },
                    // Terrain
                    ServerGeneral::TerrainChunkUpdate { .. }
                    | ServerGeneral::LodZoneUpdate { .. }
                    | ServerGeneral::TerrainBlockUpdates(_) => {
                        PreparedMsg::new(5, &g, &self.terrain_stream_params)
                    },
                    // Always possible
                    ServerGeneral::PlayerListUpdate(_)
                    | ServerGeneral::ChatMsg(_)
                    | ServerGeneral::ChatMode(_)
                    | ServerGeneral::SetPlayerEntity(_)
                    | ServerGeneral::TimeOfDay(_, _, _, _)
                    | ServerGeneral::EntitySync(_)
                    | ServerGeneral::CompSync(_, _)
                    | ServerGeneral::CreateEntity(_)
                    | ServerGeneral::DeleteEntity(_)
                    | ServerGeneral::Disconnect(_)
                    | ServerGeneral::Notification(_)
                    | ServerGeneral::SetPlayerRole(_)
                    | ServerGeneral::PluginData(_)
                    | ServerGeneral::PluginArtifactData(_) => {
                        PreparedMsg::new(3, &g, &self.general_stream_params)
                    },
                }
            },
            ServerMsg::Ping(m) => PreparedMsg::new(4, &m, &self.ping_stream_params),
        }
    }

    pub(crate) fn terrain_params(&self) -> StreamParams { self.terrain_stream_params.clone() }

    /// Only used for Serialize Chunks in a SlowJob.
    /// TODO: find a more elegant version for this invariant
    pub(crate) fn prepare_chunk_update_msg(
        terrain_chunk_update: ServerGeneral,
        params: &StreamParams,
    ) -> PreparedMsg {
        if !matches!(
            terrain_chunk_update,
            ServerGeneral::TerrainChunkUpdate { .. }
        ) {
            unreachable!("You must not call this function without a terrain chunk update!")
        }
        PreparedMsg::new(5, &terrain_chunk_update, params)
    }

    pub(crate) fn recv<M: DeserializeOwned>(
        &mut self,
        stream_id: u8,
    ) -> Result<Option<M>, StreamError> {
        // TODO: are two systems using the same stream?? why is there contention here?
        match stream_id {
            0 => self.register_stream.try_recv(),
            1 => self.character_screen_stream.try_recv(),
            2 => self.in_game_stream.try_recv(),
            3 => self.general_stream.try_recv(),
            4 => self.ping_stream.try_recv(),
            5 => self.terrain_stream.try_recv(),
            _ => unreachable!("invalid stream id"),
        }
    }
}

impl PreparedMsg {
    fn new<M: Serialize + ?Sized>(id: u8, msg: &M, stream_params: &StreamParams) -> PreparedMsg {
        Self {
            stream_id: id,
            message: Message::serialize(&msg, stream_params.clone()),
        }
    }
}
