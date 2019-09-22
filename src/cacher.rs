use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use serde::Serialize;

use crate::state::{ServerId, State, StateLock};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopCurrentGameByNumberPlayers {
    pub server_id: ServerId,
    pub name: String,
    pub number_players: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MainPageInfo {
    top_current_games_by_number_players: Vec<TopCurrentGameByNumberPlayers>,
    // todo: пока что содержит только сервера которые сейчас онлайн
    // ключи — имена последней Game для этого ServerId
    search_index: HashMap<String, ServerId>,
}

pub struct CacherState {
    main_page: MainPageInfo,
    pub main_page_serialized: Arc<String>,
}

impl CacherState {
    pub fn new() -> Self {
        let main_page = MainPageInfo {
            top_current_games_by_number_players: Vec::new(),
            search_index: HashMap::new(),
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

        update_current_top_games_by_number_players(&state_lock.read(), &cacher_state_lock);
        update_search_index(&state_lock.read(), &cacher_state_lock);

        {
            let mut cacher_state = cacher_state_lock.write();
            let main_page_serialized = serde_json::to_string(&cacher_state.main_page).unwrap();
            cacher_state.main_page_serialized = Arc::new(main_page_serialized);
        }

        std::thread::sleep(INTERVAL);
    }
    println!("[info]  [cacher] exit");
}

fn update_current_top_games_by_number_players(state: &State, cacher_state_lock: &CacherStateLock) {
    const TOP_SIZE: usize = 10;

    let mut pairs: Vec<_> = state.current_game_ids.iter()
        .map(|&game_id| state.get_game(game_id))
        .filter(|game| game.server_id.is_some())
        .map(|game| (game, game.number_players() as u32))
        .collect();
    if pairs.len() > TOP_SIZE {
        pairs.partition_at_index_by_key(TOP_SIZE - 1, |(_, number_players)| Reverse(*number_players));
        pairs.truncate(TOP_SIZE);
    }
    pairs.sort_by_key(|(_, number_players)| Reverse(*number_players));

    let top_games = pairs.into_iter()
        .map(|(game, number_players)| TopCurrentGameByNumberPlayers {
            server_id: game.server_id.unwrap(),
            name: state.get_game_name(game.game_id).into(),
            number_players,
        })
        .collect();
    cacher_state_lock.write().main_page.top_current_games_by_number_players = top_games;
}

fn update_search_index(state: &State, cacher_state_lock: &CacherStateLock) {
    let search_index = state.current_game_ids.iter()
        .filter_map(|&game_id| {
            let game = state.get_game(game_id);
            let game_name = state.get_game_name(game_id).into();
            game.server_id.map(|server_id| (game_name, server_id))
        })
        .collect();
    cacher_state_lock.write().main_page.search_index = search_index;
}
