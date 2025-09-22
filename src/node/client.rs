use flume::{Receiver as FlumeReceiver, Sender as FlumeSender, unbounded};
use scc::HashMap as ConcurrentHashMap;
use std::collections::HashMap;
use std::result::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::oneshot::{Sender as TokioOneshotSender, channel};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;

use crate::model::anchorage::NodeManagerOptions;
use crate::model::error::LavalinkNodeError;
use crate::model::node::{LavalinkMessage, Stats};
use crate::model::player::{EventType, PlayerEvents};
use crate::model::anchorage::RestOptions;
use crate::node::rest::Rest;
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

pub struct NodeManagerData {
    /// Name of this node
    pub name: String,
    /// Authorization key for this node
    pub auth: String,
    /// UserId that this node will use
    pub id: u64,
    /// Base url for this node
    pub url: String,
    /// Penalties used for ideal node calculation
    pub penalties: f64,
    /// Status of this node
    pub statistics: Option<Stats>,
}

/// Internal websocket handler
pub struct NodeManager {
    pub name: String,
    pub auth: String,
    pub id: u64,
    pub url: String,
    pub penalties: f64,
    pub statistics: Option<Stats>,
    pub session_id: Arc<RwLock<Option<String>>>,
    pub event_senders: Arc<ConcurrentHashMap<u64, FlumeSender<EventType>>>,
    user_agent: String,
    reconnect_tries: u16,
    receiver: FlumeReceiver<NodeManagerCommands>,
    connection: Connection,
    destroyed: bool,
    reconnects: u16,
    handles: Vec<JoinHandle<()>>,
}

impl From<&NodeManager> for NodeManagerData {
    fn from(value: &NodeManager) -> Self {
        NodeManagerData {
            name: value.name.clone(),
            auth: value.auth.clone(),
            id: value.id,
            url: value.url.clone(),
            penalties: value.penalties,
            statistics: value.statistics.clone(),
        }
    }
}

