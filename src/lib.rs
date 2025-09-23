#![doc = include_str!("../README.md")]

use flume::Receiver;
use reqwest::Client as ReqwestClient;
use scc::HashMap as ConcurrentHashMap;
use scc::hash_map::OccupiedEntry;
use std::fmt::{Debug, Formatter};
use std::result::Result;
use std::sync::Arc;
use crate::model::anchorage::{Options, NodeOptions, NodeManagerOptions, PlayerOptions, ConnectionOptions};
use crate::model::error::AnchorageError;
use crate::model::player::EventType;
use crate::node::client::Node;
use crate::player::Player;

pub mod model;
pub mod node;
pub mod player;

/// Main entry point of the library that manages the nodes
pub struct Anchorage {
    /// User-Agent Anchorage will use for each request
    pub user_agent: String,
    /// Reconnect tries for a node before disconnecting it
    pub reconnect_tries: u16,
    /// List of nodes connected currently
    pub nodes: Arc<ConcurrentHashMap<String, Node>>,
    pub(crate) request: ReqwestClient,
}

impl Debug for Anchorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LavalinkClient")
            .field("user_agent", &self.user_agent)
            .field("reconnect_tries", &self.reconnect_tries)
            .field("nodes", &self.nodes.len())
            .finish()
    }
}

impl Anchorage {
    /// Creates a new instance of Anchorage
    pub fn new(mut options: Options) -> Self {
        Self {
            user_agent: options.user_agent.unwrap_or(String::from(format!("Anchorage/{}", env!("CARGO_PKG_VERSION")))),
            reconnect_tries: options.reconnect_tries.unwrap_or(u16::MAX),
            request: options
                .request
                .get_or_insert_with(ReqwestClient::new)
                .to_owned(),
            nodes: Arc::new(ConcurrentHashMap::new())
        }
    }

    /// Creates and connects all the nodes
    #[tracing::instrument(skip(self, nodes_data))]
    pub async fn start(
        &self,
        user_id: u64,
        nodes_data: Vec<impl Into<NodeOptions>>,
    ) -> Result<(), AnchorageError> {
        tracing::info!(
            "Starting Lavalink with user_id ({}) and {} node(s)",
            user_id,
            nodes_data.len()
        );

        for data in nodes_data {
            let info = data.into();
            let name = info.name.clone();

            let (node, handle) = Node::new(NodeManagerOptions {
                name: info.name,
                host: info.host,
                port: info.port,
                auth: info.auth,
                id: user_id,
                request: self.request.clone(),
                user_agent: self.user_agent.clone(),
                reconnect_tries: self.reconnect_tries,
            })
            .await?;

            let nodes = self.nodes.clone();

            tokio::spawn(async move {
                let Ok(name) = handle.await else {
                    return;
                };

                let _ = nodes.remove_async(&name).await;
            });

            self.nodes.insert_async(name, node).await.ok();
        }

        Ok(())
    }

    /// Shortcut to get an ideal node with the least amount of load
    pub async fn get_ideal_node(&self) -> Result<Node, AnchorageError> {
        let mut nodes = vec![];

        self.nodes
            .scan_async(|_, node| nodes.push(node.clone()))
            .await;

        let mut penalties: f64 = 0.0;
        let mut selected_node: Option<Node> = None;

        for node in nodes {
            let data = node.data().await?;
            
            if penalties >= data.penalties {
                selected_node = Some(node);
            }
            
            penalties = data.penalties;
        }

        match selected_node {
            Some(node) => Ok(node),
            None => Err(AnchorageError::NoNodesAvailable),
        }
    }

    /// Gets the node where a player is connected to
    pub async fn get_node_for_player(&self, guild_id: u64) -> Option<OccupiedEntry<String, Node>> {
        self.nodes
            .any_entry_async(|_, node| node.events_sender.contains(&guild_id))
            .await
    }

    /// Creates a new player, that you can interact and listen on events
    pub async fn create_player(
        &self,
        guild_id: u64,
        node: Node,
        connection: impl Into<ConnectionOptions>,
    ) -> Result<(Player, Receiver<EventType>), AnchorageError> {
        if self.get_node_for_player(guild_id).await.is_some() {
            return Err(AnchorageError::CreateExistingPlayer);
        }

        let (player, events_sender, events_receiver) = Player::new(PlayerOptions {
            node: node.clone(),
            guild_id,
            connection: connection.into(),
        })
        .await?;

        let _ = node
            .events_sender
            .insert_async(guild_id, events_sender)
            .await;

        Ok((player, events_receiver))
    }

    /// Destroys an established player
    pub async fn destroy_player(&self, guild_id: u64) -> Result<(), AnchorageError> {
        let Some(node) = self.get_node_for_player(guild_id).await else {
            return Ok(());
        };

        node.rest.destroy_player(guild_id).await?;

        if let Some(sender) = node.events_sender.get(&guild_id) {
            sender.send_async(EventType::Destroyed).await.ok();
        }

        node.events_sender.remove_async(&guild_id).await;

        Ok(())
    }

    /// Connects a disconnected node that is in cache
    pub async fn connect(&self, name: &str) -> Result<(), AnchorageError> {
        if let Some(mut data) = self.nodes.get_async(name).await {
            let node = data.get_mut();
            node.connect().await?;
        }

        Ok(())
    }

    /// Disconnects a connected node, then removes it from cache
    pub async fn disconnect(&self, name: &str, destroy: bool) -> Result<(), AnchorageError> {
        if let Some(mut data) = self.nodes.get_async(name).await {
            let node = data.get_mut();

            node.disconnect().await?;

            if destroy {
                node.destroy().await?;
                self.nodes.remove_async(name).await;
            }
        }

        Ok(())
    }
}