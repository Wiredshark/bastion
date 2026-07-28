/// `ClientGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_CLIENTGENERAL_V1: [&str; 36] = [
    "RequestCharacterList", "CreateCharacter", "DeleteCharacter", "EditCharacter", "Character",
    "Spectate", "ControllerInputs", "ControlEvent", "ControlAction", "SetViewDistance",
    "BreakBlock", "PlaceBlock", "ExitInGame", "UnlockSkill", "RequestSiteInfo",
    "UpdateMapMarker", "SetBattleMode", "SpectatePosition", "SpectateEntity",
    "BastionCameraAnchor", "BastionPlaceDesignation", "BastionApplyInfluence",
    "BastionContextAction", "BastionSpawnColony", "BastionCancelDesignation", "BastionInspect",
    "CheckpointCommitAck", "TerrainChunkRequest", "LodZoneRequest", "ChatMsg", "Command",
    "Terminate", "RequestPlayerPhysics", "RequestLossyTerrainCompression", "RequestPlugins",
    "RequestPluginArtifacts",
];

/// `ServerGeneral` variants with no golden yet. WSG-2 burns this to zero.
pub const UNCOVERED_SERVERGENERAL_V1: [&str; 32] = [
    "CharacterDataLoadResult", "CharacterListUpdate", "GroupUpdate", "Invite",
    "GroupInventoryUpdate", "InviteComplete", "InventoryUpdate", "Dialogue", "Outcomes",
    "TerrainChunkUpdate", "LodZoneUpdate", "TerrainBlockUpdates", "PlayerListUpdate",
    "ChatMsg", "ChatMode", "TimeOfDay", "CreateEntity", "Disconnect", "CheckpointBegin",
    "CheckpointBarrier", "CommandResult", "Notification", "UpdatePendingTrade",
    "FinishedTrade", "SiteEconomy", "MapMarker", "PluginData", "PluginArtifactData", "Gizmos",
    "BastionDesignation", "BastionDesignationRemoved", "BastionInspectInfo",
];