impl NodeManager {
    /// Creates a new node manager
    pub fn new(
        options: NodeManagerOptions,
        commands_receiver: FlumeReceiver<WebsocketCommand>,
    ) -> Self {
        let (websocket_connection, message_receiver) = Connection::new();

        let (node_sender, node_receiver) = unbounded::<NodeManagerCommands>();

        let mut manager = Self {
            name: options.name,
            auth: options.auth,
            id: options.id,
            url: format!("ws://{}:{}/v4/websocket", options.host, options.port),
            penalties: 0.0,
            statistics: None,
            session_id: Arc::new(RwLock::new(None)),
            event_senders: Arc::new(ConcurrentHashMap::new()),
            user_agent: options.user_agent,
            reconnect_tries: options.reconnect_tries,
            receiver: node_receiver,
            connection: websocket_connection,
            destroyed: false,
            reconnects: 0,
            handles: Vec::new(),
        };

        let name = manager.name.clone();
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

        let name = manager.name.clone();
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

    /// Starts this manager to listen for commands and messages
    /// # This function will never resolve until the node errors, or stops to listen
    pub async fn start(&mut self) -> Result<(), LavalinkNodeError> {
        let result = self.handle().await;

        // check players and handle accordingly
        self.send_players_destroy().await;

        result
    }

    /// Handles the event received
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

    /// Send destroy event on all players in this node, then clears the events cache
    async fn send_players_destroy(&mut self) {
        self.event_senders
            .scan_async(|_, sender| {
                sender.send(EventType::Destroyed).ok();
            })
            .await;

        self.event_senders.clear_async().await;
    }

    /// Handles commands received from interface struct
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
                let me = &*self;
                sender.send(Ok(me.into())).ok();
            }
        }

        Ok(())
    }

    /// Handles messages from lavalink
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

        tracing::debug!("Lavalink Node {} received a message!", self.name);

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
                    self.name,
                    data.resumed,
                    data.session_id
                );

                Ok(())
            }
            LavalinkMessage::Stats(data) => {
                let mut penalties: f64 = 0.0;

                let _ = self.statistics.insert(data.clone());

                penalties += data.players as f64;
                penalties += f64::powf(1.05, 100.0 * data.cpu.system_load).round();

                if data.frame_stats.is_some() {
                    penalties += data.frame_stats.clone().unwrap().deficit as f64;
                    penalties += (data.frame_stats.clone().unwrap().nulled as f64) * 2.0;
                }

                self.penalties = penalties;

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

    /// Connects this node
    #[tracing::instrument(skip(self))]
    pub async fn connect(&mut self) -> Result<(), LavalinkNodeError> {
        if self.connection.available() {
            return Ok(());
        }

        loop {
            let key = generate_key();
            let mut request = Request::builder()
                .method("GET")
                .header("Host", &self.url)
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", &key)
                .uri(&self.url)
                .body(())?;

            let pairs: &mut HashMap<&str, &String> = &mut HashMap::new();

            let id = self.id.to_string();

            pairs.insert("User-Id", &id);
            pairs.insert("Authorization", &self.auth);

            let session_id = match &self.session_id.read().await.as_ref() {
                Some(session_id) => String::from(*session_id),
                None => String::from(""),
            };

            pairs.insert("Session-Id", &session_id);
            pairs.insert("Client-Name", &self.user_agent);
            pairs.insert("User-Agent", &self.user_agent);

            let headers = request.headers_mut();

            for (key, value) in pairs {
                headers.append(*key, value.parse()?);
            }

            self.reconnects += 1;

            tracing::debug!(
                "Lavalink Node {} Connecting to {} [Retries: {}]",
                self.name,
                self.url,
                self.reconnects
            );

            let Err(result) = self.connection.connect(request).await else {
                break;
            };

            if self.reconnects < self.reconnect_tries {
                let duration = Duration::from_secs(5);

                tracing::debug!(
                    "Lavalink Node {} failed to connect to {}. Waiting for {} second(s)",
                    self.name,
                    self.url,
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

    /// Disconnects this node
    #[tracing::instrument(skip(self))]
    pub async fn disconnect(&mut self) {
        self.connection.disconnect().await;

        self.send_players_destroy().await;

        self.reconnects = 0;

        tracing::info!("Lavalink Node {} Disconnected...", self.name);
    }

    /// Destroys this node
    #[tracing::instrument(skip(self))]
    pub async fn destroy(&mut self) {
        self.disconnect().await;

        self.destroyed = true;
    }
}

/// Interface to communicate with the websocket
#[derive(Clone, Debug)]
pub struct Node {
    /// Rest interface for this node
    pub rest: Rest,
    /// List of events sender channel where this node will send player events on
    pub events_sender: Arc<ConcurrentHashMap<u64, FlumeSender<EventType>>>,
    commands_sender: FlumeSender<WebsocketCommand>,
}

impl Node {
    /// Creates a new Node interface and underlying worker
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
            user_agent: options.user_agent.clone(),
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
                manager.name
            );

            if let Err(error) = manager.start().await {
                tracing::error!(
                    "Lavalink Node {} threw an unrecoverable error. Cleaning up! => {:?}",
                    manager.name,
                    error
                );
            }

            manager.name
        });

        Ok((node, handle))
    }

    /// Gets the current node data
    pub async fn data(&self) -> Result<NodeManagerData, LavalinkNodeError> {
        let (sender, receiver) = channel::<Result<NodeManagerData, LavalinkNodeError>>();

        self.commands_sender
            .send_async(WebsocketCommand::GetData(sender))
            .await?;

        receiver.await?
    }

    /// Connects this node
    pub async fn connect(&self) -> Result<(), LavalinkNodeError> {
        let (sender, receiver) = channel::<Result<(), LavalinkNodeError>>();

        self.commands_sender
            .send_async(WebsocketCommand::Connect(sender))
            .await?;

        receiver.await?
    }

    /// Disconnects this node
    pub async fn disconnect(&self) -> Result<(), LavalinkNodeError> {
        let (sender, receiver) = channel::<()>();

        self.commands_sender
            .send_async(WebsocketCommand::Disconnect(sender))
            .await?;

        Ok(receiver.await?)
    }

    /// Destroys this node
    pub async fn destroy(&self) -> Result<(), LavalinkNodeError> {
        let (sender, receiver) = channel::<()>();

        self.commands_sender
            .send_async(WebsocketCommand::Destroy(sender))
            .await?;

        Ok(receiver.await?)
    }
}
