#![feature(proc_macro_hygiene)]
#![feature(type_ascription)]
#![feature(decl_macro)]

use std::{fs, thread};
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use clap::{App, value_t};
use parking_lot::RwLock;

use cacher::CacherState;
use fss::{analytics, api, cacher, external_storage, fetcher_get_game_details, fetcher_get_games, fetcher_get_games_offline, state, util, yandex_cloud_storage};
use fss::global_config::GLOBAL_CONFIG;
use fss::state::StateLock;
use fss::util::basename;

mod server;

const DEBUG_STATE_FILE: &str = "temp/state/state.bin";
//const DEBUG_STATE_FILE: &str = "temp/state/state.bin.xz";
//const DEBUG_STATE_FILE: &str = "temp/state/state.bin.lz4";

fn main() {
    dotenv::dotenv().ok();

    let arguments = App::new("Factorio servers statistics")
        .arg_from_usage("<TYPE>")
        .arg_from_usage("--number_responses [val], 'only for TYPE = create_state_from_saved_data or create_state'")
        .get_matches();
    let pipeline = arguments.value_of("TYPE").unwrap();

    if pipeline != "production" {
        GLOBAL_CONFIG.lock().unwrap().use_cache_for_get_game_details = true;
    }
    GLOBAL_CONFIG.lock().unwrap().pipeline = pipeline.to_owned();

    match pipeline {
        "production" => run_production_pipeline(),
        "web_server" => run_web_server(),
        "analytics" => run_analytics(),
        "debug_fetcher_get_games" => debug_fetcher_get_games(),
        "debug_fetcher_get_game_details" => {
            GLOBAL_CONFIG.lock().unwrap().fetcher_get_games_skip_first_sleep = true;
            debug_fetcher_get_game_details();
        }
        "debug_updater" => debug_updater(),
        "create_state_from_saved_data" => {
            GLOBAL_CONFIG.lock().unwrap().fetcher_get_game_details_exit_after_fetch_all = true;
            let number_responses = value_t!(arguments, "number_responses", u32).unwrap_or_else(|e| e.exit());
            create_state_from_saved_data(number_responses);
        }
        "create_state" => {
            let number_responses = value_t!(arguments, "number_responses", u32).unwrap_or_else(|e| e.exit());
            create_state(number_responses);
        }
        "convert_state" => convert_state(),
        "prune_backups" => external_storage::prune_state_backups().unwrap(),
        "fetch_one_game_details" => {
            api::get_game_details(6067842).unwrap().unwrap();
        }
        "fetch_latest_state" => fetch_latest_state(),
        "fetch_all_states" => fetch_all_states(),
        "fetch_latest_state_as_is" => fetch_latest_state_as_is(),
        "recompress_backups" => external_storage::recompress_backups().unwrap(),
        "compress_state" => compress_state(),
        "print_state_heap_size" => print_state_heap_size(),
        "temp" => temp(),
        _ => panic!("unknown <TYPE> option"),
    };

    println!("[info]  [main] exit");
}

fn spawn_thread_with_name<F>(name: &str, f: F) -> JoinHandle<()>
    where
        F: FnOnce(), F: Send + 'static
{
    thread::Builder::new().name(name.to_owned()).spawn(f).unwrap()
}

fn regular_saver_notifier(sender: mpsc::Sender<()>) {
    const SAVER_NOTIFY_INTERVAL: u64 = 10 * 60; // in seconds
    loop {
        thread::sleep(Duration::from_secs(SAVER_NOTIFY_INTERVAL));
        sender.send(()).unwrap();
    }
}

