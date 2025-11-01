use serde::{Deserialize, Serialize};

use super::player::{LavalinkPlayerState, PlayerEvents};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct FrameStats {
    pub sent: u64,
    pub nulled: u32,
    pub deficit: i32,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cpu {
    pub cores: u32,
    pub system_load: f64,
    pub lavalink_load: f64,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Memory {
    pub free: u64,
    pub used: u64,
    pub allocated: u64,
    pub reservable: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ready {
    pub resumed: bool,
    pub session_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerUpdate {
    pub guild_id: String,
    pub state: LavalinkPlayerState,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub players: u32,
    pub playing_players: u32,
    pub uptime: u64,
    pub memory: Memory,
    pub cpu: Cpu,
    pub frame_stats: Option<FrameStats>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op")]
#[serde(rename_all = "camelCase")]
pub enum LavalinkMessage {
    Ready(Ready),
    PlayerUpdate(PlayerUpdate),
    Stats(Stats),
    Event(Box<PlayerEvents>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    resuming: bool,
    timeout: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailingAddresses {
    pub address: String,
    pub failing_timestamp: u64,
    pub failing_time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpBlock {
    #[serde(rename = "type")]
    pub ip_type: String,
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlannerDetails {
    pub ip_block: IpBlock,
    pub failing_addresses: Vec<FailingAddresses>,
    pub rotate_index: String,
    pub ip_index: String,
    pub current_address: String,
    pub block_index: String,
    pub current_address_index: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanner {
    pub class: Option<String>,
    pub details: Option<RoutePlannerDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeVersion {
    pub semver: String,
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeGit {
    pub branch: String,
    pub commit: String,
    pub commit_time: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodePlugin {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LavalinkInfo {
    pub version: NodeVersion,
    pub build_time: u64,
    pub git: NodeGit,
    pub jvm: String,
    pub lavaplayer: String,
    pub source_managers: String,
    pub filters: Vec<String>,
    pub plugins: Vec<NodePlugin>,
}
