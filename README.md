# Anchorage

> A stable wrapper (Tokio-Based) around Lavalink in Rust

> Why is this named Anchorage? She's cute that's it

<p align="center">
    <img src="https://yuki.suou.moe/Anchorage_CN_Without_BG-zvy6Q7GP.png"> 
</p>

> Artwork from Azur Lane

### Installing

* Command Line
```
cargo add --git https://github.com/Deivu/Anchorage.git
```

* Add it to `Cargo.toml`
```
anchorage = { git = "https://github.com/Deivu/Anchorage.git", version = "0.1.0" }
```

### Documentation

> https://deivu.github.io/Anchorage/anchorage/index.html

### Examples

* Starting the library

```rs
use anchorage::Anchorage;
use anchorage::model::player::{DataType, EventType, LavalinkVoice, LavalinkPlayerOptions, PlayerEvents};
use anchorage::model::anchorage::{Options, NodeOptions, ConnectionOptions};

/// supplying none on these options defaults it to it's default value
let anchorage = Anchorage::new(Options {
    user_agent: None,
    reconnect_tries: None,
    request: None,
});

let nodes = vec![NodeOptions { 
    name: "Anchorage",
    host: "127.0.0.1",
    port: 8080,
    auth: "password_you_want",
}];

let user_id: u64 = 424137718961012737;

anchorage
    .start(user_id, nodes)
    .await
    .unwrap();

/// now you can use anchorage as you wish
```

* Joining a voice channel. This is dependent on whatever library you are using (if you are using one), Just ensure that you fill up [`ConnectionOptions`] properly with the data you received from your Discord gateway
```rs
/// assuming we are using the (anchorage) instance above

/// guild id of the guild where the bot will join the voice channel
let guild_id: u64 = 423116740810244097;

/// voice data you received from your gateway
let connection = ConnectionOptions {
    channel_id: Some(564749582744027156),
    endpoint: "https://discord.com/some_voice_endpoint",
    guild_id,
    session_id: "some_session_id_from_discord",
    token: "some_token_from_disord",
    user_id: 424137718961012737,
};

/// shortcut to get an ideal node to connect to
let node = anchorage.get_ideal_node()
    .await
    .unwrap();

/// creates a new player where you can communicate with lavalink and receive events via a message channel
let (player, events) = anchorage.create_player(guild_id, node, connection)
    .await
    .unwrap();

/// handle player and events as you wish
```

* Playing a track and handling events
```rs
/// assuming that we are using the using the (node, player, events) instance from above

/// tries to resolve a track that returns DataType enum
let result = node.rest.resolve("https://www.youtube.com/watch?v=KheS1qj4fyk")
    .await
    .unwrap();


/// match the enum depending on the result
let tracks = match result {
    DataType::Track(track) => vec![track],
    _ => vec![]
};

if tracks.len() == 0 {
    return;
}

/// spawn a new green thread to handle your player events so you wont block the current thread you are on
tokio::spawn(async move {
    /// event has two types, EventType::Player and EventType::Destroyed
    /// the EventType::Player is events from lavalink itself
    /// the EventType::Destroyed is event from a node when it's destroyed
    let event = events.recv_async().await.unwrap();
    
    match event {
        EventType::Player(player_events) => {
            match player_events {
                PlayerEvents::TrackStartEvent(_) => {
                    /// do something
                }
                _ => {
                    /// additional player events
                }
            }
            
        }
        EventType::Destroyed => {
            /// When this player node is destroyed
            /// It's up to developers implementation on how they handle this
            /// Cleanup, or move the player you have, up to you
        }
    }
});

/// play the resolved track
player.play(tracks[0].encoded.clone()).await.unwrap();
```

* Handling voice server changes (Channel moves & Channel voice server changes)
```rs
/// partial Discord gateway packet for voice state update
pub struct VoiceStatePartial {
	channel_id?: Option<String>,
	session_id: String,
	self_deaf: bool,
	self_mute: bool,
}

/// discord gateway packet for voice server update
pub struct VoiceServerUpdateEvent {
    pub token: String,
    pub guild_id: Option<u64>,
    pub endpoint: Option<String>,
}

async handle(state: VoiceStatePartial, server: VoiceServerUpdateEvent) {
    let id = server.guild_id.unwrap().clone();
    let node = anchorage.get_node_for_player(id).await.unwrap();
    
    let voice = LavalinkVoice {
        token: server.token,
        endpoint: server.endpoint,
        session_id: state.session_id,
        connected: None,
        ping: None,
    };
    
   let mut options: LavalinkPlayerOptions = Default::default();

   let _ = options.voice.insert(voice);
   
   node.rest
       .update_player(id, false, options)
       .await
       .unwrap();
}
```

### Notes
* As you noticed, I used `.unwrap()` on most of these calls, but you'd want to handle the errors properly
* Anchorage is async, most of her calls won't block your current thread
* Anchorage only handles the websocket handling, and provides an interface of REST for your ease of usage, she does not move nodes automatically on disconnect nor handle any voice related activity, like other library (Shoukaku) does, it's up to developers to implement this. (see above examples)

### Other Links

- [Discord](https://discord.gg/FVqbtGu)

- [Lavalink](https://github.com/lavalink-devs/Lavalink)

> Code made with ❤ by @ichimakase (Saya) | The Shipgirl Project