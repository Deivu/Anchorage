use std::sync::Arc;
use reqwest::Client;
use scc::HashMap;
use crate::node::client::Node;

pub struct ConnectionOption {
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

pub struct Anchorage {
    pub agent: String,
    pub nodes: Arc<HashMap<String, Node>>,
    crate request: Client,
}