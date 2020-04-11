use std::num::NonZeroU32;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

// todo documentation
#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct BigString {
    #[serde(skip)]
    debug_name: String,
    // utf8-строка содержащая последовательность подстрок, разделённых символом \x00
    // ["aa", "bb", "cc"] == \x00 aa \x00 bb \x00 cc \x00
    content: Vec<u8>,
}

impl BigString {
    pub fn new() -> Self {
        BigString {
            debug_name: String::new(),
            content: vec![0],
        }
    }

    pub fn set_debug_name(&mut self, debug_name: String) {
        self.debug_name = debug_name;
    }

    pub fn add(&mut self, string: &str) -> BigStringPart {
        let string = if string.contains('\x00') {
            eprintln!("[warn]  found \\x00 in BigStringPart");
            string.replace('\x00', "\x01")
        } else {
            // todo лишнее копирование :)
            string.to_owned()
        };

        self.add_vec(string.as_bytes())
    }

    pub fn add_vec(&mut self, string: &[u8]) -> BigStringPart {
        let part_index = self.content.len() as u32;
        self.content.extend_from_slice(string);
        self.content.push(0);
        BigStringPart(NonZeroU32::new(part_index).unwrap())
    }

    // todo return &str ?
    pub fn get(&self, part_index: BigStringPart) -> FssStr {
        let begin = part_index.0.get() as usize;
        let length = self.content[begin..].iter()
            .position(|&byte| byte == 0)
            .unwrap();
        FssStr(&self.content[begin..begin + length])
    }

    pub fn get_str(&self, part_index: BigStringPart) -> String {
        self.get(part_index).into()
    }

    pub fn compress(&mut self) -> HashMap<BigStringPart, BigStringPart> /* old index → new index */ {
        let mut new_index_by_part: HashMap<&[u8], usize> = HashMap::new();
        let mut new_index_by_old_index: HashMap<BigStringPart, BigStringPart> = HashMap::new();

        // (new_index, old_index, part_length)
        let mut part_moves: Vec<(usize, usize, usize)> = Vec::new();

        let mut next_part_index = 1;
        let mut part_begin = 1;
        while part_begin != self.content.len() {
            let part_end = part_begin + self.content[part_begin..].iter().position(|&c| c == 0).unwrap();
            let part = &self.content[part_begin..part_end];

            let new_part_index = match new_index_by_part.get(part) {
                Some(&index) => index,
                None => {
                    let new_part_index = next_part_index;
                    next_part_index += part.len() + 1;
                    new_index_by_part.insert(part, new_part_index);
                    new_part_index
                }
            };
            new_index_by_old_index.insert(
                BigStringPart(NonZeroU32::new(part_begin as u32).unwrap()),
                BigStringPart(NonZeroU32::new(new_part_index as u32).unwrap()),
            );
            part_moves.push((new_part_index, part_begin, part.len()));

            part_begin = part_end + 1;
        }

        for (new_index, old_index, part_length) in part_moves {
            assert!(new_index <= old_index);
            for i in 0..part_length {
                self.content[new_index + i] = self.content[old_index + i];
            }
            self.content[new_index - 1] = 0;
            self.content[new_index + part_length] = 0;
        }
        println!("[info]  [big_string] {:20}: {} → {}", self.debug_name, self.content.len(), next_part_index);
        self.content.truncate(next_part_index);

        new_index_by_old_index
    }
}

// индекс подстроки в большой строке
#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct BigStringPart(NonZeroU32);

// type-safe часть (sub-slice) BigString
#[derive(Copy, Clone)]
pub struct FssStr<'a> (pub &'a [u8]);

impl<'a> Into<&'a str> for FssStr<'a> {
    fn into(self) -> &'a str {
        // todo from_utf8_unchecked ?
        std::str::from_utf8(self.0).unwrap()
    }
}

impl<'a> Into<String> for FssStr<'a> {
    fn into(self) -> String {
        std::str::from_utf8(self.0).unwrap().to_owned()
    }
}

#[derive(Eq, PartialEq, Hash)]
pub struct FssString(pub Vec<u8>);

impl<'a> From<FssStr<'a>> for FssString {
    fn from(string: FssStr) -> Self {
        FssString(string.0.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(std::mem::size_of::<Option<BigStringPart>>(), 4);
    }

    #[test]
    fn basic() {
        let mut big_string = BigString::new();
        let part_index = big_string.add("hello");
        assert_eq!(big_string.get_str(part_index), "hello");
    }

    #[test]
    fn compress() {
        let mut big_string = BigString::new();
        let bbb1 = big_string.add("bbb");
        let aaa1 = big_string.add("aaaa");
        let bbb2 = big_string.add("bbb");
        let aaa2 = big_string.add("aaaa");
        let ccc1 = big_string.add("cc");
        let bbb3 = big_string.add("bbb");

        let map = big_string.compress();
        assert_eq!(big_string.content, b"\x00bbb\x00aaaa\x00cc\x00");
        assert_eq!(big_string.get_str(*map.get(&bbb1).unwrap()), "bbb");
        assert_eq!(big_string.get_str(*map.get(&bbb2).unwrap()), "bbb");
        assert_eq!(big_string.get_str(*map.get(&bbb3).unwrap()), "bbb");
        assert_eq!(big_string.get_str(*map.get(&aaa1).unwrap()), "aaaa");
        assert_eq!(big_string.get_str(*map.get(&aaa2).unwrap()), "aaaa");
        assert_eq!(big_string.get_str(*map.get(&ccc1).unwrap()), "cc");
    }
}
