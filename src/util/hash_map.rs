use std::borrow::Borrow;
use std::hash::Hash;

pub use deserialize::*;
pub use into_iterator::*;
pub use serialize::*;

// We use BTreeMap to get stable order of entries for serialization and because in future probably storage will be changed (see todo in FssHashMap)
// type KeyToIndexMap<K, V> = hashbrown::hash_map::HashMap<K, V>;
type KeyToIndexMap<K, V> = std::collections::BTreeMap<K, V>;

// memory-efficient hash map designed for case when sizeof K is small (<20 bytes) and sizeof V is big (>40 bytes)
// will be used for storing state.games (Key is GameId and has size 4, Value is Game and has size ~130)
#[derive(Eq, PartialEq)]
pub struct FssMap<K, V> {
    // todo consider using Vec<K> for keys (and add K: Ord)
    //  because currently we never remove games and new game has greater GameId than all previous games
    key_to_index: KeyToIndexMap<K, u32>,
    values: Vec<V>,
}

impl<K: Eq + Hash + Ord, V> FssMap<K, V> {
    pub fn new() -> Self {
        FssMap {
            key_to_index: KeyToIndexMap::new(),
            values: vec![],
        }
    }

    pub fn with_capacity(capacity0: usize) -> Self {
        /// see [crate::analytics::print_average_number_new_games_per_day]
        /// as per April 2020, maximum number new games per day is ~9000
        const MAXIMUM_NUMBER_NEW_GAMES_PER_DAY: usize = 10000 * 4;
        let capacity = capacity0 + MAXIMUM_NUMBER_NEW_GAMES_PER_DAY;
        FssMap {
            key_to_index: KeyToIndexMap::new(),
            values: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn get<Q>(&self, k: &Q) -> Option<&V>
        where
            K: Borrow<Q>,
            Q: Eq + Hash + Ord,
    {
        let index = self.key_to_index.get(k)?;
        let index = *index as usize;
        Some(&self.values[index])
    }

    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
        where
            K: Borrow<Q>,
            Q: Eq + Hash + Ord,
    {
        let index = self.key_to_index.get(k)?;
        let index = *index as usize;
        Some(&mut self.values[index])
    }

    pub fn contains_key<Q>(&self, k: &Q) -> bool
        where
            K: Borrow<Q>,
            Q: Eq + Hash + Ord,
    {
        self.get(k).is_some()
    }

    pub fn insert(&mut self, k: K, v: V) {
        if self.values.capacity() == self.values.len() {
            eprintln!("[error] [FssHashMap] reallocation during insert: len and capacity is {}", self.values.len());
        }

        let index = self.values.len() as u32;
        self.key_to_index.insert(k, index);
        self.values.push(v);
    }

    pub fn keys(&self) -> impl Iterator<Item=&K> {
        self.key_to_index.keys()
    }

    pub fn values(&self) -> impl Iterator<Item=&V> {
        self.values.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item=&mut V> {
        self.values.iter_mut()
    }
}

mod into_iterator {
    use super::FssMap;

    pub struct IntoIteratorHelperClosure<'a, K, V> {
        map: &'a FssMap<K, V>,
    }

    impl<'a, K, V> FnOnce<((&'a K, &'a u32), )> for IntoIteratorHelperClosure<'a, K, V> {
        type Output = (&'a K, &'a V);

        extern "rust-call" fn call_once(mut self, args: ((&'a K, &'a u32), )) -> Self::Output {
            self.call_mut(args)
        }
    }

    impl<'a, K, V> FnMut<((&'a K, &'a u32), )> for IntoIteratorHelperClosure<'a, K, V> {
        extern "rust-call" fn call_mut(&mut self, ((key, &index), ): ((&'a K, &'a u32), )) -> Self::Output {
            (key, &self.map.values[index as usize])
        }
    }

    impl<'a, K, V> IntoIterator for &'a FssMap<K, V> {
        type Item = (&'a K, &'a V);

        // todo rewrite with existential types when they become stable
        //  https://github.com/rust-lang/rust/issues/66551
        //  https://github.com/rust-lang/rust/issues/62988
        // type IntoIter = impl Iterator<Item = Self::Item>;
        // type IntoIter = std::iter::Map<hash_map::Iter<'a, K, u32>, fn((&'a K, &u32)) -> (&'a K, &'a V)>;

        type IntoIter = std::iter::Map<std::collections::btree_map::Iter<'a, K, u32>, IntoIteratorHelperClosure<'a, K, V>>;

        fn into_iter(self: &'a FssMap<K, V>) -> Self::IntoIter {
            let closure = IntoIteratorHelperClosure { map: self };
            self.key_to_index.iter().map(closure)
        }
    }
}

// https://serde.rs/impl-serialize.html#serializing-a-sequence-or-map
mod serialize {
    use serde::{Serialize, Serializer};

    use super::FssMap;

    impl<K: Serialize, V: Serialize> Serialize for FssMap<K, V> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_map(self)
        }
    }
}

// https://serde.rs/deserialize-map.html
mod deserialize {
    use core::fmt;
    use std::hash::Hash;
    use std::marker::PhantomData;

    use serde::{Deserialize, Deserializer};
    use serde::de::{MapAccess, Visitor};

    use super::FssMap;

    struct FssHashMapVisitor<K, V> {
        marker: PhantomData<fn() -> FssMap<K, V>>
    }

    impl<K, V> FssHashMapVisitor<K, V> {
        fn new() -> Self {
            FssHashMapVisitor { marker: PhantomData }
        }
    }

    impl<'de, K, V> Visitor<'de> for FssHashMapVisitor<K, V>
        where
            K: Eq + Hash + Ord + Deserialize<'de>,
            V: Deserialize<'de>
    {
        type Value = FssMap<K, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("fss map")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
            let mut map = FssMap::with_capacity(access.size_hint().unwrap_or(0));
            while let Some((key, value)) = access.next_entry()? {
                map.insert(key, value);
            }
            Ok(map)
        }
    }

    impl<'de, K, V> Deserialize<'de> for FssMap<K, V>
        where
            K: Eq + Hash + Ord + Deserialize<'de>,
            V: Deserialize<'de>
    {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_map(FssHashMapVisitor::new())
        }
    }
}
