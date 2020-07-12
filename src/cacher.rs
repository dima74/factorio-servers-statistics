use std::cmp::Reverse;
use std::sync::Arc;
use std::time::Duration;

use itertools::Itertools;
use parking_lot::RwLock;
use serde::Serialize;

use crate::state::{ServerId, State, StateLock, TimeMinutes};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopGameByNumberPlayersNow {
    pub server_id: ServerId,
    pub name: String,
    pub number_players: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopGameByNumberPlayersMax {
    pub server_id: ServerId,
    pub name: String,
    pub number_players: usize,
    pub time: TimeMinutes,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MainPageInfo {
    top_games_by_number_players_now: Vec<TopGameByNumberPlayersNow>,
    top_games_by_number_players_max: Vec<TopGameByNumberPlayersMax>,
}

pub struct CacherState {
    main_page: MainPageInfo,
    pub main_page_serialized: Arc<String>,
}

impl CacherState {
    pub fn new() -> Self {
        let main_page = MainPageInfo {
            top_games_by_number_players_now: Vec::new(),
            top_games_by_number_players_max: Vec::new(),
        };
        Self {
            main_page,
            main_page_serialized: Arc::new("{}".to_owned()),
        }
    }
}

pub type CacherStateLock = Arc<RwLock<CacherState>>;

const INTERVAL: Duration = Duration::from_secs(10 * 60);  // 10 minutes

/// переодически рассчитывает различные статистики (в основном для главной страницы) на основе State
pub fn cacher(cacher_state_lock: CacherStateLock, state_lock: StateLock) {
    for i in 0.. {
        println!("[info]  [cacher] start iteration #{}", i);

        update_top_games_by_number_players_current(&state_lock.read(), &cacher_state_lock);
        update_top_games_by_number_players_maximum(&state_lock.read(), &cacher_state_lock);

        {
            let mut cacher_state = cacher_state_lock.write();
            let main_page_serialized = serde_json::to_string(&cacher_state.main_page).unwrap();
            cacher_state.main_page_serialized = Arc::new(main_page_serialized);
        }

        std::thread::sleep(INTERVAL);
    }
    println!("[info]  [cacher] exit");
}

fn get_top_n<T, K>(mut values: Vec<T>, n: usize, get_key: impl Fn(&T) -> K) -> Vec<T>
    where K: Ord
{
    if values.len() > n {
        values.partition_at_index_by_key(n - 1, &get_key);
        values.truncate(n);
    }
    values.sort_by_key(get_key);
    values
}

fn update_top_games_by_number_players_current(state: &State, cacher_state_lock: &CacherStateLock) {
    const TOP_SIZE: usize = 10;

    let pairs = state.current_game_ids.iter()
        .map(|&game_id| state.get_game(game_id))
        .filter(|game| game.server_id.is_some())
        .map(|game| (game, game.number_players_online()))
        .collect();
    let pairs = get_top_n(pairs, TOP_SIZE, |(_, number_players)| Reverse(*number_players));

    let top_games = pairs.into_iter()
        .map(|(game, number_players)| TopGameByNumberPlayersNow {
            server_id: game.server_id.unwrap(),
            name: game.get_name(&state).to_owned(),
            number_players,
        })
        .collect();
    cacher_state_lock.write().main_page.top_games_by_number_players_now = top_games;
}

// todo топ-4 игры по сути имеют одинаковый server_id
//  разобраться почему он разны
fn update_top_games_by_number_players_maximum(state: &State, cacher_state_lock: &CacherStateLock) {
    const TOP_SIZE: usize = 10;

    let pairs = state.game_ids.iter().skip(1)
        .map(|&game_id| {
            let game = state.get_game(game_id);
            (game.server_id.unwrap(), game.maximum_number_players())
        })
        .into_group_map();
    let pairs = pairs.into_iter()
        .map(|(server_id, values)| (server_id, values.into_iter().max().unwrap()))
        .collect();
    let pairs = get_top_n(pairs, TOP_SIZE, |(_, number_players)| Reverse(*number_players));

    let top_games = pairs.into_iter()
        .map(|(server_id, (number_players, time))| TopGameByNumberPlayersMax {
            server_id,
            name: state.get_server_name(server_id).to_owned(),
            number_players,
            time,
        })
        .collect();
    cacher_state_lock.write().main_page.top_games_by_number_players_max = top_games;
}
