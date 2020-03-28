use rusoto_s3::{GetObjectRequest, PutObjectRequest, S3, S3Client};
use std::io::Read;

const REGION: &str = "storage.yandexcloud.net";
const BUCKET: &str = "factorio-servers-statistics";
const BUCKET_KEY: &str = "temp.txt";
const MESSAGE: &str = "Example message";

#[test]
fn main() {
    dotenv::dotenv().ok();

    let credentials_provider = rusoto_credential::EnvironmentProvider::default();
    let region = rusoto_core::Region::Custom { name: "us-east-1".into(), endpoint: REGION.into() };
    let s3_client = S3Client::new_with(rusoto_core::HttpClient::new().unwrap(), credentials_provider, region);

    s3_client.put_object(PutObjectRequest {
        bucket: BUCKET.into(),
        key: BUCKET_KEY.into(),
        content_type: Some("text/plain".to_owned()),
        body: Some(MESSAGE.to_owned().into_bytes().into()),
        ..Default::default()
    }).sync().expect("could not upload");

    let get_object_result = s3_client.get_object(GetObjectRequest {
        bucket: BUCKET.into(),
        key: BUCKET_KEY.into(),
        ..Default::default()
    }).sync().expect("could not download");

    let mut message_stream = get_object_result.body.unwrap().into_blocking_read();
    let mut message = String::new();
    message_stream.read_to_string(&mut message).unwrap();
    assert_eq!(MESSAGE, message);
}
