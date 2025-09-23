use std::{result::Result, time::Duration};
use flume::{Receiver as FlumeReceiver, Sender as FlumeSender, unbounded};
use futures::stream::StreamExt;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tokio_tungstenite::tungstenite::{Message, handshake::client::Request};

use crate::model::error::LavalinkNodeError;
use crate::model::node::LavalinkMessage;

/// Internal websocket handler around WebsocketStream from tokio_tungstenite
pub struct ConnectionManager {
    pub stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl ConnectionManager {
    pub async fn new(request: Request) -> Result<Self, LavalinkNodeError> {
        let (stream, _) = connect_async(request).await?;

        Ok(Self { stream })
    }

    pub async fn get_message(&mut self) -> Result<Option<LavalinkMessage>, TungsteniteError> {
        let Some(result) = self.stream.next().await else {
            return Err(TungsteniteError::AlreadyClosed);
        };

        let result = match result {
            Ok(message) => message,
            Err(error) => return Err(error),
        };

        let string = match result {
            Message::Text(string) => string,
            Message::Close(_) => return Err(TungsteniteError::ConnectionClosed),
            _ => return Ok(None),
        };

        let message = match serde_json::from_str::<LavalinkMessage>(&string) {
            Ok(message) => message,
            _ => return Ok(None),
        };

        Ok(Some(message))
    }
}


/// Public facing wrapper around connection manager
pub struct Connection {
    handle: Option<JoinHandle<()>>,
    sender: FlumeSender<Result<Option<LavalinkMessage>, TungsteniteError>>,
}

impl Connection {
    pub fn new() -> (
        Self,
        FlumeReceiver<Result<Option<LavalinkMessage>, TungsteniteError>>,
    ) {
        let (sender, receiver) = unbounded::<Result<Option<LavalinkMessage>, TungsteniteError>>();

        let connection = Self {
            handle: None,
            sender,
        };

        (connection, receiver)
    }

    pub fn available(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
    }

    #[tracing::instrument(skip(self))]
    pub async fn connect(&mut self, request: Request) -> Result<(), LavalinkNodeError> {
        self.disconnect().await;

        let mut manager = ConnectionManager::new(request).await?;

        let sender = self.sender.clone();

        let handle = tokio::spawn(async move {
            loop {
                match manager.get_message().await {
                    Ok(message) => {
                        if sender.send_async(Ok(message)).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        if sender.send_async(Err(error)).await.is_err() {
                            break;
                        }
                        break;
                    }
                }
            }

            tracing::debug!("Websocket connection is closed");
        });

        tracing::debug!(
            "Websocket connection established and is running! [Join Handle Id ({})]",
            handle.id()
        );

        #[allow(clippy::let_underscore_future)]
        let _ = self.handle.insert(handle);

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn disconnect(&mut self) {
        let Some(handle) = self.handle.take() else {
            return;
        };

        handle.abort();

        while !handle.is_finished() {
            sleep(Duration::from_millis(1)).await;
        }

        tracing::debug!(
            "Websocket connection stopped and deleted! [Join Handle Id ({})]",
            handle.id()
        );
    }
}
