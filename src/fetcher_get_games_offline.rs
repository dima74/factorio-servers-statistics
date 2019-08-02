use std::fs::File;
use std::num::NonZeroU32;
use std::sync::mpsc;

use serde::{Deserialize, Serialize};

use crate::api::{ApplicationVersion, convert_from_string, Mod};
use crate::api;
use crate::fetcher_get_games::FetcherOutput;
use crate::state::TimeMinutes;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    pub game_id: NonZeroU32,
    pub name: String,
    #[serde(deserialize_with = "convert_from_string")]
    pub max_players: u32,
    #[serde(default)]
    pub players: Vec<String>,
    pub application_version: ApplicationVersion,
    #[serde(deserialize_with = "convert_from_string")]
    pub game_time_elapsed: u32,
    #[serde(deserialize_with = "convert_from_string")]
    pub has_password: bool,
    #[serde(rename = "server_id")]
    pub host_id: Option<String>,
    #[serde(default)]  // omitted if empty
    pub tags: Vec<String>,
    pub last_heartbeat: f64,
    pub has_mods: bool,
    pub mod_count: u16,
}

impl Into<api::Game> for Game {
    fn into(self) -> api::Game {
        api::Game {
            game_id: self.game_id,
            name: self.name,
            description: "".to_owned(),
            max_players: self.max_players,
            players: self.players,
            application_version: self.application_version,
            game_time_elapsed: self.game_time_elapsed,
            has_password: self.has_password,
            host_id: self.host_id,
            tags: self.tags,
            has_mods: Some(self.has_mods),
            mod_count: Some(self.mod_count),
            last_heartbeat: Some(self.last_heartbeat),
            host_address: None,
            mods: None,
            mods_crc: None,
            steam_id: None,
            require_user_verification: None,
        }
    }
}

pub fn fetcher(sender: mpsc::Sender<FetcherOutput>, number_responses: u32) {
    assert!(number_responses <= 2880);
    for i in 0..number_responses {
        if i % 10 == 0 {
            println!("{:4}", i);
        }
        let file = File::open(format!("analytics/data/{:04}.json", i)).unwrap();
        let games: Vec<Game> = serde_json::from_reader(file).unwrap();
        let mut games = games.into_iter().map(|game| game.into()).collect();
        api::clean_get_games_response(&mut games);

        let get_games_response = games;
        let response_time = TimeMinutes::new(1 + i).unwrap();
        sender.send((get_games_response, response_time)).unwrap()
    }
}
