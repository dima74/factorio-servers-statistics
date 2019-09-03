#![feature(duration_constants)]
#![feature(inner_deref)]
#![feature(slice_partition_at_index)]
#![feature(type_ascription)]
#![feature(div_duration)]

pub mod api;
pub mod external_storage;
pub mod fetcher_get_games;
pub mod fetcher_get_game_details;
pub mod fetcher_get_games_offline;
pub mod state;
pub mod util;
pub mod analytics;
pub mod global_config;
pub mod cacher;

#[cfg(test)]
pub mod tests;
