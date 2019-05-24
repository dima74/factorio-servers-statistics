use std::{fmt, fs, env};
use std::error::Error;
use std::str::FromStr;

use serde::{de, Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplicationVersion {
    game_version: String,
    #[serde(deserialize_with = "convert_to_string")]
    build_version: u32,
    build_mode: String,
    platform: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Server {
    game_id: u64,
    name: String,
    #[serde(deserialize_with = "convert_to_string")]
    max_players: u32,
    #[serde(default)]
    players: Vec<String>,
    application_version: ApplicationVersion,
    #[serde(deserialize_with = "convert_to_string")]
    game_time_elapsed: u64,
    #[serde(deserialize_with = "convert_to_string")]
    has_password: bool,
    server_id: Option<String>,
    tags: Option<Vec<String>>,
    last_heartbeat: f64,
    has_mods: bool,
    mod_count: u32,
}

fn convert_to_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where T: FromStr,
          T::Err: fmt::Display,
          D: de::Deserializer<'de>
{
    let v: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
    let s = match v {
        // иначе при вызове .to_string() получим строку обёрнутую в кавычки
        serde_json::Value::String(s) => s,
        _ => v.to_string(),
    };
    T::from_str(&s).map_err(de::Error::custom)
}

pub fn is_development() -> bool {
    env::var("IS_DEVELOPMENT").is_ok()
}

fn main() -> Result<(), Box<dyn Error>> {
    let response = if !is_development() {
        dbg!(env::vars().collect::<Vec<(String, String)>>());
        let FACTORIO_USERNAME: String = env::var("FACTORIO_USERNAME").unwrap();
        let FACTORIO_TOKEN: String = env::var("FACTORIO_TOKEN").unwrap();
        let GET_GAMES_API: String = format!("https://multiplayer.factorio.com/get-games?username={}&token={}", FACTORIO_USERNAME, FACTORIO_TOKEN);
        reqwest::get(&GET_GAMES_API)?.text()?
    } else {
        fs::read_to_string("temp/response.json")?
    };

    let servers: Vec<Server> = serde_json::from_str(&response)?;
    dbg!(servers.len());

    Ok(())
}
