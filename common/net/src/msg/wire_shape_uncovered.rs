/// `ClientGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_CLIENTGENERAL_V1: [&str; 10] = [
    "CreateCharacter", "EditCharacter", "UnlockSkill",
    "BastionPlaceDesignation",
    "BastionApplyInfluence", "BastionContextAction",
    "BastionInspect", "CheckpointCommitAck", "ChatMsg", "RequestPluginArtifacts",
];

/// `ServerGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_SERVERGENERAL_V1: [&str; 16] = [
    "GroupInventoryUpdate", "Dialogue",
    "TerrainChunkUpdate", "LodZoneUpdate", "TerrainBlockUpdates", "PlayerListUpdate",
    "ChatMsg", "CheckpointBegin",
    "CheckpointBarrier", "CommandResult", "UpdatePendingTrade",
    "SiteEconomy", "PluginArtifactData", "BastionDesignation",
    "BastionDesignationRemoved", "BastionInspectInfo",
];
