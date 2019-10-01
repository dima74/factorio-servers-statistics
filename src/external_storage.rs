use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fs::File;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use itertools::Itertools;
use parking_lot::RwLock;
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;

use crate::{fetcher_get_game_details, yandex_cloud_storage};
use crate::external_storage::SaverEvent::SIGINT;
use crate::state::{BigString, State, StateLock};
use crate::state::updater::UpdaterState;

mod backups;

const PRIMARY_STATES_DIRECTORY: &str = "states-hourly";
const STATE_TEMPORARY_FILE: &str = "state.bin.xz";

pub struct WholeState {
    pub updater_state: UpdaterState,
    pub state: State,
    pub fetcher_get_game_details_state: fetcher_get_game_details::State,
}

pub fn get_empty_state() -> WholeState {
    let updater_state = UpdaterState {
        scheduled_to_merge_host_ids: HashMap::new(),
    };

    let dummy_first_game_id = NonZeroU32::new(std::u32::MAX).unwrap();
    let state = State {
        games: HashMap::new(),
        game_ids: vec![dummy_first_game_id],
        current_game_ids: vec![],
        all_game_names: BigString::new(),
        all_game_descriptions: BigString::new(),
        all_versions: BigString::new(),
        all_tags: BigString::new(),
        all_host_addresses: BigString::new(),
        all_mod_names: BigString::new(),
        all_player_names: BigString::new(),
    };

    let fetcher_get_game_details_state = fetcher_get_game_details::State {
        game_ids: VecDeque::new()
    };

    WholeState {
        updater_state,
        state,
        fetcher_get_game_details_state,
    }
}

fn get_last_state_key() -> Option<String> {
    let keys = yandex_cloud_storage::list_bucket(PRIMARY_STATES_DIRECTORY);
    keys.into_iter().max()
}

pub fn fetch_state() -> WholeState {
    match get_last_state_key() {
        // todo remove
        None => get_empty_state(),
        Some(key) => {
            let mut reader = yandex_cloud_storage::download(&key);
            let reader = XzDecoder::new(&mut reader);

            let (updater_state, state, fetcher_get_game_details_state) = bincode::deserialize_from(reader).unwrap();
            WholeState { updater_state, state, fetcher_get_game_details_state }
        }
    }
}

pub fn load_state_from_file(filename: &str) -> WholeState {
    let mut reader = File::open(filename).unwrap();
    let reader = XzDecoder::new(&mut reader);

//    serde_json::from_reader(reader).unwrap()
    let (updater_state, state, fetcher_get_game_details_state) = bincode::deserialize_from(reader).unwrap();
    WholeState { updater_state, state, fetcher_get_game_details_state }
}

#[derive(PartialEq, Debug)]
pub enum SaverEvent {
    REGULAR,
    SIGINT,
}

pub fn save_state_to_file(
    updater_state: &UpdaterState,
    state: &State,
    fetcher_get_game_details_state: &fetcher_get_game_details::State,
    filename: &str,
) {
    let data = (updater_state, state, fetcher_get_game_details_state);

    let mut writer = File::create(filename).unwrap();
    let writer = XzEncoder::new(&mut writer, 9);

//    serde_json::to_writer(&mut writer, &data).unwrap();
    bincode::serialize_into(writer, &data).unwrap();
}

fn key_to_path(key: u64) -> String {
    format!("{}/{}.bin.xz", PRIMARY_STATES_DIRECTORY, key)
}

fn path_to_key(path: &str) -> Result<u64, std::num::ParseIntError> {
    let start = PRIMARY_STATES_DIRECTORY.len() + "/".len();
    let end = path.len() - ".bin.xz".len();
    path[start..end].parse()
}

pub fn save_state(
    updater_state: &UpdaterState,
    state: &State,
    fetcher_get_game_details_state: &fetcher_get_game_details::State,
) {
    save_state_to_file(updater_state, state, fetcher_get_game_details_state, STATE_TEMPORARY_FILE);

    let key = chrono::Utc::now().timestamp() / 3600;
    let path = key_to_path(key as u64);
    println!("[info]  [saver] upload state with path `{}`", path);
    // todo retry
    yandex_cloud_storage::upload(&path, Path::new(STATE_TEMPORARY_FILE), "application/x-xz");
}

pub fn saver(
    updater_state_lock: Arc<RwLock<UpdaterState>>,
    state_lock: StateLock,
    fetcher_get_game_details_state_lock: Arc<RwLock<fetcher_get_game_details::State>>,
    receiver: mpsc::Receiver<SaverEvent>,
) {
    for event in receiver {
        println!("[info]  [saver] start (by event {:?})", event);
        let updater_state = updater_state_lock.read();
        let state = state_lock.read();
        let fetcher_get_game_details_state = fetcher_get_game_details_state_lock.read();
        save_state(&updater_state, &state, &fetcher_get_game_details_state);
        if event == SIGINT {
            println!("[info]  [saver] exit (finished)");
            std::process::exit(77);
        }
    }
    eprintln!("[error] [saver] exit");
}

pub fn prune_state_backups_thread() {
    const DELAY: u64 = 30 * 60; // in seconds
    loop {
        thread::sleep(Duration::from_secs(DELAY));
        let result = prune_state_backups();
        if let Err(err) = result {
            eprintln!("[error] [external_storage] error when prune state backups: {}", err);
        }
    }
}

// path = "states-hourly/12345.bin.xz"
// key = 12345  (unix time divided by 3600)
// index = 1 + max(keys) - key  (latest backup has index 1)
pub fn prune_state_backups() -> Result<(), Box<dyn Error>> {
    let paths = yandex_cloud_storage::list_bucket(PRIMARY_STATES_DIRECTORY);
    let keys: Result<Vec<u64>, _> = paths.into_iter()
        .map(|path| path_to_key(&path))
        .collect();
    let keys = keys?;
    if keys.len() <= 1 {
        return Ok(());
    }

    let max_key = keys.iter().max().unwrap();
    let indexes: Vec<u64> = keys.iter()
        .map(|key| 1 + max_key - key)
        .sorted()
        .collect();

    let indexes_to_delete = backups::find_indexes_to_delete(&indexes);
    println!("[info]  [external_storage] indexes to be deleted: {:?}  (all indexes: {:?})", &indexes_to_delete, &indexes);
    for index in indexes_to_delete {
        let key = max_key + 1 - index;
        let path = key_to_path(key);
        yandex_cloud_storage::delete(&path)?;
    }
    Ok(())
}
