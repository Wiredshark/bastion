/// `ClientGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_CLIENTGENERAL_V1: [&str; 17] = [
    "CreateCharacter", "EditCharacter", "ControllerInputs", "ControlEvent", "ControlAction",
    "PlaceBlock", "UnlockSkill", "UpdateMapMarker", "BastionPlaceDesignation",
    "BastionApplyInfluence", "BastionContextAction", "BastionCancelDesignation",
    "BastionInspect", "CheckpointCommitAck", "ChatMsg", "RequestPlugins",
    "RequestPluginArtifacts",
];

/// `ServerGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_SERVERGENERAL_V1: [&str; 31] = [
    "CharacterDataLoadResult", "CharacterListUpdate", "GroupUpdate", "Invite",
    "GroupInventoryUpdate", "InviteComplete", "Dialogue", "Outcomes", "TerrainChunkUpdate",
    "LodZoneUpdate", "TerrainBlockUpdates", "PlayerListUpdate", "ChatMsg", "ChatMode",
    "TimeOfDay", "CreateEntity", "Disconnect", "CheckpointBegin", "CheckpointBarrier",
    "CommandResult", "Notification", "UpdatePendingTrade", "FinishedTrade", "SiteEconomy",
    "MapMarker", "PluginData", "PluginArtifactData", "Gizmos", "BastionDesignation",
    "BastionDesignationRemoved", "BastionInspectInfo",
];
