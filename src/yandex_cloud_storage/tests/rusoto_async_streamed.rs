use crate::yandex_cloud_storage::{download_to_file, upload};

const BUCKET_KEY: &str = "temp.txt";
const MESSAGE: &str = "Example message";

#[test]
fn main() {
    dotenv::dotenv().ok();

    let temp_file1 = std::env::temp_dir().join("file1.txt");
    std::fs::write(&temp_file1, MESSAGE).unwrap();
    upload(BUCKET_KEY, &temp_file1, "text/plain").unwrap();

    let temp_file2 = std::env::temp_dir().join("file2.txt");
    download_to_file(BUCKET_KEY, &temp_file2).unwrap();
    let result = std::fs::read_to_string(temp_file2).unwrap();

    assert_eq!(result, MESSAGE);
}
