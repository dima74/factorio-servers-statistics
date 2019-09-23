use std::{env, fmt, fs};
use std::error::Error;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use reqwest::StatusCode;
use serde::{de, Deserialize, Serialize};

use crate::global_config::GLOBAL_CONFIG;

pub type GetGamesResponse = Vec<Game>;
pub type GetGameDetailsResponse = Game;

const MOCK_API: bool = false;

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ApplicationVersion {
    pub game_version: String,
    #[serde(deserialize_with = "convert_from_string")]
    pub build_version: u32,
    pub build_mode: String,
    pub platform: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mod {
    pub name: String,
    pub version: String,
}

// более правильным было бы назвать этот класс GameSnapshot
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Game {
    // common fields
    pub game_id: NonZeroU32,
    pub name: String,
    pub description: String,
    #[serde(deserialize_with = "convert_from_string")]
    pub max_players: u32,
    #[serde(default)]  // omitted if empty
    pub players: Vec<String>,
    pub application_version: ApplicationVersion,
    #[serde(deserialize_with = "convert_from_string")]
    // in minutes
    pub game_time_elapsed: u32,
    #[serde(deserialize_with = "convert_from_string")]
    pub has_password: bool,
    #[serde(rename = "server_id")]
    pub host_id: Option<String>,
    #[serde(default)]  // omitted if empty
    pub tags: Vec<String>,

    // /get-games only fields
    pub has_mods: Option<bool>,
    pub mod_count: Option<u16>,

    // /get-game-details only fields
    // unix time (seconds since epoch)
    pub last_heartbeat: Option<f64>,
    pub host_address: Option<String>,
    pub mods: Option<Vec<Mod>>,
    pub mods_crc: Option<u64>,
    pub steam_id: Option<String>,
    pub require_user_verification: Option<String>,
}

pub fn convert_from_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
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

fn check_response(game: &Game, is_get_games_response: bool) {
    assert_eq!(game.has_mods.is_some(), is_get_games_response);
    assert_eq!(game.mod_count.is_some(), is_get_games_response);

    assert_eq!(game.last_heartbeat.is_none(), is_get_games_response);
    assert_eq!(game.host_address.is_none(), is_get_games_response);
    assert_eq!(game.mods.is_none(), is_get_games_response);
    assert_eq!(game.mods_crc.is_none(), is_get_games_response);
    if is_get_games_response {
        assert!(game.steam_id.is_none());
    }
}

const API_BASE_URL: &str = "https://multiplayer.factorio.com";

fn reqwest_get(url: &str) -> Result<String, Box<dyn Error>> {
    let mut response = reqwest::get(url)?;
    let response_text = response.text()?;
    if !response.status().is_success() {
        eprintln!("[error] [api] request failed: response text is `{}`", response_text);
    }
    response.error_for_status()?;
    Ok(response_text)
}

fn reqwest_get_with_retries(url: &str, number_retries: usize) -> Result<String, Box<dyn Error>> {
    assert!(number_retries >= 1);
    for request_index in 0..number_retries {
        match reqwest_get(url) {
            response @ Ok(_) => return response,
            Err(response) => {
                eprintln!("[error] [api] request failed (retry_index = {}):\n\turl: {}\n\terror message: {}", request_index, url, response);
                std::thread::sleep(Duration::from_secs(f32::powf(1.5, request_index as f32) as u64));

                if request_index + 1 == number_retries {
                    return Err(response);
                }
            }
        }
    }
    unreachable!()
}

pub fn get_games() -> GetGamesResponse {
    let factorio_username: String = env::var("FACTORIO_USERNAME")
        .expect("Missing FACTORIO_USERNAME env variable");
    let factorio_token: String = env::var("FACTORIO_TOKEN")
        .expect("Missing FACTORIO_TOKEN env variable");
    let api_url: String = format!("{}/get-games?username={}&token={}", API_BASE_URL, factorio_username, factorio_token);

    let response = if !MOCK_API {
        reqwest_get_with_retries(&api_url, 10).unwrap()
    } else {
        fs::read_to_string("temp/cached-data/get-games.json").unwrap()
    };
    let mut games: Vec<Game> = serde_json::from_str(&response).unwrap();
    clean_get_games_response(&mut games);
    for game in games.iter() {
        check_response(game, true);
    }
    games
}

pub fn clean_get_games_response(games: &mut Vec<Game>) {
    for game in games.iter_mut() {
        game.players.retain(|player_name| !player_name.is_empty());
    }
    games.retain(|game| game.host_id.is_some() && game.application_version.game_version != "0.0.0");
}

// Ok(None) означает ошибку 404
fn reqwest_get_and_check_for_404(url: &str) -> Result<Option<String>, Box<dyn Error>> {
    let response = reqwest::get(url)?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let mut response = response.error_for_status()?;
    let response_text = response.text()?;
    Ok(Some(response_text))
}

// Ok(None) означает что api вернул 404 первый раз
pub fn get_game_details(game_id: u64) -> Result<Option<GetGameDetailsResponse>, Box<dyn Error>> {
    let api_url: String = format!("{}/get-game-details/{}", API_BASE_URL, game_id);

    // todo: && cfg!(not(debug_assertions))
    let response = if !GLOBAL_CONFIG.lock().unwrap().use_cache_for_get_game_details {
        reqwest_get_and_check_for_404(&api_url)
    } else {
        get_game_details_cached(game_id, &api_url)
    };
    let response = match response {
        Ok(None) => return Ok(None),
        Ok(Some(response)) => response,
        Err(_) => reqwest_get_with_retries(&api_url, 4)?,
    };

    let mut game: Game = serde_json::from_str(&response).unwrap();
    check_response(&game, false);
    game.mods.as_mut().unwrap().retain(|mod_| mod_.name != "base");
    Ok(Some(game))
}

pub fn get_game_details_cached(game_id: u64, api_url: &str) -> Result<Option<String>, Box<dyn Error>> {
    let path = PathBuf::from(format!("temp/cache-get-game-details/{}.json", game_id));
    if path.exists() {
        let response = fs::read_to_string(path).unwrap();
        if response.is_empty() {
            Ok(None)
        } else {
            Ok(Some(response))
        }
    } else {
        let response = reqwest_get_and_check_for_404(&api_url);
        if let Ok(ref response) = response {
            let content = response.as_deref().unwrap_or("");
            fs::write(path, content).unwrap();
        }
        response
    }
}
