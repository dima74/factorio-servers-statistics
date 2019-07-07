use std::time::{Duration, SystemTime};

pub fn duration_since(later: SystemTime, earlier: SystemTime) -> Duration {
    later.duration_since(earlier).expect("Time went backwards")
}