fn run_production_pipeline() {
    let state_lock = StateLock::empty();
    let cacher_state_lock = Arc::new(RwLock::new(CacherState::new()));

    // Heroku forces us to bind to port within 60 seconds
    // So we have to launch Rocket ASAP
    println!("[info]  [startup] launching Rocket...");
    let rocket_thread = {
        let state_lock = state_lock.clone();
        let cacher_state_lock = cacher_state_lock.clone();
        spawn_thread_with_name("rocket", || server::init(state_lock, cacher_state_lock))
    };

    // todo убедиться что capacity(channel) == infinity, чтобы fetcher не блокировался на время подготовки данных для updater
    // fetcher_get_games
    let (sender_fetcher_get_games, receiver_fetcher_get_games) = mpsc::channel();
    spawn_thread_with_name("fetcher_get_games", move || fetcher_get_games::fetcher(sender_fetcher_get_games));

    // state
    println!("[info]  [startup] starting fetching state");
    let mut whole_state = external_storage::load_state_from_cloud();
    println!("[info]  [startup] finished fetching state");
    whole_state.state.compress();
    println!("[info]  [startup] finished compressing state");
    let updater_state_lock = Arc::new(RwLock::new(whole_state.updater_state));
    state_lock.set(whole_state.state);
    let fetcher_get_game_details_state_lock = Arc::new(RwLock::new(whole_state.fetcher_get_game_details_state));

    // fetcher_get_game_details
    let (sender_fetcher_get_game_details, receiver_fetcher_get_game_details) = mpsc::channel();
    {
        let state_lock = state_lock.clone();
        let fetcher_get_game_details_state_lock = fetcher_get_game_details_state_lock.clone();
        spawn_thread_with_name("fetcher_get_game_details", move || fetcher_get_game_details::fetcher(receiver_fetcher_get_game_details, fetcher_get_game_details_state_lock, state_lock));
    }

    // updater
    {
        let state_lock = state_lock.clone();
        let updater_state_lock = updater_state_lock.clone();
        spawn_thread_with_name("updater", move || state::updater::updater(updater_state_lock, state_lock, receiver_fetcher_get_games, sender_fetcher_get_game_details));
    }

    // saver
    let (saver_sender, saver_receiver) = mpsc::channel();
    {
        let state_lock = state_lock.clone();
        let updater_state_lock = updater_state_lock.clone();
        let fetcher_get_game_details_state_lock = fetcher_get_game_details_state_lock.clone();
        spawn_thread_with_name("saver", move || external_storage::saver(updater_state_lock, state_lock, fetcher_get_game_details_state_lock, saver_receiver));
    }

    // saver notifier
    spawn_thread_with_name("saver_notifier", move || regular_saver_notifier(saver_sender));

    // backups prune
    spawn_thread_with_name("external_storage_maintain_state_backups", external_storage::maintain_state_backups_thread);

    spawn_thread_with_name("cache", move || cacher::cacher(cacher_state_lock, state_lock));

    // [rocket_thread] should never return, here we just wait infinitely
    rocket_thread.join().unwrap();
}

fn run_web_server() {
    println!("Loading state...");
    let whole_state = external_storage::load_state_from_file(DEBUG_STATE_FILE);
    let state_lock = StateLock::new(whole_state.state);
    let cacher_state_lock = Arc::new(RwLock::new(CacherState::new()));

    {
        let state_lock = state_lock.clone();
        let cacher_state_lock = cacher_state_lock.clone();
        spawn_thread_with_name("cache", move || cacher::cacher(cacher_state_lock, state_lock));
    }

    println!("Launching Rocket...");
    server::init(state_lock, cacher_state_lock);
}

fn run_analytics() {
    let whole_state = external_storage::load_state_from_file(DEBUG_STATE_FILE);
    analytics::analytics(whole_state);
}

fn debug_fetcher_get_games() {
    let (sender, _receiver) = mpsc::channel();
    let fetcher_thread = spawn_thread_with_name("fetcher_get_games", move || fetcher_get_games::fetcher(sender));
    fetcher_thread.join().unwrap()
}

fn debug_fetcher_get_game_details() {
    // fetcher_get_games
    let (sender_fetcher_get_games, receiver_fetcher_get_games) = mpsc::channel();
    spawn_thread_with_name("fetcher_get_games", move || fetcher_get_games::fetcher(sender_fetcher_get_games));

    // state
    let whole_state = external_storage::get_empty_state();
    let updater_state_lock = Arc::new(RwLock::new(whole_state.updater_state));
    let state_lock = StateLock::new(whole_state.state);
    let fetcher_get_game_details_state_lock = Arc::new(RwLock::new(whole_state.fetcher_get_game_details_state));

    // fetcher_get_game_details
    let (sender_fetcher_get_game_details, receiver_fetcher_get_game_details) = mpsc::channel();
    let fetcher_get_game_details_thread = {
        let state_lock = state_lock.clone();
        spawn_thread_with_name("fetcher_get_game_details", move || fetcher_get_game_details::fetcher(receiver_fetcher_get_game_details, fetcher_get_game_details_state_lock, state_lock))
    };

    // updater
    {
        let state_lock = state_lock.clone();
        spawn_thread_with_name("updater", move || state::updater::updater(updater_state_lock, state_lock, receiver_fetcher_get_games, sender_fetcher_get_game_details));
    }

    fetcher_get_game_details_thread.join().unwrap();
}

