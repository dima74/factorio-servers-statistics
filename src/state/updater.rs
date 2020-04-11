use std::convert::TryInto;
use std::iter::FromIterator;
use std::ops::Deref;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use hashbrown::{HashMap, HashSet};
use itertools::Itertools;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::api;
use crate::fetcher_get_games::FetcherOutput;
use crate::global_config::GLOBAL_CONFIG;
use crate::state::{Game, GameId, HostId, PlayerInterval, State, StateLock, TimeMinutes};

//impl From<api::Mod> for Mod {
//    fn from(v: api::Mod) -> Self {
//        Self { name: v.name, version: v.version }
//    }
//}
//
//impl From<api::ApplicationVersion> for ApplicationVersion {
//    fn from(v: api::ApplicationVersion) -> Self {
//        Self {
//            game_version: v.game_version,
//            build_version: v.build_version,
//            build_mode: v.build_mode,
//            platform: v.platform,
//        }
//    }
//}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct UpdaterState {
    pub scheduled_to_merge_host_ids: HashMap<HostId, HostIdMergeInfo>,
}

pub const HOST_ID_MERGE_DELAY: u32 = 20;  // in minutes

fn convert_snapshot_to_game(game_snapshot: &api::Game, state: &mut State, time: TimeMinutes) -> Game {
    // host_id.unwrap() можно делать, потому что в api::get_games игры без host_id удаляются
    let host_id = base64::decode(game_snapshot.host_id.as_ref().unwrap())
        .expect("failed to decode host_id as base64");
    assert_eq!(host_id.len(), 32);
    let host_id = host_id.deref().try_into().clone().unwrap();

    let tags = game_snapshot.tags.iter()
        .map(|tag| tag.replace('\x02', "\x01"))
        .join("\x02");

    let players_intervals = game_snapshot.players.iter()
        .map(|player| {
            let player_id = state.all_player_names.add(player);
            PlayerInterval::new(player_id, time)
        }).collect();

    Game {
        game_id: game_snapshot.game_id,
        server_id: None,
        prev_game_id: None,
        next_game_id: None,
        time_begin: time,
        time_end: None,
        players_intervals,
        host_id,
        name: state.all_game_names.add(&game_snapshot.name),
        description: state.all_game_descriptions.add(&game_snapshot.description),
        max_players: game_snapshot.max_players,
        game_version: state.all_versions.add(&game_snapshot.application_version.game_version),
        game_time_elapsed: game_snapshot.game_time_elapsed,
        has_password: game_snapshot.has_password,
        tags: state.all_tags.add(&tags),
        mod_count: game_snapshot.mod_count.unwrap_or(0),
        host_address: None,
        mods: None,
    }
}

#[allow(warnings)]
fn check_game_match_snapshot(state: &State, game: &Game, game_snapshot: &api::Game) {
    // todo
//    check_field_equal(game_prev.host_id, game.host_id, "host_id");
//    check_field_equal(game_prev., game., "");
    // name max_players game_version
}

