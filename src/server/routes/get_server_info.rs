use rocket::get;
use rocket_contrib::json::Json;
use serde::Serialize;

use fss::state::{GameId, ServerId, State, StateLock, TimeMinutes};
use fss::state;

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
    pub mod_count: u16,

    pub description: String,
    pub host_address: String,
    // pair is (name, version)
    pub mods: Option<Vec<(String, String)>>,
}

fn convert_game(game: &state::Game, state: &State, time_begin: TimeMinutes, time_end: TimeMinutes) -> Game {
    let players_intervals = game.players_intervals.iter()
        .filter(|interval| !(interval.end.unwrap_or(time_end) <= time_begin || time_end <= interval.begin))
        .map(|interval| {
            let player_name = state.all_player_names.get(interval.player_index);
            (player_name.into(), interval.begin, interval.end)
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
        description: state.all_game_descriptions.get(game.description).into(),
        max_players: game.max_players,
        game_version: state.all_versions.get(game.game_version).into(),
        game_time_elapsed: game.game_time_elapsed,
        has_password: game.has_password,
        tags: (state.all_tags.get(game.tags).into(): &str)
            .split("\n").to_owned().map(ToOwned::to_owned).collect(),
        mod_count: game.mod_count,
        host_address: state.all_host_addresses.get(game.host_address.unwrap()).into(),
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

#[get("/server/<server_id>?<time_begin>&<time_end>")]
//pub fn get_server_info(server_id: ServerId, state_lock: rocket::State<StateLock>) -> Option<Json<Server>> {
pub fn get_server_info(
    server_id: usize,
    time_begin: u32,
    time_end: u32,
    state_lock: rocket::State<StateLock>,
) -> Option<Json<Server>> {
    let state = state_lock.read();

    // todo update rocket
    let server_id: ServerId = state.as_server_id(server_id)?;
    let time_begin = TimeMinutes::new(time_begin)?;
    let time_end = TimeMinutes::new(time_end)?;

    let game_ids = state.get_server_games_in_interval(server_id, time_begin, time_end);
    let games = game_ids.into_iter()
        .map(|game_id| convert_game(state.get_game(game_id), &state, time_begin, time_end))
        .collect();

    // todo посмотреть на типичный размер json, кажется метаинформация занимает очень много
    Some(Json(Server { games }))
}