fn debug_updater() {
    // fetcher_get_games
    let (sender_fetcher_get_games, receiver_fetcher_get_games) = mpsc::channel();
    let fetcher_thread = spawn_thread_with_name("fetcher_get_games", move || fetcher_get_games::fetcher(sender_fetcher_get_games));

    // state
    let whole_state = external_storage::get_empty_state();
    let updater_state_lock = Arc::new(RwLock::new(whole_state.updater_state));
    let state_lock = StateLock::new(whole_state.state);

    let (sender_fetcher_get_game_details, _receiver_fetcher_get_game_details) = mpsc::channel();

    // updater
    let updater_thread = spawn_thread_with_name("updater", move || state::updater::updater(updater_state_lock, state_lock, receiver_fetcher_get_games, sender_fetcher_get_game_details));

    fetcher_thread.join().unwrap();
    updater_thread.join().unwrap();
}

fn create_state_from_saved_data(number_responses: u32) {
    assert!(number_responses <= 2880);

    // fetcher_get_games
    let (sender_fetcher_get_games, receiver_fetcher_get_games) = mpsc::channel();
    fetcher_get_games_offline::fetcher(sender_fetcher_get_games, number_responses);

    // state
    let whole_state = external_storage::get_empty_state();
    let updater_state_lock = Arc::new(RwLock::new(whole_state.updater_state));
    let state_lock = StateLock::new(whole_state.state);
    let fetcher_get_game_details_state_lock = Arc::new(RwLock::new(whole_state.fetcher_get_game_details_state));

    // fetcher_get_game_details
    let (sender_fetcher_get_game_details, receiver_fetcher_get_game_details) = mpsc::channel();
    let fetcher_get_game_details_thread = {
        let state_lock = state_lock.clone();
        let fetcher_get_game_details_state_lock = fetcher_get_game_details_state_lock.clone();
        // fetcher_get_game_details обязательно должен быть в отдельном потоке и работать параллельно с updater
        // иначе updater будет бесконечно откладывать merge и любые перезапуски серверов не будут учтены
        spawn_thread_with_name("fetcher_get_game_details", move || fetcher_get_game_details::fetcher(receiver_fetcher_get_game_details, fetcher_get_game_details_state_lock, state_lock))
    };

    // updater
    {
        let state_lock = state_lock.clone();
        let updater_state_lock = updater_state_lock.clone();
        state::updater::updater(updater_state_lock, state_lock, receiver_fetcher_get_games, sender_fetcher_get_game_details);
    }

    fetcher_get_game_details_thread.join().unwrap();

    let updater_state = updater_state_lock.read();
    let state = state_lock.read();

    let number_games_with_prev_game_id = state.games.values().filter(|game| game.prev_game_id.is_some()).count();
    dbg!(number_games_with_prev_game_id);
    assert_ne!(number_games_with_prev_game_id, 0);

    let fetcher_get_game_details_state = fetcher_get_game_details_state_lock.read();
    let filename = format!("temp/state-offline/{}/state.bin.xz", number_responses);
    fs::create_dir_all(Path::new(&filename).parent().unwrap()).unwrap();
    external_storage::save_state_to_file((&updater_state, &state, &fetcher_get_game_details_state), &filename);
}

