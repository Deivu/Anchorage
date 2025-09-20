use std::sync::Arc;
use reqwest::Client;
use tokio::sync::RwLock;
use reqwest::Client as ReqwestClient;

use crate::node::client::Node;

#[derive(Clone)]
pub struct NodeManagerOptions {
    pub name: String,
    pub host: String,
    pub port: u32,
    pub auth: String,
    pub id: u64,
    pub request: ReqwestClient,
    pub agent: String,
}

pub struct RestOptions {
    pub request: Client,
    pub url: String,
    pub auth: String,
    pub agent: String,
    pub session_id: Arc<RwLock<Option<String>>>,
}

pub struct PlayerOptions {
    pub node: Node,
    pub connection: ConnectionOptions,
    pub guild_id: u64,
}

pub struct ConnectionOptions {
    pub channel_id: Option<u64>,
    pub endpoint: String,
    pub guild_id: u64,
    pub session_id: String,
    pub token: String,
    pub user_id: u64,
}

pub struct NodeOptions {
    pub name: String,
    pub host: String,
    pub port: u32,
    pub auth: String,
}

pub struct Options {
    pub agent: String,
    pub request: Option<Client>,
}
