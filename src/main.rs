#![feature(proc_macro_hygiene)]
#![feature(type_ascription)]
#![feature(decl_macro)]

use std::{fs, thread};
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use clap::{App, value_t};
use parking_lot::RwLock;

use fss::{analytics, cacher, external_storage, fetcher_get_game_details, fetcher_get_games, fetcher_get_games_offline, state};
use fss::external_storage::SaverEvent;
use fss::global_config::GLOBAL_CONFIG;
use fss::state::StateLock;

mod server;

const DEBUG_STATE_FILE: &str = "temp/state-offline/2880/state.bin.xz";
//const DEBUG_STATE_FILE: &str = "temp/state-online/60/state.bin.xz";

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

fn regular_saver_notifier(sender: mpsc::Sender<SaverEvent>) {
    const SAVER_NOTIFY_INTERVAL: u64 = 10 * 60; // in seconds
    loop {
        thread::sleep(Duration::from_secs(SAVER_NOTIFY_INTERVAL));
        let result = sender.send(SaverEvent::REGULAR);
        // если .send() вернул ошибку, это означает что saver получил SaverEvent::SIGINT и завершил работу
        if result.is_err() {
            break;
        }
    }
    println!("[info]  [regular_saver_notifier] exit");
}

fn init_server_with_cacher(state_lock: StateLock) {
    let cacher_state = cacher::CacherState::new();
    let cacher_state_lock = Arc::new(RwLock::new(cacher_state));

    {
        let state_lock = state_lock.clone();
        let cacher_state_lock = cacher_state_lock.clone();
        spawn_thread_with_name("cache", move || cacher::cacher(cacher_state_lock, state_lock));
    }

    server::init(state_lock, cacher_state_lock);
}

fn run_production_pipeline() {
    // todo убедиться что capacity(channel) == infinity, чтобы fetcher не блокировался на время подготовки данных для updater
    // fetcher_get_games
    let (sender_fetcher_get_games, receiver_fetcher_get_games) = mpsc::channel();
    spawn_thread_with_name("fetcher_get_games", move || fetcher_get_games::fetcher(sender_fetcher_get_games));

    // state
    let mut whole_state = external_storage::fetch_state();
    whole_state.state.compress_big_strings();
    let updater_state_lock = Arc::new(RwLock::new(whole_state.updater_state));
    let state_lock = Arc::new(RwLock::new(whole_state.state));
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
    {
        let saver_sender = saver_sender.clone();
        spawn_thread_with_name("saver_notifier", move || regular_saver_notifier(saver_sender));
    }

    // SIGINT handler
    {
        let saver_sender = saver_sender.clone();
        let already_received_sigint = AtomicBool::new(false);
        ctrlc::set_handler(move || {
            if already_received_sigint.swap(true, Ordering::SeqCst) {
                println!("[warn]  [sigint_handler] already received sigint");
                return;
            }
            // .send() возвращает ошибку если receiver был уничтожен (deallocate), но у нас такого не может быть
            saver_sender.send(SaverEvent::SIGINT).unwrap();
        }).expect("Error setting SIGINT handler");
    }

    // external storage prune state backups
    {
        spawn_thread_with_name("external_storage_prune_state_backups", || {
            const DELAY: u64 = 3600; // in seconds
            thread::sleep(Duration::from_secs(DELAY));
            let result = external_storage::prune_state_backups();
            if let Err(err) = result {
                eprintln!("[error] [external_storage] error when prune state backups: {}", err);
            }
        });
    }

    init_server_with_cacher(state_lock);
}

fn run_web_server() {
    let whole_state = external_storage::load_state_from_file(DEBUG_STATE_FILE);
    let state_lock = Arc::new(RwLock::new(whole_state.state));
    init_server_with_cacher(state_lock);
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
    let state_lock = Arc::new(RwLock::new(whole_state.state));
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
    let state_lock = Arc::new(RwLock::new(whole_state.state));

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
    let state_lock = Arc::new(RwLock::new(whole_state.state));
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
    assert!(number_games_with_prev_game_id != 0);

    let fetcher_get_game_details_state = fetcher_get_game_details_state_lock.read();
    let filename = format!("temp/state-offline/{}/state.bin.xz", number_responses);
    fs::create_dir_all(Path::new(&filename).parent().unwrap()).unwrap();
    external_storage::save_state_to_file(&updater_state, &state, &fetcher_get_game_details_state, &filename);
}

fn create_state(number_responses: u32) {
    // fetcher_get_games
    let (sender_fetcher_get_games, receiver_fetcher_get_games) = mpsc::channel();
    spawn_thread_with_name("fetcher_get_games", move || fetcher_get_games::fetcher(sender_fetcher_get_games));

    // state
    let whole_state = external_storage::get_empty_state();
    let updater_state_lock = Arc::new(RwLock::new(whole_state.updater_state));
    let state_lock = Arc::new(RwLock::new(whole_state.state));
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
    external_storage::save_state_to_file(&updater_state, &state, &fetcher_get_game_details_state, &filename);
}

fn convert_state() {
    let whole_state = external_storage::load_state_from_file(DEBUG_STATE_FILE);
    let updater_state = whole_state.updater_state;
    let state = whole_state.state;
    let fetcher_get_game_details_state = whole_state.fetcher_get_game_details_state;
    external_storage::save_state_to_file(&updater_state, &state, &fetcher_get_game_details_state, DEBUG_STATE_FILE);
}
