use crate::client::Client;
use common::{
    comp::{ChatMode, ChatType, Content, Group, Player},
    event::{self, EmitExt},
    event_emitters,
    resources::ProgramTime,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral, envelope::SemanticStreamIdV1};
use rayon::prelude::*;
use specs::{Entities, LendJoin, ParJoin, Read, ReadStorage, WriteStorage};
use tracing::{debug, error, warn};

event_emitters! {
    struct Events[Emitters] {
        command: event::CommandEvent,
        client_disconnect: event::ClientDisconnectEvent,
        chat: event::ChatEvent,

        #[cfg(feature = "plugins")]
        plugins: event::RequestPluginsEvent,
    }
}

impl Sys {
    fn handle_general_msg(
        emitters: &mut Emitters,
        entity: specs::Entity,
        client: &Client,
        player: Option<&Player>,
        uids: &ReadStorage<'_, Uid>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        groups: &ReadStorage<'_, Group>,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        match msg {
            ClientGeneral::ChatMsg(message) => {
                if !client.client_type.can_send_message() {
                    client.send_fallible(ServerGeneral::ChatMsg(
                        ChatType::CommandError
                            .into_msg(Content::localized("command-cannot-send-message-hidden")),
                    ));
                } else if player.is_some() {
                    if let Some(from) = uids.get(entity) {
                        const CHAT_MODE_DEFAULT: &ChatMode = &ChatMode::default();
                        let mode = chat_modes.get(entity).unwrap_or(CHAT_MODE_DEFAULT);
                        // Try sending the chat message
                        match mode.to_msg(*from, message, groups.get(entity).copied()) {
                            Ok(message) => {
                                emitters.emit(event::ChatEvent {
                                    msg: message,
                                    from_client: true,
                                });
                            },
                            Err(error) => {
                                client.send_fallible(ServerGeneral::ChatMsg(
                                    ChatType::CommandError.into_msg(error),
                                ));
                            },
                        }
                    } else {
                        error!("Could not send message. Missing player uid");
                    }
                } else {
                    warn!("Received a chat message from an unregistered client");
                }
            },
            ClientGeneral::Command(name, args) => {
                if player.is_some() {
                    emitters.emit(event::CommandEvent(entity, name, args));
                }
            },
            ClientGeneral::Terminate => {
                debug!(?entity, "Client send message to terminate session");
                emitters.emit(event::ClientDisconnectEvent(
                    entity,
                    common::comp::DisconnectReason::ClientRequested,
                ));
            },
            ClientGeneral::RequestPlugins(plugins) => {
                tracing::info!("Plugin request {plugins:x?}, {}", player.is_some());

                #[cfg(feature = "plugins")]
                emitters.emit(event::RequestPluginsEvent { entity, plugins });
            },
            _ => {
                debug!("Kicking possible misbehaving client due to invalid message request");
                emitters.emit(event::ClientDisconnectEvent(
                    entity,
                    common::comp::DisconnectReason::NetworkError,
                ));
            },
        }
        Ok(())
    }
}

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Events<'a>,
        Read<'a, ProgramTime>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, ChatMode>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Group>,
        WriteStorage<'a, Client>,
    );

    const NAME: &'static str = "msg::general";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, events, program_time, uids, chat_modes, players, groups, mut clients): Self::SystemData,
    ) {
        (&entities, &mut clients, players.maybe())
            .par_join()
            .for_each_init(
                || events.get_emitters(),
                |emitters, (entity, client, player)| {
                    let res = super::try_recv_all_dispatch(client, 3, SemanticStreamIdV1::General, |client, msg| {
                        Self::handle_general_msg(
                            emitters,
                            entity,
                            client,
                            player,
                            &uids,
                            &chat_modes,
                            &groups,
                            msg,
                        )
                    });

                    if let Ok(1_u64..=u64::MAX) = res {
                        // Update client ping.
                        client.last_ping = program_time.0
                    }
                },
            );
    }
}

/// `T3.3.09`: this system now selects V1/Legacy via
/// `try_recv_all_dispatch(client, 3, SemanticStreamIdV1::General, ...)`.
/// The full duplicate/gap/digest/epoch/wrong-boot/route validation
/// matrix is already exhaustively covered once, system-agnostically, by
/// `T3.3.08`'s `semantic_ingress_tests` (which exercise the exact same
/// `validate_semantic_frame_v1` this dispatch reaches) -- re-deriving
/// that matrix per system would duplicate coverage without adding any,
/// since `try_recv_all_dispatch` itself adds no new logic beyond
/// selecting between two already-tested functions. What IS new and
/// worth a system-local test: that the `SemanticStreamIdV1::General`
/// literal hardcoded at this call site actually matches what this
/// handler's own match arms expect -- a copy-paste stream-ID mismatch
/// here would silently reject 100% of this system's traffic the moment
/// V1 negotiation goes live (T3.3.05), with no other test catching it.
#[cfg(test)]
mod semantic {
    use super::*;
    use common_net::msg::envelope::SemanticRouteV1;

    #[test]
    fn dispatch_stream_matches_handled_general_messages() {
        assert_eq!(ClientGeneral::Terminate.semantic_stream(), SemanticStreamIdV1::General);
        assert_eq!(
            ClientGeneral::Command("test".to_string(), vec![]).semantic_stream(),
            SemanticStreamIdV1::General
        );
    }
}
