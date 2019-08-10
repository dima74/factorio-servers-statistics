#![allow(warnings)]

use crate::external_storage::WholeState;

pub fn analytics(whole_state: WholeState) {
    let state = whole_state.state;
    let updater_state = whole_state.updater_state;

    println!("число наблюдаемых game_id: {}", state.games.len());
    println!("число наблюдаемых серверов: {}", state.game_ids.len());
    println!("scheduled_to_merge_host_ids.len(): {}", updater_state.scheduled_to_merge_host_ids.len());
    println!("game_ids_in_last_get_games_response.len(): {}", state.current_game_ids.len());

    println!("Игр с полученными details: {}",
             state.games.values().filter(|game| game.are_details_fetched()).count());
    println!("Игр с prev_game_id != None: {}",
             state.games.values().filter(|game| game.prev_game_id.is_some()).count());
    println!("Игр с server_id != None: {}",
             state.games.values().filter(|game| game.server_id.is_some()).count());

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
