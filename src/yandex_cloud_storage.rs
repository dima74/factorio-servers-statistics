use std::io;
use std::path::Path;

use rusoto_core::RusotoError;
use rusoto_s3::{GetObjectRequest, ListObjectsV2Request, PutObjectRequest, S3, S3Client, StreamingBody};

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

pub fn list_bucket(key: &str) -> Vec<String> {
    let mut key = key.to_owned();
    if !key.ends_with('/') {
        key.push('/');
    }

    let result = YANDEX_CLOUD.s3_client.list_objects_v2(ListObjectsV2Request {
        bucket: "factorio-servers-statistics".to_owned(),
        start_after: Some(key.to_owned()),
        ..Default::default()
    }).sync()
        .unwrap_or_else(|err| panic!(format!(
            "[error] [yandex_cloud] Can't list bucket `{}`: {}", key, err)));

    result.contents.unwrap_or_default().into_iter()
        .filter_map(|object| object.key)
        .collect()
}

fn get_rusoto_streaming_body(filename: &Path) -> StreamingBody {
    let file = std::fs::File::open(filename).unwrap();

    // https://stackoverflow.com/a/57812269/5812238
    use tokio::codec;
    use tokio::prelude::Stream;
    let file = tokio::fs::File::from_std(file);
    let file = codec::FramedRead::new(file, codec::BytesCodec::new())
        .map(|r| r.freeze());

    StreamingBody::new(file)
}

pub fn upload(key: &str, filename: &Path, content_type: &str) {
    let streaming_body = get_rusoto_streaming_body(filename);
    let result = YANDEX_CLOUD.s3_client.put_object(PutObjectRequest {
        bucket: BUCKET.to_owned(),
        key: key.to_string(),
        body: Some(streaming_body),
        content_type: Some(content_type.to_owned()),
        ..Default::default()
    }).sync();

    if let Err(err) = result {
        eprintln!("[error] [yandex_cloud] Can't upload: {}", err);
        if let RusotoError::Unknown(err) = err {
            eprintln!("Can't upload{:?}", err.body);
        }
    }
}

pub fn download(key: &str) -> impl io::Read {
    let get_request = GetObjectRequest {
        bucket: BUCKET.to_owned(),
        key: key.to_owned(),
        ..Default::default()
    };

    let s3_client: &S3Client = &YANDEX_CLOUD.s3_client;
    let result = s3_client.get_object(get_request).sync()
        .expect(&format!("Couldn't download {} object from Yandex.Cloud", key));
    result.body.unwrap().into_blocking_read()
}
