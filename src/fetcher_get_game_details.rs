use std::collections::VecDeque;
use std::sync::{Arc, mpsc};
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::api;
use crate::state::{GameId, Mod, StateLock, TimeMinutes};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub game_ids: VecDeque<GameId>,
}

pub fn fetcher(receiver: mpsc::Receiver<GameId>, fetcher_state_lock: Arc<RwLock<State>>, state_lock: StateLock) {
    for iteration in 0.. {
        thread::sleep(Duration::SECOND);

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
                    Err(TryRecvError::Disconnected) => panic!("[error] [fetcher_get_game_details] channel disconnected"),
                }
            }

            fetch_one_game_details(&fetcher_state_lock, &state_lock);

            if number_game_ids > 100 && iteration % 200 == 1 {
                eprintln!("[warn]  [fetcher_get_game_details] number game_ids to fetch is too big: {}", number_game_ids);
            }
        }
    }
    println!("[info]  [fetcher_get_game_details] exit");
}

fn fetch_one_game_details(fetcher_state_lock: &Arc<RwLock<State>>, state_lock: &StateLock) {
    let (game_id, number_game_ids): (GameId, usize) = {
        let game_ids = &fetcher_state_lock.read().game_ids;
        (*game_ids.front().unwrap(), game_ids.len())
    };
    println!("[info]  [fetcher_get_game_details] fetch game_id {:8} at {}    (game ids queue length = {})",
             game_id, Utc::now(), number_game_ids);
    let game_snapshot = api::get_game_details(game_id.get() as u64);

    match game_snapshot {
        Err(_) => eprintln!("[error] [fetcher_get_game_details] failed to fetch /get-game-details for game_id {}", game_id),
        Ok(game_snapshot) => {
            let mut fetcher_state = fetcher_state_lock.write();
            let mut state = state_lock.write();

            let (host_address, mods) = match game_snapshot {
                Some(game_snapshot) => (
                    game_snapshot.host_address.unwrap(),
                    game_snapshot.mods.unwrap(),
                ),
                None => (
                    // todo подумать действительно ли это хороший план
                    "unknown".to_owned(),
                    vec![api::Mod { name: "unknown".to_owned(), version: "unknown".to_owned() }]
                ),
            };

            let game_host_address = state.all_host_addresses.add(&host_address);
            let mods = mods.iter().map(|mod_| {
                let name = state.all_mod_names.add(&mod_.name);
                let version = state.all_versions.add(&mod_.version);
                Mod { name, version }
            }).collect();

            let game = state.get_game_mut(game_id);
            game.host_address = Some(game_host_address);
            game.mods = Some(mods);

            fetcher_state.game_ids.pop_front();
        }
    }
}
