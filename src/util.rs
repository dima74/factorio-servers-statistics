use std::time::{Duration, SystemTime};

pub fn duration_since(later: SystemTime, earlier: SystemTime) -> Duration {
    later.duration_since(earlier).expect("Time went backwards")
}

pub fn basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}
