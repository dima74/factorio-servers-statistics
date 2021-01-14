#![allow(warnings)]

use hashbrown::{HashMap, HashSet};
use itertools::Itertools;

use crate::external_storage::WholeState;
use crate::state::{Game, Mod, PlayerInterval, State};

pub fn analytics(whole_state: WholeState) {
    let state = whole_state.state;
    let updater_state = whole_state.updater_state;

    println!("\tБазовая статистика:");
    println!("число наблюдаемых game_id: {}", state.games.len());
    println!("число наблюдаемых серверов: {}", state.game_ids.len());
    println!("game_ids_in_last_get_games_response.len(): {}", state.current_game_ids.len());

    println!("Игр с полученными details: {}",
             state.games.values().filter(|game| game.are_details_fetched()).count());
    println!("Игр с prev_game_id != None: {}",
             state.games.values().filter(|game| game.prev_game_id.is_some()).count());
    println!("Игр с server_id != None: {}",
             state.games.values().filter(|game| game.server_id.is_some()).count());

    // merge
    {
        println!("scheduled_to_merge_host_ids.len(): {}", updater_state.scheduled_to_merge_host_ids.len());

        let number_game_ids_to_merge: usize = updater_state
            .scheduled_to_merge_host_ids
            .values()
            .map(|merge_info| merge_info.game_ids.len())
            .sum();
        println!("scheduled to merge game_ids: {:?}", number_game_ids_to_merge);
    }

    // mods duplication (when game has same mods as prev_game)
    {
        let mut number_matched_games = 0;
        let mut number_matched_mods = 0;
        for game in state.games.values() {
            let prev_game_id = match game.prev_game_id {
                Some(prev_game_id) => prev_game_id,
                None => continue,
            };
            let prev_game = state.get_game(prev_game_id);
            let (mods, prev_mods) = match (&game.mods, &prev_game.get_mods(&state)) {
                (Some(mods), Some(prev_mods)) => (mods, prev_mods),
                _ => continue,
            };

            if (mods == prev_mods && !mods.is_empty()) {
                number_matched_games += 1;
                number_matched_mods += mods.len();
            }
        }
        println!("number games which has same mods as prev_game: {:?}", number_matched_games);
        println!("number duplicated mods: {:?}", number_matched_mods);
    }

    // mods unique count
    {
        let mods_all: Vec<Mod> = state.games.values()
            .flat_map(|game| game.mods.clone().unwrap_or_default())
            .collect();
        let number_mods_all = mods_all.len();
        let number_mods_unique = mods_all.iter().unique().count();
        println!("`Mod` objects unique/all: {}/{}", number_mods_unique, number_mods_all);

        let mod_sets_all: Vec<Vec<Mod>> = state.games.values()
            .map(|game| game.mods.clone().unwrap_or_default())
            .filter(|mods| !mods.is_empty())
            .collect();
        let mod_sets_unique: Vec<Vec<Mod>> = mod_sets_all.iter().unique().cloned().collect();
        println!("mod sets (Vec<Mod>) unique/all: {}/{}", mod_sets_unique.len(), mod_sets_all.len());
        println!(
            "number mods in mod sets unique/mod sets all: {}/{}",
            mod_sets_unique.iter().map(|mods| mods.len()).sum::<usize>(),
            mod_sets_all.iter().map(|mods| mods.len()).sum::<usize>()
        );
    }

    // объём памяти занимаемый games (map из GameId в Game) в идеальном случае
    {
        use std::mem::size_of;
        const MB: usize = 1024 * 1024;

        let number_player_interval_objects: usize = state.games.values()
            .map(|game| game.players_intervals.len())
            .sum();
        let number_mod_objects: usize = state.games.values()
            .map(|game| game.mods.as_deref().map_or(0, |mods| mods.len()))
            .sum();
        let size_player_interval_objects = number_player_interval_objects * size_of::<PlayerInterval>();
        let size_mod_objects = number_mod_objects * size_of::<Mod>();
        let number_games = state.games.len();
        let size_games = number_games * size_of::<Game>();
        println!("\n\tМинимально возможный объём памяти занимаемый games:");
        println!("total PlayerInterval size: {}MB  (count={:?})", size_player_interval_objects / MB, number_player_interval_objects);
        println!("total Mod size: {}MB  (count={:?})", size_mod_objects / MB, number_mod_objects);
        println!("games self size = {}MB  (count={})", size_games / MB, number_games);
        println!("total size = {}MB", (size_player_interval_objects + size_mod_objects + size_games) / MB);
    }

    println!("\n\tРаспределения:");

    // распределение длительности отдельных game_id
    {
        let durations = state.games.values()
            .filter_map(|game| {
                game.time_end.map(|time_end| {
                    let duration = time_end.get() - game.time_begin.get();
                    duration as u64
                })
            });

        println!("\tgame_id duration percentiles (in minutes):");
        print_histogram(durations);
    }

    // распределение числа уникалььных игроков для game_id
    {
        let unique_players_count = state.games.values()
            .map(|game| game.number_players_all() as u64);

        println!("\tgame_id unique players count:");
        print_histogram(unique_players_count);
    }

    // распределение игроков-часов для game_id (сумма времени игры по всем игрокам)
    {
        let unique_players_count = state.games.values()
            .map(|game| game.total_player_minutes() / 60);

        println!("\tgame_id player-hours:");
        print_histogram(unique_players_count);
    }

    print_average_number_new_games_per_day(&state);

//    посчитать число game_id достижимых если идти по prev_game_id от одного из серверов (state.game_ids)
//    подумать что делать с играми, которые исчезают почти сразу после появления (проверить что их 30% --- столько игр без server_id)


//    {
////        let game_id = state.current_game_ids.iter()
////            .max_by_key(|&&game_id| state.get_game(game_id).number_players()).unwrap();
////        println!("")
//        println!("Игры с более чем 5 игроками:");
//        for &game_id in state.current_game_ids.iter() {
//            if state.get_game(game_id).number_players() > 5 {
//                let game_name: &str = state.get_game_name(game_id).into();
//                println!("\t{}", game_name);
//            }
//        }
//    }

    for (game_id, game) in &state.games {
        let players_online = game.players_intervals.iter()
            .filter(|interval| interval.end.is_none())
            .count();
        let intervals_all = game.players_intervals.len();
//        println!("{:5}:  {:4} / {:4}", game_id, players_online, intervals_all);
        if players_online != 0 && players_online != intervals_all {
//            println!("!=");
//            println!("{:5}:  {:4} / {:4}", game_id, players_online, intervals_all);
        }
    }
}

fn print_average_number_new_games_per_day(state: &State) {
    const DAY_IN_MINUTES: u32 = 24 * 60;
    type Day = u32;

    let mut counts: HashMap<Day, u64> = HashMap::new();
    for game in state.games.values() {
        let time_begin = game.time_begin;
        let day_begin: Day = time_begin.get() / DAY_IN_MINUTES;
        counts.insert(day_begin, 1 + counts.get(&day_begin).unwrap_or(&0));
    }

    println!("\taverage number new games per day:");
    print_histogram(counts.values().copied());
}

fn print_histogram(values: impl Iterator<Item=u64>) {
    use histogram::Histogram;

    let mut histogram = Histogram::new();
    for value in values {
        histogram.increment(value);
    }

    let mut percentiles: Vec<f64> = (10..100).step_by(10)
        .map(Into::into).collect();
    percentiles.extend_from_slice(&[95.0, 99.0, 99.5, 99.9]);

    for percentile in percentiles {
        println!("{: <5}: {}", percentile, histogram.percentile(percentile).unwrap());
    }
    println!("Min: {}  Avg: {}  Max: {}",
             histogram.minimum().unwrap(),
             histogram.mean().unwrap(),
             histogram.maximum().unwrap(),
    );
    println!();
}
