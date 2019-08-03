use std::fs::File;
use std::sync::mpsc;

use crate::api;
use crate::fetcher_get_games::FetcherOutput;
use crate::state::TimeMinutes;

pub fn fetcher(sender: mpsc::Sender<FetcherOutput>, number_responses: u32) {
    assert!(number_responses <= 2880);
    for i in 0..number_responses {
        if i % 10 == 0 {
            println!("[info]  [fetcher_get_games_offline] iteration: {:4}", i);
        }
        let file = File::open(format!("temp/cache-get-games/{:04}.json", i)).unwrap();
        let games: Vec<api::Game> = serde_json::from_reader(file).unwrap();
        let mut games = games.into_iter().map(|game| game.into()).collect();
        api::clean_get_games_response(&mut games);

        let get_games_response = games;
        let response_time = TimeMinutes::new(1 + i).unwrap();
        sender.send((get_games_response, response_time)).unwrap()
    }
}
