use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref GLOBAL_CONFIG: Mutex<GlobalConfig> = Mutex::new(GlobalConfig::new());
}

pub struct GlobalConfig {
    pub fetcher_get_games_skip_first_sleep: bool,
    pub use_cache_for_get_game_details: bool,
}

impl GlobalConfig {
    fn new() -> GlobalConfig {
        GlobalConfig {
            fetcher_get_games_skip_first_sleep: false,
            use_cache_for_get_game_details: false,
        }
    }
}
