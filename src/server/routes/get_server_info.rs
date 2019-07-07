use rocket::get;
use rocket_contrib::json::Json;
use serde::Serialize;

use fss::state::{GameId, ServerId, State, StateLock, TimeMinutes};
use fss::state;
use std::num::NonZeroU32;

#[derive(Serialize)]
pub struct Server {
    games: Vec<Game>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub game_id: GameId,
    pub server_id: ServerId,
    pub prev_game_id: Option<GameId>,
    pub next_game_id: Option<GameId>,
    pub time_begin: TimeMinutes,
    pub time_end: Option<TimeMinutes>,

    // (player, time_begin, time_end)
    pub players_intervals: Vec<(String, TimeMinutes, Option<TimeMinutes>)>,

    pub host_id: String,
    pub name: String,
    pub max_players: u32,
    pub game_version: String,
    pub game_time_elapsed: u32,
    pub has_password: bool,
    pub tags: Vec<String>,
    pub last_heartbeat: f64,
    pub mod_count: u16,

    pub description: String,
    pub host_address: String,
    // pair is (name, version)
    pub mods: Option<Vec<(String, String)>>,
}

fn convert_game(game: &state::Game, state: &State) -> Game {
    let players_intervals = game.players_intervals.iter()
        .map(|interval| {
            let player_name = state.all_player_names.get(interval.player_index);
            (player_name.into(), interval.start, interval.end)
        }).collect();

    Game {
        game_id: game.game_id,
        server_id: game.server_id.unwrap(),
        prev_game_id: game.prev_game_id,
        next_game_id: game.next_game_id,
        time_begin: game.time_begin,
        time_end: game.time_end,
        players_intervals,
        // todo не объединять game_id, пока не отправили запрос на /get-game-details
//        host_id: String::from_utf8(game.host_id.to_vec()).unwrap(),
        host_id: "todo".to_owned(),
        name: state.all_game_names.get(game.name).into(),
        max_players: game.max_players,
        game_version: state.all_versions.get(game.game_version).into(),
        game_time_elapsed: game.game_time_elapsed,
        has_password: game.has_password,
//        tags: state.all_tags.get(game.tags).into().split("\n"),
        // todo
        tags: vec![],
        last_heartbeat: game.last_heartbeat,
        mod_count: game.mod_count,
//        description: state.all_game_descriptions.get(game.description.unwrap()).into(),
//        host_address: state.all_host_addresses.get(game.host_address.unwrap()).into(),
        description: "todo".to_owned(),
        host_address: "todo".to_owned(),
        // todo
        mods: None,
    }
}

//impl<'a> FromParam<'a> for NonZeroU32 {
//    type Error = &'a RawStr;
//
//    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
//        NonZeroU32::from_str(param.as_str()).map_err(|_| param)
//    }
//}

#[get("/server/<id>")]
//pub fn get_server_info(id: ServerId, state_lock: rocket::State<StateLock>) -> Option<Json<Server>> {
pub fn get_server_info(id: u32, state_lock: rocket::State<StateLock>) -> Option<Json<Server>> {
    //todo update rocket
    let id: ServerId = NonZeroU32::new(id).unwrap();

    let state = state_lock.read();
    let game_id = state.get_server_last_game_id(id)?;
    let mut games = vec![state.get_game(game_id).clone()];
    while let Some(game_id) = games.last().unwrap().prev_game_id {
        let game = state.get_game(game_id);
        if TimeMinutes::now().get() - game.time_end.unwrap().get() > TimeMinutes::WEEK {
            break;
        }
        games.push(game.clone());
    }

    let games = games.iter()
        .map(|game| convert_game(game, &state))
        .collect();

//    games.reverse();
    // todo посмотреть на типичный размер json, кажется метаинформация занимает очень много
    Some(Json(Server { games }))
}
