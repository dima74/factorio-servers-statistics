use std::num::NonZeroU32;
use std::sync::{Arc, mpsc};
use std::thread;

use parking_lot::RwLock;

use crate::{api, external_storage, state};
use crate::state::{GameId, Mod, StateLock, TimeMinutes};

pub fn fetcher_get_game_details(receiver: mpsc::Receiver<GameId>, state_lock: StateLock) {
    let mut state = state_lock.write();
    let host_address = state.all_host_addresses.add("fake_host_address");
    let mod_name = state.all_mod_names.add("fake_mod_name");
    let mod_version = state.all_versions.add("fake_mod_version");
    drop(state);

    for game_id in receiver {
        let mut state = state_lock.write();
        let game = state.get_game_mut(game_id);
        game.host_address = Some(host_address);
        game.mods = Some(vec![Mod { name: mod_name, version: mod_version }]);
    }
}

fn prepare_games(games: Vec<(u8, u32) /* host_id, game_id */>) -> Vec<api::Game> {
    games.into_iter()
        .map(|(host_index, game_id)| {
            let mut host_id = [0u8; 32];
            host_id[0] = host_index;

            api::Game {
                game_id: NonZeroU32::new(game_id).unwrap(),
                host_id: Some(base64::encode(&host_id)),
                players: vec![],

                name: "fake".to_string(),
                description: "fake".to_string(),
                max_players: 0,
                application_version: api::ApplicationVersion {
                    game_version: "fake".to_string(),
                    build_version: 0,
                    build_mode: "fake".to_string(),
                    platform: "fake".to_string(),
                },
                game_time_elapsed: 0,
                has_password: false,
                tags: vec![],
                has_mods: None,
                mod_count: None,
                last_heartbeat: None,
                host_address: None,
                mods: None,
                mods_crc: None,
                steam_id: None,
                require_user_verification: None,
            }
        })
        .collect()
}

#[test]
fn merge() {
    let (sender_fetcher_get_games, receiver_fetcher_get_games) = mpsc::channel();
    let (sender_fetcher_get_game_details, receiver_fetcher_get_game_details) = mpsc::channel();

    let whole_state = external_storage::get_empty_state();
    let updater_state_lock = Arc::new(RwLock::new(whole_state.updater_state));
    let state_lock = Arc::new(RwLock::new(whole_state.state));

    // fetcher_get_game_details
    {
        let state_lock = state_lock.clone();
        thread::spawn(move || fetcher_get_game_details(receiver_fetcher_get_game_details, state_lock));
    }

    // updater
    let updater_thread = {
        let state_lock = state_lock.clone();
        let updater_state_lock = updater_state_lock.clone();
        thread::spawn(move || state::updater::updater(updater_state_lock, state_lock, receiver_fetcher_get_games, sender_fetcher_get_game_details))
    };

    // (host_id, game_id)
    let mut games = Vec::new();
    for _ in 0..10 { games.push(vec![(1, 1)]); }
    games.push(vec![(1, 1), (1, 2)]);
    for _ in 0..10 { games.push(vec![(1, 2)]); }
    for _ in 0..10 { games.push(vec![]); }
    for (time, games) in games.into_iter().enumerate() {
        let games = prepare_games(games);
        let time = TimeMinutes::new(time as u32 + 1).unwrap();
        sender_fetcher_get_games.send((games, time)).unwrap();
    }
    drop(sender_fetcher_get_games);

    updater_thread.join().unwrap();

    let state = state_lock.read();
    assert_eq!(state.games.len(), 2);
    let game2 = state.get_game(NonZeroU32::new(2).unwrap());
    assert_eq!(game2.prev_game_id, Some(NonZeroU32::new(1).unwrap()));
}
