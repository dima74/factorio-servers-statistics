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

use crate::{fetcher_get_game_details, yandex_cloud_storage};
use crate::external_storage::SaverEvent::SIGINT;
use crate::state::{BigString, State, StateLock};
use crate::state::updater::UpdaterState;

mod backups;
mod compression;

const PRIMARY_STATES_DIRECTORY: &str = "states-hourly";
const TEMPORARY_STATE_FILE: &str = "state.bin.lz4";
const TEMPORARY_LZ4_FILE_FOR_RECOMPRESS: &str = "state-recompress.bin.lz4";
const TEMPORARY_XZ_FILE_FOR_RECOMPRESS: &str = "state-recompress.bin.xz";
const CONTENT_TYPE: &str = "application/octet-stream";

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

pub fn get_state_paths() -> Vec<String> {
    yandex_cloud_storage::list_bucket(PRIMARY_STATES_DIRECTORY)
}

pub fn get_last_state_path() -> Option<String> {
    let paths = get_state_paths();
    paths.into_iter().max()
}

pub fn fetch_state() -> WholeState {
    let path = get_last_state_path().unwrap();
    let mut reader = yandex_cloud_storage::download(&path)
        .expect(&format!("Couldn't download {} object from Yandex.Cloud", path));
    let reader = compression::new_decoder(&mut reader, &path);

    let (updater_state, state, fetcher_get_game_details_state) = bincode::deserialize_from(reader).unwrap();
    WholeState { updater_state, state, fetcher_get_game_details_state }
}

pub fn load_state_from_file(filename: &str) -> WholeState {
    let mut reader = File::open(filename).unwrap();
    let reader = compression::new_decoder(&mut reader, filename);

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
    let writer = compression::new_encoder(&mut writer, filename);

    bincode::serialize_into(writer, &data).unwrap();
}

fn key_to_path(key: u64) -> String {
    format!("{}/{}.bin.lz4", PRIMARY_STATES_DIRECTORY, key)
}

fn path_to_key(path: &str) -> Result<u64, std::num::ParseIntError> {
    let start = PRIMARY_STATES_DIRECTORY.len() + "/".len();
    let end = path.find('.').unwrap();
    path[start..end].parse()
}

pub fn save_state(
    updater_state: &UpdaterState,
    state: &State,
    fetcher_get_game_details_state: &fetcher_get_game_details::State,
) {
    save_state_to_file(updater_state, state, fetcher_get_game_details_state, TEMPORARY_STATE_FILE);

    let key = chrono::Utc::now().timestamp() / 3600;
    let path = key_to_path(key as u64);
    println!("[info]  [saver] start uploading state with path `{}`", path);
    // todo retry
    yandex_cloud_storage::upload(&path, Path::new(TEMPORARY_STATE_FILE), CONTENT_TYPE);
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
        println!("[info]  [saver] done");
        if event == SIGINT {
            println!("[info]  [saver] exit (finished)");
            std::process::exit(77);
        }
    }
    eprintln!("[error] [saver] exit");
}

pub fn maintain_state_backups_thread() {
    const DELAY: u64 = 20 * 60; // in seconds
    loop {
        thread::sleep(Duration::from_secs(DELAY));
        let result = prune_state_backups();
        if let Err(err) = result {
            eprintln!("[error] [external_storage] error when prune state backups: {}", err);
        }
    }
}

// lz4 -> xz
pub fn recompress_backups() -> Result<(), Box<dyn Error>> {
    let paths = get_state_paths();
    let latest_path = match paths.iter().max() {
        Some(path) => path,
        None => return Ok(()),
    };
    let paths = paths.iter()
        .filter(|&path| path.ends_with(".lz4") && path != latest_path);
    for path_lz4 in paths {
        let path_xz = path_lz4.replace(".lz4", ".xz");
        println!("[info]  [external_storage] recompress backup: {} -> {}", path_lz4, path_xz);

        yandex_cloud_storage::download_to_file(&path_lz4, Path::new(TEMPORARY_LZ4_FILE_FOR_RECOMPRESS))?;

        let mut reader = File::open(TEMPORARY_LZ4_FILE_FOR_RECOMPRESS)?;
        let mut reader = compression::new_decoder(&mut reader, &path_lz4);

        let mut writer = File::create(TEMPORARY_XZ_FILE_FOR_RECOMPRESS)?;
        let mut writer = compression::new_encoder(&mut writer, &path_xz);

        std::io::copy(&mut reader, &mut writer)?;
        yandex_cloud_storage::upload(&path_xz, Path::new(TEMPORARY_XZ_FILE_FOR_RECOMPRESS), CONTENT_TYPE);

        yandex_cloud_storage::delete(&path_lz4)?;
    }
    Ok(())
}

// path = "states-hourly/12345.bin.<compression>"
// key = 12345  (unix time divided by 3600)
// index = 1 + max(keys) - key  (latest backup has index 1)
pub fn prune_state_backups() -> Result<(), Box<dyn Error>> {
    let paths = yandex_cloud_storage::list_bucket(PRIMARY_STATES_DIRECTORY);
    let key_to_path: Result<HashMap<u64, String>, _> = paths.into_iter()
        .map(|path| path_to_key(&path).map(|key| (key, path)))
        .collect();
    let key_to_path = key_to_path?;
    let keys: Vec<u64> = key_to_path.keys().copied().collect();
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
        let path = key_to_path.get(&key).unwrap();
        yandex_cloud_storage::delete(&path)?;
    }
    Ok(())
}