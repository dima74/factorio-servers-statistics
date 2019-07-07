use crate::state::State;

pub fn analytics(state: State) {
    println!("число наблюдаемых game_id: {}", state.games.len());
    println!("число наблюдаемых серверов: {}", state.game_ids.len());

    {
//        let game_id = state.current_game_ids.iter()
//            .max_by_key(|&&game_id| state.get_game(game_id).number_players()).unwrap();
//        println!("")
        println!("Игры с более чем 5 игроками:");
        for &game_id in state.current_game_ids.iter() {
            if state.get_game(game_id).number_players() > 5 {
                let game_name: &str = state.get_game_name(game_id).into();
                println!("\t{}", game_name);
            }
        }
    }

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
