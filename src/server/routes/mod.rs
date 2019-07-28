use std::collections::HashMap;

use rocket::get;
use rocket_contrib::json::Json;
use serde::Serialize;

use fss::state::{ServerId, StateLock};

pub mod get_server_info;

#[get("/")]
pub fn index() -> &'static str {
    "api works!"
}

#[derive(Serialize)]
pub struct SearchIndex {
    // todo
    // пока что содержит только сервера которые сейчас онлайн
    // ключи — имена последней Game для этого ServerId
    servers: HashMap<String, ServerId>,
}

#[get("/servers_search_index")]
pub fn servers_search_index(state_lock: rocket::State<StateLock>) -> Json<SearchIndex> {
    let state = state_lock.read();
    let servers = state.current_game_ids.iter().filter_map(|&game_id| {
        let game = state.get_game(game_id);
        let game_name = state.get_game_name(game_id).into();
        game.server_id.map(|server_id| (game_name, server_id))
    }).collect();
    Json(SearchIndex { servers })
}

