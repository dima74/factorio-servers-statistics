use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref GLOBAL_CONFIG: Mutex<GlobalConfig> = Mutex::new(GlobalConfig::new());
}

pub struct GlobalConfig {
    pub fetcher_get_games_skip_first_sleep: bool,
}

impl GlobalConfig {
    fn new() -> GlobalConfig {
        GlobalConfig {
            fetcher_get_games_skip_first_sleep: false
        }
    }
}
