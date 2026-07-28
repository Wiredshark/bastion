/// `ClientGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_CLIENTGENERAL_V1: [&str; 13] = [
    "CreateCharacter", "EditCharacter", "PlaceBlock", "UnlockSkill",
    "BastionPlaceDesignation",
    "BastionApplyInfluence", "BastionContextAction", "BastionCancelDesignation",
    "BastionInspect", "CheckpointCommitAck", "ChatMsg", "RequestPluginArtifacts",
    "ControllerInputs",
];

/// `ServerGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_SERVERGENERAL_V1: [&str; 21] = [
    "Invite", "GroupInventoryUpdate", "InviteComplete", "Dialogue",
    "TerrainChunkUpdate", "LodZoneUpdate", "TerrainBlockUpdates", "PlayerListUpdate",
    "ChatMsg", "TimeOfDay", "CheckpointBegin",
    "CheckpointBarrier", "CommandResult", "Notification", "UpdatePendingTrade",
    "FinishedTrade", "SiteEconomy", "PluginArtifactData", "BastionDesignation",
    "BastionDesignationRemoved", "BastionInspectInfo",
];
