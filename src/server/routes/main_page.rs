use std::sync::Mutex;

use itertools::Itertools;
use lazy_static::lazy_static;
use rocket::{get, State};
use rocket::response::content;
use rocket_contrib::json::Json;
use serde::Serialize;

use fss::cacher::CacherStateLock;
use fss::state::{ServerId, StateLock, TimeMinutes};

use crate::server::routes::util::ArcResponder;

#[get("/main-page")]
pub fn main_page(cacher_state_lock: State<CacherStateLock>) -> content::Json<ArcResponder<String>> {
    let cacher_state = cacher_state_lock.read();
    content::Json(ArcResponder(cacher_state.main_page_serialized.clone()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSearchInfo {
    pub server_id: ServerId,
    pub name: String,
    pub time_begin: TimeMinutes,
    pub time_end: Option<TimeMinutes>,
}

// because search is resource-consuming
lazy_static! {
    static ref SEARCH_MUTEX: Mutex<()> = Mutex::new(());
}

#[get("/search-servers?<query>")]
pub fn search(query: String, state_lock: State<StateLock>) -> Json<Vec<GameSearchInfo>> {
    use regex::{escape, RegexBuilder};
    use std::cmp::Reverse;

    const NUMBER_GAMES: usize = 100;

    let _guard = SEARCH_MUTEX.lock().unwrap();
    let query_regex = RegexBuilder::new(&escape(&query))
        .case_insensitive(true)
        .build().unwrap();

    let state = state_lock.read();
    let games = state.game_ids.iter()
        .skip(1)
        .rev()  // select top NUMBER_GAMES among latest ones
        .filter(|&game_id| query_regex.is_match(state.get_game_name(*game_id)))
        .map(|&game_id| state.get_game(game_id).server_id.unwrap())
        .unique()
        .take(NUMBER_GAMES)
        .map(|server_id| {
            let last_game_id = state.get_server_last_game_id(server_id);
            let first_game_id = state.get_server_first_game_id(server_id);
            let last_game = state.get_game(last_game_id);
            let first_game = state.get_game(first_game_id);
            GameSearchInfo {
                server_id,
                name: last_game.get_name(&state).to_owned(),
                time_begin: first_game.time_begin,
                time_end: last_game.time_end,
            }
        })
        .sorted_by_key(|info| (info.time_end.is_some(), Reverse(info.time_end), info.time_begin))
        .collect();
    Json(games)
}
