use super::str_to_u64;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Common,
    Suspicious,
    Fault,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadType {
    Track,
    Playlist,
    Search,
    Empty,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "loadType", content = "data")]
pub enum DataType {
    Track(Track),
    Playlist(TrackPlaylist),
    Search(Vec<Track>),
    Error(TrackLoadException),
    Empty(Option<Value>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistInfo {
    pub name: String,
    pub selected_track: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPlaylist {
    pub info: PlaylistInfo,
    pub plugin_info: Value,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackLoadException {
    pub message: String,
    pub severity: Severity,
    pub cause: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LavalinkFilters {
    pub volume: Option<f64>,
    pub equalizer: Option<Vec<Equalizer>>,
    pub karaoke: Option<Karaoke>,
    pub timescale: Option<Timescale>,
    pub tremolo: Option<Tremolo>,
    pub vibrato: Option<Vibrato>,
    pub rotation: Option<Rotation>,
    pub distortion: Option<Distortion>,
    pub channel_mix: Option<ChannelMix>,
    pub low_pass: Option<LowPass>,
    pub plugin_filters: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tremolo {
    pub frequency: Option<f64>,
    pub depth: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vibrato {
    pub frequency: Option<f64>,
    pub depth: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timescale {
    pub speed: Option<f64>,
    pub pitch: Option<f64>,
    pub rate: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rotation {
    pub rotation_hz: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LowPass {
    pub smoothing: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Karaoke {
    pub level: Option<f64>,
    pub mono_level: Option<f64>,
    pub filter_band: Option<f64>,
    pub filter_width: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Equalizer {
    pub band: u16,
    pub gain: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Distortion {
    pub sin_offset: Option<f64>,
    pub sin_scale: Option<f64>,
    pub cos_offset: Option<f64>,
    pub cos_scale: Option<f64>,
    pub tan_offset: Option<f64>,
    pub tan_scale: Option<f64>,
    pub offset: Option<f64>,
    pub scale: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMix {
    pub left_to_left: Option<f64>,
    pub left_to_right: Option<f64>,
    pub right_to_left: Option<f64>,
    pub right_to_right: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LavalinkVoice {
    pub token: String,
    pub endpoint: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LavalinkPlayerState {
    pub time: u64,
    pub position: u32,
    pub connected: bool,
    pub ping: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LavalinkPlayer {
    #[serde(deserialize_with = "str_to_u64")]
    pub guild_id: u64,
    pub track: Option<Track>,
    pub volume: u32,
    pub paused: bool,
    pub state: LavalinkPlayerState,
    pub voice: LavalinkVoice,
    pub filters: LavalinkFilters,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub identifier: String,
    pub is_seekable: bool,
    pub author: String,
    pub length: usize,
    pub is_stream: bool,
    pub position: usize,
    pub title: String,
    pub uri: Option<String>,
    pub artwork_url: Option<String>,
    pub isrc: Option<String>,
    pub source_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub encoded: String,
    pub info: TrackInfo,
    pub plugin_info: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exception {
    #[serde(deserialize_with = "str_to_u64")]
    pub guild_id: u64,
    pub message: Option<String>,
    pub severity: String,
    pub cause: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackStart {
    #[serde(deserialize_with = "str_to_u64")]
    pub guild_id: u64,
    pub track: Track,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackEnd {
    #[serde(deserialize_with = "str_to_u64")]
    pub guild_id: u64,
    pub track: Track,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackException {
    #[serde(deserialize_with = "str_to_u64")]
    pub guild_id: u64,
    pub track: Track,
    pub exception: Exception,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackStuck {
    #[serde(deserialize_with = "str_to_u64")]
    pub guild_id: u64,
    pub track: Track,
    pub threshold_ms: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketClosed {
    #[serde(deserialize_with = "str_to_u64")]
    pub guild_id: u64,
    pub code: usize,
    pub reason: String,
    pub by_remote: bool,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayerEvents {
    TrackStartEvent(TrackStart),
    TrackEndEvent(TrackEnd),
    TrackExceptionEvent(TrackException),
    TrackStuckEvent(TrackStuck),
    WebSocketClosedEvent(WebSocketClosed),
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlayerTrack {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoded: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<Value>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LavalinkPlayerOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<UpdatePlayerTrack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<LavalinkFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<LavalinkVoice>,
}

impl LavalinkFilters {
    pub fn merge(&mut self, other: LavalinkFilters) {
        self.volume = other.volume.or(self.volume);
        self.equalizer = other.equalizer.or(self.equalizer.clone());
        self.karaoke = other.karaoke.or(self.karaoke.clone());
        self.timescale = other.timescale.or(self.timescale.clone());
        self.tremolo = other.tremolo.or(self.tremolo.clone());
        self.vibrato = other.vibrato.or(self.vibrato.clone());
        self.rotation = other.rotation.or(self.rotation.clone());
        self.distortion = other.distortion.or(self.distortion.clone());
        self.channel_mix = other.channel_mix.or(self.channel_mix.clone());
        self.low_pass = other.low_pass.or(self.low_pass.clone());
        self.plugin_filters = other.plugin_filters.or(self.plugin_filters.clone());
    }
}

pub enum EventType {
    Player(PlayerEvents),
    Destroyed,
}
