use flume::{Receiver as FlumeReceiver, Sender as FlumeSender, unbounded};
use serde_json::Value;
use std::result::Result;

use crate::model::anchorage::{ConnectionOptions, PlayerOptions};
use crate::model::error::LavalinkPlayerError;
use crate::model::player::{
    EventType, LavalinkFilters, LavalinkPlayer, LavalinkPlayerOptions, LavalinkVoice,
    UpdatePlayerTrack,
};
use crate::node::client::Node;

/// A player instance
pub struct Player {
    /// GuildId for this player
    pub guild_id: u64,
    /// Node where this player is
    node: Node,
}

impl Player {
    /// Creates a new player
    pub async fn new(
        options: PlayerOptions,
    ) -> Result<(Self, FlumeSender<EventType>, FlumeReceiver<EventType>), LavalinkPlayerError> {
        let (events_sender, events_receiver) = unbounded::<EventType>();

        let player = Self {
            guild_id: options.guild_id,
            node: options.node,
        };

        player.update_connection(options.connection).await?;

        Ok((player, events_sender, events_receiver))
    }

    /// Gets the data of this player from lavalink
    pub async fn get_data(&self) -> Result<LavalinkPlayer, LavalinkPlayerError> {
        Ok(self.node.rest.get_player(self.guild_id).await?)
    }

    /// Plays a track
    pub async fn play(&self, track: String) -> Result<(), LavalinkPlayerError> {
        let mut options: LavalinkPlayerOptions = Default::default();
        let mut update_track: UpdatePlayerTrack = Default::default();

        let _ = update_track.encoded.insert(Value::String(track));

        let _ = options.track.insert(update_track);

        self.send_update_player(false, options).await?;

        Ok(())
    }

    /// Stops the current playback
    pub async fn stop(&self) -> Result<(), LavalinkPlayerError> {
        let mut options: LavalinkPlayerOptions = Default::default();
        let mut update_track: UpdatePlayerTrack = Default::default();

        let _ = update_track.encoded.insert(Value::Null);

        let _ = options.track.insert(update_track);

        self.send_update_player(false, options).await?;

        Ok(())
    }

    /// Destroys the player on lavalink
    pub async fn destroy(&self) -> Result<(), LavalinkPlayerError> {
        self.node.rest.destroy_player(self.guild_id).await?;

        Ok(())
    }

    /// Pauses the player
    pub async fn pause(&self) -> Result<(), LavalinkPlayerError> {
        let data = self.get_data().await?;

        let mut options: LavalinkPlayerOptions = Default::default();

        let _ = options.paused.insert(!data.paused);

        self.send_update_player(false, options).await?;

        Ok(())
    }

    /// Changes the player volume
    pub async fn update_volume(&self, volume: u32) -> Result<(), LavalinkPlayerError> {
        let mut options: LavalinkPlayerOptions = Default::default();

        let _ = options.volume.insert(volume);

        self.send_update_player(false, options).await?;

        Ok(())
    }


    /// Seeks the player
    pub async fn update_position(&mut self, position: u32) -> Result<(), LavalinkPlayerError> {
        let mut options: LavalinkPlayerOptions = Default::default();

        let _ = options.position.insert(position);

        self.send_update_player(false, options).await?;

        Ok(())
    }

    /// Updates the playback filter of the player
    pub async fn update_filters(
        &self,
        mut filters: LavalinkFilters,
    ) -> Result<(), LavalinkPlayerError> {
        let data = self.get_data().await?;

        filters.merge(data.filters.clone());

        let mut options: LavalinkPlayerOptions = Default::default();

        let _ = options.filters.insert(filters);

        self.send_update_player(false, options).await?;

        Ok(())
    }

    /// Clears the filters applied in the player
    pub async fn clear_filters(&self) -> Result<(), LavalinkPlayerError> {
        let filters = Default::default();

        let mut options: LavalinkPlayerOptions = Default::default();

        let _ = options.filters.insert(filters);

        self.send_update_player(false, options).await?;

        Ok(())
    }

    /// Updates the connection info of the player
    pub async fn update_connection(
        &self,
        connection: ConnectionOptions,
    ) -> Result<(), LavalinkPlayerError> {
        let voice = LavalinkVoice {
            token: connection.token,
            endpoint: connection.endpoint,
            session_id: connection.session_id,
            connected: None,
            ping: None,
        };

        let mut options: LavalinkPlayerOptions = Default::default();

        let _ = options.voice.insert(voice);

        self.send_update_player(false, options).await?;

        Ok(())
    }

    /// Sends the updated player data to lavalink
    async fn send_update_player(
        &self,
        no_replace: bool,
        options: LavalinkPlayerOptions,
    ) -> Result<(), LavalinkPlayerError> {
        self.node
            .rest
            .update_player(self.guild_id, no_replace, options)
            .await?;

        Ok(())
    }
}
