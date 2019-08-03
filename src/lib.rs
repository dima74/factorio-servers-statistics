#![feature(duration_constants)]
#![feature(duration_float)]
#![feature(inner_deref)]
// todo
#![allow(warnings)]

pub mod api;
pub mod external_storage;
pub mod fetcher_get_games;
pub mod fetcher_get_game_details;
pub mod fetcher_get_games_offline;
pub mod state;
pub mod util;
pub mod analytics;
pub mod global_config;
