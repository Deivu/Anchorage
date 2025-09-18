use flume::{Receiver as FlumeReceiver, Sender as FlumeSender, unbounded};
use scc::HashMap as ConcurrentHashMap;
use std::collections::HashMap;
use std::result::Result;
use std::sync::Arc;
use std::time::Duration;
use reqwest::Client as RequestClient;
use tokio::sync::RwLock;
use tokio::sync::oneshot::{Sender as TokioOneshotSender, channel};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;

use crate::model::node::{LavalinkMessage, Stats};
use crate::model::player::{EventType, PlayerEvents};
use crate::model::error::LavalinkNodeError;
use crate::node::rest::{Rest, RestOptions};
use crate::node::websocket::Connection;

pub enum WebsocketCommand {
    Connect(TokioOneshotSender<Result<(), LavalinkNodeError>>),
    Disconnect(TokioOneshotSender<()>),
    Destroy(TokioOneshotSender<()>),
    GetData(TokioOneshotSender<Result<NodeManagerData, LavalinkNodeError>>),
}

pub enum NodeManagerCommands {
    Command(WebsocketCommand),
    Message(Result<Option<LavalinkMessage>, TungsteniteError>),
}

#[derive(Clone, Debug)]
pub struct NodeManagerData {
    pub name: String,
    pub auth: String,
    pub id: u64,
    pub url: String,
    pub penalties: f64,
    pub statistics: Option<Stats>,
}

#[derive(Clone)]
pub struct NodeManagerOptions {
    pub name: String,
    pub host: String,
    pub port: u32,
    pub auth: String,
    pub id: u64,
    pub request: RequestClient,
    pub nodes: Arc<ConcurrentHashMap<String, Node>>,
    pub agent: String,
}

pub struct NodeManager {
    pub data: NodeManagerData,
    pub event_senders: Arc<ConcurrentHashMap<u64, FlumeSender<EventType>>>,
    pub session_id: Arc<RwLock<Option<String>>>,
    agent: String,
    nodes: Arc<ConcurrentHashMap<String, Node>>,
    receiver: FlumeReceiver<NodeManagerCommands>,
    connection: Connection,
    destroyed: bool,
    reconnects: usize,
    handles: Vec<JoinHandle<()>>,
}

impl NodeManager {
    pub fn new(
        options: NodeManagerOptions,
        commands_receiver: FlumeReceiver<WebsocketCommand>,
    ) -> Self {
        let (websocket_connection, message_receiver) = Connection::new();

        let (node_sender, node_receiver) = unbounded::<NodeManagerCommands>();

        let mut manager = Self {
            data: NodeManagerData {
                name: options.name,
                auth: options.auth,
                id: options.id,
                url: format!("ws://{}:{}/v4/websocket", options.host, options.port),
                penalties: 0.0,
                statistics: None,
            },
            // I decided to make these two variables shared to reduce complexity
            event_senders: Arc::new(ConcurrentHashMap::new()),
            session_id: Arc::new(RwLock::new(None)),
            agent: options.agent,
            nodes: options.nodes,
            receiver: node_receiver,
            connection: websocket_connection,
            destroyed: false,
            reconnects: 0,
            handles: Vec::new(),
        };

        let name = manager.data.name.clone();
        let sender = node_sender.clone();

        manager.handles.push(tokio::spawn(async move {
            while let Ok(command) = commands_receiver.recv_async().await {
                sender
                    .send_async(NodeManagerCommands::Command(command))
                    .await
                    .ok();
            }

            tracing::debug!("Lavalink Node {} stopped on listening for commands", name);
        }));

        let name = manager.data.name.clone();
        let sender = node_sender.clone();

        manager.handles.push(tokio::spawn(async move {
            while let Ok(command) = message_receiver.recv_async().await {
                sender
                    .send_async(NodeManagerCommands::Message(command))
                    .await
                    .ok();
            }

            tracing::debug!("Lavalink Node {} stopped on listening for messages", name);
        }));

        manager
    }

