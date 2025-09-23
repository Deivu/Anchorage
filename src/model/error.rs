use thiserror::Error as ThisError;

/// List of errors that can throw from an instance of Lavalink Node
#[derive(ThisError, Debug)]
pub enum LavalinkNodeError {
    #[error(transparent)]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),
    #[error(transparent)]
    TungsteniteHttp(#[from] tokio_tungstenite::tungstenite::http::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error(transparent)]
    FlumeRecv(#[from] flume::RecvError),
    #[error(transparent)]
    FlumeTimeout(#[from] flume::RecvTimeoutError),
    #[error("Failed to send data to node worker ({0})")]
    TokioOneshotChannelSend(String),
    #[error("Failed to receive data from node worker => {}", .0.to_string())]
    TokioOneshotChannelRecv(#[from] tokio::sync::oneshot::error::RecvError),
}

/// List of errors that can throw from an instance of Lavalink Rest
#[derive(ThisError, Debug)]
pub enum LavalinkRestError {
    #[error(transparent)]
    LavalinkNode(#[from] LavalinkNodeError),
    #[error(transparent)]
    SerdeParse(#[from] serde_json::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("Response received is not ok ({})", .0.to_string())]
    ResponseReceivedNotOk(reqwest::StatusCode),
    #[error("No Session Id present to create this request")]
    NoSessionId,
    #[error("Unexpected none result on a function that should have a result")]
    NothingReturned,
}

/// List of errors that can throw from an instance of Lavalink Player
#[derive(ThisError, Debug)]
pub enum LavalinkPlayerError {
    #[error(transparent)]
    LavalinkRest(#[from] LavalinkRestError),
    #[error(transparent)]
    FlumeRecv(#[from] flume::RecvError),
    #[error(transparent)]
    TokioRecv(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("Failed to send an event ({0})")]
    FlumeSend(String),
}

/// List of errors that can throw from an instance of Anchorage
#[derive(ThisError, Debug)]
pub enum AnchorageError {
    #[error(transparent)]
    LavalinkNode(#[from] LavalinkNodeError),
    #[error(transparent)]
    LavalinkPlayer(#[from] LavalinkPlayerError),
    #[error(transparent)]
    LavalinkRest(#[from] LavalinkRestError),
    #[error("Tried to create a new player when there is already an existing one")]
    CreateExistingPlayer,
    #[error("No nodes available to get")]
    NoNodesAvailable,
}

impl<T> From<flume::SendError<T>> for LavalinkPlayerError {
    fn from(value: flume::SendError<T>) -> Self {
        LavalinkPlayerError::FlumeSend(value.to_string())
    }
}

impl<T> From<flume::SendError<T>> for LavalinkNodeError {
    fn from(value: flume::SendError<T>) -> Self {
        LavalinkNodeError::TokioOneshotChannelSend(value.to_string())
    }
}
