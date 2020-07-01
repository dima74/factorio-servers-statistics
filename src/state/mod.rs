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
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
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

// == base64decode(поле server_id в json)
pub type HostId = [u8; 32];

/// будем использовать собственную нумерацию серверов, обозначаемую ServerId
/// ServerId — индекс для массива game.game_ids
/// `game_ids[ServerId]` — последний game_id этого сервера (такой что .next_game_id == None)
#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ServerId(NonZeroU32);

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub name: BigStringPart,
    pub version: BigStringPart,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInterval {
    pub player_index: BigStringPart,
    // полуинтервал: [begin, end)
    pub begin: TimeMinutes,
    pub end: Option<TimeMinutes>,
}

impl PlayerInterval {
    pub fn new(player_index: BigStringPart, begin: TimeMinutes) -> Self {
        PlayerInterval {
            player_index,
            begin,
            end: None,
        }
    }
}

// содержит всю информацию об одной сессии сервера (одна сессия == один game_id)
// в течении сессии метаинформация о сервере (название, версия, моды и т.д.) не должны меняться
// ожидается, что сессия длится непрерывный отрезок по времени
//     (однако по наблюдениям сессия может прерываться на очень большой промежуток времени, вплоть до ~30 часов)
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub game_id: GameId,
    // todo подумать точно ли оно нужно
    pub server_id: Option<ServerId>,
    // None если предыдущей/следующей игры не было или она ещё не вычислена
    pub prev_game_id: Option<GameId>,
    pub next_game_id: Option<GameId>,
    // полуинтервал: [time_begin, time_end)
    // todo добавить куда-нибудь проверку, что интервалы соседних игр перекрываются не более чем на 10-15 минут
    pub time_begin: TimeMinutes,
    pub time_end: Option<TimeMinutes>,

    // гарантируется, что все игроки которые сейчас онлайн находятся в конце
    pub players_intervals: Vec<PlayerInterval>,

    pub host_id: HostId,
    pub name: BigStringPart,
    pub description: BigStringPart,
    pub max_players: u32,
    pub game_version: BigStringPart,
    pub game_time_elapsed: u32,
    pub has_password: bool,
    // разделённые символом \x02
    pub tags: BigStringPart,
    pub mod_count: u16,

    // None означает что значение ещё не получено (с помощью запроса на /get-game-details)
    pub host_address: Option<BigStringPart>,
    // None означает что значение ещё не получено или что такое же как у prev_game_id
    // todo: "такое же как у prev_game_id"
    pub mods: Option<Vec<Mod>>,
}

impl Game {
    pub fn number_players_online(&self) -> usize {
        let first_online_player_index = self.players_intervals.iter()
            .rposition(|player_interval| player_interval.end.is_some())
            .map(|index| index + 1)
            .unwrap_or(0);
        self.players_intervals.len() - first_online_player_index
    }

    pub fn maximum_number_players(&self) -> (usize, TimeMinutes) {
        #[derive(Ord, PartialOrd, Eq, PartialEq)]
        enum EventType { Begin, End }

        let now = TimeMinutes::now();
        let mut events = Vec::with_capacity(self.players_intervals.len() * 2);
        for player_interval in &self.players_intervals {
            events.push((player_interval.begin, EventType::Begin));
            events.push((player_interval.end.unwrap_or(now), EventType::End));
        }
        events.sort();

        let mut current_number_players = 0;
        let mut maximum_number_players = 0;
        let mut result_time = TimeMinutes::new(1 /* fake value */).unwrap();
        for (time, event_type) in events {
            if event_type == EventType::Begin {
                current_number_players += 1
            } else {
                current_number_players -= 1
            };
            if current_number_players >= maximum_number_players {
                maximum_number_players = current_number_players;
                result_time = time;
            }
        }

        (maximum_number_players, result_time)
    }

    pub fn number_players_all(&self) -> usize {
        use hashbrown::HashSet;

        self.players_intervals.iter()
            .map(|player_interval| player_interval.player_index)
            .collect::<HashSet<_>>()
            .len()
    }

    /// сумма (число минут которые игрок был онлайн) по всем игрокам
    pub fn total_player_minutes(&self) -> u64 {
        let now = TimeMinutes::now();
        self.players_intervals.iter()
            .map(|player_interval| {
                let begin = player_interval.begin;
                let end = player_interval.end.unwrap_or(now);
                let duration = end.get() - begin.get();
                duration as u64
            })
            .sum()
    }

    pub fn are_details_fetched(&self) -> bool {
        self.host_address.is_some()
    }

