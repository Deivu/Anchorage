use flume::Receiver;
use reqwest::Client as ReqwestClient;
use scc::HashMap as ConcurrentHashMap;
use scc::hash_map::OccupiedEntry;
use std::fmt::{Debug, Formatter};
use std::result::Result;
use std::sync::Arc;

use crate::model::anchorage::{Anchorage, Options, NodeOptions, ConnectionOption};
use crate::model::error::LavalinkError;
use crate::model::player::EventType;
use crate::node::client::{Node, NodeManagerOptions};
use crate::player::CreatePlayerOptions;
use crate::player::Player;

pub mod model;
pub mod node;
pub mod player;

impl Debug for Anchorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LavalinkClient")
            .field("agent", &self.agent)
            .field("nodes", &self.nodes.len())
            .finish()
    }
}

impl Anchorage {
    pub fn new(mut options: Options) -> Self {
        Self {
            agent: String::from("Kashima/5.0.0-dev"),
            nodes: Arc::new(ConcurrentHashMap::new()),
            request: options
                .request
                .get_or_insert_with(ReqwestClient::new)
                .to_owned(),
        }
    }

    #[tracing::instrument(skip(self, nodes_data))]
    pub async fn start(
        &self,
        user_id: u64,
        nodes_data: Vec<impl Into<NodeOptions>>,
    ) -> Result<(), LavalinkError> {
        tracing::info!(
            "Starting Lavalink with user_id ({}) and {} node(s)",
            user_id,
            nodes_data.len()
        );

        for data in nodes_data {
            let info = data.into();
            let name = info.name.clone();

            let (node, handle) = Node::new(NodeManagerOptions {
                client: self.clone(),
                name: info.name,
                host: info.host,
                port: info.port,
                auth: info.auth,
                id: user_id,
                request: self.request.clone(),
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

    pub async fn get_ideal_node(&self) -> Result<Node, LavalinkError> {
        let mut nodes = vec![];

        self.nodes
            .scan_async(|_, node| nodes.push(node.clone()))
            .await;

        let mut penalties: f64;
        let mut selected_node: Option<Node> = None;

        for node in nodes {
            let data = node.data().await?;
            penalties = data.penalties;
            if penalties >= data.penalties {
                selected_node = Some(node);
            }
        }

        match selected_node {
            Some(node) => Ok(node),
            None => Err(LavalinkError::NoNodesAvailable),
        }
    }

    pub async fn get_node_for_player(&self, guild_id: u64) -> Option<OccupiedEntry<String, Node>> {
        self.nodes
            .any_entry_async(|_, node| node.events_sender.contains(&guild_id))
            .await
    }

    pub async fn create_player(
        &self,
        guild_id: u64,
        node: Node,
        connection: impl Into<ConnectionOption>,
    ) -> Result<(Player, Receiver<EventType>), LavalinkError> {
        if self.get_node_for_player(guild_id).await.is_some() {
            return Err(LavalinkError::CreateExistingPlayer);
        }

        let (player, events_sender, events_receiver) = Player::new(CreatePlayerOptions {
            agent: self.agent.clone(),
            node: node.clone(),
            guild_id,
            connection,
        })
        .await?;

        let _ = node
            .events_sender
            .insert_async(guild_id, events_sender)
            .await;

        Ok((player, events_receiver))
    }

    pub async fn destroy_player(&self, guild_id: u64) -> Result<(), LavalinkError> {
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

    pub async fn connect(&self, name: String) -> Result<(), LavalinkError> {
        if let Some(mut data) = self.nodes.get_async(&*name).await {
            let node = data.get_mut();
            node.connect().await?;
        }

        Ok(())
    }

    pub async fn disconnect(&self, name: String, destroy: bool) -> Result<(), LavalinkError> {
        if let Some(mut data) = self.nodes.get_async(&*name).await {
            let node = data.get_mut();

            node.disconnect().await?;

            if destroy {
                node.destroy().await?;
                self.nodes.remove_async(&*name).await;
            }
        }

        Ok(())
    }
}