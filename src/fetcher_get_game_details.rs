use std::collections::VecDeque;
use std::sync::{Arc, mpsc};
use std::sync::mpsc::TryRecvError;

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::api;
use crate::state::{GameId, Mod, StateLock};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub game_ids: VecDeque<GameId>,
}

pub fn fetcher(receiver: mpsc::Receiver<GameId>, fetcher_state_lock: Arc<RwLock<State>>, state_lock: StateLock) {
    for iteration in 0.. {
        // если game_ids пусто, то блокирующе получаем один game_id из mpsc
        // если game_ids не пусто, то неблокирующе получаем все game_id из mpsc, затем обрабатываем один /get-game-details
        let number_game_ids = fetcher_state_lock.read().game_ids.len();
        if number_game_ids == 0 {
            let game_id = receiver.recv().unwrap();
            fetcher_state_lock.write().game_ids.push_back(game_id);
        } else {
            loop {
                match receiver.try_recv() {
                    Ok(game_id) => fetcher_state_lock.write().game_ids.push_back(game_id),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => panic!("[fetcher_get_game_details] channel disconnected"),
                }
            }

            fetch_one_game_details(&fetcher_state_lock, &state_lock);

            if number_game_ids > 100 && iteration % 200 == 0 {
                eprintln!("[fetcher_get_game_details] warning: number game_ids to fetch is too big: {}", number_game_ids);
            }
        }
    }
    println!("[fetcher_get_game_details] exit");
}

fn fetch_one_game_details(fetcher_state_lock: &Arc<RwLock<State>>, state_lock: &StateLock) {
    let game_id: GameId = *fetcher_state_lock.read().game_ids.front().unwrap();
    println!("[fetcher_get_game_details] fetch game_id {} at {}", game_id, Utc::now());
    let game_snapshot = api::get_game_details(game_id.get() as u64);

    match game_snapshot {
        Err(_) => eprintln!("[fetcher_get_game_details] error: failed to fetch /get-game-details for game_id {}", game_id),
        Ok(game_snapshot) => {
            let mut fetcher_state = fetcher_state_lock.write();
            let mut state = state_lock.write();

            let game_description = state.all_game_descriptions.add(&game_snapshot.description.unwrap());
            let game_host_address = state.all_host_addresses.add(&game_snapshot.host_address.unwrap());
            let mods = game_snapshot.mods.unwrap().iter().map(|mod_| {
                let name = state.all_mod_names.add(&mod_.name);
                let version = state.all_versions.add(&mod_.version);
                Mod { name, version }
            }).collect();

            let game = state.get_game_mut(game_snapshot.game_id);
            game.description = Some(game_description);
            game.host_address = Some(game_host_address);
            game.mods = Some(mods);

            fetcher_state.game_ids.pop_front();
        }
    }
}
