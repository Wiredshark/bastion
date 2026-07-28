/// `ClientGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_CLIENTGENERAL_V1: [&str; 16] = [
    "CreateCharacter", "EditCharacter", "ControllerInputs", "ControlEvent", "ControlAction",
    "PlaceBlock", "UnlockSkill", "UpdateMapMarker", "BastionPlaceDesignation",
    "BastionApplyInfluence", "BastionContextAction", "BastionCancelDesignation",
    "BastionInspect", "CheckpointCommitAck", "ChatMsg", "RequestPluginArtifacts",
];

/// `ServerGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_SERVERGENERAL_V1: [&str; 26] = [
    "GroupUpdate", "Invite", "GroupInventoryUpdate", "InviteComplete", "Dialogue",
    "TerrainChunkUpdate", "LodZoneUpdate", "TerrainBlockUpdates", "PlayerListUpdate",
    "ChatMsg", "ChatMode", "TimeOfDay", "CreateEntity", "Disconnect", "CheckpointBegin",
    "CheckpointBarrier", "CommandResult", "Notification", "UpdatePendingTrade",
    "FinishedTrade", "SiteEconomy", "MapMarker", "PluginArtifactData", "BastionDesignation",
    "BastionDesignationRemoved", "BastionInspectInfo",
];
