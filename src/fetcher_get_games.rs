use std::ops::Sub;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Utc;

use crate::api;
use crate::global_config::GLOBAL_CONFIG;
use crate::state::TimeMinutes;
use crate::util::duration_since;

pub type FetcherOutput = (api::GetGamesResponse, TimeMinutes);

pub fn fetcher(sender: mpsc::Sender<FetcherOutput>) {
    let minute = 60;  // в секундах

    let sleep_to_nearest_minute = |first_time: bool| {
        if first_time && GLOBAL_CONFIG.lock().unwrap().fetcher_get_games_skip_first_sleep {
            return;
        }

        let time = duration_since(SystemTime::now(), UNIX_EPOCH);
        let time_next = Duration::from_secs((time.as_secs() / minute + 1) * minute);
        if !first_time && (time_next - time).as_secs() * 2 < minute {
            eprintln!("[warn]  [fetcher_get_games] время сна меньше чем половина минуты");
        }
        thread::sleep(time_next.sub(time));
    };

    let mut last_fetch_time = None;
    loop {
        sleep_to_nearest_minute(last_fetch_time.is_none());
        let current_fetch_time = SystemTime::now();
        if let Some(last_fetch_time) = last_fetch_time {
            let duration_between_fetches: Duration = duration_since(current_fetch_time, last_fetch_time);
            let relative_error = f64::abs(duration_between_fetches
                .div_duration_f64(Duration::from_secs(minute)) - 1.0);
            if relative_error > 0.1 {
                eprintln!("[warn]  [fetcher_get_games] duration between fetches differs from 60 seconds, observed duration is {:?}", duration_between_fetches);
            }
        }
        last_fetch_time = Some(current_fetch_time);

        let response_time = TimeMinutes::now();
        println!("[info]  [fetcher_get_games] fetch at secs={}, minutes={}, utc={}",
                 duration_since(SystemTime::now(), UNIX_EPOCH).as_secs(),
                 response_time.get(),
                 Utc::now()
        );

        let get_games_response = api::get_games();
        sender.send((get_games_response, response_time)).unwrap();
    }
}
