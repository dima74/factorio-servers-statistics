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

/// runs `runnable` at most `number_retries` times
/// after each failure calls `error_handler` (which is supposed for logging)
pub fn run_with_retries<R, E, F, H>(number_retries: usize, mut runnable: F, mut error_handler: H) -> Result<R, E>
    where
        F: FnMut() -> Result<R, E>,
        H: FnMut(usize, &E),
{
    assert!(number_retries >= 1);
    for request_index in 0..number_retries {
        match runnable() {
            result @ Ok(_) => return result,
            Err(err) => {
                error_handler(request_index, &err);
                std::thread::sleep(Duration::from_secs(f32::powf(1.5, request_index as f32) as u64));

                if request_index + 1 == number_retries {
                    return Err(err);
                }
            }
        }
    }
    unreachable!()
}
