#![feature(duration_constants)]
#![feature(slice_partition_at_index)]
#![feature(div_duration)]
#![feature(type_alias_impl_trait)]

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
pub mod yandex_cloud_storage;

#[cfg(test)]
pub mod tests;
