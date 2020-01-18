#![allow(warnings)]

use crate::external_storage::WholeState;
use itertools::Itertools;
use std::collections::HashSet;

pub fn analytics(whole_state: WholeState) {
    let state = whole_state.state;
    let updater_state = whole_state.updater_state;

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
        println!("scheduled to merge game_ids: {:?}", number_game_ids_to_merge)
    }

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
}
