use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use serde_json::to_string;
use std::result::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::model::error::LavalinkRestError;
use crate::model::node::{LavalinkInfo, RoutePlanner, SessionInfo, Stats};
use crate::model::player::{DataType, LavalinkPlayer, PlayerOptions, Track};

pub struct RestOptions {
    pub request: Client,
    pub url: String,
    pub auth: String,
    pub agent: String,
    pub session_id: Arc<RwLock<Option<String>>>,
}

#[derive(Clone, Debug)]
pub struct Rest {
    pub request: Client,
    pub url: String,
    pub auth: String,
    pub agent: String,
    session_id: Arc<RwLock<Option<String>>>,
}

impl Rest {
    pub fn new(options: RestOptions) -> Self {
        Self {
            request: options.request,
            url: options.url,
            auth: options.auth,
            agent: options.agent,
            session_id: options.session_id,
        }
    }

    pub async fn get_session_id(&self) -> Result<String, LavalinkRestError> {
        let option = self.session_id.read().await.clone();
        option.ok_or(LavalinkRestError::NoSessionId)
    }

    pub async fn resolve(&self, identifier: String) -> Result<DataType, LavalinkRestError> {
        let request = self
            .request
            .get(format!("{}/loadtracks", self.url))
            .query(&[("identifier", &identifier)]);

        self.make_request::<DataType>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    pub async fn decode(&self, track: String) -> Result<Track, LavalinkRestError> {
        let request = self
            .request
            .get(format!("{}/decodetrack", self.url))
            .query(&[("track", &track)]);

        self.make_request::<Track>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    pub async fn get_player(&self, guild_id: u64) -> Result<LavalinkPlayer, LavalinkRestError> {
        let request = self.request.get(format!(
            "{}/sessions/{}/players/{}",
            self.url,
            self.get_session_id().await?,
            guild_id
        ));

        self.make_request::<LavalinkPlayer>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    pub async fn get_players(&self) -> Result<Vec<LavalinkPlayer>, LavalinkRestError> {
        let request = self.request.get(format!(
            "{}/sessions/{}/players",
            self.url,
            self.get_session_id().await?
        ));

        self.make_request::<Vec<LavalinkPlayer>>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    pub async fn update_player(
        &self,
        guild_id: u64,
        no_replace: bool,
        options: PlayerOptions,
    ) -> Result<LavalinkPlayer, LavalinkRestError> {
        let request = self
            .request
            .patch(format!(
                "{}/sessions/{}/players/{}",
                self.url,
                self.get_session_id().await?,
                guild_id
            ))
            .query(&[("noReplace", &no_replace)])
            .header("Content-Type", "application/json")
            .body(to_string(&options)?);

        self.make_request::<LavalinkPlayer>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    pub async fn destroy_player(&self, guild_id: u64) -> Result<(), LavalinkRestError> {
        let request = self.request.delete(format!(
            "{}/sessions/{}/players/{}",
            self.url,
            self.get_session_id().await?,
            guild_id
        ));

        self.make_request::<()>(request).await?;

        Ok(())
    }

    pub async fn update_session(
        &self,
        options: SessionInfo,
    ) -> Result<SessionInfo, LavalinkRestError> {
        let request = self
            .request
            .patch(format!(
                "{}/sessions/{}",
                self.url,
                self.get_session_id().await?
            ))
            .body(to_string(&options)?);

        self.make_request::<SessionInfo>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    pub async fn stats(&self) -> Result<Stats, LavalinkRestError> {
        let request = self.request.get(format!("{}/stats", self.url));

        self.make_request::<Stats>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    pub async fn route_planner_status(&self) -> Result<RoutePlanner, LavalinkRestError> {
        let request = self
            .request
            .get(format!("{}/routeplanner/status", self.url));

        self.make_request::<RoutePlanner>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    pub async fn unmark_failed_address(&self, address: String) -> Result<(), LavalinkRestError> {
        let request = self
            .request
            .post(format!("{}/routeplanner/free/address", self.url))
            .header("Content-Type", "application/json")
            .body(format!("{{ address:{} }}", address));

        self.make_request::<()>(request).await?;

        Ok(())
    }

    pub async fn info(&self) -> Result<LavalinkInfo, LavalinkRestError> {
        let request = self.request.get(format!("{}/info", self.url));

        self.make_request::<LavalinkInfo>(request)
            .await?
            .ok_or(LavalinkRestError::NothingReturned)
    }

    async fn make_request<T: for<'de> Deserialize<'de>>(
        &self,
        builder: RequestBuilder,
    ) -> Result<Option<T>, LavalinkRestError> {
        let request = builder
            .header("Authorization", self.auth.as_str())
            .header("User-Agent", self.agent.as_str())
            .build()?;

        let response = self.request.execute(request).await?;

        if !response.status().is_success() {
            return Err(LavalinkRestError::ResponseReceivedNotOk(response.status()));
        }

        let text = response.text().await?;

        if text.is_empty() {
            return Ok(None);
        }

        Ok(Some(serde_json::from_str::<T>(&text)?))
    }
}