    pub fn prev_game<'a>(&self, state: &'a State) -> Option<&'a Game> {
        self.prev_game_id.map(|id| state.get_game(id))
    }

    pub fn next_game<'a>(&self, state: &'a State) -> Option<&'a Game> {
        self.next_game_id.map(|id| state.get_game(id))
    }

    pub fn get_mods<'a>(&'a self, state: &'a State) -> &'a Option<Vec<Mod>> {
        if !self.are_details_fetched() { return &None; }
        match (&self.mods, self.prev_game_id) {
            (mods @ Some(_), _) => mods,
            (None, Some(prev_game_id)) => {
                let prev_game = state.get_game(prev_game_id);
                prev_game.get_mods(state)
            }
            (None, None) => &None,
        }
    }
}

pub type StateLock = Arc<RwLock<State>>;

// pub type GamesMap = std::collections::BTreeMap<GameId, Game>;
pub type GamesMap = crate::util::games_map::GamesMap;

#[derive(Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub games: GamesMap,
    // индексы — ServerId, значения — последний GameId для данного ServerId
    // game_ids[0] == u32::MAX
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

    pub fn get_game_name(&self, id: GameId) -> &str {
        let game = self.get_game(id);
        self.all_game_names.get(game.name).into()
    }

    pub fn get_server_name(&self, id: ServerId) -> &str {
        let game_id = self.get_server_last_game_id(id);
        self.get_game_name(game_id)
    }

    pub fn as_server_id(&self, id: usize) -> Option<ServerId> {
        if 1 <= id && id < self.game_ids.len() {
            Some(ServerId(NonZeroU32::new(id as u32).unwrap()))
        } else {
            None
        }
    }

    pub fn get_server_last_game_id(&self, id: ServerId) -> GameId {
        self.game_ids[id.0.get() as usize].clone()
    }

    pub fn get_server_first_game_id(&self, id: ServerId) -> GameId {
        let mut game_id = self.get_server_last_game_id(id);
        while let Some(prev_game_id) = self.get_game(game_id).prev_game_id {
            game_id = prev_game_id;
        }
        game_id
    }

    // [time_begin, time_end)
    // первые по времени игры в начале
    pub fn get_server_games_in_interval(&self, server_id: ServerId, time_begin: TimeMinutes, time_end: TimeMinutes) -> Vec<GameId> {
        assert!(time_begin < time_end);
        let mut last_game = self.get_game(self.get_server_last_game_id(server_id));
        while last_game.time_begin >= time_end {
            match last_game.prev_game_id {
                Some(prev_game_id) => last_game = self.get_game(prev_game_id),
                None => return Vec::new(),
            }
        }
        if let Some(game_time_end) = last_game.time_end {
            if game_time_end <= time_begin {
                return Vec::new();
            }
        }

        let mut game_ids = vec![last_game.game_id];
        while let Some(game_id) = self.get_game(*game_ids.last().unwrap()).prev_game_id {
            if self.get_game(game_id).time_end.unwrap() > time_begin {
                game_ids.push(game_id);
            } else {
                break;
            }
        }
        game_ids.reverse();
        game_ids
    }

    fn get_game_host(&self, id: GameId) -> Option<&str> {
        let game = self.get_game(id);
        game.host_address.map(|host_address| self.all_host_addresses.get(host_address).into())
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

    pub fn compress(&mut self) {
        self.compress_big_strings();
        self.compress_mods();
    }

    fn compress_big_strings(&mut self) {
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
            game.description = *map_descriptions.get(&game.description).unwrap();
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

    fn compress_mods(&mut self) {
        // игры у которых такие же моды как у prev_game
        let mut games_with_same_mods = Vec::new();
        for prev_game in self.games.values() {
            let prev_game_mods = &prev_game.mods;
            match prev_game_mods {
                Some(prev_game_mods) if prev_game_mods.is_empty() => continue,
                None => continue,
                _ => {}
            }

            let mut next_game_id = prev_game.next_game_id;
            while let Some(game_id) = next_game_id {
                let game = self.get_game(game_id);
                next_game_id = game.next_game_id;

                if game.mods.is_none() { continue; }
                if &game.mods == prev_game_mods {
                    games_with_same_mods.push(game_id);
                }
                break;
            }
        }

        println!("[info]  [state] cleared mods in {} games", games_with_same_mods.len());
        for game_id in games_with_same_mods {
            let game = self.get_game_mut(game_id);
            game.mods = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        use std::mem::size_of;
        assert_eq!(size_of::<PlayerInterval>(), 12);
        assert_eq!(size_of::<Option<ServerId>>(), 4);
        assert_eq!(size_of::<Game>(), 136);
    }
}
