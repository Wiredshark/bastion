/// `ClientGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_CLIENTGENERAL_V1: [&str; 2] = [
    "UnlockSkill", "ChatMsg",
];

/// `ServerGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_SERVERGENERAL_V1: [&str; 9] = [
    "GroupInventoryUpdate", "Dialogue",
    "TerrainChunkUpdate", "PlayerListUpdate",
    "ChatMsg", "CheckpointBegin",
    "UpdatePendingTrade",
    "SiteEconomy", "PluginArtifactData",
];
