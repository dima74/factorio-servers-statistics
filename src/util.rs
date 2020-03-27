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

pub fn print_heap_stats() {
    use jemalloc_ctl::{stats, epoch};

    // many statistics are cached and only updated when the epoch is advanced.
    epoch::advance().unwrap();

    let allocated = stats::allocated::read().unwrap();
    let resident = stats::resident::read().unwrap();
    const MB: usize = 1024 * 1024;
    println!("{} MB allocated / {} MB resident", allocated / MB, resident / MB);
}