fn update_game(game_snapshot: &api::Game, state: &mut State, time: TimeMinutes) {
//    check_match_metainfo(&mut state, game, &game_snapshot);

    let game = state.games.get_mut(&game_snapshot.game_id).unwrap();
    game.game_time_elapsed = game_snapshot.game_time_elapsed;

    // все игроки, которые были онлайн, будут в конце game.players_intervals
    // находим тех из них, которые уже не онлайн, обновляем player_interval.end и перемещаем левее
    // новых игроков добавляем в конец

    // todo заменить FssString на String?

    let mut player_names: HashSet<Vec<u8>> = game_snapshot.players.iter()
        .map(|player| player.to_owned().into_bytes()).collect();
    let mut first_online_player_index = game.players_intervals.iter()
        .rposition(|player_interval| player_interval.end.is_some())
        .map(|index| index + 1)
        .unwrap_or(0);
    for i in first_online_player_index..game.players_intervals.len() {
        let player_interval = &mut game.players_intervals[i];
        let player_name = state.all_player_names.get(player_interval.player_index);
        let player_name = player_name.0;
        if player_names.contains(player_name) {
            player_names.remove(player_name);
        } else {
            player_interval.end = Some(time);
            game.players_intervals.swap(i, first_online_player_index);
            first_online_player_index += 1;
        }
    }

    // в player_names остались только новые игроки
    for player_name in player_names {
        let player_index = state.all_player_names.add_vec(&player_name);
        let player_interval = PlayerInterval::new(player_index, time);
        game.players_intervals.push(player_interval);
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct HostIdMergeInfo {
    // время первого события появлении/исчезновении game_id
    pub time_begin: TimeMinutes,
    // время последнего события появлении/исчезновении game_id
    pub time_end: TimeMinutes,
    // список game_ids сразу перед time_begin
    pub game_ids: Vec<GameId>,
}

fn group_game_ids_by_host<'a>(game_ids: impl IntoIterator<Item=&'a GameId>, state: &State) -> HashMap<HostId, Vec<GameId>> {
    let mut game_ids_by_host: HashMap<HostId, Vec<GameId>> = HashMap::new();
    for &game_id in game_ids {
        let game = state.get_game(game_id);
        if let Some(game_ids) = game_ids_by_host.get_mut(&game.host_id) {
            game_ids.push(game.game_id);
        } else {
            game_ids_by_host.insert(game.host_id, vec![game.game_id]);
        }
    }
    game_ids_by_host
}

fn merge_games(curr_game_id: GameId, prev_game_id: Option<GameId>, state: &mut State) {
    let server_id = if let Some(prev_game_id) = prev_game_id {
        let prev_game = state.get_game_mut(prev_game_id);
        assert!(prev_game.time_end.is_some());
        // todo: что если prev_game.next_game_id != None (мб такое возможно при приостановке)
        prev_game.next_game_id = Some(curr_game_id);
        let curr_game = state.get_game_mut(curr_game_id);
        curr_game.prev_game_id = Some(prev_game_id);

        let server_id = state.game_ids.iter()
            .position(|&game_id| game_id == prev_game_id)
            // prev_game_id был добавлен в state.game_ids когда происходило объединение множеств {...} и {..., prev_game_id}
            .unwrap();
        state.game_ids[server_id] = curr_game_id;
        server_id
    } else {
        let server_id = state.game_ids.len();
        state.game_ids.push(curr_game_id);
        server_id
    };

    let server_id = state.as_server_id(server_id).unwrap();
    state.get_game_mut(curr_game_id).server_id = Some(server_id);
}

fn update_finished_games(prev_game_ids_all: &HashSet<GameId>, curr_game_ids_all: &HashSet<GameId>, state: &mut State, time: TimeMinutes) {
    let removed_game_ids = prev_game_ids_all.difference(&curr_game_ids_all);
    for &removed_game_id in removed_game_ids {
        let game = state.get_game_mut(removed_game_id);
        game.time_end = Some(time);
        for player_interval in game.players_intervals.iter_mut().rev() {
            if player_interval.end.is_some() {
                break;
            }
            player_interval.end = Some(time);
        }
    }
}

fn try_match_by_property<F>(
    prev_game_ids_host: &Vec<GameId>,
    curr_game_ids_host: &Vec<GameId>,
    state: &mut State,
    get_property: F,
) -> bool
    where F: Fn(&GameId, &State) -> String
{
    let prev_game_ids_by_property: HashMap<String, GameId> = prev_game_ids_host
        .iter().map(|&game_id| (get_property(&game_id, state), game_id)).collect();
    let curr_game_ids_by_property: HashMap<String, GameId> = curr_game_ids_host
        .iter().map(|&game_id| (get_property(&game_id, state), game_id)).collect();
    // если все property уникальны
    if prev_game_ids_by_property.len() == prev_game_ids_host.len() && curr_game_ids_by_property.len() == curr_game_ids_host.len() {
        for (property_value, game_id) in curr_game_ids_by_property {
            let prev_game_id = prev_game_ids_by_property.get(&property_value);
            merge_games(game_id, prev_game_id.copied(), state);
        }
        true
    } else {
        false
    }
}

fn try_merge_host(prev_game_ids_host: &[GameId], curr_game_ids_host: &[GameId], state: &mut State) -> bool {
    // не рассматриваем game_id, которые как были так и остались
    let prev_game_ids_host: HashSet<GameId> = prev_game_ids_host.iter().copied().collect();
    let curr_game_ids_host: HashSet<GameId> = curr_game_ids_host.iter().copied().collect();
    let common_game_ids_host = prev_game_ids_host.intersection(&curr_game_ids_host).copied().collect();
    let prev_game_ids_host: Vec<GameId> = prev_game_ids_host.difference(&common_game_ids_host).copied().collect();
    let curr_game_ids_host: Vec<GameId> = curr_game_ids_host.difference(&common_game_ids_host).copied().collect();

    for &game_id in &prev_game_ids_host {
        assert!(state.get_game(game_id).are_details_fetched());
    }

    // не объединяем game_ids пока не отправили запрос на /get-game-details
    // по идее проверка для prev_game_ids_host лишняя, так как для них уже проверяли когда объединяли их
    for &game_id in &curr_game_ids_host {
        if !state.get_game(game_id).are_details_fetched() {
            return false;
        }
    }

    if curr_game_ids_host.len() == 1 && prev_game_ids_host.len() == 1 {
        merge_games(curr_game_ids_host[0], Some(prev_game_ids_host[0]), state);
    } else if prev_game_ids_host.len() == 0 {
        for game_id in curr_game_ids_host {
            merge_games(game_id, None, state);
        }
    } else {
        let get_game_name = |&game_id: &GameId, state: &State| state.get_game_name(game_id).to_owned();
        let get_game_host = |&game_id: &GameId, state: &State| state.get_game_host(game_id).unwrap().to_owned();
        let success_matched = try_match_by_property(&prev_game_ids_host, &curr_game_ids_host, state, get_game_name)
            || try_match_by_property(&prev_game_ids_host, &curr_game_ids_host, state, get_game_host);
        if !success_matched {
            // todo log warning
            for game_id in curr_game_ids_host {
                merge_games(game_id, None, state);
            }
        }
    }
    true
}

pub fn try_merge_host_ids(updater_state: &mut UpdaterState, state: &mut State, time: TimeMinutes) {
    let curr_game_ids_by_host = group_game_ids_by_host(&state.current_game_ids, state);

    // было бы здорово если бы у HashMap был метод .drain_filter(): https://github.com/rust-lang/rust/issues/59618
    updater_state.scheduled_to_merge_host_ids = updater_state.scheduled_to_merge_host_ids.drain()
        .filter_map(|(host_id, merge_info)| {
            if time.get() - merge_info.time_end.get() < HOST_ID_MERGE_DELAY {
                return Some((host_id, merge_info));
            }

            let prev_game_ids_host = &merge_info.game_ids;
            let curr_game_ids_host = curr_game_ids_by_host.get(&host_id);
            if curr_game_ids_host.is_none() {
                // новых game_id не появилось: нечего объединять
                return None;
            }
            let curr_game_ids_host = curr_game_ids_host.unwrap();

            if try_merge_host(prev_game_ids_host, curr_game_ids_host, state) {
                None
            } else {
                Some((host_id, merge_info))
            }
        })
        .collect();
}

fn schedule_host_ids_merging(prev_game_ids_all: &HashSet<GameId>, curr_game_ids_all: &HashSet<GameId>, updater_state: &mut UpdaterState, state: &mut State, time: TimeMinutes) {
    let prev_game_ids_by_host = group_game_ids_by_host(prev_game_ids_all, &state);

    let changed_game_ids = curr_game_ids_all.symmetric_difference(&prev_game_ids_all);
    let changed_host_ids: HashSet<HostId> = changed_game_ids
        .map(|&game_id| state.get_game(game_id).host_id)
        .collect();
    for host_id in changed_host_ids {
        if let Some(merge_info) = updater_state.scheduled_to_merge_host_ids.get_mut(&host_id) {
            merge_info.time_end = time;
        } else {
            let game_ids = prev_game_ids_by_host.get(&host_id).map_or(Vec::new(), ToOwned::to_owned);
            let merge_info = HostIdMergeInfo {
                time_begin: time,
                time_end: time,
                game_ids,
            };
            updater_state.scheduled_to_merge_host_ids.insert(host_id.clone(), merge_info);
        }
    }
}

pub fn updater(
    updater_state_lock: Arc<RwLock<UpdaterState>>,
    state_lock: StateLock,
    receiver_fetcher_get_games: mpsc::Receiver<FetcherOutput>,
    sender_fetcher_get_game_details: mpsc::Sender<GameId>,
) {
    for (mut get_games_response, time) in receiver_fetcher_get_games {
        if GLOBAL_CONFIG.lock().unwrap().pipeline == "create_state_from_saved_data" && time.get() == 2 {
            // в начале второй итерации, чтобы fetcher_get_game_details успел обработать большое число игр, добавленных на первой итерации
            std::thread::sleep(Duration::from_millis(1000));
        }
        println!("[info]  [updater] handle response for minutes={}", time.get());

        // для fetcher_get_game_details, чтобы кеширование лучше работало
        get_games_response.sort_by_key(|game| game.game_id);

        let mut updater_state = updater_state_lock.write();
        let mut state = state_lock.write();
        for game_snapshot in &get_games_response {
            let game_id = game_snapshot.game_id;
            if state.games.contains_key(&game_id) {
                update_game(game_snapshot, &mut state, time);
            } else {
                let game = convert_snapshot_to_game(game_snapshot, &mut state, time);
                state.games.insert(game_id, game);

                sender_fetcher_get_game_details.send(game_id).unwrap();
            }
        }

        let curr_game_ids_all: HashSet<GameId> = get_games_response
            .iter().map(|game| game.game_id).collect();
        let prev_game_ids_all: HashSet<GameId> = state.current_game_ids
            .iter().copied().collect();

        update_finished_games(&prev_game_ids_all, &curr_game_ids_all, &mut state, time);

        schedule_host_ids_merging(&prev_game_ids_all, &curr_game_ids_all, &mut updater_state, &mut state, time);

        state.current_game_ids = Vec::from_iter(curr_game_ids_all);

        try_merge_host_ids(&mut updater_state, &mut state, time);
    }
    println!("[info]  [updater] exit");
}
