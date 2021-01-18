use std::io::{BufReader, BufWriter, Read, Write};
use std::time::{Duration, SystemTime};

pub mod games_map;
pub mod map_deref;

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
pub fn run_with_retries<R, E>(
    number_retries: usize,
    mut runnable: impl FnMut() -> Result<R, E>,
    mut error_handler: impl FnMut(usize, &E),
) -> Result<R, E> {
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

const BUFFER_SIZE: usize = 64 * 1024;  // 64KiB

pub fn new_buf_reader(reader: impl Read) -> BufReader<impl Read> {
    BufReader::with_capacity(BUFFER_SIZE, reader)
}

pub fn new_buf_writer(writer: impl std::io::Write) -> BufWriter<impl Write> {
    BufWriter::with_capacity(BUFFER_SIZE, writer)
}
