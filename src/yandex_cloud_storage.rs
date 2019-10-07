use std::error::Error;
use std::io;
use std::path::Path;

use rusoto_core::RusotoError;
use rusoto_s3::{DeleteObjectRequest, GetObjectRequest, ListObjectsV2Request, PutObjectRequest, S3, S3Client, StreamingBody};

use lazy_static::lazy_static;

const BUCKET: &str = "factorio-servers-statistics";

struct YandexCloud {
    s3_client: S3Client,
}

impl YandexCloud {
    pub fn new() -> Self {
        let credentials_provider = rusoto_credential::EnvironmentProvider::default();
        let region = rusoto_core::Region::Custom {
            name: String::from("us-east-1"),
            endpoint: String::from("storage.yandexcloud.net"),
        };
        let s3_client = S3Client::new_with(rusoto_core::HttpClient::new().unwrap(), credentials_provider, region);

        YandexCloud {
            s3_client,
        }
    }
}

lazy_static! {
    static ref YANDEX_CLOUD: YandexCloud = YandexCloud::new();
}

pub fn list_bucket(path: &str) -> Vec<String> {
    let mut path = path.to_owned();
    if !path.ends_with('/') {
        path.push('/');
    }

    let result = YANDEX_CLOUD.s3_client.list_objects_v2(ListObjectsV2Request {
        bucket: "factorio-servers-statistics".to_owned(),
        prefix: Some(path.to_owned()),
        ..Default::default()
    }).sync()
        .unwrap_or_else(|err| panic!(format!(
            "[error] [yandex_cloud] Can't list bucket `{}`: {}", path, err)));

    result.contents.unwrap_or_default().into_iter()
        .filter_map(|object| object.key)
        .filter(|key| key != &path)
        .collect()
}

fn get_rusoto_streaming_body(filename: &Path) -> (StreamingBody, u64) {
    let file = std::fs::File::open(filename).unwrap();
    file.sync_all().unwrap();
    let file_length = file.metadata().unwrap().len();

    // https://stackoverflow.com/a/57812269/5812238
    use tokio::codec;
    use tokio::prelude::Stream;
    let file = tokio::fs::File::from_std(file);
    let file = codec::FramedRead::new(file, codec::BytesCodec::new())
        .map(|r| r.freeze());

    (StreamingBody::new(file), file_length)
}

pub fn upload(path: &str, filename: &Path, content_type: &str) {
    let (streaming_body, file_length) = get_rusoto_streaming_body(filename);
    let put_request = PutObjectRequest {
        bucket: BUCKET.to_owned(),
        key: path.to_string(),
        body: Some(streaming_body),
        // Yandex.Cloud стал требовать заголовок Content-Length 07.10.2019 ~16:00 UTC
        content_length: Some(file_length as i64),
        content_type: Some(content_type.to_owned()),
        ..Default::default()
    };

    let result = YANDEX_CLOUD.s3_client.put_object(put_request).sync();
    if let Err(err) = &result {
        eprintln!("[error] [yandex_cloud] Can't upload: {}", err);
        if let RusotoError::Unknown(err) = err {
            eprintln!("[error] [yandex_cloud] Can't upload: {:?}", err.body);
        }

        // todo
        result.unwrap();
    }
}

pub fn download(path: &str) -> impl io::Read {
    let get_request = GetObjectRequest {
        bucket: BUCKET.to_owned(),
        key: path.to_owned(),
        ..Default::default()
    };

    let result = YANDEX_CLOUD.s3_client.get_object(get_request).sync()
        .expect(&format!("Couldn't download {} object from Yandex.Cloud", path));
    result.body.unwrap().into_blocking_read()
}

pub fn delete(path: &str) -> Result<(), Box<dyn Error>> {
    let delete_request = DeleteObjectRequest {
        bucket: BUCKET.to_owned(),
        key: path.to_owned(),
        ..Default::default()
    };
    YANDEX_CLOUD.s3_client.delete_object(delete_request).sync()?;
    Ok(())
}
