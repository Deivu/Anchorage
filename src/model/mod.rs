use serde::Deserialize;

/// Contains the errors the library is using
pub mod error;
/// Contains various structure data for lavalink node
pub mod node;
/// Contains various structure data for lavalink player
pub mod player;
/// Contains various structure data for anchorage use
pub mod anchorage;

fn str_to_u64<'de, T, D>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    String::deserialize(de)?
        .parse()
        .map_err(serde::de::Error::custom)
}