fn create_state(number_responses: u32) {
    // fetcher_get_games
    let (sender_fetcher_get_games, receiver_fetcher_get_games) = mpsc::channel();
    spawn_thread_with_name("fetcher_get_games", move || fetcher_get_games::fetcher(sender_fetcher_get_games));

    // state
    let whole_state = external_storage::get_empty_state();
    let updater_state_lock = Arc::new(RwLock::new(whole_state.updater_state));
    let state_lock = StateLock::new(whole_state.state);
    let fetcher_get_game_details_state_lock = Arc::new(RwLock::new(whole_state.fetcher_get_game_details_state));

    // fetcher_get_game_details
    let (sender_fetcher_get_game_details, receiver_fetcher_get_game_details) = mpsc::channel();
    {
        let state_lock = state_lock.clone();
        let fetcher_get_game_details_state_lock = fetcher_get_game_details_state_lock.clone();
        spawn_thread_with_name("fetcher_get_game_details", move || fetcher_get_game_details::fetcher(receiver_fetcher_get_game_details, fetcher_get_game_details_state_lock, state_lock));
    }

    // updater
    {
        let state_lock = state_lock.clone();
        let updater_state_lock = updater_state_lock.clone();
        spawn_thread_with_name("updater", move || state::updater::updater(updater_state_lock, state_lock, receiver_fetcher_get_games, sender_fetcher_get_game_details));
    }

    thread::sleep(Duration::from_secs((number_responses * 60) as u64));

    let updater_state = updater_state_lock.read();
    let state = state_lock.read();
    let fetcher_get_game_details_state = fetcher_get_game_details_state_lock.read();
    let filename = format!("temp/state-online/{}/state.bin.xz", number_responses);
    fs::create_dir_all(Path::new(&filename).parent().unwrap()).unwrap();
    external_storage::save_state_to_file((&updater_state, &state, &fetcher_get_game_details_state), &filename);
}

fn convert_state() {
    let whole_state = external_storage::load_state_from_file(DEBUG_STATE_FILE);
    external_storage::save_state_to_file(whole_state.deref(), DEBUG_STATE_FILE);
}

fn fetch_latest_state() {
    let mut whole_state = external_storage::load_state_from_cloud();
    whole_state.state.compress();
    external_storage::save_state_to_file(whole_state.deref(), DEBUG_STATE_FILE);
}

fn fetch_latest_state_as_is() {
    let state_path = external_storage::get_last_state_path().unwrap();
    let filename = format!("temp/state/{}", basename(&state_path));
    yandex_cloud_storage::download_to_file(&state_path, Path::new(&filename)).unwrap();
}

fn fetch_all_states() {
    let paths = external_storage::get_state_paths();
    for path in paths {
        let path_basename = basename(&path);
        let filename = format!("temp/backup/state-from-yandex-cloud/{}", path_basename);
        yandex_cloud_storage::download_to_file(&path, Path::new(&filename)).unwrap();
    }
}

fn compress_state() {
    let mut whole_state = external_storage::load_state_from_file(DEBUG_STATE_FILE);
    util::print_heap_stats();

    whole_state.state.compress();
    println!("\tAfter compress:");
    util::print_heap_stats();
}

// #[global_allocator]
// static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn print_state_heap_size() {
    // todo research why with FssHashMap games self size is 180MB
    //  looks like that for some reason size of games.values is 20MB more than expected

    // results:
    //                   actual vs minimum
    // games self           291 vs 132  (hashbrown::HashMap)
    // games self           180 vs 132  (FssHashMap)
    // mods                  87 vs 80
    // player_intervals      36 vs 29

    let mut whole_state = external_storage::load_state_from_file(DEBUG_STATE_FILE);
    println!("\tОбъём WholeState:");
    util::print_heap_stats();

    let mut games = state::GamesMap::new();
    std::mem::swap(&mut whole_state.state.games, &mut games);
    drop(whole_state);
    println!("\tОбъём games:");
    util::print_heap_stats();

    for game in games.values_mut() {
        game.mods = None;
    }
    println!("\tОбъём games без mods:");
    util::print_heap_stats();

    for game in games.values_mut() {
        game.players_intervals = Vec::new();
    }
    println!("\tОбъём games без mods и player_intervals:");
    util::print_heap_stats();

    dbg!(games.len(), games.len() * std::mem::size_of::<state::Game>() / (1024 * 1024));
    drop(games);
    util::print_heap_stats();
}

fn temp() {}
