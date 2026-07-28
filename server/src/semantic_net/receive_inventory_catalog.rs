//! `APEX-T3.3.20`: GENERATED -- do not hand-edit; regenerate the same
//! way this catalog was built (see the `T3.3.20` commit message for the
//! classification rules). Frozen catalog of every receive-shaped call
//! site found by this row's own scan pattern
//! (`\.recv\(|\.try_recv\(|try_recv_all\(` over `server/src`), each
//! classified exactly once.
//!
//! Keyed by (file path relative to `server/src/`, exact trimmed matched
//! line text, 0-based occurrence index within that file) -- same
//! convention `send_inventory_catalog.rs` uses, for the same reasons.

use super::ReceiveSiteClassV1::{self, LegacyMechanism, NotAClientReceive, Ping, PreAuth};

pub(super) const RECEIVE_SITE_CATALOG: [(&str, &str, u32, ReceiveSiteClassV1); 24] = [
    ("chat.rs", "while let Some(msg) = self.chat_r.recv().await {", 0, NotAClientReceive),
    ("chunk_generator.rs", "while let Ok((key, res)) = self.chunk_rx.try_recv() {", 0, NotAClientReceive),
    ("chunk_generator.rs", "while let Ok((key, res)) = self.chunk_rx.try_recv() {", 1, NotAClientReceive),
    ("chunk_generator.rs", "while let Ok((key, res)) = self.chunk_rx.try_recv() {", 2, NotAClientReceive),
    ("client.rs", "0 => self.register_stream.try_recv(),", 0, LegacyMechanism),
    ("client.rs", "1 => self.character_screen_stream.try_recv(),", 0, LegacyMechanism),
    ("client.rs", "2 => self.in_game_stream.try_recv(),", 0, LegacyMechanism),
    ("client.rs", "3 => self.general_stream.try_recv(),", 0, LegacyMechanism),
    ("client.rs", "4 => self.ping_stream.try_recv(),", 0, LegacyMechanism),
    ("client.rs", "5 => self.terrain_stream.try_recv(),", 0, LegacyMechanism),
    ("connection_handler.rs", "let server_data = receiver.recv()?;", 0, NotAClientReceive),
    ("lib.rs", "while let Ok(sender) = self.connection_handler.info_requester_receiver.try_recv() {", 0, NotAClientReceive),
    ("lib.rs", "while let Ok(incoming) = self.connection_handler.client_receiver.try_recv() {", 0, NotAClientReceive),
    ("login_provider.rs", "match pending.pending_r.try_recv() {", 0, NotAClientReceive),
    ("persistence/character_updater.rs", "while let Ok(action) = update_rx.recv() {", 0, NotAClientReceive),
    ("rtsim/mod.rs", "while let Ok(data) = rx.recv() {", 0, NotAClientReceive),
    ("sys/msg/mod.rs", "let msg = match client.recv(stream_id) {", 0, LegacyMechanism),
    ("sys/msg/mod.rs", "let raw: Vec<u8> = match client.recv(stream_id) {", 0, LegacyMechanism),
    ("sys/msg/mod.rs", "try_recv_all(client, stream_id, handler)", 0, LegacyMechanism),
    ("sys/msg/ping.rs", "let res = super::try_recv_all(client, 4, Self::handle_ping_msg);", 0, Ping),
    ("sys/msg/register.rs", "let _ = super::try_recv_all(client, 0, |_, msg: ClientRegister| {", 0, PreAuth),
    ("weather/tick.rs", "&& let Ok((new_grid, new_lightning_cells, sim)) = weather_job.weather_rx.try_recv()", 0, NotAClientReceive),
    ("weather/tick.rs", "rx.try_recv(),", 0, NotAClientReceive),
    ("weather/tick.rs", "rx.try_recv(),", 1, NotAClientReceive),
];
