pub use deserialize::*;
pub use into_iterator::*;
pub use serialize::*;

use crate::state::{Game, GameId};

// memory-efficient hash map designed for case when sizeof K is small (<20 bytes) and sizeof V is big (>40 bytes)
// is used for storing state.games (Key is GameId and has size 4, Value is Game and has size ~130)
#[derive(Eq, PartialEq)]
pub struct GamesMap {
    // ordered by game_id
    values: Vec<Game>,
}

impl GamesMap {
    pub fn new() -> Self {
        GamesMap { values: Vec::new() }
    }

    pub fn with_capacity(capacity0: usize) -> Self {
        /// see [crate::analytics::print_average_number_new_games_per_day]
        /// as per April 2020, maximum number new games per day is ~9000
        const MAXIMUM_NUMBER_NEW_GAMES_PER_DAY: usize = 10000 * 4;
        let capacity = capacity0 + MAXIMUM_NUMBER_NEW_GAMES_PER_DAY;
        GamesMap { values: Vec::with_capacity(capacity) }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn get(&self, k: &GameId) -> Option<&Game> {
        let index = self.values.binary_search_by_key(k, |game| game.game_id);
        let index = index.ok()?;
        Some(&self.values[index])
    }

    pub fn get_mut(&mut self, k: &GameId) -> Option<&mut Game> {
        let index = self.values.binary_search_by_key(k, |game| game.game_id);
        let index = index.ok()?;
        Some(&mut self.values[index])
    }

    pub fn contains_key(&self, k: &GameId) -> bool {
        self.get(k).is_some()
    }

    pub fn insert(&mut self, k: GameId, v: Game) {
        if self.values.capacity() == self.values.len() {
            eprintln!("[error] [GamesMap] reallocation during insert: len and capacity is {}", self.values.len());
        }

        match self.values.last() {
            None => {
                self.values.push(v)
            }
            Some(last_game) if last_game.game_id < k => {
                self.values.push(v)
            }
            _ => {
                /* При корректной работе код не должен заходить в данную ветку. */

                // Это означает что существует game_id (k),
                // и два момента времени t1 и t2 (current), такие что:
                // - в момент времени t1 в snapshot не было game_id, но была какая-то игра с большим game_id
                // - в момент времени t2 в snapshot был game_id
                // То есть, кажется (?), игра с таким game_id была раньше, пропала, и снова появилась
                let last_game_id = self.values.last().map(|game| game.game_id);
                eprintln!("[warn]  [GamesMap] adding game with inconsistent id {} (last_game_id={:?})", k, last_game_id);

                match self.values.binary_search_by_key(&k, |game| game.game_id) {
                    Ok(_) => panic!("GamesMap already contains game with id {}", k),
                    Err(index) => {
                        self.values.insert(index, v);
                    }
                }
            }
        }
    }

    pub fn values(&self) -> impl Iterator<Item=&Game> {
        self.values.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item=&mut Game> {
        self.values.iter_mut()
    }
}

mod into_iterator {
    use crate::state::{Game, GameId};

    use super::GamesMap;

    impl<'a> IntoIterator for &'a GamesMap {
        type Item = (&'a GameId, &'a Game);
        type IntoIter = impl Iterator<Item=Self::Item>;

        fn into_iter(self: &'a GamesMap) -> Self::IntoIter {
            self.values.iter().map(|game| (&game.game_id, game))
        }
    }
}

// https://serde.rs/impl-serialize.html#serializing-a-sequence-or-map
mod serialize {
    use serde::{Serialize, Serializer};

    use super::GamesMap;

    impl Serialize for GamesMap {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_map(self)
        }
    }
}

// https://serde.rs/deserialize-map.html
mod deserialize {
    use core::fmt;
    use std::marker::PhantomData;

    use serde::{Deserialize, Deserializer};
    use serde::de::{MapAccess, Visitor};

    use super::GamesMap;

    struct FssHashMapVisitor {
        marker: PhantomData<fn() -> GamesMap>
    }

    impl FssHashMapVisitor {
        fn new() -> Self {
            FssHashMapVisitor { marker: PhantomData }
        }
    }

    // todo remove K, V
    impl<'de> Visitor<'de> for FssHashMapVisitor {
        type Value = GamesMap;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("fss map")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            let capacity = access.size_hint().unwrap_or(0);
            let mut map = GamesMap::with_capacity(capacity);
            while let Some((key, value)) = access.next_entry()? {
                map.insert(key, value);
            }
            Ok(map)
        }
    }

    impl<'de> Deserialize<'de> for GamesMap {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_map(FssHashMapVisitor::new())
        }
    }
}
