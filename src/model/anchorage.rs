use std::sync::Arc;
use reqwest::Client;
use tokio::sync::RwLock;
use reqwest::Client as ReqwestClient;

use crate::node::client::Node;

/// Options to initialize an internal NodeManager
#[derive(Clone)]
pub struct NodeManagerOptions {
    pub name: String,
    pub host: String,
    pub port: u32,
    pub auth: String,
    pub id: u64,
    pub request: ReqwestClient,
    pub user_agent: String,
    pub reconnect_tries: u16,
}

/// Options to initialize a Rest client
pub struct RestOptions {
    pub request: Client,
    pub url: String,
    pub auth: String,
    pub user_agent: String,
    pub session_id: Arc<RwLock<Option<String>>>,
}

/// Options to create a player
pub struct PlayerOptions {
    pub node: Node,
    pub connection: ConnectionOptions,
    pub guild_id: u64,
}

/// Options to be used to connect to a voice channel
pub struct ConnectionOptions {
    pub channel_id: Option<u64>,
    pub endpoint: String,
    pub guild_id: u64,
    pub session_id: String,
    pub token: String,
    pub user_id: u64,
}

/// User node options used to create a node
pub struct NodeOptions {
    pub name: String,
    pub host: String,
    pub port: u32,
    pub auth: String,
}

/// Options to initialize an Anchorage client
pub struct Options {
    pub user_agent: Option<String>,
    pub reconnect_tries: Option<u16>,
    pub request: Option<Client>,
}
