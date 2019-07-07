use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use big_string::*;

use crate::util::duration_since;

pub mod updater;
mod big_string;

/// unix time, с точностью до минут
/// (число минут, прошедшее с UNIX_EPOCH)
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TimeMinutes(pub NonZeroU32);

impl TimeMinutes {
    pub const WEEK: u32 = 7 * 24 * 60;

    pub fn new(value: u32) -> Option<Self> {
        NonZeroU32::new(value)
            .map(|value| TimeMinutes(value))
    }

    pub fn now() -> Self {
        let time = duration_since(SystemTime::now(), UNIX_EPOCH);
        let time_minutes = (time.as_secs_f64() / 60.0 + 0.5) as u32;
        TimeMinutes::new(time_minutes).unwrap()
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

pub type GameId = NonZeroU32;
pub type ServerId = NonZeroU32;
// == base64decode(поле server_id в json)
pub type HostId = [u8; 32];

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub name: BigStringPart,
    pub version: BigStringPart,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInterval {
    pub player_index: BigStringPart,
    // полуинтервал: [begin, end)
    pub start: TimeMinutes,
    pub end: Option<TimeMinutes>,
}

impl PlayerInterval {
    pub fn new(player_index: BigStringPart, start: TimeMinutes) -> Self {
        PlayerInterval {
            player_index,
            start,
            end: None,
        }
    }
}

// содержит всю информацию об одной сессии сервера (одна сессия == один game_id)
// в течении сессии метаинформация о сервере (название, версия, моды и т.д.) не должны меняться
// ожидается, что сессия длится непрерывный отрезок по времени
//     (однако по наблюдениям сессия может прерываться на очень большой промежуток времени, вплоть до ~30 часов)
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub game_id: GameId,
    // todo подумать точно ли оно нужно
    pub server_id: Option<ServerId>,
    // None если предыдущей/следующей игры не было или она ещё не вычислена
    pub prev_game_id: Option<GameId>,
    pub next_game_id: Option<GameId>,
    // полуинтервал: [time_begin, time_end)
    // todo добавить куда-нибудь проверку, что интервалы соседных игр перекрываются не более чем на 10-15 минут
    pub time_begin: TimeMinutes,
    pub time_end: Option<TimeMinutes>,

    // гарантируется, что все игроки которые сейчас онлайн находятся в конце
    pub players_intervals: Vec<PlayerInterval>,

    pub host_id: HostId,
    pub name: BigStringPart,
    pub max_players: u32,
    pub game_version: BigStringPart,
    pub game_time_elapsed: u32,
    pub has_password: bool,
    // разделённые символом \x02
    pub tags: BigStringPart,
    pub last_heartbeat: f64,
    pub mod_count: u16,

    // None означает что значение ещё не получено (с помощью запроса на /get-game-details)
    pub description: Option<BigStringPart>,
    pub host_address: Option<BigStringPart>,
    // None означает что значение ещё не получено или что такое же как у prev_game_id
    // todo: "такое же как у prev_game_id"
    pub mods: Option<Vec<Mod>>,
}

impl Game {
    pub fn number_players(&self) -> usize {
        let mut first_online_player_index = self.players_intervals.iter()
            .rposition(|player_interval| player_interval.end.is_some())
            .map(|index| index + 1)
            .unwrap_or(0);
        self.players_intervals.len() - first_online_player_index
    }
}

pub type StateLock = Arc<RwLock<State>>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub games: HashMap<GameId, Game>,
    // будем использовать собственную нумерацию серверов, обозначаемую ServerId
    // ServerId — индекс для массива game_ids
    // game_ids[ServerId] — последний game_id этого сервера (такой что .next_game_id == None)
    pub game_ids: Vec<GameId>,
    // game_id из last_get_games_response
    pub current_game_ids: Vec<GameId>,

    pub all_game_names: BigString,
    pub all_game_descriptions: BigString,
    pub all_versions: BigString,
    pub all_tags: BigString,
    pub all_host_addresses: BigString,
    pub all_mod_names: BigString,
    pub all_player_names: BigString,
}

impl State {
    pub fn get_game(&self, id: GameId) -> &Game {
        self.games.get(&id).unwrap()
    }

    pub fn get_game_mut(&mut self, id: GameId) -> &mut Game {
        self.games.get_mut(&id).unwrap()
    }

    pub fn get_game_name(&self, id: GameId) -> FssStr {
        let game = self.get_game(id);
        self.all_game_names.get(game.name)
    }

    pub fn get_server_last_game_id(&self, id: ServerId) -> Option<GameId> {
        self.game_ids.get(id.get() as usize).copied()
    }

    fn get_game_host(&self, id: GameId) -> Option<FssStr> {
        let game = self.get_game(id);
        game.host_address.map(|host_address| self.all_host_addresses.get(host_address))
    }

    fn set_debug_names(&mut self) {
        self.all_game_names.set_debug_name("game_names".to_owned());
        self.all_game_descriptions.set_debug_name("game_descriptions".to_owned());
        self.all_versions.set_debug_name("versions".to_owned());
        self.all_tags.set_debug_name("tags".to_owned());
        self.all_host_addresses.set_debug_name("host_addresses".to_owned());
        self.all_mod_names.set_debug_name("mod_names".to_owned());
        self.all_player_names.set_debug_name("player_names".to_owned());
    }

    pub fn compress_big_strings(&mut self) {
        self.set_debug_names();

        let map_names = self.all_game_names.compress();
        let map_descriptions = self.all_game_descriptions.compress();
        let map_versions = self.all_versions.compress();
        let map_tags = self.all_tags.compress();
        let map_host_addresses = self.all_host_addresses.compress();
        let map_mod_names = self.all_mod_names.compress();
        let map_player_names = self.all_player_names.compress();
        for game in self.games.values_mut() {
            game.name = *map_names.get(&game.name).unwrap();
            if let Some(ref mut description) = game.description {
                *description = *map_descriptions.get(description).unwrap();
            }
            game.game_version = *map_versions.get(&game.game_version).unwrap();
            game.tags = *map_tags.get(&game.tags).unwrap();
            if let Some(ref mut host_address) = game.host_address {
                *host_address = *map_host_addresses.get(host_address).unwrap();
            }

            if let Some(ref mut mods) = game.mods {
                for mod_ in mods {
                    mod_.name = *map_mod_names.get(&mod_.name).unwrap();
                    mod_.version = *map_versions.get(&mod_.version).unwrap();
                }
            }

            for players_interval in &mut game.players_intervals {
                players_interval.player_index = *map_player_names.get(&players_interval.player_index).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(std::mem::size_of::<PlayerInterval>(), 12);
    }
}
