use std::cmp::min;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use rand::Rng;

use crate::yandex_cloud_storage::upload_with_retries;

fn generate_random_file(path: &Path, size: usize) {
    let f = File::create(path).unwrap();
    let mut writer = BufWriter::new(f);

    let mut rng = rand::thread_rng();
    let mut buffer = [0; 1024];
    let mut remaining_size = size;

    while remaining_size > 0 {
        let to_write = min(remaining_size, buffer.len());
        let buffer = &mut buffer[..to_write];
        rng.fill(buffer);
        writer.write(buffer).unwrap();

        remaining_size -= to_write;
    }
}

/// Тест для воспроизвденеия ошибки "dispatch dropped without returning error"
/// Но почему-то не воспроизводится
// #[test]
fn main() {
    dotenv::dotenv().ok();

    const NUMBER_REPEATS: usize = 1000;
    const FILE_SIZE: usize = 1024 * 1024;

    let temp_file = std::env::temp_dir().join("file1.txt");
    generate_random_file(&temp_file, FILE_SIZE);

    for i in 0..NUMBER_REPEATS {
        println!("{}", i);

        let bucket_key = "temp/temp.bin".to_owned();
        upload_with_retries(&bucket_key, &temp_file, "text/plain", 5);
    }
}
