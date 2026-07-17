use crate::{Client, ClientType, ServerInfo};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use futures_util::future::FutureExt;
use network::{Network, Participant, Promises};
use std::time::Duration;
use tokio::{runtime::Runtime, select, sync::oneshot};
use tracing::{debug, error, trace, warn};

pub(crate) struct ServerInfoPacket {
    pub info: ServerInfo,
    pub time: f64,
}

pub(crate) type IncomingClient = Client;

pub(crate) struct ConnectionHandler {
    /// We never actually use this, but if it's dropped before the network has a
    /// chance to exit, it won't block the main thread, and if it is dropped
    /// after the network thread ends, it will drop the network here (rather
    /// than delaying the network thread).  So it emulates the effects of
    /// storing the network in an Arc, without us losing mutability in the
    /// network thread.
    network_receiver: Option<oneshot::Receiver<Network>>,
    thread_handle: Option<tokio::task::JoinHandle<()>>,
    pub client_receiver: Receiver<IncomingClient>,
    pub info_requester_receiver: Receiver<Sender<ServerInfoPacket>>,
    stop_sender: Option<oneshot::Sender<()>>,
}

/// Instead of waiting the main loop we are handling connections, especially
/// their slow network .await part on a different thread. We need to communicate
/// to the Server main thread sometimes though to get the current server_info
/// and time
impl ConnectionHandler {
    pub fn new(network: Network, runtime: &Runtime) -> Self {
        let (stop_sender, stop_receiver) = oneshot::channel();
        let (network_sender, network_receiver) = oneshot::channel();

        let (client_sender, client_receiver) = unbounded::<IncomingClient>();
        let (info_requester_sender, info_requester_receiver) =
            bounded::<Sender<ServerInfoPacket>>(1);

        let thread_handle = Some(runtime.spawn(Self::work(
            network,
            client_sender,
            info_requester_sender,
            stop_receiver,
            network_sender,
        )));

        Self {
            thread_handle,
            client_receiver,
            info_requester_receiver,
            stop_sender: Some(stop_sender),
            network_receiver: Some(network_receiver),
        }
    }

    /// Stop the connection task and return the `Network` to this synchronous
    /// caller so its existing `Drop` shutdown can finish while `runtime` is
    /// still alive.
    ///
    /// The normal `Drop` implementation remains a non-blocking fallback for
    /// production shutdown. Headless acceptance harnesses call this explicit
    /// path before dropping `Server`; otherwise aborting the task can make the
    /// network itself drop from an async task, where shutdown is merely
    /// deferred until the runtime is already being torn down.
    pub(crate) fn shutdown_and_wait(&mut self, runtime: &Runtime) -> Result<(), String> {
        if let Some(stop_sender) = self.stop_sender.take() {
            // A closed receiver means the worker already finished; joining it
            // and recovering the network below is still the correct path.
            let _ = stop_sender.send(());
        }

        if let Some(thread_handle) = self.thread_handle.take() {
            runtime
                .block_on(thread_handle)
                .map_err(|error| format!("connection handler join failed: {error}"))?;
        }

        let receiver = self
            .network_receiver
            .take()
            .ok_or_else(|| "connection handler network was already recovered".to_owned())?;
        let network = runtime
            .block_on(receiver)
            .map_err(|error| format!("connection handler did not return network: {error}"))?;

        // Deliberately outside `runtime.block_on`: Network::drop's existing
        // synchronous wait is the authoritative clean-shutdown path.
        drop(network);
        Ok(())
    }

    async fn work(
        network: Network,
        client_sender: Sender<IncomingClient>,
        info_requester_sender: Sender<Sender<ServerInfoPacket>>,
        stop_receiver: oneshot::Receiver<()>,
        network_sender: oneshot::Sender<Network>,
    ) {
        // Emulate the effects of storing the network in an Arc, without losing
        // mutability.
        let mut network_sender = Some(network_sender);
        let mut network = drop_guard::guard(network, move |network| {
            // If the network receiver was already dropped, we just drop the network here,
            // just like Arc, so we don't care about the result.
            let _ = network_sender
                .take()
                .expect("Only used once in drop")
                .send(network);
        });
        let mut stop_receiver = stop_receiver.fuse();
        loop {
            let participant = match select!(
                _ = &mut stop_receiver => None,
                p = network.connected().fuse() => Some(p),
            ) {
                None => break,
                Some(Ok(p)) => p,
                Some(Err(e)) => {
                    error!(
                        ?e,
                        "Stopping Connection Handler, no new connections can be made to server \
                         now!"
                    );
                    break;
                },
            };

            let client_sender = client_sender.clone();
            let info_requester_sender = info_requester_sender.clone();

            match select!(
                _ = &mut stop_receiver => None,
                e = Self::init_participant(participant, client_sender, info_requester_sender).fuse() => Some(e),
            ) {
                None => break,
                Some(Ok(())) => (),
                Some(Err(e)) => warn!(?e, "drop new participant, because an error occurred"),
            }
        }
    }

    async fn init_participant(
        mut participant: Participant,
        client_sender: Sender<IncomingClient>,
        info_requester_sender: Sender<Sender<ServerInfoPacket>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("New Participant connected to the server");
        let (sender, receiver) = bounded(1);
        info_requester_sender.send(sender)?;

        let reliable = Promises::ORDERED | Promises::CONSISTENCY;
        let reliablec = reliable | Promises::COMPRESSED;

        let general_stream = participant.open(3, reliablec, 500).await?;
        let ping_stream = participant.open(2, reliable, 500).await?;
        let mut register_stream = participant.open(3, reliablec, 500).await?;
        let character_screen_stream = participant.open(3, reliablec, 500).await?;
        let in_game_stream = participant.open(3, reliablec, 100_000).await?;
        let terrain_stream = participant.open(4, reliable, 20_000).await?;

        let server_data = receiver.recv()?;

        register_stream.send(server_data.info)?;

        const TIMEOUT: Duration = Duration::from_secs(5);
        let client_type = match select!(
            _ = tokio::time::sleep(TIMEOUT).fuse() => None,
            t = register_stream.recv::<ClientType>().fuse() => Some(t),
        ) {
            None => {
                debug!("Timeout for incoming client elapsed, aborting connection");
                return Ok(());
            },
            Some(client_type) => client_type?,
        };

        use network::ParticipantEvent;
        let connected_from = match select!(
            _ = tokio::time::sleep(TIMEOUT).fuse() => None,
            connected_from = participant.fetch_event().fuse() => Some(connected_from),
        ) {
            None => {
                error!("Did not receive initial channel created event. This is a bug!");
                return Ok(());
            },
            Some(Err(err)) => {
                debug!("Participant error when trying to receive event: {err:?}");
                return Ok(());
            },
            Some(Ok(ParticipantEvent::ChannelDeleted(_))) => {
                error!(
                    "Received channel deleted event instead of the initial channel created event. \
                     This is a bug!"
                );
                return Ok(());
            },
            Some(Ok(ParticipantEvent::ChannelCreated(connected_from))) => connected_from,
        };

        let client = Client::new(
            client_type,
            participant,
            connected_from,
            server_data.time,
            None,
            general_stream,
            ping_stream,
            register_stream,
            character_screen_stream,
            in_game_stream,
            terrain_stream,
        );

        client_sender.send(client)?;
        Ok(())
    }
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        if let Some(stop_sender) = self.stop_sender.take() {
            let _ = stop_sender.send(());
        }
        if let Some(thread_handle) = self.thread_handle.take() {
            trace!("aborting ConnectionHandler");
            thread_handle.abort();
            trace!("aborted ConnectionHandler!");
        }
    }
}
