/// `ClientGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_CLIENTGENERAL_V1: [&str; 0] = [
];

/// `ServerGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_SERVERGENERAL_V1: [&str; 6] = [
    "Dialogue",
    "TerrainChunkUpdate", "PlayerListUpdate",
    "CheckpointBegin",
    "UpdatePendingTrade",
    "SiteEconomy",
];