    pub async fn start(&mut self) -> Result<(), LavalinkNodeError> {
        let result = self.handle().await;

        // check players and handle accordingly
        self.send_players_destroy().await;

        result
    }

    async fn handle(&mut self) -> Result<(), LavalinkNodeError> {
        while !self.destroyed {
            let data = self.receiver.recv_async().await?;

            match data {
                NodeManagerCommands::Command(command) => self.handle_command(command).await?,
                NodeManagerCommands::Message(message) => self.handle_message(message).await?,
            }
        }

        Ok(())
    }

    async fn send_players_destroy(&mut self) {
        self.event_senders
            .scan_async(|_, sender| {
                sender.send(EventType::Destroyed).ok();
            })
            .await;

        self.event_senders.clear_async().await;
    }

    async fn handle_command(&mut self, command: WebsocketCommand) -> Result<(), LavalinkNodeError> {
        match command {
            WebsocketCommand::Connect(sender) => {
                sender.send(self.connect().await).ok();
            }
            WebsocketCommand::Disconnect(sender) => {
                self.disconnect().await;
                sender.send(()).ok();
            }
            WebsocketCommand::Destroy(sender) => {
                self.destroy().await;
                sender.send(()).ok();
            }
            WebsocketCommand::GetData(sender) => {
                sender.send(Ok(self.data.clone())).ok();
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn handle_message(
        &mut self,
        result: Result<Option<LavalinkMessage>, TungsteniteError>,
    ) -> Result<(), LavalinkNodeError> {
        let Ok(option) = result else {
            self.connect().await?;
            return Ok(());
        };

        let Some(message) = option else {
            return Ok(());
        };

        tracing::debug!("Lavalink Node {} received a message!", self.data.name);

        match message {
            LavalinkMessage::Ready(data) => {
                {
                    let _ = self
                        .session_id
                        .write()
                        .await
                        .insert(data.session_id.clone());
                }

                tracing::info!(
                    "Lavalink Node {} is now ready! [Resumed: {}] [Session Id: {}]",
                    self.data.name,
                    data.resumed,
                    data.session_id
                );

                Ok(())
            }
            LavalinkMessage::Stats(data) => {
                let mut penalties: f64 = 0.0;

                let _ = self.data.statistics.insert(data.clone());

                penalties += data.players as f64;
                penalties += f64::powf(1.05, 100.0 * data.cpu.system_load).round();

                if data.frame_stats.is_some() {
                    penalties += data.frame_stats.clone().unwrap().deficit as f64;
                    penalties += (data.frame_stats.clone().unwrap().nulled as f64) * 2.0;
                }

                self.data.penalties = penalties;

                Ok(())
            }
            LavalinkMessage::Event(data) => {
                let guild_id = match &data {
                    PlayerEvents::TrackStartEvent(data) => &data.guild_id,
                    PlayerEvents::TrackEndEvent(data) => &data.guild_id,
                    PlayerEvents::TrackExceptionEvent(data) => &data.guild_id,
                    PlayerEvents::TrackStuckEvent(data) => &data.guild_id,
                    PlayerEvents::WebSocketClosedEvent(data) => &data.guild_id,
                };

                let Some(sender) = self.event_senders.get_async(guild_id).await else {
                    return Ok(());
                };

                sender.send_async(EventType::Player(data)).await.ok();

                Ok(())
            }
            _ => Ok(()),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn connect(&mut self) -> Result<(), LavalinkNodeError> {
        if self.connection.available() {
            return Ok(());
        }

        loop {
            let key = generate_key();
            let mut request = Request::builder()
                .method("GET")
                .header("Host", &self.data.url)
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", &key)
                .uri(&self.data.url)
                .body(())?;

            let pairs: &mut HashMap<&str, &String> = &mut HashMap::new();

            let id = self.data.id.to_string();

            pairs.insert("User-Id", &id);
            pairs.insert("Authorization", &self.data.auth);

            let session_id = match &self.session_id.read().await.as_ref() {
                Some(session_id) => String::from(*session_id),
                None => String::from(""),
            };

            pairs.insert("Session-Id", &session_id);
            pairs.insert("Client-Name", &self.agent);
            pairs.insert("User-Agent", &self.agent);

            let headers = request.headers_mut();

            for (key, value) in pairs {
                headers.append(*key, value.parse()?);
            }

            self.reconnects += 1;

            tracing::debug!(
                "Lavalink Node {} Connecting to {} [Retries: {}]",
                self.data.name,
                self.data.url,
                self.reconnects
            );

            let Err(result) = self.connection.connect(request).await else {
                break;
            };

            // todo!() reconnect tries will not be static and will be available to configure later
            if self.reconnects < 3 {
                // todo!() will not be static and will be available to configure later
                let duration = Duration::from_secs(5);

                tracing::debug!(
                    "Lavalink Node {} failed to connect to {}. Waiting for {} second(s)",
                    self.data.name,
                    self.data.url,
                    duration.as_secs()
                );

                sleep(duration).await;

                continue;
            }

            self.reconnects = 0;

            return Err(result);
        }

        self.reconnects = 0;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn disconnect(&mut self) {
        self.connection.disconnect().await;

        self.send_players_destroy().await;

        self.reconnects = 0;

        tracing::info!("Lavalink Node {} Disconnected...", self.data.name);
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy(&mut self) {
        self.disconnect().await;

        self.destroyed = true;
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub rest: Rest,
    pub events_sender: Arc<ConcurrentHashMap<u64, FlumeSender<EventType>>>,
    commands_sender: FlumeSender<WebsocketCommand>,
}

impl Node {
    pub async fn new(
        options: NodeManagerOptions,
    ) -> Result<(Self, JoinHandle<String>), LavalinkNodeError> {
        let (commands_sender, commands_receiver) = unbounded::<WebsocketCommand>();

        let mut manager = NodeManager::new(options.clone(), commands_receiver);

        manager.connect().await?;

        let rest = Rest::new(RestOptions {
            request: options.request,
            url: format!("http://{}:{}/v4", options.host, options.port),
            auth: options.auth.clone(),
            agent: options.agent.clone(),
            session_id: manager.session_id.clone(),
        });

        let node = Self {
            rest,
            events_sender: manager.event_senders.clone(),
            commands_sender,
        };

        let handle = tokio::spawn(async move {
            tracing::debug!(
                "Lavalink Node {} started to listen for websocket and commands",
                manager.data.name
            );

            if let Err(error) = manager.start().await {
                tracing::error!(
                    "Lavalink Node {} threw an unrecoverable error. Cleaning up! => {:?}",
                    manager.data.name,
                    error
                );
            }

            manager.data.name
        });

        Ok((node, handle))
    }

    pub async fn data(&self) -> Result<NodeManagerData, LavalinkNodeError> {
        let (sender, receiver) = channel::<Result<NodeManagerData, LavalinkNodeError>>();

        self.commands_sender
            .send_async(WebsocketCommand::GetData(sender))
            .await?;

        receiver.await?
    }

    pub async fn connect(&self) -> Result<(), LavalinkNodeError> {
        let (sender, receiver) = channel::<Result<(), LavalinkNodeError>>();

        self.commands_sender
            .send_async(WebsocketCommand::Connect(sender))
            .await?;

        receiver.await?
    }

    pub async fn disconnect(&self) -> Result<(), LavalinkNodeError> {
        let (sender, receiver) = channel::<()>();

        self.commands_sender
            .send_async(WebsocketCommand::Disconnect(sender))
            .await?;

        Ok(receiver.await?)
    }

    pub async fn destroy(&self) -> Result<(), LavalinkNodeError> {
        let (sender, receiver) = channel::<()>();

        self.commands_sender
            .send_async(WebsocketCommand::Destroy(sender))
            .await?;

        Ok(receiver.await?)
    }
}
